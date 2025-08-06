//! # Window Module
//!
//! The `window` module uses [`winit`] to create cross-platform windows and poll events from the
//! user and OS. This module also uses [`raw_window_handle`] to retrieve the window and display
//! handles safely for the vulkan API to use.

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};
use thiserror::Error;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window as WinitWindow, WindowId},
};

/// Custom error types for winit and raw-window-handle.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum WindowError {
    /// Caller attempted to reference the winit window before it was created.
    #[error("Window has not been created yet.")]
    NotInitialized,

    /// raw-window-handle failed to retrieve the window or display handle.
    #[error(transparent)]
    BadHandle(#[from] HandleError),
}

#[derive(Default)]
pub struct Window {
    /// The winit window object
    inner: Option<WinitWindow>,
}

impl Window {
    /// Gets a reference to the winit window object.
    ///
    /// # Errors
    ///
    /// - [`WindowError::NotInitialized`]
    pub fn window(&self) -> Result<&WinitWindow, WindowError> {
        self.inner.as_ref().ok_or(WindowError::NotInitialized)
    }

    /// Gets a mutable reference to the winit window object.
    ///
    /// # Errors
    ///
    /// - [`WindowError::NotInitialized`]
    pub fn window_mut(&mut self) -> Result<&mut WinitWindow, WindowError> {
        self.inner.as_mut().ok_or(WindowError::NotInitialized)
    }

    /// Gets the native [`WindowHandle`] from the winit window. The lifetime of WindowHandle is
    /// guaranteed to be valid as long as `&self` is valid.
    ///
    /// # Errors
    ///
    /// - [`WindowError::NotInitialized`]
    /// - [`WindowError::BadHandle`]
    pub fn window_handle(&self) -> Result<WindowHandle<'_>, WindowError> {
        Ok(self.window()?.window_handle()?)
    }

    /// Gets the native [`DisplayHandle`] from the winit window. The lifetime of DisplayHandle is
    /// guaranteed to be valid as long as `&self` is valid.
    ///
    /// # Errors
    ///
    /// - [`WindowError::NotInitialized`]
    /// - [`WindowError::BadHandle`]
    pub fn display_handle(&self) -> Result<DisplayHandle<'_>, WindowError> {
        Ok(self.window()?.display_handle()?)
    }
}

impl ApplicationHandler for Window {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(WinitWindow::default_attributes())
            .unwrap();

        self.inner = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            _ => (),
        }
    }
}