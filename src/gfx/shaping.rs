//! Font Shaping and Programming Ligature Engine.
//! 100% pure standard-library Rust with zero third-party dependencies.

use super::font::{FontRasterizer, FONT_HEIGHT, FONT_WIDTH};

/// Identifies supported programming code ligatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LigatureKind {
    /// Right arrow `->`
    Arrow,
    /// Fat right arrow `=>`
    FatArrow,
    /// Not equal `!=`
    NotEqual,
    /// Double equal `==`
    DoubleEqual,
    /// Triple equal `===`
    TripleEqual,
    /// Not double equal `!==`
    NotDoubleEqual,
    /// Less than or equal `<=`
    LessEqual,
    /// Greater than or equal `>=`
    GreaterEqual,
    /// Walrus assignment `:=`
    Walrus,
    /// Logical AND `&&`
    LogicalAnd,
    /// Logical OR `||`
    LogicalOr,
    /// Scope resolution `::`
    Scope,
    /// Range `..`
    Range,
    /// Ellipsis `...`
    Ellipsis,
    /// Block comment start `/*`
    CommentStart,
    /// Block comment end `*/`
    CommentEnd,
    /// Increment `++`
    Increment,
    /// Decrement `--`
    Decrement,
}

impl LigatureKind {
    /// Returns the character length of the ligature in source code (2 or 3).
    pub const fn char_length(&self) -> usize {
        match self {
            Self::TripleEqual | Self::NotDoubleEqual | Self::Ellipsis => 3,
            _ => 2,
        }
    }

    /// Matches a text prefix to a known programming ligature.
    pub fn match_prefix(s: &str) -> Option<(Self, usize)> {
        if s.starts_with("===") {
            Some((Self::TripleEqual, 3))
        } else if s.starts_with("!==") {
            Some((Self::NotDoubleEqual, 3))
        } else if s.starts_with("...") {
            Some((Self::Ellipsis, 3))
        } else if s.starts_with("->") {
            Some((Self::Arrow, 2))
        } else if s.starts_with("=>") {
            Some((Self::FatArrow, 2))
        } else if s.starts_with("!=") {
            Some((Self::NotEqual, 2))
        } else if s.starts_with("==") {
            Some((Self::DoubleEqual, 2))
        } else if s.starts_with("<=") {
            Some((Self::LessEqual, 2))
        } else if s.starts_with(">=") {
            Some((Self::GreaterEqual, 2))
        } else if s.starts_with(":=") {
            Some((Self::Walrus, 2))
        } else if s.starts_with("&&") {
            Some((Self::LogicalAnd, 2))
        } else if s.starts_with("||") {
            Some((Self::LogicalOr, 2))
        } else if s.starts_with("::") {
            Some((Self::Scope, 2))
        } else if s.starts_with("..") {
            Some((Self::Range, 2))
        } else if s.starts_with("/*") {
            Some((Self::CommentStart, 2))
        } else if s.starts_with("*/") {
            Some((Self::CommentEnd, 2))
        } else if s.starts_with("++") {
            Some((Self::Increment, 2))
        } else if s.starts_with("--") {
            Some((Self::Decrement, 2))
        } else {
            None
        }
    }
}

/// A shaped glyph or ligature ready for rasterization.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedGlyph {
    /// Text cluster or representation (e.g. "->", "==", "a").
    pub cluster: String,
    /// Number of monospaced character cells occupied in text space.
    pub advance_cells: usize,
    /// Physical width in pixels according to font scale.
    pub width: usize,
    /// Physical height in pixels according to font scale.
    pub height: usize,
    /// Anti-aliased coverage mask (width * height bytes, values 0..=255).
    pub coverage: Vec<u8>,
    /// Whether this shaped glyph is a programming ligature.
    pub is_ligature: bool,
}

/// Advanced text shaper for programming code ligatures and unicode combining marks.
pub struct FontShaper;

impl FontShaper {
    /// Shapes an input string into a list of positioned `ShapedGlyph`s.
    pub fn shape_text(text: &str, scale: usize, enable_ligatures: bool) -> Vec<ShapedGlyph> {
        let scale = scale.max(1);
        let mut result = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let slice: String = chars[i..].iter().collect();

            // 1. Check for programming ligatures if enabled
            if enable_ligatures {
                if let Some((lig, len)) = LigatureKind::match_prefix(&slice) {
                    let shaped = Self::rasterize_ligature(lig, scale);
                    result.push(shaped);
                    i += len;
                    continue;
                }
            }

            // 2. Check for combining diacritics (base char + combining mark)
            let base_char = chars[i];
            if i + 1 < chars.len() && Self::is_combining_mark(chars[i + 1]) {
                let mark = chars[i + 1];
                let combined_ch = Self::compose_diacritic(base_char, mark);
                let (w, h, cov) = FontRasterizer::rasterize_glyph_scaled(combined_ch, scale);
                result.push(ShapedGlyph {
                    cluster: format!("{}{}", base_char, mark),
                    advance_cells: 1,
                    width: w,
                    height: h,
                    coverage: cov,
                    is_ligature: false,
                });
                i += 2;
                continue;
            }

            // 3. Standard single character
            let (w, h, cov) = FontRasterizer::rasterize_glyph_scaled(base_char, scale);
            result.push(ShapedGlyph {
                cluster: base_char.to_string(),
                advance_cells: 1,
                width: w,
                height: h,
                coverage: cov,
                is_ligature: false,
            });
            i += 1;
        }

        result
    }

    /// Checks if a character is a Unicode combining diacritical mark.
    pub fn is_combining_mark(ch: char) -> bool {
        matches!(ch, '\u{0300}'..='\u{036F}')
    }

    /// Composes a base character with a combining mark into a precomposed Unicode character if possible.
    pub fn compose_diacritic(base: char, mark: char) -> char {
        match (base, mark) {
            ('a', '\u{0301}') => 'á',
            ('e', '\u{0301}') => 'é',
            ('i', '\u{0301}') => 'í',
            ('o', '\u{0301}') => 'ó',
            ('u', '\u{0301}') => 'ú',
            ('A', '\u{0301}') => 'Á',
            ('E', '\u{0301}') => 'É',
            ('I', '\u{0301}') => 'Í',
            ('O', '\u{0301}') => 'Ó',
            ('U', '\u{0301}') => 'Ú',
            ('n', '\u{0303}') => 'ñ',
            ('N', '\u{0303}') => 'Ñ',
            ('u', '\u{0308}') => 'ü',
            ('U', '\u{0308}') => 'Ü',
            _ => base,
        }
    }

    /// Rasterizes a programming ligature into an anti-aliased coverage mask.
    pub fn rasterize_ligature(lig: LigatureKind, scale: usize) -> ShapedGlyph {
        let char_len = lig.char_length();
        let w = FONT_WIDTH * char_len * scale;
        let h = FONT_HEIGHT * scale;
        let mut cov = vec![0u8; w * h];

        // Draw ligature vector art in normalized coordinate space [0..w, 0..h]
        match lig {
            LigatureKind::Arrow => {
                // "->" : Continuous horizontal line with connected chevron tip
                let line_y = h / 2;
                let stroke_w = (scale as f64).max(1.0);
                Self::draw_horizontal_line(&mut cov, w, h, 2 * scale, w - 4 * scale, line_y, stroke_w);
                // Arrow head: upper and lower diagonal wings
                let tip_x = w - 3 * scale;
                let wing_len = 5 * scale;
                Self::draw_line_segment(&mut cov, w, h, tip_x as f64, line_y as f64, (tip_x.saturating_sub(wing_len)) as f64, (line_y.saturating_sub(wing_len)) as f64, stroke_w);
                Self::draw_line_segment(&mut cov, w, h, tip_x as f64, line_y as f64, (tip_x.saturating_sub(wing_len)) as f64, (line_y + wing_len) as f64, stroke_w);
            }
            LigatureKind::FatArrow => {
                // "=>" : Double parallel horizontal lines with connected fat chevron tip
                let stroke_w = (scale as f64).max(1.0);
                let y1 = h / 2 - 2 * scale;
                let y2 = h / 2 + 2 * scale;
                Self::draw_horizontal_line(&mut cov, w, h, 2 * scale, w - 4 * scale, y1, stroke_w);
                Self::draw_horizontal_line(&mut cov, w, h, 2 * scale, w - 4 * scale, y2, stroke_w);
                let tip_x = w - 3 * scale;
                let wing_len = 5 * scale;
                Self::draw_line_segment(&mut cov, w, h, tip_x as f64, (h / 2) as f64, (tip_x.saturating_sub(wing_len)) as f64, (h / 2 - wing_len) as f64, stroke_w * 1.2);
                Self::draw_line_segment(&mut cov, w, h, tip_x as f64, (h / 2) as f64, (tip_x.saturating_sub(wing_len)) as f64, (h / 2 + wing_len) as f64, stroke_w * 1.2);
            }
            LigatureKind::NotEqual => {
                // "!=" : Double horizontal lines with diagonal slash across both
                let stroke_w = (scale as f64).max(1.0);
                let y1 = h / 2 - 2 * scale;
                let y2 = h / 2 + 2 * scale;
                Self::draw_horizontal_line(&mut cov, w, h, 3 * scale, w - 3 * scale, y1, stroke_w);
                Self::draw_horizontal_line(&mut cov, w, h, 3 * scale, w - 3 * scale, y2, stroke_w);
                // Slash across the middle
                let mid_x = w / 2;
                Self::draw_line_segment(&mut cov, w, h, (mid_x + 3 * scale) as f64, (y1 - 3 * scale) as f64, (mid_x.saturating_sub(3 * scale)) as f64, (y2 + 3 * scale) as f64, stroke_w);
            }
            LigatureKind::DoubleEqual => {
                // "==" : Seamless continuous double horizontal bars across 2 character widths
                let stroke_w = (scale as f64).max(1.0);
                let y1 = h / 2 - 2 * scale;
                let y2 = h / 2 + 2 * scale;
                Self::draw_horizontal_line(&mut cov, w, h, 2 * scale, w - 2 * scale, y1, stroke_w);
                Self::draw_horizontal_line(&mut cov, w, h, 2 * scale, w - 2 * scale, y2, stroke_w);
            }
            LigatureKind::TripleEqual => {
                // "===" : Seamless continuous triple horizontal bars across 3 character widths
                let stroke_w = (scale as f64).max(1.0);
                let y1 = h / 2 - 3 * scale;
                let y2 = h / 2;
                let y3 = h / 2 + 3 * scale;
                Self::draw_horizontal_line(&mut cov, w, h, 2 * scale, w - 2 * scale, y1, stroke_w);
                Self::draw_horizontal_line(&mut cov, w, h, 2 * scale, w - 2 * scale, y2, stroke_w);
                Self::draw_horizontal_line(&mut cov, w, h, 2 * scale, w - 2 * scale, y3, stroke_w);
            }
            LigatureKind::NotDoubleEqual => {
                // "!==" : Triple horizontal bars with diagonal slash
                let stroke_w = (scale as f64).max(1.0);
                let y1 = h / 2 - 3 * scale;
                let y2 = h / 2;
                let y3 = h / 2 + 3 * scale;
                Self::draw_horizontal_line(&mut cov, w, h, 2 * scale, w - 2 * scale, y1, stroke_w);
                Self::draw_horizontal_line(&mut cov, w, h, 2 * scale, w - 2 * scale, y2, stroke_w);
                Self::draw_horizontal_line(&mut cov, w, h, 2 * scale, w - 2 * scale, y3, stroke_w);
                let mid_x = w / 2;
                Self::draw_line_segment(&mut cov, w, h, (mid_x + 4 * scale) as f64, (y1 - 2 * scale) as f64, (mid_x.saturating_sub(4 * scale)) as f64, (y3 + 2 * scale) as f64, stroke_w);
            }
            LigatureKind::LessEqual => {
                // "<=" : Left-pointing chevron with bottom horizontal bar
                let stroke_w = (scale as f64).max(1.0);
                let y_bar = h / 2 + 3 * scale;
                Self::draw_horizontal_line(&mut cov, w, h, 4 * scale, w - 3 * scale, y_bar, stroke_w);
                let tip_x = 4 * scale;
                let end_x = w - 4 * scale;
                let tip_y = h / 2 - scale;
                let top_y = h / 2 - 5 * scale;
                Self::draw_line_segment(&mut cov, w, h, end_x as f64, top_y as f64, tip_x as f64, tip_y as f64, stroke_w);
                Self::draw_line_segment(&mut cov, w, h, tip_x as f64, tip_y as f64, end_x as f64, (tip_y + 3 * scale) as f64, stroke_w);
            }
            LigatureKind::GreaterEqual => {
                // ">=" : Right-pointing chevron with bottom horizontal bar
                let stroke_w = (scale as f64).max(1.0);
                let y_bar = h / 2 + 3 * scale;
                Self::draw_horizontal_line(&mut cov, w, h, 3 * scale, w - 4 * scale, y_bar, stroke_w);
                let tip_x = w - 4 * scale;
                let start_x = 4 * scale;
                let tip_y = h / 2 - scale;
                let top_y = h / 2 - 5 * scale;
                Self::draw_line_segment(&mut cov, w, h, start_x as f64, top_y as f64, tip_x as f64, tip_y as f64, stroke_w);
                Self::draw_line_segment(&mut cov, w, h, tip_x as f64, tip_y as f64, start_x as f64, (tip_y + 3 * scale) as f64, stroke_w);
            }
            LigatureKind::Walrus => {
                // ":=" : Colon dots centered with equals sign
                let stroke_w = (scale as f64).max(1.0);
                let dot_x = 4 * scale;
                let dot_r = (scale as f64 * 0.9).max(1.0);
                Self::draw_circle(&mut cov, w, h, dot_x as f64, (h / 2 - 3 * scale) as f64, dot_r);
                Self::draw_circle(&mut cov, w, h, dot_x as f64, (h / 2 + 3 * scale) as f64, dot_r);
                let y1 = h / 2 - 2 * scale;
                let y2 = h / 2 + 2 * scale;
                Self::draw_horizontal_line(&mut cov, w, h, 8 * scale, w - 2 * scale, y1, stroke_w);
                Self::draw_horizontal_line(&mut cov, w, h, 8 * scale, w - 2 * scale, y2, stroke_w);
            }
            LigatureKind::LogicalAnd => {
                // "&&" : Connected double ampersand glyphs
                let (w1, _, cov1) = FontRasterizer::rasterize_glyph_scaled('&', scale);
                let offset_x = (FONT_WIDTH * scale).saturating_sub(scale);
                Self::blit_coverage(&mut cov, w, h, &cov1, w1, h, 0, 0);
                Self::blit_coverage(&mut cov, w, h, &cov1, w1, h, offset_x, 0);
            }
            LigatureKind::LogicalOr => {
                // "||" : Parallel vertical bars cleanly separated
                let stroke_w = (scale as f64).max(1.0);
                let x1 = w / 3;
                let x2 = (2 * w) / 3;
                Self::draw_vertical_line(&mut cov, w, h, x1, 2 * scale, h - 2 * scale, stroke_w);
                Self::draw_vertical_line(&mut cov, w, h, x2, 2 * scale, h - 2 * scale, stroke_w);
            }
            LigatureKind::Scope => {
                // "::" : Double vertical colon dots horizontally aligned
                let dot_r = (scale as f64 * 0.9).max(1.0);
                let x1 = w / 3;
                let x2 = (2 * w) / 3;
                let y1 = h / 2 - 3 * scale;
                let y2 = h / 2 + 3 * scale;
                Self::draw_circle(&mut cov, w, h, x1 as f64, y1 as f64, dot_r);
                Self::draw_circle(&mut cov, w, h, x1 as f64, y2 as f64, dot_r);
                Self::draw_circle(&mut cov, w, h, x2 as f64, y1 as f64, dot_r);
                Self::draw_circle(&mut cov, w, h, x2 as f64, y2 as f64, dot_r);
            }
            LigatureKind::Range => {
                // ".." : Two baseline dots
                let dot_r = (scale as f64 * 1.0).max(1.0);
                let y = h - 4 * scale;
                let x1 = w / 3;
                let x2 = (2 * w) / 3;
                Self::draw_circle(&mut cov, w, h, x1 as f64, y as f64, dot_r);
                Self::draw_circle(&mut cov, w, h, x2 as f64, y as f64, dot_r);
            }
            LigatureKind::Ellipsis => {
                // "..." : Three baseline dots spaced across 3 cells
                let dot_r = (scale as f64 * 1.0).max(1.0);
                let y = h - 4 * scale;
                let x1 = w / 4;
                let x2 = w / 2;
                let x3 = (3 * w) / 4;
                Self::draw_circle(&mut cov, w, h, x1 as f64, y as f64, dot_r);
                Self::draw_circle(&mut cov, w, h, x2 as f64, y as f64, dot_r);
                Self::draw_circle(&mut cov, w, h, x3 as f64, y as f64, dot_r);
            }
            LigatureKind::CommentStart => {
                let (w1, _, cov1) = FontRasterizer::rasterize_glyph_scaled('/', scale);
                let (w2, _, cov2) = FontRasterizer::rasterize_glyph_scaled('*', scale);
                Self::blit_coverage(&mut cov, w, h, &cov1, w1, h, 0, 0);
                Self::blit_coverage(&mut cov, w, h, &cov2, w2, h, FONT_WIDTH * scale, 0);
            }
            LigatureKind::CommentEnd => {
                let (w1, _, cov1) = FontRasterizer::rasterize_glyph_scaled('*', scale);
                let (w2, _, cov2) = FontRasterizer::rasterize_glyph_scaled('/', scale);
                Self::blit_coverage(&mut cov, w, h, &cov1, w1, h, 0, 0);
                Self::blit_coverage(&mut cov, w, h, &cov2, w2, h, FONT_WIDTH * scale, 0);
            }
            LigatureKind::Increment => {
                let (w1, _, cov1) = FontRasterizer::rasterize_glyph_scaled('+', scale);
                let offset_x = (FONT_WIDTH * scale).saturating_sub(scale);
                Self::blit_coverage(&mut cov, w, h, &cov1, w1, h, 0, 0);
                Self::blit_coverage(&mut cov, w, h, &cov1, w1, h, offset_x, 0);
            }
            LigatureKind::Decrement => {
                let stroke_w = (scale as f64).max(1.0);
                let y = h / 2;
                Self::draw_horizontal_line(&mut cov, w, h, 2 * scale, w - 2 * scale, y, stroke_w);
            }
        }

        let label = match lig {
            LigatureKind::Arrow => "->",
            LigatureKind::FatArrow => "=>",
            LigatureKind::NotEqual => "!=",
            LigatureKind::DoubleEqual => "==",
            LigatureKind::TripleEqual => "===",
            LigatureKind::NotDoubleEqual => "!==",
            LigatureKind::LessEqual => "<=",
            LigatureKind::GreaterEqual => ">=",
            LigatureKind::Walrus => ":=",
            LigatureKind::LogicalAnd => "&&",
            LigatureKind::LogicalOr => "||",
            LigatureKind::Scope => "::",
            LigatureKind::Range => "..",
            LigatureKind::Ellipsis => "...",
            LigatureKind::CommentStart => "/*",
            LigatureKind::CommentEnd => "*/",
            LigatureKind::Increment => "++",
            LigatureKind::Decrement => "--",
        };

        ShapedGlyph {
            cluster: label.to_string(),
            advance_cells: char_len,
            width: w,
            height: h,
            coverage: cov,
            is_ligature: true,
        }
    }

    /// Blits one coverage mask onto another with saturation.
    fn blit_coverage(dst: &mut [u8], dw: usize, dh: usize, src: &[u8], sw: usize, sh: usize, ox: usize, oy: usize) {
        let max_y = (oy + sh).min(dh);
        let max_x = (ox + sw).min(dw);
        for y in oy..max_y {
            let sy = y - oy;
            for x in ox..max_x {
                let sx = x - ox;
                let s_val = src[sy * sw + sx];
                let d_idx = y * dw + x;
                dst[d_idx] = dst[d_idx].saturating_add(s_val);
            }
        }
    }

    /// Draws an anti-aliased horizontal line on a coverage mask.
    fn draw_horizontal_line(buf: &mut [u8], w: usize, h: usize, x0: usize, x1: usize, y: usize, stroke_w: f64) {
        let half_w = stroke_w * 0.5;
        let y_min = ((y as f64 - half_w).floor() as usize).min(h.saturating_sub(1));
        let y_max = ((y as f64 + half_w).ceil() as usize).min(h.saturating_sub(1));

        for py in y_min..=y_max {
            let dist = (py as f64 + 0.5 - (y as f64 + 0.5)).abs();
            let alpha = ((1.0 - (dist - half_w).max(0.0)).clamp(0.0, 1.0) * 255.0).round() as u8;
            if alpha > 0 {
                for px in x0.min(w)..x1.min(w) {
                    let idx = py * w + px;
                    buf[idx] = buf[idx].max(alpha);
                }
            }
        }
    }

    /// Draws an anti-aliased vertical line on a coverage mask.
    fn draw_vertical_line(buf: &mut [u8], w: usize, h: usize, x: usize, y0: usize, y1: usize, stroke_w: f64) {
        let half_w = stroke_w * 0.5;
        let x_min = ((x as f64 - half_w).floor() as usize).min(w.saturating_sub(1));
        let x_max = ((x as f64 + half_w).ceil() as usize).min(w.saturating_sub(1));

        for px in x_min..=x_max {
            let dist = (px as f64 + 0.5 - (x as f64 + 0.5)).abs();
            let alpha = ((1.0 - (dist - half_w).max(0.0)).clamp(0.0, 1.0) * 255.0).round() as u8;
            if alpha > 0 {
                for py in y0.min(h)..y1.min(h) {
                    let idx = py * w + px;
                    buf[idx] = buf[idx].max(alpha);
                }
            }
        }
    }

    /// Draws an anti-aliased line segment using analytical distance.
    fn draw_line_segment(buf: &mut [u8], w: usize, h: usize, x0: f64, y0: f64, x1: f64, y1: f64, stroke_w: f64) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len_sq = dx * dx + dy * dy;
        if len_sq < 0.0001 {
            return;
        }

        let min_x = (x0.min(x1) - stroke_w - 1.0).max(0.0).floor() as usize;
        let max_x = (x0.max(x1) + stroke_w + 1.0).min(w as f64 - 1.0).ceil() as usize;
        let min_y = (y0.min(y1) - stroke_w - 1.0).max(0.0).floor() as usize;
        let max_y = (y0.max(y1) + stroke_w + 1.0).min(h as f64 - 1.0).ceil() as usize;

        let half_w = stroke_w * 0.5;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let cx = px as f64 + 0.5;
                let cy = py as f64 + 0.5;

                // Project (cx, cy) onto segment
                let t = (((cx - x0) * dx + (cy - y0) * dy) / len_sq).clamp(0.0, 1.0);
                let proj_x = x0 + t * dx;
                let proj_y = y0 + t * dy;
                let dist = ((cx - proj_x).hypot(cy - proj_y)).abs();

                let cov = (1.0 - (dist - half_w).max(0.0)).clamp(0.0, 1.0);
                let alpha = (cov * 255.0).round() as u8;
                if alpha > 0 {
                    let idx = py * w + px;
                    buf[idx] = buf[idx].max(alpha);
                }
            }
        }
    }

    /// Draws an anti-aliased filled circle.
    fn draw_circle(buf: &mut [u8], w: usize, h: usize, cx: f64, cy: f64, radius: f64) {
        let min_x = (cx - radius - 1.0).max(0.0).floor() as usize;
        let max_x = (cx + radius + 1.0).min(w as f64 - 1.0).ceil() as usize;
        let min_y = (cy - radius - 1.0).max(0.0).floor() as usize;
        let max_y = (cy + radius + 1.0).min(h as f64 - 1.0).ceil() as usize;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let px_c = px as f64 + 0.5;
                let py_c = py as f64 + 0.5;
                let dist = (px_c - cx).hypot(py_c - cy);
                let cov = (0.5 + radius - dist).clamp(0.0, 1.0);
                let alpha = (cov * 255.0).round() as u8;
                if alpha > 0 {
                    let idx = py * w + px;
                    buf[idx] = buf[idx].max(alpha);
                }
            }
        }
    }
}
