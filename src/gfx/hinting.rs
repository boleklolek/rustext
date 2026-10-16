//! Font Hinting and Grid-Fitting Engine.
//! Snaps stems, baselines, and stroke contours to physical integer pixel grids.
//! 100% pure standard-library Rust with zero third-party dependencies.

/// Level of font hinting and grid-fitting applied during rasterization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HintingLevel {
    /// No hinting; purely continuous float sampling.
    None,
    /// Slight hinting: snaps baselines and line metrics to integer pixels.
    Slight,
    /// Medium hinting: snaps baselines and vertical stems to integer pixel columns (Recommended).
    #[default]
    Medium,
    /// Full hinting: snaps both vertical and horizontal stems, baselines, and boundaries.
    Full,
}

impl HintingLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().trim() {
            "none" | "off" | "false" => Self::None,
            "slight" | "light" => Self::Slight,
            "full" | "strong" => Self::Full,
            _ => Self::Medium,
        }
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Slight => "slight",
            Self::Medium => "medium",
            Self::Full => "full",
        }
    }
}

/// Font hinting and subpixel grid-alignment processor.
pub struct FontHinter;

impl FontHinter {
    /// Snaps a floating-point coordinate to the nearest integer pixel boundary.
    #[inline]
    pub fn snap_to_pixel(coord: f64) -> usize {
        coord.round().max(0.0) as usize
    }

    /// Snaps a logical coordinate with subpixel offset to physical grid.
    #[inline]
    pub fn snap_coord(coord: f64, level: HintingLevel) -> f64 {
        match level {
            HintingLevel::None => coord,
            HintingLevel::Slight | HintingLevel::Medium | HintingLevel::Full => coord.round(),
        }
    }

    /// Snaps font baseline Y position relative to line height and font size.
    #[inline]
    pub fn snap_baseline(y: usize, line_height: usize, font_height: usize, level: HintingLevel) -> usize {
        if level == HintingLevel::None {
            return y + (line_height.saturating_sub(font_height)) / 2;
        }
        let raw_y = y + (line_height.saturating_sub(font_height)) / 2;
        // Snap to whole integer pixel grid
        raw_y
    }

    /// Applies stem snapping and contrast enhancement to a glyph coverage mask.
    pub fn hint_coverage(
        coverage: &[u8],
        width: usize,
        height: usize,
        level: HintingLevel,
    ) -> Vec<u8> {
        if level == HintingLevel::None || width == 0 || height == 0 {
            return coverage.to_vec();
        }

        let mut out = coverage.to_vec();

        // 1. Vertical Stem Snapping (Medium & Full):
        // Detect columns with strong vertical stroke energy and consolidate split subpixels
        if matches!(level, HintingLevel::Medium | HintingLevel::Full) {
            Self::snap_vertical_stems(&mut out, width, height);
        }

        // 2. Horizontal Stem Snapping (Full only):
        // Detect horizontal bars (like '-', '=', 'E', 'F', 'T') and lock them to integer rows
        if level == HintingLevel::Full {
            Self::snap_horizontal_stems(&mut out, width, height);
        }

        // 3. Minimum Stem Contrast Enforcement:
        // Ensure that thin stems (width 1 px) don't fade into grey obscurity
        Self::enforce_minimum_stem_contrast(&mut out, width, height);

        out
    }

    /// Snaps vertical stroke stems to whole pixel columns.
    fn snap_vertical_stems(buf: &mut [u8], width: usize, height: usize) {
        if width < 3 {
            return;
        }

        // Compute column energy sums
        let mut col_energy = vec![0u32; width];
        for y in 0..height {
            for x in 0..width {
                col_energy[x] += buf[y * width + x] as u32;
            }
        }

        let threshold = (height as u32 * 60).max(100);

        // Find pairs of split adjacent columns (e.g. col x and col x+1 both ~50%)
        let mut x = 0;
        while x + 1 < width {
            let e1 = col_energy[x];
            let e2 = col_energy[x + 1];

            if e1 > threshold / 2 && e2 > threshold / 2 && (e1 + e2) > threshold {
                // Dominant column receives energy, subordinate is trimmed to sharpen edge
                let target_col = if e1 >= e2 { x } else { x + 1 };
                let other_col = if e1 >= e2 { x + 1 } else { x };

                for y in 0..height {
                    let idx_target = y * width + target_col;
                    let idx_other = y * width + other_col;
                    let v_target = buf[idx_target];
                    let v_other = buf[idx_other];

                    // If both pixels are already solid stroke pixels, preserve intentional multi-pixel stroke!
                    if v_target >= 200 && v_other >= 200 {
                        continue;
                    }

                    let combined = v_target as u16 + v_other as u16;

                    // Boost target column, attenuate split bleeding
                    buf[idx_target] = combined.min(255) as u8;
                    buf[idx_other] = (v_other / 4).min(40);
                }
                x += 2;
            } else {
                x += 1;
            }
        }
    }

    /// Snaps horizontal stroke bars to whole pixel rows.
    fn snap_horizontal_stems(buf: &mut [u8], width: usize, height: usize) {
        if height < 3 {
            return;
        }

        // Compute row energy sums
        let mut row_energy = vec![0u32; height];
        for y in 0..height {
            for x in 0..width {
                row_energy[y] += buf[y * width + x] as u32;
            }
        }

        let threshold = (width as u32 * 60).max(80);

        let mut y = 0;
        while y + 1 < height {
            let e1 = row_energy[y];
            let e2 = row_energy[y + 1];

            if e1 > threshold / 2 && e2 > threshold / 2 && (e1 + e2) > threshold {
                let target_row = if e1 >= e2 { y } else { y + 1 };
                let other_row = if e1 >= e2 { y + 1 } else { y };

                for x in 0..width {
                    let idx_target = target_row * width + x;
                    let idx_other = other_row * width + x;
                    let v_target = buf[idx_target];
                    let v_other = buf[idx_other];

                    // If both pixels are already solid stroke pixels, preserve intentional multi-pixel stroke!
                    if v_target >= 200 && v_other >= 200 {
                        continue;
                    }

                    let combined = v_target as u16 + v_other as u16;

                    buf[idx_target] = combined.min(255) as u8;
                    buf[idx_other] = (v_other / 4).min(40);
                }
                y += 2;
            } else {
                y += 1;
            }
        }
    }

    /// Enforces minimum stroke contrast on thin stems so they remain perceptible.
    fn enforce_minimum_stem_contrast(buf: &mut [u8], width: usize, height: usize) {
        for y in 0..height {
            for x in 0..width {
                let val = buf[y * width + x];
                // If a pixel is an active stroke center (val in 100..200 range with lower neighbors)
                if val >= 100 && val < 210 {
                    let left = if x > 0 { buf[y * width + x - 1] } else { 0 };
                    let right = if x + 1 < width { buf[y * width + x + 1] } else { 0 };
                    // If it is a local peak, boost it to solid contrast
                    if val > left && val > right {
                        buf[y * width + x] = (val as u16 * 13 / 10).min(255) as u8;
                    }
                }
            }
        }
    }
}
