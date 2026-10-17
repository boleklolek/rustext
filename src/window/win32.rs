//! Native Windows Win32 window backend with zero third-party dependencies.
//! Pure standard-library Rust interfacing with user32.dll and gdi32.dll.

#![cfg(target_os = "windows")]

use std::ffi::c_void;
use std::ptr;

use super::events::{MouseButton, WindowEvent};
use crate::gfx::Framebuffer;
use crate::ui::terminal::Key;

type HWND = *mut c_void;
type HDC = *mut c_void;
type HINSTANCE = *mut c_void;
type HICON = *mut c_void;
type HCURSOR = *mut c_void;
type HBRUSH = *mut c_void;
type HMENU = *mut c_void;
type LPARAM = isize;
type WPARAM = usize;
type LRESULT = isize;
type WNDPROC = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

#[repr(C)]
struct WNDCLASSEXW {
    cb_size: u32,
    style: u32,
    lpfn_wndproc: WNDPROC,
    cb_cls_extra: i32,
    cb_wnd_extra: i32,
    h_instance: HINSTANCE,
    h_icon: HICON,
    h_cursor: HCURSOR,
    hbr_background: HBRUSH,
    lpsz_menu_name: *const u16,
    lpsz_class_name: *const u16,
    h_icon_sm: HICON,
}

#[repr(C)]
struct POINT {
    x: i32,
    y: i32,
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    time: u32,
    pt: POINT,
}

#[repr(C)]
struct BITMAPINFOHEADER {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_pels_per_meter: i32,
    bi_y_pels_per_meter: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}

#[repr(C)]
struct BITMAPINFO {
    bmi_header: BITMAPINFOHEADER,
    bmi_colors: [u32; 1],
}

const WS_OVERLAPPEDWINDOW: u32 = 0x00CF0000;
const WS_VISIBLE: u32 = 0x10000000;
const PM_REMOVE: u32 = 0x0001;
const WM_DESTROY: u32 = 0x0002;
const WM_SIZE: u32 = 0x0005;
const WM_CLOSE: u32 = 0x0010;
const WM_KEYDOWN: u32 = 0x0100;
const WM_CHAR: u32 = 0x0102;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_RBUTTONDOWN: u32 = 0x0204;
const WM_RBUTTONUP: u32 = 0x0205;
const WM_MBUTTONDOWN: u32 = 0x0207;
const WM_MBUTTONUP: u32 = 0x0208;
const WM_MOUSEWHEEL: u32 = 0x020A;
const DIB_RGB_COLORS: u32 = 0;
const SRCCOPY: u32 = 0x00CC0020;

#[link(name = "user32")]
#[link(name = "gdi32")]
unsafe extern "system" {
    fn RegisterClassExW(lp_wnd_class: *const WNDCLASSEXW) -> u16;
    fn CreateWindowExW(
        dw_ex_style: u32,
        lp_class_name: *const u16,
        lp_window_name: *const u16,
        dw_style: u32,
        x: i32,
        y: i32,
        n_width: i32,
        n_height: i32,
        h_wnd_parent: HWND,
        h_menu: HMENU,
        h_instance: HINSTANCE,
        lp_param: *mut c_void,
    ) -> HWND;
    fn DefWindowProcW(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
    fn PeekMessageW(
        lp_msg: *mut MSG,
        hwnd: HWND,
        w_msg_filter_min: u32,
        w_msg_filter_max: u32,
        w_remove_msg: u32,
    ) -> i32;
    fn TranslateMessage(lp_msg: *const MSG) -> i32;
    fn DispatchMessageW(lp_msg: *const MSG) -> LRESULT;
    fn GetDC(hwnd: HWND) -> HDC;
    fn ReleaseDC(hwnd: HWND, hdc: HDC) -> i32;
    fn DestroyWindow(hwnd: HWND) -> i32;
    fn SetWindowTextW(hwnd: HWND, lp_string: *const u16) -> i32;
    fn GetAsyncKeyState(v_key: i32) -> i16;
    fn StretchDIBits(
        hdc: HDC,
        x_dest: i32,
        y_dest: i32,
        dest_width: i32,
        dest_height: i32,
        x_src: i32,
        y_src: i32,
        src_width: i32,
        src_height: i32,
        lp_bits: *const c_void,
        lpbmi: *const BITMAPINFO,
        i_usage: u32,
        rop: u32,
    ) -> i32;
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

pub fn detect_scale_factor() -> f64 {
    if let Ok(val) = std::env::var("RUSTEXT_SCALE_FACTOR") {
        if let Ok(scale) = val.trim().parse::<f64>() {
            if scale >= 0.5 && scale <= 5.0 {
                return scale;
            }
        }
    }
    if let Ok(val) = std::env::var("QT_SCALE_FACTOR") {
        if let Ok(scale) = val.trim().parse::<f64>() {
            if scale >= 0.5 && scale <= 4.0 {
                return scale;
            }
        }
    }
    1.0
}

pub struct Win32Window {
    hwnd: HWND,
    hdc: HDC,
    pub width: usize,
    pub height: usize,
    pub scale_factor: f64,
    should_close: bool,
}

impl Win32Window {
    pub fn new(title: &str, logical_width: usize, logical_height: usize) -> Result<Self, String> {
        let scale_factor = detect_scale_factor();
        let physical_w = ((logical_width as f64) * scale_factor).round() as usize;
        let physical_h = ((logical_height as f64) * scale_factor).round() as usize;
        unsafe {
            let class_name: Vec<u16> = "RustextWindowClass\0".encode_utf16().collect();
            let window_title: Vec<u16> = format!("{}\0", title).encode_utf16().collect();

            let wc = WNDCLASSEXW {
                cb_size: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: 0,
                lpfn_wndproc: window_proc,
                cb_cls_extra: 0,
                cb_wnd_extra: 0,
                h_instance: ptr::null_mut(),
                h_icon: ptr::null_mut(),
                h_cursor: ptr::null_mut(),
                hbr_background: ptr::null_mut(),
                lpsz_menu_name: ptr::null(),
                lpsz_class_name: class_name.as_ptr(),
                h_icon_sm: ptr::null_mut(),
            };

            RegisterClassExW(&wc);

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                window_title.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                100,
                100,
                physical_w as i32,
                physical_h as i32,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );

            if hwnd.is_null() {
                return Err("Failed to create Win32 Window".to_string());
            }

            let hdc = GetDC(hwnd);

            Ok(Self {
                hwnd,
                hdc,
                width: physical_w,
                height: physical_h,
                scale_factor,
                should_close: false,
            })
        }
    }

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// Dynamically updates the native window title bar.
    pub fn set_title(&mut self, title: &str) {
        unsafe {
            let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
            SetWindowTextW(self.hwnd, wide.as_ptr());
        }
    }

    pub fn poll_event(&mut self) -> WindowEvent {
        unsafe {
            let mut msg = std::mem::zeroed::<MSG>();
            if PeekMessageW(&mut msg, self.hwnd, 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);

                match msg.message {
                    WM_CLOSE | WM_DESTROY => {
                        self.should_close = true;
                        return WindowEvent::Close;
                    }
                    WM_SIZE => {
                        let new_w = (msg.lparam & 0xFFFF) as usize;
                        let new_h = ((msg.lparam >> 16) & 0xFFFF) as usize;
                        if new_w > 0 && new_h > 0 && (new_w != self.width || new_h != self.height) {
                            self.width = new_w;
                            self.height = new_h;
                            return WindowEvent::Resize {
                                width: new_w,
                                height: new_h,
                            };
                        }
                    }
                    WM_CHAR => {
                        let is_ctrl = (GetAsyncKeyState(0x11) as i32 & 0x8000) != 0; // VK_CONTROL
                        let is_shift = (GetAsyncKeyState(0x10) as i32 & 0x8000) != 0; // VK_SHIFT
                        if is_ctrl {
                            let ch = if raw_byte >= 1 && raw_byte <= 26 {
                                (b'a' + raw_byte - 1) as char
                            } else {
                                (raw_byte as char).to_ascii_lowercase()
                            };
                            if is_shift {
                                return WindowEvent::Key(Key::CtrlShift(ch));
                            } else {
                                return WindowEvent::Key(Key::Ctrl(ch));
                            }
                        } else if raw_byte >= b' ' {
                            return WindowEvent::Char(raw_byte as char);
                        }
                    }
                    WM_LBUTTONDOWN => {
                        let x = (msg.lparam & 0xFFFF) as usize;
                        let y = ((msg.lparam >> 16) & 0xFFFF) as usize;
                        return WindowEvent::MouseDown { x, y, button: MouseButton::Left };
                    }
                    WM_LBUTTONUP => {
                        let x = (msg.lparam & 0xFFFF) as usize;
                        let y = ((msg.lparam >> 16) & 0xFFFF) as usize;
                        return WindowEvent::MouseUp { x, y, button: MouseButton::Left };
                    }
                    WM_RBUTTONDOWN => {
                        let x = (msg.lparam & 0xFFFF) as usize;
                        let y = ((msg.lparam >> 16) & 0xFFFF) as usize;
                        return WindowEvent::MouseDown { x, y, button: MouseButton::Right };
                    }
                    WM_MBUTTONDOWN => {
                        let x = (msg.lparam & 0xFFFF) as usize;
                        let y = ((msg.lparam >> 16) & 0xFFFF) as usize;
                        return WindowEvent::MouseDown { x, y, button: MouseButton::Middle };
                    }
                    WM_MOUSEMOVE => {
                        let x = (msg.lparam & 0xFFFF) as usize;
                        let y = ((msg.lparam >> 16) & 0xFFFF) as usize;
                        return WindowEvent::MouseMove { x, y };
                    }
                    WM_MOUSEWHEEL => {
                        let x = (msg.lparam & 0xFFFF) as usize;
                        let y = ((msg.lparam >> 16) & 0xFFFF) as usize;
                        let delta = ((msg.wparam >> 16) & 0xFFFF) as i16;
                        if delta > 0 {
                            return WindowEvent::ScrollUp { x, y };
                        } else {
                            return WindowEvent::ScrollDown { x, y };
                        }
                    }
                    WM_KEYDOWN => {
                        match msg.wparam {
                            0x08 => return WindowEvent::Key(Key::Backspace),
                            0x09 => {
                                let is_shift = (GetAsyncKeyState(0x10) as i32 & 0x8000) != 0; // VK_SHIFT
                                if is_shift {
                                    return WindowEvent::Key(Key::BackTab);
                                } else {
                                    return WindowEvent::Key(Key::Tab);
                                }
                            }
                            0x0D => return WindowEvent::Key(Key::Enter),
                            0x1B => return WindowEvent::Key(Key::Esc),
                            0x25 => return WindowEvent::Key(Key::Left),
                            0x26 => return WindowEvent::Key(Key::Up),
                            0x27 => return WindowEvent::Key(Key::Right),
                            0x28 => return WindowEvent::Key(Key::Down),
                            0x24 => return WindowEvent::Key(Key::Home),
                            0x23 => return WindowEvent::Key(Key::End),
                            0x21 => return WindowEvent::Key(Key::PageUp),
                            0x22 => return WindowEvent::Key(Key::PageDown),
                            0x2E => return WindowEvent::Key(Key::Delete),
                            0x70..=0x7B => {
                                return WindowEvent::Key(Key::F((msg.wparam - 0x70 + 1) as u8))
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::None
        }
    }

    pub fn present(&mut self, fb: &Framebuffer) {
        unsafe {
            let bmi = BITMAPINFO {
                bmi_header: BITMAPINFOHEADER {
                    bi_size: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    bi_width: fb.width as i32,
                    bi_height: -(fb.height as i32), // Top-down DIB
                    bi_planes: 1,
                    bi_bit_count: 32,
                    bi_compression: 0, // BI_RGB
                    bi_size_image: 0,
                    bi_x_pels_per_meter: 0,
                    bi_y_pels_per_meter: 0,
                    bi_clr_used: 0,
                    bi_clr_important: 0,
                },
                bmi_colors: [0],
            };

            StretchDIBits(
                self.hdc,
                0,
                0,
                fb.width as i32,
                fb.height as i32,
                0,
                0,
                fb.width as i32,
                fb.height as i32,
                fb.pixels.as_ptr() as *const c_void,
                &bmi,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
        }
    }

    pub fn should_close(&self) -> bool {
        self.should_close
    }

    pub fn cancel_close(&mut self) {
        self.should_close = false;
    }

    pub fn start_interactive_move(&mut self) {}
}

impl Drop for Win32Window {
    fn drop(&mut self) {
        unsafe {
            if !self.hdc.is_null() {
                ReleaseDC(self.hwnd, self.hdc);
            }
            if !self.hwnd.is_null() {
                DestroyWindow(self.hwnd);
            }
        }
    }
}
