//! Keymapping and editor command definitions.
//! Zero third-party dependencies, purely standard library Rust.

use crate::ui::terminal::Key;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorAction {
    InsertChar(char),
    InsertNewline,
    InsertTab,
    Backspace,
    Delete,
    MoveLeft { select: bool },
    MoveRight { select: bool },
    MoveUp { select: bool },
    MoveDown { select: bool },
    MoveLineStart { select: bool },
    MoveLineEnd { select: bool },
    AddCursorAbove,
    AddCursorBelow,
    SelectNextOccurrence,
    Undo,
    Redo,
    Save,
    OpenSearchPrompt,
    OpenReplacePrompt,
    OpenCommandPalette,
    OpenSaveAsPrompt,
    OpenOpenFilePrompt,
    OpenAboutDialog,
    OpenExtensionsManager,
    ReloadExtensions,
    CreateSampleExtension,
    ExecuteExtensionCommand(String),
    CollapseCursors,
    OpenConfig,
    ReloadConfig,
    SetTheme(crate::editor::config::EditorTheme),
    // Clipboard
    Copy,
    Cut,
    Paste,
    // Tabs & Multiple Buffers
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    Quit,
    None,
}

pub struct Keymap;

impl Keymap {
    /// Maps a raw key event to an editor action in normal editing mode.
    pub fn resolve_key(key: Key) -> EditorAction {
        match key {
            // Document navigation
            Key::Left => EditorAction::MoveLeft { select: false },
            Key::Right => EditorAction::MoveRight { select: false },
            Key::Up => EditorAction::MoveUp { select: false },
            Key::Down => EditorAction::MoveDown { select: false },
            Key::Home => EditorAction::MoveLineStart { select: false },
            Key::End => EditorAction::MoveLineEnd { select: false },

            // Tabs navigation
            Key::PageDown => EditorAction::NextTab,
            Key::PageUp => EditorAction::PrevTab,
            Key::Ctrl('t') => EditorAction::NewTab,
            Key::Ctrl('w') => EditorAction::CloseTab,
            Key::Ctrl('o') => EditorAction::OpenOpenFilePrompt,

            // Clipboard shortcuts
            Key::Ctrl('c') => EditorAction::Copy,
            Key::Ctrl('x') => EditorAction::Cut,
            Key::Ctrl('v') => EditorAction::Paste,

            // Multi-cursor shortcuts
            Key::Ctrl('d') => EditorAction::SelectNextOccurrence, // Sublime / VS Code Ctrl+D
            Key::Ctrl('k') => EditorAction::AddCursorAbove,
            Key::Ctrl('j') => EditorAction::AddCursorBelow,

            // Text editing
            Key::Char(c) => EditorAction::InsertChar(c),
            Key::Enter => EditorAction::InsertNewline,
            Key::Tab => EditorAction::InsertTab,
            Key::Backspace => EditorAction::Backspace,
            Key::Delete => EditorAction::Delete,

            // History
            Key::Ctrl('z') => EditorAction::Undo,
            Key::Ctrl('y') => EditorAction::Redo,

            // File & Modal commands
            Key::Ctrl('s') => EditorAction::Save,
            Key::Ctrl('f') => EditorAction::OpenSearchPrompt,
            Key::Ctrl('h') | Key::Ctrl('r') => EditorAction::OpenReplacePrompt,
            Key::Ctrl('p') => EditorAction::OpenCommandPalette,
            Key::Ctrl('q') => EditorAction::Quit,

            // Help & About
            Key::F(1) => EditorAction::OpenAboutDialog,

            // Escape collapses multi-cursors to single cursor
            Key::Esc => EditorAction::CollapseCursors,

            _ => EditorAction::None,
        }
    }
}
