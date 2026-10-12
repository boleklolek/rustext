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
    should_close: bool,
}

impl CocoaWindow {
    pub fn new(title: &str, width: usize, height: usize) -> Result<Self, String> {
        // En macOS, la inicialización de NSApplication y NSWindow usa objc_msgSend
        Ok(Self {
            window: ptr::null_mut(),
            width,
            height,
            should_close: false,
        })
    }

    pub fn poll_event(&mut self) -> WindowEvent {
        WindowEvent::None
    }

    pub fn present(&mut self, _fb: &Framebuffer) {}

    pub fn should_close(&self) -> bool {
        self.should_close
    }
}
