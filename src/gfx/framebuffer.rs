//! Software framebuffer with 2D rendering primitives and font rasterization.
//! Zero third-party dependencies, purely standard library Rust.

use super::analytic_aa::AnalyticAA;
use super::blur::BlurEngine;
use super::edge_perception::EdgePerception;
use super::font::{get_glyph, FontFamily, FontWeight, FontRasterizer, FONT_HEIGHT, FONT_WIDTH};
use super::gamma::{blend_pixel_gamma, blend_pixel_subpixel_gamma};
use super::hinting::{FontHinter, HintingLevel};
use super::sdf::SdfFontRasterizer;
use super::shaping::FontShaper;
use super::stem_darkening::StemDarkener;

/// Software pixel buffer in 32-bit ARGB (0xAARRGGBB) format.
#[derive(Debug, Clone)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
    pub font_scale: usize,
    pub font_family: FontFamily,
    pub font_weight: FontWeight,
    pub font_size: usize,
    pub subpixel_antialiasing: bool,
    pub letter_spacing: usize,
    pub font_aspect_ratio: f64,
    pub stem_darkening: bool,
    pub stem_darkening_amount: f64,
    pub sdf_font_rendering: bool,
    pub use_ttf_font: bool,
}

impl Framebuffer {
    /// Creates a new framebuffer with given pixel dimensions.
    pub fn new(width: usize, height: usize) -> Self {
        let size = width * height;
        Self {
            width,
            height,
            pixels: vec![0xFF000000; size],
            font_scale: 1,
            font_family: FontFamily::Standard,
            font_weight: FontWeight::Normal,
            font_size: 12,
            subpixel_antialiasing: false,
            letter_spacing: 0,
            font_aspect_ratio: 0.50,
            stem_darkening: true,
            stem_darkening_amount: 1.0,
            sdf_font_rendering: false,
            use_ttf_font: false,
        }
    }

    /// Resizes the framebuffer, preserving existing pixels when possible.
    pub fn resize(&mut self, new_width: usize, new_height: usize) {
        if self.width == new_width && self.height == new_height {
            return;
        }

        let mut new_pixels = vec![0xFF000000; new_width * new_height];
        let copy_w = self.width.min(new_width);
        let copy_h = self.height.min(new_height);

        for y in 0..copy_h {
            let src_start = y * self.width;
            let dst_start = y * new_width;
            new_pixels[dst_start..dst_start + copy_w]
                .copy_from_slice(&self.pixels[src_start..src_start + copy_w]);
        }

        self.width = new_width;
        self.height = new_height;
        self.pixels = new_pixels;
    }

    /// Fills the entire framebuffer with a single color.
    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    /// Sets a pixel at (x, y) with boundary clipping.
    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = color;
        }
    }

    /// Gets pixel color at (x, y).
    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x]
        } else {
            0
        }
    }

    /// Fills an axis-aligned rectangle with clipping.
    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        if x >= self.width || y >= self.height || w == 0 || h == 0 {
            return;
        }

        let max_x = (x + w).min(self.width);
        let max_y = (y + h).min(self.height);
        let fill_w = max_x - x;

        for cur_y in y..max_y {
            let offset = cur_y * self.width + x;
            self.pixels[offset..offset + fill_w].fill(color);
        }
    }

    /// Draws a hollow rectangle outline.
    pub fn draw_rect_outline(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.fill_rect(x, y, w, 1, color); // Top
        self.fill_rect(x, y + h - 1, w, 1, color); // Bottom
        self.fill_rect(x, y, 1, h, color); // Left
        self.fill_rect(x + w - 1, y, 1, h, color); // Right
    }

    /// Blends a color with alpha (0..=255) over the existing pixel at (x, y).
    #[inline]
    pub fn blend_pixel(&mut self, x: usize, y: usize, color: u32, alpha: u8) {
        if x < self.width && y < self.height {
            if alpha == 255 {
                self.pixels[y * self.width + x] = color;
                return;
            }
            if alpha == 0 {
                return;
            }

            let idx = y * self.width + x;
            let dst = self.pixels[idx];
            let inv_a = 255 - alpha as u32;
            let a = alpha as u32;

            let sr = (color >> 16) & 0xFF;
            let sg = (color >> 8) & 0xFF;
            let sb = color & 0xFF;

            let dr = (dst >> 16) & 0xFF;
            let dg = (dst >> 8) & 0xFF;
            let db = dst & 0xFF;

            let r = (sr * a + dr * inv_a) / 255;
            let g = (sg * a + dg * inv_a) / 255;
            let b = (sb * a + db * inv_a) / 255;

            self.pixels[idx] = 0xFF000000 | (r << 16) | (g << 8) | b;
        }
    }

    /// Blends a color with alpha (0..=255) over the existing pixel at (x, y) with gamma-correct physical light integration.
    #[inline]
    pub fn blend_pixel_gamma(&mut self, x: usize, y: usize, color: u32, alpha: u8) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            let dst = self.pixels[idx];
            self.pixels[idx] = super::gamma::blend_pixel_gamma(dst, color, alpha);
        }
    }

    /// Fills a rectangle with semi-transparent alpha blending.
    pub fn fill_rect_alpha(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32, alpha: u8) {
        if alpha == 0 || x >= self.width || y >= self.height || w == 0 || h == 0 {
            return;
        }
        if alpha == 255 {
            self.fill_rect(x, y, w, h, color);
            return;
        }

        let max_x = (x + w).min(self.width);
        let max_y = (y + h).min(self.height);

        for cur_y in y..max_y {
            for cur_x in x..max_x {
                self.blend_pixel(cur_x, cur_y, color, alpha);
            }
        }
    }

    /// Fills a modern rectangle with smoothly rounded corners.
    pub fn fill_rounded_rect(&mut self, x: usize, y: usize, w: usize, h: usize, radius: usize, color: u32) {
        if w == 0 || h == 0 {
            return;
        }
        let r = radius.min(w / 2).min(h / 2);
        if r == 0 {
            self.fill_rect(x, y, w, h, color);
            return;
        }

        let max_x = (x + w).min(self.width);
        let max_y = (y + h).min(self.height);
        let r_sq = (r * r) as i32;

        for cur_y in y..max_y {
            let dy = if cur_y < y + r {
                (y + r - 1) as i32 - cur_y as i32
            } else if cur_y >= y + h - r {
                cur_y as i32 - (y + h - r) as i32
            } else {
                -1
            };

            for cur_x in x..max_x {
                let dx = if cur_x < x + r {
                    (x + r - 1) as i32 - cur_x as i32
                } else if cur_x >= x + w - r {
                    cur_x as i32 - (x + w - r) as i32
                } else {
                    -1
                };

                if dx >= 0 && dy >= 0 {
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq <= r_sq {
                        self.set_pixel(cur_x, cur_y, color);
                    }
                } else {
                    self.set_pixel(cur_x, cur_y, color);
                }
            }
        }
    }

    /// Fills a rounded rectangle with semi-transparent alpha blending.
    pub fn fill_rounded_rect_alpha(&mut self, x: usize, y: usize, w: usize, h: usize, radius: usize, color: u32, alpha: u8) {
        if alpha == 0 || w == 0 || h == 0 {
            return;
        }
        let r = radius.min(w / 2).min(h / 2);
        if r == 0 {
            self.fill_rect_alpha(x, y, w, h, color, alpha);
            return;
        }

        let max_x = (x + w).min(self.width);
        let max_y = (y + h).min(self.height);
        let r_sq = (r * r) as i32;

        for cur_y in y..max_y {
            let dy = if cur_y < y + r {
                (y + r - 1) as i32 - cur_y as i32
            } else if cur_y >= y + h - r {
                cur_y as i32 - (y + h - r) as i32
            } else {
                -1
            };

            for cur_x in x..max_x {
                let dx = if cur_x < x + r {
                    (x + r - 1) as i32 - cur_x as i32
                } else if cur_x >= x + w - r {
                    cur_x as i32 - (x + w - r) as i32
                } else {
                    -1
                };

                if dx >= 0 && dy >= 0 {
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq <= r_sq {
                        self.blend_pixel(cur_x, cur_y, color, alpha);
                    }
                } else {
                    self.blend_pixel(cur_x, cur_y, color, alpha);
                }
            }
        }
    }

    /// Draws a modern outline with rounded corners.
    pub fn draw_rounded_outline(&mut self, x: usize, y: usize, w: usize, h: usize, radius: usize, color: u32) {
        if w < 2 || h < 2 {
            return;
        }
        let r = radius.min(w / 2).min(h / 2);
        if r == 0 {
            self.draw_rect_outline(x, y, w, h, color);
            return;
        }

        let max_x = (x + w).min(self.width);
        let max_y = (y + h).min(self.height);
        let r_outer_sq = (r * r) as i32;
        let r_inner_sq = ((r.saturating_sub(1)) * (r.saturating_sub(1))) as i32;

        for cur_y in y..max_y {
            let dy = if cur_y < y + r {
                (y + r - 1) as i32 - cur_y as i32
            } else if cur_y >= y + h - r {
                cur_y as i32 - (y + h - r) as i32
            } else {
                -1
            };

            let is_edge_y = cur_y == y || cur_y == y + h - 1;

            for cur_x in x..max_x {
                let dx = if cur_x < x + r {
                    (x + r - 1) as i32 - cur_x as i32
                } else if cur_x >= x + w - r {
                    cur_x as i32 - (x + w - r) as i32
                } else {
                    -1
                };

                let is_edge_x = cur_x == x || cur_x == x + w - 1;

                if dx >= 0 && dy >= 0 {
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq <= r_outer_sq && dist_sq >= r_inner_sq {
                        self.set_pixel(cur_x, cur_y, color);
                    }
                } else if is_edge_y || is_edge_x {
                    self.set_pixel(cur_x, cur_y, color);
                }
            }
        }
    }

    /// Renders a soft, multi-layered, semi-transparent drop shadow underneath a floating element.
    pub fn draw_drop_shadow(&mut self, x: usize, y: usize, w: usize, h: usize, radius: usize, blur_radius: usize) {
        if blur_radius == 0 || w == 0 || h == 0 {
            return;
        }

        let base_alpha: u32 = 60; // Subtle, elegant translucent dark shadow
        for i in (1..=blur_radius).rev() {
            let offset_y = blur_radius / 3 + 2;
            let shadow_x = x.saturating_sub(i);
            let shadow_y = (y + offset_y).saturating_sub(i);
            let shadow_w = w + i * 2;
            let shadow_h = h + i * 2;
            let alpha = ((base_alpha * (blur_radius - i + 1) as u32) / (blur_radius as u32 * 2)) as u8;

            self.fill_rounded_rect_alpha(shadow_x, shadow_y, shadow_w, shadow_h, radius + i, 0x00000000, alpha);
        }
    }

    /// Sets the font scaling factor for automatic scaling during draw_char and draw_text.
    pub fn set_font_scale(&mut self, scale: usize) {
        self.font_scale = scale.max(1);
    }

    /// Sets whether subpixel LCD anti-aliasing (RGB ClearType) is enabled for font rendering.
    pub fn set_subpixel_antialiasing(&mut self, enabled: bool) {
        self.subpixel_antialiasing = enabled;
    }

    /// Sets additional horizontal letter spacing (tracking) in pixels.
    pub fn set_letter_spacing(&mut self, spacing: usize) {
        self.letter_spacing = spacing;
    }

    /// Sets the ergonomic font aspect ratio (width / height). Default is 0.50 (compact 8x16).
    pub fn set_font_aspect_ratio(&mut self, ratio: f64) {
        self.font_aspect_ratio = ratio;
    }

    /// Sets whether optical stem darkening in dark mode is enabled.
    pub fn set_stem_darkening(&mut self, enabled: bool) {
        self.stem_darkening = enabled;
    }

    /// Sets the optical stem darkening strength factor (0.0 to 2.0).
    pub fn set_stem_darkening_amount(&mut self, amount: f64) {
        self.stem_darkening_amount = amount;
    }

    /// Sets whether high-resolution Signed Distance Field (SDF) rasterization is enabled.
    pub fn set_sdf_font_rendering(&mut self, enabled: bool) {
        self.sdf_font_rendering = enabled;
    }

    /// Sets whether TrueType vector outline rasterization is enabled.
    pub fn set_use_ttf_font(&mut self, enabled: bool) {
        self.use_ttf_font = enabled;
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

    /// Returns the character cell dimensions `(width, height)` in pixels including letter spacing.
    pub fn char_cell_size(&self) -> (usize, usize) {
        let h = self.char_height();
        let w = self.glyph_width() + self.letter_spacing * self.font_scale;
        (w, h)
    }

    /// Draws a single character glyph at pixel coordinates (x, y) with subpixel anti-aliasing and gamma correction.
    pub fn draw_char(&mut self, ch: char, x: usize, y: usize, fg: u32, bg: Option<u32>) {
        let (char_w, char_h) = self.char_cell_size();
        let is_box = FontRasterizer::is_box_drawing(ch);
        let glyph_w = if is_box { char_w } else { self.glyph_width() };
        let offset_x = if is_box { 0 } else { (char_w.saturating_sub(glyph_w)) / 2 };

        if let Some(bg_color) = bg {
            self.fill_rect(x, y, char_w, char_h, bg_color);
        }

        if self.subpixel_antialiasing {
            self.draw_char_subpixel(ch, x + offset_x, y, fg, bg, glyph_w, char_h);
            return;
        }

        if self.font_scale > 1 || self.font_size != 12 || self.font_family != FontFamily::Standard || self.font_weight != FontWeight::Normal || glyph_w != FONT_WIDTH || char_h != FONT_HEIGHT || self.sdf_font_rendering || self.use_ttf_font {
            self.draw_char_sized(ch, x + offset_x, y, fg, bg, glyph_w, char_h);
            return;
        }

        if x + offset_x >= self.width || y >= self.height {
            return;
        }

        let coverage = FontRasterizer::get_glyph_coverage(ch);
        let max_w = (x + offset_x + FONT_WIDTH).min(self.width) - (x + offset_x);
        let max_h = (y + FONT_HEIGHT).min(self.height) - y;

        for row in 0..max_h {
            let py = y + row;
            let row_offset = row * FONT_WIDTH;
            let fb_offset = py * self.width + x + offset_x;

            for col in 0..max_w {
                let alpha = coverage[row_offset + col];
                if alpha == 0 {
                    if let Some(bg_color) = bg {
                        self.pixels[fb_offset + col] = bg_color;
                    }
                } else if alpha == 255 {
                    self.pixels[fb_offset + col] = fg;
                } else {
                    let dst = match bg {
                        Some(bg_color) => bg_color,
                        None => self.pixels[fb_offset + col],
                    };
                    let eff_alpha = if self.stem_darkening && !is_box {
                        let k = StemDarkener::compute_darkening_factor(dst, fg, self.font_size, self.stem_darkening_amount);
                        StemDarkener::darken_alpha(alpha, k)
                    } else {
                        alpha
                    };
                    self.pixels[fb_offset + col] = blend_pixel_gamma(dst, fg, eff_alpha);
                }
            }
        }
    }

    /// Draws a character glyph sized to `(target_w x target_h)` pixels.
    pub fn draw_char_sized(&mut self, ch: char, x: usize, y: usize, fg: u32, bg: Option<u32>, target_w: usize, target_h: usize) {
        if x >= self.width || y >= self.height {
            return;
        }

        if self.subpixel_antialiasing {
            self.draw_char_subpixel(ch, x, y, fg, bg, target_w, target_h);
            return;
        }

        let is_box = FontRasterizer::is_box_drawing(ch);
        let ttf_cov;
        let sdf_cov;
        let font_cov;
        let (w, h, coverage): (usize, usize, &[u8]) = if self.use_ttf_font && !is_box {
            let font = crate::gfx::ttf::SystemFontLoader::get_active_font();
            let (cw, ch_h, cov) = crate::gfx::ttf::TtfRasterizer::rasterize_char_cached(&font, ch, target_w, target_h);
            ttf_cov = cov;
            (cw, ch_h, &ttf_cov)
        } else if self.sdf_font_rendering && !is_box {
            sdf_cov = SdfFontRasterizer::rasterize_glyph_sdf(ch, self.font_family, target_w, target_h, self.font_weight, 0.0);
            (sdf_cov.0, sdf_cov.1, &sdf_cov.2)
        } else {
            font_cov = FontRasterizer::rasterize_glyph_sized(ch, self.font_family, target_w, target_h, self.font_weight);
            (font_cov.0, font_cov.1, &font_cov.2)
        };
        let max_w = (x + w).min(self.width) - x;
        let max_h = (y + h).min(self.height) - y;

        for row in 0..max_h {
            let py = y + row;
            let row_offset = row * w;
            let fb_offset = py * self.width + x;

            for col in 0..max_w {
                let alpha = coverage[row_offset + col];
                if alpha == 0 {
                    if let Some(bg_color) = bg {
                        self.pixels[fb_offset + col] = bg_color;
                    }
                } else if alpha == 255 {
                    self.pixels[fb_offset + col] = fg;
                } else {
                    let dst = match bg {
                        Some(bg_color) => bg_color,
                        None => self.pixels[fb_offset + col],
                    };
                    let eff_alpha = if self.stem_darkening && !is_box {
                        let k = StemDarkener::compute_darkening_factor(dst, fg, self.font_size, self.stem_darkening_amount);
                        StemDarkener::darken_alpha(alpha, k)
                    } else {
                        alpha
                    };
                    self.pixels[fb_offset + col] = blend_pixel_gamma(dst, fg, eff_alpha);
                }
            }
        }
    }

    /// Draws a single character glyph with mathematically exact LCD RGB subpixel anti-aliasing and gamma correction.
    pub fn draw_char_subpixel(
        &mut self,
        ch: char,
        x: usize,
        y: usize,
        fg: u32,
        bg: Option<u32>,
        target_w: usize,
        target_h: usize,
    ) {
        if x >= self.width || y >= self.height {
            return;
        }

        let is_box = FontRasterizer::is_box_drawing(ch);
        let ttf_sub;
        let sdf_sub;
        let font_sub;
        let (w, h, subpixel_cov): (usize, usize, &[[u8; 3]]) = if self.use_ttf_font && !is_box {
            let font = crate::gfx::ttf::SystemFontLoader::get_active_font();
            let (cw, ch_h, sub) = crate::gfx::ttf::TtfRasterizer::rasterize_char_subpixel_cached(&font, ch, target_w, target_h);
            ttf_sub = sub;
            (cw, ch_h, &ttf_sub)
        } else if self.sdf_font_rendering && !is_box {
            sdf_sub = SdfFontRasterizer::rasterize_glyph_sdf_subpixel(
                ch,
                self.font_family,
                target_w,
                target_h,
                self.font_weight,
                0.0,
            );
            (sdf_sub.0, sdf_sub.1, &sdf_sub.2)
        } else if target_w == FONT_WIDTH
            && target_h == FONT_HEIGHT
            && self.font_family == FontFamily::Standard
            && self.font_weight == FontWeight::Normal
        {
            (FONT_WIDTH, FONT_HEIGHT, FontRasterizer::get_glyph_subpixel(ch))
        } else {
            font_sub = FontRasterizer::rasterize_glyph_subpixel(
                ch,
                self.font_family,
                target_w,
                target_h,
                self.font_weight,
            );
            (font_sub.0, font_sub.1, &font_sub.2)
        };

        let max_w = (x + w).min(self.width) - x;
        let max_h = (y + h).min(self.height) - y;

        for row in 0..max_h {
            let py = y + row;
            let row_offset = row * w;
            let fb_offset = py * self.width + x;

            for col in 0..max_w {
                let [ar, ag, ab] = subpixel_cov[row_offset + col];
                if ar == 0 && ag == 0 && ab == 0 {
                    if let Some(bg_color) = bg {
                        self.pixels[fb_offset + col] = bg_color;
                    }
                } else if ar == 255 && ag == 255 && ab == 255 {
                    self.pixels[fb_offset + col] = fg;
                } else {
                    let dst = match bg {
                        Some(bg_color) => bg_color,
                        None => self.pixels[fb_offset + col],
                    };
                    let [dr, dg, db] = if self.stem_darkening && !is_box {
                        let k = StemDarkener::compute_darkening_factor(dst, fg, self.font_size, self.stem_darkening_amount);
                        StemDarkener::darken_subpixel(ar, ag, ab, k)
                    } else {
                        [ar, ag, ab]
                    };
                    self.pixels[fb_offset + col] = blend_pixel_subpixel_gamma(dst, fg, dr, dg, db);
                }
            }
        }
    }

    /// Draws a string of text starting at pixel coordinates (x, y) with anti-aliasing and gamma correction.
    pub fn draw_text(&mut self, text: &str, x: usize, y: usize, fg: u32, bg: Option<u32>) {
        let (char_w, char_h) = self.char_cell_size();
        let mut cur_x = x;
        for ch in text.chars() {
            if cur_x + char_w > self.width {
                break;
            }
            if self.subpixel_antialiasing {
                self.draw_char_subpixel(ch, cur_x, y, fg, bg, char_w, char_h);
            } else if self.font_scale > 1 || self.font_size != 12 || self.font_family != FontFamily::Standard || self.font_weight != FontWeight::Normal || self.sdf_font_rendering || self.use_ttf_font {
                self.draw_char_sized(ch, cur_x, y, fg, bg, char_w, char_h);
            } else {
                self.draw_char(ch, cur_x, y, fg, bg);
            }
            cur_x += char_w;
        }
    }

    /// Draws a string of text starting at pixel coordinates (x, y) with hard boundary clipping at `max_x`.
    pub fn draw_text_clipped(&mut self, text: &str, x: usize, y: usize, max_x: usize, fg: u32, bg: Option<u32>) {
        let (char_w, char_h) = self.char_cell_size();
        let mut cur_x = x;
        for ch in text.chars() {
            if cur_x + char_w > max_x || cur_x + char_w > self.width {
                break;
            }
            if self.subpixel_antialiasing {
                self.draw_char_subpixel(ch, cur_x, y, fg, bg, char_w, char_h);
            } else if self.font_scale > 1 || self.font_size != 12 || self.font_family != FontFamily::Standard || self.font_weight != FontWeight::Normal || self.sdf_font_rendering || self.use_ttf_font {
                self.draw_char_sized(ch, cur_x, y, fg, bg, char_w, char_h);
            } else {
                self.draw_char(ch, cur_x, y, fg, bg);
            }
            cur_x += char_w;
        }
    }

    /// Draws a string of text starting at pixel coordinates (x, y) truncated with ellipsis ("...") if it exceeds `max_x`.
    pub fn draw_text_truncated(&mut self, text: &str, x: usize, y: usize, max_x: usize, fg: u32, bg: Option<u32>) {
        if x >= max_x || x >= self.width {
            return;
        }
        let (char_w, _) = self.char_cell_size();
        let avail_w = (max_x.min(self.width)).saturating_sub(x);
        let max_chars = avail_w / char_w.max(1);
        let char_count = text.chars().count();
        if char_count <= max_chars {
            self.draw_text_clipped(text, x, y, max_x, fg, bg);
        } else if max_chars > 3 {
            let mut s: String = text.chars().take(max_chars - 3).collect();
            s.push_str("...");
            self.draw_text_clipped(&s, x, y, max_x, fg, bg);
        } else {
            let s: String = text.chars().take(max_chars).collect();
            self.draw_text_clipped(&s, x, y, max_x, fg, bg);
        }
    }

    /// Draws a single character glyph with integer scaling factor (1 = 1x, 2 = 2x HiDPI)
    /// using smooth continuous bilinear supersampling and gamma correction.
    pub fn draw_char_scaled(&mut self, ch: char, x: usize, y: usize, fg: u32, bg: Option<u32>, scale: usize) {
        let scale = scale.max(1);
        if self.subpixel_antialiasing {
            self.draw_char_subpixel(ch, x, y, fg, bg, FONT_WIDTH * scale, FONT_HEIGHT * scale);
            return;
        }

        if scale <= 1 {
            self.draw_char(ch, x, y, fg, bg);
            return;
        }

        if x >= self.width || y >= self.height {
            return;
        }

        let (w, h, coverage) = FontRasterizer::rasterize_glyph_scaled(ch, scale);
        let max_w = (x + w).min(self.width) - x;
        let max_h = (y + h).min(self.height) - y;

        for row in 0..max_h {
            let py = y + row;
            let row_offset = row * w;
            let fb_offset = py * self.width + x;

            for col in 0..max_w {
                let alpha = coverage[row_offset + col];
                if alpha == 0 {
                    if let Some(bg_color) = bg {
                        self.pixels[fb_offset + col] = bg_color;
                    }
                } else if alpha == 255 {
                    self.pixels[fb_offset + col] = fg;
                } else {
                    let dst = match bg {
                        Some(bg_color) => bg_color,
                        None => self.pixels[fb_offset + col],
                    };
                    self.pixels[fb_offset + col] = blend_pixel_gamma(dst, fg, alpha);
                }
            }
        }
    }

    /// Draws a string of text with scaling factor using smooth anti-aliasing and gamma correction.
    pub fn draw_text_scaled(&mut self, text: &str, x: usize, y: usize, fg: u32, bg: Option<u32>, scale: usize) {
        if scale <= 1 {
            self.draw_text(text, x, y, fg, bg);
            return;
        }
        let char_w = FONT_WIDTH * scale;
        let mut cur_x = x;
        for ch in text.chars() {
            if cur_x + char_w > self.width {
                break;
            }
            self.draw_char_scaled(ch, cur_x, y, fg, bg, scale);
            cur_x += char_w;
        }
    }

    /// Draws a single character glyph using legacy 1-bit binary pixel thresholding without anti-aliasing.
    pub fn draw_char_raw(&mut self, ch: char, x: usize, y: usize, fg: u32, bg: Option<u32>) {
        if x >= self.width || y >= self.height {
            return;
        }

        let glyph = get_glyph(ch);

        for (row_idx, &row_byte) in glyph.iter().enumerate() {
            let py = y + row_idx;
            if py >= self.height {
                break;
            }

            for col_idx in 0..FONT_WIDTH {
                let px = x + col_idx;
                if px >= self.width {
                    break;
                }

                let bit_set = (row_byte & (0x80 >> col_idx)) != 0;
                if bit_set {
                    self.pixels[py * self.width + px] = fg;
                } else if let Some(bg_color) = bg {
                    self.pixels[py * self.width + px] = bg_color;
                }
            }
        }
    }

    /// Draws a string of text without anti-aliasing.
    pub fn draw_text_raw(&mut self, text: &str, x: usize, y: usize, fg: u32, bg: Option<u32>) {
        let mut cur_x = x;
        for ch in text.chars() {
            if cur_x + FONT_WIDTH > self.width {
                break;
            }
            self.draw_char_raw(ch, cur_x, y, fg, bg);
            cur_x += FONT_WIDTH;
        }
    }

    /// Draws text applying full font shaping (programming ligatures), font hinting, and perceptual edge contrast tuning.
    pub fn draw_shaped_text(
        &mut self,
        text: &str,
        x: usize,
        y: usize,
        fg: u32,
        bg: Option<u32>,
        enable_ligatures: bool,
        hinting: HintingLevel,
        edge_enhance: bool,
    ) {
        let (cell_w, cell_h) = self.char_cell_size();
        let glyphs = FontShaper::shape_text_ex(text, self.font_scale, enable_ligatures, self.font_family, self.font_weight, cell_w, cell_h);
        let mut cur_x = x;

        for glyph in glyphs {
            if cur_x >= self.width {
                break;
            }

            let effective_cov = if hinting != HintingLevel::None {
                FontHinter::hint_coverage(&glyph.coverage, glyph.width, glyph.height, hinting)
            } else {
                glyph.coverage.clone()
            };

            let max_w = (cur_x + glyph.width).min(self.width) - cur_x;
            let max_h = (y + glyph.height).min(self.height) - y;

            for row in 0..max_h {
                let py = y + row;
                let row_offset = row * glyph.width;
                let fb_offset = py * self.width + cur_x;

                for col in 0..max_w {
                    let mut alpha = effective_cov[row_offset + col];
                    if alpha == 0 {
                        if let Some(bg_color) = bg {
                            self.pixels[fb_offset + col] = bg_color;
                        }
                    } else if alpha == 255 {
                        self.pixels[fb_offset + col] = fg;
                    } else {
                        let dst = match bg {
                            Some(bg_color) => bg_color,
                            None => self.pixels[fb_offset + col],
                        };
                        if edge_enhance {
                            alpha = EdgePerception::adapt_edge_coverage(alpha, fg, dst);
                        }
                        self.pixels[fb_offset + col] = blend_pixel_gamma(dst, fg, alpha);
                    }
                }
            }

            cur_x += glyph.width;
        }
    }

    /// Draws a wavy squiggle (sinusoidal/zigzag line) commonly used for compiler diagnostics and linting.
    /// Spans from `x_start` to `x_end` along horizontal baseline `y`.
    /// Coordinates are bounds-checked and scaled according to `scale`.
    pub fn draw_wavy_squiggle(&mut self, x_start: usize, x_end: usize, y: usize, color: u32, scale: usize) {
        if x_start >= x_end || x_start >= self.width || y >= self.height {
            return;
        }

        let sc = scale.max(1);
        let end_x = x_end.min(self.width);

        for x in x_start..end_x {
            let phase = (x / sc) % 4;
            let dy: isize = match phase {
                0 => 0,
                1 => -(sc as isize),
                2 => 0,
                _ => sc as isize,
            };

            let py = (y as isize + dy).max(0) as usize;
            for thick in 0..sc {
                let target_y = py + thick;
                if target_y < self.height {
                    self.set_pixel(x, target_y, color);
                }
            }
        }
    }

    /// Analytical geometric anti-aliased line with subpixel coordinates and thickness.
    #[inline]
    pub fn draw_line_aa(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, width: f64, color: u32) {
        AnalyticAA::draw_line(self, x0, y0, x1, y1, width, color);
    }

    /// Analytical anti-aliased circle.
    #[inline]
    pub fn fill_circle_aa(&mut self, cx: f64, cy: f64, radius: f64, color: u32) {
        AnalyticAA::fill_circle(self, cx, cy, radius, color);
    }

    /// Analytical anti-aliased circle outline (ring).
    #[inline]
    pub fn draw_circle_aa(&mut self, cx: f64, cy: f64, radius: f64, stroke_width: f64, color: u32) {
        AnalyticAA::draw_circle_outline(self, cx, cy, radius, stroke_width, color);
    }

    /// Analytical anti-aliased rounded rectangle with exact SDF coverage.
    #[inline]
    pub fn fill_rounded_rect_aa(&mut self, x: f64, y: f64, w: f64, h: f64, radius: f64, color: u32) {
        AnalyticAA::fill_rounded_rect(self, x, y, w, h, radius, color);
    }

    /// Analytical anti-aliased rounded rectangle with transparency.
    #[inline]
    pub fn fill_rounded_rect_alpha_aa(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        radius: f64,
        color: u32,
        alpha: u8,
    ) {
        AnalyticAA::fill_rounded_rect_alpha(self, x, y, w, h, radius, color, alpha);
    }

    /// Analytical anti-aliased rounded rectangle outline.
    #[inline]
    pub fn draw_rounded_outline_aa(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        radius: f64,
        stroke_width: f64,
        color: u32,
    ) {
        AnalyticAA::draw_rounded_outline(self, x, y, w, h, radius, stroke_width, color);
    }

    /// Analytical anti-aliased triangle.
    #[inline]
    pub fn fill_triangle_aa(&mut self, p0: (f64, f64), p1: (f64, f64), p2: (f64, f64), color: u32) {
        AnalyticAA::fill_triangle(self, p0, p1, p2, color);
    }

    /// Fast Gaussian blur on a subregion.
    #[inline]
    pub fn blur_subregion(&mut self, x: usize, y: usize, w: usize, h: usize, radius: usize) {
        BlurEngine::blur_subregion(self, x, y, w, h, radius);
    }

    /// Modern frosted glass (acrylic) backdrop.
    #[inline]
    pub fn frosted_glass(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        radius: usize,
        blur_radius: usize,
        tint_color: u32,
        tint_alpha: u8,
    ) {
        BlurEngine::frosted_glass(self, x, y, w, h, radius, blur_radius, tint_color, tint_alpha);
    }

    /// Physically-based Gaussian smooth shadow.
    pub fn draw_smooth_shadow_box(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        radius: usize,
        blur_radius: usize,
        offset_x: isize,
        offset_y: isize,
        shadow_color: u32,
        shadow_alpha: u8,
    ) {
        BlurEngine::draw_smooth_shadow(self, x, y, w, h, radius, blur_radius, offset_x, offset_y, shadow_color, shadow_alpha);
    }

    /// Unsharp mask filter for perceived edge contrast enhancement.
    #[inline]
    pub fn unsharp_mask_region(&mut self, x: usize, y: usize, w: usize, h: usize, amount: f64) {
        EdgePerception::unsharp_mask_region(self, x, y, w, h, amount);
    }

    /// Returns pixel data as a 32-bit integer slice.
    pub fn as_slice(&self) -> &[u32] {
        &self.pixels
    }

    /// Returns pixel data as a raw byte slice for low-level blitting.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.pixels.as_ptr() as *const u8,
                self.pixels.len() * std::mem::size_of::<u32>(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framebuffer_basics() {
        let mut fb = Framebuffer::new(100, 50);
        assert_eq!(fb.width, 100);
        assert_eq!(fb.height, 50);
        assert_eq!(fb.pixels.len(), 5000);

        fb.clear(0x00FF0000);
        assert_eq!(fb.get_pixel(0, 0), 0x00FF0000);

        fb.fill_rect(10, 10, 20, 20, 0x0000FF00);
        assert_eq!(fb.get_pixel(15, 15), 0x0000FF00);
        assert_eq!(fb.get_pixel(5, 5), 0x00FF0000);
    }

    #[test]
    fn test_framebuffer_text_drawing() {
        let mut fb = Framebuffer::new(80, 24);
        fb.clear(0x00000000);
        fb.draw_text("Hi", 0, 0, 0xFFFFFFFF, None);

        // Check that pixels were painted (both solid and anti-aliased gradations)
        let painted_pixels = fb.pixels.iter().filter(|&&p| p != 0x00000000).count();
        assert!(painted_pixels > 0);
    }

    #[test]
    fn test_framebuffer_antialiased_text_gamma_blend() {
        let mut fb = Framebuffer::new(40, 24);
        // Pure black background
        fb.clear(0xFF000000);
        // Draw white letter 'o' with Some(0xFF000000) background
        fb.draw_char('o', 0, 0, 0xFFFFFFFF, Some(0xFF000000));

        let solid_count = fb.pixels.iter().filter(|&&p| p == 0xFFFFFFFF).count();
        let background_count = fb.pixels.iter().filter(|&&p| p == 0xFF000000).count();
        let antialiased_count = fb.pixels.iter().filter(|&&p| p != 0xFFFFFFFF && p != 0xFF000000).count();

        assert!(solid_count > 0, "Must have solid core pixels");
        assert!(background_count > 0, "Must have background pixels");
        assert!(antialiased_count > 0, "Must have intermediate gamma-blended edge pixels");

        // Verify that intermediate pixels are clean monochrome grays (R == G == B)
        for &p in fb.pixels.iter() {
            let r = (p >> 16) & 0xFF;
            let g = (p >> 8) & 0xFF;
            let b = p & 0xFF;
            assert_eq!(r, g, "Gray channels must match");
            assert_eq!(g, b, "Gray channels must match");
        }
    }

    #[test]
    fn test_framebuffer_draw_text_scaled_bilinear() {
        let mut fb = Framebuffer::new(80, 48);
        fb.clear(0xFF000000);
        fb.draw_text_scaled("OK", 0, 0, 0xFFFFFFFF, Some(0xFF000000), 2);

        // Scaled text width is 2 chars * (FONT_WIDTH * 2) = 32 px
        // Scaled text height is FONT_HEIGHT * 2 = 32 px
        let mut max_x = 0;
        let mut max_y = 0;
        for y in 0..48 {
            for x in 0..80 {
                if fb.get_pixel(x, y) != 0xFF000000 {
                    if x > max_x { max_x = x; }
                    if y > max_y { max_y = y; }
                }
            }
        }

        assert!(max_x >= 20 && max_x < 32, "Max x should be within 2x bounds (got {})", max_x);
        assert!(max_y >= 16 && max_y < 32, "Max y should be within 2x bounds (got {})", max_y);
    }

    #[test]
    fn test_framebuffer_subpixel_lcd_rendering() {
        let mut fb = Framebuffer::new(40, 24);
        fb.set_subpixel_antialiasing(true);
        fb.clear(0xFF000000);
        fb.draw_char('o', 0, 0, 0xFFFFFFFF, Some(0xFF000000));

        // Subpixel rendering will produce color-differentiated edge pixels where r != g or g != b
        let has_subpixel_diff = fb.pixels.iter().any(|&p| {
            let r = (p >> 16) & 0xFF;
            let g = (p >> 8) & 0xFF;
            let b = p & 0xFF;
            (r != g || g != b) && (r > 0 || g > 0 || b > 0)
        });
        assert!(has_subpixel_diff, "Subpixel rendering must produce color-differentiated LCD edge pixels");

        // Verify solid core pixels are present
        let solid_count = fb.pixels.iter().filter(|&&p| p == 0xFFFFFFFF).count();
        assert!(solid_count > 0, "Solid core pixels must remain pure 0xFFFFFFFF");
    }
}
