//! Human Visual System (HVS) Edge Perception, Weber-Fechner Contrast, and Unsharp Tuning.
//! Optimizes perceived edge sharpness and legibility across diverse display luminances.
//! 100% pure standard-library Rust with zero third-party dependencies.

use super::framebuffer::Framebuffer;

/// Perceptual edge tuning and spatial contrast enhancement engine.
pub struct EdgePerception;

impl EdgePerception {
    /// Computes perceptual relative luminance of an sRGB color (0.0 ..= 1.0) according to ITU-R BT.709.
    #[inline]
    pub fn relative_luminance(color: u32) -> f64 {
        let r = ((color >> 16) & 0xFF) as f64 / 255.0;
        let g = ((color >> 8) & 0xFF) as f64 / 255.0;
        let b = (color & 0xFF) as f64 / 255.0;
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// Adapts edge coverage values based on local background vs. foreground luminance contrast.
    /// Counteracts the human irradiation illusion (light text on dark backgrounds appearing blurred).
    #[inline]
    pub fn adapt_edge_coverage(alpha: u8, fg_color: u32, bg_color: u32) -> u8 {
        if alpha == 0 || alpha == 255 {
            return alpha;
        }

        let lum_fg = Self::relative_luminance(fg_color);
        let lum_bg = Self::relative_luminance(bg_color);

        let a = alpha as f64 / 255.0;

        // Dark background with light foreground: boost middle transitions slightly
        let adapted = if lum_fg > lum_bg {
            let contrast_delta = (lum_fg - lum_bg).clamp(0.0, 1.0);
            // S-curve contrast boost according to Weber-Fechner perception
            let boosted = a.powf(1.0 - 0.25 * contrast_delta);
            boosted.clamp(0.0, 1.0)
        } else {
            // Light background with dark foreground: maintain crisp slender stems
            let contrast_delta = (lum_bg - lum_fg).clamp(0.0, 1.0);
            let slender = a.powf(1.0 + 0.15 * contrast_delta);
            slender.clamp(0.0, 1.0)
        };

        (adapted * 255.0).round() as u8
    }

    /// Enhances edge sharpness on a glyph coverage mask using local gradient magnitude.
    pub fn sharpen_coverage_mask(
        mask: &[u8],
        width: usize,
        height: usize,
        amount: f64,
    ) -> Vec<u8> {
        if amount <= 0.0 || width < 3 || height < 3 {
            return mask.to_vec();
        }

        let mut out = mask.to_vec();
        let k = amount.clamp(0.0, 1.5);

        for y in 1..(height - 1) {
            for x in 1..(width - 1) {
                let idx = y * width + x;
                let c = mask[idx] as f64;

                // If pixel is an edge transition (not pure background or pure interior)
                if c > 10.0 && c < 245.0 {
                    // Laplacian filter: center * 4 - (up + down + left + right)
                    let up = mask[(y - 1) * width + x] as f64;
                    let down = mask[(y + 1) * width + x] as f64;
                    let left = mask[y * width + (x - 1)] as f64;
                    let right = mask[y * width + (x + 1)] as f64;

                    let laplacian = 4.0 * c - (up + down + left + right);
                    let sharpened = (c + k * laplacian).clamp(0.0, 255.0);
                    out[idx] = sharpened.round() as u8;
                }
            }
        }

        out
    }

    /// Applies an unsharp masking filter to a region of the framebuffer to enhance perceived text and UI edges.
    pub fn unsharp_mask_region(
        fb: &mut Framebuffer,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        amount: f64,
    ) {
        if amount <= 0.0 || w < 3 || h < 3 || x >= fb.width || y >= fb.height {
            return;
        }

        let rw = w.min(fb.width - x);
        let rh = h.min(fb.height - y);

        // Copy source region
        let mut original = Vec::with_capacity(rw * rh);
        for py in 0..rh {
            let row_start = (y + py) * fb.width + x;
            original.extend_from_slice(&fb.pixels[row_start..row_start + rw]);
        }

        // Generate blurred low-pass version
        let mut blurred = original.clone();
        super::blur::BlurEngine::gaussian_blur_argb(&mut blurred, rw, rh, 1);

        let k = amount.clamp(0.0, 2.0);

        // High-pass sharpening: Output = Original + k * (Original - Blurred)
        for py in 0..rh {
            let row_start = (y + py) * fb.width + x;
            let patch_start = py * rw;

            for px in 0..rw {
                let orig_px = original[patch_start + px];
                let blur_px = blurred[patch_start + px];

                let (oa, or, og, ob) = (
                    (orig_px >> 24) & 0xFF,
                    (orig_px >> 16) & 0xFF,
                    (orig_px >> 8) & 0xFF,
                    orig_px & 0xFF,
                );

                let (_ba, br, bg, bb) = (
                    (blur_px >> 24) & 0xFF,
                    (blur_px >> 16) & 0xFF,
                    (blur_px >> 8) & 0xFF,
                    blur_px & 0xFF,
                );

                let nr = (or as f64 + k * (or as f64 - br as f64)).clamp(0.0, 255.0).round() as u32;
                let ng = (og as f64 + k * (og as f64 - bg as f64)).clamp(0.0, 255.0).round() as u32;
                let nb = (ob as f64 + k * (ob as f64 - bb as f64)).clamp(0.0, 255.0).round() as u32;

                fb.pixels[row_start + px] = (oa << 24) | (nr << 16) | (ng << 8) | nb;
            }
        }
    }
}
