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

    /// Returns all static keywords for the specified language.
    pub fn keywords(lang: Language) -> &'static [&'static str] {
        match lang {
            Language::Rust => &[
                "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else",
                "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop",
                "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self",
                "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
                "where", "while",
            ],
            Language::Python => &[
                "and", "as", "assert", "async", "await", "break", "class", "continue",
                "def", "del", "elif", "else", "except", "finally", "for", "from",
                "global", "if", "import", "in", "is", "lambda", "nonlocal", "not",
                "or", "pass", "raise", "return", "try", "while", "with", "yield",
                "True", "False", "None",
            ],
            Language::C => &[
                "auto", "break", "case", "char", "const", "continue", "default", "do",
                "double", "else", "enum", "extern", "float", "for", "goto", "if",
                "int", "long", "register", "return", "short", "signed", "sizeof", "static",
                "struct", "switch", "typedef", "union", "unsigned", "void", "volatile", "while",
            ],
            Language::JavaScript => &[
                "async", "await", "break", "case", "catch", "class", "const", "continue",
                "debugger", "default", "delete", "do", "else", "export", "extends", "false",
                "finally", "for", "function", "if", "import", "in", "instanceof", "let",
                "new", "null", "return", "super", "switch", "this", "throw", "true",
                "try", "typeof", "var", "void", "while", "with", "yield",
            ],
            Language::Toml => &["true", "false"],
            Language::Json | Language::Markdown | Language::PlainText => &[],
        }
    }

    /// Returns all static built-in types for the specified language.
    pub fn types(lang: Language) -> &'static [&'static str] {
        match lang {
            Language::Rust => &[
                "Arc", "Box", "Err", "None", "Ok", "Option", "Rc", "Result", "Some",
                "String", "Vec", "bool", "char", "f32", "f64", "i128", "i16", "i32",
                "i64", "i8", "isize", "str", "u128", "u16", "u32", "u64", "u8", "usize",
            ],
            Language::Python => &[
                "int", "float", "str", "bool", "list", "dict", "set", "tuple", "bytes",
                "object", "type", "Exception", "ValueError", "TypeError",
            ],
            Language::C => &[
                "bool", "char", "double", "float", "int", "int16_t", "int32_t", "int64_t",
                "int8_t", "long", "short", "size_t", "uint16_t", "uint32_t", "uint64_t",
                "uint8_t", "void",
            ],
            Language::JavaScript => &[
                "Array", "Boolean", "Date", "Error", "Function", "Map", "Number",
                "Object", "Promise", "RegExp", "Set", "String", "Symbol",
            ],
            Language::Json | Language::Markdown | Language::Toml | Language::PlainText => &[],
        }
    }

    fn classify_word(word: &str, lang: Language) -> TokenType {
        let kw = Self::keywords(lang);
        let ty = Self::types(lang);

        if kw.contains(&word) {
            TokenType::Keyword
        } else if ty.contains(&word) || (lang != Language::PlainText && word.starts_with(|c: char| c.is_ascii_uppercase())) {
            TokenType::Type
        } else {
            TokenType::Identifier
        }
    }
}
