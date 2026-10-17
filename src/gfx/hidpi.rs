//! Automatic HiDPI Display Detection and ISO 9241-303 Ergonomic Scaling Engine.
//! Provides hardware-level pixel density calculation, VESA EDID parsing,
//! visual angle measurement, and ergonomic legibility assessment under ISO 9241-303.
//! Pure standard-library Rust with zero external crates.

use std::fs;
use std::path::Path;

/// Display form factor category based on physical diagonal screen size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayFormFactor {
    /// Ultra-compact / netbook / tablet (< 13.0 inches).
    UltraCompact,
    /// Standard laptop / ultrabook (13.0 to 15.0 inches).
    Laptop,
    /// Large mobile workstation / portable monitor (15.0 to 18.0 inches).
    LargeLaptop,
    /// Standard desktop monitor (18.0 to 27.0 inches).
    Desktop,
    /// Large format / Ultrawide / TV (> 27.0 inches).
    LargeDisplay,
    /// Unknown or virtual display.
    Unknown,
}

impl DisplayFormFactor {
    /// Returns a human-friendly Spanish description of the form factor.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::UltraCompact => "Ultra-portátil (< 13\")",
            Self::Laptop => "Portátil (13\"-15\")",
            Self::LargeLaptop => "Portátil grande (15\"-18\")",
            Self::Desktop => "Monitor de escritorio (18\"-27\")",
            Self::LargeDisplay => "Pantalla grande (> 27\")",
            Self::Unknown => "Pantalla genérica",
        }
    }

    /// Nominal ergonomic viewing distance in millimeters per ISO 9241-303 guidelines.
    pub fn default_viewing_distance_mm(&self) -> f64 {
        match self {
            Self::UltraCompact => 450.0, // 45 cm
            Self::Laptop => 480.0,       // 48 cm
            Self::LargeLaptop => 540.0,  // 54 cm
            Self::Desktop => 600.0,      // 60 cm (standard desktop arm's reach)
            Self::LargeDisplay => 750.0, // 75 cm
            Self::Unknown => 600.0,
        }
    }
}

/// Physical display geometry and resolution metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayMetrics {
    /// Horizontal addressable resolution in pixels.
    pub width_px: usize,
    /// Vertical addressable resolution in pixels.
    pub height_px: usize,
    /// Physical active display width in millimeters.
    pub width_mm: f64,
    /// Physical active display height in millimeters.
    pub height_mm: f64,
    /// Horizontal pixel density (Dots/Pixels Per Inch).
    pub dpi_x: f64,
    /// Vertical pixel density (Dots/Pixels Per Inch).
    pub dpi_y: f64,
    /// Average pixel density.
    pub dpi: f64,
    /// Physical screen diagonal in inches.
    pub diagonal_inches: f64,
    /// Detected form factor category.
    pub form_factor: DisplayFormFactor,
    /// Default ergonomic viewing distance in mm.
    pub default_viewing_distance_mm: f64,
    /// Information source for diagnostic and reporting purposes.
    pub source: &'static str,
}

impl Default for DisplayMetrics {
    fn default() -> Self {
        Self {
            width_px: 1920,
            height_px: 1080,
            width_mm: 508.0,
            height_mm: 285.0,
            dpi_x: 96.0,
            dpi_y: 96.0,
            dpi: 96.0,
            diagonal_inches: 22.9,
            form_factor: DisplayFormFactor::Desktop,
            default_viewing_distance_mm: 600.0,
            source: "Estándar 96 DPI Baseline",
        }
    }
}

/// Legibility rating according to ISO 9241-303 electronic visual display requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsoComplianceRating {
    /// Visual angle < 16.0 arcmin: Violates ISO 9241-303. Causes eye fatigue and illegibility.
    BelowMinimum,
    /// Visual angle 16.0 .. 20.0 arcmin: Complies with legal minimum, but suboptimal for prolonged coding.
    Acceptable,
    /// Visual angle 20.0 .. 24.0 arcmin: Recommended ergonomic sweet spot for continuous text reading.
    Optimal,
    /// Visual angle 24.0 .. 28.0 arcmin: High legibility / accessible viewing.
    Relaxed,
    /// Visual angle > 28.0 arcmin: Oversized; reduces information density.
    Oversized,
}

impl IsoComplianceRating {
    /// Short badge label for GUI chips.
    pub fn badge_label(&self) -> &'static str {
        match self {
            Self::BelowMinimum => "Deficiente",
            Self::Acceptable => "Aceptable",
            Self::Optimal => "Óptimo",
            Self::Relaxed => "Relajado",
            Self::Oversized => "Grande",
        }
    }

    /// Color representation (ARGB hex) for status chips.
    pub fn badge_color(&self) -> u32 {
        match self {
            Self::BelowMinimum => 0x00F44747, // Red
            Self::Acceptable => 0x00CCA700,   // Yellow / Amber
            Self::Optimal => 0x004EC9B0,      // Teal / Green
            Self::Relaxed => 0x00569CD6,      // Blue
            Self::Oversized => 0x00CE9178,    // Orange
        }
    }

    /// Returns true if the rating satisfies ISO 9241-303 minimum standards (>= 16 arcmin).
    pub fn is_compliant(&self) -> bool {
        !matches!(self, Self::BelowMinimum)
    }

    /// Returns true if the rating is within the recommended optimal range (20 to 24 arcmin).
    pub fn is_optimal(&self) -> bool {
        matches!(self, Self::Optimal)
    }
}

/// Ergonomic legibility assessment report under ISO 9241-303.
#[derive(Debug, Clone, PartialEq)]
pub struct IsoLegibilityReport {
    /// Subtended visual angle in arcminutes (minutes of arc, ').
    pub visual_angle_arcmin: f64,
    /// Physical character height on the screen surface in millimeters.
    pub physical_height_mm: f64,
    /// Standard ISO 9241-303 compliance category.
    pub rating: IsoComplianceRating,
    /// Recommended ergonomic scale factor to reach optimal target (22.0 arcmin).
    pub recommended_scale: f64,
    /// Viewing distance used for assessment in millimeters.
    pub viewing_distance_mm: f64,
    /// Active pixel density (DPI).
    pub dpi: f64,
    /// Human-readable explanation and recommendation.
    pub description: &'static str,
    /// Short badge text for GUI display (e.g., "ISO: 22.1' [Óptimo]").
    pub badge_text: String,
}

/// Automatic HiDPI detector and ISO 9241-303 ergonomic scaling calculator.
pub struct HiDpiDetector;

impl HiDpiDetector {
    /// Recommended target visual angle in arcminutes per ISO 9241-303 (20 to 24 arcmin sweet spot).
    pub const ISO_RECOMMENDED_ARCMIN: f64 = 22.0;

    /// Standard discrete ergonomic scaling steps avoiding fractional bitmap artifacts.
    pub const ERGONOMIC_SCALE_STEPS: [f64; 8] = [1.0, 1.25, 1.5, 1.75, 2.0, 2.25, 2.5, 3.0];

    /// Parses a 128-byte VESA EDID binary block to extract horizontal and vertical active pixels
    /// and millimeter physical display dimensions.
    ///
    /// Returns `Some((width_px, height_px, width_mm, height_mm))` if the EDID block is valid.
    pub fn parse_edid(bytes: &[u8]) -> Option<(usize, usize, f64, f64)> {
        if bytes.len() < 128 {
            return None;
        }

        // 1. Verify EDID magic header: 00 FF FF FF FF FF FF 00
        const EDID_HEADER: [u8; 8] = [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
        if bytes[0..8] != EDID_HEADER {
            return None;
        }

        // 2. Verify checksum (sum of all 128 bytes mod 256 must be 0)
        let sum: usize = bytes[0..128].iter().map(|&b| b as usize).sum();
        if sum % 256 != 0 {
            return None;
        }

        // 3. Extract physical dimensions from Basic Display Parameters (bytes 21 and 22 in cm)
        let cm_w = bytes[21] as f64;
        let cm_h = bytes[22] as f64;
        let mut width_mm = cm_w * 10.0;
        let mut height_mm = cm_h * 10.0;

        // 4. Parse Detailed Timing Descriptor 1 (bytes 54..72) for preferred mode and sub-millimeter size
        let pixel_clock = ((bytes[55] as u16) << 8) | (bytes[54] as u16);
        let mut width_px = 0usize;
        let mut height_px = 0usize;

        if pixel_clock > 0 {
            // Horizontal active pixels: lower 8 bits at byte 56, upper 4 bits at byte 58 [7:4]
            let h_active_low = bytes[56] as usize;
            let h_active_high = (bytes[58] as usize & 0xF0) << 4;
            width_px = h_active_high | h_active_low;

            // Vertical active pixels: lower 8 bits at byte 59, upper 4 bits at byte 61 [7:4]
            let v_active_low = bytes[59] as usize;
            let v_active_high = (bytes[61] as usize & 0xF0) << 4;
            height_px = v_active_high | v_active_low;

            // Image size in millimeters in DTD (bytes 66, 67, 68)
            let h_mm_low = bytes[66] as usize;
            let v_mm_low = bytes[67] as usize;
            let mm_high = bytes[68] as usize;
            let h_mm_high = (mm_high & 0xF0) << 4;
            let v_mm_high = (mm_high & 0x0F) << 8;

            let dtd_w_mm = (h_mm_high | h_mm_low) as f64;
            let dtd_h_mm = (v_mm_high | v_mm_low) as f64;

            if dtd_w_mm > 0.0 && dtd_h_mm > 0.0 {
                width_mm = dtd_w_mm;
                height_mm = dtd_h_mm;
            }
        }

        if width_px > 0 && height_px > 0 && width_mm > 0.0 && height_mm > 0.0 {
            Some((width_px, height_px, width_mm, height_mm))
        } else if width_mm > 0.0 && height_mm > 0.0 {
            // Fallback resolution 1920x1080 if DTD was absent but dimensions are valid
            Some((1920, 1080, width_mm, height_mm))
        } else {
            None
        }
    }

    /// Attempts to query Linux DRM sysfs (`/sys/class/drm/`) for active connectors and EDID data.
    pub fn detect_from_drm_sysfs() -> Option<DisplayMetrics> {
        let drm_path = Path::new("/sys/class/drm");
        if !drm_path.is_dir() {
            return None;
        }

        let entries = fs::read_dir(drm_path).ok()?;
        let mut candidates: Vec<(DisplayMetrics, bool)> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

            // Look for connector directories (e.g. card1-eDP-1, card0-DP-1, card0-HDMI-A-1)
            if !file_name.starts_with("card") || !file_name.contains('-') {
                continue;
            }

            // Check if status is "connected"
            let status_path = path.join("status");
            let is_connected = fs::read_to_string(&status_path)
                .map(|s| s.trim() == "connected")
                .unwrap_or(false);

            if !is_connected {
                continue;
            }

            // Read EDID
            let edid_path = path.join("edid");
            if let Ok(bytes) = fs::read(&edid_path) {
                if let Some((mut w_px, mut h_px, w_mm, h_mm)) = Self::parse_edid(&bytes) {
                    // If modes file exists, ensure we have the active resolution
                    let modes_path = path.join("modes");
                    if let Ok(modes_str) = fs::read_to_string(&modes_path) {
                        if let Some(first_mode) = modes_str.lines().next() {
                            let parts: Vec<&str> = first_mode.trim().split('x').collect();
                            if parts.len() == 2 {
                                if let (Ok(mw), Ok(mh)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                                    if mw > 0 && mh > 0 {
                                        w_px = mw;
                                        h_px = mh;
                                    }
                                }
                            }
                        }
                    }

                    let dpi_x = (w_px as f64 * 25.4) / w_mm;
                    let dpi_y = (h_px as f64 * 25.4) / h_mm;
                    let dpi = (dpi_x + dpi_y) / 2.0;
                    let diag_mm = (w_mm * w_mm + h_mm * h_mm).sqrt();
                    let diag_in = diag_mm / 25.4;

                    let form_factor = if diag_in < 13.0 {
                        DisplayFormFactor::UltraCompact
                    } else if diag_in < 15.0 {
                        DisplayFormFactor::Laptop
                    } else if diag_in < 18.0 {
                        DisplayFormFactor::LargeLaptop
                    } else if diag_in <= 27.0 {
                        DisplayFormFactor::Desktop
                    } else {
                        DisplayFormFactor::LargeDisplay
                    };

                    let default_dist = form_factor.default_viewing_distance_mm();
                    let is_internal = file_name.contains("eDP") || file_name.contains("LVDS") || file_name.contains("DSI");

                    candidates.push((
                        DisplayMetrics {
                            width_px: w_px,
                            height_px: h_px,
                            width_mm: w_mm,
                            height_mm: h_mm,
                            dpi_x,
                            dpi_y,
                            dpi,
                            diagonal_inches: diag_in,
                            form_factor,
                            default_viewing_distance_mm: default_dist,
                            source: "Linux DRM EDID",
                        },
                        is_internal,
                    ));
                }
            }
        }

        // Prefer internal display (laptop panel eDP) or the first connected monitor
        candidates.sort_by_key(|(_, internal)| if *internal { 0 } else { 1 });
        candidates.into_iter().next().map(|(m, _)| m)
    }

    /// Automatically detects display metrics using all available platform mechanisms.
    pub fn detect_display_metrics() -> DisplayMetrics {
        // 1. Hardware DRM EDID parsing on Linux
        if let Some(metrics) = Self::detect_from_drm_sysfs() {
            return metrics;
        }

        // 2. Check standard environment overrides
        if let Ok(val) = std::env::var("RUSTEXT_SCALE_FACTOR") {
            if let Ok(s) = val.trim().parse::<f64>() {
                if s >= 0.5 && s <= 5.0 {
                    let dpi = s * 96.0;
                    return DisplayMetrics {
                        dpi,
                        dpi_x: dpi,
                        dpi_y: dpi,
                        source: "Variable RUSTEXT_SCALE_FACTOR",
                        ..DisplayMetrics::default()
                    };
                }
            }
        }

        if let Ok(val) = std::env::var("GDK_SCALE") {
            if let Ok(s) = val.trim().parse::<f64>() {
                if s >= 1.0 && s <= 4.0 {
                    let dpi = s * 96.0;
                    return DisplayMetrics {
                        dpi,
                        dpi_x: dpi,
                        dpi_y: dpi,
                        source: "Variable GDK_SCALE",
                        ..DisplayMetrics::default()
                    };
                }
            }
        }

        // 3. Baseline standard fallback
        DisplayMetrics::default()
    }

    /// Calculates visual angle in arcminutes (minutes of arc, ') from physical character height
    /// on screen and user viewing distance in millimeters.
    ///
    /// Formula per ISO 9241-303:
    /// $\theta = 2 \cdot \arctan\left(\frac{h}{2 \cdot D}\right) \cdot \frac{10800}{\pi}$
    #[inline]
    pub fn calculate_visual_angle_arcmin(height_mm: f64, viewing_distance_mm: f64) -> f64 {
        if viewing_distance_mm <= 0.0 || height_mm <= 0.0 {
            return 0.0;
        }
        let half_angle_rad = (height_mm / (2.0 * viewing_distance_mm)).atan();
        let full_angle_rad = 2.0 * half_angle_rad;
        full_angle_rad * (180.0 * 60.0 / std::f64::consts::PI)
    }

    /// Calculates the required physical height in millimeters to achieve a target visual angle
    /// at a given viewing distance in millimeters.
    ///
    /// $h = 2 \cdot D \cdot \tan\left(\frac{\theta_{\text{rad}}}{2}\right)$
    #[inline]
    pub fn target_physical_height_mm(target_arcmin: f64, viewing_distance_mm: f64) -> f64 {
        if viewing_distance_mm <= 0.0 || target_arcmin <= 0.0 {
            return 0.0;
        }
        let target_rad = target_arcmin * (std::f64::consts::PI / (180.0 * 60.0));
        2.0 * viewing_distance_mm * (target_rad / 2.0).tan()
    }

    /// Computes the continuous ergonomic scale factor required to achieve the target visual angle
    /// (default 22.0 arcmin) given display DPI and viewing distance.
    ///
    /// Baseline: A standard 12pt font unscaled is 16 pixels high.
    pub fn compute_continuous_scale(dpi: f64, viewing_distance_mm: f64, target_arcmin: f64) -> f64 {
        let dpi = if dpi > 10.0 { dpi } else { 96.0 };
        let dist = if viewing_distance_mm > 50.0 { viewing_distance_mm } else { 600.0 };
        let target_angle = if target_arcmin > 5.0 { target_arcmin } else { Self::ISO_RECOMMENDED_ARCMIN };

        let target_h_mm = Self::target_physical_height_mm(target_angle, dist);
        let target_px = (target_h_mm * dpi) / 25.4;
        let baseline_px = 16.0; // 12pt standard cell height
        (target_px / baseline_px).max(0.5)
    }

    /// Quantizes a continuous scale factor to the closest ergonomic discrete step
    /// (1.0x, 1.25x, 1.5x, 1.75x, 2.0x, 2.25x, 2.5x, 3.0x) to eliminate fractional rasterization blur.
    pub fn quantize_ergonomic_scale(raw_scale: f64) -> f64 {
        if raw_scale <= 1.125 {
            1.0
        } else if raw_scale <= 1.375 {
            1.25
        } else if raw_scale <= 1.625 {
            1.5
        } else if raw_scale <= 1.875 {
            1.75
        } else if raw_scale <= 2.125 {
            2.0
        } else if raw_scale <= 2.375 {
            2.25
        } else if raw_scale <= 2.750 {
            2.5
        } else {
            3.0
        }
    }

    /// Computes the recommended ergonomic scale factor quantized to standard steps.
    pub fn compute_ergonomic_scale(dpi: f64, viewing_distance_mm: f64, target_arcmin: f64) -> f64 {
        let raw = Self::compute_continuous_scale(dpi, viewing_distance_mm, target_arcmin);
        Self::quantize_ergonomic_scale(raw)
    }

    /// Assesses character legibility against ISO 9241-303 criteria.
    ///
    /// - `char_height_px`: Character cell height in logical pixels (e.g. 16 for 12pt).
    /// - `scale_factor`: Active UI/HiDPI scale factor (e.g. 1.0, 1.25, 1.5, 2.0).
    /// - `dpi`: Display pixel density in dots per inch.
    /// - `viewing_distance_mm`: Distance from user's eyes to display surface.
    pub fn assess_legibility(
        char_height_px: usize,
        scale_factor: f64,
        dpi: f64,
        viewing_distance_mm: f64,
    ) -> IsoLegibilityReport {
        let scale = if scale_factor > 0.1 { scale_factor } else { 1.0 };
        let dpi = if dpi > 10.0 { dpi } else { 96.0 };
        let dist = if viewing_distance_mm > 50.0 { viewing_distance_mm } else { 600.0 };

        let physical_char_px = (char_height_px as f64) * scale;
        let physical_height_mm = (physical_char_px * 25.4) / dpi;
        let visual_angle = Self::calculate_visual_angle_arcmin(physical_height_mm, dist);

        let (rating, description) = if visual_angle < 16.0 {
            (
                IsoComplianceRating::BelowMinimum,
                "Deficiente (< 16') — Viola ISO 9241-303. Causa fatiga y estrés visual.",
            )
        } else if visual_angle < 20.0 {
            (
                IsoComplianceRating::Acceptable,
                "Aceptable (16'-20') — Cumple el mínimo ISO 9241-303; fatigante en sesiones largas.",
            )
        } else if visual_angle <= 24.0 {
            (
                IsoComplianceRating::Optimal,
                "Óptimo (20'-24') — Rango ergonómico recomendado por ISO 9241-303 para lectura continua.",
            )
        } else if visual_angle <= 28.0 {
            (
                IsoComplianceRating::Relaxed,
                "Relajado (24'-28') — Excelente legibilidad y confort visual accesible.",
            )
        } else {
            (
                IsoComplianceRating::Oversized,
                "Grande (> 28') — Caracteres muy grandes; reduce la densidad de información.",
            )
        };

        let recommended_scale = Self::compute_ergonomic_scale(dpi, dist, Self::ISO_RECOMMENDED_ARCMIN);
        let badge_text = format!("ISO 9241-303: {:.1}' [{}]", visual_angle, rating.badge_label());

        IsoLegibilityReport {
            visual_angle_arcmin: visual_angle,
            physical_height_mm,
            rating,
            recommended_scale,
            viewing_distance_mm: dist,
            dpi,
            description,
            badge_text,
        }
    }

    /// Global entry point to determine active scale factor:
    /// Checks environment variables, hardware DRM EDID, and calculates ISO 9241-303 ergonomic scale.
    pub fn detect_scale_factor() -> f64 {
        // 1. Explicit override via environment variables
        if let Ok(val) = std::env::var("RUSTEXT_SCALE_FACTOR") {
            if let Ok(s) = val.trim().parse::<f64>() {
                if s >= 0.5 && s <= 5.0 {
                    return s;
                }
            }
        }
        if let Ok(val) = std::env::var("GDK_SCALE") {
            if let Ok(s) = val.trim().parse::<f64>() {
                if s >= 1.0 && s <= 4.0 {
                    return s;
                }
            }
        }
        if let Ok(val) = std::env::var("QT_SCALE_FACTOR") {
            if let Ok(s) = val.trim().parse::<f64>() {
                if s >= 0.5 && s <= 4.0 {
                    return s;
                }
            }
        }
        if let Ok(val) = std::env::var("XCURSOR_SIZE") {
            if let Ok(size) = val.trim().parse::<f64>() {
                if size >= 32.0 {
                    let scale = size / 24.0;
                    return (scale * 2.0).round() / 2.0;
                }
            }
        }

        // 2. Query hardware display metrics via DRM EDID sysfs (sub-millimeter precision)
        if let Some(metrics) = Self::detect_from_drm_sysfs() {
            if metrics.dpi >= 115.0 {
                return Self::compute_ergonomic_scale(
                    metrics.dpi,
                    metrics.default_viewing_distance_mm,
                    Self::ISO_RECOMMENDED_ARCMIN,
                );
            }
        }

        1.0
    }
}
