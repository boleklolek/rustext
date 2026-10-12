//! Search and multi-cursor occurrence matching engine.
//! Zero third-party dependencies, purely standard library Rust.

use super::buffer::TextBuffer;
use super::cursor::{Cursor, CursorSet};

/// Search utility for text matching and multi-cursor occurrence expansion.
pub struct SearchEngine;

impl SearchEngine {
    /// Finds all matching character ranges for `query` in the given buffer.
    pub fn find_all(buffer: &TextBuffer, query: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
        if query.is_empty() {
            return Vec::new();
        }

        let full_text = buffer.to_string();
        let query_char_len = query.chars().count();
        let mut matches = Vec::new();

        if case_sensitive {
            let chars: Vec<char> = full_text.chars().collect();
            let query_chars: Vec<char> = query.chars().collect();

            if chars.len() < query_chars.len() {
                return matches;
            }

            for i in 0..=(chars.len() - query_chars.len()) {
                if chars[i..i + query_chars.len()] == query_chars[..] {
                    matches.push((i, i + query_char_len));
                }
            }
        } else {
            let full_lower: Vec<char> = full_text.to_lowercase().chars().collect();
            let query_lower: Vec<char> = query.to_lowercase().chars().collect();

            if full_lower.len() < query_lower.len() {
                return matches;
            }

            for i in 0..=(full_lower.len() - query_lower.len()) {
                if full_lower[i..i + query_lower.len()] == query_lower[..] {
                    matches.push((i, i + query_char_len));
                }
            }
        }

        matches
    }

    /// Finds the next occurrence of `query` after `from_char` (cycles to beginning).
    pub fn find_next(
        buffer: &TextBuffer,
        query: &str,
        from_char: usize,
        case_sensitive: bool,
    ) -> Option<(usize, usize)> {
        let matches = Self::find_all(buffer, query, case_sensitive);
        if matches.is_empty() {
            return None;
        }

        for m in &matches {
            if m.0 >= from_char {
                return Some(*m);
            }
        }

        // Cycle to first match
        matches.first().copied()
    }

    /// Finds the previous occurrence of `query` before `from_char` (cycles to end).
    pub fn find_prev(
        buffer: &TextBuffer,
        query: &str,
        from_char: usize,
        case_sensitive: bool,
    ) -> Option<(usize, usize)> {
        let matches = Self::find_all(buffer, query, case_sensitive);
        if matches.is_empty() {
            return None;
        }

        for m in matches.iter().rev() {
            if m.1 <= from_char {
                return Some(*m);
            }
        }

        // Cycle to last match
        matches.last().copied()
    }

    /// Returns the character range of the word at `char_offset` (alphanumeric + underscore).
    pub fn word_at(buffer: &TextBuffer, char_offset: usize) -> Option<(usize, usize)> {
        let text = buffer.to_string();
        let chars: Vec<char> = text.chars().collect();
        let total = chars.len();

        if total == 0 || char_offset >= total {
            return None;
        }

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

        if !is_word_char(chars[char_offset]) {
            return None;
        }

        let mut start = char_offset;
        while start > 0 && is_word_char(chars[start - 1]) {
            start -= 1;
        }

        let mut end = char_offset;
        while end < total && is_word_char(chars[end]) {
            end += 1;
        }

        if start < end {
            Some((start, end))
        } else {
            None
        }
    }

    /// Extends cursors by selecting the next occurrence of the currently selected word (like Ctrl+D).
    pub fn select_next_occurrence(buffer: &TextBuffer, cursor_set: &mut CursorSet) -> bool {
        let primary = cursor_set.primary();

        if !primary.has_selection() {
            // No selection: select word under primary cursor
            if let Some((start, end)) = Self::word_at(buffer, primary.head) {
                cursor_set.set_single(start);
                cursor_set.cursors_mut()[0] = Cursor::with_selection(start, end);
                return true;
            }
            return false;
        }

        let sel_range = primary.selection_range();
        let selected_text = buffer.get_text_range(sel_range.start, sel_range.end);
        if selected_text.is_empty() {
            return false;
        }

        // Find matches in buffer
        let matches = Self::find_all(buffer, &selected_text, true);
        if matches.len() <= 1 {
            return false;
        }

        // Find first match not already covered by an existing cursor
        let existing_ranges: Vec<(usize, usize)> = cursor_set
            .cursors()
            .iter()
            .map(|c| (c.selection_range().start, c.selection_range().end))
            .collect();

        // Start search after the last cursor's head
        let last_head = cursor_set
            .cursors()
            .iter()
            .map(|c| c.head)
            .max()
            .unwrap_or(0);

        let mut target_match = None;
        // Check after last cursor
        for m in &matches {
            if m.0 >= last_head && !existing_ranges.contains(m) {
                target_match = Some(*m);
                break;
            }
        }

        // If not found, wrap around
        if target_match.is_none() {
            for m in &matches {
                if !existing_ranges.contains(m) {
                    target_match = Some(*m);
                    break;
                }
            }
        }

        if let Some((start, end)) = target_match {
            cursor_set.add_cursor(Cursor::with_selection(start, end));
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_and_ctrl_d() {
        let buf = TextBuffer::from_str("let foo = 1;\nlet foo = 2;\nlet foo = 3;");
        let matches = SearchEngine::find_all(&buf, "foo", true);
        assert_eq!(matches.len(), 3);

        let mut cs = CursorSet::single(4); // at 'f' of first foo
        // Select word
        assert!(SearchEngine::select_next_occurrence(&buf, &mut cs));
        assert_eq!(cs.cursors().len(), 1);
        assert_eq!(cs.cursors()[0].selection_range(), 4..7);

        // Add second occurrence
        assert!(SearchEngine::select_next_occurrence(&buf, &mut cs));
        assert_eq!(cs.cursors().len(), 2);
    }
}
