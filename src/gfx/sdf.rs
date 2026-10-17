//! High-Resolution Signed Distance Fields (SDF) Glyph Rasterization Engine.
//! Computes exact Euclidean Signed Distance Transforms in O(N) linear time (Meijster's algorithm),
//! enabling resolution-independent contour evaluation, arbitrary zoom scaling without blur,
//! dynamic stroke-width tuning, and analytical subpixel anti-aliasing.
//! Pure standard-library Rust with zero external crates.

use std::collections::BTreeMap;
use std::sync::{OnceLock, RwLock};
use super::font::{FontFamily, FontWeight, FontRasterizer, FONT_HEIGHT, FONT_WIDTH};

/// A 2D Signed Distance Field representation of a glyph silhouette.
#[derive(Debug, Clone, PartialEq)]
pub struct SignedDistanceField {
    pub width: usize,
    pub height: usize,
    /// Signed Euclidean distance for each grid cell in pixel units.
    /// Positive values (> 0.0) represent interior glyph points.
    /// Zero (0.0) represents the exact contour boundary.
    /// Negative values (< 0.0) represent exterior background points.
    pub distances: Vec<f32>,
}

impl SignedDistanceField {
    /// Creates an empty distance field of given dimensions.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            distances: vec![0.0; width * height],
        }
    }

    /// Obtains signed distance at integer pixel coordinate (x, y).
    #[inline]
    pub fn get_distance(&self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            self.distances[y * self.width + x]
        } else {
            -1e6
        }
    }

    /// Computes the exact Euclidean Signed Distance Transform from an anti-aliased coverage mask.
    /// Uses Meijster's O(W * H) linear-time algorithm with sub-pixel boundary initialization.
    pub fn from_coverage(coverage: &[u8], width: usize, height: usize) -> Self {
        if width == 0 || height == 0 || coverage.len() < width * height {
            return Self::new(width.max(1), height.max(1));
        }

        // 1. Build binary boundary masks with sub-pixel fractional initialization
        // Inside: coverage >= 128 (feature = inside)
        // Outside: coverage < 128 (feature = outside)
        let total_cells = width * height;
        let mut interior_feature = vec![false; total_cells];
        let mut exterior_feature = vec![false; total_cells];

        for i in 0..total_cells {
            let cov = coverage[i];
            if cov >= 128 {
                interior_feature[i] = true;
            } else {
                exterior_feature[i] = true;
            }
        }

        // 2. Compute Euclidean Distance Transform to exterior (for interior points)
        // and to interior (for exterior points) using Meijster's linear-time 2D transform
        let dist_to_ext = Self::compute_edt_squared(&exterior_feature, width, height);
        let dist_to_int = Self::compute_edt_squared(&interior_feature, width, height);

        let mut distances = vec![0.0f32; total_cells];

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let cov = coverage[idx] as f32;

                if interior_feature[idx] {
                    // Inside the glyph: positive distance
                    let d_raw = dist_to_ext[idx].sqrt();
                    // Sub-pixel edge refinement based on fractional alpha
                    let subpixel_offset = (cov - 128.0) / 255.0;
                    distances[idx] = (d_raw + subpixel_offset * 0.5).max(0.0);
                } else {
                    // Outside the glyph: negative distance
                    let d_raw = dist_to_int[idx].sqrt();
                    let subpixel_offset = (128.0 - cov) / 255.0;
                    distances[idx] = -(d_raw + subpixel_offset * 0.5).max(0.0);
                }
            }
        }

        Self {
            width,
            height,
            distances,
        }
    }

    /// Computes squared Euclidean Distance Transform using Meijster's 2-pass algorithm.
    /// Time complexity: O(W * H) strictly linear in pixel count.
    fn compute_edt_squared(features: &[bool], width: usize, height: usize) -> Vec<f32> {
        let inf = (width * width + height * height) as f32 * 4.0;
        let mut g = vec![inf; width * height];

        // Pass 1: 1D vertical distance transform per column
        for x in 0..width {
            // Forward pass (top to bottom)
            if features[x] {
                g[x] = 0.0;
            }
            for y in 1..height {
                let idx = y * width + x;
                let prev_idx = (y - 1) * width + x;
                if features[idx] {
                    g[idx] = 0.0;
                } else if g[prev_idx] < inf {
                    g[idx] = g[prev_idx] + 1.0;
                }
            }

            // Backward pass (bottom to top)
            for y in (0..height.saturating_sub(1)).rev() {
                let idx = y * width + x;
                let next_idx = (y + 1) * width + x;
                if g[next_idx] + 1.0 < g[idx] {
                    g[idx] = g[next_idx] + 1.0;
                }
            }
        }

        let mut dt2 = vec![inf; width * height];
        let mut s = vec![0usize; width];
        let mut t = vec![0usize; width];

        // Pass 2: 1D horizontal parabolic lower-envelope per row
        for y in 0..height {
            let row_offset = y * width;
            let mut q = 0usize;
            s[0] = 0;
            t[0] = 0;

            // Parabolic intersection helper:
            // f(i, u) = (u - i)^2 + g(i)^2
            // s(i, j) = ((j^2 + g(j)^2) - (i^2 + g(i)^2)) / (2 * (j - i))
            let f = |i: usize, u: usize| -> f32 {
                let diff = (u as f32) - (i as f32);
                let gi = g[row_offset + i];
                diff * diff + gi * gi
            };

            let sep = |i: usize, j: usize| -> f32 {
                let gi = g[row_offset + i];
                let gj = g[row_offset + j];
                let num = ((j * j) as f32 + gj * gj) - ((i * i) as f32 + gi * gi);
                let den = 2.0 * ((j as f32) - (i as f32));
                num / den
            };

            for u in 1..width {
                while q > 0 && sep(s[q], u) <= (t[q] as f32) {
                    q -= 1;
                }
                let w = sep(s[q], u).floor() as isize + 1;
                let w = w.max(0) as usize;
                if w < width {
                    q += 1;
                    s[q] = u;
                    t[q] = w;
                }
            }

            for u in (0..width).rev() {
                while q > 0 && (t[q] > u) {
                    q -= 1;
                }
                dt2[row_offset + u] = f(s[q], u);
            }
        }

        dt2
    }

    /// Evaluates signed distance at continuous fractional grid coordinates `(u, v)`
    /// using continuous bilinear interpolation.
    #[inline]
    pub fn sample_bilinear(&self, u: f64, v: f64) -> f32 {
        if self.width == 0 || self.height == 0 {
            return -10.0;
        }

        let u_clamped = u.clamp(0.0, (self.width - 1) as f64);
        let v_clamped = v.clamp(0.0, (self.height - 1) as f64);

        let x0 = u_clamped.floor() as usize;
        let y0 = v_clamped.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);

        let fx = (u_clamped - x0 as f64) as f32;
        let fy = (v_clamped - y0 as f64) as f32;

        let d00 = self.distances[y0 * self.width + x0];
        let d10 = self.distances[y0 * self.width + x1];
        let d01 = self.distances[y1 * self.width + x0];
        let d11 = self.distances[y1 * self.width + x1];

        let top = (1.0 - fx) * d00 + fx * d10;
        let bot = (1.0 - fx) * d01 + fx * d11;

        (1.0 - fy) * top + fy * bot
    }

    /// Converts a continuous signed distance value to anti-aliased alpha coverage [0..=255]
    /// with analytical transition width and stroke thickness offset.
    #[inline]
    pub fn distance_to_alpha(dist: f32, transition_width: f32, weight_offset: f32) -> u8 {
        let w = transition_width.max(0.1);
        let val = ((dist + weight_offset) / w) + 0.5;
        (val.clamp(0.0, 1.0) * 255.0).round() as u8
    }

    /// Rasterizes glyph from this signed distance field to target dimensions `(target_w x target_h)`
    /// using analytical continuous anti-aliasing and optional stroke-weight tuning.
    pub fn render_sized(
        &self,
        target_w: usize,
        target_h: usize,
        weight_offset: f32,
    ) -> (usize, usize, Vec<u8>) {
        let target_w = target_w.max(1);
        let target_h = target_h.max(1);
        let mut out = vec![0u8; target_w * target_h];

        let sx_scale = (self.width as f64) / (target_w as f64);
        let sy_scale = (self.height as f64) / (target_h as f64);

        // Transition width adapts to pixel footprint in distance space
        let transition_w = (sx_scale.min(sy_scale) as f32) * 0.75;

        for y in 0..target_h {
            let v = (y as f64 + 0.5) * sy_scale - 0.5;
            for x in 0..target_w {
                let u = (x as f64 + 0.5) * sx_scale - 0.5;
                let dist = self.sample_bilinear(u, v);
                out[y * target_w + x] = Self::distance_to_alpha(dist, transition_w, weight_offset);
            }
        }

        (target_w, target_h, out)
    }

    /// Rasterizes glyph to target dimensions using spatial LCD RGB subpixel sampling.
    /// Evaluates 3 distinct horizontal subpixel offsets directly from the continuous distance field,
    /// yielding maximum horizontal sharpness with zero color fringing.
    pub fn render_subpixel(
        &self,
        target_w: usize,
        target_h: usize,
        weight_offset: f32,
    ) -> (usize, usize, Vec<[u8; 3]>) {
        let target_w = target_w.max(1);
        let target_h = target_h.max(1);
        let mut out = vec![[0u8; 3]; target_w * target_h];

        let sx_scale = (self.width as f64) / (target_w as f64);
        let sy_scale = (self.height as f64) / (target_h as f64);
        let sub_step = sx_scale / 3.0;
        let transition_w = (sub_step as f32) * 1.1;

        for y in 0..target_h {
            let v = (y as f64 + 0.5) * sy_scale - 0.5;
            for x in 0..target_w {
                let center_u = (x as f64 + 0.5) * sx_scale - 0.5;
                let u_r = center_u - sub_step;
                let u_g = center_u;
                let u_b = center_u + sub_step;

                let d_r = self.sample_bilinear(u_r, v);
                let d_g = self.sample_bilinear(u_g, v);
                let d_b = self.sample_bilinear(u_b, v);

                let ar = Self::distance_to_alpha(d_r, transition_w, weight_offset);
                let ag = Self::distance_to_alpha(d_g, transition_w, weight_offset);
                let ab = Self::distance_to_alpha(d_b, transition_w, weight_offset);

                out[y * target_w + x] = [ar, ag, ab];
            }
        }

        (target_w, target_h, out)
    }
}

/// Global thread-safe cache for precomputed Signed Distance Fields.
static SDF_CACHE: OnceLock<RwLock<BTreeMap<(char, FontFamily, FontWeight), SignedDistanceField>>> = OnceLock::new();

fn get_sdf_cache() -> &'static RwLock<BTreeMap<(char, FontFamily, FontWeight), SignedDistanceField>> {
    SDF_CACHE.get_or_init(|| RwLock::new(BTreeMap::new()))
}

/// High-level Signed Distance Field font manager.
pub struct SdfFontRasterizer;

impl SdfFontRasterizer {
    /// Obtains or computes the cached Signed Distance Field for a glyph.
    pub fn get_or_create_sdf(ch: char, family: FontFamily, weight: FontWeight) -> SignedDistanceField {
        let cache = get_sdf_cache();
        if let Ok(guard) = cache.read() {
            if let Some(sdf) = guard.get(&(ch, family, weight)) {
                return sdf.clone();
            }
        }

        // Generate base high-resolution 16x32 coverage to extract precise distance contours
        let (gw, gh, cov) = FontRasterizer::rasterize_glyph_sized(ch, family, FONT_WIDTH * 2, FONT_HEIGHT * 2, weight);
        let sdf = SignedDistanceField::from_coverage(&cov, gw, gh);

        if let Ok(mut guard) = cache.write() {
            guard.insert((ch, family, weight), sdf.clone());
        }

        sdf
    }

    /// Rasterizes glyph sized to `(target_w x target_h)` using high-resolution Signed Distance Fields.
    pub fn rasterize_glyph_sdf(
        ch: char,
        family: FontFamily,
        target_w: usize,
        target_h: usize,
        weight: FontWeight,
        weight_offset: f32,
    ) -> (usize, usize, Vec<u8>) {
        // Fast-path: box drawing glyphs use analytical straight line rasterization
        if FontRasterizer::is_box_drawing(ch) {
            return FontRasterizer::rasterize_glyph_sized(ch, family, target_w, target_h, weight);
        }

        let sdf = Self::get_or_create_sdf(ch, family, weight);
        sdf.render_sized(target_w, target_h, weight_offset)
    }

    /// Rasterizes glyph using Signed Distance Fields with spatial subpixel LCD anti-aliasing.
    pub fn rasterize_glyph_sdf_subpixel(
        ch: char,
        family: FontFamily,
        target_w: usize,
        target_h: usize,
        weight: FontWeight,
        weight_offset: f32,
    ) -> (usize, usize, Vec<[u8; 3]>) {
        if FontRasterizer::is_box_drawing(ch) {
            return FontRasterizer::rasterize_glyph_subpixel(ch, family, target_w, target_h, weight);
        }

        let sdf = Self::get_or_create_sdf(ch, family, weight);
        sdf.render_subpixel(target_w, target_h, weight_offset)
    }
}
