//! Dynamic user theme loader and parser for Rustext.
//! Discovers, parses, validates, and loads external `.theme` and `.ini` palette files.
//! Zero third-party dependencies, purely standard library Rust.
//! Includes strict security validations (path traversal CWE-22, DoS CWE-400)
//! and a high-performance single-pass parser.

use std::fs;
use std::path::{Path, PathBuf};

use super::renderer::ThemeColors;

/// Maximum allowed theme file size (512 KB) to prevent DoS via unbounded memory allocation (CWE-400).
pub const MAX_THEME_FILE_SIZE: u64 = 512 * 1024;

/// Maximum number of theme files discovered per directory to prevent directory bomb DoS.
pub const MAX_THEME_SCAN_LIMIT: usize = 256;

/// Metadata for an externally discovered user theme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserThemeInfo {
    pub id: String,
    pub name: String,
    pub author: String,
    pub is_dark: bool,
    pub file_path: PathBuf,
}

// -----------------------------------------------------------------------------
// Security Checks (CWE-22 Path Traversal & Sanity Defense)
// -----------------------------------------------------------------------------

/// Validates whether a theme name is safe to use in file lookups.
/// Protects against directory traversal (CWE-22), null byte injection, and unauthorized file access.
pub fn is_safe_theme_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return false;
    }
    // Disallow path separators, directory traversal sequences, or control characters
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") || trimmed.contains('\0') {
        return false;
    }
    // Only allow alphanumeric characters, underscores, hyphens, and dots
    trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

// -----------------------------------------------------------------------------
// High-Performance Hex Color Parser (Zero Allocation)
// -----------------------------------------------------------------------------

/// Parses a color string in `#RRGGBB`, `0xRRGGBB`, `RRGGBB`, or `#RGB` format into a 32-bit ARGB `u32`.
/// Optimized pure-bitwise parser with zero string allocations or radix heap calls.
#[inline]
pub fn parse_hex_color(val: &str) -> Option<u32> {
    let s = val.trim();
    if s.is_empty() {
        return None;
    }

    let hex = if let Some(stripped) = s.strip_prefix('#') {
        stripped
    } else if let Some(stripped) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        stripped
    } else {
        s
    };

    let bytes = hex.as_bytes();
    match bytes.len() {
        6 => {
            let mut v = 0u32;
            for &b in bytes {
                let d = match b {
                    b'0'..=b'9' => (b - b'0') as u32,
                    b'a'..=b'f' => (b - b'a' + 10) as u32,
                    b'A'..=b'F' => (b - b'A' + 10) as u32,
                    _ => return None,
                };
                v = (v << 4) | d;
            }
            Some(v)
        }
        8 => {
            let mut v = 0u32;
            for &b in bytes {
                let d = match b {
                    b'0'..=b'9' => (b - b'0') as u32,
                    b'a'..=b'f' => (b - b'a' + 10) as u32,
                    b'A'..=b'F' => (b - b'A' + 10) as u32,
                    _ => return None,
                };
                v = (v << 4) | d;
            }
            Some(v & 0x00FFFFFF)
        }
        3 => {
            let mut rgb = [0u32; 3];
            for i in 0..3 {
                let b = bytes[i];
                let d = match b {
                    b'0'..=b'9' => (b - b'0') as u32,
                    b'a'..=b'f' => (b - b'a' + 10) as u32,
                    b'A'..=b'F' => (b - b'A' + 10) as u32,
                    _ => return None,
                };
                rgb[i] = d * 17;
            }
            Some((rgb[0] << 16) | (rgb[1] << 8) | rgb[2])
        }
        _ => None,
    }
}

// -----------------------------------------------------------------------------
// Single-Pass Theme String Parser
// -----------------------------------------------------------------------------

const SLOT_BACKGROUND: usize = 0;
const SLOT_GUTTER_BG: usize = 1;
const SLOT_GUTTER_FG: usize = 2;
const SLOT_GUTTER_BORDER: usize = 3;
const SLOT_TAB_BAR_BG: usize = 4;
const SLOT_TAB_ACTIVE_BG: usize = 5;
const SLOT_TAB_INACTIVE_BG: usize = 6;
const SLOT_TAB_ACTIVE_FG: usize = 7;
const SLOT_TAB_INACTIVE_FG: usize = 8;
const SLOT_STATUS_BAR_BG: usize = 9;
const SLOT_STATUS_BAR_FG: usize = 10;
const SLOT_SELECTION_BG: usize = 11;
const SLOT_CURSOR: usize = 12;
const SLOT_ACCENT: usize = 13;
const SLOT_CURRENT_LINE_BG: usize = 14;
const SLOT_RULER_COLOR: usize = 15;
const SLOT_STICKY_HEADER_BG: usize = 16;

const SLOT_MODAL_BG: usize = 17;
const SLOT_MODAL_HEADER_BG: usize = 18;
const SLOT_MODAL_BORDER: usize = 19;
const SLOT_INPUT_BG: usize = 20;
const SLOT_INPUT_BORDER: usize = 21;
const SLOT_BUTTON_BG: usize = 22;
const SLOT_BUTTON_HOVER_BG: usize = 23;
const SLOT_BUTTON_FG: usize = 24;
const SLOT_CHIP_BG: usize = 25;
const SLOT_CHIP_FG: usize = 26;

const SLOT_SCROLLBAR_BG: usize = 27;
const SLOT_SCROLLBAR_BORDER: usize = 28;
const SLOT_SCROLLBAR_THUMB: usize = 29;
const SLOT_SCROLLBAR_THUMB_HOVER: usize = 30;
const SLOT_SCROLLBAR_THUMB_ACTIVE: usize = 31;

const SLOT_SYNTAX_KEYWORD: usize = 32;
const SLOT_SYNTAX_TYPE: usize = 33;
const SLOT_SYNTAX_STRING: usize = 34;
const SLOT_SYNTAX_NUMBER: usize = 35;
const SLOT_SYNTAX_COMMENT: usize = 36;
const SLOT_SYNTAX_OPERATOR: usize = 37;
const SLOT_SYNTAX_PUNCTUATION: usize = 38;
const SLOT_SYNTAX_PLAIN: usize = 39;
const SLOT_SYNTAX_FUNCTION: usize = 40;
const SLOT_SYNTAX_METHOD: usize = 41;
const SLOT_SYNTAX_STRUCT: usize = 42;
const SLOT_SYNTAX_TRAIT: usize = 43;
const SLOT_SYNTAX_MACRO: usize = 44;
const SLOT_SYNTAX_PARAMETER: usize = 45;
const SLOT_SYNTAX_CONSTANT: usize = 46;
const SLOT_SYNTAX_FIELD: usize = 47;

const SLOT_BRACKET_0: usize = 48;
const SLOT_BRACKET_1: usize = 49;
const SLOT_BRACKET_2: usize = 50;

const TOTAL_SLOTS: usize = 51;

#[inline]
fn key_to_slot(key: &str) -> Option<usize> {
    match key {
        "background" | "bg" => Some(SLOT_BACKGROUND),
        "gutter_bg" => Some(SLOT_GUTTER_BG),
        "gutter_fg" => Some(SLOT_GUTTER_FG),
        "gutter_border" => Some(SLOT_GUTTER_BORDER),
        "tab_bar_bg" => Some(SLOT_TAB_BAR_BG),
        "tab_active_bg" => Some(SLOT_TAB_ACTIVE_BG),
        "tab_inactive_bg" => Some(SLOT_TAB_INACTIVE_BG),
        "tab_active_fg" => Some(SLOT_TAB_ACTIVE_FG),
        "tab_inactive_fg" => Some(SLOT_TAB_INACTIVE_FG),
        "status_bar_bg" | "status_bg" => Some(SLOT_STATUS_BAR_BG),
        "status_bar_fg" | "status_fg" => Some(SLOT_STATUS_BAR_FG),
        "selection_bg" | "selection" => Some(SLOT_SELECTION_BG),
        "cursor" => Some(SLOT_CURSOR),
        "accent" => Some(SLOT_ACCENT),
        "current_line_bg" | "current_line" => Some(SLOT_CURRENT_LINE_BG),
        "ruler_color" | "ruler" => Some(SLOT_RULER_COLOR),
        "sticky_header_bg" | "sticky_bg" => Some(SLOT_STICKY_HEADER_BG),

        "modal_bg" => Some(SLOT_MODAL_BG),
        "modal_header_bg" => Some(SLOT_MODAL_HEADER_BG),
        "modal_border" => Some(SLOT_MODAL_BORDER),
        "input_bg" => Some(SLOT_INPUT_BG),
        "input_border" => Some(SLOT_INPUT_BORDER),
        "button_bg" => Some(SLOT_BUTTON_BG),
        "button_hover_bg" => Some(SLOT_BUTTON_HOVER_BG),
        "button_fg" => Some(SLOT_BUTTON_FG),
        "chip_bg" => Some(SLOT_CHIP_BG),
        "chip_fg" => Some(SLOT_CHIP_FG),

        "scrollbar_bg" => Some(SLOT_SCROLLBAR_BG),
        "scrollbar_border" => Some(SLOT_SCROLLBAR_BORDER),
        "scrollbar_thumb" => Some(SLOT_SCROLLBAR_THUMB),
        "scrollbar_thumb_hover" => Some(SLOT_SCROLLBAR_THUMB_HOVER),
        "scrollbar_thumb_active" => Some(SLOT_SCROLLBAR_THUMB_ACTIVE),

        "syntax_keyword" | "keyword" => Some(SLOT_SYNTAX_KEYWORD),
        "syntax_type" | "type" => Some(SLOT_SYNTAX_TYPE),
        "syntax_string" | "string" => Some(SLOT_SYNTAX_STRING),
        "syntax_number" | "number" => Some(SLOT_SYNTAX_NUMBER),
        "syntax_comment" | "comment" => Some(SLOT_SYNTAX_COMMENT),
        "syntax_operator" | "operator" => Some(SLOT_SYNTAX_OPERATOR),
        "syntax_punctuation" | "punctuation" => Some(SLOT_SYNTAX_PUNCTUATION),
        "syntax_plain" | "plain" | "fg" | "foreground" => Some(SLOT_SYNTAX_PLAIN),
        "syntax_function" | "function" => Some(SLOT_SYNTAX_FUNCTION),
        "syntax_method" | "method" => Some(SLOT_SYNTAX_METHOD),
        "syntax_struct" | "struct" => Some(SLOT_SYNTAX_STRUCT),
        "syntax_trait" | "trait" => Some(SLOT_SYNTAX_TRAIT),
        "syntax_macro" | "macro" => Some(SLOT_SYNTAX_MACRO),
        "syntax_parameter" | "parameter" => Some(SLOT_SYNTAX_PARAMETER),
        "syntax_constant" | "constant" => Some(SLOT_SYNTAX_CONSTANT),
        "syntax_field" | "field" => Some(SLOT_SYNTAX_FIELD),

        "bracket_0" => Some(SLOT_BRACKET_0),
        "bracket_1" => Some(SLOT_BRACKET_1),
        "bracket_2" => Some(SLOT_BRACKET_2),

        _ => None,
    }
}

fn apply_slots(colors: &mut ThemeColors, slots: &[Option<u32>; TOTAL_SLOTS]) {
    if let Some(c) = slots[SLOT_BACKGROUND] { colors.background = c; }
    if let Some(c) = slots[SLOT_GUTTER_BG] { colors.gutter_bg = c; }
    if let Some(c) = slots[SLOT_GUTTER_FG] { colors.gutter_fg = c; }
    if let Some(c) = slots[SLOT_GUTTER_BORDER] { colors.gutter_border = c; }
    if let Some(c) = slots[SLOT_TAB_BAR_BG] { colors.tab_bar_bg = c; }
    if let Some(c) = slots[SLOT_TAB_ACTIVE_BG] { colors.tab_active_bg = c; }
    if let Some(c) = slots[SLOT_TAB_INACTIVE_BG] { colors.tab_inactive_bg = c; }
    if let Some(c) = slots[SLOT_TAB_ACTIVE_FG] { colors.tab_active_fg = c; }
    if let Some(c) = slots[SLOT_TAB_INACTIVE_FG] { colors.tab_inactive_fg = c; }
    if let Some(c) = slots[SLOT_STATUS_BAR_BG] { colors.status_bar_bg = c; }
    if let Some(c) = slots[SLOT_STATUS_BAR_FG] { colors.status_bar_fg = c; }
    if let Some(c) = slots[SLOT_SELECTION_BG] { colors.selection_bg = c; }
    if let Some(c) = slots[SLOT_CURSOR] { colors.cursor = c; }
    if let Some(c) = slots[SLOT_ACCENT] { colors.accent = c; }
    if let Some(c) = slots[SLOT_CURRENT_LINE_BG] { colors.current_line_bg = c; }
    if let Some(c) = slots[SLOT_RULER_COLOR] { colors.ruler_color = c; }
    if let Some(c) = slots[SLOT_STICKY_HEADER_BG] { colors.sticky_header_bg = c; }

    if let Some(c) = slots[SLOT_MODAL_BG] { colors.modal_bg = c; }
    if let Some(c) = slots[SLOT_MODAL_HEADER_BG] { colors.modal_header_bg = c; }
    if let Some(c) = slots[SLOT_MODAL_BORDER] { colors.modal_border = c; }
    if let Some(c) = slots[SLOT_INPUT_BG] { colors.input_bg = c; }
    if let Some(c) = slots[SLOT_INPUT_BORDER] { colors.input_border = c; }
    if let Some(c) = slots[SLOT_BUTTON_BG] { colors.button_bg = c; }
    if let Some(c) = slots[SLOT_BUTTON_HOVER_BG] { colors.button_hover_bg = c; }
    if let Some(c) = slots[SLOT_BUTTON_FG] { colors.button_fg = c; }
    if let Some(c) = slots[SLOT_CHIP_BG] { colors.chip_bg = c; }
    if let Some(c) = slots[SLOT_CHIP_FG] { colors.chip_fg = c; }

    if let Some(c) = slots[SLOT_SCROLLBAR_BG] { colors.scrollbar_bg = c; }
    if let Some(c) = slots[SLOT_SCROLLBAR_BORDER] { colors.scrollbar_border = c; }
    if let Some(c) = slots[SLOT_SCROLLBAR_THUMB] { colors.scrollbar_thumb = c; }
    if let Some(c) = slots[SLOT_SCROLLBAR_THUMB_HOVER] { colors.scrollbar_thumb_hover = c; }
    if let Some(c) = slots[SLOT_SCROLLBAR_THUMB_ACTIVE] { colors.scrollbar_thumb_active = c; }

    if let Some(c) = slots[SLOT_SYNTAX_KEYWORD] { colors.syntax_keyword = c; }
    if let Some(c) = slots[SLOT_SYNTAX_TYPE] { colors.syntax_type = c; }
    if let Some(c) = slots[SLOT_SYNTAX_STRING] { colors.syntax_string = c; }
    if let Some(c) = slots[SLOT_SYNTAX_NUMBER] { colors.syntax_number = c; }
    if let Some(c) = slots[SLOT_SYNTAX_COMMENT] { colors.syntax_comment = c; }
    if let Some(c) = slots[SLOT_SYNTAX_OPERATOR] { colors.syntax_operator = c; }
    if let Some(c) = slots[SLOT_SYNTAX_PUNCTUATION] { colors.syntax_punctuation = c; }
    if let Some(c) = slots[SLOT_SYNTAX_PLAIN] { colors.syntax_plain = c; }
    if let Some(c) = slots[SLOT_SYNTAX_FUNCTION] { colors.syntax_function = c; }
    if let Some(c) = slots[SLOT_SYNTAX_METHOD] { colors.syntax_method = c; }
    if let Some(c) = slots[SLOT_SYNTAX_STRUCT] { colors.syntax_struct = c; }
    if let Some(c) = slots[SLOT_SYNTAX_TRAIT] { colors.syntax_trait = c; }
    if let Some(c) = slots[SLOT_SYNTAX_MACRO] { colors.syntax_macro = c; }
    if let Some(c) = slots[SLOT_SYNTAX_PARAMETER] { colors.syntax_parameter = c; }
    if let Some(c) = slots[SLOT_SYNTAX_CONSTANT] { colors.syntax_constant = c; }
    if let Some(c) = slots[SLOT_SYNTAX_FIELD] { colors.syntax_field = c; }

    if let Some(c) = slots[SLOT_BRACKET_0] { colors.bracket_0 = c; }
    if let Some(c) = slots[SLOT_BRACKET_1] { colors.bracket_1 = c; }
    if let Some(c) = slots[SLOT_BRACKET_2] { colors.bracket_2 = c; }
}

/// Parses a `.theme` or `.ini` content string into a complete `ThemeColors` struct.
/// Missing colors gracefully fall back to the base theme (dark or light).
/// Executes in a single linear pass with zero heap allocations for slot management.
pub fn parse_theme_str(content: &str) -> ThemeColors {
    let mut is_light = false;
    let mut slots: [Option<u32>; TOTAL_SLOTS] = [None; TOTAL_SLOTS];

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        // Section headers like [ui], [syntax], [brackets] are optional and ignored
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            continue;
        }

        if let Some((raw_k, raw_v)) = trimmed.split_once('=') {
            let key = raw_k.trim().to_lowercase();
            // Strip inline comments if present (handle ';' or '#' preceded by whitespace)
            let mut val = raw_v.trim();
            if let Some((v, _)) = val.split_once(';') {
                val = v.trim();
            }
            if let Some(idx) = val.find(" #") {
                val = val[..idx].trim();
            }
            if let Some(idx) = val.find("\t#") {
                val = val[..idx].trim();
            }

            // Check for base theme declaration
            if (key == "type" || key == "base")
                && (val.eq_ignore_ascii_case("light") || val.eq_ignore_ascii_case("claro"))
            {
                is_light = true;
                continue;
            }

            if let Some(slot_idx) = key_to_slot(&key) {
                if let Some(c) = parse_hex_color(val) {
                    slots[slot_idx] = Some(c);
                }
            }
        }
    }

    let mut colors = if is_light {
        ThemeColors::light()
    } else {
        ThemeColors::modern_dark()
    };

    apply_slots(&mut colors, &slots);
    colors
}

// -----------------------------------------------------------------------------
// Directory Discovery & Safe Theme Loading
// -----------------------------------------------------------------------------

/// Returns standard search directories for user themes in priority order.
pub fn theme_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 1. User home: ~/.rustext/themes and ~/.config/rustext/themes
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let p = PathBuf::from(home);
        dirs.push(p.join(".rustext").join("themes"));
        dirs.push(p.join(".config").join("rustext").join("themes"));
    }

    // 2. Workspace local: ./.rustext/themes
    if let Ok(cur) = std::env::current_dir() {
        let local = cur.join(".rustext").join("themes");
        if !dirs.contains(&local) {
            dirs.push(local);
        }
    }

    dirs
}

/// Primary directory where user themes are placed.
pub fn default_themes_dir() -> PathBuf {
    theme_search_dirs()
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("./.rustext/themes"))
}

/// Finds a theme file matching `name` (with or without `.theme` / `.ini` extension).
/// Enforces path containment to eliminate directory traversal (CWE-22).
pub fn find_theme_file(name: &str) -> Option<PathBuf> {
    let raw = name.trim();
    if !is_safe_theme_name(raw) {
        return None;
    }

    let candidates = [
        format!("{}.theme", raw),
        format!("{}.ini", raw),
        raw.to_string(),
        format!("{}.theme", raw.to_lowercase().replace(' ', "_")),
        format!("{}.theme", raw.to_lowercase().replace(' ', "-")),
    ];

    for base_dir in theme_search_dirs() {
        if !base_dir.exists() || !base_dir.is_dir() {
            continue;
        }

        let canonical_base = base_dir.canonicalize().ok();

        for candidate in &candidates {
            let candidate_path = base_dir.join(candidate);
            if candidate_path.exists() && candidate_path.is_file() {
                // Verify candidate strictly stays within base_dir (defense against symlinks pointing outside)
                if let (Some(cb), Ok(cp)) = (&canonical_base, candidate_path.canonicalize()) {
                    if cp.starts_with(cb) {
                        return Some(candidate_path);
                    }
                } else if !candidate_path.is_symlink() {
                    return Some(candidate_path);
                }
            }
        }
    }

    None
}

/// Loads a custom user theme by name from disk if available.
/// Enforces file size limits to prevent memory exhaustion DoS (CWE-400).
pub fn load_user_theme(name: &str) -> Option<ThemeColors> {
    let path = find_theme_file(name)?;
    let meta = fs::metadata(&path).ok()?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > MAX_THEME_FILE_SIZE {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    Some(parse_theme_str(&content))
}

/// Scans theme directories and lists all available external user themes.
/// Enforces scanning limits and size caps for defense-in-depth.
pub fn list_user_themes() -> Vec<UserThemeInfo> {
    let mut themes = Vec::new();
    let default_dir = default_themes_dir();

    // Ensure directory exists and has sample template if empty
    if !default_dir.exists() {
        let _ = fs::create_dir_all(&default_dir);
    }
    if let Ok(entries) = fs::read_dir(&default_dir) {
        if entries.count() == 0 {
            let _ = scaffold_sample_themes(&default_dir);
        }
    }

    for base_dir in theme_search_dirs() {
        if !base_dir.exists() || !base_dir.is_dir() {
            continue;
        }

        if let Ok(read_dir) = fs::read_dir(base_dir) {
            let mut count = 0;
            for entry in read_dir.flatten() {
                count += 1;
                if count > MAX_THEME_SCAN_LIMIT {
                    break;
                }

                let path = entry.path();
                if path.is_file() {
                    if let Ok(meta) = entry.metadata() {
                        if meta.len() > MAX_THEME_FILE_SIZE {
                            continue;
                        }
                    }

                    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                    if ext.eq_ignore_ascii_case("theme") || ext.eq_ignore_ascii_case("ini") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("custom");
                            let mut id = stem.to_string();
                            let mut name = stem.replace('_', " ").replace('-', " ");
                            let mut author = "Usuario".to_string();
                            let mut is_dark = true;

                            for line in content.lines() {
                                let trimmed = line.trim();
                                if let Some((k, v)) = trimmed.split_once('=') {
                                    match k.trim().to_lowercase().as_str() {
                                        "id" => id = v.trim().to_string(),
                                        "name" => name = v.trim().to_string(),
                                        "author" => author = v.trim().to_string(),
                                        "type" | "base" => {
                                            let val = v.trim().to_lowercase();
                                            if val == "light" || val == "claro" {
                                                is_dark = false;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }

                            if !themes.iter().any(|t: &UserThemeInfo| t.id == id) {
                                themes.push(UserThemeInfo {
                                    id,
                                    name,
                                    author,
                                    is_dark,
                                    file_path: path,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    themes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    themes
}

/// Scaffolds the official built-in `.theme` files in the user themes folder if empty.
pub fn scaffold_sample_themes(dir: &Path) -> std::io::Result<()> {
    let themes: [(&str, &str); 8] = [
        ("tokyo-night.theme", include_str!("../../assets/themes/tokyo-night.theme")),
        ("catppuccin-mocha.theme", include_str!("../../assets/themes/catppuccin-mocha.theme")),
        ("one-dark.theme", include_str!("../../assets/themes/one-dark.theme")),
        ("modern-dark.theme", include_str!("../../assets/themes/modern-dark.theme")),
        ("monokai.theme", include_str!("../../assets/themes/monokai.theme")),
        ("solarized-dark.theme", include_str!("../../assets/themes/solarized-dark.theme")),
        ("light.theme", include_str!("../../assets/themes/light.theme")),
        ("dracula.theme", include_str!("../../assets/themes/dracula.theme")),
    ];

    for (file_name, content) in &themes {
        let target = dir.join(file_name);
        if !target.exists() {
            let _ = fs::write(&target, content);
        }
    }

    Ok(())
}
