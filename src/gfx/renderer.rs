//! Graphical renderer translating editor state into framebuffer pixels.
//! Zero third-party dependencies, purely standard library Rust.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock, RwLock};

use super::edge_perception::EdgePerception;
use super::font::{FontFamily, FontRasterizer, FontWeight, FONT_HEIGHT, FONT_WIDTH};
use super::framebuffer::Framebuffer;
use super::gamma::{blend_pixel_gamma, blend_pixel_subpixel_gamma};
use super::hinting::{FontHinter, HintingLevel};
use super::optical_spacing::OpticalPunctuationSpacing;
use super::sdf::SdfFontRasterizer;
use super::shaping::FontShaper;
use super::stem_darkening::StemDarkener;
use crate::core::syntax::SyntaxHighlighter;
use crate::editor::Document;

pub use super::colors::ThemeColors;
pub use super::geometry::{AutocompleteGeometry, MinimapGeometry, PaneGeometry, ScrollbarGeometry};

/// Cache key for rendered minimap background and syntax lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct MinimapCacheKey {
    buffer_id: u64,
    buffer_version: u64,
    total_lines: usize,
    width: usize,
    height: usize,
    gutter_bg: u32,
    gutter_border: u32,
    gutter_fg: u32,
    syntax_keyword: u32,
    syntax_type: u32,
    syntax_plain: u32,
    language: crate::core::syntax::Language,
}

static MINIMAP_SURFACE_CACHE: OnceLock<RwLock<BTreeMap<MinimapCacheKey, Arc<Vec<u32>>>>> = OnceLock::new();

/// Formats a line number into a caller-provided fixed stack buffer without heap allocation.
#[inline(always)]
fn format_line_number_into(num: usize, width: usize, buf: &mut [u8; 16]) -> &str {
    let mut n = num;
    let mut pos = 16;
    if n == 0 {
        pos -= 1;
        buf[pos] = b'0';
    } else {
        while n > 0 && pos > 0 {
            pos -= 1;
            buf[pos] = b'0' + (n % 10) as u8;
            n /= 10;
        }
    }
    let digits_len = 16 - pos;
    let pad = if width > digits_len { width - digits_len } else { 0 };
    let start = pos.saturating_sub(pad);
    buf[start..pos].fill(b' ');
    unsafe { std::str::from_utf8_unchecked(&buf[start..16]) }
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
    pub workspace_name: Option<String>,
    pub git_statuses: Option<std::collections::HashMap<std::path::PathBuf, crate::editor::git::GitFileStatus>>,
    pub git_gutter_enabled: bool,
    pub inline_blame: Option<String>,
    pub active_hunk_popup: Option<crate::editor::git::GitHunk>,
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
    pub tab_size: usize,
    pub indent_guides: bool,
    pub highlight_active_indent_guide: bool,
    pub highlight_current_line: bool,
    pub rulers: Vec<usize>,
    pub sticky_scroll: bool,
    pub max_sticky_lines: usize,
    pub signature_help: Option<crate::editor::lsp::LspSignatureHelp>,
    pub modal_label: Option<String>,
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
            workspace_name: None,
            git_statuses: None,
            git_gutter_enabled: true,
            inline_blame: None,
            active_hunk_popup: None,
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
            tab_size: 4,
            indent_guides: true,
            highlight_active_indent_guide: true,
            highlight_current_line: true,
            rulers: vec![80, 120],
            sticky_scroll: true,
            max_sticky_lines: 3,
            signature_help: None,
            modal_label: None,
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
                                let sdf_sub_arc;
                                let font_sub;
                                let subpixel_cov: &[[u8; 3]] = if self.use_ttf_font && !is_box {
                                    let ttf_font = crate::gfx::ttf::SystemFontLoader::get_active_font();
                                    let (_, _, sub) = crate::gfx::ttf::TtfRasterizer::rasterize_char_subpixel_cached(&ttf_font, ch, draw_w, glyph.height);
                                    ttf_sub = sub;
                                    &ttf_sub
                                } else if self.sdf_font_rendering && !is_box {
                                    sdf_sub_arc = SdfFontRasterizer::get_cached_glyph_sdf_subpixel(
                                        ch,
                                        self.font_family,
                                        draw_w,
                                        glyph.height,
                                        self.font_weight,
                                        0.0,
                                    );
                                    &sdf_sub_arc
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
                                    let sdf_cov_arc = SdfFontRasterizer::get_cached_glyph_sdf(
                                        ch,
                                        self.font_family,
                                        draw_w,
                                        glyph.height,
                                        self.font_weight,
                                        0.0,
                                    );
                                    if dark_factor > 0.001 {
                                        darkened_sdf = StemDarkener::darken_coverage_mask(&sdf_cov_arc, draw_w, glyph.height, dark_factor);
                                        &darkened_sdf
                                    } else {
                                        sdf_cov = sdf_cov_arc;
                                        &sdf_cov
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

        let total_lines = doc.buffer.len_lines();
        let key = MinimapCacheKey {
            buffer_id: doc.buffer.id(),
            buffer_version: doc.buffer.version(),
            total_lines,
            width: geom.width,
            height: geom.height,
            gutter_bg: self.colors.gutter_bg,
            gutter_border: self.colors.gutter_border,
            gutter_fg: self.colors.gutter_fg,
            syntax_keyword: self.colors.syntax_keyword,
            syntax_type: self.colors.syntax_type,
            syntax_plain: self.colors.syntax_plain,
            language: doc.language,
        };

        let cache = MINIMAP_SURFACE_CACHE.get_or_init(|| RwLock::new(BTreeMap::new()));
        let cached_surface = if let Ok(guard) = cache.read() {
            guard.get(&key).cloned()
        } else {
            None
        };

        let surface_arc = if let Some(arc) = cached_surface {
            arc
        } else {
            // Render the miniature code lines and background offscreen
            let mut surface = vec![self.colors.gutter_bg; geom.width * geom.height];

            // 1. Subtle left border
            for y in 0..geom.height {
                surface[y * geom.width] = self.colors.gutter_border;
            }

            // 2. Miniature code lines
            let draw_x = 3;
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
                        line_idx * 2
                    } else {
                        (line_idx * (geom.height.saturating_sub(2))) / total_lines
                    };

                    if my >= geom.height {
                        break;
                    }

                    let row_offset = my * geom.width;

                    if use_fast_density {
                        let line_len = doc.buffer.line_len_chars(line_idx);
                        if line_len > 0 {
                            let draw_w = (line_len * 2).min(max_draw_w).max(2);
                            let end_x = (draw_x + draw_w).min(geom.width);
                            for px in draw_x..end_x {
                                surface[row_offset + px] = self.colors.gutter_fg;
                            }
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
                            let end_x = (cur_x + draw_w).min(geom.width);
                            for px in cur_x..end_x {
                                surface[row_offset + px] = color;
                            }
                            cur_x += draw_w + 1;
                        }
                    }

                    line_idx += lines_step;
                }
            }

            let arc = Arc::new(surface);
            if let Ok(mut guard) = cache.write() {
                if guard.len() >= 8 {
                    guard.clear();
                }
                guard.insert(key, Arc::clone(&arc));
            }
            arc
        };

        // Blit cached miniature code lines and background into framebuffer
        let copy_w = geom.width.min(fb.width.saturating_sub(geom.x));
        let copy_h = geom.height.min(fb.height.saturating_sub(geom.y));
        for row in 0..copy_h {
            let dst_start = (geom.y + row) * fb.width + geom.x;
            let src_start = row * geom.width;
            fb.pixels[dst_start..dst_start + copy_w]
                .copy_from_slice(&surface_arc[src_start..src_start + copy_w]);
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

    /// Renders a floating signature help tooltip near the cursor showing parameter hints.
    pub fn render_signature_help(
        &self,
        fb: &mut Framebuffer,
        sig_help: &crate::editor::lsp::LspSignatureHelp,
        doc: &Document,
        sidebar_w: usize,
        has_menu_bar: bool,
    ) {
        if sig_help.signatures.is_empty() {
            return;
        }

        let sig_idx = sig_help.active_signature.min(sig_help.signatures.len().saturating_sub(1));
        let sig = &sig_help.signatures[sig_idx];
        if sig.label.is_empty() {
            return;
        }

        let char_w = self.char_width();
        let line_h = self.line_height();

        // Calculate cursor position on screen
        let cursor_head = doc.cursors.primary().head;
        let cursor_pos = doc.buffer.char_to_position(cursor_head);
        let gutter_cols = doc.buffer.len_lines().to_string().len().max(3);
        let gutter_w = (gutter_cols + 2) * char_w;

        let content_y = (if has_menu_bar { self.menu_bar_height } else { 0 }) + self.tab_bar_height;
        let text_start_x = sidebar_w + gutter_w;

        let screen_row = cursor_pos.line.saturating_sub(doc.scroll_row);
        let cursor_y = content_y + screen_row * line_h;
        let cursor_x = text_start_x + cursor_pos.col.saturating_sub(doc.scroll_col) * char_w;

        let padding_x = self.scale_coord(10);
        let padding_y = self.scale_coord(6);
        let max_text_len = sig.label.chars().count();
        let box_w = (max_text_len * char_w + padding_x * 2)
            .max(self.scale_coord(180))
            .min(fb.width.saturating_sub(sidebar_w + self.scale_coord(20)));
        let has_doc = !sig.documentation.is_empty();
        let box_h = if has_doc {
            line_h * 2 + padding_y * 2
        } else {
            line_h + padding_y * 2
        };

        // Prefer placing above cursor, fall back to below if too close to top
        let box_x = cursor_x
            .saturating_sub(padding_x)
            .min(fb.width.saturating_sub(box_w + self.scale_coord(10)))
            .max(sidebar_w + self.scale_coord(5));
        let box_y = if cursor_y >= content_y + box_h + self.scale_coord(4) {
            cursor_y - box_h - self.scale_coord(4)
        } else {
            cursor_y + line_h + self.scale_coord(4)
        };

        // 1. Soft Shadow
        if self.shadows {
            self.render_shadow(fb, box_x, box_y, box_w, box_h, 6, 4);
        } else {
            fb.fill_rect(box_x + 2, box_y + 2, box_w, box_h, 0x00101010);
        }

        // 2. Background and Border
        let bg = self.colors.modal_bg;
        let border = self.colors.modal_border;
        if self.rounded_corners {
            fb.fill_rounded_rect(box_x, box_y, box_w, box_h, 4, bg);
            fb.draw_rounded_outline(box_x, box_y, box_w, box_h, 4, border);
        } else {
            fb.fill_rect(box_x, box_y, box_w, box_h, bg);
            fb.draw_rect_outline(box_x, box_y, box_w, box_h, border);
        }

        // 3. Render signature label
        let text_y = box_y + padding_y + (line_h.saturating_sub(self.char_height())) / 2;
        let text_x = box_x + padding_x;

        let active_param_idx = sig.active_parameter.unwrap_or(sig_help.active_parameter);
        let active_param_label = sig.parameters.get(active_param_idx).map(|p| p.label.as_str()).unwrap_or("");

        if !active_param_label.is_empty() && sig.label.contains(active_param_label) {
            if let Some(param_start) = sig.label.find(active_param_label) {
                let before = &sig.label[..param_start];
                let param = &sig.label[param_start..param_start + active_param_label.len()];
                let after = &sig.label[param_start + active_param_label.len()..];

                fb.draw_text(before, text_x, text_y, self.colors.syntax_plain, None);
                let before_w = before.chars().count() * char_w;
                let param_x = text_x + before_w;
                let param_w = param.chars().count() * char_w;
                fb.fill_rect(param_x, text_y, param_w, self.char_height(), self.colors.chip_bg);
                fb.draw_text(param, param_x, text_y, self.colors.accent, None);
                let after_x = param_x + param_w;
                fb.draw_text(after, after_x, text_y, self.colors.syntax_plain, None);
            } else {
                fb.draw_text(&sig.label, text_x, text_y, self.colors.syntax_plain, None);
            }
        } else {
            fb.draw_text(&sig.label, text_x, text_y, self.colors.syntax_plain, None);
        }

        // 4. Render documentation line if present
        if has_doc {
            let doc_y = text_y + line_h;
            let first_doc_line = sig.documentation.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
            let max_doc_chars = box_w.saturating_sub(padding_x * 2) / char_w.max(1);
            let doc_display = if first_doc_line.chars().count() > max_doc_chars {
                let mut s: String = first_doc_line.chars().take(max_doc_chars.saturating_sub(1)).collect();
                s.push('…');
                s
            } else {
                first_doc_line.to_string()
            };
            fb.draw_text(&doc_display, text_x, doc_y, self.colors.gutter_fg, None);
        }
    }

    /// Renders a floating Hunk Diff modal showing old lines vs new lines and revert action.
    pub fn render_hunk_diff_popup(
        &self,
        fb: &mut Framebuffer,
        hunk: &crate::editor::git::GitHunk,
    ) {
        let total_diff_rows = (hunk.old_lines.len() + hunk.new_lines.len()).min(16).max(2);
        let row_h = self.line_height();

        let pop_w = self.scale_coord(640).min(fb.width.saturating_sub(40)).max(360);
        let header_h = self.scale_coord(32);
        let footer_h = self.scale_coord(38);
        let content_h = total_diff_rows * row_h + self.scale_coord(16);
        let pop_h = (header_h + content_h + footer_h).min(fb.height.saturating_sub(60));

        let pop_x = (fb.width.saturating_sub(pop_w)) / 2;
        let pop_y = (fb.height.saturating_sub(pop_h)) / 2;

        // Dim overlay background
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 100);

        // Drop shadow
        if self.shadows {
            self.render_shadow(fb, pop_x, pop_y, pop_w, pop_h, 12, 8);
        } else {
            fb.fill_rect(pop_x + 4, pop_y + 4, pop_w, pop_h, 0x00101010);
        }

        // Modal container
        if self.rounded_corners {
            fb.fill_rounded_rect(pop_x, pop_y, pop_w, pop_h, 6, self.colors.modal_bg);
            fb.draw_rounded_outline(pop_x, pop_y, pop_w, pop_h, 6, self.colors.modal_border);
        } else {
            fb.fill_rect(pop_x, pop_y, pop_w, pop_h, self.colors.modal_bg);
            fb.draw_rect_outline(pop_x, pop_y, pop_w, pop_h, self.colors.modal_border);
        }

        // Header
        fb.fill_rect(pop_x, pop_y, pop_w, header_h, self.colors.modal_header_bg);
        let header_title = format!(" Git Diff Hunk (Líneas {}..={})", hunk.start_line + 1, hunk.end_line + 1);
        let title_y = pop_y + (header_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(&header_title, pop_x + self.scale_coord(12), title_y, self.colors.accent, None);

        let close_hint = "[Esc] Cerrar  [Alt+R] Deshacer";
        let hint_w = close_hint.chars().count() * self.char_width();
        if pop_x + pop_w >= hint_w + self.scale_coord(12) {
            fb.draw_text(close_hint, pop_x + pop_w - hint_w - self.scale_coord(12), title_y, self.colors.gutter_fg, None);
        }
        fb.fill_rect(pop_x, pop_y + header_h.saturating_sub(1), pop_w, 1, self.colors.gutter_border);

        // Content: list old lines (- in red), new lines (+ in green)
        let mut curr_y = pop_y + header_h + self.scale_coord(8);
        let content_max_y = pop_y + pop_h - footer_h;
        let pad_x = pop_x + self.scale_coord(12);
        let max_text_x = pop_x + pop_w - self.scale_coord(12);

        // Old lines
        if hunk.old_lines.is_empty() {
            if curr_y + row_h <= content_max_y {
                fb.draw_text_truncated("(Sin líneas previas en HEAD — Todo añadido)", pad_x, curr_y, max_text_x, self.colors.gutter_fg, None);
                curr_y += row_h;
            }
        } else {
            for old_l in &hunk.old_lines {
                if curr_y + row_h > content_max_y {
                    break;
                }
                fb.fill_rect_alpha(pop_x + 4, curr_y, pop_w.saturating_sub(8), row_h, 0x00F7768E, 40);
                let diff_line = format!("- {}", old_l);
                fb.draw_text_truncated(&diff_line, pad_x, curr_y, max_text_x, 0x00F7768E, None);
                curr_y += row_h;
            }
        }

        // New lines
        if hunk.new_lines.is_empty() {
            if curr_y + row_h <= content_max_y {
                fb.draw_text_truncated("(Líneas eliminadas en esta versión)", pad_x, curr_y, max_text_x, self.colors.gutter_fg, None);
                curr_y += row_h;
            }
        } else {
            for new_l in &hunk.new_lines {
                if curr_y + row_h > content_max_y {
                    break;
                }
                fb.fill_rect_alpha(pop_x + 4, curr_y, pop_w.saturating_sub(8), row_h, 0x009ECE6A, 40);
                let diff_line = format!("+ {}", new_l);
                fb.draw_text_truncated(&diff_line, pad_x, curr_y, max_text_x, 0x009ECE6A, None);
                curr_y += row_h;
            }
        }

        // Footer
        let _ = curr_y;
        let footer_y = pop_y + pop_h - footer_h;
        fb.fill_rect(pop_x, footer_y, pop_w, 1, self.colors.gutter_border);

        let btn_h = self.scale_coord(26);
        let btn_y = footer_y + (footer_h.saturating_sub(btn_h)) / 2;

        // Revert button
        let rev_text = " Deshacer Hunk (Alt+R) ";
        let rev_w = rev_text.chars().count() * self.char_width() + self.scale_coord(12);
        let rev_x = pop_x + self.scale_coord(12);
        if self.rounded_corners {
            fb.fill_rounded_rect(rev_x, btn_y, rev_w, btn_h, 4, self.colors.button_bg);
        } else {
            fb.fill_rect(rev_x, btn_y, rev_w, btn_h, self.colors.button_bg);
        }
        let btn_text_y = btn_y + (btn_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(rev_text, rev_x + self.scale_coord(6), btn_text_y, self.colors.button_fg, None);

        // Close button
        let close_text = " Cerrar (Esc) ";
        let close_w = close_text.chars().count() * self.char_width() + self.scale_coord(12);
        let close_x = rev_x + rev_w + self.scale_coord(12);
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x, btn_y, close_w, btn_h, 4, self.colors.input_bg);
        } else {
            fb.fill_rect(close_x, btn_y, close_w, btn_h, self.colors.input_bg);
        }
        fb.draw_text(close_text, close_x + self.scale_coord(6), btn_text_y, self.colors.button_fg, None);
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

    /// Computes the leading indentation in spaces for a line (expanding tabs).
    pub fn line_indent_spaces(doc: &Document, line_idx: usize, tab_size: usize) -> usize {
        let line_cow = doc.buffer.line_cow(line_idx).unwrap_or(std::borrow::Cow::Borrowed(""));
        let mut spaces = 0;
        let ts = tab_size.max(1);
        for ch in line_cow.chars() {
            if ch == ' ' {
                spaces += 1;
            } else if ch == '\t' {
                spaces += ts;
            } else {
                break;
            }
        }
        spaces
    }

    /// Checks if a line is empty or contains only whitespace.
    pub fn is_blank_line(doc: &Document, line_idx: usize) -> bool {
        let line_cow = doc.buffer.line_cow(line_idx).unwrap_or(std::borrow::Cow::Borrowed(""));
        line_cow.trim().is_empty()
    }

    /// Determines the inferred indentation of a blank line by checking nearby non-blank lines.
    pub fn blank_line_indent(doc: &Document, line_idx: usize, tab_size: usize) -> usize {
        let total_lines = doc.buffer.len_lines();
        let mut prev_indent = 0;
        let mut p = line_idx;
        while p > 0 {
            p -= 1;
            if !Self::is_blank_line(doc, p) {
                prev_indent = Self::line_indent_spaces(doc, p, tab_size);
                break;
            }
            if line_idx - p > 20 {
                break;
            }
        }

        let mut next_indent = 0;
        let mut n = line_idx + 1;
        while n < total_lines {
            if !Self::is_blank_line(doc, n) {
                next_indent = Self::line_indent_spaces(doc, n, tab_size);
                break;
            }
            if n - line_idx > 20 {
                break;
            }
            n += 1;
        }

        prev_indent.min(next_indent)
    }

    /// Renders sticky scope headers pinned at the top of the editor pane (`content_y`).
    /// Returns the number of sticky lines rendered.
    pub fn render_sticky_scroll(
        &self,
        fb: &mut Framebuffer,
        doc: &Document,
        pane_x: usize,
        content_y: usize,
        gutter_w: usize,
        gutter_cols: usize,
        max_code_x: usize,
        line_height: usize,
    ) -> usize {
        if !self.sticky_scroll || self.max_sticky_lines == 0 || doc.scroll_row == 0 {
            return 0;
        }

        let sticky_lines = doc.folding.sticky_lines(doc.scroll_row, self.max_sticky_lines);
        if sticky_lines.is_empty() {
            return 0;
        }

        let total_lines = doc.buffer.len_lines();
        let char_w = self.char_width();
        let count = sticky_lines.len();

        for (idx, &line_idx) in sticky_lines.iter().enumerate() {
            if line_idx >= total_lines {
                continue;
            }

            let py = content_y + idx * line_height;
            let text_y = py + self.line_spacing / 2;
            let row_w = max_code_x.saturating_sub(pane_x);

            // 1. Draw sticky row background across the code canvas
            fb.fill_rect(pane_x, py, row_w, line_height, self.colors.sticky_header_bg);

            // 2. Gutter area background & separator
            if self.show_line_numbers && gutter_w > 0 {
                fb.fill_rect(pane_x, py, gutter_w, line_height, self.colors.gutter_bg);
                fb.fill_rect(pane_x + gutter_w.saturating_sub(1), py, 1, line_height, self.colors.gutter_border);

                // Gutter line number with sticky accent highlight
                let mut num_buf = [0u8; 16];
                        let num_str = format_line_number_into(line_idx + 1, gutter_cols, &mut num_buf);
                fb.draw_text(&num_str, pane_x + char_w, text_y, self.colors.accent, None);
            }

            // 3. Render syntax highlighted code tokens
            let text_start_x = pane_x + gutter_w + char_w;
            let line_cow = doc.buffer.line_cow(line_idx).unwrap_or(std::borrow::Cow::Borrowed(""));
            let lexical_tokens = crate::core::syntax::SyntaxHighlighter::tokenize_line(&line_cow, doc.language);
            let tokens = if let Some(sem_spans) = doc.semantic_tokens.get(&line_idx) {
                crate::core::syntax::SyntaxHighlighter::apply_semantic_spans(&line_cow, lexical_tokens, sem_spans)
            } else {
                lexical_tokens
            };
            self.render_token_line(fb, &tokens, text_start_x, text_y, max_code_x, doc.scroll_col, self.colors.sticky_header_bg);
            self.render_diagnostic_squiggles_for_line(fb, doc, line_idx, &line_cow, text_start_x, text_y, max_code_x, 0, None, doc.scroll_col);

            // 4. Row separator (gutter_border) or bottom border on the last sticky line (accent)
            let is_last = idx + 1 == count;
            let border_color = if is_last {
                self.colors.accent
            } else {
                self.colors.gutter_border
            };
            fb.fill_rect(pane_x, py + line_height.saturating_sub(1), row_w, 1, border_color);
        }

        count
    }

    /// Draws wavy squiggles (sinusoidal/zigzag underline) for compiler diagnostics matching `line_idx`.
    pub fn render_diagnostic_squiggles_for_line(
        &self,
        fb: &mut Framebuffer,
        doc: &Document,
        line_idx: usize,
        line_text: &str,
        text_start_x: usize,
        text_y: usize,
        max_code_x: usize,
        char_start_offset: usize,
        char_end_bound: Option<usize>,
        scroll_col: usize,
    ) {
        let dc = match &self.diagnostics {
            Some(dc) if !dc.is_empty() => dc,
            _ => return,
        };

        let doc_path = doc.buffer.file_path();
        let diags = dc.for_line_and_file(line_idx + 1, doc_path);
        if diags.is_empty() {
            return;
        }

        let squiggle_y = text_y + self.char_height().saturating_sub(self.scale_coord(1));
        let line_chars: Vec<char> = line_text.chars().collect();

        for d in diags {
            let start_col = d.col.saturating_sub(1);
            let end_col = if let Some(ec) = d.end_col {
                ec.saturating_sub(1).max(start_col + 1)
            } else {
                let word_len = line_chars
                    .iter()
                    .skip(start_col)
                    .take_while(|&&c| c.is_alphanumeric() || c == '_')
                    .count()
                    .max(1);
                start_col + word_len
            };

            let bound = char_end_bound.unwrap_or(usize::MAX);
            if end_col <= char_start_offset || start_col >= bound {
                continue;
            }

            let seg_start = start_col.max(char_start_offset);
            let seg_end = end_col.min(bound);

            if seg_end <= seg_start {
                continue;
            }

            let col_in_slice_start = seg_start - char_start_offset;
            let col_in_slice_end = seg_end - char_start_offset;

            if col_in_slice_end <= scroll_col {
                continue;
            }

            let vis_start = col_in_slice_start.saturating_sub(scroll_col);
            let vis_end = col_in_slice_end.saturating_sub(scroll_col);

            let sx = text_start_x + vis_start * self.char_width();
            let ex = text_start_x + vis_end * self.char_width();

            if sx >= max_code_x {
                continue;
            }

            let clamped_sx = sx.max(text_start_x);
            let clamped_ex = ex.min(max_code_x);

            if clamped_ex > clamped_sx {
                fb.draw_wavy_squiggle(clamped_sx, clamped_ex, squiggle_y, d.severity.color_hex(), self.font_scale);
            }
        }
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

        // Compute active indent guide based on primary cursor position
        let effective_tab_size = self.tab_size.max(1);
        let (active_guide_col, active_guide_start_line, active_guide_end_line) = if self.highlight_active_indent_guide && is_active {
            let cursor_pos = doc.buffer.char_to_position(doc.cursors.primary().head);
            let cur_indent = if Self::is_blank_line(doc, cursor_pos.line) {
                Self::blank_line_indent(doc, cursor_pos.line, effective_tab_size)
            } else {
                Self::line_indent_spaces(doc, cursor_pos.line, effective_tab_size)
            };
            if cur_indent >= effective_tab_size {
                let guide_col = ((cur_indent.saturating_sub(1)) / effective_tab_size) * effective_tab_size;
                let mut start_l = cursor_pos.line;
                while start_l > 0 {
                    let prev = start_l - 1;
                    if !Self::is_blank_line(doc, prev) {
                        let ind = Self::line_indent_spaces(doc, prev, effective_tab_size);
                        if ind < guide_col + effective_tab_size {
                            start_l = prev;
                            break;
                        }
                    }
                    start_l = prev;
                    if cursor_pos.line - start_l > 500 {
                        break;
                    }
                }
                let mut end_l = cursor_pos.line;
                let total_l = doc.buffer.len_lines();
                while end_l + 1 < total_l {
                    let next = end_l + 1;
                    if !Self::is_blank_line(doc, next) {
                        let ind = Self::line_indent_spaces(doc, next, effective_tab_size);
                        if ind < guide_col + effective_tab_size {
                            end_l = next;
                            break;
                        }
                    }
                    end_l = next;
                    if end_l - cursor_pos.line > 500 {
                        break;
                    }
                }
                (Some(guide_col), start_l, end_l)
            } else {
                (None, 0, 0)
            }
        } else {
            (None, 0, 0)
        };

        if !self.word_wrap {
            let mut current_line = doc.scroll_row;
            while current_line > 0 && doc.folding.is_line_hidden(current_line) {
                current_line = doc.folding.prev_visible_line(current_line);
            }

            for row in 0..visible_lines {
                let py = content_y + row * line_height;
                let text_y = py + self.line_spacing / 2;

                if current_line < total_lines {
                    let line_idx = current_line;
                    // Gutter line number
                    if self.show_line_numbers {
                        let diag = self.diagnostics.as_ref().and_then(|dc| dc.for_line(line_idx + 1));
                        let (fg_col, badge) = if let Some(d) = diag {
                            (d.severity.color_hex(), Some(d.severity.badge()))
                        } else {
                            (self.colors.gutter_fg, None)
                        };

                        let is_cur_line = cursor_positions.iter().any(|p| p.line == line_idx);
                        let mut num_buf = [0u8; 16];
                        let num_str = format_line_number_into(line_idx + 1, gutter_cols, &mut num_buf);
                        let line_num_fg = if diag.is_some() {
                            fg_col
                        } else if is_cur_line && is_active {
                            self.colors.tab_active_fg
                        } else {
                            fg_col
                        };
                        fb.draw_text(&num_str, pane_x + self.char_width(), text_y, line_num_fg, None);

                        if let Some(b) = badge {
                            fb.draw_text(b, pane_x + 2, text_y, fg_col, None);
                        }

                        // Code folding indicator in gutter
                        if doc.folding.enabled {
                            let fold_x = pane_x + (gutter_cols + 1) * self.char_width();
                            match doc.folding.is_fold_start(line_idx) {
                                Some(true) => {
                                    fb.draw_char('▶', fold_x, text_y, self.colors.accent, None);
                                }
                                Some(false) => {
                                    fb.draw_char('▼', fold_x, text_y, self.colors.gutter_fg, None);
                                }
                                None => {}
                            }
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

                    // Current Line Highlight
                    let is_cur_line = cursor_positions.iter().any(|p| p.line == line_idx);
                    if self.highlight_current_line && is_active && is_cur_line {
                        let highlight_w = max_code_x.saturating_sub(text_start_x);
                        fb.fill_rect(text_start_x, py, highlight_w, line_height, self.colors.current_line_bg);
                    }

                    // Column Rulers
                    for &ruler_col in &self.rulers {
                        if ruler_col >= doc.scroll_col {
                            let rx = text_start_x + (ruler_col - doc.scroll_col) * self.char_width();
                            if rx >= text_start_x && rx < max_code_x {
                                fb.fill_rect(rx, py, 1, line_height, self.colors.ruler_color);
                            }
                        }
                    }

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

                    // Indent Guides (vertical lines at tab stops)
                    if self.indent_guides {
                        let is_blank = Self::is_blank_line(doc, line_idx);
                        let indent_spaces = if is_blank {
                            Self::blank_line_indent(doc, line_idx, effective_tab_size)
                        } else {
                            Self::line_indent_spaces(doc, line_idx, effective_tab_size)
                        };
                        let num_guides = indent_spaces / effective_tab_size;
                        for g in 0..num_guides {
                            let g_col = g * effective_tab_size;
                            if g_col >= doc.scroll_col {
                                let gx = text_start_x + (g_col - doc.scroll_col) * self.char_width();
                                if gx >= text_start_x && gx < max_code_x {
                                    let is_active = active_guide_col == Some(g_col)
                                        && line_idx >= active_guide_start_line
                                        && line_idx <= active_guide_end_line;
                                    let color = if is_active { self.colors.gutter_fg } else { self.colors.gutter_border };
                                    fb.fill_rect(gx, py, 1, line_height, color);
                                }
                            }
                        }
                    }

                    // Syntax Highlighted Tokens
                    let lexical_tokens = SyntaxHighlighter::tokenize_line(line_text, doc.language);
                    let tokens = if let Some(sem_spans) = doc.semantic_tokens.get(&line_idx) {
                        SyntaxHighlighter::apply_semantic_spans(line_text, lexical_tokens, sem_spans)
                    } else {
                        lexical_tokens
                    };
                    self.render_token_line(fb, &tokens, text_start_x, text_y, max_code_x, doc.scroll_col, self.colors.background);
                    self.render_diagnostic_squiggles_for_line(fb, doc, line_idx, line_text, text_start_x, text_y, max_code_x, 0, None, doc.scroll_col);

                    // Folded line indicator badge: " ··· "
                    if doc.folding.fold_end_for_line(line_idx).is_some() {
                        let text_chars = line_text.chars().count();
                        let badge_x = text_start_x + text_chars.saturating_sub(doc.scroll_col) * self.char_width() + 6;
                        if badge_x + self.char_width() * 4 < max_code_x {
                            let badge_w = self.char_width() * 5;
                            fb.fill_rect(badge_x, py + 2, badge_w, line_height.saturating_sub(4), self.colors.gutter_bg);
                            fb.draw_text(" ··· ", badge_x, text_y, self.colors.accent, None);
                        }
                    }

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

                        // Inline Git Blame (dimmed annotation at end of primary cursor line)
                        if doc.buffer.char_to_position(doc.cursors.primary().head).line == line_idx {
                            if let Some(ref blame) = self.inline_blame {
                                let text_chars = line_text.chars().count();
                                let blame_x = text_start_x + (text_chars.saturating_sub(doc.scroll_col) + 4) * self.char_width();
                                if blame_x < max_code_x {
                                    fb.draw_text_truncated(blame, blame_x, text_y, max_code_x, self.colors.gutter_fg, None);
                                }
                            }
                        }
                    }

                    // Advance to next visible line for the next screen row
                    let next = doc.folding.next_visible_line(current_line, total_lines);
                    if next <= current_line {
                        current_line = total_lines;
                    } else {
                        current_line = next;
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
            while line_idx > 0 && doc.folding.is_line_hidden(line_idx) {
                line_idx = doc.folding.prev_visible_line(line_idx);
            }
            let mut start_sub_idx = doc.scroll_subline;

            while screen_row < visible_lines && line_idx < total_lines {
                let line_cow = doc.buffer.line_cow(line_idx).unwrap_or(std::borrow::Cow::Borrowed(""));
                let line_text: &str = &line_cow;
                let sublines = crate::editor::wrap::WordWrap::compute_sublines(line_text, max_cols);

                if start_sub_idx >= sublines.len() {
                    start_sub_idx = 0;
                }

                let lexical_tokens = SyntaxHighlighter::tokenize_line(line_text, doc.language);
                let tokens = if let Some(sem_spans) = doc.semantic_tokens.get(&line_idx) {
                    SyntaxHighlighter::apply_semantic_spans(line_text, lexical_tokens, sem_spans)
                } else {
                    lexical_tokens
                };

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

                            let is_cur_line = cursor_positions.iter().any(|p| p.line == line_idx);
                            let mut num_buf = [0u8; 16];
                        let num_str = format_line_number_into(line_idx + 1, gutter_cols, &mut num_buf);
                            let line_num_fg = if diag.is_some() {
                                fg_col
                            } else if is_cur_line && is_active {
                                self.colors.tab_active_fg
                            } else {
                                fg_col
                            };
                            fb.draw_text(&num_str, pane_x + self.char_width(), text_y, line_num_fg, None);

                            if let Some(b) = badge {
                                fb.draw_text(b, pane_x + 2, text_y, fg_col, None);
                            }

                            // Code folding indicator in gutter
                            if doc.folding.enabled {
                                let fold_x = pane_x + (gutter_cols + 1) * self.char_width();
                                match doc.folding.is_fold_start(line_idx) {
                                    Some(true) => {
                                        fb.draw_char('▶', fold_x, text_y, self.colors.accent, None);
                                    }
                                    Some(false) => {
                                        fb.draw_char('▼', fold_x, text_y, self.colors.gutter_fg, None);
                                    }
                                    None => {}
                                }
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

                    // Current Line Highlight in Word Wrap
                    let is_cur_subline = is_active && cursor_positions.iter().any(|p| {
                        if p.line != line_idx {
                            return false;
                        }
                        if sub_idx + 1 == sublines.len() {
                            p.col >= sub.char_start && p.col <= sub.char_end
                        } else {
                            p.col >= sub.char_start && p.col < sub.char_end
                        }
                    });
                    if self.highlight_current_line && is_cur_subline {
                        let highlight_w = max_code_x.saturating_sub(text_start_x);
                        fb.fill_rect(text_start_x, py, highlight_w, line_height, self.colors.current_line_bg);
                    }

                    // Column Rulers
                    for &ruler_col in &self.rulers {
                        let rx = text_start_x + ruler_col * self.char_width();
                        if rx >= text_start_x && rx < max_code_x {
                            fb.fill_rect(rx, py, 1, line_height, self.colors.ruler_color);
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

                    // Indent Guides (vertical lines at tab stops)
                    if self.indent_guides && !sub.is_continuation {
                        let is_blank = Self::is_blank_line(doc, line_idx);
                        let indent_spaces = if is_blank {
                            Self::blank_line_indent(doc, line_idx, effective_tab_size)
                        } else {
                            Self::line_indent_spaces(doc, line_idx, effective_tab_size)
                        };
                        let num_guides = indent_spaces / effective_tab_size;
                        for g in 0..num_guides {
                            let g_col = g * effective_tab_size;
                            let gx = text_start_x + g_col * self.char_width();
                            if gx >= text_start_x && gx < max_code_x {
                                let is_active = active_guide_col == Some(g_col)
                                    && line_idx >= active_guide_start_line
                                    && line_idx <= active_guide_end_line;
                                let color = if is_active { self.colors.gutter_fg } else { self.colors.gutter_border };
                                fb.fill_rect(gx, py, 1, line_height, color);
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
                    self.render_diagnostic_squiggles_for_line(fb, doc, line_idx, line_text, text_start_x, text_y, max_code_x, sub.char_start, Some(sub.char_end), 0);

                    // Folded line indicator badge: " ··· "
                    if sub_idx + 1 == sublines.len() && doc.folding.fold_end_for_line(line_idx).is_some() {
                        let text_chars = sub.len_chars();
                        let badge_x = text_start_x + text_chars * self.char_width() + 6;
                        if badge_x + self.char_width() * 4 < max_code_x {
                            let badge_w = self.char_width() * 5;
                            fb.fill_rect(badge_x, py + 2, badge_w, line_height.saturating_sub(4), self.colors.gutter_bg);
                            fb.draw_text(" ··· ", badge_x, text_y, self.colors.accent, None);
                        }
                    }

                    // LSP Inlay Hints
                    if sub_idx + 1 == sublines.len() {
                        if let Some(line_hints) = doc.inlay_hints.get(&line_idx) {
                            if !line_hints.is_empty() {
                                let fold_offset = if doc.folding.fold_end_for_line(line_idx).is_some() {
                                    self.char_width() * 6
                                } else {
                                    0
                                };
                                let mut cur_hint_x = text_start_x + sub.len_chars() * self.char_width() + self.scale_coord(14) + fold_offset;
                                for hint in line_hints {
                                    if cur_hint_x >= max_code_x {
                                        break;
                                    }
                                    let hint_label = hint.label.trim();
                                    let hint_str = match hint.kind {
                                        crate::editor::lsp::InlayHintKind::Parameter => format!(" {}: ", hint_label.trim_end_matches(':')),
                                        crate::editor::lsp::InlayHintKind::Type => {
                                            if hint_label.starts_with(':') {
                                                format!(" {} ", hint_label)
                                            } else {
                                                format!(" : {} ", hint_label)
                                            }
                                        }
                                        _ => format!(" {} ", hint_label),
                                    };
                                    let hw = hint_str.chars().count() * self.char_width();
                                    if cur_hint_x + hw < max_code_x {
                                        let chip_bg = 0x0023283B;
                                        let chip_fg = match hint.kind {
                                            crate::editor::lsp::InlayHintKind::Type => 0x007AA2F7,
                                            crate::editor::lsp::InlayHintKind::Parameter => 0x00BB9AF7,
                                            _ => 0x0073DACA,
                                        };
                                        let chip_h = self.char_height().saturating_sub(2);
                                        let chip_y = text_y + 1;
                                        if self.rounded_corners {
                                            fb.fill_rounded_rect(cur_hint_x, chip_y, hw, chip_h, 3, chip_bg);
                                        } else {
                                            fb.fill_rect(cur_hint_x, chip_y, hw, chip_h, chip_bg);
                                        }
                                        fb.draw_text_clipped(&hint_str, cur_hint_x, text_y, max_code_x, chip_fg, None);
                                        cur_hint_x += hw + self.scale_coord(6);
                                    }
                                }
                            }
                        }
                    }

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

                let next_line = doc.folding.next_visible_line(line_idx, total_lines);
                if next_line <= line_idx {
                    break;
                }
                line_idx = next_line;
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

        // Welcome / Hero watermark on clean empty untitled buffer
        if doc.buffer.len_chars() == 0 && doc.buffer.file_path().is_none() && pane_w >= self.scale_coord(260) && content_h >= self.scale_coord(220) {
            let hero_size = self.scale_coord(64);
            let hero_x = pane_x + (pane_w.saturating_sub(hero_size)) / 2;
            let hero_y = content_y + (content_h.saturating_sub(hero_size + self.scale_coord(60))) / 2;
            self.render_logo(fb, hero_x, hero_y, hero_size, true);

            let welcome_str = "Rustext";
            let w_width = welcome_str.chars().count() * self.char_width();
            let w_x = pane_x + (pane_w.saturating_sub(w_width)) / 2;
            let w_y = hero_y + hero_size + self.scale_coord(10);
            fb.draw_text(welcome_str, w_x, w_y, 0x00E2E8F0, None);

            let hint_str = "Ctrl+P: Archivos  •  Ctrl+Shift+P: Comandos";
            let h_width = hint_str.chars().count() * self.char_width();
            let h_x = pane_x + (pane_w.saturating_sub(h_width)) / 2;
            let h_y = w_y + self.char_height() + self.scale_coord(4);
            if h_x > pane_x + self.scale_coord(10) {
                fb.draw_text_truncated(hint_str, h_x, h_y, pane_x + pane_w - self.scale_coord(10), 0x0064748B, None);
            }
        }

        // 5b. Sticky Scroll Scope Headers
        self.render_sticky_scroll(
            fb,
            doc,
            pane_x,
            content_y,
            gutter_w,
            gutter_cols,
            max_code_x,
            line_height,
        );

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

            // Compute active indent guide based on primary cursor position
            let effective_tab_size = self.tab_size.max(1);
            let (active_guide_col, active_guide_start_line, active_guide_end_line) = if self.highlight_active_indent_guide {
                let cursor_pos = doc.buffer.char_to_position(doc.cursors.primary().head);
                let cur_indent = if Self::is_blank_line(doc, cursor_pos.line) {
                    Self::blank_line_indent(doc, cursor_pos.line, effective_tab_size)
                } else {
                    Self::line_indent_spaces(doc, cursor_pos.line, effective_tab_size)
                };
                if cur_indent >= effective_tab_size {
                    let guide_col = ((cur_indent.saturating_sub(1)) / effective_tab_size) * effective_tab_size;
                    let mut start_l = cursor_pos.line;
                    while start_l > 0 {
                        let prev = start_l - 1;
                        if !Self::is_blank_line(doc, prev) {
                            let ind = Self::line_indent_spaces(doc, prev, effective_tab_size);
                            if ind < guide_col + effective_tab_size {
                                start_l = prev;
                                break;
                            }
                        }
                        start_l = prev;
                        if cursor_pos.line - start_l > 500 {
                            break;
                        }
                    }
                    let mut end_l = cursor_pos.line;
                    let total_l = doc.buffer.len_lines();
                    while end_l + 1 < total_l {
                        let next = end_l + 1;
                        if !Self::is_blank_line(doc, next) {
                            let ind = Self::line_indent_spaces(doc, next, effective_tab_size);
                            if ind < guide_col + effective_tab_size {
                                end_l = next;
                                break;
                            }
                        }
                        end_l = next;
                        if end_l - cursor_pos.line > 500 {
                            break;
                        }
                    }
                    (Some(guide_col), start_l, end_l)
                } else {
                    (None, 0, 0)
                }
            } else {
                (None, 0, 0)
            };

            if !self.word_wrap {
                let mut current_line = doc.scroll_row;
                while current_line > 0 && doc.folding.is_line_hidden(current_line) {
                    current_line = doc.folding.prev_visible_line(current_line);
                }

                for row in 0..visible_lines {
                    let line_idx = current_line;
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

                            let is_cur_line = cursor_positions.iter().any(|p| p.line == line_idx);
                            let mut num_buf = [0u8; 16];
                        let num_str = format_line_number_into(line_idx + 1, gutter_cols, &mut num_buf);
                            let line_num_fg = if diag.is_some() {
                                fg_col
                            } else if is_cur_line {
                                self.colors.tab_active_fg
                            } else {
                                fg_col
                            };
                            fb.draw_text(&num_str, canvas_x_start + self.char_width(), text_y, line_num_fg, None);

                            if let Some(b) = badge {
                                fb.draw_text(b, canvas_x_start + 2, text_y, fg_col, None);
                            }

                            // Code folding indicator in gutter
                            if doc.folding.enabled {
                                let fold_x = canvas_x_start + (gutter_cols + 1) * self.char_width();
                                match doc.folding.is_fold_start(line_idx) {
                                    Some(true) => {
                                        fb.draw_char('▶', fold_x, text_y, self.colors.accent, None);
                                    }
                                    Some(false) => {
                                        fb.draw_char('▼', fold_x, text_y, self.colors.gutter_fg, None);
                                    }
                                    None => {}
                                }
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

                        let line_cow = doc.buffer.line_cow(line_idx).unwrap_or(std::borrow::Cow::Borrowed(""));
                        let line_text: &str = &line_cow;

                        // Current Line Highlight
                        let is_cur_line = cursor_positions.iter().any(|p| p.line == line_idx);
                        if self.highlight_current_line && is_cur_line {
                            let highlight_w = max_code_x.saturating_sub(text_start_x);
                            fb.fill_rect(text_start_x, py, highlight_w, line_height, self.colors.current_line_bg);
                        }

                        // Column Rulers
                        for &ruler_col in &self.rulers {
                            if ruler_col >= doc.scroll_col {
                                let rx = text_start_x + (ruler_col - doc.scroll_col) * self.char_width();
                                if rx >= text_start_x && rx < max_code_x {
                                    fb.fill_rect(rx, py, 1, line_height, self.colors.ruler_color);
                                }
                            }
                        }

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

                        // Indent Guides (vertical lines at tab stops)
                        if self.indent_guides {
                            let is_blank = Self::is_blank_line(doc, line_idx);
                            let indent_spaces = if is_blank {
                                Self::blank_line_indent(doc, line_idx, effective_tab_size)
                            } else {
                                Self::line_indent_spaces(doc, line_idx, effective_tab_size)
                            };
                            let num_guides = indent_spaces / effective_tab_size;
                            for g in 0..num_guides {
                                let g_col = g * effective_tab_size;
                                if g_col >= doc.scroll_col {
                                    let gx = text_start_x + (g_col - doc.scroll_col) * self.char_width();
                                    if gx >= text_start_x && gx < max_code_x {
                                        let is_active = active_guide_col == Some(g_col)
                                            && line_idx >= active_guide_start_line
                                            && line_idx <= active_guide_end_line;
                                        let color = if is_active { self.colors.gutter_fg } else { self.colors.gutter_border };
                                        fb.fill_rect(gx, py, 1, line_height, color);
                                    }
                                }
                            }
                        }

                        let tokens = SyntaxHighlighter::tokenize_line(line_text, doc.language);
                        self.render_token_line(fb, &tokens, text_start_x, text_y, max_code_x, doc.scroll_col, self.colors.background);
                        self.render_diagnostic_squiggles_for_line(fb, doc, line_idx, line_text, text_start_x, text_y, max_code_x, 0, None, doc.scroll_col);

                        // Folded line indicator badge: " ··· "
                        if doc.folding.fold_end_for_line(line_idx).is_some() {
                            let text_chars = line_text.chars().count();
                            let badge_x = text_start_x + text_chars * self.char_width() + 6;
                            if badge_x + self.char_width() * 4 < max_code_x {
                                let badge_w = self.char_width() * 5;
                                fb.fill_rect(badge_x, py + 2, badge_w, line_height.saturating_sub(4), self.colors.gutter_bg);
                                fb.draw_text(" ··· ", badge_x, text_y, self.colors.accent, None);
                            }
                        }

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

                        // Advance to next visible line for the next screen row
                        let next = doc.folding.next_visible_line(current_line, total_lines);
                        if next <= current_line {
                            current_line = total_lines;
                        } else {
                            current_line = next;
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
                while line_idx > 0 && doc.folding.is_line_hidden(line_idx) {
                    line_idx = doc.folding.prev_visible_line(line_idx);
                }
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

                                let is_cur_line = cursor_positions.iter().any(|p| p.line == line_idx);
                                let mut num_buf = [0u8; 16];
                        let num_str = format_line_number_into(line_idx + 1, gutter_cols, &mut num_buf);
                                let line_num_fg = if diag.is_some() {
                                    fg_col
                                } else if is_cur_line {
                                    self.colors.tab_active_fg
                                } else {
                                    fg_col
                                };
                                fb.draw_text(&num_str, canvas_x_start + self.char_width(), text_y, line_num_fg, None);

                                if let Some(b) = badge {
                                    fb.draw_text(b, canvas_x_start + 2, text_y, fg_col, None);
                                }

                                // Code folding indicator in gutter
                                if doc.folding.enabled {
                                    let fold_x = canvas_x_start + (gutter_cols + 1) * self.char_width();
                                    match doc.folding.is_fold_start(line_idx) {
                                        Some(true) => {
                                            fb.draw_char('▶', fold_x, text_y, self.colors.accent, None);
                                        }
                                        Some(false) => {
                                            fb.draw_char('▼', fold_x, text_y, self.colors.gutter_fg, None);
                                        }
                                        None => {}
                                    }
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

                        // Current Line Highlight in Word Wrap
                        let is_cur_subline = cursor_positions.iter().any(|p| {
                            if p.line != line_idx {
                                return false;
                            }
                            if sub_idx + 1 == sublines.len() {
                                p.col >= sub.char_start && p.col <= sub.char_end
                            } else {
                                p.col >= sub.char_start && p.col < sub.char_end
                            }
                        });
                        if self.highlight_current_line && is_cur_subline {
                            let highlight_w = max_code_x.saturating_sub(text_start_x);
                            fb.fill_rect(text_start_x, py, highlight_w, line_height, self.colors.current_line_bg);
                        }

                        // Column Rulers
                        for &ruler_col in &self.rulers {
                            let rx = text_start_x + ruler_col * self.char_width();
                            if rx >= text_start_x && rx < max_code_x {
                                fb.fill_rect(rx, py, 1, line_height, self.colors.ruler_color);
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

                        // Indent Guides (vertical lines at tab stops)
                        if self.indent_guides && !sub.is_continuation {
                            let is_blank = Self::is_blank_line(doc, line_idx);
                            let indent_spaces = if is_blank {
                                Self::blank_line_indent(doc, line_idx, effective_tab_size)
                            } else {
                                Self::line_indent_spaces(doc, line_idx, effective_tab_size)
                            };
                            let num_guides = indent_spaces / effective_tab_size;
                            for g in 0..num_guides {
                                let g_col = g * effective_tab_size;
                                let gx = text_start_x + g_col * self.char_width();
                                if gx >= text_start_x && gx < max_code_x {
                                    let is_active = active_guide_col == Some(g_col)
                                        && line_idx >= active_guide_start_line
                                        && line_idx <= active_guide_end_line;
                                    let color = if is_active { self.colors.gutter_fg } else { self.colors.gutter_border };
                                    fb.fill_rect(gx, py, 1, line_height, color);
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
                        self.render_diagnostic_squiggles_for_line(fb, doc, line_idx, line_text, text_start_x, text_y, max_code_x, sub.char_start, Some(sub.char_end), 0);

                        // Folded line indicator badge: " ··· "
                        if sub_idx + 1 == sublines.len() && doc.folding.fold_end_for_line(line_idx).is_some() {
                            let text_chars = sub.len_chars();
                            let badge_x = text_start_x + text_chars * self.char_width() + 6;
                            if badge_x + self.char_width() * 4 < max_code_x {
                                let badge_w = self.char_width() * 5;
                                fb.fill_rect(badge_x, py + 2, badge_w, line_height.saturating_sub(4), self.colors.gutter_bg);
                                fb.draw_text(" ··· ", badge_x, text_y, self.colors.accent, None);
                            }
                        }

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

                    let next = doc.folding.next_visible_line(line_idx, total_lines);
                    if next <= line_idx {
                        line_idx = total_lines;
                    } else {
                        line_idx = next;
                    }
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

            // Welcome / Hero watermark on clean empty untitled buffer
            let canvas_w = max_code_x.saturating_sub(canvas_x_start);
            if doc.buffer.len_chars() == 0 && doc.buffer.file_path().is_none() && canvas_w >= self.scale_coord(260) && content_height >= self.scale_coord(220) {
                let hero_size = self.scale_coord(80);
                let hero_x = canvas_x_start + (canvas_w.saturating_sub(hero_size)) / 2;
                let hero_y = content_y_start + (content_height.saturating_sub(hero_size + self.scale_coord(60))) / 2;
                self.render_logo(fb, hero_x, hero_y, hero_size, true);

                let welcome_str = "Rustext";
                let w_width = welcome_str.chars().count() * self.char_width();
                let w_x = canvas_x_start + (canvas_w.saturating_sub(w_width)) / 2;
                let w_y = hero_y + hero_size + self.scale_coord(12);
                fb.draw_text(welcome_str, w_x, w_y, 0x00E2E8F0, None);

                let hint_str = "Ctrl+P: Archivos  •  Ctrl+Shift+P: Comandos  •  F1: Acerca de";
                let h_width = hint_str.chars().count() * self.char_width();
                let h_x = canvas_x_start + (canvas_w.saturating_sub(h_width)) / 2;
                let h_y = w_y + self.char_height() + self.scale_coord(4);
                if h_x > canvas_x_start + self.scale_coord(10) {
                    fb.draw_text_truncated(hint_str, h_x, h_y, max_code_x - self.scale_coord(10), 0x0064748B, None);
                }
            }

            // 7b. Sticky Scroll Scope Headers
            self.render_sticky_scroll(
                fb,
                doc,
                canvas_x_start,
                content_y_start,
                gutter_width,
                gutter_cols,
                max_code_x,
                line_height,
            );

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

        // 10b. Draw Floating Signature Help Tooltip if present
        if let Some(ref sig) = self.signature_help {
            self.render_signature_help(fb, sig, doc, sidebar_w, menu_bar.is_some());
        }

        // 11. Draw Git Hunk Diff Popup if active
        if let Some(ref hunk) = self.active_hunk_popup {
            self.render_hunk_diff_popup(fb, hunk);
        }

        // 12. Draw Prompt Modal overlay if present
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

        // Render client-side window control buttons (Minimize, Maximize, Close) in top-right corner
        let btn_w = menu_bar.control_button_width();
        let total_w = menu_bar.window_controls_total_width();
        if fb.width > total_w + cur_x + 10 {
            let start_x = fb.width - total_w;
            let btn_h = self.menu_bar_height;
            let icon_y = (btn_h.saturating_sub(self.char_height())) / 2;

            // 1. Minimize button (index 0)
            let is_min_hover = menu_bar.hovered_control == Some(crate::editor::menu::WindowControlButton::Minimize);
            let min_bg = if is_min_hover { self.colors.modal_border } else { self.colors.modal_header_bg };
            fb.fill_rect(start_x, 0, btn_w, btn_h, min_bg);
            let min_tx = start_x + (btn_w.saturating_sub(self.char_width())) / 2;
            let min_fg = if is_min_hover { 0x00FFFFFF } else { 0x00A0A0B0 };
            fb.draw_text("—", min_tx, icon_y, min_fg, None);

            // 2. Maximize / Restore button (index 1)
            let is_max_hover = menu_bar.hovered_control == Some(crate::editor::menu::WindowControlButton::Maximize);
            let max_bg = if is_max_hover { self.colors.modal_border } else { self.colors.modal_header_bg };
            let max_x = start_x + btn_w;
            fb.fill_rect(max_x, 0, btn_w, btn_h, max_bg);
            let max_tx = max_x + (btn_w.saturating_sub(self.char_width())) / 2;
            let max_fg = if is_max_hover { 0x00FFFFFF } else { 0x00A0A0B0 };
            fb.draw_text("□", max_tx, icon_y, max_fg, None);

            // 3. Close button (index 2)
            let is_close_hover = menu_bar.hovered_control == Some(crate::editor::menu::WindowControlButton::Close);
            let close_bg = if is_close_hover { 0x00E05252 } else { self.colors.modal_header_bg };
            let close_x = start_x + btn_w * 2;
            fb.fill_rect(close_x, 0, btn_w, btn_h, close_bg);
            let close_tx = close_x + (btn_w.saturating_sub(self.char_width())) / 2;
            let close_fg = if is_close_hover { 0x00FFFFFF } else { 0x00A0A0B0 };
            fb.draw_text("✕", close_tx, icon_y, close_fg, None);
        }
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

    /// Computes the horizontal `(x, width)` rectangle of every tab in the tab bar.
    /// Mirrors the layout used by `render_tab_bar` so hit-testing and drag & drop stay aligned.
    pub fn tab_rects(&self, titles: &[String], offset_x: usize) -> Vec<(usize, usize)> {
        let mut cur_x = offset_x + self.scale_coord(8);
        let mut rects = Vec::with_capacity(titles.len());
        for (idx, title) in titles.iter().enumerate() {
            let tab_text = format!(" {} {} × ", idx + 1, title);
            let tab_w = tab_text.chars().count() * self.char_width() + self.scale_coord(8);
            rects.push((cur_x, tab_w));
            cur_x += tab_w + self.scale_coord(3);
        }
        rects
    }

    /// Returns the final index a dragged tab `src` should take when the mouse is at `x`.
    /// Counts the other tabs whose midpoint lies left of the cursor, which gives natural
    /// hysteresis: a tab only swaps once the cursor crosses its neighbour's midpoint.
    pub fn tab_drop_index(rects: &[(usize, usize)], src: usize, x: usize) -> usize {
        rects
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != src)
            .filter(|(_, (start, width))| start + width / 2 < x)
            .count()
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

    pub fn render_status_bar(&self, fb: &mut Framebuffer, doc: &Document, status_msg: Option<&str>) {
        let bar_y = fb.height.saturating_sub(self.status_bar_height);
        fb.fill_rect(0, bar_y, fb.width, self.status_bar_height, self.colors.status_bar_bg);

        let chip_h = self.status_bar_height.saturating_sub(self.scale_coord(4)).max(self.scale_coord(14));
        let chip_y = bar_y + 2;
        let text_y = bar_y + (self.status_bar_height.saturating_sub(self.char_height())) / 2;

        let mut left_x = self.scale_coord(6);

        // 1. Mode Chip: [ EDIT ] or Modal Mode: [ NORMAL ], [ INSERT ], [ VISUAL ]
        let custom_label;
        let mode_label: &str = if let Some(m) = &self.modal_label {
            custom_label = format!(" {} ", m);
            &custom_label
        } else {
            " EDIT "
        };
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
            // Workspace / Project Chip
            if let Some(ws) = &self.workspace_name {
                let ws_text = format!(" 📦 {} ", ws);
                let ws_w = ws_text.chars().count() * self.char_width() + self.scale_coord(4);
                if self.rounded_corners {
                    fb.fill_rounded_rect(left_x, chip_y, ws_w, chip_h, 3, self.colors.chip_bg);
                } else {
                    fb.fill_rect(left_x, chip_y, ws_w, chip_h, self.colors.chip_bg);
                }
                fb.draw_text(&ws_text, left_x + 2, text_y, self.colors.accent, None);
                left_x += ws_w + self.scale_coord(6);
            }

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
    /// Can be rendered as an application icon badge (with dark squircle and subtle border)
    /// or as a standalone faceted vector graphic.
    pub fn render_logo(&self, fb: &mut Framebuffer, x: usize, y: usize, size: usize, with_badge: bool) {
        if size == 0 {
            return;
        }

        let ox = x as f64;
        let oy = y as f64;
        let sf = size as f64;

        if with_badge {
            let radius = sf * 0.203;
            // 1. Dark squircle app icon base (#181926)
            fb.fill_rounded_rect_alpha_aa(ox, oy, sf, sf, radius, 0x00181926, 255);
            // 2. Subtle top inner highlight
            fb.fill_rounded_rect_alpha_aa(
                ox + 1.0,
                oy + 1.0,
                (sf - 2.0).max(0.0),
                (sf * 0.38).min(sf),
                (radius - 1.0).max(0.0),
                0x0024283B,
                80,
            );
            // 3. Squircle perimeter border (#3B4261)
            let stroke_w = (sf / 160.0).max(1.0);
            fb.draw_rounded_outline_aa(ox, oy, sf, sf, radius, stroke_w, 0x003B4261);
        }

        let pt = |u: f64, v: f64| -> (f64, f64) {
            (ox + (u / 512.0) * sf, oy + (v / 512.0) * sf)
        };

        // Concept 1: The Faceted Ribbon R Geometry
        // 1. Left Spine Facet (Orange Top / Spine)
        fb.fill_triangle_aa(pt(170.0, 110.0), pt(240.0, 110.0), pt(240.0, 260.0), 0x00FF6B35);
        fb.fill_triangle_aa(pt(170.0, 110.0), pt(240.0, 260.0), pt(170.0, 330.0), 0x00E0401D);

        // 2. Left Spine Lower Facet (Cyan Wedge)
        fb.fill_triangle_aa(pt(170.0, 330.0), pt(240.0, 260.0), pt(240.0, 390.0), 0x000EA5E9);
        fb.fill_triangle_aa(pt(170.0, 330.0), pt(240.0, 390.0), pt(170.0, 390.0), 0x000284C7);

        // 3. Top Horizontal Bar (Bright Orange)
        fb.fill_triangle_aa(pt(240.0, 110.0), pt(330.0, 110.0), pt(290.0, 180.0), 0x00FF7A45);
        fb.fill_triangle_aa(pt(240.0, 110.0), pt(290.0, 180.0), pt(240.0, 180.0), 0x00FF6B35);

        // 4. Upper Outer Loop (Electric Cyan Wing)
        fb.fill_triangle_aa(pt(330.0, 110.0), pt(380.0, 165.0), pt(290.0, 180.0), 0x007DD3FC);
        fb.fill_triangle_aa(pt(290.0, 180.0), pt(380.0, 165.0), pt(330.0, 210.0), 0x0038BDF8);
        fb.fill_triangle_aa(pt(380.0, 165.0), pt(380.0, 225.0), pt(330.0, 210.0), 0x000EA5E9);
        fb.fill_triangle_aa(pt(330.0, 210.0), pt(380.0, 225.0), pt(330.0, 270.0), 0x000284C7);
        fb.fill_triangle_aa(pt(330.0, 210.0), pt(330.0, 270.0), pt(290.0, 270.0), 0x000369A1);

        // 5. Diagonal Leg Upper Transition (Faceted Shading)
        fb.fill_triangle_aa(pt(240.0, 240.0), pt(290.0, 240.0), pt(330.0, 270.0), 0x00C2410C);
        fb.fill_triangle_aa(pt(240.0, 240.0), pt(330.0, 270.0), pt(270.0, 290.0), 0x009A3412);

        // 6. Diagonal Leg Forward Cut (Bright Orange Core)
        fb.fill_triangle_aa(pt(270.0, 290.0), pt(330.0, 270.0), pt(380.0, 390.0), 0x00F97316);
        fb.fill_triangle_aa(pt(270.0, 290.0), pt(380.0, 390.0), pt(310.0, 390.0), 0x00EA580C);

        // 7. Inner Negative Space Cutout
        let cutout_col = if with_badge { 0x00181926 } else { self.colors.background };
        fb.fill_triangle_aa(pt(240.0, 180.0), pt(290.0, 180.0), pt(310.0, 210.0), cutout_col);
        fb.fill_triangle_aa(pt(240.0, 180.0), pt(310.0, 210.0), pt(290.0, 240.0), cutout_col);
        fb.fill_triangle_aa(pt(240.0, 180.0), pt(290.0, 240.0), pt(240.0, 240.0), cutout_col);

        // 8. Titanium Highlights & Crease Lines
        let stroke_w = (sf / 160.0).max(1.0);
        // Spine left highlight
        fb.draw_line_aa(ox + (170.0 / 512.0) * sf, oy + (390.0 / 512.0) * sf, ox + (170.0 / 512.0) * sf, oy + (110.0 / 512.0) * sf, stroke_w, 0x00E2E8F0);
        // Top edge highlight
        fb.draw_line_aa(ox + (170.0 / 512.0) * sf, oy + (110.0 / 512.0) * sf, ox + (330.0 / 512.0) * sf, oy + (110.0 / 512.0) * sf, stroke_w, 0x00FFFFFF);
        // Outer loop shoulder highlight
        fb.draw_line_aa(ox + (330.0 / 512.0) * sf, oy + (110.0 / 512.0) * sf, ox + (380.0 / 512.0) * sf, oy + (165.0 / 512.0) * sf, stroke_w, 0x00CBD5E1);

        // Leg highlight crease
        fb.draw_line_aa(ox + (240.0 / 512.0) * sf, oy + (240.0 / 512.0) * sf, ox + (330.0 / 512.0) * sf, oy + (270.0 / 512.0) * sf, stroke_w * 0.8, 0x00FDBA74);
        fb.draw_line_aa(ox + (330.0 / 512.0) * sf, oy + (270.0 / 512.0) * sf, ox + (380.0 / 512.0) * sf, oy + (390.0 / 512.0) * sf, stroke_w * 0.9, 0x00FED7AA);

        // Cyan wedge crease
        fb.draw_line_aa(ox + (170.0 / 512.0) * sf, oy + (330.0 / 512.0) * sf, ox + (240.0 / 512.0) * sf, oy + (260.0 / 512.0) * sf, stroke_w * 0.8, 0x007DD3FC);
    }

    /// Renders a native-styled About Dialog Window with title bar, close button, author details, and logo.
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
