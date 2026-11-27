//! Mouse event processing and coordinate mapping.
//! Zero third-party dependencies, pure standard library Rust.

use crate::core::buffer::Position;
use super::app::Editor;
use super::document::Document;

impl Editor {
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
            let target_line = doc.folding.visible_line_for_screen_row(doc.scroll_row, row, total_lines)?;
            let col = if x >= text_start_x {
                (x - text_start_x) / char_w + doc.scroll_col
            } else {
                0
            };
            let pos = Position::new(target_line, col);
            Some(doc.buffer.position_to_char(pos))
        } else {
            let mut current_vrow = 0;
            let mut cur_line = doc.scroll_row;
            while cur_line > 0 && doc.folding.is_line_hidden(cur_line) {
                cur_line = doc.folding.prev_visible_line(cur_line);
            }
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
                let next_line = doc.folding.next_visible_line(cur_line, total_lines);
                if next_line <= cur_line {
                    break;
                }
                cur_line = next_line;
                cur_sub = 0;
            }
            None
        }
    }

    /// Handles mouse clicks on the editor canvas: toggling folds from gutter chevrons,
    /// expanding blocks via the inline collapsed badge, or positioning cursors.
    pub fn handle_code_canvas_click(
        &mut self,
        row: usize,
        x: usize,
        text_start_x: usize,
        char_w: usize,
        code_cols: usize,
    ) -> (bool, bool) {
        // 0. Sticky Scroll header jump: clicking a pinned scope header jumps directly to that line
        if self.config.editor.sticky_scroll && self.doc().scroll_row > 0 {
            let sticky_lines = self.doc().folding.sticky_lines(self.doc().scroll_row, self.config.editor.max_sticky_lines);
            if row < sticky_lines.len() {
                let target_line = sticky_lines[row];
                let char_offset = self.doc().buffer.position_to_char(crate::core::buffer::Position::new(target_line, 0));
                self.doc_mut().cursors.set_single(char_offset);
                self.doc_mut().scroll_row = target_line;
                self.set_status(format!("Saltó al encabezado fijo en línea {}", target_line + 1));
                return (true, true);
            }
        }

        let total_lines = self.doc().buffer.len_lines();
        let target_line = if !self.word_wrap {
            self.doc().folding.visible_line_for_screen_row(self.doc().scroll_row, row, total_lines)
        } else {
            let mut current_vrow = 0;
            let mut cur_line = self.doc().scroll_row;
            while cur_line > 0 && self.doc().folding.is_line_hidden(cur_line) {
                cur_line = self.doc().folding.prev_visible_line(cur_line);
            }
            let mut found = None;
            while cur_line < total_lines && current_vrow <= row {
                let cow = self.doc().buffer.line_cow(cur_line).unwrap_or(std::borrow::Cow::Borrowed(""));
                let sublines = crate::editor::wrap::WordWrap::compute_sublines(&cow, code_cols);
                if row < current_vrow + sublines.len() {
                    found = Some(cur_line);
                    break;
                }
                current_vrow += sublines.len();
                let next_l = self.doc().folding.next_visible_line(cur_line, total_lines);
                if next_l <= cur_line {
                    break;
                }
                cur_line = next_l;
            }
            found
        };

        if let Some(tl) = target_line {
            // 1. Gutter fold chevron click
            if x < text_start_x {
                if self.doc().folding.is_fold_start(tl).is_some() {
                    self.doc_mut().folding.toggle_fold_at_line(tl);
                    self.set_status(format!("Plegado alternado en línea {}", tl + 1));
                    return (false, true);
                }
            } else if self.doc().folding.fold_end_for_line(tl).is_some() {
                // 2. Collapsed badge " ··· " click
                let line_len = self.doc().buffer.line_len(tl);
                let badge_x = text_start_x + line_len.saturating_sub(self.doc().scroll_col) * char_w;
                if x >= badge_x && x <= badge_x + char_w * 8 {
                    self.doc_mut().folding.toggle_fold_at_line(tl);
                    self.set_status(format!("Bloque desplegado en línea {}", tl + 1));
                    return (false, true);
                }
            }
        }

        // 3. Normal text click
        if let Some(char_offset) = Self::find_click_position(self.doc(), row, x, text_start_x, char_w, self.word_wrap, code_cols) {
            self.doc_mut().cursors.set_single(char_offset);
            (true, true)
        } else {
            (false, false)
        }
    }
}
