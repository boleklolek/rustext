//! Zero-dependency source code formatting module.
//! Interfaces safely with system formatters (rustfmt, black, clang-format, prettier)
//! via standard piped process streams without external crates.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use crate::core::syntax::Language;

/// Formats source code for a given language using system tools if installed.
/// Returns Ok(formatted_string) on success, or Err(error_message) without modifying original code.
pub fn format_code(
    language: Language,
    code: &str,
    file_path: Option<&Path>,
) -> Result<String, String> {
    match language {
        Language::Rust => format_rust(code),
        Language::Python => format_python(code),
        Language::C => format_c(code),
        Language::JavaScript | Language::Json | Language::Markdown => {
            format_prettier(language, code, file_path)
        }
        Language::Toml | Language::PlainText => {
            Err(format!("No hay formateador por defecto para {}", language.name()))
        }
    }
}

/// Formats Rust code using `rustfmt --emit stdout`.
fn format_rust(code: &str) -> Result<String, String> {
    let mut cmd = Command::new("rustfmt");
    cmd.args(["--emit", "stdout", "--edition", "2021"]);
    execute_formatter(cmd, code)
}

/// Formats Python code using `black -q -` or `ruff format -`.
fn format_python(code: &str) -> Result<String, String> {
    // Try black first
    let mut cmd = Command::new("black");
    cmd.args(["-q", "-"]);
    if let Ok(formatted) = execute_formatter(cmd, code) {
        return Ok(formatted);
    }

    // Fallback to ruff
    let mut ruff_cmd = Command::new("ruff");
    ruff_cmd.args(["format", "-"]);
    execute_formatter(ruff_cmd, code)
}

/// Formats C/C++ code using `clang-format`.
fn format_c(code: &str) -> Result<String, String> {
    let mut cmd = Command::new("clang-format");
    cmd.args(["-assume-filename=main.c"]);
    execute_formatter(cmd, code)
}

/// Formats JS/JSON/MD using `prettier`.
fn format_prettier(language: Language, code: &str, file_path: Option<&Path>) -> Result<String, String> {
    let mut cmd = Command::new("prettier");
    let parser = match language {
        Language::JavaScript => "babel",
        Language::Json => "json",
        Language::Markdown => "markdown",
        _ => "babel",
    };

    if let Some(path) = file_path {
        cmd.args(["--stdin-filepath", path.to_str().unwrap_or("file")]);
    } else {
        cmd.args(["--parser", parser]);
    }

    execute_formatter(cmd, code)
}

/// Spawns a formatter process, writes source code to stdin, and collects formatted stdout.
fn execute_formatter(mut cmd: Command, code: &str) -> Result<String, String> {
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Herramienta de formateo no disponible ({})", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(code.as_bytes())
            .map_err(|e| format!("Error enviando código al formateador: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Error esperando respuesta del formateador: {}", e))?;

    if output.status.success() {
        let formatted = String::from_utf8(output.stdout)
            .map_err(|e| format!("Salida del formateador no es UTF-8 válido: {}", e))?;
        if formatted.is_empty() && !code.is_empty() {
            Err("Formateador retornó salida vacía".to_string())
        } else {
            Ok(formatted)
        }
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("Error en el formateador: {}", err_msg.trim()))
    }
}
