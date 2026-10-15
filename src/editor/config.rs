//! Persistent Editor Configuration System (`config.ini`).
//! Pure standard-library Rust with zero third-party dependencies.

use std::fs;
use std::path::PathBuf;

/// Theme identifier for menus, actions, and config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTheme {
    Dark,
    Monokai,
    SolarizedDark,
    Light,
}

impl EditorTheme {
    pub const fn as_str(&self) -> &'static str {
        match self {
            EditorTheme::Dark => "dark",
            EditorTheme::Monokai => "monokai",
            EditorTheme::SolarizedDark => "solarized",
            EditorTheme::Light => "light",
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().trim() {
            "monokai" => EditorTheme::Monokai,
            "solarized" | "solarized_dark" | "solarized-dark" => EditorTheme::SolarizedDark,
            "light" | "github_light" | "github-light" | "claro" => EditorTheme::Light,
            _ => EditorTheme::Dark,
        }
    }
}

/// Editor core editing options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorSettings {
    pub tab_size: usize,
    pub auto_indent: bool,
    pub auto_close_brackets: bool,
    pub auto_surround: bool,
    pub show_line_numbers: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            tab_size: 4,
            auto_indent: true,
            auto_close_brackets: true,
            auto_surround: true,
            show_line_numbers: true,
        }
    }
}

/// Theme visual style settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeSettings {
    pub name: String,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            name: "dark".to_string(),
        }
    }
}

/// Native GUI desktop window options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuiSettings {
    pub window_width: usize,
    pub window_height: usize,
    pub menu_bar: bool,
    pub status_bar: bool,
    pub scrollbar: bool,
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            window_width: 1024,
            window_height: 640,
            menu_bar: true,
            status_bar: true,
            scrollbar: true,
        }
    }
}

/// Master configuration model for Rustext.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorConfig {
    pub editor: EditorSettings,
    pub theme: ThemeSettings,
    pub gui: GuiSettings,
    pub config_path: Option<PathBuf>,
}

impl EditorConfig {
    /// Formatted default `config.ini` content with informative comments.
    pub fn default_ini_content() -> &'static str {
        r#"; ====================================================================
; Archivo de Configuración de Rustext (config.ini)
; Editor de texto y código puro en Rust sin dependencias externas.
; ====================================================================

[editor]
; Espacios por cada nivel de tabulación (ej. 2, 4, 8)
tab_size = 4

; Auto-indentación inteligente al presionar Enter
auto_indent = true

; Cierre automático de delimitadores: (), [], {}, "", '', ``
auto_close_brackets = true

; Rodear texto seleccionado al escribir delimitadores
auto_surround = true

; Mostrar u ocultar columna de números de línea
show_line_numbers = true

[theme]
; Tema de interfaz gráfica y terminal:
; Opciones disponibles: dark (por defecto), monokai, solarized, light
name = dark

[gui]
; Ancho y alto inicial de la ventana nativa de escritorio (píxeles)
window_width = 1024
window_height = 640

; Mostrar u ocultar componentes de la interfaz
menu_bar = true
status_bar = true
scrollbar = true
"#
    }

    /// Serializes current configuration to INI format.
    pub fn to_ini_string(&self) -> String {
        format!(
            r#"; Archivo de Configuración de Rustext (config.ini)

[editor]
tab_size = {}
auto_indent = {}
auto_close_brackets = {}
auto_surround = {}
show_line_numbers = {}

[theme]
name = {}

[gui]
window_width = {}
window_height = {}
menu_bar = {}
status_bar = {}
scrollbar = {}
"#,
            self.editor.tab_size,
            self.editor.auto_indent,
            self.editor.auto_close_brackets,
            self.editor.auto_surround,
            self.editor.show_line_numbers,
            self.theme.name,
            self.gui.window_width,
            self.gui.window_height,
            self.gui.menu_bar,
            self.gui.status_bar,
            self.gui.scrollbar,
        )
    }

    /// Parses an INI string into an `EditorConfig` instance.
    pub fn parse(content: &str) -> Self {
        let mut config = Self::default();
        let mut current_section = String::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current_section = trimmed[1..trimmed.len() - 1].trim().to_lowercase();
                continue;
            }

            if let Some((raw_key, raw_val)) = trimmed.split_once('=') {
                let key = raw_key.trim().to_lowercase();
                let val_part = raw_val.split(&[';', '#'][..]).next().unwrap_or("").trim();
                let val_unquoted = if (val_part.starts_with('"') && val_part.ends_with('"'))
                    || (val_part.starts_with('\'') && val_part.ends_with('\''))
                {
                    if val_part.len() >= 2 {
                        &val_part[1..val_part.len() - 1]
                    } else {
                        val_part
                    }
                } else {
                    val_part
                };

                let val_lower = val_unquoted.to_lowercase();
                let as_bool = match val_lower.as_str() {
                    "true" | "1" | "yes" | "on" | "si" | "sí" => Some(true),
                    "false" | "0" | "no" | "off" => Some(false),
                    _ => None,
                };

                match current_section.as_str() {
                    "editor" => match key.as_str() {
                        "tab_size" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                config.editor.tab_size = n.clamp(1, 16);
                            }
                        }
                        "auto_indent" => {
                            if let Some(b) = as_bool {
                                config.editor.auto_indent = b;
                            }
                        }
                        "auto_close_brackets" => {
                            if let Some(b) = as_bool {
                                config.editor.auto_close_brackets = b;
                            }
                        }
                        "auto_surround" => {
                            if let Some(b) = as_bool {
                                config.editor.auto_surround = b;
                            }
                        }
                        "show_line_numbers" => {
                            if let Some(b) = as_bool {
                                config.editor.show_line_numbers = b;
                            }
                        }
                        _ => {}
                    },
                    "theme" => match key.as_str() {
                        "name" => {
                            if !val_unquoted.is_empty() {
                                config.theme.name = val_unquoted.to_string();
                            }
                        }
                        _ => {}
                    },
                    "gui" => match key.as_str() {
                        "window_width" => {
                            if let Ok(w) = val_unquoted.parse::<usize>() {
                                config.gui.window_width = w.clamp(320, 7680);
                            }
                        }
                        "window_height" => {
                            if let Ok(h) = val_unquoted.parse::<usize>() {
                                config.gui.window_height = h.clamp(200, 4320);
                            }
                        }
                        "menu_bar" => {
                            if let Some(b) = as_bool {
                                config.gui.menu_bar = b;
                            }
                        }
                        "status_bar" => {
                            if let Some(b) = as_bool {
                                config.gui.status_bar = b;
                            }
                        }
                        "scrollbar" => {
                            if let Some(b) = as_bool {
                                config.gui.scrollbar = b;
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        config
    }

    /// Resolves the canonical configuration file path.
    /// Prefers local `./.rustext/config.ini` if present, otherwise `~/.rustext/config.ini`.
    pub fn config_file_path() -> PathBuf {
        let local = PathBuf::from(".rustext").join("config.ini");
        if local.exists() {
            return local;
        }

        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from);

        if let Some(h) = home {
            return h.join(".rustext").join("config.ini");
        }

        local
    }

    /// Loads the configuration from disk, creating the default file if it doesn't exist.
    pub fn load() -> Self {
        let path = Self::config_file_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                let mut cfg = Self::parse(&content);
                cfg.config_path = Some(path);
                return cfg;
            }
        }

        // Write default configuration if possible
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, Self::default_ini_content());

        let mut cfg = Self::default();
        cfg.config_path = Some(path);
        cfg
    }

    /// Persists the current configuration to disk.
    pub fn save(&self) -> std::io::Result<()> {
        let path = self.config_path.clone().unwrap_or_else(Self::config_file_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, self.to_ini_string())
    }
}
