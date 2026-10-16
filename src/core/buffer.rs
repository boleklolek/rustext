//! TextBuffer: high-level document representation with line indexing and file I/O.
//! Zero third-party dependencies, purely standard library Rust.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::piece_table::PieceTable;

/// Represents a 0-indexed line and column position in the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

impl Position {
    pub const ZERO: Self = Self { line: 0, col: 0 };

    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

/// A text document buffer backed by a PieceTable with fast line caching.
#[derive(Debug, Clone)]
pub struct TextBuffer {
    table: PieceTable,
    /// Character offset for the beginning of each line (0-indexed).
    line_starts: Vec<usize>,
    file_path: Option<PathBuf>,
    is_dirty: bool,
}

impl TextBuffer {
    /// Creates a new empty text buffer.
    pub fn new() -> Self {
        let table = PieceTable::new();
        let mut buf = Self {
            table,
            line_starts: vec![0],
            file_path: None,
            is_dirty: false,
        };
        buf.rebuild_line_index();
        buf
    }

    /// Creates a text buffer initialized with the given string.
    pub fn from_str(text: &str) -> Self {
        let table = PieceTable::from_str(text);
        let mut buf = Self {
            table,
            line_starts: vec![0],
            file_path: None,
            is_dirty: false,
        };
        buf.rebuild_line_index();
        buf
    }

    /// Loads a text buffer from a file on disk.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path_ref = path.as_ref();
        let content = fs::read_to_string(path_ref)?;
        let mut buf = Self::from_str(&content);
        buf.file_path = Some(path_ref.to_path_buf());
        buf.is_dirty = false;
        Ok(buf)
    }

    /// Saves the buffer content to its associated file path.
    pub fn save(&mut self) -> io::Result<()> {
        if let Some(path) = &self.file_path {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            let content = self.to_string();
            let mut file = fs::File::create(path)?;
            file.write_all(content.as_bytes())?;
            file.flush()?;
            self.is_dirty = false;
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No file path specified for buffer",
            ))
        }
    }

    /// Saves the buffer content to a specified path and updates the buffer path.
    pub fn save_as<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let path_buf = path.as_ref().to_path_buf();
        self.file_path = Some(path_buf);
        self.save()
    }

    /// Rebuilds the line index (`line_starts`).
    pub fn rebuild_line_index(&mut self) {
        self.line_starts.clear();
        self.line_starts.push(0);

        let mut char_offset = 0;
        for chunk in self.table.chunks() {
            for ch in chunk.chars() {
                char_offset += 1;
                if ch == '\n' {
                    self.line_starts.push(char_offset);
                }
            }
        }
    }

    /// Returns the total number of characters in the buffer.
    pub fn len_chars(&self) -> usize {
        self.table.len_chars()
    }

    /// Checks if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Returns the number of lines (at least 1).
    pub fn len_lines(&self) -> usize {
        self.line_starts.len()
    }

    /// Returns whether the buffer has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    /// Explicitly sets the dirty state of the buffer.
    pub fn set_dirty(&mut self, dirty: bool) {
        self.is_dirty = dirty;
    }

    /// Returns the file path if one is associated with this buffer.
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    /// Sets the file path.
    pub fn set_file_path(&mut self, path: PathBuf) {
        self.file_path = Some(path);
    }

    /// Converts a character offset to a (line, col) Position.
    pub fn char_to_position(&self, char_idx: usize) -> Position {
        let clamped = char_idx.min(self.table.len_chars());
        // Binary search in line_starts
        let line_idx = match self.line_starts.binary_search(&clamped) {
            Ok(exact) => exact,
            Err(insert_idx) => insert_idx.saturating_sub(1),
        };

        let line_start = self.line_starts[line_idx];
        let col = clamped.saturating_sub(line_start);

        Position::new(line_idx, col)
    }

    /// Converts a (line, col) Position to a character offset.
    pub fn position_to_char(&self, pos: Position) -> usize {
        if self.line_starts.is_empty() {
            return 0;
        }

        let line = pos.line.min(self.line_starts.len() - 1);
        let line_start = self.line_starts[line];
        let line_end = if line + 1 < self.line_starts.len() {
            // Note: subtract 1 if line ends in newline, to stay on the line
            self.line_starts[line + 1].saturating_sub(1)
        } else {
            self.table.len_chars()
        };

        let col = pos.col.min(line_end.saturating_sub(line_start));
        line_start + col
    }

    /// Inserts text at the given character offset.
    pub fn insert(&mut self, char_idx: usize, text: &str) {
        if text.is_empty() {
            return;
        }
        self.table.insert(char_idx, text);
        self.rebuild_line_index();
        self.is_dirty = true;
    }

    /// Inserts text at a specific Position (line, col).
    pub fn insert_at_pos(&mut self, pos: Position, text: &str) {
        let char_idx = self.position_to_char(pos);
        self.insert(char_idx, text);
    }

    /// Deletes character range [start_char..end_char].
    pub fn delete_range(&mut self, start_char: usize, end_char: usize) {
        if start_char >= end_char {
            return;
        }
        self.table.delete(start_char, end_char);
        self.rebuild_line_index();
        self.is_dirty = true;
    }

    /// Retrieves text within a character range.
    pub fn get_text_range(&self, start_char: usize, end_char: usize) -> String {
        self.table.slice_chars(start_char, end_char)
    }

    /// Retrieves a single character at a character offset, if within bounds.
    pub fn char_at(&self, char_idx: usize) -> Option<char> {
        self.table.char_at(char_idx)
    }

    /// Returns the number of pieces currently in the piece table.
    pub fn piece_count(&self) -> usize {
        self.table.piece_count()
    }

    /// Returns the total byte length across all pieces.
    pub fn len_bytes(&self) -> usize {
        self.table.len_bytes()
    }

    /// Retrieves the content of a specific line as a String (excluding the trailing newline).
    pub fn line(&self, line_idx: usize) -> Option<String> {
        if line_idx >= self.line_starts.len() {
            return None;
        }

        let start = self.line_starts[line_idx];
        let end = if line_idx + 1 < self.line_starts.len() {
            self.line_starts[line_idx + 1]
        } else {
            self.table.len_chars()
        };

        let raw = self.table.slice_chars(start, end);
        let trimmed = raw.trim_end_matches(['\r', '\n']);
        Some(trimmed.to_string())
    }

    /// Returns the length of a specific line in characters (excluding trailing newline).
    pub fn line_len(&self, line_idx: usize) -> usize {
        self.line(line_idx).map(|s| s.chars().count()).unwrap_or(0)
    }

    /// Returns the complete text of the buffer.
    pub fn to_string(&self) -> String {
        self.table.to_string()
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_lines_and_positions() {
        let text = "first line\nsecond line\nthird";
        let buf = TextBuffer::from_str(text);

        assert_eq!(buf.len_lines(), 3);
        assert_eq!(buf.line(0), Some("first line".to_string()));
        assert_eq!(buf.line(1), Some("second line".to_string()));
        assert_eq!(buf.line(2), Some("third".to_string()));
        assert_eq!(buf.line(3), None);

        // Position to char and char to position
        let pos1 = Position::new(1, 7); // "second " -> 'l'
        let char_idx = buf.position_to_char(pos1);
        let roundtrip_pos = buf.char_to_position(char_idx);
        assert_eq!(pos1, roundtrip_pos);
    }

    #[test]
    fn test_buffer_edits() {
        let mut buf = TextBuffer::from_str("Hello\nWorld");
        assert_eq!(buf.len_lines(), 2);

        // Insert newline in the middle
        let pos = Position::new(0, 5);
        buf.insert_at_pos(pos, "\nBeautiful");
        assert_eq!(buf.len_lines(), 3);
        assert_eq!(buf.line(1), Some("Beautiful".to_string()));

        // Delete range
        let start = buf.position_to_char(Position::new(1, 0));
        let end = buf.position_to_char(Position::new(2, 0));
        buf.delete_range(start, end);
        assert_eq!(buf.len_lines(), 2);
    }
}
