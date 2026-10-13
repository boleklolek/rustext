//! Screen renderer with double-buffered atomic flushing and viewport scrolling.
//! Zero third-party dependencies, purely standard library Rust.

use std::cmp::max;
use std::io::{self, Write};

use super::terminal::TerminalGuard;
use super::theme::Theme;
use crate::core::buffer::TextBuffer;
use crate::core::cursor::CursorSet;
use crate::core::syntax::{Language, SyntaxHighlighter};

pub struct Screen {
    pub rows: usize,
    pub cols: usize,
    pub scroll_row: usize,
    pub scroll_col: usize,
    pub has_tab_bar: bool,
    pub theme: Theme,
    draw_buffer: String,
}

impl Screen {
    /// Creates a new Screen with detected terminal dimensions.
    pub fn new() -> Self {
        let (rows, cols) = TerminalGuard::size();
        Self {
            rows,
            cols,
            scroll_row: 0,
            scroll_col: 0,
            has_tab_bar: true,
            theme: Theme::modern_dark(),
            draw_buffer: String::with_capacity(rows * cols * 4),
        }
    }

    /// Refreshes terminal dimensions.
    pub fn update_dimensions(&mut self) {
        let (rows, cols) = TerminalGuard::size();
        self.rows = rows;
        self.cols = cols;
    }

    /// Available height for document text (excluding tab bar, status, and prompt lines).
    pub fn text_area_height(&self) -> usize {
        if self.has_tab_bar {
            self.rows.saturating_sub(3)
        } else {
            self.rows.saturating_sub(2)
        }
    }

    /// Adjusts scroll offsets so that the cursor position is within the visible viewport.
    pub fn adjust_viewport(&mut self, cursor_line: usize, cursor_col: usize, gutter_width: usize) {
        let height = self.text_area_height();
        let width = self.cols.saturating_sub(gutter_width + 1);

        // Vertical scroll
        if cursor_line < self.scroll_row {
            self.scroll_row = cursor_line;
        } else if cursor_line >= self.scroll_row + height {
            self.scroll_row = cursor_line - height + 1;
        }

        // Horizontal scroll
        if cursor_col < self.scroll_col {
            self.scroll_col = cursor_col;
        } else if cursor_col >= self.scroll_col + width {
            self.scroll_col = cursor_col - width + 1;
        }
    }

    /// Clears the drawing buffer and moves to top-left.
    pub fn begin_frame(&mut self) {
        self.draw_buffer.clear();
        // Hide cursor while drawing to prevent flicker
        self.draw_buffer.push_str("\x1b[?25l\x1b[H");
    }

    /// Flushes the complete frame to stdout and restores cursor.
    pub fn end_frame(&mut self, cursor_screen_row: usize, cursor_screen_col: usize) -> io::Result<()> {
        // Move cursor to active position and show it
        let move_cursor = format!("\x1b[{};{}H\x1b[?25h", cursor_screen_row + 1, cursor_screen_col + 1);
        self.draw_buffer.push_str(&move_cursor);

        let mut stdout = io::stdout();
        stdout.write_all(self.draw_buffer.as_bytes())?;
        stdout.flush()?;
        Ok(())
    }

    /// Calculates gutter width based on total lines.
    pub fn calculate_gutter_width(total_lines: usize) -> usize {
        let digits = total_lines.to_string().len();
        max(digits, 3) + 2 // padding and divider
    }

    /// Renders the document view with line numbers, syntax highlighting, selections, and cursors.
    pub fn render_document(
        &mut self,
        buffer: &TextBuffer,
        cursor_set: &CursorSet,
        lang: Language,
    ) {
        let total_lines = buffer.len_lines();
        let gutter_width = Self::calculate_gutter_width(total_lines);
        let text_height = self.text_area_height();

        // Collect cursor positions and selections for fast checks
        let cursors = cursor_set.cursors();
        let mut cursor_positions = Vec::new();
        let mut selections = Vec::new();

        for c in cursors {
            let pos = buffer.char_to_position(c.head);
            cursor_positions.push(pos);
            if c.has_selection() {
                let range = c.selection_range();
                let start_pos = buffer.char_to_position(range.start);
                let end_pos = buffer.char_to_position(range.end);
                selections.push((start_pos, end_pos));
            }
        }

        for screen_row in 0..text_height {
            let doc_line = self.scroll_row + screen_row;

            if doc_line < total_lines {
                // 1. Line numbers gutter
                let line_num_str = format!("{:>width$} │", doc_line + 1, width = gutter_width - 2);
                self.draw_buffer.push_str(self.theme.line_number_bg);
                self.draw_buffer.push_str(self.theme.line_number_fg);
                self.draw_buffer.push_str(&line_num_str);
                self.draw_buffer.push_str(self.theme.bg_reset);
                self.draw_buffer.push_str(self.theme.fg_reset);

                // 2. Line content & syntax highlighting
                let line_str = buffer.line(doc_line).unwrap_or_default();
                let available_cols = self.cols.saturating_sub(gutter_width);

                // Tokenize line
                let spans = SyntaxHighlighter::tokenize_line(&line_str, lang);

                // Render characters with highlighting and selection checks
                let mut col_offset = 0;
                let mut rendered_cols = 0;

                for span in spans {
                    for ch in span.text.chars() {
                        let doc_col = col_offset;
                        col_offset += 1;

                        if doc_col < self.scroll_col {
                            continue;
                        }

                        if rendered_cols >= available_cols {
                            break;
                        }

                        // Check if inside selection
                        let in_selection = selections.iter().any(|(start, end)| {
                            if doc_line > start.line && doc_line < end.line {
                                true
                            } else if doc_line == start.line && doc_line == end.line {
                                doc_col >= start.col && doc_col < end.col
                            } else if doc_line == start.line {
                                doc_col >= start.col
                            } else if doc_line == end.line {
                                doc_col < end.col
                            } else {
                                false
                            }
                        });

                        // Check if a secondary cursor is at this cell
                        let is_cursor = cursor_positions
                            .iter()
                            .any(|p| p.line == doc_line && p.col == doc_col);

                        if is_cursor {
                            self.draw_buffer.push_str(self.theme.cursor_style);
                        } else if in_selection {
                            self.draw_buffer.push_str(self.theme.selection_bg);
                        } else {
                            self.draw_buffer.push_str(span.token_type.ansi_code());
                        }

                        self.draw_buffer.push(ch);
                        self.draw_buffer.push_str("\x1b[0m"); // Reset styling
                        rendered_cols += 1;
                    }
                }

                // Fill remaining space on this line
                if rendered_cols < available_cols {
                    let fill_count = available_cols - rendered_cols;
                    self.draw_buffer.push_str(&" ".repeat(fill_count));
                }
            } else {
                // Empty lines past end of document: display '~'
                let tilde = format!("~{:width$}", "", width = self.cols.saturating_sub(1));
                self.draw_buffer.push_str("\x1b[38;5;240m");
                self.draw_buffer.push_str(&tilde);
                self.draw_buffer.push_str("\x1b[0m");
            }

            self.draw_buffer.push_str("\r\n");
        }
    }

    /// Renders the top tab bar displaying open buffers.
    pub fn render_tab_bar(&mut self, tab_titles: &[String], active_idx: usize) {
        self.draw_buffer.push_str("\x1b[48;5;234m"); // Tab bar background

        let mut rendered_len = 0;
        for (i, title) in tab_titles.iter().enumerate() {
            let is_active = i == active_idx;
            let tab_str = if is_active {
                format!("\x1b[48;5;238m\x1b[1;37m {} {} \x1b[48;5;234m", i + 1, title)
            } else {
                format!("\x1b[38;5;244m {} {} ", i + 1, title)
            };
            self.draw_buffer.push_str(&tab_str);
            rendered_len += 4 + title.chars().count();
        }

        let padding = self.cols.saturating_sub(rendered_len);
        self.draw_buffer.push_str(&" ".repeat(padding));
        self.draw_buffer.push_str("\x1b[0m\r\n");
    }

    /// Renders the editor status bar.
    pub fn render_status_bar(
        &mut self,
        mode_name: &str,
        file_name: &str,
        is_dirty: bool,
        cursor_pos: (usize, usize),
        cursor_count: usize,
        total_lines: usize,
        total_chars: usize,
        lang_name: &str,
    ) {
        self.draw_buffer.push_str(self.theme.status_bar_bg);
        self.draw_buffer.push_str(self.theme.status_bar_fg);

        let dirty_flag = if is_dirty { " [+]" } else { "" };
        let mode_tag = format!(" [{}] ", mode_name);
        let left = format!("{}{}{}", mode_tag, file_name, dirty_flag);

        let cursors_tag = if cursor_count > 1 {
            format!(" [{} cursors]", cursor_count)
        } else {
            String::new()
        };

        let right = format!(
            "{} │ Ln {}, Col {} │ {}L, {}C │ {} ",
            lang_name,
            cursor_pos.0 + 1,
            cursor_pos.1 + 1,
            total_lines,
            total_chars,
            cursors_tag
        );

        let total_content_len = left.chars().count() + right.chars().count();
        let padding = if self.cols > total_content_len {
            self.cols - total_content_len
        } else {
            0
        };

        self.draw_buffer.push_str(&left);
        self.draw_buffer.push_str(&" ".repeat(padding));
        self.draw_buffer.push_str(&right);
        self.draw_buffer.push_str("\x1b[0m\r\n");
    }

    /// Renders the prompt line or message area at the bottom.
    pub fn render_prompt_line(&mut self, content: &str) {
        let max_len = self.cols;
        let truncated: String = content.chars().take(max_len).collect();
        let padding = max_len.saturating_sub(truncated.chars().count());

        self.draw_buffer.push_str("\x1b[38;5;250m");
        self.draw_buffer.push_str(&truncated);
        self.draw_buffer.push_str(&" ".repeat(padding));
        self.draw_buffer.push_str("\x1b[0m");
    }
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}
