//! Graphical renderer translating editor state into framebuffer pixels.
//! Zero third-party dependencies, purely standard library Rust.

use super::font::{FONT_HEIGHT, FONT_WIDTH};
use super::framebuffer::Framebuffer;
use crate::core::syntax::{SyntaxHighlighter, TokenType};
use crate::editor::Document;

pub struct ThemeColors {
    pub background: u32,
    pub gutter_bg: u32,
    pub gutter_fg: u32,
    pub gutter_border: u32,
    pub tab_bar_bg: u32,
    pub tab_active_bg: u32,
    pub tab_inactive_bg: u32,
    pub tab_active_fg: u32,
    pub tab_inactive_fg: u32,
    pub status_bar_bg: u32,
    pub status_bar_fg: u32,
    pub selection_bg: u32,
    pub cursor: u32,
    // Syntax colors
    pub syntax_keyword: u32,
    pub syntax_type: u32,
    pub syntax_string: u32,
    pub syntax_number: u32,
    pub syntax_comment: u32,
    pub syntax_operator: u32,
    pub syntax_punctuation: u32,
    pub syntax_plain: u32,
}

impl ThemeColors {
    pub fn modern_dark() -> Self {
        Self {
            background: 0x001E1E1E,
            gutter_bg: 0x00252526,
            gutter_fg: 0x00858585,
            gutter_border: 0x00333333,
            tab_bar_bg: 0x002D2D2D,
            tab_active_bg: 0x001E1E1E,
            tab_inactive_bg: 0x00252526,
            tab_active_fg: 0x00FFFFFF,
            tab_inactive_fg: 0x00969696,
            status_bar_bg: 0x00007ACC,
            status_bar_fg: 0x00FFFFFF,
            selection_bg: 0x00264F78,
            cursor: 0x00528BFF,
            // Syntax
            syntax_keyword: 0x00C586C0,   // Purple
            syntax_type: 0x004EC9B0,      // Cyan/Teal
            syntax_string: 0x00CE9178,    // Orange
            syntax_number: 0x00B5CEA8,    // Light green
            syntax_comment: 0x006A9955,   // Dark green
            syntax_operator: 0x00D4D4D4,  // Light gray
            syntax_punctuation: 0x00D4D4D4,
            syntax_plain: 0x00D4D4D4,
        }
    }

    pub fn token_color(&self, token_type: TokenType) -> u32 {
        match token_type {
            TokenType::Keyword => self.syntax_keyword,
            TokenType::Type => self.syntax_type,
            TokenType::StringLiteral => self.syntax_string,
            TokenType::Number => self.syntax_number,
            TokenType::Comment => self.syntax_comment,
            TokenType::Operator => self.syntax_operator,
            TokenType::Punctuation => self.syntax_punctuation,
            TokenType::Identifier | TokenType::PlainText => self.syntax_plain,
        }
    }
}

pub struct GfxRenderer {
    pub colors: ThemeColors,
    pub tab_bar_height: usize,
    pub status_bar_height: usize,
}

impl GfxRenderer {
    pub fn new() -> Self {
        Self {
            colors: ThemeColors::modern_dark(),
            tab_bar_height: 28,
            status_bar_height: 24,
        }
    }

    /// Renders a complete editor frame into the software framebuffer.
    pub fn render(
        &self,
        fb: &mut Framebuffer,
        doc: &Document,
        tab_titles: &[String],
        active_tab_idx: usize,
        status_msg: Option<&str>,
        modal: Option<&crate::ui::prompt::PromptModal>,
    ) {
        // Clear background
        fb.clear(self.colors.background);

        // 1. Draw Tab Bar at top
        self.render_tab_bar(fb, tab_titles, active_tab_idx);

        // 2. Compute Viewport geometry
        let total_lines = doc.buffer.len_lines();
        let gutter_cols = total_lines.to_string().len().max(3);
        let gutter_width = (gutter_cols + 2) * FONT_WIDTH;

        let content_y_start = self.tab_bar_height;
        let content_y_end = fb.height.saturating_sub(self.status_bar_height);
        let content_height = content_y_end.saturating_sub(content_y_start);
        let visible_lines = content_height / FONT_HEIGHT;

        // 3. Draw Gutter background & divider
        fb.fill_rect(0, content_y_start, gutter_width, content_height, self.colors.gutter_bg);
        fb.fill_rect(gutter_width - 1, content_y_start, 1, content_height, self.colors.gutter_border);

        // 4. Collect selections & cursor pixel positions
        let mut selections = Vec::new();
        let mut cursor_positions = Vec::new();

        for c in doc.cursors.cursors() {
            let pos = doc.buffer.char_to_position(c.head);
            cursor_positions.push(pos);

            if c.has_selection() {
                let range = c.selection_range();
                let start = doc.buffer.char_to_position(range.start);
                let end = doc.buffer.char_to_position(range.end);
                selections.push((start, end));
            }
        }

        // 5. Draw Document Lines & Syntax
        for row in 0..visible_lines {
            let line_idx = doc.scroll_row + row;
            let py = content_y_start + row * FONT_HEIGHT;

            if line_idx < total_lines {
                // Line Number in gutter
                let num_str = format!("{:>width$}", line_idx + 1, width = gutter_cols);
                fb.draw_text(&num_str, FONT_WIDTH, py, self.colors.gutter_fg, None);

                // Line Content
                let line_text = doc.buffer.line(line_idx).unwrap_or_default();
                let text_start_x = gutter_width + FONT_WIDTH;

                // Selection background
                for &(start, end) in &selections {
                    if line_idx >= start.line && line_idx <= end.line {
                        let sel_col_start = if line_idx == start.line { start.col } else { 0 };
                        let sel_col_end = if line_idx == end.line {
                            end.col
                        } else {
                            line_text.chars().count() + 1
                        };

                        if sel_col_end > sel_col_start {
                            let sx = text_start_x + sel_col_start.saturating_sub(doc.scroll_col) * FONT_WIDTH;
                            let sw = (sel_col_end - sel_col_start) * FONT_WIDTH;
                            fb.fill_rect(sx, py, sw, FONT_HEIGHT, self.colors.selection_bg);
                        }
                    }
                }

                // Highlighted syntax tokens
                let tokens = SyntaxHighlighter::tokenize_line(&line_text, doc.language);
                let mut col_accum = 0;

                for token in tokens {
                    let token_color = self.colors.token_color(token.token_type);
                    for ch in token.text.chars() {
                        let col = col_accum;
                        col_accum += 1;

                        if col >= doc.scroll_col {
                            let px = text_start_x + (col - doc.scroll_col) * FONT_WIDTH;
                            if px + FONT_WIDTH <= fb.width {
                                fb.draw_char(ch, px, py, token_color, None);
                            }
                        }
                    }
                }

                // Draw Cursors on this line
                for pos in &cursor_positions {
                    if pos.line == line_idx && pos.col >= doc.scroll_col {
                        let cx = text_start_x + (pos.col - doc.scroll_col) * FONT_WIDTH;
                        // 2-pixel wide cursor bar
                        fb.fill_rect(cx, py, 2, FONT_HEIGHT, self.colors.cursor);
                    }
                }
            } else {
                // Tilde past end of file
                fb.draw_char('~', FONT_WIDTH, py, self.colors.gutter_fg, None);
            }
        }

        // 6. Draw Status Bar at bottom
        self.render_status_bar(fb, doc, status_msg);

        // 7. Draw Prompt Modal overlay if present
        if let Some(m) = modal {
            self.render_prompt_overlay(fb, m);
        }
    }

    fn render_tab_bar(&self, fb: &mut Framebuffer, tab_titles: &[String], active_tab_idx: usize) {
        fb.fill_rect(0, 0, fb.width, self.tab_bar_height, self.colors.tab_bar_bg);

        let mut cur_x = 8;
        for (idx, title) in tab_titles.iter().enumerate() {
            let is_active = idx == active_tab_idx;
            let tab_text = format!(" {} {} ", idx + 1, title);
            let tab_w = tab_text.chars().count() * FONT_WIDTH + 8;

            let (bg, fg) = if is_active {
                (self.colors.tab_active_bg, self.colors.tab_active_fg)
            } else {
                (self.colors.tab_inactive_bg, self.colors.tab_inactive_fg)
            };

            fb.fill_rect(cur_x, 2, tab_w, self.tab_bar_height - 2, bg);
            let text_y = (self.tab_bar_height - FONT_HEIGHT) / 2 + 1;
            fb.draw_text(&tab_text, cur_x + 4, text_y, fg, None);

            // Active tab indicator border on top
            if is_active {
                fb.fill_rect(cur_x, 0, tab_w, 2, self.colors.cursor);
            }

            cur_x += tab_w + 2;
        }
    }

    fn render_status_bar(&self, fb: &mut Framebuffer, doc: &Document, status_msg: Option<&str>) {
        let bar_y = fb.height.saturating_sub(self.status_bar_height);
        fb.fill_rect(0, bar_y, fb.width, self.status_bar_height, self.colors.status_bar_bg);

        let text_y = bar_y + (self.status_bar_height - FONT_HEIGHT) / 2;

        // Left: message or file info
        let file_name = doc
            .buffer
            .file_path()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[Untitled]");
        let dirty = if doc.buffer.is_dirty() { " [+]" } else { "" };

        let left_str = if let Some(msg) = status_msg {
            msg.to_string()
        } else {
            format!(" {} {}{} ", doc.language.name(), file_name, dirty)
        };
        fb.draw_text(&left_str, 8, text_y, self.colors.status_bar_fg, None);

        // Right: Line/Col, Total lines, Cursors
        let primary_pos = doc.buffer.char_to_position(doc.cursors.primary().head);
        let cursor_count = doc.cursors.cursors().len();
        let cursors_tag = if cursor_count > 1 {
            format!(" [{} cursors]", cursor_count)
        } else {
            String::new()
        };

        let right_str = format!(
            "Ln {}, Col {} │ {} lines{} ",
            primary_pos.line + 1,
            primary_pos.col + 1,
            doc.buffer.len_lines(),
            cursors_tag
        );
        let right_w = right_str.chars().count() * FONT_WIDTH;
        let right_x = fb.width.saturating_sub(right_w + 8);
        fb.draw_text(&right_str, right_x, text_y, self.colors.status_bar_fg, None);
    }

    /// Renders a centered floating prompt modal overlay (Search, Save As, Open, Command Palette).
    pub fn render_prompt_overlay(&self, fb: &mut Framebuffer, modal: &crate::ui::prompt::PromptModal) {
        let box_w = fb.width.min(640).max(320);
        let box_h: usize = 40;
        let box_x = (fb.width.saturating_sub(box_w)) / 2;
        let box_y: usize = 48;

        // Outer border
        fb.fill_rect(box_x.saturating_sub(2), box_y.saturating_sub(2), box_w + 4, box_h + 4, 0x00007ACC);
        // Background
        fb.fill_rect(box_x, box_y, box_w, box_h, 0x00252526);

        // Prompt text
        let display_text = modal.formatted_display();
        let text_y = box_y + (box_h - FONT_HEIGHT) / 2;
        fb.draw_text(&display_text, box_x + 12, text_y, 0x00FFFFFF, None);

        // Caret bar
        let cursor_x = box_x + 12 + (modal.kind.label().chars().count() + modal.cursor_col) * FONT_WIDTH;
        if cursor_x < box_x + box_w - 4 {
            fb.fill_rect(cursor_x, text_y, 2, FONT_HEIGHT, self.colors.cursor);
        }
    }
}

impl Default for GfxRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::buffer::TextBuffer;
    use crate::ui::prompt::{PromptKind, PromptModal};

    #[test]
    fn test_gfx_render_frame() {
        let mut fb = Framebuffer::new(800, 600);
        let doc = Document::new(TextBuffer::from_str("fn main() {\n    println!(\"Hello\");\n}"));
        let renderer = GfxRenderer::new();

        let tabs = vec!["main.rs".to_string()];
        renderer.render(&mut fb, &doc, &tabs, 0, Some("Ready"), None);

        // Verify non-zero pixels rendered
        let colored_pixels = fb.pixels.iter().filter(|&&p| p != 0x001E1E1E).count();
        assert!(colored_pixels > 100);
    }

    #[test]
    fn test_gfx_render_modal_overlay() {
        let mut fb = Framebuffer::new(800, 600);
        let doc = Document::new(TextBuffer::from_str("line 1\nline 2"));
        let renderer = GfxRenderer::new();
        let tabs = vec!["test.txt".to_string()];

        let mut modal = PromptModal::new(PromptKind::Find);
        modal.insert_char('r');
        modal.insert_char('u');
        modal.insert_char('s');
        modal.insert_char('t');

        renderer.render(&mut fb, &doc, &tabs, 0, None, Some(&modal));
        // Verify prompt modal overlay drew pixels
        let prompt_box_pixel = fb.get_pixel(400, 55);
        assert_eq!(prompt_box_pixel, 0x00252526);
    }
}
