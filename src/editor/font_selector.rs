//! Interactive Font Selector and Typography Customization Modal Dialog.
//! Pure standard-library Rust with zero third-party dependencies.

use crate::gfx::font::{FontFamily, FontWeight};
use crate::gfx::HintingLevel;
use crate::ui::terminal::Key;

pub const AVAILABLE_SIZES: [usize; 8] = [10, 11, 12, 14, 16, 18, 20, 24];
pub const AVAILABLE_SPACINGS: [usize; 4] = [1, 3, 5, 8];
pub const AVAILABLE_LETTER_SPACINGS: [usize; 4] = [0, 1, 2, 3];
pub const AVAILABLE_ASPECT_RATIOS: [f64; 4] = [0.50, 0.55, 0.60, 0.65];
pub const AVAILABLE_SCALES: [f64; 7] = [0.0, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5];

/// State and geometry for the interactive desktop font selector dialog.
#[derive(Debug, Clone, PartialEq)]
pub struct FontSelectorDialog {
    pub selected_family: FontFamily,
    pub selected_size: usize,
    pub selected_weight: FontWeight,
    pub line_spacing: usize,
    pub ligatures: bool,
    pub antialiasing: bool,
    pub hinting: HintingLevel,
    pub letter_spacing: usize,
    pub font_aspect_ratio: f64,
    pub stem_darkening: bool,
    pub scale_factor: f64,
    pub ergonomic_scaling: bool,
    pub sdf_rendering: bool,
    pub optical_spacing: bool,
    pub use_ttf: bool,
    /// Focused UI section:
    /// 0 = Family List
    /// 1 = Size Selector
    /// 2 = Weight Selector
    /// 3 = Advanced Options (Ligatures / AA / Spacing / Aspect / Stem / HiDPI / SDF / Optic / TTF)
    /// 4 = Action Buttons (Apply / Cancel)
    pub focus_section: usize,
    pub family_idx: usize,
    pub size_idx: usize,
    pub weight_idx: usize,
    pub selected_button: usize, // 0 = Apply, 1 = Cancel
    pub hovered_family: Option<usize>,
    pub hovered_size: Option<usize>,
    pub hovered_weight: Option<usize>,
    pub hovered_button: Option<usize>,
}

impl Eq for FontSelectorDialog {}

impl FontSelectorDialog {
    /// Creates a new font selector initialized with current active editor settings.
    pub fn new(
        family: FontFamily,
        size: usize,
        weight: FontWeight,
        line_spacing: usize,
        ligatures: bool,
        antialiasing: bool,
        hinting: HintingLevel,
        letter_spacing: usize,
        font_aspect_ratio: f64,
        stem_darkening: bool,
        scale_factor: f64,
        ergonomic_scaling: bool,
        sdf_rendering: bool,
        optical_spacing: bool,
        use_ttf: bool,
    ) -> Self {
        let family_idx = FontFamily::ALL
            .iter()
            .position(|&f| f == family)
            .unwrap_or(0);

        let size_idx = AVAILABLE_SIZES
            .iter()
            .position(|&s| s == size)
            .unwrap_or(2); // default 12pt is index 2

        let weight_idx = FontWeight::ALL
            .iter()
            .position(|&w| w == weight)
            .unwrap_or(0);

        Self {
            selected_family: family,
            selected_size: size,
            selected_weight: weight,
            line_spacing,
            ligatures,
            antialiasing,
            hinting,
            letter_spacing,
            font_aspect_ratio: if font_aspect_ratio > 0.0 { font_aspect_ratio } else { 0.50 },
            stem_darkening,
            scale_factor,
            ergonomic_scaling,
            sdf_rendering,
            optical_spacing,
            use_ttf,
            focus_section: 0,
            family_idx,
            size_idx,
            weight_idx,
            selected_button: 0,
            hovered_family: None,
            hovered_size: None,
            hovered_weight: None,
            hovered_button: None,
        }
    }

    /// Selects a font family by index.
    pub fn select_family_by_idx(&mut self, idx: usize) {
        if idx < FontFamily::ALL.len() {
            self.family_idx = idx;
            self.selected_family = FontFamily::ALL[idx];
        }
    }

    /// Advances to the next font family.
    pub fn select_next_family(&mut self) {
        let next = (self.family_idx + 1) % FontFamily::ALL.len();
        self.select_family_by_idx(next);
    }

    /// Moves to the previous font family.
    pub fn select_prev_family(&mut self) {
        let prev = if self.family_idx == 0 {
            FontFamily::ALL.len() - 1
        } else {
            self.family_idx - 1
        };
        self.select_family_by_idx(prev);
    }

    /// Selects a font size by index in AVAILABLE_SIZES.
    pub fn select_size_by_idx(&mut self, idx: usize) {
        if idx < AVAILABLE_SIZES.len() {
            self.size_idx = idx;
            self.selected_size = AVAILABLE_SIZES[idx];
        }
    }

    /// Advances to the next font size.
    pub fn select_next_size(&mut self) {
        if self.size_idx + 1 < AVAILABLE_SIZES.len() {
            self.select_size_by_idx(self.size_idx + 1);
        }
    }

    /// Moves to the previous font size.
    pub fn select_prev_size(&mut self) {
        if self.size_idx > 0 {
            self.select_size_by_idx(self.size_idx - 1);
        }
    }

    /// Selects font weight by index.
    pub fn select_weight_by_idx(&mut self, idx: usize) {
        if idx < FontWeight::ALL.len() {
            self.weight_idx = idx;
            self.selected_weight = FontWeight::ALL[idx];
        }
    }

    /// Cycles through font weights.
    pub fn cycle_weight(&mut self) {
        let next = (self.weight_idx + 1) % FontWeight::ALL.len();
        self.select_weight_by_idx(next);
    }

    /// Cycles through available line spacing presets (1px, 3px, 5px, 8px).
    pub fn cycle_line_spacing(&mut self) {
        let current_pos = AVAILABLE_SPACINGS
            .iter()
            .position(|&s| s == self.line_spacing)
            .unwrap_or(1);
        let next_pos = (current_pos + 1) % AVAILABLE_SPACINGS.len();
        self.line_spacing = AVAILABLE_SPACINGS[next_pos];
    }

    /// Toggles programming ligatures on/off.
    pub fn toggle_ligatures(&mut self) {
        self.ligatures = !self.ligatures;
    }

    /// Toggles subpixel anti-aliasing on/off.
    pub fn toggle_antialiasing(&mut self) {
        self.antialiasing = !self.antialiasing;
    }

    /// Cycles through font hinting levels (None -> Slight -> Medium -> Full).
    pub fn cycle_hinting(&mut self) {
        self.hinting = match self.hinting {
            HintingLevel::None => HintingLevel::Slight,
            HintingLevel::Slight => HintingLevel::Medium,
            HintingLevel::Medium => HintingLevel::Full,
            HintingLevel::Full => HintingLevel::None,
        };
    }

    /// Cycles through available letter spacing presets (0px, 1px, 2px, 3px).
    pub fn cycle_letter_spacing(&mut self) {
        let current_pos = AVAILABLE_LETTER_SPACINGS
            .iter()
            .position(|&s| s == self.letter_spacing)
            .unwrap_or(0);
        let next_pos = (current_pos + 1) % AVAILABLE_LETTER_SPACINGS.len();
        self.letter_spacing = AVAILABLE_LETTER_SPACINGS[next_pos];
    }

    /// Cycles through available ergonomic aspect ratio presets (0.50, 0.55, 0.60, 0.65).
    pub fn cycle_aspect_ratio(&mut self) {
        let current_pos = AVAILABLE_ASPECT_RATIOS
            .iter()
            .position(|&a| (a - self.font_aspect_ratio).abs() < 0.02)
            .unwrap_or(0);
        let next_pos = (current_pos + 1) % AVAILABLE_ASPECT_RATIOS.len();
        self.font_aspect_ratio = AVAILABLE_ASPECT_RATIOS[next_pos];
    }

    /// Returns a human-friendly name for current font aspect ratio.
    pub fn aspect_ratio_name(&self) -> &'static str {
        if (self.font_aspect_ratio - 0.50).abs() < 0.02 {
            "Clásico (1:2)"
        } else if (self.font_aspect_ratio - 0.55).abs() < 0.02 {
            "Equilibrado"
        } else if (self.font_aspect_ratio - 0.60).abs() < 0.02 {
            "Confort (3:5)"
        } else if (self.font_aspect_ratio - 0.65).abs() < 0.02 {
            "Amplio"
        } else {
            "Personalizado"
        }
    }

    /// Toggles optical stem darkening in dark mode on/off.
    pub fn toggle_stem_darkening(&mut self) {
        self.stem_darkening = !self.stem_darkening;
    }

    /// Cycles through available HiDPI scale factor presets (0.0=Auto, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5).
    pub fn cycle_scale_factor(&mut self) {
        let current_pos = AVAILABLE_SCALES
            .iter()
            .position(|&s| (s - self.scale_factor).abs() < 0.02)
            .unwrap_or(0);
        let next_pos = (current_pos + 1) % AVAILABLE_SCALES.len();
        self.scale_factor = AVAILABLE_SCALES[next_pos];
    }

    /// Toggles ISO 9241-303 ergonomic auto-scaling mode.
    pub fn toggle_ergonomic_scaling(&mut self) {
        self.ergonomic_scaling = !self.ergonomic_scaling;
    }

    /// Returns a human-friendly label for current scale factor.
    pub fn scale_label(&self) -> String {
        if self.scale_factor == 0.0 {
            "Auto (ISO)".to_string()
        } else {
            format!("{:.2}x", self.scale_factor)
        }
    }

    /// Toggles high-resolution Signed Distance Field (SDF) rasterization.
    pub fn toggle_sdf_rendering(&mut self) {
        self.sdf_rendering = !self.sdf_rendering;
    }

    /// Toggles dynamic optical punctuation spacing.
    pub fn toggle_optical_spacing(&mut self) {
        self.optical_spacing = !self.optical_spacing;
    }

    /// Toggles pure Rust TrueType (.ttf) vector outline rasterization.
    pub fn toggle_ttf_rendering(&mut self) {
        self.use_ttf = !self.use_ttf;
    }

    /// Handles keyboard events inside the font selector dialog.
    /// Returns `Some(true)` to apply, `Some(false)` to cancel, or `None` to stay open.
    pub fn handle_key(&mut self, key: Key) -> Option<bool> {
        match key {
            Key::Esc => Some(false),
            Key::Enter => {
                if self.focus_section == 4 && self.selected_button == 1 {
                    Some(false)
                } else {
                    Some(true)
                }
            }
            Key::Tab => {
                self.focus_section = (self.focus_section + 1) % 5;
                None
            }
            Key::BackTab => {
                self.focus_section = if self.focus_section == 0 { 4 } else { self.focus_section - 1 };
                None
            }
            Key::Up => {
                match self.focus_section {
                    0 => self.select_prev_family(),
                    1 => self.select_prev_size(),
                    2 => self.cycle_weight(),
                    3 => self.cycle_line_spacing(),
                    4 => self.selected_button = 0,
                    _ => {}
                }
                None
            }
            Key::Down => {
                match self.focus_section {
                    0 => self.select_next_family(),
                    1 => self.select_next_size(),
                    2 => self.cycle_weight(),
                    3 => self.cycle_line_spacing(),
                    4 => self.selected_button = 1,
                    _ => {}
                }
                None
            }
            Key::Left => {
                match self.focus_section {
                    1 => self.select_prev_size(),
                    2 => {
                        let prev = if self.weight_idx == 0 { FontWeight::ALL.len() - 1 } else { self.weight_idx - 1 };
                        self.select_weight_by_idx(prev);
                    }
                    4 => self.selected_button = 0,
                    _ => {}
                }
                None
            }
            Key::Right => {
                match self.focus_section {
                    1 => self.select_next_size(),
                    2 => {
                        let next = (self.weight_idx + 1) % FontWeight::ALL.len();
                        self.select_weight_by_idx(next);
                    }
                    4 => self.selected_button = 1,
                    _ => {}
                }
                None
            }
            Key::Char('+') | Key::Char('=') => {
                self.select_next_size();
                None
            }
            Key::Char('-') => {
                self.select_prev_size();
                None
            }
            Key::Char('0') => {
                self.selected_size = 12;
                self.size_idx = 2;
                None
            }
            Key::Char('l') | Key::Char('L') => {
                self.toggle_ligatures();
                None
            }
            Key::Char('a') | Key::Char('A') => {
                self.toggle_antialiasing();
                None
            }
            Key::Char('h') | Key::Char('H') => {
                self.cycle_hinting();
                None
            }
            Key::Char('s') | Key::Char('S') => {
                self.cycle_line_spacing();
                None
            }
            Key::Char('e') | Key::Char('E') => {
                self.cycle_letter_spacing();
                None
            }
            Key::Char('p') | Key::Char('P') => {
                self.cycle_aspect_ratio();
                None
            }
            Key::Char('d') | Key::Char('D') => {
                self.toggle_stem_darkening();
                None
            }
            Key::Char('i') | Key::Char('I') => {
                self.cycle_scale_factor();
                None
            }
            Key::Char('r') | Key::Char('R') => {
                self.toggle_ergonomic_scaling();
                None
            }
            Key::Char('f') | Key::Char('F') => {
                self.toggle_sdf_rendering();
                None
            }
            Key::Char('o') | Key::Char('O') => {
                self.toggle_optical_spacing();
                None
            }
            Key::Char('t') | Key::Char('T') => {
                self.toggle_ttf_rendering();
                None
            }
            _ => None,
        }
    }
}
