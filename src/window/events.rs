//! Window events representing user input and window state changes.
//! Zero third-party dependencies, purely standard library Rust.

use crate::ui::terminal::Key;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowEvent {
    Char(char),
    Key(Key),
    Resize { width: usize, height: usize },
    Close,
    None,
}
