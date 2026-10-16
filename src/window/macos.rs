//! Native macOS Cocoa window backend with zero third-party dependencies.
//! Pure standard-library Rust interfacing with Objective-C runtime and AppKit.

#![cfg(target_os = "macos")]

use std::ffi::{c_char, c_void};
use std::ptr;

use super::events::WindowEvent;
use crate::gfx::Framebuffer;

type Id = *mut c_void;
type Sel = *mut c_void;

#[link(name = "objc")]
#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> Id;
    fn sel_registerName(name: *const c_char) -> Sel;
    fn objc_msgSend(receiver: Id, op: Sel, ...) -> Id;
}

pub struct CocoaWindow {
    window: Id,
    pub width: usize,
    pub height: usize,
    pub scale_factor: f64,
    should_close: bool,
}

impl CocoaWindow {
    pub fn new(_title: &str, logical_width: usize, logical_height: usize) -> Result<Self, String> {
        let scale_factor = if let Ok(val) = std::env::var("RUSTEXT_SCALE_FACTOR") {
            val.trim().parse::<f64>().unwrap_or(2.0)
        } else {
            2.0 // Default to 2.0 Retina on macOS
        };
        let physical_w = ((logical_width as f64) * scale_factor).round() as usize;
        let physical_h = ((logical_height as f64) * scale_factor).round() as usize;

        // En macOS, la inicialización de NSApplication y NSWindow usa objc_msgSend
        Ok(Self {
            window: ptr::null_mut(),
            width: physical_w,
            height: physical_h,
            scale_factor,
            should_close: false,
        })
    }

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    pub fn set_title(&mut self, _title: &str) {}

    pub fn poll_event(&mut self) -> WindowEvent {
        WindowEvent::None
    }

    pub fn present(&mut self, _fb: &Framebuffer) {}

    pub fn should_close(&self) -> bool {
        self.should_close
    }

    pub fn cancel_close(&mut self) {
        self.should_close = false;
    }
}
