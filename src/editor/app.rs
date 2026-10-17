//! Main editor state machine, multi-cursor edit engine, tabs, and event loop.
//! Zero third-party dependencies, purely standard library Rust.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::document::Document;
use super::keymap::{EditorAction, Keymap};
use crate::core::buffer::{Position, TextBuffer};
use crate::core::clipboard::Clipboard;
use crate::core::cursor::Cursor;
use crate::core::history::{EditOp, Transaction};
use crate::core::search::SearchEngine;
use crate::core::syntax::Language;
use crate::plugin::{
    ExtensionManager, PluginContext, PluginRegistry, TrailingWhitespacePlugin, WordCountPlugin,
};
use crate::ui::prompt::{PromptKind, PromptModal};
use crate::ui::screen::Screen;
use crate::ui::terminal::{Key, TerminalGuard};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsavedConfirmAction {
    CloseTab(usize),
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsavedConfirmDialog {
    pub action: UnsavedConfirmAction,
    pub file_name: String,
    pub dirty_count: usize,
    pub selected_button: usize, // 0 = Guardar, 1 = Descartar, 2 = Cancelar
}

impl UnsavedConfirmDialog {
    pub fn for_close_tab(tab_idx: usize, file_name: String) -> Self {
        Self {
            action: UnsavedConfirmAction::CloseTab(tab_idx),
            file_name,
            dirty_count: 1,
            selected_button: 0,
        }
    }

    pub fn for_quit(dirty_count: usize, file_name: String) -> Self {
        Self {
            action: UnsavedConfirmAction::Quit,
            file_name,
            dirty_count,
            selected_button: 0,
        }
    }

    pub fn title(&self) -> &'static str {
        match self.action {
            UnsavedConfirmAction::CloseTab(_) => "Cerrar Pestaña con Cambios",
            UnsavedConfirmAction::Quit => "Salir de Rustext - Cambios sin Guardar",
        }
    }

    pub fn message_lines(&self) -> (String, String) {
        match self.action {
            UnsavedConfirmAction::CloseTab(_) => (
                format!("El archivo '{}' tiene cambios sin guardar.", self.file_name),
                "¿Desea guardar los cambios antes de cerrar la pestaña?".to_string(),
            ),
            UnsavedConfirmAction::Quit => {
                if self.dirty_count > 1 {
                    (
                        format!("Hay {} archivos con cambios sin guardar en la sesión.", self.dirty_count),
                        "¿Desea guardar todos los cambios antes de salir?".to_string(),
                    )
                } else {
                    (
                        format!("El archivo '{}' tiene cambios sin guardar.", self.file_name),
                        "¿Desea guardar los cambios antes de salir de Rustext?".to_string(),
                    )
                }
            }
        }
    }

    pub fn button_labels(&self) -> (&'static str, &'static str, &'static str) {
        match self.action {
            UnsavedConfirmAction::CloseTab(_) => ("Guardar", "No guardar", "Cancelar"),
            UnsavedConfirmAction::Quit => {
                if self.dirty_count > 1 {
                    ("Guardar Todo", "Descartar Todo", "Cancelar")
                } else {
                    ("Guardar", "No guardar", "Cancelar")
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    ModalPrompt(PromptModal),
    AboutDialog,
    ExtensionsManager { selected_idx: usize, scroll_offset: usize },
    UnsavedConfirm(UnsavedConfirmDialog),
    FontSelector(crate::editor::font_selector::FontSelectorDialog),
}

pub struct Editor {
    pub documents: Vec<Document>,
    pub active_doc_idx: usize,
    pub clipboard: Clipboard,
    pub screen: Screen,
    pub mode: EditorMode,
    pub status_message: Option<(String, Instant)>,
    pub plugin_registry: PluginRegistry,
    pub extension_manager: ExtensionManager,
    pub menu_bar: crate::editor::menu::MenuBar,
    pub config: crate::editor::config::EditorConfig,
    pub tab_size: usize,
    pub auto_indent: bool,
    pub auto_close_brackets: bool,
    pub auto_surround: bool,
    pub snippet_session: Option<crate::editor::snippet::SnippetSession>,
    pub diagnostics: crate::editor::diagnostic::DiagnosticCollection,
    pub sidebar: crate::editor::sidebar::FileExplorer,
    pub autocomplete: crate::editor::autocomplete::AutocompleteState,
    pub split: crate::editor::split::SplitState,
    pub git: crate::editor::git::GitManager,
    pub session_manager: crate::editor::session::SessionManager,
    pub recent_files: Vec<PathBuf>,
    pub terminal: crate::editor::terminal_panel::TerminalPanel,
    pub word_wrap: bool,
    pub last_code_cols: usize,
    pub should_quit: bool,
}

impl Editor {
    /// Creates a new editor instance with an initial buffer.
    pub fn new(buffer: TextBuffer) -> Self {
        let initial_doc = Document::new(buffer);

        let mut registry = PluginRegistry::new();
        registry.register(Box::new(TrailingWhitespacePlugin));
        registry.register(Box::new(WordCountPlugin));

        let mut ctx = PluginContext {
            file_path: initial_doc.buffer.file_path().map(|p| p.to_path_buf()),
            buffer_length: initial_doc.buffer.len_chars(),
            line_count: initial_doc.buffer.len_lines(),
            cursor_count: 1,
            status_message: None,
        };
        registry.trigger_init(&mut ctx);

        let extension_manager = ExtensionManager::new();
        let config = crate::editor::config::EditorConfig::load();
        let tab_size = config.editor.tab_size;
        let auto_indent = config.editor.auto_indent;
        let auto_close_brackets = config.editor.auto_close_brackets;
        let auto_surround = config.editor.auto_surround;
        let word_wrap = config.editor.word_wrap;

        let mut screen = Screen::new();
        screen.theme = crate::ui::Theme::by_name(&config.theme.name);

        let root_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut sidebar = crate::editor::sidebar::FileExplorer::new(root_dir.clone());
        sidebar.is_visible = config.gui.sidebar;
        sidebar.width = config.gui.sidebar_width;

        let git = crate::editor::git::GitManager::new(&root_dir);

        let session_manager = crate::editor::session::SessionManager::new();
        let mut recent_files = session_manager.load_session().map(|s| s.recent_files).unwrap_or_default();
        if let Some(fp) = initial_doc.buffer.file_path() {
            crate::editor::session::SessionManager::add_recent_file(&mut recent_files, fp.to_path_buf(), config.editor.max_recent_files);
        }

        let mut terminal = crate::editor::terminal_panel::TerminalPanel::new(root_dir);
        terminal.height = config.gui.terminal_height;
        if !config.gui.terminal_shell.is_empty() {
            terminal.shell_name = config.gui.terminal_shell.clone();
        }

        Self {
            documents: vec![initial_doc],
            active_doc_idx: 0,
            clipboard: Clipboard::new(),
            screen,
            mode: EditorMode::Normal,
            status_message: Some((
                "Rustext ready. Ctrl+T: New tab | Ctrl+W: Close | PageUp/Dn: Switch tab | Ctrl+C/V: Clipboard | Ctrl+S: Save".to_string(),
                Instant::now(),
            )),
            plugin_registry: registry,
            extension_manager,
            menu_bar: crate::editor::menu::MenuBar::new(),
            config,
            tab_size,
            auto_indent,
            auto_close_brackets,
            auto_surround,
            snippet_session: None,
            diagnostics: crate::editor::diagnostic::DiagnosticCollection::new(),
            sidebar,
            autocomplete: crate::editor::autocomplete::AutocompleteState::new(),
            split: crate::editor::split::SplitState::new(),
            git,
            session_manager,
            recent_files,
            terminal,
            word_wrap,
            last_code_cols: 80,
            should_quit: false,
        }
    }

    /// Reference to the active document.
    #[inline]
    pub fn doc(&self) -> &Document {
        &self.documents[self.active_doc_idx]
    }

    /// Mutable reference to the active document.
    #[inline]
    pub fn doc_mut(&mut self) -> &mut Document {
        &mut self.documents[self.active_doc_idx]
    }

    /// Sets a temporary status message visible in the status bar.
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    /// Primary application loop.
    pub fn run(&mut self) -> io::Result<()> {
        let _guard = TerminalGuard::enter()?;

        while !self.should_quit {
            self.screen.update_dimensions();
            self.render()?;

            let key = TerminalGuard::read_key()?;
            self.handle_key(key);
        }

        let _ = self.save_session();
        Ok(())
    }

    /// Dispatches input key events depending on editor mode.
    pub fn handle_key(&mut self, key: Key) {
        // If integrated terminal panel is visible and focused, route keys to it
        if self.terminal.is_visible && self.terminal.is_focused {
            if matches!(key, Key::Ctrl('`') | Key::Ctrl('~')) {
                self.execute_action(EditorAction::ToggleTerminal);
                return;
            }
            self.terminal.handle_key(key);
            return;
        }

        match &mut self.mode {
            EditorMode::UnsavedConfirm(dialog) => {
                match key {
                    Key::Esc | Key::Char('c') | Key::Char('C') => {
                        self.confirm_unsaved_action(2);
                    }
                    Key::Left | Key::BackTab | Key::Up => {
                        dialog.selected_button = if dialog.selected_button == 0 { 2 } else { dialog.selected_button - 1 };
                    }
                    Key::Right | Key::Tab | Key::Down => {
                        dialog.selected_button = (dialog.selected_button + 1) % 3;
                    }
                    Key::Char('g') | Key::Char('G') | Key::Char('s') | Key::Char('S') => {
                        self.confirm_unsaved_action(0);
                    }
                    Key::Char('d') | Key::Char('D') | Key::Char('n') | Key::Char('N') => {
                        self.confirm_unsaved_action(1);
                    }
                    Key::Enter | Key::Char(' ') => {
                        let choice = dialog.selected_button;
                        self.confirm_unsaved_action(choice);
                    }
                    _ => {}
                }
            }
            EditorMode::AboutDialog => match key {
                Key::Esc | Key::Enter | Key::Char(' ') => {
                    self.mode = EditorMode::Normal;
                }
                _ => {}
            },
            EditorMode::FontSelector(dialog) => {
                if let Some(applied) = dialog.handle_key(key) {
                    if applied {
                        let family = dialog.selected_family;
                        let size = dialog.selected_size;
                        let weight = dialog.selected_weight;
                        let line_spacing = dialog.line_spacing;
                        let ligatures = dialog.ligatures;
                        let antialiasing = dialog.antialiasing;
                        let hinting = dialog.hinting;
                        let letter_spacing = dialog.letter_spacing;
                        let font_aspect_ratio = dialog.font_aspect_ratio;
                        let stem_darkening = dialog.stem_darkening;
                        self.config.gui.font_family = family.as_str().to_string();
                        self.config.gui.font_size = size;
                        self.config.gui.font_weight = weight.as_str().to_string();
                        self.config.gui.line_spacing = line_spacing;
                        self.config.gui.font_ligatures = ligatures;
                        self.config.gui.font_antialiasing = antialiasing;
                        self.config.gui.letter_spacing = letter_spacing;
                        self.config.gui.font_aspect_ratio = font_aspect_ratio;
                        self.config.gui.stem_darkening = stem_darkening;
                        self.config.gui.scale_factor = dialog.scale_factor;
                        self.config.gui.ergonomic_scaling = dialog.ergonomic_scaling;
                        self.config.gui.sdf_font_rendering = dialog.sdf_rendering;
                        self.config.gui.optical_punctuation_spacing = dialog.optical_spacing;
                        self.config.gui.use_ttf_font = dialog.use_ttf;
                        self.config.gui.font_hinting = match hinting {
                            crate::gfx::HintingLevel::None => "none".to_string(),
                            crate::gfx::HintingLevel::Slight => "slight".to_string(),
                            crate::gfx::HintingLevel::Full => "full".to_string(),
                            crate::gfx::HintingLevel::Medium => "medium".to_string(),
                        };
                        let _ = self.config.save();
                        self.set_status(format!(
                            "Tipografía actualizada: {} {}pt ({})",
                            family.display_name(),
                            size,
                            weight.display_name()
                        ));
                    } else {
                        self.set_status("Selector de fuentes cancelado");
                    }
                    self.mode = EditorMode::Normal;
                }
            },
            EditorMode::ExtensionsManager { selected_idx, scroll_offset } => {
                let ext_count = self.extension_manager.extensions.len();
                match key {
                    Key::Esc => {
                        self.mode = EditorMode::Normal;
                    }
                    Key::Up => {
                        if *selected_idx > 0 {
                            *selected_idx -= 1;
                            if *selected_idx < *scroll_offset {
                                *scroll_offset = *selected_idx;
                            }
                        }
                    }
                    Key::Down => {
                        if ext_count > 0 && *selected_idx + 1 < ext_count {
                            *selected_idx += 1;
                            if *selected_idx >= *scroll_offset + 5 {
                                *scroll_offset = selected_idx.saturating_sub(4);
                            }
                        }
                    }
                    Key::Enter | Key::Char(' ') => {
                        if ext_count > 0 && *selected_idx < ext_count {
                            let name = self.extension_manager.extensions[*selected_idx].name.clone();
                            let state = self.extension_manager.toggle_extension(*selected_idx);
                            self.set_status(format!("Extensión '{}': {}", name, if state { "Activada" } else { "Desactivada" }));
                        }
                    }
                    Key::Char('r') | Key::Char('R') => {
                        self.extension_manager.reload();
                        self.set_status(format!("Extensiones recargadas ({} encontradas)", self.extension_manager.extensions.len()));
                    }
                    Key::Char('n') | Key::Char('N') => {
                        match self.extension_manager.create_sample_extension("mi_extension") {
                            Ok(p) => self.set_status(format!("Extensión creada en {}", p.display())),
                            Err(e) => self.set_status(format!("Error: {}", e)),
                        }
                    }
                    _ => {}
                }
            },
            EditorMode::ModalPrompt(modal) => match key {
                Key::Esc => {
                    self.mode = EditorMode::Normal;
                    self.set_status("Cancelled");
                }
                Key::Enter => {
                    if modal.kind == PromptKind::Replace {
                        let query = modal.input.clone();
                        self.find_next_match(&query);
                        return;
                    }
                    if modal.kind == PromptKind::CommandPalette {
                        let matches = modal.matching_commands();
                        if let Some(cmd) = matches.get(modal.selected_idx) {
                            let action = cmd.action.clone();
                            self.mode = EditorMode::Normal;
                            self.execute_action(action);
                            return;
                        }
                    }
                    if modal.is_file_dialog() {
                        let resolved = modal.resolve_full_path();
                        if resolved.is_dir() {
                            modal.navigate_to(resolved);
                            modal.input.clear();
                            modal.cursor_col = 0;
                            return;
                        }
                        if !modal.dir_entries.is_empty() && modal.selected_entry_idx < modal.dir_entries.len() {
                            let entry = &modal.dir_entries[modal.selected_entry_idx];
                            if entry.is_dir && (modal.input.trim().is_empty() || modal.input.trim() == entry.name.trim_end_matches('/')) {
                                let path = entry.path.clone();
                                modal.navigate_to(path);
                                modal.input.clear();
                                modal.cursor_col = 0;
                                return;
                            }
                        }
                    }
                    let modal_clone = modal.clone();
                    self.mode = EditorMode::Normal;
                    self.commit_prompt(&modal_clone);
                }
                Key::Tab => {
                    if modal.kind == PromptKind::Replace {
                        modal.toggle_active_field();
                        return;
                    }
                    if modal.is_file_dialog() {
                        if !modal.dir_entries.is_empty() && modal.selected_entry_idx < modal.dir_entries.len() {
                            let entry = &modal.dir_entries[modal.selected_entry_idx];
                            if entry.is_dir {
                                let path = entry.path.clone();
                                modal.navigate_to(path);
                                modal.input.clear();
                                modal.cursor_col = 0;
                            } else {
                                modal.input = entry.name.clone();
                                modal.cursor_col = modal.input.chars().count();
                            }
                        }
                    }
                }
                Key::Down => {
                    if modal.kind == PromptKind::Replace {
                        modal.active_field = 1;
                    } else if modal.kind == PromptKind::CommandPalette {
                        let count = modal.matching_commands().len();
                        if count > 0 {
                            modal.selected_idx = (modal.selected_idx + 1).min(count - 1);
                        }
                    } else if modal.kind == PromptKind::RecentFiles {
                        let count = modal.matching_recent_files().len();
                        if count > 0 {
                            modal.selected_idx = (modal.selected_idx + 1).min(count - 1);
                        }
                    } else if modal.is_file_dialog() {
                        modal.select_next_entry(10);
                    } else if modal.kind == PromptKind::GlobalSearch {
                        modal.select_next_global_result(12);
                    }
                }
                Key::Up => {
                    if modal.kind == PromptKind::Replace {
                        modal.active_field = 0;
                    } else if modal.kind == PromptKind::CommandPalette {
                        modal.selected_idx = modal.selected_idx.saturating_sub(1);
                    } else if modal.kind == PromptKind::RecentFiles {
                        modal.selected_idx = modal.selected_idx.saturating_sub(1);
                    } else if modal.is_file_dialog() {
                        modal.select_prev_entry();
                    } else if modal.kind == PromptKind::GlobalSearch {
                        modal.select_prev_global_result();
                    }
                }
                Key::PageDown => {
                    if modal.is_file_dialog() {
                        for _ in 0..8 {
                            modal.select_next_entry(10);
                        }
                    } else if modal.kind == PromptKind::RecentFiles {
                        let count = modal.matching_recent_files().len();
                        if count > 0 {
                            modal.selected_idx = (modal.selected_idx + 6).min(count - 1);
                        }
                    } else if modal.kind == PromptKind::GlobalSearch {
                        for _ in 0..8 {
                            modal.select_next_global_result(12);
                        }
                    }
                }
                Key::PageUp => {
                    if modal.is_file_dialog() {
                        for _ in 0..8 {
                            modal.select_prev_entry();
                        }
                    } else if modal.kind == PromptKind::RecentFiles {
                        modal.selected_idx = modal.selected_idx.saturating_sub(6);
                    } else if modal.kind == PromptKind::GlobalSearch {
                        for _ in 0..8 {
                            modal.select_prev_global_result();
                        }
                    }
                }
                Key::Backspace => {
                    modal.backspace();
                }
                Key::Left => {
                    modal.move_cursor_left();
                }
                Key::Right => {
                    modal.move_cursor_right();
                }
                Key::Alt(c) => {
                    if modal.kind == PromptKind::Replace {
                        match c.to_ascii_lowercase() {
                            'r' => {
                                let query = modal.input.clone();
                                let repl = modal.replace_input.clone();
                                self.replace_current_match(&query, &repl);
                            }
                            'a' => {
                                let query = modal.input.clone();
                                let repl = modal.replace_input.clone();
                                self.replace_all_matches(&query, &repl);
                            }
                            'p' => {
                                let query = modal.input.clone();
                                self.find_prev_match(&query);
                            }
                            'n' => {
                                let query = modal.input.clone();
                                self.find_next_match(&query);
                            }
                            _ => {}
                        }
                    } else if modal.kind == PromptKind::GlobalSearch {
                        if c.to_ascii_lowercase() == 'c' {
                            modal.toggle_case_sensitivity();
                        }
                    }
                }
                Key::Ctrl(c) => {
                    if modal.kind == PromptKind::Replace {
                        match c.to_ascii_lowercase() {
                            'p' => {
                                let query = modal.input.clone();
                                self.find_prev_match(&query);
                            }
                            'n' => {
                                let query = modal.input.clone();
                                self.find_next_match(&query);
                            }
                            _ => {}
                        }
                    }
                }
                Key::Char(c) => {
                    modal.insert_char(c);
                }
                _ => {}
            },
            EditorMode::Normal => {
                // If autocomplete popup is active, intercept navigation and commit keys
                if self.autocomplete.is_active {
                    match key {
                        Key::Down => {
                            self.autocomplete.select_next();
                            return;
                        }
                        Key::Up => {
                            self.autocomplete.select_prev();
                            return;
                        }
                        Key::Tab => {
                            if self.snippet_session.is_some() {
                                self.autocomplete.close();
                                self.handle_tab_key();
                                return;
                            } else if self.commit_autocomplete() {
                                return;
                            }
                        }
                        Key::Enter => {
                            if self.commit_autocomplete() {
                                return;
                            }
                        }
                        Key::Esc => {
                            self.autocomplete.close();
                            return;
                        }
                        Key::Left | Key::Right | Key::Home | Key::End | Key::PageUp | Key::PageDown => {
                            self.autocomplete.close();
                        }
                        Key::Backspace => {
                            let action = Keymap::resolve_key(key);
                            self.execute_action(action);
                            self.trigger_autocomplete(false);
                            return;
                        }
                        Key::Char(c) => {
                            let action = Keymap::resolve_key(key);
                            self.execute_action(action);
                            if crate::editor::autocomplete::AutocompleteState::is_identifier_char(c) || c == '.' || c == ':' {
                                self.trigger_autocomplete(false);
                            } else {
                                self.autocomplete.close();
                            }
                            return;
                        }
                        _ => {}
                    }
                } else if let Key::Char(c) = key {
                    let action = Keymap::resolve_key(key);
                    self.execute_action(action);
                    if crate::editor::autocomplete::AutocompleteState::is_identifier_char(c) || c == '.' || c == ':' {
                        self.trigger_autocomplete(false);
                    }
                    return;
                }

                let action = Keymap::resolve_key(key);
                self.execute_action(action);
            }
        }
    }

    /// Executes high-level editor actions.
    pub fn execute_action(&mut self, action: EditorAction) {
        match action {
            // Text editing
            EditorAction::InsertChar(c) => self.insert_char_multicursor(c),
            EditorAction::InsertNewline => self.insert_newline_multicursor(),
            EditorAction::InsertTab => self.handle_tab_key(),
            EditorAction::PreviousTabstop => self.handle_backtab_key(),
            EditorAction::InsertSnippet(ref trigger) => {
                let t = trigger.clone();
                self.expand_snippet_trigger(&t);
            }
            EditorAction::Backspace => self.backspace_multicursor(),
            EditorAction::Delete => self.delete_multicursor(),

            // Clipboard
            EditorAction::Copy => self.copy(),
            EditorAction::Cut => self.cut(),
            EditorAction::Paste => self.paste(),

            // Tabs / Multiple Buffers
            EditorAction::NewTab => self.new_tab(),
            EditorAction::CloseTab => self.close_tab(),
            EditorAction::NextTab => self.next_tab(),
            EditorAction::PrevTab => self.prev_tab(),
            EditorAction::OpenOpenFilePrompt => {
                let dir = self.doc().buffer.file_path()
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
                self.mode = EditorMode::ModalPrompt(PromptModal::new_with_dir(PromptKind::OpenFile, dir, None));
            }

            // Navigation
            EditorAction::MoveLeft { select } => {
                let doc = self.doc_mut();
                doc.cursors.move_left(&doc.buffer, select);
            }
            EditorAction::MoveRight { select } => {
                let doc = self.doc_mut();
                doc.cursors.move_right(&doc.buffer, select);
            }
            EditorAction::MoveUp { select } => {
                if self.word_wrap {
                    let max_cols = self.active_max_cols();
                    let doc = self.doc_mut();
                    crate::editor::wrap::WordWrap::move_up(&mut doc.cursors, &doc.buffer, max_cols, select);
                } else {
                    let doc = self.doc_mut();
                    doc.cursors.move_up(&doc.buffer, select);
                }
            }
            EditorAction::MoveDown { select } => {
                if self.word_wrap {
                    let max_cols = self.active_max_cols();
                    let doc = self.doc_mut();
                    crate::editor::wrap::WordWrap::move_down(&mut doc.cursors, &doc.buffer, max_cols, select);
                } else {
                    let doc = self.doc_mut();
                    doc.cursors.move_down(&doc.buffer, select);
                }
            }
            EditorAction::MoveLineStart { select } => {
                if self.word_wrap {
                    let max_cols = self.active_max_cols();
                    let doc = self.doc_mut();
                    crate::editor::wrap::WordWrap::move_to_line_start(&mut doc.cursors, &doc.buffer, max_cols, select);
                } else {
                    let doc = self.doc_mut();
                    doc.cursors.move_to_line_start(&doc.buffer, select);
                }
            }
            EditorAction::MoveLineEnd { select } => {
                if self.word_wrap {
                    let max_cols = self.active_max_cols();
                    let doc = self.doc_mut();
                    crate::editor::wrap::WordWrap::move_to_line_end(&mut doc.cursors, &doc.buffer, max_cols, select);
                } else {
                    let doc = self.doc_mut();
                    doc.cursors.move_to_line_end(&doc.buffer, select);
                }
            }

            // Multi-cursors
            EditorAction::AddCursorAbove => {
                let count = {
                    let doc = self.doc_mut();
                    doc.cursors.add_cursor_above(&doc.buffer);
                    doc.cursors.cursors().len()
                };
                self.set_status(format!("Active cursors: {}", count));
            }
            EditorAction::AddCursorBelow => {
                let count = {
                    let doc = self.doc_mut();
                    doc.cursors.add_cursor_below(&doc.buffer);
                    doc.cursors.cursors().len()
                };
                self.set_status(format!("Active cursors: {}", count));
            }
            EditorAction::SelectNextOccurrence => {
                let found = {
                    let doc = self.doc_mut();
                    SearchEngine::select_next_occurrence(&doc.buffer, &mut doc.cursors)
                };
                if found {
                    self.set_status(format!("Cursors: {}", self.doc().cursors.cursors().len()));
                } else {
                    self.set_status("No further occurrences found");
                }
            }

            // History
            EditorAction::Undo => {
                let remaining = {
                    let doc = self.doc_mut();
                    if let Some(restored) = doc.history.undo(&mut doc.buffer) {
                        doc.cursors.set_cursors(restored);
                        Some(doc.history.undo_count())
                    } else {
                        None
                    }
                };
                if let Some(count) = remaining {
                    self.set_status(format!("Undo (remaining: {})", count));
                } else {
                    self.set_status("Already at oldest change");
                }
            }
            EditorAction::Redo => {
                let remaining = {
                    let doc = self.doc_mut();
                    if let Some(restored) = doc.history.redo(&mut doc.buffer) {
                        doc.cursors.set_cursors(restored);
                        Some(doc.history.redo_count())
                    } else {
                        None
                    }
                };
                if let Some(count) = remaining {
                    self.set_status(format!("Redo (remaining: {})", count));
                } else {
                    self.set_status("Already at newest change");
                }
            }

            // File & Modal commands
            EditorAction::Save => self.save_document(),
            EditorAction::OpenSearchPrompt => {
                let mut modal = PromptModal::new(PromptKind::Find);
                if self.doc().cursors.primary().has_selection() {
                    let range = self.doc().cursors.primary().selection_range();
                    let sel_text = self.doc().buffer.get_text_range(range.start, range.end);
                    if !sel_text.contains('\n') && sel_text.chars().count() <= 60 {
                        modal.cursor_col = sel_text.chars().count();
                        modal.input = sel_text;
                    }
                }
                self.mode = EditorMode::ModalPrompt(modal);
            }
            EditorAction::OpenReplacePrompt => {
                let mut modal = PromptModal::new(PromptKind::Replace);
                if self.doc().cursors.primary().has_selection() {
                    let range = self.doc().cursors.primary().selection_range();
                    let sel_text = self.doc().buffer.get_text_range(range.start, range.end);
                    if !sel_text.contains('\n') && sel_text.chars().count() <= 60 {
                        modal.cursor_col = sel_text.chars().count();
                        modal.input = sel_text;
                    }
                }
                self.mode = EditorMode::ModalPrompt(modal);
            }
            EditorAction::OpenGlobalSearchPrompt => {
                let initial_query = if self.doc().cursors.primary().has_selection() {
                    let range = self.doc().cursors.primary().selection_range();
                    let sel_text = self.doc().buffer.get_text_range(range.start, range.end);
                    if !sel_text.contains('\n') && sel_text.chars().count() <= 60 {
                        Some(sel_text)
                    } else {
                        None
                    }
                } else {
                    None
                };
                let root = self.sidebar.root_dir.clone();
                self.mode = EditorMode::ModalPrompt(PromptModal::new_global_search(root, initial_query.as_deref()));
            }
            EditorAction::OpenSaveAsPrompt => {
                let (dir, filename) = if let Some(path) = self.doc().buffer.file_path() {
                    let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
                    let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
                    (dir, Some(filename))
                } else {
                    let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                    (dir, None)
                };
                self.mode = EditorMode::ModalPrompt(PromptModal::new_with_dir(PromptKind::SaveAs, dir, filename));
            }
            EditorAction::OpenAboutDialog => {
                self.mode = EditorMode::AboutDialog;
            }
            EditorAction::OpenCommandPalette => {
                let mut modal = PromptModal::new(PromptKind::CommandPalette);
                modal.custom_commands = self.extension_manager.get_active_commands();
                self.mode = EditorMode::ModalPrompt(modal);
            }
            EditorAction::OpenRecentFilesPrompt => {
                self.mode = EditorMode::ModalPrompt(PromptModal::new_recent_files(self.recent_files.clone()));
            }
            EditorAction::OpenExtensionsManager => {
                self.mode = EditorMode::ExtensionsManager { selected_idx: 0, scroll_offset: 0 };
            }
            EditorAction::ReloadExtensions => {
                self.extension_manager.reload();
                self.set_status(format!("Extensiones recargadas ({} encontradas)", self.extension_manager.extensions.len()));
            }
            EditorAction::CreateSampleExtension => {
                match self.extension_manager.create_sample_extension("mi_extension") {
                    Ok(p) => self.set_status(format!("Extensión creada en {}", p.display())),
                    Err(e) => self.set_status(format!("Error: {}", e)),
                }
            }
            EditorAction::ExecuteExtensionCommand(ref cmd_id) => {
                let id = cmd_id.clone();
                self.execute_extension_command(&id);
            }

            EditorAction::OpenConfig => {
                let config_path = crate::editor::config::EditorConfig::config_file_path();
                if !config_path.exists() {
                    if let Some(parent) = config_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&config_path, crate::editor::config::EditorConfig::default_ini_content());
                }
                self.open_file(&config_path);
                self.set_status("Configuración abierta (config.ini). Guarde con Ctrl+S para aplicar cambios.");
            }

            EditorAction::ReloadConfig => {
                self.reload_config();
            }

            EditorAction::SetTheme(theme) => {
                self.set_theme(theme.as_str());
            }

            EditorAction::OpenFontSelector => {
                let family = crate::gfx::FontFamily::from_name(&self.config.gui.font_family);
                let size = self.config.gui.font_size;
                let weight = crate::gfx::FontWeight::from_name(&self.config.gui.font_weight);
                let line_spacing = self.config.gui.line_spacing;
                let ligatures = self.config.gui.font_ligatures;
                let antialiasing = self.config.gui.font_antialiasing;
                let hinting = crate::gfx::HintingLevel::from_str(&self.config.gui.font_hinting);
                let letter_spacing = self.config.gui.letter_spacing;
                let font_aspect_ratio = self.config.gui.font_aspect_ratio;
                let stem_darkening = self.config.gui.stem_darkening;
                let scale_factor = self.config.gui.scale_factor;
                let ergonomic_scaling = self.config.gui.ergonomic_scaling;
                let sdf_rendering = self.config.gui.sdf_font_rendering;
                let optical_spacing = self.config.gui.optical_punctuation_spacing;
                let use_ttf = self.config.gui.use_ttf_font;
                self.mode = EditorMode::FontSelector(crate::editor::font_selector::FontSelectorDialog::new(
                    family,
                    size,
                    weight,
                    line_spacing,
                    ligatures,
                    antialiasing,
                    hinting,
                    letter_spacing,
                    font_aspect_ratio,
                    stem_darkening,
                    scale_factor,
                    ergonomic_scaling,
                    sdf_rendering,
                    optical_spacing,
                    use_ttf,
                ));
            }

            EditorAction::IncreaseFontSize => {
                let current = self.config.gui.font_size;
                let sizes = crate::editor::font_selector::AVAILABLE_SIZES;
                let next = sizes.iter().copied().find(|&s| s > current).unwrap_or(*sizes.last().unwrap_or(&24));
                self.config.gui.font_size = next;
                let _ = self.config.save();
                self.set_status(format!("Tamaño de fuente: {} pt", next));
            }

            EditorAction::DecreaseFontSize => {
                let current = self.config.gui.font_size;
                let sizes = crate::editor::font_selector::AVAILABLE_SIZES;
                let prev = sizes.iter().copied().rev().find(|&s| s < current).unwrap_or(*sizes.first().unwrap_or(&10));
                self.config.gui.font_size = prev;
                let _ = self.config.save();
                self.set_status(format!("Tamaño de fuente: {} pt", prev));
            }

            EditorAction::ResetFontSize => {
                self.config.gui.font_size = 12;
                let _ = self.config.save();
                self.set_status("Tamaño de fuente restablecido a 12 pt");
            }

            EditorAction::SetFontFamily(family) => {
                self.config.gui.font_family = family.as_str().to_string();
                let _ = self.config.save();
                self.set_status(format!("Familia tipográfica: {}", family.display_name()));
            }

            EditorAction::SetFontSize(size) => {
                self.config.gui.font_size = size;
                let _ = self.config.save();
                self.set_status(format!("Tamaño de fuente: {} pt", size));
            }

            EditorAction::SetFontWeight(weight) => {
                self.config.gui.font_weight = weight.as_str().to_string();
                let _ = self.config.save();
                self.set_status(format!("Grosor de fuente: {}", weight.display_name()));
            }

            EditorAction::CollapseCursors => {
                self.snippet_session = None;
                let primary = self.doc().cursors.primary();
                self.doc_mut().cursors.set_single(primary.head);
                self.set_status("Collapsed to single cursor");
            }

            EditorAction::Quit => {
                let any_dirty = self.documents.iter().any(|d| d.buffer.is_dirty());
                if any_dirty && !self.config.editor.hot_exit {
                    self.prompt_unsaved_quit();
                } else {
                    let _ = self.save_session();
                    self.should_quit = true;
                }
            }

            EditorAction::ToggleSidebar => {
                self.sidebar.is_visible = !self.sidebar.is_visible;
                if self.sidebar.is_visible {
                    self.sidebar.refresh();
                    self.set_status("Barra lateral mostrada");
                } else {
                    self.set_status("Barra lateral oculta");
                }
            }
            EditorAction::RefreshSidebar => {
                self.sidebar.refresh();
                self.set_status("Árbol de archivos actualizado");
            }
            EditorAction::FocusSidebar => {
                if self.sidebar.is_visible {
                    self.sidebar.is_focused = !self.sidebar.is_focused;
                }
            }

            EditorAction::FormatDocument => {
                self.format_active_document();
            }
            EditorAction::CheckDocument => {
                self.check_active_document();
            }
            EditorAction::NextDiagnostic => {
                self.jump_to_next_diagnostic();
            }
            EditorAction::PrevDiagnostic => {
                self.jump_to_prev_diagnostic();
            }

            EditorAction::TriggerAutocomplete => {
                self.trigger_autocomplete(true);
            }

            EditorAction::ToggleWordWrap => {
                self.toggle_word_wrap();
            }

            EditorAction::ToggleMinimap => {
                self.toggle_minimap();
            }
            EditorAction::ToggleSplitVertical => {
                self.toggle_split_vertical();
            }
            EditorAction::ToggleSplitHorizontal => {
                self.toggle_split_horizontal();
            }
            EditorAction::SwitchSplitPane => {
                self.switch_split_pane();
            }
            EditorAction::CloseSplit => {
                self.close_split();
            }

            EditorAction::RefreshGit => {
                self.refresh_git();
            }
            EditorAction::ToggleGitGutter => {
                self.toggle_git_gutter();
            }

            EditorAction::ToggleTerminal => {
                self.terminal.toggle();
                if self.terminal.is_visible {
                    self.set_status(format!("Terminal Integrado activado ({}) - Ctrl+` para alternar", self.terminal.shell_name));
                } else {
                    self.set_status("Terminal Integrado oculto");
                }
            }
            EditorAction::FocusTerminal => {
                if self.terminal.is_visible {
                    self.terminal.is_focused = true;
                }
            }
            EditorAction::ClearTerminal => {
                self.terminal.clear();
                self.set_status("Terminal limpiado");
            }

            EditorAction::None => {}
        }
    }

    /// Refreshes Git working tree statuses and current branch.
    pub fn refresh_git(&mut self) {
        self.git.refresh_statuses();
        let branch = self.git.branch_name().unwrap_or("sin git");
        self.set_status(format!("Git actualizado: rama '{}'", branch));
    }

    /// Toggles live Git Gutter diff indicator bars.
    pub fn toggle_git_gutter(&mut self) {
        self.config.gui.git_gutter = !self.config.gui.git_gutter;
        self.set_status(format!("Git Gutter {}", if self.config.gui.git_gutter { "activado" } else { "desactivado" }));
    }

    /// Returns the active column width available for text wrapping.
    #[inline]
    pub fn active_max_cols(&self) -> usize {
        self.last_code_cols.max(10)
    }

    /// Toggles visual word wrap on/off and persists the preference.
    pub fn toggle_word_wrap(&mut self) {
        self.word_wrap = !self.word_wrap;
        self.config.editor.word_wrap = self.word_wrap;
        let _ = self.config.save();
        self.set_status(format!(
            "Ajuste de línea (Word Wrap): {}",
            if self.word_wrap { "ACTIVADO [Alt+Z]" } else { "DESACTIVADO [Alt+Z]" }
        ));
    }

    /// Helper to resolve mouse click coordinates to a character offset with or without word wrap.
    pub fn find_click_position(
        doc: &Document,
        row: usize,
        x: usize,
        text_start_x: usize,
        char_w: usize,
        word_wrap: bool,
        code_cols: usize,
    ) -> Option<usize> {
        let total_lines = doc.buffer.len_lines();
        if !word_wrap {
            let target_line = doc.scroll_row + row;
            if target_line < total_lines {
                let col = if x >= text_start_x {
                    (x - text_start_x) / char_w + doc.scroll_col
                } else {
                    0
                };
                let pos = Position::new(target_line, col);
                Some(doc.buffer.position_to_char(pos))
            } else {
                None
            }
        } else {
            let mut current_vrow = 0;
            let mut cur_line = doc.scroll_row;
            let mut cur_sub = doc.scroll_subline;

            while cur_line < total_lines && current_vrow <= row {
                let line_cow = doc.buffer.line_cow(cur_line).unwrap_or(std::borrow::Cow::Borrowed(""));
                let sublines = crate::editor::wrap::WordWrap::compute_sublines(&line_cow, code_cols);
                for s_idx in cur_sub..sublines.len() {
                    if current_vrow == row {
                        let vcol = if x >= text_start_x {
                            (x - text_start_x) / char_w
                        } else {
                            0
                        };
                        let col = crate::editor::wrap::WordWrap::visual_to_col(s_idx, vcol, &sublines);
                        let pos = Position::new(cur_line, col);
                        return Some(doc.buffer.position_to_char(pos));
                    }
                    current_vrow += 1;
                }
                cur_line += 1;
                cur_sub = 0;
            }
            None
        }
    }

    /// Toggles visibility of the code minimap.
    pub fn toggle_minimap(&mut self) {
        self.config.gui.minimap = !self.config.gui.minimap;
        self.set_status(format!("Minimapa {}", if self.config.gui.minimap { "activado" } else { "desactivado" }));
    }

    /// Toggles vertical (side-by-side) split editor view.
    pub fn toggle_split_vertical(&mut self) {
        let total = self.documents.len();
        let current = self.active_doc_idx;
        self.split.toggle_vertical(total, current);
        if self.split.is_split() {
            self.active_doc_idx = self.split.secondary_doc_idx;
            self.set_status("División vertical activada (Ctrl+\\ para alternar, Alt+\\ para cambiar foco)");
        } else {
            self.active_doc_idx = self.split.primary_doc_idx;
            self.set_status("División de editor cerrada");
        }
    }

    /// Toggles horizontal (stacked) split editor view.
    pub fn toggle_split_horizontal(&mut self) {
        let total = self.documents.len();
        let current = self.active_doc_idx;
        self.split.toggle_horizontal(total, current);
        if self.split.is_split() {
            self.active_doc_idx = self.split.secondary_doc_idx;
            self.set_status("División horizontal activada (Ctrl+Shift+\\ para alternar, Alt+\\ para cambiar foco)");
        } else {
            self.active_doc_idx = self.split.primary_doc_idx;
            self.set_status("División de editor cerrada");
        }
    }

    /// Switches keyboard and editing focus between the two split panes.
    pub fn switch_split_pane(&mut self) {
        if self.split.is_split() {
            let new_pane = self.split.switch_pane();
            self.active_doc_idx = if new_pane == 1 {
                self.split.secondary_doc_idx
            } else {
                self.split.primary_doc_idx
            };
            self.set_status(format!("Foco en Panel {}", new_pane + 1));
        }
    }

    /// Closes the secondary split pane and restores single-pane editing.
    pub fn close_split(&mut self) {
        if self.split.is_split() {
            self.active_doc_idx = self.split.primary_doc_idx;
            self.split.close_split();
            self.set_status("División de editor cerrada");
        }
    }

    /// Synchronizes split pane document indices with the active tab.
    pub fn sync_split_doc_indices(&mut self) {
        if self.split.is_split() {
            if self.split.active_pane == 1 {
                self.split.secondary_doc_idx = self.active_doc_idx;
            } else {
                self.split.primary_doc_idx = self.active_doc_idx;
            }
            if self.split.primary_doc_idx >= self.documents.len() {
                self.split.primary_doc_idx = self.documents.len().saturating_sub(1);
            }
            if self.split.secondary_doc_idx >= self.documents.len() {
                self.split.secondary_doc_idx = self.documents.len().saturating_sub(1);
            }
        }
    }

    /// Reloads configuration from disk and applies updated parameters immediately.
    pub fn reload_config(&mut self) {
        let new_cfg = crate::editor::config::EditorConfig::load();
        self.tab_size = new_cfg.editor.tab_size;
        self.auto_indent = new_cfg.editor.auto_indent;
        self.auto_close_brackets = new_cfg.editor.auto_close_brackets;
        self.auto_surround = new_cfg.editor.auto_surround;
        self.sidebar.is_visible = new_cfg.gui.sidebar;
        self.sidebar.width = new_cfg.gui.sidebar_width;
        self.screen.theme = crate::ui::Theme::by_name(&new_cfg.theme.name);
        let theme_name = new_cfg.theme.name.clone();
        self.config = new_cfg;
        self.set_status(format!("Configuración recargada. Tema: '{}', tab_size: {}", theme_name, self.tab_size));
    }

    /// Sets the active theme and persists preference in config.ini.
    pub fn set_theme(&mut self, theme_name: &str) {
        self.config.theme.name = theme_name.to_string();
        let _ = self.config.save();
        self.screen.theme = crate::ui::Theme::by_name(theme_name);
        self.set_status(format!("Tema cambiado a '{}'", theme_name));
    }

    /// Formats the current document using the system formatter for its language.
    pub fn format_active_document(&mut self) {
        let (lang, text, file_path) = {
            let doc = self.doc();
            (doc.language, doc.buffer.to_string(), doc.buffer.file_path().map(|p| p.to_path_buf()))
        };

        match crate::editor::formatter::format_code(lang, &text, file_path.as_deref()) {
            Ok(formatted) => {
                if formatted != text {
                    let doc = self.doc_mut();
                    let mut tx = Transaction::new(doc.cursors.cursors().to_vec());
                    let total_chars = doc.buffer.len_chars();
                    doc.buffer.delete_range(0, total_chars);
                    tx.ops.push(EditOp::Delete { offset: 0, text: text.clone() });
                    doc.buffer.insert(0, &formatted);
                    tx.ops.push(EditOp::Insert { offset: 0, text: formatted.clone() });
                    let primary_head = doc.cursors.primary().head.min(formatted.chars().count());
                    doc.cursors.set_single(primary_head);
                    tx.cursors_after = doc.cursors.cursors().to_vec();
                    doc.history.record(tx);
                    self.set_status(format!("Documento formateado con éxito ({})", lang.name()));
                } else {
                    self.set_status("Documento ya está formateado correctamente");
                }
            }
            Err(e) => {
                self.set_status(format!("Formateo: {}", e));
            }
        }
    }

    /// Checks the current document for compiler/linter diagnostics.
    pub fn check_active_document(&mut self) {
        let (lang, text, file_path) = {
            let doc = self.doc();
            (doc.language, doc.buffer.to_string(), doc.buffer.file_path().map(|p| p.to_path_buf()))
        };

        match crate::editor::diagnostic::DiagnosticCollection::run_check(lang, file_path.as_deref(), &text) {
            Ok(diags) => {
                let count = diags.len();
                let errors = diags.iter().filter(|d| d.severity == crate::editor::diagnostic::DiagnosticSeverity::Error).count();
                let warnings = diags.iter().filter(|d| d.severity == crate::editor::diagnostic::DiagnosticSeverity::Warning).count();
                self.diagnostics.set(diags);
                if count == 0 {
                    self.set_status(format!("Verificación {}: 0 errores, 0 avisos (Código limpio)", lang.name()));
                } else {
                    self.set_status(format!(
                        "Diagnósticos {}: {} errores, {} avisos [F8: Siguiente error]",
                        lang.name(), errors, warnings
                    ));
                }
            }
            Err(e) => {
                self.set_status(format!("Verificación: {}", e));
            }
        }
    }

    /// Jumps the primary cursor to the next diagnostic line.
    pub fn jump_to_next_diagnostic(&mut self) {
        let current_line = {
            let doc = self.doc();
            let pos = doc.buffer.char_to_position(doc.cursors.primary().head);
            pos.line + 1
        };

        if let Some(diag) = self.diagnostics.next_diagnostic(current_line).cloned() {
            let target_line = diag.line.saturating_sub(1);
            let target_col = diag.col.saturating_sub(1);
            let doc = self.doc_mut();
            let total_lines = doc.buffer.len_lines();
            let line = target_line.min(total_lines.saturating_sub(1));
            let char_offset = doc.buffer.position_to_char(Position::new(line, target_col));
            doc.cursors.set_single(char_offset);
            self.set_status(diag.to_display_string());
        } else {
            self.set_status("No hay más diagnósticos ni errores registrados");
        }
    }

    /// Jumps the primary cursor to the previous diagnostic line.
    pub fn jump_to_prev_diagnostic(&mut self) {
        let current_line = {
            let doc = self.doc();
            let pos = doc.buffer.char_to_position(doc.cursors.primary().head);
            pos.line + 1
        };

        if let Some(diag) = self.diagnostics.prev_diagnostic(current_line).cloned() {
            let target_line = diag.line.saturating_sub(1);
            let target_col = diag.col.saturating_sub(1);
            let doc = self.doc_mut();
            let total_lines = doc.buffer.len_lines();
            let line = target_line.min(total_lines.saturating_sub(1));
            let char_offset = doc.buffer.position_to_char(Position::new(line, target_col));
            doc.cursors.set_single(char_offset);
            self.set_status(diag.to_display_string());
        } else {
            self.set_status("No hay diagnósticos ni errores registrados");
        }
    }

    /// Executes an extension command on the current selection or entire document buffer.
    pub fn execute_extension_command(&mut self, cmd_id: &str) {
        let has_sel = self.doc().cursors.primary().has_selection();
        let input_text = if has_sel {
            let range = self.doc().cursors.primary().selection_range();
            self.doc().buffer.get_text_range(range.start, range.end)
        } else {
            self.doc().buffer.to_string()
        };

        match self.extension_manager.execute_command(cmd_id, &input_text) {
            Some(Ok(output)) => {
                let doc = self.doc_mut();
                let mut tx = Transaction::new(doc.cursors.cursors().to_vec());
                if has_sel {
                    let range = doc.cursors.primary().selection_range();
                    let deleted = doc.buffer.get_text_range(range.start, range.end);
                    doc.buffer.delete_range(range.start, range.end);
                    tx.ops.push(EditOp::Delete { offset: range.start, text: deleted });
                    doc.buffer.insert(range.start, &output);
                    tx.ops.push(EditOp::Insert { offset: range.start, text: output.clone() });
                    let new_end = range.start + output.chars().count();
                    doc.cursors.set_single(new_end);
                } else {
                    let full_text = doc.buffer.to_string();
                    let total_chars = doc.buffer.len_chars();
                    doc.buffer.delete_range(0, total_chars);
                    tx.ops.push(EditOp::Delete { offset: 0, text: full_text });
                    doc.buffer.insert(0, &output);
                    tx.ops.push(EditOp::Insert { offset: 0, text: output.clone() });
                    let head = doc.cursors.primary().head.min(output.chars().count());
                    doc.cursors.set_single(head);
                }
                tx.cursors_after = doc.cursors.cursors().to_vec();
                doc.history.record(tx);
                self.set_status(format!("Comando '{}' ejecutado", cmd_id));
            }
            Some(Err(e)) => {
                self.set_status(format!("Error ejecutando '{}': {}", cmd_id, e));
            }
            None => {
                self.set_status(format!("Comando '{}' no encontrado o extensión inactiva", cmd_id));
            }
        }
    }

    // ---------------- Clipboard Operations ----------------

    /// Copies selected text (or current lines if no selection) to the clipboard.
    pub fn copy(&mut self) {
        let doc = self.doc();
        let mut parts = Vec::new();

        for c in doc.cursors.cursors() {
            if c.has_selection() {
                let range = c.selection_range();
                parts.push(doc.buffer.get_text_range(range.start, range.end));
            } else {
                let pos = doc.buffer.char_to_position(c.head);
                if let Some(line) = doc.buffer.line(pos.line) {
                    parts.push(line);
                }
            }
        }

        let full_text = parts.join("\n");
        if !full_text.is_empty() {
            self.clipboard.set_text(&full_text);
            self.set_status(format!("Copied {} chars", full_text.chars().count()));
        }
    }

    /// Cuts selected text (or current lines) to the clipboard.
    pub fn cut(&mut self) {
        self.copy();

        let doc = self.doc_mut();
        let mut tx = Transaction::new(doc.cursors.cursors().to_vec());
        doc.cursors.normalize();
        let cursors_snapshot = doc.cursors.cursors().to_vec();
        let mut new_cursors = Vec::with_capacity(cursors_snapshot.len());
        let mut offset_shift: isize = 0;

        for cursor in cursors_snapshot {
            let adj_anchor = (cursor.anchor as isize + offset_shift).max(0) as usize;
            let adj_head = (cursor.head as isize + offset_shift).max(0) as usize;

            if cursor.has_selection() {
                let start = std::cmp::min(adj_anchor, adj_head);
                let end = std::cmp::max(adj_anchor, adj_head);
                let deleted = doc.buffer.get_text_range(start, end);
                doc.buffer.delete_range(start, end);
                tx.ops.push(EditOp::Delete {
                    offset: start,
                    text: deleted,
                });

                let del_len = end - start;
                offset_shift -= del_len as isize;
                new_cursors.push(Cursor::new(start));
            } else {
                // Delete line
                let pos = doc.buffer.char_to_position(adj_head);
                let line_start = doc.buffer.position_to_char(Position::new(pos.line, 0));
                let line_end = if pos.line + 1 < doc.buffer.len_lines() {
                    doc.buffer.position_to_char(Position::new(pos.line + 1, 0))
                } else {
                    doc.buffer.len_chars()
                };

                let deleted = doc.buffer.get_text_range(line_start, line_end);
                doc.buffer.delete_range(line_start, line_end);
                tx.ops.push(EditOp::Delete {
                    offset: line_start,
                    text: deleted,
                });

                let del_len = line_end - line_start;
                offset_shift -= del_len as isize;
                new_cursors.push(Cursor::new(line_start));
            }
        }

        let doc = self.doc_mut();
        doc.cursors.set_cursors(new_cursors);
        tx.cursors_after = doc.cursors.cursors().to_vec();
        doc.history.record(tx);
        self.set_status("Cut to clipboard");
    }

    /// Pastes text from clipboard, with smart multi-cursor line mapping.
    pub fn paste(&mut self) {
        let text = self.clipboard.get_text();
        if text.is_empty() {
            return;
        }

        let doc = self.doc_mut();
        let mut tx = Transaction::new(doc.cursors.cursors().to_vec());
        doc.cursors.normalize();
        let cursors_snapshot = doc.cursors.cursors().to_vec();

        let lines: Vec<&str> = text.lines().collect();
        let is_multi_line_match = lines.len() == cursors_snapshot.len() && lines.len() > 1;

        let mut new_cursors = Vec::with_capacity(cursors_snapshot.len());
        let mut offset_shift: isize = 0;

        for (i, cursor) in cursors_snapshot.into_iter().enumerate() {
            let insert_str = if is_multi_line_match {
                lines[i]
            } else {
                &text
            };

            let adj_anchor = (cursor.anchor as isize + offset_shift).max(0) as usize;
            let adj_head = (cursor.head as isize + offset_shift).max(0) as usize;

            if cursor.has_selection() {
                let start = std::cmp::min(adj_anchor, adj_head);
                let end = std::cmp::max(adj_anchor, adj_head);
                let deleted = doc.buffer.get_text_range(start, end);
                doc.buffer.delete_range(start, end);
                tx.ops.push(EditOp::Delete {
                    offset: start,
                    text: deleted,
                });

                doc.buffer.insert(start, insert_str);
                tx.ops.push(EditOp::Insert {
                    offset: start,
                    text: insert_str.to_string(),
                });

                let del_len = end - start;
                let add_len = insert_str.chars().count();
                offset_shift += add_len as isize - del_len as isize;
                new_cursors.push(Cursor::new(start + add_len));
            } else {
                doc.buffer.insert(adj_head, insert_str);
                tx.ops.push(EditOp::Insert {
                    offset: adj_head,
                    text: insert_str.to_string(),
                });

                let add_len = insert_str.chars().count();
                offset_shift += add_len as isize;
                new_cursors.push(Cursor::new(adj_head + add_len));
            }
        }

        let doc = self.doc_mut();
        doc.cursors.set_cursors(new_cursors);
        tx.cursors_after = doc.cursors.cursors().to_vec();
        doc.history.record(tx);
        self.set_status("Pasted from clipboard");
    }

    // ---------------- Tabs / Multiple Buffers ----------------

    /// Opens a new empty tab.
    pub fn new_tab(&mut self) {
        let new_doc = Document::new(TextBuffer::new());
        self.documents.push(new_doc);
        self.active_doc_idx = self.documents.len() - 1;
        self.sync_split_doc_indices();
        self.set_status(format!("Tab {}/{}", self.active_doc_idx + 1, self.documents.len()));
    }

    /// Closes the current active tab, prompting if there are unsaved changes.
    pub fn close_tab(&mut self) {
        self.close_tab_at(self.active_doc_idx);
    }

    /// Closes the tab at a specific index, prompting if there are unsaved changes.
    pub fn close_tab_at(&mut self, idx: usize) {
        if idx >= self.documents.len() {
            return;
        }
        if self.documents[idx].buffer.is_dirty() {
            let file_name = self.documents[idx]
                .buffer
                .file_path()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("[Sin título]")
                .to_string();
            self.mode = EditorMode::UnsavedConfirm(UnsavedConfirmDialog::for_close_tab(idx, file_name));
        } else {
            self.force_close_tab_at(idx);
        }
    }

    /// Closes the current tab immediately without checking for unsaved changes.
    pub fn force_close_tab(&mut self) {
        self.force_close_tab_at(self.active_doc_idx);
    }

    /// Force closes the tab at a specific index without checking for unsaved changes.
    pub fn force_close_tab_at(&mut self, idx: usize) {
        if idx >= self.documents.len() {
            return;
        }
        if self.documents.len() > 1 {
            self.documents.remove(idx);
            if self.active_doc_idx >= self.documents.len() {
                self.active_doc_idx = self.documents.len() - 1;
            }
            self.sync_split_doc_indices();
            self.set_status(format!("Tab {}/{}", self.active_doc_idx + 1, self.documents.len()));
        } else {
            self.documents[0] = Document::new(TextBuffer::new());
            self.sync_split_doc_indices();
            self.set_status("Buffer cleared");
        }
    }

    /// Prompts confirmation for quitting when there are unsaved changes.
    pub fn prompt_unsaved_quit(&mut self) {
        let dirty_count = self.documents.iter().filter(|d| d.buffer.is_dirty()).count();
        if dirty_count == 0 {
            self.should_quit = true;
            return;
        }
        let file_name = self.doc()
            .buffer
            .file_path()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[Sin título]")
            .to_string();
        self.mode = EditorMode::UnsavedConfirm(UnsavedConfirmDialog::for_quit(dirty_count, file_name));
    }

    /// Prompts confirmation for closing a specific tab when it has unsaved changes.
    pub fn prompt_unsaved_close_tab(&mut self, idx: usize) {
        if idx >= self.documents.len() {
            return;
        }
        let file_name = self.documents[idx]
            .buffer
            .file_path()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[Sin título]")
            .to_string();
        self.mode = EditorMode::UnsavedConfirm(UnsavedConfirmDialog::for_close_tab(idx, file_name));
    }

    /// Executes the user's choice in the unsaved confirmation dialog.
    /// choice: 0 = Guardar / Guardar todo, 1 = Descartar / No guardar, 2 = Cancelar
    pub fn confirm_unsaved_action(&mut self, choice: usize) {
        let dialog = match &self.mode {
            EditorMode::UnsavedConfirm(d) => d.clone(),
            _ => return,
        };

        match choice {
            0 => {
                // Guardar
                match dialog.action {
                    UnsavedConfirmAction::CloseTab(tab_idx) => {
                        if tab_idx < self.documents.len() {
                            let has_path = self.documents[tab_idx].buffer.file_path().is_some();
                            if has_path {
                                self.save_document_at(tab_idx);
                                self.force_close_tab_at(tab_idx);
                                self.mode = EditorMode::Normal;
                                self.set_status("Archivo guardado y pestaña cerrada");
                            } else {
                                self.active_doc_idx = tab_idx;
                                self.sync_split_doc_indices();
                                let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                                self.mode = EditorMode::ModalPrompt(PromptModal::new_with_dir(PromptKind::SaveAs, dir, None));
                                self.set_status("Escriba un nombre para guardar el archivo antes de cerrar");
                            }
                        } else {
                            self.mode = EditorMode::Normal;
                        }
                    }
                    UnsavedConfirmAction::Quit => {
                        let untitled_dirty_idx = self.documents.iter().enumerate().position(|(_, d)| {
                            d.buffer.is_dirty() && d.buffer.file_path().is_none()
                        });

                        if let Some(idx) = untitled_dirty_idx {
                            self.active_doc_idx = idx;
                            self.sync_split_doc_indices();
                            let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                            self.mode = EditorMode::ModalPrompt(PromptModal::new_with_dir(PromptKind::SaveAs, dir, None));
                            self.set_status("Guarde los archivos sin título antes de salir");
                        } else {
                            for i in 0..self.documents.len() {
                                if self.documents[i].buffer.is_dirty() {
                                    self.save_document_at(i);
                                }
                            }
                            self.mode = EditorMode::Normal;
                            let _ = self.save_session();
                            self.should_quit = true;
                        }
                    }
                }
            }
            1 => {
                // Descartar
                match dialog.action {
                    UnsavedConfirmAction::CloseTab(tab_idx) => {
                        self.force_close_tab_at(tab_idx);
                        self.mode = EditorMode::Normal;
                        self.set_status(format!("Cerrado sin guardar: {}", dialog.file_name));
                    }
                    UnsavedConfirmAction::Quit => {
                        self.session_manager.clear_all_drafts();
                        self.mode = EditorMode::Normal;
                        let _ = self.save_session();
                        self.should_quit = true;
                    }
                }
            }
            _ => {
                // Cancelar
                self.mode = EditorMode::Normal;
                self.set_status("Operación cancelada");
            }
        }
    }

    /// Switches to the next tab.
    pub fn next_tab(&mut self) {
        if self.documents.len() > 1 {
            self.active_doc_idx = (self.active_doc_idx + 1) % self.documents.len();
            self.sync_split_doc_indices();
            self.set_status(format!("Tab {}/{}", self.active_doc_idx + 1, self.documents.len()));
        }
    }

    /// Switches to the previous tab.
    pub fn prev_tab(&mut self) {
        if self.documents.len() > 1 {
            self.active_doc_idx = if self.active_doc_idx == 0 {
                self.documents.len() - 1
            } else {
                self.active_doc_idx - 1
            };
            self.sync_split_doc_indices();
            self.set_status(format!("Tab {}/{}", self.active_doc_idx + 1, self.documents.len()));
        }
    }

    /// Adds a file path to the list of recent files.
    pub fn add_recent_file(&mut self, path: PathBuf) {
        crate::editor::session::SessionManager::add_recent_file(
            &mut self.recent_files,
            path,
            self.config.editor.max_recent_files,
        );
    }

    /// Persists current open tabs, cursor positions, scroll state, split layout and unsaved drafts.
    pub fn save_session(&mut self) -> io::Result<()> {
        if !self.config.editor.restore_session && !self.config.editor.auto_save_session {
            return Ok(());
        }

        let mut tabs = Vec::new();
        for (idx, doc) in self.documents.iter().enumerate() {
            let file_path = doc.buffer.file_path().map(|p| p.to_path_buf());
            let is_dirty = doc.buffer.is_dirty();
            let mut draft_path = None;

            if self.config.editor.hot_exit && is_dirty {
                let d_id = format!("tab_{}", idx);
                let content = doc.buffer.to_string();
                if let Ok(path) = self.session_manager.save_draft(&d_id, &content) {
                    draft_path = Some(path);
                }
            }

            let primary = doc.cursors.primary();
            let cursor_pos = doc.buffer.char_to_position(primary.head);
            tabs.push(crate::editor::session::TabSession {
                file_path,
                cursor_line: cursor_pos.line,
                cursor_col: cursor_pos.col,
                scroll_row: doc.scroll_row,
                scroll_col: doc.scroll_col,
                is_dirty,
                draft_path,
            });
        }

        let split_direction_str = match self.split.direction {
            crate::editor::split::SplitDirection::Vertical => "vertical".to_string(),
            crate::editor::split::SplitDirection::Horizontal => "horizontal".to_string(),
            crate::editor::split::SplitDirection::None => "none".to_string(),
        };

        let state = crate::editor::session::SessionState {
            open_tabs: tabs,
            active_tab_idx: self.active_doc_idx,
            split_active: self.split.is_split(),
            split_direction: split_direction_str,
            split_secondary_idx: self.split.secondary_doc_idx,
            split_active_pane: self.split.active_pane,
            split_ratio: self.split.ratio,
            sidebar_visible: self.sidebar.is_visible,
            sidebar_root: Some(self.sidebar.root_dir.clone()),
            recent_files: self.recent_files.clone(),
            window_width: self.screen.cols.max(80),
            window_height: self.screen.rows.max(24),
        };

        self.session_manager.save_session(&state)
    }

    /// Restores previously saved tabs, positions, split views and unsaved drafts.
    pub fn restore_session(&mut self) -> bool {
        if !self.config.editor.restore_session {
            return false;
        }

        let state = match self.session_manager.load_session() {
            Some(s) => s,
            None => return false,
        };

        if state.open_tabs.is_empty() {
            return false;
        }

        let mut restored_docs = Vec::new();
        for tab in state.open_tabs {
            let mut doc_opt = None;

            // 1. If hot exit draft exists, restore directly from draft
            if self.config.editor.hot_exit {
                if let Some(ref d_path) = tab.draft_path {
                    if d_path.exists() {
                        if let Ok(content) = self.session_manager.read_draft(d_path) {
                            let mut buf = TextBuffer::from_str(&content);
                            if let Some(ref f_path) = tab.file_path {
                                buf.set_file_path(f_path.clone());
                            }
                            buf.set_dirty(tab.is_dirty);
                            doc_opt = Some(Document::new(buf));
                        }
                    }
                }
            }

            // 2. Otherwise load existing file from disk
            if doc_opt.is_none() {
                if let Some(ref f_path) = tab.file_path {
                    if f_path.exists() && f_path.is_file() {
                        if let Ok(buf) = TextBuffer::open(f_path) {
                            doc_opt = Some(Document::new(buf));
                        }
                    }
                }
            }

            if let Some(mut doc) = doc_opt {
                let total_lines = doc.buffer.len_lines();
                let safe_line = tab.cursor_line.min(total_lines.saturating_sub(1));
                let line_len = doc.buffer.line_len(safe_line);
                let safe_col = tab.cursor_col.min(line_len);
                let offset = doc.buffer.position_to_char(Position::new(safe_line, safe_col));
                doc.cursors.set_single(offset);
                doc.scroll_row = tab.scroll_row;
                doc.scroll_col = tab.scroll_col;
                restored_docs.push(doc);
            }
        }

        if restored_docs.is_empty() {
            return false;
        }

        self.documents = restored_docs;
        self.active_doc_idx = state.active_tab_idx.min(self.documents.len().saturating_sub(1));
        self.split.direction = match state.split_direction.as_str() {
            "vertical" => crate::editor::split::SplitDirection::Vertical,
            "horizontal" => crate::editor::split::SplitDirection::Horizontal,
            _ => crate::editor::split::SplitDirection::None,
        };
        self.split.active_pane = state.split_active_pane;
        self.split.ratio = state.split_ratio;
        self.split.primary_doc_idx = state.active_tab_idx.min(self.documents.len().saturating_sub(1));
        self.split.secondary_doc_idx = state.split_secondary_idx.min(self.documents.len().saturating_sub(1));
        self.sidebar.is_visible = state.sidebar_visible;
        if let Some(root) = state.sidebar_root {
            if root.exists() && root.is_dir() {
                self.sidebar.set_root_dir(root);
            }
        }
        self.sync_split_doc_indices();
        self.session_manager.clear_all_drafts();
        true
    }

    /// Opens a file into a tab, or switches to it if already open.
    pub fn open_file(&mut self, path: &Path) {
        self.add_recent_file(path.to_path_buf());
        if let Some(pos) = self.documents.iter().position(|d| d.buffer.file_path() == Some(path)) {
            self.active_doc_idx = pos;
            self.sync_split_doc_indices();
            self.set_status(format!("Switched to tab {}", pos + 1));
            return;
        }

        match TextBuffer::open(path) {
            Ok(new_buf) => {
                let new_doc = Document::new(new_buf);
                self.documents.push(new_doc);
                self.active_doc_idx = self.documents.len() - 1;
                self.sync_split_doc_indices();
                self.set_status(format!("Opened {}", path.display()));
            }
            Err(e) => {
                self.set_status(format!("Could not open file: {}", e));
            }
        }
    }

    /// Handles submission of interactive modal prompts.
    pub fn commit_prompt(&mut self, modal: &PromptModal) {
        match modal.kind {
            PromptKind::Find | PromptKind::Replace => {
                let input = modal.input.trim();
                if !input.is_empty() {
                    self.find_next_match(input);
                }
            }
            PromptKind::SaveAs => {
                let target_path = modal.resolve_full_path();
                if target_path.is_dir() {
                    self.set_status(format!("Escriba un nombre de archivo para guardar en {}", target_path.display()));
                    return;
                }
                if let Some(parent) = target_path.parent() {
                    if !parent.as_os_str().is_empty() && !parent.exists() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                }
                let doc = self.doc_mut();
                if let Err(e) = doc.buffer.save_as(&target_path) {
                    self.set_status(format!("Error guardando archivo: {}", e));
                } else {
                    doc.language = Language::from_path(&target_path);
                    self.add_recent_file(target_path.clone());
                    self.set_status(format!("Guardado en {}", target_path.display()));
                }
            }
            PromptKind::OpenFile => {
                let target_path = modal.resolve_full_path();
                if target_path.is_dir() {
                    self.set_status(format!("'{}' es una carpeta. Seleccione un archivo.", target_path.display()));
                    return;
                }
                self.open_file(&target_path);
            }
            PromptKind::RecentFiles => {
                let matching = modal.matching_recent_files();
                if !matching.is_empty() && modal.selected_idx < matching.len() {
                    let target_path = matching[modal.selected_idx].clone();
                    self.open_file(&target_path);
                } else {
                    let direct_path = modal.resolve_full_path();
                    if direct_path.exists() && direct_path.is_file() {
                        self.open_file(&direct_path);
                    }
                }
            }
            PromptKind::CommandPalette => {
                let input = modal.input.trim();
                if input.is_empty() {
                    return;
                }
                let doc = self.doc();
                let mut ctx = PluginContext {
                    file_path: doc.buffer.file_path().map(|p| p.to_path_buf()),
                    buffer_length: doc.buffer.len_chars(),
                    line_count: doc.buffer.len_lines(),
                    cursor_count: doc.cursors.cursors().len(),
                    status_message: None,
                };

                let handled = self.plugin_registry.trigger_command(&mut ctx, input);
                if handled {
                    if let Some(msg) = ctx.status_message {
                        self.set_status(msg);
                    } else {
                        self.set_status(format!("Command '{}' executed", input));
                    }
                } else {
                    self.set_status(format!("Unknown command: '{}'", input));
                }
            }
            PromptKind::GlobalSearch => {
                if !modal.global_results.is_empty() && modal.selected_idx < modal.global_results.len() {
                    let result = &modal.global_results[modal.selected_idx];
                    let file_path = result.file_path.clone();
                    let target_line = result.line_number;
                    let target_col = result.col;
                    let rel_path = result.relative_path.clone();

                    self.open_file(&file_path);
                    let view_h = self.screen.rows.saturating_sub(2);
                    let doc = self.doc_mut();
                    let line_idx = target_line.saturating_sub(1);
                    let col_idx = target_col.saturating_sub(1);
                    let total_lines = doc.buffer.len_lines();
                    let safe_line = line_idx.min(total_lines.saturating_sub(1));
                    let char_offset = doc.buffer.position_to_char(Position::new(safe_line, col_idx));
                    doc.cursors.set_single(char_offset);
                    doc.scroll_row = safe_line.saturating_sub(view_h / 2);
                    self.set_status(format!("Salto a {}:{}:{}", rel_path, target_line, target_col));
                }
            }
        }
    }

    /// Finds the next occurrence of `query` in the current document.
    pub fn find_next_match(&mut self, query: &str) {
        let input = query.trim();
        if input.is_empty() {
            return;
        }
        let matched = {
            let doc = self.doc();
            let current_head = doc.cursors.primary().head;
            let search_from = if doc.cursors.primary().has_selection() {
                doc.cursors.primary().selection_range().end
            } else {
                current_head
            };
            SearchEngine::find_next(&doc.buffer, input, search_from, false)
        };
        if let Some((start, end)) = matched {
            let doc = self.doc_mut();
            doc.cursors.set_single(start);
            doc.cursors.cursors_mut()[0] = Cursor::with_selection(start, end);
            self.set_status(format!("Coincidencia en offset {}", start));
        } else {
            self.set_status(format!("No se encontró: '{}'", input));
        }
    }

    /// Finds the previous occurrence of `query` in the current document.
    pub fn find_prev_match(&mut self, query: &str) {
        let input = query.trim();
        if input.is_empty() {
            return;
        }
        let matched = {
            let doc = self.doc();
            let current_head = doc.cursors.primary().head;
            let search_from = if doc.cursors.primary().has_selection() {
                doc.cursors.primary().selection_range().start
            } else {
                current_head
            };
            SearchEngine::find_prev(&doc.buffer, input, search_from, false)
        };
        if let Some((start, end)) = matched {
            let doc = self.doc_mut();
            doc.cursors.set_single(start);
            doc.cursors.cursors_mut()[0] = Cursor::with_selection(start, end);
            self.set_status(format!("Coincidencia anterior en offset {}", start));
        } else {
            self.set_status(format!("No se encontró: '{}'", input));
        }
    }

    /// Replaces the currently selected match and advances to the next occurrence.
    pub fn replace_current_match(&mut self, query: &str, replacement: &str) {
        let input = query.trim();
        if input.is_empty() {
            return;
        }
        let doc = self.doc_mut();
        let primary = doc.cursors.primary();
        let mut replaced = false;
        if primary.has_selection() {
            let range = primary.selection_range();
            let selected_text = doc.buffer.get_text_range(range.start, range.end);
            if selected_text.eq_ignore_ascii_case(input) {
                let mut tx = Transaction::new(doc.cursors.cursors().to_vec());
                let deleted = doc.buffer.get_text_range(range.start, range.end);
                doc.buffer.delete_range(range.start, range.end);
                tx.ops.push(EditOp::Delete { offset: range.start, text: deleted });
                doc.buffer.insert(range.start, replacement);
                tx.ops.push(EditOp::Insert { offset: range.start, text: replacement.to_string() });
                let new_head = range.start + replacement.chars().count();
                doc.cursors.set_single(new_head);
                tx.cursors_after = doc.cursors.cursors().to_vec();
                doc.history.record(tx);
                replaced = true;
            }
        }
        if replaced {
            self.set_status(format!("Reemplazado '{}' por '{}'", input, replacement));
        }
        self.find_next_match(input);
    }

    /// Replaces all occurrences of `query` in the current document buffer atomically.
    pub fn replace_all_matches(&mut self, query: &str, replacement: &str) {
        let input = query.trim();
        if input.is_empty() {
            return;
        }
        let matches = SearchEngine::find_all(&self.doc().buffer, input, false);
        if matches.is_empty() {
            self.set_status(format!("No se encontraron coincidencias para '{}'", input));
            return;
        }
        let count = matches.len();
        let doc = self.doc_mut();
        let mut tx = Transaction::new(doc.cursors.cursors().to_vec());
        for (start, end) in matches.into_iter().rev() {
            let deleted = doc.buffer.get_text_range(start, end);
            doc.buffer.delete_range(start, end);
            tx.ops.push(EditOp::Delete { offset: start, text: deleted });
            doc.buffer.insert(start, replacement);
            tx.ops.push(EditOp::Insert { offset: start, text: replacement.to_string() });
        }
        doc.cursors.set_single(0);
        tx.cursors_after = doc.cursors.cursors().to_vec();
        doc.history.record(tx);
        self.set_status(format!("Reemplazadas {} ocurrencias de '{}'", count, input));
    }

    /// Saves a specific document buffer to disk by its index.
    pub fn save_document_at(&mut self, idx: usize) {
        if idx >= self.documents.len() {
            return;
        }
        if self.documents[idx].buffer.file_path().is_none() {
            let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            self.active_doc_idx = idx;
            self.sync_split_doc_indices();
            self.mode = EditorMode::ModalPrompt(PromptModal::new_with_dir(PromptKind::SaveAs, dir, None));
            return;
        }

        let doc = &self.documents[idx];
        let mut ctx = PluginContext {
            file_path: doc.buffer.file_path().map(|p| p.to_path_buf()),
            buffer_length: doc.buffer.len_chars(),
            line_count: doc.buffer.len_lines(),
            cursor_count: doc.cursors.cursors().len(),
            status_message: None,
        };

        let mut content = self.documents[idx].buffer.to_string();
        self.plugin_registry.trigger_before_save(&mut ctx, &mut content);

        if let Some(transformed) = self.extension_manager.trigger_on_save(&content) {
            content = transformed;
        }

        // Auto-format on save if enabled in configuration
        if self.config.editor.format_on_save {
            let lang = self.documents[idx].language;
            let file_path = self.documents[idx].buffer.file_path().map(|p| p.to_path_buf());
            if let Ok(formatted) = crate::editor::formatter::format_code(lang, &content, file_path.as_deref()) {
                content = formatted;
            }
        }

        let doc = &mut self.documents[idx];
        if content != doc.buffer.to_string() {
            let path = doc.buffer.file_path().map(|p| p.to_path_buf());
            doc.buffer = TextBuffer::from_str(&content);
            if let Some(p) = path {
                doc.buffer.set_file_path(p);
            }
        }

        match self.documents[idx].buffer.save() {
            Ok(()) => {
                self.plugin_registry.trigger_after_save(&mut ctx);
                let saved_path = self.documents[idx].buffer.file_path().map(|p| p.to_path_buf());
                let path_str = saved_path
                    .as_ref()
                    .and_then(|p| p.to_str())
                    .unwrap_or("file");
                self.set_status(format!("Saved {}", path_str));

                // Auto-run diagnostics check if enabled in config
                if self.config.editor.check_on_save {
                    self.check_active_document();
                }

                // Refresh Git status and gutter diffs on save
                self.git.refresh_statuses();

                // Auto-reload configuration if saving config.ini
                if let Some(p) = saved_path {
                    let config_path = crate::editor::config::EditorConfig::config_file_path();
                    if p == config_path || p.ends_with("config.ini") {
                        self.reload_config();
                    }
                }
            }
            Err(e) => {
                self.set_status(format!("Error saving file: {}", e));
            }
        }
    }

    /// Saves the current buffer to disk.
    pub fn save_document(&mut self) {
        self.save_document_at(self.active_doc_idx);
    }

    // ---------------- Parallel Vector Multi-Cursor Editing ----------------

    /// Returns the matching closing bracket/quote for an opening bracket/quote.
    #[inline]
    pub fn matching_closing_delimiter(ch: char) -> Option<char> {
        match ch {
            '(' => Some(')'),
            '[' => Some(']'),
            '{' => Some('}'),
            '"' => Some('"'),
            '\'' => Some('\''),
            '`' => Some('`'),
            _ => None,
        }
    }

    /// Checks if a character is a closing bracket or quote.
    #[inline]
    pub fn is_closing_delimiter(ch: char) -> bool {
        matches!(ch, ')' | ']' | '}' | '"' | '\'' | '`')
    }

    /// Inserts a character across all cursors simultaneously, with smart bracket pairing, overtyping, and surround selection.
    pub fn insert_char_multicursor(&mut self, ch: char) {
        let auto_surround = self.auto_surround;
        let auto_close_brackets = self.auto_close_brackets;
        let tab_size = self.tab_size;

        let doc = self.doc_mut();
        let mut tx = Transaction::new(doc.cursors.cursors().to_vec());
        let s = ch.to_string();

        doc.cursors.normalize();
        let cursors_snapshot = doc.cursors.cursors().to_vec();
        let mut new_cursors = Vec::with_capacity(cursors_snapshot.len());
        let mut offset_shift: isize = 0;

        for cursor in cursors_snapshot {
            let adj_anchor = (cursor.anchor as isize + offset_shift).max(0) as usize;
            let adj_head = (cursor.head as isize + offset_shift).max(0) as usize;

            if cursor.has_selection() {
                let start = std::cmp::min(adj_anchor, adj_head);
                let end = std::cmp::max(adj_anchor, adj_head);

                if auto_surround && Self::matching_closing_delimiter(ch).is_some() {
                    let closing = Self::matching_closing_delimiter(ch).unwrap();
                    let open_str = ch.to_string();
                    let close_str = closing.to_string();

                    doc.buffer.insert(start, &open_str);
                    tx.ops.push(EditOp::Insert {
                        offset: start,
                        text: open_str,
                    });

                    doc.buffer.insert(end + 1, &close_str);
                    tx.ops.push(EditOp::Insert {
                        offset: end + 1,
                        text: close_str,
                    });

                    offset_shift += 2;
                    new_cursors.push(Cursor::with_selection(start + 1, end + 1));
                } else {
                    let deleted = doc.buffer.get_text_range(start, end);
                    doc.buffer.delete_range(start, end);
                    tx.ops.push(EditOp::Delete {
                        offset: start,
                        text: deleted,
                    });

                    doc.buffer.insert(start, &s);
                    tx.ops.push(EditOp::Insert {
                        offset: start,
                        text: s.clone(),
                    });

                    let del_len = end - start;
                    let add_len = s.chars().count();
                    offset_shift += add_len as isize - del_len as isize;

                    new_cursors.push(Cursor::new(start + add_len));
                }
            } else {
                // No selection
                if auto_close_brackets {
                    // 1. Overtype closing bracket/quote if cursor is right before it
                    if Self::is_closing_delimiter(ch) && doc.buffer.char_at(adj_head) == Some(ch) {
                        new_cursors.push(Cursor::new(adj_head + 1));
                        continue;
                    }

                    // 2. Dedent when typing '}' on a line with only whitespace indentation
                    if ch == '}' {
                        let line_pos = doc.buffer.char_to_position(adj_head);
                        let line_start_char = doc.buffer.position_to_char(Position::new(line_pos.line, 0));
                        let before_cursor = doc.buffer.get_text_range(line_start_char, adj_head);
                        let only_spaces = !before_cursor.is_empty() && before_cursor.chars().all(|c| c == ' ');

                        if only_spaces && line_pos.col >= tab_size {
                            let del_start = adj_head - tab_size;
                            let deleted = doc.buffer.get_text_range(del_start, adj_head);
                            doc.buffer.delete_range(del_start, adj_head);
                            tx.ops.push(EditOp::Delete {
                                offset: del_start,
                                text: deleted,
                            });
                            offset_shift -= tab_size as isize;

                            let new_pos = del_start;
                            doc.buffer.insert(new_pos, "}");
                            tx.ops.push(EditOp::Insert {
                                offset: new_pos,
                                text: "}".to_string(),
                            });
                            offset_shift += 1;
                            new_cursors.push(Cursor::new(new_pos + 1));
                            continue;
                        }
                    }

                    // 3. Auto-close opening delimiters
                    if let Some(closing) = Self::matching_closing_delimiter(ch) {
                        let is_quote = ch == '"' || ch == '\'' || ch == '`';
                        let prev_alphanumeric = adj_head > 0
                            && doc
                                .buffer
                                .char_at(adj_head - 1)
                                .map(|c| c.is_alphanumeric())
                                .unwrap_or(false);

                        if !is_quote || !prev_alphanumeric {
                            let pair = format!("{}{}", ch, closing);
                            doc.buffer.insert(adj_head, &pair);
                            tx.ops.push(EditOp::Insert {
                                offset: adj_head,
                                text: pair,
                            });

                            offset_shift += 2;
                            new_cursors.push(Cursor::new(adj_head + 1));
                            continue;
                        }
                    }
                }

                doc.buffer.insert(adj_head, &s);
                tx.ops.push(EditOp::Insert {
                    offset: adj_head,
                    text: s.clone(),
                });

                let add_len = s.chars().count();
                offset_shift += add_len as isize;

                new_cursors.push(Cursor::new(adj_head + add_len));
            }
        }

        Self::notify_snippet_ops(&mut self.snippet_session, &tx.ops);

        let doc = self.doc_mut();
        doc.cursors.set_cursors(new_cursors);
        tx.cursors_after = doc.cursors.cursors().to_vec();
        doc.history.record(tx);
    }

    /// Inserts a newline with smart auto-indentation and block bracket expansion.
    pub fn insert_newline_multicursor(&mut self) {
        let auto_indent = self.auto_indent;
        let tab_size = self.tab_size;

        let doc = self.doc_mut();
        let mut tx = Transaction::new(doc.cursors.cursors().to_vec());

        doc.cursors.normalize();
        let cursors_snapshot = doc.cursors.cursors().to_vec();
        let mut new_cursors = Vec::with_capacity(cursors_snapshot.len());
        let mut offset_shift: isize = 0;

        for cursor in cursors_snapshot {
            let adj_anchor = (cursor.anchor as isize + offset_shift).max(0) as usize;
            let mut adj_head = (cursor.head as isize + offset_shift).max(0) as usize;

            if cursor.has_selection() {
                let start = std::cmp::min(adj_anchor, adj_head);
                let end = std::cmp::max(adj_anchor, adj_head);
                let deleted = doc.buffer.get_text_range(start, end);
                doc.buffer.delete_range(start, end);
                tx.ops.push(EditOp::Delete {
                    offset: start,
                    text: deleted,
                });

                let del_len = end - start;
                offset_shift -= del_len as isize;
                adj_head = start;
            }

            let line_pos = doc.buffer.char_to_position(adj_head);
            let current_line = doc.buffer.line(line_pos.line).unwrap_or_default();
            let base_indent: String = current_line
                .chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .collect();

            let line_start_char = doc.buffer.position_to_char(Position::new(line_pos.line, 0));
            let before_cursor = doc.buffer.get_text_range(line_start_char, adj_head);
            let trimmed_before = before_cursor.trim_end();
            let next_char = doc.buffer.char_at(adj_head);

            let tab_spaces = " ".repeat(tab_size);

            if auto_indent {
                // Case 1: Cursor is between matching brackets: { | }, [ | ], ( | )
                let is_between_braces = (trimmed_before.ends_with('{') && next_char == Some('}'))
                    || (trimmed_before.ends_with('[') && next_char == Some(']'))
                    || (trimmed_before.ends_with('(') && next_char == Some(')'));

                if is_between_braces {
                    let insert_str = format!("\n{}{}\n{}", base_indent, tab_spaces, base_indent);
                    doc.buffer.insert(adj_head, &insert_str);
                    tx.ops.push(EditOp::Insert {
                        offset: adj_head,
                        text: insert_str.clone(),
                    });

                    let cursor_dest = adj_head + 1 + base_indent.chars().count() + tab_size;
                    offset_shift += insert_str.chars().count() as isize;
                    new_cursors.push(Cursor::new(cursor_dest));
                    continue;
                }

                // Case 2: Line ends with block opening delimiter: '{', '(', '[', or ':'
                let opens_block = trimmed_before.ends_with('{')
                    || trimmed_before.ends_with('(')
                    || trimmed_before.ends_with('[')
                    || trimmed_before.ends_with(':');

                if opens_block {
                    let insert_str = format!("\n{}{}", base_indent, tab_spaces);
                    let add_len = insert_str.chars().count();
                    doc.buffer.insert(adj_head, &insert_str);
                    tx.ops.push(EditOp::Insert {
                        offset: adj_head,
                        text: insert_str,
                    });

                    offset_shift += add_len as isize;
                    new_cursors.push(Cursor::new(adj_head + add_len));
                    continue;
                }
            }

            // Case 3: Standard newline with matching base indentation
            let insert_str = format!("\n{}", base_indent);
            let add_len = insert_str.chars().count();
            doc.buffer.insert(adj_head, &insert_str);
            tx.ops.push(EditOp::Insert {
                offset: adj_head,
                text: insert_str,
            });

            offset_shift += add_len as isize;
            new_cursors.push(Cursor::new(adj_head + add_len));
        }

        Self::notify_snippet_ops(&mut self.snippet_session, &tx.ops);

        let doc = self.doc_mut();
        doc.cursors.set_cursors(new_cursors);
        tx.cursors_after = doc.cursors.cursors().to_vec();
        doc.history.record(tx);
    }

    /// Dispatches editing operations to active snippet session to keep tabstop offsets aligned.
    pub fn notify_snippet_ops(session: &mut Option<crate::editor::snippet::SnippetSession>, ops: &[EditOp]) {
        if let Some(session) = session {
            for op in ops {
                match op {
                    EditOp::Insert { offset, text } => {
                        session.on_buffer_edit(*offset, 0, text.chars().count());
                    }
                    EditOp::Delete { offset, text } => {
                        session.on_buffer_edit(*offset, text.chars().count(), 0);
                    }
                }
            }
        }
    }

    /// Dispatches editing transaction to active snippet session to keep tabstop offsets aligned.
    pub fn notify_snippet_session(&mut self, tx: &Transaction) {
        Self::notify_snippet_ops(&mut self.snippet_session, &tx.ops);
    }

    /// Handles Tab key: expands snippet if trigger matches, navigates active snippet tabstops,
    /// or inserts tab_size spaces.
    pub fn handle_tab_key(&mut self) {
        // 1. If an active snippet session exists, advance to next tabstop
        if let Some(mut session) = self.snippet_session.take() {
            if let Some((sel_start, sel_end)) = session.next_tabstop() {
                self.doc_mut().cursors.set_single_selection(sel_start, sel_end);
                self.snippet_session = Some(session);
                return;
            } else {
                // Snippet finished
                self.set_status("Snippet completado");
                return;
            }
        }

        // 2. If single cursor without selection, check for snippet trigger before cursor
        let doc = self.doc();
        if doc.cursors.cursors().len() == 1 && !doc.cursors.primary().has_selection() {
            let head = doc.cursors.primary().head;
            let pos = doc.buffer.char_to_position(head);
            if let Some(line) = doc.buffer.line(pos.line) {
                let chars_before: Vec<char> = line.chars().take(pos.col).collect();
                // Find contiguous identifier/trigger characters before cursor
                let mut trigger_start = chars_before.len();
                while trigger_start > 0
                    && (chars_before[trigger_start - 1].is_alphanumeric()
                        || chars_before[trigger_start - 1] == '_')
                {
                    trigger_start -= 1;
                }

                if trigger_start < chars_before.len() {
                    let trigger: String = chars_before[trigger_start..].iter().collect();
                    let base_indent: String = chars_before[..trigger_start]
                        .iter()
                        .take_while(|&&c| c == ' ' || c == '\t')
                        .collect();

                    if let Some(snippet) =
                        crate::editor::snippet::SnippetRegistry::find(doc.language, &trigger)
                    {
                        let trigger_len = trigger.chars().count();
                        let start_char = head - trigger_len;

                        // Delete trigger word
                        let doc = self.doc_mut();
                        let mut tx = Transaction::new(doc.cursors.cursors().to_vec());
                        let deleted = doc.buffer.get_text_range(start_char, head);
                        doc.buffer.delete_range(start_char, head);
                        tx.ops.push(EditOp::Delete {
                            offset: start_char,
                            text: deleted,
                        });

                        // Expand snippet with base indentation
                        let expanded = snippet.expand(&base_indent);
                        doc.buffer.insert(start_char, &expanded.text);
                        tx.ops.push(EditOp::Insert {
                            offset: start_char,
                            text: expanded.text.clone(),
                        });

                        // Start snippet session
                        let session = crate::editor::snippet::SnippetSession::new(
                            start_char,
                            expanded.tabstops,
                        );
                        if let Some((sel_start, sel_end)) = session.current_selection() {
                            doc.cursors.set_single_selection(sel_start, sel_end);
                        } else {
                            doc.cursors.set_single(start_char + expanded.text.chars().count());
                        }

                        tx.cursors_after = doc.cursors.cursors().to_vec();
                        doc.history.record(tx);

                        let desc = snippet.description;
                        self.snippet_session = Some(session);
                        self.set_status(format!(
                            "Snippet expandido: {} ({}) [Tab: Siguiente campo, Esc: Salir]",
                            trigger, desc
                        ));
                        return;
                    }
                }
            }
        }

        // 3. Fallback to normal tab insertion
        self.insert_tab_multicursor();
    }

    /// Handles Shift+Tab / BackTab key: jumps to the previous snippet placeholder.
    pub fn handle_backtab_key(&mut self) {
        if let Some(mut session) = self.snippet_session.take() {
            if let Some((sel_start, sel_end)) = session.prev_tabstop() {
                self.doc_mut().cursors.set_single_selection(sel_start, sel_end);
                self.snippet_session = Some(session);
            }
        }
    }

    /// Expands a specific snippet by trigger name at the current cursor location.
    pub fn expand_snippet_trigger(&mut self, trigger: &str) {
        let doc = self.doc();
        let head = doc.cursors.primary().head;
        let pos = doc.buffer.char_to_position(head);
        let base_indent = if let Some(line) = doc.buffer.line(pos.line) {
            line.chars().take_while(|&c| c == ' ' || c == '\t').collect::<String>()
        } else {
            String::new()
        };

        if let Some(snippet) = crate::editor::snippet::SnippetRegistry::find(doc.language, trigger) {
            let doc = self.doc_mut();
            let mut tx = Transaction::new(doc.cursors.cursors().to_vec());
            if doc.cursors.primary().has_selection() {
                let range = doc.cursors.primary().selection_range();
                let deleted = doc.buffer.get_text_range(range.start, range.end);
                doc.buffer.delete_range(range.start, range.end);
                tx.ops.push(EditOp::Delete {
                    offset: range.start,
                    text: deleted,
                });
            }

            let start_char = doc.cursors.primary().head;
            let expanded = snippet.expand(&base_indent);
            doc.buffer.insert(start_char, &expanded.text);
            tx.ops.push(EditOp::Insert {
                offset: start_char,
                text: expanded.text.clone(),
            });

            let session = crate::editor::snippet::SnippetSession::new(
                start_char,
                expanded.tabstops,
            );
            if let Some((sel_start, sel_end)) = session.current_selection() {
                doc.cursors.set_single_selection(sel_start, sel_end);
            } else {
                doc.cursors.set_single(start_char + expanded.text.chars().count());
            }

            tx.cursors_after = doc.cursors.cursors().to_vec();
            doc.history.record(tx);

            self.snippet_session = Some(session);
            self.set_status(format!("Snippet expandido: {} ({})", trigger, snippet.description));
        } else {
            self.set_status(format!("No se encontró snippet para '{}'", trigger));
        }
    }

    /// Triggers the autocomplete engine. If `force` is true, displays suggestions even with an empty prefix.
    pub fn trigger_autocomplete(&mut self, force: bool) {
        if !self.config.editor.autocomplete && !force {
            self.autocomplete.close();
            return;
        }

        let head = self.doc().cursors.primary().head;
        let (prefix, anchor) = crate::editor::autocomplete::AutocompleteState::extract_word_prefix(
            &self.doc().buffer,
            head,
        );

        if !force && prefix.len() < self.config.editor.autocomplete_min_chars {
            self.autocomplete.close();
            return;
        }

        let candidates = crate::editor::autocomplete::AutocompleteState::collect_candidates(
            self.doc(),
            &self.documents,
            head,
        );

        let ranked = crate::editor::autocomplete::AutocompleteState::filter_and_rank(&candidates, &prefix);
        if ranked.is_empty() {
            self.autocomplete.close();
        } else {
            self.autocomplete.open(prefix, anchor, ranked);
        }
    }

    /// Commits the currently selected autocomplete item and replaces the typed prefix.
    pub fn commit_autocomplete(&mut self) -> bool {
        if !self.autocomplete.is_active {
            return false;
        }

        let selected = match self.autocomplete.selected_item().cloned() {
            Some(item) => item,
            None => {
                self.autocomplete.close();
                return false;
            }
        };

        let prefix_len = self.autocomplete.prefix.chars().count();
        let is_snippet = selected.kind == crate::editor::autocomplete::CompletionKind::Snippet;
        let label = selected.label.clone();
        self.autocomplete.close();

        if is_snippet && self.doc().cursors.cursors().len() == 1 {
            // For snippet completion with single cursor: first delete the typed prefix at primary cursor, then expand snippet
            let head = self.doc().cursors.primary().head;
            let start = head.saturating_sub(prefix_len);
            if start < head {
                let doc = self.doc_mut();
                let mut tx = Transaction::new(doc.cursors.cursors().to_vec());
                let deleted = doc.buffer.get_text_range(start, head);
                doc.buffer.delete_range(start, head);
                tx.ops.push(EditOp::Delete {
                    offset: start,
                    text: deleted,
                });
                doc.cursors.set_single(start);
                tx.cursors_after = doc.cursors.cursors().to_vec();
                doc.history.record(tx);
            }
            self.expand_snippet_trigger(&label);
        } else {
            // Multicursor-aware completion: replace prefix before each cursor head with item label
            let doc = self.doc_mut();
            let mut tx = Transaction::new(doc.cursors.cursors().to_vec());
            doc.cursors.normalize();
            let cursors_snapshot = doc.cursors.cursors().to_vec();
            let mut new_cursors = Vec::with_capacity(cursors_snapshot.len());
            let mut offset_shift: isize = 0;
            let label_len = label.chars().count();

            for cursor in cursors_snapshot {
                let adj_head = (cursor.head as isize + offset_shift).max(0) as usize;
                let start = adj_head.saturating_sub(prefix_len);

                if start < adj_head {
                    let deleted = doc.buffer.get_text_range(start, adj_head);
                    doc.buffer.delete_range(start, adj_head);
                    tx.ops.push(EditOp::Delete {
                        offset: start,
                        text: deleted,
                    });
                    offset_shift -= prefix_len as isize;
                }

                doc.buffer.insert(start, &label);
                tx.ops.push(EditOp::Insert {
                    offset: start,
                    text: label.clone(),
                });
                offset_shift += label_len as isize;
                new_cursors.push(Cursor::new(start + label_len));
            }

            Self::notify_snippet_ops(&mut self.snippet_session, &tx.ops);

            let doc = self.doc_mut();
            doc.cursors.set_cursors(new_cursors);
            tx.cursors_after = doc.cursors.cursors().to_vec();
            doc.history.record(tx);
        }

        true
    }

    /// Inserts tab characters or spaces across all cursors.
    pub fn insert_tab_multicursor(&mut self) {
        for _ in 0..self.tab_size {
            self.insert_char_multicursor(' ');
        }
    }

    /// Deletes character before cursor across all cursors (Backspace), with smart pair deletion and tab dedent.
    pub fn backspace_multicursor(&mut self) {
        let auto_close_brackets = self.auto_close_brackets;
        let tab_size = self.tab_size;

        let doc = self.doc_mut();
        let mut tx = Transaction::new(doc.cursors.cursors().to_vec());

        doc.cursors.normalize();
        let cursors_snapshot = doc.cursors.cursors().to_vec();
        let mut new_cursors = Vec::with_capacity(cursors_snapshot.len());
        let mut offset_shift: isize = 0;

        for cursor in cursors_snapshot {
            let adj_anchor = (cursor.anchor as isize + offset_shift).max(0) as usize;
            let adj_head = (cursor.head as isize + offset_shift).max(0) as usize;

            if cursor.has_selection() {
                let start = std::cmp::min(adj_anchor, adj_head);
                let end = std::cmp::max(adj_anchor, adj_head);
                let deleted = doc.buffer.get_text_range(start, end);
                doc.buffer.delete_range(start, end);
                tx.ops.push(EditOp::Delete {
                    offset: start,
                    text: deleted,
                });

                let del_len = end - start;
                offset_shift -= del_len as isize;
                new_cursors.push(Cursor::new(start));
            } else if adj_head > 0 {
                // 1. Smart Pair Deletion: delete both when cursor is between empty brackets/quotes
                if auto_close_brackets && adj_head < doc.buffer.len_chars() {
                    let prev_c = doc.buffer.char_at(adj_head - 1);
                    let next_c = doc.buffer.char_at(adj_head);
                    let is_empty_pair = matches!(
                        (prev_c, next_c),
                        (Some('('), Some(')'))
                            | (Some('['), Some(']'))
                            | (Some('{'), Some('}'))
                            | (Some('"'), Some('"'))
                            | (Some('\''), Some('\''))
                            | (Some('`'), Some('`'))
                    );

                    if is_empty_pair {
                        let target_offset = adj_head - 1;
                        let deleted = doc.buffer.get_text_range(target_offset, target_offset + 2);
                        doc.buffer.delete_range(target_offset, target_offset + 2);
                        tx.ops.push(EditOp::Delete {
                            offset: target_offset,
                            text: deleted,
                        });

                        offset_shift -= 2;
                        new_cursors.push(Cursor::new(target_offset));
                        continue;
                    }
                }

                // 2. Smart Tab Dedent: delete tab_size spaces if line before cursor consists purely of indentation spaces
                let line_pos = doc.buffer.char_to_position(adj_head);
                let line_start_char = doc.buffer.position_to_char(Position::new(line_pos.line, 0));
                let before_cursor = doc.buffer.get_text_range(line_start_char, adj_head);
                let only_spaces = !before_cursor.is_empty() && before_cursor.chars().all(|c| c == ' ');

                if only_spaces && line_pos.col >= tab_size && line_pos.col % tab_size == 0 {
                    let target_offset = adj_head - tab_size;
                    let deleted = doc.buffer.get_text_range(target_offset, adj_head);
                    doc.buffer.delete_range(target_offset, adj_head);
                    tx.ops.push(EditOp::Delete {
                        offset: target_offset,
                        text: deleted,
                    });

                    offset_shift -= tab_size as isize;
                    new_cursors.push(Cursor::new(target_offset));
                    continue;
                }

                // 3. Standard single character deletion
                let target_offset = adj_head - 1;
                let deleted = doc.buffer.get_text_range(target_offset, adj_head);
                doc.buffer.delete_range(target_offset, adj_head);
                tx.ops.push(EditOp::Delete {
                    offset: target_offset,
                    text: deleted,
                });

                offset_shift -= 1;
                new_cursors.push(Cursor::new(target_offset));
            } else {
                new_cursors.push(Cursor::new(0));
            }
        }

        Self::notify_snippet_ops(&mut self.snippet_session, &tx.ops);

        let doc = self.doc_mut();
        doc.cursors.set_cursors(new_cursors);
        tx.cursors_after = doc.cursors.cursors().to_vec();
        doc.history.record(tx);
    }

    /// Deletes character after cursor across all cursors (Delete).
    pub fn delete_multicursor(&mut self) {
        let doc = self.doc_mut();
        let mut tx = Transaction::new(doc.cursors.cursors().to_vec());

        doc.cursors.normalize();
        let cursors_snapshot = doc.cursors.cursors().to_vec();
        let mut new_cursors = Vec::with_capacity(cursors_snapshot.len());
        let mut offset_shift: isize = 0;

        for cursor in cursors_snapshot {
            let adj_anchor = (cursor.anchor as isize + offset_shift).max(0) as usize;
            let adj_head = (cursor.head as isize + offset_shift).max(0) as usize;
            let total_chars = doc.buffer.len_chars();

            if cursor.has_selection() {
                let start = std::cmp::min(adj_anchor, adj_head);
                let end = std::cmp::max(adj_anchor, adj_head);
                let deleted = doc.buffer.get_text_range(start, end);
                doc.buffer.delete_range(start, end);
                tx.ops.push(EditOp::Delete {
                    offset: start,
                    text: deleted,
                });

                let del_len = end - start;
                offset_shift -= del_len as isize;
                new_cursors.push(Cursor::new(start));
            } else if adj_head < total_chars {
                let target_offset = adj_head;
                let deleted = doc.buffer.get_text_range(target_offset, target_offset + 1);
                doc.buffer.delete_range(target_offset, target_offset + 1);
                tx.ops.push(EditOp::Delete {
                    offset: target_offset,
                    text: deleted,
                });

                offset_shift -= 1;
                new_cursors.push(Cursor::new(target_offset));
            } else {
                new_cursors.push(Cursor::new(adj_head));
            }
        }

        Self::notify_snippet_ops(&mut self.snippet_session, &tx.ops);

        let doc = self.doc_mut();
        doc.cursors.set_cursors(new_cursors);
        tx.cursors_after = doc.cursors.cursors().to_vec();
        doc.history.record(tx);
    }

    /// Renders the complete editor view.
    pub fn render(&mut self) -> io::Result<()> {
        let active_idx = self.active_doc_idx;
        let primary_head = self.documents[active_idx].cursors.primary().head;
        let primary_pos = self.documents[active_idx].buffer.char_to_position(primary_head);
        let total_lines = self.documents[active_idx].buffer.len_lines();
        let total_chars = self.documents[active_idx].buffer.len_chars();
        let is_dirty = self.documents[active_idx].buffer.is_dirty();
        let cursor_count = self.documents[active_idx].cursors.cursors().len();
        let lang_name = self.documents[active_idx].language.name();
        let gutter_width = Screen::calculate_gutter_width(total_lines);

        let file_name = self.documents[active_idx]
            .buffer
            .file_path()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[Untitled]")
            .to_string();

        let tab_titles: Vec<String> = self.documents.iter().map(|d| d.tab_title()).collect();

        // Adjust viewport to keep primary cursor in view
        self.screen.adjust_viewport(primary_pos.line, primary_pos.col, gutter_width);
        self.screen.begin_frame();

        // 1. Tab Bar
        self.screen.render_tab_bar(&tab_titles, active_idx);

        // 2. Document canvas
        let doc = &self.documents[active_idx];
        self.screen.render_document(&doc.buffer, &doc.cursors, doc.language);

        // 3. Status bar
        let mode_str = match &self.mode {
            EditorMode::Normal => "EDIT",
            EditorMode::ModalPrompt(_) => "PROMPT",
            EditorMode::AboutDialog => "ABOUT",
            EditorMode::ExtensionsManager { .. } => "EXTENSIONS",
            EditorMode::UnsavedConfirm(_) => "CONFIRM",
            EditorMode::FontSelector(_) => "FONT",
        };

        self.screen.render_status_bar(
            mode_str,
            &file_name,
            is_dirty,
            (primary_pos.line, primary_pos.col),
            cursor_count,
            total_lines,
            total_chars,
            lang_name,
        );

        // 4. Prompt or message line
        match &self.mode {
            EditorMode::ModalPrompt(modal) => {
                let prompt_text = modal.formatted_display();
                self.screen.render_prompt_line(&prompt_text);
            }
            EditorMode::AboutDialog => {
                self.screen.render_prompt_line("Rustext v0.1.0 │ 100% Rust stdlib │ Autor: boleklolek <bolek.lolek@gmail.com> │ [Esc para cerrar]");
            }
            EditorMode::ExtensionsManager { selected_idx, .. } => {
                let total = self.extension_manager.extensions.len();
                let ext_name = if total > 0 && *selected_idx < total {
                    &self.extension_manager.extensions[*selected_idx].name
                } else {
                    "Ninguna"
                };
                self.screen.render_prompt_line(&format!(
                    "Extensiones ({}/{}): {} │ [Espacio: Alternar] [R: Recargar] [N: Crear ejemplo] [Esc: Salir]",
                    if total > 0 { selected_idx + 1 } else { 0 }, total, ext_name
                ));
            }
            EditorMode::UnsavedConfirm(dialog) => {
                let (lbl0, lbl1, lbl2) = dialog.button_labels();
                let sel = dialog.selected_button;
                let b0 = if sel == 0 { format!("[{}]", lbl0) } else { lbl0.to_string() };
                let b1 = if sel == 1 { format!("[{}]", lbl1) } else { lbl1.to_string() };
                let b2 = if sel == 2 { format!("[{}]", lbl2) } else { lbl2.to_string() };
                let prompt_text = format!(
                    "⚠ {} │ {}  {}  {} │ [G: Guardar] [D: Descartar] [Esc: Cancelar]",
                    dialog.file_name, b0, b1, b2
                );
                self.screen.render_prompt_line(&prompt_text);
            }
            EditorMode::FontSelector(dialog) => {
                let prompt_text = format!(
                    "Fuente: {} │ Tamaño: {}pt │ Grosor: {} │ [Tab: Sección] [Enter: Aplicar] [Esc: Cancelar]",
                    dialog.selected_family.display_name(),
                    dialog.selected_size,
                    dialog.selected_weight.display_name(),
                );
                self.screen.render_prompt_line(&prompt_text);
            }
            EditorMode::Normal => {
                let diag_msg = self.diagnostics.for_line(primary_pos.line + 1).map(|d| d.to_display_string());
                let msg_text = self
                    .status_message
                    .as_ref()
                    .map(|(msg, _)| msg.as_str())
                    .or(diag_msg.as_deref())
                    .unwrap_or("");
                self.screen.render_prompt_line(msg_text);
            }
        }

        // Calculate cursor screen position (accounting for row 0 tab bar)
        let (cursor_screen_row, cursor_screen_col) = match &self.mode {
            EditorMode::ModalPrompt(modal) => {
                let prompt_row = self.screen.rows.saturating_sub(1);
                let prompt_col = modal.kind.label().chars().count() + modal.cursor_col;
                (prompt_row, prompt_col)
            }
            EditorMode::AboutDialog | EditorMode::ExtensionsManager { .. } | EditorMode::UnsavedConfirm(_) | EditorMode::FontSelector(_) => {
                (self.screen.rows.saturating_sub(1), 0)
            }
            EditorMode::Normal => {
                let tab_offset = if self.screen.has_tab_bar { 1 } else { 0 };
                let row = tab_offset + primary_pos.line.saturating_sub(self.screen.scroll_row);
                let col = gutter_width + primary_pos.col.saturating_sub(self.screen.scroll_col);
                (row, col)
            }
        };

        self.screen.end_frame(cursor_screen_row, cursor_screen_col)?;
        Ok(())
    }

    /// Runs the editor in native desktop GUI window mode.
    /// Interacts directly with OS windowing (X11 / Win32 / Cocoa) with zero external dependencies.
    pub fn run_gui(&mut self) -> io::Result<()> {
        let initial_width = self.config.gui.window_width.max(400);
        let initial_height = self.config.gui.window_height.max(300);
        let mut window = crate::window::NativeWindow::new("Rustext", initial_width, initial_height)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let monitor_scale = window.scale_factor();
        let effective_scale = if self.config.gui.scale_factor > 0.0 {
            self.config.gui.scale_factor
        } else {
            monitor_scale
        };

        let mut fb = crate::gfx::Framebuffer::new(window.width, window.height);
        let mut renderer = crate::gfx::GfxRenderer::new();
        renderer.set_scale_factor(effective_scale);
        self.menu_bar.set_scale_factor(effective_scale);
        self.menu_bar.set_window_width(fb.width);
        if self.config.gui.font_scale > 1 {
            renderer.font_scale = self.config.gui.font_scale;
        }
        fb.font_scale = renderer.font_scale;
        renderer.font_family = crate::gfx::FontFamily::from_name(&self.config.gui.font_family);
        renderer.font_size = self.config.gui.font_size;
        renderer.font_weight = crate::gfx::FontWeight::from_name(&self.config.gui.font_weight);
        fb.font_family = renderer.font_family;
        fb.font_size = renderer.font_size;
        fb.font_weight = renderer.font_weight;
        renderer.colors = crate::gfx::ThemeColors::by_name(&self.config.theme.name);
        renderer.show_line_numbers = self.config.editor.show_line_numbers;
        renderer.font_antialiasing = self.config.gui.font_antialiasing;
        renderer.gamma_correction = self.config.gui.gamma_correction;
        renderer.rounded_corners = self.config.gui.rounded_corners;
        renderer.shadows = self.config.gui.shadows;
        renderer.cursor_style = self.config.gui.cursor_style.clone();
        renderer.cursor_blink = self.config.gui.cursor_blink;
        renderer.font_ligatures = self.config.gui.font_ligatures;
        renderer.font_hinting = crate::gfx::HintingLevel::from_str(&self.config.gui.font_hinting);
        renderer.smooth_shadows = self.config.gui.smooth_shadows;
        renderer.animation_enabled = self.config.gui.animation_enabled;
        renderer.edge_perception = self.config.gui.edge_perception;
        renderer.sdf_font_rendering = self.config.gui.sdf_font_rendering;
        renderer.optical_punctuation_spacing = self.config.gui.optical_punctuation_spacing;
        renderer.use_ttf_font = self.config.gui.use_ttf_font;
        renderer.word_wrap = self.word_wrap;
        fb.sdf_font_rendering = self.config.gui.sdf_font_rendering;
        fb.use_ttf_font = self.config.gui.use_ttf_font;
        if !self.config.gui.ttf_font_path.trim().is_empty() {
            crate::gfx::ttf::SystemFontLoader::set_active_font(Some(&self.config.gui.ttf_font_path));
        }
        self.sidebar.is_visible = self.config.gui.sidebar;
        self.sidebar.width = renderer.scale_coord(self.config.gui.sidebar_width);
        renderer.minimap_enabled = self.config.gui.minimap;
        renderer.minimap_width = renderer.scale_coord(self.config.gui.minimap_width);
        self.terminal.height = renderer.scale_coord(self.config.gui.terminal_height);

        #[cfg(target_os = "linux")]
        let backend_info = format!("{} ({}x{} @ {:.1}x)", window.backend_name(), window.width, window.height, effective_scale);
        #[cfg(not(target_os = "linux"))]
        let backend_info = format!("{}x{} @ {:.1}x", window.width, window.height, effective_scale);

        eprintln!("[Rustext] Ventana nativa iniciada: {}", backend_info);
        self.set_status(format!("Rustext listo • {}", backend_info));

        let mut active_theme_name = self.config.theme.name.clone();
        let mut active_show_line_numbers = self.config.editor.show_line_numbers;
        let mut active_gui_settings = self.config.gui.clone();

        let mut mouse_x = 0;
        let mut mouse_y = 0;
        let mut is_dragging_scrollbar = false;
        let mut scrollbar_drag_offset_y = 0;
        let mut scrollbar_hovered = false;
        let mut is_dragging_minimap = false;
        let mut minimap_hovered = false;
        let mut is_dragging_splitter = false;
        let mut is_dragging_term_splitter = false;
        let mut adjust_viewport = true;
        let mut needs_redraw = true;
        let mut cached_title = String::new();
        let mut cached_primary_git: Option<(Option<PathBuf>, u64, crate::editor::git::GitGutterDiff)> = None;
        let mut cached_secondary_git: Option<(Option<PathBuf>, u64, crate::editor::git::GitGutterDiff)> = None;
        let mut cached_git_statuses: Option<std::collections::HashMap<PathBuf, crate::editor::git::GitFileStatus>> = None;

        while !self.should_quit && !window.should_close() {
            // Check dynamic config changes
            if renderer.word_wrap != self.word_wrap {
                renderer.word_wrap = self.word_wrap;
                needs_redraw = true;
                adjust_viewport = true;
            }
            if self.config.theme.name != active_theme_name {
                renderer.colors = crate::gfx::ThemeColors::by_name(&self.config.theme.name);
                active_theme_name = self.config.theme.name.clone();
                needs_redraw = true;
            }
            if self.config.editor.show_line_numbers != active_show_line_numbers {
                renderer.show_line_numbers = self.config.editor.show_line_numbers;
                active_show_line_numbers = self.config.editor.show_line_numbers;
                needs_redraw = true;
            }
            if self.config.gui != active_gui_settings {
                let effective_scale = if self.config.gui.scale_factor > 0.0 {
                    self.config.gui.scale_factor
                } else {
                    window.scale_factor()
                };
                renderer.set_scale_factor(effective_scale);
                self.menu_bar.set_scale_factor(effective_scale);
                if self.config.gui.font_scale > 1 {
                    renderer.font_scale = self.config.gui.font_scale;
                }
                fb.font_scale = renderer.font_scale;
                renderer.font_family = crate::gfx::FontFamily::from_name(&self.config.gui.font_family);
                renderer.font_size = self.config.gui.font_size;
                renderer.font_weight = crate::gfx::FontWeight::from_name(&self.config.gui.font_weight);
                fb.font_family = renderer.font_family;
                fb.font_size = renderer.font_size;
                fb.font_weight = renderer.font_weight;
                renderer.font_antialiasing = self.config.gui.font_antialiasing;
                renderer.gamma_correction = self.config.gui.gamma_correction;
                renderer.rounded_corners = self.config.gui.rounded_corners;
                renderer.shadows = self.config.gui.shadows;
                renderer.cursor_style = self.config.gui.cursor_style.clone();
                renderer.cursor_blink = self.config.gui.cursor_blink;
                renderer.font_ligatures = self.config.gui.font_ligatures;
                renderer.font_hinting = crate::gfx::HintingLevel::from_str(&self.config.gui.font_hinting);
                renderer.smooth_shadows = self.config.gui.smooth_shadows;
                renderer.animation_enabled = self.config.gui.animation_enabled;
                renderer.edge_perception = self.config.gui.edge_perception;
                renderer.subpixel_antialiasing = self.config.gui.subpixel_antialiasing;
                fb.subpixel_antialiasing = self.config.gui.subpixel_antialiasing;
                renderer.letter_spacing = self.config.gui.letter_spacing;
                fb.letter_spacing = self.config.gui.letter_spacing;
                renderer.font_aspect_ratio = self.config.gui.font_aspect_ratio;
                fb.font_aspect_ratio = self.config.gui.font_aspect_ratio;
                renderer.stem_darkening = self.config.gui.stem_darkening;
                fb.stem_darkening = self.config.gui.stem_darkening;
                renderer.stem_darkening_amount = self.config.gui.stem_darkening_amount;
                fb.stem_darkening_amount = self.config.gui.stem_darkening_amount;
                renderer.sdf_font_rendering = self.config.gui.sdf_font_rendering;
                renderer.optical_punctuation_spacing = self.config.gui.optical_punctuation_spacing;
                renderer.use_ttf_font = self.config.gui.use_ttf_font;
                fb.sdf_font_rendering = self.config.gui.sdf_font_rendering;
                fb.use_ttf_font = self.config.gui.use_ttf_font;
                if !self.config.gui.ttf_font_path.trim().is_empty() {
                    crate::gfx::ttf::SystemFontLoader::set_active_font(Some(&self.config.gui.ttf_font_path));
                }
                self.sidebar.is_visible = self.config.gui.sidebar;
                self.sidebar.width = renderer.scale_coord(self.config.gui.sidebar_width);
                renderer.minimap_enabled = self.config.gui.minimap;
                renderer.minimap_width = renderer.scale_coord(self.config.gui.minimap_width);
                self.terminal.height = renderer.scale_coord(self.config.gui.terminal_height);
                active_gui_settings = self.config.gui.clone();
                needs_redraw = true;
            }
            // Check status message expiry (clear after 6 seconds)
            if let Some((_, created_at)) = self.status_message {
                if created_at.elapsed() > std::time::Duration::from_secs(6) {
                    self.status_message = None;
                    needs_redraw = true;
                }
            }
            // Poll background terminal output if active
            if self.terminal.is_visible && self.terminal.poll_output() {
                needs_redraw = true;
            }

            // Drain all pending window events
            loop {
                let event = window.poll_event();
                match event {
                    crate::window::WindowEvent::None => break,
                    crate::window::WindowEvent::Close => {
                        let any_dirty = self.documents.iter().any(|d| d.buffer.is_dirty());
                        if any_dirty && !self.config.editor.hot_exit {
                            self.prompt_unsaved_quit();
                            window.cancel_close();
                            needs_redraw = true;
                        } else {
                            let _ = self.save_session();
                            self.should_quit = true;
                            break;
                        }
                    }
                    crate::window::WindowEvent::Resize { width, height } => {
                        fb.resize(width, height);
                        self.menu_bar.set_window_width(width);
                        needs_redraw = true;
                    }
                    crate::window::WindowEvent::MouseDown { x, y, button } => {
                        mouse_x = x;
                        mouse_y = y;
                        needs_redraw = true;
                        if button == crate::window::MouseButton::Left {
                            // 0. Check Unsaved Confirm Dialog interaction
                            if let EditorMode::UnsavedConfirm(dialog) = &mut self.mode {
                                let win_w = renderer.scale_coord(520).min(fb.width.saturating_sub(renderer.scale_coord(20)));
                                let win_h = renderer.scale_coord(190).min(fb.height.saturating_sub(renderer.scale_coord(20)));
                                let win_x = (fb.width.saturating_sub(win_w)) / 2;
                                let win_y = (fb.height.saturating_sub(win_h)) / 2;
                                let title_h = renderer.scale_coord(28);
                                let close_w = renderer.scale_coord(36);
                                let close_x = win_x + win_w - close_w;

                                let hit_close = x >= close_x && x < close_x + close_w && y >= win_y && y < win_y + title_h;
                                let hit_outside = x < win_x || x >= win_x + win_w || y < win_y || y >= win_y + win_h;

                                let btn_bar_y = win_y + win_h - renderer.scale_coord(42);
                                let btn_h = renderer.scale_coord(28);
                                let (lbl_save, lbl_discard, lbl_cancel) = dialog.button_labels();
                                let btn_save_w = (lbl_save.chars().count() * renderer.char_width() + renderer.scale_coord(24)).max(renderer.scale_coord(100));
                                let btn_discard_w = (lbl_discard.chars().count() * renderer.char_width() + renderer.scale_coord(24)).max(renderer.scale_coord(110));
                                let btn_cancel_w = (lbl_cancel.chars().count() * renderer.char_width() + renderer.scale_coord(24)).max(renderer.scale_coord(90));
                                let total_btns_w = btn_save_w + btn_discard_w + btn_cancel_w + renderer.scale_coord(16);
                                let btn_start_x = win_x + win_w - total_btns_w - renderer.scale_coord(16);

                                let b0_x = btn_start_x;
                                let b1_x = b0_x + btn_save_w + renderer.scale_coord(8);
                                let b2_x = b1_x + btn_discard_w + renderer.scale_coord(8);

                                let hit_b0 = y >= btn_bar_y && y < btn_bar_y + btn_h && x >= b0_x && x < b0_x + btn_save_w;
                                let hit_b1 = y >= btn_bar_y && y < btn_bar_y + btn_h && x >= b1_x && x < b1_x + btn_discard_w;
                                let hit_b2 = y >= btn_bar_y && y < btn_bar_y + btn_h && x >= b2_x && x < b2_x + btn_cancel_w;

                                if hit_close || hit_outside || hit_b2 {
                                    self.confirm_unsaved_action(2); // Cancel
                                } else if hit_b0 {
                                    self.confirm_unsaved_action(0); // Save
                                } else if hit_b1 {
                                    self.confirm_unsaved_action(1); // Discard
                                }
                            // A. Check About Dialog Window interaction
                            } else if self.mode == EditorMode::AboutDialog {
                                let win_w = renderer.scale_coord(640).min(fb.width.saturating_sub(renderer.scale_coord(20)));
                                let win_h = renderer.scale_coord(450).min(fb.height.saturating_sub(renderer.scale_coord(20)));
                                let win_x = (fb.width.saturating_sub(win_w)) / 2;
                                let win_y = (fb.height.saturating_sub(win_h)) / 2;
                                let close_w = renderer.scale_coord(36);
                                let close_x = win_x + win_w - close_w;
                                let btn_w = renderer.scale_coord(130);
                                let btn_h = renderer.scale_coord(28);
                                let btn_x = win_x + (win_w - btn_w) / 2;
                                let btn_y = win_y + win_h - renderer.scale_coord(38);

                                let hit_close = x >= close_x && x < close_x + close_w && y >= win_y && y < win_y + renderer.scale_coord(28);
                                let hit_ok = x >= btn_x && x < btn_x + btn_w && y >= btn_y && y < btn_y + btn_h;
                                let hit_outside = x < win_x || x >= win_x + win_w || y < win_y || y >= win_y + win_h;

                                if hit_close || hit_ok || hit_outside {
                                    self.mode = EditorMode::Normal;
                                }
                            // B. Check Extensions Manager Dialog interaction
                            } else if let EditorMode::ExtensionsManager { selected_idx, scroll_offset } = &mut self.mode {
                                let win_w = renderer.scale_coord(680).min(fb.width.saturating_sub(renderer.scale_coord(20)));
                                let win_h = renderer.scale_coord(480).min(fb.height.saturating_sub(renderer.scale_coord(20)));
                                let win_x = (fb.width.saturating_sub(win_w)) / 2;
                                let win_y = (fb.height.saturating_sub(win_h)) / 2;
                                let close_w = renderer.scale_coord(36);
                                let close_x = win_x + win_w - close_w;
                                let hit_top_close = x >= close_x && x < close_x + close_w && y >= win_y && y < win_y + renderer.scale_coord(28);

                                let footer_y = win_y + win_h - renderer.scale_coord(34);
                                let close_btn_w = renderer.scale_coord(80);
                                let close_btn_x = win_x + win_w - close_btn_w - renderer.scale_coord(12);
                                let hit_bottom_close = x >= close_btn_x && x < close_btn_x + close_btn_w && y >= footer_y && y < footer_y + renderer.scale_coord(26);

                                let hit_outside = x < win_x || x >= win_x + win_w || y < win_y || y >= win_y + win_h;

                                if hit_top_close || hit_bottom_close || hit_outside {
                                    self.mode = EditorMode::Normal;
                                } else {
                                    let body_y = win_y + renderer.scale_coord(28) + 1;
                                    let bar_y = body_y + renderer.scale_coord(8);
                                    let reload_btn_w = renderer.scale_coord(90);
                                    let create_btn_w = renderer.scale_coord(120);
                                    let reload_btn_x = win_x + win_w - reload_btn_w - renderer.scale_coord(12);
                                    let create_btn_x = reload_btn_x - create_btn_w - renderer.scale_coord(8);

                                    if x >= create_btn_x && x < create_btn_x + create_btn_w && y >= bar_y && y < bar_y + renderer.scale_coord(24) {
                                        self.execute_action(EditorAction::CreateSampleExtension);
                                    } else if x >= reload_btn_x && x < reload_btn_x + reload_btn_w && y >= bar_y && y < bar_y + renderer.scale_coord(24) {
                                        self.execute_action(EditorAction::ReloadExtensions);
                                    } else {
                                        let list_x = win_x + renderer.scale_coord(12);
                                        let list_y = bar_y + renderer.scale_coord(30);
                                        let list_w = win_w.saturating_sub(renderer.scale_coord(24));
                                        let list_h = win_h.saturating_sub(renderer.scale_coord(115));
                                        let card_h = (renderer.char_height() * 3 + renderer.scale_coord(10)).max(renderer.scale_coord(58));

                                        if x >= list_x && x < list_x + list_w && y >= list_y && y < list_y + list_h {
                                            let card_idx = (y - list_y) / card_h;
                                            let actual_idx = *scroll_offset + card_idx;
                                            if actual_idx < self.extension_manager.extensions.len() {
                                                *selected_idx = actual_idx;
                                                let card_y = list_y + 2 + card_idx * card_h;
                                                let toggle_w = renderer.scale_coord(64);
                                                let toggle_x = list_x + list_w - toggle_w - renderer.scale_coord(8);
                                                if x >= toggle_x && x < toggle_x + toggle_w && y >= card_y + 4 && y < card_y + renderer.scale_coord(24) {
                                                    let name = self.extension_manager.extensions[actual_idx].name.clone();
                                                    let state = self.extension_manager.toggle_extension(actual_idx);
                                                    self.set_status(format!("Extensión '{}': {}", name, if state { "Activada" } else { "Desactivada" }));
                                                }
                                            }
                                        }
                                    }
                                }
                            // B2. Check Font Selector Dialog interaction
                            } else if let EditorMode::FontSelector(dialog) = &mut self.mode {
                                let win_w = renderer.scale_coord(760).min(fb.width.saturating_sub(renderer.scale_coord(20)));
                                let win_h = renderer.scale_coord(520).min(fb.height.saturating_sub(renderer.scale_coord(20)));
                                let win_x = (fb.width.saturating_sub(win_w)) / 2;
                                let win_y = (fb.height.saturating_sub(win_h)) / 2;
                                let title_h = renderer.scale_coord(28);
                                let close_w = renderer.scale_coord(36);
                                let close_x = win_x + win_w - close_w;

                                let hit_close = x >= close_x && x < close_x + close_w && y >= win_y && y < win_y + title_h;
                                let hit_outside = x < win_x || x >= win_x + win_w || y < win_y || y >= win_y + win_h;

                                let footer_h = renderer.scale_coord(44);
                                let footer_y = win_y + win_h - footer_h;
                                let btn_cancel_w = renderer.scale_coord(95);
                                let btn_apply_w = renderer.scale_coord(145);
                                let btn_h = renderer.scale_coord(26);
                                let btn_y = footer_y + (footer_h.saturating_sub(btn_h)) / 2;
                                let btn_apply_x = win_x + win_w - btn_apply_w - renderer.scale_coord(14);
                                let btn_cancel_x = btn_apply_x - btn_cancel_w - renderer.scale_coord(10);

                                let hit_apply = x >= btn_apply_x && x < btn_apply_x + btn_apply_w && y >= btn_y && y < btn_y + btn_h;
                                let hit_cancel = x >= btn_cancel_x && x < btn_cancel_x + btn_cancel_w && y >= btn_y && y < btn_y + btn_h;

                                if hit_close || hit_outside || hit_cancel {
                                    self.mode = EditorMode::Normal;
                                    self.set_status("Selector de fuentes cancelado");
                                } else if hit_apply {
                                    let dialog_clone = dialog.clone();
                                    self.config.gui.font_family = dialog_clone.selected_family.as_str().to_string();
                                    self.config.gui.font_size = dialog_clone.selected_size;
                                    self.config.gui.font_weight = dialog_clone.selected_weight.as_str().to_string();
                                    self.config.gui.line_spacing = dialog_clone.line_spacing;
                                    self.config.gui.font_ligatures = dialog_clone.ligatures;
                                    self.config.gui.font_antialiasing = dialog_clone.antialiasing;
                                    self.config.gui.letter_spacing = dialog_clone.letter_spacing;
                                    self.config.gui.font_aspect_ratio = dialog_clone.font_aspect_ratio;
                                    self.config.gui.stem_darkening = dialog_clone.stem_darkening;
                                    self.config.gui.scale_factor = dialog_clone.scale_factor;
                                    self.config.gui.ergonomic_scaling = dialog_clone.ergonomic_scaling;
                                    self.config.gui.sdf_font_rendering = dialog_clone.sdf_rendering;
                                    self.config.gui.optical_punctuation_spacing = dialog_clone.optical_spacing;
                                    self.config.gui.use_ttf_font = dialog_clone.use_ttf;
                                    self.config.gui.font_hinting = match dialog_clone.hinting {
                                        crate::gfx::HintingLevel::None => "none".to_string(),
                                        crate::gfx::HintingLevel::Slight => "slight".to_string(),
                                        crate::gfx::HintingLevel::Full => "full".to_string(),
                                        crate::gfx::HintingLevel::Medium => "medium".to_string(),
                                    };
                                    let _ = self.config.save();
                                    self.set_status(format!(
                                        "Tipografía actualizada: {} {}pt ({})",
                                        dialog_clone.selected_family.display_name(),
                                        dialog_clone.selected_size,
                                        dialog_clone.selected_weight.display_name()
                                    ));
                                    self.mode = EditorMode::Normal;
                                } else {
                                    let body_y = win_y + title_h + 1;
                                    let col_w = (win_w.saturating_sub(renderer.scale_coord(36))) / 2;
                                    let left_x = win_x + renderer.scale_coord(14);
                                    let content_y = body_y + renderer.scale_coord(8);

                                    // Font families
                                    let fam_start_y = content_y + renderer.scale_coord(16);
                                    let fam_card_h = renderer.scale_coord(34);
                                    for (i, &f) in crate::gfx::FontFamily::ALL.iter().enumerate() {
                                        let card_y = fam_start_y + i * (fam_card_h + renderer.scale_coord(3));
                                        if x >= left_x && x < left_x + col_w && y >= card_y && y < card_y + fam_card_h {
                                            dialog.selected_family = f;
                                            dialog.family_idx = i;
                                            break;
                                        }
                                    }

                                    // Font sizes
                                    let size_sec_y = fam_start_y + 5 * (fam_card_h + renderer.scale_coord(3)) + renderer.scale_coord(6);
                                    let size_pills_y = size_sec_y + renderer.scale_coord(16);
                                    let pill_h = renderer.scale_coord(22);
                                    let pill_w = (col_w.saturating_sub(renderer.scale_coord(28))) / crate::editor::font_selector::AVAILABLE_SIZES.len().max(1);
                                    for (idx, &s) in crate::editor::font_selector::AVAILABLE_SIZES.iter().enumerate() {
                                        let px = left_x + idx * (pill_w + renderer.scale_coord(4));
                                        if x >= px && x < px + pill_w && y >= size_pills_y && y < size_pills_y + pill_h {
                                            dialog.selected_size = s;
                                            dialog.size_idx = idx;
                                            break;
                                        }
                                    }

                                    // Font weights
                                    let weight_sec_y = size_pills_y + pill_h + renderer.scale_coord(8);
                                    let weight_pills_y = weight_sec_y + renderer.scale_coord(16);
                                    let weight_w = (col_w.saturating_sub(renderer.scale_coord(16))) / 3;
                                    for (idx, &w) in crate::gfx::FontWeight::ALL.iter().enumerate() {
                                        let wx = left_x + idx * (weight_w + renderer.scale_coord(8));
                                        if x >= wx && x < wx + weight_w && y >= weight_pills_y && y < weight_pills_y + pill_h {
                                            dialog.selected_weight = w;
                                            dialog.weight_idx = idx;
                                            break;
                                        }
                                    }

                                    // Advanced options
                                    let opts_sec_y = weight_pills_y + pill_h + renderer.scale_coord(8);
                                    let opts_chips_y = opts_sec_y + renderer.scale_coord(16);
                                    let chip_w = (col_w.saturating_sub(renderer.scale_coord(16))) / 3;
                                    let c0_x = left_x;
                                    let c1_x = c0_x + chip_w + renderer.scale_coord(8);
                                    let c2_x = c1_x + chip_w + renderer.scale_coord(8);

                                    let opts_row2_y = opts_chips_y + pill_h + renderer.scale_coord(6);
                                    let chip2_w = (col_w.saturating_sub(renderer.scale_coord(16))) / 3;
                                    let r2_c0_x = left_x;
                                    let r2_c1_x = r2_c0_x + chip2_w + renderer.scale_coord(8);
                                    let r2_c2_x = r2_c1_x + chip2_w + renderer.scale_coord(8);

                                    let opts_row3_y = opts_row2_y + pill_h + renderer.scale_coord(6);
                                    let chip3_w = (col_w.saturating_sub(renderer.scale_coord(16))) / 3;
                                    let r3_c0_x = left_x;
                                    let r3_c1_x = r3_c0_x + chip3_w + renderer.scale_coord(8);
                                    let r3_c2_x = r3_c1_x + chip3_w + renderer.scale_coord(8);

                                    if y >= opts_chips_y && y < opts_chips_y + pill_h {
                                        if x >= c0_x && x < c0_x + chip_w {
                                            dialog.toggle_ligatures();
                                        } else if x >= c1_x && x < c1_x + chip_w {
                                            dialog.toggle_antialiasing();
                                        } else if x >= c2_x && x < c2_x + chip_w {
                                            dialog.cycle_line_spacing();
                                        }
                                    } else if y >= opts_row2_y && y < opts_row2_y + pill_h {
                                        if x >= r2_c0_x && x < r2_c0_x + chip2_w {
                                            dialog.cycle_letter_spacing();
                                        } else if x >= r2_c1_x && x < r2_c1_x + chip2_w {
                                            dialog.cycle_aspect_ratio();
                                        } else if x >= r2_c2_x && x < r2_c2_x + chip2_w {
                                            dialog.toggle_stem_darkening();
                                        }
                                    } else if y >= opts_row3_y && y < opts_row3_y + pill_h {
                                        if x >= r3_c0_x && x < r3_c0_x + chip3_w {
                                            dialog.toggle_sdf_rendering();
                                        } else if x >= r3_c1_x && x < r3_c1_x + chip3_w {
                                            dialog.toggle_optical_spacing();
                                        } else if x >= r3_c2_x && x < r3_c2_x + chip3_w {
                                            dialog.toggle_ttf_rendering();
                                        }
                                    }

                                    // Check click on ISO 9241-303 / HiDPI card in right column
                                    let right_x = left_x + col_w + renderer.scale_coord(8);
                                    let iso_card_h = renderer.scale_coord(76);
                                    let prev_box_y = content_y + renderer.scale_coord(16);
                                    let content_h = (win_h.saturating_sub(title_h + 1)).saturating_sub(footer_h + renderer.scale_coord(12));
                                    let prev_box_h = content_h.saturating_sub(renderer.scale_coord(16) + iso_card_h + renderer.scale_coord(8));
                                    let iso_card_y = prev_box_y + prev_box_h + renderer.scale_coord(8);

                                    if x >= right_x && x < right_x + col_w && y >= iso_card_y && y < iso_card_y + iso_card_h {
                                        dialog.cycle_scale_factor();
                                    }
                                }
                            // C. Check Modal Prompt Window interaction
                            } else if let EditorMode::ModalPrompt(modal) = &mut self.mode {
                                if modal.is_file_dialog() {
                                    let box_w = fb.width.min(renderer.scale_coord(680)).max(renderer.scale_coord(420));
                                    let box_h = fb.height.min(renderer.scale_coord(480)).max(renderer.scale_coord(360));
                                    let box_x = (fb.width.saturating_sub(box_w)) / 2;
                                    let box_y = (fb.height.saturating_sub(box_h)) / 2;
                                    let close_w = renderer.scale_coord(32);
                                    let header_h = renderer.scale_coord(26);
                                    let close_x = box_x + box_w - close_w;

                                    let hit_close = x >= close_x && x < close_x + close_w && y >= box_y && y < box_y + header_h;
                                    let hit_outside = x < box_x || x >= box_x + box_w || y < box_y || y >= box_y + box_h;

                                    let btn_bar_y = box_y + box_h - renderer.scale_coord(30);
                                    let action_btn_w = renderer.scale_coord(88);
                                    let cancel_btn_w = renderer.scale_coord(80);
                                    let cancel_btn_x = box_x + box_w - cancel_btn_w - renderer.scale_coord(12);
                                    let action_btn_x = cancel_btn_x - action_btn_w - renderer.scale_coord(8);
                                    let btn_h = renderer.scale_coord(24);

                                    let hit_cancel = y >= btn_bar_y && y < btn_bar_y + btn_h && x >= cancel_btn_x && x < cancel_btn_x + cancel_btn_w;
                                    let hit_action = y >= btn_bar_y && y < btn_bar_y + btn_h && x >= action_btn_x && x < action_btn_x + action_btn_w;

                                    if hit_close || hit_outside || hit_cancel {
                                        self.mode = EditorMode::Normal;
                                    } else if hit_action {
                                        let mut navigate_path = None;
                                        if !modal.dir_entries.is_empty() && modal.selected_entry_idx < modal.dir_entries.len() {
                                            let entry = &modal.dir_entries[modal.selected_entry_idx];
                                            if entry.is_dir && (modal.input.trim().is_empty() || modal.input.trim() == entry.name.trim_end_matches('/')) {
                                                navigate_path = Some(entry.path.clone());
                                            }
                                        }
                                        if let Some(target) = navigate_path {
                                            modal.navigate_to(target);
                                            modal.input.clear();
                                            modal.cursor_col = 0;
                                        } else {
                                            let modal_clone = modal.clone();
                                            self.mode = EditorMode::Normal;
                                            self.commit_prompt(&modal_clone);
                                        }
                                    } else {
                                        // [Subir] button
                                        let dir_bar_y = box_y + header_h + 1 + renderer.scale_coord(6);
                                        let up_btn_w = renderer.scale_coord(64);
                                        let up_btn_x = box_x + box_w - up_btn_w - renderer.scale_coord(12);
                                        if x >= up_btn_x && x < up_btn_x + up_btn_w && y >= dir_bar_y + renderer.scale_coord(2) && y < dir_bar_y + renderer.scale_coord(22) {
                                            modal.navigate_up();
                                        } else {
                                            // Bookmarks
                                            let bm_bar_y = dir_bar_y + renderer.scale_coord(24) + renderer.scale_coord(4);
                                            let bookmarks = crate::ui::prompt::get_quick_bookmarks();
                                            let mut cur_bm_x = box_x + renderer.scale_coord(8);
                                            let mut clicked_bm = false;
                                            for bm in &bookmarks {
                                                let bm_text = format!("[{}]", bm.label);
                                                let bm_w = bm_text.chars().count() * renderer.char_width() + renderer.scale_coord(8);
                                                if cur_bm_x + bm_w < box_x + box_w - renderer.scale_coord(8) {
                                                    if y >= bm_bar_y && y < bm_bar_y + renderer.scale_coord(20) && x >= cur_bm_x && x < cur_bm_x + bm_w {
                                                        modal.navigate_to(bm.path.clone());
                                                        clicked_bm = true;
                                                        break;
                                                    }
                                                    cur_bm_x += bm_w + renderer.scale_coord(6);
                                                }
                                            }

                                            if !clicked_bm {
                                                // Directory entries list
                                                let list_y = bm_bar_y + renderer.scale_coord(26);
                                                let list_h = box_h.saturating_sub(renderer.scale_coord(190));
                                                let list_w = box_w.saturating_sub(renderer.scale_coord(16));
                                                let list_x = box_x + renderer.scale_coord(8);
                                                let row_h = (renderer.char_height() + renderer.scale_coord(6)).max(renderer.scale_coord(22));

                                                if x >= list_x && x < list_x + list_w && y >= list_y && y < list_y + list_h {
                                                    let row_click_idx = (y - list_y) / row_h;
                                                    let actual_idx = modal.scroll_offset + row_click_idx;
                                                    if actual_idx < modal.dir_entries.len() {
                                                        let entry = modal.dir_entries[actual_idx].clone();
                                                        if actual_idx == modal.selected_entry_idx {
                                                            if entry.is_dir {
                                                                modal.navigate_to(entry.path);
                                                                modal.input.clear();
                                                                modal.cursor_col = 0;
                                                            } else {
                                                                modal.input = entry.name.clone();
                                                                modal.cursor_col = modal.input.chars().count();
                                                                if modal.kind == PromptKind::OpenFile {
                                                                    let modal_clone = modal.clone();
                                                                    self.mode = EditorMode::Normal;
                                                                    self.commit_prompt(&modal_clone);
                                                                }
                                                            }
                                                        } else {
                                                            modal.selected_entry_idx = actual_idx;
                                                            if !entry.is_dir {
                                                                modal.input = entry.name.clone();
                                                                modal.cursor_col = modal.input.chars().count();
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else if modal.kind == PromptKind::Replace {
                                    let box_w = fb.width.min(renderer.scale_coord(540)).max(renderer.scale_coord(380));
                                    let box_h = renderer.scale_coord(168);
                                    let box_x = (fb.width.saturating_sub(box_w)) / 2;
                                    let box_y = renderer.scale_coord(28);
                                    let close_w = renderer.scale_coord(30);
                                    let header_h = renderer.scale_coord(24);
                                    let close_x = box_x + box_w - close_w;

                                    let hit_close = x >= close_x && x < close_x + close_w && y >= box_y && y < box_y + header_h;
                                    let hit_outside = x < box_x || x >= box_x + box_w || y < box_y || y >= box_y + box_h;

                                    if hit_close || hit_outside {
                                        self.mode = EditorMode::Normal;
                                    } else {
                                        let body_y = box_y + header_h + 1;
                                        let input_margin = renderer.scale_coord(12);
                                        let label_w = renderer.scale_coord(100);
                                        let input_w = box_w.saturating_sub(input_margin * 2 + label_w);
                                        let input_h = renderer.scale_coord(24);
                                        let row1_y = body_y + renderer.scale_coord(12);
                                        let row2_y = row1_y + input_h + renderer.scale_coord(10);
                                        let input_x = box_x + input_margin + label_w;

                                        // Click row 1 input (Buscar)
                                        if x >= input_x && x < input_x + input_w && y >= row1_y && y < row1_y + input_h {
                                            modal.active_field = 0;
                                        // Click row 2 input (Reemplazar)
                                        } else if x >= input_x && x < input_x + input_w && y >= row2_y && y < row2_y + input_h {
                                            modal.active_field = 1;
                                        } else {
                                            let btn_y = row2_y + input_h + renderer.scale_coord(16);
                                            let btn_h = renderer.scale_coord(24);
                                            let b1_x = box_x + input_margin;
                                            let b1_w = renderer.scale_coord(68);
                                            let b2_x = b1_x + b1_w + renderer.scale_coord(8);
                                            let b2_w = renderer.scale_coord(68);
                                            let b3_x = b2_x + b2_w + renderer.scale_coord(14);
                                            let b3_w = renderer.scale_coord(98);
                                            let b4_x = b3_x + b3_w + renderer.scale_coord(8);
                                            let b4_w = renderer.scale_coord(126);

                                            if y >= btn_y && y < btn_y + btn_h {
                                                if x >= b1_x && x < b1_x + b1_w {
                                                    let query = modal.input.clone();
                                                    self.find_prev_match(&query);
                                                } else if x >= b2_x && x < b2_x + b2_w {
                                                    let query = modal.input.clone();
                                                    self.find_next_match(&query);
                                                } else if x >= b3_x && x < b3_x + b3_w {
                                                    let query = modal.input.clone();
                                                    let repl = modal.replace_input.clone();
                                                    self.replace_current_match(&query, &repl);
                                                } else if x >= b4_x && x < b4_x + b4_w {
                                                    let query = modal.input.clone();
                                                    let repl = modal.replace_input.clone();
                                                    self.replace_all_matches(&query, &repl);
                                                }
                                            }
                                        }
                                    }
                                } else if modal.kind == PromptKind::GlobalSearch {
                                    let box_w = fb.width.min(renderer.scale_coord(740)).max(renderer.scale_coord(460));
                                    let box_h = fb.height.min(renderer.scale_coord(500)).max(renderer.scale_coord(360));
                                    let box_x = (fb.width.saturating_sub(box_w)) / 2;
                                    let box_y = renderer.scale_coord(28);
                                    let header_h = renderer.scale_coord(24);
                                    let close_w = renderer.scale_coord(30);
                                    let close_x = box_x + box_w - close_w;

                                    let hit_close = x >= close_x && x < close_x + close_w && y >= box_y && y < box_y + header_h;
                                    let hit_outside = x < box_x || x >= box_x + box_w || y < box_y || y >= box_y + box_h;

                                    if hit_close || hit_outside {
                                        self.mode = EditorMode::Normal;
                                    } else {
                                        let body_y = box_y + header_h + 1;
                                        let input_margin = renderer.scale_coord(12);
                                        let input_y = body_y + renderer.scale_coord(8);
                                        let input_h = renderer.scale_coord(24);
                                        let subbar_y = input_y + input_h + renderer.scale_coord(8);
                                        let subbar_h = renderer.scale_coord(22);
                                        let input_w = box_w.saturating_sub(input_margin * 2);
                                        let case_btn_text = if modal.case_sensitive { "[Aa Mayús: Sí]" } else { "[Aa Mayús: No]" };
                                        let case_w = case_btn_text.chars().count() * renderer.char_width() + renderer.scale_coord(8);
                                        let case_x = box_x + input_margin + input_w - case_w;

                                        if y >= subbar_y && y < subbar_y + subbar_h && x >= case_x && x < case_x + case_w {
                                            modal.toggle_case_sensitivity();
                                        } else {
                                            let list_y = subbar_y + subbar_h + renderer.scale_coord(8);
                                            let footer_h = renderer.scale_coord(24);
                                            let list_h = (box_y + box_h).saturating_sub(list_y + footer_h + renderer.scale_coord(4));
                                            let row_h = (renderer.char_height() + renderer.scale_coord(6)).max(renderer.scale_coord(22));
                                            let max_visible = list_h / row_h;

                                            if x >= box_x + input_margin && x < box_x + box_w - input_margin && y >= list_y && y < list_y + max_visible * row_h {
                                                let row_idx = (y - list_y) / row_h;
                                                let item_idx = modal.scroll_offset + row_idx;
                                                if item_idx < modal.global_results.len() {
                                                    modal.selected_idx = item_idx;
                                                    let modal_clone = modal.clone();
                                                    self.mode = EditorMode::Normal;
                                                    self.commit_prompt(&modal_clone);
                                                }
                                            }
                                        }
                                    }
                                } else if modal.kind == PromptKind::RecentFiles {
                                    let box_w = fb.width.min(renderer.scale_coord(680)).max(renderer.scale_coord(400));
                                    let box_h = renderer.scale_coord(340);
                                    let box_x = (fb.width.saturating_sub(box_w)) / 2;
                                    let box_y = renderer.scale_coord(28);
                                    let header_h = renderer.scale_coord(24);
                                    let close_w = renderer.scale_coord(30);
                                    let close_x = box_x + box_w - close_w;

                                    let hit_close = x >= close_x && x < close_x + close_w && y >= box_y && y < box_y + header_h;
                                    let hit_outside = x < box_x || x >= box_x + box_w || y < box_y || y >= box_y + box_h;

                                    if hit_close || hit_outside {
                                        self.mode = EditorMode::Normal;
                                    } else {
                                        let input_margin = renderer.scale_coord(12);
                                        let item_start_y = box_y + header_h + 1 + renderer.scale_coord(8) + renderer.scale_coord(24) + renderer.scale_coord(8);
                                        let item_h = (renderer.char_height() + renderer.scale_coord(10)).max(renderer.scale_coord(26));
                                        let matching = modal.matching_recent_files();
                                        if x >= box_x + input_margin && x < box_x + box_w - input_margin && y >= item_start_y {
                                            let row_idx = (y - item_start_y) / item_h;
                                            if row_idx < matching.len().min(9) {
                                                let target_path = matching[row_idx].clone();
                                                self.mode = EditorMode::Normal;
                                                self.open_file(&target_path);
                                            }
                                        }
                                    }
                                } else {
                                    let is_palette = modal.kind == PromptKind::CommandPalette;
                                    let box_w = fb.width.min(renderer.scale_coord(580)).max(renderer.scale_coord(340));
                                    let box_h = if is_palette { renderer.scale_coord(340) } else { renderer.scale_coord(120) };
                                    let box_x = (fb.width.saturating_sub(box_w)) / 2;
                                    let box_y = renderer.scale_coord(28);
                                    let header_h = renderer.scale_coord(24);
                                    let close_w = renderer.scale_coord(30);
                                    let close_x = box_x + box_w - close_w;

                                    let hit_close = x >= close_x && x < close_x + close_w && y >= box_y && y < box_y + header_h;
                                    let hit_outside = x < box_x || x >= box_x + box_w || y < box_y || y >= box_y + box_h;

                                    if hit_close || hit_outside {
                                        self.mode = EditorMode::Normal;
                                    } else if is_palette {
                                        let body_y = box_y + header_h + 1;
                                        let input_margin = renderer.scale_coord(12);
                                        let input_y = body_y + renderer.scale_coord(8);
                                        let item_start_y = input_y + renderer.scale_coord(24) + renderer.scale_coord(8);
                                        let item_h = (renderer.char_height() + renderer.scale_coord(8)).max(renderer.scale_coord(24));
                                        let matching = modal.matching_commands();
                                        if x >= box_x + input_margin && x < box_x + box_w - input_margin && y >= item_start_y {
                                            let row_idx = (y - item_start_y) / item_h;
                                            if row_idx < matching.len().min(9) {
                                                let action = matching[row_idx].action.clone();
                                                self.mode = EditorMode::Normal;
                                                self.execute_action(action);
                                            }
                                        }
                                    }
                                }
                            // C. Normal Editor Mode interaction
                            } else {
                                let sidebar_w = if self.sidebar.is_visible {
                                    self.sidebar.width.min(fb.width.saturating_sub(100))
                                } else {
                                    0
                                };

                                // Check Integrated Terminal interaction
                                let term_hit = if self.terminal.is_visible {
                                    let term_geom = renderer.terminal_geometry(fb.width, fb.height, sidebar_w, self.terminal.height);
                                    if term_geom.contains_point(x, y) {
                                        if term_geom.is_over_close_btn(x, y) {
                                            self.terminal.is_visible = false;
                                            self.terminal.is_focused = false;
                                            self.set_status("Terminal Integrado oculto");
                                        } else if term_geom.is_over_clear_btn(x, y) {
                                            self.terminal.clear();
                                            self.set_status("Terminal limpiado");
                                        } else if term_geom.is_over_splitter(x, y) {
                                            is_dragging_term_splitter = true;
                                        } else {
                                            self.terminal.is_focused = true;
                                            self.sidebar.is_focused = false;
                                        }
                                        needs_redraw = true;
                                        true
                                    } else {
                                        if self.terminal.is_focused {
                                            self.terminal.is_focused = false;
                                            needs_redraw = true;
                                        }
                                        false
                                    }
                                } else {
                                    false
                                };

                                if !term_hit {
                                    // 0. Check Autocomplete Floating Popup Hit Test
                                let ac_hit = if self.autocomplete.is_active && !self.autocomplete.items.is_empty() {
                                    let ac_geom = renderer.autocomplete_geometry(
                                        fb.width,
                                        fb.height,
                                        self.doc(),
                                        sidebar_w,
                                        self.config.gui.menu_bar,
                                        self.autocomplete.items.len(),
                                    );
                                    if ac_geom.contains_point(x, y) {
                                        if let Some(item_idx) = ac_geom.item_index_at(y, self.autocomplete.scroll_offset) {
                                            self.autocomplete.selected_idx = item_idx;
                                            self.commit_autocomplete();
                                            adjust_viewport = true;
                                            needs_redraw = true;
                                        }
                                        true
                                    } else {
                                        self.autocomplete.close();
                                        needs_redraw = true;
                                        false
                                    }
                                } else {
                                    false
                                };

                                if !ac_hit {
                                    // 1. Try Menu Bar and open dropdown
                                    if let Some(action) = self.menu_bar.handle_mouse_click(x, y, renderer.char_width()) {
                                        self.execute_action(action);
                                    } else if !self.menu_bar.is_open() {
                                        let menu_offset = if self.config.gui.menu_bar { renderer.menu_bar_height } else { 0 };
                                        let content_y_end = fb.height.saturating_sub(renderer.status_bar_height);

                                        // 1.5 Click on empty area of menu bar -> start interactive window move
                                        if self.config.gui.menu_bar && y < menu_offset {
                                            window.start_interactive_move();
                                        } else if sidebar_w > 0 && x < sidebar_w && y >= menu_offset && y < content_y_end {
                                        let rel_x = x;
                                        let rel_y = y - menu_offset;
                                        let row_h = (renderer.char_height() + renderer.scale_coord(4)).max(renderer.scale_coord(18));
                                        let header_h = renderer.tab_bar_height;
                                        match self.sidebar.handle_click(rel_x, rel_y, row_h, header_h) {
                                            crate::editor::sidebar::SidebarClickResult::OpenFile(path) => {
                                                self.open_file(&path);
                                                self.sidebar.is_focused = false;
                                                adjust_viewport = true;
                                            }
                                            crate::editor::sidebar::SidebarClickResult::NewFile => {
                                                self.new_tab();
                                                self.sidebar.is_focused = false;
                                                adjust_viewport = true;
                                            }
                                            crate::editor::sidebar::SidebarClickResult::Refresh => {
                                                self.set_status("Árbol de archivos actualizado");
                                            }
                                            crate::editor::sidebar::SidebarClickResult::CloseSidebar => {
                                                self.sidebar.is_visible = false;
                                                self.set_status("Barra lateral cerrada (Ctrl+B para reabrir)");
                                            }
                                            crate::editor::sidebar::SidebarClickResult::ToggleFolder => {
                                                // Directory expanded/collapsed
                                            }
                                            crate::editor::sidebar::SidebarClickResult::None => {}
                                        }
                                        needs_redraw = true;
                                    } else if self.split.is_split() {
                                        let (p0_geom, p1_geom, splitter_opt) = renderer.pane_geometries(fb.width, fb.height, sidebar_w, self.config.gui.menu_bar, &self.split);

                                        let mut hit_splitter = false;
                                        if let Some((sx, sy, sw, sh)) = splitter_opt {
                                            if sw > sh {
                                                hit_splitter = y >= sy.saturating_sub(3) && y < sy + sh + 3 && x >= sx && x < sx + sw;
                                            } else {
                                                hit_splitter = x >= sx.saturating_sub(3) && x < sx + sw + 3 && y >= sy && y < sy + sh;
                                            }
                                        }

                                        if hit_splitter {
                                            is_dragging_splitter = true;
                                            needs_redraw = true;
                                        } else if p0_geom.contains_point(x, y) {
                                            if self.split.active_pane != 0 {
                                                self.split.active_pane = 0;
                                                self.active_doc_idx = self.split.primary_doc_idx.min(self.documents.len().saturating_sub(1));
                                            }
                                            let (sb_geom, mm_geom_opt, max_code_x) = renderer.pane_subgeometries(&p0_geom, self.doc());
                                            if sb_geom.contains_point(x, y) {
                                                if sb_geom.thumb_contains_point(x, y) {
                                                    is_dragging_scrollbar = true;
                                                    scrollbar_drag_offset_y = y.saturating_sub(sb_geom.thumb_y);
                                                } else if y < sb_geom.thumb_y {
                                                    let visible = sb_geom.visible_lines.max(1);
                                                    self.doc_mut().scroll_row = self.doc().scroll_row.saturating_sub(visible);
                                                } else {
                                                    let visible = sb_geom.visible_lines.max(1);
                                                    self.doc_mut().scroll_row = (self.doc().scroll_row + visible).min(sb_geom.max_scroll);
                                                }
                                                adjust_viewport = false;
                                                needs_redraw = true;
                                            } else if let Some(ref mm_geom) = mm_geom_opt.filter(|mg| mg.contains_point(x, y)) {
                                                if mm_geom.slider_contains_point(x, y) {
                                                    is_dragging_minimap = true;
                                                } else {
                                                    let target_line = mm_geom.line_for_click_y(y);
                                                    self.doc_mut().scroll_row = target_line.saturating_sub(mm_geom.visible_lines / 2);
                                                }
                                                adjust_viewport = false;
                                                needs_redraw = true;
                                             } else if x < max_code_x && y >= p0_geom.y + renderer.tab_bar_height {
                                                let content_y_start = p0_geom.y + renderer.tab_bar_height;
                                                let total_lines = self.doc().buffer.len_lines();
                                                let gutter_cols = total_lines.to_string().len().max(3);
                                                let gutter_width = if renderer.show_line_numbers {
                                                    (gutter_cols + 2) * renderer.char_width()
                                                } else {
                                                    renderer.scale_coord(8)
                                                };
                                                let line_height = renderer.line_height();
                                                let row = (y - content_y_start) / line_height;
                                                let text_start_x = p0_geom.x + gutter_width + renderer.char_width();
                                                let code_width = max_code_x.saturating_sub(text_start_x);
                                                let code_cols = (code_width / renderer.char_width()).max(10);
                                                if let Some(char_offset) = Self::find_click_position(self.doc(), row, x, text_start_x, renderer.char_width(), self.word_wrap, code_cols) {
                                                    self.doc_mut().cursors.set_single(char_offset);
                                                    adjust_viewport = true;
                                                    needs_redraw = true;
                                                }
                                            }
                                        } else if p1_geom.contains_point(x, y) {
                                            if self.split.active_pane != 1 {
                                                self.split.active_pane = 1;
                                                self.active_doc_idx = self.split.secondary_doc_idx.min(self.documents.len().saturating_sub(1));
                                            }
                                            let (sb_geom, mm_geom_opt, max_code_x) = renderer.pane_subgeometries(&p1_geom, self.doc());
                                            if sb_geom.contains_point(x, y) {
                                                if sb_geom.thumb_contains_point(x, y) {
                                                    is_dragging_scrollbar = true;
                                                    scrollbar_drag_offset_y = y.saturating_sub(sb_geom.thumb_y);
                                                } else if y < sb_geom.thumb_y {
                                                    let visible = sb_geom.visible_lines.max(1);
                                                    self.doc_mut().scroll_row = self.doc().scroll_row.saturating_sub(visible);
                                                } else {
                                                    let visible = sb_geom.visible_lines.max(1);
                                                    self.doc_mut().scroll_row = (self.doc().scroll_row + visible).min(sb_geom.max_scroll);
                                                }
                                                adjust_viewport = false;
                                                needs_redraw = true;
                                            } else if let Some(ref mm_geom) = mm_geom_opt.filter(|mg| mg.contains_point(x, y)) {
                                                if mm_geom.slider_contains_point(x, y) {
                                                    is_dragging_minimap = true;
                                                } else {
                                                    let target_line = mm_geom.line_for_click_y(y);
                                                    self.doc_mut().scroll_row = target_line.saturating_sub(mm_geom.visible_lines / 2);
                                                }
                                                adjust_viewport = false;
                                                needs_redraw = true;
                                            } else if x < max_code_x && y >= p1_geom.y + renderer.tab_bar_height {
                                                let content_y_start = p1_geom.y + renderer.tab_bar_height;
                                                let total_lines = self.doc().buffer.len_lines();
                                                let gutter_cols = total_lines.to_string().len().max(3);
                                                let gutter_width = if renderer.show_line_numbers {
                                                    (gutter_cols + 2) * renderer.char_width()
                                                } else {
                                                    renderer.scale_coord(8)
                                                };
                                                let line_height = renderer.line_height();
                                                let row = (y - content_y_start) / line_height;
                                                let text_start_x = p1_geom.x + gutter_width + renderer.char_width();
                                                let code_width = max_code_x.saturating_sub(text_start_x);
                                                let code_cols = (code_width / renderer.char_width()).max(10);
                                                if let Some(char_offset) = Self::find_click_position(self.doc(), row, x, text_start_x, renderer.char_width(), self.word_wrap, code_cols) {
                                                    self.doc_mut().cursors.set_single(char_offset);
                                                    adjust_viewport = true;
                                                    needs_redraw = true;
                                                }
                                            }
                                        }
                                    } else {
                                        // 3. Single pane mode: Check Scrollbar Hit Test FIRST
                                        let scroll_geom = renderer.scrollbar_geometry(fb.width, fb.height, self.doc(), true);
                                        let mm_geom = renderer.minimap_geometry(fb.width, fb.height, self.doc(), true);
                                        if scroll_geom.contains_point(x, y) {
                                            if scroll_geom.thumb_contains_point(x, y) {
                                                is_dragging_scrollbar = true;
                                                scrollbar_drag_offset_y = y.saturating_sub(scroll_geom.thumb_y);
                                            } else if y < scroll_geom.thumb_y {
                                                let visible = scroll_geom.visible_lines.max(1);
                                                self.doc_mut().scroll_row = self.doc().scroll_row.saturating_sub(visible);
                                            } else {
                                                let visible = scroll_geom.visible_lines.max(1);
                                                self.doc_mut().scroll_row = (self.doc().scroll_row + visible).min(scroll_geom.max_scroll);
                                            }
                                            adjust_viewport = false;
                                            needs_redraw = true;
                                        } else if renderer.minimap_enabled && mm_geom.contains_point(x, y) {
                                            if mm_geom.slider_contains_point(x, y) {
                                                is_dragging_minimap = true;
                                            } else {
                                                let target_line = mm_geom.line_for_click_y(y);
                                                self.doc_mut().scroll_row = target_line.saturating_sub(mm_geom.visible_lines / 2);
                                            }
                                            adjust_viewport = false;
                                            needs_redraw = true;
                                        } else {
                                            // 4. Check Tab Bar
                                            let tab_bar_y = menu_offset;
                                            let tab_bar_h = renderer.tab_bar_height;
                                            if y >= tab_bar_y && y < tab_bar_y + tab_bar_h {
                                                let mut cur_x = sidebar_w + renderer.scale_coord(8);
                                                let mut clicked = false;
                                                for idx in 0..self.documents.len() {
                                                    let title = self.documents[idx].tab_title();
                                                    let tab_text = format!(" {} {} × ", idx + 1, title);
                                                    let tab_w = tab_text.chars().count() * renderer.char_width() + renderer.scale_coord(8);
                                                    if x >= cur_x && x < cur_x + tab_w {
                                                        let close_x = cur_x + tab_w - 2 * renderer.char_width() - renderer.scale_coord(4);
                                                        if x >= close_x {
                                                            self.close_tab_at(idx);
                                                        } else {
                                                            self.active_doc_idx = idx;
                                                            self.sync_split_doc_indices();
                                                            self.sidebar.is_focused = false;
                                                            adjust_viewport = true;
                                                        }
                                                        clicked = true;
                                                        break;
                                                    }
                                                    cur_x += tab_w + renderer.scale_coord(3);
                                                }
                                                if !clicked && x >= cur_x && x < cur_x + renderer.scale_coord(24) {
                                                    self.new_tab();
                                                    self.sidebar.is_focused = false;
                                                    adjust_viewport = true;
                                                } else if !clicked {
                                                    window.start_interactive_move();
                                                }
                                            } else {
                                                // 5. Check Editor Code Canvas
                                                let content_y_start = menu_offset + renderer.tab_bar_height;
                                                let total_lines = self.doc().buffer.len_lines();
                                                let gutter_cols = total_lines.to_string().len().max(3);
                                                let gutter_width = if renderer.show_line_numbers {
                                                    (gutter_cols + 2) * renderer.char_width()
                                                } else {
                                                    renderer.scale_coord(8)
                                                };
                                                let canvas_w = fb.width.saturating_sub(sidebar_w);
                                                let mm_w = if renderer.minimap_enabled && canvas_w >= renderer.scale_coord(320) { renderer.minimap_width.min(canvas_w / 4) } else { 0 };
                                                let max_code_x = fb.width.saturating_sub(renderer.scrollbar_width + mm_w);
                                                if y >= content_y_start && x >= sidebar_w && x < max_code_x {
                                                    self.sidebar.is_focused = false;
                                                    let line_height = renderer.line_height();
                                                    let row = (y - content_y_start) / line_height;
                                                    let text_start_x = sidebar_w + gutter_width + renderer.char_width();
                                                    let code_width = max_code_x.saturating_sub(text_start_x);
                                                    let code_cols = (code_width / renderer.char_width()).max(10);
                                                    if let Some(char_offset) = Self::find_click_position(self.doc(), row, x, text_start_x, renderer.char_width(), self.word_wrap, code_cols) {
                                                        self.doc_mut().cursors.set_single(char_offset);
                                                        adjust_viewport = true;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        }
                        }
                    }
                    crate::window::WindowEvent::MouseUp { .. } => {
                        if is_dragging_term_splitter {
                            is_dragging_term_splitter = false;
                            needs_redraw = true;
                        }
                        if is_dragging_scrollbar {
                            is_dragging_scrollbar = false;
                            needs_redraw = true;
                        }
                        if is_dragging_minimap {
                            is_dragging_minimap = false;
                            needs_redraw = true;
                        }
                        if is_dragging_splitter {
                            is_dragging_splitter = false;
                            needs_redraw = true;
                        }
                    }
                    crate::window::WindowEvent::MouseMove { x, y } => {
                        let old_my = mouse_y;
                        mouse_x = x;
                        mouse_y = y;
                        if self.mode == EditorMode::Normal {
                            let sidebar_w = if self.sidebar.is_visible { self.sidebar.width.min(fb.width.saturating_sub(100)) } else { 0 };

                            if self.autocomplete.is_active && !self.autocomplete.items.is_empty() {
                                let ac_geom = renderer.autocomplete_geometry(fb.width, fb.height, self.doc(), sidebar_w, self.config.gui.menu_bar, self.autocomplete.items.len());
                                if ac_geom.contains_point(x, y) {
                                    if let Some(item_idx) = ac_geom.item_index_at(y, self.autocomplete.scroll_offset) {
                                        if self.autocomplete.selected_idx != item_idx {
                                            self.autocomplete.selected_idx = item_idx;
                                            needs_redraw = true;
                                        }
                                    }
                                }
                            }

                            if is_dragging_term_splitter {
                                let bottom_anchor = fb.height.saturating_sub(renderer.status_bar_height);
                                let new_height = bottom_anchor.saturating_sub(y);
                                let max_allowed = fb.height.saturating_sub(renderer.menu_bar_height + renderer.tab_bar_height + renderer.status_bar_height + 40);
                                self.terminal.height = new_height.clamp(60, max_allowed);
                                needs_redraw = true;
                            } else if is_dragging_splitter && self.split.is_split() {
                                let (p0_geom, p1_geom, _) = renderer.pane_geometries(fb.width, fb.height, sidebar_w, self.config.gui.menu_bar, &self.split);
                                match self.split.direction {
                                    crate::editor::split::SplitDirection::Vertical => {
                                        let total_w = p0_geom.width + p1_geom.width;
                                        if total_w > 0 {
                                            let rel_x = x.saturating_sub(p0_geom.x);
                                            let new_ratio = (rel_x as f32 / total_w as f32).clamp(0.15, 0.85);
                                            self.split.ratio = new_ratio;
                                            needs_redraw = true;
                                        }
                                    }
                                    crate::editor::split::SplitDirection::Horizontal => {
                                        let total_h = p0_geom.height + p1_geom.height;
                                        if total_h > 0 {
                                            let rel_y = y.saturating_sub(p0_geom.y);
                                            let new_ratio = (rel_y as f32 / total_h as f32).clamp(0.15, 0.85);
                                            self.split.ratio = new_ratio;
                                            needs_redraw = true;
                                        }
                                    }
                                    crate::editor::split::SplitDirection::None => {}
                                }
                            } else if is_dragging_scrollbar {
                                if self.split.is_split() {
                                    let (p0_geom, p1_geom, _) = renderer.pane_geometries(fb.width, fb.height, sidebar_w, self.config.gui.menu_bar, &self.split);
                                    let active_pane_geom = if self.split.active_pane == 0 { &p0_geom } else { &p1_geom };
                                    let (scroll_geom, _, _) = renderer.pane_subgeometries(active_pane_geom, self.doc());
                                    let target_thumb_y = y.saturating_sub(scrollbar_drag_offset_y);
                                    let new_row = scroll_geom.row_for_thumb_y(target_thumb_y);
                                    if self.doc().scroll_row != new_row {
                                        self.doc_mut().scroll_row = new_row;
                                        adjust_viewport = false;
                                        needs_redraw = true;
                                    }
                                } else {
                                    let scroll_geom = renderer.scrollbar_geometry(fb.width, fb.height, self.doc(), true);
                                    let target_thumb_y = y.saturating_sub(scrollbar_drag_offset_y);
                                    let new_row = scroll_geom.row_for_thumb_y(target_thumb_y);
                                    if self.doc().scroll_row != new_row {
                                        self.doc_mut().scroll_row = new_row;
                                        adjust_viewport = false;
                                        needs_redraw = true;
                                    }
                                }
                            } else if is_dragging_minimap {
                                if self.split.is_split() {
                                    let (p0_geom, p1_geom, _) = renderer.pane_geometries(fb.width, fb.height, sidebar_w, self.config.gui.menu_bar, &self.split);
                                    let active_pane_geom = if self.split.active_pane == 0 { &p0_geom } else { &p1_geom };
                                    let (_, mm_geom_opt, _) = renderer.pane_subgeometries(active_pane_geom, self.doc());
                                    if let Some(mm_geom) = mm_geom_opt {
                                        let new_row = mm_geom.row_for_slider_y(y.saturating_sub(mm_geom.slider_height / 2));
                                        if self.doc().scroll_row != new_row {
                                            self.doc_mut().scroll_row = new_row;
                                            adjust_viewport = false;
                                            needs_redraw = true;
                                        }
                                    }
                                } else {
                                    let mm_geom = renderer.minimap_geometry(fb.width, fb.height, self.doc(), true);
                                    let new_row = mm_geom.row_for_slider_y(y.saturating_sub(mm_geom.slider_height / 2));
                                    if self.doc().scroll_row != new_row {
                                        self.doc_mut().scroll_row = new_row;
                                        adjust_viewport = false;
                                        needs_redraw = true;
                                    }
                                }
                            } else {
                                if !self.split.is_split() {
                                    let scroll_geom = renderer.scrollbar_geometry(fb.width, fb.height, self.doc(), true);
                                    let sb_hover = scroll_geom.contains_point(x, y);
                                    if sb_hover != scrollbar_hovered {
                                        scrollbar_hovered = sb_hover;
                                        needs_redraw = true;
                                    }
                                    if renderer.minimap_enabled {
                                        let mm_geom = renderer.minimap_geometry(fb.width, fb.height, self.doc(), true);
                                        let mm_hover = mm_geom.contains_point(x, y);
                                        if mm_hover != minimap_hovered {
                                            minimap_hovered = mm_hover;
                                            needs_redraw = true;
                                        }
                                    }
                                } else {
                                    let (p0_geom, p1_geom, _) = renderer.pane_geometries(fb.width, fb.height, sidebar_w, self.config.gui.menu_bar, &self.split);
                                    let active_pane_geom = if self.split.active_pane == 0 { &p0_geom } else { &p1_geom };
                                    let (sb_geom, mm_geom_opt, _) = renderer.pane_subgeometries(active_pane_geom, self.doc());
                                    let sb_hover = sb_geom.contains_point(x, y);
                                    if sb_hover != scrollbar_hovered {
                                        scrollbar_hovered = sb_hover;
                                        needs_redraw = true;
                                    }
                                    if let Some(mm_geom) = mm_geom_opt {
                                        let mm_hover = mm_geom.contains_point(x, y);
                                        if mm_hover != minimap_hovered {
                                            minimap_hovered = mm_hover;
                                            needs_redraw = true;
                                        }
                                    }
                                }
                            }

                            let in_menu_zone = self.menu_bar.is_open() || y < renderer.menu_bar_height || old_my < renderer.menu_bar_height;
                            if in_menu_zone {
                                self.menu_bar.handle_mouse_move(x, y, renderer.char_width());
                                needs_redraw = true;
                            }
                            if self.terminal.is_visible {
                                let tg = renderer.terminal_geometry(fb.width, fb.height, sidebar_w, self.terminal.height);
                                if tg.contains_point(x, y) || tg.contains_point(mouse_x, old_my) {
                                    needs_redraw = true;
                                }
                            }
                        } else {
                            if let EditorMode::FontSelector(dialog) = &mut self.mode {
                                let win_w = renderer.scale_coord(760).min(fb.width.saturating_sub(renderer.scale_coord(20)));
                                let win_h = renderer.scale_coord(520).min(fb.height.saturating_sub(renderer.scale_coord(20)));
                                let win_x = (fb.width.saturating_sub(win_w)) / 2;
                                let win_y = (fb.height.saturating_sub(win_h)) / 2;
                                let title_h = renderer.scale_coord(28);
                                let body_y = win_y + title_h + 1;
                                let col_w = (win_w.saturating_sub(renderer.scale_coord(36))) / 2;
                                let left_x = win_x + renderer.scale_coord(14);
                                let content_y = body_y + renderer.scale_coord(8);
                                let fam_start_y = content_y + renderer.scale_coord(16);
                                let fam_card_h = renderer.scale_coord(34);

                                dialog.hovered_family = None;
                                for (i, _) in crate::gfx::FontFamily::ALL.iter().enumerate() {
                                    let card_y = fam_start_y + i * (fam_card_h + renderer.scale_coord(3));
                                    if x >= left_x && x < left_x + col_w && y >= card_y && y < card_y + fam_card_h {
                                        dialog.hovered_family = Some(i);
                                        break;
                                    }
                                }

                                let footer_h = renderer.scale_coord(44);
                                let footer_y = win_y + win_h - footer_h;
                                let btn_cancel_w = renderer.scale_coord(95);
                                let btn_apply_w = renderer.scale_coord(145);
                                let btn_h = renderer.scale_coord(26);
                                let btn_y = footer_y + (footer_h.saturating_sub(btn_h)) / 2;
                                let btn_apply_x = win_x + win_w - btn_apply_w - renderer.scale_coord(14);
                                let btn_cancel_x = btn_apply_x - btn_cancel_w - renderer.scale_coord(10);

                                dialog.hovered_button = if y >= btn_y && y < btn_y + btn_h {
                                    if x >= btn_apply_x && x < btn_apply_x + btn_apply_w {
                                        Some(0)
                                    } else if x >= btn_cancel_x && x < btn_cancel_x + btn_cancel_w {
                                        Some(1)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };
                            }
                            needs_redraw = true;
                        }
                    }
                    crate::window::WindowEvent::ScrollUp { .. } => {
                        needs_redraw = true;
                        if self.autocomplete.is_active && !self.autocomplete.items.is_empty() {
                            self.autocomplete.select_prev();
                        } else {
                            let sidebar_w = if self.sidebar.is_visible {
                                self.sidebar.width.min(fb.width.saturating_sub(100))
                            } else {
                                0
                            };
                            let menu_offset = if self.config.gui.menu_bar { renderer.menu_bar_height } else { 0 };
                            let content_y_end = fb.height.saturating_sub(renderer.status_bar_height);

                            if self.terminal.is_visible && renderer.terminal_geometry(fb.width, fb.height, sidebar_w, self.terminal.height).contains_point(mouse_x, mouse_y) {
                                self.terminal.scroll_offset = self.terminal.scroll_offset.saturating_sub(3).max(1);
                                needs_redraw = true;
                            } else if let EditorMode::ModalPrompt(modal) = &mut self.mode {
                                if modal.is_file_dialog() {
                                    modal.scroll_up(3);
                                } else if modal.kind == PromptKind::GlobalSearch {
                                    for _ in 0..3 {
                                        modal.select_prev_global_result();
                                    }
                                }
                            } else if let EditorMode::FontSelector(dialog) = &mut self.mode {
                                dialog.select_next_size();
                                needs_redraw = true;
                            } else if let EditorMode::ExtensionsManager { scroll_offset, .. } = &mut self.mode {
                                *scroll_offset = scroll_offset.saturating_sub(1);
                            } else if sidebar_w > 0 && mouse_x < sidebar_w && mouse_y >= menu_offset && mouse_y < content_y_end {
                                let max_visible = content_y_end.saturating_sub(menu_offset + renderer.tab_bar_height) / 20;
                                self.sidebar.handle_scroll(1, max_visible);
                            } else if self.split.is_split() {
                                let (_p0_geom, p1_geom, _) = renderer.pane_geometries(fb.width, fb.height, sidebar_w, self.config.gui.menu_bar, &self.split);
                                if p1_geom.contains_point(mouse_x, mouse_y) {
                                    let idx = self.split.secondary_doc_idx;
                                    if idx < self.documents.len() {
                                        self.documents[idx].scroll_row = self.documents[idx].scroll_row.saturating_sub(3);
                                    }
                                } else {
                                    let idx = self.split.primary_doc_idx;
                                    if idx < self.documents.len() {
                                        self.documents[idx].scroll_row = self.documents[idx].scroll_row.saturating_sub(3);
                                    }
                                }
                                adjust_viewport = false;
                            } else {
                                if !self.word_wrap {
                                    self.doc_mut().scroll_row = self.doc().scroll_row.saturating_sub(3);
                                } else {
                                    let max_cols = self.last_code_cols.max(10);
                                    let doc = self.doc_mut();
                                    for _ in 0..3 {
                                        if doc.scroll_subline > 0 {
                                            doc.scroll_subline -= 1;
                                        } else if doc.scroll_row > 0 {
                                            doc.scroll_row -= 1;
                                            let line_cow = doc.buffer.line_cow(doc.scroll_row).unwrap_or(std::borrow::Cow::Borrowed(""));
                                            let sublines = crate::editor::wrap::WordWrap::compute_sublines(&line_cow, max_cols);
                                            doc.scroll_subline = sublines.len().saturating_sub(1);
                                        } else {
                                            break;
                                        }
                                    }
                                }
                                adjust_viewport = false;
                            }
                        }
                    }
                    crate::window::WindowEvent::ScrollDown { .. } => {
                        needs_redraw = true;
                        if self.autocomplete.is_active && !self.autocomplete.items.is_empty() {
                            self.autocomplete.select_next();
                        } else {
                            let sidebar_w = if self.sidebar.is_visible {
                                self.sidebar.width.min(fb.width.saturating_sub(100))
                            } else {
                                0
                            };
                            let menu_offset = if self.config.gui.menu_bar { renderer.menu_bar_height } else { 0 };
                            let content_y_end = fb.height.saturating_sub(renderer.status_bar_height);

                            if self.terminal.is_visible && renderer.terminal_geometry(fb.width, fb.height, sidebar_w, self.terminal.height).contains_point(mouse_x, mouse_y) {
                                self.terminal.scroll_offset = (self.terminal.scroll_offset + 3).min(self.terminal.lines.len());
                                needs_redraw = true;
                            } else if let EditorMode::ModalPrompt(modal) = &mut self.mode {
                                if modal.is_file_dialog() {
                                    let box_h = fb.height.min(480).max(360);
                                    let list_h = box_h.saturating_sub(190);
                                    let max_visible = list_h / 22;
                                    modal.scroll_down(3, max_visible);
                                } else if modal.kind == PromptKind::GlobalSearch {
                                    for _ in 0..3 {
                                        modal.select_next_global_result(12);
                                    }
                                }
                            } else if let EditorMode::FontSelector(dialog) = &mut self.mode {
                                dialog.select_prev_size();
                                needs_redraw = true;
                            } else if let EditorMode::ExtensionsManager { scroll_offset, .. } = &mut self.mode {
                                let ext_count = self.extension_manager.extensions.len();
                                if *scroll_offset + 1 < ext_count {
                                    *scroll_offset += 1;
                                }
                            } else if sidebar_w > 0 && mouse_x < sidebar_w && mouse_y >= menu_offset && mouse_y < content_y_end {
                                let max_visible = content_y_end.saturating_sub(menu_offset + renderer.tab_bar_height) / 20;
                                self.sidebar.handle_scroll(-1, max_visible);
                            } else if self.split.is_split() {
                                let (_p0_geom, p1_geom, _) = renderer.pane_geometries(fb.width, fb.height, sidebar_w, self.config.gui.menu_bar, &self.split);
                                if p1_geom.contains_point(mouse_x, mouse_y) {
                                    let idx = self.split.secondary_doc_idx;
                                    if idx < self.documents.len() {
                                        let max_lines = self.documents[idx].buffer.len_lines();
                                        self.documents[idx].scroll_row = (self.documents[idx].scroll_row + 3).min(max_lines.saturating_sub(1));
                                    }
                                } else {
                                    let idx = self.split.primary_doc_idx;
                                    if idx < self.documents.len() {
                                        let max_lines = self.documents[idx].buffer.len_lines();
                                        self.documents[idx].scroll_row = (self.documents[idx].scroll_row + 3).min(max_lines.saturating_sub(1));
                                    }
                                }
                                adjust_viewport = false;
                            } else {
                                if !self.word_wrap {
                                    self.doc_mut().scroll_row = (self.doc().scroll_row + 3).min(self.doc().buffer.len_lines().saturating_sub(1));
                                } else {
                                    let max_cols = self.last_code_cols.max(10);
                                    let total_lines = self.doc().buffer.len_lines();
                                    let doc = self.doc_mut();
                                    for _ in 0..3 {
                                        let line_cow = doc.buffer.line_cow(doc.scroll_row).unwrap_or(std::borrow::Cow::Borrowed(""));
                                        let sublines = crate::editor::wrap::WordWrap::compute_sublines(&line_cow, max_cols);
                                        if doc.scroll_subline + 1 < sublines.len() {
                                            doc.scroll_subline += 1;
                                        } else if doc.scroll_row + 1 < total_lines {
                                            doc.scroll_row += 1;
                                            doc.scroll_subline = 0;
                                        } else {
                                            break;
                                        }
                                    }
                                }
                                adjust_viewport = false;
                            }
                        }
                    }
                    crate::window::WindowEvent::Char(c) => {
                        needs_redraw = true;
                        adjust_viewport = true;
                        self.sidebar.is_focused = false;
                        if self.menu_bar.is_open() {
                            self.menu_bar.close();
                        }
                        self.handle_key(Key::Char(c));
                    }
                    crate::window::WindowEvent::Key(key) => {
                        needs_redraw = true;
                        adjust_viewport = true;
                        match key {
                            Key::Esc if self.menu_bar.is_open() => {
                                self.menu_bar.close();
                            }
                            Key::Alt(c) => {
                                match c.to_ascii_lowercase() {
                                    'a' => self.menu_bar.active_category = Some(0),
                                    'e' => self.menu_bar.active_category = Some(1),
                                    's' => self.menu_bar.active_category = Some(2),
                                    'v' => self.menu_bar.active_category = Some(3),
                                    'y' => self.menu_bar.active_category = Some(4),
                                    _ => self.handle_key(key),
                                }
                            }
                            _ => {
                                if self.menu_bar.is_open() {
                                    self.menu_bar.close();
                                }
                                if self.sidebar.is_focused && self.sidebar.is_visible && self.mode == EditorMode::Normal {
                                    let menu_offset = if self.config.gui.menu_bar { renderer.menu_bar_height } else { 0 };
                                    let content_y_end = fb.height.saturating_sub(renderer.status_bar_height);
                                    let max_visible = content_y_end.saturating_sub(menu_offset + renderer.tab_bar_height) / 20;
                                    match key {
                                        Key::Up => self.sidebar.move_up(),
                                        Key::Down => self.sidebar.move_down(max_visible),
                                        Key::Right => { self.sidebar.expand_selected(); }
                                        Key::Left => { self.sidebar.collapse_selected(); }
                                        Key::Enter => {
                                            if let Some(path) = self.sidebar.toggle_selected() {
                                                self.open_file(&path);
                                                self.sidebar.is_focused = false;
                                            }
                                        }
                                        Key::Esc => self.sidebar.is_focused = false,
                                        Key::Ctrl('b') => {
                                            self.execute_action(EditorAction::ToggleSidebar);
                                        }
                                        _ => self.handle_key(key),
                                    }
                                } else {
                                    self.handle_key(key);
                                }
                            }
                        }
                    }
                }
            }

            if self.should_quit {
                break;
            }

            if !needs_redraw {
                std::thread::sleep(std::time::Duration::from_millis(15));
                continue;
            }

            // Adjust viewport for active document in GUI
            let active_idx = self.active_doc_idx;
            let primary_pos = {
                let doc = &self.documents[active_idx];
                doc.buffer.char_to_position(doc.cursors.primary().head)
            };

            let total_lines = self.documents[active_idx].buffer.len_lines();
            let gutter_cols = total_lines.to_string().len().max(3);
            let gutter_width = if renderer.show_line_numbers { (gutter_cols + 2) * renderer.char_width() } else { renderer.scale_coord(8) };
            let sidebar_w = if self.sidebar.is_visible { self.sidebar.width.min(fb.width.saturating_sub(renderer.scale_coord(100))) } else { 0 };

            let term_inset = if self.terminal.is_visible {
                renderer.terminal_geometry(fb.width, fb.height, sidebar_w, self.terminal.height).height
            } else {
                0
            };

            let (visible_lines, visible_cols) = if self.split.is_split() {
                let (p0_geom, p1_geom, _) = renderer.pane_geometries_with_inset(fb.width, fb.height, sidebar_w, self.config.gui.menu_bar, &self.split, term_inset);
                let active_geom = if self.split.active_pane == 0 { p0_geom } else { p1_geom };
                let pane_content_h = active_geom.height.saturating_sub(renderer.tab_bar_height);
                let line_height = renderer.line_height();
                let vl = pane_content_h / line_height;
                let mini_w = if renderer.minimap_enabled && active_geom.width >= renderer.scale_coord(320) { renderer.minimap_width.min(active_geom.width / 4) } else { 0 };
                let pane_content_w = active_geom.width.saturating_sub(gutter_width + renderer.scrollbar_width + mini_w);
                let vc = pane_content_w / renderer.char_width();
                (vl, vc)
            } else {
                let content_y_start = renderer.menu_bar_height + renderer.tab_bar_height;
                let content_h = fb.height.saturating_sub(content_y_start + renderer.status_bar_height + term_inset);
                let line_height = renderer.line_height();
                let vl = content_h / line_height;
                let canvas_w = fb.width.saturating_sub(sidebar_w);
                let mini_w = if renderer.minimap_enabled && canvas_w >= renderer.scale_coord(320) { renderer.minimap_width.min(canvas_w / 4) } else { 0 };
                let content_w = fb.width.saturating_sub(sidebar_w + gutter_width + renderer.scrollbar_width + mini_w);
                let vc = content_w / renderer.char_width();
                (vl, vc)
            };

            self.last_code_cols = visible_cols.max(10);

            if adjust_viewport {
                let doc = &mut self.documents[active_idx];
                if self.word_wrap {
                    doc.scroll_col = 0;
                    crate::editor::wrap::WordWrap::adjust_viewport(doc, visible_lines, self.last_code_cols);
                } else {
                    if visible_lines > 0 {
                        if primary_pos.line < doc.scroll_row {
                            doc.scroll_row = primary_pos.line;
                        } else if primary_pos.line >= doc.scroll_row + visible_lines {
                            doc.scroll_row = primary_pos.line - visible_lines + 1;
                        }
                    }
                    if visible_cols > 0 {
                        if primary_pos.col < doc.scroll_col {
                            doc.scroll_col = primary_pos.col;
                        } else if primary_pos.col >= doc.scroll_col + visible_cols {
                            doc.scroll_col = primary_pos.col - visible_cols + 1;
                        }
                    }
                }
                adjust_viewport = false;
            }

            // Update OS native window title with active file and dirty status
            let file_name = self.doc().buffer.file_path().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("[Sin título]");
            let dirty = if self.doc().buffer.is_dirty() { " [+]" } else { "" };
            let current_title = format!("Rustext - {}{}", file_name, dirty);
            if current_title != cached_title {
                window.set_title(&current_title);
                cached_title = current_title;
            }

            // Render current frame into framebuffer with scrollbar hover/drag state
            let tab_titles: Vec<String> = self.documents.iter().map(|d| d.tab_title()).collect();
            renderer.diagnostics = Some(self.diagnostics.clone());
            let diag_line_msg = if self.status_message.is_none() {
                let primary_head = self.documents[active_idx].cursors.primary().head;
                let primary_pos = self.documents[active_idx].buffer.char_to_position(primary_head);
                self.diagnostics.for_line(primary_pos.line + 1).map(|d| d.to_display_string())
            } else {
                None
            };
            let status_msg = self.status_message.as_ref().map(|(msg, _)| msg.as_str()).or(diag_line_msg.as_deref());

            let menu_bar = if self.config.gui.menu_bar {
                Some(&self.menu_bar)
            } else {
                None
            };
            renderer.scrollbar_width = if self.config.gui.scrollbar { 14 } else { 0 };

            let sidebar = if self.sidebar.is_visible {
                Some(&self.sidebar)
            } else {
                None
            };

            let primary_doc = if self.split.is_split() {
                &self.documents[self.split.primary_doc_idx.min(self.documents.len().saturating_sub(1))]
            } else {
                &self.documents[active_idx]
            };

            let secondary_doc = if self.split.is_split() {
                Some(&self.documents[self.split.secondary_doc_idx.min(self.documents.len().saturating_sub(1))])
            } else {
                None
            };

            let primary_git_diff = if self.config.gui.git_gutter {
                let path = primary_doc.buffer.file_path().map(|p| p.to_path_buf());
                let ver = primary_doc.buffer.version();
                if primary_doc.buffer.len_bytes() > 2 * 1024 * 1024 {
                    Some(crate::editor::git::GitGutterDiff::default())
                } else if let Some((ref cp, cv, ref diff)) = cached_primary_git {
                    if cp == &path && cv == ver {
                        Some(diff.clone())
                    } else {
                        let text = primary_doc.buffer.to_string();
                        let diff = self.git.diff_document(path.as_deref(), &text);
                        cached_primary_git = Some((path, ver, diff.clone()));
                        Some(diff)
                    }
                } else {
                    let text = primary_doc.buffer.to_string();
                    let diff = self.git.diff_document(path.as_deref(), &text);
                    cached_primary_git = Some((path, ver, diff.clone()));
                    Some(diff)
                }
            } else {
                None
            };

            let secondary_git_diff = if self.config.gui.git_gutter {
                if let Some(sec_doc) = secondary_doc {
                    let path = sec_doc.buffer.file_path().map(|p| p.to_path_buf());
                    let ver = sec_doc.buffer.version();
                    if sec_doc.buffer.len_bytes() > 2 * 1024 * 1024 {
                        Some(crate::editor::git::GitGutterDiff::default())
                    } else if let Some((ref cp, cv, ref diff)) = cached_secondary_git {
                        if cp == &path && cv == ver {
                            Some(diff.clone())
                        } else {
                            let text = sec_doc.buffer.to_string();
                            let diff = self.git.diff_document(path.as_deref(), &text);
                            cached_secondary_git = Some((path, ver, diff.clone()));
                            Some(diff)
                        }
                    } else {
                        let text = sec_doc.buffer.to_string();
                        let diff = self.git.diff_document(path.as_deref(), &text);
                        cached_secondary_git = Some((path, ver, diff.clone()));
                        Some(diff)
                    }
                } else {
                    None
                }
            } else {
                None
            };

            renderer.git_diff = primary_git_diff;
            renderer.secondary_git_diff = secondary_git_diff;
            renderer.git_branch = self.git.branch.clone();
            let git_statuses_len = self.git.file_statuses.len();
            if cached_git_statuses.as_ref().map(|s| s.len()) != Some(git_statuses_len) {
                cached_git_statuses = Some(self.git.file_statuses.clone());
            }
            renderer.git_statuses = cached_git_statuses.clone();
            renderer.git_gutter_enabled = self.config.gui.git_gutter;
            renderer.terminal_inset = term_inset;

            renderer.render_full_frame_with_split(
                &mut fb,
                primary_doc,
                secondary_doc,
                if self.split.is_split() { Some(&self.split) } else { None },
                &tab_titles,
                active_idx,
                status_msg,
                None,
                menu_bar,
                sidebar,
                scrollbar_hovered,
                is_dragging_scrollbar,
                minimap_hovered,
                is_dragging_minimap,
                Some(&self.autocomplete),
            );

            // Render integrated terminal panel if visible
            if self.terminal.is_visible {
                let term_geom = renderer.terminal_geometry(fb.width, fb.height, sidebar_w, self.terminal.height);
                let hover_clear = term_geom.is_over_clear_btn(mouse_x, mouse_y);
                let hover_close = term_geom.is_over_close_btn(mouse_x, mouse_y);
                renderer.render_terminal_panel(&mut fb, &self.terminal, &term_geom, hover_clear, hover_close);
            }

            // Render Prompt Modal if active (with interactive hover state)
            if let EditorMode::ModalPrompt(modal) = &self.mode {
                let box_w = if modal.kind == PromptKind::GlobalSearch {
                    fb.width.min(renderer.scale_coord(740)).max(renderer.scale_coord(460))
                } else if modal.is_file_dialog() || modal.kind == PromptKind::RecentFiles {
                    fb.width.min(renderer.scale_coord(680)).max(renderer.scale_coord(400))
                } else if modal.kind == PromptKind::Replace {
                    fb.width.min(renderer.scale_coord(540)).max(renderer.scale_coord(380))
                } else {
                    fb.width.min(renderer.scale_coord(580)).max(renderer.scale_coord(340))
                };
                let box_h = if modal.kind == PromptKind::GlobalSearch {
                    fb.height.min(renderer.scale_coord(500)).max(renderer.scale_coord(360))
                } else if modal.is_file_dialog() {
                    fb.height.min(renderer.scale_coord(480)).max(renderer.scale_coord(360))
                } else if modal.kind == PromptKind::CommandPalette || modal.kind == PromptKind::RecentFiles {
                    renderer.scale_coord(340)
                } else if modal.kind == PromptKind::Replace {
                    renderer.scale_coord(168)
                } else {
                    renderer.scale_coord(120)
                };
                let box_x = (fb.width.saturating_sub(box_w)) / 2;
                let box_y = if modal.is_file_dialog() {
                    (fb.height.saturating_sub(box_h)) / 2
                } else {
                    renderer.scale_coord(28)
                };
                let close_w = if modal.is_file_dialog() { renderer.scale_coord(32) } else { renderer.scale_coord(30) };
                let close_h = if modal.is_file_dialog() { renderer.scale_coord(26) } else { renderer.scale_coord(24) };
                let close_x = box_x + box_w - close_w;
                let close_hovered = mouse_x >= close_x && mouse_x < close_x + close_w && mouse_y >= box_y && mouse_y < box_y + close_h;
                renderer.render_prompt_overlay_with_hover(&mut fb, modal, close_hovered);
            }

            // Render About Dialog Window if active (with interactive hover state)
            if self.mode == EditorMode::AboutDialog {
                let win_w = renderer.scale_coord(640).min(fb.width.saturating_sub(renderer.scale_coord(20)));
                let win_h = renderer.scale_coord(450).min(fb.height.saturating_sub(renderer.scale_coord(20)));
                let win_x = (fb.width.saturating_sub(win_w)) / 2;
                let win_y = (fb.height.saturating_sub(win_h)) / 2;
                let close_w = renderer.scale_coord(36);
                let close_x = win_x + win_w - close_w;
                let btn_w = renderer.scale_coord(130);
                let btn_h = renderer.scale_coord(28);
                let btn_x = win_x + (win_w - btn_w) / 2;
                let btn_y = win_y + win_h - renderer.scale_coord(38);

                let close_hovered = mouse_x >= close_x && mouse_x < close_x + close_w && mouse_y >= win_y && mouse_y < win_y + renderer.scale_coord(28);
                let ok_hovered = mouse_x >= btn_x && mouse_x < btn_x + btn_w && mouse_y >= btn_y && mouse_y < btn_y + btn_h;

                renderer.render_about_dialog(&mut fb, close_hovered, ok_hovered);
            }

            // Render Extensions Manager Dialog Window if active
            if let EditorMode::ExtensionsManager { selected_idx, scroll_offset } = self.mode {
                let win_w = renderer.scale_coord(680).min(fb.width.saturating_sub(renderer.scale_coord(20)));
                let win_x = (fb.width.saturating_sub(win_w)) / 2;
                let win_y = (fb.height.saturating_sub(renderer.scale_coord(480).min(fb.height.saturating_sub(renderer.scale_coord(20))))) / 2;
                let close_w = renderer.scale_coord(36);
                let close_x = win_x + win_w - close_w;
                let close_hovered = mouse_x >= close_x && mouse_x < close_x + close_w && mouse_y >= win_y && mouse_y < win_y + renderer.scale_coord(28);

                renderer.render_extensions_dialog(&mut fb, &self.extension_manager, selected_idx, scroll_offset, close_hovered);
            }

            // Render Unsaved Changes Confirmation Dialog if active
            if let EditorMode::UnsavedConfirm(dialog) = &self.mode {
                let win_w = renderer.scale_coord(520).min(fb.width.saturating_sub(renderer.scale_coord(20)));
                let win_h = renderer.scale_coord(190).min(fb.height.saturating_sub(renderer.scale_coord(20)));
                let win_x = (fb.width.saturating_sub(win_w)) / 2;
                let win_y = (fb.height.saturating_sub(win_h)) / 2;
                let title_h = renderer.scale_coord(28);
                let close_w = renderer.scale_coord(36);
                let close_x = win_x + win_w - close_w;

                let btn_bar_y = win_y + win_h - renderer.scale_coord(42);
                let btn_h = renderer.scale_coord(28);
                let (lbl_save, lbl_discard, lbl_cancel) = dialog.button_labels();
                let btn_save_w = (lbl_save.chars().count() * renderer.char_width() + renderer.scale_coord(24)).max(renderer.scale_coord(100));
                let btn_discard_w = (lbl_discard.chars().count() * renderer.char_width() + renderer.scale_coord(24)).max(renderer.scale_coord(110));
                let btn_cancel_w = (lbl_cancel.chars().count() * renderer.char_width() + renderer.scale_coord(24)).max(renderer.scale_coord(90));
                let total_btns_w = btn_save_w + btn_discard_w + btn_cancel_w + renderer.scale_coord(16);
                let btn_start_x = win_x + win_w - total_btns_w - renderer.scale_coord(16);

                let b0_x = btn_start_x;
                let b1_x = b0_x + btn_save_w + renderer.scale_coord(8);
                let b2_x = b1_x + btn_discard_w + renderer.scale_coord(8);

                let hovered_btn = if mouse_y >= btn_bar_y && mouse_y < btn_bar_y + btn_h {
                    if mouse_x >= b0_x && mouse_x < b0_x + btn_save_w {
                        Some(0)
                    } else if mouse_x >= b1_x && mouse_x < b1_x + btn_discard_w {
                        Some(1)
                    } else if mouse_x >= b2_x && mouse_x < b2_x + btn_cancel_w {
                        Some(2)
                    } else {
                        None
                    }
                } else if mouse_x >= close_x && mouse_x < close_x + close_w && mouse_y >= win_y && mouse_y < win_y + title_h {
                    Some(3)
                } else {
                    None
                };

                renderer.render_unsaved_confirm_dialog(&mut fb, dialog, hovered_btn);
            }

            // Render Font Selector Dialog if active
            if let EditorMode::FontSelector(dialog) = &self.mode {
                let win_w = renderer.scale_coord(760).min(fb.width.saturating_sub(renderer.scale_coord(20)));
                let win_h = renderer.scale_coord(520).min(fb.height.saturating_sub(renderer.scale_coord(20)));
                let win_x = (fb.width.saturating_sub(win_w)) / 2;
                let win_y = (fb.height.saturating_sub(win_h)) / 2;
                let title_h = renderer.scale_coord(28);
                let close_w = renderer.scale_coord(36);
                let close_x = win_x + win_w - close_w;
                let close_hovered = mouse_x >= close_x && mouse_x < close_x + close_w && mouse_y >= win_y && mouse_y < win_y + title_h;

                renderer.render_font_selector_dialog(&mut fb, dialog, close_hovered);
            }

            // Present framebuffer to OS window
            window.present(&fb);
            needs_redraw = false;

            // Small sleep to yield timeslice
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let _ = self.save_session();
        Ok(())
    }
}
