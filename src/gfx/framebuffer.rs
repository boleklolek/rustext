//! Software framebuffer with 2D rendering primitives and font rasterization.
//! Zero third-party dependencies, purely standard library Rust.

use super::font::{get_glyph, FONT_WIDTH};

/// Software pixel buffer in 32-bit ARGB (0xAARRGGBB) format.
#[derive(Debug, Clone)]
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

impl Framebuffer {
    /// Creates a new framebuffer with given pixel dimensions.
    pub fn new(width: usize, height: usize) -> Self {
        let size = width * height;
        Self {
            width,
            height,
            pixels: vec![0xFF000000; size],
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

    /// Draws a single character glyph at pixel coordinates (x, y).
    pub fn draw_char(&mut self, ch: char, x: usize, y: usize, fg: u32, bg: Option<u32>) {
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

    /// Draws a string of text starting at pixel coordinates (x, y).
    pub fn draw_text(&mut self, text: &str, x: usize, y: usize, fg: u32, bg: Option<u32>) {
        let mut cur_x = x;
        for ch in text.chars() {
            if cur_x + FONT_WIDTH > self.width {
                break;
            }
            self.draw_char(ch, cur_x, y, fg, bg);
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
        fb.draw_text("Hi", 0, 0, 0x00FFFFFF, None);

        // Check that some pixels were painted
        let non_zero_pixels = fb.pixels.iter().filter(|&&p| p == 0x00FFFFFF).count();
        assert!(non_zero_pixels > 0);
    }
}
