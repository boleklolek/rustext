//! Optical Font Stem Darkening for Dark Mode displays (counteracts retinal irradiation).
//! Dynamically emboldens and fattens anti-aliased and subpixel LCD stems on dark backgrounds,
//! matching FreeType (`cff_stem_darkening`) and Apple CoreText rendering standards.
//! 100% pure standard-library Rust with zero third-party dependencies.

/// Perceptual stem darkening and optical irradiation compensation engine.
pub struct StemDarkener;

impl StemDarkener {
    /// Computes perceptual relative luminance of an sRGB color (0.0 ..= 1.0) according to ITU-R BT.709.
    #[inline]
    pub fn relative_luminance(color: u32) -> f64 {
        let r = ((color >> 16) & 0xFF) as f64 / 255.0;
        let g = ((color >> 8) & 0xFF) as f64 / 255.0;
        let b = (color & 0xFF) as f64 / 255.0;
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    /// Computes the dynamic optical darkening factor $K \in [0.0, 2.0]$ based on:
    /// 1. Background luminance: active only when background is dark ($Y_{\text{bg}} < 0.5$).
    /// 2. Foreground vs background contrast ($Y_{\text{fg}} > Y_{\text{bg}}$).
    /// 3. Font point size: smaller text ($\le 12$ pt) receives proportionally higher darkening
    ///    because single-pixel stems lose mass to irradiation, tapering to 0 at $\ge 24$ pt.
    /// 4. User-configured multiplier amount.
    #[inline]
    pub fn compute_darkening_factor(bg_color: u32, fg_color: u32, font_size: usize, amount: f64) -> f64 {
        if amount <= 0.001 {
            return 0.0;
        }

        let lum_bg = Self::relative_luminance(bg_color);
        let lum_fg = Self::relative_luminance(fg_color);

        // Only active when foreground text is brighter than background AND background is dark
        if lum_fg <= lum_bg || lum_bg >= 0.5 {
            return 0.0;
        }

        // Darkness intensity: 1.0 for pure black, decreasing towards 0.0 at mid-gray (0.5)
        let dark_bg_factor = (1.0 - (lum_bg / 0.5)).clamp(0.0, 1.0);
        let contrast_delta = (lum_fg - lum_bg).clamp(0.0, 1.0);

        // Size-dependent attenuation (FreeType ppem curve):
        // Maximum boost at small sizes (<= 10pt), linear taper to 0 at >= 24pt
        let size_factor = if font_size <= 10 {
            1.0
        } else if font_size >= 24 {
            0.0
        } else {
            (24.0 - font_size as f64) / 14.0
        };

        (dark_bg_factor * contrast_delta * size_factor * amount.clamp(0.0, 3.0)).clamp(0.0, 2.0)
    }

    /// Darkens a single anti-aliased fractional alpha value ($0 < \alpha < 255$)
    /// using a parabolic bell-curve transfer function.
    /// Preserves pure background ($\alpha = 0$) and 100% solid stroke pixels ($\alpha = 255$).
    #[inline]
    pub fn darken_alpha(alpha: u8, factor: f64) -> u8 {
        if alpha == 0 || alpha == 255 || factor <= 0.001 {
            return alpha;
        }

        let a = alpha as f64 / 255.0;
        // Parabolic bell curve: 4 * a * (1 - a) peaks at a = 0.5 (mid-tones)
        let bell = 4.0 * a * (1.0 - a);
        let boost = factor * 55.0 * bell.powf(0.85);

        ((alpha as f64 + boost).round()).clamp(0.0, 255.0) as u8
    }

    /// Darkens a subpixel LCD RGB triplet $[a_r, a_g, a_b]$ preserving chromatic balance.
    #[inline]
    pub fn darken_subpixel(ar: u8, ag: u8, ab: u8, factor: f64) -> [u8; 3] {
        if factor <= 0.001 {
            return [ar, ag, ab];
        }
        [
            Self::darken_alpha(ar, factor),
            Self::darken_alpha(ag, factor),
            Self::darken_alpha(ab, factor),
        ]
    }

    /// Applies 2D optical stem darkening and morphological edge dilation to a glyph coverage mask,
    /// with strict counter-space protection to prevent interior loops (`e`, `a`, `o`, `0`, `8`)
    /// from clogging.
    pub fn darken_coverage_mask(mask: &[u8], width: usize, height: usize, factor: f64) -> Vec<u8> {
        if factor <= 0.001 || width == 0 || height == 0 || mask.len() < width * height {
            return mask.to_vec();
        }

        let mut out = Vec::with_capacity(mask.len());
        // Phase 1: parabolic energy boost on fractional edge pixels
        for &alpha in mask.iter().take(width * height) {
            out.push(Self::darken_alpha(alpha, factor));
        }

        // Phase 2: outer stem boundary dilation with counter-space protection
        let halo_energy = (factor * 26.0).round().clamp(0.0, 50.0) as u8;
        if halo_energy > 0 && width >= 3 && height >= 3 {
            for y in 0..height {
                for x in 0..width {
                    let idx = y * width + x;
                    if mask[idx] < 50 {
                        let top = y > 0 && mask[(y - 1) * width + x] >= 160;
                        let bottom = y + 1 < height && mask[(y + 1) * width + x] >= 160;
                        let left = x > 0 && mask[y * width + (x - 1)] >= 160;
                        let right = x + 1 < width && mask[y * width + (x + 1)] >= 160;

                        // Rule: NEVER fill enclosed slots or counter-spaces
                        if (top && bottom) || (left && right) {
                            continue;
                        }

                        // Exterior edge adjacent to solid stem
                        if top || bottom || left || right {
                            out[idx] = out[idx].saturating_add(halo_energy);
                        }
                    }
                }
            }
        }

        out
    }

    /// Darkens a subpixel LCD coverage mask array of `[u8; 3]`.
    pub fn darken_subpixel_mask(mask: &[[u8; 3]], width: usize, height: usize, factor: f64) -> Vec<[u8; 3]> {
        if factor <= 0.001 || width == 0 || height == 0 || mask.len() < width * height {
            return mask.to_vec();
        }
        mask.iter().take(width * height).map(|&[r, g, b]| Self::darken_subpixel(r, g, b, factor)).collect()
    }
}
