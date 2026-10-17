//! Encapsulates an open document (tab) with its buffer, cursors, history, and viewport state.
//! Zero third-party dependencies, purely standard library Rust.

use crate::core::buffer::TextBuffer;
use crate::core::cursor::CursorSet;
use crate::core::history::History;
use crate::core::syntax::Language;

#[derive(Debug, Clone)]
pub struct Document {
    pub buffer: TextBuffer,
    pub cursors: CursorSet,
    pub history: History,
    pub language: Language,
    pub scroll_row: usize,
    pub scroll_col: usize,
    pub scroll_subline: usize,
}

impl Document {
    /// Creates a new Document wrapping a TextBuffer.
    pub fn new(buffer: TextBuffer) -> Self {
        let lang = if let Some(path) = buffer.file_path() {
            Language::from_path(path)
        } else {
            Language::PlainText
        };

        Self {
            buffer,
            cursors: CursorSet::new(),
            history: History::new(),
            language: lang,
            scroll_row: 0,
            scroll_col: 0,
            scroll_subline: 0,
        }
    }

    /// Display title for the tab bar.
    pub fn tab_title(&self) -> String {
        let name = self
            .buffer
            .file_path()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[Untitled]");

        if self.buffer.is_dirty() {
            format!("{}*", name)
        } else {
            name.to_string()
        }
    }
}
