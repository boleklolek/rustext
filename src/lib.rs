//! Rustext: Next-Generation Text Editor in pure standard-library Rust.
//! Zero third-party dependencies, ultra-low latency, and native multi-cursor support.

pub mod core;
pub mod plugin;
pub mod ui;
pub mod editor;

pub use core::{
    PieceTable, TextBuffer, Position, Cursor, CursorSet, History, EditOp, Transaction,
    SearchEngine, Language, SyntaxHighlighter,
};
pub use plugin::{Plugin, PluginContext, PluginRegistry, ProcessPluginRunner};
pub use ui::{Key, TerminalGuard, Screen, Theme, PromptModal, PromptKind};
pub use editor::{Editor, EditorMode, EditorAction, Keymap};
