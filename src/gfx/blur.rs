//! Fast Separable Blur, Frosted Glass, and Physically-Based Soft Shadows.
//! Implements O(1) sliding-window box blurs converging to true 2D Gaussian blur via Central Limit Theorem.
//! 100% pure standard-library Rust with zero third-party dependencies.

use super::framebuffer::Framebuffer;

/// Fast image blur, frosted glass effect, and Gaussian soft drop shadow generator.
pub struct BlurEngine;

impl BlurEngine {
    /// Applies a 3-pass fast separable Gaussian blur approximation to an ARGB pixel buffer in O(N) time.
    pub fn gaussian_blur_argb(pixels: &mut [u32], width: usize, height: usize, radius: usize) {
        if radius == 0 || width < 2 || height < 2 {
            return;
        }

        let r = radius.min(width / 2).min(height / 2).max(1);
        let mut temp = vec![0u32; width * height];

        // 3 successive passes of horizontal + vertical box blur mathematically converge to Gaussian
        for _ in 0..3 {
            Self::box_blur_h(pixels, &mut temp, width, height, r);
            Self::box_blur_v(&temp, pixels, width, height, r);
        }
    }

    /// Horizontal 1D box blur with O(1) moving sliding window sum.
    fn box_blur_h(src: &[u32], dst: &mut [u32], width: usize, height: usize, r: usize) {
        let div = (2 * r + 1) as u32;

        for y in 0..height {
            let row_offset = y * width;

            let first_pixel = src[row_offset];
            let last_pixel = src[row_offset + width - 1];

            let (fa, fr, fg, fb) = Self::unpack_argb(first_pixel);
            let (la, lr, lg, lb) = Self::unpack_argb(last_pixel);

            let mut sum_a = fa * (r as u32 + 1);
            let mut sum_r = fr * (r as u32 + 1);
            let mut sum_g = fg * (r as u32 + 1);
            let mut sum_b = fb * (r as u32 + 1);

            for i in 0..r {
                let p = src[row_offset + i.min(width - 1)];
                let (a, r, g, b) = Self::unpack_argb(p);
                sum_a += a;
                sum_r += r;
                sum_g += g;
                sum_b += b;
            }

            for x in 0..width {
                dst[row_offset + x] = Self::pack_argb(sum_a / div, sum_r / div, sum_g / div, sum_b / div);

                let add_x = (x + r + 1).min(width - 1);
                let (aa, ar, ag, ab) = if x + r + 1 < width {
                    Self::unpack_argb(src[row_offset + add_x])
                } else {
                    (la, lr, lg, lb)
                };

                let (sa, sr, sg, sb) = if x >= r {
                    Self::unpack_argb(src[row_offset + (x - r)])
                } else {
                    (fa, fr, fg, fb)
                };

                sum_a = sum_a.saturating_add(aa).saturating_sub(sa);
                sum_r = sum_r.saturating_add(ar).saturating_sub(sr);
                sum_g = sum_g.saturating_add(ag).saturating_sub(sg);
                sum_b = sum_b.saturating_add(ab).saturating_sub(sb);
            }
        }
    }

    /// Vertical 1D box blur with O(1) moving sliding window sum.
    fn box_blur_v(src: &[u32], dst: &mut [u32], width: usize, height: usize, r: usize) {
        let div = (2 * r + 1) as u32;

        for x in 0..width {
            let first_pixel = src[x];
            let last_pixel = src[(height - 1) * width + x];

            let (fa, fr, fg, fb) = Self::unpack_argb(first_pixel);
            let (la, lr, lg, lb) = Self::unpack_argb(last_pixel);

            let mut sum_a = fa * (r as u32 + 1);
            let mut sum_r = fr * (r as u32 + 1);
            let mut sum_g = fg * (r as u32 + 1);
            let mut sum_b = fb * (r as u32 + 1);

            for i in 0..r {
                let p = src[i.min(height - 1) * width + x];
                let (a, r, g, b) = Self::unpack_argb(p);
                sum_a += a;
                sum_r += r;
                sum_g += g;
                sum_b += b;
            }

            for y in 0..height {
                dst[y * width + x] = Self::pack_argb(sum_a / div, sum_r / div, sum_g / div, sum_b / div);

                let add_y = (y + r + 1).min(height - 1);
                let (aa, ar, ag, ab) = if y + r + 1 < height {
                    Self::unpack_argb(src[add_y * width + x])
                } else {
                    (la, lr, lg, lb)
                };

                let (sa, sr, sg, sb) = if y >= r {
                    Self::unpack_argb(src[(y - r) * width + x])
                } else {
                    (fa, fr, fg, fb)
                };

                sum_a = sum_a.saturating_add(aa).saturating_sub(sa);
                sum_r = sum_r.saturating_add(ar).saturating_sub(sr);
                sum_g = sum_g.saturating_add(ag).saturating_sub(sg);
                sum_b = sum_b.saturating_add(ab).saturating_sub(sb);
            }
        }
    }

    /// Blurs an isolated rectangular subregion of the framebuffer in-place.
    pub fn blur_subregion(fb: &mut Framebuffer, x: usize, y: usize, w: usize, h: usize, radius: usize) {
        if radius == 0 || w < 2 || h < 2 || x >= fb.width || y >= fb.height {
            return;
        }

        let rw = w.min(fb.width - x);
        let rh = h.min(fb.height - y);
        let mut patch = Vec::with_capacity(rw * rh);

        for py in 0..rh {
            let row_start = (y + py) * fb.width + x;
            patch.extend_from_slice(&fb.pixels[row_start..row_start + rw]);
        }

        Self::gaussian_blur_argb(&mut patch, rw, rh, radius);

        for py in 0..rh {
            let row_start = (y + py) * fb.width + x;
            let patch_start = py * rw;
            fb.pixels[row_start..row_start + rw]
                .copy_from_slice(&patch[patch_start..patch_start + rw]);
        }
    }

    /// Renders a modern frosted glass (acrylic/vibrancy) backdrop over a rectangular area.
    pub fn frosted_glass(
        fb: &mut Framebuffer,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        radius: usize,
        blur_radius: usize,
        tint_color: u32,
        tint_alpha: u8,
    ) {
        if w < 2 || h < 2 || x >= fb.width || y >= fb.height {
            return;
        }

        // 1. Blur backdrop region
        Self::blur_subregion(fb, x, y, w, h, blur_radius);

        // 2. Blend translucent acrylic tint on top with analytical rounded corners
        super::analytic_aa::AnalyticAA::fill_rounded_rect_alpha(
            fb,
            x as f64,
            y as f64,
            w as f64,
            h as f64,
            radius as f64,
            tint_color,
            tint_alpha,
        );

        // 3. Subtle translucent 1px luminous top/left edge reflection
        super::analytic_aa::AnalyticAA::draw_rounded_outline(
            fb,
            x as f64,
            y as f64,
            w as f64,
            h as f64,
            radius as f64,
            1.0,
            0x40FFFFFF, // 25% white specular highlight
        );
    }

    /// Renders a physically-attenuated smooth Gaussian drop shadow underneath a floating element.
    pub fn draw_smooth_shadow(
        fb: &mut Framebuffer,
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
        if blur_radius == 0 || shadow_alpha == 0 || w == 0 || h == 0 {
            return;
        }

        let margin = blur_radius * 2 + 2;
        let pad_w = w + margin * 2;
        let pad_h = h + margin * 2;

        let mut mask = vec![0u8; pad_w * pad_h];

        // Fill inner rounded rect in mask with full 255 opacity
        let r = radius as f64;
        let inner_x = margin as f64;
        let inner_y = margin as f64;
        let inner_w = w as f64;
        let inner_h = h as f64;
        let center_x = inner_x + inner_w * 0.5;
        let center_y = inner_y + inner_h * 0.5;
        let half_w = inner_w * 0.5;
        let half_h = inner_h * 0.5;

        for py in 0..pad_h {
            let p_y = py as f64 + 0.5;
            for px in 0..pad_w {
                let p_x = px as f64 + 0.5;

                let q_x = (p_x - center_x).abs() - (half_w - r);
                let q_y = (p_y - center_y).abs() - (half_h - r);
                let outside_x = q_x.max(0.0);
                let outside_y = q_y.max(0.0);
                let inside = q_x.max(q_y).min(0.0);
                let dist = outside_x.hypot(outside_y) + inside - r;

                let cov = (0.5 - dist).clamp(0.0, 1.0);
                mask[py * pad_w + px] = (cov * 255.0).round() as u8;
            }
        }

        // Apply separable blur to mask (3 passes)
        let mut temp_mask = vec![0u8; pad_w * pad_h];
        let r_blur = blur_radius.max(1);

        for _ in 0..3 {
            Self::box_blur_mask_h(&mask, &mut temp_mask, pad_w, pad_h, r_blur);
            Self::box_blur_mask_v(&temp_mask, &mut mask, pad_w, pad_h, r_blur);
        }

        // Composite blurred shadow mask onto target Framebuffer
        let dest_origin_x = x as isize + offset_x - margin as isize;
        let dest_origin_y = y as isize + offset_y - margin as isize;
        let base_alpha_f = shadow_alpha as f64 / 255.0;

        for my in 0..pad_h {
            let fy = dest_origin_y + my as isize;
            if fy < 0 || fy >= fb.height as isize {
                continue;
            }
            let uy = fy as usize;

            for mx in 0..pad_w {
                let fx = dest_origin_x + mx as isize;
                if fx < 0 || fx >= fb.width as isize {
                    continue;
                }
                let ux = fx as usize;

                let m_val = mask[my * pad_w + mx];
                if m_val > 1 {
                    let a = ((m_val as f64 * base_alpha_f)).round().clamp(0.0, 255.0) as u8;
                    fb.blend_pixel_gamma(ux, uy, shadow_color, a);
                }
            }
        }
    }

    fn box_blur_mask_h(src: &[u8], dst: &mut [u8], width: usize, height: usize, r: usize) {
        let div = (2 * r + 1) as u32;
        for y in 0..height {
            let row = y * width;
            let mut sum: u32 = src[row] as u32 * (r as u32 + 1);
            for i in 0..r {
                sum += src[row + i.min(width - 1)] as u32;
            }
            for x in 0..width {
                dst[row + x] = (sum / div) as u8;
                let add_x = (x + r + 1).min(width - 1);
                let sub_x = x.saturating_sub(r);
                sum = sum.saturating_add(src[row + add_x] as u32).saturating_sub(src[row + sub_x] as u32);
            }
        }
    }

    fn box_blur_mask_v(src: &[u8], dst: &mut [u8], width: usize, height: usize, r: usize) {
        let div = (2 * r + 1) as u32;
        for x in 0..width {
            let mut sum: u32 = src[x] as u32 * (r as u32 + 1);
            for i in 0..r {
                sum += src[i.min(height - 1) * width + x] as u32;
            }
            for y in 0..height {
                dst[y * width + x] = (sum / div) as u8;
                let add_y = (y + r + 1).min(height - 1);
                let sub_y = y.saturating_sub(r);
                sum = sum.saturating_add(src[add_y * width + x] as u32).saturating_sub(src[sub_y * width + x] as u32);
            }
        }
    }

    #[inline]
    fn unpack_argb(c: u32) -> (u32, u32, u32, u32) {
        ((c >> 24) & 0xFF, (c >> 16) & 0xFF, (c >> 8) & 0xFF, c & 0xFF)
    }

    #[inline]
    fn pack_argb(a: u32, r: u32, g: u32, b: u32) -> u32 {
        ((a.min(255)) << 24) | ((r.min(255)) << 16) | ((g.min(255)) << 8) | (b.min(255))
    }
}
