//! Rustext: Next-Generation Text Editor in pure standard-library Rust.
//! Zero third-party dependencies, ultra-low latency, and native multi-cursor support.

pub mod core;
pub mod plugin;
pub mod ui;
pub mod gfx;
pub mod window;
pub mod editor;

pub use core::{
    PieceTable, TextBuffer, Position, Cursor, CursorSet, History, EditOp, Transaction,
    SearchEngine, Language, SyntaxHighlighter, Clipboard,
};
pub use plugin::{
    ExtensionCommand, ExtensionInfo, ExtensionManager, Plugin, PluginContext, PluginRegistry,
    ProcessPluginRunner,
};
pub use ui::{Key, TerminalGuard, Screen, Theme, PromptModal, PromptKind};
pub use gfx::{Framebuffer, GfxRenderer, ThemeColors};
pub use window::{NativeWindow, WindowEvent, MouseButton};
pub use editor::{
    Editor, EditorMode, UnsavedConfirmAction, UnsavedConfirmDialog, EditorAction, Keymap, Document, EditorConfig, EditorTheme,
    Snippet, SnippetSession, SnippetRegistry,
    Diagnostic, DiagnosticSeverity, DiagnosticCollection, format_code,
    SessionManager, SessionState, TabSession, TerminalPanel,
};
