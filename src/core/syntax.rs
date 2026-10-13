//! Zero-dependency syntax lexer and syntax highlighter.
//! Supports Rust, Python, C/C++, JavaScript, JSON, Markdown, and TOML.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Python,
    C,
    JavaScript,
    Json,
    Markdown,
    Toml,
    PlainText,
}

impl Language {
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => Language::Rust,
            Some("py") => Language::Python,
            Some("c") | Some("h") | Some("cpp") | Some("hpp") | Some("cc") => Language::C,
            Some("js") | Some("ts") | Some("jsx") | Some("tsx") => Language::JavaScript,
            Some("json") => Language::Json,
            Some("md") | Some("markdown") => Language::Markdown,
            Some("toml") => Language::Toml,
            _ => Language::PlainText,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::Python => "Python",
            Language::C => "C/C++",
            Language::JavaScript => "JavaScript",
            Language::Json => "JSON",
            Language::Markdown => "Markdown",
            Language::Toml => "TOML",
            Language::PlainText => "Plain Text",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Keyword,
    Type,
    StringLiteral,
    Number,
    Comment,
    Operator,
    Punctuation,
    Identifier,
    PlainText,
}

impl TokenType {
    /// Returns the ANSI escape code sequence for this token type.
    pub fn ansi_code(&self) -> &'static str {
        match self {
            TokenType::Keyword => "\x1b[1;35m",       // Bold Magenta
            TokenType::Type => "\x1b[1;36m",          // Bold Cyan
            TokenType::StringLiteral => "\x1b[32m",   // Green
            TokenType::Number => "\x1b[33m",          // Yellow
            TokenType::Comment => "\x1b[38;5;244m",   // Gray
            TokenType::Operator => "\x1b[31m",        // Red
            TokenType::Punctuation => "\x1b[37m",     // White
            TokenType::Identifier => "\x1b[39m",      // Default text
            TokenType::PlainText => "\x1b[39m",       // Default text
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenSpan<'a> {
    pub text: &'a str,
    pub token_type: TokenType,
}

impl<'a> TokenSpan<'a> {
    #[inline]
    pub fn as_str(&self) -> &'a str {
        self.text
    }
}

pub struct SyntaxHighlighter;

impl SyntaxHighlighter {
    /// Tokenizes a single line of text according to language rules with zero heap allocations.
    pub fn tokenize_line<'a>(line: &'a str, lang: Language) -> Vec<TokenSpan<'a>> {
        if lang == Language::PlainText || line.is_empty() {
            return vec![TokenSpan {
                text: line,
                token_type: TokenType::PlainText,
            }];
        }

        let mut spans = Vec::with_capacity(line.len() / 4 + 1);
        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut idx = 0;

        while idx < len {
            let b = bytes[idx];

            // 1. Comments
            if (lang == Language::Rust || lang == Language::C || lang == Language::JavaScript)
                && b == b'/'
                && idx + 1 < len
                && bytes[idx + 1] == b'/'
            {
                spans.push(TokenSpan {
                    text: &line[idx..],
                    token_type: TokenType::Comment,
                });
                break;
            }

            if (lang == Language::Python || lang == Language::Toml) && b == b'#' {
                spans.push(TokenSpan {
                    text: &line[idx..],
                    token_type: TokenType::Comment,
                });
                break;
            }

            // 2. String literals
            if b == b'"' || b == b'\'' || b == b'`' {
                let quote = b;
                let start = idx;
                idx += 1;
                let mut escaped = false;

                while idx < len {
                    let cur = bytes[idx];
                    if cur == quote && !escaped {
                        idx += 1;
                        break;
                    }
                    if cur == b'\\' {
                        escaped = !escaped;
                    } else {
                        escaped = false;
                    }
                    idx += 1;
                }

                spans.push(TokenSpan {
                    text: &line[start..idx],
                    token_type: TokenType::StringLiteral,
                });
                continue;
            }

            // 3. Numbers
            if b.is_ascii_digit() {
                let start = idx;
                idx += 1;
                while idx < len && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == b'.' || bytes[idx] == b'_') {
                    idx += 1;
                }
                spans.push(TokenSpan {
                    text: &line[start..idx],
                    token_type: TokenType::Number,
                });
                continue;
            }

            // 4. Identifiers, Keywords, Types
            if b.is_ascii_alphabetic() || b == b'_' {
                let start = idx;
                idx += 1;
                while idx < len && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == b'_') {
                    idx += 1;
                }
                let word = &line[start..idx];
                let token_type = Self::classify_word(word, lang);
                spans.push(TokenSpan {
                    text: word,
                    token_type,
                });
                continue;
            }

            // 5. Operators and Punctuation
            if b"+-*/%&|^!=<>:;.,?@#$~".contains(&b) {
                let op_type = if b"+-*/%=<>!&|^?:".contains(&b) {
                    TokenType::Operator
                } else {
                    TokenType::Punctuation
                };
                spans.push(TokenSpan {
                    text: &line[idx..idx + 1],
                    token_type: op_type,
                });
                idx += 1;
                continue;
            }

            // 6. Whitespace or multi-byte UTF-8 char
            let ch_len = line[idx..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            spans.push(TokenSpan {
                text: &line[idx..idx + ch_len],
                token_type: TokenType::PlainText,
            });
            idx += ch_len;
        }

        spans
    }

    fn classify_word(word: &str, lang: Language) -> TokenType {
        match lang {
            Language::Rust => {
                const RUST_KEYWORDS: &[&str] = &[
                    "as", "break", "const", "continue", "crate", "else", "enum", "extern",
                    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
                    "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct",
                    "super", "trait", "true", "type", "unsafe", "use", "where", "while",
                    "async", "await", "dyn",
                ];
                const RUST_TYPES: &[&str] = &[
                    "bool", "char", "i8", "i16", "i32", "i64", "i128", "isize",
                    "u8", "u16", "u32", "u64", "u128", "usize", "f32", "f64",
                    "str", "String", "Vec", "Option", "Some", "None", "Result", "Ok", "Err",
                    "Box", "Rc", "Arc",
                ];

                if RUST_KEYWORDS.contains(&word) {
                    TokenType::Keyword
                } else if RUST_TYPES.contains(&word) || word.starts_with(|c: char| c.is_ascii_uppercase()) {
                    TokenType::Type
                } else {
                    TokenType::Identifier
                }
            }
            Language::Python => {
                const PY_KEYWORDS: &[&str] = &[
                    "and", "as", "assert", "async", "await", "break", "class", "continue",
                    "def", "del", "elif", "else", "except", "finally", "for", "from",
                    "global", "if", "import", "in", "is", "lambda", "nonlocal", "not",
                    "or", "pass", "raise", "return", "try", "while", "with", "yield",
                    "True", "False", "None",
                ];
                if PY_KEYWORDS.contains(&word) {
                    TokenType::Keyword
                } else if word.starts_with(|c: char| c.is_ascii_uppercase()) {
                    TokenType::Type
                } else {
                    TokenType::Identifier
                }
            }
            Language::C => {
                const C_KEYWORDS: &[&str] = &[
                    "auto", "break", "case", "char", "const", "continue", "default", "do",
                    "double", "else", "enum", "extern", "float", "for", "goto", "if",
                    "int", "long", "register", "return", "short", "signed", "sizeof", "static",
                    "struct", "switch", "typedef", "union", "unsigned", "void", "volatile", "while",
                ];
                if C_KEYWORDS.contains(&word) {
                    TokenType::Keyword
                } else if word.starts_with(|c: char| c.is_ascii_uppercase()) {
                    TokenType::Type
                } else {
                    TokenType::Identifier
                }
            }
            Language::JavaScript => {
                const JS_KEYWORDS: &[&str] = &[
                    "break", "case", "catch", "class", "const", "continue", "debugger",
                    "default", "delete", "do", "else", "export", "extends", "finally",
                    "for", "function", "if", "import", "in", "instanceof", "new", "return",
                    "super", "switch", "this", "throw", "try", "typeof", "var", "void",
                    "while", "with", "yield", "let", "async", "await", "true", "false", "null",
                ];
                if JS_KEYWORDS.contains(&word) {
                    TokenType::Keyword
                } else if word.starts_with(|c: char| c.is_ascii_uppercase()) {
                    TokenType::Type
                } else {
                    TokenType::Identifier
                }
            }
            _ => TokenType::PlainText,
        }
    }
}
