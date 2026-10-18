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
    pub word_wrap: bool,
    pub code_folding: bool,
    pub indent_guides: bool,
    pub highlight_active_indent_guide: bool,
    pub highlight_current_line: bool,
    pub rulers: Vec<usize>,
    pub sticky_scroll: bool,
    pub max_sticky_lines: usize,
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
            word_wrap: false,
            code_folding: true,
            indent_guides: true,
            highlight_active_indent_guide: true,
            highlight_current_line: true,
            rulers: vec![80, 120],
            sticky_scroll: true,
            max_sticky_lines: 3,
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
    pub letter_spacing: usize,
    pub font_aspect_ratio: f64,
    pub stem_darkening: bool,
    pub stem_darkening_amount: f64,
    pub ergonomic_scaling: bool,
    pub viewing_distance_mm: f64,
    pub iso_target_arcmin: f64,
    pub sdf_font_rendering: bool,
    pub optical_punctuation_spacing: bool,
    pub use_ttf_font: bool,
    pub ttf_font_path: String,
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
            letter_spacing: 0,
            font_aspect_ratio: 0.50,
            stem_darkening: true,
            stem_darkening_amount: 1.0,
            ergonomic_scaling: true,
            viewing_distance_mm: 0.0,
            iso_target_arcmin: 22.0,
            sdf_font_rendering: true,
            optical_punctuation_spacing: true,
            use_ttf_font: false,
            ttf_font_path: String::new(),
        }
    }
}

/// Language Server Protocol (LSP) settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspSettings {
    pub enabled: bool,
    pub auto_start: bool,
    pub rust_server: String,
    pub python_server: String,
    pub c_server: String,
    pub typescript_server: String,
    pub go_server: String,
    pub html_css_server: String,
    pub json_server: String,
    pub markdown_server: String,
    pub zig_server: String,
    pub inlay_hints: bool,
}

impl Default for LspSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_start: true,
            rust_server: "rust-analyzer".to_string(),
            python_server: "pylsp".to_string(),
            c_server: "clangd".to_string(),
            typescript_server: "typescript-language-server --stdio".to_string(),
            go_server: "gopls".to_string(),
            html_css_server: "vscode-html-language-server --stdio".to_string(),
            json_server: "vscode-json-language-server --stdio".to_string(),
            markdown_server: "marksman server".to_string(),
            zig_server: "zls".to_string(),
            inlay_hints: true,
        }
    }
}

/// Master configuration model for Rustext.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EditorConfig {
    pub editor: EditorSettings,
    pub theme: ThemeSettings,
    pub gui: GuiSettings,
    pub lsp: LspSettings,
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

; Ajuste de línea visual suave (Soft Wrap / Word Wrap) sin alterar el buffer de texto
word_wrap = false

; Plegado de código sintáctico e indentación con indicadores en el gutter (Code Folding)
code_folding = true

; Guías de indentación visuales verticales (Indent Guides)
indent_guides = true

; Resaltar la guía de indentación del bloque activo bajo el cursor
highlight_active_indent_guide = true

; Resaltar la línea activa donde se encuentra el cursor (Current Line Highlight)
highlight_current_line = true

; Columnas con líneas guía de límite de código (separadas por comas, e.g. 80, 120)
rulers = 80, 120

; Fijar encabezados de ámbito/funciones en la parte superior al desplazarse (Sticky Scroll)
sticky_scroll = true

; Número máximo de encabezados de ámbito simultáneos en Sticky Scroll (1 a 10)
max_sticky_lines = 3

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

; Espaciado horizontal entre caracteres (tracking / letter-spacing extra en píxeles)
letter_spacing = 0

; Relación de aspecto ergonómica de la fuente (ancho/alto): 0.50 (clásico), 0.55 (equilibrado), 0.60 (confort), 0.65 (amplio)
font_aspect_ratio = 0.50

; Engrosamiento óptico de trazo en modo oscuro (compensa la irradiación retiniana)
stem_darkening = true

; Nivel de intensidad de engrosamiento óptico (0.0 a 2.0, por defecto 1.0)
stem_darkening_amount = 1.0

; Escala ergonómica ISO 9241-303 (true = escala cuantizada óptima según densidad y distancia de visión)
ergonomic_scaling = true

; Distancia estimada de visualización en milímetros (0.0 = automático según tamaño de pantalla: 480mm laptop, 600mm escritorio)
viewing_distance_mm = 0.0

; Ángulo visual objetivo en minutos de arco según ISO 9241-303 (recomendado: 20.0 a 24.0, por defecto 22.0)
iso_target_arcmin = 22.0

; Renderizado de fuentes mediante Campos de Distancia con Signo (SDF) en alta resolución (true = contornos vectoriales puros)
sdf_font_rendering = true

; Espaciado óptico dinámico de signos de puntuación y delimitadores (true = kerning visual armónico sin romper columnas)
optical_punctuation_spacing = true

; Motor vectorial TrueType (.ttf/.otf) analítico puro en Rust (true = rasterizado vectorial de curvas Bézier exactas)
use_ttf_font = false

; Ruta personalizada a archivo de fuente TrueType .ttf (vacío para usar fuentes del sistema o sintetizada integrada)
ttf_font_path = 

[lsp]
; Habilitar o deshabilitar integración con servidores LSP (Language Server Protocol)
enabled = true

; Iniciar automáticamente el servidor LSP al abrir archivos compatibles (.rs, .py, .c, .cpp)
auto_start = true

; Binario o comando para el servidor LSP de Rust
rust_server = rust-analyzer

; Binario o comando para el servidor LSP de Python
python_server = pylsp

; Binario o comando para el servidor LSP de C / C++
c_server = clangd

; Servidor LSP de TypeScript / JavaScript
typescript_server = typescript-language-server --stdio

; Servidor LSP de Go
go_server = gopls

; Servidor LSP de HTML / CSS
html_css_server = vscode-html-language-server --stdio

; Servidor LSP de JSON
json_server = vscode-json-language-server --stdio

; Servidor LSP de Markdown
markdown_server = marksman server

; Servidor LSP de Zig
zig_server = zls

; Mostrar sugerencias de tipo y parámetros en línea (Inlay Hints)
inlay_hints = true
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
word_wrap = {}
code_folding = {}
indent_guides = {}
highlight_active_indent_guide = {}
highlight_current_line = {}
rulers = {}
sticky_scroll = {}
max_sticky_lines = {}

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
letter_spacing = {}
font_aspect_ratio = {:.2}
stem_darkening = {}
stem_darkening_amount = {:.2}
ergonomic_scaling = {}
viewing_distance_mm = {:.1}
iso_target_arcmin = {:.1}
sdf_font_rendering = {}
optical_punctuation_spacing = {}
use_ttf_font = {}
ttf_font_path = {}

[lsp]
enabled = {}
auto_start = {}
rust_server = {}
python_server = {}
c_server = {}
typescript_server = {}
go_server = {}
html_css_server = {}
json_server = {}
markdown_server = {}
zig_server = {}
inlay_hints = {}
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
            self.editor.word_wrap,
            self.editor.code_folding,
            self.editor.indent_guides,
            self.editor.highlight_active_indent_guide,
            self.editor.highlight_current_line,
            self.editor.rulers.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", "),
            self.editor.sticky_scroll,
            self.editor.max_sticky_lines,
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
            self.gui.letter_spacing,
            self.gui.font_aspect_ratio,
            self.gui.stem_darkening,
            self.gui.stem_darkening_amount,
            self.gui.ergonomic_scaling,
            self.gui.viewing_distance_mm,
            self.gui.iso_target_arcmin,
            self.gui.sdf_font_rendering,
            self.gui.optical_punctuation_spacing,
            self.gui.use_ttf_font,
            self.gui.ttf_font_path,
            self.lsp.enabled,
            self.lsp.auto_start,
            self.lsp.rust_server,
            self.lsp.python_server,
            self.lsp.c_server,
            self.lsp.typescript_server,
            self.lsp.go_server,
            self.lsp.html_css_server,
            self.lsp.json_server,
            self.lsp.markdown_server,
            self.lsp.zig_server,
            self.lsp.inlay_hints,
        )
    }

    /// Parses an INI string into an `EditorConfig` instance.
    pub fn parse(content: &str) -> Self {
        let mut config = Self::default();
        config.apply_ini_str(content);
        config
    }

    /// Applies configuration overrides from an INI string onto `self`.
    pub fn apply_ini_str(&mut self, content: &str) {
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
                                self.editor.tab_size = n.clamp(1, 16);
                            }
                        }
                        "auto_indent" => {
                            if let Some(b) = as_bool {
                                self.editor.auto_indent = b;
                            }
                        }
                        "auto_close_brackets" => {
                            if let Some(b) = as_bool {
                                self.editor.auto_close_brackets = b;
                            }
                        }
                        "auto_surround" => {
                            if let Some(b) = as_bool {
                                self.editor.auto_surround = b;
                            }
                        }
                        "show_line_numbers" => {
                            if let Some(b) = as_bool {
                                self.editor.show_line_numbers = b;
                            }
                        }
                        "format_on_save" => {
                            if let Some(b) = as_bool {
                                self.editor.format_on_save = b;
                            }
                        }
                        "check_on_save" => {
                            if let Some(b) = as_bool {
                                self.editor.check_on_save = b;
                            }
                        }
                        "autocomplete" => {
                            if let Some(b) = as_bool {
                                self.editor.autocomplete = b;
                            }
                        }
                        "autocomplete_min_chars" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                self.editor.autocomplete_min_chars = n.clamp(1, 10);
                            }
                        }
                        "restore_session" => {
                            if let Some(b) = as_bool {
                                self.editor.restore_session = b;
                            }
                        }
                        "auto_save_session" => {
                            if let Some(b) = as_bool {
                                self.editor.auto_save_session = b;
                            }
                        }
                        "hot_exit" => {
                            if let Some(b) = as_bool {
                                self.editor.hot_exit = b;
                            }
                        }
                        "max_recent_files" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                self.editor.max_recent_files = n.clamp(1, 100);
                            }
                        }
                        "word_wrap" => {
                            if let Some(b) = as_bool {
                                self.editor.word_wrap = b;
                            }
                        }
                        "code_folding" => {
                            if let Some(b) = as_bool {
                                self.editor.code_folding = b;
                            }
                        }
                        "indent_guides" => {
                            if let Some(b) = as_bool {
                                self.editor.indent_guides = b;
                            }
                        }
                        "highlight_active_indent_guide" => {
                            if let Some(b) = as_bool {
                                self.editor.highlight_active_indent_guide = b;
                            }
                        }
                        "highlight_current_line" => {
                            if let Some(b) = as_bool {
                                self.editor.highlight_current_line = b;
                            }
                        }
                        "rulers" => {
                            let list: Vec<usize> = val_unquoted
                                .split(',')
                                .filter_map(|s| s.trim().parse::<usize>().ok())
                                .collect();
                            self.editor.rulers = list;
                        }
                        "sticky_scroll" => {
                            if let Some(b) = as_bool {
                                self.editor.sticky_scroll = b;
                            }
                        }
                        "max_sticky_lines" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                self.editor.max_sticky_lines = n.clamp(1, 10);
                            }
                        }
                        _ => {}
                    },
                    "theme" => match key.as_str() {
                        "name" => {
                            if !val_unquoted.is_empty() {
                                self.theme.name = val_unquoted.to_string();
                            }
                        }
                        _ => {}
                    },
                    "gui" => match key.as_str() {
                        "window_width" => {
                            if let Ok(w) = val_unquoted.parse::<usize>() {
                                self.gui.window_width = w.clamp(320, 7680);
                            }
                        }
                        "window_height" => {
                            if let Ok(h) = val_unquoted.parse::<usize>() {
                                self.gui.window_height = h.clamp(200, 4320);
                            }
                        }
                        "menu_bar" => {
                            if let Some(b) = as_bool {
                                self.gui.menu_bar = b;
                            }
                        }
                        "status_bar" => {
                            if let Some(b) = as_bool {
                                self.gui.status_bar = b;
                            }
                        }
                        "scrollbar" => {
                            if let Some(b) = as_bool {
                                self.gui.scrollbar = b;
                            }
                        }
                        "scale_factor" => {
                            if let Ok(f) = val_unquoted.parse::<f64>() {
                                if f >= 0.0 && f <= 5.0 {
                                    self.gui.scale_factor = f;
                                }
                            }
                        }
                        "font_scale" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                self.gui.font_scale = n.clamp(1, 3);
                            }
                        }
                        "font_antialiasing" => {
                            if let Some(b) = as_bool {
                                self.gui.font_antialiasing = b;
                            }
                        }
                        "gamma_correction" => {
                            if let Some(b) = as_bool {
                                self.gui.gamma_correction = b;
                            }
                        }
                        "line_spacing" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                self.gui.line_spacing = n.min(24);
                            }
                        }
                        "rounded_corners" => {
                            if let Some(b) = as_bool {
                                self.gui.rounded_corners = b;
                            }
                        }
                        "shadows" => {
                            if let Some(b) = as_bool {
                                self.gui.shadows = b;
                            }
                        }
                        "cursor_style" => {
                            if !val_unquoted.is_empty() {
                                self.gui.cursor_style = val_unquoted.to_string();
                            }
                        }
                        "cursor_blink" => {
                            if let Some(b) = as_bool {
                                self.gui.cursor_blink = b;
                            }
                        }
                        "sidebar" => {
                            if let Some(b) = as_bool {
                                self.gui.sidebar = b;
                            }
                        }
                        "sidebar_width" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                self.gui.sidebar_width = n.clamp(140, 600);
                            }
                        }
                        "minimap" => {
                            if let Some(b) = as_bool {
                                self.gui.minimap = b;
                            }
                        }
                        "minimap_width" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                self.gui.minimap_width = n.clamp(40, 300);
                            }
                        }
                        "git_gutter" => {
                            if let Some(b) = as_bool {
                                self.gui.git_gutter = b;
                            }
                        }
                        "terminal_height" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                self.gui.terminal_height = n.clamp(60, 1000);
                            }
                        }
                        "terminal_shell" => {
                            self.gui.terminal_shell = val_unquoted.to_string();
                        }
                        "font_ligatures" => {
                            if let Some(b) = as_bool {
                                self.gui.font_ligatures = b;
                            }
                        }
                        "font_hinting" => {
                            if !val_unquoted.is_empty() {
                                self.gui.font_hinting = val_unquoted.to_string();
                            }
                        }
                        "font_family" => {
                            if !val_unquoted.is_empty() {
                                self.gui.font_family = val_unquoted.to_string();
                            }
                        }
                        "font_size" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                self.gui.font_size = n.clamp(8, 48);
                            }
                        }
                        "font_weight" => {
                            if !val_unquoted.is_empty() {
                                self.gui.font_weight = val_unquoted.to_string();
                            }
                        }
                        "smooth_shadows" => {
                            if let Some(b) = as_bool {
                                self.gui.smooth_shadows = b;
                            }
                        }
                        "animation_enabled" => {
                            if let Some(b) = as_bool {
                                self.gui.animation_enabled = b;
                            }
                        }
                        "edge_perception" => {
                            if let Some(b) = as_bool {
                                self.gui.edge_perception = b;
                            }
                        }
                        "subpixel_antialiasing" => {
                            if let Some(b) = as_bool {
                                self.gui.subpixel_antialiasing = b;
                            }
                        }
                        "letter_spacing" => {
                            if let Ok(n) = val_unquoted.parse::<usize>() {
                                self.gui.letter_spacing = n.min(12);
                            }
                        }
                        "font_aspect_ratio" => {
                            if let Ok(f) = val_unquoted.parse::<f64>() {
                                if f >= 0.40 && f <= 0.90 {
                                    self.gui.font_aspect_ratio = f;
                                }
                            } else {
                                match val_unquoted.to_lowercase().as_str() {
                                    "compact" | "classic" => self.gui.font_aspect_ratio = 0.50,
                                    "normal" | "balanced" => self.gui.font_aspect_ratio = 0.55,
                                    "comfort" | "ergonomic" => self.gui.font_aspect_ratio = 0.60,
                                    "wide" => self.gui.font_aspect_ratio = 0.65,
                                    _ => {}
                                }
                            }
                        }
                        "stem_darkening" => {
                            if let Some(b) = as_bool {
                                self.gui.stem_darkening = b;
                            }
                        }
                        "stem_darkening_amount" => {
                            if let Ok(f) = val_unquoted.parse::<f64>() {
                                self.gui.stem_darkening_amount = f.clamp(0.0, 3.0);
                            }
                        }
                        "ergonomic_scaling" => {
                            if let Some(b) = as_bool {
                                self.gui.ergonomic_scaling = b;
                            }
                        }
                        "viewing_distance_mm" => {
                            if let Ok(f) = val_unquoted.parse::<f64>() {
                                self.gui.viewing_distance_mm = f.clamp(0.0, 3000.0);
                            }
                        }
                        "iso_target_arcmin" => {
                            if let Ok(f) = val_unquoted.parse::<f64>() {
                                self.gui.iso_target_arcmin = f.clamp(10.0, 40.0);
                            }
                        }
                        "sdf_font_rendering" => {
                            if let Some(b) = as_bool {
                                self.gui.sdf_font_rendering = b;
                            }
                        }
                        "optical_punctuation_spacing" => {
                            if let Some(b) = as_bool {
                                self.gui.optical_punctuation_spacing = b;
                            }
                        }
                        "use_ttf_font" => {
                            if let Some(b) = as_bool {
                                self.gui.use_ttf_font = b;
                            }
                        }
                        "ttf_font_path" => {
                            self.gui.ttf_font_path = val_unquoted.to_string();
                        }
                        _ => {}
                    },
                    "lsp" => match key.as_str() {
                        "enabled" => {
                            if let Some(b) = as_bool {
                                self.lsp.enabled = b;
                            }
                        }
                        "auto_start" => {
                            if let Some(b) = as_bool {
                                self.lsp.auto_start = b;
                            }
                        }
                        "rust_server" => {
                            if !val_unquoted.is_empty() {
                                self.lsp.rust_server = val_unquoted.to_string();
                            }
                        }
                        "python_server" => {
                            if !val_unquoted.is_empty() {
                                self.lsp.python_server = val_unquoted.to_string();
                            }
                        }
                        "c_server" => {
                            if !val_unquoted.is_empty() {
                                self.lsp.c_server = val_unquoted.to_string();
                            }
                        }
                        "typescript_server" => {
                            if !val_unquoted.is_empty() {
                                self.lsp.typescript_server = val_unquoted.to_string();
                            }
                        }
                        "go_server" => {
                            if !val_unquoted.is_empty() {
                                self.lsp.go_server = val_unquoted.to_string();
                            }
                        }
                        "html_css_server" => {
                            if !val_unquoted.is_empty() {
                                self.lsp.html_css_server = val_unquoted.to_string();
                            }
                        }
                        "json_server" => {
                            if !val_unquoted.is_empty() {
                                self.lsp.json_server = val_unquoted.to_string();
                            }
                        }
                        "markdown_server" => {
                            if !val_unquoted.is_empty() {
                                self.lsp.markdown_server = val_unquoted.to_string();
                            }
                        }
                        "zig_server" => {
                            if !val_unquoted.is_empty() {
                                self.lsp.zig_server = val_unquoted.to_string();
                            }
                        }
                        "inlay_hints" => {
                            if let Some(b) = as_bool {
                                self.lsp.inlay_hints = b;
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
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
