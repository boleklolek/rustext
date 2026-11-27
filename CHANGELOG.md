# 📦 Changelog — Historial de Cambios de Rustext

Todos los cambios notables de **Rustext** se documentan en este archivo.

El formato está basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/), y este proyecto se adhiere a [Semantic Versioning](https://semver.org/lang/es/).

---

## [Sin publicar]

## [v0.4.0] - 2026-11-27

### ✨ Añadido (Added)
- **Visor Git Diff Lado a Lado (*Side-by-Side Diff*) y Operaciones de Staging**:
  - Comando `OpenGitDiffSplit` (`Alt+D` / Paleta de Comandos) para comparar interactivamente el archivo activo en vista dividida contra la versión comprometida en `git HEAD`.
  - Creación de pestaña de solo lectura `[Git HEAD] <archivo>` ubicada a la izquierda, posicionando la copia de trabajo a la derecha para edición directa y revisión simultánea de cambios.
  - Comandos para gestión del índice Git directamente desde el editor:
    - `GitStageFile`: prepara el archivo actual en el área de preparación (`git add <file>`).
    - `GitUnstageFile`: retira el archivo del área de preparación (`git restore --staged <file>` / `git reset HEAD <file>`).
  - Integración en la barra de menú superior (**Herramientas > Git**), en la Paleta de Comandos (`Ctrl+P` / `Ctrl+Shift+P`) y en el mapa de teclas estándar.
- **Vista de Esquema de Código Nativa (Code Outline) y Sticky Scroll Multinivel**:
  - Motor extractor de símbolos del documento sin dependencias en [`src/editor/folding.rs`](src/editor/folding.rs) (`extract_document_outline`):
    - Detección precisa de funciones (`fn`, `def`, `func`), structs, clases (`class`), traits/interfaces, enums, bloques `impl`, módulos y constantes.
    - Soporte específico para lenguajes populares: Rust, Python, Go, Zig, Lua, C, JavaScript y encabezados jerárquicos Markdown (`#`, `##`, `###`).
    - Integración nativa como respaldo instantáneo en la paleta de símbolos (`Ctrl+Shift+R`), garantizando disponibilidad inmediata incluso si el servidor LSP está ausente o iniciándose.
  - Expansión de soporte multilingüe en el motor de plegado de código (`FoldingManager`): coincidencia sintáctica de llaves `{}` y cálculo de ámbitos jerárquicos para Go, Zig y Lua.
  - Plegado y encabezados fijos de ámbito (*Sticky Scroll*) multinivel que fijan con precisión módulos, estructuras y funciones en cascada mientras se navega el código.
- **Protocolo IPC Estructurado y JSON-RPC 2.0 para Plugins y Extensiones**:
  - Implementación completa de [`src/plugin/ipc.rs`](src/plugin/ipc.rs) con cero dependencias externas, utilizando el analizador JSON AST nativo (`JsonValue`).
  - Solicitudes estructuradas con contexto ampliado del editor: `ExtensionIpcRequest` transmite ID de solicitud, método, comando invocado, ruta del archivo, contenido del búfer, texto seleccionado, posición del cursor (`line`, `col`), total de líneas, lenguaje activo y parámetros personalizados.
  - Soporte de múltiples acciones en respuestas (`ExtensionIpcResponse` y `ExtensionAction`):
    - `ReplaceText`: reemplazo del texto seleccionado o búfer actual.
    - `ReplaceBuffer`: reemplazo incondicional del búfer completo.
    - `SetStatus`: actualización del mensaje en la barra de estado inferior.
    - `SetClipboard`: copia de texto al portapapeles del sistema/editor.
    - `OpenPanel`: apertura de panel informativo con título y lista de líneas.
  - Compatibilidad total hacia atrás: si un comando externo (CLI simple como `tr`, `sed`, `sh` o scripts heredados) devuelve texto plano en lugar de JSON-RPC, el editor interpreta la salida de manera transparente como reemplazo de texto.
  - Integración nativa en `ExtensionManager::execute_command_ipc` y ejecución en `Editor::execute_extension_command`.

- **Backend Gráfico Nativo para macOS (`CocoaWindow`)**:
  - Implementación completa sin dependencias externas de `CocoaWindow` en [`src/window/macos.rs`](src/window/macos.rs) utilizando AppKit y CoreGraphics mediante el runtime de Objective-C (`objc_msgSend`).
  - Creación y activación de ventana nativa `NSWindow` con estilo estándar (cerrar, minimizar, redimensionar, pantalla completa) y soporte Retina hidpi (`scale_factor`).
  - Despacho de eventos de usuario en `poll_event`: pulsaciones de teclas (`Char` / `Key` con soporte para modificadores `Command`/`Ctrl`, `Shift`, `Option`/`Alt`), clics y arrastre del ratón (`MouseDown`, `MouseUp`, `MouseMove`), desplazamiento vertical (`ScrollWheel`) y redimensionamiento dinámico (`Resize`).
  - Renderizado de framebuffer por software sobre `NSView` / `CALayer` mediante `CGBitmapContext` y `CGImage`.
  - Soporte para acciones de ventana: minimizar (`miniaturize:`), alternar zoom/maximizar (`zoom:`) y arrastre interactivo (`performWindowDragWithEvent:`).
  - *Nota*: Desarrollado y validado a nivel de tipos; no probado visualmente en hardware Mac en esta sesión.
- **Expansión de Sintaxis y Lenguajes (Go, Zig, Lua)**:
  - Soporte de primer nivel con cero dependencias en el lexer y tokenizador nativo para:
    - **Go (`.go`)**: palabras clave (`func`, `defer`, `chan`, `go`, `package`, etc.), tipos integrados (`bool`, `int`, `rune`, `string`, etc.), literales de cadena multilínea en comillas invertidas (`` `raw string` ``) y comentarios de línea (`//`) y bloque (`/* ... */`).
    - **Zig (`.zig`)**: palabras clave (`fn`, `pub`, `comptime`, `defer`, `errdefer`, etc.), tipos de tamaño fijo (`i32`, `u8`, `usize`, etc.), macros/funciones integradas con `@` (`@import`, `@as`, etc.) y comentarios de línea (`//`).
    - **Lua (`.lua`)**: palabras clave (`local`, `function`, `then`, `end`, etc.), tipos base (`table`, `thread`, etc.), comentarios de línea (`--`) y comentarios de bloque multilínea (`--[[ ... ]]`).
  - Configuración automática de tokens de comentario para alternar comentarios con `Ctrl+/` (`--` en Lua, `//` en Go y Zig).
  - Catálogo de snippets integrados con Tabstops para Go (`fn`, `main`, `struct`, `iferr`, `for`), Zig (`fn`, `main`, `struct`, `test`) y Lua (`fn`, `for`, `if`).
  - Sincronización e integración con formateadores externos del sistema (`gofmt`, `zig fmt`, `stylua`) mediante conductos estándar.
- **Línea de Comandos Vim (`:`) y Operadores Avanzados**:
  - Modal interactivo de línea de comandos estilo Vim (`:`) que soporta comandos esenciales:
    - `:w` / `:write` y `:w <ruta>` para guardar el búfer activo o guardar en una ruta específica.
    - `:q` / `:quit` para cerrar la pestaña o salir del editor (respetando comprobación de cambios no guardados).
    - `:q!` / `:quit!` para forzar el cierre descartando modificaciones.
    - `:wq` / `:x` para guardar y cerrar la pestaña o el editor.
    - `:qa` / `:qall` y `:qa!` para cerrar todas las pestañas.
    - `:e <ruta>` / `:edit <ruta>` para abrir archivos directamente.
    - `:<número>` para salto instantáneo a la línea indicada con centrado de viewport.
    - `:s/patrón/reemplazo/` y `:%s/patrón/reemplazo/` para sustitución global en todo el búfer.
  - Movimientos avanzados de texto en Vim:
    - `w`: avance al inicio de la siguiente palabra.
    - `b`: retroceso al inicio de la palabra anterior.
    - `G`: salto al final del archivo.
    - `g g`: salto al inicio del archivo mediante acorde compuesto.
  - Operadores de línea e inserción:
    - `d d`: borrado de la línea completa con corte al portapapeles.
    - `y y`: copia de la línea completa al portapapeles.
    - `o`: apertura de una nueva línea abajo con auto-indentación e ingreso automático a modo *Insert*.
    - `O`: apertura de una nueva línea arriba con auto-indentación e ingreso automático a modo *Insert*.
  - **Cursor dinámico según el modo modal**:
    - En GUI nativa: cursor de bloque sólido (`block`) en modo *Normal* y *Visual*, y cursor de barra delgada (`bar`) en modo *Insert*.
    - En TUI terminal: emisión reactiva de secuencias de escape ANSI DECSCUSR (`\x1b[2 q` para bloque y `\x1b[6 q` para barra/línea).

---

## [v0.3.0] - 2026-11-27

### ✨ Añadido (Added)
- **Modularización y Desacoplamiento de Componentes Clave (Fase 1)**:
  - Extracción del AST y analizador sintáctico JSON independiente en [`src/core/json.rs`](src/core/json.rs), desacoplándolo del protocolo LSP y facilitando su uso transversal con cero dependencias externas.
  - Extracción de diálogos modales y confirmaciones de salida/guardado (`UnsavedConfirmDialog`, `EditorMode`) en [`src/editor/dialogs.rs`](src/editor/dialogs.rs).
  - Desacoplamiento de primitivas de geometría interactiva (`ScrollbarGeometry`, `MinimapGeometry`, `AutocompleteGeometry`, `PaneGeometry`) en [`src/gfx/geometry.rs`](src/gfx/geometry.rs).
  - Extracción de paletas de temas integradas y estructura `ThemeColors` en [`src/gfx/colors.rs`](src/gfx/colors.rs), descongestionando el renderizador principal `src/gfx/renderer.rs`.
- **Sistema Modular de Mapas de Teclas (`.keymap`) y Perfiles Vim / Emacs**:
  - Arquitectura declarativa de plantillas de atajos `.keymap` inspirada en el sistema de temas: plantillas oficiales modularizadas en `assets/keymaps/` (`default.keymap`, `vim.keymap`, `emacs.keymap`) y soporte para mapas personalizados de usuario en `~/.rustext/keymaps/` y `./.rustext/keymaps/`.
  - Soporte completo para acordes compuestos de 2 pasos (`KeyChord::Sequence`) como `Ctrl+X Ctrl+S` en Emacs y `g g` en Vim.
  - Soporte para edición modal (Normal, Insert, Visual) para Vim con transiciones directas (`i`, `a`, `v`, `Esc`) e indicador reactivo en la barra de estado (`[ NORMAL ]`, `[ INSERT ]`, `[ VISUAL ]`).
  - Configuración persistente `keymap = default` en `config.ini`, menú interactivo en *Preferencias > Mapa de Teclas* y acciones en la Paleta de Comandos (`Ctrl+Shift+P`).
- **Controles de ventana integrados (CSD) para Wayland**: botones nativos de Minimizar (`—`), Maximizar/Restaurar (`□`) y Cerrar (`✕`) en la barra de menús con efectos hover y soporte para `xdg_toplevel.set_minimized`, `set_maximized` y `unset_maximized`. Permite control total de la ventana en compositores sin decoraciones del servidor como GNOME/Mutter.
- **Pestañas reordenables arrastrando con el ratón** en la barra de pestañas (GUI): `Editor::move_tab`, `GfxRenderer::tab_rects` y `GfxRenderer::tab_drop_index`. La pestaña activa y los índices de los paneles divididos siguen a su documento.
- **Portapapeles nativo** en Windows (`clip` / `Get-Clipboard`) y macOS (`pbcopy` / `pbpaste`), y `xsel` como respaldo en Linux.
- **Backend Win32**: atajos `Alt+Letra/Dígito/[/\/Flechas`, DPI real del sistema (`GetDpiForSystem`) y coordenadas de cliente correctas en la rueda del ratón.

### 🐛 Corregido (Fixed)
- **Wayland en GNOME/Mutter**: los IDs de objeto del cliente ahora se asignan de forma secuencial y solo se consumen para los globales realmente anunciados (`wl_seat`, `zxdg_decoration_manager_v1`). Antes un hueco en los IDs hacía que el compositor cerrara la conexión con `invalid arguments for wl_compositor#N.create_surface` y la GUI no abría. Además, un fallo de escritura ahora informa el error de protocolo real (`wl_display.error`) en vez de solo `Broken pipe`.

### 🔧 Cambiado (Changed)
- **macOS**: el release compila un binario universal real (`aarch64` + `x86_64` con `lipo`, `MACOSX_DEPLOYMENT_TARGET=11.0`) y el CI prueba en runners Apple Silicon e Intel.
- **Documentación y Badges**: actualización de métricas del suite de pruebas en `README.md` y `README_EN.md` a 248 tests pasando. Traducción técnica completa de toda la documentación al inglés: `SECURITY_EN.md`, `CONTRIBUTING_EN.md`, `CHANGELOG_EN.md`, `MANUAL_EN.md` y `MANUAL_THEMES_AND_EXTENSIONS_EN.md`.

---

## [v0.2.0] - 2026-10-21

### ✨ Añadido (Added)
- **Instaladores Nativos Multiplataforma Automatizados en CI/CD**:
  - **Windows (`.msi` y `.zip` portable)**: Integración con WiX Toolset (`installer/windows/rustext.wxs`) y empaquetador PowerShell (`installer/windows/build.ps1`) con accesos directos en el Menú Inicio.
  - **Linux Debian/Ubuntu (`.deb`)**: Script de construcción [`installer/linux/build_deb.sh`](installer/linux/build_deb.sh) que instala `/usr/bin/rustext`, entrada de escritorio `.desktop`, icono vectorial `.svg`, temas en `/usr/share/rustext/themes/` y manuales en `/usr/share/doc/rustext/`.
  - Publicación automática de binarios y sumas criptográficas (`SHA256SUMS.txt`) para Linux (`x86_64`), Windows (`x86_64 MSVC`) y macOS (`apple-darwin`) en GitHub Releases.
- **Motor Dinámico y Declarativo de Temas (`.theme`) Sin Recompilación**:
  - Descubrimiento automático y recarga en caliente de temas desde `~/.rustext/themes/` y `./.rustext/themes/`.
  - 8 temas oficiales modularizados en [`assets/themes/`](assets/themes/): `tokyo-night.theme`, `catppuccin-mocha.theme`, `one-dark.theme`, `modern-dark.theme`, `monokai.theme`, `solarized-dark.theme`, `light.theme` y `dracula.theme`.
  - Auto-despliegue de plantillas oficiales en `~/.rustext/themes/` al primer inicio.
- **Documentación Bilingüe y Manuales Técnicos**:
  - Nuevo [`README_EN.md`](README_EN.md) completo en inglés con selector bilingüe (`🇪🇸 Español` / `🇬🇧 English`).
  - Nuevo [`MANUAL_TEMAS_Y_EXTENSIONES.md`](MANUAL_TEMAS_Y_EXTENSIONES.md) con referencia completa de los 55+ slots de color ARGB y arquitectura IPC de extensiones.
  - Nuevos archivos de estándares de comunidad Open Source: [`CHANGELOG.md`](CHANGELOG.md), [`CONTRIBUTING.md`](CONTRIBUTING.md) y [`SECURITY.md`](SECURITY.md).

### 🛡️ Seguridad (Security)
- **SEC-07 (CWE-377 / CWE-59)**: Apertura exclusiva `O_CREAT | O_EXCL` con permisos UNIX `0600` para búferes de memoria compartida en `/dev/shm` cuando `memfd_create` no está disponible.
- **SEC-08 (CWE-22)**: Validación estricta de lista blanca `is_safe_theme_name()` (máx. 64 bytes) y confinamiento canónico `starts_with(canonical_base)` contra ataques de *Path Traversal* en temas externos.
- **SEC-09 (CWE-400 / CWE-776)**: Límite estricto `MAX_THEME_FILE_SIZE = 512 KB` y cuota máxima de 256 archivos `.theme` por directorio para prevenir denegación de servicio (DoS) por agotamiento de memoria.
- **SEC-10 (CWE-401)**: Migración de `Box::leak` a `Cow<'static, str>` en `CommandItem` de la Paleta de Comandos, eliminando fugas acumulativas de memoria al abrir y cerrar el modal repetidamente.

### ⚡ Rendimiento y Correcciones (Performance & Fixed)
- **Optimización de Renderizado y Blending CPU en Tiempo Real (Fase 2)**:
  - Vectorización de filas contiguas en `Framebuffer::fill_rect_alpha` con precomputación de canales fuente fuera del bucle interior de píxeles, reduciendo el costo de dibujo de selecciones, fondos translúcidos y cursores.
  - Formateo de números de línea *zero-allocation* (`format_line_number_into`) mediante búfer de pila fijo de 16 bytes, eliminando la creación repetitiva de objetos `String` en el heap por cada línea visible en cada frame.
  - Transferencia optimizada de búfer SHM en Wayland (`copy_from_slice`) sustituyendo la iteración manual píxel por píxel con copia de memoria acelerada por hardware (`memcpy`).
- **Optimizaciones en Parser de Temas**: Conversión hexadecimal de tiempo constante $\mathcal{O}(1)$ mediante aritmética de bits (`hex_nibble`) y eliminación de asignaciones intermedias en `parse_theme_ini`.
- **Win32 Backend (`src/window/win32.rs`)**: Corrección de decodificación de `WM_CHAR` (`raw_byte` y soporte Unicode UTF-32), manejo de `WM_RBUTTONUP` / `WM_MBUTTONUP` y bloques `unsafe` explícitos requeridos por Rust 2024 Edition.
- **GitHub Actions**: Actualización de todos los workflows a acciones nativas sobre **Node.js 24** (`actions/checkout@v7`, `actions/upload-artifact@v7`, `actions/download-artifact@v8`, `softprops/action-gh-release@v3`).

---

## [v0.1.0] - 2026-10-18

### ✨ Lanzamiento Inicial (Initial Release)
- **Núcleo 100% Rust Estándar (`std::*`) con Cero Dependencias**:
  - Estructura de datos **Piece Table** de alto rendimiento con iteración *zero-copy* y soporte completo UTF-8.
  - **Motor Multicursor Vectorial Paralelo** con sincronización automática de offsets y transacciones Undo/Redo atómicas.
- **Interfaz Gráfica Nativa (Desktop GUI) y Terminal (TUI)**:
  - Backends nativos sin librerías de terceros para **Wayland** (`wl_shm` + `xdg_shell` + `memfd_create`), **X11** (`libX11.so`), **Windows** (`user32.dll` / `gdi32.dll`) y **macOS** (`AppKit`).
  - Motor de renderizado por software en `Framebuffer` ARGB de 32 bits con soporte HiDPI, esquinas redondeadas analíticas (SDF), sombras gaussianas $\mathcal{O}(1)$ y efecto *Frosted Glass*.
  - Modo TUI de doble búfer sin parpadeo con recuperación segura del terminal (`TerminalGuard` y `panic::set_hook`).
- **Tipografía Avanzada y Motor Vectorial TrueType (`.ttf`)**:
  - Decodificador binario SFNT 100% Rust puro, rasterizador analítico trapezoidal, antialiasing subpíxel LCD ClearType, caché de glifos de dos niveles (`TtfGlyphCache`), corrección gamma lineal sRGB (IEC 61966-2-1), ligaduras de programación y selector interactivo de fuentes (`Ctrl + Shift + T`).
- **Motor de Layout Flexbox y Arquitectura Reactiva-Retenida**:
  - Implementación W3C CSS Flexbox en Rust puro combinada con primitivas reactivas de grano fino (`Signal<T>`, `Computed<T>`, `Effect`) y árbol de widgets retenido con `DirtyFlags`.
- **Herramientas de Desarrollo y Suite LSP de Élite**:
  - Cliente LSP nativo preconfigurado para 10 lenguajes (Rust, Python, C/C++, TypeScript/JS, Go, HTML/CSS, JSON, Markdown, Zig y Julia) con soporte para Ir a Definición (`F12`), Referencias (`Ctrl+Shift+U`), Renombrado en Workspace (`F2`), Code Actions (`Ctrl+.`), Outline (`Ctrl+Shift+R`), Inlay Hints, Semantic Tokens (`F5`) y Signature Help.
  - Integración Git en vivo: Gutter diff en tiempo real, Inline Git Blame $\mathcal{O}(1)$, Hunk Diff Popup (`Alt+H`) y reversión atómica de hunks (`Alt+R`).
  - Minimapa sintáctico (`Ctrl+M`), división de paneles vertical/horizontal (`Ctrl+\`), explorador lateral de archivos (`Ctrl+B`), búsqueda global (`Ctrl+Shift+F`), terminal integrado acoplado (`Ctrl+\``) y gestión de espacios de trabajo con sesiones aisladas y *Hot Exit*.
- **Auditoría de Seguridad Inicial (`SEC-01` a `SEC-06`)**:
  - Guardado atómico de archivos, confinamiento de extensiones y borradores, protección contra enlaces simbólicos cíclicos y canalización segura al portapapeles Wayland.

[Sin publicar]: https://github.com/boleklolek/rustext/compare/v0.4.0...HEAD
[v0.4.0]: https://github.com/boleklolek/rustext/compare/v0.3.0...v0.4.0
[v0.3.0]: https://github.com/boleklolek/rustext/compare/v0.2.0...v0.3.0
[v0.2.0]: https://github.com/boleklolek/rustext/compare/v0.1.0...v0.2.0
[v0.1.0]: https://github.com/boleklolek/rustext/releases/tag/v0.1.0
