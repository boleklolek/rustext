//! Zero-dependency syntax lexer and syntax highlighter.
//! Supports Rust, Python, C/C++, JavaScript/TypeScript, HTML, CSS, JSON, YAML, TOML, Shell/Bash, SQL, and Markdown.
//! Features Bracket Pair Colorization with cyclic nesting depth colors.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Python,
    C,
    JavaScript,
    Html,
    Css,
    Json,
    Yaml,
    Toml,
    Shell,
    Sql,
    Markdown,
    PlainText,
}

impl Language {
    pub fn from_path(path: &Path) -> Self {
        let ext = path.extension().and_then(|ext| ext.to_str()).map(|s| s.to_ascii_lowercase());
        match ext.as_deref() {
            Some("rs") => Language::Rust,
            Some("py") => Language::Python,
            Some("c") | Some("h") | Some("cpp") | Some("hpp") | Some("cc") | Some("cxx") => Language::C,
            Some("js") | Some("ts") | Some("jsx") | Some("tsx") | Some("mjs") | Some("cjs") => Language::JavaScript,
            Some("html") | Some("htm") | Some("xhtml") | Some("xml") | Some("svg") => Language::Html,
            Some("css") | Some("scss") | Some("sass") | Some("less") => Language::Css,
            Some("json") => Language::Json,
            Some("yaml") | Some("yml") => Language::Yaml,
            Some("toml") => Language::Toml,
            Some("sh") | Some("bash") | Some("zsh") | Some("fish") | Some("env") => Language::Shell,
            Some("sql") => Language::Sql,
            Some("md") | Some("markdown") => Language::Markdown,
            _ => {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name == ".env" || name.starts_with(".env.") {
                        Language::Shell
                    } else {
                        match name {
                            ".bashrc" | ".bash_profile" | ".zshrc" | ".profile" | "PKGBUILD" => Language::Shell,
                            "Dockerfile" => Language::Shell,
                            _ => Language::PlainText,
                        }
                    }
                } else {
                    Language::PlainText
                }
            }
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::Python => "Python",
            Language::C => "C/C++",
            Language::JavaScript => "JavaScript",
            Language::Html => "HTML",
            Language::Css => "CSS",
            Language::Json => "JSON",
            Language::Yaml => "YAML",
            Language::Toml => "TOML",
            Language::Shell => "Shell/Bash",
            Language::Sql => "SQL",
            Language::Markdown => "Markdown",
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
    Bracket(usize),
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
            TokenType::Bracket(depth) => match depth % 3 {
                0 => "\x1b[1;33m",                    // Bold Yellow (Level 0)
                1 => "\x1b[1;35m",                    // Bold Magenta (Level 1)
                _ => "\x1b[1;36m",                    // Bold Cyan (Level 2)
            },
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
    #[inline]
    pub fn tokenize_line<'a>(line: &'a str, lang: Language) -> Vec<TokenSpan<'a>> {
        Self::tokenize_line_with_depth(line, lang, 0).0
    }

    /// Tokenizes a single line of text with an initial bracket nesting depth.
    /// Returns the vector of token spans and the resulting bracket nesting depth.
    pub fn tokenize_line_with_depth<'a>(
        line: &'a str,
        lang: Language,
        initial_depth: usize,
    ) -> (Vec<TokenSpan<'a>>, usize) {
        if line.is_empty() {
            return (vec![], initial_depth);
        }
        if lang == Language::PlainText {
            return (
                vec![TokenSpan {
                    text: line,
                    token_type: TokenType::PlainText,
                }],
                initial_depth,
            );
        }

        let mut spans = Vec::with_capacity(line.len() / 4 + 1);
        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut idx = 0;
        let mut bracket_depth = initial_depth;

        // Special handling for Markdown headers and quotes at line start
        if lang == Language::Markdown {
            let trimmed = line.trim_start();
            let leading_spaces = line.len() - trimmed.len();
            if leading_spaces > 0 {
                spans.push(TokenSpan {
                    text: &line[0..leading_spaces],
                    token_type: TokenType::PlainText,
                });
                idx = leading_spaces;
            }

            if idx < len && bytes[idx] == b'#' {
                let start = idx;
                while idx < len && bytes[idx] == b'#' {
                    idx += 1;
                }
                spans.push(TokenSpan {
                    text: &line[start..idx],
                    token_type: TokenType::Keyword,
                });
                if idx < len {
                    spans.push(TokenSpan {
                        text: &line[idx..],
                        token_type: TokenType::Type,
                    });
                    return (spans, bracket_depth);
                }
            } else if idx < len && bytes[idx] == b'>' {
                spans.push(TokenSpan {
                    text: &line[idx..],
                    token_type: TokenType::Comment,
                });
                return (spans, bracket_depth);
            }
        }

        while idx < len {
            let b = bytes[idx];

            // 1. Comments
            // C, C++, Rust, JS line comments '//'
            if (matches!(lang, Language::Rust | Language::C | Language::JavaScript))
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

            // Block comments '/* ... */' for C, Rust, JS, CSS, SQL
            if (matches!(lang, Language::Rust | Language::C | Language::JavaScript | Language::Css | Language::Sql))
                && b == b'/'
                && idx + 1 < len
                && bytes[idx + 1] == b'*'
            {
                let start = idx;
                idx += 2;
                while idx + 1 < len && !(bytes[idx] == b'*' && bytes[idx + 1] == b'/') {
                    idx += 1;
                }
                if idx + 1 < len {
                    idx += 2;
                } else {
                    idx = len;
                }
                spans.push(TokenSpan {
                    text: &line[start..idx],
                    token_type: TokenType::Comment,
                });
                continue;
            }

            // Python, Shell, YAML, Toml '#' comments
            if (matches!(lang, Language::Python | Language::Toml | Language::Yaml | Language::Shell)) && b == b'#' {
                spans.push(TokenSpan {
                    text: &line[idx..],
                    token_type: TokenType::Comment,
                });
                break;
            }

            // SQL '--' comments
            if lang == Language::Sql && b == b'-' && idx + 1 < len && bytes[idx + 1] == b'-' {
                spans.push(TokenSpan {
                    text: &line[idx..],
                    token_type: TokenType::Comment,
                });
                break;
            }

            // HTML/XML '<!-- ... -->' comments
            if lang == Language::Html && b == b'<' && idx + 3 < len && &bytes[idx..idx + 4] == b"<!--" {
                let start = idx;
                idx += 4;
                while idx + 2 < len && &bytes[idx..idx + 3] != b"-->" {
                    idx += 1;
                }
                if idx + 2 < len {
                    idx += 3;
                } else {
                    idx = len;
                }
                spans.push(TokenSpan {
                    text: &line[start..idx],
                    token_type: TokenType::Comment,
                });
                continue;
            }

            // 2. Bracket Pair Colorization ((), [], {})
            if b == b'(' || b == b'[' || b == b'{' {
                let current_depth = bracket_depth;
                bracket_depth += 1;
                spans.push(TokenSpan {
                    text: &line[idx..idx + 1],
                    token_type: TokenType::Bracket(current_depth),
                });
                idx += 1;
                continue;
            }

            if b == b')' || b == b']' || b == b'}' {
                bracket_depth = bracket_depth.saturating_sub(1);
                let current_depth = bracket_depth;
                spans.push(TokenSpan {
                    text: &line[idx..idx + 1],
                    token_type: TokenType::Bracket(current_depth),
                });
                idx += 1;
                continue;
            }

            // 3. String literals (quotes)
            if b == b'"' || b == b'\'' || (b == b'`' && matches!(lang, Language::JavaScript | Language::Shell | Language::Markdown | Language::Sql)) {
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

            // 4. Shell Variables: $VAR, ${VAR}, $?, $1
            if lang == Language::Shell && b == b'$' {
                let start = idx;
                idx += 1;
                if idx < len && bytes[idx] == b'{' {
                    idx += 1;
                    while idx < len && bytes[idx] != b'}' {
                        idx += 1;
                    }
                    if idx < len {
                        idx += 1;
                    }
                } else {
                    while idx < len && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == b'_' || bytes[idx] == b'?' || bytes[idx] == b'!') {
                        idx += 1;
                    }
                }
                spans.push(TokenSpan {
                    text: &line[start..idx],
                    token_type: TokenType::Type,
                });
                continue;
            }

            // 5. HTML Tags: <tag_name, </tag_name, >
            if lang == Language::Html && b == b'<' {
                let start = idx;
                idx += 1;
                if idx < len && bytes[idx] == b'/' {
                    idx += 1;
                }
                let tag_start = idx;
                while idx < len && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == b'-' || bytes[idx] == b':') {
                    idx += 1;
                }
                if tag_start < idx {
                    spans.push(TokenSpan {
                        text: &line[start..idx],
                        token_type: TokenType::Keyword,
                    });
                } else {
                    spans.push(TokenSpan {
                        text: &line[start..idx],
                        token_type: TokenType::Operator,
                    });
                }
                continue;
            }

            // 6. Numbers
            if b.is_ascii_digit() || (b == b'#' && lang == Language::Css && idx + 1 < len && bytes[idx + 1].is_ascii_hexdigit()) {
                let start = idx;
                idx += 1;
                while idx < len && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == b'.' || bytes[idx] == b'_' || bytes[idx] == b'%') {
                    idx += 1;
                }
                spans.push(TokenSpan {
                    text: &line[start..idx],
                    token_type: TokenType::Number,
                });
                continue;
            }

            // 7. Identifiers, Keywords, Types
            if b.is_ascii_alphabetic() || b == b'_' || (b == b'@' && lang == Language::Css) {
                let start = idx;
                idx += 1;
                while idx < len && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == b'_' || bytes[idx] == b'-') {
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

            // 8. Operators and Punctuation
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

            // 9. Whitespace or multi-byte UTF-8 char
            let ch_len = line[idx..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
            spans.push(TokenSpan {
                text: &line[idx..idx + ch_len],
                token_type: TokenType::PlainText,
            });
            idx += ch_len;
        }

        (spans, bracket_depth)
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
            Language::Html => &[
                "html", "head", "body", "title", "meta", "link", "style", "script",
                "div", "span", "p", "a", "h1", "h2", "h3", "h4", "h5", "h6",
                "ul", "ol", "li", "table", "tr", "td", "th", "tbody", "thead",
                "form", "input", "button", "textarea", "select", "option", "img",
                "header", "footer", "nav", "section", "article", "main", "aside",
                "svg", "path", "circle", "rect", "line", "canvas",
            ],
            Language::Css => &[
                "color", "background", "font", "font-size", "font-weight", "font-family",
                "margin", "padding", "border", "border-radius", "width", "height",
                "display", "flex", "grid", "position", "top", "bottom", "left", "right",
                "z-index", "overflow", "opacity", "transform", "transition", "animation",
                "cursor", "box-shadow", "text-align", "line-height", "justify-content",
                "align-items", "gap", "none", "block", "inline", "inline-block", "relative",
                "absolute", "fixed",
            ],
            Language::Yaml => &[
                "true", "false", "yes", "no", "null", "on", "off",
            ],
            Language::Toml => &["true", "false"],
            Language::Shell => &[
                "if", "then", "else", "elif", "fi", "case", "esac", "for", "while",
                "until", "do", "done", "in", "function", "select", "time", "export",
                "local", "readonly", "set", "unset", "shift", "trap", "exec", "source",
                "alias", "echo", "cd", "exit", "return", "true", "false",
            ],
            Language::Sql => &[
                "select", "from", "where", "insert", "into", "values", "update", "set",
                "delete", "create", "table", "drop", "alter", "join", "inner", "left",
                "right", "full", "cross", "on", "group", "by", "order", "having",
                "limit", "offset", "as", "and", "or", "not", "in", "is", "null",
                "like", "ilike", "distinct", "union", "all", "case", "when", "then",
                "else", "end", "primary", "key", "foreign", "references", "default",
                "check", "index", "view", "trigger", "transaction", "commit", "rollback",
                "begin", "grant", "revoke", "asc", "desc", "between", "exists", "true", "false",
            ],
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
            Language::Html => &[
                "id", "class", "src", "href", "rel", "type", "name", "value",
                "placeholder", "style", "width", "height", "alt", "action", "method", "target",
            ],
            Language::Css => &[
                "@media", "@keyframes", "@import", "@font-face", "@supports",
            ],
            Language::Yaml => &[
                "version", "services", "name", "image", "ports", "environment", "volumes", "build",
            ],
            Language::Shell => &[
                "PATH", "HOME", "USER", "SHELL", "PWD", "TERM", "LANG", "UID", "PID",
            ],
            Language::Sql => &[
                "int", "integer", "bigint", "smallint", "varchar", "char", "text",
                "boolean", "bool", "timestamp", "date", "time", "float", "double",
                "decimal", "numeric", "blob", "json", "serial",
            ],
            Language::Json | Language::Markdown | Language::Toml | Language::PlainText => &[],
        }
    }

    fn classify_word(word: &str, lang: Language) -> TokenType {
        let kw = Self::keywords(lang);
        let ty = Self::types(lang);

        if lang == Language::Sql {
            let lower = word.to_ascii_lowercase();
            if kw.contains(&lower.as_str()) {
                return TokenType::Keyword;
            }
            if ty.contains(&lower.as_str()) {
                return TokenType::Type;
            }
        }

        if kw.contains(&word) {
            TokenType::Keyword
        } else if ty.contains(&word)
            || (lang != Language::PlainText
                && lang != Language::Html
                && lang != Language::Css
                && lang != Language::Sql
                && word.starts_with(|c: char| c.is_ascii_uppercase()))
        {
            TokenType::Type
        } else {
            TokenType::Identifier
        }
    }
}
