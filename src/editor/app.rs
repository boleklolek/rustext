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
pub enum EditorMode {
    Normal,
    ModalPrompt(PromptModal),
    AboutDialog,
    ExtensionsManager { selected_idx: usize, scroll_offset: usize },
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

        let mut screen = Screen::new();
        screen.theme = crate::ui::Theme::by_name(&config.theme.name);

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

        Ok(())
    }

    /// Dispatches input key events depending on editor mode.
    pub fn handle_key(&mut self, key: Key) {
        match &mut self.mode {
            EditorMode::AboutDialog => match key {
                Key::Esc | Key::Enter | Key::Char(' ') => {
                    self.mode = EditorMode::Normal;
                }
                _ => {}
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
                    } else if modal.is_file_dialog() {
                        modal.select_next_entry(10);
                    }
                }
                Key::Up => {
                    if modal.kind == PromptKind::Replace {
                        modal.active_field = 0;
                    } else if modal.kind == PromptKind::CommandPalette {
                        modal.selected_idx = modal.selected_idx.saturating_sub(1);
                    } else if modal.is_file_dialog() {
                        modal.select_prev_entry();
                    }
                }
                Key::PageDown => {
                    if modal.is_file_dialog() {
                        for _ in 0..8 {
                            modal.select_next_entry(10);
                        }
                    }
                }
                Key::PageUp => {
                    if modal.is_file_dialog() {
                        for _ in 0..8 {
                            modal.select_prev_entry();
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
                let buf = self.doc().buffer.clone();
                self.doc_mut().cursors.move_left(&buf, select);
            }
            EditorAction::MoveRight { select } => {
                let buf = self.doc().buffer.clone();
                self.doc_mut().cursors.move_right(&buf, select);
            }
            EditorAction::MoveUp { select } => {
                let buf = self.doc().buffer.clone();
                self.doc_mut().cursors.move_up(&buf, select);
            }
            EditorAction::MoveDown { select } => {
                let buf = self.doc().buffer.clone();
                self.doc_mut().cursors.move_down(&buf, select);
            }
            EditorAction::MoveLineStart { select } => {
                let buf = self.doc().buffer.clone();
                self.doc_mut().cursors.move_to_line_start(&buf, select);
            }
            EditorAction::MoveLineEnd { select } => {
                let buf = self.doc().buffer.clone();
                self.doc_mut().cursors.move_to_line_end(&buf, select);
            }

            // Multi-cursors
            EditorAction::AddCursorAbove => {
                let buf = self.doc().buffer.clone();
                self.doc_mut().cursors.add_cursor_above(&buf);
                self.set_status(format!("Active cursors: {}", self.doc().cursors.cursors().len()));
            }
            EditorAction::AddCursorBelow => {
                let buf = self.doc().buffer.clone();
                self.doc_mut().cursors.add_cursor_below(&buf);
                self.set_status(format!("Active cursors: {}", self.doc().cursors.cursors().len()));
            }
            EditorAction::SelectNextOccurrence => {
                let buf = self.doc().buffer.clone();
                let mut cs = self.doc().cursors.clone();
                if SearchEngine::select_next_occurrence(&buf, &mut cs) {
                    self.doc_mut().cursors = cs;
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

            EditorAction::CollapseCursors => {
                self.snippet_session = None;
                let primary = self.doc().cursors.primary();
                self.doc_mut().cursors.set_single(primary.head);
                self.set_status("Collapsed to single cursor");
            }

            EditorAction::Quit => {
                let any_dirty = self.documents.iter().any(|d| d.buffer.is_dirty());
                if any_dirty {
                    self.set_status("Unsaved changes! Press Ctrl+Q again to discard or Ctrl+S to save");
                    self.should_quit = true;
                } else {
                    self.should_quit = true;
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

            EditorAction::None => {}
        }
    }

    /// Reloads configuration from disk and applies updated parameters immediately.
    pub fn reload_config(&mut self) {
        let new_cfg = crate::editor::config::EditorConfig::load();
        self.tab_size = new_cfg.editor.tab_size;
        self.auto_indent = new_cfg.editor.auto_indent;
        self.auto_close_brackets = new_cfg.editor.auto_close_brackets;
        self.auto_surround = new_cfg.editor.auto_surround;
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
        self.set_status(format!("Tab {}/{}", self.active_doc_idx + 1, self.documents.len()));
    }

    /// Closes the current active tab.
    pub fn close_tab(&mut self) {
        if self.documents.len() > 1 {
            self.documents.remove(self.active_doc_idx);
            if self.active_doc_idx >= self.documents.len() {
                self.active_doc_idx = self.documents.len() - 1;
            }
            self.set_status(format!("Tab {}/{}", self.active_doc_idx + 1, self.documents.len()));
        } else {
            self.documents[0] = Document::new(TextBuffer::new());
            self.set_status("Buffer cleared");
        }
    }

    /// Closes the tab at a specific index.
    pub fn close_tab_at(&mut self, idx: usize) {
        if idx >= self.documents.len() {
            return;
        }
        if self.documents.len() > 1 {
            self.documents.remove(idx);
            if self.active_doc_idx >= self.documents.len() {
                self.active_doc_idx = self.documents.len() - 1;
            }
            self.set_status(format!("Tab {}/{}", self.active_doc_idx + 1, self.documents.len()));
        } else {
            self.documents[0] = Document::new(TextBuffer::new());
            self.set_status("Buffer cleared");
        }
    }

    /// Switches to the next tab.
    pub fn next_tab(&mut self) {
        if self.documents.len() > 1 {
            self.active_doc_idx = (self.active_doc_idx + 1) % self.documents.len();
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
            self.set_status(format!("Tab {}/{}", self.active_doc_idx + 1, self.documents.len()));
        }
    }

    /// Opens a file into a tab, or switches to it if already open.
    pub fn open_file(&mut self, path: &Path) {
        if let Some(pos) = self.documents.iter().position(|d| d.buffer.file_path() == Some(path)) {
            self.active_doc_idx = pos;
            self.set_status(format!("Switched to tab {}", pos + 1));
            return;
        }

        match TextBuffer::open(path) {
            Ok(new_buf) => {
                let new_doc = Document::new(new_buf);
                self.documents.push(new_doc);
                self.active_doc_idx = self.documents.len() - 1;
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
        }
    }

    /// Finds the next occurrence of `query` in the current document.
    pub fn find_next_match(&mut self, query: &str) {
        let input = query.trim();
        if input.is_empty() {
            return;
        }
        let current_head = self.doc().cursors.primary().head;
        let buf = self.doc().buffer.clone();
        let search_from = if self.doc().cursors.primary().has_selection() {
            self.doc().cursors.primary().selection_range().end
        } else {
            current_head
        };
        if let Some((start, end)) = SearchEngine::find_next(&buf, input, search_from, false) {
            self.doc_mut().cursors.set_single(start);
            self.doc_mut().cursors.cursors_mut()[0] = Cursor::with_selection(start, end);
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
        let current_head = self.doc().cursors.primary().head;
        let buf = self.doc().buffer.clone();
        let search_from = if self.doc().cursors.primary().has_selection() {
            self.doc().cursors.primary().selection_range().start
        } else {
            current_head
        };
        if let Some((start, end)) = SearchEngine::find_prev(&buf, input, search_from, false) {
            self.doc_mut().cursors.set_single(start);
            self.doc_mut().cursors.cursors_mut()[0] = Cursor::with_selection(start, end);
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

    /// Saves the current buffer to disk.
    pub fn save_document(&mut self) {
        if self.doc().buffer.file_path().is_none() {
            let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            self.mode = EditorMode::ModalPrompt(PromptModal::new_with_dir(PromptKind::SaveAs, dir, None));
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

        let mut content = self.doc().buffer.to_string();
        self.plugin_registry.trigger_before_save(&mut ctx, &mut content);

        if let Some(transformed) = self.extension_manager.trigger_on_save(&content) {
            content = transformed;
        }

        // Auto-format on save if enabled in configuration
        if self.config.editor.format_on_save {
            let lang = self.doc().language;
            let file_path = self.doc().buffer.file_path().map(|p| p.to_path_buf());
            if let Ok(formatted) = crate::editor::formatter::format_code(lang, &content, file_path.as_deref()) {
                content = formatted;
            }
        }

        let doc = self.doc_mut();
        if content != doc.buffer.to_string() {
            let path = doc.buffer.file_path().map(|p| p.to_path_buf());
            doc.buffer = TextBuffer::from_str(&content);
            if let Some(p) = path {
                doc.buffer.set_file_path(p);
            }
        }

        match doc.buffer.save() {
            Ok(()) => {
                self.plugin_registry.trigger_after_save(&mut ctx);
                let saved_path = self.doc().buffer.file_path().map(|p| p.to_path_buf());
                let path_str = saved_path
                    .as_ref()
                    .and_then(|p| p.to_str())
                    .unwrap_or("file");
                self.set_status(format!("Saved {}", path_str));

                // Auto-run diagnostics check if enabled in config
                if self.config.editor.check_on_save {
                    self.check_active_document();
                }

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
            EditorMode::AboutDialog | EditorMode::ExtensionsManager { .. } => {
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

        let mut fb = crate::gfx::Framebuffer::new(initial_width, initial_height);
        let mut renderer = crate::gfx::GfxRenderer::new();
        renderer.colors = crate::gfx::ThemeColors::by_name(&self.config.theme.name);
        renderer.show_line_numbers = self.config.editor.show_line_numbers;
        let mut active_theme_name = self.config.theme.name.clone();
        let mut active_show_line_numbers = self.config.editor.show_line_numbers;

        let mut mouse_x = 0;
        let mut mouse_y = 0;
        let mut is_dragging_scrollbar = false;
        let mut scrollbar_drag_offset_y = 0;
        let mut scrollbar_hovered = false;
        let mut adjust_viewport = true;
        let mut needs_redraw = true;
        let mut cached_title = String::new();

        while !self.should_quit && !window.should_close() {
            // Check dynamic config changes
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
            // Check status message expiry (clear after 6 seconds)
            if let Some((_, created_at)) = self.status_message {
                if created_at.elapsed() > std::time::Duration::from_secs(6) {
                    self.status_message = None;
                    needs_redraw = true;
                }
            }

            // Drain all pending window events
            loop {
                let event = window.poll_event();
                match event {
                    crate::window::WindowEvent::None => break,
                    crate::window::WindowEvent::Close => {
                        self.should_quit = true;
                        break;
                    }
                    crate::window::WindowEvent::Resize { width, height } => {
                        fb.resize(width, height);
                        needs_redraw = true;
                    }
                    crate::window::WindowEvent::MouseDown { x, y, button } => {
                        mouse_x = x;
                        mouse_y = y;
                        needs_redraw = true;
                        if button == crate::window::MouseButton::Left {
                            // A. Check About Dialog Window interaction
                            if self.mode == EditorMode::AboutDialog {
                                let win_w = 640.min(fb.width.saturating_sub(20));
                                let win_h = 450.min(fb.height.saturating_sub(20));
                                let win_x = (fb.width.saturating_sub(win_w)) / 2;
                                let win_y = (fb.height.saturating_sub(win_h)) / 2;
                                let close_w = 36;
                                let close_x = win_x + win_w - close_w;
                                let btn_w = 130;
                                let btn_h = 28;
                                let btn_x = win_x + (win_w - btn_w) / 2;
                                let btn_y = win_y + win_h - 38;

                                let hit_close = x >= close_x && x < close_x + close_w && y >= win_y && y < win_y + 28;
                                let hit_ok = x >= btn_x && x < btn_x + btn_w && y >= btn_y && y < btn_y + btn_h;
                                let hit_outside = x < win_x || x >= win_x + win_w || y < win_y || y >= win_y + win_h;

                                if hit_close || hit_ok || hit_outside {
                                    self.mode = EditorMode::Normal;
                                }
                            // B. Check Extensions Manager Dialog interaction
                            } else if let EditorMode::ExtensionsManager { selected_idx, scroll_offset } = &mut self.mode {
                                let win_w = 680.min(fb.width.saturating_sub(20));
                                let win_h = 480.min(fb.height.saturating_sub(20));
                                let win_x = (fb.width.saturating_sub(win_w)) / 2;
                                let win_y = (fb.height.saturating_sub(win_h)) / 2;
                                let close_w = 36;
                                let close_x = win_x + win_w - close_w;
                                let hit_top_close = x >= close_x && x < close_x + close_w && y >= win_y && y < win_y + 28;

                                let footer_y = win_y + win_h - 34;
                                let close_btn_w = 80;
                                let close_btn_x = win_x + win_w - close_btn_w - 12;
                                let hit_bottom_close = x >= close_btn_x && x < close_btn_x + close_btn_w && y >= footer_y && y < footer_y + 26;

                                let hit_outside = x < win_x || x >= win_x + win_w || y < win_y || y >= win_y + win_h;

                                if hit_top_close || hit_bottom_close || hit_outside {
                                    self.mode = EditorMode::Normal;
                                } else {
                                    let body_y = win_y + 28 + 1;
                                    let bar_y = body_y + 8;
                                    let reload_btn_w = 90;
                                    let create_btn_w = 120;
                                    let reload_btn_x = win_x + win_w - reload_btn_w - 12;
                                    let create_btn_x = reload_btn_x - create_btn_w - 8;

                                    if x >= create_btn_x && x < create_btn_x + create_btn_w && y >= bar_y && y < bar_y + 24 {
                                        self.execute_action(EditorAction::CreateSampleExtension);
                                    } else if x >= reload_btn_x && x < reload_btn_x + reload_btn_w && y >= bar_y && y < bar_y + 24 {
                                        self.execute_action(EditorAction::ReloadExtensions);
                                    } else {
                                        let list_x = win_x + 12;
                                        let list_y = bar_y + 30;
                                        let list_w = win_w.saturating_sub(24);
                                        let list_h = win_h.saturating_sub(115);
                                        let card_h = 58;

                                        if x >= list_x && x < list_x + list_w && y >= list_y && y < list_y + list_h {
                                            let card_idx = (y - list_y) / card_h;
                                            let actual_idx = *scroll_offset + card_idx;
                                            if actual_idx < self.extension_manager.extensions.len() {
                                                *selected_idx = actual_idx;
                                                let card_y = list_y + 2 + card_idx * card_h;
                                                let toggle_w = 64;
                                                let toggle_x = list_x + list_w - toggle_w - 8;
                                                if x >= toggle_x && x < toggle_x + toggle_w && y >= card_y + 4 && y < card_y + 24 {
                                                    let name = self.extension_manager.extensions[actual_idx].name.clone();
                                                    let state = self.extension_manager.toggle_extension(actual_idx);
                                                    self.set_status(format!("Extensión '{}': {}", name, if state { "Activada" } else { "Desactivada" }));
                                                }
                                            }
                                        }
                                    }
                                }
                            // C. Check Modal Prompt Window interaction
                            } else if let EditorMode::ModalPrompt(modal) = &mut self.mode {
                                if modal.is_file_dialog() {
                                    let box_w = fb.width.min(680).max(420);
                                    let box_h = fb.height.min(480).max(360);
                                    let box_x = (fb.width.saturating_sub(box_w)) / 2;
                                    let box_y = (fb.height.saturating_sub(box_h)) / 2;
                                    let close_w = 32;
                                    let header_h = 26;
                                    let close_x = box_x + box_w - close_w;

                                    let hit_close = x >= close_x && x < close_x + close_w && y >= box_y && y < box_y + header_h;
                                    let hit_outside = x < box_x || x >= box_x + box_w || y < box_y || y >= box_y + box_h;

                                    let btn_bar_y = box_y + box_h - 30;
                                    let action_btn_w = 88;
                                    let cancel_btn_w = 80;
                                    let cancel_btn_x = box_x + box_w - cancel_btn_w - 12;
                                    let action_btn_x = cancel_btn_x - action_btn_w - 8;

                                    let hit_cancel = y >= btn_bar_y && y < btn_bar_y + 24 && x >= cancel_btn_x && x < cancel_btn_x + cancel_btn_w;
                                    let hit_action = y >= btn_bar_y && y < btn_bar_y + 24 && x >= action_btn_x && x < action_btn_x + action_btn_w;

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
                                        let dir_bar_y = box_y + header_h + 1 + 6;
                                        let up_btn_w = 64;
                                        let up_btn_x = box_x + box_w - up_btn_w - 12;
                                        if x >= up_btn_x && x < up_btn_x + up_btn_w && y >= dir_bar_y + 2 && y < dir_bar_y + 22 {
                                            modal.navigate_up();
                                        } else {
                                            // Bookmarks
                                            let bm_bar_y = dir_bar_y + 24 + 4;
                                            let bookmarks = crate::ui::prompt::get_quick_bookmarks();
                                            let mut cur_bm_x = box_x + 8;
                                            let mut clicked_bm = false;
                                            for bm in &bookmarks {
                                                let bm_text = format!("[{}]", bm.label);
                                                let bm_w = bm_text.chars().count() * crate::gfx::FONT_WIDTH + 8;
                                                if cur_bm_x + bm_w < box_x + box_w - 8 {
                                                    if y >= bm_bar_y && y < bm_bar_y + 20 && x >= cur_bm_x && x < cur_bm_x + bm_w {
                                                        modal.navigate_to(bm.path.clone());
                                                        clicked_bm = true;
                                                        break;
                                                    }
                                                    cur_bm_x += bm_w + 6;
                                                }
                                            }

                                            if !clicked_bm {
                                                // Directory entries list
                                                let list_y = bm_bar_y + 26;
                                                let list_h = box_h.saturating_sub(190);
                                                let list_w = box_w.saturating_sub(16);
                                                let list_x = box_x + 8;
                                                let row_h = 22;

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
                                    let box_w = fb.width.min(540).max(380);
                                    let box_h = 168;
                                    let box_x = (fb.width.saturating_sub(box_w)) / 2;
                                    let box_y = 28;
                                    let close_w = 30;
                                    let close_x = box_x + box_w - close_w;

                                    let hit_close = x >= close_x && x < close_x + close_w && y >= box_y && y < box_y + 24;
                                    let hit_outside = x < box_x || x >= box_x + box_w || y < box_y || y >= box_y + box_h;

                                    if hit_close || hit_outside {
                                        self.mode = EditorMode::Normal;
                                    } else {
                                        let body_y = box_y + 24 + 1;
                                        let input_margin = 12;
                                        let label_w = 100;
                                        let input_w = box_w.saturating_sub(input_margin * 2 + label_w);
                                        let input_h = 24;
                                        let row1_y = body_y + 12;
                                        let row2_y = row1_y + input_h + 10;
                                        let input_x = box_x + input_margin + label_w;

                                        // Click row 1 input (Buscar)
                                        if x >= input_x && x < input_x + input_w && y >= row1_y && y < row1_y + input_h {
                                            modal.active_field = 0;
                                        // Click row 2 input (Reemplazar)
                                        } else if x >= input_x && x < input_x + input_w && y >= row2_y && y < row2_y + input_h {
                                            modal.active_field = 1;
                                        } else {
                                            let btn_y = row2_y + input_h + 16;
                                            let btn_h = 24;
                                            let b1_x = box_x + input_margin;
                                            let b1_w = 68;
                                            let b2_x = b1_x + b1_w + 8;
                                            let b2_w = 68;
                                            let b3_x = b2_x + b2_w + 14;
                                            let b3_w = 98;
                                            let b4_x = b3_x + b3_w + 8;
                                            let b4_w = 126;

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
                                } else {
                                    let is_palette = modal.kind == PromptKind::CommandPalette;
                                    let box_w = fb.width.min(580).max(340);
                                    let box_h = if is_palette { 340 } else { 120 };
                                    let box_x = (fb.width.saturating_sub(box_w)) / 2;
                                    let box_y = 28;
                                    let close_w = 30;
                                    let close_x = box_x + box_w - close_w;

                                    let hit_close = x >= close_x && x < close_x + close_w && y >= box_y && y < box_y + 24;
                                    let hit_outside = x < box_x || x >= box_x + box_w || y < box_y || y >= box_y + box_h;

                                    if hit_close || hit_outside {
                                        self.mode = EditorMode::Normal;
                                    } else if is_palette {
                                        let body_y = box_y + 24 + 1;
                                        let input_y = body_y + 8;
                                        let item_start_y = input_y + 24 + 8;
                                        let item_h = 24;
                                        let matching = modal.matching_commands();
                                        if x >= box_x + 12 && x < box_x + box_w - 12 && y >= item_start_y {
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
                                // 1. Try Menu Bar and open dropdown
                                if let Some(action) = self.menu_bar.handle_mouse_click(x, y, crate::gfx::FONT_WIDTH) {
                                    self.execute_action(action);
                                } else if !self.menu_bar.is_open() {
                                    // 2. Check Scrollbar Hit Test FIRST
                                    let scroll_geom = renderer.scrollbar_geometry(fb.width, fb.height, self.doc(), true);
                                    if scroll_geom.contains_point(x, y) {
                                        if scroll_geom.thumb_contains_point(x, y) {
                                            is_dragging_scrollbar = true;
                                            scrollbar_drag_offset_y = y.saturating_sub(scroll_geom.thumb_y);
                                        } else if y < scroll_geom.thumb_y {
                                            // Click track above thumb -> Page Up
                                            let visible = scroll_geom.visible_lines.max(1);
                                            self.doc_mut().scroll_row = self.doc().scroll_row.saturating_sub(visible);
                                        } else {
                                            // Click track below thumb -> Page Down
                                            let visible = scroll_geom.visible_lines.max(1);
                                            self.doc_mut().scroll_row = (self.doc().scroll_row + visible).min(scroll_geom.max_scroll);
                                        }
                                        adjust_viewport = false;
                                        needs_redraw = true;
                                    } else {
                                        // 3. Check Tab Bar
                                        let tab_bar_y = renderer.menu_bar_height;
                                        let tab_bar_h = renderer.tab_bar_height;
                                        if y >= tab_bar_y && y < tab_bar_y + tab_bar_h {
                                            let mut cur_x = 8;
                                            let mut clicked = false;
                                            for idx in 0..self.documents.len() {
                                                let title = self.documents[idx].tab_title();
                                                let tab_text = format!(" {} {} × ", idx + 1, title);
                                                let tab_w = tab_text.chars().count() * crate::gfx::FONT_WIDTH + 8;
                                                if x >= cur_x && x < cur_x + tab_w {
                                                    let close_x = cur_x + tab_w - 2 * crate::gfx::FONT_WIDTH - 4;
                                                    if x >= close_x {
                                                        self.close_tab_at(idx);
                                                    } else {
                                                        self.active_doc_idx = idx;
                                                        adjust_viewport = true;
                                                    }
                                                    clicked = true;
                                                    break;
                                                }
                                                cur_x += tab_w + 2;
                                            }
                                            if !clicked && x >= cur_x && x < cur_x + 24 {
                                                self.new_tab();
                                                adjust_viewport = true;
                                            }
                                        } else {
                                            // 4. Check Editor Code Canvas
                                            let content_y_start = renderer.menu_bar_height + renderer.tab_bar_height;
                                            let total_lines = self.doc().buffer.len_lines();
                                            let gutter_cols = total_lines.to_string().len().max(3);
                                            let gutter_width = (gutter_cols + 2) * crate::gfx::FONT_WIDTH;
                                            let max_code_x = fb.width.saturating_sub(renderer.scrollbar_width);
                                            if y >= content_y_start && x < max_code_x {
                                                let row = (y - content_y_start) / crate::gfx::FONT_HEIGHT;
                                                let target_line = self.doc().scroll_row + row;
                                                if target_line < total_lines {
                                                    let text_start_x = gutter_width + crate::gfx::FONT_WIDTH;
                                                    let col = if x >= text_start_x {
                                                        (x - text_start_x) / crate::gfx::FONT_WIDTH + self.doc().scroll_col
                                                    } else {
                                                        0
                                                    };
                                                    let pos = Position::new(target_line, col);
                                                    let char_offset = self.doc().buffer.position_to_char(pos);
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
                    crate::window::WindowEvent::MouseUp { .. } => {
                        if is_dragging_scrollbar {
                            is_dragging_scrollbar = false;
                            needs_redraw = true;
                        }
                    }
                    crate::window::WindowEvent::MouseMove { x, y } => {
                        let old_my = mouse_y;
                        mouse_x = x;
                        mouse_y = y;
                        if self.mode == EditorMode::Normal {
                            let scroll_geom = renderer.scrollbar_geometry(fb.width, fb.height, self.doc(), true);
                            if is_dragging_scrollbar {
                                let target_thumb_y = y.saturating_sub(scrollbar_drag_offset_y);
                                let new_row = scroll_geom.row_for_thumb_y(target_thumb_y);
                                if self.doc().scroll_row != new_row {
                                    self.doc_mut().scroll_row = new_row;
                                    adjust_viewport = false;
                                    needs_redraw = true;
                                }
                            } else {
                                let hovered = scroll_geom.contains_point(x, y);
                                if hovered != scrollbar_hovered {
                                    scrollbar_hovered = hovered;
                                    needs_redraw = true;
                                }
                            }

                            let in_menu_zone = self.menu_bar.is_open() || y < renderer.menu_bar_height || old_my < renderer.menu_bar_height;
                            if in_menu_zone {
                                self.menu_bar.handle_mouse_move(x, y, crate::gfx::FONT_WIDTH);
                                needs_redraw = true;
                            }
                        } else {
                            needs_redraw = true;
                        }
                    }
                    crate::window::WindowEvent::ScrollUp { .. } => {
                        needs_redraw = true;
                        if let EditorMode::ModalPrompt(modal) = &mut self.mode {
                            if modal.is_file_dialog() {
                                modal.scroll_up(3);
                            }
                        } else if let EditorMode::ExtensionsManager { scroll_offset, .. } = &mut self.mode {
                            *scroll_offset = scroll_offset.saturating_sub(1);
                        } else {
                            self.doc_mut().scroll_row = self.doc().scroll_row.saturating_sub(3);
                            adjust_viewport = false;
                        }
                    }
                    crate::window::WindowEvent::ScrollDown { .. } => {
                        needs_redraw = true;
                        if let EditorMode::ModalPrompt(modal) = &mut self.mode {
                            if modal.is_file_dialog() {
                                let box_h = fb.height.min(480).max(360);
                                let list_h = box_h.saturating_sub(190);
                                let max_visible = list_h / 22;
                                modal.scroll_down(3, max_visible);
                            }
                        } else if let EditorMode::ExtensionsManager { scroll_offset, .. } = &mut self.mode {
                            let ext_count = self.extension_manager.extensions.len();
                            if *scroll_offset + 1 < ext_count {
                                *scroll_offset += 1;
                            }
                        } else {
                            self.doc_mut().scroll_row = (self.doc().scroll_row + 3).min(self.doc().buffer.len_lines().saturating_sub(1));
                            adjust_viewport = false;
                        }
                    }
                    crate::window::WindowEvent::Char(c) => {
                        needs_redraw = true;
                        adjust_viewport = true;
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
                                    _ => {}
                                }
                            }
                            _ => {
                                if self.menu_bar.is_open() {
                                    self.menu_bar.close();
                                }
                                self.handle_key(key);
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
            let gutter_width = (gutter_cols + 2) * crate::gfx::FONT_WIDTH;
            let content_y_start = renderer.menu_bar_height + renderer.tab_bar_height;
            let content_h = fb.height.saturating_sub(content_y_start + renderer.status_bar_height);
            let visible_lines = content_h / crate::gfx::FONT_HEIGHT;
            let content_w = fb.width.saturating_sub(gutter_width + crate::gfx::FONT_WIDTH);
            let visible_cols = content_w / crate::gfx::FONT_WIDTH;

            if adjust_viewport {
                let doc = &mut self.documents[active_idx];
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

            renderer.render_with_scrollbar_state(
                &mut fb,
                &self.documents[active_idx],
                &tab_titles,
                active_idx,
                status_msg,
                None,
                menu_bar,
                scrollbar_hovered,
                is_dragging_scrollbar,
            );

            // Render Prompt Modal if active (with interactive hover state)
            if let EditorMode::ModalPrompt(modal) = &self.mode {
                let box_w = if modal.is_file_dialog() {
                    fb.width.min(680).max(420)
                } else if modal.kind == PromptKind::Replace {
                    fb.width.min(540).max(380)
                } else {
                    fb.width.min(580).max(340)
                };
                let box_h = if modal.is_file_dialog() {
                    fb.height.min(480).max(360)
                } else if modal.kind == PromptKind::CommandPalette {
                    340
                } else if modal.kind == PromptKind::Replace {
                    168
                } else {
                    120
                };
                let box_x = (fb.width.saturating_sub(box_w)) / 2;
                let box_y = if modal.is_file_dialog() {
                    (fb.height.saturating_sub(box_h)) / 2
                } else {
                    28
                };
                let close_w = if modal.is_file_dialog() { 32 } else { 30 };
                let close_h = if modal.is_file_dialog() { 26 } else { 24 };
                let close_x = box_x + box_w - close_w;
                let close_hovered = mouse_x >= close_x && mouse_x < close_x + close_w && mouse_y >= box_y && mouse_y < box_y + close_h;
                renderer.render_prompt_overlay_with_hover(&mut fb, modal, close_hovered);
            }

            // Render About Dialog Window if active (with interactive hover state)
            if self.mode == EditorMode::AboutDialog {
                let win_w = 640.min(fb.width.saturating_sub(20));
                let win_h = 450.min(fb.height.saturating_sub(20));
                let win_x = (fb.width.saturating_sub(win_w)) / 2;
                let win_y = (fb.height.saturating_sub(win_h)) / 2;
                let close_w = 36;
                let close_x = win_x + win_w - close_w;
                let btn_w = 130;
                let btn_h = 28;
                let btn_x = win_x + (win_w - btn_w) / 2;
                let btn_y = win_y + win_h - 38;

                let close_hovered = mouse_x >= close_x && mouse_x < close_x + close_w && mouse_y >= win_y && mouse_y < win_y + 28;
                let ok_hovered = mouse_x >= btn_x && mouse_x < btn_x + btn_w && mouse_y >= btn_y && mouse_y < btn_y + btn_h;

                renderer.render_about_dialog(&mut fb, close_hovered, ok_hovered);
            }

            // Render Extensions Manager Dialog Window if active
            if let EditorMode::ExtensionsManager { selected_idx, scroll_offset } = self.mode {
                let win_w = 680.min(fb.width.saturating_sub(20));
                let win_x = (fb.width.saturating_sub(win_w)) / 2;
                let win_y = (fb.height.saturating_sub(480.min(fb.height.saturating_sub(20)))) / 2;
                let close_w = 36;
                let close_x = win_x + win_w - close_w;
                let close_hovered = mouse_x >= close_x && mouse_x < close_x + close_w && mouse_y >= win_y && mouse_y < win_y + 28;

                renderer.render_extensions_dialog(&mut fb, &self.extension_manager, selected_idx, scroll_offset, close_hovered);
            }

            // Present framebuffer to OS window
            window.present(&fb);
            needs_redraw = false;

            // Small sleep to yield timeslice
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        Ok(())
    }
}
