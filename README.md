# 🦀 Rustext

[![CI](https://github.com/boleklolek/rustext/actions/workflows/ci.yml/badge.svg)](https://github.com/boleklolek/rustext/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Dependencies](https://img.shields.io/badge/Dependencies-0%20(Pure%20std)-brightgreen.svg)](Cargo.toml)
[![Platforms](https://img.shields.io/badge/Platforms-Linux%20%7C%20Windows%20%7C%20macOS-orange.svg)](#)

**Rustext** es un editor de código y texto nativo de ultra-baja latencia y alto rendimiento, escrito **100% en Rust puro utilizando exclusivamente la biblioteca estándar (`std`)**, con **cero dependencias de terceros** (`Cargo.toml` completamente vacío).

Inspirado en la velocidad e inmediatez de **Sublime Text** y la ergonomía moderna de **VS Code / Zed**, Rustext ofrece una experiencia de desarrollo nativa fluida en cualquier máquina, arrancando en **~2 milisegundos** y utilizando apenas una fracción de la memoria de los editores convencionales.

Consulte el [**📖 Manual de Usuario y Referencia Técnica Completa (MANUAL.md)**](MANUAL.md) para explicaciones en profundidad de la arquitectura, configuración y herramientas.

---

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ Archivo   Editar   Selección   Ver   Preferencias   Extensiones   Herramientas   Ayuda │  ← Menú Nativo
├──────────────────┬─────────────────────────────────────────────────────────────────────┤
│ PROYECTO [+] ↺ × │  1 main.rs ×    2 config.ini ×   [+]                                │  ← Pestañas
├──────────────────┼─────────────────────────────────────────────────────────────────────┤
│ ▼ src            │  1 │ pub fn main() {                                             ▲  │
│   • lib.rs       │  2 │     let config = EditorConfig::load();                      │  │
│   • main.rs      │  3 │     println!("Rustext listo: 0 dependencias");              █  │  ← Scrollbar Interactivo
│ ▶ editor         │  4 │ }                                                           │  │
│ ▶ gfx            │  ~ │                                                             ▼  │
│ • Cargo.toml     │  ~ │                                                                │
├──────────────────┴─────────────────────────────────────────────────────────────────────┤
│ [ EDIT ]  src/main.rs  |  Línea 3, Col 5  |  UTF-8  |  Rust  |  Cursores: 1            │  ← Barra de Estado
└────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## ⚡ Características Principales

- **Cero Dependencias Externas (`Cargo.toml` 100% limpio)**:
  - Binario autónomo compilado de apenas **~650 KB**.
  - Compilación completa en **menos de 2 segundos**.
  - Libre de riesgos de seguridad en la cadena de suministro y dependencias desactualizadas.

- **Arquitectura de Búfer Piece Table de Alto Rendimiento**:
  - Estructura `PieceTable` con memoria original inmutable y fragmentación dinámica.
  - Iteración *zero-copy* sobre fragmentos sin duplicación de cadenas.
  - Soporte completo y verificado para caracteres y símbolos UTF-8 multibyte.

- **Motor Multicursor Vectorial Paralelo**:
  - Edición en paralelo con sincronización automática de desplazamientos (`offset shift`).
  - Selección de siguiente ocurrencia con `Ctrl + D` (estilo Sublime Text / VS Code).
  - Añadir cursores en líneas contiguas con `Ctrl + K` (arriba) y `Ctrl + J` (abajo).
  - Transacciones atómicas reversibles con `Ctrl + Z` (Undo) y `Ctrl + Y` (Redo).

- **Interfaz Gráfica Moderna (Desktop GUI)**:
  - Motor de renderizado de software con Framebuffer de 32-bit ARGB.
  - Esquinas redondeadas (*anti-aliased*), sombras proyectadas suaves y píldoras informativas.
  - Barra de menús nativa con navegación por ratón y atajos `Alt + Letra`.
  - Barra de desplazamiento (scrollbar) con pulgar arrastrable y clics de página.
  - 8 Paletas de color integradas: Tokyo Night, Catppuccin Mocha, One Dark, Modern Dark, Monokai, Solarized Dark, GitHub Light y High Contrast.

- **Árbol de Proyecto y Explorador Lateral (File Explorer)**:
  - Barra lateral acoplada a la izquierda con visibilidad alternable mediante `Ctrl + B`.
  - Escaneo perezoso de carpetas (solo inspecciona en disco lo que se expande).
  - Pistas de color automáticas por extensión de archivo (`.rs`, `.toml`, `.md`, `.py`, `.c`, etc.).
  - Controles de ratón: clic para conmutar/abrir archivo, expandir/contraer carpetas, botón de nueva pestaña `[+]`, refrescar `[↺]` y cerrar `[×]`.
  - Navegación completa por teclado (`↑`/`↓`, `→`/`Enter`, `←`, `Esc`).

- **Diagnósticos del Compilador y Linter en Vivo**:
  - Integración nativa con `cargo check --message-format=json`, `pyflakes` y `clang`.
  - Insignias de severidad en el gutter (`[E]` Error, `[W]` Warning, `[I]` Info) y mensaje detallado en la barra de estado.
  - Navegación rápida entre errores con `F8` (Siguiente) y `Shift + F8` (Anterior).

- **Formateo Automático de Código**:
  - Formateo instantáneo con `Ctrl + Shift + I` mediante los formateadores del sistema (`rustfmt`, `black`, `clang-format`, `prettier`, `gofmt`).
  - Opción de formateo y comprobación automática al guardar (`format_on_save`, `check_on_save`).

- **Motor de Snippets e Indentación Inteligente**:
  - Snippets de lenguaje expandibles con `Tab` (`fn`, `struct`, `for`, `match`, `test`).
  - Navegación entre marcadores de posición mediante `Tab` y `Shift + Tab`.
  - Auto-cierre de delimitadores (`()`, `[]`, `{}`, `""`, `''`) y auto-envoltorio de selecciones.

- **Modo Terminal TUI de Doble Búfer**:
  - Renderizado atómico libre de parpadeo (*flicker-free*) para entornos SSH o servidores sin servidor gráfico.
  - Restauración segura del modo raw mediante guardia RAII (`TerminalGuard`).

- **Multiplataforma Nativo**:
  - **Linux**: Integración directa con X11 (`libX11.so`).
  - **Windows**: Integración directa con Win32 API (`user32.dll`, `gdi32.dll`).
  - **macOS**: Integración directa con Cocoa / AppKit (`objc_msgSend`).

---

## 📊 Perfil de Memoria y Benchmarks de Rendimiento

Rustext incluye un perfilador y benchmark integrado en [`src/bin/memory_profile.rs`](src/bin/memory_profile.rs) que audita byte por byte el uso del heap mediante un asignador atómico propio y mide la memoria física residente (`VmRSS`) reportada por el kernel:

```bash
cargo run --release --bin memory_profile
```

### Resultados Medidos en Máquina de Prueba (x86_64 Linux):

| Escenario | Tamaño Archivo | Líneas | Memoria Heap Neta | Ratio Memoria | VmRSS del SO (RAM Física) | Throughput Carga | Renderizado Viewport |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **Script Pequeño** | 1.00 KB | 21 | **1.28 KB** | **1.28x** | 2.67 MB | 136.9 MB/s | 599.6 µs (~1,667 FPS) |
| **Módulo Típico** | 10.00 KB | 204 | **12.03 KB** | **1.20x** | 2.80 MB | 422.9 MB/s | 806.6 µs (~1,240 FPS) |
| **Módulo Extenso** | 50.00 KB | 1,013 | **58.03 KB** | **1.16x** | 4.64 MB | 617.3 MB/s | 928.0 µs (~1,077 FPS) |
| **Archivo Mediano** | 500.00 KB | 10,131 | **628.03 KB** | **1.26x** | 4.78 MB | 603.0 MB/s | 1.35 ms (~740 FPS) |
| **Bundle / Dataset**| 2.00 MB | 41,497 | **2.50 MB** | **1.25x** | 8.30 MB | 240.4 MB/s | 621.1 µs (~1,610 FPS) |
| **Volcado de Código**| 10.00 MB | 207,481 | **12.00 MB** | **1.20x** | 24.40 MB | 410.2 MB/s | 647.4 µs (~1,544 FPS) |
| **Log Grande** | 25.00 MB | 518,702 | **29.00 MB** | **1.16x** | 58.23 MB | 452.9 MB/s | 542.0 µs (~1,845 FPS) |
| **Dump Servidor** | 50.00 MB | 1,036,678 | **58.00 MB** | **1.16x** | 112.60 MB | 481.3 MB/s | 651.5 µs (~1,535 FPS) |
| **Archivo Masivo** | **100.00 MB** | **2,053,953** | **116.00 MB** | **1.16x** | **220.55 MB** | **386.6 MB/s** | **638.0 µs (~1,567 FPS)** |

### Sesión Multi-Pestaña Cotidiana (5 pestañas abiertas simultáneamente):
- Archivos cargados: `main.rs` (15 KB), `config.ini` (2 KB), `parser.rs` (45 KB), `large_module.rs` (250 KB), `data_dump.json` (1 MB).
- **Memoria Heap neta usada por las 5 pestañas**: **1.64 MB** (~336 KB promedio por pestaña).
- **Memoria física total del proceso**: **36.30 MB** (incluyendo el Framebuffer gráfico 800×600 a 32-bit de 1.9 MB, glifos bitmap, barra de menús, árbol de proyecto e historial).
- **Renderizado del fotograma completo con Sidebar + Menú**: **1,128 µs** (~886 FPS sostenidos).

> [!TIP]
> **Renderizado en Tiempo Constante $\mathcal{O}(1)$**:  
> Observe la columna de renderizado: abrir y dibujar un archivo de **1 KB** toma ~600 µs, y un archivo de **100 MB** con **2 millones de líneas** toma exactamente lo mismo (~638 µs). El motor gráfico calcula el recorte de visibilidad en el viewport y solo consulta las líneas activas en pantalla.

---

## ⌨️ Atajos de Teclado Principales

| Atajo | Acción |
| :--- | :--- |
| `Ctrl + S` | Guardar el archivo actual |
| `Ctrl + O` | Abrir archivo mediante diálogo interactivo |
| `Ctrl + T` | Abrir nueva pestaña vacía |
| `Ctrl + W` | Cerrar pestaña activa |
| `Ctrl + Q` | Salir de Rustext |
| `Ctrl + B` | **Alternar Explorador Lateral (Árbol de Proyecto)** |
| `Ctrl + D` | **Seleccionar palabra / Añadir siguiente ocurrencia (Multicursor)** |
| `Ctrl + K` / `Ctrl + J` | Añadir cursor en la línea superior / inferior |
| `Esc` | Colapsar multicursores a cursor único / Cancelar modal |
| `Ctrl + Z` / `Ctrl + Y` | Deshacer / Rehacer atómico |
| `Ctrl + F` | Buscar en el documento |
| `Ctrl + H` | Buscar y Reemplazar |
| `Ctrl + P` | **Abrir Paleta de Comandos** |
| `PageUp` / `PageDown` | Cambiar de pestaña (Anterior / Siguiente) |
| `Tab` | Indentación o expandir snippet (`fn`, `struct`, etc.) |
| `Ctrl + Shift + I` | **Formatear documento con herramienta del sistema** |
| `F7` | **Comprobar errores (Lint / Check)** |
| `F8` / `Shift + F8` | Saltar al siguiente / anterior error de compilación |
| `F1` | Ventana Acerca de Rustext |

---

## ⚙️ Configuración (`config.ini`)

Rustext crea y lee automáticamente su configuración en formato INI estándar:
- **Linux / macOS**: `~/.config/rustext/config.ini`
- **Windows**: `%APPDATA%\rustext\config.ini`

Cualquier cambio guardado en `config.ini` se recarga en caliente de manera inmediata sin necesidad de reiniciar el editor.

```ini
[editor]
tab_size = 4
auto_indent = true
auto_close_brackets = true
auto_surround = true
show_line_numbers = true
format_on_save = false
check_on_save = false

[theme]
name = tokyo-night   # tokyo-night, catppuccin-mocha, one-dark, monokai, solarized-dark, light

[gui]
window_width = 1100
window_height = 720
font_scale = 1
line_spacing = 2
cursor_style = bar   # bar, block, underline
cursor_blink = true
rounded_corners = true
shadows = true
scrollbar = true
menu_bar = true
sidebar = true
sidebar_width = 220
```

---

## 🚀 Inicio Rápido

### Compilar y Ejecutar Pruebas:
```bash
cargo test
```

### Iniciar en Modo GUI de Escritorio (Recomendado):
```bash
cargo run --release -- --gui
# O directamente sobre un archivo/carpeta:
cargo run --release -- -g src/main.rs
```

### Iniciar en Modo Terminal (TUI):
```bash
cargo run --release
cargo run --release src/main.rs
```

### Ejecutar Suite de Perfilado de Memoria:
```bash
cargo run --release --bin memory_profile
```

---

## 📚 Documentación Adicional

Para más detalles sobre la arquitectura interna, diseño del Piece Table, protocolo de plugins externos y guía de desarrollo, revise el documento:
- [**MANUAL.md - Manual de Usuario y Referencia Técnica**](MANUAL.md)

---

## 📄 Licencia

Este proyecto está bajo la Licencia MIT. Consulte el archivo `LICENSE` para más detalles.
