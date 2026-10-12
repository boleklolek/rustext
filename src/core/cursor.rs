//! Multi-cursor and selection engine for vector parallel editing.
//! Zero third-party dependencies, purely standard library Rust.

use std::cmp::{max, min};
use std::ops::Range;

use super::buffer::{Position, TextBuffer};

/// A single cursor with an anchor (for selection) and head (active point).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub anchor: usize, // Character offset where selection started
    pub head: usize,   // Character offset of current cursor position
}

impl Cursor {
    /// Creates a collapsed cursor at a single character offset.
    pub fn new(position: usize) -> Self {
        Self {
            anchor: position,
            head: position,
        }
    }

    /// Creates a cursor with a selection between anchor and head.
    pub fn with_selection(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    /// Checks if the cursor has an active selection.
    pub fn has_selection(&self) -> bool {
        self.anchor != self.head
    }

    /// Returns the character range of the selection (ordered min..max).
    pub fn selection_range(&self) -> Range<usize> {
        min(self.anchor, self.head)..max(self.anchor, self.head)
    }

    /// Collapses the cursor selection to the head position.
    pub fn collapse(&mut self) {
        self.anchor = self.head;
    }

    /// Collapses to the beginning of the selection.
    pub fn collapse_to_start(&mut self) {
        let start = min(self.anchor, self.head);
        self.anchor = start;
        self.head = start;
    }

    /// Collapses to the end of the selection.
    pub fn collapse_to_end(&mut self) {
        let end = max(self.anchor, self.head);
        self.anchor = end;
        self.head = end;
    }
}

/// Collection of cursors with vector operations and automatic deduplication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorSet {
    cursors: Vec<Cursor>,
    primary_idx: usize,
}

impl CursorSet {
    /// Creates a new cursor set with a single cursor at offset 0.
    pub fn new() -> Self {
        Self {
            cursors: vec![Cursor::new(0)],
            primary_idx: 0,
        }
    }

    /// Creates a cursor set with a single cursor at the given offset.
    pub fn single(pos: usize) -> Self {
        Self {
            cursors: vec![Cursor::new(pos)],
            primary_idx: 0,
        }
    }

    /// Returns the list of active cursors.
    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    /// Returns mutable reference to the cursors.
    pub fn cursors_mut(&mut self) -> &mut [Cursor] {
        &mut self.cursors
    }

    /// Returns the primary cursor.
    pub fn primary(&self) -> Cursor {
        self.cursors.get(self.primary_idx).copied().unwrap_or(Cursor::new(0))
    }

    /// Sets a single cursor and discards all others.
    pub fn set_single(&mut self, pos: usize) {
        self.cursors = vec![Cursor::new(pos)];
        self.primary_idx = 0;
    }

    /// Replaces the cursor list with new cursors and normalizes.
    pub fn set_cursors(&mut self, cursors: Vec<Cursor>) {
        self.cursors = cursors;
        self.normalize();
    }

    /// Adds a new cursor, keeping the list sorted and deduplicated.
    pub fn add_cursor(&mut self, cursor: Cursor) {
        self.cursors.push(cursor);
        self.normalize();
    }

    /// Sorts cursors by position and merges any overlapping selections.
    pub fn normalize(&mut self) {
        if self.cursors.len() <= 1 {
            return;
        }

        // Sort by selection start then head
        self.cursors.sort_by_key(|c| (min(c.anchor, c.head), max(c.anchor, c.head)));

        let mut merged = Vec::with_capacity(self.cursors.len());
        let cursors = std::mem::take(&mut self.cursors);
        let mut iter = cursors.into_iter();

        if let Some(first) = iter.next() {
            let mut current = first;

            for next in iter {
                let curr_range = current.selection_range();
                let next_range = next.selection_range();

                // Check overlap or identical position
                if curr_range.end >= next_range.start && (curr_range.start != curr_range.end || curr_range.end == next_range.start) {
                    // Merge
                    let new_start = min(curr_range.start, next_range.start);
                    let new_end = max(curr_range.end, next_range.end);
                    // Retain direction of the later cursor
                    current = if next.head >= next.anchor {
                        Cursor::with_selection(new_start, new_end)
                    } else {
                        Cursor::with_selection(new_end, new_start)
                    };
                } else {
                    merged.push(current);
                    current = next;
                }
            }
            merged.push(current);
        }

        self.cursors = merged;
        if self.primary_idx >= self.cursors.len() {
            self.primary_idx = self.cursors.len().saturating_sub(1);
        }
    }

    /// Moves all cursors left by one character.
    pub fn move_left(&mut self, _buffer: &TextBuffer, select: bool) {
        for c in &mut self.cursors {
            if !select && c.has_selection() {
                c.collapse_to_start();
            } else if c.head > 0 {
                c.head -= 1;
                if !select {
                    c.anchor = c.head;
                }
            }
        }
        self.normalize();
    }

    /// Moves all cursors right by one character.
    pub fn move_right(&mut self, buffer: &TextBuffer, select: bool) {
        let max_len = buffer.len_chars();
        for c in &mut self.cursors {
            if !select && c.has_selection() {
                c.collapse_to_end();
            } else if c.head < max_len {
                c.head += 1;
                if !select {
                    c.anchor = c.head;
                }
            }
        }
        self.normalize();
    }

    /// Moves all cursors up by one line.
    pub fn move_up(&mut self, buffer: &TextBuffer, select: bool) {
        for c in &mut self.cursors {
            let pos = buffer.char_to_position(c.head);
            if pos.line > 0 {
                let target_line = pos.line - 1;
                let line_len = buffer.line_len(target_line);
                let target_col = min(pos.col, line_len);
                let target_char = buffer.position_to_char(Position::new(target_line, target_col));
                c.head = target_char;
                if !select {
                    c.anchor = target_char;
                }
            }
        }
        self.normalize();
    }

    /// Moves all cursors down by one line.
    pub fn move_down(&mut self, buffer: &TextBuffer, select: bool) {
        let total_lines = buffer.len_lines();
        for c in &mut self.cursors {
            let pos = buffer.char_to_position(c.head);
            if pos.line + 1 < total_lines {
                let target_line = pos.line + 1;
                let line_len = buffer.line_len(target_line);
                let target_col = min(pos.col, line_len);
                let target_char = buffer.position_to_char(Position::new(target_line, target_col));
                c.head = target_char;
                if !select {
                    c.anchor = target_char;
                }
            }
        }
        self.normalize();
    }

    /// Moves all cursors to the start of their current line.
    pub fn move_to_line_start(&mut self, buffer: &TextBuffer, select: bool) {
        for c in &mut self.cursors {
            let pos = buffer.char_to_position(c.head);
            let target_char = buffer.position_to_char(Position::new(pos.line, 0));
            c.head = target_char;
            if !select {
                c.anchor = target_char;
            }
        }
        self.normalize();
    }

    /// Moves all cursors to the end of their current line.
    pub fn move_to_line_end(&mut self, buffer: &TextBuffer, select: bool) {
        for c in &mut self.cursors {
            let pos = buffer.char_to_position(c.head);
            let line_len = buffer.line_len(pos.line);
            let target_char = buffer.position_to_char(Position::new(pos.line, line_len));
            c.head = target_char;
            if !select {
                c.anchor = target_char;
            }
        }
        self.normalize();
    }

    /// Adds a cursor on the line directly below each existing cursor.
    pub fn add_cursor_below(&mut self, buffer: &TextBuffer) {
        let mut new_cursors = Vec::new();
        let total_lines = buffer.len_lines();

        for c in &self.cursors {
            let pos = buffer.char_to_position(c.head);
            if pos.line + 1 < total_lines {
                let target_line = pos.line + 1;
                let line_len = buffer.line_len(target_line);
                let target_col = min(pos.col, line_len);
                let new_pos = buffer.position_to_char(Position::new(target_line, target_col));
                new_cursors.push(Cursor::new(new_pos));
            }
        }

        for nc in new_cursors {
            self.cursors.push(nc);
        }
        self.normalize();
    }

    /// Adds a cursor on the line directly above each existing cursor.
    pub fn add_cursor_above(&mut self, buffer: &TextBuffer) {
        let mut new_cursors = Vec::new();

        for c in &self.cursors {
            let pos = buffer.char_to_position(c.head);
            if pos.line > 0 {
                let target_line = pos.line - 1;
                let line_len = buffer.line_len(target_line);
                let target_col = min(pos.col, line_len);
                let new_pos = buffer.position_to_char(Position::new(target_line, target_col));
                new_cursors.push(Cursor::new(new_pos));
            }
        }

        for nc in new_cursors {
            self.cursors.push(nc);
        }
        self.normalize();
    }
}

impl Default for CursorSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_selection() {
        let c = Cursor::with_selection(5, 10);
        assert!(c.has_selection());
        assert_eq!(c.selection_range(), 5..10);

        let c_rev = Cursor::with_selection(10, 5);
        assert!(c_rev.has_selection());
        assert_eq!(c_rev.selection_range(), 5..10);
    }

    #[test]
    fn test_multicursor_deduplication() {
        let mut cs = CursorSet::new();
        cs.set_single(5);
        cs.add_cursor(Cursor::new(5)); // duplicate
        cs.add_cursor(Cursor::new(10));
        assert_eq!(cs.cursors().len(), 2);
    }

    #[test]
    fn test_multicursor_motions() {
        let buf = TextBuffer::from_str("line 1\nline 2\nline 3");
        let mut cs = CursorSet::new();
        cs.set_single(0); // at 'l' in line 1

        cs.add_cursor_below(&buf); // adds cursor at line 2
        assert_eq!(cs.cursors().len(), 2);

        cs.move_right(&buf, false);
        assert_eq!(cs.cursors()[0].head, 1);
        assert_eq!(cs.cursors()[1].head, 8); // 7 (newline) + 1
    }
}
