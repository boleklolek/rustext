//! Analytical scanline area TrueType vector rasterizer.
//! Implements high-precision fractional trapezoidal winding accumulation (FreeType/stb_truetype algorithm)
//! with support for grayscale anti-aliasing and ClearType LCD subpixel RGB sampling.
//! Zero third-party dependencies, purely standard library Rust.

use super::outline::Point2D;
use super::parser::TrueTypeFont;

#[derive(Clone, Copy, Default)]
struct CellAccumulator {
    area: f32,
    delta_winding: f32,
}

/// High-performance TrueType vector glyph rasterizer.
pub struct TtfRasterizer;

impl TtfRasterizer {
    /// Rasterizes an outline composed of flattened closed polygon contours
    /// into an 8-bit anti-aliased coverage mask (0..=255 per pixel).
    pub fn rasterize_contours(
        contours: &[Vec<Point2D>],
        width: usize,
        height: usize,
    ) -> Vec<u8> {
        if width == 0 || height == 0 || contours.is_empty() {
            return vec![0u8; width * height];
        }

        let mut cells = vec![CellAccumulator::default(); width * height];

        for contour in contours {
            let n = contour.len();
            if n < 2 {
                continue;
            }

            for i in 0..n {
                let p1 = contour[i];
                let p2 = contour[(i + 1) % n];
                Self::rasterize_segment(&mut cells, width, height, p1, p2);
            }
        }

        let mut coverage = vec![0u8; width * height];
        for row in 0..height {
            let mut running_winding = 0.0f32;
            let row_offset = row * width;
            for col in 0..width {
                let idx = row_offset + col;
                let total_area = running_winding + cells[idx].area;
                running_winding += cells[idx].delta_winding;

                let alpha = total_area.abs().min(1.0);
                coverage[idx] = (alpha * 255.0).round() as u8;
            }
        }

        coverage
    }

    /// Clips a directed line segment to horizontal scanline rows and accumulates
    /// trapezoidal area and signed winding in pixel cells.
    fn rasterize_segment(
        cells: &mut [CellAccumulator],
        width: usize,
        height: usize,
        p1: Point2D,
        p2: Point2D,
    ) {
        let (x1, y1) = (p1.x, p1.y);
        let (x2, y2) = (p2.x, p2.y);

        let dy_total = y2 - y1;
        if dy_total.abs() < 1e-6 {
            return; // Horizontal lines have zero vertical span and contribute no area
        }

        let y_min = y1.min(y2);
        let y_max = y1.max(y2);

        if y_max <= 0.0 || y_min >= height as f32 {
            return;
        }

        let row_start = (y_min.floor() as isize).max(0) as usize;
        let row_end = (y_max.ceil() as isize).min(height as isize) as usize;

        for row in row_start..row_end {
            let row_top = row as f32;
            let row_bot = (row + 1) as f32;

            let t_top = (row_top - y1) / dy_total;
            let t_bot = (row_bot - y1) / dy_total;

            let t_min = t_top.min(t_bot).clamp(0.0, 1.0);
            let t_max = t_top.max(t_bot).clamp(0.0, 1.0);

            if (t_max - t_min).abs() < 1e-6 {
                continue;
            }

            let t_in = t_min;
            let t_out = t_max;

            let x_in = x1 + t_in * (x2 - x1);
            let y_in = y1 + t_in * dy_total;
            let x_out = x1 + t_out * (x2 - x1);
            let y_out = y1 + t_out * dy_total;

            let col_min_f = x_in.min(x_out);
            let col_max_f = x_in.max(x_out);

            let col_start = (col_min_f.floor() as isize).max(0) as usize;
            let col_end = (col_max_f.ceil() as isize).min(width as isize) as usize;

            if col_start >= width {
                continue;
            }

            if col_start == col_end || (col_max_f - col_min_f).abs() < 1e-6 {
                let col = col_start.min(width - 1);
                let dy = y_out - y_in;
                let x_mid = ((x_in + x_out) * 0.5) - col as f32;
                let area = dy * (1.0 - x_mid.clamp(0.0, 1.0));

                let idx = row * width + col;
                cells[idx].area += area;
                cells[idx].delta_winding += dy;
            } else {
                let dx_sub = x_out - x_in;
                let mut prev_t = 0.0f32;
                let mut prev_x = x_in;
                let mut prev_y = y_in;

                let step_dir = if dx_sub > 0.0 { 1isize } else { -1isize };
                let start_bound = if dx_sub > 0.0 {
                    (col_start + 1) as isize
                } else {
                    col_end as isize
                };

                let mut cur_x_bound = start_bound;
                while (dx_sub > 0.0 && (cur_x_bound as usize) <= col_end)
                    || (dx_sub < 0.0 && cur_x_bound > col_start as isize)
                {
                    let xb = cur_x_bound as f32;
                    let t_b = ((xb - x_in) / dx_sub).clamp(0.0, 1.0);

                    if t_b > prev_t + 1e-6 {
                        let curr_x = x_in + t_b * dx_sub;
                        let curr_y = y_in + t_b * (y_out - y_in);

                        let col_idx = ((prev_x + curr_x) * 0.5).floor() as isize;
                        if col_idx >= 0 && (col_idx as usize) < width {
                            let c = col_idx as usize;
                            let dy = curr_y - prev_y;
                            let x_mid = ((prev_x + curr_x) * 0.5) - c as f32;
                            let area = dy * (1.0 - x_mid.clamp(0.0, 1.0));
                            let idx = row * width + c;
                            cells[idx].area += area;
                            cells[idx].delta_winding += dy;
                        } else if col_idx < 0 {
                            let dy = curr_y - prev_y;
                            cells[row * width].delta_winding += dy;
                        }

                        prev_t = t_b;
                        prev_x = curr_x;
                        prev_y = curr_y;
                    }

                    cur_x_bound += step_dir;
                }

                if (1.0 - prev_t) > 1e-6 {
                    let curr_x = x_out;
                    let curr_y = y_out;
                    let col_idx = ((prev_x + curr_x) * 0.5).floor() as isize;
                    if col_idx >= 0 && (col_idx as usize) < width {
                        let c = col_idx as usize;
                        let dy = curr_y - prev_y;
                        let x_mid = ((prev_x + curr_x) * 0.5) - c as f32;
                        let area = dy * (1.0 - x_mid.clamp(0.0, 1.0));
                        let idx = row * width + c;
                        cells[idx].area += area;
                        cells[idx].delta_winding += dy;
                    } else if col_idx < 0 {
                        let dy = curr_y - prev_y;
                        cells[row * width].delta_winding += dy;
                    }
                }
            }
        }
    }

    /// Rasterizes a TrueType glyph to exact target dimensions `(target_w x target_h)`
    /// using smooth continuous analytical anti-aliasing.
    pub fn rasterize_glyph(
        font: &TrueTypeFont,
        glyph_id: u16,
        target_w: usize,
        target_h: usize,
    ) -> (usize, usize, Vec<u8>) {
        let outline = match font.get_glyph_outline(glyph_id) {
            Ok(o) => o,
            Err(_) => return (target_w, target_h, vec![0u8; target_w * target_h]),
        };

        if outline.is_empty() {
            return (target_w, target_h, vec![0u8; target_w * target_h]);
        }

        let em = font.units_per_em.max(1) as f32;
        let font_height = ((font.ascender - font.descender) as f32).max(em);
        let scale_y = (target_h as f32 * 0.88) / font_height;

        let metrics = font.get_glyph_metrics(glyph_id);
        let adv = if metrics.advance_width > 0 {
            metrics.advance_width as f32
        } else {
            em * 0.6
        };
        let max_scale_x = (target_w as f32 * 0.92) / adv;
        let scale_x = scale_y.min(max_scale_x);

        let baseline_y = (target_h as f32 * 0.78).round();

        let advance_px = adv * scale_x;
        let offset_x = ((target_w as f32 - advance_px) * 0.5).max(0.0);

        let flattened = outline.scale_and_flatten(scale_x, -scale_y, offset_x, baseline_y, 0.25);
        let coverage = Self::rasterize_contours(&flattened, target_w, target_h);

        (target_w, target_h, coverage)
    }

    /// Rasterizes a TrueType glyph with spatial LCD ClearType subpixel anti-aliasing (RGB).
    /// Performs 3x horizontal oversampling aligned to physical liquid crystal subpixels
    /// filtered with a 5-tap FreeType LCD low-pass FIR filter to eliminate chromatic fringing.
    pub fn rasterize_glyph_subpixel(
        font: &TrueTypeFont,
        glyph_id: u16,
        target_w: usize,
        target_h: usize,
    ) -> (usize, usize, Vec<[u8; 3]>) {
        let outline = match font.get_glyph_outline(glyph_id) {
            Ok(o) => o,
            Err(_) => return (target_w, target_h, vec![[0u8; 3]; target_w * target_h]),
        };

        if outline.is_empty() {
            return (target_w, target_h, vec![[0u8; 3]; target_w * target_h]);
        }

        let em = font.units_per_em.max(1) as f32;
        let font_height = ((font.ascender - font.descender) as f32).max(em);
        let scale_y = (target_h as f32 * 0.88) / font_height;

        let metrics = font.get_glyph_metrics(glyph_id);
        let adv = if metrics.advance_width > 0 {
            metrics.advance_width as f32
        } else {
            em * 0.6
        };
        let max_scale_x = (target_w as f32 * 0.92) / adv;
        let scale_x = scale_y.min(max_scale_x);
        let scale_x_3x = scale_x * 3.0;

        let baseline_y = (target_h as f32 * 0.78).round();

        let width_3x = target_w * 3;
        let advance_px_3x = adv * scale_x_3x;
        let offset_x = ((width_3x as f32 - advance_px_3x) * 0.5).max(0.0);

        let flattened = outline.scale_and_flatten(scale_x_3x, -scale_y, offset_x, baseline_y, 0.25);
        let cov_3x = Self::rasterize_contours(&flattened, width_3x, target_h);

        // 5-tap FreeType LCD low-pass FIR filter: [1, 4, 6, 4, 1] / 16
        // Eliminates chromatic aberration / color fringes while maintaining subpixel acuity.
        let mut filtered_3x = vec![0u8; width_3x * target_h];
        for y in 0..target_h {
            let row = y * width_3x;
            for x in 0..width_3x {
                let get = |ix: isize| -> u32 {
                    if ix < 0 {
                        cov_3x[row] as u32
                    } else if (ix as usize) >= width_3x {
                        cov_3x[row + width_3x - 1] as u32
                    } else {
                        cov_3x[row + ix as usize] as u32
                    }
                };
                let xi = x as isize;
                let c = (get(xi - 2) * 1 + get(xi - 1) * 4 + get(xi) * 6 + get(xi + 1) * 4 + get(xi + 2) * 1 + 8) >> 4;
                filtered_3x[row + x] = c.min(255) as u8;
            }
        }

        let mut subpixel_out = vec![[0u8; 3]; target_w * target_h];
        for y in 0..target_h {
            let row_in = y * width_3x;
            let row_out = y * target_w;
            for x in 0..target_w {
                let r = filtered_3x[row_in + x * 3];
                let g = filtered_3x[row_in + x * 3 + 1];
                let b = filtered_3x[row_in + x * 3 + 2];
                subpixel_out[row_out + x] = [r, g, b];
            }
        }

        (target_w, target_h, subpixel_out)
    }

    /// Rasterizes a Unicode character using TrueType vector contours.
    pub fn rasterize_char(
        font: &TrueTypeFont,
        ch: char,
        target_w: usize,
        target_h: usize,
    ) -> (usize, usize, Vec<u8>) {
        if let Some(gid) = font.char_to_glyph_id(ch) {
            Self::rasterize_glyph(font, gid, target_w, target_h)
        } else {
            (target_w, target_h, vec![0u8; target_w * target_h])
        }
    }

    /// Rasterizes a Unicode character with LCD ClearType subpixel rendering.
    pub fn rasterize_char_subpixel(
        font: &TrueTypeFont,
        ch: char,
        target_w: usize,
        target_h: usize,
    ) -> (usize, usize, Vec<[u8; 3]>) {
        if let Some(gid) = font.char_to_glyph_id(ch) {
            Self::rasterize_glyph_subpixel(font, gid, target_w, target_h)
        } else {
            (target_w, target_h, vec![[0u8; 3]; target_w * target_h])
        }
    }

    /// Rasterizes a Unicode character with automatic global caching.
    pub fn rasterize_char_cached(
        font: &TrueTypeFont,
        ch: char,
        target_w: usize,
        target_h: usize,
    ) -> (usize, usize, std::sync::Arc<[u8]>) {
        let cov = super::cache::GlobalTtfCache::get_grayscale(font, ch, target_w, target_h);
        (target_w, target_h, cov)
    }

    /// Rasterizes a Unicode character with LCD ClearType subpixel rendering and automatic caching.
    pub fn rasterize_char_subpixel_cached(
        font: &TrueTypeFont,
        ch: char,
        target_w: usize,
        target_h: usize,
    ) -> (usize, usize, std::sync::Arc<[[u8; 3]]>) {
        let sub = super::cache::GlobalTtfCache::get_subpixel(font, ch, target_w, target_h);
        (target_w, target_h, sub)
    }

    /// Pre-populates the cache with printable ASCII characters for the given target size.
    pub fn warmup_cache(font: &TrueTypeFont, target_w: usize, target_h: usize) {
        super::cache::GlobalTtfCache::warmup(font, target_w, target_h);
    }

    /// Clears the global typography cache.
    pub fn clear_cache() {
        super::cache::GlobalTtfCache::clear();
    }

    /// Retrieves current typography cache utilization and performance statistics.
    pub fn cache_stats() -> super::cache::CacheStats {
        super::cache::GlobalTtfCache::stats()
    }
}
