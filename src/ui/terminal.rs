//! Terminal raw mode control, window size querying, and key parsing.
//! Zero third-party dependencies, purely standard library Rust.

use std::io::{self, Read, Write};
use std::process::Command;

/// Represents a parsed keyboard key or event.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    Char(char),
    Ctrl(char),
    Alt(char),
    CtrlShift(char),
    Enter,
    Backspace,
    Tab,
    BackTab,
    Esc,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    AltUp,
    AltDown,
    AltLeft,
    AltRight,
    ShiftAltUp,
    ShiftAltDown,
    F(u8),
    Unknown,
}

/// RAII Guard that manages raw mode and terminal screen buffers.
pub struct TerminalGuard {
    _private: (),
}

impl TerminalGuard {
    /// Enters raw mode, switches to alternate screen buffer, and hides default cursor.
    pub fn enter() -> io::Result<Self> {
        // Use stty to switch to raw non-canonical mode without echo and disable XON/XOFF flow control (-ixon)
        // so that Ctrl+S (save) and Ctrl+Q (quit) pass through directly even over SSH connections.
        let status = Command::new("stty")
            .arg("-echo")
            .arg("-icanon")
            .arg("-isig")
            .arg("-ixon")
            .arg("-iexten")
            .arg("min")
            .arg("1")
            .arg("time")
            .arg("0")
            .status()?;

        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to configure terminal via stty",
            ));
        }

        let mut stdout = io::stdout();
        // Enter alternate screen buffer & clear screen
        stdout.write_all(b"\x1b[?1049h\x1b[2J\x1b[H")?;
        stdout.flush()?;

        Ok(Self { _private: () })
    }

    /// Queries terminal dimensions (rows, cols).
    pub fn size() -> (usize, usize) {
        #[cfg(unix)]
        {
            #[repr(C)]
            struct Winsize {
                ws_row: u16,
                ws_col: u16,
                ws_xpixel: u16,
                ws_ypixel: u16,
            }

            unsafe extern "C" {
                fn ioctl(fd: i32, request: u64, ...) -> i32;
            }

            let mut ws = Winsize { ws_row: 0, ws_col: 0, ws_xpixel: 0, ws_ypixel: 0 };
            // Try standard descriptors first (1 = stdout, 0 = stdin, 2 = stderr)
            for fd in [1, 0, 2] {
                if unsafe { ioctl(fd, 0x5413, &mut ws) } == 0 && ws.ws_row > 0 && ws.ws_col > 0 {
                    return (ws.ws_row as usize, ws.ws_col as usize);
                }
            }

            // If running in subshell or redirected, query controlling terminal /dev/tty directly
            if let Ok(tty) = std::fs::File::open("/dev/tty") {
                use std::os::fd::AsRawFd;
                if unsafe { ioctl(tty.as_raw_fd(), 0x5413, &mut ws) } == 0 && ws.ws_row > 0 && ws.ws_col > 0 {
                    return (ws.ws_row as usize, ws.ws_col as usize);
                }
            }
        }

        // Secondary fallback: query environmental variables LINES and COLUMNS
        if let (Ok(lines_s), Ok(cols_s)) = (std::env::var("LINES"), std::env::var("COLUMNS")) {
            if let (Ok(r), Ok(c)) = (lines_s.parse::<usize>(), cols_s.parse::<usize>()) {
                if r > 0 && c > 0 {
                    return (r, c);
                }
            }
        }

        // Tertiary fallback: stty size
        if let Ok(output) = Command::new("stty").arg("size").output() {
            if let Ok(s) = String::from_utf8(output.stdout) {
                let parts: Vec<&str> = s.split_whitespace().collect();
                if parts.len() == 2 {
                    if let (Ok(rows), Ok(cols)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                        if rows > 0 && cols > 0 {
                            return (rows, cols);
                        }
                    }
                }
            }
        }

        (24, 80) // Default safe fallback
    }

    /// Reads a single key event from standard input.
    pub fn read_key() -> io::Result<Key> {
        let mut stdin = io::stdin();
        let mut buf = [0u8; 1];

        if stdin.read(&mut buf)? == 0 {
            return Ok(Key::Unknown);
        }

        let b = buf[0];

        match b {
            // Escape sequence
            0x1b => {
                let mut seq = [0u8; 3];
                // Try reading follow-up escape sequence bytes
                let _ = stdin.read(&mut seq[..1]);
                if seq[0] == b'[' {
                    let _ = stdin.read(&mut seq[1..2]);
                    match seq[1] {
                        b'A' => Ok(Key::Up),
                        b'B' => Ok(Key::Down),
                        b'C' => Ok(Key::Right),
                        b'D' => Ok(Key::Left),
                        b'H' => Ok(Key::Home),
                        b'F' => Ok(Key::End),
                        b'Z' => Ok(Key::BackTab),
                        b'3' => {
                            let _ = stdin.read(&mut seq[2..3]); // Consume '~'
                            Ok(Key::Delete)
                        }
                        b'5' => {
                            let _ = stdin.read(&mut seq[2..3]); // Consume '~'
                            Ok(Key::PageUp)
                        }
                        b'6' => {
                            let _ = stdin.read(&mut seq[2..3]); // Consume '~'
                            Ok(Key::PageDown)
                        }
                        _ => Ok(Key::Esc),
                    }
                } else if seq[0] == b'O' {
                    let _ = stdin.read(&mut seq[1..2]);
                    match seq[1] {
                        b'H' => Ok(Key::Home),
                        b'F' => Ok(Key::End),
                        _ => Ok(Key::Esc),
                    }
                } else if seq[0].is_ascii_alphanumeric() || seq[0] == b'\\' {
                    Ok(Key::Alt(seq[0] as char))
                } else {
                    Ok(Key::Esc)
                }
            }
            // Control keys
            0x00 => Ok(Key::Ctrl('@')),
            0x09 => Ok(Key::Tab),
            0x0a | 0x0d => Ok(Key::Enter),
            0x7f | 0x08 => Ok(Key::Backspace),
            0x1f => Ok(Key::Ctrl('/')),
            1..=26 => {
                let ch = ((b - 1) + b'a') as char;
                Ok(Key::Ctrl(ch))
            }
            // Normal UTF-8 char
            _ => {
                // Read full UTF-8 character if multibyte
                let char_len = if b < 0x80 {
                    1
                } else if b < 0xE0 {
                    2
                } else if b < 0xF0 {
                    3
                } else {
                    4
                };

                let mut utf8_buf = vec![b];
                if char_len > 1 {
                    let mut rest = vec![0u8; char_len - 1];
                    stdin.read_exact(&mut rest)?;
                    utf8_buf.extend(rest);
                }

                if let Ok(s) = String::from_utf8(utf8_buf) {
                    if let Some(c) = s.chars().next() {
                        return Ok(Key::Char(c));
                    }
                }
                Ok(Key::Unknown)
            }
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Restore alternate screen, show cursor, reset formatting
        let mut stdout = io::stdout();
        let _ = stdout.write_all(b"\x1b[?25h\x1b[0m\x1b[?1049l");
        let _ = stdout.flush();
        // Restore terminal to sane state
        let _ = Command::new("stty").arg("sane").status();
    }
}
