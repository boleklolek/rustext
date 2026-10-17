use std::collections::BTreeMap;
use std::sync::OnceLock;

pub const FONT_WIDTH: usize = 8;
pub const FONT_HEIGHT: usize = 16;

/// Fixed-size 8x16 anti-aliased coverage mask (128 bytes, 0..=255 per subpixel).
pub type GlyphCoverage16 = [u8; 128];

/// Fixed-size 8x16 subpixel LCD coverage mask (128 pixels, 3 subpixels per pixel [R, G, B] = 384 bytes).
pub type SubpixelCoverage16 = [[u8; 3]; 128];

/// Font Family identifier for monospace coding fonts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FontFamily {
    Standard,
    Terminus,
    Modern,
    Typewriter,
    Retro,
}

impl FontFamily {
    pub const fn as_str(&self) -> &'static str {
        match self {
            FontFamily::Standard => "standard",
            FontFamily::Terminus => "terminus",
            FontFamily::Modern => "modern",
            FontFamily::Typewriter => "typewriter",
            FontFamily::Retro => "retro",
        }
    }

    pub const fn display_name(&self) -> &'static str {
        match self {
            FontFamily::Standard => "Standard Mono",
            FontFamily::Terminus => "Terminus Console",
            FontFamily::Modern => "Modern Coding Sans",
            FontFamily::Typewriter => "Typewriter Slab",
            FontFamily::Retro => "Retro Pixel 8-Bit",
        }
    }

    pub const fn description(&self) -> &'static str {
        match self {
            FontFamily::Standard => "Equilibrada, limpia y optimizada para lectura prolongada",
            FontFamily::Terminus => "Alto contraste, cero tachado y estetica de consola",
            FontFamily::Modern => "Curvas suaves, aperturas abiertas y codigo minimalista",
            FontFamily::Typewriter => "Serifas mecanicas marcadas de estilo academico",
            FontFamily::Retro => "Pixeles puros estilo arcade y computacion de 8 bits",
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().trim() {
            "terminus" | "console" | "terminal" => FontFamily::Terminus,
            "modern" | "sans" | "code" => FontFamily::Modern,
            "typewriter" | "slab" | "serif" => FontFamily::Typewriter,
            "retro" | "pixel" | "8bit" | "arcade" => FontFamily::Retro,
            _ => FontFamily::Standard,
        }
    }

    pub const ALL: [FontFamily; 5] = [
        FontFamily::Standard,
        FontFamily::Terminus,
        FontFamily::Modern,
        FontFamily::Typewriter,
        FontFamily::Retro,
    ];
}

/// Font stroke weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FontWeight {
    Normal,
    Semibold,
    Bold,
}

impl FontWeight {
    pub const fn as_str(&self) -> &'static str {
        match self {
            FontWeight::Normal => "normal",
            FontWeight::Semibold => "semibold",
            FontWeight::Bold => "bold",
        }
    }

    pub const fn display_name(&self) -> &'static str {
        match self {
            FontWeight::Normal => "Normal",
            FontWeight::Semibold => "Semibold",
            FontWeight::Bold => "Bold",
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().trim() {
            "semibold" | "semi" | "medium" => FontWeight::Semibold,
            "bold" | "negrita" => FontWeight::Bold,
            _ => FontWeight::Normal,
        }
    }

    pub const ALL: [FontWeight; 3] = [
        FontWeight::Normal,
        FontWeight::Semibold,
        FontWeight::Bold,
    ];
}

/// Advanced font rasterizer with subpixel spatial anti-aliasing,
/// box-drawing edge preservation, multi-family fonts, and smooth continuous scaling.
pub struct FontRasterizer;

static ASCII_AA_CACHE: OnceLock<Box<[GlyphCoverage16; 95]>> = OnceLock::new();
static SPECIAL_AA_CACHE: OnceLock<BTreeMap<char, GlyphCoverage16>> = OnceLock::new();
static UNKNOWN_AA: OnceLock<GlyphCoverage16> = OnceLock::new();

static ASCII_SUBPIXEL_CACHE: OnceLock<Box<[SubpixelCoverage16; 95]>> = OnceLock::new();
static SPECIAL_SUBPIXEL_CACHE: OnceLock<BTreeMap<char, SubpixelCoverage16>> = OnceLock::new();
static UNKNOWN_SUBPIXEL: OnceLock<SubpixelCoverage16> = OnceLock::new();

impl FontRasterizer {
    /// Determines if a character is a box-drawing symbol that requires 100% sharp connected edges.
    pub fn is_box_drawing(ch: char) -> bool {
        matches!(ch, '│' | '─' | '█' | '\u{2500}'..='\u{257F}' | '\u{2580}'..='\u{259F}')
    }

    /// Computes spatial anti-aliased coverage mask (8x16 = 128 bytes, 0..=255) from a 1-bit bitmap.
    /// Preserves 100% solid opacity (255) for all active stroke pixels to prevent broken or faint text,
    /// protects interior loops and straight edges from blur bleeding, and applies subtle subpixel smoothing
    /// on curved corners and diagonal transitions.
    pub fn compute_coverage(bitmap: &[u8; 16], is_box_drawing: bool) -> GlyphCoverage16 {
        let mut out = [0u8; 128];

        let get_bit = |x: i32, y: i32| -> bool {
            if x < 0 || x >= 8 || y < 0 || y >= 16 {
                return false;
            }
            (bitmap[y as usize] & (0x80 >> (x as usize))) != 0
        };

        for y in 0..16 {
            for x in 0..8 {
                let is_set = get_bit(x, y);

                if is_box_drawing {
                    out[(y as usize) * 8 + (x as usize)] = if is_set { 255 } else { 0 };
                    continue;
                }

                // 1. All active stroke pixels are 100% solid (255).
                // This eliminates broken stems, faded punctuation tips, and washed-out gamma text.
                if is_set {
                    out[(y as usize) * 8 + (x as usize)] = 255;
                    continue;
                }

                // 2. Neighbor analysis for background pixel smoothing:
                let top = get_bit(x, y - 1);
                let bot = get_bit(x, y + 1);
                let left = get_bit(x - 1, y);
                let right = get_bit(x + 1, y);
                let tl = get_bit(x - 1, y - 1);
                let tr = get_bit(x + 1, y - 1);
                let bl = get_bit(x - 1, y + 1);
                let br = get_bit(x + 1, y + 1);

                // Rule A: Counter / slot protection - NEVER fill interior loops of 'e', 'a', 'o', '0', etc.
                if (top && bot) || (left && right) {
                    out[(y as usize) * 8 + (x as usize)] = 0;
                    continue;
                }

                // Rule B: Straight edge protection - NEVER bleed into straight parallel background pixels.
                if (left && tl && bl) || (right && tr && br) || (top && tl && tr) || (bot && bl && br) {
                    out[(y as usize) * 8 + (x as usize)] = 0;
                    continue;
                }

                // Rule C: Outer and inner corner curves / diagonal steps
                if (bot && right) || (bot && left) || (top && right) || (top && left) {
                    out[(y as usize) * 8 + (x as usize)] = 60;
                    continue;
                }

                // Rule D: Diagonal transitions
                if (top && (bl || br)) || (bot && (tl || tr)) || (left && (tr || br)) || (right && (tl || bl)) {
                    out[(y as usize) * 8 + (x as usize)] = 45;
                    continue;
                }

                // Rule E: Diagonal bridge
                if (tl && br) || (tr && bl) {
                    out[(y as usize) * 8 + (x as usize)] = 45;
                    continue;
                }

                out[(y as usize) * 8 + (x as usize)] = 0;
            }
        }

        out
    }

    /// Retrieves precomputed or cached anti-aliased glyph coverage (O(1)).
    pub fn get_glyph_coverage(ch: char) -> &'static GlyphCoverage16 {
        let ascii = ch as usize;
        if (32..=126).contains(&ascii) {
            let cache = ASCII_AA_CACHE.get_or_init(|| {
                let mut table = Vec::with_capacity(95);
                for idx in 0..95 {
                    let bmp = &ASCII_GLYPHS[idx];
                    table.push(Self::compute_coverage(bmp, false));
                }
                table.into_boxed_slice().try_into().unwrap()
            });
            &cache[ascii - 32]
        } else {
            let special_map = SPECIAL_AA_CACHE.get_or_init(|| {
                let mut map = BTreeMap::new();
                let chars = [
                    '│', '─', '█', 'á', 'é', 'í', 'ó', 'ú', 'Á', 'É', 'Í', 'Ó', 'Ú',
                    'ñ', 'Ñ', '¡', '¿', '×', 'ü', 'Ü', '•', '©', '▼', '▶', '↺',
                ];
                for &c in &chars {
                    let is_box = Self::is_box_drawing(c);
                    let bmp = get_glyph(c);
                    map.insert(c, Self::compute_coverage(bmp, is_box));
                }
                map
            });

            if let Some(cov) = special_map.get(&ch) {
                cov
            } else {
                UNKNOWN_AA.get_or_init(|| {
                    Self::compute_coverage(&GLYPH_UNKNOWN, false)
                })
            }
        }
    }

    /// Computes coverage mask taking into account font family (e.g. sharp retro) and weight (dilation/bolding).
    pub fn compute_coverage_ex(bitmap: &[u8; 16], is_box_drawing: bool, family: FontFamily, weight: FontWeight) -> GlyphCoverage16 {
        let mut dilated = [0u8; 16];
        let effective_bitmap = match weight {
            FontWeight::Bold => {
                for y in 0..16 {
                    dilated[y] = bitmap[y] | (bitmap[y] >> 1);
                }
                &dilated
            }
            _ => bitmap,
        };

        if family == FontFamily::Retro {
            let mut out = [0u8; 128];
            for y in 0..16 {
                for x in 0..8 {
                    let is_set = (effective_bitmap[y] & (0x80 >> x)) != 0;
                    out[y * 8 + x] = if is_set { 255 } else { 0 };
                }
            }
            return out;
        }

        let mut out = Self::compute_coverage(effective_bitmap, is_box_drawing);
        if weight == FontWeight::Semibold {
            for val in out.iter_mut() {
                if *val > 0 && *val < 255 {
                    *val = (*val as u16 + 35).min(255) as u8;
                }
            }
        }
        out
    }

    /// Retrieves glyph coverage for a specific font family and weight.
    pub fn get_glyph_coverage_for(ch: char, family: FontFamily, weight: FontWeight) -> GlyphCoverage16 {
        if family == FontFamily::Standard && weight == FontWeight::Normal {
            return *Self::get_glyph_coverage(ch);
        }
        let is_box = Self::is_box_drawing(ch);
        let bmp = get_family_glyph(ch, family);
        Self::compute_coverage_ex(bmp, is_box, family, weight)
    }

    /// Rasterizes glyph sized to arbitrary target width and height `(target_w x target_h)`
    /// applying analytical continuous area box-filter integration.
    /// Preserves 100% full stem weight, prevents alpha fading on fractional zoom levels,
    /// and generates mathematically exact subpixel fractional coverage on diagonal/curved edges.
    pub fn rasterize_glyph_sized(
        ch: char,
        family: FontFamily,
        target_w: usize,
        target_h: usize,
        weight: FontWeight,
    ) -> (usize, usize, Vec<u8>) {
        let target_w = target_w.max(1);
        let target_h = target_h.max(1);

        if target_w == FONT_WIDTH && target_h == FONT_HEIGHT && family == FontFamily::Standard && weight == FontWeight::Normal {
            return (FONT_WIDTH, FONT_HEIGHT, Self::get_glyph_coverage(ch).to_vec());
        }

        let base = Self::get_glyph_coverage_for(ch, family, weight);
        if target_w == FONT_WIDTH && target_h == FONT_HEIGHT {
            return (FONT_WIDTH, FONT_HEIGHT, base.to_vec());
        }

        let mut out = vec![0u8; target_w * target_h];
        let sx_scale = 8.0 / (target_w as f64);
        let sy_scale = 16.0 / (target_h as f64);
        let cell_area = sx_scale * sy_scale;

        for y in 0..target_h {
            let v0 = (y as f64) * sy_scale;
            let v1 = ((y + 1) as f64) * sy_scale;
            let sy_start = (v0.floor() as usize).min(15);
            let sy_end = (((v1 - 1e-6).floor() as usize)).min(15);

            for x in 0..target_w {
                let u0 = (x as f64) * sx_scale;
                let u1 = ((x + 1) as f64) * sx_scale;
                let sx_start = (u0.floor() as usize).min(7);
                let sx_end = (((u1 - 1e-6).floor() as usize)).min(7);

                let mut accum = 0.0;
                for sy in sy_start..=sy_end {
                    let y_overlap = (v1.min(sy as f64 + 1.0) - v0.max(sy as f64)).max(0.0);
                    for sx in sx_start..=sx_end {
                        let x_overlap = (u1.min(sx as f64 + 1.0) - u0.max(sx as f64)).max(0.0);
                        let overlap_area = x_overlap * y_overlap;
                        accum += (overlap_area / cell_area) * (base[sy * 8 + sx] as f64);
                    }
                }

                out[y * target_w + x] = accum.round().clamp(0.0, 255.0) as u8;
            }
        }

        (target_w, target_h, out)
    }

    /// Rasterizes glyph scaled to `(8 * scale) x (16 * scale)` with continuous bilinear sampling.
    pub fn rasterize_glyph_scaled(ch: char, scale: usize) -> (usize, usize, Vec<u8>) {
        let scale = scale.max(1);
        Self::rasterize_glyph_sized(ch, FontFamily::Standard, FONT_WIDTH * scale, FONT_HEIGHT * scale, FontWeight::Normal)
    }

    /// Computes spatial LCD RGB subpixel anti-aliased coverage mask (8x16 = 128 pixels, 3 subpixels [R, G, B] per pixel).
    /// Triples horizontal spatial sampling to 24 subpixels and applies a [1/4, 2/4, 1/4] FIR filter across subpixels
    /// to eliminate color fringing while preserving 3x horizontal edge sharpness.
    pub fn compute_subpixel_coverage(bitmap: &[u8; 16], is_box_drawing: bool) -> SubpixelCoverage16 {
        let mut out = [[0u8; 3]; 128];

        if is_box_drawing {
            let cov = Self::compute_coverage(bitmap, true);
            for i in 0..128 {
                let v = cov[i];
                out[i] = [v, v, v];
            }
            return out;
        }

        let get_bit = |x: i32, y: i32| -> bool {
            if x < 0 || x >= 8 || y < 0 || y >= 16 {
                return false;
            }
            (bitmap[y as usize] & (0x80 >> (x as usize))) != 0
        };

        // 24 subpixel columns horizontally across 8 pixel columns (3 subpixels per pixel)
        let mut raw_sub = [0u8; 24 * 16];

        for y in 0..16 {
            for s in 0..24 {
                let x_src = s / 3;
                let is_set = get_bit(x_src as i32, y as i32);
                raw_sub[(y * 24 + s) as usize] = if is_set { 255 } else { 0 };
            }
        }

        // Apply FIR filter [1/4, 2/4, 1/4] along each horizontal row
        for y in 0..16 {
            let row_offset = y * 24;
            let mut filtered = [0u8; 24];
            for s in 0..24 {
                let left = if s > 0 { raw_sub[row_offset + s - 1] as u16 } else { 0 };
                let center = raw_sub[row_offset + s] as u16;
                let right = if s + 1 < 24 { raw_sub[row_offset + s + 1] as u16 } else { 0 };
                filtered[s] = ((left + center * 2 + right + 2) / 4).min(255) as u8;
            }

            for x in 0..8 {
                let r = filtered[x * 3 + 0];
                let g = filtered[x * 3 + 1];
                let b = filtered[x * 3 + 2];
                out[y * 8 + x] = [r, g, b];
            }
        }

        out
    }

    /// Retrieves precomputed or cached subpixel LCD glyph coverage (O(1)).
    pub fn get_glyph_subpixel(ch: char) -> &'static SubpixelCoverage16 {
        let ascii = ch as usize;
        if (32..=126).contains(&ascii) {
            let cache = ASCII_SUBPIXEL_CACHE.get_or_init(|| {
                let mut table = Vec::with_capacity(95);
                for idx in 0..95 {
                    let bmp = &ASCII_GLYPHS[idx];
                    table.push(Self::compute_subpixel_coverage(bmp, false));
                }
                table.into_boxed_slice().try_into().unwrap()
            });
            &cache[ascii - 32]
        } else {
            let special_map = SPECIAL_SUBPIXEL_CACHE.get_or_init(|| {
                let mut map = BTreeMap::new();
                let chars = [
                    '│', '─', '█', 'á', 'é', 'í', 'ó', 'ú', 'Á', 'É', 'Í', 'Ó', 'Ú',
                    'ñ', 'Ñ', '¡', '¿', '×', 'ü', 'Ü', '•', '©', '▼', '▶', '↺',
                ];
                for &c in &chars {
                    let is_box = Self::is_box_drawing(c);
                    let bmp = get_glyph(c);
                    map.insert(c, Self::compute_subpixel_coverage(bmp, is_box));
                }
                map
            });

            if let Some(cov) = special_map.get(&ch) {
                cov
            } else {
                UNKNOWN_SUBPIXEL.get_or_init(|| {
                    Self::compute_subpixel_coverage(&GLYPH_UNKNOWN, false)
                })
            }
        }
    }

    /// Rasterizes glyph sized to arbitrary target width and height `(target_w x target_h)`
    /// with 3x horizontal LCD subpixel resolution and [1/4, 2/4, 1/4] FIR color-fringing filter.
    pub fn rasterize_glyph_subpixel(
        ch: char,
        family: FontFamily,
        target_w: usize,
        target_h: usize,
        weight: FontWeight,
    ) -> (usize, usize, Vec<[u8; 3]>) {
        let target_w = target_w.max(1);
        let target_h = target_h.max(1);

        if target_w == FONT_WIDTH && target_h == FONT_HEIGHT && family == FontFamily::Standard && weight == FontWeight::Normal {
            return (FONT_WIDTH, FONT_HEIGHT, Self::get_glyph_subpixel(ch).to_vec());
        }

        if Self::is_box_drawing(ch) || family == FontFamily::Retro {
            let (_, _, cov) = Self::rasterize_glyph_sized(ch, family, target_w, target_h, weight);
            let sub: Vec<[u8; 3]> = cov.into_iter().map(|v| [v, v, v]).collect();
            return (target_w, target_h, sub);
        }

        let bmp = get_family_glyph(ch, family);
        let mut dilated = [0u8; 16];
        let effective_bitmap = match weight {
            FontWeight::Bold => {
                for y in 0..16 {
                    dilated[y] = bmp[y] | (bmp[y] >> 1);
                }
                &dilated
            }
            _ => bmp,
        };

        let sub_w = target_w * 3;
        let sx_scale = 8.0 / (sub_w as f64);
        let sy_scale = 16.0 / (target_h as f64);
        let cell_area = sx_scale * sy_scale;

        let mut raw_sub = vec![0u8; sub_w * target_h];

        for y in 0..target_h {
            let v0 = (y as f64) * sy_scale;
            let v1 = ((y + 1) as f64) * sy_scale;
            let sy_start = (v0.floor() as usize).min(15);
            let sy_end = (((v1 - 1e-6).floor() as usize)).min(15);

            for s in 0..sub_w {
                let u0 = (s as f64) * sx_scale;
                let u1 = ((s + 1) as f64) * sx_scale;
                let sx_start = (u0.floor() as usize).min(7);
                let sx_end = (((u1 - 1e-6).floor() as usize)).min(7);

                let mut accum = 0.0;
                for sy in sy_start..=sy_end {
                    let y_overlap = (v1.min(sy as f64 + 1.0) - v0.max(sy as f64)).max(0.0);
                    for sx in sx_start..=sx_end {
                        let x_overlap = (u1.min(sx as f64 + 1.0) - u0.max(sx as f64)).max(0.0);
                        let is_set = (effective_bitmap[sy] & (0x80 >> sx)) != 0;
                        if is_set {
                            accum += (x_overlap * y_overlap / cell_area) * 255.0;
                        }
                    }
                }

                let mut val = accum.round().clamp(0.0, 255.0) as u8;
                if weight == FontWeight::Semibold && val > 0 && val < 255 {
                    val = (val as u16 + 35).min(255) as u8;
                }
                raw_sub[y * sub_w + s] = val;
            }
        }

        // Apply FIR filter [1/4, 2/4, 1/4] horizontally across subpixel columns
        let mut out = Vec::with_capacity(target_w * target_h);

        for y in 0..target_h {
            let row_offset = y * sub_w;
            let mut filtered = vec![0u8; sub_w];
            for s in 0..sub_w {
                let left = if s > 0 { raw_sub[row_offset + s - 1] as u16 } else { 0 };
                let center = raw_sub[row_offset + s] as u16;
                let right = if s + 1 < sub_w { raw_sub[row_offset + s + 1] as u16 } else { 0 };
                filtered[s] = ((left + center * 2 + right + 2) / 4).min(255) as u8;
            }

            for x in 0..target_w {
                let r = filtered[x * 3 + 0];
                let g = filtered[x * 3 + 1];
                let b = filtered[x * 3 + 2];
                out.push([r, g, b]);
            }
        }

        (target_w, target_h, out)
    }

    /// Rasterizes glyph scaled to `(8 * scale) x (16 * scale)` with 3x horizontal LCD subpixel resolution.
    pub fn rasterize_glyph_subpixel_scaled(ch: char, scale: usize) -> (usize, usize, Vec<[u8; 3]>) {
        let scale = scale.max(1);
        Self::rasterize_glyph_subpixel(ch, FontFamily::Standard, FONT_WIDTH * scale, FONT_HEIGHT * scale, FontWeight::Normal)
    }
}

/// Retorna el mapa de bits crudo de 8x16 según la familia de fuente especificada.
pub fn get_family_glyph(ch: char, family: FontFamily) -> &'static [u8; 16] {
    match family {
        FontFamily::Standard => get_glyph(ch),
        FontFamily::Terminus => match ch {
            '0' => &TERMINUS_0,
            '1' => &TERMINUS_1,
            'l' => &TERMINUS_L,
            'I' => &TERMINUS_CAP_I,
            'Z' => &TERMINUS_CAP_Z,
            'z' => &TERMINUS_Z,
            _ => get_glyph(ch),
        },
        FontFamily::Modern => match ch {
            'a' => &MODERN_A,
            'g' => &MODERN_G,
            'f' => &MODERN_F,
            't' => &MODERN_T,
            '0' => &MODERN_0,
            'Q' => &MODERN_CAP_Q,
            _ => get_glyph(ch),
        },
        FontFamily::Typewriter => match ch {
            'A' => &TYPEWRITER_CAP_A,
            'H' => &TYPEWRITER_CAP_H,
            'M' => &TYPEWRITER_CAP_M,
            'i' => &TYPEWRITER_I,
            'l' => &TYPEWRITER_L,
            'r' => &TYPEWRITER_R,
            _ => get_glyph(ch),
        },
        FontFamily::Retro => match ch {
            '0' => &RETRO_0,
            'C' => &RETRO_CAP_C,
            'E' => &RETRO_CAP_E,
            'o' => &RETRO_O,
            's' => &RETRO_S,
            _ => get_glyph(ch),
        },
    }
}

// Glifos específicos para Terminus Console
const TERMINUS_0: [u8; 16] = [
    0x00, 0x00, 0x38, 0x6C, 0xC6, 0xCE, 0xD6, 0xE6, 0xC6, 0xC6, 0x6C, 0x38, 0x00, 0x00, 0x00, 0x00,
];
const TERMINUS_1: [u8; 16] = [
    0x00, 0x00, 0x18, 0x38, 0x78, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00, 0x00, 0x00, 0x00,
];
const TERMINUS_L: [u8; 16] = [
    0x00, 0x00, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x1E, 0x00, 0x00, 0x00, 0x00,
];
const TERMINUS_CAP_I: [u8; 16] = [
    0x00, 0x00, 0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00, 0x00, 0x00, 0x00,
];
const TERMINUS_CAP_Z: [u8; 16] = [
    0x00, 0x00, 0x7E, 0x06, 0x0C, 0x18, 0x3C, 0x30, 0x60, 0xC0, 0xC6, 0x7E, 0x00, 0x00, 0x00, 0x00,
];
const TERMINUS_Z: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x0C, 0x18, 0x3C, 0x30, 0x60, 0x7E, 0x00, 0x00, 0x00, 0x00,
];

// Glifos específicos para Modern Coding Sans
const MODERN_A: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x3C, 0x06, 0x3E, 0x66, 0x66, 0x66, 0x3F, 0x00, 0x00, 0x00, 0x00,
];
const MODERN_G: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x06, 0x66, 0x66, 0x3C, 0x00, 0x00,
];
const MODERN_F: [u8; 16] = [
    0x00, 0x00, 0x1C, 0x30, 0x30, 0x7E, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x00, 0x00, 0x00, 0x00,
];
const MODERN_T: [u8; 16] = [
    0x00, 0x00, 0x20, 0x20, 0x20, 0x7C, 0x20, 0x20, 0x20, 0x20, 0x22, 0x1C, 0x00, 0x00, 0x00, 0x00,
];
const MODERN_0: [u8; 16] = [
    0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x7E, 0x7E, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00,
];
const MODERN_CAP_Q: [u8; 16] = [
    0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x0E, 0x07, 0x00, 0x00, 0x00,
];

// Glifos específicos para Typewriter Slab Mono
const TYPEWRITER_CAP_A: [u8; 16] = [
    0x00, 0x00, 0x18, 0x3C, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x66, 0xE7, 0x00, 0x00, 0x00, 0x00,
];
const TYPEWRITER_CAP_H: [u8; 16] = [
    0x00, 0x00, 0xEE, 0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x66, 0xEE, 0x00, 0x00, 0x00, 0x00,
];
const TYPEWRITER_CAP_M: [u8; 16] = [
    0x00, 0x00, 0xEF, 0x77, 0x7B, 0x6B, 0x63, 0x63, 0x63, 0x63, 0x63, 0xF7, 0x00, 0x00, 0x00, 0x00,
];
const TYPEWRITER_I: [u8; 16] = [
    0x00, 0x00, 0x18, 0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00, 0x00, 0x00, 0x00,
];
const TYPEWRITER_L: [u8; 16] = [
    0x00, 0x00, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00, 0x00, 0x00, 0x00,
];
const TYPEWRITER_R: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x76, 0x3C, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00,
];

// Glifos específicos para Retro Pixel 8-Bit
const RETRO_0: [u8; 16] = [
    0x00, 0x00, 0x7E, 0x42, 0x42, 0x5A, 0x5A, 0x5A, 0x42, 0x42, 0x42, 0x7E, 0x00, 0x00, 0x00, 0x00,
];
const RETRO_CAP_C: [u8; 16] = [
    0x00, 0x00, 0x7E, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x7E, 0x00, 0x00, 0x00, 0x00,
];
const RETRO_CAP_E: [u8; 16] = [
    0x00, 0x00, 0x7E, 0x40, 0x40, 0x40, 0x7C, 0x40, 0x40, 0x40, 0x40, 0x7E, 0x00, 0x00, 0x00, 0x00,
];
const RETRO_O: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x3C, 0x24, 0x24, 0x24, 0x24, 0x24, 0x3C, 0x00, 0x00, 0x00, 0x00,
];
const RETRO_S: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x3C, 0x20, 0x20, 0x3C, 0x04, 0x04, 0x38, 0x00, 0x00, 0x00, 0x00,
];

/// Retorna el mapa de bits crudo de 8x16 (16 bytes, 1 byte por fila) para un caracter dado.
pub fn get_glyph(ch: char) -> &'static [u8; 16] {
    let ascii = ch as usize;
    if (32..=126).contains(&ascii) {
        &ASCII_GLYPHS[ascii - 32]
    } else {
        match ch {
            '│' => &GLYPH_VLINE,
            '─' => &GLYPH_HLINE,
            '█' => &GLYPH_SOLID_BLOCK,
            'á' => &GLYPH_A_ACCENT,
            'é' => &GLYPH_E_ACCENT,
            'í' => &GLYPH_I_ACCENT,
            'ó' => &GLYPH_O_ACCENT,
            'ú' => &GLYPH_U_ACCENT,
            'Á' => &GLYPH_CAP_A_ACCENT,
            'É' => &GLYPH_CAP_E_ACCENT,
            'Í' => &GLYPH_CAP_I_ACCENT,
            'Ó' => &GLYPH_CAP_O_ACCENT,
            'Ú' => &GLYPH_CAP_U_ACCENT,
            'ñ' => &GLYPH_N_TILDE,
            'Ñ' => &GLYPH_CAP_N_TILDE,
            '¡' => &GLYPH_INV_EXCLAMATION,
            '¿' => &GLYPH_INV_QUESTION,
            '×' => &GLYPH_CROSS,
            'ü' => &GLYPH_U_UMLAUT,
            'Ü' => &GLYPH_CAP_U_UMLAUT,
            '•' => &GLYPH_BULLET,
            '©' => &GLYPH_COPYRIGHT,
            '▼' => &GLYPH_DOWN_ARROW,
            '▶' => &GLYPH_RIGHT_ARROW,
            '↺' => &GLYPH_REFRESH,
            _ => &GLYPH_UNKNOWN,
        }
    }
}

// Glifos especiales e internacionales
const GLYPH_VLINE: [u8; 16] = [
    0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18,
    0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18,
];

const GLYPH_HLINE: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_SOLID_BLOCK: [u8; 16] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

const GLYPH_CROSS: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0xC6, 0x6C, 0x38,
    0x38, 0x6C, 0xC6, 0x00, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_BULLET: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x3C, 0x7E,
    0x7E, 0x3C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_COPYRIGHT: [u8; 16] = [
    0x00, 0x00, 0x7C, 0x82, 0xBA, 0xA2, 0xA2, 0xA2,
    0xBA, 0x82, 0x7C, 0x00, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_DOWN_ARROW: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFE, 0x7C,
    0x38, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_RIGHT_ARROW: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x10, 0x30, 0x70, 0xF0,
    0x70, 0x30, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_REFRESH: [u8; 16] = [
    0x00, 0x00, 0x00, 0x00, 0x3C, 0x66, 0xCF, 0xDF,
    0xDB, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// Spanish á, é, í, ó, ú, ñ
const GLYPH_A_ACCENT: [u8; 16] = [
    0x00, 0x18, 0x30, 0x60, 0x00, 0x78, 0x0C, 0x7C,
    0xCC, 0xCC, 0xCC, 0x76, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_E_ACCENT: [u8; 16] = [
    0x00, 0x0C, 0x18, 0x30, 0x00, 0x7C, 0xC6, 0xFE,
    0xC0, 0xC0, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_I_ACCENT: [u8; 16] = [
    0x00, 0x0C, 0x18, 0x30, 0x00, 0x38, 0x18, 0x18,
    0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_O_ACCENT: [u8; 16] = [
    0x00, 0x18, 0x30, 0x60, 0x00, 0x7C, 0xC6, 0xC6,
    0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_U_ACCENT: [u8; 16] = [
    0x00, 0x18, 0x30, 0x60, 0x00, 0xCC, 0xCC, 0xCC,
    0xCC, 0xCC, 0xCC, 0x76, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_N_TILDE: [u8; 16] = [
    0x00, 0x00, 0x76, 0xDC, 0x00, 0xDC, 0x66, 0x66,
    0x66, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00,
];

// Spanish Á, É, Í, Ó, Ú, Ñ
const GLYPH_CAP_A_ACCENT: [u8; 16] = [
    0x30, 0x60, 0x10, 0x38, 0x6C, 0xC6, 0xC6, 0xFE,
    0xC6, 0xC6, 0xC6, 0xC6, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_CAP_E_ACCENT: [u8; 16] = [
    0x0C, 0x18, 0x00, 0xFE, 0x66, 0x62, 0x68, 0x78,
    0x68, 0x62, 0x66, 0xFE, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_CAP_I_ACCENT: [u8; 16] = [
    0x0C, 0x18, 0x00, 0x3C, 0x18, 0x18, 0x18, 0x18,
    0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_CAP_O_ACCENT: [u8; 16] = [
    0x18, 0x30, 0x00, 0x38, 0x6C, 0xC6, 0xC6, 0xC6,
    0xC6, 0xC6, 0x6C, 0x38, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_CAP_U_ACCENT: [u8; 16] = [
    0x18, 0x30, 0x00, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6,
    0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_CAP_N_TILDE: [u8; 16] = [
    0x76, 0xDC, 0x00, 0xC6, 0xE6, 0xF6, 0xFE, 0xDE,
    0xCE, 0xC6, 0xC6, 0xC6, 0x00, 0x00, 0x00, 0x00,
];

// Inverted punctuation ¡, ¿
const GLYPH_INV_EXCLAMATION: [u8; 16] = [
    0x00, 0x00, 0x18, 0x18, 0x00, 0x18, 0x18, 0x18,
    0x3C, 0x3C, 0x3C, 0x18, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_INV_QUESTION: [u8; 16] = [
    0x00, 0x00, 0x18, 0x18, 0x00, 0x18, 0x18, 0x30,
    0x60, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_U_UMLAUT: [u8; 16] = [
    0x00, 0x00, 0xC6, 0x00, 0x00, 0xCC, 0xCC, 0xCC,
    0xCC, 0xCC, 0xCC, 0x76, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_CAP_U_UMLAUT: [u8; 16] = [
    0x00, 0xC6, 0x00, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6,
    0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00,
];

const GLYPH_UNKNOWN: [u8; 16] = [
    0x00, 0x00, 0xFE, 0x82, 0x82, 0x82, 0x82, 0x82,
    0x82, 0x82, 0x82, 0xFE, 0x00, 0x00, 0x00, 0x00,
];

/// Tabla completa de caracteres imprimibles ASCII (32 a 126).
static ASCII_GLYPHS: [[u8; 16]; 95] = [
    // 32: ' '
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    // 33: '!'
    [0x00, 0x00, 0x18, 0x3C, 0x3C, 0x3C, 0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00],
    // 34: '\"'
    [0x00, 0x66, 0x66, 0x66, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    // 35: '#'
    [0x00, 0x00, 0x00, 0x6C, 0x6C, 0xFE, 0x6C, 0x6C, 0x6C, 0xFE, 0x6C, 0x6C, 0x00, 0x00, 0x00, 0x00],
    // 36: '$'
    [0x18, 0x18, 0x7C, 0xC6, 0xC2, 0xC0, 0x7C, 0x06, 0x06, 0x86, 0xC6, 0x7C, 0x18, 0x18, 0x00, 0x00],
    // 37: '%'
    [0x00, 0x00, 0x00, 0x00, 0xC2, 0xC6, 0x0C, 0x18, 0x30, 0x60, 0xC6, 0x86, 0x00, 0x00, 0x00, 0x00],
    // 38: '&'
    [0x00, 0x00, 0x38, 0x6C, 0x6C, 0x38, 0x76, 0xDC, 0xCC, 0xCC, 0xCC, 0x76, 0x00, 0x00, 0x00, 0x00],
    // 39: '''
    [0x00, 0x30, 0x30, 0x30, 0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    // 40: '('
    [0x00, 0x00, 0x0C, 0x18, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x18, 0x0C, 0x00, 0x00, 0x00, 0x00],
    // 41: ')'
    [0x00, 0x00, 0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x18, 0x30, 0x00, 0x00, 0x00, 0x00],
    // 42: '*'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x66, 0x3C, 0xFF, 0x3C, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    // 43: '+'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x7E, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    // 44: ','
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x18, 0x30, 0x00, 0x00, 0x00],
    // 45: '-'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFE, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    // 46: '.'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00],
    // 47: '/'
    [0x00, 0x00, 0x00, 0x00, 0x02, 0x06, 0x0C, 0x18, 0x30, 0x60, 0xC0, 0x80, 0x00, 0x00, 0x00, 0x00],
    // 48: '0'
    [0x00, 0x00, 0x38, 0x6C, 0xC6, 0xC6, 0xD6, 0xD6, 0xC6, 0xC6, 0x6C, 0x38, 0x00, 0x00, 0x00, 0x00],
    // 49: '1'
    [0x00, 0x00, 0x18, 0x38, 0x78, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00, 0x00, 0x00, 0x00],
    // 50: '2'
    [0x00, 0x00, 0x7C, 0xC6, 0x06, 0x0C, 0x18, 0x30, 0x60, 0xC0, 0xC6, 0xFE, 0x00, 0x00, 0x00, 0x00],
    // 51: '3'
    [0x00, 0x00, 0x7C, 0xC6, 0x06, 0x06, 0x3C, 0x06, 0x06, 0x06, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
    // 52: '4'
    [0x00, 0x00, 0x0C, 0x1C, 0x3C, 0x6C, 0xCC, 0xFE, 0x0C, 0x0C, 0x0C, 0x1E, 0x00, 0x00, 0x00, 0x00],
    // 53: '5'
    [0x00, 0x00, 0xFE, 0xC0, 0xC0, 0xC0, 0xFC, 0x06, 0x06, 0x06, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
    // 54: '6'
    [0x00, 0x00, 0x38, 0x60, 0xC0, 0xC0, 0xFC, 0xC6, 0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
    // 55: '7'
    [0x00, 0x00, 0xFE, 0xC6, 0x06, 0x06, 0x0C, 0x18, 0x30, 0x30, 0x30, 0x30, 0x00, 0x00, 0x00, 0x00],
    // 56: '8'
    [0x00, 0x00, 0x7C, 0xC6, 0xC6, 0xC6, 0x7C, 0xC6, 0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
    // 57: '9'
    [0x00, 0x00, 0x7C, 0xC6, 0xC6, 0xC6, 0x7E, 0x06, 0x06, 0x06, 0x0C, 0x78, 0x00, 0x00, 0x00, 0x00],
    // 58: ':'
    [0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00],
    // 59: ';'
    [0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30, 0x00, 0x00, 0x00, 0x00],
    // 60: '<'
    [0x00, 0x00, 0x00, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x30, 0x18, 0x0C, 0x06, 0x00, 0x00, 0x00, 0x00],
    // 61: '='
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    // 62: '>'
    [0x00, 0x00, 0x00, 0x60, 0x30, 0x18, 0x0C, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x00, 0x00, 0x00, 0x00],
    // 63: '?'
    [0x00, 0x00, 0x7C, 0xC6, 0xC6, 0x0C, 0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00],
    // 64: '@'
    [0x00, 0x00, 0x00, 0x7C, 0xC6, 0xC6, 0xDE, 0xDE, 0xDE, 0xDC, 0xC0, 0x7C, 0x00, 0x00, 0x00, 0x00],
    // 65: 'A'
    [0x00, 0x00, 0x10, 0x38, 0x6C, 0xC6, 0xC6, 0xFE, 0xC6, 0xC6, 0xC6, 0xC6, 0x00, 0x00, 0x00, 0x00],
    // 66: 'B'
    [0x00, 0x00, 0xFC, 0x66, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x66, 0x66, 0xFC, 0x00, 0x00, 0x00, 0x00],
    // 67: 'C'
    [0x00, 0x00, 0x3C, 0x66, 0xC2, 0xC0, 0xC0, 0xC0, 0xC0, 0xC2, 0x66, 0x3C, 0x00, 0x00, 0x00, 0x00],
    // 68: 'D'
    [0x00, 0x00, 0xF8, 0x6C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x6C, 0xF8, 0x00, 0x00, 0x00, 0x00],
    // 69: 'E'
    [0x00, 0x00, 0xFE, 0x66, 0x62, 0x68, 0x78, 0x68, 0x60, 0x62, 0x66, 0xFE, 0x00, 0x00, 0x00, 0x00],
    // 70: 'F'
    [0x00, 0x00, 0xFE, 0x66, 0x62, 0x68, 0x78, 0x68, 0x60, 0x60, 0x60, 0xF0, 0x00, 0x00, 0x00, 0x00],
    // 71: 'G'
    [0x00, 0x00, 0x3C, 0x66, 0xC2, 0xC0, 0xC0, 0xDE, 0xC6, 0xC6, 0x66, 0x3A, 0x00, 0x00, 0x00, 0x00],
    // 72: 'H'
    [0x00, 0x00, 0xC6, 0xC6, 0xC6, 0xC6, 0xFE, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0x00, 0x00, 0x00, 0x00],
    // 73: 'I'
    [0x00, 0x00, 0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00],
    // 74: 'J'
    [0x00, 0x00, 0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0xCC, 0xCC, 0xCC, 0x78, 0x00, 0x00, 0x00, 0x00],
    // 75: 'K'
    [0x00, 0x00, 0xE6, 0x66, 0x66, 0x6C, 0x78, 0x78, 0x6C, 0x66, 0x66, 0xE6, 0x00, 0x00, 0x00, 0x00],
    // 76: 'L'
    [0x00, 0x00, 0xF0, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x62, 0x66, 0xFE, 0x00, 0x00, 0x00, 0x00],
    // 77: 'M'
    [0x00, 0x00, 0xC6, 0xEE, 0xFE, 0xFE, 0xD6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0x00, 0x00, 0x00, 0x00],
    // 78: 'N'
    [0x00, 0x00, 0xC6, 0xE6, 0xF6, 0xFE, 0xDE, 0xCE, 0xC6, 0xC6, 0xC6, 0xC6, 0x00, 0x00, 0x00, 0x00],
    // 79: 'O'
    [0x00, 0x00, 0x7C, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
    // 80: 'P'
    [0x00, 0x00, 0xFC, 0x66, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x60, 0xF0, 0x00, 0x00, 0x00, 0x00],
    // 81: 'Q'
    [0x00, 0x00, 0x7C, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xD6, 0xDE, 0x7C, 0x0C, 0x0E, 0x00, 0x00],
    // 82: 'R'
    [0x00, 0x00, 0xFC, 0x66, 0x66, 0x66, 0x7C, 0x6C, 0x66, 0x66, 0x66, 0xE6, 0x00, 0x00, 0x00, 0x00],
    // 83: 'S'
    [0x00, 0x00, 0x7C, 0xC6, 0xC6, 0x60, 0x38, 0x0C, 0x06, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
    // 84: 'T'
    [0x00, 0x00, 0x7E, 0x7E, 0x5A, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00],
    // 85: 'U'
    [0x00, 0x00, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
    // 86: 'V'
    [0x00, 0x00, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0x6C, 0x38, 0x10, 0x00, 0x00, 0x00, 0x00],
    // 87: 'W'
    [0x00, 0x00, 0xC6, 0xC6, 0xC6, 0xC6, 0xD6, 0xD6, 0xD6, 0xFE, 0xEE, 0x6C, 0x00, 0x00, 0x00, 0x00],
    // 88: 'X'
    [0x00, 0x00, 0xC6, 0xC6, 0x6C, 0x7C, 0x38, 0x38, 0x7C, 0x6C, 0xC6, 0xC6, 0x00, 0x00, 0x00, 0x00],
    // 89: 'Y'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00],
    // 90: 'Z'
    [0x00, 0x00, 0xFE, 0xC6, 0x86, 0x0C, 0x18, 0x30, 0x60, 0xC2, 0xC6, 0xFE, 0x00, 0x00, 0x00, 0x00],
    // 91: '['
    [0x00, 0x00, 0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00, 0x00, 0x00, 0x00],
    // 92: '\\'
    [0x00, 0x00, 0x00, 0x80, 0xC0, 0xE0, 0x70, 0x38, 0x1C, 0x0E, 0x06, 0x02, 0x00, 0x00, 0x00, 0x00],
    // 93: ']'
    [0x00, 0x00, 0x3C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x3C, 0x00, 0x00, 0x00, 0x00],
    // 94: '^'
    [0x10, 0x38, 0x6C, 0xC6, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    // 95: '_'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x00],
    // 96: '`'
    [0x00, 0x30, 0x18, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    // 97: 'a'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x78, 0x0C, 0x7C, 0xCC, 0xCC, 0xCC, 0x76, 0x00, 0x00, 0x00, 0x00],
    // 98: 'b'
    [0x00, 0x00, 0xE0, 0x60, 0x60, 0x78, 0x6C, 0x66, 0x66, 0x66, 0x66, 0x7C, 0x00, 0x00, 0x00, 0x00],
    // 99: 'c'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0xC6, 0xC0, 0xC0, 0xC0, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
    // 100: 'd'
    [0x00, 0x00, 0x1C, 0x0C, 0x0C, 0x3C, 0x6C, 0xCC, 0xCC, 0xCC, 0xCC, 0x76, 0x00, 0x00, 0x00, 0x00],
    // 101: 'e'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0xC6, 0xFE, 0xC0, 0xC0, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
    // 102: 'f'
    [0x00, 0x00, 0x1C, 0x36, 0x32, 0x30, 0x78, 0x30, 0x30, 0x30, 0x30, 0x78, 0x00, 0x00, 0x00, 0x00],
    // 103: 'g'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x76, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0x7C, 0x0C, 0xCC, 0x78, 0x00],
    // 104: 'h'
    [0x00, 0x00, 0xE0, 0x60, 0x60, 0x6C, 0x76, 0x66, 0x66, 0x66, 0x66, 0xE6, 0x00, 0x00, 0x00, 0x00],
    // 105: 'i'
    [0x00, 0x00, 0x18, 0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00],
    // 106: 'j'
    [0x00, 0x00, 0x06, 0x06, 0x00, 0x0E, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x66, 0x66, 0x3C, 0x00],
    // 107: 'k'
    [0x00, 0x00, 0xE0, 0x60, 0x60, 0x66, 0x6C, 0x78, 0x78, 0x6C, 0x66, 0xE6, 0x00, 0x00, 0x00, 0x00],
    // 108: 'l'
    [0x00, 0x00, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00, 0x00, 0x00],
    // 109: 'm'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0xEC, 0xFE, 0xD6, 0xD6, 0xD6, 0xD6, 0xC6, 0x00, 0x00, 0x00, 0x00],
    // 110: 'n'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0xDC, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00],
    // 111: 'o'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
    // 112: 'p'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0xDC, 0x66, 0x66, 0x66, 0x66, 0x66, 0x7C, 0x60, 0x60, 0xF0, 0x00],
    // 113: 'q'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x76, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0x7C, 0x0C, 0x0C, 0x1E, 0x00],
    // 114: 'r'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0xDC, 0x76, 0x66, 0x60, 0x60, 0x60, 0xF0, 0x00, 0x00, 0x00, 0x00],
    // 115: 's'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0xC6, 0x60, 0x38, 0x0C, 0xC6, 0x7C, 0x00, 0x00, 0x00, 0x00],
    // 116: 't'
    [0x00, 0x00, 0x10, 0x30, 0x30, 0xFC, 0x30, 0x30, 0x30, 0x30, 0x36, 0x1C, 0x00, 0x00, 0x00, 0x00],
    // 117: 'u'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0x76, 0x00, 0x00, 0x00, 0x00],
    // 118: 'v'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0x6C, 0x38, 0x00, 0x00, 0x00, 0x00],
    // 119: 'w'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0xC6, 0xC6, 0xD6, 0xD6, 0xD6, 0xFE, 0x6C, 0x00, 0x00, 0x00, 0x00],
    // 120: 'x'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0xC6, 0x6C, 0x38, 0x38, 0x38, 0x6C, 0xC6, 0x00, 0x00, 0x00, 0x00],
    // 121: 'y'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0x7E, 0x06, 0x0C, 0xF8, 0x00],
    // 122: 'z'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0xFE, 0xCC, 0x18, 0x30, 0x60, 0xC6, 0xFE, 0x00, 0x00, 0x00, 0x00],
    // 123: '{'
    [0x00, 0x00, 0x0E, 0x18, 0x18, 0x18, 0x70, 0x18, 0x18, 0x18, 0x18, 0x0E, 0x00, 0x00, 0x00, 0x00],
    // 124: '|'
    [0x00, 0x00, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00],
    // 125: '}'
    [0x00, 0x00, 0x70, 0x18, 0x18, 0x18, 0x0E, 0x18, 0x18, 0x18, 0x18, 0x70, 0x00, 0x00, 0x00, 0x00],
    // 126: '~'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x76, 0xDC, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spanish_glyphs() {
        let glyph_o = get_glyph('ó');
        assert_ne!(glyph_o, &GLYPH_UNKNOWN);

        let glyph_n = get_glyph('ñ');
        assert_ne!(glyph_n, &GLYPH_UNKNOWN);

        let glyph_inv_q = get_glyph('¿');
        assert_ne!(glyph_inv_q, &GLYPH_UNKNOWN);
    }

    #[test]
    fn test_font_rasterizer_antialiasing_curves_and_diagonals() {
        let cov_o = FontRasterizer::get_glyph_coverage('o');
        assert_eq!(cov_o.len(), 128);

        // Verify that letter 'o' has intermediate coverage values (anti-aliasing)
        let has_intermediate = cov_o.iter().any(|&val| val > 0 && val < 255);
        assert!(has_intermediate, "Letter 'o' must have anti-aliased intermediate pixel values");

        // Verify letter 'A' with diagonals has intermediate coverage
        let cov_a = FontRasterizer::get_glyph_coverage('A');
        let has_intermediate_a = cov_a.iter().any(|&val| val > 0 && val < 255);
        assert!(has_intermediate_a, "Letter 'A' must have anti-aliased intermediate pixel values");

        // Verify solid core exists
        let has_solid = cov_o.iter().any(|&val| val == 255);
        assert!(has_solid, "Letter 'o' must have solid core pixels");
    }

    #[test]
    fn test_font_rasterizer_box_drawing_edges_preserved() {
        // Vertical line character '│' must preserve 100% full opacity on connected edges
        let cov_vline = FontRasterizer::get_glyph_coverage('│');
        assert!(FontRasterizer::is_box_drawing('│'));

        // All non-zero pixels in box-drawing character must be fully opaque 255
        for &val in cov_vline.iter() {
            assert!(val == 0 || val == 255, "Box drawing characters must preserve sharp 255 connected lines");
        }
    }

    #[test]
    fn test_font_rasterizer_scaled_bilinear() {
        let (w, h, data) = FontRasterizer::rasterize_glyph_scaled('o', 2);
        assert_eq!(w, 16);
        assert_eq!(h, 32);
        assert_eq!(data.len(), 16 * 32);

        // Bilinear scaling should produce smooth transitions
        let intermediate_count = data.iter().filter(|&&val| val > 0 && val < 255).count();
        assert!(intermediate_count >= 10, "Scaled 2x glyph must have smooth bilinear gradations");
    }

    #[test]
    fn test_font_rasterizer_unicode_spanish_antialiasing() {
        let cov_accent = FontRasterizer::get_glyph_coverage('á');
        assert!(cov_accent.iter().any(|&val| val > 0 && val < 255));

        let cov_tilde = FontRasterizer::get_glyph_coverage('ñ');
        assert!(cov_tilde.iter().any(|&val| val > 0 && val < 255));
    }

    #[test]
    fn test_font_rasterizer_subpixel_lcd_cleartype() {
        let cov_sub = FontRasterizer::get_glyph_subpixel('o');
        assert_eq!(cov_sub.len(), 128);

        // Verify that subpixel rendering differentiates R, G, B components on diagonal/curve edges
        let has_subpixel_differentiation = cov_sub.iter().any(|&[r, g, b]| {
            (r != g || g != b) && (r > 0 || g > 0 || b > 0)
        });
        assert!(has_subpixel_differentiation, "Subpixel rendering must produce color-differentiated LCD components");

        // Verify solid core pixels
        let has_solid_rgb = cov_sub.iter().any(|&[r, g, b]| r == 255 && g == 255 && b == 255);
        assert!(has_solid_rgb, "Subpixel rendering must preserve solid 255 core pixels");

        // Test scaled subpixel rasterization
        let (w, h, sub_data) = FontRasterizer::rasterize_glyph_subpixel('A', FontFamily::Standard, 10, 20, FontWeight::Normal);
        assert_eq!(w, 10);
        assert_eq!(h, 20);
        assert_eq!(sub_data.len(), 200);
        let has_scaled_diff = sub_data.iter().any(|&[r, g, b]| (r != g || g != b) && (r > 0 || g > 0 || b > 0));
        assert!(has_scaled_diff, "Scaled subpixel glyph must maintain differentiated LCD components");
    }
}
