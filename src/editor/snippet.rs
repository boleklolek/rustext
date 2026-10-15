//! Intelligent code snippet and template expansion engine.
//! Pure standard-library Rust with zero external dependencies.

use crate::core::syntax::Language;

/// A single tabstop (placeholder) inside an expanded snippet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnippetTabstop {
    /// 1-based tabstop order (1, 2, ...). Index 0 is the final exit location.
    pub index: usize,
    /// Character offset relative to snippet start.
    pub offset: usize,
    /// Length of placeholder text (0 for pure cursor placement).
    pub len: usize,
    /// Default placeholder label.
    pub placeholder: String,
}

/// An expanded snippet ready for insertion into the document buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedSnippet {
    pub text: String,
    pub tabstops: Vec<SnippetTabstop>,
}

impl ExpandedSnippet {
    /// Parses template syntax containing `$0`, `$1`, `${1:placeholder}`.
    pub fn parse(template: &str) -> Self {
        let mut text = String::new();
        let mut tabstops = Vec::new();
        let chars: Vec<char> = template.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '$' && i + 1 < chars.len() {
                // Escaped literal: $$
                if chars[i + 1] == '$' {
                    text.push('$');
                    i += 2;
                    continue;
                }

                // Placeholder format: ${1:name} or ${1}
                if chars[i + 1] == '{' {
                    if let Some(close_pos) = chars[i + 2..].iter().position(|&c| c == '}') {
                        let inner: String = chars[i + 2..i + 2 + close_pos].iter().collect();
                        let (idx_part, default_part) = match inner.split_once(':') {
                            Some((idx, def)) => (idx.trim(), def),
                            None => (inner.trim(), ""),
                        };

                        if let Ok(idx) = idx_part.parse::<usize>() {
                            let offset = text.chars().count();
                            let len = default_part.chars().count();
                            tabstops.push(SnippetTabstop {
                                index: idx,
                                offset,
                                len,
                                placeholder: default_part.to_string(),
                            });
                            text.push_str(default_part);
                            i = i + 2 + close_pos + 1;
                            continue;
                        }
                    }
                }

                // Simple tabstop format: $0, $1, $2...
                if chars[i + 1].is_ascii_digit() {
                    let mut end = i + 1;
                    while end < chars.len() && chars[end].is_ascii_digit() {
                        end += 1;
                    }
                    let digit_str: String = chars[i + 1..end].iter().collect();
                    if let Ok(idx) = digit_str.parse::<usize>() {
                        let offset = text.chars().count();
                        tabstops.push(SnippetTabstop {
                            index: idx,
                            offset,
                            len: 0,
                            placeholder: String::new(),
                        });
                        i = end;
                        continue;
                    }
                }
            }

            text.push(chars[i]);
            i += 1;
        }

        // Ensure a final exit tabstop exists ($0)
        if !tabstops.iter().any(|ts| ts.index == 0) {
            tabstops.push(SnippetTabstop {
                index: 0,
                offset: text.chars().count(),
                len: 0,
                placeholder: String::new(),
            });
        }

        Self { text, tabstops }
    }
}

/// A snippet template definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    pub trigger: &'static str,
    pub description: &'static str,
    pub template: &'static str,
    pub language: Language,
}

impl Snippet {
    pub const fn new(
        trigger: &'static str,
        description: &'static str,
        template: &'static str,
        language: Language,
    ) -> Self {
        Self {
            trigger,
            description,
            template,
            language,
        }
    }

    /// Expands this snippet while preserving the line's base indentation.
    pub fn expand(&self, base_indent: &str) -> ExpandedSnippet {
        if base_indent.is_empty() {
            return ExpandedSnippet::parse(self.template);
        }

        let mut indented = String::new();
        let lines: Vec<&str> = self.template.lines().collect();

        for (idx, line) in lines.iter().enumerate() {
            if idx > 0 {
                indented.push_str(base_indent);
            }
            indented.push_str(line);
            if idx + 1 < lines.len() || self.template.ends_with('\n') {
                indented.push('\n');
            }
        }

        ExpandedSnippet::parse(&indented)
    }
}

/// Active snippet session tracking placeholder progression and text offsets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnippetSession {
    /// Buffer character offset where snippet starts.
    pub start_char: usize,
    /// Current step in `tabstops` vector.
    pub current_step: usize,
    /// Ordered tabstops (1, 2, 3... followed by 0).
    pub tabstops: Vec<SnippetTabstop>,
}

impl SnippetSession {
    /// Creates a new snippet session from an expanded snippet inserted at `start_char`.
    pub fn new(start_char: usize, mut tabstops: Vec<SnippetTabstop>) -> Self {
        // Sort tabstops: positive indices ascending (1, 2, 3...), followed by 0
        tabstops.sort_by(|a, b| {
            let key_a = if a.index == 0 { usize::MAX } else { a.index };
            let key_b = if b.index == 0 { usize::MAX } else { b.index };
            key_a.cmp(&key_b)
        });

        Self {
            start_char,
            current_step: 0,
            tabstops,
        }
    }

    /// Returns the selection range `(anchor, head)` for the current tabstop.
    pub fn current_selection(&self) -> Option<(usize, usize)> {
        if self.tabstops.is_empty() || self.current_step >= self.tabstops.len() {
            return None;
        }

        let ts = &self.tabstops[self.current_step];
        let sel_start = self.start_char + ts.offset;
        let sel_end = sel_start + ts.len;
        Some((sel_start, sel_end))
    }

    /// Advances to the next tabstop. Returns `Some((start, end))` or `None` if completed.
    pub fn next_tabstop(&mut self) -> Option<(usize, usize)> {
        if self.current_step + 1 < self.tabstops.len() {
            self.current_step += 1;
            self.current_selection()
        } else {
            None
        }
    }

    /// Moves back to the previous tabstop.
    pub fn prev_tabstop(&mut self) -> Option<(usize, usize)> {
        if self.current_step > 0 {
            self.current_step -= 1;
            self.current_selection()
        } else {
            self.current_selection()
        }
    }

    /// Updates internal tabstop offsets when the buffer text is modified.
    pub fn on_buffer_edit(&mut self, edit_start: usize, old_len: usize, new_len: usize) {
        let diff = new_len as isize - old_len as isize;
        if diff == 0 {
            return;
        }

        let rel_edit = edit_start.saturating_sub(self.start_char);

        for (i, ts) in self.tabstops.iter_mut().enumerate() {
            let ts_end = ts.offset + ts.len;
            if rel_edit < ts.offset {
                // Edit happened before this tabstop: shift offset
                ts.offset = (ts.offset as isize + diff).max(0) as usize;
            } else if rel_edit <= ts_end && i == self.current_step {
                // Edit happened inside currently active placeholder: adjust length
                ts.len = (ts.len as isize + diff).max(0) as usize;
            }
        }
    }
}

/// Central registry of language snippets.
pub struct SnippetRegistry;

impl SnippetRegistry {
    /// Built-in snippets catalog across all supported languages.
    pub const SNIPPETS: &'static [Snippet] = &[
        // ==================== Rust ====================
        Snippet::new(
            "fn",
            "Definición de función",
            "fn ${1:name}(${2:params}) -> ${3:Result<(), Box<dyn std::error::Error>>} {\n    ${0:todo!()}\n}",
            Language::Rust,
        ),
        Snippet::new(
            "pfn",
            "Función pública",
            "pub fn ${1:name}(${2:params}) -> ${3:()} {\n    ${0}\n}",
            Language::Rust,
        ),
        Snippet::new(
            "struct",
            "Estructura de datos",
            "#[derive(Debug, Clone)]\npub struct ${1:Name} {\n    ${0}\n}",
            Language::Rust,
        ),
        Snippet::new(
            "enum",
            "Enumeración",
            "#[derive(Debug, Clone, PartialEq, Eq)]\npub enum ${1:Name} {\n    ${0}\n}",
            Language::Rust,
        ),
        Snippet::new(
            "impl",
            "Bloque de implementación",
            "impl ${1:Type} {\n    ${0}\n}",
            Language::Rust,
        ),
        Snippet::new(
            "for",
            "Bucle iterativo",
            "for ${1:item} in ${2:iter} {\n    ${0}\n}",
            Language::Rust,
        ),
        Snippet::new(
            "match",
            "Expresión match",
            "match ${1:expr} {\n    ${2:pattern} => ${3:expr},\n    ${0:_ => todo!(),}\n}",
            Language::Rust,
        ),
        Snippet::new(
            "if",
            "Condicional if",
            "if ${1:condition} {\n    ${0}\n}",
            Language::Rust,
        ),
        Snippet::new(
            "test",
            "Test unitario",
            "#[test]\nfn ${1:test_name}() {\n    ${0:assert!(true);}\n}",
            Language::Rust,
        ),
        Snippet::new(
            "pl",
            "Macro println!",
            "println!(\"${1:message}\", ${0});",
            Language::Rust,
        ),
        Snippet::new(
            "println",
            "Macro println!",
            "println!(\"${1:message}\", ${0});",
            Language::Rust,
        ),
        Snippet::new(
            "main",
            "Punto de entrada fn main()",
            "fn main() {\n    ${0:println!(\"Hello, world!\");}\n}",
            Language::Rust,
        ),
        Snippet::new(
            "mod",
            "Declaración de módulo",
            "pub mod ${1:name};\n",
            Language::Rust,
        ),

        // ==================== Python ====================
        Snippet::new(
            "def",
            "Definición de función",
            "def ${1:name}(${2:args}):\n    \"\"\"${3:Docstring}\"\"\"\n    ${0:pass}",
            Language::Python,
        ),
        Snippet::new(
            "class",
            "Definición de clase",
            "class ${1:ClassName}:\n    def __init__(self${2:, args}):\n        ${0:pass}",
            Language::Python,
        ),
        Snippet::new(
            "if",
            "Condicional if",
            "if ${1:condition}:\n    ${0:pass}",
            Language::Python,
        ),
        Snippet::new(
            "for",
            "Bucle for in",
            "for ${1:item} in ${2:iterable}:\n    ${0:pass}",
            Language::Python,
        ),
        Snippet::new(
            "main",
            "Bloque if __name__ == '__main__'",
            "if __name__ == \"__main__\":\n    ${0:main()}",
            Language::Python,
        ),
        Snippet::new(
            "try",
            "Bloque try-except",
            "try:\n    ${1:pass}\nexcept ${2:Exception} as ${3:e}:\n    ${0:pass}",
            Language::Python,
        ),
        Snippet::new(
            "pr",
            "print(f\"...\")",
            "print(f\"${1:message}\")",
            Language::Python,
        ),
        Snippet::new(
            "print",
            "print(f\"...\")",
            "print(f\"${1:message}\")",
            Language::Python,
        ),

        // ==================== C / C++ ====================
        Snippet::new(
            "main",
            "Función principal main",
            "int main(int argc, char *argv[]) {\n    ${0:return 0;}\n}",
            Language::C,
        ),
        Snippet::new(
            "fn",
            "Definición de función",
            "${1:void} ${2:name}(${3:void}) {\n    ${0}\n}",
            Language::C,
        ),
        Snippet::new(
            "struct",
            "Estructura typedef struct",
            "typedef struct ${1:Name} {\n    ${0}\n} ${1:Name};",
            Language::C,
        ),
        Snippet::new(
            "for",
            "Bucle for indexado",
            "for (int ${1:i} = 0; ${1:i} < ${2:count}; ${1:i}++) {\n    ${0}\n}",
            Language::C,
        ),
        Snippet::new(
            "if",
            "Condicional if",
            "if (${1:condition}) {\n    ${0}\n}",
            Language::C,
        ),
        Snippet::new(
            "inc",
            "Directiva #include",
            "#include <${1:stdio.h}>\n",
            Language::C,
        ),

        // ==================== JavaScript / TypeScript ====================
        Snippet::new(
            "fn",
            "Función estándar",
            "function ${1:name}(${2:params}) {\n    ${0}\n}",
            Language::JavaScript,
        ),
        Snippet::new(
            "afn",
            "Función flecha (arrow)",
            "const ${1:name} = (${2:params}) => {\n    ${0}\n};",
            Language::JavaScript,
        ),
        Snippet::new(
            "clg",
            "console.log",
            "console.log(${0});",
            Language::JavaScript,
        ),
        Snippet::new(
            "log",
            "console.log",
            "console.log(${0});",
            Language::JavaScript,
        ),
        Snippet::new(
            "for",
            "Bucle for...of",
            "for (const ${1:item} of ${2:items}) {\n    ${0}\n}",
            Language::JavaScript,
        ),
        Snippet::new(
            "if",
            "Condicional if",
            "if (${1:condition}) {\n    ${0}\n}",
            Language::JavaScript,
        ),
        Snippet::new(
            "try",
            "Bloque try-catch",
            "try {\n    ${1}\n} catch (${2:error}) {\n    ${0}\n}",
            Language::JavaScript,
        ),

        // ==================== Markdown ====================
        Snippet::new(
            "link",
            "Enlace markdown [texto](url)",
            "[${1:text}](${2:url})",
            Language::Markdown,
        ),
        Snippet::new(
            "img",
            "Imagen markdown ![alt](url)",
            "![${1:alt}](${2:url})",
            Language::Markdown,
        ),
        Snippet::new(
            "code",
            "Bloque de código con resaltado",
            "```${1:rust}\n${0}\n```",
            Language::Markdown,
        ),

        // ==================== JSON ====================
        Snippet::new(
            "obj",
            "Objeto JSON",
            "{\n  \"${1:key}\": \"${2:value}\"\n}",
            Language::Json,
        ),
        Snippet::new(
            "arr",
            "Arreglo JSON",
            "[\n  ${0}\n]",
            Language::Json,
        ),

        // ==================== TOML ====================
        Snippet::new(
            "pkg",
            "Sección [package]",
            "[package]\nname = \"${1:name}\"\nversion = \"${2:0.1.0}\"\nedition = \"2024\"\n",
            Language::Toml,
        ),
        Snippet::new(
            "sec",
            "Nueva sección [seccion]",
            "[${1:section}]\n${0}",
            Language::Toml,
        ),

        // ==================== Plain Text ====================
        Snippet::new(
            "todo",
            "Elemento TODO",
            "TODO: ${1:tarea} [${0}]",
            Language::PlainText,
        ),
    ];

    /// Finds a snippet by language and trigger keyword.
    pub fn find(language: Language, trigger: &str) -> Option<Snippet> {
        let t_clean = trigger.trim();
        // 1. Exact language match
        for snip in Self::SNIPPETS {
            if snip.language == language && snip.trigger == t_clean {
                return Some(snip.clone());
            }
        }
        // 2. Fallback to PlainText universal snippets
        if language != Language::PlainText {
            for snip in Self::SNIPPETS {
                if snip.language == Language::PlainText && snip.trigger == t_clean {
                    return Some(snip.clone());
                }
            }
        }
        None
    }

    /// Lists all snippets available for the given language.
    pub fn list_for_language(language: Language) -> Vec<Snippet> {
        Self::SNIPPETS
            .iter()
            .filter(|s| s.language == language || s.language == Language::PlainText)
            .cloned()
            .collect()
    }
}
