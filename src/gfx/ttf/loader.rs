//! System TrueType font discovery, path resolution, and thread-safe loading.
//! Automatically locates high-quality monospace fonts (Liberation, Noto, DejaVu, etc.)
//! or falls back cleanly to the built-in pure Rust synthesized vector TrueType font.
//! Zero third-party dependencies, purely standard library Rust.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};
use super::generator::TtfFontGenerator;
use super::parser::TrueTypeFont;

/// System font search paths and active TrueType font manager.
pub struct SystemFontLoader;

static ACTIVE_TTF_FONT: OnceLock<RwLock<Arc<TrueTypeFont>>> = OnceLock::new();

impl SystemFontLoader {
    /// Common candidate paths for monospace TrueType fonts on Linux and Unix systems.
    pub const CANDIDATE_FONT_PATHS: &'static [&'static str] = &[
        // Liberation Mono
        "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        // Noto Sans Mono
        "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
        // DejaVu Sans Mono
        "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
        // Bitstream Vera Sans Mono
        "/usr/share/fonts/TTF/VeraMono.ttf",
        "/usr/share/fonts/truetype/ttf-bitstream-vera/VeraMono.ttf",
        // Meslo / Hack / FreeMono
        "/usr/share/fonts/TTF/MesloLGSNerdFontMono-Regular.ttf",
        "/usr/share/fonts/TTF/Hack-Regular.ttf",
        "/usr/share/fonts/truetype/freefont/FreeMono.ttf",
    ];

    /// Scans the system for the first existing, valid monospace TrueType font file.
    pub fn find_best_monospace_font() -> Option<PathBuf> {
        for path_str in Self::CANDIDATE_FONT_PATHS {
            let p = Path::new(path_str);
            if p.is_file() {
                return Some(p.to_path_buf());
            }
        }

        // Check user font directory ~/.local/share/fonts
        if let Ok(home) = std::env::var("HOME") {
            let user_fonts = Path::new(&home).join(".local/share/fonts");
            if user_fonts.is_dir() {
                if let Ok(entries) = std::fs::read_dir(user_fonts) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(ext) = path.extension() {
                            if ext.eq_ignore_ascii_case("ttf") {
                                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                    if name.to_lowercase().contains("mono") {
                                        return Some(path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Loads a TrueType font, checking in order:
    /// 1. Custom user path (if specified and valid)
    /// 2. Best detected system monospace font
    /// 3. Built-in pure Rust synthesized vector TrueType font
    pub fn load_font(custom_path: Option<&str>) -> Arc<TrueTypeFont> {
        // 1. Try custom path if provided
        if let Some(path_str) = custom_path {
            let trimmed = path_str.trim();
            if !trimmed.is_empty() {
                if let Ok(font) = TrueTypeFont::from_file(trimmed) {
                    return Arc::new(font);
                }
            }
        }

        // 2. Try detected system monospace font
        if let Some(sys_path) = Self::find_best_monospace_font() {
            if let Ok(font) = TrueTypeFont::from_file(&sys_path) {
                return Arc::new(font);
            }
        }

        // 3. Fallback to built-in synthesized vector TTF font
        let bundled_bytes = TtfFontGenerator::get_bundled_ttf();
        let fallback_font = TrueTypeFont::from_bytes(bundled_bytes)
            .expect("Bundled synthesized TTF font must be 100% valid SFNT");
        Arc::new(fallback_font)
    }

    /// Gets or initializes the globally active TrueType font instance.
    pub fn get_active_font() -> Arc<TrueTypeFont> {
        let lock = ACTIVE_TTF_FONT.get_or_init(|| {
            let font = Self::load_font(None);
            RwLock::new(font)
        });

        if let Ok(guard) = lock.read() {
            Arc::clone(&guard)
        } else {
            let bundled_bytes = TtfFontGenerator::get_bundled_ttf();
            Arc::new(TrueTypeFont::from_bytes(bundled_bytes).unwrap())
        }
    }

    /// Dynamically switches the active TrueType font.
    pub fn set_active_font(custom_path: Option<&str>) {
        let new_font = Self::load_font(custom_path);
        let lock = ACTIVE_TTF_FONT.get_or_init(|| RwLock::new(Arc::clone(&new_font)));
        if let Ok(mut guard) = lock.write() {
            *guard = new_font;
        }
        super::rasterizer::TtfRasterizer::clear_cache();
    }
}
