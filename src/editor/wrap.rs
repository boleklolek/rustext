//! Visual Soft Wrap / Word Wrap engine.
//! Zero third-party dependencies, purely standard library Rust.
//! Projects long physical lines into visual sublines without modifying the underlying text buffer.

use std::cmp::min;
use crate::core::buffer::{Position, TextBuffer};
use crate::core::cursor::CursorSet;
use super::document::Document;

/// Describes a single visual subline projected from a physical line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SublineInfo {
    /// Starting character index within the physical line (0-indexed).
    pub char_start: usize,
    /// Ending character index within the physical line (exclusive).
    pub char_end: usize,
    /// Starting byte offset in the UTF-8 string for this subline.
    pub byte_start: usize,
    /// Ending byte offset in the UTF-8 string for this subline.
    pub byte_end: usize,
    /// True if this is a continuation row of an already-started physical line.
    pub is_continuation: bool,
}

impl SublineInfo {
    /// Returns the character count of this visual subline.
    #[inline]
    pub fn len_chars(&self) -> usize {
        self.char_end.saturating_sub(self.char_start)
    }

    /// Returns true if this visual subline has 0 characters.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.char_start >= self.char_end
    }
}

/// Static utilities and algorithms for soft word wrapping.
pub struct WordWrap;

impl WordWrap {
    /// Determines whether a character is a suitable word-break boundary.
    #[inline]
    pub fn is_word_break_char(c: char) -> bool {
        c == ' ' || c == '\t' || matches!(c, ',' | ';' | ':' | '.' | ')' | ']' | '}' | '>' | '-' | '_' | '/' | '\\')
    }

    /// Breaks a physical line into one or more visual sublines fitting inside `max_cols`.
    ///
    /// Respects word boundaries (whitespace and punctuation) when near `max_cols`,
    /// falling back to a hard character break only for single unbroken tokens longer than `max_cols`.
    pub fn compute_sublines(line_text: &str, max_cols: usize) -> Vec<SublineInfo> {
        let cols = max_cols.max(1);

        if line_text.is_empty() {
            return vec![SublineInfo {
                char_start: 0,
                char_end: 0,
                byte_start: 0,
                byte_end: 0,
                is_continuation: false,
            }];
        }

        let char_indices: Vec<(usize, char)> = line_text.char_indices().collect();
        let total_chars = char_indices.len();
        let total_bytes = line_text.len();

        if total_chars <= cols {
            return vec![SublineInfo {
                char_start: 0,
                char_end: total_chars,
                byte_start: 0,
                byte_end: total_bytes,
                is_continuation: false,
            }];
        }

        let mut sublines = Vec::new();
        let mut cur_char_start = 0;

        while cur_char_start < total_chars {
            let remaining_chars = total_chars - cur_char_start;
            if remaining_chars <= cols {
                let byte_start = char_indices[cur_char_start].0;
                sublines.push(SublineInfo {
                    char_start: cur_char_start,
                    char_end: total_chars,
                    byte_start,
                    byte_end: total_bytes,
                    is_continuation: !sublines.is_empty(),
                });
                break;
            }

            let end_limit = cur_char_start + cols;
            // Search backwards for a suitable word break candidate in the last 15 columns
            let min_break = cur_char_start + (cols.saturating_sub(15)).max(1);
            let mut break_idx = None;

            for idx in (min_break..=end_limit).rev() {
                let ch = char_indices[idx - 1].1;
                if Self::is_word_break_char(ch) {
                    break_idx = Some(idx);
                    break;
                }
            }

            let chosen_char_end = break_idx.unwrap_or(end_limit);
            let byte_start = char_indices[cur_char_start].0;
            let byte_end = if chosen_char_end < total_chars {
                char_indices[chosen_char_end].0
            } else {
                total_bytes
            };

            sublines.push(SublineInfo {
                char_start: cur_char_start,
                char_end: chosen_char_end,
                byte_start,
                byte_end,
                is_continuation: !sublines.is_empty(),
            });

            cur_char_start = chosen_char_end;
        }

        if sublines.is_empty() {
            sublines.push(SublineInfo {
                char_start: 0,
                char_end: 0,
                byte_start: 0,
                byte_end: 0,
                is_continuation: false,
            });
        }

        sublines
    }

    /// Converts a physical character column into `(subline_index, visual_col)`.
    pub fn col_to_visual(col: usize, sublines: &[SublineInfo]) -> (usize, usize) {
        if sublines.is_empty() {
            return (0, col);
        }

        for (i, sub) in sublines.iter().enumerate() {
            if col >= sub.char_start && col < sub.char_end {
                return (i, col - sub.char_start);
            }
        }

        // Clamped to the last subline if col is at or beyond the line's end
        let last_idx = sublines.len() - 1;
        let last = &sublines[last_idx];
        let vcol = col.saturating_sub(last.char_start);
        (last_idx, vcol)
    }

    /// Converts `(subline_index, visual_col)` back into a physical character column.
    pub fn visual_to_col(subline_idx: usize, visual_col: usize, sublines: &[SublineInfo]) -> usize {
        if sublines.is_empty() {
            return visual_col;
        }

        let idx = subline_idx.min(sublines.len() - 1);
        let sub = &sublines[idx];
        let sub_len = sub.char_end.saturating_sub(sub.char_start);
        sub.char_start + visual_col.min(sub_len)
    }

    /// Moves all cursors up by one visual row.
    pub fn move_up(cursors: &mut CursorSet, buffer: &TextBuffer, max_cols: usize, select: bool) {
        for c in cursors.cursors_mut() {
            let pos = buffer.char_to_position(c.head);
            let line_cow = buffer.line_cow(pos.line).unwrap_or(std::borrow::Cow::Borrowed(""));
            let sublines = Self::compute_sublines(&line_cow, max_cols);
            let (sub_idx, vcol) = Self::col_to_visual(pos.col, &sublines);

            if sub_idx > 0 {
                // Move up within the same physical line to previous subline
                let target_sub_idx = sub_idx - 1;
                let target_sub = &sublines[target_sub_idx];
                let target_sub_len = target_sub.char_end.saturating_sub(target_sub.char_start);
                let new_vcol = min(vcol, target_sub_len);
                let new_col = target_sub.char_start + new_vcol;
                let target_char = buffer.position_to_char(Position::new(pos.line, new_col));
                c.head = target_char;
                if !select {
                    c.anchor = target_char;
                }
            } else if pos.line > 0 {
                // Move to previous physical line's last subline
                let target_line = pos.line - 1;
                let prev_cow = buffer.line_cow(target_line).unwrap_or(std::borrow::Cow::Borrowed(""));
                let prev_sublines = Self::compute_sublines(&prev_cow, max_cols);
                let last_sub_idx = prev_sublines.len().saturating_sub(1);
                let last_sub = &prev_sublines[last_sub_idx];
                let last_sub_len = last_sub.char_end.saturating_sub(last_sub.char_start);
                let new_vcol = min(vcol, last_sub_len);
                let new_col = last_sub.char_start + new_vcol;
                let target_char = buffer.position_to_char(Position::new(target_line, new_col));
                c.head = target_char;
                if !select {
                    c.anchor = target_char;
                }
            } else {
                // At line 0, subline 0: move to start of line
                let target_char = buffer.position_to_char(Position::new(0, 0));
                c.head = target_char;
                if !select {
                    c.anchor = target_char;
                }
            }
        }
        cursors.normalize();
    }

    /// Moves all cursors down by one visual row.
    pub fn move_down(cursors: &mut CursorSet, buffer: &TextBuffer, max_cols: usize, select: bool) {
        let total_lines = buffer.len_lines();
        for c in cursors.cursors_mut() {
            let pos = buffer.char_to_position(c.head);
            let line_cow = buffer.line_cow(pos.line).unwrap_or(std::borrow::Cow::Borrowed(""));
            let sublines = Self::compute_sublines(&line_cow, max_cols);
            let (sub_idx, vcol) = Self::col_to_visual(pos.col, &sublines);

            if sub_idx + 1 < sublines.len() {
                // Move down within the same physical line to next subline
                let target_sub_idx = sub_idx + 1;
                let target_sub = &sublines[target_sub_idx];
                let target_sub_len = target_sub.char_end.saturating_sub(target_sub.char_start);
                let new_vcol = min(vcol, target_sub_len);
                let new_col = target_sub.char_start + new_vcol;
                let target_char = buffer.position_to_char(Position::new(pos.line, new_col));
                c.head = target_char;
                if !select {
                    c.anchor = target_char;
                }
            } else if pos.line + 1 < total_lines {
                // Move to next physical line's first subline (sub_idx = 0)
                let target_line = pos.line + 1;
                let next_cow = buffer.line_cow(target_line).unwrap_or(std::borrow::Cow::Borrowed(""));
                let next_sublines = Self::compute_sublines(&next_cow, max_cols);
                let first_sub = &next_sublines[0];
                let first_sub_len = first_sub.char_end.saturating_sub(first_sub.char_start);
                let new_vcol = min(vcol, first_sub_len);
                let new_col = first_sub.char_start + new_vcol;
                let target_char = buffer.position_to_char(Position::new(target_line, new_col));
                c.head = target_char;
                if !select {
                    c.anchor = target_char;
                }
            } else {
                // At last line, last subline: move to end of file
                let max_len = buffer.line_len(pos.line);
                let target_char = buffer.position_to_char(Position::new(pos.line, max_len));
                c.head = target_char;
                if !select {
                    c.anchor = target_char;
                }
            }
        }
        cursors.normalize();
    }

    /// Moves cursors to the start of the current visual subline (or physical line if already at subline start).
    pub fn move_to_line_start(cursors: &mut CursorSet, buffer: &TextBuffer, max_cols: usize, select: bool) {
        for c in cursors.cursors_mut() {
            let pos = buffer.char_to_position(c.head);
            let line_cow = buffer.line_cow(pos.line).unwrap_or(std::borrow::Cow::Borrowed(""));
            let sublines = Self::compute_sublines(&line_cow, max_cols);
            let (sub_idx, _vcol) = Self::col_to_visual(pos.col, &sublines);
            let sub_start = sublines[sub_idx].char_start;

            let target_col = if pos.col > sub_start {
                sub_start
            } else if sub_idx > 0 {
                0
            } else {
                0
            };

            let target_char = buffer.position_to_char(Position::new(pos.line, target_col));
            c.head = target_char;
            if !select {
                c.anchor = target_char;
            }
        }
        cursors.normalize();
    }

    /// Moves cursors to the end of the current visual subline (or physical line if already at subline end).
    pub fn move_to_line_end(cursors: &mut CursorSet, buffer: &TextBuffer, max_cols: usize, select: bool) {
        for c in cursors.cursors_mut() {
            let pos = buffer.char_to_position(c.head);
            let line_cow = buffer.line_cow(pos.line).unwrap_or(std::borrow::Cow::Borrowed(""));
            let sublines = Self::compute_sublines(&line_cow, max_cols);
            let (sub_idx, _vcol) = Self::col_to_visual(pos.col, &sublines);
            let sub_end = sublines[sub_idx].char_end;
            let full_len = buffer.line_len(pos.line);

            let target_col = if pos.col < sub_end {
                sub_end
            } else {
                full_len
            };

            let target_char = buffer.position_to_char(Position::new(pos.line, target_col));
            c.head = target_char;
            if !select {
                c.anchor = target_char;
            }
        }
        cursors.normalize();
    }

    /// Adjusts viewport vertical scrolling so that the primary cursor's visual row is within the view.
    pub fn adjust_viewport(doc: &mut Document, visible_lines: usize, max_cols: usize) {
        if visible_lines == 0 {
            return;
        }

        let primary_head = doc.cursors.primary().head;
        let primary_pos = doc.buffer.char_to_position(primary_head);
        let cursor_line = primary_pos.line;

        // If cursor line is above current top line, scroll up immediately
        if cursor_line < doc.scroll_row {
            doc.scroll_row = cursor_line;
            doc.scroll_subline = 0;
            return;
        }

        // Cursor line is at or below doc.scroll_row
        let cursor_cow = doc.buffer.line_cow(cursor_line).unwrap_or(std::borrow::Cow::Borrowed(""));
        let cursor_sublines = Self::compute_sublines(&cursor_cow, max_cols);
        let (cursor_sub_idx, _) = Self::col_to_visual(primary_pos.col, &cursor_sublines);

        if cursor_line == doc.scroll_row {
            if cursor_sub_idx < doc.scroll_subline {
                doc.scroll_subline = cursor_sub_idx;
            } else if cursor_sub_idx >= doc.scroll_subline + visible_lines {
                doc.scroll_subline = cursor_sub_idx - visible_lines + 1;
            }
            return;
        }

        // Count visual rows from (doc.scroll_row, doc.scroll_subline) to (cursor_line, cursor_sub_idx)
        let mut visual_rows_between = 0;

        // First, remaining sublines on doc.scroll_row
        let top_cow = doc.buffer.line_cow(doc.scroll_row).unwrap_or(std::borrow::Cow::Borrowed(""));
        let top_sublines = Self::compute_sublines(&top_cow, max_cols);
        if doc.scroll_subline < top_sublines.len() {
            visual_rows_between += top_sublines.len() - doc.scroll_subline;
        }

        // Lines in between
        for l in (doc.scroll_row + 1)..cursor_line {
            let mid_cow = doc.buffer.line_cow(l).unwrap_or(std::borrow::Cow::Borrowed(""));
            let mid_sub = Self::compute_sublines(&mid_cow, max_cols);
            visual_rows_between += mid_sub.len();
            if visual_rows_between >= visible_lines + 50 {
                break;
            }
        }

        visual_rows_between += cursor_sub_idx;

        if visual_rows_between >= visible_lines {
            // Cursor is below visible area: scroll down so cursor row is at the bottom
            // Walk backwards from (cursor_line, cursor_sub_idx) by visible_lines - 1 rows
            let mut rows_to_rewind = visible_lines.saturating_sub(1);
            let mut cur_l = cursor_line;
            let mut cur_s = cursor_sub_idx;

            while rows_to_rewind > 0 {
                if cur_s > 0 {
                    let take = min(cur_s, rows_to_rewind);
                    cur_s -= take;
                    rows_to_rewind -= take;
                } else if cur_l > 0 {
                    cur_l -= 1;
                    let prev_cow = doc.buffer.line_cow(cur_l).unwrap_or(std::borrow::Cow::Borrowed(""));
                    let prev_sub = Self::compute_sublines(&prev_cow, max_cols);
                    let last_s = prev_sub.len().saturating_sub(1);
                    cur_s = last_s;
                    rows_to_rewind -= 1;
                } else {
                    break;
                }
            }

            doc.scroll_row = cur_l;
            doc.scroll_subline = cur_s;
        }
    }
}
