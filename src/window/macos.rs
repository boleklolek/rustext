//! Native macOS Cocoa window backend with zero third-party dependencies.
//! Pure standard-library Rust interfacing with Objective-C runtime and AppKit.

#![cfg(target_os = "macos")]

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;

use super::events::{MouseButton, WindowEvent};
use crate::gfx::Framebuffer;
use crate::ui::terminal::Key;

type Id = *mut c_void;
type Sel = *mut c_void;
type CGFloat = f64;
type NSUInteger = usize;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct CGPoint {
    pub x: CGFloat,
    pub y: CGFloat,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct CGSize {
    pub width: CGFloat,
    pub height: CGFloat,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

#[cfg_attr(target_os = "macos", link(name = "objc"))]
#[cfg_attr(target_os = "macos", link(name = "AppKit", kind = "framework"))]
#[cfg_attr(target_os = "macos", link(name = "CoreGraphics", kind = "framework"))]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> Id;
    fn sel_registerName(name: *const c_char) -> Sel;
    fn objc_msgSend(receiver: Id, op: Sel, ...) -> Id;

    fn CGColorSpaceCreateDeviceRGB() -> *mut c_void;
    fn CGColorSpaceRelease(space: *mut c_void);
    fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        space: *mut c_void,
        bitmap_info: u32,
    ) -> *mut c_void;
    fn CGContextRelease(context: *mut c_void);
    fn CGBitmapContextCreateImage(context: *mut c_void) -> *mut c_void;
    fn CGImageRelease(image: *mut c_void);
}

// Helper to register selectors safely
unsafe fn sel(name: &str) -> Sel {
    let c_name = CString::new(name).unwrap_or_default();
    unsafe { sel_registerName(c_name.as_ptr()) }
}

// Helper to get class
unsafe fn class(name: &str) -> Id {
    let c_name = CString::new(name).unwrap_or_default();
    unsafe { objc_getClass(c_name.as_ptr()) }
}

// Common Cocoa constants
const NS_BACKING_STORE_BUFFERED: NSUInteger = 2;
const NS_WINDOW_STYLE_MASK_TITLED: NSUInteger = 1 << 0;
const NS_WINDOW_STYLE_MASK_CLOSABLE: NSUInteger = 1 << 1;
const NS_WINDOW_STYLE_MASK_MINIATURIZABLE: NSUInteger = 1 << 2;
const NS_WINDOW_STYLE_MASK_RESIZABLE: NSUInteger = 1 << 3;

// NSEventType constants
const NS_EVENT_TYPE_LEFT_MOUSE_DOWN: NSUInteger = 1;
const NS_EVENT_TYPE_LEFT_MOUSE_UP: NSUInteger = 2;
const NS_EVENT_TYPE_RIGHT_MOUSE_DOWN: NSUInteger = 3;
const NS_EVENT_TYPE_RIGHT_MOUSE_UP: NSUInteger = 4;
const NS_EVENT_TYPE_MOUSE_MOVED: NSUInteger = 5;
const NS_EVENT_TYPE_LEFT_MOUSE_DRAGGED: NSUInteger = 6;
const NS_EVENT_TYPE_RIGHT_MOUSE_DRAGGED: NSUInteger = 7;
const NS_EVENT_TYPE_KEY_DOWN: NSUInteger = 10;
const NS_EVENT_TYPE_SCROLL_WHEEL: NSUInteger = 22;

// Modifier flags
const NS_EVENT_MODIFIER_FLAG_SHIFT: NSUInteger = 1 << 17;
const NS_EVENT_MODIFIER_FLAG_CONTROL: NSUInteger = 1 << 18;
const NS_EVENT_MODIFIER_FLAG_OPTION: NSUInteger = 1 << 19;
const NS_EVENT_MODIFIER_FLAG_COMMAND: NSUInteger = 1 << 20;

pub struct CocoaWindow {
    pub window: Id,
    pub view: Id,
    pub width: usize,
    pub height: usize,
    pub scale_factor: f64,
    should_close: bool,
    is_maximized: bool,
    last_mouse_x: usize,
    last_mouse_y: usize,
}

impl CocoaWindow {
    /// Creates and configures a native macOS Cocoa window with NSApplication and NSWindow.
    pub fn new(title: &str, logical_width: usize, logical_height: usize) -> Result<Self, String> {
        let scale_factor = if let Ok(val) = std::env::var("RUSTEXT_SCALE_FACTOR") {
            val.trim().parse::<f64>().unwrap_or(2.0)
        } else {
            2.0 // Default to Retina 2.0x on macOS
        };

        let physical_w = ((logical_width as f64) * scale_factor).round().max(400.0) as usize;
        let physical_h = ((logical_height as f64) * scale_factor).round().max(300.0) as usize;

        unsafe {
            // 1. Initialize NSApplication
            let ns_app_class = class("NSApplication");
            let app: Id = objc_msgSend(ns_app_class, sel("sharedApplication"));
            if app.is_null() {
                return Err("Failed to obtain NSApplication.sharedApplication".to_string());
            }

            // Set activation policy: NSApplicationActivationPolicyRegular (0)
            let _ = objc_msgSend(app, sel("setActivationPolicy:"), 0isize);

            // 2. Create Rect for window
            let frame = CGRect {
                origin: CGPoint { x: 100.0, y: 100.0 },
                size: CGSize {
                    width: logical_width as CGFloat,
                    height: logical_height as CGFloat,
                },
            };

            // 3. Alloc and init NSWindow
            let window_class = class("NSWindow");
            let window_alloc: Id = objc_msgSend(window_class, sel("alloc"));
            if window_alloc.is_null() {
                return Err("Failed to allocate NSWindow".to_string());
            }

            let style_mask: NSUInteger = NS_WINDOW_STYLE_MASK_TITLED
                | NS_WINDOW_STYLE_MASK_CLOSABLE
                | NS_WINDOW_STYLE_MASK_MINIATURIZABLE
                | NS_WINDOW_STYLE_MASK_RESIZABLE;

            type InitWithRectFn = unsafe extern "C" fn(
                Id,
                Sel,
                CGRect,
                NSUInteger,
                NSUInteger,
                c_int,
            ) -> Id;

            let init_fn: InitWithRectFn = std::mem::transmute(objc_msgSend as *const ());
            let window: Id = init_fn(
                window_alloc,
                sel("initWithContentRect:styleMask:backing:defer:"),
                frame,
                style_mask,
                NS_BACKING_STORE_BUFFERED,
                0,
            );

            if window.is_null() {
                return Err("Failed to initialize NSWindow".to_string());
            }

            // Set Title
            let title_c = CString::new(title).unwrap_or_default();
            let ns_string_class = class("NSString");
            let title_ns: Id = objc_msgSend(
                ns_string_class,
                sel("stringWithUTF8String:"),
                title_c.as_ptr(),
            );
            let _ = objc_msgSend(window, sel("setTitle:"), title_ns);

            // Center window and bring to front
            let _ = objc_msgSend(window, sel("center"));
            let _ = objc_msgSend(
                window,
                sel("makeKeyAndOrderFront:"),
                ptr::null_mut::<c_void>(),
            );

            // Activate app
            let _ = objc_msgSend(app, sel("activateIgnoringOtherApps:"), 1 as c_int);

            // Content view
            let content_view: Id = objc_msgSend(window, sel("contentView"));

            Ok(Self {
                window,
                view: content_view,
                width: physical_w,
                height: physical_h,
                scale_factor,
                should_close: false,
                is_maximized: false,
                last_mouse_x: 0,
                last_mouse_y: 0,
            })
        }
    }

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    pub fn set_title(&mut self, title: &str) {
        if self.window.is_null() {
            return;
        }
        unsafe {
            let title_c = CString::new(title).unwrap_or_default();
            let ns_string_class = class("NSString");
            let title_ns: Id = objc_msgSend(
                ns_string_class,
                sel("stringWithUTF8String:"),
                title_c.as_ptr(),
            );
            let _ = objc_msgSend(self.window, sel("setTitle:"), title_ns);
        }
    }

    /// Polls pending Cocoa/AppKit events from NSApplication event queue.
    pub fn poll_event(&mut self) -> WindowEvent {
        if self.window.is_null() {
            return WindowEvent::None;
        }

        unsafe {
            let app_class = class("NSApplication");
            let app: Id = objc_msgSend(app_class, sel("sharedApplication"));
            if app.is_null() {
                return WindowEvent::None;
            }

            // [NSDate distantPast]
            let date_class = class("NSDate");
            let distant_past: Id = objc_msgSend(date_class, sel("distantPast"));

            // [app nextEventMatchingMask:NSEventMaskAny untilDate:distantPast inMode:NSDefaultRunLoopMode dequeue:YES]
            let run_loop_mode: Id = objc_msgSend(class("NSString"), sel("stringWithUTF8String:"), b"kCFRunLoopDefaultMode\0".as_ptr());
            let any_mask: NSUInteger = !0; // NSUIntegerMax

            type NextEventFn = unsafe extern "C" fn(Id, Sel, NSUInteger, Id, Id, c_int) -> Id;
            let next_event_fn: NextEventFn = std::mem::transmute(objc_msgSend as *const ());
            let event: Id = next_event_fn(
                app,
                sel("nextEventMatchingMask:untilDate:inMode:dequeue:"),
                any_mask,
                distant_past,
                run_loop_mode,
                1,
            );

            if event.is_null() {
                return WindowEvent::None;
            }

            type TypeFn = unsafe extern "C" fn(Id, Sel) -> NSUInteger;
            let type_fn: TypeFn = std::mem::transmute(objc_msgSend as *const ());
            let event_type: NSUInteger = type_fn(event, sel("type"));

            // Forward to NSApplication to keep standard menus / controls responsive
            let _ = objc_msgSend(app, sel("sendEvent:"), event);

            // Check if window is closing
            type BoolFn = unsafe extern "C" fn(Id, Sel) -> c_int;
            let bool_fn: BoolFn = std::mem::transmute(objc_msgSend as *const ());
            let is_visible = bool_fn(self.window, sel("isVisible")) != 0;
            if !is_visible && !self.should_close {
                self.should_close = true;
                return WindowEvent::Close;
            }

            // Window frame / size changes
            let content_view: Id = objc_msgSend(self.window, sel("contentView"));
            if !content_view.is_null() {
                type FrameFn = unsafe extern "C" fn(Id, Sel) -> CGRect;
                let frame_fn: FrameFn = std::mem::transmute(objc_msgSend as *const ());
                let view_rect: CGRect = frame_fn(content_view, sel("frame"));
                let cur_w = ((view_rect.size.width * self.scale_factor).round() as usize).max(100);
                let cur_h = ((view_rect.size.height * self.scale_factor).round() as usize).max(100);
                if cur_w != self.width || cur_h != self.height {
                    self.width = cur_w;
                    self.height = cur_h;
                    return WindowEvent::Resize {
                        width: cur_w,
                        height: cur_h,
                    };
                }
            }

            match event_type {
                NS_EVENT_TYPE_LEFT_MOUSE_DOWN | NS_EVENT_TYPE_RIGHT_MOUSE_DOWN => {
                    let loc = self.event_location(event);
                    self.last_mouse_x = loc.0;
                    self.last_mouse_y = loc.1;
                    let btn = if event_type == NS_EVENT_TYPE_LEFT_MOUSE_DOWN {
                        MouseButton::Left
                    } else {
                        MouseButton::Right
                    };
                    WindowEvent::MouseDown {
                        x: loc.0,
                        y: loc.1,
                        button: btn,
                    }
                }
                NS_EVENT_TYPE_LEFT_MOUSE_UP | NS_EVENT_TYPE_RIGHT_MOUSE_UP => {
                    let loc = self.event_location(event);
                    self.last_mouse_x = loc.0;
                    self.last_mouse_y = loc.1;
                    let btn = if event_type == NS_EVENT_TYPE_LEFT_MOUSE_UP {
                        MouseButton::Left
                    } else {
                        MouseButton::Right
                    };
                    WindowEvent::MouseUp {
                        x: loc.0,
                        y: loc.1,
                        button: btn,
                    }
                }
                NS_EVENT_TYPE_MOUSE_MOVED | NS_EVENT_TYPE_LEFT_MOUSE_DRAGGED | NS_EVENT_TYPE_RIGHT_MOUSE_DRAGGED => {
                    let loc = self.event_location(event);
                    self.last_mouse_x = loc.0;
                    self.last_mouse_y = loc.1;
                    WindowEvent::MouseMove {
                        x: loc.0,
                        y: loc.1,
                    }
                }
                NS_EVENT_TYPE_SCROLL_WHEEL => {
                    type DeltaYFn = unsafe extern "C" fn(Id, Sel) -> CGFloat;
                    let delta_y_fn: DeltaYFn = std::mem::transmute(objc_msgSend as *const ());
                    let delta_y = delta_y_fn(event, sel("scrollingDeltaY"));
                    if delta_y > 0.0 {
                        WindowEvent::ScrollUp {
                            x: self.last_mouse_x,
                            y: self.last_mouse_y,
                        }
                    } else if delta_y < 0.0 {
                        WindowEvent::ScrollDown {
                            x: self.last_mouse_x,
                            y: self.last_mouse_y,
                        }
                    } else {
                        WindowEvent::None
                    }
                }
                NS_EVENT_TYPE_KEY_DOWN => {
                    self.parse_key_event(event)
                }
                _ => WindowEvent::None,
            }
        }
    }

    /// Converts Cocoa window coordinates (bottom-left origin) to Framebuffer coordinates (top-left origin).
    unsafe fn event_location(&self, event: Id) -> (usize, usize) {
        unsafe {
            type LocFn = unsafe extern "C" fn(Id, Sel) -> CGPoint;
            let loc_fn: LocFn = std::mem::transmute(objc_msgSend as *const ());
            let loc: CGPoint = loc_fn(event, sel("locationInWindow"));

            let content_view: Id = objc_msgSend(self.window, sel("contentView"));
            let height_logical = if !content_view.is_null() {
                type FrameFn = unsafe extern "C" fn(Id, Sel) -> CGRect;
                let frame_fn: FrameFn = std::mem::transmute(objc_msgSend as *const ());
                let r = frame_fn(content_view, sel("frame"));
                r.size.height
            } else {
                (self.height as f64) / self.scale_factor
            };

            let x_phys = (loc.x * self.scale_factor).round() as usize;
            let y_flipped = (height_logical - loc.y).max(0.0);
            let y_phys = (y_flipped * self.scale_factor).round() as usize;

            (x_phys.min(self.width.saturating_sub(1)), y_phys.min(self.height.saturating_sub(1)))
        }
    }

    /// Parses key code and modifier flags into WindowEvent (Key / Char).
    unsafe fn parse_key_event(&self, event: Id) -> WindowEvent {
        unsafe {
            type KeyCodeFn = unsafe extern "C" fn(Id, Sel) -> u16;
            let key_code_fn: KeyCodeFn = std::mem::transmute(objc_msgSend as *const ());
            let key_code: u16 = key_code_fn(event, sel("keyCode"));

            type FlagsFn = unsafe extern "C" fn(Id, Sel) -> NSUInteger;
            let flags_fn: FlagsFn = std::mem::transmute(objc_msgSend as *const ());
            let flags: NSUInteger = flags_fn(event, sel("modifierFlags"));

            let is_shift = (flags & NS_EVENT_MODIFIER_FLAG_SHIFT) != 0;
            let is_ctrl = (flags & NS_EVENT_MODIFIER_FLAG_CONTROL) != 0 || (flags & NS_EVENT_MODIFIER_FLAG_COMMAND) != 0;
            let is_alt = (flags & NS_EVENT_MODIFIER_FLAG_OPTION) != 0;

            // Common special keys by macOS virtual keycodes
            match key_code {
                36 => return WindowEvent::Key(Key::Enter),
                51 => return WindowEvent::Key(Key::Backspace),
                48 => {
                    if is_shift {
                        return WindowEvent::Key(Key::BackTab);
                    } else {
                        return WindowEvent::Key(Key::Tab);
                    }
                }
                53 => return WindowEvent::Key(Key::Esc),
                126 => {
                    if is_alt && is_shift {
                        return WindowEvent::Key(Key::ShiftAltUp);
                    } else if is_alt {
                        return WindowEvent::Key(Key::AltUp);
                    } else {
                        return WindowEvent::Key(Key::Up);
                    }
                }
                125 => {
                    if is_alt && is_shift {
                        return WindowEvent::Key(Key::ShiftAltDown);
                    } else if is_alt {
                        return WindowEvent::Key(Key::AltDown);
                    } else {
                        return WindowEvent::Key(Key::Down);
                    }
                }
                123 => {
                    if is_alt {
                        return WindowEvent::Key(Key::AltLeft);
                    } else {
                        return WindowEvent::Key(Key::Left);
                    }
                }
                124 => {
                    if is_alt {
                        return WindowEvent::Key(Key::AltRight);
                    } else {
                        return WindowEvent::Key(Key::Right);
                    }
                }
                115 => return WindowEvent::Key(Key::Home),
                119 => return WindowEvent::Key(Key::End),
                116 => return WindowEvent::Key(Key::PageUp),
                121 => return WindowEvent::Key(Key::PageDown),
                117 => return WindowEvent::Key(Key::Delete),
                _ => {}
            }

            // Extract UTF-8 characters from event
            let characters: Id = objc_msgSend(event, sel("characters"));
            if !characters.is_null() {
                let utf8_ptr: *const c_char = objc_msgSend(characters, sel("UTF8String")) as *const c_char;
                if !utf8_ptr.is_null() {
                    let cstr = CStr::from_ptr(utf8_ptr);
                    if let Ok(text) = cstr.to_str() {
                        if let Some(ch) = text.chars().next() {
                            if is_ctrl {
                                let c_lower = ch.to_ascii_lowercase();
                                if is_shift {
                                    return WindowEvent::Key(Key::CtrlShift(c_lower));
                                } else {
                                    return WindowEvent::Key(Key::Ctrl(c_lower));
                                }
                            }
                            if is_alt {
                                return WindowEvent::Key(Key::Alt(ch));
                            }
                            if !ch.is_control() {
                                return WindowEvent::Char(ch);
                            }
                        }
                    }
                }
            }

            WindowEvent::None
        }
    }

    /// Presents framebuffer onto Cocoa NSWindow using CoreGraphics bitmap context and CGImage.
    pub fn present(&mut self, fb: &Framebuffer) {
        if self.window.is_null() || fb.width == 0 || fb.height == 0 {
            return;
        }

        unsafe {
            let content_view: Id = objc_msgSend(self.window, sel("contentView"));
            if content_view.is_null() {
                return;
            }

            // 1. Create DeviceRGB ColorSpace
            let color_space = CGColorSpaceCreateDeviceRGB();
            if color_space.is_null() {
                return;
            }

            // 2. Create Bitmap Context (kCGBitmapByteOrder32Little | kCGImageAlphaNoneSkipFirst = 0x2002)
            // Framebuffer format in Rustext is BGRA/RGBA compatible
            let bitmap_info: u32 = 8194; // 8192 (kCGBitmapByteOrder32Little) | 2 (kCGImageAlphaNoneSkipFirst)
            let bytes_per_row = fb.width * 4;

            let ctx = CGBitmapContextCreate(
                fb.pixels.as_ptr() as *mut c_void,
                fb.width,
                fb.height,
                8,
                bytes_per_row,
                color_space,
                bitmap_info,
            );

            CGColorSpaceRelease(color_space);

            if ctx.is_null() {
                return;
            }

            // 3. Create CGImage from bitmap context
            let cg_image = CGBitmapContextCreateImage(ctx);
            CGContextRelease(ctx);

            if cg_image.is_null() {
                return;
            }

            // 4. Update NSView layer contents with CGImage
            let _ = objc_msgSend(content_view, sel("setWantsLayer:"), 1 as c_int);
            let layer: Id = objc_msgSend(content_view, sel("layer"));
            if !layer.is_null() {
                let _ = objc_msgSend(layer, sel("setContents:"), cg_image);
            }

            CGImageRelease(cg_image);
        }
    }

    pub fn should_close(&self) -> bool {
        self.should_close
    }

    pub fn cancel_close(&mut self) {
        self.should_close = false;
    }

    pub fn start_interactive_move(&mut self) {
        if self.window.is_null() {
            return;
        }
        unsafe {
            let _ = objc_msgSend(self.window, sel("performWindowDragWithEvent:"), ptr::null_mut::<c_void>());
        }
    }

    pub fn minimize(&mut self) {
        if self.window.is_null() {
            return;
        }
        unsafe {
            let _ = objc_msgSend(self.window, sel("miniaturize:"), ptr::null_mut::<c_void>());
        }
    }

    pub fn toggle_maximize(&mut self) {
        if self.window.is_null() {
            return;
        }
        unsafe {
            let _ = objc_msgSend(self.window, sel("zoom:"), ptr::null_mut::<c_void>());
            self.is_maximized = !self.is_maximized;
        }
    }

    pub fn is_maximized(&self) -> bool {
        if self.window.is_null() {
            return false;
        }
        unsafe {
            type ZoomedFn = unsafe extern "C" fn(Id, Sel) -> c_int;
            let zoomed_fn: ZoomedFn = std::mem::transmute(objc_msgSend as *const ());
            zoomed_fn(self.window, sel("isZoomed")) != 0
        }
    }
}
