//! Ergonomic line editing operations (move, duplicate, delete, copy, comment).
//! Zero third-party dependencies, pure standard library Rust.

use crate::core::buffer::Position;
use crate::core::cursor::Cursor;
use crate::core::history::{EditOp, Transaction};
use super::app::Editor;

impl Editor {
    // ========================================================================
    // Ergonomics & Advanced Editing
    // ========================================================================

    /// Moves the current line or selected line range(s) up by one row (Alt+Up).
    pub fn move_lines_up(&mut self) {
        let (ranges, _total_lines) = {
            let doc = self.doc();
            let total = doc.buffer.len_lines();
            if total <= 1 {
                return;
            }
            let mut line_ranges = Vec::new();
            for c in doc.cursors.cursors() {
                let (start_off, end_off) = if c.has_selection() {
                    let r = c.selection_range();
                    (r.start, r.end)
                } else {
                    (c.head, c.head)
                };
                let p1 = doc.buffer.char_to_position(start_off);
                let p2 = doc.buffer.char_to_position(end_off);
                let start_line = p1.line;
                let end_line = if p2.line > p1.line && p2.col == 0 {
                    p2.line - 1
                } else {
                    p2.line
                };
                line_ranges.push((start_line, end_line));
            }
            (line_ranges, total)
        };

        let mut merged: Vec<(usize, usize)> = Vec::new();
        let mut sorted = ranges;
        sorted.sort_by_key(|&(s, _)| s);
        for (s, e) in sorted {
            if let Some(last) = merged.last_mut() {
                if s <= last.1 + 1 {
                    last.1 = last.1.max(e);
                    continue;
                }
            }
            merged.push((s, e));
        }

        if let Some(&(first_s, _)) = merged.first() {
            if first_s == 0 {
                return;
            }
        }

        let doc = self.doc_mut();
        let cursors_before = doc.cursors.cursors().to_vec();
        let mut tx = Transaction::new(cursors_before.clone());

        for &(start_line, end_line) in &merged {
            let above_line_idx = start_line - 1;
            let above_text = doc.buffer.line(above_line_idx).unwrap_or_default();
            let mut block_lines: Vec<String> = Vec::new();
            for l in start_line..=end_line {
                block_lines.push(doc.buffer.line(l).unwrap_or_default());
            }

            let start_char = doc.buffer.position_to_char(crate::core::buffer::Position::new(above_line_idx, 0));
            let (end_char, had_trailing_newline) = if end_line + 1 < doc.buffer.len_lines() {
                (doc.buffer.position_to_char(crate::core::buffer::Position::new(end_line + 1, 0)), true)
            } else {
                let len = doc.buffer.len_chars();
                let ends_nl = doc.buffer.char_at(len.saturating_sub(1)) == Some('\n');
                (len, ends_nl)
            };

            let old_text = doc.buffer.get_text_range(start_char, end_char);
            doc.buffer.delete_range(start_char, end_char);
            tx.ops.push(EditOp::Delete { offset: start_char, text: old_text });

            let mut new_text = block_lines.join("\n");
            new_text.push('\n');
            new_text.push_str(&above_text);
            if had_trailing_newline {
                new_text.push('\n');
            }

            doc.buffer.insert(start_char, &new_text);
            tx.ops.push(EditOp::Insert { offset: start_char, text: new_text });
        }

        let mut new_cursors = Vec::with_capacity(cursors_before.len());
        for c in &cursors_before {
            let p_head = doc.buffer.char_to_position(c.head);
            let p_anc = doc.buffer.char_to_position(c.anchor);
            let new_head_line = p_head.line.saturating_sub(1);
            let new_anc_line = p_anc.line.saturating_sub(1);
            let h_len = doc.buffer.line_len(new_head_line);
            let a_len = doc.buffer.line_len(new_anc_line);
            let new_h_col = p_head.col.min(h_len);
            let new_a_col = p_anc.col.min(a_len);
            let h_off = doc.buffer.position_to_char(crate::core::buffer::Position::new(new_head_line, new_h_col));
            let a_off = doc.buffer.position_to_char(crate::core::buffer::Position::new(new_anc_line, new_a_col));
            let mut nc = crate::core::cursor::Cursor::new(h_off);
            nc.anchor = a_off;
            new_cursors.push(nc);
        }
        doc.cursors.set_cursors(new_cursors);
        tx.cursors_after = doc.cursors.cursors().to_vec();
        doc.history.record(tx);
        self.lsp_notify_change_active_doc();
    }

    /// Moves the current line or selected line range(s) down by one row (Alt+Down).
    pub fn move_lines_down(&mut self) {
        let (ranges, total_lines) = {
            let doc = self.doc();
            let total = doc.buffer.len_lines();
            if total <= 1 {
                return;
            }
            let mut line_ranges = Vec::new();
            for c in doc.cursors.cursors() {
                let (start_off, end_off) = if c.has_selection() {
                    let r = c.selection_range();
                    (r.start, r.end)
                } else {
                    (c.head, c.head)
                };
                let p1 = doc.buffer.char_to_position(start_off);
                let p2 = doc.buffer.char_to_position(end_off);
                let start_line = p1.line;
                let end_line = if p2.line > p1.line && p2.col == 0 {
                    p2.line - 1
                } else {
                    p2.line
                };
                line_ranges.push((start_line, end_line));
            }
            (line_ranges, total)
        };

        let mut merged: Vec<(usize, usize)> = Vec::new();
        let mut sorted = ranges;
        sorted.sort_by_key(|&(s, _)| s);
        for (s, e) in sorted {
            if let Some(last) = merged.last_mut() {
                if s <= last.1 + 1 {
                    last.1 = last.1.max(e);
                    continue;
                }
            }
            merged.push((s, e));
        }

        if let Some(&(_, last_e)) = merged.last() {
            if last_e + 1 >= total_lines {
                return;
            }
        }

        let doc = self.doc_mut();
        let cursors_before = doc.cursors.cursors().to_vec();
        let mut tx = Transaction::new(cursors_before.clone());

        for &(start_line, end_line) in merged.iter().rev() {
            let below_line_idx = end_line + 1;
            let below_text = doc.buffer.line(below_line_idx).unwrap_or_default();
            let mut block_lines: Vec<String> = Vec::new();
            for l in start_line..=end_line {
                block_lines.push(doc.buffer.line(l).unwrap_or_default());
            }

            let start_char = doc.buffer.position_to_char(crate::core::buffer::Position::new(start_line, 0));
            let (end_char, had_trailing_newline) = if below_line_idx + 1 < doc.buffer.len_lines() {
                (doc.buffer.position_to_char(crate::core::buffer::Position::new(below_line_idx + 1, 0)), true)
            } else {
                let len = doc.buffer.len_chars();
                let ends_nl = doc.buffer.char_at(len.saturating_sub(1)) == Some('\n');
                (len, ends_nl)
            };

            let old_text = doc.buffer.get_text_range(start_char, end_char);
            doc.buffer.delete_range(start_char, end_char);
            tx.ops.push(EditOp::Delete { offset: start_char, text: old_text });

            let mut new_text = below_text;
            new_text.push('\n');
            new_text.push_str(&block_lines.join("\n"));
            if had_trailing_newline {
                new_text.push('\n');
            }

            doc.buffer.insert(start_char, &new_text);
            tx.ops.push(EditOp::Insert { offset: start_char, text: new_text });
        }

        let mut new_cursors = Vec::with_capacity(cursors_before.len());
        for c in &cursors_before {
            let p_head = doc.buffer.char_to_position(c.head);
            let p_anc = doc.buffer.char_to_position(c.anchor);
            let new_head_line = (p_head.line + 1).min(doc.buffer.len_lines().saturating_sub(1));
            let new_anc_line = (p_anc.line + 1).min(doc.buffer.len_lines().saturating_sub(1));
            let h_len = doc.buffer.line_len(new_head_line);
            let a_len = doc.buffer.line_len(new_anc_line);
            let new_h_col = p_head.col.min(h_len);
            let new_a_col = p_anc.col.min(a_len);
            let h_off = doc.buffer.position_to_char(crate::core::buffer::Position::new(new_head_line, new_h_col));
            let a_off = doc.buffer.position_to_char(crate::core::buffer::Position::new(new_anc_line, new_a_col));
            let mut nc = crate::core::cursor::Cursor::new(h_off);
            nc.anchor = a_off;
            new_cursors.push(nc);
        }
        doc.cursors.set_cursors(new_cursors);
        tx.cursors_after = doc.cursors.cursors().to_vec();
        doc.history.record(tx);
        self.lsp_notify_change_active_doc();
    }

    /// Duplicates current line(s) immediately below (Shift+Alt+Down / Ctrl+Shift+D).
    pub fn duplicate_lines_down(&mut self) {
        let ranges = {
            let doc = self.doc();
            let mut line_ranges = Vec::new();
            for c in doc.cursors.cursors() {
                let (start_off, end_off) = if c.has_selection() {
                    let r = c.selection_range();
                    (r.start, r.end)
                } else {
                    (c.head, c.head)
                };
                let p1 = doc.buffer.char_to_position(start_off);
                let p2 = doc.buffer.char_to_position(end_off);
                let start_line = p1.line;
                let end_line = if p2.line > p1.line && p2.col == 0 {
                    p2.line - 1
                } else {
                    p2.line
                };
                line_ranges.push((start_line, end_line));
            }
            line_ranges
        };

        let mut merged: Vec<(usize, usize)> = Vec::new();
        let mut sorted = ranges;
        sorted.sort_by_key(|&(s, _)| s);
        for (s, e) in sorted {
            if let Some(last) = merged.last_mut() {
                if s <= last.1 + 1 {
                    last.1 = last.1.max(e);
                    continue;
                }
            }
            merged.push((s, e));
        }

        let doc = self.doc_mut();
        let cursors_before = doc.cursors.cursors().to_vec();
        let mut tx = Transaction::new(cursors_before.clone());

        let mut line_shift: usize = 0;
        for &(start_line, end_line) in merged.iter().rev() {
            let mut block_lines: Vec<String> = Vec::new();
            for l in start_line..=end_line {
                block_lines.push(doc.buffer.line(l).unwrap_or_default());
            }
            let block_text = block_lines.join("\n");
            let count_lines = end_line - start_line + 1;
            line_shift += count_lines;

            if end_line + 1 < doc.buffer.len_lines() {
                let insert_char = doc.buffer.position_to_char(crate::core::buffer::Position::new(end_line + 1, 0));
                let insert_str = format!("{}\n", block_text);
                doc.buffer.insert(insert_char, &insert_str);
                tx.ops.push(EditOp::Insert { offset: insert_char, text: insert_str });
            } else {
                let insert_char = doc.buffer.len_chars();
                let insert_str = format!("\n{}", block_text);
                doc.buffer.insert(insert_char, &insert_str);
                tx.ops.push(EditOp::Insert { offset: insert_char, text: insert_str });
            }
        }

        let mut new_cursors = Vec::with_capacity(cursors_before.len());
        for c in &cursors_before {
            let p_head = doc.buffer.char_to_position(c.head);
            let p_anc = doc.buffer.char_to_position(c.anchor);
            let new_h_line = (p_head.line + line_shift).min(doc.buffer.len_lines().saturating_sub(1));
            let new_a_line = (p_anc.line + line_shift).min(doc.buffer.len_lines().saturating_sub(1));
            let h_len = doc.buffer.line_len(new_h_line);
            let a_len = doc.buffer.line_len(new_a_line);
            let new_h_col = p_head.col.min(h_len);
            let new_a_col = p_anc.col.min(a_len);
            let h_off = doc.buffer.position_to_char(crate::core::buffer::Position::new(new_h_line, new_h_col));
            let a_off = doc.buffer.position_to_char(crate::core::buffer::Position::new(new_a_line, new_a_col));
            let mut nc = crate::core::cursor::Cursor::new(h_off);
            nc.anchor = a_off;
            new_cursors.push(nc);
        }
        doc.cursors.set_cursors(new_cursors);
        tx.cursors_after = doc.cursors.cursors().to_vec();
        doc.history.record(tx);
        self.lsp_notify_change_active_doc();
    }

    /// Deletes the entire line(s) spanned by each cursor (Vim `dd`).
    pub fn delete_lines(&mut self) {
        let ranges = {
            let doc = self.doc();
            let mut line_ranges = Vec::new();
            for c in doc.cursors.cursors() {
                let (start_off, end_off) = if c.has_selection() {
                    let r = c.selection_range();
                    (r.start, r.end)
                } else {
                    (c.head, c.head)
                };
                let p1 = doc.buffer.char_to_position(start_off);
                let p2 = doc.buffer.char_to_position(end_off);
                let start_line = p1.line;
                let end_line = if p2.line > p1.line && p2.col == 0 {
                    p2.line.saturating_sub(1)
                } else {
                    p2.line
                };
                line_ranges.push((start_line, end_line));
            }
            line_ranges
        };

        let mut merged: Vec<(usize, usize)> = Vec::new();
        let mut sorted = ranges;
        sorted.sort_by_key(|&(s, _)| s);
        for (s, e) in sorted {
            if let Some(last) = merged.last_mut() {
                if s <= last.1 + 1 {
                    last.1 = last.1.max(e);
                    continue;
                }
            }
            merged.push((s, e));
        }

        let doc = self.doc_mut();
        let mut tx = Transaction::new(doc.cursors.cursors().to_vec());
        let total_lines = doc.buffer.len_lines();

        let mut cut_lines = Vec::new();
        for &(start_line, end_line) in merged.iter().rev() {
            let safe_start = start_line.min(total_lines.saturating_sub(1));
            let safe_end = end_line.min(total_lines.saturating_sub(1));
            let start_char = doc.buffer.position_to_char(Position::new(safe_start, 0));
            let end_char = if safe_end + 1 < total_lines {
                doc.buffer.position_to_char(Position::new(safe_end + 1, 0))
            } else {
                doc.buffer.len_chars()
            };

            if start_char < end_char {
                let deleted = doc.buffer.get_text_range(start_char, end_char);
                cut_lines.push(deleted.clone());
                doc.buffer.delete_range(start_char, end_char);
                tx.ops.push(EditOp::Delete { offset: start_char, text: deleted });
            }
        }

        if !cut_lines.is_empty() {
            cut_lines.reverse();
            let clipboard_text = cut_lines.join("");
            self.clipboard.set_text(&clipboard_text);
        }

        let doc = self.doc_mut();
        let remaining_lines = doc.buffer.len_lines();
        let mut new_cursors = Vec::new();
        for &(start_line, _) in &merged {
            let target_line = start_line.min(remaining_lines.saturating_sub(1));
            let target_char = doc.buffer.position_to_char(Position::new(target_line, 0));
            new_cursors.push(Cursor::new(target_char));
        }
        if new_cursors.is_empty() {
            new_cursors.push(Cursor::new(0));
        }
        doc.cursors.set_cursors(new_cursors);
        tx.cursors_after = doc.cursors.cursors().to_vec();
        doc.history.record(tx);
        self.lsp_notify_change_active_doc();
    }

    /// Copies the entire line(s) spanned by each cursor to the clipboard (Vim `yy`).
    pub fn copy_lines(&mut self) {
        let doc = self.doc();
        let mut line_ranges = Vec::new();
        for c in doc.cursors.cursors() {
            let (start_off, end_off) = if c.has_selection() {
                let r = c.selection_range();
                (r.start, r.end)
            } else {
                (c.head, c.head)
            };
            let p1 = doc.buffer.char_to_position(start_off);
            let p2 = doc.buffer.char_to_position(end_off);
            let start_line = p1.line;
            let end_line = if p2.line > p1.line && p2.col == 0 {
                p2.line.saturating_sub(1)
            } else {
                p2.line
            };
            line_ranges.push((start_line, end_line));
        }

        let mut merged: Vec<(usize, usize)> = Vec::new();
        line_ranges.sort_by_key(|&(s, _)| s);
        for (s, e) in line_ranges {
            if let Some(last) = merged.last_mut() {
                if s <= last.1 + 1 {
                    last.1 = last.1.max(e);
                    continue;
                }
            }
            merged.push((s, e));
        }

        let mut copied_parts = Vec::new();
        for (s, e) in merged {
            for l in s..=e {
                if let Some(line) = doc.buffer.line(l) {
                    copied_parts.push(line);
                }
            }
        }

        let full_text = copied_parts.join("\n") + "\n";
        self.clipboard.set_text(&full_text);
        self.set_status(format!("Copiadas {} líneas", copied_parts.len()));
    }

    /// Opens a newline below the current line and switches to Insert mode (Vim `o`).
    pub fn insert_line_below(&mut self) {
        {
            let doc = self.doc_mut();
            doc.cursors.move_to_line_end(&doc.buffer, false);
        }
        self.insert_newline_multicursor();
        self.keymap_engine.set_modal_mode(crate::editor::keymap_loader::ModalMode::Insert);
    }

    /// Opens a newline above the current line and switches to Insert mode (Vim `O`).
    pub fn insert_line_above(&mut self) {
        {
            let doc = self.doc_mut();
            doc.cursors.move_to_line_start(&doc.buffer, false);
        }
        let doc = self.doc_mut();
        let mut tx = Transaction::new(doc.cursors.cursors().to_vec());
        let cursors = doc.cursors.cursors().to_vec();
        let mut new_cursors = Vec::new();
        let mut shift = 0;

        for c in cursors {
            let adj_pos = c.head + shift;
            let p = doc.buffer.char_to_position(adj_pos);
            let indent: String = doc.buffer.line(p.line)
                .unwrap_or_default()
                .chars()
                .take_while(|ch| *ch == ' ' || *ch == '\t')
                .collect();
            let text_to_insert = format!("{}\n", indent);
            let insert_len = text_to_insert.chars().count();
            doc.buffer.insert(adj_pos, &text_to_insert);
            tx.ops.push(EditOp::Insert { offset: adj_pos, text: text_to_insert });

            let cursor_target = adj_pos + indent.chars().count();
            new_cursors.push(Cursor::new(cursor_target));
            shift += insert_len;
        }

        doc.cursors.set_cursors(new_cursors);
        tx.cursors_after = doc.cursors.cursors().to_vec();
        doc.history.record(tx);
        self.keymap_engine.set_modal_mode(crate::editor::keymap_loader::ModalMode::Insert);
        self.lsp_notify_change_active_doc();
    }

    /// Returns comment tokens `(prefix, suffix)` for the specified language.
    pub fn comment_tokens_for_language(lang: crate::core::syntax::Language) -> (&'static str, Option<&'static str>) {
        match lang {
            crate::core::syntax::Language::Python
            | crate::core::syntax::Language::Yaml
            | crate::core::syntax::Language::Toml
            | crate::core::syntax::Language::Shell
            | crate::core::syntax::Language::Julia => ("# ", None),
            crate::core::syntax::Language::Sql
            | crate::core::syntax::Language::Lua => ("-- ", None),
            crate::core::syntax::Language::Html
            | crate::core::syntax::Language::Markdown => ("<!-- ", Some(" -->")),
            crate::core::syntax::Language::Css => ("/* ", Some(" */")),
            _ => ("// ", None),
        }
    }

    /// Toggles line comments on the current line or selection(s) (Ctrl+/ / Ctrl+_).
    pub fn toggle_comment(&mut self) {
        let lang = self.doc().language;
        let (prefix, suffix) = Self::comment_tokens_for_language(lang);
        let prefix_trimmed = prefix.trim();
        let suffix_trimmed = suffix.map(|s| s.trim());

        let ranges = {
            let doc = self.doc();
            let mut line_ranges = Vec::new();
            for c in doc.cursors.cursors() {
                let (start_off, end_off) = if c.has_selection() {
                    let r = c.selection_range();
                    (r.start, r.end)
                } else {
                    (c.head, c.head)
                };
                let p1 = doc.buffer.char_to_position(start_off);
                let p2 = doc.buffer.char_to_position(end_off);
                let start_line = p1.line;
                let end_line = if p2.line > p1.line && p2.col == 0 {
                    p2.line - 1
                } else {
                    p2.line
                };
                line_ranges.push((start_line, end_line));
            }
            line_ranges
        };

        let mut merged: Vec<(usize, usize)> = Vec::new();
        let mut sorted = ranges;
        sorted.sort_by_key(|&(s, _)| s);
        for (s, e) in sorted {
            if let Some(last) = merged.last_mut() {
                if s <= last.1 + 1 {
                    last.1 = last.1.max(e);
                    continue;
                }
            }
            merged.push((s, e));
        }

        let doc = self.doc_mut();
        let cursors_before = doc.cursors.cursors().to_vec();
        let mut tx = Transaction::new(cursors_before.clone());

        for &(start_line, end_line) in merged.iter().rev() {
            let mut lines: Vec<String> = Vec::new();
            for l in start_line..=end_line {
                lines.push(doc.buffer.line(l).unwrap_or_default());
            }

            let non_empty_lines: Vec<&String> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
            let all_commented = !non_empty_lines.is_empty() && non_empty_lines.iter().all(|l| {
                let tr = l.trim_start();
                let starts = tr.starts_with(prefix_trimmed);
                let ends = match suffix_trimmed {
                    Some(suf) => l.trim_end().ends_with(suf),
                    None => true,
                };
                starts && ends
            });

            let mut new_lines = Vec::with_capacity(lines.len());
            if all_commented {
                for line in &lines {
                    let mut s = line.clone();
                    if s.trim().is_empty() {
                        new_lines.push(s);
                        continue;
                    }
                    if let Some(pos) = s.find(prefix) {
                        s.replace_range(pos..pos + prefix.len(), "");
                    } else if let Some(pos) = s.find(prefix_trimmed) {
                        s.replace_range(pos..pos + prefix_trimmed.len(), "");
                    }
                    if let Some(suf) = suffix {
                        if let Some(pos) = s.rfind(suf) {
                            s.replace_range(pos..pos + suf.len(), "");
                        } else if let Some(suf_tr) = suffix_trimmed {
                            if let Some(pos) = s.rfind(suf_tr) {
                                s.replace_range(pos..pos + suf_tr.len(), "");
                            }
                        }
                    }
                    new_lines.push(s);
                }
            } else {
                let min_indent = non_empty_lines
                    .iter()
                    .map(|l| l.chars().take_while(|c| c.is_whitespace()).count())
                    .min()
                    .unwrap_or(0);

                for line in &lines {
                    if line.trim().is_empty() {
                        new_lines.push(line.clone());
                        continue;
                    }
                    let indent = line.chars().take(min_indent).collect::<String>();
                    let rest = line.chars().skip(min_indent).collect::<String>();
                    let mut commented_line = format!("{}{}{}", indent, prefix, rest);
                    if let Some(suf) = suffix {
                        commented_line.push_str(suf);
                    }
                    new_lines.push(commented_line);
                }
            }

            let start_char = doc.buffer.position_to_char(crate::core::buffer::Position::new(start_line, 0));
            let (end_char, had_trailing_newline) = if end_line + 1 < doc.buffer.len_lines() {
                (doc.buffer.position_to_char(crate::core::buffer::Position::new(end_line + 1, 0)), true)
            } else {
                let len = doc.buffer.len_chars();
                let ends_nl = doc.buffer.char_at(len.saturating_sub(1)) == Some('\n');
                (len, ends_nl)
            };

            let old_text = doc.buffer.get_text_range(start_char, end_char);
            doc.buffer.delete_range(start_char, end_char);
            tx.ops.push(EditOp::Delete { offset: start_char, text: old_text });

            let mut new_text = new_lines.join("\n");
            if had_trailing_newline {
                new_text.push('\n');
            }
            doc.buffer.insert(start_char, &new_text);
            tx.ops.push(EditOp::Insert { offset: start_char, text: new_text });
        }

        let mut new_cursors = Vec::with_capacity(cursors_before.len());
        for c in &cursors_before {
            let p_head = doc.buffer.char_to_position(c.head);
            let p_anc = doc.buffer.char_to_position(c.anchor);
            let h_len = doc.buffer.line_len(p_head.line);
            let a_len = doc.buffer.line_len(p_anc.line);
            let h_off = doc.buffer.position_to_char(crate::core::buffer::Position::new(p_head.line, p_head.col.min(h_len)));
            let a_off = doc.buffer.position_to_char(crate::core::buffer::Position::new(p_anc.line, p_anc.col.min(a_len)));
            let mut nc = crate::core::cursor::Cursor::new(h_off);
            nc.anchor = a_off;
            new_cursors.push(nc);
        }
        doc.cursors.set_cursors(new_cursors);
        tx.cursors_after = doc.cursors.cursors().to_vec();
        doc.history.record(tx);
        self.lsp_notify_change_active_doc();
    }
}
