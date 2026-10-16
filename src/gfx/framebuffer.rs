//! Software framebuffer with 2D rendering primitives and font rasterization.
//! Zero third-party dependencies, purely standard library Rust.

use super::font::{get_glyph, FontRasterizer, FONT_HEIGHT, FONT_WIDTH};
use super::gamma::blend_pixel_gamma;

/// Software pixel buffer in 32-bit ARGB (0xAARRGGBB) format.
#[derive(Debug, Clone)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
    pub font_scale: usize,
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

    /// Draws a single character glyph at pixel coordinates (x, y) with subpixel anti-aliasing and gamma correction.
    pub fn draw_char(&mut self, ch: char, x: usize, y: usize, fg: u32, bg: Option<u32>) {
        if self.font_scale > 1 {
            self.draw_char_scaled(ch, x, y, fg, bg, self.font_scale);
            return;
        }

        if x >= self.width || y >= self.height {
            return;
        }

        let coverage = FontRasterizer::get_glyph_coverage(ch);
        let max_w = (x + FONT_WIDTH).min(self.width) - x;
        let max_h = (y + FONT_HEIGHT).min(self.height) - y;

        for row in 0..max_h {
            let py = y + row;
            let row_offset = row * FONT_WIDTH;
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

    /// Draws a string of text starting at pixel coordinates (x, y) with anti-aliasing and gamma correction.
    pub fn draw_text(&mut self, text: &str, x: usize, y: usize, fg: u32, bg: Option<u32>) {
        if self.font_scale > 1 {
            self.draw_text_scaled(text, x, y, fg, bg, self.font_scale);
            return;
        }

        let mut cur_x = x;
        for ch in text.chars() {
            if cur_x + FONT_WIDTH > self.width {
                break;
            }
            self.draw_char(ch, cur_x, y, fg, bg);
            cur_x += FONT_WIDTH;
        }
    }

    /// Draws a single character glyph with integer scaling factor (1 = 1x, 2 = 2x HiDPI)
    /// using smooth continuous bilinear supersampling and gamma correction.
    pub fn draw_char_scaled(&mut self, ch: char, x: usize, y: usize, fg: u32, bg: Option<u32>, scale: usize) {
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
}
