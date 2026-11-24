//! Dialog definitions and confirmations for unsaved files and editor modals.

use crate::ui::prompt::PromptModal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsavedConfirmAction {
    CloseTab(usize),
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsavedConfirmDialog {
    pub action: UnsavedConfirmAction,
    pub file_name: String,
    pub dirty_count: usize,
    pub selected_button: usize, // 0 = Guardar, 1 = Descartar, 2 = Cancelar
}

impl UnsavedConfirmDialog {
    pub fn for_close_tab(tab_idx: usize, file_name: String) -> Self {
        Self {
            action: UnsavedConfirmAction::CloseTab(tab_idx),
            file_name,
            dirty_count: 1,
            selected_button: 0,
        }
    }

    pub fn for_quit(dirty_count: usize, file_name: String) -> Self {
        Self {
            action: UnsavedConfirmAction::Quit,
            file_name,
            dirty_count,
            selected_button: 0,
        }
    }

    pub fn title(&self) -> &'static str {
        match self.action {
            UnsavedConfirmAction::CloseTab(_) => "Cerrar Pestaña con Cambios",
            UnsavedConfirmAction::Quit => "Salir de Rustext - Cambios sin Guardar",
        }
    }

    pub fn message_lines(&self) -> (String, String) {
        match self.action {
            UnsavedConfirmAction::CloseTab(_) => (
                format!("El archivo '{}' tiene cambios sin guardar.", self.file_name),
                "¿Desea guardar los cambios antes de cerrar la pestaña?".to_string(),
            ),
            UnsavedConfirmAction::Quit => {
                if self.dirty_count > 1 {
                    (
                        format!("Hay {} archivos con cambios sin guardar en la sesión.", self.dirty_count),
                        "¿Desea guardar todos los cambios antes de salir?".to_string(),
                    )
                } else {
                    (
                        format!("El archivo '{}' tiene cambios sin guardar.", self.file_name),
                        "¿Desea guardar los cambios antes de salir de Rustext?".to_string(),
                    )
                }
            }
        }
    }

    pub fn button_labels(&self) -> (&'static str, &'static str, &'static str) {
        match self.action {
            UnsavedConfirmAction::CloseTab(_) => ("Guardar", "No guardar", "Cancelar"),
            UnsavedConfirmAction::Quit => {
                if self.dirty_count > 1 {
                    ("Guardar Todo", "Descartar Todo", "Cancelar")
                } else {
                    ("Guardar", "No guardar", "Cancelar")
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorMode {
    Normal,
    ModalPrompt(PromptModal),
    AboutDialog,
    ExtensionsManager { selected_idx: usize, scroll_offset: usize },
    UnsavedConfirm(UnsavedConfirmDialog),
    FontSelector(crate::editor::font_selector::FontSelectorDialog),
}
