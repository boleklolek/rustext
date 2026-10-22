# 📦 Changelog — Historial de Cambios de Rustext

Todos los cambios notables de **Rustext** se documentan en este archivo.

El formato está basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/), y este proyecto se adhiere a [Semantic Versioning](https://semver.org/lang/es/).

---

## [Sin publicar]

### ✨ Añadido (Added)
- **Pestañas reordenables arrastrando con el ratón** en la barra de pestañas (GUI): `Editor::move_tab`, `GfxRenderer::tab_rects` y `GfxRenderer::tab_drop_index`. La pestaña activa y los índices de los paneles divididos siguen a su documento.
- **Portapapeles nativo** en Windows (`clip` / `Get-Clipboard`) y macOS (`pbcopy` / `pbpaste`), y `xsel` como respaldo en Linux.
- **Backend Win32**: atajos `Alt+Letra/Dígito/[/\/Flechas`, DPI real del sistema (`GetDpiForSystem`) y coordenadas de cliente correctas en la rueda del ratón.

### 🔧 Cambiado (Changed)
- **macOS**: el release compila un binario universal real (`aarch64` + `x86_64` con `lipo`, `MACOSX_DEPLOYMENT_TARGET=11.0`) y el CI prueba en runners Apple Silicon e Intel.

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

[v0.2.0]: https://github.com/boleklolek/rustext/compare/v0.1.0...v0.2.0
[v0.1.0]: https://github.com/boleklolek/rustext/releases/tag/v0.1.0
