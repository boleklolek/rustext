//! Native desktop menu bar and dropdown system.
//! Zero third-party dependencies, purely standard library Rust.

use crate::editor::keymap::EditorAction;
use crate::editor::config::EditorTheme;
use crate::gfx::FontFamily;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    pub label: &'static str,
    pub shortcut: &'static str,
    pub action: Option<EditorAction>,
    pub is_separator: bool,
}

impl MenuItem {
    pub const fn action(label: &'static str, shortcut: &'static str, action: EditorAction) -> Self {
        Self {
            label,
            shortcut,
            action: Some(action),
            is_separator: false,
        }
    }

    pub const fn separator() -> Self {
        Self {
            label: "",
            shortcut: "",
            action: None,
            is_separator: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuCategory {
    pub name: &'static str,
    pub items: Vec<MenuItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuBar {
    pub categories: Vec<MenuCategory>,
    pub active_category: Option<usize>,
    pub hovered_item: Option<usize>,
    pub height: usize,
    pub scale_factor: f64,
    pub window_width: usize,
}

impl MenuBar {
    pub fn new() -> Self {
        let categories = vec![
            MenuCategory {
                name: "Archivo",
                items: vec![
                    MenuItem::action("Nueva Pestaña", "Ctrl+T", EditorAction::NewTab),
                    MenuItem::action("Abrir Archivo...", "Ctrl+O", EditorAction::OpenOpenFilePrompt),
                    MenuItem::action("Abrir Archivo Reciente...", "Ctrl+R", EditorAction::OpenRecentFilesPrompt),
                    MenuItem::action("Guardar", "Ctrl+S", EditorAction::Save),
                    MenuItem::action("Guardar Como...", "", EditorAction::OpenSaveAsPrompt),
                    MenuItem::separator(),
                    MenuItem::action("Cerrar Pestaña", "Ctrl+W", EditorAction::CloseTab),
                    MenuItem::separator(),
                    MenuItem::action("Salir", "Ctrl+Q", EditorAction::Quit),
                ],
            },
            MenuCategory {
                name: "Editar",
                items: vec![
                    MenuItem::action("Deshacer", "Ctrl+Z", EditorAction::Undo),
                    MenuItem::action("Rehacer", "Ctrl+Y", EditorAction::Redo),
                    MenuItem::separator(),
                    MenuItem::action("Cortar", "Ctrl+X", EditorAction::Cut),
                    MenuItem::action("Copiar", "Ctrl+C", EditorAction::Copy),
                    MenuItem::action("Pegar", "Ctrl+V", EditorAction::Paste),
                    MenuItem::separator(),
                    MenuItem::action("Buscar...", "Ctrl+F", EditorAction::OpenSearchPrompt),
                    MenuItem::action("Reemplazar...", "Ctrl+H", EditorAction::OpenReplacePrompt),
                    MenuItem::action("Buscar en el Proyecto...", "Ctrl+Shift+F", EditorAction::OpenGlobalSearchPrompt),
                    MenuItem::separator(),
                    MenuItem::action("Insertar Snippet: Función", "Tab", EditorAction::InsertSnippet("fn".to_string())),
                ],
            },
            MenuCategory {
                name: "Selección",
                items: vec![
                    MenuItem::action("Añadir Cursor Arriba", "Ctrl+K", EditorAction::AddCursorAbove),
                    MenuItem::action("Añadir Cursor Abajo", "Ctrl+J", EditorAction::AddCursorBelow),
                    MenuItem::action("Siguiente Ocurrencia", "Ctrl+D", EditorAction::SelectNextOccurrence),
                    MenuItem::separator(),
                    MenuItem::action("Cursor Único", "Esc", EditorAction::CollapseCursors),
                ],
            },
            MenuCategory {
                name: "Ver",
                items: vec![
                    MenuItem::action("Explorador Lateral", "Ctrl+B", EditorAction::ToggleSidebar),
                    MenuItem::action("Actualizar Explorador", "", EditorAction::RefreshSidebar),
                    MenuItem::action("Ajuste de Línea (Word Wrap)", "Alt+Z", EditorAction::ToggleWordWrap),
                    MenuItem::action("Plegar/Desplegar Bloque", "Alt+[", EditorAction::ToggleFold),
                    MenuItem::action("Plegar Todo", "Alt+0", EditorAction::FoldAll),
                    MenuItem::action("Desplegar Todo", "Alt+J", EditorAction::UnfoldAll),
                    MenuItem::separator(),
                    MenuItem::action("Minimapa", "Ctrl+M", EditorAction::ToggleMinimap),
                    MenuItem::action("Terminal Integrado", "Ctrl+`", EditorAction::ToggleTerminal),
                    MenuItem::action("Dividir Verticalmente", "Ctrl+\\", EditorAction::ToggleSplitVertical),
                    MenuItem::action("Dividir Horizontalmente", "Ctrl+Shift+\\", EditorAction::ToggleSplitHorizontal),
                    MenuItem::action("Cambiar Foco de Panel", "Alt+\\", EditorAction::SwitchSplitPane),
                    MenuItem::action("Cerrar División", "", EditorAction::CloseSplit),
                    MenuItem::separator(),
                    MenuItem::action("Pestaña Siguiente", "PageDown", EditorAction::NextTab),
                    MenuItem::action("Pestaña Anterior", "PageUp", EditorAction::PrevTab),
                    MenuItem::separator(),
                    MenuItem::action("Paleta de Comandos...", "Ctrl+P", EditorAction::OpenCommandPalette),
                ],
            },
            MenuCategory {
                name: "Preferencias",
                items: vec![
                    MenuItem::action("Abrir Configuración...", "", EditorAction::OpenConfig),
                    MenuItem::action("Recargar Configuración", "", EditorAction::ReloadConfig),
                    MenuItem::separator(),
                    MenuItem::action("Selector de Fuentes...", "Ctrl+Shift+T", EditorAction::OpenFontSelector),
                    MenuItem::action("Aumentar Tamaño (+)", "Ctrl++", EditorAction::IncreaseFontSize),
                    MenuItem::action("Disminuir Tamaño (-)", "Ctrl+-", EditorAction::DecreaseFontSize),
                    MenuItem::action("Restablecer Tamaño (12pt)", "Ctrl+0", EditorAction::ResetFontSize),
                    MenuItem::separator(),
                    MenuItem::action("Fuente: Standard Mono", "", EditorAction::SetFontFamily(FontFamily::Standard)),
                    MenuItem::action("Fuente: Terminus Console", "", EditorAction::SetFontFamily(FontFamily::Terminus)),
                    MenuItem::action("Fuente: Modern Coding Sans", "", EditorAction::SetFontFamily(FontFamily::Modern)),
                    MenuItem::action("Fuente: Typewriter Slab", "", EditorAction::SetFontFamily(FontFamily::Typewriter)),
                    MenuItem::action("Fuente: Retro Pixel", "", EditorAction::SetFontFamily(FontFamily::Retro)),
                    MenuItem::separator(),
                    MenuItem::action("Tema: Tokyo Night", "", EditorAction::SetTheme(EditorTheme::TokyoNight)),
                    MenuItem::action("Tema: Catppuccin Mocha", "", EditorAction::SetTheme(EditorTheme::Catppuccin)),
                    MenuItem::action("Tema: One Dark", "", EditorAction::SetTheme(EditorTheme::OneDark)),
                    MenuItem::action("Tema: Modern Dark", "", EditorAction::SetTheme(EditorTheme::Dark)),
                    MenuItem::action("Tema: Monokai", "", EditorAction::SetTheme(EditorTheme::Monokai)),
                    MenuItem::action("Tema: Solarized Dark", "", EditorAction::SetTheme(EditorTheme::SolarizedDark)),
                    MenuItem::action("Tema: Claro (GitHub Light)", "", EditorAction::SetTheme(EditorTheme::Light)),
                ],
            },
            MenuCategory {
                name: "Extensiones",
                items: vec![
                    MenuItem::action("Administrador de Extensiones...", "Ctrl+Shift+X", EditorAction::OpenExtensionsManager),
                    MenuItem::action("Recargar Extensiones", "", EditorAction::ReloadExtensions),
                    MenuItem::action("Crear Extensión de Ejemplo", "", EditorAction::CreateSampleExtension),
                ],
            },
            MenuCategory {
                name: "Herramientas",
                items: vec![
                    MenuItem::action("Formatear Documento", "Ctrl+Shift+I", EditorAction::FormatDocument),
                    MenuItem::action("Comprobar Errores (Check / Lint)", "F7", EditorAction::CheckDocument),
                    MenuItem::separator(),
                    MenuItem::action("Siguiente Error", "F8", EditorAction::NextDiagnostic),
                    MenuItem::action("Error Anterior", "Shift+F8", EditorAction::PrevDiagnostic),
                    MenuItem::separator(),
                    MenuItem::action("Actualizar Estado Git", "Ctrl+Shift+G", EditorAction::RefreshGit),
                    MenuItem::action("Alternar Git Gutter", "", EditorAction::ToggleGitGutter),
                ],
            },
            MenuCategory {
                name: "Ayuda",
                items: vec![
                    MenuItem::action("Paleta de Comandos", "Ctrl+P", EditorAction::OpenCommandPalette),
                    MenuItem::separator(),
                    MenuItem::action("Acerca de Rustext", "F1", EditorAction::OpenAboutDialog),
                ],
            },
        ];

        Self {
            categories,
            active_category: None,
            hovered_item: None,
            height: 24,
            scale_factor: 1.0,
            window_width: 0,
        }
    }

    /// Sets the host window width to ensure dropdowns never render off-screen.
    pub fn set_window_width(&mut self, width: usize) {
        self.window_width = width;
    }

    /// Calculates dynamic dropdown width based on contents to prevent label/shortcut truncation.
    pub fn dropdown_width(&self, cat_idx: usize, font_width: usize) -> usize {
        let cat = match self.categories.get(cat_idx) {
            Some(c) => c,
            None => return (260.0 * self.scale_factor).round() as usize,
        };
        let base_min = (260.0 * self.scale_factor).round() as usize;
        let mut max_chars = 0;
        for item in &cat.items {
            if item.is_separator {
                continue;
            }
            let l_len = item.label.chars().count();
            let s_len = item.shortcut.chars().count();
            let total = l_len + if s_len > 0 { s_len + 4 } else { 0 };
            if total > max_chars {
                max_chars = total;
            }
        }
        let calculated = max_chars * font_width + (32.0 * self.scale_factor).round() as usize;
        base_min.max(calculated)
    }

    /// Computes the bounding rectangle (x, y, width, height) of a category's dropdown menu,
    /// clamping `x` so the dropdown never overflows `max_width`.
    pub fn dropdown_rect(&self, cat_idx: usize, font_width: usize, max_width: usize) -> (usize, usize, usize, usize) {
        let cat = match self.categories.get(cat_idx) {
            Some(c) => c,
            None => return (0, self.height, (260.0 * self.scale_factor).round() as usize, 0),
        };
        let drop_w = self.dropdown_width(cat_idx, font_width);
        let (mut drop_x, _) = self.category_bounds(cat_idx, font_width);
        if max_width > 0 && drop_x + drop_w > max_width {
            drop_x = max_width.saturating_sub(drop_w);
        }
        let item_h = (22.0 * self.scale_factor).round() as usize;
        let drop_h = cat.items.len() * item_h + 4;
        let drop_y = self.height;
        (drop_x, drop_y, drop_w, drop_h)
    }

    /// Sets the UI scale factor and updates logical height.
    pub fn set_scale_factor(&mut self, factor: f64) {
        self.scale_factor = factor;
        self.height = (24.0 * factor).round() as usize;
    }

    /// Returns horizontal (start_x, end_x) for a category index.
    pub fn category_bounds(&self, index: usize, font_width: usize) -> (usize, usize) {
        let pad = (16.0 * self.scale_factor).round() as usize;
        let mut cur_x = (8.0 * self.scale_factor).round() as usize;
        for (i, cat) in self.categories.iter().enumerate() {
            let w = cat.name.chars().count() * font_width + pad;
            if i == index {
                return (cur_x, cur_x + w);
            }
            cur_x += w;
        }
        (cur_x, cur_x)
    }

    /// Handles a mouse click at (x, y).
    /// If clicked on menu bar, toggles or opens dropdown.
    /// If clicked inside open dropdown, returns the triggered EditorAction.
    /// If clicked outside, closes dropdown.
    pub fn handle_mouse_click(&mut self, x: usize, y: usize, font_width: usize) -> Option<EditorAction> {
        let pad = (16.0 * self.scale_factor).round() as usize;
        let start_x = (8.0 * self.scale_factor).round() as usize;
        // 1. Click on Menu Bar header
        if y < self.height {
            let mut cur_x = start_x;
            for (idx, cat) in self.categories.iter().enumerate() {
                let w = cat.name.chars().count() * font_width + pad;
                if x >= cur_x && x < cur_x + w {
                    if self.active_category == Some(idx) {
                        self.active_category = None;
                    } else {
                        self.active_category = Some(idx);
                    }
                    self.hovered_item = None;
                    return None;
                }
                cur_x += w;
            }
            self.active_category = None;
            return None;
        }

        // 2. Click inside open dropdown menu
        if let Some(cat_idx) = self.active_category {
            let (drop_x, drop_y, drop_w, drop_h) = self.dropdown_rect(cat_idx, font_width, self.window_width);
            let cat = &self.categories[cat_idx];
            let item_h = (22.0 * self.scale_factor).round() as usize;

            if x >= drop_x && x < drop_x + drop_w && y >= drop_y && y < drop_y + drop_h {
                let items_start_y = drop_y + 2;
                if y >= items_start_y {
                    let item_idx = (y - items_start_y) / item_h;
                    if item_idx < cat.items.len() {
                        let item = &cat.items[item_idx];
                        if !item.is_separator {
                            let action = item.action.clone();
                            self.active_category = None;
                            self.hovered_item = None;
                            return action;
                        }
                    }
                }
                return None;
            }
        }

        // Click outside closes active menu
        if self.active_category.is_some() {
            self.active_category = None;
            self.hovered_item = None;
        }

        None
    }

    /// Updates hover state when mouse moves over menu bar or open dropdown.
    pub fn handle_mouse_move(&mut self, x: usize, y: usize, font_width: usize) {
        if self.active_category.is_none() {
            return;
        }

        let pad = (16.0 * self.scale_factor).round() as usize;
        let start_x = (8.0 * self.scale_factor).round() as usize;

        // Switch category if hovering over another header while a menu is open
        if y < self.height {
            let mut cur_x = start_x;
            for (idx, cat) in self.categories.iter().enumerate() {
                let w = cat.name.chars().count() * font_width + pad;
                if x >= cur_x && x < cur_x + w {
                    self.active_category = Some(idx);
                    self.hovered_item = None;
                    return;
                }
                cur_x += w;
            }
            return;
        }

        // Update hovered item inside open dropdown
        if let Some(cat_idx) = self.active_category {
            let (drop_x, drop_y, drop_w, drop_h) = self.dropdown_rect(cat_idx, font_width, self.window_width);
            let cat = &self.categories[cat_idx];
            let item_h = (22.0 * self.scale_factor).round() as usize;

            if x >= drop_x && x < drop_x + drop_w && y >= drop_y && y < drop_y + drop_h {
                let items_start_y = drop_y + 2;
                if y >= items_start_y {
                    let item_idx = (y - items_start_y) / item_h;
                    if item_idx < cat.items.len() && !cat.items[item_idx].is_separator {
                        self.hovered_item = Some(item_idx);
                        return;
                    }
                }
            }
            self.hovered_item = None;
        }
    }

    pub fn close(&mut self) {
        self.active_category = None;
        self.hovered_item = None;
    }

    pub fn is_open(&self) -> bool {
        self.active_category.is_some()
    }
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}
