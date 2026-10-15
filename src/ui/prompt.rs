//! Interactive modal prompt for search, file saving/opening, and command palette.
//! Zero third-party dependencies, purely standard library Rust.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    Find,
    Replace,
    SaveAs,
    OpenFile,
    CommandPalette,
    GlobalSearch,
}

impl PromptKind {
    pub fn label(&self) -> &'static str {
        match self {
            PromptKind::Find => "Buscar: ",
            PromptKind::Replace => "Buscar: ",
            PromptKind::SaveAs => "Guardar como: ",
            PromptKind::OpenFile => "Abrir: ",
            PromptKind::CommandPalette => "Comando: ",
            PromptKind::GlobalSearch => "Buscar en Proyecto: ",
        }
    }

    pub fn window_title(&self) -> &'static str {
        match self {
            PromptKind::Find => "Buscar en Documento",
            PromptKind::Replace => "Buscar y Reemplazar",
            PromptKind::SaveAs => "Guardar Archivo Como",
            PromptKind::OpenFile => "Abrir Archivo",
            PromptKind::CommandPalette => "Paleta de Comandos",
            PromptKind::GlobalSearch => "Búsqueda Global en el Proyecto (Ctrl+Shift+F)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandItem {
    pub name: &'static str,
    pub shortcut: &'static str,
    pub action: crate::editor::keymap::EditorAction,
}

pub fn standard_commands() -> Vec<CommandItem> {
    use crate::editor::keymap::EditorAction;
    vec![
        CommandItem { name: "Nueva Pestaña", shortcut: "Ctrl+T", action: EditorAction::NewTab },
        CommandItem { name: "Abrir Archivo...", shortcut: "Ctrl+O", action: EditorAction::OpenOpenFilePrompt },
        CommandItem { name: "Guardar", shortcut: "Ctrl+S", action: EditorAction::Save },
        CommandItem { name: "Guardar Como...", shortcut: "", action: EditorAction::OpenSaveAsPrompt },
        CommandItem { name: "Cerrar Pestaña", shortcut: "Ctrl+W", action: EditorAction::CloseTab },
        CommandItem { name: "Buscar en Documento...", shortcut: "Ctrl+F", action: EditorAction::OpenSearchPrompt },
        CommandItem { name: "Reemplazar en Documento...", shortcut: "Ctrl+H", action: EditorAction::OpenReplacePrompt },
        CommandItem { name: "Buscar en Todo el Proyecto...", shortcut: "Ctrl+Shift+F", action: EditorAction::OpenGlobalSearchPrompt },
        CommandItem { name: "Deshacer", shortcut: "Ctrl+Z", action: EditorAction::Undo },
        CommandItem { name: "Rehacer", shortcut: "Ctrl+Y", action: EditorAction::Redo },
        CommandItem { name: "Cortar", shortcut: "Ctrl+X", action: EditorAction::Cut },
        CommandItem { name: "Copiar", shortcut: "Ctrl+C", action: EditorAction::Copy },
        CommandItem { name: "Pegar", shortcut: "Ctrl+V", action: EditorAction::Paste },
        CommandItem { name: "Añadir Cursor Arriba", shortcut: "Ctrl+K", action: EditorAction::AddCursorAbove },
        CommandItem { name: "Añadir Cursor Abajo", shortcut: "Ctrl+J", action: EditorAction::AddCursorBelow },
        CommandItem { name: "Siguiente Ocurrencia", shortcut: "Ctrl+D", action: EditorAction::SelectNextOccurrence },
        CommandItem { name: "Cursor Único", shortcut: "Esc", action: EditorAction::CollapseCursors },
        CommandItem { name: "Pestaña Siguiente", shortcut: "PageDown", action: EditorAction::NextTab },
        CommandItem { name: "Pestaña Anterior", shortcut: "PageUp", action: EditorAction::PrevTab },
        CommandItem { name: "Ver: Alternar Explorador Lateral", shortcut: "Ctrl+B", action: EditorAction::ToggleSidebar },
        CommandItem { name: "Ver: Actualizar Árbol del Explorador", shortcut: "", action: EditorAction::RefreshSidebar },
        CommandItem { name: "Ver: Alternar Minimapa", shortcut: "Ctrl+M", action: EditorAction::ToggleMinimap },
        CommandItem { name: "Ver: Dividir Editor Verticalmente", shortcut: "Ctrl+\\", action: EditorAction::ToggleSplitVertical },
        CommandItem { name: "Ver: Dividir Editor Horizontalmente", shortcut: "Ctrl+Shift+\\", action: EditorAction::ToggleSplitHorizontal },
        CommandItem { name: "Ver: Cambiar Foco de Panel", shortcut: "Alt+\\", action: EditorAction::SwitchSplitPane },
        CommandItem { name: "Ver: Cerrar División de Editor", shortcut: "", action: EditorAction::CloseSplit },
        CommandItem { name: "Administrador de Extensiones...", shortcut: "Ctrl+Shift+X", action: EditorAction::OpenExtensionsManager },
        CommandItem { name: "Extensiones: Recargar", shortcut: "", action: EditorAction::ReloadExtensions },
        CommandItem { name: "Extensiones: Crear Extensión de Ejemplo", shortcut: "", action: EditorAction::CreateSampleExtension },
        CommandItem { name: "Preferencias: Abrir Configuración (config.ini)", shortcut: "", action: EditorAction::OpenConfig },
        CommandItem { name: "Preferencias: Recargar Configuración", shortcut: "", action: EditorAction::ReloadConfig },
        CommandItem { name: "Tema: Tokyo Night", shortcut: "", action: EditorAction::SetTheme(crate::editor::config::EditorTheme::TokyoNight) },
        CommandItem { name: "Tema: Catppuccin Mocha", shortcut: "", action: EditorAction::SetTheme(crate::editor::config::EditorTheme::Catppuccin) },
        CommandItem { name: "Tema: One Dark", shortcut: "", action: EditorAction::SetTheme(crate::editor::config::EditorTheme::OneDark) },
        CommandItem { name: "Tema: Modern Dark", shortcut: "", action: EditorAction::SetTheme(crate::editor::config::EditorTheme::Dark) },
        CommandItem { name: "Tema: Monokai", shortcut: "", action: EditorAction::SetTheme(crate::editor::config::EditorTheme::Monokai) },
        CommandItem { name: "Tema: Solarized Dark", shortcut: "", action: EditorAction::SetTheme(crate::editor::config::EditorTheme::SolarizedDark) },
        CommandItem { name: "Tema: Claro (GitHub Light)", shortcut: "", action: EditorAction::SetTheme(crate::editor::config::EditorTheme::Light) },
        CommandItem { name: "Snippet: Insertar Función (fn)", shortcut: "Tab", action: EditorAction::InsertSnippet("fn".to_string()) },
        CommandItem { name: "Snippet: Insertar Estructura (struct)", shortcut: "Tab", action: EditorAction::InsertSnippet("struct".to_string()) },
        CommandItem { name: "Snippet: Insertar Bucle for", shortcut: "Tab", action: EditorAction::InsertSnippet("for".to_string()) },
        CommandItem { name: "Snippet: Insertar Expresión match", shortcut: "Tab", action: EditorAction::InsertSnippet("match".to_string()) },
        CommandItem { name: "Snippet: Insertar Test unitario (test)", shortcut: "Tab", action: EditorAction::InsertSnippet("test".to_string()) },
        CommandItem { name: "Herramientas: Formatear Documento", shortcut: "Ctrl+Shift+I", action: EditorAction::FormatDocument },
        CommandItem { name: "Herramientas: Comprobar Errores (Lint / Check)", shortcut: "F7", action: EditorAction::CheckDocument },
        CommandItem { name: "Herramientas: Siguiente Error", shortcut: "F8", action: EditorAction::NextDiagnostic },
        CommandItem { name: "Herramientas: Error Anterior", shortcut: "Shift+F8", action: EditorAction::PrevDiagnostic },
        CommandItem { name: "Git: Actualizar Estado del Repositorio", shortcut: "Ctrl+Shift+G", action: EditorAction::RefreshGit },
        CommandItem { name: "Git: Alternar Diff en Gutter (Git Gutter)", shortcut: "", action: EditorAction::ToggleGitGutter },
        CommandItem { name: "Acerca de Rustext", shortcut: "F1", action: EditorAction::OpenAboutDialog },
        CommandItem { name: "Salir de Rustext", shortcut: "Ctrl+Q", action: EditorAction::Quit },
    ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntryItem {
    pub name: String,
    pub is_dir: bool,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickBookmark {
    pub label: &'static str,
    pub path: PathBuf,
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub fn expand_tilde<P: AsRef<Path>>(path: P) -> PathBuf {
    let p = path.as_ref();
    if p == Path::new("~") {
        return home_dir().unwrap_or_else(|| p.to_path_buf());
    }
    if let Ok(stripped) = p.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(stripped);
        }
    }
    p.to_path_buf()
}

pub fn get_quick_bookmarks() -> Vec<QuickBookmark> {
    let mut bookmarks = Vec::new();

    // 1. Current working dir
    if let Ok(cur) = std::env::current_dir() {
        bookmarks.push(QuickBookmark {
            label: "Proyecto",
            path: cur,
        });
    }

    // 2. Home directory
    if let Some(home) = home_dir() {
        bookmarks.push(QuickBookmark {
            label: "Inicio (~)",
            path: home.clone(),
        });

        let docs = home.join("Documentos");
        if docs.exists() {
            bookmarks.push(QuickBookmark {
                label: "Documentos",
                path: docs,
            });
        } else {
            let docs_en = home.join("Documents");
            if docs_en.exists() {
                bookmarks.push(QuickBookmark {
                    label: "Documentos",
                    path: docs_en,
                });
            }
        }

        let downloads = home.join("Descargas");
        if downloads.exists() {
            bookmarks.push(QuickBookmark {
                label: "Descargas",
                path: downloads,
            });
        } else {
            let downloads_en = home.join("Downloads");
            if downloads_en.exists() {
                bookmarks.push(QuickBookmark {
                    label: "Descargas",
                    path: downloads_en,
                });
            }
        }

        let desk = home.join("Escritorio");
        if desk.exists() {
            bookmarks.push(QuickBookmark {
                label: "Escritorio",
                path: desk,
            });
        } else {
            let desk_en = home.join("Desktop");
            if desk_en.exists() {
                bookmarks.push(QuickBookmark {
                    label: "Escritorio",
                    path: desk_en,
                });
            }
        }
    }

    bookmarks
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptModal {
    pub kind: PromptKind,
    pub input: String,
    pub cursor_col: usize,
    pub selected_idx: usize,
    // Replace fields for PromptKind::Replace
    pub replace_input: String,
    pub replace_cursor_col: usize,
    pub active_field: usize, // 0 = Find input, 1 = Replace input
    // File/Folder browser fields for SaveAs and OpenFile
    pub current_dir: PathBuf,
    pub dir_entries: Vec<DirEntryItem>,
    pub selected_entry_idx: usize,
    pub scroll_offset: usize,
    // Custom commands injected by extension manager
    pub custom_commands: Vec<CommandItem>,
    // Global search fields for PromptKind::GlobalSearch
    pub global_results: Vec<crate::core::GlobalSearchMatch>,
    pub global_total_files: usize,
    pub global_matching_files: usize,
    pub global_truncated: bool,
    pub case_sensitive: bool,
    pub root_dir: PathBuf,
}

impl PromptModal {
    pub fn new(kind: PromptKind) -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut modal = Self {
            kind,
            input: String::new(),
            cursor_col: 0,
            selected_idx: 0,
            replace_input: String::new(),
            replace_cursor_col: 0,
            active_field: 0,
            current_dir: current_dir.clone(),
            dir_entries: Vec::new(),
            selected_entry_idx: 0,
            scroll_offset: 0,
            custom_commands: Vec::new(),
            global_results: Vec::new(),
            global_total_files: 0,
            global_matching_files: 0,
            global_truncated: false,
            case_sensitive: false,
            root_dir: current_dir,
        };
        if modal.is_file_dialog() {
            modal.refresh_dir_entries();
        }
        modal
    }

    pub fn new_with_dir(kind: PromptKind, initial_dir: PathBuf, default_filename: Option<&str>) -> Self {
        let dir = if initial_dir.is_dir() {
            initial_dir
        } else if let Some(parent) = initial_dir.parent() {
            if parent.as_os_str().is_empty() {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            } else {
                parent.to_path_buf()
            }
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        };

        let input = default_filename.unwrap_or("").to_string();
        let cursor_col = input.chars().count();

        let mut modal = Self {
            kind,
            input,
            cursor_col,
            selected_idx: 0,
            replace_input: String::new(),
            replace_cursor_col: 0,
            active_field: 0,
            current_dir: dir.clone(),
            dir_entries: Vec::new(),
            selected_entry_idx: 0,
            scroll_offset: 0,
            custom_commands: Vec::new(),
            global_results: Vec::new(),
            global_total_files: 0,
            global_matching_files: 0,
            global_truncated: false,
            case_sensitive: false,
            root_dir: dir,
        };
        if modal.is_file_dialog() {
            modal.refresh_dir_entries();
        }
        modal
    }

    pub fn new_global_search(root_dir: PathBuf, initial_query: Option<&str>) -> Self {
        let input = initial_query.unwrap_or("").to_string();
        let cursor_col = input.chars().count();
        let mut modal = Self {
            kind: PromptKind::GlobalSearch,
            input,
            cursor_col,
            selected_idx: 0,
            replace_input: String::new(),
            replace_cursor_col: 0,
            active_field: 0,
            current_dir: root_dir.clone(),
            dir_entries: Vec::new(),
            selected_entry_idx: 0,
            scroll_offset: 0,
            custom_commands: Vec::new(),
            global_results: Vec::new(),
            global_total_files: 0,
            global_matching_files: 0,
            global_truncated: false,
            case_sensitive: false,
            root_dir,
        };
        if !modal.input.trim().is_empty() {
            modal.update_global_search();
        }
        modal
    }

    pub fn is_file_dialog(&self) -> bool {
        matches!(self.kind, PromptKind::SaveAs | PromptKind::OpenFile)
    }

    pub fn refresh_dir_entries(&mut self) {
        let mut entries = Vec::new();

        // 1. Parent folder entry ".." if current_dir has a parent
        if let Some(parent) = self.current_dir.parent() {
            entries.push(DirEntryItem {
                name: ".. [Subir nivel]".to_string(),
                is_dir: true,
                path: parent.to_path_buf(),
            });
        }

        // 2. Read entries from current_dir
        if let Ok(read_dir) = fs::read_dir(&self.current_dir) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for entry in read_dir.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                // Filter hidden files/folders starting with '.' unless input starts with '.'
                if file_name.starts_with('.') && !self.input.starts_with('.') {
                    continue;
                }
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let path = entry.path();

                if is_dir {
                    dirs.push(DirEntryItem {
                        name: format!("{}/", file_name),
                        is_dir: true,
                        path,
                    });
                } else {
                    files.push(DirEntryItem {
                        name: file_name,
                        is_dir: false,
                        path,
                    });
                }
            }

            // Sort case-insensitively
            dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            entries.extend(dirs);
            entries.extend(files);
        }

        self.dir_entries = entries;
        self.selected_entry_idx = 0;
        self.scroll_offset = 0;
    }

    pub fn navigate_to(&mut self, target: PathBuf) {
        let clean_path = if let Ok(canonical) = fs::canonicalize(&target) {
            canonical
        } else {
            target
        };
        if clean_path.is_dir() {
            self.current_dir = clean_path;
            self.refresh_dir_entries();
        }
    }

    pub fn navigate_up(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            let parent_buf = parent.to_path_buf();
            self.navigate_to(parent_buf);
        }
    }

    pub fn select_next_entry(&mut self, visible_count: usize) {
        if !self.dir_entries.is_empty() && self.selected_entry_idx + 1 < self.dir_entries.len() {
            self.selected_entry_idx += 1;
            if self.selected_entry_idx >= self.scroll_offset + visible_count {
                self.scroll_offset = self.selected_entry_idx.saturating_sub(visible_count) + 1;
            }
            if !self.dir_entries[self.selected_entry_idx].is_dir {
                self.input = self.dir_entries[self.selected_entry_idx].name.clone();
                self.cursor_col = self.input.chars().count();
            }
        }
    }

    pub fn select_prev_entry(&mut self) {
        if self.selected_entry_idx > 0 {
            self.selected_entry_idx -= 1;
            if self.selected_entry_idx < self.scroll_offset {
                self.scroll_offset = self.selected_entry_idx;
            }
            if !self.dir_entries[self.selected_entry_idx].is_dir {
                self.input = self.dir_entries[self.selected_entry_idx].name.clone();
                self.cursor_col = self.input.chars().count();
            }
        }
    }

    pub fn scroll_down(&mut self, count: usize, max_visible: usize) {
        if !self.dir_entries.is_empty() {
            let max_offset = self.dir_entries.len().saturating_sub(max_visible);
            self.scroll_offset = (self.scroll_offset + count).min(max_offset);
        }
    }

    pub fn scroll_up(&mut self, count: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(count);
    }

    pub fn resolve_full_path(&self) -> PathBuf {
        let trimmed = self.input.trim();
        if trimmed.is_empty() {
            return self.current_dir.clone();
        }

        let path = expand_tilde(trimmed);
        if path.is_absolute() {
            path
        } else {
            self.current_dir.join(path)
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        if self.kind == PromptKind::Replace && self.active_field == 1 {
            let char_count = self.replace_input.chars().count();
            let idx = self.replace_cursor_col.min(char_count);

            let mut new_input = String::with_capacity(self.replace_input.len() + 4);
            for (i, c) in self.replace_input.chars().enumerate() {
                if i == idx {
                    new_input.push(ch);
                }
                new_input.push(c);
            }
            if idx == char_count {
                new_input.push(ch);
            }

            self.replace_input = new_input;
            self.replace_cursor_col += 1;
            return;
        }

        let char_count = self.input.chars().count();
        let idx = self.cursor_col.min(char_count);

        let mut new_input = String::with_capacity(self.input.len() + 4);
        for (i, c) in self.input.chars().enumerate() {
            if i == idx {
                new_input.push(ch);
            }
            new_input.push(c);
        }
        if idx == char_count {
            new_input.push(ch);
        }

        self.input = new_input;
        self.cursor_col += 1;
        self.selected_idx = 0;
        if self.kind == PromptKind::GlobalSearch {
            self.update_global_search();
        }
    }

    pub fn backspace(&mut self) {
        if self.kind == PromptKind::Replace && self.active_field == 1 {
            if self.replace_cursor_col == 0 || self.replace_input.is_empty() {
                return;
            }

            let delete_idx = self.replace_cursor_col - 1;
            self.replace_input = self
                .replace_input
                .chars()
                .enumerate()
                .filter(|(i, _)| *i != delete_idx)
                .map(|(_, c)| c)
                .collect();

            self.replace_cursor_col -= 1;
            return;
        }

        if self.cursor_col == 0 || self.input.is_empty() {
            return;
        }

        let delete_idx = self.cursor_col - 1;
        self.input = self
            .input
            .chars()
            .enumerate()
            .filter(|(i, _)| *i != delete_idx)
            .map(|(_, c)| c)
            .collect();

        self.cursor_col -= 1;
        self.selected_idx = 0;
        if self.kind == PromptKind::GlobalSearch {
            self.update_global_search();
        }
    }

    /// Re-runs global project search against current input query.
    pub fn update_global_search(&mut self) {
        let trimmed = self.input.trim();
        if trimmed.is_empty() {
            self.global_results.clear();
            self.global_total_files = 0;
            self.global_matching_files = 0;
            self.global_truncated = false;
            self.selected_idx = 0;
            self.scroll_offset = 0;
            return;
        }

        let res = crate::core::GlobalSearchEngine::search_project(
            &self.root_dir,
            trimmed,
            self.case_sensitive,
            1000,
        );

        self.global_results = res.matches;
        self.global_total_files = res.total_files_scanned;
        self.global_matching_files = res.matching_files_count;
        self.global_truncated = res.truncated;
        self.selected_idx = 0;
        self.scroll_offset = 0;
    }

    /// Toggles case-sensitive matching and re-evaluates the query.
    pub fn toggle_case_sensitivity(&mut self) {
        self.case_sensitive = !self.case_sensitive;
        self.update_global_search();
    }

    /// Moves selection to the next matching result in the list.
    pub fn select_next_global_result(&mut self, max_visible: usize) {
        if self.global_results.is_empty() {
            return;
        }
        if self.selected_idx + 1 < self.global_results.len() {
            self.selected_idx += 1;
        }
        self.adjust_global_scroll(max_visible);
    }

    /// Moves selection to the previous matching result in the list.
    pub fn select_prev_global_result(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
        }
        if self.selected_idx < self.scroll_offset {
            self.scroll_offset = self.selected_idx;
        }
    }

    /// Adjusts scroll offset to ensure `selected_idx` remains within visible viewport.
    pub fn adjust_global_scroll(&mut self, max_visible: usize) {
        let max_vis = max_visible.max(1);
        if self.selected_idx < self.scroll_offset {
            self.scroll_offset = self.selected_idx;
        } else if self.selected_idx >= self.scroll_offset + max_vis {
            self.scroll_offset = self.selected_idx + 1 - max_vis;
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.kind == PromptKind::Replace && self.active_field == 1 {
            if self.replace_cursor_col > 0 {
                self.replace_cursor_col -= 1;
            }
        } else if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.kind == PromptKind::Replace && self.active_field == 1 {
            let char_count = self.replace_input.chars().count();
            if self.replace_cursor_col < char_count {
                self.replace_cursor_col += 1;
            }
        } else {
            let char_count = self.input.chars().count();
            if self.cursor_col < char_count {
                self.cursor_col += 1;
            }
        }
    }

    pub fn toggle_active_field(&mut self) {
        if self.kind == PromptKind::Replace {
            self.active_field = (self.active_field + 1) % 2;
        }
    }

    pub fn formatted_display(&self) -> String {
        if self.is_file_dialog() {
            format!("{}[{}] {}", self.kind.label(), self.current_dir.display(), self.input)
        } else if self.kind == PromptKind::Replace {
            let field_hint = if self.active_field == 0 { "Campo: Buscar" } else { "Campo: Reemplazar" };
            format!(
                "Buscar: {} │ Reemplazar con: {} │ [{}] [Tab: Cambiar] [Enter: Sig] [Alt+R: Reemplazar] [Alt+A: Todo]",
                self.input, self.replace_input, field_hint
            )
        } else {
            format!("{}{}", self.kind.label(), self.input)
        }
    }

    pub fn matching_commands(&self) -> Vec<CommandItem> {
        if self.kind != PromptKind::CommandPalette {
            return Vec::new();
        }
        let term = self.input.trim().to_lowercase();
        let mut all = standard_commands();
        all.extend(self.custom_commands.clone());
        if term.is_empty() {
            return all;
        }
        all.into_iter()
            .filter(|cmd| cmd.name.to_lowercase().contains(&term))
            .collect()
    }
}
