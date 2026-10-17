//! Unified Linux window abstraction with seamless Wayland and X11 support.
//! Zero third-party dependencies, purely standard library Rust.
//! Automatically prefers native Wayland if running inside a Wayland compositor,
//! with instant, transparent fallback to X11 if Wayland is not available.

#![cfg(target_os = "linux")]

use super::events::WindowEvent;
use super::wayland::WaylandWindow;
use super::x11::X11Window;
use crate::gfx::Framebuffer;

/// Active native display backend on Linux.
pub enum LinuxBackend {
    Wayland(WaylandWindow),
    X11(X11Window),
}

/// Unified Linux desktop window bridging Wayland and X11 seamlessly.
pub struct LinuxWindow {
    pub width: usize,
    pub height: usize,
    backend: LinuxBackend,
}

impl LinuxWindow {
    /// Creates a native Linux window, prioritizing Wayland before falling back to X11.
    /// Can be explicitly overridden with the `RUSTEXT_BACKEND=wayland` or `RUSTEXT_BACKEND=x11` environment variable.
    pub fn new(title: &str, logical_width: usize, logical_height: usize) -> Result<Self, String> {
        // 1. Check explicit user override
        if let Ok(forced) = std::env::var("RUSTEXT_BACKEND") {
            let lower = forced.to_lowercase();
            if lower == "x11" {
                let win = X11Window::new(title, logical_width, logical_height)?;
                return Ok(Self {
                    width: win.width,
                    height: win.height,
                    backend: LinuxBackend::X11(win),
                });
            } else if lower == "wayland" {
                let win = WaylandWindow::new(title, logical_width, logical_height)?;
                return Ok(Self {
                    width: win.width,
                    height: win.height,
                    backend: LinuxBackend::Wayland(win),
                });
            }
        }

        // 2. Default: Prefer native Wayland if socket is available
        if WaylandWindow::is_available() {
            if let Ok(win) = WaylandWindow::new(title, logical_width, logical_height) {
                return Ok(Self {
                    width: win.width,
                    height: win.height,
                    backend: LinuxBackend::Wayland(win),
                });
            }
        }

        // 3. Transparent Fallback to X11
        let win = X11Window::new(title, logical_width, logical_height)?;
        Ok(Self {
            width: win.width,
            height: win.height,
            backend: LinuxBackend::X11(win),
        })
    }

    /// Returns true if currently running on a native Wayland compositor.
    pub fn is_wayland(&self) -> bool {
        matches!(self.backend, LinuxBackend::Wayland(_))
    }

    /// Returns true if currently running under an X11 server or XWayland.
    pub fn is_x11(&self) -> bool {
        matches!(self.backend, LinuxBackend::X11(_))
    }

    /// Returns the human-readable display backend name ("Wayland" or "X11").
    pub fn backend_name(&self) -> &'static str {
        match self.backend {
            LinuxBackend::Wayland(_) => "Wayland",
            LinuxBackend::X11(_) => "X11",
        }
    }

    /// Returns the monitor HiDPI scale factor.
    pub fn scale_factor(&self) -> f64 {
        match &self.backend {
            LinuxBackend::Wayland(w) => w.scale_factor(),
            LinuxBackend::X11(x) => x.scale_factor(),
        }
    }

    /// Sets the native window title bar text.
    pub fn set_title(&mut self, title: &str) {
        match &mut self.backend {
            LinuxBackend::Wayland(w) => w.set_title(title),
            LinuxBackend::X11(x) => x.set_title(title),
        }
    }

    /// Polls a single pending window event (non-blocking).
    pub fn poll_event(&mut self) -> WindowEvent {
        let ev = match &mut self.backend {
            LinuxBackend::Wayland(w) => w.poll_event(),
            LinuxBackend::X11(x) => x.poll_event(),
        };

        if let WindowEvent::Resize { width, height } = ev {
            self.width = width;
            self.height = height;
        }

        ev
    }

    /// Blits the software framebuffer onto the screen.
    pub fn present(&mut self, fb: &Framebuffer) {
        match &mut self.backend {
            LinuxBackend::Wayland(w) => w.present(fb),
            LinuxBackend::X11(x) => x.present(fb),
        }
    }

    /// Returns whether the window close action was requested.
    pub fn should_close(&self) -> bool {
        match &self.backend {
            LinuxBackend::Wayland(w) => w.should_close(),
            LinuxBackend::X11(x) => x.should_close(),
        }
    }

    /// Cancels a pending window close action.
    pub fn cancel_close(&mut self) {
        match &mut self.backend {
            LinuxBackend::Wayland(w) => w.cancel_close(),
            LinuxBackend::X11(x) => x.cancel_close(),
        }
    }

    /// Initiates an interactive window move via the Wayland compositor.
    pub fn start_interactive_move(&mut self) {
        if let LinuxBackend::Wayland(ref mut w) = self.backend {
            w.start_interactive_move();
        }
    }

    /// Sets the cursor shape (e.g. 1 = arrow, 9 = text I-beam) on Wayland.
    pub fn set_cursor_shape(&mut self, shape: u32) {
        if let LinuxBackend::Wayland(ref mut w) = self.backend {
            w.set_cursor_shape(shape);
        }
    }
}
