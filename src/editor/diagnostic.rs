//! Diagnostic and compiler/linter error reporting system.
//! Pure standard-library Rust with zero third-party dependencies.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use crate::core::syntax::Language;

/// Severity level of a diagnostic report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

impl DiagnosticSeverity {
    pub fn badge(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "E",
            DiagnosticSeverity::Warning => "W",
            DiagnosticSeverity::Information => "I",
            DiagnosticSeverity::Hint => "H",
        }
    }

    pub fn color_hex(&self) -> u32 {
        match self {
            DiagnosticSeverity::Error => 0x00FF5555,      // Red
            DiagnosticSeverity::Warning => 0x00FFB86C,    // Amber / Yellow
            DiagnosticSeverity::Information => 0x008BE9FD,// Cyan
            DiagnosticSeverity::Hint => 0x006272A4,       // Muted Purple
        }
    }
}

/// A single diagnostic finding reported by a compiler or linter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub file: Option<PathBuf>,
    pub line: usize,   // 1-indexed
    pub col: usize,    // 1-indexed
    pub end_col: Option<usize>, // 1-indexed column end
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub code: Option<String>,
}

impl Diagnostic {
    pub fn new(
        file: Option<PathBuf>,
        line: usize,
        col: usize,
        severity: DiagnosticSeverity,
        message: String,
        code: Option<String>,
    ) -> Self {
        Self {
            file,
            line: line.max(1),
            col: col.max(1),
            end_col: None,
            severity,
            message,
            code,
        }
    }

    /// Sets the ending column (1-indexed, inclusive or exclusive) for the diagnostic span.
    pub fn with_end_col(mut self, end_col: usize) -> Self {
        self.end_col = Some(end_col.max(self.col));
        self
    }

    /// Formats diagnostic for single-line display in status bar or command palette.
    pub fn to_display_string(&self) -> String {
        let code_str = self
            .code
            .as_ref()
            .map(|c| format!(" [{}]", c))
            .unwrap_or_default();
        let sev_str = match self.severity {
            DiagnosticSeverity::Error => "Error",
            DiagnosticSeverity::Warning => "Aviso",
            DiagnosticSeverity::Information => "Info",
            DiagnosticSeverity::Hint => "Pista",
        };
        format!("{}{} (L{}:{}): {}", sev_str, code_str, self.line, self.col, self.message)
    }
}

/// Collection of diagnostics currently active for open documents.
#[derive(Debug, Clone, Default)]
pub struct DiagnosticCollection {
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticCollection {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }

    pub fn set(&mut self, diagnostics: Vec<Diagnostic>) {
        self.diagnostics = diagnostics;
        self.diagnostics.sort_by_key(|d| (d.line, d.col, d.severity));
    }

    /// Replaces diagnostics for a specific file, preserving diagnostics for other files.
    pub fn update_for_file(&mut self, file: &Path, new_diags: Vec<Diagnostic>) {
        self.diagnostics.retain(|d| {
            if let Some(df) = &d.file {
                !(df == file || file.ends_with(df) || df.ends_with(file))
            } else {
                false
            }
        });
        self.diagnostics.extend(new_diags);
        self.diagnostics.sort_by_key(|d| (d.line, d.col, d.severity));
    }

    pub fn add(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
        self.diagnostics.sort_by_key(|d| (d.line, d.col, d.severity));
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn all(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == DiagnosticSeverity::Error)
    }

    pub fn has_warnings(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == DiagnosticSeverity::Warning)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == DiagnosticSeverity::Error).count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == DiagnosticSeverity::Warning).count()
    }

    /// Returns the most critical diagnostic on a specific 1-indexed line.
    pub fn for_line(&self, line: usize) -> Option<&Diagnostic> {
        self.diagnostics.iter().find(|d| d.line == line)
    }

    /// Returns all diagnostics on a specific 1-indexed line matching the given file path.
    pub fn for_line_and_file(&self, line: usize, doc_path: Option<&Path>) -> Vec<&Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| {
                if d.line != line {
                    return false;
                }
                match (&d.file, doc_path) {
                    (Some(df), Some(p)) => {
                        df == p || p.ends_with(df) || df.ends_with(p)
                    }
                    (None, _) => true,
                    _ => false,
                }
            })
            .collect()
    }

    /// Finds the next diagnostic after the given line.
    pub fn next_diagnostic(&self, current_line: usize) -> Option<&Diagnostic> {
        if self.diagnostics.is_empty() {
            return None;
        }
        self.diagnostics
            .iter()
            .find(|d| d.line > current_line)
            .or_else(|| self.diagnostics.first())
    }

    /// Finds the previous diagnostic before the given line.
    pub fn prev_diagnostic(&self, current_line: usize) -> Option<&Diagnostic> {
        if self.diagnostics.is_empty() {
            return None;
        }
        self.diagnostics
            .iter()
            .rfind(|d| d.line < current_line)
            .or_else(|| self.diagnostics.last())
    }

    /// Runs compiler/linter check for a file or in-memory code.
    pub fn run_check(
        language: Language,
        file_path: Option<&Path>,
        _content: &str,
    ) -> Result<Vec<Diagnostic>, String> {
        match language {
            Language::Rust => Self::check_rust(file_path),
            Language::Python => Self::check_python(file_path),
            Language::C => Self::check_c(file_path),
            Language::Shell => Self::check_shell(file_path),
            _ => Ok(Vec::new()),
        }
    }

    /// Runs `cargo check --message-format=json` for Rust projects.
    fn check_rust(file_path: Option<&Path>) -> Result<Vec<Diagnostic>, String> {
        let manifest_dir = if let Some(path) = file_path {
            Self::find_cargo_manifest_dir(path)
        } else {
            None
        };

        let mut cmd = Command::new("cargo");
        cmd.args(["check", "--message-format=json"]);
        if let Some(dir) = manifest_dir {
            cmd.current_dir(dir);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd.output().map_err(|e| format!("cargo check no disponible: {}", e))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed = Self::parse_cargo_json(&stdout);
        Ok(parsed)
    }

    /// Runs `python3 -m py_compile <path>` for Python.
    fn check_python(file_path: Option<&Path>) -> Result<Vec<Diagnostic>, String> {
        let path = match file_path {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };

        let output = Command::new("python3")
            .args(["-m", "py_compile"])
            .arg(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("python3 no disponible: {}", e))?;

        if output.status.success() {
            Ok(Vec::new())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(Self::parse_standard_errors(&stderr, file_path))
        }
    }

    /// Runs `gcc -fsyntax-only` or `clang -fsyntax-only` for C/C++.
    fn check_c(file_path: Option<&Path>) -> Result<Vec<Diagnostic>, String> {
        let path = match file_path {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };

        let output = Command::new("gcc")
            .args(["-fsyntax-only", "-Wall"])
            .arg(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .or_else(|_| {
                Command::new("clang")
                    .args(["-fsyntax-only", "-Wall"])
                    .arg(path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
            })
            .map_err(|e| format!("gcc/clang no disponible: {}", e))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(Self::parse_standard_errors(&stderr, file_path))
    }

    /// Checks Shell script syntax using `bash -n`.
    fn check_shell(file_path: Option<&Path>) -> Result<Vec<Diagnostic>, String> {
        let path = match file_path {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };
        let output = Command::new("bash")
            .args(["-n", path.to_str().unwrap_or("")])
            .output()
            .map_err(|e| format!("Error ejecutando bash -n: {}", e))?;

        if output.status.success() {
            return Ok(Vec::new());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut diags = Vec::new();
        for line in stderr.lines() {
            if let Some(pos) = line.find("line ") {
                let rest = &line[pos + 5..];
                if let Some(colon) = rest.find(':') {
                    if let Ok(line_num) = rest[..colon].trim().parse::<usize>() {
                        let msg = rest[colon + 1..].trim();
                        diags.push(Diagnostic::new(
                            Some(path.to_path_buf()),
                            line_num,
                            1,
                            DiagnosticSeverity::Error,
                            format!("bash: {}", msg),
                            None,
                        ));
                    }
                }
            }
        }
        Ok(diags)
    }

    /// Traverses up directory tree looking for `Cargo.toml`.
    fn find_cargo_manifest_dir(start: &Path) -> Option<PathBuf> {
        let mut curr = if start.is_file() {
            start.parent()?.to_path_buf()
        } else {
            start.to_path_buf()
        };

        loop {
            if curr.join("Cargo.toml").exists() {
                return Some(curr);
            }
            if !curr.pop() {
                break;
            }
        }
        None
    }

    /// Parses compiler messages from `cargo check --message-format=json`.
    pub fn parse_cargo_json(output: &str) -> Vec<Diagnostic> {
        let mut result = Vec::new();

        for line in output.lines() {
            let line = line.trim();
            if !line.starts_with('{') || !line.ends_with('}') {
                continue;
            }
            if !line.contains(r#""reason":"compiler-message""#) {
                continue;
            }

            // Extract level
            let severity = if line.contains(r#""level":"error""#) {
                DiagnosticSeverity::Error
            } else if line.contains(r#""level":"warning""#) {
                DiagnosticSeverity::Warning
            } else {
                DiagnosticSeverity::Information
            };

            // Extract error message
            let msg = Self::extract_json_str(line, r#""message":"#).unwrap_or_default();
            if msg.is_empty() {
                continue;
            }

            // Extract code if any (e.g. "code":{"code":"E0499"...)
            let code = Self::extract_json_nested_code(line);

            // Extract primary span
            let (file, line_num, col_num, col_end) = Self::extract_primary_span(line);

            let mut diag = Diagnostic::new(
                file.map(PathBuf::from),
                line_num,
                col_num,
                severity,
                msg,
                code,
            );
            if let Some(ec) = col_end {
                diag = diag.with_end_col(ec);
            }
            result.push(diag);
        }

        result
    }

    /// Parses standard `path:line:col: error: message` compiler output lines.
    pub fn parse_standard_errors(output: &str, file_fallback: Option<&Path>) -> Vec<Diagnostic> {
        let mut result = Vec::new();

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(5, ':').collect();
            if parts.len() >= 4 {
                let file_part = parts[0].trim();
                let line_num: usize = parts[1].trim().parse().unwrap_or(1);
                let col_num: usize = parts[2].trim().parse().unwrap_or(1);
                let rest = parts[3..].join(":").trim().to_string();

                let severity = if rest.to_lowercase().contains("error") {
                    DiagnosticSeverity::Error
                } else if rest.to_lowercase().contains("warning") {
                    DiagnosticSeverity::Warning
                } else {
                    DiagnosticSeverity::Information
                };

                let path = if !file_part.is_empty() {
                    Some(PathBuf::from(file_part))
                } else {
                    file_fallback.map(|p| p.to_path_buf())
                };

                result.push(Diagnostic::new(
                    path,
                    line_num,
                    col_num,
                    severity,
                    rest,
                    None,
                ));
            }
        }

        result
    }

    fn extract_json_str(json: &str, key: &str) -> Option<String> {
        let needle = if key.ends_with(':') {
            format!("{}\"", key)
        } else if !key.ends_with('"') {
            format!("\"{}\":\"", key)
        } else {
            key.to_string()
        };

        let idx = json.find(&needle)?;
        let rest = &json[idx + needle.len()..];
        let end = rest.find('"')?;
        Some(rest[..end].replace(r#"\""#, "\"").replace(r#"\\"#, "\\"))
    }

    fn extract_json_nested_code(json: &str) -> Option<String> {
        let idx = json.find(r#""code":"#)?;
        let rest = &json[idx + 7..];
        if let Some(inner_idx) = rest.find(r#""code":""#) {
            let rest2 = &rest[inner_idx + 8..];
            let end = rest2.find('"')?;
            return Some(rest2[..end].to_string());
        }
        None
    }

    fn extract_primary_span(json: &str) -> (Option<String>, usize, usize, Option<usize>) {
        // Find "is_primary":true
        if let Some(primary_idx) = json.find(r#""is_primary":true"#) {
            let window_start = primary_idx.saturating_sub(250);
            let window_end = (primary_idx + 250).min(json.len());
            let window = &json[window_start..window_end];

            let file_name = Self::extract_json_str(window, r#""file_name":"#);
            let line_start = Self::extract_json_number(window, r#""line_start":"#).unwrap_or(1);
            let col_start = Self::extract_json_number(window, r#""column_start":"#).unwrap_or(1);
            let col_end = Self::extract_json_number(window, r#""column_end":"#);

            (file_name, line_start, col_start, col_end)
        } else {
            (None, 1, 1, None)
        }
    }

    fn extract_json_number(json: &str, key: &str) -> Option<usize> {
        let idx = json.find(key)?;
        let rest = &json[idx + key.len()..];
        let num_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        num_str.parse().ok()
    }
}
