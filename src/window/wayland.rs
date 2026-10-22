//! Native Linux Wayland client window backend with zero third-party dependencies.
//! Pure standard-library Rust interfacing directly with the Wayland compositor socket
//! using the binary Wayland wire protocol and POSIX shared memory (wl_shm + memfd).
//! Compatible with GNOME, KDE Plasma, Sway, Hyprland, Weston, and all standard Wayland compositors.

#![cfg(target_os = "linux")]

use std::collections::VecDeque;
use std::ffi::{c_int, c_long, c_uint, c_void, CString};
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::ptr;

use super::events::{MouseButton, WindowEvent};
use crate::gfx::Framebuffer;
use crate::ui::terminal::Key;

// --- POSIX & Linux C Types for Shared Memory and sendmsg ---
#[repr(C)]
struct Msghdr {
    msg_name: *mut c_void,
    msg_namelen: u32,
    msg_iov: *mut Iovec,
    msg_iovlen: usize,
    msg_control: *mut c_void,
    msg_controllen: usize,
    msg_flags: c_int,
}

#[repr(C)]
struct Iovec {
    iov_base: *mut c_void,
    iov_len: usize,
}

#[repr(C)]
struct Cmsghdr {
    cmsg_len: usize,
    cmsg_level: c_int,
    cmsg_type: c_int,
}

const SOL_SOCKET: c_int = 1;
const SCM_RIGHTS: c_int = 1;

const PROT_READ: c_int = 0x1;
const PROT_WRITE: c_int = 0x2;
const MAP_SHARED: c_int = 0x01;
const MAP_FAILED: *mut c_void = !0 as *mut c_void;

const MFD_CLOEXEC: c_uint = 0x0001;

unsafe extern "C" {
    fn sendmsg(fd: c_int, msg: *const Msghdr, flags: c_int) -> isize;
    fn ftruncate(fd: c_int, length: c_long) -> c_int;
    fn mmap(
        addr: *mut c_void,
        length: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: c_long,
    ) -> *mut c_void;
    fn munmap(addr: *mut c_void, length: usize) -> c_int;
    fn close(fd: c_int) -> c_int;
}

// Fallback to syscall 319 (SYS_memfd_create on x86_64) or /dev/shm
unsafe fn create_shm_fd(name: &str, size: usize) -> io::Result<c_int> {
    // 1. Try memfd_create syscall
    let c_name = CString::new(name).unwrap_or_else(|_| CString::new("rustext-shm").unwrap());
    let fd = unsafe {
        // Syscall 319 is SYS_memfd_create on Linux x86_64, 279 on aarch64, 385 on riscv64
        #[cfg(target_arch = "x86_64")]
        let nr = 319isize;
        #[cfg(target_arch = "aarch64")]
        let nr = 279isize;
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        let nr = 319isize;

        libc_syscall2(nr, c_name.as_ptr() as usize, MFD_CLOEXEC as usize) as c_int
    };

    if fd >= 0 {
        if unsafe { ftruncate(fd, size as c_long) } == 0 {
            return Ok(fd);
        }
        unsafe { close(fd) };
    }

    // 2. Fallback to POSIX /dev/shm with O_CREAT | O_EXCL and 0600 mode to prevent symlink attacks (CWE-377 / CWE-59)
    use std::os::unix::fs::OpenOptionsExt;
    let shm_dir = Path::new("/dev/shm");
    for attempt in 0..10 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let fallback_path = if shm_dir.is_dir() {
            shm_dir.join(format!("{}-{}-{}-{}", name, std::process::id(), now, attempt))
        } else {
            PathBuf::from(format!("/tmp/{}-{}-{}-{}", name, std::process::id(), now, attempt))
        };

        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true) // O_CREAT | O_EXCL prevents symlink / hijacking attacks
            .mode(0o600)      // Restricted to current user
            .open(&fallback_path)
        {
            Ok(file) => {
                let _ = std::fs::remove_file(&fallback_path); // Unlink immediately so file is anonymous
                let raw_fd = file.as_raw_fd();
                if unsafe { ftruncate(raw_fd, size as c_long) } == 0 {
                    std::mem::forget(file);
                    return Ok(raw_fd);
                }
                unsafe { close(raw_fd) };
                return Err(io::Error::last_os_error());
            }
            Err(e) if attempt == 9 => return Err(e),
            Err(_) => continue,
        }
    }

    Err(io::Error::last_os_error())
}

unsafe extern "C" {
    fn syscall(number: c_long, ...) -> c_long;
}

unsafe fn libc_syscall2(nr: isize, arg1: usize, arg2: usize) -> isize {
    unsafe { syscall(nr as c_long, arg1, arg2) as isize }
}

// --- Wayland Object IDs ---
const DISPLAY_ID: u32 = 1;
const REGISTRY_ID: u32 = 2;

// --- Wayland Opcodes ---
// wl_display
const WL_DISPLAY_SYNC: u16 = 0;
const WL_DISPLAY_GET_REGISTRY: u16 = 1;

// wl_registry
const WL_REGISTRY_BIND: u16 = 0;

// wl_compositor
const WL_COMPOSITOR_CREATE_SURFACE: u16 = 0;

// wl_shm
const WL_SHM_CREATE_POOL: u16 = 0;

// wl_shm_pool
const WL_SHM_POOL_CREATE_BUFFER: u16 = 0;
#[allow(dead_code)]
const WL_SHM_POOL_DESTROY: u16 = 1;
const WL_SHM_POOL_RESIZE: u16 = 2;

// wl_buffer
const WL_BUFFER_DESTROY: u16 = 0;

// wl_surface
const WL_SURFACE_ATTACH: u16 = 1;
const WL_SURFACE_DAMAGE: u16 = 2;
const WL_SURFACE_COMMIT: u16 = 6;
#[allow(dead_code)]
const WL_SURFACE_SET_BUFFER_SCALE: u16 = 8;

// xdg_wm_base
const XDG_WM_BASE_GET_XDG_SURFACE: u16 = 2;
const XDG_WM_BASE_PONG: u16 = 3;

// xdg_surface
const XDG_SURFACE_GET_TOPLEVEL: u16 = 1;
const XDG_SURFACE_ACK_CONFIGURE: u16 = 4;

// xdg_toplevel
const XDG_TOPLEVEL_SET_TITLE: u16 = 2;
const XDG_TOPLEVEL_SET_APP_ID: u16 = 3;
const XDG_TOPLEVEL_MOVE: u16 = 5;
#[allow(dead_code)]
const XDG_TOPLEVEL_RESIZE: u16 = 6;
const XDG_TOPLEVEL_SET_MIN_SIZE: u16 = 8;

// zxdg_decoration_manager_v1
const ZXDG_DECORATION_MANAGER_V1_GET_TOPLEVEL_DECORATION: u16 = 1;

// zxdg_toplevel_decoration_v1
const ZXDG_TOPLEVEL_DECORATION_V1_SET_MODE: u16 = 1;
#[allow(dead_code)]
const ZXDG_TOPLEVEL_DECORATION_V1_MODE_CLIENT_SIDE: u32 = 1;
const ZXDG_TOPLEVEL_DECORATION_V1_MODE_SERVER_SIDE: u32 = 2;

// wl_seat
const WL_SEAT_GET_POINTER: u16 = 0;
const WL_SEAT_GET_KEYBOARD: u16 = 1;

// Linux evdev mouse button constants
const BTN_LEFT: u32 = 0x110;   // 272
const BTN_RIGHT: u32 = 0x111;  // 273
const BTN_MIDDLE: u32 = 0x112; // 274

/// Native Wayland window implementation communicating over a Unix domain socket.
pub struct WaylandWindow {
    stream: UnixStream,
    pub width: usize,
    pub height: usize,
    buffer_width: usize,
    buffer_height: usize,
    scale_factor: f64,
    should_close: bool,
    title: String,

    // Dynamic Wayland IDs
    #[allow(dead_code)]
    compositor_id: u32,
    #[allow(dead_code)]
    shm_id: u32,
    xdg_wm_base_id: u32,
    seat_id: u32,
    surface_id: u32,
    xdg_surface_id: u32,
    xdg_toplevel_id: u32,
    pointer_id: u32,
    keyboard_id: u32,
    shm_pool_id: u32,
    buffer_ids: [u32; 2],
    active_buf_idx: usize,
    slot_size: usize,
    #[allow(dead_code)]
    decor_mgr_id: u32,
    #[allow(dead_code)]
    toplevel_decor_id: u32,
    #[allow(dead_code)]
    cursor_shape_mgr_id: u32,
    cursor_shape_dev_id: u32,

    // Shared Memory Backing Buffer
    shm_fd: c_int,
    shm_ptr: *mut u32,
    shm_capacity: usize,

    // Event buffering & input state
    event_queue: VecDeque<WindowEvent>,
    read_buf: Vec<u8>,
    mouse_x: usize,
    mouse_y: usize,
    last_pointer_serial: u32,
    ctrl_pressed: bool,
    alt_pressed: bool,
    shift_pressed: bool,
    temp_payload: Vec<u8>,

    // Keyboard auto-repeat state
    repeat_rate: u32,
    repeat_delay: u32,
    repeating_keycode: Option<u32>,
    repeat_deadline: Option<std::time::Instant>,
}

impl WaylandWindow {
    /// Detects if a Wayland display socket exists and is connectable.
    pub fn is_available() -> bool {
        Self::find_socket_path().is_some()
    }

    /// Locates the Wayland Unix domain socket path.
    pub fn find_socket_path() -> Option<PathBuf> {
        if let Ok(display) = std::env::var("WAYLAND_DISPLAY") {
            let p = Path::new(&display);
            if p.is_absolute() && p.exists() {
                return Some(p.to_path_buf());
            }
            if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
                let candidate = Path::new(&runtime).join(&display);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }

        if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
            let candidate = Path::new(&runtime).join("wayland-0");
            if candidate.exists() {
                return Some(candidate);
            }
        }

        None
    }

    /// Creates and connects a new native Wayland window.
    pub fn new(title: &str, logical_width: usize, logical_height: usize) -> Result<Self, String> {
        let socket_path = Self::find_socket_path()
            .ok_or_else(|| "No Wayland compositor socket found (WAYLAND_DISPLAY / XDG_RUNTIME_DIR)".to_string())?;

        let mut stream = UnixStream::connect(&socket_path)
            .map_err(|e| format!("Failed to connect to Wayland socket {:?}: {}", socket_path, e))?;

        // 1. Initial Handshake: Request registry and sync to discover globals
        let mut msg_get_reg = Vec::with_capacity(12);
        Self::write_header(&mut msg_get_reg, DISPLAY_ID, WL_DISPLAY_GET_REGISTRY, 12);
        msg_get_reg.extend_from_slice(&REGISTRY_ID.to_ne_bytes());
        stream.write_all(&msg_get_reg).map_err(|e| e.to_string())?;

        let sync_id = 3u32;
        let mut msg_sync = Vec::with_capacity(12);
        Self::write_header(&mut msg_sync, DISPLAY_ID, WL_DISPLAY_SYNC, 12);
        msg_sync.extend_from_slice(&sync_id.to_ne_bytes());
        stream.write_all(&msg_sync).map_err(|e| e.to_string())?;

        // Read globals until sync callback fires
        let mut globals = Vec::new();
        let mut raw_buf = [0u8; 8192];
        let mut accumulated = Vec::new();
        let mut synced = false;

        while !synced {
            let n = stream.read(&mut raw_buf).map_err(|e| e.to_string())?;
            if n == 0 {
                return Err("Wayland connection closed during initial discovery".to_string());
            }
            accumulated.extend_from_slice(&raw_buf[..n]);

            let mut offset = 0;
            while offset + 8 <= accumulated.len() {
                let obj_id = u32::from_ne_bytes(accumulated[offset..offset + 4].try_into().unwrap());
                let op_sz = u32::from_ne_bytes(accumulated[offset + 4..offset + 8].try_into().unwrap());
                let opcode = (op_sz & 0xFFFF) as u16;
                let size = (op_sz >> 16) as usize;

                if size < 8 || offset + size > accumulated.len() {
                    break;
                }

                if obj_id == REGISTRY_ID && opcode == 0 {
                    // wl_registry.global(name: uint, interface: string, version: uint)
                    if size >= 16 {
                        let name = u32::from_ne_bytes(accumulated[offset + 8..offset + 12].try_into().unwrap());
                        let str_len = u32::from_ne_bytes(accumulated[offset + 12..offset + 16].try_into().unwrap()) as usize;
                        if offset + 16 + str_len <= offset + size {
                            let str_bytes = &accumulated[offset + 16..offset + 16 + str_len.saturating_sub(1)];
                            if let Ok(iface_name) = std::str::from_utf8(str_bytes) {
                                let ver_off = offset + 16 + ((str_len + 3) & !3);
                                if ver_off + 4 <= offset + size {
                                    let version = u32::from_ne_bytes(accumulated[ver_off..ver_off + 4].try_into().unwrap());
                                    globals.push((name, iface_name.to_string(), version));
                                }
                            }
                        }
                    }
                } else if obj_id == sync_id {
                    synced = true;
                }

                offset += size;
            }

            if offset > 0 {
                accumulated.drain(..offset);
            }
        }

        // 2. Identify required globals
        let comp_global = globals.iter().find(|(_, iface, _)| iface == "wl_compositor")
            .ok_or_else(|| "Wayland compositor missing wl_compositor interface".to_string())?;
        let shm_global = globals.iter().find(|(_, iface, _)| iface == "wl_shm")
            .ok_or_else(|| "Wayland compositor missing wl_shm interface".to_string())?;
        let xdg_global = globals.iter().find(|(_, iface, _)| iface == "xdg_wm_base")
            .ok_or_else(|| "Wayland compositor missing xdg_wm_base interface".to_string())?;
        let seat_global = globals.iter().find(|(_, iface, _)| iface == "wl_seat");
        let decor_global = globals.iter().find(|(_, iface, _)| iface == "zxdg_decoration_manager_v1");
        let _cursor_shape_global = globals.iter().find(|(_, iface, _)| iface == "wp_cursor_shape_manager_v1");

        // Client-allocated object IDs MUST be sequential in creation order (no gaps);
        // otherwise the compositor kills the client with
        // "invalid arguments for wl_compositor#N.create_surface". Optional globals
        // (wl_seat, zxdg_decoration_manager_v1 -- absent on GNOME/Mutter) only consume
        // an ID when the compositor really advertises them. `0` means "not present".
        let mut next_object_id = 4u32;
        let mut alloc_id = || {
            let id = next_object_id;
            next_object_id += 1;
            id
        };
        let compositor_id = alloc_id();
        let shm_id = alloc_id();
        let xdg_wm_base_id = alloc_id();
        let seat_id = if seat_global.is_some() { alloc_id() } else { 0 };
        let decor_mgr_id = if decor_global.is_some() { alloc_id() } else { 0 };
        let surface_id = alloc_id();
        let xdg_surface_id = alloc_id();
        let xdg_toplevel_id = alloc_id();
        let toplevel_decor_id = if decor_global.is_some() { alloc_id() } else { 0 };
        let pointer_id = if seat_global.is_some() { alloc_id() } else { 0 };
        let keyboard_id = if seat_global.is_some() { alloc_id() } else { 0 };
        let shm_pool_id = alloc_id();
        let buffer_id_0 = alloc_id();
        let buffer_id_1 = alloc_id();
        let _cursor_shape_mgr_id = 0u32;
        let _cursor_shape_dev_id = 0u32;

        // 3. Bind Globals
        Self::send_bind(&mut stream, comp_global.0, "wl_compositor", comp_global.2.min(4), compositor_id)?;
        Self::send_bind(&mut stream, shm_global.0, "wl_shm", 1, shm_id)?;
        Self::send_bind(&mut stream, xdg_global.0, "xdg_wm_base", 1, xdg_wm_base_id)?;
        if let Some(seat) = seat_global {
            Self::send_bind(&mut stream, seat.0, "wl_seat", seat.2.min(7), seat_id)?;
        }
        if let Some(decor) = decor_global {
            Self::send_bind(&mut stream, decor.0, "zxdg_decoration_manager_v1", 1, decor_mgr_id)?;
        }
        /*
        if let Some(cs) = cursor_shape_global {
            Self::send_bind(&mut stream, cs.0, "wp_cursor_shape_manager_v1", 1, cursor_shape_mgr_id)?;
        }
        */

        // 4. Create Surface & XDG Toplevel
        // wl_compositor.create_surface(new_id: surface_id)
        Self::send_request(&mut stream, compositor_id, WL_COMPOSITOR_CREATE_SURFACE, &[surface_id])?;

        // xdg_wm_base.get_xdg_surface(new_id: xdg_surface_id, surface: surface_id)
        Self::send_request(&mut stream, xdg_wm_base_id, XDG_WM_BASE_GET_XDG_SURFACE, &[xdg_surface_id, surface_id])?;

        // xdg_surface.get_toplevel(new_id: xdg_toplevel_id)
        Self::send_request(&mut stream, xdg_surface_id, XDG_SURFACE_GET_TOPLEVEL, &[xdg_toplevel_id])?;

        // Set Title & App ID
        Self::send_string_request(&mut stream, xdg_toplevel_id, XDG_TOPLEVEL_SET_TITLE, title)?;
        Self::send_string_request(&mut stream, xdg_toplevel_id, XDG_TOPLEVEL_SET_APP_ID, "rustext")?;

        // Set minimum window size (400x300)
        Self::send_request(&mut stream, xdg_toplevel_id, XDG_TOPLEVEL_SET_MIN_SIZE, &[400, 300])?;

        // Request Server-Side Window Decorations (Title bar, min/max/close buttons, frame, shadows)
        if decor_global.is_some() {
            Self::send_request(
                &mut stream,
                decor_mgr_id,
                ZXDG_DECORATION_MANAGER_V1_GET_TOPLEVEL_DECORATION,
                &[toplevel_decor_id, xdg_toplevel_id],
            )?;
            Self::send_request(
                &mut stream,
                toplevel_decor_id,
                ZXDG_TOPLEVEL_DECORATION_V1_SET_MODE,
                &[ZXDG_TOPLEVEL_DECORATION_V1_MODE_SERVER_SIDE],
            )?;
        }

        // If seat exists, request pointer and keyboard
        if seat_global.is_some() {
            Self::send_request(&mut stream, seat_id, WL_SEAT_GET_POINTER, &[pointer_id])?;
            Self::send_request(&mut stream, seat_id, WL_SEAT_GET_KEYBOARD, &[keyboard_id])?;

            /*
            if cursor_shape_global.is_some() {
                // wp_cursor_shape_manager_v1.get_pointer(cursor_shape_device: new_id, pointer: object) -> opcode 1
                let _ = Self::send_request(&mut stream, cursor_shape_mgr_id, 1, &[cursor_shape_dev_id, pointer_id]);
            }
            */
        }

        // Commit surface to trigger initial configure events
        Self::send_request(&mut stream, surface_id, WL_SURFACE_COMMIT, &[])?;

        // 5. Detect monitor scale factor
        let scale_factor = Self::detect_scale_factor();
        let physical_w = ((logical_width as f64) * scale_factor).round() as usize;
        let physical_h = ((logical_height as f64) * scale_factor).round() as usize;

        // 6. Allocate Double-Buffered Shared Memory Pool (supports up to 4K displays)
        let max_w = physical_w.max(3840);
        let max_h = physical_h.max(2160);
        let slot_size = max_w * max_h * 4;
        let shm_capacity = slot_size * 2;

        let shm_fd = unsafe { create_shm_fd("rustext-wayland-shm", shm_capacity).map_err(|e| e.to_string())? };
        let shm_ptr = unsafe {
            let p = mmap(ptr::null_mut(), shm_capacity, PROT_READ | PROT_WRITE, MAP_SHARED, shm_fd, 0);
            if p == MAP_FAILED {
                close(shm_fd);
                return Err("Failed to mmap shared memory for Wayland Framebuffer".to_string());
            }
            p as *mut u32
        };

        // Initialize entire shared memory pool with opaque dark background (0xFF1A1B26)
        unsafe {
            let total_words = shm_capacity / 4;
            for i in 0..total_words {
                *shm_ptr.add(i) = 0xFF1A1B26;
            }
        }

        // Create wl_shm_pool passing shm_fd via SCM_RIGHTS
        Self::send_create_shm_pool(&mut stream, shm_id, shm_pool_id, shm_fd, shm_capacity as i32)?;

        // Create two wl_buffers for double buffering
        let stride = (physical_w * 4) as i32;
        Self::send_request(
            &mut stream,
            shm_pool_id,
            WL_SHM_POOL_CREATE_BUFFER,
            &[buffer_id_0, 0, physical_w as u32, physical_h as u32, stride as u32, 1],
        )?;
        Self::send_request(
            &mut stream,
            shm_pool_id,
            WL_SHM_POOL_CREATE_BUFFER,
            &[buffer_id_1, slot_size as u32, physical_w as u32, physical_h as u32, stride as u32, 1],
        )?;

        // 7. Synchronously wait for the initial xdg_surface.configure from compositor
        let mut initial_configured = false;
        let mut configure_serial = 0u32;
        let mut conf_raw = [0u8; 4096];

        while !initial_configured {
            let n = stream.read(&mut conf_raw).map_err(|e| e.to_string())?;
            if n == 0 {
                return Err("Wayland connection closed waiting for initial configure".to_string());
            }
            accumulated.extend_from_slice(&conf_raw[..n]);

            let mut offset = 0;
            while offset + 8 <= accumulated.len() {
                let obj_id = u32::from_ne_bytes(accumulated[offset..offset + 4].try_into().unwrap());
                let op_sz = u32::from_ne_bytes(accumulated[offset + 4..offset + 8].try_into().unwrap());
                let opcode = (op_sz & 0xFFFF) as u16;
                let size = (op_sz >> 16) as usize;

                if size < 8 || offset + size > accumulated.len() {
                    break;
                }

                if obj_id == 1 && opcode == 0 && size >= 20 {
                    let failed_obj = u32::from_ne_bytes(accumulated[offset + 8..offset + 12].try_into().unwrap());
                    let err_code = u32::from_ne_bytes(accumulated[offset + 12..offset + 16].try_into().unwrap());
                    let str_len = u32::from_ne_bytes(accumulated[offset + 16..offset + 20].try_into().unwrap()) as usize;
                    let end = (offset + 20 + str_len).min(accumulated.len());
                    let err_msg = String::from_utf8_lossy(&accumulated[offset + 20..end]);
                    return Err(format!("Wayland protocol error on obj {}: code {}, msg: {}", failed_obj, err_code, err_msg));
                }

                if obj_id == xdg_surface_id && opcode == 0 && size >= 12 {
                    configure_serial = u32::from_ne_bytes(accumulated[offset + 8..offset + 12].try_into().unwrap());
                    initial_configured = true;
                } else if obj_id == xdg_wm_base_id && opcode == 0 && size >= 12 {
                    let ping_serial = u32::from_ne_bytes(accumulated[offset + 8..offset + 12].try_into().unwrap());
                    let _ = Self::send_request(&mut stream, xdg_wm_base_id, XDG_WM_BASE_PONG, &[ping_serial]);
                }

                offset += size;
            }

            if offset > 0 {
                accumulated.drain(..offset);
            }
        }

        // Acknowledge configure
        Self::send_request(&mut stream, xdg_surface_id, XDG_SURFACE_ACK_CONFIGURE, &[configure_serial])?;

        // Attach initial buffer, damage, and commit to map window on screen
        Self::send_request(&mut stream, surface_id, WL_SURFACE_ATTACH, &[buffer_id_0, 0, 0])?;
        Self::send_request(&mut stream, surface_id, WL_SURFACE_DAMAGE, &[0, 0, physical_w as u32, physical_h as u32])?;
        Self::send_request(&mut stream, surface_id, WL_SURFACE_COMMIT, &[])?;
        let _ = stream.flush();

        // Switch socket to non-blocking mode for interactive event loop
        stream.set_nonblocking(true).map_err(|e| e.to_string())?;

        let win = Self {
            stream,
            width: physical_w,
            height: physical_h,
            buffer_width: physical_w,
            buffer_height: physical_h,
            scale_factor,
            should_close: false,
            title: title.to_string(),
            compositor_id,
            shm_id,
            xdg_wm_base_id,
            seat_id,
            surface_id,
            xdg_surface_id,
            xdg_toplevel_id,
            pointer_id,
            keyboard_id,
            shm_pool_id,
            buffer_ids: [buffer_id_0, buffer_id_1],
            active_buf_idx: 0,
            slot_size,
            decor_mgr_id,
            toplevel_decor_id,
            cursor_shape_mgr_id: 0,
            cursor_shape_dev_id: 0,
            shm_fd,
            shm_ptr,
            shm_capacity,
            event_queue: VecDeque::new(),
            read_buf: accumulated,
            mouse_x: 0,
            mouse_y: 0,
            last_pointer_serial: 0,
            ctrl_pressed: false,
            alt_pressed: false,
            shift_pressed: false,
            temp_payload: Vec::with_capacity(1024),
            repeat_rate: 25,
            repeat_delay: 300,
            repeating_keycode: None,
            repeat_deadline: None,
        };

        Ok(win)
    }

    /// Queries the display density from environment or desktop settings.
    pub fn detect_scale_factor() -> f64 {
        if let Ok(val) = std::env::var("RUSTEXT_SCALE_FACTOR") {
            if let Ok(scale) = val.trim().parse::<f64>() {
                if scale >= 0.5 && scale <= 5.0 {
                    return scale;
                }
            }
        }
        if let Ok(val) = std::env::var("GDK_SCALE") {
            if let Ok(scale) = val.trim().parse::<f64>() {
                if scale >= 1.0 && scale <= 4.0 {
                    return scale;
                }
            }
        }
        if let Ok(val) = std::env::var("WAYLAND_SCALE") {
            if let Ok(scale) = val.trim().parse::<f64>() {
                if scale >= 1.0 && scale <= 4.0 {
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

    /// Returns the monitor HiDPI scale factor.
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// Sets the native window title.
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
        let _ = Self::send_string_request(&mut self.stream, self.xdg_toplevel_id, XDG_TOPLEVEL_SET_TITLE, title);
        let _ = Self::send_request(&mut self.stream, self.surface_id, WL_SURFACE_COMMIT, &[]);
    }

    /// Returns whether the window close button was clicked.
    pub fn should_close(&self) -> bool {
        self.should_close
    }

    /// Cancels a pending close request (e.g. when unsaved changes prompt is cancelled).
    pub fn cancel_close(&mut self) {
        self.should_close = false;
    }

    /// Sets cursor shape using wp_cursor_shape_device_v1 (1 = default arrow, 9 = text I-beam).
    pub fn set_cursor_shape(&mut self, shape: u32) {
        if self.cursor_shape_dev_id != 0 && self.last_pointer_serial != 0 {
            let _ = Self::send_request(&mut self.stream, self.cursor_shape_dev_id, 1, &[self.last_pointer_serial, shape]);
            let _ = self.stream.flush();
        }
    }

    /// Initiates an interactive window move via the Wayland compositor.
    pub fn start_interactive_move(&mut self) {
        if self.seat_id != 0 && self.last_pointer_serial != 0 {
            let _ = Self::send_request(
                &mut self.stream,
                self.xdg_toplevel_id,
                XDG_TOPLEVEL_MOVE,
                &[self.seat_id, self.last_pointer_serial],
            );
            let _ = self.stream.flush();
        }
    }

    /// Presents a software framebuffer onto the Wayland surface via shared memory.
    pub fn present(&mut self, fb: &Framebuffer) {
        if fb.width == 0 || fb.height == 0 || self.shm_ptr.is_null() {
            return;
        }

        // If framebuffer size changed from current buffer size, resize shared buffer and recreate wl_buffers
        if fb.width != self.buffer_width || fb.height != self.buffer_height {
            let needed_slot = fb.width * fb.height * 4;
            let needed_total = needed_slot * 2;
            if needed_total > self.shm_capacity {
                let old_capacity = self.shm_capacity;
                self.shm_capacity = needed_total.max(old_capacity * 2);
                self.slot_size = self.shm_capacity / 2;
                unsafe {
                    let _ = munmap(self.shm_ptr as *mut c_void, old_capacity);
                    let _ = ftruncate(self.shm_fd, self.shm_capacity as c_long);
                    let p = mmap(ptr::null_mut(), self.shm_capacity, PROT_READ | PROT_WRITE, MAP_SHARED, self.shm_fd, 0);
                    if p != MAP_FAILED {
                        self.shm_ptr = p as *mut u32;
                    }
                }
                // Notify Wayland compositor that shm pool size expanded
                let _ = Self::send_request(&mut self.stream, self.shm_pool_id, WL_SHM_POOL_RESIZE, &[self.shm_capacity as u32]);
            }

            // Destroy previous wl_buffers (opcode 0 on wl_buffer)
            let _ = Self::send_request(&mut self.stream, self.buffer_ids[0], WL_BUFFER_DESTROY, &[]);
            let _ = Self::send_request(&mut self.stream, self.buffer_ids[1], WL_BUFFER_DESTROY, &[]);
            self.buffer_ids[0] = self.buffer_ids[0].wrapping_add(2);
            self.buffer_ids[1] = self.buffer_ids[1].wrapping_add(2);

            let stride = (fb.width * 4) as u32;
            let _ = Self::send_request(
                &mut self.stream,
                self.shm_pool_id,
                WL_SHM_POOL_CREATE_BUFFER,
                &[self.buffer_ids[0], 0, fb.width as u32, fb.height as u32, stride, 1],
            );
            let _ = Self::send_request(
                &mut self.stream,
                self.shm_pool_id,
                WL_SHM_POOL_CREATE_BUFFER,
                &[self.buffer_ids[1], self.slot_size as u32, fb.width as u32, fb.height as u32, stride, 1],
            );

            self.buffer_width = fb.width;
            self.buffer_height = fb.height;
            self.width = fb.width;
            self.height = fb.height;
        }

        // Toggle double-buffering slot
        self.active_buf_idx = 1 - self.active_buf_idx;
        let buf_id = self.buffer_ids[self.active_buf_idx];
        let offset_bytes = self.active_buf_idx * self.slot_size;
        let dest_ptr = unsafe { (self.shm_ptr as *mut u8).add(offset_bytes) as *mut u32 };

        let count = (fb.width * fb.height).min(self.slot_size / 4);
        unsafe {
            for i in 0..count {
                *dest_ptr.add(i) = *fb.pixels.get_unchecked(i) | 0xFF000000;
            }
        }

        // Attach new buffer, damage, and commit to Wayland compositor
        let _ = Self::send_request(&mut self.stream, self.surface_id, WL_SURFACE_ATTACH, &[buf_id, 0, 0]);
        let _ = Self::send_request(&mut self.stream, self.surface_id, WL_SURFACE_DAMAGE, &[0, 0, fb.width as u32, fb.height as u32]);
        let _ = Self::send_request(&mut self.stream, self.surface_id, WL_SURFACE_COMMIT, &[]);
        let _ = self.stream.flush();
    }

    /// Polls a single pending window event (non-blocking).
    pub fn poll_event(&mut self) -> WindowEvent {
        // Return queued event if available
        if let Some(ev) = self.event_queue.pop_front() {
            return ev;
        }

        // Read available wire data from Wayland socket
        let mut tmp = [0u8; 8192];
        loop {
            match self.stream.read(&mut tmp) {
                Ok(0) => {
                    self.should_close = true;
                    return WindowEvent::Close;
                }
                Ok(n) => {
                    self.read_buf.extend_from_slice(&tmp[..n]);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        // Parse messages
        let mut offset = 0;
        let mut payload_buf = std::mem::take(&mut self.temp_payload);
        while offset + 8 <= self.read_buf.len() {
            let obj_id = u32::from_ne_bytes(self.read_buf[offset..offset + 4].try_into().unwrap());
            let op_sz = u32::from_ne_bytes(self.read_buf[offset + 4..offset + 8].try_into().unwrap());
            let opcode = (op_sz & 0xFFFF) as u16;
            let size = (op_sz >> 16) as usize;

            if size < 8 {
                // Malformed packet length: advance cursor to skip corrupt byte and avoid infinite freeze
                offset += 1;
                continue;
            }
            if offset + size > self.read_buf.len() {
                // Incomplete message: await remaining bytes from socket
                break;
            }
            payload_buf.clear();
            payload_buf.extend_from_slice(&self.read_buf[offset + 8..offset + size]);
            self.handle_event(obj_id, opcode, &payload_buf);
            offset += size;
        }
        self.temp_payload = payload_buf;

        if offset > 0 {
            self.read_buf.drain(..offset);
        }

        if let Some(ev) = self.event_queue.pop_front() {
            return ev;
        }

        // Keyboard typematic auto-repeat:
        // Wayland compositors never send repeated key events; client-side repeat is mandated.
        if let (Some(keycode), Some(deadline)) = (self.repeating_keycode, self.repeat_deadline) {
            let now = std::time::Instant::now();
            if now >= deadline {
                if self.repeat_rate > 0 {
                    let interval_ms = (1000 / self.repeat_rate).max(10) as u64;
                    self.repeat_deadline = Some(now + std::time::Duration::from_millis(interval_ms));
                    if let Some(ev) = self.translate_evdev_key(keycode) {
                        return ev;
                    }
                } else {
                    self.repeating_keycode = None;
                    self.repeat_deadline = None;
                }
            }
        }

        WindowEvent::None
    }

    /// Internal handler for deserialized Wayland protocol events.
    fn handle_event(&mut self, obj_id: u32, opcode: u16, payload: &[u8]) {
        // 0. wl_display.error(object_id: object, code: uint, message: string)
        if obj_id == DISPLAY_ID && opcode == 0 && payload.len() >= 12 {
            let failed_obj = u32::from_ne_bytes(payload[0..4].try_into().unwrap());
            let err_code = u32::from_ne_bytes(payload[4..8].try_into().unwrap());
            let str_len = u32::from_ne_bytes(payload[8..12].try_into().unwrap()) as usize;
            let end = (12 + str_len).min(payload.len());
            let err_msg = String::from_utf8_lossy(&payload[12..end]);
            eprintln!("[WAYLAND PROTOCOL ERROR] obj: {}, code: {}, msg: '{}'", failed_obj, err_code, err_msg);
            self.should_close = true;
            self.event_queue.push_back(WindowEvent::Close);
            return;
        }

        // 1. xdg_wm_base.ping(serial: uint) -> must reply with xdg_wm_base.pong(serial)
        if obj_id == self.xdg_wm_base_id && opcode == 0 && payload.len() >= 4 {
            let serial = u32::from_ne_bytes(payload[0..4].try_into().unwrap());
            let _ = Self::send_request(&mut self.stream, self.xdg_wm_base_id, XDG_WM_BASE_PONG, &[serial]);
            let _ = self.stream.flush();
            return;
        }

        // 2. xdg_surface.configure(serial: uint) -> acknowledge configure
        if obj_id == self.xdg_surface_id && opcode == 0 && payload.len() >= 4 {
            let serial = u32::from_ne_bytes(payload[0..4].try_into().unwrap());
            let _ = Self::send_request(&mut self.stream, self.xdg_surface_id, XDG_SURFACE_ACK_CONFIGURE, &[serial]);
            let _ = self.stream.flush();
            return;
        }

        // 3. xdg_toplevel.configure(width: int, height: int, states: array)
        if obj_id == self.xdg_toplevel_id && opcode == 0 && payload.len() >= 8 {
            let new_w = i32::from_ne_bytes(payload[0..4].try_into().unwrap());
            let new_h = i32::from_ne_bytes(payload[4..8].try_into().unwrap());
            if new_w > 0 && new_h > 0 {
                let uw = ((new_w as f64) * self.scale_factor).round() as usize;
                let uh = ((new_h as f64) * self.scale_factor).round() as usize;
                if uw != self.width || uh != self.height {
                    self.width = uw;
                    self.height = uh;
                    self.event_queue.push_back(WindowEvent::Resize { width: uw, height: uh });
                }
            }
            return;
        }

        // 4. xdg_toplevel.close()
        if obj_id == self.xdg_toplevel_id && opcode == 1 {
            self.should_close = true;
            self.event_queue.push_back(WindowEvent::Close);
            return;
        }

        // 5. wl_pointer events
        if obj_id == self.pointer_id {
            match opcode {
                // enter(serial: uint, surface: object, surface_x: fixed, surface_y: fixed)
                0 if payload.len() >= 16 => {
                    let serial = u32::from_ne_bytes(payload[0..4].try_into().unwrap());
                    self.last_pointer_serial = serial;
                    let fx = i32::from_ne_bytes(payload[8..12].try_into().unwrap());
                    let fy = i32::from_ne_bytes(payload[12..16].try_into().unwrap());
                    let x = ((fx as f64) / 256.0).max(0.0) as usize;
                    let y = ((fy as f64) / 256.0).max(0.0) as usize;
                    self.mouse_x = x;
                    self.mouse_y = y;

                    // Set standard arrow cursor via wp_cursor_shape_device_v1 (shape 1 = default arrow)
                    if self.cursor_shape_dev_id != 0 {
                        let _ = Self::send_request(&mut self.stream, self.cursor_shape_dev_id, 1, &[serial, 1]);
                        let _ = self.stream.flush();
                    }

                    if let Some(WindowEvent::MouseMove { x: mx, y: my }) = self.event_queue.back_mut() {
                        *mx = x;
                        *my = y;
                    } else {
                        self.event_queue.push_back(WindowEvent::MouseMove { x, y });
                    }
                }
                // motion(time: uint, surface_x: fixed, surface_y: fixed)
                2 if payload.len() >= 12 => {
                    let fx = i32::from_ne_bytes(payload[4..8].try_into().unwrap());
                    let fy = i32::from_ne_bytes(payload[8..12].try_into().unwrap());
                    let x = ((fx as f64) / 256.0).max(0.0) as usize;
                    let y = ((fy as f64) / 256.0).max(0.0) as usize;
                    self.mouse_x = x;
                    self.mouse_y = y;

                    if let Some(WindowEvent::MouseMove { x: mx, y: my }) = self.event_queue.back_mut() {
                        *mx = x;
                        *my = y;
                    } else {
                        self.event_queue.push_back(WindowEvent::MouseMove { x, y });
                    }
                }
                // button(serial: uint, time: uint, button: uint, state: uint)
                3 if payload.len() >= 16 => {
                    let serial = u32::from_ne_bytes(payload[0..4].try_into().unwrap());
                    self.last_pointer_serial = serial;
                    let btn_code = u32::from_ne_bytes(payload[8..12].try_into().unwrap());
                    let state = u32::from_ne_bytes(payload[12..16].try_into().unwrap());
                    let btn = match btn_code {
                        BTN_LEFT => Some(MouseButton::Left),
                        BTN_RIGHT => Some(MouseButton::Right),
                        BTN_MIDDLE => Some(MouseButton::Middle),
                        _ => None,
                    };
                    if let Some(button) = btn {
                        let ev = if state == 1 {
                            WindowEvent::MouseDown { x: self.mouse_x, y: self.mouse_y, button }
                        } else {
                            WindowEvent::MouseUp { x: self.mouse_x, y: self.mouse_y, button }
                        };
                        self.event_queue.push_back(ev);
                    }
                }
                // axis(time: uint, axis: uint, value: fixed)
                4 if payload.len() >= 12 => {
                    let axis = u32::from_ne_bytes(payload[4..8].try_into().unwrap());
                    let val = i32::from_ne_bytes(payload[8..12].try_into().unwrap());
                    if axis == 0 { // Vertical scroll
                        if val > 0 {
                            self.event_queue.push_back(WindowEvent::ScrollDown { x: self.mouse_x, y: self.mouse_y });
                        } else if val < 0 {
                            self.event_queue.push_back(WindowEvent::ScrollUp { x: self.mouse_x, y: self.mouse_y });
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // 6. zxdg_toplevel_decoration_v1 events
        if obj_id == self.toplevel_decor_id {
            return;
        }

        // 7. wl_buffer.release (opcode 0)
        if (obj_id == self.buffer_ids[0] || obj_id == self.buffer_ids[1]) && opcode == 0 {
            return;
        }

        // 8. wl_keyboard events
        if obj_id == self.keyboard_id {
            match opcode {
                // leave(serial, surface)
                2 => {
                    self.repeating_keycode = None;
                    self.repeat_deadline = None;
                    self.shift_pressed = false;
                    self.ctrl_pressed = false;
                    self.alt_pressed = false;
                }
                // key(serial, time, key, state)
                3 if payload.len() >= 16 => {
                    let keycode = u32::from_ne_bytes(payload[8..12].try_into().unwrap());
                    let state = u32::from_ne_bytes(payload[12..16].try_into().unwrap());

                    if state == 1 { // Pressed
                        if let Some(ev) = self.translate_evdev_key(keycode) {
                            self.event_queue.push_back(ev);
                            if Self::is_repeatable_key(keycode) && self.repeat_rate > 0 {
                                self.repeating_keycode = Some(keycode);
                                self.repeat_deadline = Some(
                                    std::time::Instant::now() + std::time::Duration::from_millis(self.repeat_delay as u64)
                                );
                            }
                        }
                    } else if state == 0 { // Released
                        if self.repeating_keycode == Some(keycode) {
                            self.repeating_keycode = None;
                            self.repeat_deadline = None;
                        }
                    }
                }
                // modifiers(serial, depressed, latched, locked, group)
                4 if payload.len() >= 16 => {
                    let mods = u32::from_ne_bytes(payload[4..8].try_into().unwrap());
                    self.shift_pressed = (mods & 0x01) != 0;
                    self.ctrl_pressed = (mods & 0x04) != 0;
                    self.alt_pressed = (mods & 0x08) != 0;
                }
                // repeat_info(rate: int, delay: int) - introduced in wl_keyboard v4
                5 if payload.len() >= 8 => {
                    let rate = i32::from_ne_bytes(payload[0..4].try_into().unwrap());
                    let delay = i32::from_ne_bytes(payload[4..8].try_into().unwrap());
                    if rate > 0 && delay > 0 {
                        self.repeat_rate = rate as u32;
                        self.repeat_delay = delay as u32;
                    } else {
                        self.repeat_rate = 0;
                        self.repeating_keycode = None;
                        self.repeat_deadline = None;
                    }
                }
                _ => {}
            }
        }
    }

    /// Checks whether an evdev scancode should repeat when held down.
    fn is_repeatable_key(scancode: u32) -> bool {
        !matches!(
            scancode,
            29   // KEY_LEFTCTRL
            | 97  // KEY_RIGHTCTRL
            | 42  // KEY_LEFTSHIFT
            | 54  // KEY_RIGHTSHIFT
            | 56  // KEY_LEFTALT
            | 100 // KEY_RIGHTALT
            | 125 // KEY_LEFTMETA
            | 126 // KEY_RIGHTMETA
            | 58  // KEY_CAPSLOCK
            | 69  // KEY_NUMLOCK
            | 70  // KEY_SCROLLLOCK
            | 127 // KEY_COMPOSE
        )
    }

    /// Maps standard Linux evdev input event scancodes to Rustext WindowEvents.
    fn translate_evdev_key(&self, scancode: u32) -> Option<WindowEvent> {
        // Modifiers & Navigation
        match scancode {
            1 => return Some(WindowEvent::Key(Key::Esc)),
            14 => return Some(WindowEvent::Key(Key::Backspace)),
            15 => return Some(WindowEvent::Key(if self.shift_pressed { Key::BackTab } else { Key::Tab })),
            28 | 96 => return Some(WindowEvent::Key(Key::Enter)),
            102 => return Some(WindowEvent::Key(Key::Home)),
            103 => return Some(WindowEvent::Key(
                if self.alt_pressed && self.shift_pressed { Key::ShiftAltUp }
                else if self.alt_pressed { Key::AltUp }
                else { Key::Up }
            )),
            104 => return Some(WindowEvent::Key(Key::PageUp)),
            105 => return Some(WindowEvent::Key(
                if self.alt_pressed { Key::AltLeft }
                else { Key::Left }
            )),
            106 => return Some(WindowEvent::Key(
                if self.alt_pressed { Key::AltRight }
                else { Key::Right }
            )),
            107 => return Some(WindowEvent::Key(Key::End)),
            108 => return Some(WindowEvent::Key(
                if self.alt_pressed && self.shift_pressed { Key::ShiftAltDown }
                else if self.alt_pressed { Key::AltDown }
                else { Key::Down }
            )),
            109 => return Some(WindowEvent::Key(Key::PageDown)),
            111 => return Some(WindowEvent::Key(Key::Delete)),
            // Function Keys F1..F10
            59..=68 => return Some(WindowEvent::Key(Key::F((scancode - 59 + 1) as u8))),
            87 => return Some(WindowEvent::Key(Key::F(11))),
            88 => return Some(WindowEvent::Key(Key::F(12))),
            _ => {}
        }

        // Alphabet Keys (Q..P, A..L, Z..M)
        let alpha_char = match scancode {
            16 => 'q', 17 => 'w', 18 => 'e', 19 => 'r', 20 => 't',
            21 => 'y', 22 => 'u', 23 => 'i', 24 => 'o', 25 => 'p',
            30 => 'a', 31 => 's', 32 => 'd', 33 => 'f', 34 => 'g',
            35 => 'h', 36 => 'j', 37 => 'k', 38 => 'l',
            44 => 'z', 45 => 'x', 46 => 'c', 47 => 'v', 48 => 'b',
            49 => 'n', 50 => 'm',
            _ => '\0',
        };

        if alpha_char != '\0' {
            if self.ctrl_pressed {
                if self.shift_pressed {
                    return Some(WindowEvent::Key(Key::CtrlShift(alpha_char)));
                } else {
                    return Some(WindowEvent::Key(Key::Ctrl(alpha_char)));
                }
            }
            if self.alt_pressed {
                return Some(WindowEvent::Key(Key::Alt(alpha_char)));
            }
            let ch = if self.shift_pressed {
                alpha_char.to_ascii_uppercase()
            } else {
                alpha_char
            };
            return Some(WindowEvent::Char(ch));
        }

        // Numeric Keys (1..9, 0)
        let num_char = match scancode {
            2 => if self.shift_pressed { '!' } else { '1' },
            3 => if self.shift_pressed { '@' } else { '2' },
            4 => if self.shift_pressed { '#' } else { '3' },
            5 => if self.shift_pressed { '$' } else { '4' },
            6 => if self.shift_pressed { '%' } else { '5' },
            7 => if self.shift_pressed { '^' } else { '6' },
            8 => if self.shift_pressed { '&' } else { '7' },
            9 => if self.shift_pressed { '*' } else { '8' },
            10 => if self.shift_pressed { '(' } else { '9' },
            11 => if self.shift_pressed { ')' } else { '0' },
            _ => '\0',
        };

        if num_char != '\0' {
            if self.ctrl_pressed {
                return Some(WindowEvent::Key(Key::Ctrl(num_char)));
            }
            return Some(WindowEvent::Char(num_char));
        }

        // Punctuation & Delimiters
        let punct = match scancode {
            12 => if self.shift_pressed { '_' } else { '-' },
            13 => if self.shift_pressed { '+' } else { '=' },
            26 => if self.shift_pressed { '{' } else { '[' },
            27 => if self.shift_pressed { '}' } else { ']' },
            39 => if self.shift_pressed { ':' } else { ';' },
            40 => if self.shift_pressed { '"' } else { '\'' },
            41 => if self.shift_pressed { '~' } else { '`' },
            43 => if self.shift_pressed { '|' } else { '\\' },
            51 => if self.shift_pressed { '<' } else { ',' },
            52 => if self.shift_pressed { '>' } else { '.' },
            53 => if self.shift_pressed { '?' } else { '/' },
            57 => ' ', // Space
            _ => '\0',
        };

        if punct != '\0' {
            if self.ctrl_pressed && punct == ' ' {
                return Some(WindowEvent::Key(Key::Ctrl(' ')));
            }
            if self.ctrl_pressed && (punct == '-' || punct == '=' || punct == '/' || punct == '?' || punct == '\\') {
                return Some(WindowEvent::Key(Key::Ctrl(punct)));
            }
            return Some(WindowEvent::Char(punct));
        }

        None
    }

    // --- Wire Protocol Encoders ---

    fn write_header(buf: &mut Vec<u8>, obj_id: u32, opcode: u16, size: u16) {
        buf.extend_from_slice(&obj_id.to_ne_bytes());
        let op_sz = (opcode as u32) | ((size as u32) << 16);
        buf.extend_from_slice(&op_sz.to_ne_bytes());
    }

    fn send_request(stream: &mut UnixStream, obj_id: u32, opcode: u16, args: &[u32]) -> Result<(), String> {
        let size = (8 + args.len() * 4) as u16;
        let mut msg = Vec::with_capacity(size as usize);
        Self::write_header(&mut msg, obj_id, opcode, size);
        for &arg in args {
            msg.extend_from_slice(&arg.to_ne_bytes());
        }
        stream.write_all(&msg).map_err(|e| e.to_string())
    }

    fn send_string_request(stream: &mut UnixStream, obj_id: u32, opcode: u16, s: &str) -> Result<(), String> {
        let bytes = s.as_bytes();
        let str_len = (bytes.len() + 1) as u32; // Include null terminator
        let pad = (4 - (str_len % 4)) % 4;
        let total_size = (8 + 4 + str_len + pad) as u16;

        let mut msg = Vec::with_capacity(total_size as usize);
        Self::write_header(&mut msg, obj_id, opcode, total_size);
        msg.extend_from_slice(&str_len.to_ne_bytes());
        msg.extend_from_slice(bytes);
        msg.push(0); // Null byte
        for _ in 0..pad {
            msg.push(0);
        }

        stream.write_all(&msg).map_err(|e| e.to_string())
    }

    fn send_bind(stream: &mut UnixStream, name: u32, iface: &str, version: u32, new_id: u32) -> Result<(), String> {
        let bytes = iface.as_bytes();
        let str_len = (bytes.len() + 1) as u32;
        let pad = (4 - (str_len % 4)) % 4;
        let total_size = (8 + 4 + 4 + str_len + pad + 4 + 4) as u16;

        let mut msg = Vec::with_capacity(total_size as usize);
        Self::write_header(&mut msg, REGISTRY_ID, WL_REGISTRY_BIND, total_size);
        msg.extend_from_slice(&name.to_ne_bytes());
        msg.extend_from_slice(&str_len.to_ne_bytes());
        msg.extend_from_slice(bytes);
        msg.push(0);
        for _ in 0..pad {
            msg.push(0);
        }
        msg.extend_from_slice(&version.to_ne_bytes());
        msg.extend_from_slice(&new_id.to_ne_bytes());

        stream.write_all(&msg).map_err(|e| e.to_string())
    }

    fn send_create_shm_pool(stream: &mut UnixStream, shm_id: u32, pool_id: u32, fd: c_int, size: i32) -> Result<(), String> {
        // Header (8) + new_id (4) + size (4) = 16 bytes. FD is passed via SCM_RIGHTS.
        let msg_len = 16u16;
        let mut msg_bytes = Vec::with_capacity(16);
        Self::write_header(&mut msg_bytes, shm_id, WL_SHM_CREATE_POOL, msg_len);
        msg_bytes.extend_from_slice(&pool_id.to_ne_bytes());
        msg_bytes.extend_from_slice(&size.to_ne_bytes());

        unsafe {
            let mut iov = Iovec {
                iov_base: msg_bytes.as_mut_ptr() as *mut c_void,
                iov_len: msg_bytes.len(),
            };

            // 24-byte control buffer for cmsghdr + c_int fd (aligned to usize)
            let mut cmsg_buf = [0u64; 3];
            let cmsg_ptr = cmsg_buf.as_mut_ptr() as *mut Cmsghdr;
            (*cmsg_ptr).cmsg_len = 20; // 16 bytes header + 4 bytes c_int
            (*cmsg_ptr).cmsg_level = SOL_SOCKET;
            (*cmsg_ptr).cmsg_type = SCM_RIGHTS;

            let fd_ptr = (cmsg_ptr as *mut u8).add(16) as *mut c_int;
            *fd_ptr = fd;

            let msg = Msghdr {
                msg_name: ptr::null_mut(),
                msg_namelen: 0,
                msg_iov: &mut iov,
                msg_iovlen: 1,
                msg_control: cmsg_buf.as_mut_ptr() as *mut c_void,
                msg_controllen: 24,
                msg_flags: 0,
            };

            let socket_fd = stream.as_raw_fd();
            let ret = sendmsg(socket_fd, &msg, 0);
            if ret < 0 {
                let os_err = io::Error::last_os_error();
                let reason = Self::read_display_error(stream)
                    .map(|e| format!(" (compositor protocol error: {})", e))
                    .unwrap_or_default();
                return Err(format!("sendmsg failed to pass shm fd to Wayland compositor: {}{}", os_err, reason));
            }
        }

        Ok(())
    }

    /// Reads pending events after a failed write and extracts `wl_display.error`
    /// (object 1, opcode 0: object_id, code, message) so the real cause of a
    /// closed connection is reported instead of a bare `Broken pipe`.
    fn read_display_error(stream: &mut UnixStream) -> Option<String> {
        let _ = stream.set_nonblocking(true);
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).unwrap_or(0);
        let _ = stream.set_nonblocking(false);
        let data = &buf[..n];
        let mut off = 0;
        while off + 8 <= data.len() {
            let obj_id = u32::from_ne_bytes(data[off..off + 4].try_into().ok()?);
            let op_sz = u32::from_ne_bytes(data[off + 4..off + 8].try_into().ok()?);
            let opcode = (op_sz & 0xFFFF) as u16;
            let size = (op_sz >> 16) as usize;
            if size < 8 || off + size > data.len() {
                break;
            }
            if obj_id == 1 && opcode == 0 && size >= 20 {
                let target = u32::from_ne_bytes(data[off + 8..off + 12].try_into().ok()?);
                let code = u32::from_ne_bytes(data[off + 12..off + 16].try_into().ok()?);
                let len = u32::from_ne_bytes(data[off + 16..off + 20].try_into().ok()?) as usize;
                let end = (off + 20 + len.saturating_sub(1)).min(off + size);
                let msg = String::from_utf8_lossy(&data[off + 20..end]).to_string();
                return Some(format!("object {} code {}: {}", target, code, msg));
            }
            off += size;
        }
        None
    }
}

impl Drop for WaylandWindow {
    fn drop(&mut self) {
        if !self.shm_ptr.is_null() && self.shm_capacity > 0 {
            unsafe {
                let _ = munmap(self.shm_ptr as *mut c_void, self.shm_capacity);
            }
        }
        if self.shm_fd >= 0 {
            unsafe {
                let _ = close(self.shm_fd);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::terminal::Key;
    use std::time::{Duration, Instant};

    #[test]
    fn test_is_repeatable_key_filtering() {
        // Repeatable navigation and editing keys
        assert!(WaylandWindow::is_repeatable_key(103)); // Up
        assert!(WaylandWindow::is_repeatable_key(108)); // Down
        assert!(WaylandWindow::is_repeatable_key(105)); // Left
        assert!(WaylandWindow::is_repeatable_key(106)); // Right
        assert!(WaylandWindow::is_repeatable_key(14));  // Backspace
        assert!(WaylandWindow::is_repeatable_key(111)); // Delete
        assert!(WaylandWindow::is_repeatable_key(28));  // Enter
        assert!(WaylandWindow::is_repeatable_key(15));  // Tab
        assert!(WaylandWindow::is_repeatable_key(30));  // 'a'

        // Non-repeatable modifier and toggle keys
        assert!(!WaylandWindow::is_repeatable_key(29));  // Left Ctrl
        assert!(!WaylandWindow::is_repeatable_key(97));  // Right Ctrl
        assert!(!WaylandWindow::is_repeatable_key(42));  // Left Shift
        assert!(!WaylandWindow::is_repeatable_key(54));  // Right Shift
        assert!(!WaylandWindow::is_repeatable_key(56));  // Left Alt
        assert!(!WaylandWindow::is_repeatable_key(100)); // Right Alt
        assert!(!WaylandWindow::is_repeatable_key(125)); // Super / Meta
        assert!(!WaylandWindow::is_repeatable_key(58));  // CapsLock
        assert!(!WaylandWindow::is_repeatable_key(69));  // NumLock
        assert!(!WaylandWindow::is_repeatable_key(70));  // ScrollLock
    }

    fn create_test_window() -> (WaylandWindow, UnixStream) {
        let (stream, peer) = UnixStream::pair().unwrap();
        stream.set_nonblocking(true).unwrap();
        peer.set_nonblocking(true).unwrap();
        let win = WaylandWindow {
            stream,
            width: 800,
            height: 600,
            buffer_width: 800,
            buffer_height: 600,
            scale_factor: 1.0,
            should_close: false,
            title: "Test".to_string(),
            compositor_id: 1,
            shm_id: 2,
            xdg_wm_base_id: 3,
            seat_id: 4,
            surface_id: 5,
            xdg_surface_id: 6,
            xdg_toplevel_id: 7,
            toplevel_decor_id: 8,
            decor_mgr_id: 0,
            cursor_shape_mgr_id: 0,
            cursor_shape_dev_id: 0,
            pointer_id: 9,
            keyboard_id: 10,
            shm_pool_id: 11,
            buffer_ids: [12, 13],
            active_buf_idx: 0,
            slot_size: 0,
            shm_fd: -1,
            shm_ptr: ptr::null_mut(),
            shm_capacity: 0,
            event_queue: VecDeque::new(),
            read_buf: Vec::new(),
            mouse_x: 0,
            mouse_y: 0,
            last_pointer_serial: 0,
            ctrl_pressed: false,
            alt_pressed: false,
            shift_pressed: false,
            temp_payload: Vec::new(),
            repeat_rate: 25,
            repeat_delay: 300,
            repeating_keycode: None,
            repeat_deadline: None,
        };
        (win, peer)
    }

    #[test]
    fn test_repeat_info_opcode_parsing() {
        let (mut win, _peer) = create_test_window();

        // Construct wl_keyboard.repeat_info payload: rate=30, delay=250
        let mut payload = Vec::new();
        payload.extend_from_slice(&30i32.to_ne_bytes());
        payload.extend_from_slice(&250i32.to_ne_bytes());
        win.handle_event(10, 5, &payload);

        assert_eq!(win.repeat_rate, 30);
        assert_eq!(win.repeat_delay, 250);

        // Test repeat_info disabling repeat (rate = 0)
        let mut disabled_payload = Vec::new();
        disabled_payload.extend_from_slice(&0i32.to_ne_bytes());
        disabled_payload.extend_from_slice(&250i32.to_ne_bytes());
        win.handle_event(10, 5, &disabled_payload);
        assert_eq!(win.repeat_rate, 0);
        assert!(win.repeating_keycode.is_none());
    }

    #[test]
    fn test_keyboard_press_repeat_and_release_lifecycle() {
        let (mut win, _peer) = create_test_window();
        win.repeat_delay = 200;

        // 1. Send wl_keyboard.key event: Down Arrow (keycode 108), state=1 (Pressed)
        let mut press_payload = Vec::new();
        press_payload.extend_from_slice(&1u32.to_ne_bytes()); // serial
        press_payload.extend_from_slice(&100u32.to_ne_bytes()); // time
        press_payload.extend_from_slice(&108u32.to_ne_bytes()); // key: KEY_DOWN
        press_payload.extend_from_slice(&1u32.to_ne_bytes()); // state: 1 (pressed)
        win.handle_event(10, 3, &press_payload);

        assert_eq!(win.repeating_keycode, Some(108));
        assert!(win.repeat_deadline.is_some());
        assert_eq!(win.event_queue.len(), 1);

        // First event popped: initial press
        let ev1 = win.poll_event();
        assert_eq!(ev1, WindowEvent::Key(Key::Down));

        // Immediately polling again before deadline: should return None
        let ev2 = win.poll_event();
        assert_eq!(ev2, WindowEvent::None);

        // Manually elapse deadline to simulate timer expiration
        win.repeat_deadline = Some(Instant::now() - Duration::from_millis(5));

        // Next poll should yield a repeated Down key event and schedule next deadline
        let ev_repeat = win.poll_event();
        assert_eq!(ev_repeat, WindowEvent::Key(Key::Down));
        assert!(win.repeat_deadline.unwrap() > Instant::now());

        // 2. Send key release event (state = 0)
        let mut release_payload = Vec::new();
        release_payload.extend_from_slice(&2u32.to_ne_bytes()); // serial
        release_payload.extend_from_slice(&350u32.to_ne_bytes()); // time
        release_payload.extend_from_slice(&108u32.to_ne_bytes()); // key: KEY_DOWN
        release_payload.extend_from_slice(&0u32.to_ne_bytes()); // state: 0 (released)
        win.handle_event(10, 3, &release_payload);

        assert_eq!(win.repeating_keycode, None);
        assert_eq!(win.repeat_deadline, None);

        // After release, no repeat events should fire
        let ev_after = win.poll_event();
        assert_eq!(ev_after, WindowEvent::None);
    }

    #[test]
    fn test_keyboard_leave_resets_repeat_and_modifiers() {
        let (mut win, _peer) = create_test_window();
        win.ctrl_pressed = true;
        win.alt_pressed = true;
        win.shift_pressed = true;
        win.repeating_keycode = Some(103); // Up arrow
        win.repeat_deadline = Some(Instant::now());

        // Opcode 2 is wl_keyboard.leave
        win.handle_event(10, 2, &[]);

        assert_eq!(win.repeating_keycode, None);
        assert_eq!(win.repeat_deadline, None);
        assert!(!win.shift_pressed);
        assert!(!win.ctrl_pressed);
        assert!(!win.alt_pressed);
    }
}
