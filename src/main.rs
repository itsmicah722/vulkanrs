#![allow(
    dead_code,
    unused_variables,
    clippy::too_many_arguments,
    clippy::unnecessary_wraps
)]

// Here we use `Result<T, anyhow::Error>` aka - `Result<()>` for easier error handling and
// propagation, since `anyhow` wraps those types and can store any error. Propagation can be done
// via "?", allowing us to avoid more matches or unwraps.
use anyhow::Result;
use log::trace;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

fn main() -> Result<()> {
    pretty_env_logger::init();

    // Create the window.
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("Vulkan App")
        .with_inner_size(LogicalSize::new(600, 400))
        .with_active(false)
        .build(&event_loop)?;

    // Create the Vulkan app
    let mut app = unsafe { App::create(&window)? };
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
                    trace!("Vulkan app requested a redraw.")
                }

                // Destroy the App.
                WindowEvent::CloseRequested => {
                    window_target.exit();
                    unsafe {
                        app.destroy();
                    }
                    trace!("Vulkan App was destroyed.")
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
struct App {}

impl App {
    /// Creates the Vulkan App.
    unsafe fn create(window: &Window) -> Result<Self> {
        Ok(Self {})
    }

    /// Renders a frame for our Vulkan App.
    unsafe fn render(&mut self, window: &Window) -> Result<()> {
        Ok(())
    }

    /// Destroys the Vulkan app.
    unsafe fn destroy(&mut self) {}
}

/// All Vulkan handles and associated properties used by `App`.
#[derive(Clone, Debug, Default)]
struct AppData {}