//! Window events representing user input and window state changes.
//! Zero third-party dependencies, purely standard library Rust.

use crate::ui::terminal::Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowEvent {
    Char(char),
    Key(Key),
    MouseDown { x: usize, y: usize, button: MouseButton },
    MouseUp { x: usize, y: usize, button: MouseButton },
    MouseMove { x: usize, y: usize },
    ScrollUp { x: usize, y: usize },
    ScrollDown { x: usize, y: usize },
    Resize { width: usize, height: usize },
    Close,
    None,
}
