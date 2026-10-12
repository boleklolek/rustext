//! Main editor state machine, multi-cursor edit engine, and event loop.
//! Zero third-party dependencies, purely standard library Rust.

use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::keymap::{EditorAction, Keymap};
use crate::core::buffer::TextBuffer;
use crate::core::cursor::{Cursor, CursorSet};
use crate::core::history::{EditOp, History, Transaction};
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
    pub buffer: TextBuffer,
    pub cursors: CursorSet,
    pub history: History,
    pub screen: Screen,
    pub mode: EditorMode,
    pub status_message: Option<(String, Instant)>,
    pub plugin_registry: PluginRegistry,
    pub language: Language,
    pub tab_size: usize,
    pub should_quit: bool,
}

impl Editor {
    /// Creates a new editor instance with an initial buffer.
    pub fn new(buffer: TextBuffer) -> Self {
        let lang = if let Some(path) = buffer.file_path() {
            Language::from_path(path)
        } else {
            Language::PlainText
        };

        let mut registry = PluginRegistry::new();
        registry.register(Box::new(TrailingWhitespacePlugin));
        registry.register(Box::new(WordCountPlugin));

        let mut ctx = PluginContext {
            file_path: buffer.file_path().map(|p| p.to_path_buf()),
            buffer_length: buffer.len_chars(),
            line_count: buffer.len_lines(),
            cursor_count: 1,
            status_message: None,
        };
        registry.trigger_init(&mut ctx);

        Self {
            buffer,
            cursors: CursorSet::new(),
            history: History::new(),
            screen: Screen::new(),
            mode: EditorMode::Normal,
            status_message: Some((
                "Rustext ready. Ctrl+S: Save | Ctrl+F: Find | Ctrl+D: Multi-cursor | Ctrl+Q: Quit".to_string(),
                Instant::now(),
            )),
            plugin_registry: registry,
            language: lang,
            tab_size: 4,
            should_quit: false,
        }
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
            EditorAction::InsertChar(c) => self.insert_char_multicursor(c),
            EditorAction::InsertNewline => self.insert_newline_multicursor(),
            EditorAction::InsertTab => self.insert_tab_multicursor(),
            EditorAction::Backspace => self.backspace_multicursor(),
            EditorAction::Delete => self.delete_multicursor(),

            EditorAction::MoveLeft { select } => self.cursors.move_left(&self.buffer, select),
            EditorAction::MoveRight { select } => self.cursors.move_right(&self.buffer, select),
            EditorAction::MoveUp { select } => self.cursors.move_up(&self.buffer, select),
            EditorAction::MoveDown { select } => self.cursors.move_down(&self.buffer, select),
            EditorAction::MoveLineStart { select } => self.cursors.move_to_line_start(&self.buffer, select),
            EditorAction::MoveLineEnd { select } => self.cursors.move_to_line_end(&self.buffer, select),

            EditorAction::AddCursorAbove => {
                self.cursors.add_cursor_above(&self.buffer);
                self.set_status(format!("Active cursors: {}", self.cursors.cursors().len()));
            }
            EditorAction::AddCursorBelow => {
                self.cursors.add_cursor_below(&self.buffer);
                self.set_status(format!("Active cursors: {}", self.cursors.cursors().len()));
            }
            EditorAction::SelectNextOccurrence => {
                if SearchEngine::select_next_occurrence(&self.buffer, &mut self.cursors) {
                    self.set_status(format!("Cursors: {}", self.cursors.cursors().len()));
                } else {
                    self.set_status("No further occurrences found");
                }
            }

            EditorAction::Undo => {
                if let Some(restored) = self.history.undo(&mut self.buffer) {
                    self.cursors.cursors_mut().clone_from_slice(&restored);
                    self.set_status(format!("Undo (remaining: {})", self.history.undo_count()));
                } else {
                    self.set_status("Already at oldest change");
                }
            }
            EditorAction::Redo => {
                if let Some(restored) = self.history.redo(&mut self.buffer) {
                    self.cursors.cursors_mut().clone_from_slice(&restored);
                    self.set_status(format!("Redo (remaining: {})", self.history.redo_count()));
                } else {
                    self.set_status("Already at newest change");
                }
            }

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
                let primary = self.cursors.primary();
                self.cursors.set_single(primary.head);
                self.set_status("Collapsed to single cursor");
            }

            EditorAction::Quit => {
                if self.buffer.is_dirty() {
                    self.set_status("Unsaved changes! Press Ctrl+Q again to discard or Ctrl+S to save");
                    self.should_quit = true;
                } else {
                    self.should_quit = true;
                }
            }

            EditorAction::None => {}
        }
    }

    /// Handles submission of interactive modal prompts.
    fn commit_prompt(&mut self, kind: PromptKind, input: &str) {
        if input.is_empty() {
            return;
        }

        match kind {
            PromptKind::Find => {
                let current_head = self.cursors.primary().head;
                if let Some((start, end)) = SearchEngine::find_next(&self.buffer, input, current_head, false) {
                    self.cursors.set_single(start);
                    self.cursors.cursors_mut()[0] = Cursor::with_selection(start, end);
                    self.set_status(format!("Found match at offset {}", start));
                } else {
                    self.set_status(format!("Pattern not found: '{}'", input));
                }
            }
            PromptKind::SaveAs => {
                let path = PathBuf::from(input);
                if let Err(e) = self.buffer.save_as(&path) {
                    self.set_status(format!("Error saving file: {}", e));
                } else {
                    self.language = Language::from_path(&path);
                    self.set_status(format!("Saved to {}", input));
                }
            }
            PromptKind::OpenFile => {
                let path = Path::new(input);
                match TextBuffer::open(path) {
                    Ok(new_buf) => {
                        self.language = Language::from_path(path);
                        self.buffer = new_buf;
                        self.cursors = CursorSet::new();
                        self.history = History::new();
                        self.set_status(format!("Opened {}", input));
                    }
                    Err(e) => {
                        self.set_status(format!("Could not open file: {}", e));
                    }
                }
            }
            PromptKind::CommandPalette => {
                let mut ctx = PluginContext {
                    file_path: self.buffer.file_path().map(|p| p.to_path_buf()),
                    buffer_length: self.buffer.len_chars(),
                    line_count: self.buffer.len_lines(),
                    cursor_count: self.cursors.cursors().len(),
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
        if self.buffer.file_path().is_none() {
            self.mode = EditorMode::ModalPrompt(PromptModal::new(PromptKind::SaveAs));
            return;
        }

        // Before save hook
        let mut ctx = PluginContext {
            file_path: self.buffer.file_path().map(|p| p.to_path_buf()),
            buffer_length: self.buffer.len_chars(),
            line_count: self.buffer.len_lines(),
            cursor_count: self.cursors.cursors().len(),
            status_message: None,
        };

        let mut content = self.buffer.to_string();
        self.plugin_registry.trigger_before_save(&mut ctx, &mut content);

        if content != self.buffer.to_string() {
            self.buffer = TextBuffer::from_str(&content);
            if let Some(p) = ctx.file_path.clone() {
                self.buffer.set_file_path(p);
            }
        }

        match self.buffer.save() {
            Ok(()) => {
                self.plugin_registry.trigger_after_save(&mut ctx);
                let path_str = self
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
        let mut tx = Transaction::new(self.cursors.cursors().to_vec());
        let s = ch.to_string();

        self.cursors.normalize();
        let cursors_snapshot = self.cursors.cursors().to_vec();
        let mut new_cursors = Vec::with_capacity(cursors_snapshot.len());
        let mut offset_shift: isize = 0;

        for cursor in cursors_snapshot {
            let adj_anchor = (cursor.anchor as isize + offset_shift).max(0) as usize;
            let adj_head = (cursor.head as isize + offset_shift).max(0) as usize;

            if cursor.has_selection() {
                let start = std::cmp::min(adj_anchor, adj_head);
                let end = std::cmp::max(adj_anchor, adj_head);
                let deleted = self.buffer.get_text_range(start, end);
                self.buffer.delete_range(start, end);
                tx.ops.push(EditOp::Delete {
                    offset: start,
                    text: deleted,
                });

                self.buffer.insert(start, &s);
                tx.ops.push(EditOp::Insert {
                    offset: start,
                    text: s.clone(),
                });

                let del_len = end - start;
                let add_len = s.chars().count();
                offset_shift += add_len as isize - del_len as isize;

                new_cursors.push(Cursor::new(start + add_len));
            } else {
                self.buffer.insert(adj_head, &s);
                tx.ops.push(EditOp::Insert {
                    offset: adj_head,
                    text: s.clone(),
                });

                let add_len = s.chars().count();
                offset_shift += add_len as isize;

                new_cursors.push(Cursor::new(adj_head + add_len));
            }
        }

        self.cursors.set_cursors(new_cursors);
        tx.cursors_after = self.cursors.cursors().to_vec();
        self.history.record(tx);
    }

    /// Inserts a newline with automatic matching indentation.
    pub fn insert_newline_multicursor(&mut self) {
        let mut tx = Transaction::new(self.cursors.cursors().to_vec());

        self.cursors.normalize();
        let cursors_snapshot = self.cursors.cursors().to_vec();
        let mut new_cursors = Vec::with_capacity(cursors_snapshot.len());
        let mut offset_shift: isize = 0;

        for cursor in cursors_snapshot {
            let adj_anchor = (cursor.anchor as isize + offset_shift).max(0) as usize;
            let adj_head = (cursor.head as isize + offset_shift).max(0) as usize;

            // Compute indentation of current line
            let line_pos = self.buffer.char_to_position(adj_head);
            let current_line = self.buffer.line(line_pos.line).unwrap_or_default();
            let indent: String = current_line
                .chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .collect();

            let insert_str = format!("\n{}", indent);
            let add_len = insert_str.chars().count();

            if cursor.has_selection() {
                let start = std::cmp::min(adj_anchor, adj_head);
                let end = std::cmp::max(adj_anchor, adj_head);
                let deleted = self.buffer.get_text_range(start, end);
                self.buffer.delete_range(start, end);
                tx.ops.push(EditOp::Delete {
                    offset: start,
                    text: deleted,
                });

                self.buffer.insert(start, &insert_str);
                tx.ops.push(EditOp::Insert {
                    offset: start,
                    text: insert_str,
                });

                let del_len = end - start;
                offset_shift += add_len as isize - del_len as isize;
                new_cursors.push(Cursor::new(start + add_len));
            } else {
                self.buffer.insert(adj_head, &insert_str);
                tx.ops.push(EditOp::Insert {
                    offset: adj_head,
                    text: insert_str,
                });

                offset_shift += add_len as isize;
                new_cursors.push(Cursor::new(adj_head + add_len));
            }
        }

        self.cursors.set_cursors(new_cursors);
        tx.cursors_after = self.cursors.cursors().to_vec();
        self.history.record(tx);
    }

    /// Inserts tab characters or spaces across all cursors.
    pub fn insert_tab_multicursor(&mut self) {
        for _ in 0..self.tab_size {
            self.insert_char_multicursor(' ');
        }
    }

    /// Deletes character before cursor across all cursors (Backspace).
    pub fn backspace_multicursor(&mut self) {
        let mut tx = Transaction::new(self.cursors.cursors().to_vec());

        self.cursors.normalize();
        let cursors_snapshot = self.cursors.cursors().to_vec();
        let mut new_cursors = Vec::with_capacity(cursors_snapshot.len());
        let mut offset_shift: isize = 0;

        for cursor in cursors_snapshot {
            let adj_anchor = (cursor.anchor as isize + offset_shift).max(0) as usize;
            let adj_head = (cursor.head as isize + offset_shift).max(0) as usize;

            if cursor.has_selection() {
                let start = std::cmp::min(adj_anchor, adj_head);
                let end = std::cmp::max(adj_anchor, adj_head);
                let deleted = self.buffer.get_text_range(start, end);
                self.buffer.delete_range(start, end);
                tx.ops.push(EditOp::Delete {
                    offset: start,
                    text: deleted,
                });

                let del_len = end - start;
                offset_shift -= del_len as isize;
                new_cursors.push(Cursor::new(start));
            } else if adj_head > 0 {
                let target_offset = adj_head - 1;
                let deleted = self.buffer.get_text_range(target_offset, adj_head);
                self.buffer.delete_range(target_offset, adj_head);
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

        self.cursors.set_cursors(new_cursors);
        tx.cursors_after = self.cursors.cursors().to_vec();
        self.history.record(tx);
    }

    /// Deletes character after cursor across all cursors (Delete).
    pub fn delete_multicursor(&mut self) {
        let mut tx = Transaction::new(self.cursors.cursors().to_vec());

        self.cursors.normalize();
        let cursors_snapshot = self.cursors.cursors().to_vec();
        let mut new_cursors = Vec::with_capacity(cursors_snapshot.len());
        let mut offset_shift: isize = 0;

        for cursor in cursors_snapshot {
            let adj_anchor = (cursor.anchor as isize + offset_shift).max(0) as usize;
            let adj_head = (cursor.head as isize + offset_shift).max(0) as usize;
            let total_chars = self.buffer.len_chars();

            if cursor.has_selection() {
                let start = std::cmp::min(adj_anchor, adj_head);
                let end = std::cmp::max(adj_anchor, adj_head);
                let deleted = self.buffer.get_text_range(start, end);
                self.buffer.delete_range(start, end);
                tx.ops.push(EditOp::Delete {
                    offset: start,
                    text: deleted,
                });

                let del_len = end - start;
                offset_shift -= del_len as isize;
                new_cursors.push(Cursor::new(start));
            } else if adj_head < total_chars {
                let target_offset = adj_head;
                let deleted = self.buffer.get_text_range(target_offset, target_offset + 1);
                self.buffer.delete_range(target_offset, target_offset + 1);
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

        self.cursors.set_cursors(new_cursors);
        tx.cursors_after = self.cursors.cursors().to_vec();
        self.history.record(tx);
    }

    /// Renders the complete editor view.
    pub fn render(&mut self) -> io::Result<()> {
        let primary_head = self.cursors.primary().head;
        let primary_pos = self.buffer.char_to_position(primary_head);
        let gutter_width = Screen::calculate_gutter_width(self.buffer.len_lines());

        // Adjust viewport to keep primary cursor in view
        self.screen.adjust_viewport(primary_pos.line, primary_pos.col, gutter_width);

        self.screen.begin_frame();

        // 1. Document canvas
        self.screen.render_document(&self.buffer, &self.cursors, self.language);

        // 2. Status bar
        let file_name = self
            .buffer
            .file_path()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[Untitled]");

        let mode_str = match &self.mode {
            EditorMode::Normal => "EDIT",
            EditorMode::ModalPrompt(_) => "PROMPT",
        };

        self.screen.render_status_bar(
            mode_str,
            file_name,
            self.buffer.is_dirty(),
            (primary_pos.line, primary_pos.col),
            self.cursors.cursors().len(),
            self.buffer.len_lines(),
            self.buffer.len_chars(),
            self.language.name(),
        );

        // 3. Prompt or message line
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

        // Calculate cursor screen position
        let (cursor_screen_row, cursor_screen_col) = match &self.mode {
            EditorMode::ModalPrompt(modal) => {
                let prompt_row = self.screen.rows.saturating_sub(1);
                let prompt_col = modal.kind.label().chars().count() + modal.cursor_col;
                (prompt_row, prompt_col)
            }
            EditorMode::Normal => {
                let row = primary_pos.line.saturating_sub(self.screen.scroll_row);
                let col = gutter_width + primary_pos.col.saturating_sub(self.screen.scroll_col);
                (row, col)
            }
        };

        self.screen.end_frame(cursor_screen_row, cursor_screen_col)?;
        Ok(())
    }
}
