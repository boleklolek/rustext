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
    // Scrollbar colors
    pub scrollbar_bg: u32,
    pub scrollbar_border: u32,
    pub scrollbar_thumb: u32,
    pub scrollbar_thumb_hover: u32,
    pub scrollbar_thumb_active: u32,
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
            // Scrollbar
            scrollbar_bg: 0x001E1E1E,
            scrollbar_border: 0x002D2D2D,
            scrollbar_thumb: 0x00424242,
            scrollbar_thumb_hover: 0x004F4F4F,
            scrollbar_thumb_active: 0x00686868,
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

    pub fn monokai() -> Self {
        Self {
            background: 0x00272822,
            gutter_bg: 0x001F201C,
            gutter_fg: 0x0075715E,
            gutter_border: 0x003E3D32,
            tab_bar_bg: 0x001E1F1C,
            tab_active_bg: 0x00272822,
            tab_inactive_bg: 0x001A1B18,
            tab_active_fg: 0x00F8F8F2,
            tab_inactive_fg: 0x0075715E,
            status_bar_bg: 0x0075715E,
            status_bar_fg: 0x00F8F8F2,
            selection_bg: 0x0049483E,
            cursor: 0x00F8F8F0,
            scrollbar_bg: 0x00272822,
            scrollbar_border: 0x003E3D32,
            scrollbar_thumb: 0x0059584E,
            scrollbar_thumb_hover: 0x0075715E,
            scrollbar_thumb_active: 0x00A6E22E,
            syntax_keyword: 0x00F92672,
            syntax_type: 0x0066D9EF,
            syntax_string: 0x00E6DB74,
            syntax_number: 0x00AE81FF,
            syntax_comment: 0x0075715E,
            syntax_operator: 0x00F92672,
            syntax_punctuation: 0x00F8F8F2,
            syntax_plain: 0x00F8F8F2,
        }
    }

    pub fn solarized_dark() -> Self {
        Self {
            background: 0x00002B36,
            gutter_bg: 0x00073642,
            gutter_fg: 0x00586E75,
            gutter_border: 0x00073642,
            tab_bar_bg: 0x00073642,
            tab_active_bg: 0x00002B36,
            tab_inactive_bg: 0x0005222B,
            tab_active_fg: 0x0093A1A1,
            tab_inactive_fg: 0x00586E75,
            status_bar_bg: 0x00268BD2,
            status_bar_fg: 0x00FDF6E3,
            selection_bg: 0x00073642,
            cursor: 0x002AA198,
            scrollbar_bg: 0x00002B36,
            scrollbar_border: 0x00073642,
            scrollbar_thumb: 0x00586E75,
            scrollbar_thumb_hover: 0x00657B83,
            scrollbar_thumb_active: 0x002AA198,
            syntax_keyword: 0x00859900,
            syntax_type: 0x00B58900,
            syntax_string: 0x002AA198,
            syntax_number: 0x00D33682,
            syntax_comment: 0x00586E75,
            syntax_operator: 0x00859900,
            syntax_punctuation: 0x0093A1A1,
            syntax_plain: 0x00839496,
        }
    }

    pub fn light() -> Self {
        Self {
            background: 0x00FFFFFF,
            gutter_bg: 0x00F6F8FA,
            gutter_fg: 0x008C959F,
            gutter_border: 0x00D0D7DE,
            tab_bar_bg: 0x00EAEAEA,
            tab_active_bg: 0x00FFFFFF,
            tab_inactive_bg: 0x00F0F0F0,
            tab_active_fg: 0x0024292F,
            tab_inactive_fg: 0x0057606A,
            status_bar_bg: 0x00007ACC,
            status_bar_fg: 0x00FFFFFF,
            selection_bg: 0x00B4D5FE,
            cursor: 0x000969DA,
            scrollbar_bg: 0x00F6F8FA,
            scrollbar_border: 0x00D0D7DE,
            scrollbar_thumb: 0x00D0D7DE,
            scrollbar_thumb_hover: 0x00AFB8C1,
            scrollbar_thumb_active: 0x000969DA,
            syntax_keyword: 0x00CF222E,
            syntax_type: 0x008250DF,
            syntax_string: 0x000A3069,
            syntax_number: 0x000550AE,
            syntax_comment: 0x006E7781,
            syntax_operator: 0x00CF222E,
            syntax_punctuation: 0x0024292F,
            syntax_plain: 0x0024292F,
        }
    }

    pub fn by_name(name: &str) -> Self {
        match name.to_lowercase().trim() {
            "monokai" => Self::monokai(),
            "solarized" | "solarized_dark" | "solarized-dark" => Self::solarized_dark(),
            "light" | "github_light" | "github-light" | "claro" => Self::light(),
            _ => Self::modern_dark(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbarGeometry {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub thumb_y: usize,
    pub thumb_height: usize,
    pub visible_lines: usize,
    pub total_lines: usize,
    pub max_scroll: usize,
}

impl ScrollbarGeometry {
    /// Returns whether the point (x, y) is inside the scrollbar track.
    pub fn contains_point(&self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// Returns whether the point (x, y) is inside the scrollbar draggable thumb.
    pub fn thumb_contains_point(&self, x: usize, y: usize) -> bool {
        self.contains_point(x, y) && y >= self.thumb_y && y < self.thumb_y + self.thumb_height
    }

    /// Translates a target thumb Y pixel position into the corresponding document scroll row.
    pub fn row_for_thumb_y(&self, target_thumb_y: usize) -> usize {
        let scrollable_h = self.height.saturating_sub(self.thumb_height);
        if scrollable_h == 0 || self.max_scroll == 0 {
            0
        } else {
            let clamped_y = target_thumb_y.clamp(self.y, self.y + scrollable_h);
            let rel_y = clamped_y.saturating_sub(self.y);
            let ratio = rel_y as f32 / scrollable_h as f32;
            (ratio * self.max_scroll as f32).round() as usize
        }
    }
}

pub struct GfxRenderer {
    pub colors: ThemeColors,
    pub menu_bar_height: usize,
    pub tab_bar_height: usize,
    pub status_bar_height: usize,
    pub scrollbar_width: usize,
    pub show_line_numbers: bool,
    pub diagnostics: Option<crate::editor::diagnostic::DiagnosticCollection>,
}

impl GfxRenderer {
    pub fn new() -> Self {
        Self {
            colors: ThemeColors::modern_dark(),
            menu_bar_height: 24,
            tab_bar_height: 26,
            status_bar_height: 22,
            scrollbar_width: 14,
            show_line_numbers: true,
            diagnostics: None,
        }
    }

    /// Computes scrollbar dimensions and thumb position for the given document and framebuffer.
    pub fn scrollbar_geometry(
        &self,
        fb_w: usize,
        fb_h: usize,
        doc: &Document,
        has_menu_bar: bool,
    ) -> ScrollbarGeometry {
        let menu_offset = if has_menu_bar { self.menu_bar_height } else { 0 };
        let content_y_start = menu_offset + self.tab_bar_height;
        let content_y_end = fb_h.saturating_sub(self.status_bar_height);
        let track_height = content_y_end.saturating_sub(content_y_start);
        let track_width = self.scrollbar_width;
        let track_x = fb_w.saturating_sub(track_width);
        let track_y = content_y_start;

        let visible_lines = if FONT_HEIGHT > 0 {
            track_height / FONT_HEIGHT
        } else {
            0
        };
        let total_lines = doc.buffer.len_lines().max(1);
        let max_scroll = total_lines.saturating_sub(visible_lines);

        let thumb_height = if total_lines == 0 || track_height == 0 {
            track_height
        } else {
            let ratio = (visible_lines as f32 / total_lines as f32).min(1.0);
            let th = (ratio * track_height as f32).round() as usize;
            th.clamp(20, track_height)
        };

        let scrollable_h = track_height.saturating_sub(thumb_height);
        let thumb_y = if max_scroll > 0 && scrollable_h > 0 {
            let ratio = (doc.scroll_row as f32 / max_scroll as f32).min(1.0);
            track_y + (ratio * scrollable_h as f32).round() as usize
        } else {
            track_y
        };

        ScrollbarGeometry {
            x: track_x,
            y: track_y,
            width: track_width,
            height: track_height,
            thumb_y,
            thumb_height,
            visible_lines,
            total_lines,
            max_scroll,
        }
    }

    /// Renders the vertical scrollbar track and draggable thumb.
    pub fn render_scrollbar(
        &self,
        fb: &mut Framebuffer,
        geom: &ScrollbarGeometry,
        is_hovered: bool,
        is_dragging: bool,
    ) {
        if geom.width == 0 || geom.height == 0 {
            return;
        }

        // 1. Track background
        fb.fill_rect(geom.x, geom.y, geom.width, geom.height, self.colors.scrollbar_bg);

        // 2. 1px left divider border separating code canvas from scrollbar
        fb.fill_rect(geom.x, geom.y, 1, geom.height, self.colors.scrollbar_border);

        // 3. Draggable thumb with 2px horizontal margin
        let thumb_color = if is_dragging {
            self.colors.scrollbar_thumb_active
        } else if is_hovered {
            self.colors.scrollbar_thumb_hover
        } else {
            self.colors.scrollbar_thumb
        };

        let thumb_margin_x = 2;
        let thumb_x = geom.x + thumb_margin_x;
        let thumb_w = geom.width.saturating_sub(thumb_margin_x * 2);

        if geom.thumb_height > 0 && thumb_w > 0 {
            fb.fill_rect(thumb_x, geom.thumb_y, thumb_w, geom.thumb_height, thumb_color);
        }
    }

    /// Renders a complete editor frame into the software framebuffer with default scrollbar state.
    pub fn render(
        &self,
        fb: &mut Framebuffer,
        doc: &Document,
        tab_titles: &[String],
        active_tab_idx: usize,
        status_msg: Option<&str>,
        modal: Option<&crate::ui::prompt::PromptModal>,
        menu_bar: Option<&crate::editor::menu::MenuBar>,
    ) {
        self.render_with_scrollbar_state(
            fb,
            doc,
            tab_titles,
            active_tab_idx,
            status_msg,
            modal,
            menu_bar,
            false,
            false,
        );
    }

    /// Renders a complete editor frame into the software framebuffer with interactive scrollbar state.
    pub fn render_with_scrollbar_state(
        &self,
        fb: &mut Framebuffer,
        doc: &Document,
        tab_titles: &[String],
        active_tab_idx: usize,
        status_msg: Option<&str>,
        modal: Option<&crate::ui::prompt::PromptModal>,
        menu_bar: Option<&crate::editor::menu::MenuBar>,
        scrollbar_hovered: bool,
        scrollbar_dragging: bool,
    ) {
        // Clear background
        fb.clear(self.colors.background);

        let menu_offset = if menu_bar.is_some() { self.menu_bar_height } else { 0 };

        // 1. Draw Menu Bar (if present)
        if let Some(mb) = menu_bar {
            self.render_menu_bar(fb, mb);
        }

        // 2. Draw Tab Bar at top
        self.render_tab_bar(fb, tab_titles, active_tab_idx, menu_offset);

        // 3. Compute Viewport geometry
        let total_lines = doc.buffer.len_lines();
        let gutter_cols = total_lines.to_string().len().max(3);
        let gutter_width = if self.show_line_numbers {
            (gutter_cols + 2) * FONT_WIDTH
        } else {
            8
        };

        let content_y_start = menu_offset + self.tab_bar_height;
        let content_y_end = fb.height.saturating_sub(self.status_bar_height);
        let content_height = content_y_end.saturating_sub(content_y_start);
        let visible_lines = content_height / FONT_HEIGHT;
        let max_code_x = fb.width.saturating_sub(self.scrollbar_width);

        // 3. Draw Gutter background & divider
        if self.show_line_numbers {
            fb.fill_rect(0, content_y_start, gutter_width, content_height, self.colors.gutter_bg);
            fb.fill_rect(gutter_width - 1, content_y_start, 1, content_height, self.colors.gutter_border);
        }

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
                // Line Number in gutter with diagnostic badge
                if self.show_line_numbers {
                    let diag = self.diagnostics.as_ref().and_then(|dc| dc.for_line(line_idx + 1));
                    let (fg_col, badge) = if let Some(d) = diag {
                        (d.severity.color_hex(), Some(d.severity.badge()))
                    } else {
                        (self.colors.gutter_fg, None)
                    };

                    let num_str = format!("{:>width$}", line_idx + 1, width = gutter_cols);
                    fb.draw_text(&num_str, FONT_WIDTH, py, fg_col, None);

                    if let Some(b) = badge {
                        fb.draw_text(b, 2, py, fg_col, None);
                    }
                }

                // Line Content
                let line_text = doc.buffer.line(line_idx).unwrap_or_default();
                let text_start_x = if self.show_line_numbers {
                    gutter_width + FONT_WIDTH
                } else {
                    8
                };

                // Selection background (clamped to max_code_x)
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
                            if sx < max_code_x {
                                let sw = ((sel_col_end - sel_col_start) * FONT_WIDTH).min(max_code_x - sx);
                                fb.fill_rect(sx, py, sw, FONT_HEIGHT, self.colors.selection_bg);
                            }
                        }
                    }
                }

                // Highlighted syntax tokens (clamped to max_code_x)
                let tokens = SyntaxHighlighter::tokenize_line(&line_text, doc.language);
                let mut col_accum = 0;

                for token in tokens {
                    let token_color = self.colors.token_color(token.token_type);
                    for ch in token.text.chars() {
                        let col = col_accum;
                        col_accum += 1;

                        if col >= doc.scroll_col {
                            let px = text_start_x + (col - doc.scroll_col) * FONT_WIDTH;
                            if px + FONT_WIDTH <= max_code_x {
                                fb.draw_char(ch, px, py, token_color, None);
                            }
                        }
                    }
                }

                // Draw Cursors on this line (clamped to max_code_x)
                for pos in &cursor_positions {
                    if pos.line == line_idx && pos.col >= doc.scroll_col {
                        let cx = text_start_x + (pos.col - doc.scroll_col) * FONT_WIDTH;
                        if cx + 2 <= max_code_x {
                            // 2-pixel wide cursor bar
                            fb.fill_rect(cx, py, 2, FONT_HEIGHT, self.colors.cursor);
                        }
                    }
                }
            } else if self.show_line_numbers {
                // Tilde past end of file
                fb.draw_char('~', FONT_WIDTH, py, self.colors.gutter_fg, None);
            }
        }

        // 6. Draw Interactive Scrollbar
        let scroll_geom = self.scrollbar_geometry(fb.width, fb.height, doc, menu_bar.is_some());
        self.render_scrollbar(fb, &scroll_geom, scrollbar_hovered, scrollbar_dragging);

        // 7. Draw Status Bar at bottom
        self.render_status_bar(fb, doc, status_msg);

        // 8. Draw Prompt Modal overlay if present
        if let Some(m) = modal {
            self.render_prompt_overlay(fb, m);
        }

        // 9. Draw Open Dropdown Menu if present (drawn on top)
        if let Some(mb) = menu_bar {
            self.render_menu_dropdown(fb, mb);
        }
    }

    fn render_menu_bar(&self, fb: &mut Framebuffer, menu_bar: &crate::editor::menu::MenuBar) {
        fb.fill_rect(0, 0, fb.width, self.menu_bar_height, 0x002D2D2D);

        let mut cur_x = 8;
        let text_y = (self.menu_bar_height - FONT_HEIGHT) / 2;

        for (idx, cat) in menu_bar.categories.iter().enumerate() {
            let is_active = menu_bar.active_category == Some(idx);
            let item_w = cat.name.chars().count() * FONT_WIDTH + 16;

            let (bg, fg) = if is_active {
                (0x00007ACC, 0x00FFFFFF)
            } else {
                (0x002D2D2D, 0x00CCCCCC)
            };

            fb.fill_rect(cur_x, 0, item_w, self.menu_bar_height, bg);
            fb.draw_text(cat.name, cur_x + 8, text_y, fg, None);

            cur_x += item_w;
        }

        // Thin bottom divider under menu bar
        fb.fill_rect(0, self.menu_bar_height - 1, fb.width, 1, 0x003A3D41);
    }

    fn render_menu_dropdown(&self, fb: &mut Framebuffer, menu_bar: &crate::editor::menu::MenuBar) {
        if let Some(cat_idx) = menu_bar.active_category {
            let cat = &menu_bar.categories[cat_idx];
            let (drop_x, _) = menu_bar.category_bounds(cat_idx, FONT_WIDTH);
            let drop_w = 260;
            let item_h = 22;
            let drop_h = cat.items.len() * item_h + 4;
            let drop_y = self.menu_bar_height;

            // Drop shadow
            fb.fill_rect(drop_x + 4, drop_y + 4, drop_w, drop_h, 0x00101010);
            // Border
            fb.fill_rect(drop_x.saturating_sub(1), drop_y.saturating_sub(1), drop_w + 2, drop_h + 2, 0x00007ACC);
            // Background
            fb.fill_rect(drop_x, drop_y, drop_w, drop_h, 0x00252526);

            for (item_idx, item) in cat.items.iter().enumerate() {
                let iy = drop_y + 2 + item_idx * item_h;

                if item.is_separator {
                    // Separator line
                    fb.fill_rect(drop_x + 8, iy + item_h / 2, drop_w.saturating_sub(16), 1, 0x003E3E42);
                } else {
                    let is_hovered = menu_bar.hovered_item == Some(item_idx);
                    if is_hovered {
                        fb.fill_rect(drop_x + 2, iy, drop_w - 4, item_h, 0x00007ACC);
                    }

                    let (text_color, shortcut_color) = if is_hovered {
                        (0x00FFFFFF, 0x00D4D4D4)
                    } else {
                        (0x00D4D4D4, 0x00858585)
                    };

                    let text_y = iy + (item_h - FONT_HEIGHT) / 2;
                    // Draw Label
                    fb.draw_text(item.label, drop_x + 12, text_y, text_color, None);

                    // Draw Shortcut right-aligned
                    if !item.shortcut.is_empty() {
                        let sc_w = item.shortcut.chars().count() * FONT_WIDTH;
                        let sc_x = drop_x + drop_w - sc_w - 12;
                        fb.draw_text(item.shortcut, sc_x, text_y, shortcut_color, None);
                    }
                }
            }
        }
    }

    fn render_tab_bar(
        &self,
        fb: &mut Framebuffer,
        tab_titles: &[String],
        active_tab_idx: usize,
        offset_y: usize,
    ) {
        fb.fill_rect(0, offset_y, fb.width, self.tab_bar_height, self.colors.tab_bar_bg);

        let mut cur_x = 8;
        let text_y = offset_y + (self.tab_bar_height - FONT_HEIGHT) / 2;

        for (idx, title) in tab_titles.iter().enumerate() {
            let is_active = idx == active_tab_idx;
            let tab_text = format!(" {} {} × ", idx + 1, title);
            let tab_w = tab_text.chars().count() * FONT_WIDTH + 8;

            let (bg, fg) = if is_active {
                (self.colors.tab_active_bg, self.colors.tab_active_fg)
            } else {
                (self.colors.tab_inactive_bg, self.colors.tab_inactive_fg)
            };

            fb.fill_rect(cur_x, offset_y + 2, tab_w, self.tab_bar_height - 2, bg);
            fb.draw_text(&tab_text, cur_x + 4, text_y, fg, None);

            // Active tab indicator border on top
            if is_active {
                fb.fill_rect(cur_x, offset_y, tab_w, 2, self.colors.cursor);
            }

            cur_x += tab_w + 2;
        }

        // New Tab [+] Button
        fb.fill_rect(cur_x, offset_y + 2, 24, self.tab_bar_height - 2, self.colors.tab_inactive_bg);
        fb.draw_text("+", cur_x + 8, text_y, self.colors.tab_inactive_fg, None);
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
        self.render_prompt_overlay_with_hover(fb, modal, false);
    }

    pub fn render_find_replace_dialog(
        &self,
        fb: &mut Framebuffer,
        modal: &crate::ui::prompt::PromptModal,
        close_hovered: bool,
    ) {
        let box_w = fb.width.min(540).max(380);
        let box_h = 168;
        let box_x = (fb.width.saturating_sub(box_w)) / 2;
        let box_y: usize = 28;

        // 1. Drop shadow
        fb.fill_rect(box_x + 6, box_y + 6, box_w, box_h, 0x000A0A0A);

        // 2. Outer border
        fb.fill_rect(box_x.saturating_sub(1), box_y.saturating_sub(1), box_w + 2, box_h + 2, 0x00007ACC);

        // 3. Title bar
        let header_h = 24;
        fb.fill_rect(box_x, box_y, box_w, header_h, 0x002D2D2D);
        fb.draw_text("Buscar y Reemplazar", box_x + 10, box_y + 4, 0x00FFFFFF, None);

        // Close button [×]
        let close_w = 30;
        let close_x = box_x + box_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { 0x002D2D2D };
        fb.fill_rect(close_x, box_y, close_w, header_h, close_bg);
        fb.draw_text("×", close_x + 11, box_y + 4, 0x00FFFFFF, None);

        // Header bottom divider
        fb.fill_rect(box_x, box_y + header_h, box_w, 1, 0x003A3D41);

        // 4. Window body
        let body_y = box_y + header_h + 1;
        let body_h = box_h.saturating_sub(header_h + 1);
        fb.fill_rect(box_x, body_y, box_w, body_h, 0x00252526);

        let input_margin = 12;
        let label_w = 100;
        let input_w = box_w.saturating_sub(input_margin * 2 + label_w);
        let input_h = 24;

        // Row 1: Buscar
        let row1_y = body_y + 12;
        fb.draw_text("Buscar:", box_x + input_margin, row1_y + 4, 0x00CCCCCC, None);

        let input1_x = box_x + input_margin + label_w;
        let border1 = if modal.active_field == 0 { 0x00007ACC } else { 0x003A3D41 };
        fb.fill_rect(input1_x.saturating_sub(1), row1_y.saturating_sub(1), input_w + 2, input_h + 2, border1);
        fb.fill_rect(input1_x, row1_y, input_w, input_h, 0x003C3C3C);
        fb.draw_text(&modal.input, input1_x + 6, row1_y + 4, 0x00FFFFFF, None);

        if modal.active_field == 0 {
            let cursor_x = input1_x + 6 + modal.cursor_col * FONT_WIDTH;
            if cursor_x < input1_x + input_w - 4 {
                fb.fill_rect(cursor_x, row1_y + 4, 2, FONT_HEIGHT, self.colors.cursor);
            }
        }

        // Row 2: Reemplazar
        let row2_y = row1_y + input_h + 10;
        fb.draw_text("Reemplazar:", box_x + input_margin, row2_y + 4, 0x00CCCCCC, None);

        let input2_x = box_x + input_margin + label_w;
        let border2 = if modal.active_field == 1 { 0x00007ACC } else { 0x003A3D41 };
        fb.fill_rect(input2_x.saturating_sub(1), row2_y.saturating_sub(1), input_w + 2, input_h + 2, border2);
        fb.fill_rect(input2_x, row2_y, input_w, input_h, 0x003C3C3C);
        fb.draw_text(&modal.replace_input, input2_x + 6, row2_y + 4, 0x00FFFFFF, None);

        if modal.active_field == 1 {
            let cursor_x = input2_x + 6 + modal.replace_cursor_col * FONT_WIDTH;
            if cursor_x < input2_x + input_w - 4 {
                fb.fill_rect(cursor_x, row2_y + 4, 2, FONT_HEIGHT, self.colors.cursor);
            }
        }

        // Row 3: Action Buttons
        let btn_y = row2_y + input_h + 16;
        let btn_h = 24;

        // Button 1: [< Ant]
        let b1_x = box_x + input_margin;
        let b1_w = 68;
        fb.fill_rect(b1_x.saturating_sub(1), btn_y.saturating_sub(1), b1_w + 2, btn_h + 2, 0x003A3D41);
        fb.fill_rect(b1_x, btn_y, b1_w, btn_h, 0x002D2D2D);
        fb.draw_text("< Ant", b1_x + 12, btn_y + 4, 0x00FFFFFF, None);

        // Button 2: [Sig >]
        let b2_x = b1_x + b1_w + 8;
        let b2_w = 68;
        fb.fill_rect(b2_x.saturating_sub(1), btn_y.saturating_sub(1), b2_w + 2, btn_h + 2, 0x003A3D41);
        fb.fill_rect(b2_x, btn_y, b2_w, btn_h, 0x002D2D2D);
        fb.draw_text("Sig >", b2_x + 14, btn_y + 4, 0x00FFFFFF, None);

        // Button 3: [Reemplazar]
        let b3_x = b2_x + b2_w + 14;
        let b3_w = 98;
        fb.fill_rect(b3_x.saturating_sub(1), btn_y.saturating_sub(1), b3_w + 2, btn_h + 2, 0x003A3D41);
        fb.fill_rect(b3_x, btn_y, b3_w, btn_h, 0x002D2D2D);
        fb.draw_text("Reemplazar", b3_x + 8, btn_y + 4, 0x00FFFFFF, None);

        // Button 4: [Reemplazar Todo]
        let b4_x = b3_x + b3_w + 8;
        let b4_w = 126;
        fb.fill_rect(b4_x.saturating_sub(1), btn_y.saturating_sub(1), b4_w + 2, btn_h + 2, 0x00007ACC);
        fb.fill_rect(b4_x, btn_y, b4_w, btn_h, 0x00007ACC);
        fb.draw_text("Reemplazar Todo", b4_x + 6, btn_y + 4, 0x00FFFFFF, None);
    }

    /// Renders a floating prompt window dialog with title bar, close button, drop shadow, and suggestions.
    pub fn render_prompt_overlay_with_hover(
        &self,
        fb: &mut Framebuffer,
        modal: &crate::ui::prompt::PromptModal,
        close_hovered: bool,
    ) {
        if modal.is_file_dialog() {
            self.render_file_dialog(fb, modal, close_hovered);
            return;
        }

        if modal.kind == crate::ui::prompt::PromptKind::Replace {
            self.render_find_replace_dialog(fb, modal, close_hovered);
            return;
        }

        let is_palette = modal.kind == crate::ui::prompt::PromptKind::CommandPalette;
        let box_w = fb.width.min(580).max(340);
        let box_h = if is_palette { 340 } else { 120 };
        let box_x = (fb.width.saturating_sub(box_w)) / 2;
        let box_y: usize = 28;

        // 1. Drop shadow
        fb.fill_rect(box_x + 6, box_y + 6, box_w, box_h, 0x000A0A0A);

        // 2. Outer border
        fb.fill_rect(box_x.saturating_sub(1), box_y.saturating_sub(1), box_w + 2, box_h + 2, 0x00007ACC);

        // 3. Window Header (Title Bar)
        let header_h = 24;
        fb.fill_rect(box_x, box_y, box_w, header_h, 0x002D2D2D);
        fb.draw_text(modal.kind.window_title(), box_x + 10, box_y + 4, 0x00FFFFFF, None);

        // Header close button [×]
        let close_w = 30;
        let close_x = box_x + box_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { 0x002D2D2D };
        fb.fill_rect(close_x, box_y, close_w, header_h, close_bg);
        fb.draw_text("×", close_x + 11, box_y + 4, 0x00FFFFFF, None);

        // Header bottom divider
        fb.fill_rect(box_x, box_y + header_h, box_w, 1, 0x003A3D41);

        // 4. Window Body
        let body_y = box_y + header_h + 1;
        let body_h = box_h.saturating_sub(header_h + 1);
        fb.fill_rect(box_x, body_y, box_w, body_h, 0x00252526);

        // 5. Input Field Box inside Window
        let input_margin = 12;
        let input_x = box_x + input_margin;
        let input_y = body_y + 8;
        let input_w = box_w.saturating_sub(input_margin * 2);
        let input_h = 24;

        fb.fill_rect(input_x.saturating_sub(1), input_y.saturating_sub(1), input_w + 2, input_h + 2, 0x00007ACC);
        fb.fill_rect(input_x, input_y, input_w, input_h, 0x003C3C3C);

        // Prompt label & input text
        let label = modal.kind.label();
        let text_y = input_y + (input_h - FONT_HEIGHT) / 2;
        fb.draw_text(label, input_x + 6, text_y, 0x00528BFF, None);

        let input_text_x = input_x + 6 + label.chars().count() * FONT_WIDTH;
        fb.draw_text(&modal.input, input_text_x, text_y, 0x00FFFFFF, None);

        // Caret bar
        let cursor_x = input_text_x + modal.cursor_col * FONT_WIDTH;
        if cursor_x < input_x + input_w - 4 {
            fb.fill_rect(cursor_x, text_y, 2, FONT_HEIGHT, self.colors.cursor);
        }

        // 6. Content below input
        if is_palette {
            let matching = modal.matching_commands();
            let mut item_y = input_y + input_h + 8;
            let item_h = 24;
            let max_visible = 9;

            for (idx, cmd) in matching.iter().take(max_visible).enumerate() {
                let is_sel = idx == modal.selected_idx;
                let (row_bg, row_fg, sc_fg) = if is_sel {
                    (0x00007ACC, 0x00FFFFFF, 0x00D4D4D4)
                } else {
                    (0x00252526, 0x00D4D4D4, 0x00858585)
                };

                fb.fill_rect(input_x, item_y, input_w, item_h, row_bg);
                let row_text_y = item_y + (item_h - FONT_HEIGHT) / 2;
                fb.draw_text(cmd.name, input_x + 8, row_text_y, row_fg, None);

                if !cmd.shortcut.is_empty() {
                    let sc_w = cmd.shortcut.chars().count() * FONT_WIDTH;
                    let sc_x = input_x + input_w - sc_w - 8;
                    fb.draw_text(cmd.shortcut, sc_x, row_text_y, sc_fg, None);
                }

                item_y += item_h;
            }

            // Footer hint
            let hint = "↑/↓: Seleccionar  •  Enter: Ejecutar  •  Esc: Cerrar";
            let hint_x = box_x + (box_w.saturating_sub(hint.chars().count() * FONT_WIDTH)) / 2;
            let hint_y = box_y + box_h - 18;
            fb.draw_text(hint, hint_x, hint_y, 0x00858585, None);
        } else {
            // Footer hint for Find
            let hint = "Enter: Confirmar  •  Esc: Cancelar";
            let hint_x = box_x + (box_w.saturating_sub(hint.chars().count() * FONT_WIDTH)) / 2;
            let hint_y = box_y + box_h - 22;
            fb.draw_text(hint, hint_x, hint_y, 0x00858585, None);
        }
    }

    /// Renders a native-styled File & Folder Dialog Window for saving and opening files in any directory.
    pub fn render_file_dialog(
        &self,
        fb: &mut Framebuffer,
        modal: &crate::ui::prompt::PromptModal,
        close_hovered: bool,
    ) {
        // Dim background
        for p in fb.pixels.iter_mut() {
            let r = (((*p >> 16) & 0xFF) * 3) / 4;
            let g = (((*p >> 8) & 0xFF) * 3) / 4;
            let b = ((*p & 0xFF) * 3) / 4;
            *p = (r << 16) | (g << 8) | b;
        }

        let box_w = fb.width.min(680).max(420);
        let box_h = fb.height.min(480).max(360);
        let box_x = (fb.width.saturating_sub(box_w)) / 2;
        let box_y = (fb.height.saturating_sub(box_h)) / 2;

        // 1. Drop shadow
        fb.fill_rect(box_x + 6, box_y + 6, box_w, box_h, 0x000A0A0A);

        // 2. Outer border
        fb.fill_rect(box_x.saturating_sub(1), box_y.saturating_sub(1), box_w + 2, box_h + 2, 0x00007ACC);

        // 3. Window Header (Title Bar)
        let header_h = 26;
        fb.fill_rect(box_x, box_y, box_w, header_h, 0x002D2D2D);
        fb.draw_text(modal.kind.window_title(), box_x + 10, box_y + 5, 0x00FFFFFF, None);

        // Header close button [×]
        let close_w = 32;
        let close_x = box_x + box_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { 0x002D2D2D };
        fb.fill_rect(close_x, box_y, close_w, header_h, close_bg);
        fb.draw_text("×", close_x + 12, box_y + 5, 0x00FFFFFF, None);

        // Header bottom divider
        fb.fill_rect(box_x, box_y + header_h, box_w, 1, 0x003A3D41);

        // 4. Window Body
        let body_y = box_y + header_h + 1;
        let body_h = box_h.saturating_sub(header_h + 1);
        fb.fill_rect(box_x, body_y, box_w, body_h, 0x00202020);

        // 5. Current Directory Bar
        let dir_bar_y = body_y + 6;
        let dir_bar_h = 24;
        fb.fill_rect(box_x + 8, dir_bar_y, box_w.saturating_sub(16), dir_bar_h, 0x00282828);
        fb.draw_text("Carpeta:", box_x + 14, dir_bar_y + 4, 0x00528BFF, None);

        let path_str = modal.current_dir.display().to_string();
        let max_path_chars = (box_w.saturating_sub(190)) / FONT_WIDTH;
        let display_path = if path_str.chars().count() > max_path_chars && max_path_chars > 3 {
            let skip = path_str.chars().count().saturating_sub(max_path_chars - 3);
            format!("...{}", path_str.chars().skip(skip).collect::<String>())
        } else {
            path_str
        };
        fb.draw_text(&display_path, box_x + 14 + 9 * FONT_WIDTH, dir_bar_y + 4, 0x00FFFFFF, None);

        // [Subir] button
        let up_btn_w = 64;
        let up_btn_x = box_x + box_w - up_btn_w - 12;
        fb.fill_rect(up_btn_x, dir_bar_y + 2, up_btn_w, 20, 0x003A3D41);
        fb.draw_text("[Subir]", up_btn_x + 6, dir_bar_y + 4, 0x00FFFFFF, None);

        // 6. Bookmarks Bar
        let bm_bar_y = dir_bar_y + dir_bar_h + 4;
        let bookmarks = crate::ui::prompt::get_quick_bookmarks();
        let mut cur_bm_x = box_x + 8;
        for bm in &bookmarks {
            let is_active = modal.current_dir == bm.path;
            let bm_text = format!("[{}]", bm.label);
            let bm_w = bm_text.chars().count() * FONT_WIDTH + 8;
            if cur_bm_x + bm_w < box_x + box_w - 8 {
                let bg = if is_active { 0x00007ACC } else { 0x002D2D2D };
                let fg = if is_active { 0x00FFFFFF } else { 0x00CCCCCC };
                fb.fill_rect(cur_bm_x, bm_bar_y, bm_w, 20, bg);
                fb.draw_text(&bm_text, cur_bm_x + 4, bm_bar_y + 2, fg, None);
                cur_bm_x += bm_w + 6;
            }
        }

        // 7. Directory Entries List
        let list_y = bm_bar_y + 26;
        let list_h = box_h.saturating_sub(190);
        let list_w = box_w.saturating_sub(16);
        let list_x = box_x + 8;

        fb.fill_rect(list_x.saturating_sub(1), list_y.saturating_sub(1), list_w + 2, list_h + 2, 0x003A3D41);
        fb.fill_rect(list_x, list_y, list_w, list_h, 0x00181818);

        let row_h = 22;
        let max_visible = list_h / row_h;

        if modal.dir_entries.is_empty() {
            fb.draw_text("(Carpeta vacía o sin permisos de lectura)", list_x + 12, list_y + 10, 0x00858585, None);
        } else {
            for (idx, entry) in modal.dir_entries.iter().skip(modal.scroll_offset).take(max_visible).enumerate() {
                let actual_idx = modal.scroll_offset + idx;
                let is_selected = actual_idx == modal.selected_entry_idx;
                let ry = list_y + idx * row_h;

                let row_bg = if is_selected { 0x00007ACC } else { 0x00181818 };
                fb.fill_rect(list_x, ry, list_w, row_h, row_bg);

                let tag = if entry.is_dir { "[DIR] " } else { "      " };
                let tag_fg = if is_selected { 0x00FFFFFF } else { 0x004EC9B0 };
                let name_fg = if is_selected { 0x00FFFFFF } else if entry.is_dir { 0x00FFFFFF } else { 0x00D4D4D4 };

                fb.draw_text(tag, list_x + 8, ry + 3, tag_fg, None);
                fb.draw_text(&entry.name, list_x + 8 + 6 * FONT_WIDTH, ry + 3, name_fg, None);
            }
        }

        // 8. Filename Input Field
        let input_y = box_y + box_h - 76;
        fb.draw_text("Nombre de archivo:", list_x, input_y, 0x00D4D4D4, None);

        let input_box_y = input_y + 18;
        let input_box_h = 24;
        fb.fill_rect(list_x.saturating_sub(1), input_box_y.saturating_sub(1), list_w + 2, input_box_h + 2, 0x00007ACC);
        fb.fill_rect(list_x, input_box_y, list_w, input_box_h, 0x002D2D2D);

        let input_text_y = input_box_y + (input_box_h - FONT_HEIGHT) / 2;
        fb.draw_text(&modal.input, list_x + 8, input_text_y, 0x00FFFFFF, None);

        // Caret
        let cursor_x = list_x + 8 + modal.cursor_col * FONT_WIDTH;
        if cursor_x < list_x + list_w - 4 {
            fb.fill_rect(cursor_x, input_text_y, 2, FONT_HEIGHT, self.colors.cursor);
        }

        // 9. Bottom Buttons
        let btn_bar_y = box_y + box_h - 30;
        let action_btn_text = if modal.kind == crate::ui::prompt::PromptKind::SaveAs { "Guardar" } else { "Abrir" };
        let action_btn_w = 88;
        let cancel_btn_w = 80;

        let cancel_btn_x = box_x + box_w - cancel_btn_w - 12;
        let action_btn_x = cancel_btn_x - action_btn_w - 8;

        // Action button [Guardar] / [Abrir]
        fb.fill_rect(action_btn_x.saturating_sub(1), btn_bar_y.saturating_sub(1), action_btn_w + 2, 24, 0x001F8AD2);
        fb.fill_rect(action_btn_x, btn_bar_y, action_btn_w, 22, 0x00007ACC);
        let at_x = action_btn_x + (action_btn_w - action_btn_text.chars().count() * FONT_WIDTH) / 2;
        fb.draw_text(action_btn_text, at_x, btn_bar_y + 3, 0x00FFFFFF, None);

        // Cancel button [Cancelar]
        fb.fill_rect(cancel_btn_x.saturating_sub(1), btn_bar_y.saturating_sub(1), cancel_btn_w + 2, 24, 0x003E3E42);
        fb.fill_rect(cancel_btn_x, btn_bar_y, cancel_btn_w, 22, 0x002D2D2D);
        let ct_x = cancel_btn_x + (cancel_btn_w - 8 * FONT_WIDTH) / 2;
        fb.draw_text("Cancelar", ct_x, btn_bar_y + 3, 0x00CCCCCC, None);

        // Footer hint
        fb.draw_text("Doble clic o Enter para abrir carpeta", list_x, btn_bar_y + 4, 0x00858585, None);
    }

    /// Renders a native-styled About Dialog Window with title bar, close button, author details, and logo.
    pub fn render_about_dialog(
        &self,
        fb: &mut Framebuffer,
        close_hovered: bool,
        ok_hovered: bool,
    ) {
        // 1. Semi-transparent backdrop tint (darken by 50%)
        for p in fb.pixels.iter_mut() {
            let r = ((*p >> 16) & 0xFF) / 2;
            let g = ((*p >> 8) & 0xFF) / 2;
            let b = (*p & 0xFF) / 2;
            *p = (r << 16) | (g << 8) | b;
        }

        // 2. Dialog box dimensions
        let win_w = 640.min(fb.width.saturating_sub(20));
        let win_h = 450.min(fb.height.saturating_sub(20));
        let win_x = (fb.width.saturating_sub(win_w)) / 2;
        let win_y = (fb.height.saturating_sub(win_h)) / 2;

        // 3. Drop shadow
        fb.fill_rect(win_x + 6, win_y + 6, win_w, win_h, 0x000A0A0A);

        // 4. Outer border (VS Code accent 0x00007ACC)
        fb.fill_rect(win_x.saturating_sub(1), win_y.saturating_sub(1), win_w + 2, win_h + 2, 0x00007ACC);

        // 5. Window Title Bar (Header)
        let title_h = 28;
        fb.fill_rect(win_x, win_y, win_w, title_h, 0x002D2D2D);
        fb.draw_text("Rustext - Acerca de", win_x + 12, win_y + 6, 0x00FFFFFF, None);

        // Close button [×] on the right
        let close_w = 36;
        let close_x = win_x + win_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { 0x002D2D2D };
        fb.fill_rect(close_x, win_y, close_w, title_h, close_bg);
        fb.draw_text("×", close_x + 14, win_y + 6, 0x00FFFFFF, None);

        // Title bar separator
        fb.fill_rect(win_x, win_y + title_h, win_w, 1, 0x003A3D41);

        // 6. Window Body background
        let body_y = win_y + title_h + 1;
        let body_h = win_h.saturating_sub(title_h + 1);
        fb.fill_rect(win_x, body_y, win_w, body_h, 0x001E1E1E);

        // 7. Rustext ASCII Banner
        let banner_lines = [
            "  ____            _            _   ",
            " |  _ \\ _   _ ___| |_ _____  _| |_ ",
            " | |_) | | | / __| __/ _ \\ \\/ / __|",
            " |  _ <| |_| \\__ \\ ||  __/>  <| |_ ",
            " |_| \\_\\\\__,_|___/\\__\\___/_/\\_\\\\__|",
        ];
        let mut cur_y = body_y + 12;
        let banner_color = 0x00528BFF; // Vibrant Rustext blue
        for line in &banner_lines {
            let line_w = line.chars().count() * FONT_WIDTH;
            let line_x = win_x + (win_w.saturating_sub(line_w)) / 2;
            fb.draw_text(line, line_x, cur_y, banner_color, None);
            cur_y += 16;
        }

        // Subtitle & Version
        cur_y += 4;
        let title_str = "Rustext Editor v0.1.0";
        let title_x = win_x + (win_w.saturating_sub(title_str.chars().count() * FONT_WIDTH)) / 2;
        fb.draw_text(title_str, title_x, cur_y, 0x00FFFFFF, None);
        cur_y += 18;

        let tag_str = "100% Pure Rust • Zero Dependencias Externas";
        let tag_x = win_x + (win_w.saturating_sub(tag_str.chars().count() * FONT_WIDTH)) / 2;
        fb.draw_text(tag_str, tag_x, cur_y, 0x004EC9B0, None);
        cur_y += 20;

        // Separator line
        fb.fill_rect(win_x + 20, cur_y, win_w.saturating_sub(40), 1, 0x003A3D41);
        cur_y += 10;

        // Project and Author Details
        let details = [
            ("• Desarrollador: ", "boleklolek <bolek.lolek@gmail.com>"),
            ("• Licencia:      ", "MIT License (Código Abierto)"),
            ("• Arquitectura:  ", "100% Rust Estándar (Sin crates ni librerías)"),
            ("• Motor:         ", "Piece Table O(log N) con Multi-Cursor"),
            ("• Renderizado:   ", "Framebuffer RGB32 Nativo por Software"),
            ("• Plataformas:   ", "Linux (X11) • Windows (Win32) • macOS"),
            ("• Repositorio:   ", "https://github.com/boleklolek/rustext"),
        ];

        let left_margin = win_x + 24;
        for (lbl, val) in &details {
            fb.draw_text(lbl, left_margin, cur_y, 0x00858585, None);
            let val_x = left_margin + lbl.chars().count() * FONT_WIDTH;
            let val_color = if lbl.contains("Repositorio") { 0x00007ACC } else { 0x00D4D4D4 };
            fb.draw_text(val, val_x, cur_y, val_color, None);
            cur_y += 17;
        }

        cur_y += 4;
        // Separator line
        fb.fill_rect(win_x + 20, cur_y, win_w.saturating_sub(40), 1, 0x003A3D41);
        cur_y += 8;

        // Essential shortcuts reference
        let sc_str = "Ctrl+T: Pestaña  •  Ctrl+S: Guardar  •  Ctrl+F: Buscar  •  Ctrl+P: Comandos";
        let sc_x = win_x + (win_w.saturating_sub(sc_str.chars().count() * FONT_WIDTH)) / 2;
        fb.draw_text(sc_str, sc_x, cur_y, 0x00858585, None);

        // 8. Bottom Button: [ Aceptar ]
        let btn_w = 130;
        let btn_h = 28;
        let btn_x = win_x + (win_w - btn_w) / 2;
        let btn_y = win_y + win_h - 38;

        let (btn_bg, btn_border) = if ok_hovered {
            (0x00007ACC, 0x001F8AD2)
        } else {
            (0x002D2D2D, 0x003E3E42)
        };
        fb.fill_rect(btn_x.saturating_sub(1), btn_y.saturating_sub(1), btn_w + 2, btn_h + 2, btn_border);
        fb.fill_rect(btn_x, btn_y, btn_w, btn_h, btn_bg);

        let btn_text = "Aceptar";
        let text_x = btn_x + (btn_w - btn_text.chars().count() * FONT_WIDTH) / 2;
        let text_y = btn_y + (btn_h - FONT_HEIGHT) / 2;
        fb.draw_text(btn_text, text_x, text_y, 0x00FFFFFF, None);
    }

    /// Renders a native-styled Extensions Manager Dialog Window.
    pub fn render_extensions_dialog(
        &self,
        fb: &mut Framebuffer,
        ext_mgr: &crate::plugin::ExtensionManager,
        selected_idx: usize,
        scroll_offset: usize,
        close_hovered: bool,
    ) {
        // Dim background
        for p in fb.pixels.iter_mut() {
            let r = (((*p >> 16) & 0xFF) * 3) / 4;
            let g = (((*p >> 8) & 0xFF) * 3) / 4;
            let b = ((*p & 0xFF) * 3) / 4;
            *p = (r << 16) | (g << 8) | b;
        }

        let win_w = 680.min(fb.width.saturating_sub(20));
        let win_h = 480.min(fb.height.saturating_sub(20));
        let win_x = (fb.width.saturating_sub(win_w)) / 2;
        let win_y = (fb.height.saturating_sub(win_h)) / 2;

        // Drop shadow & border
        fb.fill_rect(win_x + 6, win_y + 6, win_w, win_h, 0x000A0A0A);
        fb.fill_rect(win_x.saturating_sub(1), win_y.saturating_sub(1), win_w + 2, win_h + 2, 0x00007ACC);

        // Header Title Bar
        let title_h = 28;
        fb.fill_rect(win_x, win_y, win_w, title_h, 0x002D2D2D);
        fb.draw_text("Administrador de Extensiones", win_x + 12, win_y + 6, 0x00FFFFFF, None);

        // Close button [×]
        let close_w = 36;
        let close_x = win_x + win_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { 0x002D2D2D };
        fb.fill_rect(close_x, win_y, close_w, title_h, close_bg);
        fb.draw_text("×", close_x + 14, win_y + 6, 0x00FFFFFF, None);

        // Header bottom divider
        fb.fill_rect(win_x, win_y + title_h, win_w, 1, 0x003A3D41);

        // Body background
        let body_y = win_y + title_h + 1;
        let body_h = win_h.saturating_sub(title_h + 1);
        fb.fill_rect(win_x, body_y, win_w, body_h, 0x001E1E1E);

        // Toolbar: Directory path & action buttons
        let bar_y = body_y + 8;
        let count = ext_mgr.extensions.len();
        let dir_str = format!("Carpeta: ~/.rustext/extensions  •  {} extensiones", count);
        fb.draw_text(&dir_str, win_x + 12, bar_y + 4, 0x00528BFF, None);

        // Action buttons: [ + Crear Ejemplo ] & [ ↺ Recargar ]
        let reload_btn_w = 90;
        let create_btn_w = 120;
        let reload_btn_x = win_x + win_w - reload_btn_w - 12;
        let create_btn_x = reload_btn_x - create_btn_w - 8;

        fb.fill_rect(create_btn_x.saturating_sub(1), bar_y.saturating_sub(1), create_btn_w + 2, 24, 0x003A3D41);
        fb.fill_rect(create_btn_x, bar_y, create_btn_w, 22, 0x002D2D2D);
        fb.draw_text("+ Crear Ejemplo", create_btn_x + 6, bar_y + 3, 0x00FFFFFF, None);

        fb.fill_rect(reload_btn_x.saturating_sub(1), bar_y.saturating_sub(1), reload_btn_w + 2, 24, 0x003A3D41);
        fb.fill_rect(reload_btn_x, bar_y, reload_btn_w, 22, 0x002D2D2D);
        fb.draw_text("↺ Recargar", reload_btn_x + 10, bar_y + 3, 0x00FFFFFF, None);

        // List Area
        let list_x = win_x + 12;
        let list_y = bar_y + 30;
        let list_w = win_w.saturating_sub(24);
        let list_h = win_h.saturating_sub(115);

        fb.fill_rect(list_x.saturating_sub(1), list_y.saturating_sub(1), list_w + 2, list_h + 2, 0x003A3D41);
        fb.fill_rect(list_x, list_y, list_w, list_h, 0x00181818);

        if ext_mgr.extensions.is_empty() {
            fb.draw_text("No hay extensiones instaladas. Haz clic en '+ Crear Ejemplo'.", list_x + 16, list_y + 20, 0x00858585, None);
        } else {
            let card_h = 58;
            let max_visible = list_h / card_h;
            let mut cur_card_y = list_y + 2;

            for (idx, ext) in ext_mgr.extensions.iter().skip(scroll_offset).take(max_visible).enumerate() {
                let actual_idx = scroll_offset + idx;
                let is_selected = actual_idx == selected_idx;

                let card_bg = if is_selected { 0x00242830 } else { 0x001E1E1E };
                let border_color = if is_selected { 0x00007ACC } else { 0x002D2D2D };

                fb.fill_rect(list_x + 2, cur_card_y, list_w.saturating_sub(4), card_h - 4, card_bg);
                fb.fill_rect(list_x + 2, cur_card_y, 3, card_h - 4, border_color);

                // Extension Name & Version
                let title_text = format!("{}  v{}", ext.name, ext.version);
                fb.draw_text(&title_text, list_x + 12, cur_card_y + 4, 0x00FFFFFF, None);

                // Status Badge: [ACTIVA] or [DESACTIVADA]
                let (badge_text, badge_bg, badge_fg) = if ext.enabled {
                    ("[ACTIVA]", 0x000E7030, 0x004EC9B0)
                } else {
                    ("[DESACTIVADA]", 0x003A3D41, 0x00858585)
                };
                let badge_x = list_x + list_w - 180;
                fb.fill_rect(badge_x, cur_card_y + 4, 100, 18, badge_bg);
                fb.draw_text(badge_text, badge_x + 6, cur_card_y + 5, badge_fg, None);

                // Description
                let desc = if ext.description.is_empty() { "Sin descripción" } else { &ext.description };
                fb.draw_text(desc, list_x + 12, cur_card_y + 22, 0x00CCCCCC, None);

                // Commands tag
                let cmds_summary = if ext.commands.is_empty() {
                    "Sin comandos registrados".to_string()
                } else {
                    let names: Vec<&str> = ext.commands.iter().map(|c| c.label.as_str()).collect();
                    format!("Comandos: {}", names.join(", "))
                };
                fb.draw_text(&cmds_summary, list_x + 12, cur_card_y + 38, 0x00858585, None);

                // Toggle Action Button on row
                let toggle_text = if ext.enabled { "Desactivar" } else { "Activar" };
                let toggle_w = 64;
                let toggle_x = list_x + list_w - toggle_w - 8;
                let toggle_y = cur_card_y + 4;
                fb.fill_rect(toggle_x, toggle_y, toggle_w, 20, 0x002D2D2D);
                fb.draw_text(toggle_text, toggle_x + 4, toggle_y + 2, 0x00FFFFFF, None);

                cur_card_y += card_h;
            }
        }

        // Footer hints & Close button
        let footer_y = win_y + win_h - 34;
        let hint_text = "Enter / Espacio: Alternar  •  'n': Nueva  •  'r': Recargar  •  Esc: Salir";
        fb.draw_text(hint_text, list_x, footer_y + 6, 0x00858585, None);

        let close_btn_w = 80;
        let close_btn_x = win_x + win_w - close_btn_w - 12;
        fb.fill_rect(close_btn_x.saturating_sub(1), footer_y.saturating_sub(1), close_btn_w + 2, 26, 0x003A3D41);
        fb.fill_rect(close_btn_x, footer_y, close_btn_w, 24, 0x002D2D2D);
        fb.draw_text("Cerrar", close_btn_x + 16, footer_y + 4, 0x00FFFFFF, None);
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
        renderer.render(&mut fb, &doc, &tabs, 0, Some("Ready"), None, None);

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

        renderer.render(&mut fb, &doc, &tabs, 0, None, Some(&modal), None);
        // Verify prompt modal overlay drew pixels
        let prompt_box_pixel = fb.get_pixel(400, 55);
        assert_eq!(prompt_box_pixel, 0x00252526);
    }

    #[test]
    fn test_gfx_render_menu_dropdown() {
        let mut fb = Framebuffer::new(800, 600);
        let doc = Document::new(TextBuffer::from_str("let x = 42;"));
        let renderer = GfxRenderer::new();
        let tabs = vec!["code.rs".to_string()];

        let mut menu_bar = crate::editor::menu::MenuBar::new();
        menu_bar.active_category = Some(0); // Open "Archivo"

        renderer.render(&mut fb, &doc, &tabs, 0, None, None, Some(&menu_bar));
        // Dropdown background pixel
        let drop_pixel = fb.get_pixel(15, 30);
        assert_eq!(drop_pixel, 0x00252526);
    }
}
