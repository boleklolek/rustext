//! EditorAction dispatch logic.
//! Zero third-party dependencies, pure standard library Rust.

use std::path::PathBuf;

use crate::core::buffer::Position;
use crate::core::search::SearchEngine;
use crate::editor::dialogs::EditorMode;
use crate::editor::keymap::EditorAction;
use crate::ui::prompt::{PromptKind, PromptModal};
use super::app::{user_theme_command_items, Editor};

impl Editor {
    pub fn execute_action(&mut self, action: EditorAction) {
        let mut was_edit = matches!(
            &action,
            EditorAction::InsertChar(_)
                | EditorAction::InsertNewline
                | EditorAction::InsertTab
                | EditorAction::PreviousTabstop
                | EditorAction::InsertSnippet(_)
                | EditorAction::Backspace
                | EditorAction::Delete
                | EditorAction::DeleteLine
                | EditorAction::InsertLineBelow
                | EditorAction::InsertLineAbove
                | EditorAction::Cut
                | EditorAction::Paste
                | EditorAction::Undo
                | EditorAction::Redo
                | EditorAction::MoveLineUp
                | EditorAction::MoveLineDown
                | EditorAction::DuplicateLineDown
                | EditorAction::ToggleComment
        );

        match action {
            // Text editing
            EditorAction::InsertChar(c) => self.insert_char_multicursor(c),
            EditorAction::InsertNewline => self.insert_newline_multicursor(),
            EditorAction::InsertTab => self.handle_tab_key(),
            EditorAction::PreviousTabstop => self.handle_backtab_key(),
            EditorAction::InsertSnippet(ref trigger) => {
                let t = trigger.clone();
                self.expand_snippet_trigger(&t);
            }
            EditorAction::Backspace => self.backspace_multicursor(),
            EditorAction::Delete => self.delete_multicursor(),

            // Clipboard
            EditorAction::Copy => self.copy(),
            EditorAction::Cut => self.cut(),
            EditorAction::Paste => self.paste(),

            // Tabs / Multiple Buffers
            EditorAction::NewTab => self.new_tab(),
            EditorAction::CloseTab => self.close_tab(),
            EditorAction::NextTab => self.next_tab(),
            EditorAction::PrevTab => self.prev_tab(),
            EditorAction::OpenOpenFilePrompt => {
                let dir = self.doc().buffer.file_path()
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
                self.mode = EditorMode::ModalPrompt(PromptModal::new_with_dir(PromptKind::OpenFile, dir, None));
            }
            EditorAction::OpenFolderPrompt => {
                let dir = self.active_workspace.clone().unwrap_or_else(|| {
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                });
                self.mode = EditorMode::ModalPrompt(PromptModal::new_with_dir(PromptKind::OpenFolder, dir, None));
            }
            EditorAction::OpenRecentProjectsPrompt => {
                self.recent_projects = self.project_registry.load_recent_projects();
                self.mode = EditorMode::ModalPrompt(PromptModal::new_recent_projects(self.recent_projects.clone()));
            }
            EditorAction::OpenWorkspace(path) => {
                self.open_workspace(path);
            }
            EditorAction::CloseWorkspace => {
                self.close_workspace();
            }

            // Navigation
            EditorAction::MoveLeft { select } => {
                let doc = self.doc_mut();
                doc.cursors.move_left(&doc.buffer, select);
            }
            EditorAction::MoveRight { select } => {
                let doc = self.doc_mut();
                doc.cursors.move_right(&doc.buffer, select);
            }
            EditorAction::MoveUp { select } => {
                let is_word_wrap = self.word_wrap;
                let max_cols = self.active_max_cols();
                let doc = self.doc_mut();
                doc.update_folding();
                if is_word_wrap {
                    crate::editor::wrap::WordWrap::move_up(&mut doc.cursors, &doc.buffer, max_cols, select);
                    if doc.folding.enabled && doc.folding.hidden_line_count() > 0 {
                        for c in doc.cursors.cursors_mut() {
                            let pos = doc.buffer.char_to_position(c.head);
                            if doc.folding.is_line_hidden(pos.line) {
                                let target_line = doc.folding.prev_visible_line(pos.line);
                                let line_len = doc.buffer.line_len(target_line);
                                let target_col = std::cmp::min(pos.col, line_len);
                                let target_char = doc.buffer.position_to_char(Position::new(target_line, target_col));
                                c.head = target_char;
                                if !select {
                                    c.anchor = target_char;
                                }
                            }
                        }
                        doc.cursors.normalize();
                    }
                } else if doc.folding.enabled && doc.folding.hidden_line_count() > 0 {
                    for c in doc.cursors.cursors_mut() {
                        let pos = doc.buffer.char_to_position(c.head);
                        let target_line = doc.folding.prev_visible_line(pos.line);
                        let line_len = doc.buffer.line_len(target_line);
                        let target_col = std::cmp::min(pos.col, line_len);
                        let target_char = doc.buffer.position_to_char(Position::new(target_line, target_col));
                        c.head = target_char;
                        if !select {
                            c.anchor = target_char;
                        }
                    }
                    doc.cursors.normalize();
                } else {
                    doc.cursors.move_up(&doc.buffer, select);
                }
            }
            EditorAction::MoveDown { select } => {
                let is_word_wrap = self.word_wrap;
                let max_cols = self.active_max_cols();
                let doc = self.doc_mut();
                doc.update_folding();
                let total_lines = doc.buffer.len_lines();
                if is_word_wrap {
                    crate::editor::wrap::WordWrap::move_down(&mut doc.cursors, &doc.buffer, max_cols, select);
                    if doc.folding.enabled && doc.folding.hidden_line_count() > 0 {
                        for c in doc.cursors.cursors_mut() {
                            let pos = doc.buffer.char_to_position(c.head);
                            if doc.folding.is_line_hidden(pos.line) {
                                let target_line = doc.folding.next_visible_line(pos.line, total_lines);
                                let line_len = doc.buffer.line_len(target_line);
                                let target_col = std::cmp::min(pos.col, line_len);
                                let target_char = doc.buffer.position_to_char(Position::new(target_line, target_col));
                                c.head = target_char;
                                if !select {
                                    c.anchor = target_char;
                                }
                            }
                        }
                        doc.cursors.normalize();
                    }
                } else if doc.folding.enabled && doc.folding.hidden_line_count() > 0 {
                    for c in doc.cursors.cursors_mut() {
                        let pos = doc.buffer.char_to_position(c.head);
                        let target_line = doc.folding.next_visible_line(pos.line, total_lines);
                        let line_len = doc.buffer.line_len(target_line);
                        let target_col = std::cmp::min(pos.col, line_len);
                        let target_char = doc.buffer.position_to_char(Position::new(target_line, target_col));
                        c.head = target_char;
                        if !select {
                            c.anchor = target_char;
                        }
                    }
                    doc.cursors.normalize();
                } else {
                    doc.cursors.move_down(&doc.buffer, select);
                }
            }
            EditorAction::MoveLineStart { select } => {
                if self.word_wrap {
                    let max_cols = self.active_max_cols();
                    let doc = self.doc_mut();
                    crate::editor::wrap::WordWrap::move_to_line_start(&mut doc.cursors, &doc.buffer, max_cols, select);
                } else {
                    let doc = self.doc_mut();
                    doc.cursors.move_to_line_start(&doc.buffer, select);
                }
            }
            EditorAction::MoveLineEnd { select } => {
                if self.word_wrap {
                    let max_cols = self.active_max_cols();
                    let doc = self.doc_mut();
                    crate::editor::wrap::WordWrap::move_to_line_end(&mut doc.cursors, &doc.buffer, max_cols, select);
                } else {
                    let doc = self.doc_mut();
                    doc.cursors.move_to_line_end(&doc.buffer, select);
                }
            }
            EditorAction::MoveWordForward { select } => {
                let doc = self.doc_mut();
                doc.cursors.move_word_forward(&doc.buffer, select);
            }
            EditorAction::MoveWordBackward { select } => {
                let doc = self.doc_mut();
                doc.cursors.move_word_backward(&doc.buffer, select);
            }
            EditorAction::MoveFileStart { select } => {
                let doc = self.doc_mut();
                for c in doc.cursors.cursors_mut() {
                    c.head = 0;
                    if !select {
                        c.anchor = 0;
                    }
                }
                doc.cursors.normalize();
            }
            EditorAction::MoveFileEnd { select } => {
                let doc = self.doc_mut();
                let len = doc.buffer.len_chars();
                for c in doc.cursors.cursors_mut() {
                    c.head = len;
                    if !select {
                        c.anchor = len;
                    }
                }
                doc.cursors.normalize();
            }
            EditorAction::DeleteLine => {
                self.delete_lines();
                was_edit = true;
            }
            EditorAction::CopyLine => {
                self.copy_lines();
            }
            EditorAction::InsertLineBelow => {
                self.insert_line_below();
                was_edit = true;
            }
            EditorAction::InsertLineAbove => {
                self.insert_line_above();
                was_edit = true;
            }

            // Multi-cursors
            EditorAction::AddCursorAbove => {
                let count = {
                    let doc = self.doc_mut();
                    doc.cursors.add_cursor_above(&doc.buffer);
                    doc.cursors.cursors().len()
                };
                self.set_status(format!("Active cursors: {}", count));
            }
            EditorAction::AddCursorBelow => {
                let count = {
                    let doc = self.doc_mut();
                    doc.cursors.add_cursor_below(&doc.buffer);
                    doc.cursors.cursors().len()
                };
                self.set_status(format!("Active cursors: {}", count));
            }
            EditorAction::SelectNextOccurrence => {
                let found = {
                    let doc = self.doc_mut();
                    SearchEngine::select_next_occurrence(&doc.buffer, &mut doc.cursors)
                };
                if found {
                    self.set_status(format!("Cursors: {}", self.doc().cursors.cursors().len()));
                } else {
                    self.set_status("No further occurrences found");
                }
            }

            // History
            EditorAction::Undo => {
                let remaining = {
                    let doc = self.doc_mut();
                    if let Some(restored) = doc.history.undo(&mut doc.buffer) {
                        doc.cursors.set_cursors(restored);
                        Some(doc.history.undo_count())
                    } else {
                        None
                    }
                };
                if let Some(count) = remaining {
                    self.set_status(format!("Undo (remaining: {})", count));
                } else {
                    self.set_status("Already at oldest change");
                }
            }
            EditorAction::Redo => {
                let remaining = {
                    let doc = self.doc_mut();
                    if let Some(restored) = doc.history.redo(&mut doc.buffer) {
                        doc.cursors.set_cursors(restored);
                        Some(doc.history.redo_count())
                    } else {
                        None
                    }
                };
                if let Some(count) = remaining {
                    self.set_status(format!("Redo (remaining: {})", count));
                } else {
                    self.set_status("Already at newest change");
                }
            }

            // File & Modal commands
            EditorAction::Save => self.save_document(),
            EditorAction::OpenSearchPrompt => {
                let mut modal = PromptModal::new(PromptKind::Find);
                if self.doc().cursors.primary().has_selection() {
                    let range = self.doc().cursors.primary().selection_range();
                    let sel_text = self.doc().buffer.get_text_range(range.start, range.end);
                    if !sel_text.contains('\n') && sel_text.chars().count() <= 60 {
                        modal.cursor_col = sel_text.chars().count();
                        modal.input = sel_text;
                    }
                }
                self.mode = EditorMode::ModalPrompt(modal);
            }
            EditorAction::OpenReplacePrompt => {
                let mut modal = PromptModal::new(PromptKind::Replace);
                if self.doc().cursors.primary().has_selection() {
                    let range = self.doc().cursors.primary().selection_range();
                    let sel_text = self.doc().buffer.get_text_range(range.start, range.end);
                    if !sel_text.contains('\n') && sel_text.chars().count() <= 60 {
                        modal.cursor_col = sel_text.chars().count();
                        modal.input = sel_text;
                    }
                }
                self.mode = EditorMode::ModalPrompt(modal);
            }
            EditorAction::OpenGlobalSearchPrompt => {
                let initial_query = if self.doc().cursors.primary().has_selection() {
                    let range = self.doc().cursors.primary().selection_range();
                    let sel_text = self.doc().buffer.get_text_range(range.start, range.end);
                    if !sel_text.contains('\n') && sel_text.chars().count() <= 60 {
                        Some(sel_text)
                    } else {
                        None
                    }
                } else {
                    None
                };
                let root = self.sidebar.root_dir.clone();
                self.mode = EditorMode::ModalPrompt(PromptModal::new_global_search(root, initial_query.as_deref()));
            }
            EditorAction::OpenSaveAsPrompt => {
                let (dir, filename) = if let Some(path) = self.doc().buffer.file_path() {
                    let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
                    let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
                    (dir, Some(filename))
                } else {
                    let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                    (dir, None)
                };
                self.mode = EditorMode::ModalPrompt(PromptModal::new_with_dir(PromptKind::SaveAs, dir, filename));
            }
            EditorAction::OpenAboutDialog => {
                self.mode = EditorMode::AboutDialog;
            }
            EditorAction::OpenVimCommandLine => {
                self.mode = EditorMode::ModalPrompt(PromptModal::new(PromptKind::VimCommandLine));
            }
            EditorAction::OpenFileFinderPrompt => {
                let root = self.sidebar.root_dir.clone();
                let open_paths: Vec<PathBuf> = self.documents.iter().filter_map(|d| d.buffer.file_path().map(|p| p.to_path_buf())).collect();
                let mut modal = PromptModal::new_file_finder(root, &open_paths, &self.recent_files);
                modal.custom_commands = self.extension_manager.get_active_commands();
                modal.custom_commands.extend(user_theme_command_items());
                self.mode = EditorMode::ModalPrompt(modal);
            }
            EditorAction::OpenCommandPalette => {
                let mut modal = PromptModal::new(PromptKind::CommandPalette);
                modal.custom_commands = self.extension_manager.get_active_commands();
                modal.custom_commands.extend(user_theme_command_items());
                self.mode = EditorMode::ModalPrompt(modal);
            }
            EditorAction::OpenRecentFilesPrompt => {
                self.mode = EditorMode::ModalPrompt(PromptModal::new_recent_files(self.recent_files.clone()));
            }
            EditorAction::OpenExtensionsManager => {
                self.mode = EditorMode::ExtensionsManager { selected_idx: 0, scroll_offset: 0 };
            }
            EditorAction::ReloadExtensions => {
                self.extension_manager.reload();
                self.set_status(format!("Extensiones recargadas ({} encontradas)", self.extension_manager.extensions.len()));
            }
            EditorAction::CreateSampleExtension => {
                match self.extension_manager.create_sample_extension("mi_extension") {
                    Ok(p) => self.set_status(format!("Extensión creada en {}", p.display())),
                    Err(e) => self.set_status(format!("Error: {}", e)),
                }
            }
            EditorAction::ExecuteExtensionCommand(ref cmd_id) => {
                let id = cmd_id.clone();
                self.execute_extension_command(&id);
            }

            EditorAction::OpenConfig => {
                let config_path = crate::editor::config::EditorConfig::config_file_path();
                if !config_path.exists() {
                    if let Some(parent) = config_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&config_path, crate::editor::config::EditorConfig::default_ini_content());
                }
                self.open_file(&config_path);
                self.set_status("Configuración abierta (config.ini). Guarde con Ctrl+S para aplicar cambios.");
            }

            EditorAction::ReloadConfig => {
                self.reload_config();
            }

            EditorAction::SetTheme(theme) => {
                self.set_theme(theme.as_str());
            }

            EditorAction::SetThemeByName(theme_name) => {
                self.set_theme(&theme_name);
            }

            EditorAction::OpenFontSelector => {
                let family = crate::gfx::FontFamily::from_name(&self.config.gui.font_family);
                let size = self.config.gui.font_size;
                let weight = crate::gfx::FontWeight::from_name(&self.config.gui.font_weight);
                let line_spacing = self.config.gui.line_spacing;
                let ligatures = self.config.gui.font_ligatures;
                let antialiasing = self.config.gui.font_antialiasing;
                let hinting = crate::gfx::HintingLevel::from_str(&self.config.gui.font_hinting);
                let letter_spacing = self.config.gui.letter_spacing;
                let font_aspect_ratio = self.config.gui.font_aspect_ratio;
                let stem_darkening = self.config.gui.stem_darkening;
                let scale_factor = self.config.gui.scale_factor;
                let ergonomic_scaling = self.config.gui.ergonomic_scaling;
                let sdf_rendering = self.config.gui.sdf_font_rendering;
                let optical_spacing = self.config.gui.optical_punctuation_spacing;
                let use_ttf = self.config.gui.use_ttf_font;
                self.mode = EditorMode::FontSelector(crate::editor::font_selector::FontSelectorDialog::new(
                    family,
                    size,
                    weight,
                    line_spacing,
                    ligatures,
                    antialiasing,
                    hinting,
                    letter_spacing,
                    font_aspect_ratio,
                    stem_darkening,
                    scale_factor,
                    ergonomic_scaling,
                    sdf_rendering,
                    optical_spacing,
                    use_ttf,
                ));
            }

            EditorAction::IncreaseFontSize => {
                let current = self.config.gui.font_size;
                let sizes = crate::editor::font_selector::AVAILABLE_SIZES;
                let next = sizes.iter().copied().find(|&s| s > current).unwrap_or(*sizes.last().unwrap_or(&24));
                self.config.gui.font_size = next;
                let _ = self.config.save();
                self.set_status(format!("Tamaño de fuente: {} pt", next));
            }

            EditorAction::DecreaseFontSize => {
                let current = self.config.gui.font_size;
                let sizes = crate::editor::font_selector::AVAILABLE_SIZES;
                let prev = sizes.iter().copied().rev().find(|&s| s < current).unwrap_or(*sizes.first().unwrap_or(&10));
                self.config.gui.font_size = prev;
                let _ = self.config.save();
                self.set_status(format!("Tamaño de fuente: {} pt", prev));
            }

            EditorAction::ResetFontSize => {
                self.config.gui.font_size = 12;
                let _ = self.config.save();
                self.set_status("Tamaño de fuente restablecido a 12 pt");
            }

            EditorAction::SetFontFamily(family) => {
                self.config.gui.font_family = family.as_str().to_string();
                let _ = self.config.save();
                self.set_status(format!("Familia tipográfica: {}", family.display_name()));
            }

            EditorAction::SetFontSize(size) => {
                self.config.gui.font_size = size;
                let _ = self.config.save();
                self.set_status(format!("Tamaño de fuente: {} pt", size));
            }

            EditorAction::SetFontWeight(weight) => {
                self.config.gui.font_weight = weight.as_str().to_string();
                let _ = self.config.save();
                self.set_status(format!("Grosor de fuente: {}", weight.display_name()));
            }

            EditorAction::CollapseCursors => {
                self.snippet_session = None;
                self.signature_help = None;
                let primary = self.doc().cursors.primary();
                self.doc_mut().cursors.set_single(primary.head);
                self.set_status("Collapsed to single cursor");
            }

            EditorAction::Quit => {
                let any_dirty = self.documents.iter().any(|d| d.buffer.is_dirty());
                if any_dirty && !self.config.editor.hot_exit {
                    self.prompt_unsaved_quit();
                } else {
                    let _ = self.save_session();
                    self.should_quit = true;
                }
            }

            EditorAction::ToggleSidebar => {
                self.sidebar.is_visible = !self.sidebar.is_visible;
                if self.sidebar.is_visible {
                    self.sidebar.refresh();
                    self.set_status("Barra lateral mostrada");
                } else {
                    self.set_status("Barra lateral oculta");
                }
            }
            EditorAction::RefreshSidebar => {
                self.sidebar.refresh();
                self.set_status("Árbol de archivos actualizado");
            }
            EditorAction::FocusSidebar => {
                if self.sidebar.is_visible {
                    self.sidebar.is_focused = !self.sidebar.is_focused;
                }
            }

            EditorAction::FormatDocument => {
                self.format_active_document();
            }
            EditorAction::CheckDocument => {
                self.check_active_document();
            }
            EditorAction::NextDiagnostic => {
                self.jump_to_next_diagnostic();
            }
            EditorAction::PrevDiagnostic => {
                self.jump_to_prev_diagnostic();
            }

            EditorAction::TriggerAutocomplete => {
                self.trigger_autocomplete(true);
            }

            EditorAction::ToggleWordWrap => {
                self.toggle_word_wrap();
            }
            EditorAction::ToggleIndentGuides => {
                self.toggle_indent_guides();
            }
            EditorAction::ToggleHighlightCurrentLine => {
                self.toggle_highlight_current_line();
            }
            EditorAction::ToggleRulers => {
                self.toggle_rulers();
            }
            EditorAction::ToggleStickyScroll => {
                self.toggle_sticky_scroll();
            }

            EditorAction::ToggleMinimap => {
                self.toggle_minimap();
            }
            EditorAction::ToggleSplitVertical => {
                self.toggle_split_vertical();
            }
            EditorAction::ToggleSplitHorizontal => {
                self.toggle_split_horizontal();
            }
            EditorAction::SwitchSplitPane => {
                self.switch_split_pane();
            }
            EditorAction::CloseSplit => {
                self.close_split();
            }

            EditorAction::RefreshGit => {
                self.refresh_git();
            }
            EditorAction::ToggleGitGutter => {
                self.toggle_git_gutter();
            }

            EditorAction::ToggleTerminal => {
                self.terminal.toggle();
                if self.terminal.is_visible {
                    self.set_status(format!("Terminal Integrado activado ({}) - Ctrl+` para alternar", self.terminal.shell_name));
                } else {
                    self.set_status("Terminal Integrado oculto");
                }
            }
            EditorAction::FocusTerminal => {
                if self.terminal.is_visible {
                    self.terminal.is_focused = true;
                }
            }
            EditorAction::ClearTerminal => {
                self.terminal.clear();
                self.set_status("Terminal limpiado");
            }

            EditorAction::ToggleFold => {
                let primary_line = {
                    let doc = self.doc();
                    let pos = doc.buffer.char_to_position(doc.cursors.primary().head);
                    pos.line
                };
                let doc = self.doc_mut();
                doc.update_folding();
                if doc.folding.toggle_fold_at_line(primary_line) {
                    let head_pos = doc.buffer.char_to_position(doc.cursors.primary().head);
                    if doc.folding.is_line_hidden(head_pos.line) {
                        for r in &doc.folding.regions {
                            if r.is_folded && head_pos.line > r.start_line && head_pos.line <= r.end_line {
                                let target_char = doc.buffer.position_to_char(Position::new(r.start_line, doc.buffer.line_len(r.start_line)));
                                doc.cursors.set_single(target_char);
                                break;
                            }
                        }
                    }
                    self.set_status(format!("Plegado alternado en línea {}", primary_line + 1));
                } else {
                    self.set_status("No se encontró ningún bloque plegable en esta línea");
                }
            }
            EditorAction::FoldAll => {
                let doc = self.doc_mut();
                doc.update_folding();
                doc.folding.fold_all();
                let head_pos = doc.buffer.char_to_position(doc.cursors.primary().head);
                if doc.folding.is_line_hidden(head_pos.line) {
                    let visible_line = doc.folding.prev_visible_line(head_pos.line);
                    let target_char = doc.buffer.position_to_char(Position::new(visible_line, 0));
                    doc.cursors.set_single(target_char);
                }
                self.set_status("Todos los bloques plegados");
            }
            EditorAction::UnfoldAll => {
                let doc = self.doc_mut();
                doc.update_folding();
                doc.folding.unfold_all();
                self.set_status("Todos los bloques desplegados");
            }

            // Ergonomics & Advanced Editing
            EditorAction::MoveLineUp => {
                self.move_lines_up();
                was_edit = true;
            }
            EditorAction::MoveLineDown => {
                self.move_lines_down();
                was_edit = true;
            }
            EditorAction::DuplicateLineDown => {
                self.duplicate_lines_down();
                was_edit = true;
            }
            EditorAction::ToggleComment => {
                self.toggle_comment();
                was_edit = true;
            }
            EditorAction::JumpBack => {
                self.jump_back();
            }
            EditorAction::JumpForward => {
                self.jump_forward();
            }

            // Language Server Protocol (LSP)
            EditorAction::LspGoToDefinition => {
                self.lsp_go_to_definition();
            }
            EditorAction::LspHover => {
                self.lsp_hover();
            }
            EditorAction::LspFindReferences => {
                self.lsp_find_references();
            }
            EditorAction::LspRename => {
                self.lsp_rename_prompt();
            }
            EditorAction::LspCodeActions => {
                self.lsp_code_actions();
            }
            EditorAction::LspFormatDocument => {
                self.lsp_format_document();
            }
            EditorAction::LspDocumentSymbols => {
                self.lsp_document_symbols();
            }
            EditorAction::LspToggleInlayHints => {
                self.lsp_toggle_inlay_hints();
            }
            EditorAction::TriggerSignatureHelp => {
                self.trigger_signature_help(true);
            }
            EditorAction::RestartLspServer => {
                self.lsp_restart();
            }
            EditorAction::ToggleGitHunkDiff => {
                self.toggle_git_hunk_diff();
            }
            EditorAction::RevertGitHunk => {
                self.revert_active_hunk();
            }
            EditorAction::OpenGitDiffSplit => {
                self.open_git_diff_split();
            }
            EditorAction::GitStageFile => {
                self.git_stage_active_file();
            }
            EditorAction::GitUnstageFile => {
                self.git_unstage_active_file();
            }
            EditorAction::RefreshSemanticTokens => {
                self.refresh_semantic_tokens();
                self.set_status("LSP: Tokens semánticos actualizados");
            }

            EditorAction::SetKeymap(ref name) => {
                let profile = crate::editor::keymap_loader::load_keymap_profile(name);
                let label = profile.meta.name.clone();
                self.config.editor.keymap = name.clone();
                self.keymap_engine.set_profile(profile);
                self.set_status(&format!("Mapa de teclado activado: {}", label));
            }

            EditorAction::MinimizeWindow | EditorAction::ToggleMaximizeWindow => {}

            EditorAction::None => {}
        }

        if was_edit {
            self.lsp_notify_change_active_doc();
        }
    }

    // ========================================================================
    // Ergonomics & Advanced Editing
    // ========================================================================


}
