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
    RecentFiles,
    CommandPalette,
    GlobalSearch,
    FileFinder,
    RecentProjects,
    OpenFolder,
    References,
    RenameSymbol,
    CodeActions,
    DocumentSymbols,
}

impl PromptKind {
    pub fn label(&self) -> &'static str {
        match self {
            PromptKind::Find => "Buscar: ",
            PromptKind::Replace => "Buscar: ",
            PromptKind::SaveAs => "Guardar como: ",
            PromptKind::OpenFile => "Abrir: ",
            PromptKind::RecentFiles => "Reciente: ",
            PromptKind::CommandPalette => "Comando: ",
            PromptKind::GlobalSearch => "Buscar en Proyecto: ",
            PromptKind::FileFinder => "Archivo: ",
            PromptKind::RecentProjects => "Proyecto: ",
            PromptKind::OpenFolder => "Carpeta: ",
            PromptKind::References => "Filtrar: ",
            PromptKind::RenameSymbol => "Nuevo nombre: ",
            PromptKind::CodeActions => "Acción: ",
            PromptKind::DocumentSymbols => "Símbolo: ",
        }
    }

    pub fn window_title(&self) -> &'static str {
        match self {
            PromptKind::Find => "Buscar en Documento",
            PromptKind::Replace => "Buscar y Reemplazar",
            PromptKind::SaveAs => "Guardar Archivo Como",
            PromptKind::OpenFile => "Abrir Archivo",
            PromptKind::RecentFiles => "Archivos Recientes (Ctrl+R)",
            PromptKind::CommandPalette => "Paleta de Comandos (Ctrl+Shift+P)",
            PromptKind::GlobalSearch => "Búsqueda Global en el Proyecto (Ctrl+Shift+F)",
            PromptKind::FileFinder => "Ir a Archivo (Fuzzy File Finder) [Ctrl+P]",
            PromptKind::RecentProjects => "Espacios de Trabajo y Proyectos Recientes (Ctrl+Alt+P)",
            PromptKind::OpenFolder => "Abrir Carpeta / Proyecto (Ctrl+Shift+O)",
            PromptKind::References => "Buscar Todas las Referencias (Ctrl+Shift+U)",
            PromptKind::RenameSymbol => "Renombrar Símbolo en el Proyecto (F2)",
            PromptKind::CodeActions => "Acciones de Código y Quick Fixes (Ctrl+.)",
            PromptKind::DocumentSymbols => "Símbolos del Documento / Esquema (Ctrl+Shift+R)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandItem {
    pub name: std::borrow::Cow<'static, str>,
    pub shortcut: &'static str,
    pub action: crate::editor::keymap::EditorAction,
}

pub fn standard_commands() -> Vec<CommandItem> {
    use crate::editor::keymap::EditorAction;
    vec![
        CommandItem { name: "Ir a Archivo... (Fuzzy File Finder)".into(), shortcut: "Ctrl+P", action: EditorAction::OpenFileFinderPrompt },
        CommandItem { name: "Paleta de Comandos...".into(), shortcut: "Ctrl+Shift+P", action: EditorAction::OpenCommandPalette },
        CommandItem { name: "Nueva Pestaña".into(), shortcut: "Ctrl+T", action: EditorAction::NewTab },
        CommandItem { name: "Abrir Archivo...".into(), shortcut: "Ctrl+O", action: EditorAction::OpenOpenFilePrompt },
        CommandItem { name: "Abrir Archivo Reciente...".into(), shortcut: "Ctrl+R", action: EditorAction::OpenRecentFilesPrompt },
        CommandItem { name: "Espacio de Trabajo: Abrir Carpeta...".into(), shortcut: "Ctrl+Shift+O", action: EditorAction::OpenFolderPrompt },
        CommandItem { name: "Espacio de Trabajo: Proyectos Recientes...".into(), shortcut: "Ctrl+Alt+P", action: EditorAction::OpenRecentProjectsPrompt },
        CommandItem { name: "Espacio de Trabajo: Cerrar Proyecto".into(), shortcut: "", action: EditorAction::CloseWorkspace },
        CommandItem { name: "Guardar".into(), shortcut: "Ctrl+S", action: EditorAction::Save },
        CommandItem { name: "Guardar Como...".into(), shortcut: "", action: EditorAction::OpenSaveAsPrompt },
        CommandItem { name: "Cerrar Pestaña".into(), shortcut: "Ctrl+W", action: EditorAction::CloseTab },
        CommandItem { name: "Buscar en Documento...".into(), shortcut: "Ctrl+F", action: EditorAction::OpenSearchPrompt },
        CommandItem { name: "Reemplazar en Documento...".into(), shortcut: "Ctrl+H", action: EditorAction::OpenReplacePrompt },
        CommandItem { name: "Buscar en Todo el Proyecto...".into(), shortcut: "Ctrl+Shift+F", action: EditorAction::OpenGlobalSearchPrompt },
        CommandItem { name: "Deshacer".into(), shortcut: "Ctrl+Z", action: EditorAction::Undo },
        CommandItem { name: "Rehacer".into(), shortcut: "Ctrl+Y", action: EditorAction::Redo },
        CommandItem { name: "Cortar".into(), shortcut: "Ctrl+X", action: EditorAction::Cut },
        CommandItem { name: "Copiar".into(), shortcut: "Ctrl+C", action: EditorAction::Copy },
        CommandItem { name: "Pegar".into(), shortcut: "Ctrl+V", action: EditorAction::Paste },
        CommandItem { name: "Editar: Mover Línea Arriba".into(), shortcut: "Alt+Up", action: EditorAction::MoveLineUp },
        CommandItem { name: "Editar: Mover Línea Abajo".into(), shortcut: "Alt+Down", action: EditorAction::MoveLineDown },
        CommandItem { name: "Editar: Duplicar Línea Abajo".into(), shortcut: "Ctrl+Shift+D", action: EditorAction::DuplicateLineDown },
        CommandItem { name: "Editar: Alternar Comentario".into(), shortcut: "Ctrl+/", action: EditorAction::ToggleComment },
        CommandItem { name: "Navegación: Salto Atrás".into(), shortcut: "Alt+Left", action: EditorAction::JumpBack },
        CommandItem { name: "Navegación: Salto Adelante".into(), shortcut: "Alt+Right", action: EditorAction::JumpForward },
        CommandItem { name: "Añadir Cursor Arriba".into(), shortcut: "Ctrl+K", action: EditorAction::AddCursorAbove },
        CommandItem { name: "Añadir Cursor Abajo".into(), shortcut: "Ctrl+J", action: EditorAction::AddCursorBelow },
        CommandItem { name: "Siguiente Ocurrencia".into(), shortcut: "Ctrl+D", action: EditorAction::SelectNextOccurrence },
        CommandItem { name: "Cursor Único".into(), shortcut: "Esc", action: EditorAction::CollapseCursors },
        CommandItem { name: "Pestaña Siguiente".into(), shortcut: "PageDown", action: EditorAction::NextTab },
        CommandItem { name: "Pestaña Anterior".into(), shortcut: "PageUp", action: EditorAction::PrevTab },
        CommandItem { name: "Ver: Alternar Explorador Lateral".into(), shortcut: "Ctrl+B", action: EditorAction::ToggleSidebar },
        CommandItem { name: "Ver: Actualizar Árbol del Explorador".into(), shortcut: "", action: EditorAction::RefreshSidebar },
        CommandItem { name: "Ver: Alternar Ajuste de Línea (Word Wrap)".into(), shortcut: "Alt+Z", action: EditorAction::ToggleWordWrap },
        CommandItem { name: "Ver: Alternar Guías de Indentación (Indent Guides)".into(), shortcut: "Alt+I", action: EditorAction::ToggleIndentGuides },
        CommandItem { name: "Ver: Alternar Resaltado de Línea Actual".into(), shortcut: "", action: EditorAction::ToggleHighlightCurrentLine },
        CommandItem { name: "Ver: Alternar Líneas Guía de Límite (Rulers 80/120)".into(), shortcut: "", action: EditorAction::ToggleRulers },
        CommandItem { name: "Ver: Alternar Encabezados Fijos (Sticky Scroll)".into(), shortcut: "", action: EditorAction::ToggleStickyScroll },
        CommandItem { name: "Ver: Plegar/Desplegar Bloque (Toggle Fold)".into(), shortcut: "Alt+[", action: EditorAction::ToggleFold },
        CommandItem { name: "Ver: Plegar Todo (Fold All)".into(), shortcut: "Alt+0", action: EditorAction::FoldAll },
        CommandItem { name: "Ver: Desplegar Todo (Unfold All)".into(), shortcut: "Alt+J", action: EditorAction::UnfoldAll },
        CommandItem { name: "Ver: Alternar Minimapa".into(), shortcut: "Ctrl+M", action: EditorAction::ToggleMinimap },
        CommandItem { name: "Ver: Alternar Terminal Integrado".into(), shortcut: "Ctrl+`", action: EditorAction::ToggleTerminal },
        CommandItem { name: "Terminal: Limpiar Pantalla".into(), shortcut: "Ctrl+L", action: EditorAction::ClearTerminal },
        CommandItem { name: "Ver: Dividir Editor Verticalmente".into(), shortcut: "Ctrl+\\", action: EditorAction::ToggleSplitVertical },
        CommandItem { name: "Ver: Dividir Editor Horizontalmente".into(), shortcut: "Ctrl+Shift+\\", action: EditorAction::ToggleSplitHorizontal },
        CommandItem { name: "Ver: Cambiar Foco de Panel".into(), shortcut: "Alt+\\", action: EditorAction::SwitchSplitPane },
        CommandItem { name: "Ver: Cerrar División de Editor".into(), shortcut: "", action: EditorAction::CloseSplit },
        CommandItem { name: "Administrador de Extensiones...".into(), shortcut: "Ctrl+Shift+X", action: EditorAction::OpenExtensionsManager },
        CommandItem { name: "Extensiones: Recargar".into(), shortcut: "", action: EditorAction::ReloadExtensions },
        CommandItem { name: "Extensiones: Crear Extensión de Ejemplo".into(), shortcut: "", action: EditorAction::CreateSampleExtension },
        CommandItem { name: "Preferencias: Abrir Configuración (config.ini)".into(), shortcut: "", action: EditorAction::OpenConfig },
        CommandItem { name: "Preferencias: Recargar Configuración".into(), shortcut: "", action: EditorAction::ReloadConfig },
        CommandItem { name: "Preferencias: Selector de Fuentes...".into(), shortcut: "Ctrl+Shift+T", action: EditorAction::OpenFontSelector },
        CommandItem { name: "Fuente: Aumentar Tamaño de Fuente".into(), shortcut: "Ctrl++", action: EditorAction::IncreaseFontSize },
        CommandItem { name: "Fuente: Disminuir Tamaño de Fuente".into(), shortcut: "Ctrl+-", action: EditorAction::DecreaseFontSize },
        CommandItem { name: "Fuente: Restablecer Tamaño de Fuente (12pt)".into(), shortcut: "Ctrl+0", action: EditorAction::ResetFontSize },
        CommandItem { name: "Fuente: Standard Mono".into(), shortcut: "", action: EditorAction::SetFontFamily(crate::gfx::FontFamily::Standard) },
        CommandItem { name: "Fuente: Terminus Console".into(), shortcut: "", action: EditorAction::SetFontFamily(crate::gfx::FontFamily::Terminus) },
        CommandItem { name: "Fuente: Modern Coding Sans".into(), shortcut: "", action: EditorAction::SetFontFamily(crate::gfx::FontFamily::Modern) },
        CommandItem { name: "Fuente: Typewriter Slab".into(), shortcut: "", action: EditorAction::SetFontFamily(crate::gfx::FontFamily::Typewriter) },
        CommandItem { name: "Fuente: Retro Pixel".into(), shortcut: "", action: EditorAction::SetFontFamily(crate::gfx::FontFamily::Retro) },
        CommandItem { name: "Tema: Tokyo Night".into(), shortcut: "", action: EditorAction::SetTheme(crate::editor::config::EditorTheme::TokyoNight) },
        CommandItem { name: "Tema: Catppuccin Mocha".into(), shortcut: "", action: EditorAction::SetTheme(crate::editor::config::EditorTheme::Catppuccin) },
        CommandItem { name: "Tema: One Dark".into(), shortcut: "", action: EditorAction::SetTheme(crate::editor::config::EditorTheme::OneDark) },
        CommandItem { name: "Tema: Modern Dark".into(), shortcut: "", action: EditorAction::SetTheme(crate::editor::config::EditorTheme::Dark) },
        CommandItem { name: "Tema: Monokai".into(), shortcut: "", action: EditorAction::SetTheme(crate::editor::config::EditorTheme::Monokai) },
        CommandItem { name: "Tema: Solarized Dark".into(), shortcut: "", action: EditorAction::SetTheme(crate::editor::config::EditorTheme::SolarizedDark) },
        CommandItem { name: "Tema: Claro (GitHub Light)".into(), shortcut: "", action: EditorAction::SetTheme(crate::editor::config::EditorTheme::Light) },
        CommandItem { name: "Tema: Dracula".into(), shortcut: "", action: EditorAction::SetTheme(crate::editor::config::EditorTheme::Dracula) },
        CommandItem { name: "Snippet: Insertar Función (fn)".into(), shortcut: "Tab", action: EditorAction::InsertSnippet("fn".to_string()) },
        CommandItem { name: "Snippet: Insertar Estructura (struct)".into(), shortcut: "Tab", action: EditorAction::InsertSnippet("struct".to_string()) },
        CommandItem { name: "Snippet: Insertar Bucle for".into(), shortcut: "Tab", action: EditorAction::InsertSnippet("for".to_string()) },
        CommandItem { name: "Snippet: Insertar Expresión match".into(), shortcut: "Tab", action: EditorAction::InsertSnippet("match".to_string()) },
        CommandItem { name: "Snippet: Insertar Test unitario (test)".into(), shortcut: "Tab", action: EditorAction::InsertSnippet("test".to_string()) },
        CommandItem { name: "Herramientas: Formatear Documento".into(), shortcut: "Ctrl+Shift+I", action: EditorAction::FormatDocument },
        CommandItem { name: "Herramientas: Comprobar Errores (Lint / Check)".into(), shortcut: "F7", action: EditorAction::CheckDocument },
        CommandItem { name: "Herramientas: Siguiente Error".into(), shortcut: "F8", action: EditorAction::NextDiagnostic },
        CommandItem { name: "Herramientas: Error Anterior".into(), shortcut: "Shift+F8", action: EditorAction::PrevDiagnostic },
        CommandItem { name: "Git: Actualizar Estado del Repositorio".into(), shortcut: "Ctrl+Shift+G", action: EditorAction::RefreshGit },
        CommandItem { name: "Git: Alternar Diff en Gutter (Git Gutter)".into(), shortcut: "", action: EditorAction::ToggleGitGutter },
        CommandItem { name: "LSP: Ir a Definición".into(), shortcut: "F12", action: EditorAction::LspGoToDefinition },
        CommandItem { name: "LSP: Buscar Todas las Referencias".into(), shortcut: "Ctrl+Shift+U", action: EditorAction::LspFindReferences },
        CommandItem { name: "LSP: Renombrar Símbolo...".into(), shortcut: "F2", action: EditorAction::LspRename },
        CommandItem { name: "LSP: Acciones de Código y Quick Fixes...".into(), shortcut: "Ctrl+.", action: EditorAction::LspCodeActions },
        CommandItem { name: "LSP: Formatear Documento".into(), shortcut: "Ctrl+Shift+I", action: EditorAction::LspFormatDocument },
        CommandItem { name: "LSP: Símbolos del Documento (Esquema)...".into(), shortcut: "Ctrl+Shift+R", action: EditorAction::LspDocumentSymbols },
        CommandItem { name: "LSP: Información de Tipo (Hover)".into(), shortcut: "Alt+K", action: EditorAction::LspHover },
        CommandItem { name: "LSP: Firma de Función (Signature Help)".into(), shortcut: "Ctrl+Shift+Space", action: EditorAction::TriggerSignatureHelp },
        CommandItem { name: "LSP: Alternar Sugerencias en Línea (Inlay Hints)".into(), shortcut: "", action: EditorAction::LspToggleInlayHints },
        CommandItem { name: "LSP: Refrescar Resaltado Semántico".into(), shortcut: "F5", action: EditorAction::RefreshSemanticTokens },
        CommandItem { name: "LSP: Reiniciar Servidor".into(), shortcut: "", action: EditorAction::RestartLspServer },
        CommandItem { name: "Acerca de Rustext".into(), shortcut: "F1", action: EditorAction::OpenAboutDialog },
        CommandItem { name: "Salir de Rustext".into(), shortcut: "Ctrl+Q", action: EditorAction::Quit },
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
    pub recent_files: Vec<PathBuf>,
    pub recent_projects: Vec<crate::editor::workspace::WorkspaceItem>,
    // Fuzzy File Finder fields for PromptKind::FileFinder
    pub project_files: Vec<crate::core::ProjectFile>,
    pub file_finder_results: Vec<crate::core::FileFinderMatch>,
    // LSP Elite Prompt fields
    pub references: Vec<crate::editor::lsp::LspReferenceItem>,
    pub rename_symbol_target: Option<String>,
    pub code_actions: Vec<crate::editor::lsp::LspCodeAction>,
    pub document_symbols: Vec<crate::editor::lsp::LspDocumentSymbol>,
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
            recent_files: Vec::new(),
            recent_projects: Vec::new(),
            project_files: Vec::new(),
            file_finder_results: Vec::new(),
            references: Vec::new(),
            rename_symbol_target: None,
            code_actions: Vec::new(),
            document_symbols: Vec::new(),
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
            recent_files: Vec::new(),
            recent_projects: Vec::new(),
            project_files: Vec::new(),
            file_finder_results: Vec::new(),
            references: Vec::new(),
            rename_symbol_target: None,
            code_actions: Vec::new(),
            document_symbols: Vec::new(),
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
            recent_files: Vec::new(),
            recent_projects: Vec::new(),
            project_files: Vec::new(),
            file_finder_results: Vec::new(),
            references: Vec::new(),
            rename_symbol_target: None,
            code_actions: Vec::new(),
            document_symbols: Vec::new(),
        };
        if !modal.input.trim().is_empty() {
            modal.update_global_search();
        }
        modal
    }

    /// Creates a prompt modal for workspace fuzzy file finding (Ctrl+P).
    pub fn new_file_finder(
        root_dir: PathBuf,
        open_paths: &[PathBuf],
        recent_paths: &[PathBuf],
    ) -> Self {
        let project_files = crate::core::FileFinder::scan_workspace(&root_dir, open_paths, recent_paths);
        let file_finder_results = crate::core::FileFinder::filter_and_rank(&project_files, "", 40);

        Self {
            kind: PromptKind::FileFinder,
            input: String::new(),
            cursor_col: 0,
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
            recent_files: recent_paths.to_vec(),
            recent_projects: Vec::new(),
            project_files,
            file_finder_results,
            references: Vec::new(),
            rename_symbol_target: None,
            code_actions: Vec::new(),
            document_symbols: Vec::new(),
        }
    }

    /// Creates a prompt modal for selecting and filtering recent files.
    pub fn new_recent_files(recent_files: Vec<PathBuf>) -> Self {
        let mut modal = Self::new(PromptKind::RecentFiles);
        modal.recent_files = recent_files;
        modal
    }

    /// Returns matching recent files filtered by current input query.
    pub fn matching_recent_files(&self) -> Vec<&PathBuf> {
        let query = self.input.trim().to_lowercase();
        if query.is_empty() {
            return self.recent_files.iter().collect();
        }

        self.recent_files
            .iter()
            .filter(|p| {
                let s = p.to_string_lossy().to_lowercase();
                s.contains(&query)
            })
            .collect()
    }

    pub fn is_file_dialog(&self) -> bool {
        matches!(self.kind, PromptKind::SaveAs | PromptKind::OpenFile | PromptKind::OpenFolder)
    }

    /// Creates a prompt modal for selecting and filtering recent projects.
    pub fn new_recent_projects(recent_projects: Vec<crate::editor::workspace::WorkspaceItem>) -> Self {
        let mut modal = Self::new(PromptKind::RecentProjects);
        modal.recent_projects = recent_projects;
        modal
    }

    /// Returns matching recent projects filtered by current input query.
    pub fn matching_projects(&self) -> Vec<&crate::editor::workspace::WorkspaceItem> {
        let query = self.input.trim().to_lowercase();
        if query.is_empty() {
            return self.recent_projects.iter().collect();
        }

        self.recent_projects
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query)
                    || p.path.to_string_lossy().to_lowercase().contains(&query)
                    || p.kind_tag.to_lowercase().contains(&query)
            })
            .collect()
    }

    /// Creates a prompt modal for selecting from symbol references.
    pub fn new_references(references: Vec<crate::editor::lsp::LspReferenceItem>) -> Self {
        let mut modal = Self::new(PromptKind::References);
        modal.references = references;
        modal
    }

    /// Returns matching references filtered by input query.
    pub fn matching_references(&self) -> Vec<&crate::editor::lsp::LspReferenceItem> {
        let query = self.input.trim().to_lowercase();
        if query.is_empty() {
            return self.references.iter().collect();
        }

        self.references
            .iter()
            .filter(|r| {
                r.path.to_string_lossy().to_lowercase().contains(&query)
                    || r.line_text.to_lowercase().contains(&query)
            })
            .collect()
    }

    /// Creates a prompt modal for renaming a symbol.
    pub fn new_rename(current_name: &str) -> Self {
        let mut modal = Self::new(PromptKind::RenameSymbol);
        modal.input = current_name.to_string();
        modal.cursor_col = modal.input.chars().count();
        modal.rename_symbol_target = Some(current_name.to_string());
        modal
    }

    /// Creates a prompt modal for selecting code actions and quick fixes.
    pub fn new_code_actions(actions: Vec<crate::editor::lsp::LspCodeAction>) -> Self {
        let mut modal = Self::new(PromptKind::CodeActions);
        modal.code_actions = actions;
        modal
    }

    /// Returns matching code actions filtered by input query.
    pub fn matching_code_actions(&self) -> Vec<&crate::editor::lsp::LspCodeAction> {
        let query = self.input.trim().to_lowercase();
        if query.is_empty() {
            return self.code_actions.iter().collect();
        }

        self.code_actions
            .iter()
            .filter(|a| {
                a.title.to_lowercase().contains(&query)
                    || a.kind.as_deref().unwrap_or("").to_lowercase().contains(&query)
            })
            .collect()
    }

    /// Creates a prompt modal for document symbols / code outline.
    pub fn new_document_symbols(symbols: Vec<crate::editor::lsp::LspDocumentSymbol>) -> Self {
        let mut modal = Self::new(PromptKind::DocumentSymbols);
        modal.document_symbols = symbols;
        modal
    }

    /// Returns matching document symbols filtered by input query.
    pub fn matching_document_symbols(&self) -> Vec<&crate::editor::lsp::LspDocumentSymbol> {
        let query = self.input.trim().to_lowercase();
        if query.is_empty() {
            return self.document_symbols.iter().collect();
        }

        self.document_symbols
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&query)
                    || s.detail.to_lowercase().contains(&query)
                    || s.kind.badge_str().to_lowercase().contains(&query)
            })
            .collect()
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
        } else if self.kind == PromptKind::FileFinder {
            self.update_file_finder();
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
        } else if self.kind == PromptKind::FileFinder {
            self.update_file_finder();
        }
    }

    pub fn delete(&mut self) {
        if self.kind == PromptKind::Replace && self.active_field == 1 {
            if self.replace_cursor_col >= self.replace_input.chars().count() {
                return;
            }
            let delete_idx = self.replace_cursor_col;
            self.replace_input = self
                .replace_input
                .chars()
                .enumerate()
                .filter(|(i, _)| *i != delete_idx)
                .map(|(_, c)| c)
                .collect();
            return;
        }

        if self.cursor_col >= self.input.chars().count() {
            return;
        }

        let delete_idx = self.cursor_col;
        self.input = self
            .input
            .chars()
            .enumerate()
            .filter(|(i, _)| *i != delete_idx)
            .map(|(_, c)| c)
            .collect();
        self.selected_idx = 0;
        if self.kind == PromptKind::GlobalSearch {
            self.update_global_search();
        } else if self.kind == PromptKind::FileFinder {
            self.update_file_finder();
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

    pub fn is_command_mode(&self) -> bool {
        self.kind == PromptKind::FileFinder && self.input.starts_with('>')
    }

    pub fn update_file_finder(&mut self) {
        if self.is_command_mode() {
            self.selected_idx = 0;
            self.scroll_offset = 0;
            return;
        }
        self.file_finder_results = crate::core::FileFinder::filter_and_rank(&self.project_files, &self.input, 40);
        self.selected_idx = 0;
        self.scroll_offset = 0;
    }

    pub fn select_next_file_result(&mut self, max_visible: usize) {
        let count = if self.is_command_mode() {
            self.matching_commands().len()
        } else {
            self.file_finder_results.len()
        };
        if count > 0 && self.selected_idx + 1 < count {
            self.selected_idx += 1;
            if self.selected_idx >= self.scroll_offset + max_visible {
                self.scroll_offset = self.selected_idx + 1 - max_visible;
            }
        }
    }

    pub fn select_prev_file_result(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
            if self.selected_idx < self.scroll_offset {
                self.scroll_offset = self.selected_idx;
            }
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
        } else if self.kind == PromptKind::RecentFiles {
            let count = self.matching_recent_files().len();
            format!("Archivos Recientes: {} │ ({} archivos) [↑/↓: Seleccionar] [Enter: Abrir] [Esc: Cancelar]", self.input, count)
        } else if self.kind == PromptKind::RecentProjects {
            let count = self.matching_projects().len();
            format!("Proyectos Recientes: {} │ ({} proyectos) [↑/↓: Seleccionar] [Enter: Abrir] [Supr: Olvidar] [Esc: Cancelar]", self.input, count)
        } else if self.kind == PromptKind::FileFinder {
            if self.is_command_mode() {
                let count = self.matching_commands().len();
                format!("Comando: {} │ ({} comandos) [↑/↓: Seleccionar] [Enter: Ejecutar] [Esc: Cancelar]", self.input, count)
            } else {
                let count = self.file_finder_results.len();
                format!("Ir a Archivo: {} │ ({} archivos) [↑/↓: Seleccionar] [Enter: Abrir] [>: Comandos] [Esc: Cancelar]", self.input, count)
            }
        } else {
            format!("{}{}", self.kind.label(), self.input)
        }
    }

    pub fn matching_commands(&self) -> Vec<CommandItem> {
        if self.kind != PromptKind::CommandPalette && !self.is_command_mode() {
            return Vec::new();
        }
        let raw_query = if self.is_command_mode() {
            self.input.strip_prefix('>').unwrap_or(&self.input)
        } else {
            &self.input
        };
        let term = raw_query.trim().to_lowercase();
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
