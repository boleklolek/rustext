//! Graphical renderer translating editor state into framebuffer pixels.
//! Zero third-party dependencies, purely standard library Rust.

use super::edge_perception::EdgePerception;
use super::font::{FontFamily, FontRasterizer, FontWeight, FONT_HEIGHT, FONT_WIDTH};
use super::framebuffer::Framebuffer;
use super::gamma::{blend_pixel_gamma, blend_pixel_subpixel_gamma};
use super::hinting::{FontHinter, HintingLevel};
use super::optical_spacing::OpticalPunctuationSpacing;
use super::sdf::SdfFontRasterizer;
use super::shaping::FontShaper;
use super::stem_darkening::StemDarkener;
use crate::core::syntax::{SyntaxHighlighter, TokenType};
use crate::editor::Document;

pub struct ThemeColors {
    pub background: u32,
    pub gutter_bg: u32,
    pub gutter_fg: u32,
    pub gutter_border: u32,
    pub tab_bar_bg: u32,
    pub tab_active_bg: u32,
    pub tab_inactive_bg: u32,
    pub tab_active_fg: u32,
    pub tab_inactive_fg: u32,
    pub status_bar_bg: u32,
    pub status_bar_fg: u32,
    pub selection_bg: u32,
    pub cursor: u32,
    // Modern UI Elements
    pub accent: u32,
    pub modal_bg: u32,
    pub modal_header_bg: u32,
    pub modal_border: u32,
    pub input_bg: u32,
    pub input_border: u32,
    pub button_bg: u32,
    pub button_hover_bg: u32,
    pub button_fg: u32,
    pub chip_bg: u32,
    pub chip_fg: u32,
    // Scrollbar colors
    pub scrollbar_bg: u32,
    pub scrollbar_border: u32,
    pub scrollbar_thumb: u32,
    pub scrollbar_thumb_hover: u32,
    pub scrollbar_thumb_active: u32,
    // Syntax colors
    pub syntax_keyword: u32,
    pub syntax_type: u32,
    pub syntax_string: u32,
    pub syntax_number: u32,
    pub syntax_comment: u32,
    pub syntax_operator: u32,
    pub syntax_punctuation: u32,
    pub syntax_plain: u32,
    pub bracket_0: u32,
    pub bracket_1: u32,
    pub bracket_2: u32,
}

impl ThemeColors {
    pub fn tokyo_night() -> Self {
        Self {
            background: 0x001A1B26,
            gutter_bg: 0x0016161E,
            gutter_fg: 0x00565F89,
            gutter_border: 0x0024283B,
            tab_bar_bg: 0x0016161E,
            tab_active_bg: 0x001A1B26,
            tab_inactive_bg: 0x0013141C,
            tab_active_fg: 0x00C0CAF5,
            tab_inactive_fg: 0x00565F89,
            status_bar_bg: 0x0016161E,
            status_bar_fg: 0x00A9B1D6,
            selection_bg: 0x00283457,
            cursor: 0x007AA2F7,
            accent: 0x007AA2F7,
            modal_bg: 0x001F2335,
            modal_header_bg: 0x0016161E,
            modal_border: 0x003B4261,
            input_bg: 0x0024283B,
            input_border: 0x007AA2F7,
            button_bg: 0x0024283B,
            button_hover_bg: 0x003B4261,
            button_fg: 0x00C0CAF5,
            chip_bg: 0x0024283B,
            chip_fg: 0x007AA2F7,
            scrollbar_bg: 0x001A1B26,
            scrollbar_border: 0x0024283B,
            scrollbar_thumb: 0x003B4261,
            scrollbar_thumb_hover: 0x00565F89,
            scrollbar_thumb_active: 0x007AA2F7,
            syntax_keyword: 0x00BB9AF7,
            syntax_type: 0x002AC3DE,
            syntax_string: 0x009ECE6A,
            syntax_number: 0x00FF9E64,
            syntax_comment: 0x00565F89,
            syntax_operator: 0x0089DDFF,
            syntax_punctuation: 0x00C0CAF5,
            syntax_plain: 0x00C0CAF5,
            bracket_0: 0x00FFD700,
            bracket_1: 0x00BB9AF7,
            bracket_2: 0x007DCFFF,
        }
    }

    pub fn catppuccin_mocha() -> Self {
        Self {
            background: 0x001E1E2E,
            gutter_bg: 0x00181825,
            gutter_fg: 0x006C7086,
            gutter_border: 0x00313244,
            tab_bar_bg: 0x00181825,
            tab_active_bg: 0x001E1E2E,
            tab_inactive_bg: 0x0011111B,
            tab_active_fg: 0x00CDD6F4,
            tab_inactive_fg: 0x006C7086,
            status_bar_bg: 0x00181825,
            status_bar_fg: 0x00BAC2DE,
            selection_bg: 0x0045475A,
            cursor: 0x00F5E0DC,
            accent: 0x00CBA6F7,
            modal_bg: 0x001E1E2E,
            modal_header_bg: 0x00181825,
            modal_border: 0x0045475A,
            input_bg: 0x00313244,
            input_border: 0x00CBA6F7,
            button_bg: 0x00313244,
            button_hover_bg: 0x0045475A,
            button_fg: 0x00CDD6F4,
            chip_bg: 0x00313244,
            chip_fg: 0x00CBA6F7,
            scrollbar_bg: 0x001E1E2E,
            scrollbar_border: 0x00313244,
            scrollbar_thumb: 0x0045475A,
            scrollbar_thumb_hover: 0x00585B70,
            scrollbar_thumb_active: 0x00B4BEFE,
            syntax_keyword: 0x00CBA6F7,
            syntax_type: 0x0089B4FA,
            syntax_string: 0x00A6E3A1,
            syntax_number: 0x00FAB387,
            syntax_comment: 0x006C7086,
            syntax_operator: 0x0089DCEB,
            syntax_punctuation: 0x00CDD6F4,
            syntax_plain: 0x00CDD6F4,
            bracket_0: 0x00F9E2AF,
            bracket_1: 0x00CBA6F7,
            bracket_2: 0x0089DCEB,
        }
    }

    pub fn one_dark() -> Self {
        Self {
            background: 0x00282C34,
            gutter_bg: 0x0021252B,
            gutter_fg: 0x005C6370,
            gutter_border: 0x00181A1F,
            tab_bar_bg: 0x0021252B,
            tab_active_bg: 0x00282C34,
            tab_inactive_bg: 0x001B1D23,
            tab_active_fg: 0x00ABB2BF,
            tab_inactive_fg: 0x005C6370,
            status_bar_bg: 0x0021252B,
            status_bar_fg: 0x009DA5B4,
            selection_bg: 0x003E4451,
            cursor: 0x00528BFF,
            accent: 0x0061AFEF,
            modal_bg: 0x00282C34,
            modal_header_bg: 0x0021252B,
            modal_border: 0x003E4451,
            input_bg: 0x001E2227,
            input_border: 0x0061AFEF,
            button_bg: 0x00353B45,
            button_hover_bg: 0x003E4451,
            button_fg: 0x00ABB2BF,
            chip_bg: 0x00353B45,
            chip_fg: 0x0061AFEF,
            scrollbar_bg: 0x00282C34,
            scrollbar_border: 0x0021252B,
            scrollbar_thumb: 0x004B5263,
            scrollbar_thumb_hover: 0x005C6370,
            scrollbar_thumb_active: 0x0061AFEF,
            syntax_keyword: 0x00C678DD,
            syntax_type: 0x00E5C07B,
            syntax_string: 0x0098C379,
            syntax_number: 0x00D19A66,
            syntax_comment: 0x005C6370,
            syntax_operator: 0x0056B6C2,
            syntax_punctuation: 0x00ABB2BF,
            syntax_plain: 0x00ABB2BF,
            bracket_0: 0x00E5C07B,
            bracket_1: 0x00C678DD,
            bracket_2: 0x0061AFEF,
        }
    }

    pub fn modern_dark() -> Self {
        Self {
            background: 0x001E1E1E,
            gutter_bg: 0x00252526,
            gutter_fg: 0x00858585,
            gutter_border: 0x00333333,
            tab_bar_bg: 0x002D2D2D,
            tab_active_bg: 0x001E1E1E,
            tab_inactive_bg: 0x00252526,
            tab_active_fg: 0x00FFFFFF,
            tab_inactive_fg: 0x00969696,
            status_bar_bg: 0x00007ACC,
            status_bar_fg: 0x00FFFFFF,
            selection_bg: 0x00264F78,
            cursor: 0x00528BFF,
            accent: 0x00007ACC,
            modal_bg: 0x00252526,
            modal_header_bg: 0x002D2D2D,
            modal_border: 0x00007ACC,
            input_bg: 0x003C3C3C,
            input_border: 0x00007ACC,
            button_bg: 0x000E639C,
            button_hover_bg: 0x001177BB,
            button_fg: 0x00FFFFFF,
            chip_bg: 0x002D2D2D,
            chip_fg: 0x00007ACC,
            scrollbar_bg: 0x001E1E1E,
            scrollbar_border: 0x002D2D2D,
            scrollbar_thumb: 0x00424242,
            scrollbar_thumb_hover: 0x004F4F4F,
            scrollbar_thumb_active: 0x00686868,
            syntax_keyword: 0x00C586C0,
            syntax_type: 0x004EC9B0,
            syntax_string: 0x00CE9178,
            syntax_number: 0x00B5CEA8,
            syntax_comment: 0x006A9955,
            syntax_operator: 0x00D4D4D4,
            syntax_punctuation: 0x00D4D4D4,
            syntax_plain: 0x00D4D4D4,
            bracket_0: 0x00FFD700,
            bracket_1: 0x00DA70D6,
            bracket_2: 0x00179FFF,
        }
    }

    pub fn monokai() -> Self {
        Self {
            background: 0x00272822,
            gutter_bg: 0x001F201C,
            gutter_fg: 0x0075715E,
            gutter_border: 0x003E3D32,
            tab_bar_bg: 0x001E1F1C,
            tab_active_bg: 0x00272822,
            tab_inactive_bg: 0x001A1B18,
            tab_active_fg: 0x00F8F8F2,
            tab_inactive_fg: 0x0075715E,
            status_bar_bg: 0x0075715E,
            status_bar_fg: 0x00F8F8F2,
            selection_bg: 0x0049483E,
            cursor: 0x00F8F8F0,
            accent: 0x00F92672,
            modal_bg: 0x00272822,
            modal_header_bg: 0x001E1F1C,
            modal_border: 0x00F92672,
            input_bg: 0x003E3D32,
            input_border: 0x00F92672,
            button_bg: 0x00A6E22E,
            button_hover_bg: 0x00B8F53E,
            button_fg: 0x00272822,
            chip_bg: 0x003E3D32,
            chip_fg: 0x00A6E22E,
            scrollbar_bg: 0x00272822,
            scrollbar_border: 0x003E3D32,
            scrollbar_thumb: 0x0059584E,
            scrollbar_thumb_hover: 0x0075715E,
            scrollbar_thumb_active: 0x00A6E22E,
            syntax_keyword: 0x00F92672,
            syntax_type: 0x0066D9EF,
            syntax_string: 0x00E6DB74,
            syntax_number: 0x00AE81FF,
            syntax_comment: 0x0075715E,
            syntax_operator: 0x00F92672,
            syntax_punctuation: 0x00F8F8F2,
            syntax_plain: 0x00F8F8F2,
            bracket_0: 0x00E6DB74,
            bracket_1: 0x00AE81FF,
            bracket_2: 0x0066D9EF,
        }
    }

    pub fn solarized_dark() -> Self {
        Self {
            background: 0x00002B36,
            gutter_bg: 0x00073642,
            gutter_fg: 0x00586E75,
            gutter_border: 0x00073642,
            tab_bar_bg: 0x00073642,
            tab_active_bg: 0x00002B36,
            tab_inactive_bg: 0x0005222B,
            tab_active_fg: 0x0093A1A1,
            tab_inactive_fg: 0x00586E75,
            status_bar_bg: 0x00268BD2,
            status_bar_fg: 0x00FDF6E3,
            selection_bg: 0x00073642,
            cursor: 0x002AA198,
            accent: 0x00268BD2,
            modal_bg: 0x00073642,
            modal_header_bg: 0x00002B36,
            modal_border: 0x00268BD2,
            input_bg: 0x00002B36,
            input_border: 0x00268BD2,
            button_bg: 0x00268BD2,
            button_hover_bg: 0x002AA198,
            button_fg: 0x00FDF6E3,
            chip_bg: 0x00002B36,
            chip_fg: 0x002AA198,
            scrollbar_bg: 0x00002B36,
            scrollbar_border: 0x00073642,
            scrollbar_thumb: 0x00586E75,
            scrollbar_thumb_hover: 0x00657B83,
            scrollbar_thumb_active: 0x002AA198,
            syntax_keyword: 0x00859900,
            syntax_type: 0x00B58900,
            syntax_string: 0x002AA198,
            syntax_number: 0x00D33682,
            syntax_comment: 0x00586E75,
            syntax_operator: 0x00859900,
            syntax_punctuation: 0x0093A1A1,
            syntax_plain: 0x00839496,
            bracket_0: 0x00B58900,
            bracket_1: 0x006C71C4,
            bracket_2: 0x002AA198,
        }
    }

    pub fn light() -> Self {
        Self {
            background: 0x00FFFFFF,
            gutter_bg: 0x00F6F8FA,
            gutter_fg: 0x008C959F,
            gutter_border: 0x00D0D7DE,
            tab_bar_bg: 0x00EAEAEA,
            tab_active_bg: 0x00FFFFFF,
            tab_inactive_bg: 0x00F0F0F0,
            tab_active_fg: 0x0024292F,
            tab_inactive_fg: 0x0057606A,
            status_bar_bg: 0x00007ACC,
            status_bar_fg: 0x00FFFFFF,
            selection_bg: 0x00B4D5FE,
            cursor: 0x000969DA,
            accent: 0x000969DA,
            modal_bg: 0x00FFFFFF,
            modal_header_bg: 0x00F6F8FA,
            modal_border: 0x00D0D7DE,
            input_bg: 0x00F6F8FA,
            input_border: 0x000969DA,
            button_bg: 0x002DA44E,
            button_hover_bg: 0x002C974B,
            button_fg: 0x00FFFFFF,
            chip_bg: 0x00EFF1F3,
            chip_fg: 0x000969DA,
            scrollbar_bg: 0x00F6F8FA,
            scrollbar_border: 0x00D0D7DE,
            scrollbar_thumb: 0x00D0D7DE,
            scrollbar_thumb_hover: 0x00AFB8C1,
            scrollbar_thumb_active: 0x000969DA,
            syntax_keyword: 0x00CF222E,
            syntax_type: 0x008250DF,
            syntax_string: 0x000A3069,
            syntax_number: 0x000550AE,
            syntax_comment: 0x006E7781,
            syntax_operator: 0x00CF222E,
            syntax_punctuation: 0x0024292F,
            syntax_plain: 0x0024292F,
            bracket_0: 0x00B8860B,
            bracket_1: 0x008B008B,
            bracket_2: 0x00008B8B,
        }
    }

    pub fn by_name(name: &str) -> Self {
        match name.to_lowercase().trim() {
            "tokyo" | "tokyonight" | "tokyo_night" | "tokyo-night" => Self::tokyo_night(),
            "catppuccin" | "catppuccin_mocha" | "catppuccin-mocha" | "mocha" => Self::catppuccin_mocha(),
            "onedark" | "one_dark" | "one-dark" | "atom" => Self::one_dark(),
            "monokai" => Self::monokai(),
            "solarized" | "solarized_dark" | "solarized-dark" => Self::solarized_dark(),
            "light" | "github_light" | "github-light" | "claro" => Self::light(),
            _ => Self::modern_dark(),
        }
    }

    pub fn token_color(&self, token_type: TokenType) -> u32 {
        match token_type {
            TokenType::Keyword => self.syntax_keyword,
            TokenType::Type => self.syntax_type,
            TokenType::StringLiteral => self.syntax_string,
            TokenType::Number => self.syntax_number,
            TokenType::Comment => self.syntax_comment,
            TokenType::Operator => self.syntax_operator,
            TokenType::Punctuation => self.syntax_punctuation,
            TokenType::Identifier | TokenType::PlainText => self.syntax_plain,
            TokenType::Bracket(depth) => match depth % 3 {
                0 => self.bracket_0,
                1 => self.bracket_1,
                _ => self.bracket_2,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbarGeometry {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub thumb_y: usize,
    pub thumb_height: usize,
    pub visible_lines: usize,
    pub total_lines: usize,
    pub max_scroll: usize,
}

impl ScrollbarGeometry {
    /// Returns whether the point (x, y) is inside the scrollbar track.
    pub fn contains_point(&self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// Returns whether the point (x, y) is inside the scrollbar draggable thumb.
    pub fn thumb_contains_point(&self, x: usize, y: usize) -> bool {
        self.contains_point(x, y) && y >= self.thumb_y && y < self.thumb_y + self.thumb_height
    }

    /// Translates a target thumb Y pixel position into the corresponding document scroll row.
    pub fn row_for_thumb_y(&self, target_thumb_y: usize) -> usize {
        let scrollable_h = self.height.saturating_sub(self.thumb_height);
        if scrollable_h == 0 || self.max_scroll == 0 {
            0
        } else {
            let clamped_y = target_thumb_y.clamp(self.y, self.y + scrollable_h);
            let rel_y = clamped_y.saturating_sub(self.y);
            let ratio = rel_y as f32 / scrollable_h as f32;
            (ratio * self.max_scroll as f32).round() as usize
        }
    }
}

/// Geometry and bounding box for the floating autocomplete popup menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutocompleteGeometry {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub item_height: usize,
    pub visible_items: usize,
    pub total_items: usize,
}

impl AutocompleteGeometry {
    /// Returns whether the point (px, py) is inside the autocomplete popup container.
    pub fn contains_point(&self, px: usize, py: usize) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }

    /// Translates a mouse click Y pixel position into an item index within the candidate list.
    pub fn item_index_at(&self, py: usize, scroll_offset: usize) -> Option<usize> {
        let pad_y = 4;
        if py < self.y + pad_y || py >= self.y + self.height.saturating_sub(pad_y) || self.item_height == 0 {
            return None;
        }
        let row = (py - self.y - pad_y) / self.item_height;
        let idx = scroll_offset + row;
        if idx < self.total_items {
            Some(idx)
        } else {
            None
        }
    }
}

/// Geometry and bounding box for the interactive minimap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinimapGeometry {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub slider_y: usize,
    pub slider_height: usize,
    pub visible_lines: usize,
    pub total_lines: usize,
}

impl MinimapGeometry {
    /// Returns whether the point (x, y) is inside the minimap container.
    pub fn contains_point(&self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// Returns whether the point (x, y) is inside the viewport highlight slider.
    pub fn slider_contains_point(&self, x: usize, y: usize) -> bool {
        self.contains_point(x, y) && y >= self.slider_y && y < self.slider_y + self.slider_height
    }

    /// Translates a mouse click Y pixel coordinate into a document line index.
    pub fn line_for_click_y(&self, click_y: usize) -> usize {
        if self.height == 0 || self.total_lines == 0 {
            0
        } else {
            let rel_y = click_y.saturating_sub(self.y);
            (rel_y * self.total_lines) / self.height
        }
    }

    /// Translates a target slider Y pixel position into the corresponding document scroll row.
    pub fn row_for_slider_y(&self, target_slider_y: usize) -> usize {
        let scrollable_h = self.height.saturating_sub(self.slider_height);
        let max_scroll = self.total_lines.saturating_sub(self.visible_lines);
        if scrollable_h == 0 || max_scroll == 0 {
            0
        } else {
            let clamped_y = target_slider_y.clamp(self.y, self.y + scrollable_h);
            let rel_y = clamped_y.saturating_sub(self.y);
            let ratio = rel_y as f32 / scrollable_h as f32;
            (ratio * max_scroll as f32).round() as usize
        }
    }
}

/// Geometry and coordinates for a split editor pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneGeometry {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub is_active: bool,
}

impl PaneGeometry {
    pub fn contains_point(&self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

pub struct GfxRenderer {
    pub colors: ThemeColors,
    pub scale_factor: f64,
    pub menu_bar_height: usize,
    pub tab_bar_height: usize,
    pub status_bar_height: usize,
    pub scrollbar_width: usize,
    pub show_line_numbers: bool,
    pub diagnostics: Option<crate::editor::diagnostic::DiagnosticCollection>,
    pub font_scale: usize,
    pub font_antialiasing: bool,
    pub gamma_correction: bool,
    pub line_spacing: usize,
    pub rounded_corners: bool,
    pub shadows: bool,
    pub cursor_style: String,
    pub cursor_blink: bool,
    pub sidebar_width: usize,
    pub minimap_enabled: bool,
    pub minimap_width: usize,
    pub git_diff: Option<crate::editor::git::GitGutterDiff>,
    pub secondary_git_diff: Option<crate::editor::git::GitGutterDiff>,
    pub git_branch: Option<String>,
    pub git_statuses: Option<std::collections::HashMap<std::path::PathBuf, crate::editor::git::GitFileStatus>>,
    pub git_gutter_enabled: bool,
    pub terminal_inset: usize,
    pub font_ligatures: bool,
    pub font_hinting: HintingLevel,
    pub font_family: FontFamily,
    pub font_size: usize,
    pub font_weight: FontWeight,
    pub smooth_shadows: bool,
    pub animation_enabled: bool,
    pub edge_perception: bool,
    pub subpixel_antialiasing: bool,
    pub letter_spacing: usize,
    pub font_aspect_ratio: f64,
    pub stem_darkening: bool,
    pub stem_darkening_amount: f64,
    pub sdf_font_rendering: bool,
    pub optical_punctuation_spacing: bool,
    pub use_ttf_font: bool,
    pub word_wrap: bool,
}

pub const LOGICAL_MENU_BAR_HEIGHT: usize = 24;
pub const LOGICAL_TAB_BAR_HEIGHT: usize = 26;
pub const LOGICAL_STATUS_BAR_HEIGHT: usize = 22;
pub const LOGICAL_SCROLLBAR_WIDTH: usize = 14;
pub const LOGICAL_SIDEBAR_WIDTH: usize = 220;
pub const LOGICAL_MINIMAP_WIDTH: usize = 100;
pub const LOGICAL_LINE_SPACING: usize = 3;

impl GfxRenderer {
    pub fn new() -> Self {
        Self {
            colors: ThemeColors::modern_dark(),
            scale_factor: 1.0,
            menu_bar_height: LOGICAL_MENU_BAR_HEIGHT,
            tab_bar_height: LOGICAL_TAB_BAR_HEIGHT,
            status_bar_height: LOGICAL_STATUS_BAR_HEIGHT,
            scrollbar_width: LOGICAL_SCROLLBAR_WIDTH,
            show_line_numbers: true,
            diagnostics: None,
            font_scale: 1,
            font_antialiasing: true,
            gamma_correction: true,
            line_spacing: LOGICAL_LINE_SPACING,
            rounded_corners: true,
            shadows: true,
            cursor_style: "bar".to_string(),
            cursor_blink: true,
            sidebar_width: LOGICAL_SIDEBAR_WIDTH,
            minimap_enabled: true,
            minimap_width: LOGICAL_MINIMAP_WIDTH,
            git_diff: None,
            secondary_git_diff: None,
            git_branch: None,
            git_statuses: None,
            git_gutter_enabled: true,
            terminal_inset: 0,
            font_ligatures: true,
            font_hinting: HintingLevel::Medium,
            font_family: FontFamily::Standard,
            font_size: 12,
            font_weight: FontWeight::Normal,
            smooth_shadows: true,
            animation_enabled: true,
            edge_perception: true,
            subpixel_antialiasing: true,
            letter_spacing: 0,
            font_aspect_ratio: 0.50,
            stem_darkening: true,
            stem_darkening_amount: 1.0,
            sdf_font_rendering: true,
            optical_punctuation_spacing: true,
            use_ttf_font: false,
            word_wrap: false,
        }
    }

    /// Renders a modern drop shadow underneath floating dialogs, menus, and overlays.
    #[inline]
    pub fn render_shadow(
        &self,
        fb: &mut Framebuffer,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        radius: usize,
        blur: usize,
    ) {
        if !self.shadows {
            return;
        }
        if self.smooth_shadows {
            fb.draw_smooth_shadow_box(
                x,
                y,
                w,
                h,
                radius,
                blur,
                0,
                self.scale_coord(3) as isize,
                0x00000000,
                75,
            );
        } else {
            fb.draw_drop_shadow(x, y, w, h, radius, blur);
        }
    }

    /// Renders a syntax-highlighted token line with optional programming ligatures, hinting, and edge perception.
    #[inline]
    pub fn render_token_line(
        &self,
        fb: &mut Framebuffer,
        tokens: &[crate::core::syntax::TokenSpan<'_>],
        text_start_x: usize,
        text_y: usize,
        max_code_x: usize,
        scroll_col: usize,
        bg_color: u32,
    ) {
        let mut col_accum = 0;
        let char_w = self.char_width();
        let glyph_w = self.glyph_width();
        let line_chars: Option<Vec<char>> = if self.optical_punctuation_spacing {
            Some(tokens.iter().flat_map(|t| t.text.chars()).collect())
        } else {
            None
        };
        let mut char_idx_accum = 0;

        for token in tokens {
            let token_color = self.colors.token_color(token.token_type);
            let dark_factor = if self.stem_darkening {
                StemDarkener::compute_darkening_factor(bg_color, token_color, self.font_size, self.stem_darkening_amount)
            } else {
                0.0
            };

            if self.font_ligatures || self.font_hinting != HintingLevel::None || self.edge_perception || self.font_family != FontFamily::Standard || self.font_weight != FontWeight::Normal || self.font_size != 12 || self.subpixel_antialiasing || self.letter_spacing > 0 || (self.font_aspect_ratio - 0.50).abs() > 0.001 || dark_factor > 0.001 || self.sdf_font_rendering || self.optical_punctuation_spacing || self.use_ttf_font {
                let shaped_glyphs = FontShaper::shape_text_ex(&token.text, self.font_scale, self.font_ligatures, self.font_family, self.font_weight, glyph_w, self.char_height());
                for glyph in shaped_glyphs {
                    let col = col_accum;
                    col_accum += glyph.advance_cells;
                    let cluster_len = glyph.cluster.chars().count();
                    let current_char_idx = char_idx_accum;
                    char_idx_accum += cluster_len;

                    if col >= scroll_col {
                        let cell_px = text_start_x + (col - scroll_col) * char_w;
                        let ch = glyph.cluster.chars().next().unwrap_or(' ');
                        let is_box = FontRasterizer::is_box_drawing(ch);

                        let (prev_char, next_char) = if let Some(ref lc) = line_chars {
                            let p = if current_char_idx > 0 { lc.get(current_char_idx - 1).copied() } else { None };
                            let n = lc.get(current_char_idx + cluster_len).copied();
                            (p, n)
                        } else {
                            (None, None)
                        };

                        let opt_offset = if self.optical_punctuation_spacing && !is_box && !glyph.is_ligature {
                            OpticalPunctuationSpacing::compute_optical_offset(prev_char, ch, next_char, char_w, self.font_scale)
                        } else {
                            0
                        };

                        let (px, draw_w) = if is_box {
                            (cell_px, char_w)
                        } else {
                            let total_w = char_w * glyph.advance_cells;
                            let safe_x = OpticalPunctuationSpacing::compute_safe_in_cell_draw_x(cell_px, total_w, glyph.width, opt_offset);
                            (safe_x, glyph.width)
                        };

                        if px + draw_w <= max_code_x {
                            let max_w = draw_w.min(fb.width.saturating_sub(px));
                            let max_h = glyph.height.min(fb.height.saturating_sub(text_y));
                            if self.subpixel_antialiasing && !glyph.is_ligature {
                                let ttf_sub;
                                let sdf_sub;
                                let font_sub;
                                let subpixel_cov: &[[u8; 3]] = if self.use_ttf_font && !is_box {
                                    let ttf_font = crate::gfx::ttf::SystemFontLoader::get_active_font();
                                    let (_, _, sub) = crate::gfx::ttf::TtfRasterizer::rasterize_char_subpixel_cached(&ttf_font, ch, draw_w, glyph.height);
                                    ttf_sub = sub;
                                    &ttf_sub
                                } else if self.sdf_font_rendering && !is_box {
                                    sdf_sub = SdfFontRasterizer::rasterize_glyph_sdf_subpixel(
                                        ch,
                                        self.font_family,
                                        draw_w,
                                        glyph.height,
                                        self.font_weight,
                                        0.0,
                                    );
                                    &sdf_sub.2
                                } else {
                                    font_sub = FontRasterizer::rasterize_glyph_subpixel(
                                        ch,
                                        self.font_family,
                                        draw_w,
                                        glyph.height,
                                        self.font_weight,
                                    );
                                    &font_sub.2
                                };
                                for row in 0..max_h {
                                    let py = text_y + row;
                                    let row_offset = row * draw_w;
                                    let fb_offset = py * fb.width + px;
                                    for c in 0..max_w {
                                        let [mut ar, mut ag, mut ab] = subpixel_cov[row_offset + c];
                                        if ar > 0 || ag > 0 || ab > 0 {
                                            if dark_factor > 0.001 && !is_box {
                                                [ar, ag, ab] = StemDarkener::darken_subpixel(ar, ag, ab, dark_factor);
                                            }
                                            if self.edge_perception {
                                                ar = EdgePerception::adapt_edge_coverage(ar, token_color, bg_color);
                                                ag = EdgePerception::adapt_edge_coverage(ag, token_color, bg_color);
                                                ab = EdgePerception::adapt_edge_coverage(ab, token_color, bg_color);
                                            }
                                            fb.pixels[fb_offset + c] = blend_pixel_subpixel_gamma(bg_color, token_color, ar, ag, ab);
                                        }
                                    }
                                }
                            } else {
                                let box_cov;
                                let ttf_cov;
                                let darkened_ttf;
                                let sdf_cov;
                                let darkened_sdf;
                                let hinted_cov;
                                let darkened_hinted;
                                let darkened_std;
                                let effective_cov: &[u8] = if is_box && draw_w != glyph.width {
                                    box_cov = FontRasterizer::rasterize_glyph_sized(ch, self.font_family, draw_w, glyph.height, self.font_weight);
                                    &box_cov.2
                                } else if self.use_ttf_font && !glyph.is_ligature {
                                    let ttf_font = crate::gfx::ttf::SystemFontLoader::get_active_font();
                                    let (_, _, cov) = crate::gfx::ttf::TtfRasterizer::rasterize_char_cached(&ttf_font, ch, draw_w, glyph.height);
                                    if dark_factor > 0.001 {
                                        darkened_ttf = StemDarkener::darken_coverage_mask(&cov, draw_w, glyph.height, dark_factor);
                                        &darkened_ttf
                                    } else {
                                        ttf_cov = cov;
                                        &ttf_cov
                                    }
                                } else if self.sdf_font_rendering && !glyph.is_ligature {
                                    sdf_cov = SdfFontRasterizer::rasterize_glyph_sdf(
                                        ch,
                                        self.font_family,
                                        draw_w,
                                        glyph.height,
                                        self.font_weight,
                                        0.0,
                                    );
                                    if dark_factor > 0.001 {
                                        darkened_sdf = StemDarkener::darken_coverage_mask(&sdf_cov.2, draw_w, glyph.height, dark_factor);
                                        &darkened_sdf
                                    } else {
                                        &sdf_cov.2
                                    }
                                } else if self.font_hinting != HintingLevel::None {
                                    hinted_cov = FontHinter::hint_coverage(&glyph.coverage, glyph.width, glyph.height, self.font_hinting);
                                    if dark_factor > 0.001 && !is_box {
                                        darkened_hinted = StemDarkener::darken_coverage_mask(&hinted_cov, glyph.width, glyph.height, dark_factor);
                                        &darkened_hinted
                                    } else {
                                        &hinted_cov
                                    }
                                } else if dark_factor > 0.001 && !is_box {
                                    darkened_std = StemDarkener::darken_coverage_mask(&glyph.coverage, glyph.width, glyph.height, dark_factor);
                                    &darkened_std
                                } else {
                                    &glyph.coverage
                                };
                                for row in 0..max_h {
                                    let py = text_y + row;
                                    let row_offset = row * draw_w;
                                    let fb_offset = py * fb.width + px;
                                    for c in 0..max_w {
                                        let mut alpha = effective_cov[row_offset + c];
                                        if alpha > 0 {
                                            if self.edge_perception {
                                                alpha = EdgePerception::adapt_edge_coverage(alpha, token_color, bg_color);
                                            }
                                            fb.pixels[fb_offset + c] = blend_pixel_gamma(bg_color, token_color, alpha);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                for ch in token.text.chars() {
                    let col = col_accum;
                    col_accum += 1;
                    char_idx_accum += 1;
                    if col >= scroll_col {
                        let px = text_start_x + (col - scroll_col) * char_w;
                        if px + char_w <= max_code_x {
                            fb.draw_char(ch, px, text_y, token_color, None);
                        }
                    }
                }
            }
        }
    }

    /// Sets the monitor HiDPI scale factor and scales all logical layout coordinates.
    pub fn set_scale_factor(&mut self, factor: f64) {
        let factor = if factor > 0.1 { factor } else { 1.0 };
        self.scale_factor = factor;
        self.font_scale = (factor.round() as usize).max(1);
        self.menu_bar_height = self.scale_coord(LOGICAL_MENU_BAR_HEIGHT);
        self.tab_bar_height = self.scale_coord(LOGICAL_TAB_BAR_HEIGHT);
        self.status_bar_height = self.scale_coord(LOGICAL_STATUS_BAR_HEIGHT);
        self.scrollbar_width = self.scale_coord(LOGICAL_SCROLLBAR_WIDTH);
        self.sidebar_width = self.scale_coord(LOGICAL_SIDEBAR_WIDTH);
        self.minimap_width = self.scale_coord(LOGICAL_MINIMAP_WIDTH);
        self.line_spacing = self.scale_coord(LOGICAL_LINE_SPACING);
    }

    /// Multiplies a logical coordinate by the monitor scale factor to obtain physical pixels.
    #[inline]
    pub fn scale_coord(&self, logical: usize) -> usize {
        ((logical as f64) * self.scale_factor).round() as usize
    }

    /// Multiplies a logical float coordinate by the monitor scale factor.
    #[inline]
    pub fn scale_coord_f64(&self, logical: f64) -> f64 {
        logical * self.scale_factor
    }

    /// Converts physical pixels back into logical coordinate space.
    #[inline]
    pub fn to_logical_coord(&self, physical: usize) -> usize {
        if self.scale_factor > 0.0 {
            ((physical as f64) / self.scale_factor).round() as usize
        } else {
            physical
        }
    }

    /// Raw glyph width in physical pixels based on font_size, aspect ratio, and font_scale.
    #[inline]
    pub fn glyph_width(&self) -> usize {
        let h = self.char_height();
        if self.font_aspect_ratio > 0.0 && (self.font_aspect_ratio - 0.50).abs() > 0.001 {
            ((h as f64 * self.font_aspect_ratio).round() as usize).max(5)
        } else if self.font_size != 12 {
            ((self.font_size as f64 * 8.0 / 12.0).round() as usize).max(5) * self.font_scale
        } else {
            FONT_WIDTH * self.font_scale
        }
    }

    /// Character cell width in physical pixels based on font_size, aspect ratio, font_scale, and letter_spacing.
    #[inline]
    pub fn char_width(&self) -> usize {
        self.glyph_width() + self.letter_spacing * self.font_scale
    }

    /// Character height in physical pixels based on font_size and font_scale.
    #[inline]
    pub fn char_height(&self) -> usize {
        if self.font_size != 12 {
            ((self.font_size as f64 * 16.0 / 12.0).round() as usize).max(10) * self.font_scale
        } else {
            FONT_HEIGHT * self.font_scale
        }
    }

    /// Line height in physical pixels including line_spacing.
    #[inline]
    pub fn line_height(&self) -> usize {
        (self.char_height() + self.line_spacing).max(1)
    }

    /// Computes scrollbar dimensions and thumb position for a specific bounding rectangle.
    pub fn scrollbar_geometry_in_rect(
        &self,
        track_x: usize,
        track_y: usize,
        track_width: usize,
        track_height: usize,
        doc: &Document,
    ) -> ScrollbarGeometry {
        let line_height = self.line_height();
        let visible_lines = if line_height > 0 {
            track_height / line_height
        } else {
            0
        };
        let total_lines = doc.buffer.len_lines().max(1);
        let max_scroll = total_lines.saturating_sub(visible_lines);

        let thumb_height = if total_lines == 0 || track_height == 0 {
            track_height
        } else {
            let ratio = (visible_lines as f32 / total_lines as f32).min(1.0);
            let th = (ratio * track_height as f32).round() as usize;
            th.clamp(self.scale_coord(20), track_height)
        };

        let scrollable_h = track_height.saturating_sub(thumb_height);
        let thumb_y = if max_scroll > 0 && scrollable_h > 0 {
            let ratio = (doc.scroll_row as f32 / max_scroll as f32).min(1.0);
            track_y + (ratio * scrollable_h as f32).round() as usize
        } else {
            track_y
        };

        ScrollbarGeometry {
            x: track_x,
            y: track_y,
            width: track_width,
            height: track_height,
            thumb_y,
            thumb_height,
            visible_lines,
            total_lines,
            max_scroll,
        }
    }

    /// Computes scrollbar dimensions and thumb position for the given document and framebuffer.
    pub fn scrollbar_geometry(
        &self,
        fb_w: usize,
        fb_h: usize,
        doc: &Document,
        has_menu_bar: bool,
    ) -> ScrollbarGeometry {
        let menu_offset = if has_menu_bar { self.menu_bar_height } else { 0 };
        let content_y_start = menu_offset + self.tab_bar_height;
        let content_y_end = fb_h.saturating_sub(self.status_bar_height + self.terminal_inset);
        let track_height = content_y_end.saturating_sub(content_y_start);
        let track_width = self.scrollbar_width;
        let track_x = fb_w.saturating_sub(track_width);
        let track_y = content_y_start;

        self.scrollbar_geometry_in_rect(track_x, track_y, track_width, track_height, doc)
    }

    /// Computes minimap dimensions and slider position for the main viewport.
    pub fn minimap_geometry(
        &self,
        fb_w: usize,
        fb_h: usize,
        doc: &Document,
        has_menu_bar: bool,
    ) -> MinimapGeometry {
        let menu_offset = if has_menu_bar { self.menu_bar_height } else { 0 };
        let content_y_start = menu_offset + self.tab_bar_height;
        let content_y_end = fb_h.saturating_sub(self.status_bar_height + self.terminal_inset);
        let track_height = content_y_end.saturating_sub(content_y_start);
        let mm_width = self.minimap_width;
        let mm_x = fb_w.saturating_sub(self.scrollbar_width + mm_width);
        self.minimap_geometry_in_rect(mm_x, content_y_start, mm_width, track_height, doc)
    }

    /// Computes minimap dimensions and slider position within a specific rectangle.
    pub fn minimap_geometry_in_rect(
        &self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        doc: &Document,
    ) -> MinimapGeometry {
        let line_height = self.line_height();
        let visible_lines = if line_height > 0 { height / line_height } else { 1 };
        let total_lines = doc.buffer.len_lines().max(1);

        let slider_h = if total_lines == 0 || height == 0 {
            height
        } else {
            let ratio = (visible_lines as f32 / total_lines as f32).min(1.0);
            let sh = (ratio * height as f32).round() as usize;
            sh.clamp(self.scale_coord(16), height)
        };

        let scrollable_h = height.saturating_sub(slider_h);
        let max_scroll = total_lines.saturating_sub(visible_lines);
        let slider_y = if max_scroll > 0 && scrollable_h > 0 {
            let ratio = (doc.scroll_row as f32 / max_scroll as f32).min(1.0);
            y + (ratio * scrollable_h as f32).round() as usize
        } else {
            y
        };

        MinimapGeometry {
            x,
            y,
            width,
            height,
            slider_y,
            slider_height: slider_h,
            visible_lines,
            total_lines,
        }
    }

    /// Computes the bounding rectangles of both panes and the splitter bar for split editing.
    pub fn pane_geometries(
        &self,
        fb_w: usize,
        fb_h: usize,
        sidebar_w: usize,
        has_menu_bar: bool,
        split: &crate::editor::split::SplitState,
    ) -> (PaneGeometry, PaneGeometry, Option<(usize, usize, usize, usize)>) {
        self.pane_geometries_with_inset(fb_w, fb_h, sidebar_w, has_menu_bar, split, 0)
    }

    /// Computes the bounding rectangles of both panes with an optional bottom inset (e.g. terminal panel).
    pub fn pane_geometries_with_inset(
        &self,
        fb_w: usize,
        fb_h: usize,
        sidebar_w: usize,
        has_menu_bar: bool,
        split: &crate::editor::split::SplitState,
        bottom_inset: usize,
    ) -> (PaneGeometry, PaneGeometry, Option<(usize, usize, usize, usize)>) {
        let menu_offset = if has_menu_bar { self.menu_bar_height } else { 0 };
        let canvas_x = sidebar_w;
        let canvas_y = menu_offset;
        let canvas_w = fb_w.saturating_sub(sidebar_w);
        let canvas_h = fb_h.saturating_sub(menu_offset + self.status_bar_height + bottom_inset);

        match split.direction {
            crate::editor::split::SplitDirection::Vertical => {
                let splitter_size = 4;
                let p0_w = ((canvas_w.saturating_sub(splitter_size)) as f32 * split.ratio).round() as usize;
                let p1_w = canvas_w.saturating_sub(splitter_size + p0_w);
                let p0 = PaneGeometry {
                    x: canvas_x,
                    y: canvas_y,
                    width: p0_w,
                    height: canvas_h,
                    is_active: split.active_pane == 0,
                };
                let p1 = PaneGeometry {
                    x: canvas_x + p0_w + splitter_size,
                    y: canvas_y,
                    width: p1_w,
                    height: canvas_h,
                    is_active: split.active_pane == 1,
                };
                let splitter = (canvas_x + p0_w, canvas_y, splitter_size, canvas_h);
                (p0, p1, Some(splitter))
            }
            crate::editor::split::SplitDirection::Horizontal => {
                let splitter_size = 4;
                let p0_h = ((canvas_h.saturating_sub(splitter_size)) as f32 * split.ratio).round() as usize;
                let p1_h = canvas_h.saturating_sub(splitter_size + p0_h);
                let p0 = PaneGeometry {
                    x: canvas_x,
                    y: canvas_y,
                    width: canvas_w,
                    height: p0_h,
                    is_active: split.active_pane == 0,
                };
                let p1 = PaneGeometry {
                    x: canvas_x,
                    y: canvas_y + p0_h + splitter_size,
                    width: canvas_w,
                    height: p1_h,
                    is_active: split.active_pane == 1,
                };
                let splitter = (canvas_x, canvas_y + p0_h, canvas_w, splitter_size);
                (p0, p1, Some(splitter))
            }
            crate::editor::split::SplitDirection::None => {
                let p0 = PaneGeometry {
                    x: canvas_x,
                    y: canvas_y,
                    width: canvas_w,
                    height: canvas_h,
                    is_active: true,
                };
                (p0, p0, None)
            }
        }
    }

    /// Computes internal scrollbar, minimap, and code canvas boundaries for a given pane.
    pub fn pane_subgeometries(
        &self,
        pane: &PaneGeometry,
        doc: &Document,
    ) -> (ScrollbarGeometry, Option<MinimapGeometry>, usize) {
        let header_h = self.tab_bar_height;
        let content_y = pane.y + header_h;
        let content_h = pane.height.saturating_sub(header_h);
        let sb_w = self.scrollbar_width;
        let sb_x = pane.x + pane.width.saturating_sub(sb_w);
        let scroll_geom = self.scrollbar_geometry_in_rect(sb_x, content_y, sb_w, content_h, doc);

        let mini_w = if self.minimap_enabled && pane.width >= 320 {
            self.minimap_width.min(pane.width / 4)
        } else {
            0
        };
        let (mini_geom, max_code_x) = if mini_w > 0 {
            let mini_x = sb_x.saturating_sub(mini_w);
            let mg = self.minimap_geometry_in_rect(mini_x, content_y, mini_w, content_h, doc);
            (Some(mg), mini_x)
        } else {
            (None, sb_x)
        };

        (scroll_geom, mini_geom, max_code_x)
    }

    /// Renders the interactive minimap showing miniature syntax-highlighted code and viewport slider.
    pub fn render_minimap(
        &self,
        fb: &mut Framebuffer,
        doc: &Document,
        geom: &MinimapGeometry,
        is_hovered: bool,
        is_dragging: bool,
    ) {
        if geom.width == 0 || geom.height == 0 {
            return;
        }

        // 1. Background & subtle left border
        fb.fill_rect(geom.x, geom.y, geom.width, geom.height, self.colors.gutter_bg);
        fb.fill_rect(geom.x, geom.y, 1, geom.height, self.colors.gutter_border);

        // 2. Draw miniature code lines
        let total_lines = doc.buffer.len_lines();
        let draw_x = geom.x + 3;
        let max_draw_w = geom.width.saturating_sub(6);

        if total_lines > 0 && max_draw_w > 0 {
            let lines_step = if total_lines * 2 <= geom.height {
                1
            } else {
                ((total_lines + geom.height - 1) / geom.height).max(1)
            };

            let mut line_idx = 0;
            let use_fast_density = total_lines > 1500;

            while line_idx < total_lines {
                let my = if total_lines * 2 <= geom.height {
                    geom.y + line_idx * 2
                } else {
                    geom.y + (line_idx * (geom.height.saturating_sub(2))) / total_lines
                };

                if my >= geom.y + geom.height {
                    break;
                }

                if use_fast_density {
                    let line_len = doc.buffer.line_len_chars(line_idx);
                    if line_len > 0 {
                        let draw_w = (line_len * 2).min(max_draw_w).max(2);
                        fb.fill_rect(draw_x, my, draw_w, 1, self.colors.gutter_fg);
                    }
                } else if let Some(line_text) = doc.buffer.line(line_idx) {
                    let trimmed_left = line_text.trim_start();
                    let indent_chars = line_text.len().saturating_sub(trimmed_left.len());
                    let indent_px = (indent_chars / 2).min(max_draw_w / 3);

                    let tokens = SyntaxHighlighter::tokenize_line(&line_text, doc.language);
                    let mut cur_x = draw_x + indent_px;

                    for token in tokens {
                        let token_chars = token.text.chars().count();
                        let token_w = (token_chars * 2).max(1);
                        if cur_x >= draw_x + max_draw_w {
                            break;
                        }
                        let draw_w = token_w.min(draw_x + max_draw_w - cur_x);
                        let color = self.colors.token_color(token.token_type);
                        fb.fill_rect(cur_x, my, draw_w, 1, color);
                        cur_x += draw_w + 1;
                    }
                }

                line_idx += lines_step;
            }
        }

        // 3. Viewport highlight rectangle (slider)
        let alpha = if is_dragging { 110 } else if is_hovered { 80 } else { 50 };
        fb.fill_rect_alpha(geom.x + 1, geom.slider_y, geom.width.saturating_sub(1), geom.slider_height, self.colors.selection_bg, alpha);
        fb.draw_rect_outline(geom.x + 1, geom.slider_y, geom.width.saturating_sub(2), geom.slider_height, self.colors.accent);
    }

    /// Renders the vertical scrollbar track and draggable thumb.
    pub fn render_scrollbar(
        &self,
        fb: &mut Framebuffer,
        geom: &ScrollbarGeometry,
        is_hovered: bool,
        is_dragging: bool,
    ) {
        if geom.width == 0 || geom.height == 0 {
            return;
        }

        // 1. Track background
        fb.fill_rect(geom.x, geom.y, geom.width, geom.height, self.colors.scrollbar_bg);

        // 2. 1px left divider border separating code canvas from scrollbar
        fb.fill_rect(geom.x, geom.y, 1, geom.height, self.colors.scrollbar_border);

        // 3. Draggable thumb with 2px horizontal margin
        let thumb_color = if is_dragging {
            self.colors.scrollbar_thumb_active
        } else if is_hovered {
            self.colors.scrollbar_thumb_hover
        } else {
            self.colors.scrollbar_thumb
        };

        let thumb_margin_x = 2;
        let thumb_x = geom.x + thumb_margin_x;
        let thumb_w = geom.width.saturating_sub(thumb_margin_x * 2);

        if geom.thumb_height > 0 && thumb_w > 0 {
            if self.rounded_corners && geom.thumb_height >= 8 {
                let radius = (thumb_w / 2).min(4);
                fb.fill_rounded_rect(thumb_x, geom.thumb_y, thumb_w, geom.thumb_height, radius, thumb_color);
            } else {
                fb.fill_rect(thumb_x, geom.thumb_y, thumb_w, geom.thumb_height, thumb_color);
            }
        }
    }

    /// Computes floating popup window bounds anchored relative to the primary cursor.
    pub fn autocomplete_geometry(
        &self,
        fb_w: usize,
        fb_h: usize,
        doc: &Document,
        sidebar_w: usize,
        has_menu_bar: bool,
        item_count: usize,
    ) -> AutocompleteGeometry {
        let menu_offset = if has_menu_bar { self.menu_bar_height } else { 0 };
        let content_y_start = menu_offset + self.tab_bar_height;
        let total_lines = doc.buffer.len_lines();
        let gutter_cols = total_lines.to_string().len().max(3);
        let gutter_width = if self.show_line_numbers {
            (gutter_cols + 2) * self.char_width()
        } else {
            self.scale_coord(8)
        };
        let line_height = self.line_height();
        let text_start_x = sidebar_w + gutter_width + self.char_width();

        let primary_pos = doc.buffer.char_to_position(doc.cursors.primary().head);
        let cx = if primary_pos.col >= doc.scroll_col {
            text_start_x + (primary_pos.col - doc.scroll_col) * self.char_width()
        } else {
            text_start_x
        };
        let cy = if primary_pos.line >= doc.scroll_row {
            content_y_start + (primary_pos.line - doc.scroll_row) * line_height
        } else {
            content_y_start
        };

        let item_h = self.scale_coord(22);
        let pad_y = self.scale_coord(4);
        let max_vis = 7;
        let visible_items = item_count.min(max_vis);
        let popup_w = self.scale_coord(280).min(fb_w.saturating_sub(self.scale_coord(20)));
        let popup_h = visible_items * item_h + pad_y * 2;

        let mut pop_x = cx;
        let mut pop_y = cy + line_height + 2;

        let max_y = fb_h.saturating_sub(self.status_bar_height);
        if pop_y + popup_h > max_y {
            pop_y = cy.saturating_sub(popup_h + 2);
        }

        let max_x = fb_w.saturating_sub(self.scrollbar_width);
        if pop_x + popup_w > max_x {
            pop_x = max_x.saturating_sub(popup_w + 4);
        }
        if pop_x < 4 {
            pop_x = 4;
        }

        AutocompleteGeometry {
            x: pop_x,
            y: pop_y,
            width: popup_w,
            height: popup_h,
            item_height: item_h,
            visible_items,
            total_items: item_count,
        }
    }

    /// Renders the floating autocomplete popup menu with shadow, container, category chips, and selection highlight.
    pub fn render_autocomplete_popup(
        &self,
        fb: &mut Framebuffer,
        autocomplete: &crate::editor::autocomplete::AutocompleteState,
        geom: &AutocompleteGeometry,
    ) {
        if !autocomplete.is_active || autocomplete.items.is_empty() {
            return;
        }

        let pop_x = geom.x;
        let pop_y = geom.y;
        let pop_w = geom.width;
        let pop_h = geom.height;
        let item_h = geom.item_height;
        let pad_y = 4;

        // 1. Soft Drop Shadow
        if self.shadows {
            self.render_shadow(fb, pop_x, pop_y, pop_w, pop_h, 8, 6);
        } else {
            fb.fill_rect(pop_x + 3, pop_y + 3, pop_w, pop_h, 0x00101010);
        }

        // 2. Container Background & Outline
        if self.rounded_corners {
            fb.fill_rounded_rect(pop_x, pop_y, pop_w, pop_h, 6, self.colors.modal_bg);
            fb.draw_rounded_outline(pop_x, pop_y, pop_w, pop_h, 6, self.colors.modal_border);
        } else {
            fb.fill_rect(pop_x, pop_y, pop_w, pop_h, self.colors.modal_bg);
            fb.draw_rect_outline(pop_x, pop_y, pop_w, pop_h, self.colors.modal_border);
        }

        // 3. Render items
        for row in 0..geom.visible_items {
            let item_idx = autocomplete.scroll_offset + row;
            if item_idx >= autocomplete.items.len() {
                break;
            }

            let item = &autocomplete.items[item_idx];
            let is_selected = item_idx == autocomplete.selected_idx;
            let iy = pop_y + pad_y + row * item_h;

            // Selection Highlight
            if is_selected {
                if self.rounded_corners {
                    fb.fill_rounded_rect(pop_x + 3, iy, pop_w.saturating_sub(6), item_h, 4, self.colors.accent);
                } else {
                    fb.fill_rect(pop_x + 2, iy, pop_w.saturating_sub(4), item_h, self.colors.accent);
                }
            }

            let text_y = iy + (item_h.saturating_sub(self.char_height())) / 2;

            // Category Badge Chip
            let badge = item.kind.badge();
            let badge_w = badge.chars().count() * self.char_width() + self.scale_coord(6);
            let chip_h = self.scale_coord(16);
            let chip_y = iy + (item_h.saturating_sub(chip_h)) / 2;
            let badge_bg = if is_selected { 0x0024283B } else { self.colors.chip_bg };
            let badge_fg = if is_selected { 0x00FFFFFF } else { item.kind.badge_color() };

            if self.rounded_corners {
                fb.fill_rounded_rect(pop_x + 6, chip_y, badge_w, chip_h, 3, badge_bg);
            } else {
                fb.fill_rect(pop_x + 6, chip_y, badge_w, chip_h, badge_bg);
            }
            fb.draw_text(badge, pop_x + 9, text_y, badge_fg, None);

            // Label
            let label_x = pop_x + 6 + badge_w + 6;
            let label_fg = if is_selected { 0x00FFFFFF } else { self.colors.syntax_plain };
            let max_label_chars = 14;
            let label_display = if item.label.chars().count() > max_label_chars {
                let mut s: String = item.label.chars().take(max_label_chars - 1).collect();
                s.push('…');
                s
            } else {
                item.label.clone()
            };
            fb.draw_text(&label_display, label_x, text_y, label_fg, None);

            // Detail / Subtitle (right aligned)
            if !item.detail.is_empty() {
                let detail_fg = if is_selected { 0x00E0E0E0 } else { self.colors.gutter_fg };
                let max_det_chars = 10;
                let det_display = if item.detail.chars().count() > max_det_chars {
                    let mut s: String = item.detail.chars().take(max_det_chars - 1).collect();
                    s.push('…');
                    s
                } else {
                    item.detail.clone()
                };
                let det_w = det_display.chars().count() * self.char_width();
                let det_x = pop_x + pop_w - det_w - 10;
                if det_x > label_x + label_display.chars().count() * self.char_width() + 6 {
                    fb.draw_text(&det_display, det_x, text_y, detail_fg, None);
                }
            }
        }

        // 4. Scrollbar indicator if items exceed visible capacity
        if autocomplete.items.len() > geom.visible_items {
            let track_x = pop_x + pop_w - 4;
            let track_y = pop_y + pad_y;
            let track_h = geom.visible_items * item_h;
            fb.fill_rect(track_x, track_y, 2, track_h, self.colors.scrollbar_bg);

            let thumb_h = (track_h * geom.visible_items / autocomplete.items.len()).max(8);
            let max_scroll = autocomplete.items.len().saturating_sub(geom.visible_items);
            let thumb_y = if max_scroll > 0 {
                track_y + (autocomplete.scroll_offset * (track_h - thumb_h)) / max_scroll
            } else {
                track_y
            };
            fb.fill_rect(track_x, thumb_y, 2, thumb_h, self.colors.accent);
        }
    }

    /// Renders a complete editor frame into the software framebuffer with default scrollbar state.
    pub fn render(
        &self,
        fb: &mut Framebuffer,
        doc: &Document,
        tab_titles: &[String],
        active_tab_idx: usize,
        status_msg: Option<&str>,
        modal: Option<&crate::ui::prompt::PromptModal>,
        menu_bar: Option<&crate::editor::menu::MenuBar>,
    ) {
        self.render_with_scrollbar_state(
            fb,
            doc,
            tab_titles,
            active_tab_idx,
            status_msg,
            modal,
            menu_bar,
            false,
            false,
        );
    }

    /// Renders a complete editor frame into the software framebuffer with interactive scrollbar state.
    pub fn render_with_scrollbar_state(
        &self,
        fb: &mut Framebuffer,
        doc: &Document,
        tab_titles: &[String],
        active_tab_idx: usize,
        status_msg: Option<&str>,
        modal: Option<&crate::ui::prompt::PromptModal>,
        menu_bar: Option<&crate::editor::menu::MenuBar>,
        scrollbar_hovered: bool,
        scrollbar_dragging: bool,
    ) {
        self.render_full_frame(
            fb,
            doc,
            tab_titles,
            active_tab_idx,
            status_msg,
            modal,
            menu_bar,
            None,
            scrollbar_hovered,
            scrollbar_dragging,
            None,
        );
    }

    /// Renders an individual editor pane (header, gutter, code lines, minimap, scrollbar, focus outline).
    pub fn render_editor_pane(
        &self,
        fb: &mut Framebuffer,
        doc: &Document,
        pane_x: usize,
        pane_y: usize,
        pane_w: usize,
        pane_h: usize,
        is_active: bool,
        pane_title: &str,
        scrollbar_hovered: bool,
        scrollbar_dragging: bool,
        minimap_hovered: bool,
        minimap_dragging: bool,
    ) {
        if pane_w == 0 || pane_h == 0 {
            return;
        }

        fb.subpixel_antialiasing = self.subpixel_antialiasing;
        fb.letter_spacing = self.letter_spacing;
        fb.font_aspect_ratio = self.font_aspect_ratio;
        fb.stem_darkening = self.stem_darkening;
        fb.stem_darkening_amount = self.stem_darkening_amount;
        fb.font_family = self.font_family;
        fb.font_weight = self.font_weight;
        fb.font_size = self.font_size;
        fb.font_scale = self.font_scale;

        let pane_geom = PaneGeometry {
            x: pane_x,
            y: pane_y,
            width: pane_w,
            height: pane_h,
            is_active,
        };

        let header_h = self.tab_bar_height;
        let content_y = pane_y + header_h;
        let content_h = pane_h.saturating_sub(header_h);

        // 1. Pane Background
        fb.fill_rect(pane_x, pane_y, pane_w, pane_h, self.colors.background);

        // 2. Pane Header / Tab Strip
        let header_bg = if is_active { self.colors.tab_active_bg } else { self.colors.tab_inactive_bg };
        fb.fill_rect(pane_x, pane_y, pane_w, header_h, header_bg);
        fb.fill_rect(pane_x, pane_y + header_h - 1, pane_w, 1, self.colors.gutter_border);

        if is_active {
            // Distinctive accent bar for the focused pane
            fb.fill_rect(pane_x, pane_y + header_h - 2, pane_w, 2, self.colors.accent);
        }

        let title_fg = if is_active { self.colors.tab_active_fg } else { self.colors.tab_inactive_fg };
        let title_y = pane_y + (header_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(pane_title, pane_x + self.scale_coord(10), title_y, title_fg, None);

        if is_active {
            let badge_x = pane_x + self.scale_coord(10) + (pane_title.chars().count() + 1) * self.char_width();
            if badge_x + self.scale_coord(60) <= pane_x + pane_w {
                fb.draw_text("[ACTIVO]", badge_x, title_y, self.colors.accent, None);
            }
        }

        // 3. Compute Subgeometries (scrollbar, minimap, code limit)
        let (scroll_geom, mini_geom, max_code_x) = self.pane_subgeometries(&pane_geom, doc);

        // 4. Gutter
        let total_lines = doc.buffer.len_lines();
        let gutter_cols = total_lines.to_string().len().max(3);
        let gutter_w = if self.show_line_numbers { (gutter_cols + 2) * self.char_width() } else { self.scale_coord(8) };

        if self.show_line_numbers {
            fb.fill_rect(pane_x, content_y, gutter_w, content_h, self.colors.gutter_bg);
            fb.fill_rect(pane_x + gutter_w - 1, content_y, 1, content_h, self.colors.gutter_border);
        }

        // 5. Code Lines & Highlights
        let line_height = self.line_height();
        let visible_lines = content_h / line_height;
        let text_start_x = if self.show_line_numbers { pane_x + gutter_w + self.char_width() } else { pane_x + self.scale_coord(8) };

        let mut selections = Vec::new();
        let mut cursor_positions = Vec::new();

        for c in doc.cursors.cursors() {
            let pos = doc.buffer.char_to_position(c.head);
            cursor_positions.push(pos);

            if c.has_selection() {
                let range = c.selection_range();
                let start = doc.buffer.char_to_position(range.start);
                let end = doc.buffer.char_to_position(range.end);
                selections.push((start, end));
            }
        }

        if !self.word_wrap {
            for row in 0..visible_lines {
                let line_idx = doc.scroll_row + row;
                let py = content_y + row * line_height;
                let text_y = py + self.line_spacing / 2;

                if line_idx < total_lines {
                    // Gutter line number
                    if self.show_line_numbers {
                        let diag = self.diagnostics.as_ref().and_then(|dc| dc.for_line(line_idx + 1));
                        let (fg_col, badge) = if let Some(d) = diag {
                            (d.severity.color_hex(), Some(d.severity.badge()))
                        } else {
                            (self.colors.gutter_fg, None)
                        };

                        let num_str = format!("{:>width$}", line_idx + 1, width = gutter_cols);
                        fb.draw_text(&num_str, pane_x + self.char_width(), text_y, fg_col, None);

                        if let Some(b) = badge {
                            fb.draw_text(b, pane_x + 2, text_y, fg_col, None);
                        }

                        // Git Gutter diff indicator bar
                        if self.git_gutter_enabled {
                            let diff_opt = if is_active {
                                self.git_diff.as_ref()
                            } else {
                                self.secondary_git_diff.as_ref().or(self.git_diff.as_ref())
                            };
                            if let Some(diff) = diff_opt {
                                let bar_x = pane_x + gutter_w - 3;
                                match diff.status_for_line(line_idx) {
                                    crate::editor::git::GitLineStatus::Added => {
                                        fb.fill_rect(bar_x, py, 3, line_height, 0x009ECE6A);
                                    }
                                    crate::editor::git::GitLineStatus::Modified => {
                                        fb.fill_rect(bar_x, py, 3, line_height, 0x007AA2F7);
                                    }
                                    crate::editor::git::GitLineStatus::DeletedAbove => {
                                        fb.fill_rect(bar_x, py, 3, 2, 0x00F7768E);
                                    }
                                    crate::editor::git::GitLineStatus::None => {}
                                }
                            }
                        }
                    }

                    // Line Content
                    let line_cow = doc.buffer.line_cow(line_idx).unwrap_or(std::borrow::Cow::Borrowed(""));
                    let line_text: &str = &line_cow;

                    // Selections
                    for &(start, end) in &selections {
                        if line_idx >= start.line && line_idx <= end.line {
                            let sel_col_start = if line_idx == start.line { start.col } else { 0 };
                            let sel_col_end = if line_idx == end.line { end.col } else { line_text.chars().count() + 1 };

                            if sel_col_end > sel_col_start {
                                let sx = text_start_x + sel_col_start.saturating_sub(doc.scroll_col) * self.char_width();
                                if sx < max_code_x {
                                    let sw = ((sel_col_end - sel_col_start) * self.char_width()).min(max_code_x - sx);
                                    fb.fill_rect(sx, py, sw, line_height, self.colors.selection_bg);
                                }
                            }
                        }
                    }

                    // Syntax Highlighted Tokens
                    let tokens = SyntaxHighlighter::tokenize_line(line_text, doc.language);
                    self.render_token_line(fb, &tokens, text_start_x, text_y, max_code_x, doc.scroll_col, self.colors.background);

                    // Cursors (only rendered if this pane is active)
                    if is_active {
                        for pos in &cursor_positions {
                            if pos.line == line_idx && pos.col >= doc.scroll_col {
                                let cx = text_start_x + (pos.col - doc.scroll_col) * self.char_width();
                                if cx + 2 <= max_code_x {
                                    match self.cursor_style.as_str() {
                                        "block" => {
                                            let cw = self.char_width().min(max_code_x.saturating_sub(cx));
                                            fb.fill_rect_alpha(cx, text_y, cw, self.char_height(), self.colors.cursor, 140);
                                        }
                                        "underline" => {
                                            let cw = self.char_width().min(max_code_x.saturating_sub(cx));
                                            fb.fill_rect(cx, text_y + self.char_height().saturating_sub(2), cw, 2, self.colors.cursor);
                                        }
                                        _ => {
                                            fb.fill_rect(cx, text_y, 2, self.char_height(), self.colors.cursor);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if self.show_line_numbers {
                    fb.draw_char('~', pane_x + self.char_width(), text_y, self.colors.gutter_fg, None);
                }
            }
        } else {
            let code_width = max_code_x.saturating_sub(text_start_x);
            let max_cols = (code_width / self.char_width()).max(10);
            let mut screen_row = 0;
            let mut line_idx = doc.scroll_row;
            let mut start_sub_idx = doc.scroll_subline;

            while screen_row < visible_lines && line_idx < total_lines {
                let line_cow = doc.buffer.line_cow(line_idx).unwrap_or(std::borrow::Cow::Borrowed(""));
                let line_text: &str = &line_cow;
                let sublines = crate::editor::wrap::WordWrap::compute_sublines(line_text, max_cols);

                if start_sub_idx >= sublines.len() {
                    start_sub_idx = 0;
                }

                let tokens = SyntaxHighlighter::tokenize_line(line_text, doc.language);

                for sub_idx in start_sub_idx..sublines.len() {
                    if screen_row >= visible_lines {
                        break;
                    }
                    let sub = &sublines[sub_idx];
                    let py = content_y + screen_row * line_height;
                    let text_y = py + self.line_spacing / 2;

                    // Gutter
                    if self.show_line_numbers {
                        if !sub.is_continuation {
                            let diag = self.diagnostics.as_ref().and_then(|dc| dc.for_line(line_idx + 1));
                            let (fg_col, badge) = if let Some(d) = diag {
                                (d.severity.color_hex(), Some(d.severity.badge()))
                            } else {
                                (self.colors.gutter_fg, None)
                            };

                            let num_str = format!("{:>width$}", line_idx + 1, width = gutter_cols);
                            fb.draw_text(&num_str, pane_x + self.char_width(), text_y, fg_col, None);

                            if let Some(b) = badge {
                                fb.draw_text(b, pane_x + 2, text_y, fg_col, None);
                            }

                            if self.git_gutter_enabled {
                                let diff_opt = if is_active {
                                    self.git_diff.as_ref()
                                } else {
                                    self.secondary_git_diff.as_ref().or(self.git_diff.as_ref())
                                };
                                if let Some(diff) = diff_opt {
                                    let bar_x = pane_x + gutter_w - 3;
                                    match diff.status_for_line(line_idx) {
                                        crate::editor::git::GitLineStatus::Added => {
                                            fb.fill_rect(bar_x, py, 3, line_height, 0x009ECE6A);
                                        }
                                        crate::editor::git::GitLineStatus::Modified => {
                                            fb.fill_rect(bar_x, py, 3, line_height, 0x007AA2F7);
                                        }
                                        crate::editor::git::GitLineStatus::DeletedAbove => {
                                            fb.fill_rect(bar_x, py, 3, 2, 0x00F7768E);
                                        }
                                        crate::editor::git::GitLineStatus::None => {}
                                    }
                                }
                            }
                        } else {
                            let wrap_glyph_x = pane_x + gutter_w.saturating_sub(self.char_width() * 2);
                            fb.draw_char('↳', wrap_glyph_x, text_y, self.colors.gutter_fg, None);
                        }
                    }

                    // Selections
                    for &(start, end) in &selections {
                        if line_idx >= start.line && line_idx <= end.line {
                            let sel_col_start = if line_idx == start.line { start.col } else { 0 };
                            let sel_col_end = if line_idx == end.line { end.col } else { line_text.chars().count() + 1 };

                            let overlap_start = sel_col_start.max(sub.char_start);
                            let overlap_end = sel_col_end.min(sub.char_end);
                            if overlap_end > overlap_start {
                                let sx = text_start_x + (overlap_start - sub.char_start) * self.char_width();
                                if sx < max_code_x {
                                    let sw = ((overlap_end - overlap_start) * self.char_width()).min(max_code_x - sx);
                                    fb.fill_rect(sx, py, sw, line_height, self.colors.selection_bg);
                                }
                            }
                        }
                    }

                    // Subline Tokens
                    let mut sub_tokens = Vec::new();
                    let mut tok_byte_offset = 0;
                    for tok in &tokens {
                        let tok_len = tok.text.len();
                        let tok_end = tok_byte_offset + tok_len;
                        if tok_end > sub.byte_start && tok_byte_offset < sub.byte_end {
                            let slice_start = sub.byte_start.saturating_sub(tok_byte_offset);
                            let slice_end = (sub.byte_end - tok_byte_offset).min(tok_len);
                            if slice_end > slice_start {
                                sub_tokens.push(crate::core::syntax::TokenSpan {
                                    text: &tok.text[slice_start..slice_end],
                                    token_type: tok.token_type,
                                });
                            }
                        }
                        tok_byte_offset = tok_end;
                    }

                    self.render_token_line(fb, &sub_tokens, text_start_x, text_y, max_code_x, 0, self.colors.background);

                    // Cursors
                    if is_active {
                        for pos in &cursor_positions {
                            if pos.line == line_idx {
                                let is_in_sub = if sub_idx + 1 == sublines.len() {
                                    pos.col >= sub.char_start && pos.col <= sub.char_end
                                } else {
                                    pos.col >= sub.char_start && pos.col < sub.char_end
                                };
                                if is_in_sub {
                                    let cx = text_start_x + (pos.col - sub.char_start) * self.char_width();
                                    if cx + 2 <= max_code_x {
                                        match self.cursor_style.as_str() {
                                            "block" => {
                                                let cw = self.char_width().min(max_code_x.saturating_sub(cx));
                                                fb.fill_rect_alpha(cx, text_y, cw, self.char_height(), self.colors.cursor, 140);
                                            }
                                            "underline" => {
                                                let cw = self.char_width().min(max_code_x.saturating_sub(cx));
                                                fb.fill_rect(cx, text_y + self.char_height().saturating_sub(2), cw, 2, self.colors.cursor);
                                            }
                                            _ => {
                                                fb.fill_rect(cx, text_y, 2, self.char_height(), self.colors.cursor);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    screen_row += 1;
                }

                line_idx += 1;
                start_sub_idx = 0;
            }

            while screen_row < visible_lines {
                let py = content_y + screen_row * line_height;
                let text_y = py + self.line_spacing / 2;
                if self.show_line_numbers {
                    fb.draw_char('~', pane_x + self.char_width(), text_y, self.colors.gutter_fg, None);
                }
                screen_row += 1;
            }
        }

        // 6. Minimap
        if let Some(mg) = mini_geom {
            self.render_minimap(fb, doc, &mg, minimap_hovered, minimap_dragging);
        }

        // 7. Scrollbar
        self.render_scrollbar(fb, &scroll_geom, scrollbar_hovered, scrollbar_dragging);

        // 8. Focus border
        if is_active {
            fb.draw_rect_outline(pane_x, pane_y, pane_w, pane_h, self.colors.accent);
        }
    }

    /// Renders a complete editor frame into the software framebuffer with split view support.
    pub fn render_full_frame_with_split(
        &self,
        fb: &mut Framebuffer,
        doc: &Document,
        secondary_doc: Option<&Document>,
        split: Option<&crate::editor::split::SplitState>,
        tab_titles: &[String],
        active_tab_idx: usize,
        status_msg: Option<&str>,
        modal: Option<&crate::ui::prompt::PromptModal>,
        menu_bar: Option<&crate::editor::menu::MenuBar>,
        sidebar: Option<&crate::editor::sidebar::FileExplorer>,
        scrollbar_hovered: bool,
        scrollbar_dragging: bool,
        minimap_hovered: bool,
        minimap_dragging: bool,
        autocomplete: Option<&crate::editor::autocomplete::AutocompleteState>,
    ) {
        // Clear background
        fb.clear(self.colors.background);
        fb.subpixel_antialiasing = self.subpixel_antialiasing;
        fb.letter_spacing = self.letter_spacing;
        fb.font_aspect_ratio = self.font_aspect_ratio;
        fb.stem_darkening = self.stem_darkening;
        fb.stem_darkening_amount = self.stem_darkening_amount;
        fb.font_family = self.font_family;
        fb.font_weight = self.font_weight;
        fb.font_size = self.font_size;
        fb.font_scale = self.font_scale;

        let menu_offset = if menu_bar.is_some() { self.menu_bar_height } else { 0 };

        // 1. Draw Menu Bar (if present)
        if let Some(mb) = menu_bar {
            self.render_menu_bar(fb, mb);
        }

        // 2. Determine Sidebar Width & Draw Sidebar
        let sidebar_w = sidebar
            .filter(|sb| sb.is_visible)
            .map(|sb| sb.width.min(fb.width.saturating_sub(100)))
            .unwrap_or(0);

        let content_y_start = menu_offset + self.tab_bar_height;
        let content_y_end = fb.height.saturating_sub(self.status_bar_height + self.terminal_inset);
        let content_height = content_y_end.saturating_sub(content_y_start);

        if let Some(sb) = sidebar {
            if sb.is_visible && sidebar_w > 0 {
                let sb_h = fb.height.saturating_sub(menu_offset + self.status_bar_height);
                self.render_sidebar(fb, sb, menu_offset, sb_h, doc.buffer.file_path());
            }
        }

        let is_split_active = split.map(|s| s.is_split()).unwrap_or(false);

        if !is_split_active {
            // SINGLE PANE MODE
            // 3. Draw Tab Bar next to sidebar
            self.render_tab_bar(fb, tab_titles, active_tab_idx, sidebar_w, menu_offset);

            // 4. Compute Viewport geometry
            let total_lines = doc.buffer.len_lines();
            let gutter_cols = total_lines.to_string().len().max(3);
            let gutter_width = if self.show_line_numbers {
                (gutter_cols + 2) * self.char_width()
            } else {
                self.scale_coord(8)
            };

            let line_height = self.line_height();
            let visible_lines = content_height / line_height;

            let canvas_w = fb.width.saturating_sub(sidebar_w);
            let minimap_w = if self.minimap_enabled && canvas_w >= self.scale_coord(320) {
                self.minimap_width.min(canvas_w / 4)
            } else {
                0
            };
            let max_code_x = fb.width.saturating_sub(self.scrollbar_width + minimap_w);
            let canvas_x_start = sidebar_w;

            // 5. Draw Gutter background & divider
            if self.show_line_numbers {
                fb.fill_rect(canvas_x_start, content_y_start, gutter_width, content_height, self.colors.gutter_bg);
                fb.fill_rect(canvas_x_start + gutter_width - 1, content_y_start, 1, content_height, self.colors.gutter_border);
            }

            // 6. Collect selections & cursor pixel positions
            let mut selections = Vec::new();
            let mut cursor_positions = Vec::new();

            for c in doc.cursors.cursors() {
                let pos = doc.buffer.char_to_position(c.head);
                cursor_positions.push(pos);

                if c.has_selection() {
                    let range = c.selection_range();
                    let start = doc.buffer.char_to_position(range.start);
                    let end = doc.buffer.char_to_position(range.end);
                    selections.push((start, end));
                }
            }

            // 7. Draw Document Lines & Syntax
            let text_start_x = if self.show_line_numbers {
                canvas_x_start + gutter_width + self.char_width()
            } else {
                canvas_x_start + self.scale_coord(8)
            };

            if !self.word_wrap {
                for row in 0..visible_lines {
                    let line_idx = doc.scroll_row + row;
                    let py = content_y_start + row * line_height;
                    let text_y = py + self.line_spacing / 2;

                    if line_idx < total_lines {
                        if self.show_line_numbers {
                            let diag = self.diagnostics.as_ref().and_then(|dc| dc.for_line(line_idx + 1));
                            let (fg_col, badge) = if let Some(d) = diag {
                                (d.severity.color_hex(), Some(d.severity.badge()))
                            } else {
                                (self.colors.gutter_fg, None)
                            };

                            let num_str = format!("{:>width$}", line_idx + 1, width = gutter_cols);
                            fb.draw_text(&num_str, canvas_x_start + self.char_width(), text_y, fg_col, None);

                            if let Some(b) = badge {
                                fb.draw_text(b, canvas_x_start + 2, text_y, fg_col, None);
                            }

                            // Git Gutter diff indicator bar
                            if self.git_gutter_enabled {
                                if let Some(diff) = &self.git_diff {
                                    let bar_x = canvas_x_start + gutter_width - 3;
                                    match diff.status_for_line(line_idx) {
                                        crate::editor::git::GitLineStatus::Added => {
                                            fb.fill_rect(bar_x, py, 3, line_height, 0x009ECE6A);
                                        }
                                        crate::editor::git::GitLineStatus::Modified => {
                                            fb.fill_rect(bar_x, py, 3, line_height, 0x007AA2F7);
                                        }
                                        crate::editor::git::GitLineStatus::DeletedAbove => {
                                            fb.fill_rect(bar_x, py, 3, 2, 0x00F7768E);
                                        }
                                        crate::editor::git::GitLineStatus::None => {}
                                    }
                                }
                            }
                        }

                        let line_text = doc.buffer.line(line_idx).unwrap_or_default();

                        for &(start, end) in &selections {
                            if line_idx >= start.line && line_idx <= end.line {
                                let sel_col_start = if line_idx == start.line { start.col } else { 0 };
                                let sel_col_end = if line_idx == end.line {
                                    end.col
                                } else {
                                    line_text.chars().count() + 1
                                };

                                if sel_col_end > sel_col_start {
                                    let sx = text_start_x + sel_col_start.saturating_sub(doc.scroll_col) * self.char_width();
                                    if sx < max_code_x {
                                        let sw = ((sel_col_end - sel_col_start) * self.char_width()).min(max_code_x - sx);
                                        fb.fill_rect(sx, py, sw, line_height, self.colors.selection_bg);
                                    }
                                }
                            }
                        }

                        let tokens = SyntaxHighlighter::tokenize_line(&line_text, doc.language);
                        self.render_token_line(fb, &tokens, text_start_x, text_y, max_code_x, doc.scroll_col, self.colors.background);

                        for pos in &cursor_positions {
                            if pos.line == line_idx && pos.col >= doc.scroll_col {
                                let cx = text_start_x + (pos.col - doc.scroll_col) * self.char_width();
                                if cx + 2 <= max_code_x {
                                    match self.cursor_style.as_str() {
                                        "block" => {
                                            let cw = self.char_width().min(max_code_x.saturating_sub(cx));
                                            fb.fill_rect_alpha(cx, text_y, cw, self.char_height(), self.colors.cursor, 140);
                                        }
                                        "underline" => {
                                            let cw = self.char_width().min(max_code_x.saturating_sub(cx));
                                            fb.fill_rect(cx, text_y + self.char_height().saturating_sub(2), cw, 2, self.colors.cursor);
                                        }
                                        _ => {
                                            fb.fill_rect(cx, text_y, 2, self.char_height(), self.colors.cursor);
                                        }
                                    }
                                }
                            }
                        }
                    } else if self.show_line_numbers {
                        fb.draw_char('~', canvas_x_start + self.char_width(), text_y, self.colors.gutter_fg, None);
                    }
                }
            } else {
                let code_width = max_code_x.saturating_sub(text_start_x);
                let max_cols = (code_width / self.char_width()).max(10);
                let mut screen_row = 0;
                let mut line_idx = doc.scroll_row;
                let mut start_sub_idx = doc.scroll_subline;

                while screen_row < visible_lines && line_idx < total_lines {
                    let line_cow = doc.buffer.line_cow(line_idx).unwrap_or(std::borrow::Cow::Borrowed(""));
                    let line_text: &str = &line_cow;
                    let sublines = crate::editor::wrap::WordWrap::compute_sublines(line_text, max_cols);

                    if start_sub_idx >= sublines.len() {
                        start_sub_idx = 0;
                    }

                    let tokens = SyntaxHighlighter::tokenize_line(line_text, doc.language);

                    for sub_idx in start_sub_idx..sublines.len() {
                        if screen_row >= visible_lines {
                            break;
                        }
                        let sub = &sublines[sub_idx];
                        let py = content_y_start + screen_row * line_height;
                        let text_y = py + self.line_spacing / 2;

                        // Gutter
                        if self.show_line_numbers {
                            if !sub.is_continuation {
                                let diag = self.diagnostics.as_ref().and_then(|dc| dc.for_line(line_idx + 1));
                                let (fg_col, badge) = if let Some(d) = diag {
                                    (d.severity.color_hex(), Some(d.severity.badge()))
                                } else {
                                    (self.colors.gutter_fg, None)
                                };

                                let num_str = format!("{:>width$}", line_idx + 1, width = gutter_cols);
                                fb.draw_text(&num_str, canvas_x_start + self.char_width(), text_y, fg_col, None);

                                if let Some(b) = badge {
                                    fb.draw_text(b, canvas_x_start + 2, text_y, fg_col, None);
                                }

                                if self.git_gutter_enabled {
                                    if let Some(diff) = &self.git_diff {
                                        let bar_x = canvas_x_start + gutter_width - 3;
                                        match diff.status_for_line(line_idx) {
                                            crate::editor::git::GitLineStatus::Added => {
                                                fb.fill_rect(bar_x, py, 3, line_height, 0x009ECE6A);
                                            }
                                            crate::editor::git::GitLineStatus::Modified => {
                                                fb.fill_rect(bar_x, py, 3, line_height, 0x007AA2F7);
                                            }
                                            crate::editor::git::GitLineStatus::DeletedAbove => {
                                                fb.fill_rect(bar_x, py, 3, 2, 0x00F7768E);
                                            }
                                            crate::editor::git::GitLineStatus::None => {}
                                        }
                                    }
                                }
                            } else {
                                let wrap_glyph_x = canvas_x_start + gutter_width.saturating_sub(self.char_width() * 2);
                                fb.draw_char('↳', wrap_glyph_x, text_y, self.colors.gutter_fg, None);
                            }
                        }

                        // Selections
                        for &(start, end) in &selections {
                            if line_idx >= start.line && line_idx <= end.line {
                                let sel_col_start = if line_idx == start.line { start.col } else { 0 };
                                let sel_col_end = if line_idx == end.line { end.col } else { line_text.chars().count() + 1 };

                                let overlap_start = sel_col_start.max(sub.char_start);
                                let overlap_end = sel_col_end.min(sub.char_end);
                                if overlap_end > overlap_start {
                                    let sx = text_start_x + (overlap_start - sub.char_start) * self.char_width();
                                    if sx < max_code_x {
                                        let sw = ((overlap_end - overlap_start) * self.char_width()).min(max_code_x - sx);
                                        fb.fill_rect(sx, py, sw, line_height, self.colors.selection_bg);
                                    }
                                }
                            }
                        }

                        // Subline Tokens
                        let mut sub_tokens = Vec::new();
                        let mut tok_byte_offset = 0;
                        for tok in &tokens {
                            let tok_len = tok.text.len();
                            let tok_end = tok_byte_offset + tok_len;
                            if tok_end > sub.byte_start && tok_byte_offset < sub.byte_end {
                                let slice_start = sub.byte_start.saturating_sub(tok_byte_offset);
                                let slice_end = (sub.byte_end - tok_byte_offset).min(tok_len);
                                if slice_end > slice_start {
                                    sub_tokens.push(crate::core::syntax::TokenSpan {
                                        text: &tok.text[slice_start..slice_end],
                                        token_type: tok.token_type,
                                    });
                                }
                            }
                            tok_byte_offset = tok_end;
                        }

                        self.render_token_line(fb, &sub_tokens, text_start_x, text_y, max_code_x, 0, self.colors.background);

                        // Cursors
                        for pos in &cursor_positions {
                            if pos.line == line_idx {
                                let is_in_sub = if sub_idx + 1 == sublines.len() {
                                    pos.col >= sub.char_start && pos.col <= sub.char_end
                                } else {
                                    pos.col >= sub.char_start && pos.col < sub.char_end
                                };
                                if is_in_sub {
                                    let cx = text_start_x + (pos.col - sub.char_start) * self.char_width();
                                    if cx + 2 <= max_code_x {
                                        match self.cursor_style.as_str() {
                                            "block" => {
                                                let cw = self.char_width().min(max_code_x.saturating_sub(cx));
                                                fb.fill_rect_alpha(cx, text_y, cw, self.char_height(), self.colors.cursor, 140);
                                            }
                                            "underline" => {
                                                let cw = self.char_width().min(max_code_x.saturating_sub(cx));
                                                fb.fill_rect(cx, text_y + self.char_height().saturating_sub(2), cw, 2, self.colors.cursor);
                                            }
                                            _ => {
                                                fb.fill_rect(cx, text_y, 2, self.char_height(), self.colors.cursor);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        screen_row += 1;
                    }

                    line_idx += 1;
                    start_sub_idx = 0;
                }

                while screen_row < visible_lines {
                    let py = content_y_start + screen_row * line_height;
                    let text_y = py + self.line_spacing / 2;
                    if self.show_line_numbers {
                        fb.draw_char('~', canvas_x_start + self.char_width(), text_y, self.colors.gutter_fg, None);
                    }
                    screen_row += 1;
                }
            }

            // 8a. Draw Minimap if enabled
            if minimap_w > 0 {
                let mm_x = max_code_x;
                let mm_geom = self.minimap_geometry_in_rect(mm_x, content_y_start, minimap_w, content_height, doc);
                self.render_minimap(fb, doc, &mm_geom, minimap_hovered, minimap_dragging);
            }

            // 8b. Draw Interactive Scrollbar
            let scroll_geom = self.scrollbar_geometry(fb.width, fb.height, doc, menu_bar.is_some());
            self.render_scrollbar(fb, &scroll_geom, scrollbar_hovered, scrollbar_dragging);
        } else {
            // SPLIT PANE MODE (Vertical or Horizontal)
            let sp = split.unwrap();
            let (p0_geom, p1_geom, splitter_opt) = self.pane_geometries_with_inset(fb.width, fb.height, sidebar_w, menu_bar.is_some(), sp, self.terminal_inset);

            let pane_0_doc = doc;
            let pane_1_doc = secondary_doc.unwrap_or(doc);

            let p0_title = format!("1: {}", pane_0_doc.tab_title());
            let p1_title = format!("2: {}", pane_1_doc.tab_title());

            // Render Pane 0
            self.render_editor_pane(
                fb,
                pane_0_doc,
                p0_geom.x,
                p0_geom.y,
                p0_geom.width,
                p0_geom.height,
                sp.active_pane == 0,
                &p0_title,
                scrollbar_hovered && sp.active_pane == 0,
                scrollbar_dragging && sp.active_pane == 0,
                minimap_hovered && sp.active_pane == 0,
                minimap_dragging && sp.active_pane == 0,
            );

            // Render Splitter
            if let Some((sx, sy, sw, sh)) = splitter_opt {
                fb.fill_rect(sx, sy, sw, sh, self.colors.gutter_border);
                if sw > sh {
                    // Horizontal splitter
                    let mid_x = sx + sw / 2;
                    fb.fill_rect(mid_x.saturating_sub(16), sy + 1, 32, 2, self.colors.accent);
                } else {
                    // Vertical splitter
                    let mid_y = sy + sh / 2;
                    fb.fill_rect(sx + 1, mid_y.saturating_sub(16), 2, 32, self.colors.accent);
                }
            }

            // Render Pane 1
            self.render_editor_pane(
                fb,
                pane_1_doc,
                p1_geom.x,
                p1_geom.y,
                p1_geom.width,
                p1_geom.height,
                sp.active_pane == 1,
                &p1_title,
                scrollbar_hovered && sp.active_pane == 1,
                scrollbar_dragging && sp.active_pane == 1,
                minimap_hovered && sp.active_pane == 1,
                minimap_dragging && sp.active_pane == 1,
            );
        }

        // 9. Draw Status Bar at bottom
        self.render_status_bar(fb, doc, status_msg);

        // 10. Draw Floating Autocomplete Popup if active
        if let Some(ac) = autocomplete {
            if ac.is_active && !ac.items.is_empty() {
                let ac_geom = self.autocomplete_geometry(
                    fb.width,
                    fb.height,
                    doc,
                    sidebar_w,
                    menu_bar.is_some(),
                    ac.items.len(),
                );
                self.render_autocomplete_popup(fb, ac, &ac_geom);
            }
        }

        // 11. Draw Prompt Modal overlay if present
        if let Some(m) = modal {
            self.render_prompt_overlay(fb, m);
        }

        // 12. Draw Open Dropdown Menu if present (drawn on top)
        if let Some(mb) = menu_bar {
            self.render_menu_dropdown(fb, mb);
        }
    }

    /// Renders a complete editor frame into the software framebuffer with optional sidebar and scrollbar state.
    pub fn render_full_frame(
        &self,
        fb: &mut Framebuffer,
        doc: &Document,
        tab_titles: &[String],
        active_tab_idx: usize,
        status_msg: Option<&str>,
        modal: Option<&crate::ui::prompt::PromptModal>,
        menu_bar: Option<&crate::editor::menu::MenuBar>,
        sidebar: Option<&crate::editor::sidebar::FileExplorer>,
        scrollbar_hovered: bool,
        scrollbar_dragging: bool,
        autocomplete: Option<&crate::editor::autocomplete::AutocompleteState>,
    ) {
        self.render_full_frame_with_split(
            fb,
            doc,
            None,
            None,
            tab_titles,
            active_tab_idx,
            status_msg,
            modal,
            menu_bar,
            sidebar,
            scrollbar_hovered,
            scrollbar_dragging,
            false,
            false,
            autocomplete,
        );
    }

    /// Renders the file explorer sidebar docked to the left.
    pub fn render_sidebar(
        &self,
        fb: &mut Framebuffer,
        sidebar: &crate::editor::sidebar::FileExplorer,
        offset_y: usize,
        height: usize,
        active_file_path: Option<&std::path::Path>,
    ) {
        let sidebar_w = sidebar.width.min(fb.width.saturating_sub(100));
        if sidebar_w == 0 || height == 0 {
            return;
        }

        let header_h = self.tab_bar_height;
        let body_y = offset_y + header_h;
        let body_h = height.saturating_sub(header_h);

        // 1. Sidebar Container Background
        fb.fill_rect(0, offset_y, sidebar_w, height, self.colors.gutter_bg);

        // 2. Right Vertical Divider Border
        fb.fill_rect(sidebar_w - 1, offset_y, 1, height, self.colors.gutter_border);

        // 3. Header Background
        fb.fill_rect(0, offset_y, sidebar_w - 1, header_h, self.colors.modal_header_bg);
        fb.fill_rect(0, offset_y + header_h - 1, sidebar_w - 1, 1, self.colors.gutter_border);

        // Header Title: EXPLORER / Folder Name
        let title_max_chars = sidebar_w.saturating_sub(self.scale_coord(70)) / self.char_width();
        let raw_name = if sidebar.root_name.is_empty() { "EXPLORER" } else { &sidebar.root_name };
        let display_title = if raw_name.len() > title_max_chars && title_max_chars > 3 {
            format!("{}...", &raw_name[..title_max_chars - 3])
        } else {
            raw_name.to_string()
        };
        let text_y = offset_y + (header_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(&display_title.to_uppercase(), 8, text_y, self.colors.status_bar_fg, None);

        // Header Action Buttons: [+] [↺] [×]
        let btn_w = self.scale_coord(16);
        let close_x = sidebar_w.saturating_sub(btn_w + self.scale_coord(6));
        let refresh_x = close_x.saturating_sub(btn_w + self.scale_coord(4));
        let new_file_x = refresh_x.saturating_sub(btn_w + self.scale_coord(4));

        // [+] New File
        fb.draw_char('+', new_file_x + self.scale_coord(4), text_y, self.colors.tab_inactive_fg, None);

        // [↺] Refresh
        fb.draw_char('↺', refresh_x + self.scale_coord(4), text_y, self.colors.tab_inactive_fg, None);

        // [×] Close Sidebar
        fb.draw_char('×', close_x + self.scale_coord(4), text_y, self.colors.tab_inactive_fg, None);

        // 4. File Tree Rows
        let row_h = (self.char_height() + self.scale_coord(4)).max(self.scale_coord(18));
        let visible_rows = body_h / row_h;

        if sidebar.visible_items.is_empty() {
            fb.draw_text("(vacío)", 16, body_y + 8, self.colors.gutter_fg, None);
            return;
        }

        for (screen_row, item_idx) in (sidebar.scroll_offset..sidebar.visible_items.len()).enumerate() {
            if screen_row >= visible_rows {
                break;
            }
            let item = &sidebar.visible_items[item_idx];
            let row_y = body_y + screen_row * row_h;
            let item_text_y = row_y + (row_h.saturating_sub(self.char_height())) / 2;
            let is_selected = item_idx == sidebar.selected_idx;
            let is_active_file = !item.is_dir && active_file_path.map_or(false, |p| p == item.path);

            // Row Highlight
            if is_selected {
                let sel_bg = if sidebar.is_focused {
                    self.colors.selection_bg
                } else {
                    self.colors.button_hover_bg
                };
                if self.rounded_corners {
                    fb.fill_rounded_rect(4, row_y + 1, sidebar_w.saturating_sub(8), row_h - 2, 3, sel_bg);
                } else {
                    fb.fill_rect(2, row_y, sidebar_w.saturating_sub(4), row_h, sel_bg);
                }
                if sidebar.is_focused {
                    fb.fill_rect(2, row_y + 2, 2, row_h.saturating_sub(4), self.colors.accent);
                }
            } else if is_active_file {
                fb.fill_rect(2, row_y, sidebar_w.saturating_sub(4), row_h, self.colors.button_bg);
                fb.fill_rect(2, row_y + 2, 2, row_h.saturating_sub(4), self.colors.accent);
            }

            // Indentation
            let indent_x = self.scale_coord(8) + item.depth * self.scale_coord(14);

            if item.is_dir {
                // Arrow
                let arrow_char = if item.is_expanded { '▼' } else { '▶' };
                fb.draw_char(arrow_char, indent_x, item_text_y, self.colors.status_bar_fg, None);

                let label_x = indent_x + self.char_width() + self.scale_coord(4);
                let max_chars = (sidebar_w.saturating_sub(label_x + self.scale_coord(8))) / self.char_width();
                let text = if item.name.chars().count() > max_chars && max_chars > 2 {
                    let mut s: String = item.name.chars().take(max_chars.saturating_sub(2)).collect();
                    s.push_str("..");
                    s
                } else {
                    item.name.clone()
                };
                fb.draw_text(&text, label_x, item_text_y, self.colors.tab_active_fg, None);
            } else {
                let color = item.color_hint(&self.colors);
                let label_x = indent_x + self.char_width() + self.scale_coord(4);
                let max_chars = (sidebar_w.saturating_sub(label_x + self.scale_coord(20))) / self.char_width();
                let text = if item.name.chars().count() > max_chars && max_chars > 2 {
                    let mut s: String = item.name.chars().take(max_chars.saturating_sub(2)).collect();
                    s.push_str("..");
                    s
                } else {
                    item.name.clone()
                };
                fb.draw_char('•', indent_x, item_text_y, color, None);
                fb.draw_text(&text, label_x, item_text_y, color, None);

                // Git status badge ('M', 'U', 'A', 'D', 'R')
                if let Some(statuses) = &self.git_statuses {
                    let canon = item.path.canonicalize().unwrap_or_else(|_| item.path.clone());
                    if let Some(status) = statuses.get(&canon) {
                        let badge_x = sidebar_w.saturating_sub(16);
                        fb.draw_char(status.badge_char(), badge_x, item_text_y, status.color(), None);
                    }
                }
            }
        }
    }

    fn render_menu_bar(&self, fb: &mut Framebuffer, menu_bar: &crate::editor::menu::MenuBar) {
        fb.fill_rect(0, 0, fb.width, self.menu_bar_height, self.colors.modal_header_bg);

        let pad = (16.0 * self.scale_factor).round() as usize;
        let mut cur_x = (8.0 * self.scale_factor).round() as usize;
        let text_y = (self.menu_bar_height.saturating_sub(self.char_height())) / 2;

        for (idx, cat) in menu_bar.categories.iter().enumerate() {
            let is_active = menu_bar.active_category == Some(idx);
            let item_w = cat.name.chars().count() * self.char_width() + pad;

            let (bg, fg) = if is_active {
                (self.colors.accent, 0x00FFFFFF)
            } else {
                (self.colors.modal_header_bg, 0x00CCCCCC)
            };

            if is_active {
                if self.rounded_corners {
                    fb.fill_rounded_rect(cur_x, 2, item_w, self.menu_bar_height - 4, 3, bg);
                } else {
                    fb.fill_rect(cur_x, 0, item_w, self.menu_bar_height, bg);
                }
            } else {
                fb.fill_rect(cur_x, 0, item_w, self.menu_bar_height, bg);
            }
            fb.draw_text(cat.name, cur_x + (8.0 * self.scale_factor).round() as usize, text_y, fg, None);

            cur_x += item_w;
        }

        // Sleek subtle bottom divider under menu bar
        fb.fill_rect(0, self.menu_bar_height - 1, fb.width, 1, self.colors.scrollbar_border);
    }

    fn render_menu_dropdown(&self, fb: &mut Framebuffer, menu_bar: &crate::editor::menu::MenuBar) {
        if let Some(cat_idx) = menu_bar.active_category {
            let cat = &menu_bar.categories[cat_idx];
            let (drop_x, drop_y, drop_w, drop_h) = menu_bar.dropdown_rect(cat_idx, self.char_width(), fb.width);
            let item_h = self.scale_coord(22);

            // 1. Soft Drop shadow
            if self.shadows {
                self.render_shadow(fb, drop_x, drop_y, drop_w, drop_h, 8, 6);
            } else {
                fb.fill_rect(drop_x + 4, drop_y + 4, drop_w, drop_h, 0x00101010);
            }

            // 2. Container background & border
            if self.rounded_corners {
                fb.fill_rounded_rect(drop_x, drop_y, drop_w, drop_h, 5, self.colors.modal_bg);
                fb.draw_rounded_outline(drop_x, drop_y, drop_w, drop_h, 5, self.colors.modal_border);
            } else {
                fb.fill_rect(drop_x.saturating_sub(1), drop_y.saturating_sub(1), drop_w + 2, drop_h + 2, self.colors.modal_border);
                fb.fill_rect(drop_x, drop_y, drop_w, drop_h, self.colors.modal_bg);
            }

            for (item_idx, item) in cat.items.iter().enumerate() {
                let iy = drop_y + 2 + item_idx * item_h;

                if item.is_separator {
                    // Separator line
                    fb.fill_rect(drop_x + 8, iy + item_h / 2, drop_w.saturating_sub(16), 1, self.colors.scrollbar_border);
                } else {
                    let is_hovered = menu_bar.hovered_item == Some(item_idx);
                    if is_hovered {
                        if self.rounded_corners {
                            fb.fill_rounded_rect(drop_x + 4, iy + 1, drop_w - 8, item_h - 2, 4, self.colors.accent);
                        } else {
                            fb.fill_rect(drop_x + 2, iy, drop_w - 4, item_h, self.colors.accent);
                        }
                    }

                    let (text_color, shortcut_color) = if is_hovered {
                        (0x00FFFFFF, 0x00E0E0E0)
                    } else {
                        (0x00D4D4D4, 0x00858585)
                    };

                    let text_y = iy + (item_h.saturating_sub(self.char_height())) / 2;

                    // Draw Shortcut right-aligned
                    let has_shortcut = !item.shortcut.is_empty();
                    let sc_w = if has_shortcut {
                        item.shortcut.chars().count() * self.char_width()
                    } else {
                        0
                    };
                    let sc_x = drop_x + drop_w - sc_w - self.scale_coord(12);

                    // Draw Label with clipping before shortcut to guarantee no collision or overflow
                    let label_max_x = if has_shortcut {
                        sc_x.saturating_sub(self.scale_coord(10))
                    } else {
                        drop_x + drop_w - self.scale_coord(12)
                    };
                    fb.draw_text_clipped(item.label, drop_x + self.scale_coord(12), text_y, label_max_x, text_color, None);

                    if has_shortcut {
                        fb.draw_text_clipped(item.shortcut, sc_x, text_y, drop_x + drop_w - self.scale_coord(6), shortcut_color, None);
                    }
                }
            }
        }
    }

    fn render_tab_bar(
        &self,
        fb: &mut Framebuffer,
        tab_titles: &[String],
        active_tab_idx: usize,
        offset_x: usize,
        offset_y: usize,
    ) {
        let bar_w = fb.width.saturating_sub(offset_x);
        fb.fill_rect(offset_x, offset_y, bar_w, self.tab_bar_height, self.colors.tab_bar_bg);

        let mut cur_x = offset_x + self.scale_coord(8);
        let text_y = offset_y + (self.tab_bar_height.saturating_sub(self.char_height())) / 2;

        for (idx, title) in tab_titles.iter().enumerate() {
            let is_active = idx == active_tab_idx;
            let tab_text = format!(" {} {} × ", idx + 1, title);
            let tab_w = tab_text.chars().count() * self.char_width() + self.scale_coord(8);

            let (bg, fg) = if is_active {
                (self.colors.tab_active_bg, self.colors.tab_active_fg)
            } else {
                (self.colors.tab_inactive_bg, self.colors.tab_inactive_fg)
            };

            if self.rounded_corners {
                fb.fill_rounded_rect(cur_x, offset_y + 2, tab_w, self.tab_bar_height - 2, 4, bg);
            } else {
                fb.fill_rect(cur_x, offset_y + 2, tab_w, self.tab_bar_height - 2, bg);
            }
            fb.draw_text(&tab_text, cur_x + 4, text_y, fg, None);

            // Active tab indicator accent on top
            if is_active {
                if self.rounded_corners {
                    fb.fill_rounded_rect(cur_x + 2, offset_y, tab_w.saturating_sub(4), 2, 1, self.colors.accent);
                } else {
                    fb.fill_rect(cur_x, offset_y, tab_w, 2, self.colors.cursor);
                }
            }

            cur_x += tab_w + self.scale_coord(3);
        }

        // New Tab [+] Button
        let plus_w = self.scale_coord(22);
        let plus_y = offset_y + 3;
        let plus_h = self.tab_bar_height.saturating_sub(6);
        if self.rounded_corners {
            fb.fill_rounded_rect(cur_x, plus_y, plus_w, plus_h, 3, self.colors.tab_inactive_bg);
        } else {
            fb.fill_rect(cur_x, offset_y + 2, plus_w, self.tab_bar_height - 2, self.colors.tab_inactive_bg);
        }
        fb.draw_text("+", cur_x + self.scale_coord(7), text_y, self.colors.tab_inactive_fg, None);
    }

    fn render_status_bar(&self, fb: &mut Framebuffer, doc: &Document, status_msg: Option<&str>) {
        let bar_y = fb.height.saturating_sub(self.status_bar_height);
        fb.fill_rect(0, bar_y, fb.width, self.status_bar_height, self.colors.status_bar_bg);

        let chip_h = self.status_bar_height.saturating_sub(self.scale_coord(4)).max(self.scale_coord(14));
        let chip_y = bar_y + 2;
        let text_y = bar_y + (self.status_bar_height.saturating_sub(self.char_height())) / 2;

        let mut left_x = self.scale_coord(6);

        // 1. Mode Chip: [ EDIT ]
        let mode_label = " EDIT ";
        let mode_w = mode_label.chars().count() * self.char_width() + self.scale_coord(6);
        if self.rounded_corners {
            fb.fill_rounded_rect(left_x, chip_y, mode_w, chip_h, 3, self.colors.chip_bg);
        } else {
            fb.fill_rect(left_x, chip_y, mode_w, chip_h, self.colors.chip_bg);
        }
        fb.draw_text(mode_label, left_x + 3, text_y, self.colors.chip_fg, None);
        left_x += mode_w + self.scale_coord(6);

        // 2. Status message or File & Diagnostics
        if let Some(msg) = status_msg {
            let msg_w = msg.chars().count() * self.char_width() + self.scale_coord(8);
            if self.rounded_corners {
                fb.fill_rounded_rect(left_x, chip_y, msg_w, chip_h, 3, self.colors.chip_bg);
            } else {
                fb.fill_rect(left_x, chip_y, msg_w, chip_h, self.colors.chip_bg);
            }
            fb.draw_text(msg, left_x + 4, text_y, self.colors.status_bar_fg, None);
        } else {
            // File name chip
            let file_name = doc
                .buffer
                .file_path()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("[Sin Título]");
            let dirty = if doc.buffer.is_dirty() { " •" } else { "" };
            let file_text = format!(" {}{} ", file_name, dirty);
            let file_w = file_text.chars().count() * self.char_width() + self.scale_coord(4);
            if self.rounded_corners {
                fb.fill_rounded_rect(left_x, chip_y, file_w, chip_h, 3, self.colors.chip_bg);
            } else {
                fb.fill_rect(left_x, chip_y, file_w, chip_h, self.colors.chip_bg);
            }
            fb.draw_text(&file_text, left_x + 2, text_y, self.colors.status_bar_fg, None);
            left_x += file_w + self.scale_coord(6);

            // Diagnostics summary badge
            if let Some(dc) = &self.diagnostics {
                let err_count = dc.error_count();
                let warn_count = dc.warning_count();
                let (diag_text, diag_fg) = if err_count > 0 || warn_count > 0 {
                    (format!(" ● {}  ▲ {} ", err_count, warn_count), 0x00FF7B72)
                } else {
                    (" ✓ 0 errores ".to_string(), 0x007EE787)
                };
                let diag_w = diag_text.chars().count() * self.char_width() + self.scale_coord(4);
                if self.rounded_corners {
                    fb.fill_rounded_rect(left_x, chip_y, diag_w, chip_h, 3, self.colors.chip_bg);
                } else {
                    fb.fill_rect(left_x, chip_y, diag_w, chip_h, self.colors.chip_bg);
                }
                fb.draw_text(&diag_text, left_x + 2, text_y, diag_fg, None);
                left_x += diag_w + self.scale_coord(6);
            }

            // Git Branch & Stats Chip
            if let Some(branch) = &self.git_branch {
                let git_text = if let Some(diff) = &self.git_diff {
                    if !diff.is_empty() {
                        let mut parts = Vec::new();
                        if diff.added_count > 0 {
                            parts.push(format!("+{}", diff.added_count));
                        }
                        if diff.modified_count > 0 {
                            parts.push(format!("~{}", diff.modified_count));
                        }
                        if diff.deleted_count > 0 {
                            parts.push(format!("-{}", diff.deleted_count));
                        }
                        format!(" git: {} ({}) ", branch, parts.join(" "))
                    } else {
                        format!(" git: {} ", branch)
                    }
                } else {
                    format!(" git: {} ", branch)
                };
                let git_w = git_text.chars().count() * self.char_width() + self.scale_coord(4);
                if self.rounded_corners {
                    fb.fill_rounded_rect(left_x, chip_y, git_w, chip_h, 3, self.colors.chip_bg);
                } else {
                    fb.fill_rect(left_x, chip_y, git_w, chip_h, self.colors.chip_bg);
                }
                fb.draw_text(&git_text, left_x + 2, text_y, self.colors.syntax_keyword, None);
            }
        }

        // Right side chips: Language, UTF-8, Cursor Position
        let primary_pos = doc.buffer.char_to_position(doc.cursors.primary().head);
        let cursor_count = doc.cursors.cursors().len();
        let cursors_tag = if cursor_count > 1 {
            format!(" [{} cursors]", cursor_count)
        } else {
            String::new()
        };

        let lang_text = format!(" {} ", doc.language.name());
        let lang_w = lang_text.chars().count() * self.char_width() + self.scale_coord(4);

        let utf8_text = " UTF-8 ";
        let utf8_w = utf8_text.chars().count() * self.char_width() + self.scale_coord(4);

        let mut right_x = fb.width.saturating_sub(self.scale_coord(6));

        // Wrap Chip
        if self.word_wrap {
            let wrap_text = " Wrap ";
            let wrap_w = wrap_text.chars().count() * self.char_width() + self.scale_coord(4);
            right_x = right_x.saturating_sub(wrap_w);
            if self.rounded_corners {
                fb.fill_rounded_rect(right_x, chip_y, wrap_w, chip_h, 3, self.colors.chip_bg);
            } else {
                fb.fill_rect(right_x, chip_y, wrap_w, chip_h, self.colors.chip_bg);
            }
            fb.draw_text(wrap_text, right_x + 2, text_y, self.colors.accent, None);
            right_x = right_x.saturating_sub(self.scale_coord(6));
        }

        // Language Chip
        right_x = right_x.saturating_sub(lang_w);
        if self.rounded_corners {
            fb.fill_rounded_rect(right_x, chip_y, lang_w, chip_h, 3, self.colors.chip_bg);
        } else {
            fb.fill_rect(right_x, chip_y, lang_w, chip_h, self.colors.chip_bg);
        }
        fb.draw_text(&lang_text, right_x + 2, text_y, self.colors.chip_fg, None);
        right_x = right_x.saturating_sub(self.scale_coord(6));

        // UTF-8 Chip
        right_x = right_x.saturating_sub(utf8_w);
        if self.rounded_corners {
            fb.fill_rounded_rect(right_x, chip_y, utf8_w, chip_h, 3, self.colors.chip_bg);
        } else {
            fb.fill_rect(right_x, chip_y, utf8_w, chip_h, self.colors.chip_bg);
        }
        fb.draw_text(utf8_text, right_x + 2, text_y, self.colors.status_bar_fg, None);
        right_x = right_x.saturating_sub(self.scale_coord(6));

        // Position & Lines Chip
        let pos_full = format!(" Ln {}, Col {} ({} lin){} ", primary_pos.line + 1, primary_pos.col + 1, doc.buffer.len_lines(), cursors_tag);
        let pos_full_w = pos_full.chars().count() * self.char_width() + self.scale_coord(4);
        right_x = right_x.saturating_sub(pos_full_w);
        if self.rounded_corners {
            fb.fill_rounded_rect(right_x, chip_y, pos_full_w, chip_h, 3, self.colors.chip_bg);
        } else {
            fb.fill_rect(right_x, chip_y, pos_full_w, chip_h, self.colors.chip_bg);
        }
        fb.draw_text(&pos_full, right_x + 2, text_y, self.colors.status_bar_fg, None);
    }

    /// Renders a centered floating prompt modal overlay (Search, Save As, Open, Command Palette).
    pub fn render_prompt_overlay(&self, fb: &mut Framebuffer, modal: &crate::ui::prompt::PromptModal) {
        self.render_prompt_overlay_with_hover(fb, modal, false);
    }

    /// Renders the interactive Project-Wide Global Search Window (`Ctrl+Shift+F`).
    pub fn render_global_search_dialog(
        &self,
        fb: &mut Framebuffer,
        modal: &crate::ui::prompt::PromptModal,
        close_hovered: bool,
    ) {
        let box_w = fb.width.min(self.scale_coord(740)).max(self.scale_coord(460));
        let box_h = fb.height.min(self.scale_coord(500)).max(self.scale_coord(360));
        let box_x = (fb.width.saturating_sub(box_w)) / 2;
        let box_y: usize = self.scale_coord(28);

        // 1. Semi-transparent backdrop scrim
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 110);

        // 2. Drop shadow
        if self.shadows {
            self.render_shadow(fb, box_x, box_y, box_w, box_h, 10, 8);
        } else {
            fb.fill_rect(box_x + 6, box_y + 6, box_w, box_h, 0x000A0A0A);
        }

        // 3. Title bar & window outer border
        let header_h = self.scale_coord(24);
        fb.fill_rect(box_x.saturating_sub(1), box_y.saturating_sub(1), box_w + 2, box_h + 2, self.colors.modal_border);
        fb.fill_rect(box_x, box_y, box_w, header_h, self.colors.modal_header_bg);
        fb.fill_rect(box_x, box_y + header_h, box_w, box_h.saturating_sub(header_h), self.colors.modal_bg);
        if self.rounded_corners {
            fb.draw_rounded_outline(box_x, box_y, box_w, box_h, 6, self.colors.modal_border);
        }

        fb.draw_text("Búsqueda Global en el Proyecto (Ctrl+Shift+F)", box_x + self.scale_coord(10), box_y + self.scale_coord(4), 0x00FFFFFF, None);

        // Header close button [×]
        let close_w = self.scale_coord(30);
        let close_x = box_x + box_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, box_y + 2, close_w - 4, header_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, box_y, close_w, header_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(11), box_y + self.scale_coord(4), 0x00FFFFFF, None);

        // Header bottom divider
        fb.fill_rect(box_x, box_y + header_h, box_w, 1, self.colors.scrollbar_border);

        // 4. Input Box
        let body_y = box_y + header_h + 1;
        let input_margin = self.scale_coord(12);
        let input_x = box_x + input_margin;
        let input_y = body_y + self.scale_coord(8);
        let input_w = box_w.saturating_sub(input_margin * 2);
        let input_h = self.scale_coord(24);

        if self.rounded_corners {
            fb.fill_rounded_rect(input_x, input_y, input_w, input_h, 4, self.colors.input_bg);
            fb.draw_rounded_outline(input_x, input_y, input_w, input_h, 4, self.colors.input_border);
        } else {
            fb.fill_rect(input_x.saturating_sub(1), input_y.saturating_sub(1), input_w + 2, input_h + 2, self.colors.input_border);
            fb.fill_rect(input_x, input_y, input_w, input_h, self.colors.input_bg);
        }

        let label = "Buscar en Proyecto: ";
        let text_y = input_y + (input_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(label, input_x + self.scale_coord(6), text_y, self.colors.accent, None);

        let input_text_x = input_x + self.scale_coord(6) + label.chars().count() * self.char_width();
        fb.draw_text(&modal.input, input_text_x, text_y, 0x00FFFFFF, None);

        let cursor_x = input_text_x + modal.cursor_col * self.char_width();
        if cursor_x < input_x + input_w - self.scale_coord(4) {
            fb.fill_rect(cursor_x, text_y, self.scale_coord(2).max(1), self.char_height(), self.colors.cursor);
        }

        // 5. Status & Options bar
        let subbar_y = input_y + input_h + self.scale_coord(8);
        let subbar_h = self.scale_coord(22);

        let stats_text = if modal.input.trim().is_empty() {
            "Escriba para buscar en todos los archivos del proyecto...".to_string()
        } else if modal.global_results.is_empty() {
            "0 coincidencias encontradas".to_string()
        } else {
            let trunc_hint = if modal.global_truncated { " (límite 1000 alcanzado)" } else { "" };
            format!(
                "{} coincidencia{} en {} archivo{}{}",
                modal.global_results.len(),
                if modal.global_results.len() == 1 { "" } else { "s" },
                modal.global_matching_files,
                if modal.global_matching_files == 1 { "" } else { "s" },
                trunc_hint
            )
        };

        let stats_color = if modal.global_results.is_empty() {
            0x00888888
        } else {
            self.colors.syntax_string
        };
        fb.draw_text(&stats_text, input_x, subbar_y + self.scale_coord(3), stats_color, None);

        // Case-sensitive toggle button on the right of subbar
        let case_btn_text = if modal.case_sensitive { "[Aa Mayús: Sí]" } else { "[Aa Mayús: No]" };
        let case_w = case_btn_text.chars().count() * self.char_width() + self.scale_coord(8);
        let case_x = input_x + input_w - case_w;
        let case_bg = if modal.case_sensitive { self.colors.button_bg } else { self.colors.chip_bg };
        let case_border = if modal.case_sensitive { self.colors.accent } else { self.colors.scrollbar_border };
        if self.rounded_corners {
            fb.fill_rounded_rect(case_x, subbar_y, case_w, subbar_h, 3, case_bg);
            fb.draw_rounded_outline(case_x, subbar_y, case_w, subbar_h, 3, case_border);
        } else {
            fb.fill_rect(case_x, subbar_y, case_w, subbar_h, case_bg);
        }
        let case_tx = case_x + self.scale_coord(4);
        let case_ty = subbar_y + (subbar_h.saturating_sub(self.char_height())) / 2;
        let case_fg = if modal.case_sensitive { 0x00FFFFFF } else { 0x00AAAAAA };
        fb.draw_text(case_btn_text, case_tx, case_ty, case_fg, None);

        // 6. Results List
        let list_y = subbar_y + subbar_h + self.scale_coord(8);
        let footer_h = self.scale_coord(24);
        let list_h = (box_y + box_h).saturating_sub(list_y + footer_h + self.scale_coord(4));
        let list_w = input_w;
        let row_h = (self.char_height() + self.scale_coord(6)).max(self.scale_coord(22));
        let max_visible = list_h / row_h;

        // Background of results panel
        if self.rounded_corners {
            fb.fill_rounded_rect(input_x, list_y, list_w, list_h, 4, self.colors.gutter_bg);
            fb.draw_rounded_outline(input_x, list_y, list_w, list_h, 4, self.colors.scrollbar_border);
        } else {
            fb.fill_rect(input_x, list_y, list_w, list_h, self.colors.gutter_bg);
            fb.draw_rect_outline(input_x, list_y, list_w, list_h, self.colors.scrollbar_border);
        }

        let results_len = modal.global_results.len();
        let scroll_offset = modal.scroll_offset.min(results_len.saturating_sub(1));

        let path_col_w = (list_w / 3).min(self.scale_coord(240)).max(self.scale_coord(140));

        for row_idx in 0..max_visible {
            let item_idx = scroll_offset + row_idx;
            if item_idx >= results_len {
                break;
            }

            let item = &modal.global_results[item_idx];
            let is_sel = item_idx == modal.selected_idx;
            let item_y = list_y + self.scale_coord(2) + row_idx * row_h;

            if is_sel {
                let sel_bg = self.colors.accent;
                if self.rounded_corners {
                    fb.fill_rounded_rect(input_x + 2, item_y, list_w.saturating_sub(4), row_h.saturating_sub(2), 3, sel_bg);
                } else {
                    fb.fill_rect(input_x + 2, item_y, list_w.saturating_sub(4), row_h.saturating_sub(2), sel_bg);
                }
            }

            let item_text_y = item_y + (row_h.saturating_sub(self.char_height())) / 2;

            // 1. Path and line badge (e.g. "src/main.rs:42")
            let badge = format!("{}:{}", item.relative_path, item.line_number);
            let badge_max_chars = path_col_w / self.char_width().max(1);
            let badge_display = if badge.chars().count() > badge_max_chars {
                let take = badge_max_chars.saturating_sub(3);
                let tail: String = badge.chars().rev().take(take).collect::<Vec<char>>().into_iter().rev().collect();
                format!("...{}", tail)
            } else {
                badge
            };

            let badge_color = if is_sel { 0x00FFFFFF } else { self.colors.syntax_type };
            fb.draw_text(&badge_display, input_x + self.scale_coord(6), item_text_y, badge_color, None);

            // Divider line between file badge and code snippet
            let div_x = input_x + path_col_w;
            fb.fill_rect(div_x, item_y + 2, 1, row_h.saturating_sub(6), if is_sel { 0x00FFFFFF } else { self.colors.scrollbar_border });

            // 2. Code snippet preview with match highlight
            let code_x = div_x + self.scale_coord(8);
            let max_code_chars = (input_x + list_w).saturating_sub(code_x + self.scale_coord(16)) / self.char_width().max(1);
            let snippet_chars: Vec<char> = item.line_text.chars().take(max_code_chars).collect();
            let snippet_str: String = snippet_chars.iter().collect();

            // Draw full snippet line text
            let text_color = if is_sel { 0x00E0E0E0 } else { 0x00CCCCCC };
            fb.draw_text(&snippet_str, code_x, item_text_y, text_color, None);

            // Highlight match range inside snippet
            if item.match_char_offset < snippet_chars.len() {
                let hl_start = item.match_char_offset;
                let hl_len = item.match_char_len.min(snippet_chars.len() - hl_start);
                let hl_x = code_x + hl_start * self.char_width();
                let hl_w = hl_len * self.char_width();
                let hl_str: String = snippet_chars[hl_start..hl_start + hl_len].iter().collect();
                let hl_bg = if is_sel { 0x00D35400 } else { 0x00F39C12 };
                fb.fill_rect(hl_x, item_text_y, hl_w, self.char_height(), hl_bg);
                fb.draw_text(&hl_str, hl_x, item_text_y, 0x00FFFFFF, None);
            }
        }

        // 7. Right Scrollbar in Results List if items > max_visible
        if results_len > max_visible {
            let sb_w = self.scale_coord(6).max(4);
            let sb_x = input_x + list_w - sb_w - 2;
            let sb_y = list_y + self.scale_coord(4);
            let sb_h = list_h.saturating_sub(self.scale_coord(8));

            fb.fill_rect(sb_x, sb_y, sb_w, sb_h, self.colors.scrollbar_bg);
            let thumb_h = (((max_visible as f32 / results_len as f32) * sb_h as f32).max(12.0) as usize).min(sb_h);
            let max_scroll = results_len - max_visible;
            let thumb_y = sb_y + ((scroll_offset as f32 / max_scroll as f32) * (sb_h - thumb_h) as f32) as usize;
            fb.fill_rect(sb_x, thumb_y, sb_w, thumb_h, self.colors.scrollbar_thumb);
        }

        // 8. Footer hint
        let footer_y = list_y + list_h + self.scale_coord(6);
        let hint = "↑/↓: Seleccionar  •  Enter: Abrir en línea  •  Alt+C: Mayús/Minús  •  Esc: Cerrar";
        let hint_w = hint.chars().count() * self.char_width();
        let hint_x = box_x + (box_w.saturating_sub(hint_w)) / 2;
        let max_hint_x = box_x + box_w - self.scale_coord(8);
        fb.draw_text_truncated(hint, hint_x, footer_y, max_hint_x, 0x00858585, None);
    }

    pub fn render_find_replace_dialog(
        &self,
        fb: &mut Framebuffer,
        modal: &crate::ui::prompt::PromptModal,
        close_hovered: bool,
    ) {
        let box_w = fb.width.min(self.scale_coord(540)).max(self.scale_coord(380));
        let box_h = self.scale_coord(168);
        let box_x = (fb.width.saturating_sub(box_w)) / 2;
        let box_y: usize = self.scale_coord(28);

        // 1. Semi-transparent backdrop scrim
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 100);

        // 2. Drop shadow
        if self.shadows {
            self.render_shadow(fb, box_x, box_y, box_w, box_h, 10, 8);
        } else {
            fb.fill_rect(box_x + 6, box_y + 6, box_w, box_h, 0x000A0A0A);
        }

        // 3. Title bar & window outer border
        let header_h = self.scale_coord(24);
        fb.fill_rect(box_x.saturating_sub(1), box_y.saturating_sub(1), box_w + 2, box_h + 2, self.colors.modal_border);
        fb.fill_rect(box_x, box_y, box_w, header_h, self.colors.modal_header_bg);
        fb.fill_rect(box_x, box_y + header_h, box_w, box_h.saturating_sub(header_h), self.colors.modal_bg);
        if self.rounded_corners {
            fb.draw_rounded_outline(box_x, box_y, box_w, box_h, 6, self.colors.modal_border);
        }

        fb.draw_text("Buscar y Reemplazar", box_x + self.scale_coord(10), box_y + self.scale_coord(4), 0x00FFFFFF, None);

        // Close button [×]
        let close_w = self.scale_coord(30);
        let close_x = box_x + box_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, box_y + 2, close_w - 4, header_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, box_y, close_w, header_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(11), box_y + self.scale_coord(4), 0x00FFFFFF, None);

        // Header bottom divider
        fb.fill_rect(box_x, box_y + header_h, box_w, 1, self.colors.scrollbar_border);

        // 4. Window body
        let body_y = box_y + header_h + 1;
        let input_margin = self.scale_coord(12);
        let label_w = self.scale_coord(100);
        let input_w = box_w.saturating_sub(input_margin * 2 + label_w);
        let input_h = self.scale_coord(24);

        // Row 1: Buscar
        let row1_y = body_y + self.scale_coord(12);
        fb.draw_text("Buscar:", box_x + input_margin, row1_y + (input_h.saturating_sub(self.char_height())) / 2, 0x00CCCCCC, None);

        let input1_x = box_x + input_margin + label_w;
        let border1 = if modal.active_field == 0 { self.colors.input_border } else { self.colors.scrollbar_border };
        fb.fill_rect(input1_x.saturating_sub(1), row1_y.saturating_sub(1), input_w + 2, input_h + 2, border1);
        fb.fill_rect(input1_x, row1_y, input_w, input_h, self.colors.input_bg);
        let text1_y = row1_y + (input_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(&modal.input, input1_x + self.scale_coord(6), text1_y, 0x00FFFFFF, None);

        if modal.active_field == 0 {
            let cursor_x = input1_x + self.scale_coord(6) + modal.cursor_col * self.char_width();
            if cursor_x < input1_x + input_w - self.scale_coord(4) {
                fb.fill_rect(cursor_x, text1_y, self.scale_coord(2).max(1), self.char_height(), self.colors.cursor);
            }
        }

        // Row 2: Reemplazar
        let row2_y = row1_y + input_h + self.scale_coord(10);
        fb.draw_text("Reemplazar:", box_x + input_margin, row2_y + (input_h.saturating_sub(self.char_height())) / 2, 0x00CCCCCC, None);

        let input2_x = box_x + input_margin + label_w;
        let border2 = if modal.active_field == 1 { self.colors.input_border } else { self.colors.scrollbar_border };
        fb.fill_rect(input2_x.saturating_sub(1), row2_y.saturating_sub(1), input_w + 2, input_h + 2, border2);
        fb.fill_rect(input2_x, row2_y, input_w, input_h, self.colors.input_bg);
        let text2_y = row2_y + (input_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(&modal.replace_input, input2_x + self.scale_coord(6), text2_y, 0x00FFFFFF, None);

        if modal.active_field == 1 {
            let cursor_x = input2_x + self.scale_coord(6) + modal.replace_cursor_col * self.char_width();
            if cursor_x < input2_x + input_w - self.scale_coord(4) {
                fb.fill_rect(cursor_x, text2_y, self.scale_coord(2).max(1), self.char_height(), self.colors.cursor);
            }
        }

        // Row 3: Action Buttons
        let btn_y = row2_y + input_h + self.scale_coord(16);
        let btn_h = self.scale_coord(24);

        let render_dialog_btn = |fb: &mut Framebuffer, bx: usize, by: usize, bw: usize, bh: usize, text: &str, is_primary: bool| {
            let (bg, border) = if is_primary {
                (self.colors.button_bg, self.colors.accent)
            } else {
                (self.colors.chip_bg, self.colors.scrollbar_border)
            };
            if self.rounded_corners {
                fb.fill_rounded_rect(bx, by, bw, bh, 4, bg);
                fb.draw_rounded_outline(bx, by, bw, bh, 4, border);
            } else {
                fb.fill_rect(bx.saturating_sub(1), by.saturating_sub(1), bw + 2, bh + 2, border);
                fb.fill_rect(bx, by, bw, bh, bg);
            }
            let tw = text.chars().count() * self.char_width();
            let tx = bx + (bw.saturating_sub(tw)) / 2;
            let ty = by + (bh.saturating_sub(self.char_height())) / 2;
            fb.draw_text(text, tx, ty, 0x00FFFFFF, None);
        };

        // Button 1: [< Ant]
        let b1_x = box_x + input_margin;
        let b1_w = self.scale_coord(68);
        render_dialog_btn(fb, b1_x, btn_y, b1_w, btn_h, "< Ant", false);

        // Button 2: [Sig >]
        let b2_x = b1_x + b1_w + self.scale_coord(8);
        let b2_w = self.scale_coord(68);
        render_dialog_btn(fb, b2_x, btn_y, b2_w, btn_h, "Sig >", false);

        // Button 3: [Reemplazar]
        let b3_x = b2_x + b2_w + self.scale_coord(14);
        let b3_w = self.scale_coord(98);
        render_dialog_btn(fb, b3_x, btn_y, b3_w, btn_h, "Reemplazar", false);

        // Button 4: [Reemplazar Todo]
        let b4_x = b3_x + b3_w + self.scale_coord(8);
        let b4_w = self.scale_coord(126);
        render_dialog_btn(fb, b4_x, btn_y, b4_w, btn_h, "Reemplazar Todo", true);
    }

    /// Renders a floating prompt window dialog with title bar, close button, drop shadow, and suggestions.
    pub fn render_prompt_overlay_with_hover(
        &self,
        fb: &mut Framebuffer,
        modal: &crate::ui::prompt::PromptModal,
        close_hovered: bool,
    ) {
        if modal.is_file_dialog() {
            self.render_file_dialog(fb, modal, close_hovered);
            return;
        }

        if modal.kind == crate::ui::prompt::PromptKind::Replace {
            self.render_find_replace_dialog(fb, modal, close_hovered);
            return;
        }

        if modal.kind == crate::ui::prompt::PromptKind::GlobalSearch {
            self.render_global_search_dialog(fb, modal, close_hovered);
            return;
        }

        let is_palette = modal.kind == crate::ui::prompt::PromptKind::CommandPalette;
        let is_recent = modal.kind == crate::ui::prompt::PromptKind::RecentFiles;
        let box_w = if is_recent { fb.width.min(self.scale_coord(680)).max(self.scale_coord(400)) } else { fb.width.min(self.scale_coord(580)).max(self.scale_coord(340)) };
        let box_h = if is_palette || is_recent { self.scale_coord(340) } else { self.scale_coord(120) };
        let box_x = (fb.width.saturating_sub(box_w)) / 2;
        let box_y: usize = self.scale_coord(28);

        // 1. Semi-transparent backdrop scrim
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 110);

        // 2. Drop shadow
        if self.shadows {
            self.render_shadow(fb, box_x, box_y, box_w, box_h, 10, 8);
        } else {
            fb.fill_rect(box_x + 6, box_y + 6, box_w, box_h, 0x000A0A0A);
        }

        // 3. Outer border & Window Header (Title Bar)
        let header_h = self.scale_coord(24);
        fb.fill_rect(box_x.saturating_sub(1), box_y.saturating_sub(1), box_w + 2, box_h + 2, self.colors.modal_border);
        fb.fill_rect(box_x, box_y, box_w, header_h, self.colors.modal_header_bg);

        // 4. Window Body
        let body_y = box_y + header_h + 1;
        let body_h = box_h.saturating_sub(header_h + 1);
        fb.fill_rect(box_x, body_y, box_w, body_h, self.colors.modal_bg);

        if self.rounded_corners {
            fb.draw_rounded_outline(box_x, box_y, box_w, box_h, 6, self.colors.modal_border);
        }

        fb.draw_text(modal.kind.window_title(), box_x + self.scale_coord(10), box_y + self.scale_coord(4), 0x00FFFFFF, None);

        // Header close button [×]
        let close_w = self.scale_coord(30);
        let close_x = box_x + box_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, box_y + 2, close_w - 4, header_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, box_y, close_w, header_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(11), box_y + self.scale_coord(4), 0x00FFFFFF, None);

        // Header bottom divider
        fb.fill_rect(box_x, box_y + header_h, box_w, 1, self.colors.scrollbar_border);

        // 5. Input Field Box inside Window
        let input_margin = self.scale_coord(12);
        let input_x = box_x + input_margin;
        let input_y = body_y + self.scale_coord(8);
        let input_w = box_w.saturating_sub(input_margin * 2);
        let input_h = self.scale_coord(24);

        if self.rounded_corners {
            fb.fill_rounded_rect(input_x, input_y, input_w, input_h, 4, self.colors.input_bg);
            fb.draw_rounded_outline(input_x, input_y, input_w, input_h, 4, self.colors.input_border);
        } else {
            fb.fill_rect(input_x.saturating_sub(1), input_y.saturating_sub(1), input_w + 2, input_h + 2, self.colors.input_border);
            fb.fill_rect(input_x, input_y, input_w, input_h, self.colors.input_bg);
        }

        // Prompt label & input text
        let label = modal.kind.label();
        let text_y = input_y + (input_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(label, input_x + self.scale_coord(6), text_y, self.colors.accent, None);

        let input_text_x = input_x + self.scale_coord(6) + label.chars().count() * self.char_width();
        fb.draw_text(&modal.input, input_text_x, text_y, 0x00FFFFFF, None);

        // Caret bar
        let cursor_x = input_text_x + modal.cursor_col * self.char_width();
        if cursor_x < input_x + input_w - self.scale_coord(4) {
            fb.fill_rect(cursor_x, text_y, self.scale_coord(2).max(1), self.char_height(), self.colors.cursor);
        }

        // 6. Content below input
        if is_palette {
            let matching = modal.matching_commands();
            let mut item_y = input_y + input_h + self.scale_coord(8);
            let item_h = (self.char_height() + self.scale_coord(8)).max(self.scale_coord(24));
            let max_visible = 9;

            for (idx, cmd) in matching.iter().take(max_visible).enumerate() {
                let is_sel = idx == modal.selected_idx;
                let (row_bg, row_fg, sc_fg) = if is_sel {
                    (self.colors.accent, 0x00FFFFFF, 0x00E0E0E0)
                } else {
                    (self.colors.modal_bg, 0x00D4D4D4, 0x00858585)
                };

                if is_sel && self.rounded_corners {
                    fb.fill_rounded_rect(input_x, item_y, input_w, item_h, 4, row_bg);
                } else {
                    fb.fill_rect(input_x, item_y, input_w, item_h, row_bg);
                }

                let row_text_y = item_y + (item_h.saturating_sub(self.char_height())) / 2;

                if !cmd.shortcut.is_empty() {
                    let sc_w = cmd.shortcut.chars().count() * self.char_width();
                    let sc_x = input_x + input_w - sc_w - self.scale_coord(8);
                    fb.draw_text_clipped(cmd.shortcut, sc_x, row_text_y, input_x + input_w - self.scale_coord(4), sc_fg, None);
                    let name_max_x = sc_x.saturating_sub(self.scale_coord(8));
                    fb.draw_text_truncated(cmd.name, input_x + self.scale_coord(8), row_text_y, name_max_x, row_fg, None);
                } else {
                    let name_max_x = input_x + input_w - self.scale_coord(8);
                    fb.draw_text_truncated(cmd.name, input_x + self.scale_coord(8), row_text_y, name_max_x, row_fg, None);
                }

                item_y += item_h;
            }

            // Footer hint
            let hint = "↑/↓: Seleccionar  •  Enter: Ejecutar  •  Esc: Cerrar";
            let hint_w = hint.chars().count() * self.char_width();
            let hint_x = box_x + (box_w.saturating_sub(hint_w)) / 2;
            let hint_y = box_y + box_h - self.scale_coord(18);
            fb.draw_text_truncated(hint, hint_x, hint_y, box_x + box_w - self.scale_coord(8), 0x00858585, None);
        } else if is_recent {
            let matching = modal.matching_recent_files();
            let mut item_y = input_y + input_h + self.scale_coord(8);
            let item_h = (self.char_height() + self.scale_coord(10)).max(self.scale_coord(26));
            let max_visible = 9;

            if matching.is_empty() {
                fb.draw_text("(No hay archivos recientes coincidentes)", input_x + self.scale_coord(8), item_y + self.scale_coord(6), 0x00858585, None);
            } else {
                for (idx, path) in matching.iter().take(max_visible).enumerate() {
                    let is_sel = idx == modal.selected_idx;
                    let (row_bg, row_fg, dir_fg) = if is_sel {
                        (self.colors.accent, 0x00FFFFFF, 0x00E0E0E0)
                    } else {
                        (self.colors.modal_bg, 0x00FFFFFF, 0x00858585)
                    };

                    if is_sel && self.rounded_corners {
                        fb.fill_rounded_rect(input_x, item_y, input_w, item_h, 4, row_bg);
                    } else {
                        fb.fill_rect(input_x, item_y, input_w, item_h, row_bg);
                    }

                    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let parent_str = path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

                    let row_text_y = item_y + (item_h.saturating_sub(self.char_height())) / 2;
                    fb.draw_text(file_name, input_x + self.scale_coord(8), row_text_y, row_fg, None);

                    let fn_w = file_name.chars().count() * self.char_width();
                    let dir_x = input_x + self.scale_coord(8) + fn_w + self.scale_coord(12);
                    let max_dir_chars = input_w.saturating_sub(fn_w + self.scale_coord(24)) / self.char_width().max(1);
                    let display_dir = if parent_str.chars().count() > max_dir_chars && max_dir_chars > 3 {
                        format!("...{}", parent_str.chars().skip(parent_str.chars().count() - (max_dir_chars - 3)).collect::<String>())
                    } else {
                        parent_str
                    };
                    fb.draw_text(&display_dir, dir_x, row_text_y, dir_fg, None);

                    item_y += item_h;
                }
            }

            // Footer hint
            let hint = "↑/↓: Seleccionar  •  Enter: Abrir archivo  •  Esc: Cancelar";
            let hint_x = box_x + (box_w.saturating_sub(hint.chars().count() * self.char_width())) / 2;
            let hint_y = box_y + box_h - self.scale_coord(18);
            fb.draw_text(hint, hint_x, hint_y, 0x00858585, None);
        } else {
            // Footer hint for Find
            let hint = "Enter: Confirmar  •  Esc: Cancelar";
            let hint_x = box_x + (box_w.saturating_sub(hint.chars().count() * self.char_width())) / 2;
            let hint_y = box_y + box_h - self.scale_coord(22);
            fb.draw_text(hint, hint_x, hint_y, 0x00858585, None);
        }
    }

    /// Renders a native-styled File & Folder Dialog Window for saving and opening files in any directory.
    pub fn render_file_dialog(
        &self,
        fb: &mut Framebuffer,
        modal: &crate::ui::prompt::PromptModal,
        close_hovered: bool,
    ) {
        // Semi-transparent backdrop scrim
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 110);

        let box_w = fb.width.min(self.scale_coord(680)).max(self.scale_coord(420));
        let box_h = fb.height.min(self.scale_coord(480)).max(self.scale_coord(360));
        let box_x = (fb.width.saturating_sub(box_w)) / 2;
        let box_y = (fb.height.saturating_sub(box_h)) / 2;

        // 1. Soft Drop shadow
        if self.shadows {
            self.render_shadow(fb, box_x, box_y, box_w, box_h, 12, 8);
        } else {
            fb.fill_rect(box_x + 6, box_y + 6, box_w, box_h, 0x000A0A0A);
        }

        // 2. Outer border & Window Header (Title Bar)
        let header_h = self.scale_coord(26);
        fb.fill_rect(box_x.saturating_sub(1), box_y.saturating_sub(1), box_w + 2, box_h + 2, self.colors.modal_border);
        fb.fill_rect(box_x, box_y, box_w, header_h, self.colors.modal_header_bg);

        // 4. Window Body
        let body_y = box_y + header_h + 1;
        let body_h = box_h.saturating_sub(header_h + 1);
        fb.fill_rect(box_x, body_y, box_w, body_h, self.colors.modal_bg);

        if self.rounded_corners {
            fb.draw_rounded_outline(box_x, box_y, box_w, box_h, 6, self.colors.modal_border);
        }

        fb.draw_text(modal.kind.window_title(), box_x + self.scale_coord(10), box_y + self.scale_coord(5), 0x00FFFFFF, None);

        // Header close button [×]
        let close_w = self.scale_coord(32);
        let close_x = box_x + box_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, box_y + 2, close_w - 4, header_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, box_y, close_w, header_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(12), box_y + self.scale_coord(5), 0x00FFFFFF, None);

        // Header bottom divider
        fb.fill_rect(box_x, box_y + header_h, box_w, 1, self.colors.scrollbar_border);

        // 5. Current Directory Bar
        let dir_bar_y = body_y + self.scale_coord(6);
        let dir_bar_h = self.scale_coord(24);
        let dir_bar_w = box_w.saturating_sub(self.scale_coord(16));
        if self.rounded_corners {
            fb.fill_rounded_rect(box_x + self.scale_coord(8), dir_bar_y, dir_bar_w, dir_bar_h, 4, self.colors.input_bg);
        } else {
            fb.fill_rect(box_x + self.scale_coord(8), dir_bar_y, dir_bar_w, dir_bar_h, self.colors.input_bg);
        }
        fb.draw_text("Carpeta:", box_x + self.scale_coord(14), dir_bar_y + self.scale_coord(4), self.colors.accent, None);

        let path_str = modal.current_dir.display().to_string();
        let max_path_chars = (box_w.saturating_sub(self.scale_coord(190))) / self.char_width().max(1);
        let display_path = if path_str.chars().count() > max_path_chars && max_path_chars > 3 {
            let skip = path_str.chars().count().saturating_sub(max_path_chars - 3);
            format!("...{}", path_str.chars().skip(skip).collect::<String>())
        } else {
            path_str
        };
        fb.draw_text(&display_path, box_x + self.scale_coord(14) + 9 * self.char_width(), dir_bar_y + self.scale_coord(4), 0x00FFFFFF, None);

        // [Subir] button
        let up_btn_w = self.scale_coord(64);
        let up_btn_x = box_x + box_w - up_btn_w - self.scale_coord(12);
        if self.rounded_corners {
            fb.fill_rounded_rect(up_btn_x, dir_bar_y + self.scale_coord(2), up_btn_w, self.scale_coord(20), 3, self.colors.chip_bg);
            fb.draw_rounded_outline(up_btn_x, dir_bar_y + self.scale_coord(2), up_btn_w, self.scale_coord(20), 3, self.colors.scrollbar_border);
        } else {
            fb.fill_rect(up_btn_x, dir_bar_y + self.scale_coord(2), up_btn_w, self.scale_coord(20), self.colors.chip_bg);
        }
        fb.draw_text("[Subir]", up_btn_x + self.scale_coord(6), dir_bar_y + self.scale_coord(4), 0x00FFFFFF, None);

        // 6. Bookmarks Bar
        let bm_bar_y = dir_bar_y + dir_bar_h + self.scale_coord(4);
        let bookmarks = crate::ui::prompt::get_quick_bookmarks();
        let mut cur_bm_x = box_x + self.scale_coord(8);
        for bm in &bookmarks {
            let is_active = modal.current_dir == bm.path;
            let bm_text = format!("[{}]", bm.label);
            let bm_w = bm_text.chars().count() * self.char_width() + self.scale_coord(8);
            if cur_bm_x + bm_w < box_x + box_w - self.scale_coord(8) {
                let bg = if is_active { self.colors.accent } else { self.colors.chip_bg };
                let fg = if is_active { 0x00FFFFFF } else { 0x00CCCCCC };
                if self.rounded_corners {
                    fb.fill_rounded_rect(cur_bm_x, bm_bar_y, bm_w, self.scale_coord(20), 3, bg);
                } else {
                    fb.fill_rect(cur_bm_x, bm_bar_y, bm_w, self.scale_coord(20), bg);
                }
                fb.draw_text(&bm_text, cur_bm_x + self.scale_coord(4), bm_bar_y + self.scale_coord(2), fg, None);
                cur_bm_x += bm_w + self.scale_coord(6);
            }
        }

        // 7. Directory Entries List
        let list_y = bm_bar_y + self.scale_coord(26);
        let list_h = box_h.saturating_sub(self.scale_coord(190));
        let list_w = box_w.saturating_sub(self.scale_coord(16));
        let list_x = box_x + self.scale_coord(8);

        if self.rounded_corners {
            fb.fill_rounded_rect(list_x, list_y, list_w, list_h, 4, self.colors.input_bg);
            fb.draw_rounded_outline(list_x, list_y, list_w, list_h, 4, self.colors.scrollbar_border);
        } else {
            fb.fill_rect(list_x.saturating_sub(1), list_y.saturating_sub(1), list_w + 2, list_h + 2, self.colors.scrollbar_border);
            fb.fill_rect(list_x, list_y, list_w, list_h, self.colors.input_bg);
        }

        let row_h = (self.char_height() + self.scale_coord(6)).max(self.scale_coord(22));
        let max_visible = list_h / row_h;

        if modal.dir_entries.is_empty() {
            fb.draw_text("(Carpeta vacía o sin permisos de lectura)", list_x + self.scale_coord(12), list_y + self.scale_coord(10), 0x00858585, None);
        } else {
            for (idx, entry) in modal.dir_entries.iter().skip(modal.scroll_offset).take(max_visible).enumerate() {
                let actual_idx = modal.scroll_offset + idx;
                let is_selected = actual_idx == modal.selected_entry_idx;
                let ry = list_y + idx * row_h;

                let row_bg = if is_selected { self.colors.accent } else { self.colors.input_bg };
                if is_selected && self.rounded_corners {
                    fb.fill_rounded_rect(list_x + 2, ry, list_w.saturating_sub(4), row_h, 3, row_bg);
                } else {
                    fb.fill_rect(list_x, ry, list_w, row_h, row_bg);
                }

                let tag = if entry.is_dir { "[DIR] " } else { "      " };
                let tag_fg = if is_selected { 0x00FFFFFF } else { self.colors.syntax_type };
                let name_fg = if is_selected { 0x00FFFFFF } else if entry.is_dir { 0x00FFFFFF } else { 0x00D4D4D4 };

                fb.draw_text(tag, list_x + self.scale_coord(8), ry + (row_h.saturating_sub(self.char_height())) / 2, tag_fg, None);
                fb.draw_text(&entry.name, list_x + self.scale_coord(8) + 6 * self.char_width(), ry + (row_h.saturating_sub(self.char_height())) / 2, name_fg, None);
            }
        }

        // 8. Filename Input Field
        let input_y = box_y + box_h - self.scale_coord(76);
        fb.draw_text("Nombre de archivo:", list_x, input_y, 0x00D4D4D4, None);

        let input_box_y = input_y + self.scale_coord(18);
        let input_box_h = self.scale_coord(24);
        if self.rounded_corners {
            fb.fill_rounded_rect(list_x, input_box_y, list_w, input_box_h, 4, self.colors.input_bg);
            fb.draw_rounded_outline(list_x, input_box_y, list_w, input_box_h, 4, self.colors.input_border);
        } else {
            fb.fill_rect(list_x.saturating_sub(1), input_box_y.saturating_sub(1), list_w + 2, input_box_h + 2, self.colors.input_border);
            fb.fill_rect(list_x, input_box_y, list_w, input_box_h, self.colors.input_bg);
        }

        let input_text_y = input_box_y + (input_box_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(&modal.input, list_x + self.scale_coord(8), input_text_y, 0x00FFFFFF, None);

        // Caret
        let cursor_x = list_x + self.scale_coord(8) + modal.cursor_col * self.char_width();
        if cursor_x < list_x + list_w - self.scale_coord(4) {
            fb.fill_rect(cursor_x, input_text_y, self.scale_coord(2).max(1), self.char_height(), self.colors.cursor);
        }

        // 9. Bottom Buttons
        let btn_bar_y = box_y + box_h - self.scale_coord(30);
        let action_btn_text = if modal.kind == crate::ui::prompt::PromptKind::SaveAs { "Guardar" } else { "Abrir" };
        let action_btn_w = self.scale_coord(88);
        let cancel_btn_w = self.scale_coord(80);

        let cancel_btn_x = box_x + box_w - cancel_btn_w - self.scale_coord(12);
        let action_btn_x = cancel_btn_x - action_btn_w - self.scale_coord(8);
        let btn_h = self.scale_coord(24);

        // Action button [Guardar] / [Abrir]
        if self.rounded_corners {
            fb.fill_rounded_rect(action_btn_x, btn_bar_y, action_btn_w, btn_h, 4, self.colors.button_bg);
            fb.draw_rounded_outline(action_btn_x, btn_bar_y, action_btn_w, btn_h, 4, self.colors.accent);
        } else {
            fb.fill_rect(action_btn_x.saturating_sub(1), btn_bar_y.saturating_sub(1), action_btn_w + 2, btn_h, self.colors.accent);
            fb.fill_rect(action_btn_x, btn_bar_y, action_btn_w, btn_h.saturating_sub(2), self.colors.button_bg);
        }
        let at_x = action_btn_x + (action_btn_w.saturating_sub(action_btn_text.chars().count() * self.char_width())) / 2;
        fb.draw_text(action_btn_text, at_x, btn_bar_y + (btn_h.saturating_sub(self.char_height())) / 2, self.colors.button_fg, None);

        // Cancel button [Cancelar]
        if self.rounded_corners {
            fb.fill_rounded_rect(cancel_btn_x, btn_bar_y, cancel_btn_w, btn_h, 4, self.colors.chip_bg);
            fb.draw_rounded_outline(cancel_btn_x, btn_bar_y, cancel_btn_w, btn_h, 4, self.colors.scrollbar_border);
        } else {
            fb.fill_rect(cancel_btn_x.saturating_sub(1), btn_bar_y.saturating_sub(1), cancel_btn_w + 2, btn_h, self.colors.scrollbar_border);
            fb.fill_rect(cancel_btn_x, btn_bar_y, cancel_btn_w, btn_h.saturating_sub(2), self.colors.chip_bg);
        }
        let ct_x = cancel_btn_x + (cancel_btn_w.saturating_sub(8 * self.char_width())) / 2;
        fb.draw_text("Cancelar", ct_x, btn_bar_y + (btn_h.saturating_sub(self.char_height())) / 2, 0x00CCCCCC, None);

        // Footer hint
        fb.draw_text("Doble clic o Enter para abrir carpeta", list_x, btn_bar_y + self.scale_coord(4), 0x00858585, None);
    }

    /// Renders a native-styled About Dialog Window with title bar, close button, author details, and logo.
    pub fn render_about_dialog(
        &self,
        fb: &mut Framebuffer,
        close_hovered: bool,
        ok_hovered: bool,
    ) {
        // Semi-transparent backdrop tint (darken)
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 120);

        // Dialog box dimensions
        let win_w = self.scale_coord(640).min(fb.width.saturating_sub(self.scale_coord(20)));
        let win_h = self.scale_coord(450).min(fb.height.saturating_sub(self.scale_coord(20)));
        let win_x = (fb.width.saturating_sub(win_w)) / 2;
        let win_y = (fb.height.saturating_sub(win_h)) / 2;

        // Drop shadow
        if self.shadows {
            self.render_shadow(fb, win_x, win_y, win_w, win_h, 12, 8);
        } else {
            fb.fill_rect(win_x + 6, win_y + 6, win_w, win_h, 0x000A0A0A);
        }

        // Outer border & Header
        let title_h = self.scale_coord(28);
        fb.fill_rect(win_x.saturating_sub(1), win_y.saturating_sub(1), win_w + 2, win_h + 2, self.colors.modal_border);
        fb.fill_rect(win_x, win_y, win_w, title_h, self.colors.modal_header_bg);

        // Window Body background
        let body_y = win_y + title_h + 1;
        let body_h = win_h.saturating_sub(title_h + 1);
        fb.fill_rect(win_x, body_y, win_w, body_h, self.colors.modal_bg);

        if self.rounded_corners {
            fb.draw_rounded_outline(win_x, win_y, win_w, win_h, 6, self.colors.modal_border);
        }

        fb.draw_text("Rustext - Acerca de", win_x + self.scale_coord(12), win_y + self.scale_coord(6), 0x00FFFFFF, None);

        // Close button [×] on the right
        let close_w = self.scale_coord(36);
        let close_x = win_x + win_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, win_y + 2, close_w - 4, title_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, win_y, close_w, title_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(14), win_y + self.scale_coord(6), 0x00FFFFFF, None);

        // Title bar separator
        fb.fill_rect(win_x, win_y + title_h, win_w, 1, self.colors.scrollbar_border);

        // Rustext ASCII Banner
        let banner_lines = [
            "  ____            _            _   ",
            " |  _ \\ _   _ ___| |_ _____  _| |_ ",
            " | |_) | | | / __| __/ _ \\ \\/ / __|",
            " |  _ <| |_| \\__ \\ ||  __/>  <| |_ ",
            " |_| \\_\\\\__,_|___/\\__\\___/_/\\_\\\\__|",
        ];
        let mut cur_y = body_y + self.scale_coord(12);
        let banner_color = self.colors.accent;
        for line in &banner_lines {
            let line_w = line.chars().count() * self.char_width();
            let line_x = win_x + (win_w.saturating_sub(line_w)) / 2;
            fb.draw_text(line, line_x, cur_y, banner_color, None);
            cur_y += self.char_height() + 2;
        }

        // Subtitle & Version
        cur_y += self.scale_coord(4);
        let title_str = "Rustext Editor v0.1.0";
        let title_x = win_x + (win_w.saturating_sub(title_str.chars().count() * self.char_width())) / 2;
        fb.draw_text(title_str, title_x, cur_y, 0x00FFFFFF, None);
        cur_y += self.char_height() + self.scale_coord(4);

        let tag_str = "100% Pure Rust • Zero Dependencias Externas";
        let tag_x = win_x + (win_w.saturating_sub(tag_str.chars().count() * self.char_width())) / 2;
        fb.draw_text(tag_str, tag_x, cur_y, 0x004EC9B0, None);
        cur_y += self.char_height() + self.scale_coord(6);

        // Separator line
        fb.fill_rect(win_x + self.scale_coord(20), cur_y, win_w.saturating_sub(self.scale_coord(40)), 1, self.colors.scrollbar_border);
        cur_y += self.scale_coord(10);

        // Project and Author Details
        let details = [
            ("• Desarrollador: ", "boleklolek <bolek.lolek@gmail.com>"),
            ("• Licencia:      ", "MIT License (Código Abierto)"),
            ("• Arquitectura:  ", "100% Rust Estándar (Sin crates ni librerías)"),
            ("• Motor:         ", "Piece Table O(log N) con Multi-Cursor"),
            ("• Renderizado:   ", "Framebuffer RGB32 Nativo por Software"),
            ("• Plataformas:   ", "Linux (X11) • Windows (Win32) • macOS"),
            ("• Repositorio:   ", "https://github.com/boleklolek/rustext"),
        ];

        let left_margin = win_x + self.scale_coord(24);
        let max_val_x = win_x + win_w - self.scale_coord(24);
        for (lbl, val) in &details {
            fb.draw_text(lbl, left_margin, cur_y, 0x00858585, None);
            let val_x = left_margin + lbl.chars().count() * self.char_width();
            let val_color = if lbl.contains("Repositorio") { self.colors.accent } else { 0x00D4D4D4 };
            fb.draw_text_truncated(val, val_x, cur_y, max_val_x, val_color, None);
            cur_y += self.char_height() + self.scale_coord(3);
        }

        cur_y += self.scale_coord(4);
        // Separator line
        fb.fill_rect(win_x + self.scale_coord(20), cur_y, win_w.saturating_sub(self.scale_coord(40)), 1, self.colors.scrollbar_border);
        cur_y += self.scale_coord(8);

        // Essential shortcuts reference
        let sc_str = "Ctrl+T: Pestaña  •  Ctrl+S: Guardar  •  Ctrl+F: Buscar  •  Ctrl+P: Comandos";
        let sc_w = sc_str.chars().count() * self.char_width();
        let sc_x = win_x + (win_w.saturating_sub(sc_w)) / 2;
        let max_sc_x = win_x + win_w - self.scale_coord(16);
        fb.draw_text_truncated(sc_str, sc_x, cur_y, max_sc_x, 0x00858585, None);

        // 8. Bottom Button: [ Aceptar ]
        let btn_w = self.scale_coord(130);
        let btn_h = self.scale_coord(28);
        let btn_x = win_x + (win_w - btn_w) / 2;
        let btn_y = win_y + win_h - self.scale_coord(38);

        let (btn_bg, btn_border) = if ok_hovered {
            (self.colors.button_hover_bg, self.colors.accent)
        } else {
            (self.colors.button_bg, self.colors.scrollbar_border)
        };
        if self.rounded_corners {
            fb.fill_rounded_rect(btn_x, btn_y, btn_w, btn_h, 4, btn_bg);
            fb.draw_rounded_outline(btn_x, btn_y, btn_w, btn_h, 4, btn_border);
        } else {
            fb.fill_rect(btn_x.saturating_sub(1), btn_y.saturating_sub(1), btn_w + 2, btn_h + 2, btn_border);
            fb.fill_rect(btn_x, btn_y, btn_w, btn_h, btn_bg);
        }

        let btn_text = "Aceptar";
        let text_x = btn_x + (btn_w.saturating_sub(btn_text.chars().count() * self.char_width())) / 2;
        let text_y = btn_y + (btn_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(btn_text, text_x, text_y, self.colors.button_fg, None);
    }

    /// Renders a native-styled Extensions Manager Dialog Window.
    pub fn render_extensions_dialog(
        &self,
        fb: &mut Framebuffer,
        ext_mgr: &crate::plugin::ExtensionManager,
        selected_idx: usize,
        scroll_offset: usize,
        close_hovered: bool,
    ) {
        // Semi-transparent backdrop scrim
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 110);

        let win_w = self.scale_coord(680).min(fb.width.saturating_sub(self.scale_coord(20)));
        let win_h = self.scale_coord(480).min(fb.height.saturating_sub(self.scale_coord(20)));
        let win_x = (fb.width.saturating_sub(win_w)) / 2;
        let win_y = (fb.height.saturating_sub(win_h)) / 2;

        // Drop shadow
        if self.shadows {
            self.render_shadow(fb, win_x, win_y, win_w, win_h, 12, 8);
        } else {
            fb.fill_rect(win_x + 6, win_y + 6, win_w, win_h, 0x000A0A0A);
        }

        // Header Title Bar & Window Border
        let title_h = self.scale_coord(28);
        fb.fill_rect(win_x.saturating_sub(1), win_y.saturating_sub(1), win_w + 2, win_h + 2, self.colors.modal_border);
        fb.fill_rect(win_x, win_y, win_w, title_h, self.colors.modal_header_bg);

        // Body background
        let body_y = win_y + title_h + 1;
        let body_h = win_h.saturating_sub(title_h + 1);
        fb.fill_rect(win_x, body_y, win_w, body_h, self.colors.modal_bg);

        if self.rounded_corners {
            fb.draw_rounded_outline(win_x, win_y, win_w, win_h, 6, self.colors.modal_border);
        }

        fb.draw_text("Administrador de Extensiones", win_x + self.scale_coord(12), win_y + self.scale_coord(6), 0x00FFFFFF, None);

        // Close button [×]
        let close_w = self.scale_coord(36);
        let close_x = win_x + win_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, win_y + 2, close_w - 4, title_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, win_y, close_w, title_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(14), win_y + self.scale_coord(6), 0x00FFFFFF, None);

        // Header bottom divider
        fb.fill_rect(win_x, win_y + title_h, win_w, 1, self.colors.scrollbar_border);

        // Toolbar: Directory path & action buttons
        let bar_y = body_y + self.scale_coord(8);
        let count = ext_mgr.extensions.len();
        let dir_str = format!("Carpeta: ~/.rustext/extensions  •  {} extensiones", count);
        fb.draw_text(&dir_str, win_x + self.scale_coord(12), bar_y + self.scale_coord(4), self.colors.accent, None);

        // Action buttons: [ + Crear Ejemplo ] & [ ↺ Recargar ]
        let reload_btn_w = self.scale_coord(90);
        let create_btn_w = self.scale_coord(120);
        let reload_btn_x = win_x + win_w - reload_btn_w - self.scale_coord(12);
        let create_btn_x = reload_btn_x - create_btn_w - self.scale_coord(8);
        let btn_bar_h = self.scale_coord(24);

        if self.rounded_corners {
            fb.fill_rounded_rect(create_btn_x, bar_y, create_btn_w, btn_bar_h, 4, self.colors.chip_bg);
            fb.draw_rounded_outline(create_btn_x, bar_y, create_btn_w, btn_bar_h, 4, self.colors.scrollbar_border);
            fb.fill_rounded_rect(reload_btn_x, bar_y, reload_btn_w, btn_bar_h, 4, self.colors.chip_bg);
            fb.draw_rounded_outline(reload_btn_x, bar_y, reload_btn_w, btn_bar_h, 4, self.colors.scrollbar_border);
        } else {
            fb.fill_rect(create_btn_x.saturating_sub(1), bar_y.saturating_sub(1), create_btn_w + 2, btn_bar_h, self.colors.scrollbar_border);
            fb.fill_rect(create_btn_x, bar_y, create_btn_w, btn_bar_h.saturating_sub(2), self.colors.chip_bg);
            fb.fill_rect(reload_btn_x.saturating_sub(1), bar_y.saturating_sub(1), reload_btn_w + 2, btn_bar_h, self.colors.scrollbar_border);
            fb.fill_rect(reload_btn_x, bar_y, reload_btn_w, btn_bar_h.saturating_sub(2), self.colors.chip_bg);
        }
        fb.draw_text("+ Crear Ejemplo", create_btn_x + self.scale_coord(6), bar_y + (btn_bar_h.saturating_sub(self.char_height())) / 2, 0x00FFFFFF, None);
        fb.draw_text("↺ Recargar", reload_btn_x + self.scale_coord(10), bar_y + (btn_bar_h.saturating_sub(self.char_height())) / 2, 0x00FFFFFF, None);

        // List Area
        let list_x = win_x + self.scale_coord(12);
        let list_y = bar_y + self.scale_coord(30);
        let list_w = win_w.saturating_sub(self.scale_coord(24));
        let list_h = win_h.saturating_sub(self.scale_coord(115));

        if self.rounded_corners {
            fb.fill_rounded_rect(list_x, list_y, list_w, list_h, 4, self.colors.input_bg);
            fb.draw_rounded_outline(list_x, list_y, list_w, list_h, 4, self.colors.scrollbar_border);
        } else {
            fb.fill_rect(list_x.saturating_sub(1), list_y.saturating_sub(1), list_w + 2, list_h + 2, self.colors.scrollbar_border);
            fb.fill_rect(list_x, list_y, list_w, list_h, self.colors.input_bg);
        }

        if ext_mgr.extensions.is_empty() {
            fb.draw_text("No hay extensiones instaladas. Haz clic en '+ Crear Ejemplo'.", list_x + self.scale_coord(16), list_y + self.scale_coord(20), 0x00858585, None);
        } else {
            let card_h = (self.char_height() * 3 + self.scale_coord(10)).max(self.scale_coord(58));
            let max_visible = list_h / card_h;
            let mut cur_card_y = list_y + self.scale_coord(2);

            for (idx, ext) in ext_mgr.extensions.iter().skip(scroll_offset).take(max_visible).enumerate() {
                let actual_idx = scroll_offset + idx;
                let is_selected = actual_idx == selected_idx;

                let card_bg = if is_selected { self.colors.button_bg } else { self.colors.modal_bg };
                let border_color = if is_selected { self.colors.accent } else { self.colors.scrollbar_border };

                if self.rounded_corners {
                    fb.fill_rounded_rect(list_x + 2, cur_card_y, list_w.saturating_sub(4), card_h.saturating_sub(4), 4, card_bg);
                    fb.fill_rounded_rect(list_x + 2, cur_card_y, 4, card_h.saturating_sub(4), 2, border_color);
                } else {
                    fb.fill_rect(list_x + 2, cur_card_y, list_w.saturating_sub(4), card_h.saturating_sub(4), card_bg);
                    fb.fill_rect(list_x + 2, cur_card_y, 3, card_h.saturating_sub(4), border_color);
                }

                // Extension Name & Version
                let title_text = format!("{}  v{}", ext.name, ext.version);
                fb.draw_text(&title_text, list_x + self.scale_coord(12), cur_card_y + self.scale_coord(4), 0x00FFFFFF, None);

                // Status Badge: [ACTIVA] or [DESACTIVADA]
                let (badge_text, badge_bg, badge_fg) = if ext.enabled {
                    ("[ACTIVA]", 0x000E7030, 0x004EC9B0)
                } else {
                    ("[DESACTIVADA]", self.colors.chip_bg, 0x00858585)
                };
                let badge_w = self.scale_coord(100);
                let badge_h = self.scale_coord(18);
                let badge_x = list_x + list_w - self.scale_coord(180);
                if self.rounded_corners {
                    fb.fill_rounded_rect(badge_x, cur_card_y + self.scale_coord(4), badge_w, badge_h, 3, badge_bg);
                } else {
                    fb.fill_rect(badge_x, cur_card_y + self.scale_coord(4), badge_w, badge_h, badge_bg);
                }
                fb.draw_text(badge_text, badge_x + self.scale_coord(6), cur_card_y + self.scale_coord(5), badge_fg, None);

                // Description
                let desc = if ext.description.is_empty() { "Sin descripción" } else { &ext.description };
                fb.draw_text(desc, list_x + self.scale_coord(12), cur_card_y + self.char_height() + self.scale_coord(8), 0x00CCCCCC, None);

                // Commands tag
                let cmds_summary = if ext.commands.is_empty() {
                    "Sin comandos registrados".to_string()
                } else {
                    let names: Vec<&str> = ext.commands.iter().map(|c| c.label.as_str()).collect();
                    format!("Comandos: {}", names.join(", "))
                };
                fb.draw_text(&cmds_summary, list_x + self.scale_coord(12), cur_card_y + self.char_height() * 2 + self.scale_coord(10), 0x00858585, None);

                // Toggle Action Button on row
                let toggle_text = if ext.enabled { "Desactivar" } else { "Activar" };
                let toggle_w = self.scale_coord(64);
                let toggle_h = self.scale_coord(20);
                let toggle_x = list_x + list_w - toggle_w - self.scale_coord(8);
                let toggle_y = cur_card_y + self.scale_coord(4);
                if self.rounded_corners {
                    fb.fill_rounded_rect(toggle_x, toggle_y, toggle_w, toggle_h, 3, self.colors.chip_bg);
                    fb.draw_rounded_outline(toggle_x, toggle_y, toggle_w, toggle_h, 3, self.colors.scrollbar_border);
                } else {
                    fb.fill_rect(toggle_x, toggle_y, toggle_w, toggle_h, self.colors.chip_bg);
                }
                let ttx = toggle_x + (toggle_w.saturating_sub(toggle_text.chars().count() * self.char_width())) / 2;
                fb.draw_text(toggle_text, ttx, toggle_y + (toggle_h.saturating_sub(self.char_height())) / 2, 0x00FFFFFF, None);

                cur_card_y += card_h;
            }
        }

        // Footer hints & Close button
        let footer_y = win_y + win_h - self.scale_coord(34);
        let close_btn_w = self.scale_coord(80);
        let close_btn_h = self.scale_coord(24);
        let close_btn_x = win_x + win_w - close_btn_w - self.scale_coord(12);

        let max_hint_x = close_btn_x.saturating_sub(self.scale_coord(10));
        let hint_text = "Enter / Espacio: Alternar  •  'n': Nueva  •  'r': Recargar  •  Esc: Salir";
        fb.draw_text_truncated(hint_text, list_x, footer_y + self.scale_coord(6), max_hint_x, 0x00858585, None);
        if self.rounded_corners {
            fb.fill_rounded_rect(close_btn_x, footer_y, close_btn_w, close_btn_h, 4, self.colors.chip_bg);
            fb.draw_rounded_outline(close_btn_x, footer_y, close_btn_w, close_btn_h, 4, self.colors.scrollbar_border);
        } else {
            fb.fill_rect(close_btn_x.saturating_sub(1), footer_y.saturating_sub(1), close_btn_w + 2, close_btn_h + 2, self.colors.scrollbar_border);
            fb.fill_rect(close_btn_x, footer_y, close_btn_w, close_btn_h, self.colors.chip_bg);
        }
        let ctx = close_btn_x + (close_btn_w.saturating_sub(6 * self.char_width())) / 2;
        fb.draw_text("Cerrar", ctx, footer_y + (close_btn_h.saturating_sub(self.char_height())) / 2, 0x00FFFFFF, None);
    }

    /// Renders a native-styled Unsaved Changes Confirmation Dialog Window.
    pub fn render_unsaved_confirm_dialog(
        &self,
        fb: &mut Framebuffer,
        dialog: &crate::editor::app::UnsavedConfirmDialog,
        hovered_button: Option<usize>, // 0: Guardar, 1: Descartar, 2: Cancelar, 3: close_x
    ) {
        // 1. Semi-transparent backdrop scrim (dimming the background)
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 120);

        let win_w = self.scale_coord(520).min(fb.width.saturating_sub(self.scale_coord(20)));
        let win_h = self.scale_coord(190).min(fb.height.saturating_sub(self.scale_coord(20)));
        let win_x = (fb.width.saturating_sub(win_w)) / 2;
        let win_y = (fb.height.saturating_sub(win_h)) / 2;

        // 2. Drop shadow
        if self.shadows {
            self.render_shadow(fb, win_x, win_y, win_w, win_h, 12, 8);
        } else {
            fb.fill_rect(win_x + 6, win_y + 6, win_w, win_h, 0x000A0A0A);
        }

        // 3. Header Title Bar & Window Border
        let title_h = self.scale_coord(28);
        fb.fill_rect(win_x.saturating_sub(1), win_y.saturating_sub(1), win_w + 2, win_h + 2, self.colors.modal_border);
        fb.fill_rect(win_x, win_y, win_w, title_h, self.colors.modal_header_bg);

        // Body background
        let body_y = win_y + title_h + 1;
        let body_h = win_h.saturating_sub(title_h + 1);
        fb.fill_rect(win_x, body_y, win_w, body_h, self.colors.modal_bg);

        if self.rounded_corners {
            fb.draw_rounded_outline(win_x, win_y, win_w, win_h, 6, self.colors.modal_border);
        }

        // Title text: "⚠ Guardar Cambios"
        let title_str = format!("⚠ {}", dialog.title());
        fb.draw_text(&title_str, win_x + self.scale_coord(12), win_y + self.scale_coord(6), 0x00E5C07B, None); // Amber / warning gold

        // Close button [×] in top right
        let close_w = self.scale_coord(36);
        let close_x = win_x + win_w - close_w;
        let close_hovered = hovered_button == Some(3);
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, win_y + 2, close_w - 4, title_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, win_y, close_w, title_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(14), win_y + self.scale_coord(6), 0x00FFFFFF, None);

        // Header divider
        fb.fill_rect(win_x, win_y + title_h, win_w, 1, self.colors.scrollbar_border);

        // Warning Icon Badge in Body
        let icon_box_x = win_x + self.scale_coord(20);
        let icon_box_y = body_y + self.scale_coord(20);
        let icon_box_size = self.scale_coord(40);
        if self.rounded_corners {
            fb.fill_rounded_rect(icon_box_x, icon_box_y, icon_box_size, icon_box_size, 8, 0x003A3020);
            fb.draw_rounded_outline(icon_box_x, icon_box_y, icon_box_size, icon_box_size, 8, 0x00E5C07B);
        } else {
            fb.fill_rect(icon_box_x, icon_box_y, icon_box_size, icon_box_size, 0x003A3020);
        }
        fb.draw_text("!", icon_box_x + (icon_box_size.saturating_sub(self.char_width())) / 2, icon_box_y + (icon_box_size.saturating_sub(self.char_height())) / 2, 0x00E5C07B, None);

        // Message text
        let (line1, line2) = dialog.message_lines();
        let text_start_x = icon_box_x + icon_box_size + self.scale_coord(16);
        let text_max_chars = (win_x + win_w - text_start_x - self.scale_coord(16)) / self.char_width().max(1);
        let trunc_line1 = if line1.chars().count() > text_max_chars && text_max_chars > 3 {
            format!("{}...", line1.chars().take(text_max_chars - 3).collect::<String>())
        } else {
            line1
        };
        fb.draw_text(&trunc_line1, text_start_x, icon_box_y + self.scale_coord(4), 0x00FFFFFF, None);
        fb.draw_text(&line2, text_start_x, icon_box_y + self.char_height() + self.scale_coord(6), 0x00CCCCCC, None);

        // Buttons Bar at bottom
        let btn_bar_y = win_y + win_h - self.scale_coord(42);
        let btn_h = self.scale_coord(28);
        let (lbl_save, lbl_discard, lbl_cancel) = dialog.button_labels();

        let btn_save_w = (lbl_save.chars().count() * self.char_width() + self.scale_coord(24)).max(self.scale_coord(100));
        let btn_discard_w = (lbl_discard.chars().count() * self.char_width() + self.scale_coord(24)).max(self.scale_coord(110));
        let btn_cancel_w = (lbl_cancel.chars().count() * self.char_width() + self.scale_coord(24)).max(self.scale_coord(90));

        let total_btns_w = btn_save_w + btn_discard_w + btn_cancel_w + self.scale_coord(16);
        let btn_start_x = win_x + win_w - total_btns_w - self.scale_coord(16);

        let b0_x = btn_start_x;
        let b1_x = b0_x + btn_save_w + self.scale_coord(8);
        let b2_x = b1_x + btn_discard_w + self.scale_coord(8);

        let render_btn = |fb: &mut Framebuffer, bx: usize, bw: usize, label: &str, is_selected: bool, is_hovered: bool, btn_type: usize| {
            let (bg, border, fg) = if is_hovered || is_selected {
                match btn_type {
                    0 => (self.colors.button_hover_bg, self.colors.accent, 0x00FFFFFF),
                    1 => (0x00802828, 0x00F7768E, 0x00FFFFFF),
                    _ => (self.colors.chip_bg, self.colors.accent, 0x00FFFFFF),
                }
            } else {
                match btn_type {
                    0 => (self.colors.button_bg, self.colors.scrollbar_border, 0x00FFFFFF),
                    1 => (self.colors.chip_bg, self.colors.scrollbar_border, 0x00F7768E),
                    _ => (self.colors.chip_bg, self.colors.scrollbar_border, 0x00CCCCCC),
                }
            };

            if self.rounded_corners {
                fb.fill_rounded_rect(bx, btn_bar_y, bw, btn_h, 4, bg);
                fb.draw_rounded_outline(bx, btn_bar_y, bw, btn_h, 4, border);
                if is_selected {
                    fb.draw_rounded_outline(bx.saturating_sub(1), btn_bar_y.saturating_sub(1), bw + 2, btn_h + 2, 5, self.colors.accent);
                }
            } else {
                fb.fill_rect(bx.saturating_sub(1), btn_bar_y.saturating_sub(1), bw + 2, btn_h + 2, border);
                fb.fill_rect(bx, btn_bar_y, bw, btn_h, bg);
            }

            let tw = label.chars().count() * self.char_width();
            let tx = bx + (bw.saturating_sub(tw)) / 2;
            let ty = btn_bar_y + (btn_h.saturating_sub(self.char_height())) / 2;
            fb.draw_text(label, tx, ty, fg, None);
        };

        render_btn(fb, b0_x, btn_save_w, lbl_save, dialog.selected_button == 0, hovered_button == Some(0), 0);
        render_btn(fb, b1_x, btn_discard_w, lbl_discard, dialog.selected_button == 1, hovered_button == Some(1), 1);
        render_btn(fb, b2_x, btn_cancel_w, lbl_cancel, dialog.selected_button == 2, hovered_button == Some(2), 2);
    }

    /// Renders the interactive Font Selector and Typography Settings Dialog.
    pub fn render_font_selector_dialog(
        &self,
        fb: &mut Framebuffer,
        dialog: &crate::editor::font_selector::FontSelectorDialog,
        close_hovered: bool,
    ) {
        // 1. Semi-transparent backdrop scrim
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 120);

        let win_w = self.scale_coord(760).min(fb.width.saturating_sub(self.scale_coord(20)));
        let win_h = self.scale_coord(520).min(fb.height.saturating_sub(self.scale_coord(20)));
        let win_x = (fb.width.saturating_sub(win_w)) / 2;
        let win_y = (fb.height.saturating_sub(win_h)) / 2;

        // 2. Drop shadow
        if self.shadows {
            self.render_shadow(fb, win_x, win_y, win_w, win_h, 12, 8);
        } else {
            fb.fill_rect(win_x + 6, win_y + 6, win_w, win_h, 0x000A0A0A);
        }

        // 3. Header & Window Frame
        let title_h = self.scale_coord(28);
        fb.fill_rect(win_x.saturating_sub(1), win_y.saturating_sub(1), win_w + 2, win_h + 2, self.colors.modal_border);
        fb.fill_rect(win_x, win_y, win_w, title_h, self.colors.modal_header_bg);

        let body_y = win_y + title_h + 1;
        let body_h = win_h.saturating_sub(title_h + 1);
        fb.fill_rect(win_x, body_y, win_w, body_h, self.colors.modal_bg);

        if self.rounded_corners {
            fb.draw_rounded_outline(win_x, win_y, win_w, win_h, 6, self.colors.modal_border);
        }

        fb.draw_text("Tipografía y Selector de Fuentes", win_x + self.scale_coord(12), win_y + self.scale_coord(6), 0x00FFFFFF, None);

        // Close button [×]
        let close_w = self.scale_coord(36);
        let close_x = win_x + win_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, win_y + 2, close_w - 4, title_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, win_y, close_w, title_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(14), win_y + self.scale_coord(6), 0x00FFFFFF, None);

        fb.fill_rect(win_x, win_y + title_h, win_w, 1, self.colors.scrollbar_border);

        // Columns layout
        let footer_h = self.scale_coord(44);
        let content_h = body_h.saturating_sub(footer_h + self.scale_coord(12));
        let col_w = (win_w.saturating_sub(self.scale_coord(36))) / 2;
        let left_x = win_x + self.scale_coord(14);
        let right_x = left_x + col_w + self.scale_coord(8);
        let content_y = body_y + self.scale_coord(8);

        // === LEFT COLUMN: Controls ===
        // 1. Font Family header
        fb.draw_text("FAMILIAS TIPOGRÁFICAS (5 fuentes)", left_x, content_y, self.colors.accent, None);
        let fam_start_y = content_y + self.scale_coord(16);
        let fam_card_h = self.scale_coord(34);

        for (i, &f) in FontFamily::ALL.iter().enumerate() {
            let card_y = fam_start_y + i * (fam_card_h + self.scale_coord(3));
            let is_selected = f == dialog.selected_family;
            let is_hovered = dialog.hovered_family == Some(i);

            let bg = if is_selected {
                self.colors.button_bg
            } else if is_hovered {
                self.colors.input_bg
            } else {
                self.colors.modal_bg
            };

            let border = if is_selected {
                self.colors.accent
            } else {
                self.colors.scrollbar_border
            };

            if self.rounded_corners {
                fb.fill_rounded_rect(left_x, card_y, col_w, fam_card_h, 4, bg);
                fb.draw_rounded_outline(left_x, card_y, col_w, fam_card_h, 4, border);
            } else {
                fb.fill_rect(left_x, card_y, col_w, fam_card_h, bg);
            }

            let bullet = if is_selected { "[✓] " } else { "[ ] " };
            let fam_title = format!("{}{}", bullet, f.display_name());
            let title_color = if is_selected { 0x00FFFFFF } else { 0x00DDDDDD };
            fb.draw_text(&fam_title, left_x + self.scale_coord(8), card_y + self.scale_coord(3), title_color, None);

            let desc = f.description();
            let desc_max_chars = (col_w.saturating_sub(self.scale_coord(20))) / self.char_width().max(1);
            let desc_trunc: String = desc.chars().take(desc_max_chars).collect();
            fb.draw_text(&desc_trunc, left_x + self.scale_coord(16), card_y + self.scale_coord(18), 0x00888888, None);
        }

        // 2. Font Size selector
        let size_sec_y = fam_start_y + 5 * (fam_card_h + self.scale_coord(3)) + self.scale_coord(6);
        fb.draw_text("TAMAÑO DE FUENTE (Font Size)", left_x, size_sec_y, self.colors.accent, None);
        let size_pills_y = size_sec_y + self.scale_coord(16);
        let pill_h = self.scale_coord(22);
        let pill_w = (col_w.saturating_sub(self.scale_coord(28))) / crate::editor::font_selector::AVAILABLE_SIZES.len().max(1);

        for (idx, &s) in crate::editor::font_selector::AVAILABLE_SIZES.iter().enumerate() {
            let px = left_x + idx * (pill_w + self.scale_coord(4));
            let is_sel = s == dialog.selected_size;
            let bg = if is_sel { self.colors.accent } else { self.colors.chip_bg };
            let fg = if is_sel { 0x00FFFFFF } else { 0x00CCCCCC };

            if self.rounded_corners {
                fb.fill_rounded_rect(px, size_pills_y, pill_w, pill_h, 3, bg);
            } else {
                fb.fill_rect(px, size_pills_y, pill_w, pill_h, bg);
            }
            let label = format!("{}", s);
            let lx = px + (pill_w.saturating_sub(label.chars().count() * self.char_width())) / 2;
            fb.draw_text(&label, lx, size_pills_y + (pill_h.saturating_sub(self.char_height())) / 2, fg, None);
        }

        // 3. Weight / Grosor
        let weight_sec_y = size_pills_y + pill_h + self.scale_coord(8);
        fb.draw_text("GROSOR DE TRAZO (Font Weight)", left_x, weight_sec_y, self.colors.accent, None);
        let weight_pills_y = weight_sec_y + self.scale_coord(16);
        let weight_w = (col_w.saturating_sub(self.scale_coord(16))) / 3;

        for (idx, &w) in FontWeight::ALL.iter().enumerate() {
            let wx = left_x + idx * (weight_w + self.scale_coord(8));
            let is_sel = w == dialog.selected_weight;
            let bg = if is_sel { self.colors.button_bg } else { self.colors.chip_bg };
            let border = if is_sel { self.colors.accent } else { self.colors.scrollbar_border };
            let fg = if is_sel { 0x00FFFFFF } else { 0x00CCCCCC };

            if self.rounded_corners {
                fb.fill_rounded_rect(wx, weight_pills_y, weight_w, pill_h, 3, bg);
                fb.draw_rounded_outline(wx, weight_pills_y, weight_w, pill_h, 3, border);
            } else {
                fb.fill_rect(wx, weight_pills_y, weight_w, pill_h, bg);
            }
            let label = w.display_name();
            let lx = wx + (weight_w.saturating_sub(label.chars().count() * self.char_width())) / 2;
            fb.draw_text(label, lx, weight_pills_y + (pill_h.saturating_sub(self.char_height())) / 2, fg, None);
        }

        // 4. Advanced Typography Options
        let opts_sec_y = weight_pills_y + pill_h + self.scale_coord(8);
        fb.draw_text("CARACTERÍSTICAS TIPOGRÁFICAS", left_x, opts_sec_y, self.colors.accent, None);
        let opts_chips_y = opts_sec_y + self.scale_coord(16);

        let chip_lig_text = format!("Ligaduras: {}", if dialog.ligatures { "ON" } else { "OFF" });
        let chip_aa_text = format!("Anti-alias: {}", if dialog.antialiasing { "ON" } else { "OFF" });
        let chip_sp_text = format!("s: {}px", dialog.line_spacing);

        let chip_w = (col_w.saturating_sub(self.scale_coord(16))) / 3;
        let c0_x = left_x;
        let c1_x = c0_x + chip_w + self.scale_coord(8);
        let c2_x = c1_x + chip_w + self.scale_coord(8);

        let draw_opt_chip = |fb: &mut Framebuffer, cx: usize, cy: usize, cw: usize, text: &str, active: bool| {
            let bg = if active { self.colors.chip_bg } else { self.colors.modal_bg };
            let border = if active { self.colors.accent } else { self.colors.scrollbar_border };
            let fg = if active { self.colors.accent } else { 0x00888888 };
            if self.rounded_corners {
                fb.fill_rounded_rect(cx, cy, cw, pill_h, 3, bg);
                fb.draw_rounded_outline(cx, cy, cw, pill_h, 3, border);
            } else {
                fb.fill_rect(cx, cy, cw, pill_h, bg);
            }
            let tx = cx + (cw.saturating_sub(text.chars().count() * self.char_width())) / 2;
            fb.draw_text_clipped(text, tx, cy + (pill_h.saturating_sub(self.char_height())) / 2, cx + cw.saturating_sub(2), fg, None);
        };

        draw_opt_chip(fb, c0_x, opts_chips_y, chip_w, &chip_lig_text, dialog.ligatures);
        draw_opt_chip(fb, c1_x, opts_chips_y, chip_w, &chip_aa_text, dialog.antialiasing);
        draw_opt_chip(fb, c2_x, opts_chips_y, chip_w, &chip_sp_text, true);

        let opts_row2_y = opts_chips_y + pill_h + self.scale_coord(6);
        let chip2_w = (col_w.saturating_sub(self.scale_coord(16))) / 3;
        let r2_c0_x = left_x;
        let r2_c1_x = r2_c0_x + chip2_w + self.scale_coord(8);
        let r2_c2_x = r2_c1_x + chip2_w + self.scale_coord(8);

        let chip_let_text = format!("e: +{}px", dialog.letter_spacing);
        let chip_asp_text = format!("p: {:.2}", dialog.font_aspect_ratio);
        let chip_dark_text = if dialog.stem_darkening { "d: Engrosar" } else { "d: Estándar" };

        draw_opt_chip(fb, r2_c0_x, opts_row2_y, chip2_w, &chip_let_text, dialog.letter_spacing > 0);
        draw_opt_chip(fb, r2_c1_x, opts_row2_y, chip2_w, &chip_asp_text, (dialog.font_aspect_ratio - 0.50).abs() > 0.001);
        draw_opt_chip(fb, r2_c2_x, opts_row2_y, chip2_w, chip_dark_text, dialog.stem_darkening);

        let opts_row3_y = opts_row2_y + pill_h + self.scale_coord(6);
        let chip3_w = (col_w.saturating_sub(self.scale_coord(16))) / 3;
        let r3_c0_x = left_x;
        let r3_c1_x = r3_c0_x + chip3_w + self.scale_coord(8);
        let r3_c2_x = r3_c1_x + chip3_w + self.scale_coord(8);

        let chip_sdf_text = format!("f: SDF {}", if dialog.sdf_rendering { "ON" } else { "OFF" });
        let chip_opt_text = format!("o: Óptico {}", if dialog.optical_spacing { "ON" } else { "OFF" });
        let chip_ttf_text = format!("t: TTF {}", if dialog.use_ttf { "ON" } else { "OFF" });

        draw_opt_chip(fb, r3_c0_x, opts_row3_y, chip3_w, &chip_sdf_text, dialog.sdf_rendering);
        draw_opt_chip(fb, r3_c1_x, opts_row3_y, chip3_w, &chip_opt_text, dialog.optical_spacing);
        draw_opt_chip(fb, r3_c2_x, opts_row3_y, chip3_w, &chip_ttf_text, dialog.use_ttf);

        // === RIGHT COLUMN: Live Preview ===
        let prev_title = format!("VISTA PREVIA EN VIVO ({} pt • {})", dialog.selected_size, dialog.selected_family.display_name());
        fb.draw_text(&prev_title, right_x, content_y, self.colors.accent, None);

        let iso_card_h = self.scale_coord(76);
        let prev_box_y = content_y + self.scale_coord(16);
        let prev_box_h = content_h.saturating_sub(self.scale_coord(16) + iso_card_h + self.scale_coord(8));

        if self.rounded_corners {
            fb.fill_rounded_rect(right_x, prev_box_y, col_w, prev_box_h, 6, self.colors.input_bg);
            fb.draw_rounded_outline(right_x, prev_box_y, col_w, prev_box_h, 6, self.colors.modal_border);
        } else {
            fb.fill_rect(right_x, prev_box_y, col_w, prev_box_h, self.colors.input_bg);
        }

        // Render preview code sample inside preview box
        let sample_lines = [
            ("//! Vista previa en vivo con Rustext", self.colors.syntax_comment),
            ("fn benchmark_render() -> f64 {", self.colors.syntax_keyword),
            ("    let font = \"Rustext Mono\";", self.colors.syntax_plain),
            ("    let latency_us = 20;", self.colors.syntax_number),
            ("    if latency_us <= 50 && enabled {", self.colors.syntax_keyword),
            ("        println!(\"-> != => ==\");", self.colors.syntax_string),
            ("    }", self.colors.syntax_keyword),
            ("    488.6 // Throughput MB/s", self.colors.syntax_comment),
            ("}", self.colors.syntax_keyword),
        ];

        let prev_char_h = ((dialog.selected_size as f64 * 16.0 / 12.0).round() as usize).max(10);
        let prev_glyph_w = if dialog.font_aspect_ratio > 0.0 && (dialog.font_aspect_ratio - 0.50).abs() > 0.001 {
            ((prev_char_h as f64 * dialog.font_aspect_ratio).round() as usize).max(5)
        } else if dialog.selected_size != 12 {
            ((dialog.selected_size as f64 * 8.0 / 12.0).round() as usize).max(5)
        } else {
            FONT_WIDTH
        };
        let prev_char_w = prev_glyph_w + dialog.letter_spacing;
        let prev_line_h = prev_char_h + dialog.line_spacing;

        let prev_pad_x = right_x + self.scale_coord(12);
        let prev_pad_y = prev_box_y + self.scale_coord(14);

        for (line_idx, &(line_str, line_fg)) in sample_lines.iter().enumerate() {
            let ly = prev_pad_y + line_idx * prev_line_h;
            if ly + prev_char_h > prev_box_y + prev_box_h {
                break;
            }

            let dark_factor = if dialog.stem_darkening {
                StemDarkener::compute_darkening_factor(self.colors.input_bg, line_fg, dialog.selected_size, 1.0)
            } else {
                0.0
            };

            let line_chars: Vec<char> = line_str.chars().collect();

            if dialog.ligatures {
                let glyphs = FontShaper::shape_text_ex(
                    line_str,
                    1,
                    dialog.ligatures,
                    dialog.selected_family,
                    dialog.selected_weight,
                    prev_glyph_w,
                    prev_char_h,
                );
                let mut gx = prev_pad_x;
                let mut char_idx = 0;
                for g in glyphs {
                    if gx + g.width > right_x + col_w - self.scale_coord(8) {
                        break;
                    }
                    let ch = g.cluster.chars().next().unwrap_or(' ');
                    let is_box = FontRasterizer::is_box_drawing(ch);
                    let cluster_len = g.cluster.chars().count();

                    let prev_char = if char_idx > 0 { line_chars.get(char_idx - 1).copied() } else { None };
                    let next_char = line_chars.get(char_idx + cluster_len).copied();
                    char_idx += cluster_len;

                    let opt_offset = if dialog.optical_spacing && !is_box && !g.is_ligature {
                        OpticalPunctuationSpacing::compute_optical_offset(prev_char, ch, next_char, prev_char_w, 1)
                    } else {
                        0
                    };

                    let total_w = prev_char_w * g.advance_cells;
                    let (draw_x, draw_w) = if is_box {
                        (gx, prev_char_w)
                    } else {
                        let safe_x = OpticalPunctuationSpacing::compute_safe_in_cell_draw_x(gx, total_w, g.width, opt_offset);
                        (safe_x, g.width)
                    };
                    let max_w = draw_w.min(fb.width.saturating_sub(draw_x));
                    let max_h = g.height.min(fb.height.saturating_sub(ly));
                    let effective_cov = if is_box && draw_w != g.width {
                        let (_, _, box_cov) = FontRasterizer::rasterize_glyph_sized(ch, dialog.selected_family, draw_w, g.height, dialog.selected_weight);
                        box_cov
                    } else if dialog.use_ttf && !g.is_ligature {
                        let ttf_font = crate::gfx::ttf::SystemFontLoader::get_active_font();
                        let (_, _, ttf_cov) = crate::gfx::ttf::TtfRasterizer::rasterize_char_cached(&ttf_font, ch, draw_w, g.height);
                        if dark_factor > 0.001 {
                            StemDarkener::darken_coverage_mask(&ttf_cov, draw_w, g.height, dark_factor)
                        } else {
                            ttf_cov.to_vec()
                        }
                    } else if dialog.sdf_rendering && !g.is_ligature {
                        let (_, _, sdf_cov) = SdfFontRasterizer::rasterize_glyph_sdf(ch, dialog.selected_family, draw_w, g.height, dialog.selected_weight, 0.0);
                        if dark_factor > 0.001 {
                            StemDarkener::darken_coverage_mask(&sdf_cov, draw_w, g.height, dark_factor)
                        } else {
                            sdf_cov
                        }
                    } else if dark_factor > 0.001 && !is_box {
                        StemDarkener::darken_coverage_mask(&g.coverage, g.width, g.height, dark_factor)
                    } else {
                        g.coverage.clone()
                    };
                    for row in 0..max_h {
                        let py = ly + row;
                        let row_offset = row * draw_w;
                        let fb_offset = py * fb.width + draw_x;
                        for col in 0..max_w {
                            let alpha = effective_cov[row_offset + col];
                            if alpha > 0 {
                                fb.pixels[fb_offset + col] = blend_pixel_gamma(self.colors.input_bg, line_fg, alpha);
                            }
                        }
                    }
                    gx += g.advance_cells * prev_char_w;
                }
            } else {
                let mut cur_gx = prev_pad_x;
                for (char_idx, ch) in line_str.chars().enumerate() {
                    if cur_gx + prev_char_w > right_x + col_w - self.scale_coord(8) {
                        break;
                    }
                    let is_box = FontRasterizer::is_box_drawing(ch);

                    let prev_char = if char_idx > 0 { line_chars.get(char_idx - 1).copied() } else { None };
                    let next_char = line_chars.get(char_idx + 1).copied();

                    let opt_offset = if dialog.optical_spacing && !is_box {
                        OpticalPunctuationSpacing::compute_optical_offset(prev_char, ch, next_char, prev_char_w, 1)
                    } else {
                        0
                    };

                    let (draw_x, target_w) = if is_box {
                        (cur_gx, prev_char_w)
                    } else {
                        let safe_x = OpticalPunctuationSpacing::compute_safe_in_cell_draw_x(cur_gx, prev_char_w, prev_glyph_w, opt_offset);
                        (safe_x, prev_glyph_w)
                    };
                    if self.subpixel_antialiasing {
                        let (gw, gh, mut sub_cov) = if dialog.use_ttf && !is_box {
                            let ttf_font = crate::gfx::ttf::SystemFontLoader::get_active_font();
                            let (w, h, sub) = crate::gfx::ttf::TtfRasterizer::rasterize_char_subpixel_cached(
                                &ttf_font,
                                ch,
                                target_w,
                                prev_char_h,
                            );
                            (w, h, sub.to_vec())
                        } else if dialog.sdf_rendering && !is_box {
                            SdfFontRasterizer::rasterize_glyph_sdf_subpixel(
                                ch,
                                dialog.selected_family,
                                target_w,
                                prev_char_h,
                                dialog.selected_weight,
                                0.0,
                            )
                        } else {
                            FontRasterizer::rasterize_glyph_subpixel(
                                ch,
                                dialog.selected_family,
                                target_w,
                                prev_char_h,
                                dialog.selected_weight,
                            )
                        };
                        if dark_factor > 0.001 && !is_box {
                            sub_cov = StemDarkener::darken_subpixel_mask(&sub_cov, gw, gh, dark_factor);
                        }
                        let max_w = gw.min(fb.width.saturating_sub(draw_x));
                        let max_h = gh.min(fb.height.saturating_sub(ly));
                        for row in 0..max_h {
                            let py = ly + row;
                            let row_offset = row * gw;
                            let fb_offset = py * fb.width + draw_x;
                            for col in 0..max_w {
                                let [ar, ag, ab] = sub_cov[row_offset + col];
                                if ar > 0 || ag > 0 || ab > 0 {
                                    fb.pixels[fb_offset + col] = blend_pixel_subpixel_gamma(self.colors.input_bg, line_fg, ar, ag, ab);
                                }
                            }
                        }
                    } else {
                        let (gw, gh, mut cov) = if dialog.use_ttf && !is_box {
                            let ttf_font = crate::gfx::ttf::SystemFontLoader::get_active_font();
                            let (w, h, cov) = crate::gfx::ttf::TtfRasterizer::rasterize_char_cached(
                                &ttf_font,
                                ch,
                                target_w,
                                prev_char_h,
                            );
                            (w, h, cov.to_vec())
                        } else if dialog.sdf_rendering && !is_box {
                            SdfFontRasterizer::rasterize_glyph_sdf(
                                ch,
                                dialog.selected_family,
                                target_w,
                                prev_char_h,
                                dialog.selected_weight,
                                0.0,
                            )
                        } else {
                            FontRasterizer::rasterize_glyph_sized(
                                ch,
                                dialog.selected_family,
                                target_w,
                                prev_char_h,
                                dialog.selected_weight,
                            )
                        };
                        if dark_factor > 0.001 && !is_box {
                            cov = StemDarkener::darken_coverage_mask(&cov, gw, gh, dark_factor);
                        }
                        let max_w = gw.min(fb.width.saturating_sub(draw_x));
                        let max_h = gh.min(fb.height.saturating_sub(ly));
                        for row in 0..max_h {
                            let py = ly + row;
                            let row_offset = row * gw;
                            let fb_offset = py * fb.width + draw_x;
                            for col in 0..max_w {
                                let alpha = cov[row_offset + col];
                                if alpha > 0 {
                                    fb.pixels[fb_offset + col] = blend_pixel_gamma(self.colors.input_bg, line_fg, alpha);
                                }
                            }
                        }
                    }
                    cur_gx += prev_char_w;
                }
            }
        }

        // === ISO 9241-303 Ergonomics & HiDPI Card ===
        let iso_card_y = prev_box_y + prev_box_h + self.scale_coord(8);
        let metrics = crate::gfx::HiDpiDetector::detect_display_metrics();
        let effective_scale = if dialog.scale_factor > 0.0 {
            dialog.scale_factor
        } else if dialog.ergonomic_scaling {
            crate::gfx::HiDpiDetector::compute_ergonomic_scale(
                metrics.dpi,
                metrics.default_viewing_distance_mm,
                crate::gfx::HiDpiDetector::ISO_RECOMMENDED_ARCMIN,
            )
        } else {
            self.scale_factor
        };

        let char_height_logical = ((dialog.selected_size as f64 * 16.0 / 12.0).round() as usize).max(10);
        let legibility = crate::gfx::HiDpiDetector::assess_legibility(
            char_height_logical,
            effective_scale,
            metrics.dpi,
            metrics.default_viewing_distance_mm,
        );

        let card_bg = self.colors.modal_header_bg;
        let card_border = legibility.rating.badge_color();

        if self.rounded_corners {
            fb.fill_rounded_rect(right_x, iso_card_y, col_w, iso_card_h, 4, card_bg);
            fb.draw_rounded_outline(right_x, iso_card_y, col_w, iso_card_h, 4, card_border);
        } else {
            fb.fill_rect(right_x, iso_card_y, col_w, iso_card_h, card_bg);
        }

        let pad_x = right_x + self.scale_coord(10);
        let max_card_x = right_x + col_w - self.scale_coord(8);
        let l1_y = iso_card_y + self.scale_coord(6);
        let l2_y = l1_y + self.scale_coord(16);
        let l3_y = l2_y + self.scale_coord(16);
        let l4_y = l3_y + self.scale_coord(16);

        // Line 1: Header + Rating badge cleanly separated
        fb.draw_text("ERGONOMÍA VISUAL", pad_x, l1_y, self.colors.accent, None);
        let badge_str = format!("{:.1}' [{}]", legibility.visual_angle_arcmin, legibility.rating.badge_label());
        let badge_char_w = self.char_width().max(1);
        let badge_w = badge_str.chars().count() * badge_char_w;
        let badge_x = right_x + col_w - self.scale_coord(10) - badge_w;
        let min_badge_x = pad_x + 18 * badge_char_w;
        if badge_x >= min_badge_x {
            fb.draw_text_clipped(&badge_str, badge_x, l1_y, max_card_x, legibility.rating.badge_color(), None);
        }

        // Line 2: Display Density & Active Scale
        let l2_str = format!(
            "Pantalla: {:.0} DPI ({:.1}\") • HiDPI: {}",
            metrics.dpi,
            metrics.diagonal_inches,
            dialog.scale_label()
        );
        fb.draw_text_truncated(&l2_str, pad_x, l2_y, max_card_x, 0x00CCCCCC, None);

        // Line 3: Physical character size & viewing distance
        let l3_str = format!(
            "Física: {:.2}mm • Dist: {}cm • Óptima: {:.2}x",
            legibility.physical_height_mm,
            (metrics.default_viewing_distance_mm / 10.0).round() as usize,
            legibility.recommended_scale
        );
        fb.draw_text_truncated(&l3_str, pad_x, l3_y, max_card_x, 0x00999999, None);

        // Line 4: Ergonomic recommendation
        fb.draw_text_truncated(legibility.description, pad_x, l4_y, max_card_x, legibility.rating.badge_color(), None);

        // === FOOTER BAR: Buttons & Key Hints ===
        let footer_y = win_y + win_h - footer_h;
        fb.fill_rect(win_x, footer_y, win_w, 1, self.colors.scrollbar_border);

        let btn_cancel_w = self.scale_coord(95);
        let btn_apply_w = self.scale_coord(145);
        let btn_h = self.scale_coord(26);
        let btn_y = footer_y + (footer_h.saturating_sub(btn_h)) / 2;

        let btn_apply_x = win_x + win_w - btn_apply_w - self.scale_coord(14);
        let btn_cancel_x = btn_apply_x - btn_cancel_w - self.scale_coord(10);

        let max_hint_x = btn_cancel_x.saturating_sub(self.scale_coord(12));
        let hint = "Tab: Sección  •  +/-: Zoom  •  Enter: Guardar  •  Esc: Salir";
        fb.draw_text_truncated(hint, win_x + self.scale_coord(14), footer_y + (footer_h.saturating_sub(self.char_height())) / 2, max_hint_x, self.colors.gutter_fg, None);

        let is_cancel_focused = dialog.focus_section == 4 && dialog.selected_button == 1;
        let is_apply_focused = dialog.focus_section == 4 && dialog.selected_button == 0;

        let cancel_bg = if is_cancel_focused || dialog.hovered_button == Some(1) {
            self.colors.button_hover_bg
        } else {
            self.colors.chip_bg
        };

        let apply_bg = if is_apply_focused || dialog.hovered_button == Some(0) {
            self.colors.button_hover_bg
        } else {
            self.colors.button_bg
        };

        if self.rounded_corners {
            fb.fill_rounded_rect(btn_cancel_x, btn_y, btn_cancel_w, btn_h, 4, cancel_bg);
            fb.draw_rounded_outline(btn_cancel_x, btn_y, btn_cancel_w, btn_h, 4, self.colors.scrollbar_border);
            fb.fill_rounded_rect(btn_apply_x, btn_y, btn_apply_w, btn_h, 4, apply_bg);
            fb.draw_rounded_outline(btn_apply_x, btn_y, btn_apply_w, btn_h, 4, self.colors.accent);
        } else {
            fb.fill_rect(btn_cancel_x, btn_y, btn_cancel_w, btn_h, cancel_bg);
            fb.fill_rect(btn_apply_x, btn_y, btn_apply_w, btn_h, apply_bg);
        }

        let ctx = btn_cancel_x + (btn_cancel_w.saturating_sub("Cancelar".chars().count() * self.char_width())) / 2;
        let atx = btn_apply_x + (btn_apply_w.saturating_sub("Aplicar y Guardar".chars().count() * self.char_width())) / 2;
        fb.draw_text("Cancelar", ctx, btn_y + (btn_h.saturating_sub(self.char_height())) / 2, 0x00CCCCCC, None);
        fb.draw_text("Aplicar y Guardar", atx, btn_y + (btn_h.saturating_sub(self.char_height())) / 2, 0x00FFFFFF, None);
    }

    /// Computes the bounding geometry for the docked bottom terminal pane.
    pub fn terminal_geometry(&self, fb_w: usize, fb_h: usize, sidebar_w: usize, term_h: usize) -> TerminalGeometry {
        let x = sidebar_w;
        let width = fb_w.saturating_sub(sidebar_w);
        let height = term_h.min(fb_h.saturating_sub(self.menu_bar_height + self.tab_bar_height + self.status_bar_height + self.scale_coord(40)));
        let y = fb_h.saturating_sub(self.status_bar_height + height);

        let splitter_y = y;
        let splitter_h = self.scale_coord(4).max(2);

        let header_y = y + splitter_h;
        let header_h = (self.char_height() + self.scale_coord(8)).max(self.scale_coord(24));

        let content_y = header_y + header_h;
        let content_h = height.saturating_sub(splitter_h + header_h);

        let btn_close_w = self.scale_coord(26);
        let btn_close_x = x + width.saturating_sub(btn_close_w + self.scale_coord(6));

        let btn_clear_w = self.scale_coord(68);
        let btn_clear_x = btn_close_x.saturating_sub(btn_clear_w + self.scale_coord(6));

        TerminalGeometry {
            x,
            y,
            width,
            height,
            splitter_y,
            splitter_h,
            header_y,
            header_h,
            content_y,
            content_h,
            btn_clear_x,
            btn_clear_w,
            btn_close_x,
            btn_close_w,
        }
    }

    /// Renders the integrated bottom terminal panel.
    pub fn render_terminal_panel(
        &self,
        fb: &mut Framebuffer,
        terminal: &crate::editor::terminal_panel::TerminalPanel,
        geom: &TerminalGeometry,
        hovered_clear: bool,
        hovered_close: bool,
    ) {
        if geom.width == 0 || geom.height == 0 {
            return;
        }

        // 1. Splitter bar (Drag Handle)
        let splitter_bg = if terminal.is_focused { self.colors.accent } else { self.colors.gutter_border };
        fb.fill_rect(geom.x, geom.splitter_y, geom.width, geom.splitter_h, splitter_bg);

        // 2. Header Bar
        let header_bg = if terminal.is_focused { self.colors.tab_active_bg } else { self.colors.modal_header_bg };
        fb.fill_rect(geom.x, geom.header_y, geom.width, geom.header_h, header_bg);
        fb.fill_rect(geom.x, geom.header_y + geom.header_h - 1, geom.width, 1, self.colors.gutter_border);

        // Header Title: [>_ TERMINAL (bash)]
        let title_icon = ">_ ";
        let title_text = format!("TERMINAL ({})", terminal.shell_name);
        let title_x = geom.x + self.scale_coord(8);
        let title_y = geom.header_y + (geom.header_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(title_icon, title_x, title_y, self.colors.accent, None);
        fb.draw_text(&title_text, title_x + title_icon.len() * self.char_width(), title_y, 0x00FFFFFF, None);

        // Working dir hint
        let dir_str = terminal.working_dir.display().to_string();
        let max_dir_chars = geom.btn_clear_x.saturating_sub(title_x + self.scale_coord(180)) / self.char_width().max(1);
        if max_dir_chars > 8 {
            let truncated_dir = if dir_str.chars().count() > max_dir_chars {
                let tail_len = max_dir_chars.saturating_sub(3);
                let start_idx = dir_str.len().saturating_sub(tail_len);
                format!("...{}", &dir_str[start_idx..])
            } else {
                dir_str
            };
            fb.draw_text(&truncated_dir, title_x + self.scale_coord(180), title_y, 0x00858585, None);
        }

        // [Limpiar] Button
        let btn_clear_bg = if hovered_clear { self.colors.button_hover_bg } else { self.colors.chip_bg };
        let btn_clear_fg = if hovered_clear { 0x00FFFFFF } else { 0x00CCCCCC };
        let btn_y = geom.header_y + self.scale_coord(2);
        let btn_h = geom.header_h.saturating_sub(self.scale_coord(4));
        if self.rounded_corners {
            fb.fill_rounded_rect(geom.btn_clear_x, btn_y, geom.btn_clear_w, btn_h, 3, btn_clear_bg);
        } else {
            fb.fill_rect(geom.btn_clear_x, btn_y, geom.btn_clear_w, btn_h, btn_clear_bg);
        }
        let clear_label = "Limpiar";
        let clear_lbl_x = geom.btn_clear_x + (geom.btn_clear_w.saturating_sub(clear_label.len() * self.char_width())) / 2;
        fb.draw_text(clear_label, clear_lbl_x, btn_y + (btn_h.saturating_sub(self.char_height())) / 2, btn_clear_fg, None);

        // [x] Close Button
        let btn_close_bg = if hovered_close { 0x00C72C41 } else { self.colors.chip_bg };
        let btn_close_fg = if hovered_close { 0x00FFFFFF } else { 0x00CCCCCC };
        if self.rounded_corners {
            fb.fill_rounded_rect(geom.btn_close_x, btn_y, geom.btn_close_w, btn_h, 3, btn_close_bg);
        } else {
            fb.fill_rect(geom.btn_close_x, btn_y, geom.btn_close_w, btn_h, btn_close_bg);
        }
        let x_label = "x";
        let x_lbl_x = geom.btn_close_x + (geom.btn_close_w.saturating_sub(x_label.len() * self.char_width())) / 2;
        fb.draw_text(x_label, x_lbl_x, btn_y + (btn_h.saturating_sub(self.char_height())) / 2, btn_close_fg, None);

        // 3. Terminal Content Area (Background)
        let term_bg = 0x00121212;
        fb.fill_rect(geom.x, geom.content_y, geom.width, geom.content_h, term_bg);

        // Output lines & input line
        let line_height = self.line_height();
        let visible_lines = geom.content_h / line_height.max(1);
        if visible_lines == 0 {
            return;
        }

        let output_visible_lines = visible_lines.saturating_sub(1);
        let total_lines = terminal.lines.len();
        let scroll_end = terminal.scroll_offset.min(total_lines);
        let start_line = scroll_end.saturating_sub(output_visible_lines);

        let pad_x = geom.x + self.scale_coord(8);
        let mut cur_y = geom.content_y + self.scale_coord(4);

        for line_idx in start_line..scroll_end {
            if cur_y + line_height > geom.content_y + geom.content_h - line_height {
                break;
            }
            let line = &terminal.lines[line_idx];
            if line.starts_with("❯ ") {
                fb.draw_text("❯ ", pad_x, cur_y, self.colors.accent, None);
                fb.draw_text(&line[2..], pad_x + 2 * self.char_width(), cur_y, 0x00FFFFFF, None);
            } else if line.starts_with("[Error") {
                fb.draw_text(line, pad_x, cur_y, 0x00F44747, None);
            } else {
                fb.draw_text(line, pad_x, cur_y, 0x00CCCCCC, None);
            }
            cur_y += line_height;
        }

        // 4. Active Command Input Line (at the bottom of content)
        let input_y = geom.content_y + geom.content_h.saturating_sub(line_height + self.scale_coord(2));
        fb.draw_text("❯ ", pad_x, input_y, self.colors.accent, None);

        let prompt_prefix_w = 2 * self.char_width();
        let input_text_x = pad_x + prompt_prefix_w;
        fb.draw_text(&terminal.current_input, input_text_x, input_y, 0x00FFFFFF, None);

        // Draw cursor if focused
        if terminal.is_focused {
            let cursor_x = input_text_x + terminal.cursor_pos * self.char_width();
            let cursor_w = self.char_width();
            let cursor_h = self.char_height();
            fb.fill_rect(cursor_x, input_y, cursor_w, cursor_h, self.colors.cursor);
            if terminal.cursor_pos < terminal.current_input.chars().count() {
                let under_char: String = terminal.current_input.chars().skip(terminal.cursor_pos).take(1).collect();
                fb.draw_text(&under_char, cursor_x, input_y, 0x00000000, None);
            }
        } else if terminal.current_input.is_empty() {
            fb.draw_text("(Terminal inactivo - Haga clic para enfocar)", input_text_x + self.scale_coord(8), input_y, 0x00555555, None);
        }
    }
}

/// Geometric bounds of the docked terminal panel and its subcomponents.
#[derive(Debug, Clone, Copy)]
pub struct TerminalGeometry {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub splitter_y: usize,
    pub splitter_h: usize,
    pub header_y: usize,
    pub header_h: usize,
    pub content_y: usize,
    pub content_h: usize,
    pub btn_clear_x: usize,
    pub btn_clear_w: usize,
    pub btn_close_x: usize,
    pub btn_close_w: usize,
}

impl TerminalGeometry {
    #[inline]
    pub fn contains_point(&self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    #[inline]
    pub fn is_over_splitter(&self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.splitter_y && y < self.splitter_y + self.splitter_h
    }

    #[inline]
    pub fn is_over_clear_btn(&self, x: usize, y: usize) -> bool {
        x >= self.btn_clear_x && x < self.btn_clear_x + self.btn_clear_w && y >= self.header_y && y < self.header_y + self.header_h
    }

    #[inline]
    pub fn is_over_close_btn(&self, x: usize, y: usize) -> bool {
        x >= self.btn_close_x && x < self.btn_close_x + self.btn_close_w && y >= self.header_y && y < self.header_y + self.header_h
    }
}

impl Default for GfxRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::buffer::TextBuffer;
    use crate::ui::prompt::{PromptKind, PromptModal};

    #[test]
    fn test_gfx_render_frame() {
        let mut fb = Framebuffer::new(800, 600);
        let doc = Document::new(TextBuffer::from_str("fn main() {\n    println!(\"Hello\");\n}"));
        let renderer = GfxRenderer::new();

        let tabs = vec!["main.rs".to_string()];
        renderer.render(&mut fb, &doc, &tabs, 0, Some("Ready"), None, None);

        // Verify non-zero pixels rendered
        let colored_pixels = fb.pixels.iter().filter(|&&p| p != 0x001E1E1E).count();
        assert!(colored_pixels > 100);
    }

    #[test]
    fn test_gfx_render_modal_overlay() {
        let mut fb = Framebuffer::new(800, 600);
        let doc = Document::new(TextBuffer::from_str("line 1\nline 2"));
        let renderer = GfxRenderer::new();
        let tabs = vec!["test.txt".to_string()];

        let mut modal = PromptModal::new(PromptKind::Find);
        modal.insert_char('r');
        modal.insert_char('u');
        modal.insert_char('s');
        modal.insert_char('t');

        renderer.render(&mut fb, &doc, &tabs, 0, None, Some(&modal), None);
        // Verify prompt modal overlay drew pixels
        let prompt_box_pixel = fb.get_pixel(400, 55);
        assert_eq!(prompt_box_pixel, 0x00252526);
    }

    #[test]
    fn test_gfx_render_menu_dropdown() {
        let mut fb = Framebuffer::new(800, 600);
        let doc = Document::new(TextBuffer::from_str("let x = 42;"));
        let renderer = GfxRenderer::new();
        let tabs = vec!["code.rs".to_string()];

        let mut menu_bar = crate::editor::menu::MenuBar::new();
        menu_bar.active_category = Some(0); // Open "Archivo"

        renderer.render(&mut fb, &doc, &tabs, 0, None, None, Some(&menu_bar));
        // Dropdown background pixel
        let drop_pixel = fb.get_pixel(15, 30);
        assert_eq!(drop_pixel, 0x00252526);
    }
}
