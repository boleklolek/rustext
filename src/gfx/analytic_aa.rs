//! Analytical Geometric Anti-Aliasing (Analytic AA).
//! Evaluates exact subpixel geometric area coverage and signed distance fields (SDF)
//! for 2D primitives without expensive supersampling.
//! 100% pure standard-library Rust with zero third-party dependencies.

use super::framebuffer::Framebuffer;

/// Analytical geometric rasterization engine with subpixel coverage.
pub struct AnalyticAA;

impl AnalyticAA {
    /// Draws an anti-aliased line with subpixel coordinates and stroke width.
    pub fn draw_line(
        fb: &mut Framebuffer,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        width: f64,
        color: u32,
    ) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len_sq = dx * dx + dy * dy;

        if len_sq < 0.0001 {
            Self::fill_circle(fb, x0, y0, width * 0.5, color);
            return;
        }

        let radius = (width * 0.5).max(0.5);
        let min_x = (x0.min(x1) - radius - 1.0).max(0.0).floor() as usize;
        let max_x = (x0.max(x1) + radius + 1.0).min(fb.width as f64 - 1.0).ceil() as usize;
        let min_y = (y0.min(y1) - radius - 1.0).max(0.0).floor() as usize;
        let max_y = (y0.max(y1) + radius + 1.0).min(fb.height as f64 - 1.0).ceil() as usize;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let cx = px as f64 + 0.5;
                let cy = py as f64 + 0.5;

                // Project point onto line segment
                let t = (((cx - x0) * dx + (cy - y0) * dy) / len_sq).clamp(0.0, 1.0);
                let proj_x = x0 + t * dx;
                let proj_y = y0 + t * dy;
                let dist = (cx - proj_x).hypot(cy - proj_y);

                let cov = (0.5 + radius - dist).clamp(0.0, 1.0);
                if cov > 0.003 {
                    let alpha = (cov * 255.0).round() as u8;
                    fb.blend_pixel_gamma(px, py, color, alpha);
                }
            }
        }
    }

    /// Fills an anti-aliased circle with analytical distance coverage.
    pub fn fill_circle(fb: &mut Framebuffer, cx: f64, cy: f64, radius: f64, color: u32) {
        if radius <= 0.0 {
            return;
        }

        let min_x = (cx - radius - 1.0).max(0.0).floor() as usize;
        let max_x = (cx + radius + 1.0).min(fb.width as f64 - 1.0).ceil() as usize;
        let min_y = (cy - radius - 1.0).max(0.0).floor() as usize;
        let max_y = (cy + radius + 1.0).min(fb.height as f64 - 1.0).ceil() as usize;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let px_c = px as f64 + 0.5;
                let py_c = py as f64 + 0.5;
                let dist = (px_c - cx).hypot(py_c - cy);
                let cov = (0.5 + radius - dist).clamp(0.0, 1.0);

                if cov > 0.003 {
                    let alpha = (cov * 255.0).round() as u8;
                    fb.blend_pixel_gamma(px, py, color, alpha);
                }
            }
        }
    }

    /// Draws an anti-aliased circle outline (ring) with analytical distance coverage.
    pub fn draw_circle_outline(
        fb: &mut Framebuffer,
        cx: f64,
        cy: f64,
        radius: f64,
        stroke_width: f64,
        color: u32,
    ) {
        if radius <= 0.0 || stroke_width <= 0.0 {
            return;
        }

        let half_w = stroke_width * 0.5;
        let outer_r = radius + half_w;
        let min_x = (cx - outer_r - 1.0).max(0.0).floor() as usize;
        let max_x = (cx + outer_r + 1.0).min(fb.width as f64 - 1.0).ceil() as usize;
        let min_y = (cy - outer_r - 1.0).max(0.0).floor() as usize;
        let max_y = (cy + outer_r + 1.0).min(fb.height as f64 - 1.0).ceil() as usize;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let px_c = px as f64 + 0.5;
                let py_c = py as f64 + 0.5;
                let dist = (px_c - cx).hypot(py_c - cy);
                let cov = (0.5 + half_w - (dist - radius).abs()).clamp(0.0, 1.0);

                if cov > 0.003 {
                    let alpha = (cov * 255.0).round() as u8;
                    fb.blend_pixel_gamma(px, py, color, alpha);
                }
            }
        }
    }

    /// Fills an analytically anti-aliased rounded rectangle using exact 2D Box Signed Distance Field.
    pub fn fill_rounded_rect(
        fb: &mut Framebuffer,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        radius: f64,
        color: u32,
    ) {
        Self::fill_rounded_rect_alpha(fb, x, y, w, h, radius, color, 255);
    }

    /// Fills an analytically anti-aliased rounded rectangle with transparency and exact SDF coverage.
    pub fn fill_rounded_rect_alpha(
        fb: &mut Framebuffer,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        radius: f64,
        color: u32,
        base_alpha: u8,
    ) {
        if w <= 0.0 || h <= 0.0 || base_alpha == 0 {
            return;
        }

        let r = radius.min(w * 0.5).min(h * 0.5).max(0.0);
        let center_x = x + w * 0.5;
        let center_y = y + h * 0.5;
        let half_w = w * 0.5;
        let half_h = h * 0.5;

        let min_x = (x - 1.0).max(0.0).floor() as usize;
        let max_x = (x + w + 1.0).min(fb.width as f64).ceil() as usize;
        let min_y = (y - 1.0).max(0.0).floor() as usize;
        let max_y = (y + h + 1.0).min(fb.height as f64).ceil() as usize;

        let base_alpha_f = base_alpha as f64 / 255.0;

        for py in min_y..max_y {
            let p_y = py as f64 + 0.5;
            for px in min_x..max_x {
                let p_x = px as f64 + 0.5;

                // Exact 2D Rounded Box SDF
                let q_x = (p_x - center_x).abs() - (half_w - r);
                let q_y = (p_y - center_y).abs() - (half_h - r);

                let outside_x = q_x.max(0.0);
                let outside_y = q_y.max(0.0);
                let inside = q_x.max(q_y).min(0.0);

                let dist = outside_x.hypot(outside_y) + inside - r;
                let cov = (0.5 - dist).clamp(0.0, 1.0);

                if cov > 0.003 {
                    let alpha = ((cov * base_alpha_f) * 255.0).round() as u8;
                    fb.blend_pixel_gamma(px, py, color, alpha);
                }
            }
        }
    }

    /// Draws an analytically anti-aliased rounded rectangle outline using exact SDF.
    pub fn draw_rounded_outline(
        fb: &mut Framebuffer,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        radius: f64,
        stroke_width: f64,
        color: u32,
    ) {
        if w <= 0.0 || h <= 0.0 || stroke_width <= 0.0 {
            return;
        }

        let half_stroke = stroke_width * 0.5;
        let r = radius.min(w * 0.5).min(h * 0.5).max(0.0);
        let center_x = x + w * 0.5;
        let center_y = y + h * 0.5;
        let half_w = w * 0.5;
        let half_h = h * 0.5;

        let margin = stroke_width + 1.0;
        let min_x = (x - margin).max(0.0).floor() as usize;
        let max_x = (x + w + margin).min(fb.width as f64).ceil() as usize;
        let min_y = (y - margin).max(0.0).floor() as usize;
        let max_y = (y + h + margin).min(fb.height as f64).ceil() as usize;

        for py in min_y..max_y {
            let p_y = py as f64 + 0.5;
            for px in min_x..max_x {
                let p_x = px as f64 + 0.5;

                let q_x = (p_x - center_x).abs() - (half_w - r);
                let q_y = (p_y - center_y).abs() - (half_h - r);

                let outside_x = q_x.max(0.0);
                let outside_y = q_y.max(0.0);
                let inside = q_x.max(q_y).min(0.0);

                let dist = outside_x.hypot(outside_y) + inside - r;
                let cov = (0.5 + half_stroke - dist.abs()).clamp(0.0, 1.0);

                if cov > 0.003 {
                    let alpha = (cov * 255.0).round() as u8;
                    fb.blend_pixel_gamma(px, py, color, alpha);
                }
            }
        }
    }

    /// Fills an anti-aliased triangle with analytical edge distance equations.
    pub fn fill_triangle(
        fb: &mut Framebuffer,
        p0: (f64, f64),
        p1: (f64, f64),
        p2: (f64, f64),
        color: u32,
    ) {
        let min_x = p0.0.min(p1.0).min(p2.0).floor().max(0.0) as usize;
        let max_x = p0.0.max(p1.0).max(p2.0).ceil().min(fb.width as f64 - 1.0) as usize;
        let min_y = p0.1.min(p1.1).min(p2.1).floor().max(0.0) as usize;
        let max_y = p0.1.max(p1.1).max(p2.1).ceil().min(fb.height as f64 - 1.0) as usize;

        // Ensure clockwise / positive orientation
        let area = (p1.0 - p0.0) * (p2.1 - p0.1) - (p2.0 - p0.0) * (p1.1 - p0.1);
        let (a, b, c) = if area < 0.0 { (p0, p2, p1) } else { (p0, p1, p2) };

        for py in min_y..=max_y {
            let cy = py as f64 + 0.5;
            for px in min_x..=max_x {
                let cx = px as f64 + 0.5;

                // Signed distances to the 3 edges
                let d0 = Self::edge_distance(a, b, (cx, cy));
                let d1 = Self::edge_distance(b, c, (cx, cy));
                let d2 = Self::edge_distance(c, a, (cx, cy));

                // Inside if all >= 0
                let min_d = d0.min(d1).min(d2);
                let cov = (0.5 + min_d).clamp(0.0, 1.0);

                if cov > 0.003 {
                    let alpha = (cov * 255.0).round() as u8;
                    fb.blend_pixel_gamma(px, py, color, alpha);
                }
            }
        }
    }

    #[inline]
    fn edge_distance(v0: (f64, f64), v1: (f64, f64), p: (f64, f64)) -> f64 {
        let dx = v1.0 - v0.0;
        let dy = v1.1 - v0.1;
        let len = dx.hypot(dy);
        if len < 0.0001 {
            return 0.0;
        }
        // Signed perpendicular distance (positive to the right of v0->v1)
        (dx * (p.1 - v0.1) - dy * (p.0 - v0.0)) / len
    }
}
