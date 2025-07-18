#![deny(unsafe_op_in_unsafe_fn)]
#![allow(
    dead_code,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

use std::ffi::CStr;

// Here we use `Result<T, anyhow::Error>` aka - `Result<()>` for easier error handling and
// propagation, since anyhow wraps those types and can store any error that implements
// `std::error`. Propagation can be done via "?", allowing us to avoid more matches or unwraps.
use anyhow::{anyhow, Context, Result};
use log::*;
use vulkanalia::{
    loader::{LibloadingLoader, LIBRARY}, vk, vk::{HasBuilder, InstanceV1_0},
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
}

impl App {
    /// Creates the Vulkan App.
    unsafe fn create(window: &Window) -> Result<Self> {
        let (entry, instance) = unsafe {
            let loader = LibloadingLoader::new(LIBRARY)?;
            let entry = Entry::new(loader).map_err(|b| anyhow!("{}", b))?;
            let instance = match create_instance(window, &entry) {
                Ok(instance) => {
                    trace!("Created Vulkan instance.");
                    instance
                }

                Err(e) => {
                    error!("Failed to create Vulkan instance: {:?}", e);
                    return Err(e);
                }
            };

            (entry, instance)
        };

        Ok(Self { entry, instance })
    }

    /// Renders a frame for our Vulkan App.
    unsafe fn render(&mut self, window: &Window) -> Result<()> {
        Ok(())
    }

    /// Destroys the Vulkan app.
    unsafe fn destroy(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
            trace!("Destroyed Vulkan instance.")
        }
    }
}

/// All Vulkan handles and associated properties used by `App`.
#[derive(Clone, Debug, Default)]
struct AppData {}

unsafe fn create_instance(window: &Window, entry: &Entry) -> Result<Instance> {
    let application_info = vk::ApplicationInfo::builder()
        .application_name(c"Vulkan App".to_bytes_with_nul())
        .application_version(vk::make_version(1, 0, 0))
        .engine_name(c"No Engine".to_bytes_with_nul())
        .engine_version(vk::make_version(1, 0, 0))
        .api_version(vk::make_version(1, 0, 0));

    // `ext` is &&ExtensionNames, which is a reference to a list of references to static C-String
    // extension names.
    let mut extensions = Vec::new();
    for ext in vk_window::get_required_instance_extensions(window) {
        let ptr = ext.as_ptr();
        let name = unsafe { CStr::from_ptr(ptr) }.to_string_lossy();
        debug!("Adding extension: {}", name);
        extensions.push(ptr);
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

    let create_info = vk::InstanceCreateInfo::builder()
        .application_info(&application_info)
        .enabled_extension_names(&extensions)
        .flags(flags);

    unsafe { Ok(entry.create_instance(&create_info, None)?) }
}