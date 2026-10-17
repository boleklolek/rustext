//! Dynamic Optical Punctuation Spacing and Micro-typographical Kerning Engine.
//! Analyzes glyph ink mass distribution, calculates optical centers of gravity,
//! and dynamically balances white space around punctuation marks (dots, commas,
//! colons, brackets, arrows, and operators) without violating strict monospace column invariants.
//! Pure standard-library Rust with zero third-party dependencies.

/// Classification of typographical characters for contextual optical spacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PunctuationClass {
    /// Period or dot: `.`
    Period,
    /// Comma: `,`
    Comma,
    /// Colon: `:`
    Colon,
    /// Semicolon: `;`
    Semicolon,
    /// Opening parentheses: `(`
    OpenParen,
    /// Closing parentheses: `)`
    CloseParen,
    /// Opening bracket: `[`
    OpenBracket,
    /// Closing bracket: `]`
    CloseBracket,
    /// Opening brace: `{`
    OpenBrace,
    /// Closing brace: `}`
    CloseBrace,
    /// Quotation marks: `"`, `'`, ``` ` ```
    Quote,
    /// Mathematical/Logical operators: `+`, `-`, `*`, `/`, `=`, `!`, `<`, `>`, `&`, `|`, `^`, `%`, `~`
    Operator,
    /// Scope resolution or double colon: `::`
    Scope,
    /// Standard alphanumeric letter or digit: `a`..=`z`, `A`..=`Z`, `0`..=`9`, `_`
    Alphanumeric,
    /// Whitespace space or tab: `' '`, `'\t'`
    Whitespace,
    /// Other characters
    Other,
}

impl PunctuationClass {
    /// Classifies a character into its typographical punctuation category.
    pub fn from_char(ch: char) -> Self {
        match ch {
            '.' => Self::Period,
            ',' => Self::Comma,
            ':' => Self::Colon,
            ';' => Self::Semicolon,
            '(' => Self::OpenParen,
            ')' => Self::CloseParen,
            '[' => Self::OpenBracket,
            ']' => Self::CloseBracket,
            '{' => Self::OpenBrace,
            '}' => Self::CloseBrace,
            '"' | '\'' | '`' => Self::Quote,
            '+' | '-' | '*' | '/' | '=' | '!' | '<' | '>' | '&' | '|' | '^' | '%' | '~' | '?' => Self::Operator,
            ' ' | '\t' | '\n' | '\r' => Self::Whitespace,
            c if c.is_alphanumeric() || c == '_' => Self::Alphanumeric,
            _ => Self::Other,
        }
    }

    /// Returns true if this class represents a closing delimiter.
    pub fn is_closing(&self) -> bool {
        matches!(self, Self::CloseParen | Self::CloseBracket | Self::CloseBrace)
    }

    /// Returns true if this class represents an opening delimiter.
    pub fn is_opening(&self) -> bool {
        matches!(self, Self::OpenParen | Self::OpenBracket | Self::OpenBrace)
    }

    /// Returns true if this class represents a lightweight terminal mark (period, comma, semicolon, colon).
    pub fn is_terminal_mark(&self) -> bool {
        matches!(self, Self::Period | Self::Comma | Self::Semicolon | Self::Colon)
    }
}

/// Geometric and optical ink distribution metrics for a single glyph cell.
#[derive(Debug, Clone, PartialEq)]
pub struct GlyphOpticalMetrics {
    /// Leftmost horizontal column containing visible ink (coverage > 15).
    pub min_x: usize,
    /// Rightmost horizontal column containing visible ink.
    pub max_x: usize,
    /// Width of the ink silhouette: `max_x - min_x + 1`.
    pub ink_width: usize,
    /// Left whitespace margin in pixels.
    pub left_margin: usize,
    /// Right whitespace margin in pixels.
    pub right_margin: usize,
    /// Optical horizontal center of mass: $\bar{x} = \sum (x \cdot \alpha) / \sum \alpha$.
    pub center_of_mass_x: f32,
    /// Total ink mass (sum of alpha coverages).
    pub total_mass: usize,
}

impl GlyphOpticalMetrics {
    /// Measures ink distribution and center of mass from an anti-aliased coverage mask.
    pub fn from_coverage(coverage: &[u8], width: usize, height: usize) -> Self {
        if width == 0 || height == 0 || coverage.is_empty() {
            return Self {
                min_x: 0,
                max_x: 0,
                ink_width: 0,
                left_margin: 0,
                right_margin: 0,
                center_of_mass_x: 0.0,
                total_mass: 0,
            };
        }

        let mut min_x = width;
        let mut max_x = 0;
        let mut total_mass = 0usize;
        let mut moment_x = 0.0f32;

        for y in 0..height {
            let row = y * width;
            for x in 0..width {
                let alpha = coverage[row + x] as usize;
                if alpha > 15 {
                    if x < min_x { min_x = x; }
                    if x > max_x { max_x = x; }
                    total_mass += alpha;
                    moment_x += (x as f32) * (alpha as f32);
                }
            }
        }

        if total_mass == 0 || min_x > max_x {
            // Empty glyph (e.g. whitespace)
            return Self {
                min_x: 0,
                max_x: 0,
                ink_width: 0,
                left_margin: width / 2,
                right_margin: width / 2,
                center_of_mass_x: (width as f32) / 2.0,
                total_mass: 0,
            };
        }

        let ink_width = max_x - min_x + 1;
        let left_margin = min_x;
        let right_margin = width.saturating_sub(max_x + 1);
        let center_of_mass_x = moment_x / (total_mass as f32);

        Self {
            min_x,
            max_x,
            ink_width,
            left_margin,
            right_margin,
            center_of_mass_x,
            total_mass,
        }
    }
}

/// Dynamic Optical Punctuation Spacing calculator.
pub struct OpticalPunctuationSpacing;

impl OpticalPunctuationSpacing {
    /// Computes the contextual optical shift $\delta x$ for a character `curr`
    /// positioned between `prev` and `next`.
    ///
    /// Negative return value shifts ink left; positive shifts right.
    /// The shift is carefully proportioned to `font_scale` and strictly clamped
    /// so the glyph NEVER exceeds its monospace character cell boundary.
    pub fn compute_optical_offset(
        prev: Option<char>,
        curr: char,
        next: Option<char>,
        char_w: usize,
        font_scale: usize,
    ) -> i32 {
        let scale = font_scale.max(1) as i32;
        let curr_class = PunctuationClass::from_char(curr);
        let prev_class = prev.map(PunctuationClass::from_char).unwrap_or(PunctuationClass::Whitespace);
        let next_class = next.map(PunctuationClass::from_char).unwrap_or(PunctuationClass::Whitespace);

        let mut offset = 0i32;

        match curr_class {
            PunctuationClass::Period => {
                // Rule 1: Period following alphanumeric (e.g. `obj.prop` or `42.0`)
                if prev_class == PunctuationClass::Alphanumeric && next_class == PunctuationClass::Alphanumeric {
                    // Symmetrical balance between both stems
                    offset = -1 * scale;
                } else if prev_class == PunctuationClass::Alphanumeric {
                    // Terminal period or method call start
                    offset = -1 * scale;
                }
            }

            PunctuationClass::Comma => {
                // Rule 2: Comma following argument/element (`arg, next`)
                if prev_class == PunctuationClass::Alphanumeric || prev_class.is_closing() {
                    offset = -1 * scale;
                }
            }

            PunctuationClass::Semicolon => {
                // Rule 3: Semicolon at end of statement (`expr;`)
                if prev_class == PunctuationClass::Alphanumeric || prev_class.is_closing() {
                    offset = -1 * scale;
                }
            }

            PunctuationClass::Colon => {
                // Rule 4: Colon in type annotations (`val: Type` or `key: val`)
                if prev_class == PunctuationClass::Alphanumeric && next_class == PunctuationClass::Whitespace {
                    offset = -1 * scale;
                }
            }

            PunctuationClass::OpenParen | PunctuationClass::OpenBracket => {
                // Rule 5: Function invocation or indexing (`foo(` or `arr[`)
                if prev_class == PunctuationClass::Alphanumeric {
                    offset = -1 * scale;
                } else if next_class == PunctuationClass::CloseParen || next_class == PunctuationClass::CloseBracket {
                    // Empty pair `()` or `[]`: nudge right to balance
                    offset = 1 * scale;
                }
            }

            PunctuationClass::CloseParen | PunctuationClass::CloseBracket => {
                // Rule 6: Closing bracket adjacent to punctuation (e.g. `);` or `),`)
                if next_class == PunctuationClass::Semicolon || next_class == PunctuationClass::Comma {
                    offset = 1 * scale;
                } else if prev_class == PunctuationClass::OpenParen || prev_class == PunctuationClass::OpenBracket {
                    // Empty pair: nudge left
                    offset = -1 * scale;
                }
            }

            PunctuationClass::Quote => {
                // Rule 7: Opening quote vs closing quote
                if prev_class == PunctuationClass::Whitespace || prev_class.is_opening() {
                    // Opening quote: slight right tuck toward content
                    offset = 1 * scale;
                } else if next_class == PunctuationClass::Whitespace || next_class.is_terminal_mark() || next_class.is_closing() {
                    // Closing quote: slight left tuck toward content
                    offset = -1 * scale;
                }
            }

            PunctuationClass::Operator => {
                // Rule 8: Compact operators such as `!` in `!cond`
                if curr == '!' && next_class == PunctuationClass::Alphanumeric {
                    offset = 1 * scale;
                }
            }

            _ => {}
        }

        // Clamp offset to ensure glyph ink does not cross cell boundary
        let max_clamp = (char_w as i32 / 4).max(1);
        offset.clamp(-max_clamp, max_clamp)
    }

    /// Computes safe in-cell horizontal draw coordinate for a character glyph
    /// taking into account its measured optical metrics.
    /// Guarantees that `draw_x` remains strictly within `[cell_px, cell_px + char_w - draw_w]`.
    pub fn compute_safe_in_cell_draw_x(
        cell_px: usize,
        char_w: usize,
        draw_w: usize,
        optical_offset: i32,
    ) -> usize {
        let nominal_off = (char_w.saturating_sub(draw_w)) / 2;
        let candidate = (cell_px + nominal_off) as isize + (optical_offset as isize);
        let min_x = cell_px as isize;
        let max_x = (cell_px + char_w.saturating_sub(draw_w)) as isize;
        candidate.clamp(min_x, max_x) as usize
    }
}
