//! # Vulkan-RS Application
//!
//! This binary uses [`winit`] for cross-platform window management, and [`vulkanalia`] for FFI
//! to the Vulkan API. As of this version, the goal is to create a simple abstraction layer
//! around the Vulkan boilerplate to render a triangle.
//!
//! This application follows this [tutorial](https://kylemayes.github.io/vulkanalia/) for getting
//! the Vulkan boilerplate working in Rust.

mod vulkan;
mod window;

use window::Window;
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    let mut window = Window::default();
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut window).unwrap();
}
