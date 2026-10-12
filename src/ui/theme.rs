//! ANSI color themes and visual styling.
//! Zero third-party dependencies, purely standard library Rust.

#[derive(Debug, Clone)]
pub struct Theme {
    pub bg_reset: &'static str,
    pub fg_reset: &'static str,
    pub line_number_fg: &'static str,
    pub line_number_bg: &'static str,
    pub status_bar_bg: &'static str,
    pub status_bar_fg: &'static str,
    pub selection_bg: &'static str,
    pub cursor_style: &'static str,
    pub mode_insert_fg: &'static str,
    pub mode_normal_fg: &'static str,
    pub mode_command_fg: &'static str,
}

impl Theme {
    /// Creates a modern dark theme with ANSI 256/16 colors.
    pub fn modern_dark() -> Self {
        Self {
            bg_reset: "\x1b[49m",
            fg_reset: "\x1b[39m",
            line_number_fg: "\x1b[38;5;242m",
            line_number_bg: "\x1b[48;5;235m",
            status_bar_bg: "\x1b[48;5;236m",
            status_bar_fg: "\x1b[38;5;255m",
            selection_bg: "\x1b[48;5;238m",
            cursor_style: "\x1b[7m", // Inverted
            mode_insert_fg: "\x1b[1;32m",
            mode_normal_fg: "\x1b[1;36m",
            mode_command_fg: "\x1b[1;33m",
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::modern_dark()
    }
}
