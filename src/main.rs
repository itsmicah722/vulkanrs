//! Vulkan Application with the goal to render a triangle.

#![deny(unsafe_op_in_unsafe_fn)]
#![allow(
    dead_code,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

use std::{
    collections::HashSet,
    ffi::{CStr, c_void},
};

use anyhow::{Context, Result, anyhow};
use log::*;
use thiserror::Error;
use vulkanalia::{
    Device, Entry, Instance, Version,
    loader::{LIBRARY, LibloadingLoader},
    vk,
    vk::{
        DeviceV1_0, EntryV1_0, ExtDebugUtilsExtension, HasBuilder, InstanceV1_0, PhysicalDeviceType,
    },
    window as vk_window,
};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

const PORTABILITY_MACOS_VERSION: Version = Version::new(1, 3, 216);
const VALIDATION_ENABLED: bool = cfg!(debug_assertions);
const VALIDATION_LAYER: vk::ExtensionName =
    vk::ExtensionName::from_bytes(c"VK_LAYER_KHRONOS_validation".to_bytes_with_nul());

fn main() -> Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "trace");
    }
    pretty_env_logger::init();

    // Create the event loop.
    let event_loop = EventLoop::new().context("Failed to create event loop.")?;

    // Create the window.
    let window = WindowBuilder::new()
        .with_title("Vulkan App")
        .with_inner_size(LogicalSize::new(600, 400))
        .with_active(false)
        .build(&event_loop)
        .context("Failed to create window.")?;

    // Create the Vulkan app
    let mut app = unsafe { App::create(&window).context("Failed to create Vulkan App.")? };

    // Run the event loop
    event_loop.run(move |event, window_target| {
        match event {
            // Request a redraw when all events have been processed (so we can render continuously
            // even when there is no input from the OS)
            Event::AboutToWait => window.request_redraw(),

            // Check for window events (e.g. keyboard input, window close, etc.)
            Event::WindowEvent { event, .. } => match event {
                // Render a frame if the App is not being destroyed.
                WindowEvent::RedrawRequested if !window_target.exiting() => {
                    unsafe { app.render(&window) }.unwrap();
                }

                // Destroy the App.
                WindowEvent::CloseRequested => {
                    window_target.exit();
                    unsafe {
                        app.destroy();
                    }
                }

                _ => {}
            },

            _ => {}
        }
    })?;

    Ok(())
}

/// All Vulkan handles and associated properties used by `App`.
#[derive(Clone, Debug, Default)]
struct AppData {
    messenger: vk::DebugUtilsMessengerEXT,
    physical_device: vk::PhysicalDevice,
    graphics_queue: vk::Queue,
}

/// Main `App` which implements Vulkan boilerplate functionality.
#[derive(Clone, Debug)]
struct App {
    entry: Entry,
    instance: Instance,
    device: Device,
    data: AppData,
}

impl App {
    /// Initializes Vulkan resources tied to App.
    unsafe fn create(window: &Window) -> Result<Self> {
        let (entry, instance, data, device) = unsafe {
            let loader = LibloadingLoader::new(LIBRARY)?;
            let entry = Entry::new(loader).map_err(|b| anyhow!("{}", b))?;
            let mut data = AppData::default();
            let instance = create_instance(window, &entry, &mut data)?;
            pick_physical_device(&instance, &mut data)?;
            let device = create_logical_device(&entry, &instance, &mut data)?;

            (entry, instance, data, device)
        };

        Ok(Self {
            entry,
            instance,
            device,
            data,
        })
    }

    /// Renders a frame for our Vulkan App.
    unsafe fn render(&mut self, window: &Window) -> Result<()> {
        Ok(())
    }

    /// Destroys Vulkan resources tied to App.
    unsafe fn destroy(&mut self) {
        unsafe {
            self.device.destroy_device(None);

            if VALIDATION_ENABLED {
                self.instance
                    .destroy_debug_utils_messenger_ext(self.data.messenger, None);
                trace!("Destroyed Vulkan debug messenger.")
            }

            self.instance.destroy_instance(None);
            trace!("Destroyed Vulkan instance.")
        }
    }
}

unsafe fn create_logical_device(
    entry: &Entry,
    instance: &Instance,
    data: &mut AppData,
) -> Result<Device> {
    let indices = unsafe { QueueFamilyIndices::get(instance, data, data.physical_device)? };

    // Set the queue priority to highest and provide the queue create info with the family index
    // and priority float.
    let queue_priorities = &[1.0];
    let queue_info = vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(indices.graphics)
        .queue_priorities(queue_priorities);

    // Give the device level layer validation layer support (for compatibility of older
    // vulkan implementations).
    let layers = if VALIDATION_ENABLED {
        vec![VALIDATION_LAYER.as_ptr()]
    } else {
        vec![]
    };

    // Give the device level extensions portability support if the host is running macOS.
    let mut extensions = vec![];
    if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
        extensions.push(vk::KHR_PORTABILITY_SUBSET_EXTENSION.name.as_ptr());
    }

    // Get the physical device's supported features.
    let features = vk::PhysicalDeviceFeatures::builder();

    // A single logical device can be created with multiple queues from different queue families,
    // each with their own capabilities and priority. (in this case we only are using graphics
    // queue)
    let queue_infos = &[queue_info];

    // Setup logical device create info with all queue info structs, layers, extensions, and
    // features.
    let device_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(queue_infos)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .enabled_features(&features);

    // Create the Vulkan logical device.
    // let device = unsafe { instance.create_device(data.physical_device, &device_info, None)? };

    let device = unsafe {
        match instance.create_device(data.physical_device, &device_info, None) {
            Ok(device) => {
                trace!("Created Vulkan logical device.");
                device
            }

            Err(e) => {
                error!("Failed to create Vulkan logical device.");
                return Err(e.into());
            }
        }
    };

    // Set the graphics queue to the graphics family index.
    data.graphics_queue = unsafe { device.get_device_queue(indices.graphics, 0) };

    Ok(device)
}

#[derive(Debug, Error)]
#[error("Missing {0}.")]
pub struct SuitabilityError(pub &'static str);

/// A family queue contains different operations available on a physical device, (e.g. graphics,
/// compute, transfer). These 'families' are identified by their index, starting from 0. The
/// most important queue is the graphics queue, as this gives us the capability to render.
#[derive(Copy, Clone, Debug)]
struct QueueFamilyIndices {
    graphics: u32,
}

impl QueueFamilyIndices {
    /// Get the index of the queue family which supports the "graphics" operation and
    /// return it as an u32. If no family is found with graphics support, the function returns
    /// with an error, otherwise the u32 index is returned (usually 0).
    unsafe fn get(
        instance: &Instance,
        data: &AppData,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self> {
        let properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        // Iterate through the physical device queue family properties for the graphics queue flags.
        // If the graphics flag exists, convert it to u32 (index).
        let graphics = properties
            .iter()
            .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .map(|i| i as u32);

        match graphics {
            Some(graphics) => {
                Ok(Self { graphics })
            }
            None => Err(anyhow!(SuitabilityError(
                "Missing required graphics queue families."
            ))),
        }
    }
}

/// Check whether the Vulkan physical device (GPU) contains a geometry shader feature, and make
/// sure the GPU is either integrated or discrete. If any of these are not detected, we return
/// with an error.
unsafe fn check_physical_device(
    instance: &Instance,
    data: &AppData,
    physical_device: vk::PhysicalDevice,
) -> Result<()> {
    let properties = unsafe { instance.get_physical_device_properties(physical_device) };
    let features = unsafe { instance.get_physical_device_features(physical_device) };

    if properties.device_type != PhysicalDeviceType::DISCRETE_GPU
        && properties.device_type != PhysicalDeviceType::INTEGRATED_GPU
    {
        return Err(anyhow!(SuitabilityError(
            "Only discrete and integrated GPUs are supported."
        )));
    } else if features.geometry_shader != vk::TRUE {
        return Err(anyhow!(SuitabilityError(
            "Missing geometry shader support."
        )));
    } else {
        trace!("================================");
        trace!("|         GPU FOUND!           |");
        trace!("================================");
        trace!("{}", properties.device_name);
    }

    unsafe { QueueFamilyIndices::get(instance, data, physical_device) }?;

    Ok(())
}

/// Select a suitable Vulkan physical device (GPU) based on its properties and features.
fn pick_physical_device(instance: &Instance, data: &mut AppData) -> Result<()> {
    for physical_device in unsafe { instance.enumerate_physical_devices()? } {
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };

        match unsafe { check_physical_device(instance, data, physical_device) } {
            Ok(_) => {
                data.physical_device = physical_device;
                return Ok(());
            }
            Err(_) => {
                warn!("Skipping physical device (`{}`)", properties.device_name);
            }
        }
    }

    Ok(())
}

/// Vulkan debug callback for explicit control over what the validation layers will print to
/// standard output.
extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    type_: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _: *mut c_void,
) -> vk::Bool32 {
    let data = unsafe { *data };
    let message = unsafe { CStr::from_ptr(data.message).to_string_lossy() };

    if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        error!("({:?}) {}", type_, message);
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        warn!("({:?}) {}", type_, message);
    }

    vk::FALSE
}

/// Create the Vulkan instance.
unsafe fn create_instance(window: &Window, entry: &Entry, data: &mut AppData) -> Result<Instance> {
    // Optional information to specify to the Vulkan driver.
    let application_info = vk::ApplicationInfo::builder()
        .application_name(c"Vulkan App".to_bytes_with_nul())
        .application_version(vk::make_version(1, 0, 0))
        .engine_name(c"No Engine".to_bytes_with_nul())
        .engine_version(vk::make_version(1, 0, 0))
        .api_version(vk::make_version(1, 0, 0));

    // Retrieve the Vulkan available instance layers.
    let available_layers = unsafe {
        let available_layers = entry
            .enumerate_instance_layer_properties()?
            .iter()
            .map(|l| l.layer_name)
            .collect::<HashSet<_>>();

        available_layers
    };

    if VALIDATION_ENABLED && !available_layers.contains(&VALIDATION_LAYER) {
        return Err(anyhow!(
            "Vulkan Validation layer \"{}\" requested, but not supported.",
            VALIDATION_LAYER
        ));
    }

    let layers = if VALIDATION_ENABLED {
        vec![VALIDATION_LAYER.as_ptr()]
    } else {
        Vec::new()
    };

    // Retrieve required instance platform extensions.
    let mut extensions = vk_window::get_required_instance_extensions(window)
        .iter()
        .map(|e| e.as_ptr())
        .collect::<Vec<_>>();

    if VALIDATION_ENABLED {
        extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
    }

    for &ext in &extensions {
        let name = unsafe { CStr::from_ptr(ext) }.to_string_lossy();
    }

    // Required by Vulkan SDK on macOS since 1.3.216.
    let flags = if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION {
        trace!("Enabling extensions for macOS portability.");
        extensions.push(
            vk::KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_EXTENSION
                .name
                .as_ptr(),
        );
        extensions.push(vk::KHR_PORTABILITY_ENUMERATION_EXTENSION.name.as_ptr());
        vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
    } else {
        vk::InstanceCreateFlags::empty()
    };

    // Populate the CreateInfo struct with the ApplicationInfo struct, and the enabled layers,
    // extensions, and portability flags.
    let mut create_info = vk::InstanceCreateInfo::builder()
        .application_info(&application_info)
        .enabled_layer_names(&layers)
        .enabled_extension_names(&extensions)
        .flags(flags);

    // Populate DebugInfo struct with all severity flags, message types, and add
    // `debug_callback()` to the struct.
    let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
        .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .user_callback(Some(debug_callback));

    // Add DebugInfo struct to CreateInfo struct as an extension so the debug messenger can validate
    // instance creation and destruction.
    if VALIDATION_ENABLED {
        create_info = create_info.push_next(&mut debug_info)
    }

    // Create the Vulkan instance.
    // let instance = unsafe { entry.create_instance(&create_info, None)? };
    let instance = unsafe {
        match entry.create_instance(&create_info, None) {
            Ok(instance) => {
                trace!("Created Vulkan instance.");
                instance
            }
            Err(e) => {
                error!("Failed to created Vulkan instance: {:?}", e);
                return Err(e.into());
            }
        }
    };

    // Create the Vulkan debug messenger.
    if VALIDATION_ENABLED {
        data.messenger = unsafe { instance.create_debug_utils_messenger_ext(&debug_info, None)? };
        trace!("Created Vulkan debug messenger.");
    }

    Ok(instance)
}