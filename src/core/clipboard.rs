//! Portapapeles (Clipboard) nativo sin crates externas.
//! Soporta almacenamiento en memoria, secuencias OSC 52 de terminal,
//! y comandos estándar del sistema (wl-copy / xclip).

use std::io::{self, Write};
use std::process::{Command, Stdio};

/// Portapapeles autónomo de alta compatibilidad.
#[derive(Debug, Default, Clone)]
pub struct Clipboard {
    internal_buffer: String,
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            internal_buffer: String::new(),
        }
    }

    /// Guarda texto en el portapapeles interno y emite secuencias OSC 52 y utilidades de sistema.
    pub fn set_text(&mut self, text: &str) {
        self.internal_buffer = text.to_string();

        // 1. Emitir secuencia OSC 52 al terminal para copiar nativamente en Kitty, Alacritty, WezTerm, etc.
        let b64 = Self::base64_encode(text.as_bytes());
        let osc52 = format!("\x1b]52;c;{}\x1b\\", b64);
        let _ = io::stdout().write_all(osc52.as_bytes());
        let _ = io::stdout().flush();

        // 2. Intentar wl-copy (Wayland) o xclip (X11)
        Self::try_system_copy(text);
    }

    /// Obtiene el texto del portapapeles (prioriza búfer interno de sesión; si está vacío, lee sistema).
    pub fn get_text(&self) -> String {
        if !self.internal_buffer.is_empty() {
            self.internal_buffer.clone()
        } else if let Some(sys_text) = Self::try_system_paste() {
            sys_text
        } else {
            String::new()
        }
    }

    /// Lee forzosamente del portapapeles del sistema operativo si está disponible.
    pub fn get_system_text(&self) -> Option<String> {
        Self::try_system_paste()
    }

    /// Codificador Base64 en Rust puro (std) para compatibilidad con OSC 52.
    pub fn base64_encode(data: &[u8]) -> String {
        const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::with_capacity((data.len() + 2) / 3 * 4);

        for chunk in data.chunks(3) {
            let b0 = chunk[0];
            let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

            let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);

            result.push(CHARSET[((n >> 18) & 63) as usize] as char);
            result.push(CHARSET[((n >> 12) & 63) as usize] as char);

            if chunk.len() > 1 {
                result.push(CHARSET[((n >> 6) & 63) as usize] as char);
            } else {
                result.push('=');
            }

            if chunk.len() > 2 {
                result.push(CHARSET[(n & 63) as usize] as char);
            } else {
                result.push('=');
            }
        }

        result
    }

    fn try_system_copy(text: &str) {
        #[cfg(target_os = "macos")]
        {
            if let Ok(mut child) = Command::new("pbcopy")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
                return;
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(mut child) = Command::new("clip")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
                return;
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Try wl-copy (Wayland) via stdin to prevent ARG_MAX buffer limits and CLI injection (SEC-03)
            if let Ok(mut child) = Command::new("wl-copy")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                if let Ok(status) = child.wait() {
                    if status.success() {
                        return;
                    }
                }
            }

            // Try xclip (X11)
            if let Ok(mut child) = Command::new("xclip")
                .arg("-selection")
                .arg("clipboard")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
                return;
            }

            // Try xsel (X11 fallback)
            if let Ok(mut child) = Command::new("xsel")
                .arg("--clipboard")
                .arg("--input")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
            }
        }
    }

    fn try_system_paste() -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("pbpaste").output() {
                if output.status.success() {
                    if let Ok(s) = String::from_utf8(output.stdout) {
                        return Some(s);
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", "Get-Clipboard -Raw"])
                .output()
            {
                if output.status.success() {
                    if let Ok(s) = String::from_utf8(output.stdout) {
                        return Some(s.replace("\r\n", "\n"));
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Try wl-paste (Wayland)
            if let Ok(output) = Command::new("wl-paste")
                .arg("-n")
                .output()
            {
                if output.status.success() {
                    if let Ok(s) = String::from_utf8(output.stdout) {
                        return Some(s);
                    }
                }
            }

            // Try xclip (X11)
            if let Ok(output) = Command::new("xclip")
                .arg("-selection")
                .arg("clipboard")
                .arg("-o")
                .output()
            {
                if output.status.success() {
                    if let Ok(s) = String::from_utf8(output.stdout) {
                        return Some(s);
                    }
                }
            }

            // Try xsel (X11 fallback)
            if let Ok(output) = Command::new("xsel")
                .arg("--clipboard")
                .arg("--output")
                .output()
            {
                if output.status.success() {
                    if let Ok(s) = String::from_utf8(output.stdout) {
                        return Some(s);
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode() {
        assert_eq!(Clipboard::base64_encode(b"hello"), "aGVsbG8=");
        assert_eq!(Clipboard::base64_encode(b"Rustext"), "UnVzdGV4dA==");
        assert_eq!(Clipboard::base64_encode("🦀".as_bytes()), "8J+mgA==");
    }

    #[test]
    fn test_internal_clipboard() {
        let mut cb = Clipboard::new();
        cb.internal_buffer = "Copiado".to_string();
        assert_eq!(cb.internal_buffer, "Copiado");
    }
}
