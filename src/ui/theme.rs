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

    /// Monokai ANSI theme.
    pub fn monokai() -> Self {
        Self {
            bg_reset: "\x1b[49m",
            fg_reset: "\x1b[39m",
            line_number_fg: "\x1b[38;5;241m",
            line_number_bg: "\x1b[48;5;235m",
            status_bar_bg: "\x1b[48;5;238m",
            status_bar_fg: "\x1b[38;5;230m",
            selection_bg: "\x1b[48;5;237m",
            cursor_style: "\x1b[7m",
            mode_insert_fg: "\x1b[1;32m",
            mode_normal_fg: "\x1b[1;35m",
            mode_command_fg: "\x1b[1;33m",
        }
    }

    /// Solarized Dark ANSI theme.
    pub fn solarized_dark() -> Self {
        Self {
            bg_reset: "\x1b[49m",
            fg_reset: "\x1b[39m",
            line_number_fg: "\x1b[38;5;244m",
            line_number_bg: "\x1b[48;5;234m",
            status_bar_bg: "\x1b[48;5;24m",
            status_bar_fg: "\x1b[38;5;254m",
            selection_bg: "\x1b[48;5;236m",
            cursor_style: "\x1b[7m",
            mode_insert_fg: "\x1b[1;32m",
            mode_normal_fg: "\x1b[1;34m",
            mode_command_fg: "\x1b[1;33m",
        }
    }

    /// Light ANSI theme.
    pub fn light() -> Self {
        Self {
            bg_reset: "\x1b[49m",
            fg_reset: "\x1b[39m",
            line_number_fg: "\x1b[38;5;245m",
            line_number_bg: "\x1b[48;5;254m",
            status_bar_bg: "\x1b[48;5;31m",
            status_bar_fg: "\x1b[38;5;231m",
            selection_bg: "\x1b[48;5;153m",
            cursor_style: "\x1b[7m",
            mode_insert_fg: "\x1b[1;32m",
            mode_normal_fg: "\x1b[1;34m",
            mode_command_fg: "\x1b[1;33m",
        }
    }

    /// Tokyo Night ANSI theme.
    pub fn tokyo_night() -> Self {
        Self {
            bg_reset: "\x1b[49m",
            fg_reset: "\x1b[39m",
            line_number_fg: "\x1b[38;5;60m",
            line_number_bg: "\x1b[48;5;234m",
            status_bar_bg: "\x1b[48;5;235m",
            status_bar_fg: "\x1b[38;5;153m",
            selection_bg: "\x1b[48;5;238m",
            cursor_style: "\x1b[7m",
            mode_insert_fg: "\x1b[1;32m",
            mode_normal_fg: "\x1b[1;34m",
            mode_command_fg: "\x1b[1;35m",
        }
    }

    /// Catppuccin Mocha ANSI theme.
    pub fn catppuccin() -> Self {
        Self {
            bg_reset: "\x1b[49m",
            fg_reset: "\x1b[39m",
            line_number_fg: "\x1b[38;5;241m",
            line_number_bg: "\x1b[48;5;235m",
            status_bar_bg: "\x1b[48;5;236m",
            status_bar_fg: "\x1b[38;5;189m",
            selection_bg: "\x1b[48;5;238m",
            cursor_style: "\x1b[7m",
            mode_insert_fg: "\x1b[1;32m",
            mode_normal_fg: "\x1b[1;35m",
            mode_command_fg: "\x1b[1;33m",
        }
    }

    /// One Dark Pro ANSI theme.
    pub fn one_dark() -> Self {
        Self {
            bg_reset: "\x1b[49m",
            fg_reset: "\x1b[39m",
            line_number_fg: "\x1b[38;5;240m",
            line_number_bg: "\x1b[48;5;235m",
            status_bar_bg: "\x1b[48;5;237m",
            status_bar_fg: "\x1b[38;5;251m",
            selection_bg: "\x1b[48;5;239m",
            cursor_style: "\x1b[7m",
            mode_insert_fg: "\x1b[1;32m",
            mode_normal_fg: "\x1b[1;34m",
            mode_command_fg: "\x1b[1;33m",
        }
    }

    /// Converts a GUI `ThemeColors` structure into a truecolor ANSI TUI Theme.
    pub fn from_theme_colors(c: &crate::gfx::ThemeColors) -> Self {
        let line_num_fg = format!("\x1b[38;2;{};{};{}m", (c.gutter_fg >> 16) & 0xff, (c.gutter_fg >> 8) & 0xff, c.gutter_fg & 0xff);
        let line_num_bg = format!("\x1b[48;2;{};{};{}m", (c.gutter_bg >> 16) & 0xff, (c.gutter_bg >> 8) & 0xff, c.gutter_bg & 0xff);
        let status_bg = format!("\x1b[48;2;{};{};{}m", (c.status_bar_bg >> 16) & 0xff, (c.status_bar_bg >> 8) & 0xff, c.status_bar_bg & 0xff);
        let status_fg = format!("\x1b[38;2;{};{};{}m", (c.status_bar_fg >> 16) & 0xff, (c.status_bar_fg >> 8) & 0xff, c.status_bar_fg & 0xff);
        let sel_bg = format!("\x1b[48;2;{};{};{}m", (c.selection_bg >> 16) & 0xff, (c.selection_bg >> 8) & 0xff, c.selection_bg & 0xff);

        Self {
            bg_reset: "\x1b[49m",
            fg_reset: "\x1b[39m",
            line_number_fg: Box::leak(line_num_fg.into_boxed_str()),
            line_number_bg: Box::leak(line_num_bg.into_boxed_str()),
            status_bar_bg: Box::leak(status_bg.into_boxed_str()),
            status_bar_fg: Box::leak(status_fg.into_boxed_str()),
            selection_bg: Box::leak(sel_bg.into_boxed_str()),
            cursor_style: "\x1b[7m",
            mode_insert_fg: "\x1b[1;32m",
            mode_normal_fg: "\x1b[1;36m",
            mode_command_fg: "\x1b[1;33m",
        }
    }

    /// Resolves theme by name.
    pub fn by_name(name: &str) -> Self {
        let trimmed = name.to_lowercase().trim().to_string();
        // 1. Prioritize external .theme files from user or workspace directories
        if let Some(user_theme) = crate::gfx::theme_loader::load_user_theme(&trimmed) {
            return Self::from_theme_colors(&user_theme);
        }

        // 2. Fall back to built-in compiled-in palettes
        match trimmed.as_str() {
            "tokyo" | "tokyonight" | "tokyo_night" | "tokyo-night" => Self::tokyo_night(),
            "catppuccin" | "catppuccin_mocha" | "catppuccin-mocha" | "mocha" => Self::catppuccin(),
            "onedark" | "one_dark" | "one-dark" | "atom" => Self::one_dark(),
            "monokai" => Self::monokai(),
            "solarized" | "solarized_dark" | "solarized-dark" => Self::solarized_dark(),
            "light" | "github_light" | "github-light" | "claro" => Self::light(),
            _ => Self::modern_dark(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::modern_dark()
    }
}
