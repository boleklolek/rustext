//! Persistent Editor Configuration System (`config.ini`).
//! Pure standard-library Rust with zero third-party dependencies.

use std::fs;
use std::path::PathBuf;

/// Theme identifier for menus, actions, and config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTheme {
    Dark,
    TokyoNight,
    Catppuccin,
    OneDark,
    Monokai,
    SolarizedDark,
    Light,
}

impl EditorTheme {
    pub const fn as_str(&self) -> &'static str {
        match self {
            EditorTheme::Dark => "dark",
            EditorTheme::TokyoNight => "tokyo_night",
            EditorTheme::Catppuccin => "catppuccin",
            EditorTheme::OneDark => "onedark",
            EditorTheme::Monokai => "monokai",
            EditorTheme::SolarizedDark => "solarized",
            EditorTheme::Light => "light",
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().trim() {
            "tokyo" | "tokyonight" | "tokyo_night" | "tokyo-night" => EditorTheme::TokyoNight,
            "catppuccin" | "catppuccin_mocha" | "catppuccin-mocha" | "mocha" => EditorTheme::Catppuccin,
            "onedark" | "one_dark" | "one-dark" | "atom" => EditorTheme::OneDark,
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
    pub format_on_save: bool,
    pub check_on_save: bool,
    pub autocomplete: bool,
    pub autocomplete_min_chars: usize,
    pub restore_session: bool,
    pub auto_save_session: bool,
    pub hot_exit: bool,
    pub max_recent_files: usize,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            tab_size: 4,
            auto_indent: true,
            auto_close_brackets: true,
            auto_surround: true,
            show_line_numbers: true,
            format_on_save: false,
            check_on_save: true,
            autocomplete: true,
            autocomplete_min_chars: 2,
            restore_session: true,
            auto_save_session: true,
            hot_exit: false,
            max_recent_files: 20,
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
#[derive(Debug, Clone, PartialEq)]
pub struct GuiSettings {
    pub window_width: usize,
    pub window_height: usize,
    pub menu_bar: bool,
    pub status_bar: bool,
    pub scrollbar: bool,
    pub font_scale: usize,
    pub scale_factor: f64,
    pub font_antialiasing: bool,
    pub gamma_correction: bool,
    pub line_spacing: usize,
    pub rounded_corners: bool,
    pub shadows: bool,
    pub cursor_style: String,
    pub cursor_blink: bool,
    pub sidebar: bool,
    pub sidebar_width: usize,
    pub minimap: bool,
    pub minimap_width: usize,
    pub git_gutter: bool,
    pub terminal_height: usize,
    pub terminal_shell: String,
    pub font_ligatures: bool,
    pub font_hinting: String,
    pub font_family: String,
    pub font_size: usize,
    pub font_weight: String,
    pub smooth_shadows: bool,
    pub animation_enabled: bool,
    pub edge_perception: bool,
    pub subpixel_antialiasing: bool,
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            window_width: 1024,
            window_height: 640,
            menu_bar: true,
            status_bar: true,
            scrollbar: true,
            font_scale: 1,
            scale_factor: 0.0,
            font_antialiasing: true,
            gamma_correction: true,
            line_spacing: 3,
            rounded_corners: true,
            shadows: true,
            cursor_style: "bar".to_string(),
            cursor_blink: true,
            sidebar: true,
            sidebar_width: 220,
            minimap: true,
            minimap_width: 100,
            git_gutter: true,
            terminal_height: 180,
            terminal_shell: String::new(),
            font_ligatures: true,
            font_hinting: "medium".to_string(),
            font_family: "standard".to_string(),
            font_size: 12,
            font_weight: "normal".to_string(),
            smooth_shadows: true,
            animation_enabled: true,
            edge_perception: true,
            subpixel_antialiasing: true,
        }
    }
}

/// Master configuration model for Rustext.
#[derive(Debug, Clone, PartialEq, Default)]
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

; Auto-formatear código al guardar (rustfmt, black, clang-format, prettier)
format_on_save = false

; Comprobación de errores/linter al guardar archivo (cargo check, py_compile, gcc)
check_on_save = true

; Autocompletado inteligente con menú emergente flotante
autocomplete = true

; Caracteres mínimos para activar autocompletado automáticamente
autocomplete_min_chars = 2

; Restaurar sesión anterior al iniciar (pestañas abiertas, cursores, split)
restore_session = true

; Guardar sesión automáticamente al salir o cambiar de pestaña
auto_save_session = true

; Preservar borradores de archivos modificados sin guardar (Hot Exit)
hot_exit = false

; Número máximo de archivos recientes recordados
max_recent_files = 20

[theme]
; Tema de interfaz gráfica y terminal:
; Opciones disponibles: dark (por defecto), tokyo_night, catppuccin, onedark, monokai, solarized, light
name = dark

[gui]
; Ancho y alto inicial de la ventana nativa de escritorio (píxeles)
window_width = 1024
window_height = 640

; Mostrar u ocultar componentes de la interfaz
menu_bar = true
status_bar = true
scrollbar = true

; Factor de escala HiDPI (0.0 = detección automática según monitor, o forzar 1.0, 1.5, 2.0)
scale_factor = 0.0

; Escala de fuente para pantallas HiDPI / 2K / 4K (1 = normal, 2 = 2x)
font_scale = 1

; Anti-aliasing subpixel en fuentes tipográficas (true = texto suave, false = bitmap directo)
font_antialiasing = true

; Corrección gamma en composición de texto (true = luz lineal sRGB física exacta, false = blending ingenuo)
gamma_correction = true

; Espaciado vertical entre líneas (píxeles extra para que el código respire)
line_spacing = 3

; Esquinas redondeadas modernas en diálogos, botones y pestañas
rounded_corners = true

; Sombras suaves translúcidas bajo ventanas flotantes
shadows = true

; Estilo del cursor: bar (barra moderna), block (bloque clásico), underline
cursor_style = bar

; Parpadeo suave del cursor
cursor_blink = true

; Barra lateral del explorador de archivos del proyecto (true = visible, false = oculta)
sidebar = true

; Ancho en píxeles de la barra lateral (por defecto 220)
sidebar_width = 220

; Minimapa gráfico de código a la derecha (true = visible, false = oculto)
minimap = true

; Ancho en píxeles del minimapa gráfico (por defecto 100)
minimap_width = 100

; Altura inicial en píxeles del panel de terminal integrado (por defecto 180)
terminal_height = 180

; Shell personalizada para el terminal integrado (vacío para autodetección)
terminal_shell = 

; Familia de fuente tipográfica: standard, terminus, modern, typewriter, retro
font_family = standard

; Tamaño de fuente en puntos (10, 11, 12, 14, 16, 18, 20, 24)
font_size = 12

; Grosor de trazo tipográfico: normal, semibold, bold
font_weight = normal

; Renderizado subpíxel LCD RGB ClearType (true = 3x resolución horizontal suave, false = escala de grises)
subpixel_antialiasing = true
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
format_on_save = {}
check_on_save = {}
autocomplete = {}
autocomplete_min_chars = {}
restore_session = {}
auto_save_session = {}
hot_exit = {}
max_recent_files = {}

[theme]
name = {}

[gui]
window_width = {}
window_height = {}
menu_bar = {}
status_bar = {}
scrollbar = {}
scale_factor = {:.1}
font_scale = {}
font_antialiasing = {}
gamma_correction = {}
line_spacing = {}
rounded_corners = {}
shadows = {}
cursor_style = {}
cursor_blink = {}
sidebar = {}
sidebar_width = {}
minimap = {}
minimap_width = {}
git_gutter = {}
terminal_height = {}
terminal_shell = {}
font_ligatures = {}
font_hinting = {}
font_family = {}
font_size = {}
font_weight = {}
smooth_shadows = {}
animation_enabled = {}
edge_perception = {}
subpixel_antialiasing = {}
"#,
            self.editor.tab_size,
            self.editor.auto_indent,
            self.editor.auto_close_brackets,
            self.editor.auto_surround,
            self.editor.show_line_numbers,
            self.editor.format_on_save,
            self.editor.check_on_save,
            self.editor.autocomplete,
            self.editor.autocomplete_min_chars,
            self.editor.restore_session,
            self.editor.auto_save_session,
            self.editor.hot_exit,
            self.editor.max_recent_files,
            self.theme.name,
            self.gui.window_width,
            self.gui.window_height,
            self.gui.menu_bar,
            self.gui.status_bar,
            self.gui.scrollbar,
            self.gui.scale_factor,
            self.gui.font_scale,
            self.gui.font_antialiasing,
            self.gui.gamma_correction,
            self.gui.line_spacing,
            self.gui.rounded_corners,
            self.gui.shadows,
            self.gui.cursor_style,
            self.gui.cursor_blink,
            self.gui.sidebar,
            self.gui.sidebar_width,
            self.gui.minimap,
            self.gui.minimap_width,
            self.gui.git_gutter,
            self.gui.terminal_height,
            self.gui.terminal_shell,
            self.gui.font_ligatures,
            self.gui.font_hinting,
            self.gui.font_family,
            self.gui.font_size,
            self.gui.font_weight,
            self.gui.smooth_shadows,
            self.gui.animation_enabled,
            self.gui.edge_perception,
            self.gui.subpixel_antialiasing,
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
                        "format_on_save" => {
                            if let Some(b) = as_bool {
                                config.editor.format_on_save = b;
                            }
                        }
                        "check_on_save" => {
                            if let Some(b) = as_bool {
                                config.editor.check_on_save = b;
                            }
                        }
                        "autocomplete" => {
                            if let Some(b) = as_bool {
                                config.editor.autocomplete = b;
                            }
                        }
                        "autocomplete_min_chars" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                config.editor.autocomplete_min_chars = n.clamp(1, 10);
                            }
                        }
                        "restore_session" => {
                            if let Some(b) = as_bool {
                                config.editor.restore_session = b;
                            }
                        }
                        "auto_save_session" => {
                            if let Some(b) = as_bool {
                                config.editor.auto_save_session = b;
                            }
                        }
                        "hot_exit" => {
                            if let Some(b) = as_bool {
                                config.editor.hot_exit = b;
                            }
                        }
                        "max_recent_files" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                config.editor.max_recent_files = n.clamp(1, 100);
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
                        "scale_factor" => {
                            if let Ok(f) = val_unquoted.parse::<f64>() {
                                if f >= 0.0 && f <= 5.0 {
                                    config.gui.scale_factor = f;
                                }
                            }
                        }
                        "font_scale" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                config.gui.font_scale = n.clamp(1, 3);
                            }
                        }
                        "font_antialiasing" => {
                            if let Some(b) = as_bool {
                                config.gui.font_antialiasing = b;
                            }
                        }
                        "gamma_correction" => {
                            if let Some(b) = as_bool {
                                config.gui.gamma_correction = b;
                            }
                        }
                        "line_spacing" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                config.gui.line_spacing = n.min(24);
                            }
                        }
                        "rounded_corners" => {
                            if let Some(b) = as_bool {
                                config.gui.rounded_corners = b;
                            }
                        }
                        "shadows" => {
                            if let Some(b) = as_bool {
                                config.gui.shadows = b;
                            }
                        }
                        "cursor_style" => {
                            if !val_unquoted.is_empty() {
                                config.gui.cursor_style = val_unquoted.to_string();
                            }
                        }
                        "cursor_blink" => {
                            if let Some(b) = as_bool {
                                config.gui.cursor_blink = b;
                            }
                        }
                        "sidebar" => {
                            if let Some(b) = as_bool {
                                config.gui.sidebar = b;
                            }
                        }
                        "sidebar_width" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                config.gui.sidebar_width = n.clamp(140, 600);
                            }
                        }
                        "minimap" => {
                            if let Some(b) = as_bool {
                                config.gui.minimap = b;
                            }
                        }
                        "minimap_width" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                config.gui.minimap_width = n.clamp(40, 300);
                            }
                        }
                        "git_gutter" => {
                            if let Some(b) = as_bool {
                                config.gui.git_gutter = b;
                            }
                        }
                        "terminal_height" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                config.gui.terminal_height = n.clamp(60, 1000);
                            }
                        }
                        "terminal_shell" => {
                            config.gui.terminal_shell = val_unquoted.to_string();
                        }
                        "font_ligatures" => {
                            if let Some(b) = as_bool {
                                config.gui.font_ligatures = b;
                            }
                        }
                        "font_hinting" => {
                            if !val_unquoted.is_empty() {
                                config.gui.font_hinting = val_unquoted.to_string();
                            }
                        }
                        "font_family" => {
                            if !val_unquoted.is_empty() {
                                config.gui.font_family = val_unquoted.to_string();
                            }
                        }
                        "font_size" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                config.gui.font_size = n.clamp(8, 48);
                            }
                        }
                        "font_weight" => {
                            if !val_unquoted.is_empty() {
                                config.gui.font_weight = val_unquoted.to_string();
                            }
                        }
                        "smooth_shadows" => {
                            if let Some(b) = as_bool {
                                config.gui.smooth_shadows = b;
                            }
                        }
                        "animation_enabled" => {
                            if let Some(b) = as_bool {
                                config.gui.animation_enabled = b;
                            }
                        }
                        "edge_perception" => {
                            if let Some(b) = as_bool {
                                config.gui.edge_perception = b;
                            }
                        }
                        "subpixel_antialiasing" => {
                            if let Some(b) = as_bool {
                                config.gui.subpixel_antialiasing = b;
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
