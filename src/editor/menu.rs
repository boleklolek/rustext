//! Native desktop menu bar and dropdown system.
//! Zero third-party dependencies, purely standard library Rust.

use crate::editor::keymap::EditorAction;
use crate::editor::config::EditorTheme;

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

#[derive(Debug, Clone)]
pub struct MenuCategory {
    pub name: &'static str,
    pub items: Vec<MenuItem>,
}

#[derive(Debug, Clone)]
pub struct MenuBar {
    pub categories: Vec<MenuCategory>,
    pub active_category: Option<usize>,
    pub hovered_item: Option<usize>,
    pub height: usize,
}

impl MenuBar {
    pub fn new() -> Self {
        let categories = vec![
            MenuCategory {
                name: "Archivo",
                items: vec![
                    MenuItem::action("Nueva Pestaña", "Ctrl+T", EditorAction::NewTab),
                    MenuItem::action("Abrir Archivo...", "Ctrl+O", EditorAction::OpenOpenFilePrompt),
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
                    MenuItem::separator(),
                    MenuItem::action("Minimapa", "Ctrl+M", EditorAction::ToggleMinimap),
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
        }
    }

    /// Returns horizontal (start_x, end_x) for a category index.
    pub fn category_bounds(&self, index: usize, font_width: usize) -> (usize, usize) {
        let mut cur_x = 8;
        for (i, cat) in self.categories.iter().enumerate() {
            let w = cat.name.chars().count() * font_width + 16;
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
        // 1. Click on Menu Bar header
        if y < self.height {
            let mut cur_x = 8;
            for (idx, cat) in self.categories.iter().enumerate() {
                let w = cat.name.chars().count() * font_width + 16;
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
            let (drop_x, _) = self.category_bounds(cat_idx, font_width);
            let drop_w = 260;
            let cat = &self.categories[cat_idx];
            let item_h = 22;
            let drop_h = cat.items.len() * item_h + 4;

            let drop_y = self.height;
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

        // Switch category if hovering over another header while a menu is open
        if y < self.height {
            let mut cur_x = 8;
            for (idx, cat) in self.categories.iter().enumerate() {
                let w = cat.name.chars().count() * font_width + 16;
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
            let (drop_x, _) = self.category_bounds(cat_idx, font_width);
            let drop_w = 260;
            let cat = &self.categories[cat_idx];
            let item_h = 22;
            let drop_h = cat.items.len() * item_h + 4;
            let drop_y = self.height;

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
