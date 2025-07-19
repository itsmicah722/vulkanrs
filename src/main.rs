#![deny(unsafe_op_in_unsafe_fn)]
#![allow(
    dead_code,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

use std::{
    collections::HashSet,
    ffi::{c_void, CStr},
};

use anyhow::{anyhow, Context, Result};
use log::*;
use thiserror::Error;
use vulkanalia::{
    loader::{LibloadingLoader, LIBRARY}, vk, vk::{EntryV1_0, ExtDebugUtilsExtension, HasBuilder, InstanceV1_0, PhysicalDeviceType},
    window as vk_window,
    Entry,
    Instance,
    Version,
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
}

/// Main `App` which implements Vulkan boilerplate functionality.
#[derive(Clone, Debug)]
struct App {
    entry: Entry,
    instance: Instance,
    data: AppData,
}

impl App {
    /// Initializes Vulkan resources tied to App.
    unsafe fn create(window: &Window) -> Result<Self> {
        let (entry, instance, data) = unsafe {
            let loader = LibloadingLoader::new(LIBRARY)?;
            let entry = Entry::new(loader).map_err(|b| anyhow!("{}", b))?;
            let mut data = AppData::default();
            let instance = create_instance(window, &entry, &mut data)?;
            pick_physical_device(&instance, &mut data)?;

            (entry, instance, data)
        };

        Ok(Self {
            entry,
            instance,
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

#[derive(Debug, Error)]
#[error("Missing {0}.")]
pub struct SuitabilityError(pub &'static str);

#[derive(Copy, Clone, Debug)]
struct QueueFamilyIndices {
    graphics: u32,
}

impl QueueFamilyIndices {
    unsafe fn get(
        instance: &Instance,
        data: &AppData,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self> {
        let properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let graphics = properties
            .iter()
            .position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .map(|i| i as u32);

        match graphics {
            Some(graphics) => {
                trace!("Graphics queue family found and supported.");
                Ok(Self { graphics })
            }
            None => Err(anyhow!(SuitabilityError(
                "Missing required graphics queue families."
            ))),
        }
    }
}

unsafe fn check_physical_device(
    instance: &Instance,
    data: &AppData,
    physical_device: vk::PhysicalDevice,
) -> Result<()> {
    let properties = unsafe { instance.get_physical_device_properties(physical_device) };
    let features = unsafe { instance.get_physical_device_features(physical_device) };

<<<<<<< HEAD
    if properties.device_type != PhysicalDeviceType::DISCRETE_GPU
        && properties.device_type != PhysicalDeviceType::INTEGRATED_GPU
    {
        return Err(anyhow!(SuitabilityError(
            "Only discrete and integrated GPUs are supported."
=======
    if properties.device_type != vk::PhysicalDeviceType::DISCRETE_GPU {
        return Err(anyhow!(SuitabilityError(
            "Only discrete GPUs are supported."
>>>>>>> 42e69707514848f4c4e606fa5fccbe46c4bd7614
        )));
    } else if features.geometry_shader != vk::TRUE {
        return Err(anyhow!(SuitabilityError(
            "Missing geometry shader support."
        )));
    } else {
        trace!("========================");
        trace!("|      GPU FOUND!      |");
        trace!("========================");

        trace!("NAME: {}", properties.device_name);
        trace!("ID: {}", properties.device_id);
        trace!("VULKAN API VERSION: {}", properties.api_version);
        trace!("VENDOR ID: {}", properties.vendor_id);
    }

    unsafe { QueueFamilyIndices::get(instance, data, physical_device) }?;

    Ok(())
}

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
    } else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        // debug!("({:?}) {}", type_, message);
    } else {
        // trace!("({:?}) {}", type_, message);
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