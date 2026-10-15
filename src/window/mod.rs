//! Cross-platform native window abstraction for Linux, Windows, and macOS.
//! Zero third-party dependencies, purely standard library Rust.

pub mod events;

#[cfg(target_os = "linux")]
pub mod x11;

#[cfg(target_os = "windows")]
pub mod win32;

#[cfg(target_os = "macos")]
pub mod macos;

pub use events::{MouseButton, WindowEvent};

#[cfg(target_os = "linux")]
pub use x11::X11Window as NativeWindow;

#[cfg(target_os = "windows")]
pub use win32::Win32Window as NativeWindow;

#[cfg(target_os = "macos")]
pub use macos::CocoaWindow as NativeWindow;

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub struct UnsupportedWindow;

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
impl UnsupportedWindow {
    pub fn new(_title: &str, _w: usize, _h: usize) -> Result<Self, String> {
        Err("Native GUI windows not supported on this platform".to_string())
    }
    pub fn poll_event(&mut self) -> WindowEvent {
        WindowEvent::None
    }
    pub fn present(&mut self, _fb: &crate::gfx::Framebuffer) {}
    pub fn set_title(&mut self, _title: &str) {}
    pub fn should_close(&self) -> bool {
        true
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub use UnsupportedWindow as NativeWindow;
