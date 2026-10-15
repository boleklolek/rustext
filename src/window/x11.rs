//! Native Linux X11 window backend with zero third-party dependencies.
//! Pure standard-library Rust interfacing with system libX11.so.

#![cfg(target_os = "linux")]

use std::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void, CString};
use std::ptr;

use super::events::{MouseButton, WindowEvent};
use crate::gfx::Framebuffer;
use crate::ui::terminal::Key;

// --- X11 C Types ---
type Display = c_void;
type XWindow = c_ulong;
type Drawable = c_ulong;
type Visual = c_void;
type GC = *mut c_void;
type Atom = c_ulong;
type KeySym = c_ulong;

#[repr(C)]
#[derive(Clone, Copy)]
struct XConfigureEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    event: XWindow,
    window: XWindow,
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    border_width: c_int,
    above: XWindow,
    override_redirect: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XClientMessageEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    window: XWindow,
    message_type: Atom,
    format: c_int,
    data: [c_long; 5],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XKeyEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    window: XWindow,
    root: XWindow,
    subwindow: XWindow,
    time: c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: c_uint,
    keycode: c_uint,
    same_screen: c_int,
}

#[repr(C)]
struct XImage {
    width: c_int,
    height: c_int,
    xoffset: c_int,
    format: c_int,
    data: *mut c_char,
    byte_order: c_int,
    bitmap_unit: c_int,
    bitmap_bit_order: c_int,
    bitmap_pad: c_int,
    depth: c_int,
    bytes_per_line: c_int,
    bits_per_pixel: c_int,
    red_mask: c_ulong,
    green_mask: c_ulong,
    blue_mask: c_ulong,
    obdata: *mut c_void,
    // function pointers follow...
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XAnyEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    window: XWindow,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XButtonEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    window: XWindow,
    root: XWindow,
    subwindow: XWindow,
    time: c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: c_uint,
    button: c_uint,
    same_screen: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XMotionEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: c_int,
    display: *mut Display,
    window: XWindow,
    root: XWindow,
    subwindow: XWindow,
    time: c_ulong,
    x: c_int,
    y: c_int,
    x_root: c_int,
    y_root: c_int,
    state: c_uint,
    is_hint: c_char,
    same_screen: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
union XEvent {
    type_: c_int,
    any: XAnyEvent,
    key: XKeyEvent,
    button: XButtonEvent,
    motion: XMotionEvent,
    configure: XConfigureEvent,
    client: XClientMessageEvent,
    pad: [c_long; 24],
}

// --- X11 Constants ---
const KEY_PRESS: c_int = 2;
const BUTTON_PRESS: c_int = 4;
const BUTTON_RELEASE: c_int = 5;
const MOTION_NOTIFY: c_int = 6;
const DESTROY_NOTIFY: c_int = 17;
const CONFIGURE_NOTIFY: c_int = 22;
const CLIENT_MESSAGE: c_int = 33;

const KEY_PRESS_MASK: c_long = 1 << 0;
const BUTTON_PRESS_MASK: c_long = 1 << 2;
const BUTTON_RELEASE_MASK: c_long = 1 << 3;
const POINTER_MOTION_MASK: c_long = 1 << 6;
const EXPOSURE_MASK: c_long = 1 << 15;
const STRUCTURE_NOTIFY_MASK: c_long = 1 << 17;

const Z_PIXMAP: c_int = 2;

// --- Native X11 C Functions ---
#[link(name = "X11")]
unsafe extern "C" {
    fn XOpenDisplay(display_name: *const c_char) -> *mut Display;
    fn XCloseDisplay(display: *mut Display) -> c_int;
    fn XDefaultRootWindow(display: *mut Display) -> XWindow;
    fn XDefaultScreen(display: *mut Display) -> c_int;
    fn XDefaultVisual(display: *mut Display, screen_number: c_int) -> *mut Visual;
    fn XDefaultDepth(display: *mut Display, screen_number: c_int) -> c_int;
    fn XCreateSimpleWindow(
        display: *mut Display,
        parent: XWindow,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
        border_width: c_uint,
        border: c_ulong,
        background: c_ulong,
    ) -> XWindow;
    fn XSelectInput(display: *mut Display, w: XWindow, event_mask: c_long) -> c_int;
    fn XMapWindow(display: *mut Display, w: XWindow) -> c_int;
    fn XStoreName(display: *mut Display, w: XWindow, window_name: *const c_char) -> c_int;
    fn XCreateGC(display: *mut Display, d: Drawable, valuemask: c_ulong, values: *mut c_void) -> GC;
    fn XFreeGC(display: *mut Display, gc: GC) -> c_int;
    fn XFlush(display: *mut Display) -> c_int;
    fn XPending(display: *mut Display) -> c_int;
    fn XNextEvent(display: *mut Display, event_return: *mut XEvent) -> c_int;
    fn XInternAtom(display: *mut Display, atom_name: *const c_char, only_if_exists: c_int) -> Atom;
    fn XSetWMProtocols(display: *mut Display, w: XWindow, protocols: *mut Atom, count: c_int) -> c_int;
    fn XCreateImage(
        display: *mut Display,
        visual: *mut Visual,
        depth: c_uint,
        format: c_int,
        offset: c_int,
        data: *mut c_char,
        width: c_uint,
        height: c_uint,
        bitmap_pad: c_int,
        bytes_per_line: c_int,
    ) -> *mut XImage;
    fn XPutImage(
        display: *mut Display,
        d: Drawable,
        gc: GC,
        image: *mut XImage,
        src_x: c_int,
        src_y: c_int,
        dest_x: c_int,
        dest_y: c_int,
        width: c_uint,
        height: c_uint,
    ) -> c_int;
    fn XDestroyWindow(display: *mut Display, w: XWindow) -> c_int;
    fn XDestroyImage(ximage: *mut XImage) -> c_int;
    fn XLookupString(
        event_struct: *mut XKeyEvent,
        buffer_return: *mut c_char,
        bytes_buffer: c_int,
        keysym_return: *mut KeySym,
        status_in_out: *mut c_void,
    ) -> c_int;
}

pub struct X11Window {
    display: *mut Display,
    window: XWindow,
    gc: GC,
    visual: *mut Visual,
    depth: c_int,
    wm_delete_window: Atom,
    pub width: usize,
    pub height: usize,
    should_close: bool,
}

impl X11Window {
    /// Opens connection to X display and creates a native window.
    pub fn new(title: &str, width: usize, height: usize) -> Result<Self, String> {
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err("Failed to open X11 Display. Is DISPLAY set?".to_string());
            }

            let screen = XDefaultScreen(display);
            let root = XDefaultRootWindow(display);
            let visual = XDefaultVisual(display, screen);
            let depth = XDefaultDepth(display, screen);

            let window = XCreateSimpleWindow(
                display,
                root,
                100,
                100,
                width as c_uint,
                height as c_uint,
                1,
                0x000000,
                0x1E1E1E,
            );

            // Set window title
            let c_title = CString::new(title).map_err(|e| e.to_string())?;
            XStoreName(display, window, c_title.as_ptr());

            // Select events: KeyPress, Window resizing, Close event, Mouse Clicks & Motion
            let event_mask = KEY_PRESS_MASK
                | STRUCTURE_NOTIFY_MASK
                | EXPOSURE_MASK
                | BUTTON_PRESS_MASK
                | BUTTON_RELEASE_MASK
                | POINTER_MOTION_MASK;
            XSelectInput(display, window, event_mask);

            // Hook close window button [X]
            let wm_protocols = CString::new("WM_PROTOCOLS").unwrap();
            let wm_delete = CString::new("WM_DELETE_WINDOW").unwrap();
            let _atom_protocols = XInternAtom(display, wm_protocols.as_ptr(), 0);
            let atom_delete = XInternAtom(display, wm_delete.as_ptr(), 0);
            let mut protocols = [atom_delete];
            XSetWMProtocols(display, window, protocols.as_mut_ptr(), 1);

            let gc = XCreateGC(display, window, 0, ptr::null_mut());

            XMapWindow(display, window);
            XFlush(display);

            Ok(Self {
                display,
                window,
                gc,
                visual,
                depth,
                wm_delete_window: atom_delete,
                width,
                height,
                should_close: false,
            })
        }
    }

    /// Dynamically sets the native window title.
    pub fn set_title(&mut self, title: &str) {
        unsafe {
            if let Ok(c_title) = CString::new(title) {
                XStoreName(self.display, self.window, c_title.as_ptr());
                XFlush(self.display);
            }
        }
    }

    /// Polls a single pending window event (non-blocking).
    pub fn poll_event(&mut self) -> WindowEvent {
        unsafe {
            if XPending(self.display) == 0 {
                return WindowEvent::None;
            }

            let mut event = std::mem::zeroed::<XEvent>();
            XNextEvent(self.display, &mut event);

            match event.type_ {
                KEY_PRESS => {
                    let mut key_event = event.key;
                    let mut buf = [0u8; 32];
                    let mut keysym: KeySym = 0;

                    let len = XLookupString(
                        &mut key_event,
                        buf.as_mut_ptr() as *mut c_char,
                        buf.len() as c_int,
                        &mut keysym,
                        ptr::null_mut(),
                    );

                    let is_ctrl = (key_event.state & 4) != 0;
                    let is_alt = (key_event.state & 8) != 0;

                    // Special keys
                    match keysym {
                        0xFF08 => return WindowEvent::Key(Key::Backspace),
                        0xFF09 => return WindowEvent::Key(Key::Tab),
                        0xFE20 => return WindowEvent::Key(Key::BackTab),
                        0xFF0D => return WindowEvent::Key(Key::Enter),
                        0xFF1B => return WindowEvent::Key(Key::Esc),
                        0xFF51 => return WindowEvent::Key(Key::Left),
                        0xFF52 => return WindowEvent::Key(Key::Up),
                        0xFF53 => return WindowEvent::Key(Key::Right),
                        0xFF54 => return WindowEvent::Key(Key::Down),
                        0xFF50 => return WindowEvent::Key(Key::Home),
                        0xFF57 => return WindowEvent::Key(Key::End),
                        0xFF55 => return WindowEvent::Key(Key::PageUp),
                        0xFF56 => return WindowEvent::Key(Key::PageDown),
                        0xFFFF => return WindowEvent::Key(Key::Delete),
                        0xFFBE..=0xFFC9 => {
                            return WindowEvent::Key(Key::F((keysym - 0xFFBE + 1) as u8))
                        }
                        _ => {}
                    }

                    if is_ctrl {
                        let ctrl_char = if keysym >= b'a' as KeySym && keysym <= b'z' as KeySym {
                            Some((keysym as u8) as char)
                        } else if keysym >= b'A' as KeySym && keysym <= b'Z' as KeySym {
                            Some(((keysym as u8) + 32) as char)
                        } else if len > 0 && buf[0] >= 1 && buf[0] <= 26 {
                            Some((b'a' + buf[0] - 1) as char)
                        } else if len > 0 && buf[0] >= b' ' && buf[0] <= b'~' {
                            Some((buf[0] as char).to_ascii_lowercase())
                        } else {
                            None
                        };

                        if let Some(ch) = ctrl_char {
                            return WindowEvent::Key(Key::Ctrl(ch));
                        }
                    }

                    if is_alt {
                        if keysym >= b'a' as KeySym && keysym <= b'z' as KeySym {
                            return WindowEvent::Key(Key::Alt((keysym as u8) as char));
                        } else if keysym >= b'A' as KeySym && keysym <= b'Z' as KeySym {
                            return WindowEvent::Key(Key::Alt(((keysym as u8) + 32) as char));
                        }
                    }

                    if len > 0 {
                        if let Ok(s) = std::str::from_utf8(&buf[..len as usize]) {
                            if let Some(c) = s.chars().next() {
                                if c >= ' ' {
                                    return WindowEvent::Char(c);
                                }
                            }
                        }
                    }

                    WindowEvent::None
                }
                BUTTON_PRESS => {
                    let btn_ev = event.button;
                    let x = btn_ev.x.max(0) as usize;
                    let y = btn_ev.y.max(0) as usize;
                    match btn_ev.button {
                        1 => WindowEvent::MouseDown {
                            x,
                            y,
                            button: MouseButton::Left,
                        },
                        2 => WindowEvent::MouseDown {
                            x,
                            y,
                            button: MouseButton::Middle,
                        },
                        3 => WindowEvent::MouseDown {
                            x,
                            y,
                            button: MouseButton::Right,
                        },
                        4 => WindowEvent::ScrollUp { x, y },
                        5 => WindowEvent::ScrollDown { x, y },
                        _ => WindowEvent::None,
                    }
                }
                BUTTON_RELEASE => {
                    let btn_ev = event.button;
                    let x = btn_ev.x.max(0) as usize;
                    let y = btn_ev.y.max(0) as usize;
                    match btn_ev.button {
                        1 => WindowEvent::MouseUp {
                            x,
                            y,
                            button: MouseButton::Left,
                        },
                        2 => WindowEvent::MouseUp {
                            x,
                            y,
                            button: MouseButton::Middle,
                        },
                        3 => WindowEvent::MouseUp {
                            x,
                            y,
                            button: MouseButton::Right,
                        },
                        _ => WindowEvent::None,
                    }
                }
                MOTION_NOTIFY => {
                    let mot_ev = event.motion;
                    let x = mot_ev.x.max(0) as usize;
                    let y = mot_ev.y.max(0) as usize;
                    WindowEvent::MouseMove { x, y }
                }
                CONFIGURE_NOTIFY => {
                    let conf = event.configure;
                    let new_w = conf.width as usize;
                    let new_h = conf.height as usize;
                    if new_w != self.width || new_h != self.height {
                        self.width = new_w;
                        self.height = new_h;
                        return WindowEvent::Resize {
                            width: new_w,
                            height: new_h,
                        };
                    }
                    WindowEvent::None
                }
                CLIENT_MESSAGE => {
                    let client = event.client;
                    let data_atom = client.data[0] as Atom;
                    let data_atom_u32 = (client.data[0] as u32) as Atom;
                    if data_atom == self.wm_delete_window
                        || data_atom_u32 == self.wm_delete_window
                        || client.message_type == self.wm_delete_window
                    {
                        self.should_close = true;
                        return WindowEvent::Close;
                    }
                    WindowEvent::None
                }
                DESTROY_NOTIFY => {
                    self.should_close = true;
                    WindowEvent::Close
                }
                _ => WindowEvent::None,
            }
        }
    }

    /// Blits a software framebuffer to the native X11 window.
    pub fn present(&mut self, fb: &Framebuffer) {
        unsafe {
            let width = fb.width as c_uint;
            let height = fb.height as c_uint;
            let bytes_per_line = (fb.width * 4) as c_int;

            // Cast pixel pointer to char pointer
            let data_ptr = fb.pixels.as_ptr() as *mut c_char;

            let ximage = XCreateImage(
                self.display,
                self.visual,
                self.depth as c_uint,
                Z_PIXMAP,
                0,
                data_ptr,
                width,
                height,
                32,
                bytes_per_line,
            );

            if !ximage.is_null() {
                XPutImage(
                    self.display,
                    self.window,
                    self.gc,
                    ximage,
                    0,
                    0,
                    0,
                    0,
                    width,
                    height,
                );

                // Prevent XDestroyImage from freeing Rust's pixels buffer
                (*ximage).data = ptr::null_mut();
                XDestroyImage(ximage);

                XFlush(self.display);
            }
        }
    }

    pub fn should_close(&self) -> bool {
        self.should_close
    }
}

impl Drop for X11Window {
    fn drop(&mut self) {
        unsafe {
            if !self.gc.is_null() {
                XFreeGC(self.display, self.gc);
            }
            if !self.display.is_null() {
                XDestroyWindow(self.display, self.window);
                XFlush(self.display);
                XCloseDisplay(self.display);
            }
        }
    }
}
