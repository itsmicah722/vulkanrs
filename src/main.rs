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

// Here we use `Result<T, anyhow::Error>` aka - `Result<()>` for easier error handling and
// propagation, since anyhow wraps those types and can store any error that implements
// `std::error`. Propagation can be done via "?", allowing us to avoid more matches or unwraps.
use anyhow::{anyhow, Context, Result};
use log::*;
use vulkanalia::{
    loader::{LibloadingLoader, LIBRARY}, vk, vk::{EntryV1_0, ExtDebugUtilsExtension, HasBuilder, InstanceV1_0},
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
            let instance = match create_instance(window, &entry, &mut data) {
                Ok(instance) => {
                    trace!("Created Vulkan instance.");
                    instance
                }

                Err(e) => {
                    error!("Failed to create Vulkan instance: {:?}", e);
                    return Err(e);
                }
            };

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

/// All Vulkan handles and associated properties used by `App`.
#[derive(Clone, Debug, Default)]
struct AppData {
    messenger: vk::DebugUtilsMessengerEXT,
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
    let instance = unsafe { entry.create_instance(&create_info, None)? };

    // Create the Vulkan debug messenger.
    if VALIDATION_ENABLED {
        data.messenger = unsafe { instance.create_debug_utils_messenger_ext(&debug_info, None)? };
        trace!("Created Vulkan debug messenger.");
    }

    Ok(instance)
}

/// The `Debug Callback` explicitly controls what messages the Vulkan validation layers will print
/// to standard output.
///
/// # Parameters
/// - `severity` contains bitmask flags for how severe a message was that triggered the
/// validation layers (e.g. verbose, info, warning, error).
/// - `type_` is a bitmask specifying which type of event triggered the callback (e.g. general,
/// validation, performance)
/// - `data` points to detailed information about the debug message being emitted (i.e. message,
/// message ID, objects involved, etc.)
/// - `_` contains the raw pointer provided when setting up the debug messenger, which is not used
/// here.
///
/// # Return
/// The callback returns a `vk::VkBook` *Vulkan* Boolean that indicates if the Vulkan call that
/// triggered the validation layer should be aborted. If the callback returns true, then the
/// call is aborted with an error code. This is only for testing the actual validation layers
/// themselves, so we will **always** return `TRUE`.
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