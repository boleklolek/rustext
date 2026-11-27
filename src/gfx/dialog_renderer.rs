//! Modal and dialog rendering for Rustext GFX.
//! Zero third-party dependencies, pure standard library Rust.

use super::font::{FontFamily, FontRasterizer, FontWeight, FONT_WIDTH};
use super::framebuffer::Framebuffer;
use super::gamma::{blend_pixel_gamma, blend_pixel_subpixel_gamma};
use super::optical_spacing::OpticalPunctuationSpacing;
use super::sdf::SdfFontRasterizer;
use super::shaping::FontShaper;
use super::stem_darkening::StemDarkener;
use super::renderer::GfxRenderer;

impl GfxRenderer {
    pub fn render_prompt_overlay(&self, fb: &mut Framebuffer, modal: &crate::ui::prompt::PromptModal) {
        self.render_prompt_overlay_with_hover(fb, modal, false);
    }

    /// Renders the interactive Project-Wide Global Search Window (`Ctrl+Shift+F`).
    pub fn render_global_search_dialog(
        &self,
        fb: &mut Framebuffer,
        modal: &crate::ui::prompt::PromptModal,
        close_hovered: bool,
    ) {
        let box_w = fb.width.min(self.scale_coord(740)).max(self.scale_coord(460));
        let box_h = fb.height.min(self.scale_coord(500)).max(self.scale_coord(360));
        let box_x = (fb.width.saturating_sub(box_w)) / 2;
        let box_y: usize = self.scale_coord(28);

        // 1. Semi-transparent backdrop scrim
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 110);

        // 2. Drop shadow
        if self.shadows {
            self.render_shadow(fb, box_x, box_y, box_w, box_h, 10, 8);
        } else {
            fb.fill_rect(box_x + 6, box_y + 6, box_w, box_h, 0x000A0A0A);
        }

        // 3. Title bar & window outer border
        let header_h = self.scale_coord(24);
        fb.fill_rect(box_x.saturating_sub(1), box_y.saturating_sub(1), box_w + 2, box_h + 2, self.colors.modal_border);
        fb.fill_rect(box_x, box_y, box_w, header_h, self.colors.modal_header_bg);
        fb.fill_rect(box_x, box_y + header_h, box_w, box_h.saturating_sub(header_h), self.colors.modal_bg);
        if self.rounded_corners {
            fb.draw_rounded_outline(box_x, box_y, box_w, box_h, 6, self.colors.modal_border);
        }

        fb.draw_text("Búsqueda Global en el Proyecto (Ctrl+Shift+F)", box_x + self.scale_coord(10), box_y + self.scale_coord(4), 0x00FFFFFF, None);

        // Header close button [×]
        let close_w = self.scale_coord(30);
        let close_x = box_x + box_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, box_y + 2, close_w - 4, header_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, box_y, close_w, header_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(11), box_y + self.scale_coord(4), 0x00FFFFFF, None);

        // Header bottom divider
        fb.fill_rect(box_x, box_y + header_h, box_w, 1, self.colors.scrollbar_border);

        // 4. Input Box
        let body_y = box_y + header_h + 1;
        let input_margin = self.scale_coord(12);
        let input_x = box_x + input_margin;
        let input_y = body_y + self.scale_coord(8);
        let input_w = box_w.saturating_sub(input_margin * 2);
        let input_h = self.scale_coord(24);

        if self.rounded_corners {
            fb.fill_rounded_rect(input_x, input_y, input_w, input_h, 4, self.colors.input_bg);
            fb.draw_rounded_outline(input_x, input_y, input_w, input_h, 4, self.colors.input_border);
        } else {
            fb.fill_rect(input_x.saturating_sub(1), input_y.saturating_sub(1), input_w + 2, input_h + 2, self.colors.input_border);
            fb.fill_rect(input_x, input_y, input_w, input_h, self.colors.input_bg);
        }

        let label = "Buscar en Proyecto: ";
        let text_y = input_y + (input_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(label, input_x + self.scale_coord(6), text_y, self.colors.accent, None);

        let input_text_x = input_x + self.scale_coord(6) + label.chars().count() * self.char_width();
        fb.draw_text(&modal.input, input_text_x, text_y, 0x00FFFFFF, None);

        let cursor_x = input_text_x + modal.cursor_col * self.char_width();
        if cursor_x < input_x + input_w - self.scale_coord(4) {
            fb.fill_rect(cursor_x, text_y, self.scale_coord(2).max(1), self.char_height(), self.colors.cursor);
        }

        // 5. Status & Options bar
        let subbar_y = input_y + input_h + self.scale_coord(8);
        let subbar_h = self.scale_coord(22);

        let stats_text = if modal.input.trim().is_empty() {
            "Escriba para buscar en todos los archivos del proyecto...".to_string()
        } else if modal.global_results.is_empty() {
            "0 coincidencias encontradas".to_string()
        } else {
            let trunc_hint = if modal.global_truncated { " (límite 1000 alcanzado)" } else { "" };
            format!(
                "{} coincidencia{} en {} archivo{}{}",
                modal.global_results.len(),
                if modal.global_results.len() == 1 { "" } else { "s" },
                modal.global_matching_files,
                if modal.global_matching_files == 1 { "" } else { "s" },
                trunc_hint
            )
        };

        let stats_color = if modal.global_results.is_empty() {
            0x00888888
        } else {
            self.colors.syntax_string
        };
        fb.draw_text(&stats_text, input_x, subbar_y + self.scale_coord(3), stats_color, None);

        // Case-sensitive toggle button on the right of subbar
        let case_btn_text = if modal.case_sensitive { "[Aa Mayús: Sí]" } else { "[Aa Mayús: No]" };
        let case_w = case_btn_text.chars().count() * self.char_width() + self.scale_coord(8);
        let case_x = input_x + input_w - case_w;
        let case_bg = if modal.case_sensitive { self.colors.button_bg } else { self.colors.chip_bg };
        let case_border = if modal.case_sensitive { self.colors.accent } else { self.colors.scrollbar_border };
        if self.rounded_corners {
            fb.fill_rounded_rect(case_x, subbar_y, case_w, subbar_h, 3, case_bg);
            fb.draw_rounded_outline(case_x, subbar_y, case_w, subbar_h, 3, case_border);
        } else {
            fb.fill_rect(case_x, subbar_y, case_w, subbar_h, case_bg);
        }
        let case_tx = case_x + self.scale_coord(4);
        let case_ty = subbar_y + (subbar_h.saturating_sub(self.char_height())) / 2;
        let case_fg = if modal.case_sensitive { 0x00FFFFFF } else { 0x00AAAAAA };
        fb.draw_text(case_btn_text, case_tx, case_ty, case_fg, None);

        // 6. Results List
        let list_y = subbar_y + subbar_h + self.scale_coord(8);
        let footer_h = self.scale_coord(24);
        let list_h = (box_y + box_h).saturating_sub(list_y + footer_h + self.scale_coord(4));
        let list_w = input_w;
        let row_h = (self.char_height() + self.scale_coord(6)).max(self.scale_coord(22));
        let max_visible = list_h / row_h;

        // Background of results panel
        if self.rounded_corners {
            fb.fill_rounded_rect(input_x, list_y, list_w, list_h, 4, self.colors.gutter_bg);
            fb.draw_rounded_outline(input_x, list_y, list_w, list_h, 4, self.colors.scrollbar_border);
        } else {
            fb.fill_rect(input_x, list_y, list_w, list_h, self.colors.gutter_bg);
            fb.draw_rect_outline(input_x, list_y, list_w, list_h, self.colors.scrollbar_border);
        }

        let results_len = modal.global_results.len();
        let scroll_offset = modal.scroll_offset.min(results_len.saturating_sub(1));

        let path_col_w = (list_w / 3).min(self.scale_coord(240)).max(self.scale_coord(140));

        for row_idx in 0..max_visible {
            let item_idx = scroll_offset + row_idx;
            if item_idx >= results_len {
                break;
            }

            let item = &modal.global_results[item_idx];
            let is_sel = item_idx == modal.selected_idx;
            let item_y = list_y + self.scale_coord(2) + row_idx * row_h;

            if is_sel {
                let sel_bg = self.colors.accent;
                if self.rounded_corners {
                    fb.fill_rounded_rect(input_x + 2, item_y, list_w.saturating_sub(4), row_h.saturating_sub(2), 3, sel_bg);
                } else {
                    fb.fill_rect(input_x + 2, item_y, list_w.saturating_sub(4), row_h.saturating_sub(2), sel_bg);
                }
            }

            let item_text_y = item_y + (row_h.saturating_sub(self.char_height())) / 2;

            // 1. Path and line badge (e.g. "src/main.rs:42")
            let badge = format!("{}:{}", item.relative_path, item.line_number);
            let badge_max_chars = path_col_w / self.char_width().max(1);
            let badge_display = if badge.chars().count() > badge_max_chars {
                let take = badge_max_chars.saturating_sub(3);
                let tail: String = badge.chars().rev().take(take).collect::<Vec<char>>().into_iter().rev().collect();
                format!("...{}", tail)
            } else {
                badge
            };

            let badge_color = if is_sel { 0x00FFFFFF } else { self.colors.syntax_type };
            fb.draw_text(&badge_display, input_x + self.scale_coord(6), item_text_y, badge_color, None);

            // Divider line between file badge and code snippet
            let div_x = input_x + path_col_w;
            fb.fill_rect(div_x, item_y + 2, 1, row_h.saturating_sub(6), if is_sel { 0x00FFFFFF } else { self.colors.scrollbar_border });

            // 2. Code snippet preview with match highlight
            let code_x = div_x + self.scale_coord(8);
            let max_code_chars = (input_x + list_w).saturating_sub(code_x + self.scale_coord(16)) / self.char_width().max(1);
            let snippet_chars: Vec<char> = item.line_text.chars().take(max_code_chars).collect();
            let snippet_str: String = snippet_chars.iter().collect();

            // Draw full snippet line text
            let text_color = if is_sel { 0x00E0E0E0 } else { 0x00CCCCCC };
            fb.draw_text(&snippet_str, code_x, item_text_y, text_color, None);

            // Highlight match range inside snippet
            if item.match_char_offset < snippet_chars.len() {
                let hl_start = item.match_char_offset;
                let hl_len = item.match_char_len.min(snippet_chars.len() - hl_start);
                let hl_x = code_x + hl_start * self.char_width();
                let hl_w = hl_len * self.char_width();
                let hl_str: String = snippet_chars[hl_start..hl_start + hl_len].iter().collect();
                let hl_bg = if is_sel { 0x00D35400 } else { 0x00F39C12 };
                fb.fill_rect(hl_x, item_text_y, hl_w, self.char_height(), hl_bg);
                fb.draw_text(&hl_str, hl_x, item_text_y, 0x00FFFFFF, None);
            }
        }

        // 7. Right Scrollbar in Results List if items > max_visible
        if results_len > max_visible {
            let sb_w = self.scale_coord(6).max(4);
            let sb_x = input_x + list_w - sb_w - 2;
            let sb_y = list_y + self.scale_coord(4);
            let sb_h = list_h.saturating_sub(self.scale_coord(8));

            fb.fill_rect(sb_x, sb_y, sb_w, sb_h, self.colors.scrollbar_bg);
            let thumb_h = (((max_visible as f32 / results_len as f32) * sb_h as f32).max(12.0) as usize).min(sb_h);
            let max_scroll = results_len - max_visible;
            let thumb_y = sb_y + ((scroll_offset as f32 / max_scroll as f32) * (sb_h - thumb_h) as f32) as usize;
            fb.fill_rect(sb_x, thumb_y, sb_w, thumb_h, self.colors.scrollbar_thumb);
        }

        // 8. Footer hint
        let footer_y = list_y + list_h + self.scale_coord(6);
        let hint = "↑/↓: Seleccionar  •  Enter: Abrir en línea  •  Alt+C: Mayús/Minús  •  Esc: Cerrar";
        let hint_w = hint.chars().count() * self.char_width();
        let hint_x = box_x + (box_w.saturating_sub(hint_w)) / 2;
        let max_hint_x = box_x + box_w - self.scale_coord(8);
        fb.draw_text_truncated(hint, hint_x, footer_y, max_hint_x, 0x00858585, None);
    }

    pub fn render_find_replace_dialog(
        &self,
        fb: &mut Framebuffer,
        modal: &crate::ui::prompt::PromptModal,
        close_hovered: bool,
    ) {
        let box_w = fb.width.min(self.scale_coord(540)).max(self.scale_coord(380));
        let box_h = self.scale_coord(168);
        let box_x = (fb.width.saturating_sub(box_w)) / 2;
        let box_y: usize = self.scale_coord(28);

        // 1. Semi-transparent backdrop scrim
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 100);

        // 2. Drop shadow
        if self.shadows {
            self.render_shadow(fb, box_x, box_y, box_w, box_h, 10, 8);
        } else {
            fb.fill_rect(box_x + 6, box_y + 6, box_w, box_h, 0x000A0A0A);
        }

        // 3. Title bar & window outer border
        let header_h = self.scale_coord(24);
        fb.fill_rect(box_x.saturating_sub(1), box_y.saturating_sub(1), box_w + 2, box_h + 2, self.colors.modal_border);
        fb.fill_rect(box_x, box_y, box_w, header_h, self.colors.modal_header_bg);
        fb.fill_rect(box_x, box_y + header_h, box_w, box_h.saturating_sub(header_h), self.colors.modal_bg);
        if self.rounded_corners {
            fb.draw_rounded_outline(box_x, box_y, box_w, box_h, 6, self.colors.modal_border);
        }

        fb.draw_text("Buscar y Reemplazar", box_x + self.scale_coord(10), box_y + self.scale_coord(4), 0x00FFFFFF, None);

        // Close button [×]
        let close_w = self.scale_coord(30);
        let close_x = box_x + box_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, box_y + 2, close_w - 4, header_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, box_y, close_w, header_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(11), box_y + self.scale_coord(4), 0x00FFFFFF, None);

        // Header bottom divider
        fb.fill_rect(box_x, box_y + header_h, box_w, 1, self.colors.scrollbar_border);

        // 4. Window body
        let body_y = box_y + header_h + 1;
        let input_margin = self.scale_coord(12);
        let label_w = self.scale_coord(100);
        let input_w = box_w.saturating_sub(input_margin * 2 + label_w);
        let input_h = self.scale_coord(24);

        // Row 1: Buscar
        let row1_y = body_y + self.scale_coord(12);
        fb.draw_text("Buscar:", box_x + input_margin, row1_y + (input_h.saturating_sub(self.char_height())) / 2, 0x00CCCCCC, None);

        let input1_x = box_x + input_margin + label_w;
        let border1 = if modal.active_field == 0 { self.colors.input_border } else { self.colors.scrollbar_border };
        fb.fill_rect(input1_x.saturating_sub(1), row1_y.saturating_sub(1), input_w + 2, input_h + 2, border1);
        fb.fill_rect(input1_x, row1_y, input_w, input_h, self.colors.input_bg);
        let text1_y = row1_y + (input_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(&modal.input, input1_x + self.scale_coord(6), text1_y, 0x00FFFFFF, None);

        if modal.active_field == 0 {
            let cursor_x = input1_x + self.scale_coord(6) + modal.cursor_col * self.char_width();
            if cursor_x < input1_x + input_w - self.scale_coord(4) {
                fb.fill_rect(cursor_x, text1_y, self.scale_coord(2).max(1), self.char_height(), self.colors.cursor);
            }
        }

        // Row 2: Reemplazar
        let row2_y = row1_y + input_h + self.scale_coord(10);
        fb.draw_text("Reemplazar:", box_x + input_margin, row2_y + (input_h.saturating_sub(self.char_height())) / 2, 0x00CCCCCC, None);

        let input2_x = box_x + input_margin + label_w;
        let border2 = if modal.active_field == 1 { self.colors.input_border } else { self.colors.scrollbar_border };
        fb.fill_rect(input2_x.saturating_sub(1), row2_y.saturating_sub(1), input_w + 2, input_h + 2, border2);
        fb.fill_rect(input2_x, row2_y, input_w, input_h, self.colors.input_bg);
        let text2_y = row2_y + (input_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(&modal.replace_input, input2_x + self.scale_coord(6), text2_y, 0x00FFFFFF, None);

        if modal.active_field == 1 {
            let cursor_x = input2_x + self.scale_coord(6) + modal.replace_cursor_col * self.char_width();
            if cursor_x < input2_x + input_w - self.scale_coord(4) {
                fb.fill_rect(cursor_x, text2_y, self.scale_coord(2).max(1), self.char_height(), self.colors.cursor);
            }
        }

        // Row 3: Action Buttons
        let btn_y = row2_y + input_h + self.scale_coord(16);
        let btn_h = self.scale_coord(24);

        let render_dialog_btn = |fb: &mut Framebuffer, bx: usize, by: usize, bw: usize, bh: usize, text: &str, is_primary: bool| {
            let (bg, border) = if is_primary {
                (self.colors.button_bg, self.colors.accent)
            } else {
                (self.colors.chip_bg, self.colors.scrollbar_border)
            };
            if self.rounded_corners {
                fb.fill_rounded_rect(bx, by, bw, bh, 4, bg);
                fb.draw_rounded_outline(bx, by, bw, bh, 4, border);
            } else {
                fb.fill_rect(bx.saturating_sub(1), by.saturating_sub(1), bw + 2, bh + 2, border);
                fb.fill_rect(bx, by, bw, bh, bg);
            }
            let tw = text.chars().count() * self.char_width();
            let tx = bx + (bw.saturating_sub(tw)) / 2;
            let ty = by + (bh.saturating_sub(self.char_height())) / 2;
            fb.draw_text(text, tx, ty, 0x00FFFFFF, None);
        };

        // Button 1: [< Ant]
        let b1_x = box_x + input_margin;
        let b1_w = self.scale_coord(68);
        render_dialog_btn(fb, b1_x, btn_y, b1_w, btn_h, "< Ant", false);

        // Button 2: [Sig >]
        let b2_x = b1_x + b1_w + self.scale_coord(8);
        let b2_w = self.scale_coord(68);
        render_dialog_btn(fb, b2_x, btn_y, b2_w, btn_h, "Sig >", false);

        // Button 3: [Reemplazar]
        let b3_x = b2_x + b2_w + self.scale_coord(14);
        let b3_w = self.scale_coord(98);
        render_dialog_btn(fb, b3_x, btn_y, b3_w, btn_h, "Reemplazar", false);

        // Button 4: [Reemplazar Todo]
        let b4_x = b3_x + b3_w + self.scale_coord(8);
        let b4_w = self.scale_coord(126);
        render_dialog_btn(fb, b4_x, btn_y, b4_w, btn_h, "Reemplazar Todo", true);
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

        if modal.kind == crate::ui::prompt::PromptKind::GlobalSearch {
            self.render_global_search_dialog(fb, modal, close_hovered);
            return;
        }

        let is_command_mode = modal.kind == crate::ui::prompt::PromptKind::FileFinder && modal.is_command_mode();
        let is_palette = modal.kind == crate::ui::prompt::PromptKind::CommandPalette || is_command_mode;
        let is_finder = modal.kind == crate::ui::prompt::PromptKind::FileFinder && !is_command_mode;
        let is_recent = modal.kind == crate::ui::prompt::PromptKind::RecentFiles;
        let is_projects = modal.kind == crate::ui::prompt::PromptKind::RecentProjects;
        let is_references = modal.kind == crate::ui::prompt::PromptKind::References;
        let is_code_actions = modal.kind == crate::ui::prompt::PromptKind::CodeActions;
        let is_document_symbols = modal.kind == crate::ui::prompt::PromptKind::DocumentSymbols;
        let is_rename = modal.kind == crate::ui::prompt::PromptKind::RenameSymbol;
        let is_list_modal = is_palette || is_finder || is_recent || is_projects || is_references || is_code_actions || is_document_symbols;
        let box_w = if is_recent || is_projects || is_finder || is_references || is_code_actions || is_document_symbols {
            fb.width.min(self.scale_coord(680)).max(self.scale_coord(400))
        } else {
            fb.width.min(self.scale_coord(580)).max(self.scale_coord(340))
        };
        let box_h = if is_list_modal {
            self.scale_coord(340)
        } else {
            self.scale_coord(120)
        };
        let box_x = (fb.width.saturating_sub(box_w)) / 2;
        let box_y: usize = self.scale_coord(28);

        // 1. Semi-transparent backdrop scrim
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 110);

        // 2. Drop shadow
        if self.shadows {
            self.render_shadow(fb, box_x, box_y, box_w, box_h, 10, 8);
        } else {
            fb.fill_rect(box_x + 6, box_y + 6, box_w, box_h, 0x000A0A0A);
        }

        // 3. Outer border & Window Header (Title Bar)
        let header_h = self.scale_coord(24);
        fb.fill_rect(box_x.saturating_sub(1), box_y.saturating_sub(1), box_w + 2, box_h + 2, self.colors.modal_border);
        fb.fill_rect(box_x, box_y, box_w, header_h, self.colors.modal_header_bg);

        // 4. Window Body
        let body_y = box_y + header_h + 1;
        let body_h = box_h.saturating_sub(header_h + 1);
        fb.fill_rect(box_x, body_y, box_w, body_h, self.colors.modal_bg);

        if self.rounded_corners {
            fb.draw_rounded_outline(box_x, box_y, box_w, box_h, 6, self.colors.modal_border);
        }

        let title = if is_command_mode {
            "Paleta de Comandos (Ctrl+Shift+P)"
        } else {
            modal.kind.window_title()
        };
        fb.draw_text(title, box_x + self.scale_coord(10), box_y + self.scale_coord(4), 0x00FFFFFF, None);

        // Header close button [×]
        let close_w = self.scale_coord(30);
        let close_x = box_x + box_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, box_y + 2, close_w - 4, header_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, box_y, close_w, header_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(11), box_y + self.scale_coord(4), 0x00FFFFFF, None);

        // Header bottom divider
        fb.fill_rect(box_x, box_y + header_h, box_w, 1, self.colors.scrollbar_border);

        // 5. Input Field Box inside Window
        let input_margin = self.scale_coord(12);
        let input_x = box_x + input_margin;
        let input_y = body_y + self.scale_coord(8);
        let input_w = box_w.saturating_sub(input_margin * 2);
        let input_h = self.scale_coord(24);

        if self.rounded_corners {
            fb.fill_rounded_rect(input_x, input_y, input_w, input_h, 4, self.colors.input_bg);
            fb.draw_rounded_outline(input_x, input_y, input_w, input_h, 4, self.colors.input_border);
        } else {
            fb.fill_rect(input_x.saturating_sub(1), input_y.saturating_sub(1), input_w + 2, input_h + 2, self.colors.input_border);
            fb.fill_rect(input_x, input_y, input_w, input_h, self.colors.input_bg);
        }

        // Prompt label & input text
        let label = if is_command_mode {
            "> "
        } else {
            modal.kind.label()
        };
        let text_y = input_y + (input_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(label, input_x + self.scale_coord(6), text_y, self.colors.accent, None);

        let display_input = if is_command_mode {
            modal.input.strip_prefix('>').unwrap_or(&modal.input)
        } else {
            &modal.input
        };

        let input_text_x = input_x + self.scale_coord(6) + label.chars().count() * self.char_width();
        fb.draw_text(display_input, input_text_x, text_y, 0x00FFFFFF, None);

        // Caret bar
        let display_cursor_col = if is_command_mode {
            modal.cursor_col.saturating_sub(1)
        } else {
            modal.cursor_col
        };
        let cursor_x = input_text_x + display_cursor_col * self.char_width();
        if cursor_x < input_x + input_w - self.scale_coord(4) {
            fb.fill_rect(cursor_x, text_y, self.scale_coord(2).max(1), self.char_height(), self.colors.cursor);
        }

        // 6. Content below input
        if is_palette {
            let matching = modal.matching_commands();
            let mut item_y = input_y + input_h + self.scale_coord(8);
            let item_h = (self.char_height() + self.scale_coord(8)).max(self.scale_coord(24));
            let max_visible = 9;

            if matching.is_empty() {
                fb.draw_text("(No hay comandos coincidentes)", input_x + self.scale_coord(8), item_y + self.scale_coord(6), 0x00858585, None);
            } else {
                let scroll_off = if modal.kind == crate::ui::prompt::PromptKind::FileFinder {
                    modal.scroll_offset
                } else if modal.selected_idx >= max_visible {
                    modal.selected_idx + 1 - max_visible
                } else {
                    0
                };

                for (row_idx, cmd) in matching.iter().skip(scroll_off).take(max_visible).enumerate() {
                    let item_idx = scroll_off + row_idx;
                    let is_sel = item_idx == modal.selected_idx;
                    let (row_bg, row_fg, sc_fg) = if is_sel {
                        (self.colors.accent, 0x00FFFFFF, 0x00E0E0E0)
                    } else {
                        (self.colors.modal_bg, 0x00D4D4D4, 0x00858585)
                    };

                    if is_sel && self.rounded_corners {
                        fb.fill_rounded_rect(input_x, item_y, input_w, item_h, 4, row_bg);
                    } else if is_sel {
                        fb.fill_rect(input_x, item_y, input_w, item_h, row_bg);
                    }

                    let row_text_y = item_y + (item_h.saturating_sub(self.char_height())) / 2;

                    if !cmd.shortcut.is_empty() {
                        let sc_w = cmd.shortcut.chars().count() * self.char_width();
                        let sc_x = input_x + input_w - sc_w - self.scale_coord(8);
                        fb.draw_text_clipped(cmd.shortcut, sc_x, row_text_y, input_x + input_w - self.scale_coord(4), sc_fg, None);
                        let name_max_x = sc_x.saturating_sub(self.scale_coord(8));
                        fb.draw_text_truncated(&cmd.name, input_x + self.scale_coord(8), row_text_y, name_max_x, row_fg, None);
                    } else {
                        let name_max_x = input_x + input_w - self.scale_coord(8);
                        fb.draw_text_truncated(&cmd.name, input_x + self.scale_coord(8), row_text_y, name_max_x, row_fg, None);
                    }

                    item_y += item_h;
                }
            }

            // Footer hint
            let hint = "↑/↓: Seleccionar  •  Enter: Ejecutar  •  Esc: Cerrar";
            let hint_w = hint.chars().count() * self.char_width();
            let hint_x = box_x + (box_w.saturating_sub(hint_w)) / 2;
            let hint_y = box_y + box_h - self.scale_coord(18);
            fb.draw_text_truncated(hint, hint_x, hint_y, box_x + box_w - self.scale_coord(8), 0x00858585, None);
        } else if is_finder {
            let matching = &modal.file_finder_results;
            let mut item_y = input_y + input_h + self.scale_coord(8);
            let item_h = (self.char_height() + self.scale_coord(10)).max(self.scale_coord(26));
            let max_visible = 9;

            if matching.is_empty() {
                fb.draw_text("(No se encontraron archivos coincidentes)", input_x + self.scale_coord(8), item_y + self.scale_coord(6), 0x00858585, None);
            } else {
                for (row_idx, item) in matching.iter().skip(modal.scroll_offset).take(max_visible).enumerate() {
                    let item_idx = modal.scroll_offset + row_idx;
                    let is_sel = item_idx == modal.selected_idx;
                    let (row_bg, row_fg, dir_fg, hl_color) = if is_sel {
                        (self.colors.accent, 0x00FFFFFF, 0x00E0E0E0, 0x00FFFF70)
                    } else {
                        (self.colors.modal_bg, 0x00FFFFFF, 0x00858585, 0x00FFD700)
                    };

                    if is_sel && self.rounded_corners {
                        fb.fill_rounded_rect(input_x, item_y, input_w, item_h, 4, row_bg);
                    } else if is_sel {
                        fb.fill_rect(input_x, item_y, input_w, item_h, row_bg);
                    }

                    let row_text_y = item_y + (item_h.saturating_sub(self.char_height())) / 2;

                    // 1. Extension badge
                    let ext_str = item.file.extension.to_uppercase();
                    let badge_text = if ext_str.is_empty() {
                        "[FILE]".to_string()
                    } else if ext_str.len() > 4 {
                        format!("[{}]", &ext_str[..4])
                    } else {
                        format!("[{}]", ext_str)
                    };

                    let ext_color = match item.file.extension.as_str() {
                        "rs" => self.colors.syntax_number,
                        "toml" | "ini" | "json" | "yaml" | "yml" => self.colors.syntax_type,
                        "md" | "txt" => self.colors.syntax_string,
                        "c" | "cpp" | "h" | "hpp" | "sql" => self.colors.syntax_keyword,
                        "sh" | "bash" | "zsh" => self.colors.bracket_1,
                        "py" => self.colors.syntax_string,
                        "html" | "css" | "js" | "ts" => self.colors.accent,
                        _ => 0x00858585,
                    };

                    let badge_fg = if is_sel { 0x00E0E0E0 } else { ext_color };
                    let badge_x = input_x + self.scale_coord(8);
                    fb.draw_text(&badge_text, badge_x, row_text_y, badge_fg, None);

                    let fn_x = badge_x + self.scale_coord(52);

                    // 2. Status badge at right edge (Abierto / Reciente)
                    let status_badge_info = if item.file.is_open {
                        Some(("• Abierto", if is_sel { 0x00A6E3A1 } else { self.colors.syntax_string }))
                    } else if item.file.is_recent {
                        Some(("• Reciente", if is_sel { 0x00B0D0FF } else { 0x00858585 }))
                    } else {
                        None
                    };

                    let max_row_x = if let Some((tag, color)) = status_badge_info {
                        let tag_w = tag.chars().count() * self.char_width();
                        let tag_x = input_x + input_w - tag_w - self.scale_coord(8);
                        fb.draw_text(tag, tag_x, row_text_y, color, None);
                        tag_x.saturating_sub(self.scale_coord(10))
                    } else {
                        input_x + input_w - self.scale_coord(8)
                    };

                    // 3. Highlighted filename
                    let rel_chars: Vec<char> = item.file.relative_path.chars().collect();
                    let fn_start_char = rel_chars
                        .iter()
                        .rposition(|&c| c == '/' || c == '\\')
                        .map(|idx| idx + 1)
                        .unwrap_or(0);

                    let mut cur_fn_x = fn_x;
                    for (c_idx, ch) in item.file.file_name.chars().enumerate() {
                        if cur_fn_x + self.char_width() > max_row_x {
                            break;
                        }
                        let is_matched = item.matched_indices.contains(&(fn_start_char + c_idx));
                        let ch_color = if is_matched { hl_color } else { row_fg };
                        fb.draw_char(ch, cur_fn_x, row_text_y, ch_color, None);
                        cur_fn_x += self.char_width();
                    }

                    // 4. Directory path with matched letters highlighted
                    let parent_str = if fn_start_char > 0 {
                        &item.file.relative_path[..fn_start_char.saturating_sub(1)]
                    } else {
                        ""
                    };

                    if !parent_str.is_empty() {
                        let dir_x = cur_fn_x + self.scale_coord(12);
                        if dir_x < max_row_x {
                            let mut cur_dir_x = dir_x;
                            for (p_idx, p_ch) in parent_str.chars().enumerate() {
                                if cur_dir_x + self.char_width() > max_row_x {
                                    break;
                                }
                                let is_matched = item.matched_indices.contains(&p_idx);
                                let p_color = if is_matched { hl_color } else { dir_fg };
                                fb.draw_char(p_ch, cur_dir_x, row_text_y, p_color, None);
                                cur_dir_x += self.char_width();
                            }
                        }
                    }

                    item_y += item_h;
                }
            }

            // Footer hint
            let total_files = modal.project_files.len();
            let match_count = modal.file_finder_results.len();
            let hint = if modal.input.is_empty() {
                format!("{} archivos  •  ↑/↓: Seleccionar  •  Enter: Abrir  •  >: Comandos  •  Esc: Cerrar", total_files)
            } else {
                format!("{}/{} archivos  •  ↑/↓: Seleccionar  •  Enter: Abrir  •  Esc: Cerrar", match_count, total_files)
            };
            let hint_w = hint.chars().count() * self.char_width();
            let hint_x = box_x + (box_w.saturating_sub(hint_w)) / 2;
            let hint_y = box_y + box_h - self.scale_coord(18);
            fb.draw_text_truncated(&hint, hint_x, hint_y, box_x + box_w - self.scale_coord(8), 0x00858585, None);
        } else if is_recent {
            let matching = modal.matching_recent_files();
            let mut item_y = input_y + input_h + self.scale_coord(8);
            let item_h = (self.char_height() + self.scale_coord(10)).max(self.scale_coord(26));
            let max_visible = 9;

            if matching.is_empty() {
                fb.draw_text("(No hay archivos recientes coincidentes)", input_x + self.scale_coord(8), item_y + self.scale_coord(6), 0x00858585, None);
            } else {
                for (idx, path) in matching.iter().take(max_visible).enumerate() {
                    let is_sel = idx == modal.selected_idx;
                    let (row_bg, row_fg, dir_fg) = if is_sel {
                        (self.colors.accent, 0x00FFFFFF, 0x00E0E0E0)
                    } else {
                        (self.colors.modal_bg, 0x00FFFFFF, 0x00858585)
                    };

                    if is_sel && self.rounded_corners {
                        fb.fill_rounded_rect(input_x, item_y, input_w, item_h, 4, row_bg);
                    } else {
                        fb.fill_rect(input_x, item_y, input_w, item_h, row_bg);
                    }

                    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let parent_str = path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

                    let row_text_y = item_y + (item_h.saturating_sub(self.char_height())) / 2;
                    fb.draw_text(file_name, input_x + self.scale_coord(8), row_text_y, row_fg, None);

                    let fn_w = file_name.chars().count() * self.char_width();
                    let dir_x = input_x + self.scale_coord(8) + fn_w + self.scale_coord(12);
                    let max_dir_chars = input_w.saturating_sub(fn_w + self.scale_coord(24)) / self.char_width().max(1);
                    let display_dir = if parent_str.chars().count() > max_dir_chars && max_dir_chars > 3 {
                        format!("...{}", parent_str.chars().skip(parent_str.chars().count() - (max_dir_chars - 3)).collect::<String>())
                    } else {
                        parent_str
                    };
                    fb.draw_text(&display_dir, dir_x, row_text_y, dir_fg, None);

                    item_y += item_h;
                }
            }

            // Footer hint
            let hint = "↑/↓: Seleccionar  •  Enter: Abrir archivo  •  Esc: Cancelar";
            let hint_x = box_x + (box_w.saturating_sub(hint.chars().count() * self.char_width())) / 2;
            let hint_y = box_y + box_h - self.scale_coord(18);
            fb.draw_text(hint, hint_x, hint_y, 0x00858585, None);
        } else if is_projects {
            let matching = modal.matching_projects();
            let mut item_y = input_y + input_h + self.scale_coord(8);
            let item_h = (self.char_height() + self.scale_coord(10)).max(self.scale_coord(26));
            let max_visible = 9;

            if matching.is_empty() {
                fb.draw_text("(No hay proyectos recientes coincidentes)", input_x + self.scale_coord(8), item_y + self.scale_coord(6), 0x00858585, None);
            } else {
                for (idx, project) in matching.iter().take(max_visible).enumerate() {
                    let is_sel = idx == modal.selected_idx;
                    let (row_bg, row_fg, dir_fg) = if is_sel {
                        (self.colors.accent, 0x00FFFFFF, 0x00E0E0E0)
                    } else {
                        (self.colors.modal_bg, 0x00FFFFFF, 0x00858585)
                    };

                    if is_sel && self.rounded_corners {
                        fb.fill_rounded_rect(input_x, item_y, input_w, item_h, 4, row_bg);
                    } else if is_sel {
                        fb.fill_rect(input_x, item_y, input_w, item_h, row_bg);
                    }

                    let row_text_y = item_y + (item_h.saturating_sub(self.char_height())) / 2;

                    // 1. Kind tag chip (e.g. [Rust], [Node], [Python], [Carpeta])
                    let tag_fg = if is_sel { 0x00FFFFFF } else { project.tag_color };
                    let tag_x = input_x + self.scale_coord(8);
                    fb.draw_text(&project.kind_tag, tag_x, row_text_y, tag_fg, None);

                    let name_x = tag_x + self.scale_coord(64);

                    // 2. Project Name
                    fb.draw_text(&project.name, name_x, row_text_y, row_fg, None);

                    // 3. Project Directory Path
                    let path_str = project.path.to_string_lossy();
                    let name_w = project.name.chars().count() * self.char_width();
                    let dir_x = name_x + name_w + self.scale_coord(14);
                    let max_dir_chars = input_w.saturating_sub(name_w + self.scale_coord(90)) / self.char_width().max(1);
                    let display_dir = if path_str.chars().count() > max_dir_chars && max_dir_chars > 3 {
                        format!("...{}", path_str.chars().skip(path_str.chars().count() - (max_dir_chars - 3)).collect::<String>())
                    } else {
                        path_str.to_string()
                    };
                    fb.draw_text(&display_dir, dir_x, row_text_y, dir_fg, None);

                    item_y += item_h;
                }
            }

            // Footer hint
            let hint = "↑/↓: Seleccionar  •  Enter: Abrir proyecto  •  Del: Eliminar  •  Esc: Cancelar";
            let hint_w = hint.chars().count() * self.char_width();
            let hint_x = box_x + (box_w.saturating_sub(hint_w)) / 2;
            let hint_y = box_y + box_h - self.scale_coord(18);
            fb.draw_text_truncated(hint, hint_x, hint_y, box_x + box_w - self.scale_coord(8), 0x00858585, None);
        } else if is_references {
            let matching = modal.matching_references();
            let mut item_y = input_y + input_h + self.scale_coord(8);
            let item_h = (self.char_height() + self.scale_coord(10)).max(self.scale_coord(26));
            let max_visible = 9;

            if matching.is_empty() {
                fb.draw_text("(No se encontraron referencias para este símbolo)", input_x + self.scale_coord(8), item_y + self.scale_coord(6), 0x00858585, None);
            } else {
                for (idx, item) in matching.iter().take(max_visible).enumerate() {
                    let is_sel = idx == modal.selected_idx;
                    let (row_bg, row_fg, line_fg, code_fg) = if is_sel {
                        (self.colors.accent, 0x00FFFFFF, 0x00E0E0E0, 0x00FFFF80)
                    } else {
                        (self.colors.modal_bg, 0x00FFFFFF, 0x007AA2F7, 0x00D4D4D4)
                    };

                    if is_sel && self.rounded_corners {
                        fb.fill_rounded_rect(input_x, item_y, input_w, item_h, 4, row_bg);
                    } else if is_sel {
                        fb.fill_rect(input_x, item_y, input_w, item_h, row_bg);
                    }

                    let row_text_y = item_y + (item_h.saturating_sub(self.char_height())) / 2;

                    // 1. File name / path
                    let fname = item.path.file_name().and_then(|n| n.to_str()).unwrap_or("archivo");
                    let file_x = input_x + self.scale_coord(8);
                    fb.draw_text(fname, file_x, row_text_y, row_fg, None);

                    let fn_w = fname.chars().count() * self.char_width();
                    let line_badge = format!(":{}:{}", item.range.start.line + 1, item.range.start.character + 1);
                    let line_x = file_x + fn_w;
                    fb.draw_text(&line_badge, line_x, row_text_y, line_fg, None);

                    // 2. Code snippet preview
                    let lb_w = line_badge.chars().count() * self.char_width();
                    let code_x = line_x + lb_w + self.scale_coord(14);
                    fb.draw_text_truncated(&item.line_text, code_x, row_text_y, input_x + input_w - self.scale_coord(8), code_fg, None);

                    item_y += item_h;
                }
            }

            // Footer hint
            let hint = "↑/↓: Seleccionar  •  Enter: Ir a referencia  •  Esc: Cerrar";
            let hint_w = hint.chars().count() * self.char_width();
            let hint_x = box_x + (box_w.saturating_sub(hint_w)) / 2;
            let hint_y = box_y + box_h - self.scale_coord(18);
            fb.draw_text_truncated(hint, hint_x, hint_y, box_x + box_w - self.scale_coord(8), 0x00858585, None);
        } else if is_code_actions {
            let matching = modal.matching_code_actions();
            let mut item_y = input_y + input_h + self.scale_coord(8);
            let item_h = (self.char_height() + self.scale_coord(10)).max(self.scale_coord(26));
            let max_visible = 9;

            if matching.is_empty() {
                fb.draw_text("(No hay acciones ni correcciones disponibles)", input_x + self.scale_coord(8), item_y + self.scale_coord(6), 0x00858585, None);
            } else {
                for (idx, action) in matching.iter().take(max_visible).enumerate() {
                    let is_sel = idx == modal.selected_idx;
                    let (row_bg, row_fg) = if is_sel {
                        (self.colors.accent, 0x00FFFFFF)
                    } else {
                        (self.colors.modal_bg, 0x00E0E0E0)
                    };

                    if is_sel && self.rounded_corners {
                        fb.fill_rounded_rect(input_x, item_y, input_w, item_h, 4, row_bg);
                    } else if is_sel {
                        fb.fill_rect(input_x, item_y, input_w, item_h, row_bg);
                    }

                    let row_text_y = item_y + (item_h.saturating_sub(self.char_height())) / 2;

                    // 1. Kind Badge & Color
                    let kind_str = action.kind.as_deref().unwrap_or("");
                    let (badge, badge_color) = if action.is_preferred {
                        ("[★ Preferido]", 0x00FFD700)
                    } else if kind_str.contains("quickfix") {
                        ("[💡 QuickFix]", 0x007EE787)
                    } else if kind_str.contains("refactor") {
                        ("[🛠️ Refactor]", 0x007AA2F7)
                    } else if kind_str.contains("source") {
                        ("[📦 Source]", 0x00DDB6F2)
                    } else {
                        ("[⚡ Acción]", 0x00E0AF68)
                    };

                    let badge_fg = if is_sel { 0x00FFFFFF } else { badge_color };
                    let badge_x = input_x + self.scale_coord(8);
                    fb.draw_text(badge, badge_x, row_text_y, badge_fg, None);

                    // 2. Action Title
                    let bw = badge.chars().count() * self.char_width();
                    let title_x = badge_x + bw + self.scale_coord(10);
                    fb.draw_text_truncated(&action.title, title_x, row_text_y, input_x + input_w - self.scale_coord(8), row_fg, None);

                    item_y += item_h;
                }
            }

            // Footer hint
            let hint = "↑/↓: Seleccionar  •  Enter: Aplicar corrección  •  Esc: Cerrar";
            let hint_w = hint.chars().count() * self.char_width();
            let hint_x = box_x + (box_w.saturating_sub(hint_w)) / 2;
            let hint_y = box_y + box_h - self.scale_coord(18);
            fb.draw_text_truncated(hint, hint_x, hint_y, box_x + box_w - self.scale_coord(8), 0x00858585, None);
        } else if is_document_symbols {
            let matching = modal.matching_document_symbols();
            let mut item_y = input_y + input_h + self.scale_coord(8);
            let item_h = (self.char_height() + self.scale_coord(10)).max(self.scale_coord(26));
            let max_visible = 9;

            if matching.is_empty() {
                fb.draw_text("(No se encontraron símbolos coincidentes)", input_x + self.scale_coord(8), item_y + self.scale_coord(6), 0x00858585, None);
            } else {
                for (idx, sym) in matching.iter().take(max_visible).enumerate() {
                    let is_sel = idx == modal.selected_idx;
                    let (row_bg, row_fg, detail_fg) = if is_sel {
                        (self.colors.accent, 0x00FFFFFF, 0x00E0E0E0)
                    } else {
                        (self.colors.modal_bg, 0x00FFFFFF, 0x00858585)
                    };

                    if is_sel && self.rounded_corners {
                        fb.fill_rounded_rect(input_x, item_y, input_w, item_h, 4, row_bg);
                    } else if is_sel {
                        fb.fill_rect(input_x, item_y, input_w, item_h, row_bg);
                    }

                    let row_text_y = item_y + (item_h.saturating_sub(self.char_height())) / 2;

                    // 1. Kind Badge (e.g. [fn], [struct], [trait], [const])
                    let badge_text = sym.kind.badge_str();
                    let badge_color = match sym.kind {
                        crate::editor::lsp::SymbolKind::Function | crate::editor::lsp::SymbolKind::Method => self.colors.syntax_keyword,
                        crate::editor::lsp::SymbolKind::Struct | crate::editor::lsp::SymbolKind::Class | crate::editor::lsp::SymbolKind::Interface => self.colors.syntax_type,
                        crate::editor::lsp::SymbolKind::Enum | crate::editor::lsp::SymbolKind::EnumMember => self.colors.syntax_number,
                        crate::editor::lsp::SymbolKind::Constant => self.colors.syntax_string,
                        crate::editor::lsp::SymbolKind::Module | crate::editor::lsp::SymbolKind::Namespace => self.colors.bracket_1,
                        _ => 0x00858585,
                    };
                    let badge_fg = if is_sel { 0x00FFFFFF } else { badge_color };

                    let indent_px = sym.depth.min(6) * self.scale_coord(12);
                    let badge_x = input_x + self.scale_coord(8) + indent_px;
                    fb.draw_text(badge_text, badge_x, row_text_y, badge_fg, None);

                    let bw = badge_text.chars().count() * self.char_width();
                    let name_x = badge_x + bw + self.scale_coord(8);

                    // 2. Line number on the right
                    let line_str = format!("L{}", sym.range.start.line + 1);
                    let lw = line_str.chars().count() * self.char_width();
                    let line_x = input_x + input_w - lw - self.scale_coord(8);
                    let line_fg = if is_sel { 0x00E0E0E0 } else { 0x007AA2F7 };
                    fb.draw_text(&line_str, line_x, row_text_y, line_fg, None);

                    // 3. Symbol name + detail
                    let max_text_x = line_x.saturating_sub(self.scale_coord(8));
                    fb.draw_text_truncated(&sym.name, name_x, row_text_y, max_text_x, row_fg, None);

                    if !sym.detail.is_empty() {
                        let nw = sym.name.chars().count() * self.char_width();
                        let det_x = name_x + nw + self.scale_coord(8);
                        if det_x < max_text_x {
                            fb.draw_text_truncated(&sym.detail, det_x, row_text_y, max_text_x, detail_fg, None);
                        }
                    }

                    item_y += item_h;
                }
            }

            // Footer hint
            let hint = "↑/↓: Seleccionar  •  Enter: Ir a símbolo  •  Esc: Cerrar";
            let hint_w = hint.chars().count() * self.char_width();
            let hint_x = box_x + (box_w.saturating_sub(hint_w)) / 2;
            let hint_y = box_y + box_h - self.scale_coord(18);
            fb.draw_text_truncated(hint, hint_x, hint_y, box_x + box_w - self.scale_coord(8), 0x00858585, None);
        } else if is_rename {
            let hint = "Enter: Renombrar símbolo en el proyecto  •  Esc: Cancelar";
            let hint_w = hint.chars().count() * self.char_width();
            let hint_x = box_x + (box_w.saturating_sub(hint_w)) / 2;
            let hint_y = box_y + box_h - self.scale_coord(18);
            fb.draw_text_truncated(hint, hint_x, hint_y, box_x + box_w - self.scale_coord(8), 0x00858585, None);
        } else {
            // Footer hint for Find
            let hint = "Enter: Confirmar  •  Esc: Cancelar";
            let hint_x = box_x + (box_w.saturating_sub(hint.chars().count() * self.char_width())) / 2;
            let hint_y = box_y + box_h - self.scale_coord(22);
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
        // Semi-transparent backdrop scrim
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 110);

        let box_w = fb.width.min(self.scale_coord(680)).max(self.scale_coord(420));
        let box_h = fb.height.min(self.scale_coord(480)).max(self.scale_coord(360));
        let box_x = (fb.width.saturating_sub(box_w)) / 2;
        let box_y = (fb.height.saturating_sub(box_h)) / 2;

        // 1. Soft Drop shadow
        if self.shadows {
            self.render_shadow(fb, box_x, box_y, box_w, box_h, 12, 8);
        } else {
            fb.fill_rect(box_x + 6, box_y + 6, box_w, box_h, 0x000A0A0A);
        }

        // 2. Outer border & Window Header (Title Bar)
        let header_h = self.scale_coord(26);
        fb.fill_rect(box_x.saturating_sub(1), box_y.saturating_sub(1), box_w + 2, box_h + 2, self.colors.modal_border);
        fb.fill_rect(box_x, box_y, box_w, header_h, self.colors.modal_header_bg);

        // 4. Window Body
        let body_y = box_y + header_h + 1;
        let body_h = box_h.saturating_sub(header_h + 1);
        fb.fill_rect(box_x, body_y, box_w, body_h, self.colors.modal_bg);

        if self.rounded_corners {
            fb.draw_rounded_outline(box_x, box_y, box_w, box_h, 6, self.colors.modal_border);
        }

        fb.draw_text(modal.kind.window_title(), box_x + self.scale_coord(10), box_y + self.scale_coord(5), 0x00FFFFFF, None);

        // Header close button [×]
        let close_w = self.scale_coord(32);
        let close_x = box_x + box_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, box_y + 2, close_w - 4, header_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, box_y, close_w, header_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(12), box_y + self.scale_coord(5), 0x00FFFFFF, None);

        // Header bottom divider
        fb.fill_rect(box_x, box_y + header_h, box_w, 1, self.colors.scrollbar_border);

        // 5. Current Directory Bar
        let dir_bar_y = body_y + self.scale_coord(6);
        let dir_bar_h = self.scale_coord(24);
        let dir_bar_w = box_w.saturating_sub(self.scale_coord(16));
        if self.rounded_corners {
            fb.fill_rounded_rect(box_x + self.scale_coord(8), dir_bar_y, dir_bar_w, dir_bar_h, 4, self.colors.input_bg);
        } else {
            fb.fill_rect(box_x + self.scale_coord(8), dir_bar_y, dir_bar_w, dir_bar_h, self.colors.input_bg);
        }
        fb.draw_text("Carpeta:", box_x + self.scale_coord(14), dir_bar_y + self.scale_coord(4), self.colors.accent, None);

        let path_str = modal.current_dir.display().to_string();
        let max_path_chars = (box_w.saturating_sub(self.scale_coord(190))) / self.char_width().max(1);
        let display_path = if path_str.chars().count() > max_path_chars && max_path_chars > 3 {
            let skip = path_str.chars().count().saturating_sub(max_path_chars - 3);
            format!("...{}", path_str.chars().skip(skip).collect::<String>())
        } else {
            path_str
        };
        fb.draw_text(&display_path, box_x + self.scale_coord(14) + 9 * self.char_width(), dir_bar_y + self.scale_coord(4), 0x00FFFFFF, None);

        // [Subir] button
        let up_btn_w = self.scale_coord(64);
        let up_btn_x = box_x + box_w - up_btn_w - self.scale_coord(12);
        if self.rounded_corners {
            fb.fill_rounded_rect(up_btn_x, dir_bar_y + self.scale_coord(2), up_btn_w, self.scale_coord(20), 3, self.colors.chip_bg);
            fb.draw_rounded_outline(up_btn_x, dir_bar_y + self.scale_coord(2), up_btn_w, self.scale_coord(20), 3, self.colors.scrollbar_border);
        } else {
            fb.fill_rect(up_btn_x, dir_bar_y + self.scale_coord(2), up_btn_w, self.scale_coord(20), self.colors.chip_bg);
        }
        fb.draw_text("[Subir]", up_btn_x + self.scale_coord(6), dir_bar_y + self.scale_coord(4), 0x00FFFFFF, None);

        // 6. Bookmarks Bar
        let bm_bar_y = dir_bar_y + dir_bar_h + self.scale_coord(4);
        let bookmarks = crate::ui::prompt::get_quick_bookmarks();
        let mut cur_bm_x = box_x + self.scale_coord(8);
        for bm in &bookmarks {
            let is_active = modal.current_dir == bm.path;
            let bm_text = format!("[{}]", bm.label);
            let bm_w = bm_text.chars().count() * self.char_width() + self.scale_coord(8);
            if cur_bm_x + bm_w < box_x + box_w - self.scale_coord(8) {
                let bg = if is_active { self.colors.accent } else { self.colors.chip_bg };
                let fg = if is_active { 0x00FFFFFF } else { 0x00CCCCCC };
                if self.rounded_corners {
                    fb.fill_rounded_rect(cur_bm_x, bm_bar_y, bm_w, self.scale_coord(20), 3, bg);
                } else {
                    fb.fill_rect(cur_bm_x, bm_bar_y, bm_w, self.scale_coord(20), bg);
                }
                fb.draw_text(&bm_text, cur_bm_x + self.scale_coord(4), bm_bar_y + self.scale_coord(2), fg, None);
                cur_bm_x += bm_w + self.scale_coord(6);
            }
        }

        // 7. Directory Entries List
        let list_y = bm_bar_y + self.scale_coord(26);
        let list_h = box_h.saturating_sub(self.scale_coord(190));
        let list_w = box_w.saturating_sub(self.scale_coord(16));
        let list_x = box_x + self.scale_coord(8);

        if self.rounded_corners {
            fb.fill_rounded_rect(list_x, list_y, list_w, list_h, 4, self.colors.input_bg);
            fb.draw_rounded_outline(list_x, list_y, list_w, list_h, 4, self.colors.scrollbar_border);
        } else {
            fb.fill_rect(list_x.saturating_sub(1), list_y.saturating_sub(1), list_w + 2, list_h + 2, self.colors.scrollbar_border);
            fb.fill_rect(list_x, list_y, list_w, list_h, self.colors.input_bg);
        }

        let row_h = (self.char_height() + self.scale_coord(6)).max(self.scale_coord(22));
        let max_visible = list_h / row_h;

        if modal.dir_entries.is_empty() {
            fb.draw_text("(Carpeta vacía o sin permisos de lectura)", list_x + self.scale_coord(12), list_y + self.scale_coord(10), 0x00858585, None);
        } else {
            for (idx, entry) in modal.dir_entries.iter().skip(modal.scroll_offset).take(max_visible).enumerate() {
                let actual_idx = modal.scroll_offset + idx;
                let is_selected = actual_idx == modal.selected_entry_idx;
                let ry = list_y + idx * row_h;

                let row_bg = if is_selected { self.colors.accent } else { self.colors.input_bg };
                if is_selected && self.rounded_corners {
                    fb.fill_rounded_rect(list_x + 2, ry, list_w.saturating_sub(4), row_h, 3, row_bg);
                } else {
                    fb.fill_rect(list_x, ry, list_w, row_h, row_bg);
                }

                let tag = if entry.is_dir { "[DIR] " } else { "      " };
                let tag_fg = if is_selected { 0x00FFFFFF } else { self.colors.syntax_type };
                let name_fg = if is_selected { 0x00FFFFFF } else if entry.is_dir { 0x00FFFFFF } else { 0x00D4D4D4 };

                fb.draw_text(tag, list_x + self.scale_coord(8), ry + (row_h.saturating_sub(self.char_height())) / 2, tag_fg, None);
                fb.draw_text(&entry.name, list_x + self.scale_coord(8) + 6 * self.char_width(), ry + (row_h.saturating_sub(self.char_height())) / 2, name_fg, None);
            }
        }

        // 8. Filename Input Field
        let input_y = box_y + box_h - self.scale_coord(76);
        fb.draw_text("Nombre de archivo:", list_x, input_y, 0x00D4D4D4, None);

        let input_box_y = input_y + self.scale_coord(18);
        let input_box_h = self.scale_coord(24);
        if self.rounded_corners {
            fb.fill_rounded_rect(list_x, input_box_y, list_w, input_box_h, 4, self.colors.input_bg);
            fb.draw_rounded_outline(list_x, input_box_y, list_w, input_box_h, 4, self.colors.input_border);
        } else {
            fb.fill_rect(list_x.saturating_sub(1), input_box_y.saturating_sub(1), list_w + 2, input_box_h + 2, self.colors.input_border);
            fb.fill_rect(list_x, input_box_y, list_w, input_box_h, self.colors.input_bg);
        }

        let input_text_y = input_box_y + (input_box_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(&modal.input, list_x + self.scale_coord(8), input_text_y, 0x00FFFFFF, None);

        // Caret
        let cursor_x = list_x + self.scale_coord(8) + modal.cursor_col * self.char_width();
        if cursor_x < list_x + list_w - self.scale_coord(4) {
            fb.fill_rect(cursor_x, input_text_y, self.scale_coord(2).max(1), self.char_height(), self.colors.cursor);
        }

        // 9. Bottom Buttons
        let btn_bar_y = box_y + box_h - self.scale_coord(30);
        let is_open_folder = modal.kind == crate::ui::prompt::PromptKind::OpenFolder;
        let action_btn_text = if modal.kind == crate::ui::prompt::PromptKind::SaveAs {
            "Guardar"
        } else if is_open_folder {
            "Abrir Carpeta"
        } else {
            "Abrir"
        };
        let action_btn_w = self.scale_coord(if is_open_folder { 120 } else { 88 });
        let cancel_btn_w = self.scale_coord(80);

        let cancel_btn_x = box_x + box_w - cancel_btn_w - self.scale_coord(12);
        let action_btn_x = cancel_btn_x - action_btn_w - self.scale_coord(8);
        let btn_h = self.scale_coord(24);

        // Action button [Guardar] / [Abrir] / [Abrir Carpeta]
        if self.rounded_corners {
            fb.fill_rounded_rect(action_btn_x, btn_bar_y, action_btn_w, btn_h, 4, self.colors.button_bg);
            fb.draw_rounded_outline(action_btn_x, btn_bar_y, action_btn_w, btn_h, 4, self.colors.accent);
        } else {
            fb.fill_rect(action_btn_x.saturating_sub(1), btn_bar_y.saturating_sub(1), action_btn_w + 2, btn_h, self.colors.accent);
            fb.fill_rect(action_btn_x, btn_bar_y, action_btn_w, btn_h.saturating_sub(2), self.colors.button_bg);
        }
        let at_x = action_btn_x + (action_btn_w.saturating_sub(action_btn_text.chars().count() * self.char_width())) / 2;
        fb.draw_text(action_btn_text, at_x, btn_bar_y + (btn_h.saturating_sub(self.char_height())) / 2, self.colors.button_fg, None);

        // Cancel button [Cancelar]
        if self.rounded_corners {
            fb.fill_rounded_rect(cancel_btn_x, btn_bar_y, cancel_btn_w, btn_h, 4, self.colors.chip_bg);
            fb.draw_rounded_outline(cancel_btn_x, btn_bar_y, cancel_btn_w, btn_h, 4, self.colors.scrollbar_border);
        } else {
            fb.fill_rect(cancel_btn_x.saturating_sub(1), btn_bar_y.saturating_sub(1), cancel_btn_w + 2, btn_h, self.colors.scrollbar_border);
            fb.fill_rect(cancel_btn_x, btn_bar_y, cancel_btn_w, btn_h.saturating_sub(2), self.colors.chip_bg);
        }
        let ct_x = cancel_btn_x + (cancel_btn_w.saturating_sub(8 * self.char_width())) / 2;
        fb.draw_text("Cancelar", ct_x, btn_bar_y + (btn_h.saturating_sub(self.char_height())) / 2, 0x00CCCCCC, None);

        // Footer hint
        let hint = if is_open_folder {
            "Seleccione una carpeta y pulse 'Abrir Carpeta' o Enter"
        } else {
            "Doble clic o Enter para abrir carpeta"
        };
        fb.draw_text(hint, list_x, btn_bar_y + self.scale_coord(4), 0x00858585, None);
    }

    /// Renders the official Rustext logo: Concept 1 "The Faceted Ribbon R".
    /// High-performance geometric 3D faceted ribbon rendered using analytical subpixel antialiasing.

    pub fn render_about_dialog(
        &self,
        fb: &mut Framebuffer,
        close_hovered: bool,
        ok_hovered: bool,
    ) {
        // Semi-transparent backdrop tint (darken)
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 120);

        // Dialog box dimensions
        let win_w = self.scale_coord(640).min(fb.width.saturating_sub(self.scale_coord(20)));
        let win_h = self.scale_coord(450).min(fb.height.saturating_sub(self.scale_coord(20)));
        let win_x = (fb.width.saturating_sub(win_w)) / 2;
        let win_y = (fb.height.saturating_sub(win_h)) / 2;

        // Drop shadow
        if self.shadows {
            self.render_shadow(fb, win_x, win_y, win_w, win_h, 12, 8);
        } else {
            fb.fill_rect(win_x + 6, win_y + 6, win_w, win_h, 0x000A0A0A);
        }

        // Outer border & Header
        let title_h = self.scale_coord(28);
        fb.fill_rect(win_x.saturating_sub(1), win_y.saturating_sub(1), win_w + 2, win_h + 2, self.colors.modal_border);
        fb.fill_rect(win_x, win_y, win_w, title_h, self.colors.modal_header_bg);

        // Window Body background
        let body_y = win_y + title_h + 1;
        let body_h = win_h.saturating_sub(title_h + 1);
        fb.fill_rect(win_x, body_y, win_w, body_h, self.colors.modal_bg);

        if self.rounded_corners {
            fb.draw_rounded_outline(win_x, win_y, win_w, win_h, 6, self.colors.modal_border);
        }

        fb.draw_text("Rustext - Acerca de", win_x + self.scale_coord(12), win_y + self.scale_coord(6), 0x00FFFFFF, None);

        // Close button [×] on the right
        let close_w = self.scale_coord(36);
        let close_x = win_x + win_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, win_y + 2, close_w - 4, title_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, win_y, close_w, title_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(14), win_y + self.scale_coord(6), 0x00FFFFFF, None);

        // Title bar separator
        fb.fill_rect(win_x, win_y + title_h, win_w, 1, self.colors.scrollbar_border);

        // Rustext Logo Badge (Concept 1: The Faceted Ribbon R)
        let logo_size = self.scale_coord(64);
        let logo_x = win_x + (win_w.saturating_sub(logo_size)) / 2;
        let logo_y = body_y + self.scale_coord(10);
        self.render_logo(fb, logo_x, logo_y, logo_size, true);

        // Subtitle & Version
        let mut cur_y = logo_y + logo_size + self.scale_coord(8);
        let title_str = concat!("Rustext Editor v", env!("CARGO_PKG_VERSION"));
        let title_x = win_x + (win_w.saturating_sub(title_str.chars().count() * self.char_width())) / 2;
        fb.draw_text(title_str, title_x, cur_y, 0x00FFFFFF, None);
        cur_y += self.char_height() + self.scale_coord(4);

        let tag_str = "100% Pure Rust • Zero Dependencias Externas";
        let tag_x = win_x + (win_w.saturating_sub(tag_str.chars().count() * self.char_width())) / 2;
        fb.draw_text(tag_str, tag_x, cur_y, 0x004EC9B0, None);
        cur_y += self.char_height() + self.scale_coord(6);

        // Separator line
        fb.fill_rect(win_x + self.scale_coord(20), cur_y, win_w.saturating_sub(self.scale_coord(40)), 1, self.colors.scrollbar_border);
        cur_y += self.scale_coord(10);

        // Project and Author Details
        let details = [
            ("• Desarrollador: ", "boleklolek <bolek.lolek@gmail.com>"),
            ("• Licencia:      ", "GNU AGPLv3 (Código Abierto Copyleft)"),
            ("• Arquitectura:  ", "100% Rust Estándar (Sin crates ni librerías)"),
            ("• Motor:         ", "Piece Table O(log N) con Multi-Cursor"),
            ("• Renderizado:   ", "Framebuffer RGB32 Nativo por Software"),
            ("• Plataformas:   ", "Linux (Wayland / X11) • Windows • macOS"),
            ("• Repositorio:   ", "https://github.com/boleklolek/rustext"),
        ];

        let left_margin = win_x + self.scale_coord(24);
        let max_val_x = win_x + win_w - self.scale_coord(24);
        for (lbl, val) in &details {
            fb.draw_text(lbl, left_margin, cur_y, 0x00858585, None);
            let val_x = left_margin + lbl.chars().count() * self.char_width();
            let val_color = if lbl.contains("Repositorio") { self.colors.accent } else { 0x00D4D4D4 };
            fb.draw_text_truncated(val, val_x, cur_y, max_val_x, val_color, None);
            cur_y += self.char_height() + self.scale_coord(3);
        }

        cur_y += self.scale_coord(4);
        // Separator line
        fb.fill_rect(win_x + self.scale_coord(20), cur_y, win_w.saturating_sub(self.scale_coord(40)), 1, self.colors.scrollbar_border);
        cur_y += self.scale_coord(8);

        // Essential shortcuts reference
        let sc_str = "Ctrl+P: Archivos  •  Ctrl+Shift+P: Comandos  •  Ctrl+S: Guardar  •  Ctrl+F: Buscar";
        let sc_w = sc_str.chars().count() * self.char_width();
        let sc_x = win_x + (win_w.saturating_sub(sc_w)) / 2;
        let max_sc_x = win_x + win_w - self.scale_coord(16);
        fb.draw_text_truncated(sc_str, sc_x, cur_y, max_sc_x, 0x00858585, None);

        // 8. Bottom Button: [ Aceptar ]
        let btn_w = self.scale_coord(130);
        let btn_h = self.scale_coord(28);
        let btn_x = win_x + (win_w - btn_w) / 2;
        let btn_y = win_y + win_h - self.scale_coord(38);

        let (btn_bg, btn_border) = if ok_hovered {
            (self.colors.button_hover_bg, self.colors.accent)
        } else {
            (self.colors.button_bg, self.colors.scrollbar_border)
        };
        if self.rounded_corners {
            fb.fill_rounded_rect(btn_x, btn_y, btn_w, btn_h, 4, btn_bg);
            fb.draw_rounded_outline(btn_x, btn_y, btn_w, btn_h, 4, btn_border);
        } else {
            fb.fill_rect(btn_x.saturating_sub(1), btn_y.saturating_sub(1), btn_w + 2, btn_h + 2, btn_border);
            fb.fill_rect(btn_x, btn_y, btn_w, btn_h, btn_bg);
        }

        let btn_text = "Aceptar";
        let text_x = btn_x + (btn_w.saturating_sub(btn_text.chars().count() * self.char_width())) / 2;
        let text_y = btn_y + (btn_h.saturating_sub(self.char_height())) / 2;
        fb.draw_text(btn_text, text_x, text_y, self.colors.button_fg, None);
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
        // Semi-transparent backdrop scrim
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 110);

        let win_w = self.scale_coord(680).min(fb.width.saturating_sub(self.scale_coord(20)));
        let win_h = self.scale_coord(480).min(fb.height.saturating_sub(self.scale_coord(20)));
        let win_x = (fb.width.saturating_sub(win_w)) / 2;
        let win_y = (fb.height.saturating_sub(win_h)) / 2;

        // Drop shadow
        if self.shadows {
            self.render_shadow(fb, win_x, win_y, win_w, win_h, 12, 8);
        } else {
            fb.fill_rect(win_x + 6, win_y + 6, win_w, win_h, 0x000A0A0A);
        }

        // Header Title Bar & Window Border
        let title_h = self.scale_coord(28);
        fb.fill_rect(win_x.saturating_sub(1), win_y.saturating_sub(1), win_w + 2, win_h + 2, self.colors.modal_border);
        fb.fill_rect(win_x, win_y, win_w, title_h, self.colors.modal_header_bg);

        // Body background
        let body_y = win_y + title_h + 1;
        let body_h = win_h.saturating_sub(title_h + 1);
        fb.fill_rect(win_x, body_y, win_w, body_h, self.colors.modal_bg);

        if self.rounded_corners {
            fb.draw_rounded_outline(win_x, win_y, win_w, win_h, 6, self.colors.modal_border);
        }

        fb.draw_text("Administrador de Extensiones", win_x + self.scale_coord(12), win_y + self.scale_coord(6), 0x00FFFFFF, None);

        // Close button [×]
        let close_w = self.scale_coord(36);
        let close_x = win_x + win_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, win_y + 2, close_w - 4, title_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, win_y, close_w, title_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(14), win_y + self.scale_coord(6), 0x00FFFFFF, None);

        // Header bottom divider
        fb.fill_rect(win_x, win_y + title_h, win_w, 1, self.colors.scrollbar_border);

        // Toolbar: Directory path & action buttons
        let bar_y = body_y + self.scale_coord(8);
        let count = ext_mgr.extensions.len();
        let dir_str = format!("Carpeta: ~/.rustext/extensions  •  {} extensiones", count);
        fb.draw_text(&dir_str, win_x + self.scale_coord(12), bar_y + self.scale_coord(4), self.colors.accent, None);

        // Action buttons: [ + Crear Ejemplo ] & [ ↺ Recargar ]
        let reload_btn_w = self.scale_coord(90);
        let create_btn_w = self.scale_coord(120);
        let reload_btn_x = win_x + win_w - reload_btn_w - self.scale_coord(12);
        let create_btn_x = reload_btn_x - create_btn_w - self.scale_coord(8);
        let btn_bar_h = self.scale_coord(24);

        if self.rounded_corners {
            fb.fill_rounded_rect(create_btn_x, bar_y, create_btn_w, btn_bar_h, 4, self.colors.chip_bg);
            fb.draw_rounded_outline(create_btn_x, bar_y, create_btn_w, btn_bar_h, 4, self.colors.scrollbar_border);
            fb.fill_rounded_rect(reload_btn_x, bar_y, reload_btn_w, btn_bar_h, 4, self.colors.chip_bg);
            fb.draw_rounded_outline(reload_btn_x, bar_y, reload_btn_w, btn_bar_h, 4, self.colors.scrollbar_border);
        } else {
            fb.fill_rect(create_btn_x.saturating_sub(1), bar_y.saturating_sub(1), create_btn_w + 2, btn_bar_h, self.colors.scrollbar_border);
            fb.fill_rect(create_btn_x, bar_y, create_btn_w, btn_bar_h.saturating_sub(2), self.colors.chip_bg);
            fb.fill_rect(reload_btn_x.saturating_sub(1), bar_y.saturating_sub(1), reload_btn_w + 2, btn_bar_h, self.colors.scrollbar_border);
            fb.fill_rect(reload_btn_x, bar_y, reload_btn_w, btn_bar_h.saturating_sub(2), self.colors.chip_bg);
        }
        fb.draw_text("+ Crear Ejemplo", create_btn_x + self.scale_coord(6), bar_y + (btn_bar_h.saturating_sub(self.char_height())) / 2, 0x00FFFFFF, None);
        fb.draw_text("↺ Recargar", reload_btn_x + self.scale_coord(10), bar_y + (btn_bar_h.saturating_sub(self.char_height())) / 2, 0x00FFFFFF, None);

        // List Area
        let list_x = win_x + self.scale_coord(12);
        let list_y = bar_y + self.scale_coord(30);
        let list_w = win_w.saturating_sub(self.scale_coord(24));
        let list_h = win_h.saturating_sub(self.scale_coord(115));

        if self.rounded_corners {
            fb.fill_rounded_rect(list_x, list_y, list_w, list_h, 4, self.colors.input_bg);
            fb.draw_rounded_outline(list_x, list_y, list_w, list_h, 4, self.colors.scrollbar_border);
        } else {
            fb.fill_rect(list_x.saturating_sub(1), list_y.saturating_sub(1), list_w + 2, list_h + 2, self.colors.scrollbar_border);
            fb.fill_rect(list_x, list_y, list_w, list_h, self.colors.input_bg);
        }

        if ext_mgr.extensions.is_empty() {
            fb.draw_text("No hay extensiones instaladas. Haz clic en '+ Crear Ejemplo'.", list_x + self.scale_coord(16), list_y + self.scale_coord(20), 0x00858585, None);
        } else {
            let card_h = (self.char_height() * 3 + self.scale_coord(10)).max(self.scale_coord(58));
            let max_visible = list_h / card_h;
            let mut cur_card_y = list_y + self.scale_coord(2);

            for (idx, ext) in ext_mgr.extensions.iter().skip(scroll_offset).take(max_visible).enumerate() {
                let actual_idx = scroll_offset + idx;
                let is_selected = actual_idx == selected_idx;

                let card_bg = if is_selected { self.colors.button_bg } else { self.colors.modal_bg };
                let border_color = if is_selected { self.colors.accent } else { self.colors.scrollbar_border };

                if self.rounded_corners {
                    fb.fill_rounded_rect(list_x + 2, cur_card_y, list_w.saturating_sub(4), card_h.saturating_sub(4), 4, card_bg);
                    fb.fill_rounded_rect(list_x + 2, cur_card_y, 4, card_h.saturating_sub(4), 2, border_color);
                } else {
                    fb.fill_rect(list_x + 2, cur_card_y, list_w.saturating_sub(4), card_h.saturating_sub(4), card_bg);
                    fb.fill_rect(list_x + 2, cur_card_y, 3, card_h.saturating_sub(4), border_color);
                }

                // Extension Name & Version
                let title_text = format!("{}  v{}", ext.name, ext.version);
                fb.draw_text(&title_text, list_x + self.scale_coord(12), cur_card_y + self.scale_coord(4), 0x00FFFFFF, None);

                // Status Badge: [ACTIVA] or [DESACTIVADA]
                let (badge_text, badge_bg, badge_fg) = if ext.enabled {
                    ("[ACTIVA]", 0x000E7030, 0x004EC9B0)
                } else {
                    ("[DESACTIVADA]", self.colors.chip_bg, 0x00858585)
                };
                let badge_w = self.scale_coord(100);
                let badge_h = self.scale_coord(18);
                let badge_x = list_x + list_w - self.scale_coord(180);
                if self.rounded_corners {
                    fb.fill_rounded_rect(badge_x, cur_card_y + self.scale_coord(4), badge_w, badge_h, 3, badge_bg);
                } else {
                    fb.fill_rect(badge_x, cur_card_y + self.scale_coord(4), badge_w, badge_h, badge_bg);
                }
                fb.draw_text(badge_text, badge_x + self.scale_coord(6), cur_card_y + self.scale_coord(5), badge_fg, None);

                // Description
                let desc = if ext.description.is_empty() { "Sin descripción" } else { &ext.description };
                fb.draw_text(desc, list_x + self.scale_coord(12), cur_card_y + self.char_height() + self.scale_coord(8), 0x00CCCCCC, None);

                // Commands tag
                let cmds_summary = if ext.commands.is_empty() {
                    "Sin comandos registrados".to_string()
                } else {
                    let names: Vec<&str> = ext.commands.iter().map(|c| c.label.as_str()).collect();
                    format!("Comandos: {}", names.join(", "))
                };
                fb.draw_text(&cmds_summary, list_x + self.scale_coord(12), cur_card_y + self.char_height() * 2 + self.scale_coord(10), 0x00858585, None);

                // Toggle Action Button on row
                let toggle_text = if ext.enabled { "Desactivar" } else { "Activar" };
                let toggle_w = self.scale_coord(64);
                let toggle_h = self.scale_coord(20);
                let toggle_x = list_x + list_w - toggle_w - self.scale_coord(8);
                let toggle_y = cur_card_y + self.scale_coord(4);
                if self.rounded_corners {
                    fb.fill_rounded_rect(toggle_x, toggle_y, toggle_w, toggle_h, 3, self.colors.chip_bg);
                    fb.draw_rounded_outline(toggle_x, toggle_y, toggle_w, toggle_h, 3, self.colors.scrollbar_border);
                } else {
                    fb.fill_rect(toggle_x, toggle_y, toggle_w, toggle_h, self.colors.chip_bg);
                }
                let ttx = toggle_x + (toggle_w.saturating_sub(toggle_text.chars().count() * self.char_width())) / 2;
                fb.draw_text(toggle_text, ttx, toggle_y + (toggle_h.saturating_sub(self.char_height())) / 2, 0x00FFFFFF, None);

                cur_card_y += card_h;
            }
        }

        // Footer hints & Close button
        let footer_y = win_y + win_h - self.scale_coord(34);
        let close_btn_w = self.scale_coord(80);
        let close_btn_h = self.scale_coord(24);
        let close_btn_x = win_x + win_w - close_btn_w - self.scale_coord(12);

        let max_hint_x = close_btn_x.saturating_sub(self.scale_coord(10));
        let hint_text = "Enter / Espacio: Alternar  •  'n': Nueva  •  'r': Recargar  •  Esc: Salir";
        fb.draw_text_truncated(hint_text, list_x, footer_y + self.scale_coord(6), max_hint_x, 0x00858585, None);
        if self.rounded_corners {
            fb.fill_rounded_rect(close_btn_x, footer_y, close_btn_w, close_btn_h, 4, self.colors.chip_bg);
            fb.draw_rounded_outline(close_btn_x, footer_y, close_btn_w, close_btn_h, 4, self.colors.scrollbar_border);
        } else {
            fb.fill_rect(close_btn_x.saturating_sub(1), footer_y.saturating_sub(1), close_btn_w + 2, close_btn_h + 2, self.colors.scrollbar_border);
            fb.fill_rect(close_btn_x, footer_y, close_btn_w, close_btn_h, self.colors.chip_bg);
        }
        let ctx = close_btn_x + (close_btn_w.saturating_sub(6 * self.char_width())) / 2;
        fb.draw_text("Cerrar", ctx, footer_y + (close_btn_h.saturating_sub(self.char_height())) / 2, 0x00FFFFFF, None);
    }

    /// Renders a native-styled Unsaved Changes Confirmation Dialog Window.
    pub fn render_unsaved_confirm_dialog(
        &self,
        fb: &mut Framebuffer,
        dialog: &crate::editor::app::UnsavedConfirmDialog,
        hovered_button: Option<usize>, // 0: Guardar, 1: Descartar, 2: Cancelar, 3: close_x
    ) {
        // 1. Semi-transparent backdrop scrim (dimming the background)
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 120);

        let win_w = self.scale_coord(520).min(fb.width.saturating_sub(self.scale_coord(20)));
        let win_h = self.scale_coord(190).min(fb.height.saturating_sub(self.scale_coord(20)));
        let win_x = (fb.width.saturating_sub(win_w)) / 2;
        let win_y = (fb.height.saturating_sub(win_h)) / 2;

        // 2. Drop shadow
        if self.shadows {
            self.render_shadow(fb, win_x, win_y, win_w, win_h, 12, 8);
        } else {
            fb.fill_rect(win_x + 6, win_y + 6, win_w, win_h, 0x000A0A0A);
        }

        // 3. Header Title Bar & Window Border
        let title_h = self.scale_coord(28);
        fb.fill_rect(win_x.saturating_sub(1), win_y.saturating_sub(1), win_w + 2, win_h + 2, self.colors.modal_border);
        fb.fill_rect(win_x, win_y, win_w, title_h, self.colors.modal_header_bg);

        // Body background
        let body_y = win_y + title_h + 1;
        let body_h = win_h.saturating_sub(title_h + 1);
        fb.fill_rect(win_x, body_y, win_w, body_h, self.colors.modal_bg);

        if self.rounded_corners {
            fb.draw_rounded_outline(win_x, win_y, win_w, win_h, 6, self.colors.modal_border);
        }

        // Title text: "⚠ Guardar Cambios"
        let title_str = format!("⚠ {}", dialog.title());
        fb.draw_text(&title_str, win_x + self.scale_coord(12), win_y + self.scale_coord(6), 0x00E5C07B, None); // Amber / warning gold

        // Close button [×] in top right
        let close_w = self.scale_coord(36);
        let close_x = win_x + win_w - close_w;
        let close_hovered = hovered_button == Some(3);
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, win_y + 2, close_w - 4, title_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, win_y, close_w, title_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(14), win_y + self.scale_coord(6), 0x00FFFFFF, None);

        // Header divider
        fb.fill_rect(win_x, win_y + title_h, win_w, 1, self.colors.scrollbar_border);

        // Warning Icon Badge in Body
        let icon_box_x = win_x + self.scale_coord(20);
        let icon_box_y = body_y + self.scale_coord(20);
        let icon_box_size = self.scale_coord(40);
        if self.rounded_corners {
            fb.fill_rounded_rect(icon_box_x, icon_box_y, icon_box_size, icon_box_size, 8, 0x003A3020);
            fb.draw_rounded_outline(icon_box_x, icon_box_y, icon_box_size, icon_box_size, 8, 0x00E5C07B);
        } else {
            fb.fill_rect(icon_box_x, icon_box_y, icon_box_size, icon_box_size, 0x003A3020);
        }
        fb.draw_text("!", icon_box_x + (icon_box_size.saturating_sub(self.char_width())) / 2, icon_box_y + (icon_box_size.saturating_sub(self.char_height())) / 2, 0x00E5C07B, None);

        // Message text
        let (line1, line2) = dialog.message_lines();
        let text_start_x = icon_box_x + icon_box_size + self.scale_coord(16);
        let text_max_chars = (win_x + win_w - text_start_x - self.scale_coord(16)) / self.char_width().max(1);
        let trunc_line1 = if line1.chars().count() > text_max_chars && text_max_chars > 3 {
            format!("{}...", line1.chars().take(text_max_chars - 3).collect::<String>())
        } else {
            line1
        };
        fb.draw_text(&trunc_line1, text_start_x, icon_box_y + self.scale_coord(4), 0x00FFFFFF, None);
        fb.draw_text(&line2, text_start_x, icon_box_y + self.char_height() + self.scale_coord(6), 0x00CCCCCC, None);

        // Buttons Bar at bottom
        let btn_bar_y = win_y + win_h - self.scale_coord(42);
        let btn_h = self.scale_coord(28);
        let (lbl_save, lbl_discard, lbl_cancel) = dialog.button_labels();

        let btn_save_w = (lbl_save.chars().count() * self.char_width() + self.scale_coord(24)).max(self.scale_coord(100));
        let btn_discard_w = (lbl_discard.chars().count() * self.char_width() + self.scale_coord(24)).max(self.scale_coord(110));
        let btn_cancel_w = (lbl_cancel.chars().count() * self.char_width() + self.scale_coord(24)).max(self.scale_coord(90));

        let total_btns_w = btn_save_w + btn_discard_w + btn_cancel_w + self.scale_coord(16);
        let btn_start_x = win_x + win_w - total_btns_w - self.scale_coord(16);

        let b0_x = btn_start_x;
        let b1_x = b0_x + btn_save_w + self.scale_coord(8);
        let b2_x = b1_x + btn_discard_w + self.scale_coord(8);

        let render_btn = |fb: &mut Framebuffer, bx: usize, bw: usize, label: &str, is_selected: bool, is_hovered: bool, btn_type: usize| {
            let (bg, border, fg) = if is_hovered || is_selected {
                match btn_type {
                    0 => (self.colors.button_hover_bg, self.colors.accent, 0x00FFFFFF),
                    1 => (0x00802828, 0x00F7768E, 0x00FFFFFF),
                    _ => (self.colors.chip_bg, self.colors.accent, 0x00FFFFFF),
                }
            } else {
                match btn_type {
                    0 => (self.colors.button_bg, self.colors.scrollbar_border, 0x00FFFFFF),
                    1 => (self.colors.chip_bg, self.colors.scrollbar_border, 0x00F7768E),
                    _ => (self.colors.chip_bg, self.colors.scrollbar_border, 0x00CCCCCC),
                }
            };

            if self.rounded_corners {
                fb.fill_rounded_rect(bx, btn_bar_y, bw, btn_h, 4, bg);
                fb.draw_rounded_outline(bx, btn_bar_y, bw, btn_h, 4, border);
                if is_selected {
                    fb.draw_rounded_outline(bx.saturating_sub(1), btn_bar_y.saturating_sub(1), bw + 2, btn_h + 2, 5, self.colors.accent);
                }
            } else {
                fb.fill_rect(bx.saturating_sub(1), btn_bar_y.saturating_sub(1), bw + 2, btn_h + 2, border);
                fb.fill_rect(bx, btn_bar_y, bw, btn_h, bg);
            }

            let tw = label.chars().count() * self.char_width();
            let tx = bx + (bw.saturating_sub(tw)) / 2;
            let ty = btn_bar_y + (btn_h.saturating_sub(self.char_height())) / 2;
            fb.draw_text(label, tx, ty, fg, None);
        };

        render_btn(fb, b0_x, btn_save_w, lbl_save, dialog.selected_button == 0, hovered_button == Some(0), 0);
        render_btn(fb, b1_x, btn_discard_w, lbl_discard, dialog.selected_button == 1, hovered_button == Some(1), 1);
        render_btn(fb, b2_x, btn_cancel_w, lbl_cancel, dialog.selected_button == 2, hovered_button == Some(2), 2);
    }

    /// Renders the interactive Font Selector and Typography Settings Dialog.
    pub fn render_font_selector_dialog(
        &self,
        fb: &mut Framebuffer,
        dialog: &crate::editor::font_selector::FontSelectorDialog,
        close_hovered: bool,
    ) {
        // 1. Semi-transparent backdrop scrim
        fb.fill_rect_alpha(0, 0, fb.width, fb.height, 0x00000000, 120);

        let win_w = self.scale_coord(760).min(fb.width.saturating_sub(self.scale_coord(20)));
        let win_h = self.scale_coord(520).min(fb.height.saturating_sub(self.scale_coord(20)));
        let win_x = (fb.width.saturating_sub(win_w)) / 2;
        let win_y = (fb.height.saturating_sub(win_h)) / 2;

        // 2. Drop shadow
        if self.shadows {
            self.render_shadow(fb, win_x, win_y, win_w, win_h, 12, 8);
        } else {
            fb.fill_rect(win_x + 6, win_y + 6, win_w, win_h, 0x000A0A0A);
        }

        // 3. Header & Window Frame
        let title_h = self.scale_coord(28);
        fb.fill_rect(win_x.saturating_sub(1), win_y.saturating_sub(1), win_w + 2, win_h + 2, self.colors.modal_border);
        fb.fill_rect(win_x, win_y, win_w, title_h, self.colors.modal_header_bg);

        let body_y = win_y + title_h + 1;
        let body_h = win_h.saturating_sub(title_h + 1);
        fb.fill_rect(win_x, body_y, win_w, body_h, self.colors.modal_bg);

        if self.rounded_corners {
            fb.draw_rounded_outline(win_x, win_y, win_w, win_h, 6, self.colors.modal_border);
        }

        fb.draw_text("Tipografía y Selector de Fuentes", win_x + self.scale_coord(12), win_y + self.scale_coord(6), 0x00FFFFFF, None);

        // Close button [×]
        let close_w = self.scale_coord(36);
        let close_x = win_x + win_w - close_w;
        let close_bg = if close_hovered { 0x00E81123 } else { self.colors.modal_header_bg };
        if self.rounded_corners {
            fb.fill_rounded_rect(close_x + 2, win_y + 2, close_w - 4, title_h - 4, 3, close_bg);
        } else {
            fb.fill_rect(close_x, win_y, close_w, title_h, close_bg);
        }
        fb.draw_text("×", close_x + self.scale_coord(14), win_y + self.scale_coord(6), 0x00FFFFFF, None);

        fb.fill_rect(win_x, win_y + title_h, win_w, 1, self.colors.scrollbar_border);

        // Columns layout
        let footer_h = self.scale_coord(44);
        let content_h = body_h.saturating_sub(footer_h + self.scale_coord(12));
        let col_w = (win_w.saturating_sub(self.scale_coord(36))) / 2;
        let left_x = win_x + self.scale_coord(14);
        let right_x = left_x + col_w + self.scale_coord(8);
        let content_y = body_y + self.scale_coord(8);

        // === LEFT COLUMN: Controls ===
        // 1. Font Family header
        fb.draw_text("FAMILIAS TIPOGRÁFICAS (5 fuentes)", left_x, content_y, self.colors.accent, None);
        let fam_start_y = content_y + self.scale_coord(16);
        let fam_card_h = self.scale_coord(34);

        for (i, &f) in FontFamily::ALL.iter().enumerate() {
            let card_y = fam_start_y + i * (fam_card_h + self.scale_coord(3));
            let is_selected = f == dialog.selected_family;
            let is_hovered = dialog.hovered_family == Some(i);

            let bg = if is_selected {
                self.colors.button_bg
            } else if is_hovered {
                self.colors.input_bg
            } else {
                self.colors.modal_bg
            };

            let border = if is_selected {
                self.colors.accent
            } else {
                self.colors.scrollbar_border
            };

            if self.rounded_corners {
                fb.fill_rounded_rect(left_x, card_y, col_w, fam_card_h, 4, bg);
                fb.draw_rounded_outline(left_x, card_y, col_w, fam_card_h, 4, border);
            } else {
                fb.fill_rect(left_x, card_y, col_w, fam_card_h, bg);
            }

            let bullet = if is_selected { "[✓] " } else { "[ ] " };
            let fam_title = format!("{}{}", bullet, f.display_name());
            let title_color = if is_selected { 0x00FFFFFF } else { 0x00DDDDDD };
            fb.draw_text(&fam_title, left_x + self.scale_coord(8), card_y + self.scale_coord(3), title_color, None);

            let desc = f.description();
            let desc_max_chars = (col_w.saturating_sub(self.scale_coord(20))) / self.char_width().max(1);
            let desc_trunc: String = desc.chars().take(desc_max_chars).collect();
            fb.draw_text(&desc_trunc, left_x + self.scale_coord(16), card_y + self.scale_coord(18), 0x00888888, None);
        }

        // 2. Font Size selector
        let size_sec_y = fam_start_y + 5 * (fam_card_h + self.scale_coord(3)) + self.scale_coord(6);
        fb.draw_text("TAMAÑO DE FUENTE (Font Size)", left_x, size_sec_y, self.colors.accent, None);
        let size_pills_y = size_sec_y + self.scale_coord(16);
        let pill_h = self.scale_coord(22);
        let pill_w = (col_w.saturating_sub(self.scale_coord(28))) / crate::editor::font_selector::AVAILABLE_SIZES.len().max(1);

        for (idx, &s) in crate::editor::font_selector::AVAILABLE_SIZES.iter().enumerate() {
            let px = left_x + idx * (pill_w + self.scale_coord(4));
            let is_sel = s == dialog.selected_size;
            let bg = if is_sel { self.colors.accent } else { self.colors.chip_bg };
            let fg = if is_sel { 0x00FFFFFF } else { 0x00CCCCCC };

            if self.rounded_corners {
                fb.fill_rounded_rect(px, size_pills_y, pill_w, pill_h, 3, bg);
            } else {
                fb.fill_rect(px, size_pills_y, pill_w, pill_h, bg);
            }
            let label = format!("{}", s);
            let lx = px + (pill_w.saturating_sub(label.chars().count() * self.char_width())) / 2;
            fb.draw_text(&label, lx, size_pills_y + (pill_h.saturating_sub(self.char_height())) / 2, fg, None);
        }

        // 3. Weight / Grosor
        let weight_sec_y = size_pills_y + pill_h + self.scale_coord(8);
        fb.draw_text("GROSOR DE TRAZO (Font Weight)", left_x, weight_sec_y, self.colors.accent, None);
        let weight_pills_y = weight_sec_y + self.scale_coord(16);
        let weight_w = (col_w.saturating_sub(self.scale_coord(16))) / 3;

        for (idx, &w) in FontWeight::ALL.iter().enumerate() {
            let wx = left_x + idx * (weight_w + self.scale_coord(8));
            let is_sel = w == dialog.selected_weight;
            let bg = if is_sel { self.colors.button_bg } else { self.colors.chip_bg };
            let border = if is_sel { self.colors.accent } else { self.colors.scrollbar_border };
            let fg = if is_sel { 0x00FFFFFF } else { 0x00CCCCCC };

            if self.rounded_corners {
                fb.fill_rounded_rect(wx, weight_pills_y, weight_w, pill_h, 3, bg);
                fb.draw_rounded_outline(wx, weight_pills_y, weight_w, pill_h, 3, border);
            } else {
                fb.fill_rect(wx, weight_pills_y, weight_w, pill_h, bg);
            }
            let label = w.display_name();
            let lx = wx + (weight_w.saturating_sub(label.chars().count() * self.char_width())) / 2;
            fb.draw_text(label, lx, weight_pills_y + (pill_h.saturating_sub(self.char_height())) / 2, fg, None);
        }

        // 4. Advanced Typography Options
        let opts_sec_y = weight_pills_y + pill_h + self.scale_coord(8);
        fb.draw_text("CARACTERÍSTICAS TIPOGRÁFICAS", left_x, opts_sec_y, self.colors.accent, None);
        let opts_chips_y = opts_sec_y + self.scale_coord(16);

        let chip_lig_text = format!("Ligaduras: {}", if dialog.ligatures { "ON" } else { "OFF" });
        let chip_aa_text = format!("Anti-alias: {}", if dialog.antialiasing { "ON" } else { "OFF" });
        let chip_sp_text = format!("s: {}px", dialog.line_spacing);

        let chip_w = (col_w.saturating_sub(self.scale_coord(16))) / 3;
        let c0_x = left_x;
        let c1_x = c0_x + chip_w + self.scale_coord(8);
        let c2_x = c1_x + chip_w + self.scale_coord(8);

        let draw_opt_chip = |fb: &mut Framebuffer, cx: usize, cy: usize, cw: usize, text: &str, active: bool| {
            let bg = if active { self.colors.chip_bg } else { self.colors.modal_bg };
            let border = if active { self.colors.accent } else { self.colors.scrollbar_border };
            let fg = if active { self.colors.accent } else { 0x00888888 };
            if self.rounded_corners {
                fb.fill_rounded_rect(cx, cy, cw, pill_h, 3, bg);
                fb.draw_rounded_outline(cx, cy, cw, pill_h, 3, border);
            } else {
                fb.fill_rect(cx, cy, cw, pill_h, bg);
            }
            let tx = cx + (cw.saturating_sub(text.chars().count() * self.char_width())) / 2;
            fb.draw_text_clipped(text, tx, cy + (pill_h.saturating_sub(self.char_height())) / 2, cx + cw.saturating_sub(2), fg, None);
        };

        draw_opt_chip(fb, c0_x, opts_chips_y, chip_w, &chip_lig_text, dialog.ligatures);
        draw_opt_chip(fb, c1_x, opts_chips_y, chip_w, &chip_aa_text, dialog.antialiasing);
        draw_opt_chip(fb, c2_x, opts_chips_y, chip_w, &chip_sp_text, true);

        let opts_row2_y = opts_chips_y + pill_h + self.scale_coord(6);
        let chip2_w = (col_w.saturating_sub(self.scale_coord(16))) / 3;
        let r2_c0_x = left_x;
        let r2_c1_x = r2_c0_x + chip2_w + self.scale_coord(8);
        let r2_c2_x = r2_c1_x + chip2_w + self.scale_coord(8);

        let chip_let_text = format!("e: +{}px", dialog.letter_spacing);
        let chip_asp_text = format!("p: {:.2}", dialog.font_aspect_ratio);
        let chip_dark_text = if dialog.stem_darkening { "d: Engrosar" } else { "d: Estándar" };

        draw_opt_chip(fb, r2_c0_x, opts_row2_y, chip2_w, &chip_let_text, dialog.letter_spacing > 0);
        draw_opt_chip(fb, r2_c1_x, opts_row2_y, chip2_w, &chip_asp_text, (dialog.font_aspect_ratio - 0.50).abs() > 0.001);
        draw_opt_chip(fb, r2_c2_x, opts_row2_y, chip2_w, chip_dark_text, dialog.stem_darkening);

        let opts_row3_y = opts_row2_y + pill_h + self.scale_coord(6);
        let chip3_w = (col_w.saturating_sub(self.scale_coord(16))) / 3;
        let r3_c0_x = left_x;
        let r3_c1_x = r3_c0_x + chip3_w + self.scale_coord(8);
        let r3_c2_x = r3_c1_x + chip3_w + self.scale_coord(8);

        let chip_sdf_text = format!("f: SDF {}", if dialog.sdf_rendering { "ON" } else { "OFF" });
        let chip_opt_text = format!("o: Óptico {}", if dialog.optical_spacing { "ON" } else { "OFF" });
        let chip_ttf_text = format!("t: TTF {}", if dialog.use_ttf { "ON" } else { "OFF" });

        draw_opt_chip(fb, r3_c0_x, opts_row3_y, chip3_w, &chip_sdf_text, dialog.sdf_rendering);
        draw_opt_chip(fb, r3_c1_x, opts_row3_y, chip3_w, &chip_opt_text, dialog.optical_spacing);
        draw_opt_chip(fb, r3_c2_x, opts_row3_y, chip3_w, &chip_ttf_text, dialog.use_ttf);

        // === RIGHT COLUMN: Live Preview ===
        let prev_title = format!("VISTA PREVIA EN VIVO ({} pt • {})", dialog.selected_size, dialog.selected_family.display_name());
        fb.draw_text(&prev_title, right_x, content_y, self.colors.accent, None);

        let iso_card_h = self.scale_coord(76);
        let prev_box_y = content_y + self.scale_coord(16);
        let prev_box_h = content_h.saturating_sub(self.scale_coord(16) + iso_card_h + self.scale_coord(8));

        if self.rounded_corners {
            fb.fill_rounded_rect(right_x, prev_box_y, col_w, prev_box_h, 6, self.colors.input_bg);
            fb.draw_rounded_outline(right_x, prev_box_y, col_w, prev_box_h, 6, self.colors.modal_border);
        } else {
            fb.fill_rect(right_x, prev_box_y, col_w, prev_box_h, self.colors.input_bg);
        }

        // Render preview code sample inside preview box
        let sample_lines = [
            ("//! Vista previa en vivo con Rustext", self.colors.syntax_comment),
            ("fn benchmark_render() -> f64 {", self.colors.syntax_keyword),
            ("    let font = \"Rustext Mono\";", self.colors.syntax_plain),
            ("    let latency_us = 20;", self.colors.syntax_number),
            ("    if latency_us <= 50 && enabled {", self.colors.syntax_keyword),
            ("        println!(\"-> != => ==\");", self.colors.syntax_string),
            ("    }", self.colors.syntax_keyword),
            ("    488.6 // Throughput MB/s", self.colors.syntax_comment),
            ("}", self.colors.syntax_keyword),
        ];

        let prev_char_h = ((dialog.selected_size as f64 * 16.0 / 12.0).round() as usize).max(10);
        let prev_glyph_w = if dialog.font_aspect_ratio > 0.0 && (dialog.font_aspect_ratio - 0.50).abs() > 0.001 {
            ((prev_char_h as f64 * dialog.font_aspect_ratio).round() as usize).max(5)
        } else if dialog.selected_size != 12 {
            ((dialog.selected_size as f64 * 8.0 / 12.0).round() as usize).max(5)
        } else {
            FONT_WIDTH
        };
        let prev_char_w = prev_glyph_w + dialog.letter_spacing;
        let prev_line_h = prev_char_h + dialog.line_spacing;

        let prev_pad_x = right_x + self.scale_coord(12);
        let prev_pad_y = prev_box_y + self.scale_coord(14);

        for (line_idx, &(line_str, line_fg)) in sample_lines.iter().enumerate() {
            let ly = prev_pad_y + line_idx * prev_line_h;
            if ly + prev_char_h > prev_box_y + prev_box_h {
                break;
            }

            let dark_factor = if dialog.stem_darkening {
                StemDarkener::compute_darkening_factor(self.colors.input_bg, line_fg, dialog.selected_size, 1.0)
            } else {
                0.0
            };

            let line_chars: Vec<char> = line_str.chars().collect();

            if dialog.ligatures {
                let glyphs = FontShaper::shape_text_ex(
                    line_str,
                    1,
                    dialog.ligatures,
                    dialog.selected_family,
                    dialog.selected_weight,
                    prev_glyph_w,
                    prev_char_h,
                );
                let mut gx = prev_pad_x;
                let mut char_idx = 0;
                for g in glyphs {
                    if gx + g.width > right_x + col_w - self.scale_coord(8) {
                        break;
                    }
                    let ch = g.cluster.chars().next().unwrap_or(' ');
                    let is_box = FontRasterizer::is_box_drawing(ch);
                    let cluster_len = g.cluster.chars().count();

                    let prev_char = if char_idx > 0 { line_chars.get(char_idx - 1).copied() } else { None };
                    let next_char = line_chars.get(char_idx + cluster_len).copied();
                    char_idx += cluster_len;

                    let opt_offset = if dialog.optical_spacing && !is_box && !g.is_ligature {
                        OpticalPunctuationSpacing::compute_optical_offset(prev_char, ch, next_char, prev_char_w, 1)
                    } else {
                        0
                    };

                    let total_w = prev_char_w * g.advance_cells;
                    let (draw_x, draw_w) = if is_box {
                        (gx, prev_char_w)
                    } else {
                        let safe_x = OpticalPunctuationSpacing::compute_safe_in_cell_draw_x(gx, total_w, g.width, opt_offset);
                        (safe_x, g.width)
                    };
                    let max_w = draw_w.min(fb.width.saturating_sub(draw_x));
                    let max_h = g.height.min(fb.height.saturating_sub(ly));
                    let effective_cov = if is_box && draw_w != g.width {
                        let (_, _, box_cov) = FontRasterizer::rasterize_glyph_sized(ch, dialog.selected_family, draw_w, g.height, dialog.selected_weight);
                        box_cov
                    } else if dialog.use_ttf && !g.is_ligature {
                        let ttf_font = crate::gfx::ttf::SystemFontLoader::get_active_font();
                        let (_, _, ttf_cov) = crate::gfx::ttf::TtfRasterizer::rasterize_char_cached(&ttf_font, ch, draw_w, g.height);
                        if dark_factor > 0.001 {
                            StemDarkener::darken_coverage_mask(&ttf_cov, draw_w, g.height, dark_factor)
                        } else {
                            ttf_cov.to_vec()
                        }
                    } else if dialog.sdf_rendering && !g.is_ligature {
                        let (_, _, sdf_cov) = SdfFontRasterizer::rasterize_glyph_sdf(ch, dialog.selected_family, draw_w, g.height, dialog.selected_weight, 0.0);
                        if dark_factor > 0.001 {
                            StemDarkener::darken_coverage_mask(&sdf_cov, draw_w, g.height, dark_factor)
                        } else {
                            sdf_cov
                        }
                    } else if dark_factor > 0.001 && !is_box {
                        StemDarkener::darken_coverage_mask(&g.coverage, g.width, g.height, dark_factor)
                    } else {
                        g.coverage.clone()
                    };
                    for row in 0..max_h {
                        let py = ly + row;
                        let row_offset = row * draw_w;
                        let fb_offset = py * fb.width + draw_x;
                        for col in 0..max_w {
                            let alpha = effective_cov[row_offset + col];
                            if alpha > 0 {
                                fb.pixels[fb_offset + col] = blend_pixel_gamma(self.colors.input_bg, line_fg, alpha);
                            }
                        }
                    }
                    gx += g.advance_cells * prev_char_w;
                }
            } else {
                let mut cur_gx = prev_pad_x;
                for (char_idx, ch) in line_str.chars().enumerate() {
                    if cur_gx + prev_char_w > right_x + col_w - self.scale_coord(8) {
                        break;
                    }
                    let is_box = FontRasterizer::is_box_drawing(ch);

                    let prev_char = if char_idx > 0 { line_chars.get(char_idx - 1).copied() } else { None };
                    let next_char = line_chars.get(char_idx + 1).copied();

                    let opt_offset = if dialog.optical_spacing && !is_box {
                        OpticalPunctuationSpacing::compute_optical_offset(prev_char, ch, next_char, prev_char_w, 1)
                    } else {
                        0
                    };

                    let (draw_x, target_w) = if is_box {
                        (cur_gx, prev_char_w)
                    } else {
                        let safe_x = OpticalPunctuationSpacing::compute_safe_in_cell_draw_x(cur_gx, prev_char_w, prev_glyph_w, opt_offset);
                        (safe_x, prev_glyph_w)
                    };
                    if self.subpixel_antialiasing {
                        let (gw, gh, mut sub_cov) = if dialog.use_ttf && !is_box {
                            let ttf_font = crate::gfx::ttf::SystemFontLoader::get_active_font();
                            let (w, h, sub) = crate::gfx::ttf::TtfRasterizer::rasterize_char_subpixel_cached(
                                &ttf_font,
                                ch,
                                target_w,
                                prev_char_h,
                            );
                            (w, h, sub.to_vec())
                        } else if dialog.sdf_rendering && !is_box {
                            SdfFontRasterizer::rasterize_glyph_sdf_subpixel(
                                ch,
                                dialog.selected_family,
                                target_w,
                                prev_char_h,
                                dialog.selected_weight,
                                0.0,
                            )
                        } else {
                            FontRasterizer::rasterize_glyph_subpixel(
                                ch,
                                dialog.selected_family,
                                target_w,
                                prev_char_h,
                                dialog.selected_weight,
                            )
                        };
                        if dark_factor > 0.001 && !is_box {
                            sub_cov = StemDarkener::darken_subpixel_mask(&sub_cov, gw, gh, dark_factor);
                        }
                        let max_w = gw.min(fb.width.saturating_sub(draw_x));
                        let max_h = gh.min(fb.height.saturating_sub(ly));
                        for row in 0..max_h {
                            let py = ly + row;
                            let row_offset = row * gw;
                            let fb_offset = py * fb.width + draw_x;
                            for col in 0..max_w {
                                let [ar, ag, ab] = sub_cov[row_offset + col];
                                if ar > 0 || ag > 0 || ab > 0 {
                                    fb.pixels[fb_offset + col] = blend_pixel_subpixel_gamma(self.colors.input_bg, line_fg, ar, ag, ab);
                                }
                            }
                        }
                    } else {
                        let (gw, gh, mut cov) = if dialog.use_ttf && !is_box {
                            let ttf_font = crate::gfx::ttf::SystemFontLoader::get_active_font();
                            let (w, h, cov) = crate::gfx::ttf::TtfRasterizer::rasterize_char_cached(
                                &ttf_font,
                                ch,
                                target_w,
                                prev_char_h,
                            );
                            (w, h, cov.to_vec())
                        } else if dialog.sdf_rendering && !is_box {
                            SdfFontRasterizer::rasterize_glyph_sdf(
                                ch,
                                dialog.selected_family,
                                target_w,
                                prev_char_h,
                                dialog.selected_weight,
                                0.0,
                            )
                        } else {
                            FontRasterizer::rasterize_glyph_sized(
                                ch,
                                dialog.selected_family,
                                target_w,
                                prev_char_h,
                                dialog.selected_weight,
                            )
                        };
                        if dark_factor > 0.001 && !is_box {
                            cov = StemDarkener::darken_coverage_mask(&cov, gw, gh, dark_factor);
                        }
                        let max_w = gw.min(fb.width.saturating_sub(draw_x));
                        let max_h = gh.min(fb.height.saturating_sub(ly));
                        for row in 0..max_h {
                            let py = ly + row;
                            let row_offset = row * gw;
                            let fb_offset = py * fb.width + draw_x;
                            for col in 0..max_w {
                                let alpha = cov[row_offset + col];
                                if alpha > 0 {
                                    fb.pixels[fb_offset + col] = blend_pixel_gamma(self.colors.input_bg, line_fg, alpha);
                                }
                            }
                        }
                    }
                    cur_gx += prev_char_w;
                }
            }
        }

        // === ISO 9241-303 Ergonomics & HiDPI Card ===
        let iso_card_y = prev_box_y + prev_box_h + self.scale_coord(8);
        let metrics = crate::gfx::HiDpiDetector::detect_display_metrics();
        let effective_scale = if dialog.scale_factor > 0.0 {
            dialog.scale_factor
        } else if dialog.ergonomic_scaling {
            crate::gfx::HiDpiDetector::compute_ergonomic_scale(
                metrics.dpi,
                metrics.default_viewing_distance_mm,
                crate::gfx::HiDpiDetector::ISO_RECOMMENDED_ARCMIN,
            )
        } else {
            self.scale_factor
        };

        let char_height_logical = ((dialog.selected_size as f64 * 16.0 / 12.0).round() as usize).max(10);
        let legibility = crate::gfx::HiDpiDetector::assess_legibility(
            char_height_logical,
            effective_scale,
            metrics.dpi,
            metrics.default_viewing_distance_mm,
        );

        let card_bg = self.colors.modal_header_bg;
        let card_border = legibility.rating.badge_color();

        if self.rounded_corners {
            fb.fill_rounded_rect(right_x, iso_card_y, col_w, iso_card_h, 4, card_bg);
            fb.draw_rounded_outline(right_x, iso_card_y, col_w, iso_card_h, 4, card_border);
        } else {
            fb.fill_rect(right_x, iso_card_y, col_w, iso_card_h, card_bg);
        }

        let pad_x = right_x + self.scale_coord(10);
        let max_card_x = right_x + col_w - self.scale_coord(8);
        let l1_y = iso_card_y + self.scale_coord(6);
        let l2_y = l1_y + self.scale_coord(16);
        let l3_y = l2_y + self.scale_coord(16);
        let l4_y = l3_y + self.scale_coord(16);

        // Line 1: Header + Rating badge cleanly separated
        fb.draw_text("ERGONOMÍA VISUAL", pad_x, l1_y, self.colors.accent, None);
        let badge_str = format!("{:.1}' [{}]", legibility.visual_angle_arcmin, legibility.rating.badge_label());
        let badge_char_w = self.char_width().max(1);
        let badge_w = badge_str.chars().count() * badge_char_w;
        let badge_x = right_x + col_w - self.scale_coord(10) - badge_w;
        let min_badge_x = pad_x + 18 * badge_char_w;
        if badge_x >= min_badge_x {
            fb.draw_text_clipped(&badge_str, badge_x, l1_y, max_card_x, legibility.rating.badge_color(), None);
        }

        // Line 2: Display Density & Active Scale
        let l2_str = format!(
            "Pantalla: {:.0} DPI ({:.1}\") • HiDPI: {}",
            metrics.dpi,
            metrics.diagonal_inches,
            dialog.scale_label()
        );
        fb.draw_text_truncated(&l2_str, pad_x, l2_y, max_card_x, 0x00CCCCCC, None);

        // Line 3: Physical character size & viewing distance
        let l3_str = format!(
            "Física: {:.2}mm • Dist: {}cm • Óptima: {:.2}x",
            legibility.physical_height_mm,
            (metrics.default_viewing_distance_mm / 10.0).round() as usize,
            legibility.recommended_scale
        );
        fb.draw_text_truncated(&l3_str, pad_x, l3_y, max_card_x, 0x00999999, None);

        // Line 4: Ergonomic recommendation
        fb.draw_text_truncated(legibility.description, pad_x, l4_y, max_card_x, legibility.rating.badge_color(), None);

        // === FOOTER BAR: Buttons & Key Hints ===
        let footer_y = win_y + win_h - footer_h;
        fb.fill_rect(win_x, footer_y, win_w, 1, self.colors.scrollbar_border);

        let btn_cancel_w = self.scale_coord(95);
        let btn_apply_w = self.scale_coord(145);
        let btn_h = self.scale_coord(26);
        let btn_y = footer_y + (footer_h.saturating_sub(btn_h)) / 2;

        let btn_apply_x = win_x + win_w - btn_apply_w - self.scale_coord(14);
        let btn_cancel_x = btn_apply_x - btn_cancel_w - self.scale_coord(10);

        let max_hint_x = btn_cancel_x.saturating_sub(self.scale_coord(12));
        let hint = "Tab: Sección  •  +/-: Zoom  •  Enter: Guardar  •  Esc: Salir";
        fb.draw_text_truncated(hint, win_x + self.scale_coord(14), footer_y + (footer_h.saturating_sub(self.char_height())) / 2, max_hint_x, self.colors.gutter_fg, None);

        let is_cancel_focused = dialog.focus_section == 4 && dialog.selected_button == 1;
        let is_apply_focused = dialog.focus_section == 4 && dialog.selected_button == 0;

        let cancel_bg = if is_cancel_focused || dialog.hovered_button == Some(1) {
            self.colors.button_hover_bg
        } else {
            self.colors.chip_bg
        };

        let apply_bg = if is_apply_focused || dialog.hovered_button == Some(0) {
            self.colors.button_hover_bg
        } else {
            self.colors.button_bg
        };

        if self.rounded_corners {
            fb.fill_rounded_rect(btn_cancel_x, btn_y, btn_cancel_w, btn_h, 4, cancel_bg);
            fb.draw_rounded_outline(btn_cancel_x, btn_y, btn_cancel_w, btn_h, 4, self.colors.scrollbar_border);
            fb.fill_rounded_rect(btn_apply_x, btn_y, btn_apply_w, btn_h, 4, apply_bg);
            fb.draw_rounded_outline(btn_apply_x, btn_y, btn_apply_w, btn_h, 4, self.colors.accent);
        } else {
            fb.fill_rect(btn_cancel_x, btn_y, btn_cancel_w, btn_h, cancel_bg);
            fb.fill_rect(btn_apply_x, btn_y, btn_apply_w, btn_h, apply_bg);
        }

        let ctx = btn_cancel_x + (btn_cancel_w.saturating_sub("Cancelar".chars().count() * self.char_width())) / 2;
        let atx = btn_apply_x + (btn_apply_w.saturating_sub("Aplicar y Guardar".chars().count() * self.char_width())) / 2;
        fb.draw_text("Cancelar", ctx, btn_y + (btn_h.saturating_sub(self.char_height())) / 2, 0x00CCCCCC, None);
        fb.draw_text("Aplicar y Guardar", atx, btn_y + (btn_h.saturating_sub(self.char_height())) / 2, 0x00FFFFFF, None);
    }
}
