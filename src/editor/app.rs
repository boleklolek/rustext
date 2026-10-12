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
    PluginContext, PluginRegistry, TrailingWhitespacePlugin, WordCountPlugin,
};
use crate::ui::prompt::{PromptKind, PromptModal};
use crate::ui::screen::Screen;
use crate::ui::terminal::{Key, TerminalGuard};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    ModalPrompt(PromptModal),
}

pub struct Editor {
    pub documents: Vec<Document>,
    pub active_doc_idx: usize,
    pub clipboard: Clipboard,
    pub screen: Screen,
    pub mode: EditorMode,
    pub status_message: Option<(String, Instant)>,
    pub plugin_registry: PluginRegistry,
    pub tab_size: usize,
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

        Self {
            documents: vec![initial_doc],
            active_doc_idx: 0,
            clipboard: Clipboard::new(),
            screen: Screen::new(),
            mode: EditorMode::Normal,
            status_message: Some((
                "Rustext ready. Ctrl+T: New tab | Ctrl+W: Close | PageUp/Dn: Switch tab | Ctrl+C/V: Clipboard | Ctrl+S: Save".to_string(),
                Instant::now(),
            )),
            plugin_registry: registry,
            tab_size: 4,
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
            EditorMode::ModalPrompt(modal) => match key {
                Key::Esc => {
                    self.mode = EditorMode::Normal;
                    self.set_status("Cancelled");
                }
                Key::Enter => {
                    let input = modal.input.clone();
                    let kind = modal.kind.clone();
                    self.mode = EditorMode::Normal;
                    self.commit_prompt(kind, &input);
                }
                Key::Backspace => {
                    modal.backspace();
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
            EditorAction::InsertTab => self.insert_tab_multicursor(),
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
                self.mode = EditorMode::ModalPrompt(PromptModal::new(PromptKind::OpenFile));
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
                self.mode = EditorMode::ModalPrompt(PromptModal::new(PromptKind::Find));
            }
            EditorAction::OpenSaveAsPrompt => {
                self.mode = EditorMode::ModalPrompt(PromptModal::new(PromptKind::SaveAs));
            }
            EditorAction::OpenCommandPalette => {
                self.mode = EditorMode::ModalPrompt(PromptModal::new(PromptKind::CommandPalette));
            }

            EditorAction::CollapseCursors => {
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

            EditorAction::None => {}
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
    fn commit_prompt(&mut self, kind: PromptKind, input: &str) {
        if input.is_empty() {
            return;
        }

        match kind {
            PromptKind::Find => {
                let current_head = self.doc().cursors.primary().head;
                let buf = self.doc().buffer.clone();
                if let Some((start, end)) = SearchEngine::find_next(&buf, input, current_head, false) {
                    self.doc_mut().cursors.set_single(start);
                    self.doc_mut().cursors.cursors_mut()[0] = Cursor::with_selection(start, end);
                    self.set_status(format!("Found match at offset {}", start));
                } else {
                    self.set_status(format!("Pattern not found: '{}'", input));
                }
            }
            PromptKind::SaveAs => {
                let path = PathBuf::from(input);
                let doc = self.doc_mut();
                if let Err(e) = doc.buffer.save_as(&path) {
                    self.set_status(format!("Error saving file: {}", e));
                } else {
                    doc.language = Language::from_path(&path);
                    self.set_status(format!("Saved to {}", input));
                }
            }
            PromptKind::OpenFile => {
                let path = Path::new(input);
                self.open_file(path);
            }
            PromptKind::CommandPalette => {
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

    /// Saves the current buffer to disk.
    pub fn save_document(&mut self) {
        if self.doc().buffer.file_path().is_none() {
            self.mode = EditorMode::ModalPrompt(PromptModal::new(PromptKind::SaveAs));
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
                let path_str = self
                    .doc()
                    .buffer
                    .file_path()
                    .and_then(|p| p.to_str())
                    .unwrap_or("file");
                self.set_status(format!("Saved {}", path_str));
            }
            Err(e) => {
                self.set_status(format!("Error saving file: {}", e));
            }
        }
    }

    // ---------------- Parallel Vector Multi-Cursor Editing ----------------

    /// Inserts a character across all cursors simultaneously.
    pub fn insert_char_multicursor(&mut self, ch: char) {
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
            } else {
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

        let doc = self.doc_mut();
        doc.cursors.set_cursors(new_cursors);
        tx.cursors_after = doc.cursors.cursors().to_vec();
        doc.history.record(tx);
    }

    /// Inserts a newline with automatic matching indentation.
    pub fn insert_newline_multicursor(&mut self) {
        let doc = self.doc_mut();
        let mut tx = Transaction::new(doc.cursors.cursors().to_vec());

        doc.cursors.normalize();
        let cursors_snapshot = doc.cursors.cursors().to_vec();
        let mut new_cursors = Vec::with_capacity(cursors_snapshot.len());
        let mut offset_shift: isize = 0;

        for cursor in cursors_snapshot {
            let adj_anchor = (cursor.anchor as isize + offset_shift).max(0) as usize;
            let adj_head = (cursor.head as isize + offset_shift).max(0) as usize;

            // Compute indentation of current line
            let line_pos = doc.buffer.char_to_position(adj_head);
            let current_line = doc.buffer.line(line_pos.line).unwrap_or_default();
            let indent: String = current_line
                .chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .collect();

            let insert_str = format!("\n{}", indent);
            let add_len = insert_str.chars().count();

            if cursor.has_selection() {
                let start = std::cmp::min(adj_anchor, adj_head);
                let end = std::cmp::max(adj_anchor, adj_head);
                let deleted = doc.buffer.get_text_range(start, end);
                doc.buffer.delete_range(start, end);
                tx.ops.push(EditOp::Delete {
                    offset: start,
                    text: deleted,
                });

                doc.buffer.insert(start, &insert_str);
                tx.ops.push(EditOp::Insert {
                    offset: start,
                    text: insert_str,
                });

                let del_len = end - start;
                offset_shift += add_len as isize - del_len as isize;
                new_cursors.push(Cursor::new(start + add_len));
            } else {
                doc.buffer.insert(adj_head, &insert_str);
                tx.ops.push(EditOp::Insert {
                    offset: adj_head,
                    text: insert_str,
                });

                offset_shift += add_len as isize;
                new_cursors.push(Cursor::new(adj_head + add_len));
            }
        }

        let doc = self.doc_mut();
        doc.cursors.set_cursors(new_cursors);
        tx.cursors_after = doc.cursors.cursors().to_vec();
        doc.history.record(tx);
    }

    /// Inserts tab characters or spaces across all cursors.
    pub fn insert_tab_multicursor(&mut self) {
        for _ in 0..self.tab_size {
            self.insert_char_multicursor(' ');
        }
    }

    /// Deletes character before cursor across all cursors (Backspace).
    pub fn backspace_multicursor(&mut self) {
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
            EditorMode::Normal => {
                let msg_text = self
                    .status_message
                    .as_ref()
                    .map(|(msg, _)| msg.as_str())
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
        let initial_width = 1024;
        let initial_height = 640;
        let mut window = crate::window::NativeWindow::new("Rustext", initial_width, initial_height)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let mut fb = crate::gfx::Framebuffer::new(initial_width, initial_height);
        let renderer = crate::gfx::GfxRenderer::new();

        while !self.should_quit && !window.should_close() {
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
                    }
                    crate::window::WindowEvent::Char(c) => {
                        self.handle_key(Key::Char(c));
                    }
                    crate::window::WindowEvent::Key(key) => {
                        self.handle_key(key);
                    }
                }
            }

            if self.should_quit {
                break;
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
            let content_h = fb.height.saturating_sub(renderer.tab_bar_height + renderer.status_bar_height);
            let visible_lines = content_h / crate::gfx::FONT_HEIGHT;
            let content_w = fb.width.saturating_sub(gutter_width + crate::gfx::FONT_WIDTH);
            let visible_cols = content_w / crate::gfx::FONT_WIDTH;

            {
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
            }

            // Render current frame into framebuffer
            let tab_titles: Vec<String> = self.documents.iter().map(|d| d.tab_title()).collect();
            let status_msg = self.status_message.as_ref().map(|(msg, _)| msg.as_str());
            let modal_ref = match &self.mode {
                EditorMode::ModalPrompt(modal) => Some(modal),
                EditorMode::Normal => None,
            };

            renderer.render(
                &mut fb,
                &self.documents[active_idx],
                &tab_titles,
                active_idx,
                status_msg,
                modal_ref,
            );

            // Present framebuffer to OS window
            window.present(&fb);

            // Yield CPU (~60 FPS)
            std::thread::sleep(std::time::Duration::from_millis(16));
        }

        Ok(())
    }
}
