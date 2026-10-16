//! Integrated Native Terminal Panel.
//! Spawns native OS shell with bi-directional pipes and asynchronous non-blocking I/O.
//! 100% standard library Rust with zero third-party crates.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::ui::terminal::Key;

/// State and process controller for the integrated terminal pane.
pub struct TerminalPanel {
    pub is_visible: bool,
    pub is_focused: bool,
    pub height: usize,
    pub scroll_offset: usize,
    pub lines: Vec<String>,
    pub current_input: String,
    pub cursor_pos: usize,
    pub working_dir: PathBuf,
    pub shell_name: String,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub max_scrollback: usize,

    // Internal process handles
    child_stdin: Option<Arc<Mutex<ChildStdin>>>,
    output_rx: Option<Receiver<String>>,
    #[allow(dead_code)]
    child_handle: Option<Child>,
}

impl Default for TerminalPanel {
    fn default() -> Self {
        Self::new(PathBuf::from("."))
    }
}

impl TerminalPanel {
    /// Creates a new terminal panel docked at the given working directory.
    pub fn new(working_dir: PathBuf) -> Self {
        let canonical_dir = working_dir.canonicalize().unwrap_or(working_dir);
        let shell_name = Self::detect_shell_name();

        let mut panel = Self {
            is_visible: false,
            is_focused: false,
            height: 180,
            scroll_offset: 0,
            lines: Vec::new(),
            current_input: String::new(),
            cursor_pos: 0,
            working_dir: canonical_dir,
            shell_name,
            history: Vec::new(),
            history_idx: None,
            max_scrollback: 1000,
            child_stdin: None,
            output_rx: None,
            child_handle: None,
        };

        panel.lines.push(format!("Rustext Integrated Terminal [{}]", panel.shell_name));
        panel.lines.push(format!("Directorio: {}", panel.working_dir.display()));
        panel.lines.push("Escriba comandos y presione Enter. Ctrl+` para alternar foco.".to_string());
        panel.lines.push(String::new());

        panel
    }

    /// Detects default shell path and display name for current OS.
    pub fn detect_shell() -> (String, Vec<String>, String) {
        if cfg!(target_os = "windows") {
            let comspec = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
            (comspec, vec![], "cmd".to_string())
        } else {
            let shell_env = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
            let shell_path = if Path::new(&shell_env).exists() {
                shell_env
            } else if Path::new("/bin/bash").exists() {
                "/bin/bash".to_string()
            } else {
                "/bin/sh".to_string()
            };

            let name = Path::new(&shell_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("sh")
                .to_string();

            (shell_path, vec![], name)
        }
    }

    pub fn detect_shell_name() -> String {
        let (_, _, name) = Self::detect_shell();
        name
    }

    /// Launches the background shell process if not already running.
    pub fn ensure_spawned(&mut self) {
        if self.child_stdin.is_some() {
            return;
        }

        let (shell_cmd, shell_args, shell_name) = Self::detect_shell();
        self.shell_name = shell_name;

        let mut cmd = Command::new(&shell_cmd);
        cmd.args(&shell_args);
        cmd.current_dir(&self.working_dir);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        match cmd.spawn() {
            Ok(mut child) => {
                let stdin = child.stdin.take();
                let stdout = child.stdout.take();
                let stderr = child.stderr.take();

                let (tx, rx): (Sender<String>, Receiver<String>) = channel();

                if let Some(stdin) = stdin {
                    self.child_stdin = Some(Arc::new(Mutex::new(stdin)));
                }

                // Stdout reader thread
                if let Some(stdout) = stdout {
                    let tx_out = tx.clone();
                    thread::spawn(move || {
                        use std::io::{BufRead, BufReader};
                        let reader = BufReader::new(stdout);
                        for line in reader.lines() {
                            match line {
                                Ok(l) => {
                                    if tx_out.send(l).is_err() {
                                        break;
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                    });
                }

                // Stderr reader thread
                if let Some(stderr) = stderr {
                    let tx_err = tx;
                    thread::spawn(move || {
                        use std::io::{BufRead, BufReader};
                        let reader = BufReader::new(stderr);
                        for line in reader.lines() {
                            match line {
                                Ok(l) => {
                                    if tx_err.send(l).is_err() {
                                        break;
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                    });
                }

                self.output_rx = Some(rx);
                self.child_handle = Some(child);
            }
            Err(e) => {
                self.lines.push(format!("[Error al iniciar shell: {}]", e));
            }
        }
    }

    /// Polls pending output lines from the background worker threads.
    pub fn poll_output(&mut self) -> bool {
        let mut received_any = false;
        if let Some(ref rx) = self.output_rx {
            while let Ok(line) = rx.try_recv() {
                // Strip simple ANSI color escape codes for clean drawing
                let clean_line = Self::strip_ansi_escapes(&line);
                self.lines.push(clean_line);
                if self.lines.len() > self.max_scrollback {
                    let excess = self.lines.len() - self.max_scrollback;
                    self.lines.drain(0..excess);
                }
                received_any = true;
            }
        }
        if received_any {
            self.scroll_to_bottom();
        }
        received_any
    }

    /// Strips VT100 / ANSI escape sequence color codes.
    pub fn strip_ansi_escapes(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut in_escape = false;

        for c in s.chars() {
            if c == '\x1B' {
                in_escape = true;
            } else if in_escape {
                if c.is_ascii_alphabetic() {
                    in_escape = false;
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    /// Toggles visibility of the terminal panel.
    pub fn toggle(&mut self) {
        self.is_visible = !self.is_visible;
        if self.is_visible {
            self.ensure_spawned();
            self.is_focused = true;
            self.scroll_to_bottom();
        } else {
            self.is_focused = false;
        }
    }

    /// Sets the working directory and restarts shell if needed.
    pub fn set_working_dir(&mut self, dir: PathBuf) {
        self.working_dir = dir.canonicalize().unwrap_or(dir);
    }

    /// Sends a raw line to the shell process's stdin.
    pub fn send_line(&mut self, line: &str) {
        self.ensure_spawned();
        if let Some(ref stdin_mutex) = self.child_stdin {
            if let Ok(mut stdin) = stdin_mutex.lock() {
                let _ = writeln!(stdin, "{}", line);
                let _ = stdin.flush();
            }
        }
    }

    /// Clears the scrollback history buffer.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.scroll_offset = 0;
    }

    /// Scrolls view down to the latest output.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.lines.len();
    }

    /// Dispatches key event when terminal panel has input focus.
    pub fn handle_key(&mut self, key: Key) {
        self.ensure_spawned();

        match key {
            Key::Char(c) => {
                if self.cursor_pos >= self.current_input.chars().count() {
                    self.current_input.push(c);
                } else {
                    let mut chars: Vec<char> = self.current_input.chars().collect();
                    chars.insert(self.cursor_pos, c);
                    self.current_input = chars.into_iter().collect();
                }
                self.cursor_pos += 1;
            }
            Key::Backspace => {
                if self.cursor_pos > 0 {
                    let mut chars: Vec<char> = self.current_input.chars().collect();
                    chars.remove(self.cursor_pos - 1);
                    self.current_input = chars.into_iter().collect();
                    self.cursor_pos -= 1;
                }
            }
            Key::Delete => {
                let count = self.current_input.chars().count();
                if self.cursor_pos < count {
                    let mut chars: Vec<char> = self.current_input.chars().collect();
                    chars.remove(self.cursor_pos);
                    self.current_input = chars.into_iter().collect();
                }
            }
            Key::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            Key::Right => {
                if self.cursor_pos < self.current_input.chars().count() {
                    self.cursor_pos += 1;
                }
            }
            Key::Home => {
                self.cursor_pos = 0;
            }
            Key::End => {
                self.cursor_pos = self.current_input.chars().count();
            }
            Key::Enter => {
                let cmd = self.current_input.trim().to_string();
                self.lines.push(format!("❯ {}", self.current_input));

                if !cmd.is_empty() {
                    if cmd == "clear" || cmd == "cls" {
                        self.clear();
                    } else {
                        self.send_line(&cmd);
                        self.history.push(cmd);
                    }
                } else {
                    self.send_line("");
                }

                self.current_input.clear();
                self.cursor_pos = 0;
                self.history_idx = None;
                self.scroll_to_bottom();
            }
            Key::Up => {
                if !self.history.is_empty() {
                    let new_idx = match self.history_idx {
                        Some(i) => if i > 0 { i - 1 } else { 0 },
                        None => self.history.len() - 1,
                    };
                    self.history_idx = Some(new_idx);
                    self.current_input = self.history[new_idx].clone();
                    self.cursor_pos = self.current_input.chars().count();
                }
            }
            Key::Down => {
                if let Some(i) = self.history_idx {
                    if i + 1 < self.history.len() {
                        let new_idx = i + 1;
                        self.history_idx = Some(new_idx);
                        self.current_input = self.history[new_idx].clone();
                        self.cursor_pos = self.current_input.chars().count();
                    } else {
                        self.history_idx = None;
                        self.current_input.clear();
                        self.cursor_pos = 0;
                    }
                }
            }
            Key::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(6);
            }
            Key::PageDown => {
                self.scroll_offset = (self.scroll_offset + 6).min(self.lines.len());
            }
            Key::Ctrl('c') => {
                self.lines.push(format!("❯ {}^C", self.current_input));
                self.current_input.clear();
                self.cursor_pos = 0;
                self.history_idx = None;
                self.scroll_to_bottom();
            }
            Key::Ctrl('l') => {
                self.clear();
            }
            Key::Esc => {
                self.is_focused = false;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_panel_lifecycle_and_input() {
        let mut term = TerminalPanel::new(PathBuf::from("."));
        assert!(!term.is_visible);
        assert!(!term.is_focused);

        term.toggle();
        assert!(term.is_visible);
        assert!(term.is_focused);

        // Type characters
        term.handle_key(Key::Char('l'));
        term.handle_key(Key::Char('s'));
        assert_eq!(term.current_input, "ls");
        assert_eq!(term.cursor_pos, 2);

        // Backspace
        term.handle_key(Key::Backspace);
        assert_eq!(term.current_input, "l");
        assert_eq!(term.cursor_pos, 1);

        // Cancel with Ctrl+C
        term.handle_key(Key::Ctrl('c'));
        assert_eq!(term.current_input, "");
        assert_eq!(term.cursor_pos, 0);

        // Esc unfocuses
        term.handle_key(Key::Esc);
        assert!(!term.is_focused);
        assert!(term.is_visible);
    }

    #[test]
    fn test_strip_ansi_escapes() {
        let raw = "\x1b[31mError:\x1b[0m archivo no encontrado";
        let clean = TerminalPanel::strip_ansi_escapes(raw);
        assert_eq!(clean, "Error: archivo no encontrado");
    }
}
