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
    OpenGlobalSearchPrompt,
    OpenFileFinderPrompt,
    OpenCommandPalette,
    OpenSaveAsPrompt,
    OpenOpenFilePrompt,
    OpenRecentFilesPrompt,
    OpenFolderPrompt,
    OpenRecentProjectsPrompt,
    OpenWorkspace(std::path::PathBuf),
    CloseWorkspace,
    OpenAboutDialog,
    OpenExtensionsManager,
    ReloadExtensions,
    CreateSampleExtension,
    ExecuteExtensionCommand(String),
    CollapseCursors,
    OpenConfig,
    ReloadConfig,
    SetTheme(crate::editor::config::EditorTheme),
    // Font & Typography
    OpenFontSelector,
    IncreaseFontSize,
    DecreaseFontSize,
    ResetFontSize,
    SetFontFamily(crate::gfx::FontFamily),
    SetFontSize(usize),
    SetFontWeight(crate::gfx::FontWeight),
    // Clipboard
    Copy,
    Cut,
    Paste,
    PreviousTabstop,
    InsertSnippet(String),
    // Formatting & Diagnostics
    FormatDocument,
    CheckDocument,
    NextDiagnostic,
    PrevDiagnostic,
    // Tabs & Multiple Buffers
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    // File Explorer Sidebar
    ToggleSidebar,
    RefreshSidebar,
    FocusSidebar,
    // Autocomplete
    TriggerAutocomplete,
    // Minimap
    ToggleMinimap,
    // Split View
    ToggleSplitVertical,
    ToggleSplitHorizontal,
    SwitchSplitPane,
    CloseSplit,
    // Git Integration
    RefreshGit,
    ToggleGitGutter,
    ToggleGitHunkDiff,
    RevertGitHunk,
    // Integrated Terminal
    ToggleTerminal,
    FocusTerminal,
    ClearTerminal,
    // Word Wrap (Soft Wrap)
    ToggleWordWrap,
    // Indent Guides
    ToggleIndentGuides,
    // Ergonomic Visual Guides
    ToggleHighlightCurrentLine,
    ToggleRulers,
    ToggleStickyScroll,
    // Code Folding
    ToggleFold,
    FoldAll,
    UnfoldAll,
    // Ergonomics & Advanced Editing
    MoveLineUp,
    MoveLineDown,
    DuplicateLineDown,
    ToggleComment,
    JumpBack,
    JumpForward,
    // Language Server Protocol (LSP)
    LspGoToDefinition,
    LspHover,
    TriggerSignatureHelp,
    RestartLspServer,
    RefreshSemanticTokens,
    LspFindReferences,
    LspRename,
    LspCodeActions,
    LspFormatDocument,
    LspDocumentSymbols,
    LspToggleInlayHints,
    Quit,
    None,
}

pub struct Keymap;

impl Keymap {
    /// Maps a raw key event to an editor action in normal editing mode.
    pub fn resolve_key(key: Key) -> EditorAction {
        match key {
            // Line manipulation (Alt+Up/Down to move, Shift+Alt+Down or Ctrl+Shift+D to duplicate)
            Key::AltUp => EditorAction::MoveLineUp,
            Key::AltDown => EditorAction::MoveLineDown,
            Key::ShiftAltDown | Key::CtrlShift('d') | Key::CtrlShift('D') => EditorAction::DuplicateLineDown,

            // Toggle comment (Ctrl+/ or Ctrl+_)
            Key::Ctrl('/') | Key::Ctrl('_') | Key::Ctrl('7') => EditorAction::ToggleComment,

            // Jump List navigation (Alt+Left / Alt+Right)
            Key::AltLeft => EditorAction::JumpBack,
            Key::AltRight => EditorAction::JumpForward,

            // Signature Help trigger (Ctrl+Shift+Space)
            Key::CtrlShift(' ') => EditorAction::TriggerSignatureHelp,

            // Word wrap (Alt+Z)
            Key::Alt('z') | Key::Alt('Z') => EditorAction::ToggleWordWrap,

            // Indent guides (Alt+I)
            Key::Alt('i') | Key::Alt('I') => EditorAction::ToggleIndentGuides,

            // Code folding (Alt+[, Ctrl+Shift+[, F9 for toggle; Alt+0 fold all; Alt+J unfold all)
            Key::Alt('[') | Key::CtrlShift('[') | Key::CtrlShift('{') | Key::F(9) => EditorAction::ToggleFold,
            Key::Alt('0') => EditorAction::FoldAll,
            Key::Alt('j') | Key::Alt('J') => EditorAction::UnfoldAll,

            // Sidebar shortcut (Sublime / VS Code Ctrl+B)
            Key::Ctrl('b') => EditorAction::ToggleSidebar,

            // Minimap (Ctrl+M)
            Key::Ctrl('m') | Key::Ctrl('M') => EditorAction::ToggleMinimap,

            // Split View (Ctrl+\ for vertical, Ctrl+Shift+\ for horizontal, Alt+\ to switch pane)
            Key::Ctrl('\\') => EditorAction::ToggleSplitVertical,
            Key::CtrlShift('\\') | Key::CtrlShift('|') => EditorAction::ToggleSplitHorizontal,
            Key::Alt('\\') => EditorAction::SwitchSplitPane,

            // Git Integration (Ctrl+Shift+G, Alt+H for hunk diff, Alt+R for revert)
            Key::CtrlShift('g') | Key::CtrlShift('G') => EditorAction::RefreshGit,
            Key::Alt('h') | Key::Alt('H') => EditorAction::ToggleGitHunkDiff,
            Key::Alt('r') | Key::Alt('R') => EditorAction::RevertGitHunk,

            // LSP Semantic Tokens refresh (F5)
            Key::F(5) => EditorAction::RefreshSemanticTokens,

            // Integrated Terminal (Ctrl+` or Ctrl+~)
            Key::Ctrl('`') | Key::Ctrl('~') => EditorAction::ToggleTerminal,

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
            Key::CtrlShift('o') | Key::CtrlShift('O') => EditorAction::OpenFolderPrompt,
            Key::Alt('p') | Key::Alt('P') => EditorAction::OpenRecentProjectsPrompt,

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
            Key::BackTab => EditorAction::PreviousTabstop,
            Key::Backspace => EditorAction::Backspace,
            Key::Delete => EditorAction::Delete,

            // History
            Key::Ctrl('z') => EditorAction::Undo,
            Key::Ctrl('y') => EditorAction::Redo,

            // File & Modal commands
            Key::Ctrl('s') => EditorAction::Save,
            Key::Ctrl('f') => EditorAction::OpenSearchPrompt,
            Key::CtrlShift('f') | Key::CtrlShift('F') => EditorAction::OpenGlobalSearchPrompt,
            Key::Ctrl('h') => EditorAction::OpenReplacePrompt,
            Key::Ctrl('r') => EditorAction::OpenRecentFilesPrompt,
            Key::Ctrl('p') => EditorAction::OpenFileFinderPrompt,
            Key::CtrlShift('p') | Key::CtrlShift('P') => EditorAction::OpenCommandPalette,
            Key::Ctrl('q') => EditorAction::Quit,

            // Tools & Diagnostics
            Key::F(7) => EditorAction::CheckDocument,
            Key::F(8) => EditorAction::NextDiagnostic,

            // Language Server Protocol (LSP)
            Key::F(12) => EditorAction::LspGoToDefinition,
            Key::Alt('k') | Key::Alt('K') => EditorAction::LspHover,
            Key::F(2) => EditorAction::LspRename,
            Key::CtrlShift('u') | Key::CtrlShift('U') => EditorAction::LspFindReferences,
            Key::Ctrl('.') => EditorAction::LspCodeActions,
            Key::CtrlShift('i') | Key::CtrlShift('I') => EditorAction::LspFormatDocument,
            Key::CtrlShift('r') | Key::CtrlShift('R') => EditorAction::LspDocumentSymbols,

            // Autocomplete trigger (Ctrl+Space)
            Key::Ctrl(' ') | Key::Ctrl('@') => EditorAction::TriggerAutocomplete,

            // Font & Typography (Zoom / Selector)
            Key::Ctrl('+') | Key::Ctrl('=') => EditorAction::IncreaseFontSize,
            Key::Ctrl('-') => EditorAction::DecreaseFontSize,
            Key::Ctrl('0') => EditorAction::ResetFontSize,
            Key::CtrlShift('t') | Key::CtrlShift('T') => EditorAction::OpenFontSelector,

            // Help & About
            Key::F(1) => EditorAction::OpenAboutDialog,

            // Escape collapses multi-cursors to single cursor
            Key::Esc => EditorAction::CollapseCursors,

            _ => EditorAction::None,
        }
    }
}
