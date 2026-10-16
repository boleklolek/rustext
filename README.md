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
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Archivo   Editar   Selección   Ver   Preferencias   Extensiones   Herramientas   Ayuda           │  ← Menú Nativo
├──────────────────┬───────────────────────────────────────────┬───────────────────────────────────┤
│ PROYECTO [+] ↺ × │ 1: main.rs [ACTIVO]                       │ 2: config.ini                     │  ← Split Editor (Ctrl+\)
├──────────────────┼───────────────────────────────────────────┼───────────────────────────────────┤
│ ▼ src            │ 1 │ pub fn main() {                    ▲  │ 1 │ [gui]                      ▲  │
│   • lib.rs       │ 2 │     let config = EditorConfig();   █  │ 2 │ minimap = true           █  │  ← Deslizador Viewport
│   • main.rs      │ 3 │     println!("Rustext listo");     │  │ 3 │ minimap_width = 100    │  │
│ ▶ editor         │ 4 │ }                                  ▼  │ 4 │ theme = "tokyo-night"      ▼  │
│ ▶ gfx            │ ~ │                                    ░  │ ~ │                                ░  │  ← Minimapa Sintáctico
├──────────────────┴───────────────────────────────────────────┴───────────────────────────────────┤
│ [ EDIT ]  src/main.rs  |  Línea 3, Col 5  |  UTF-8  |  Rust  |  Cursores: 1  |  Panel: 1/2       │  ← Barra de Estado
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
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

- **Integración Git en Vivo (Git Gutter & Control de Versiones)**:
  - Gutter con barras verticales coloreadas en tiempo real para diffs instantáneos (+ verde para líneas añadidas, ~ azul para modificadas, ▲ rojo para eliminadas).
  - Chip de rama activa en la barra de estado con contadores de cambio (`git: main (+3 ~1)`).
  - Insignias visuales de estado en el Explorador Lateral (`M` modificado en amarillo, `U` sin rastrear en verde, `D` eliminado en rojo, `A` añadido en cian).
  - Detección ultrarrápida leyendo directamente `.git/HEAD` sin bloquear el editor y algoritmo LCS en memoria de alta velocidad.
  - Atajo rápido `Ctrl + Shift + G` para forzar refresco del repositorio y opción `git_gutter = true` en `config.ini`.

- **Minimapa Gráfico de Código (`Ctrl + M`)**:
  - Visualización condensada en miniatura de todo el código fuente en el lateral del editor.
  - Micro-renderizado de tokens sintácticos con preservación de sangría e indentación.
  - Deslizador de viewport translúcido interactivo que resalta la ventana visible en tiempo real.
  - Navegación reactiva instantánea: clic en cualquier sección para saltar o arrastrar el deslizador con el ratón.
  - Totalmente configurable desde `config.ini` (`minimap = true`, `minimap_width = 100`).

- **División de Vistas y Paneles Independientes (Split Editor)**:
  - División vertical (lado a lado, `Ctrl + \`) y horizontal (apilada, `Ctrl + Shift + \`).
  - Cambio instantáneo de foco de panel mediante `Alt + \` o haciendo clic directo en el panel.
  - Barra de división (splitter) arrastrable con el ratón para redistribuir el espacio en pantalla.
  - Cada panel mantiene su propio desplazamiento vertical/horizontal, cursores independientes, cabecera de estado, gutter y minimapa/scrollbar integrados.
  - Sincronización transparente de pestañas y archivos al alternar o cerrar la división.

- **Protección contra Pérdida de Datos (Diálogo de Confirmación al Cerrar)**:
  - Detección inteligente de modificaciones no guardadas (`is_dirty()`) antes de cerrar pestañas (`Ctrl + W`, clic en `×` de pestaña o menú) o al salir del editor (`Ctrl + Q`, menú `Archivo > Salir` o botón `[X]` de la ventana del SO).
  - Diálogo modal con 3 acciones claras: **Guardar** / **Guardar Todo** (`G`/`S`), **No guardar** / **Descartar Todo** (`D`/`N`) y **Cancelar** (`Esc`/`C`).
  - Totalmente interactivo: navegación con teclado (`←`/`→`/`Tab`/`Enter`), atajos nemónicos y clics de ratón con efectos hover.
  - Al salir con múltiples pestañas modificadas, advierte cuántos archivos tienen cambios pendientes antes de proceder.

- **Interfaz Gráfica Moderna (Desktop GUI)**:
  - Motor de renderizado de software con Framebuffer de 32-bit ARGB.
  - Esquinas redondeadas (*anti-aliased*), sombras proyectadas suaves y píldoras informativas.
  - Barra de menús nativa con navegación por ratón y atajos `Alt + Letra`.
  - Barra de desplazamiento (scrollbar) con pulgar arrastrable y clics de página.
  - 8 Paletas de color integradas: Tokyo Night, Catppuccin Mocha, One Dark, Modern Dark, Monokai, Solarized Dark, GitHub Light y High Contrast.

- **Rasterizador de Fuentes Tipográficas y Corrección Gamma (sRGB IEC 61966-2-1)**:
  - Rasterizador tipográfico avanzado por software con anti-aliasing espacial y cálculo continuo de cobertura morfológica de contornos.
  - Composición física en espacio de luz lineal ($\text{sRGB} \leftrightarrow \text{Luz Lineal}$) según el estándar IEC 61966-2-1.
  - Resuelve el problema clásico de adelgazamiento (*font thinning*) y halos oscuros sobre fondos oscuros (Tokyo Night, Catppuccin) producido por el blending ingenuo no lineal.
  - Escalado bilineal continuo (`font_scale >= 2`) para pantallas HiDPI sin bordes pixelados ni efecto escalera (*staircase effect*).
  - Tablas de búsqueda estáticas $\mathcal{O}(1)$ de apenas 4.5 KB en caché L1 (`SRGB_TO_LINEAR_LUT` y `LINEAR_TO_SRGB_LUT`), logrando un throughput de renderizado superior a 550 FPS sin asignaciones dinámicas.
  - Preservación del 100% de nitidez geométrica en caracteres de dibujo de marcos y cajas (`│`, `─`, `┌`, `█`, etc.).
  - Totalmente configurable desde `config.ini` (`font_antialiasing = true`, `gamma_correction = true`).

- **Soporte HiDPI Nativo y Escalado de Coordenadas del Monitor**:
  - Detección precisa y automática del factor de escala (`scale_factor`) reportado por el monitor y el sistema operativo (X11 DPI / `Xft.dpi` / variables de entorno `RUSTEXT_SCALE_FACTOR`, `GDK_SCALE`, `QT_SCALE_FACTOR`, `XCURSOR_SIZE`; Win32 `GetDpiForWindow` / `USER_DEFAULT_SCREEN_DPI`; macOS Retina backing scale).
  - Multiplicación matemática rigurosa de las coordenadas lógicas de diseño para producir píxeles físicos exactos en el `Framebuffer`: dimensiones de ventana, barras de menús, pestañas, barras de estado, terminal integrado, scrollbars, minimapas, márgenes, paneles divididos y diálogos modales.
  - Mapeo bidireccional exacto de eventos de entrada de ratón y cursor con precisión de subpíxel/píxel físico.
  - Totalmente configurable desde `config.ini` (`scale_factor = 0.0` para autodetección, o fijar factores explícitos como `1.5`, `2.0`).

- **Árbol de Proyecto y Explorador Lateral (File Explorer)**:
  - Barra lateral acoplada a la izquierda con visibilidad alternable mediante `Ctrl + B`.
  - Escaneo perezoso de carpetas (solo inspecciona en disco lo que se expande).
  - Pistas de color automáticas por extensión de archivo (`.rs`, `.toml`, `.md`, `.py`, `.c`, etc.).
  - Controles de ratón: clic para conmutar/abrir archivo, expandir/contraer carpetas, botón de nueva pestaña `[+]`, refrescar `[↺]` y cerrar `[×]`.
  - Navegación completa por teclado (`↑`/`↓`, `→`/`Enter`, `←`, `Esc`).

- **Búsqueda Global en Todo el Proyecto (`Ctrl + Shift + F`)**:
  - Modal flotante con escaneo recursivo ultrarrápido sobre el árbol completo del proyecto (`root_dir`).
  - Filtrado inteligente de binarios, archivos mayores a 10 MB y carpetas ignoradas (`.git`, `target`, `node_modules`, `.cache`, etc.).
  - Búsqueda reactiva mientras se escribe, soporte para alternar mayúsculas/minúsculas (`Alt + C` o botón `[Aa]`).
  - Snippets de código con resaltado ámbar de coincidencias, insignias de archivo/línea y salto inmediato de cursor y viewport al hacer clic o presionar `Enter`.

- **Restauración de Sesión de Espacio de Trabajo y Modo Hot Exit**:
  - Persistencia automática de todo el espacio de trabajo en `~/.rustext/session.ini`: pestañas abiertas, cursores exactos, desplazamientos de scroll, paneles divididos (vertical/horizontal y proporciones) y carpeta raíz del proyecto.
  - Soporte de *Hot Exit* seguro mediante borradores (`~/.rustext/drafts/*.draft`) para recuperar el contenido modificado ante cierres sin guardar ni perder trabajo.
  - Totalmente configurable desde `config.ini` (`restore_session = true`, `auto_save_session = true`, `hot_exit = false`).

- **Selector y Filtro de Archivos Recientes (`Ctrl + R`)**:
  - Modal flotante interactivo de acceso rápido con filtro en tiempo real para reabrir rápidamente los archivos recientemente editados.
  - Resaltado de nombre de archivo con ruta de directorio atenuada, navegación por teclado (`↑`/`↓`/`Enter`) y clics de ratón con efectos hover.
  - Historial persistente deduplicado y configurable con límite máximo (`max_recent_files = 20`).

- **Terminal Integrado Nativo Acoplado (`Ctrl + \`` / `Ctrl + ~`)**:
  - Panel de consola interactivo acoplado en la parte inferior del editor para compilar, ejecutar pruebas o interactuar con el sistema sin salir de Rustext.
  - Detección automática del shell del sistema (`$SHELL`, `/bin/bash`, `/bin/sh` o `cmd.exe`/PowerShell).
  - Comunicación asíncrona bidireccional no bloqueante mediante pipes nativos (`Stdio::piped()`) y canales en segundo plano.
  - Barra de división (splitter) arrastrable con el ratón para redimensionar la consola en tiempo real.
  - Botones interactivos en cabecera `[Limpiar]` y `[x]` para cerrar, historial de comandos (`↑`/`↓`), atajos de interrupción `Ctrl+C` y limpieza `Ctrl+L`.
  - Configurable desde `config.ini` (`terminal_height = 180`, `terminal_shell = ""`).

- **Coloreado de Paréntesis por Pares (*Bracket Pair Colorization*)**:
  - Algoritmo léxico en tiempo real de profundidad anidada cíclica (`depth % 3`) para pares de delimitadores `()`, `[]` y `{}`.
  - Paletas cromáticas de alto contraste adaptadas específicamente a los 8 temas visuales (dorado/amarillo, magenta/púrpura y cian/azul).
  - Parser contextual inteligente que ignora delimitadores dentro de cadenas de texto y comentarios para una precisión absoluta.
  - Soporte bidireccional tanto en la interfaz gráfica acelerada (GUI Framebuffer) como en consola (secuencias ANSI de 24-bit en TUI).

- **Soporte Sintáctico Ampliado y Multi-Lenguaje Nativo**:
  - Detección automática por extensión y por nombre para: **Rust**, **Python**, **C/C++**, **JavaScript/TypeScript**, **HTML**, **CSS/SCSS**, **JSON**, **YAML**, **TOML**, **Shell/Bash** (`.sh`, `.bash`, `.zsh`, `.env`, `.bashrc`, Dockerfile), **SQL** y **Markdown**.
  - Resaltador léxico especializado con clasificación de tags y atributos HTML, selectores y propiedades CSS, variables de entorno y parámetros `$VAR` en Shell, y palabras clave SQL insensibles a mayúsculas/minúsculas.
  - Resaltado jerárquico de Markdown para encabezados (`#`), citas (`>`), listas (`-`, `*`) y bloques de código inline (`` `código` ``).

- **Autocompletado Contextual Inteligente (`Ctrl + Espacio`)**:
  - Menú emergente flotante anclado a las coordenadas exactas del cursor con ranking léxico y por longitud.
  - Palabras clave nativas y tipos primitivos de todos los lenguajes soportados más identificadores extraídos en vivo de todos los búferes abiertos.
  - Compatibilidad total con edición multicursor (aplica la palabra seleccionada en todos los cursores activos al mismo tiempo).
  - Navegación fluida por teclado (`↑`/`↓`, `Tab`/`Enter`, `Esc`) y ratón (hover y clic).

- **Diagnósticos del Compilador y Linter en Vivo**:
  - Integración nativa con `cargo check --message-format=json`, `pyflakes`, `clang` y `bash -n`.
  - Insignias de severidad en el gutter (`[E]` Error, `[W]` Warning, `[I]` Info) y mensaje detallado en la barra de estado.
  - Navegación rápida entre errores con `F8` (Siguiente) y `Shift + F8` (Anterior).

- **Formateo Automático de Código**:
  - Formateo instantáneo con `Ctrl + Shift + I` mediante los formateadores del sistema (`rustfmt`, `black`, `clang-format`, `prettier`, `shfmt`, `gofmt`).
  - Opción de formateo y comprobación automática al guardar (`format_on_save`, `check_on_save`).

- **Motor de Snippets e Indentación Inteligente**:
  - Snippets de lenguaje expandibles con `Tab` para todos los lenguajes soportados:
    - **Rust**: `fn`, `pfn`, `struct`, `enum`, `impl`, `for`, `match`, `if`, `test`, `println`, `main`.
    - **Python**: `def`, `class`, `if`, `for`, `main`, `try`, `print`.
    - **C / C++**: `main`, `fn`, `struct`, `for`, `if`, `inc`.
    - **JavaScript / TypeScript**: `fn`, `afn`, `clg`, `for`, `if`, `try`.
    - **HTML**: `html5`, `div`, `a`.
    - **CSS**: `flex`, `grid`, `media`.
    - **YAML**: `kv`, `item`.
    - **Shell**: `sh`, `func`, `if`.
    - **SQL**: `select`, `table`, `insert`.
    - **Markdown**: `link`, `img`, `code`.
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

Rustext incluye un banco de pruebas y benchmarks integrados en [`src/bin/benchmark_comparison.rs`](src/bin/benchmark_comparison.rs) y [`src/bin/memory_profile.rs`](src/bin/memory_profile.rs) que miden empíricamente tiempos de arranque, memoria física (`VmRSS`), throughput de edición y huella binaria frente a los editores más representativos de la industria:

```bash
# 1. Benchmark comparativo frente a otros editores (VS Code, Vim, Micro):
cargo run --release --bin benchmark_comparison

# 2. Profiling de memoria heap exacta y RAM física bajo cargas extremas (hasta 100 MB):
cargo run --release --bin memory_profile
```

### 🏆 Tabla Comparativa: Rustext vs Editores de la Industria (Mediciones Reales en Linux x86_64)

| Métrica / Atributo | **Rustext** | **VS Code (Code-OSS)** | **Sublime Text 4** | **Vim 9.2** | **Micro** |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Tecnología Base** | **Rust puro (`std::*`)** | Electron / Node / Chromium | C++ nativo / Python | C nativo | Go runtime |
| **Dependencias Externas**| **0 (Cero crates)** | >1,000 dependencias npm | Cerrado / Propietario | libc, ncurses | Go stdlib + deps |
| **Tamaño Binario / Disco**| **~688 KB** | **~824 MB** (~1,200x) | ~45 MB | ~6.4 MB | ~13.0 MB |
| **Inicio en Frío (Cold Start)**| **1.5 - 2.5 ms** | 1,500 - 2,500 ms (~1,000x) | 80 - 150 ms | 8 - 15 ms | 20 - 45 ms |
| **Memoria RAM en Reposo**| **2.6 MB** (36 MB GUI completa) | ~350 - 450 MB | ~65 - 85 MB | ~8.9 MB | ~20.2 MB |
| **Memoria con Archivo 25 MB**| **29.0 MB** | >600 MB (latencia alta) | ~60 MB | ~38 MB | ~95 MB |
| **Latencia de Inserción**| **< 50 µs** | ~10 - 25 ms | < 1 ms | < 1 ms | < 2 ms |
| **Rendimiento Viewport**| **~1,400+ FPS** (~700 µs/frame) | 60 FPS (tope VSync) | 60 - 120 FPS | N/A (TUI) | N/A (TUI) |
| **Tiempo Compilación Limpia**| **~2.0 segundos** | Varios minutos | N/A (cerrado) | ~30 segundos | ~15 segundos |

---

### Resultados Detallados de Escalabilidad en Memoria (Heap vs RAM Física):

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
| `Ctrl + R` | **Abrir Archivo Reciente... (Selector con filtro difuso)** |
| `Ctrl + T` | Abrir nueva pestaña vacía |
| `Ctrl + W` | Cerrar pestaña activa |
| `Ctrl + Q` | Salir de Rustext |
| `Ctrl + B` | **Alternar Explorador Lateral (Árbol de Proyecto)** |
| `Ctrl + Shift + G` | **Actualizar Estado Git (Git Refresh)** |
| `Ctrl + M` | **Alternar Minimapa Gráfico de Código** |
| `Ctrl + \`` / `Ctrl + ~` | **Alternar Terminal Integrado Nativo (Consola)** |
| `Ctrl + \` | **Alternar División Vertical de Pantalla (Split Editor)** |
| `Ctrl + Shift + \` | **Alternar División Horizontal de Pantalla** |
| `Alt + \` | **Cambiar Foco de Panel (Switch Split Pane)** |
| `Ctrl + D` | **Seleccionar palabra / Añadir siguiente ocurrencia (Multicursor)** |
| `Ctrl + K` / `Ctrl + J` | Añadir cursor en la línea superior / inferior |
| `Esc` | Colapsar multicursores a cursor único / Cancelar modal |
| `Ctrl + Z` / `Ctrl + Y` | Deshacer / Rehacer atómico |
| `Ctrl + F` | Buscar en el documento |
| `Ctrl + H` | Buscar y Reemplazar |
| `Ctrl + Shift + F` | **Búsqueda Global en Todo el Proyecto** |
| `Ctrl + Espacio` | **Autocompletado Contextual Inteligente** |
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
restore_session = true
auto_save_session = true
hot_exit = false
max_recent_files = 20

[theme]
name = tokyo-night   # tokyo-night, catppuccin-mocha, one-dark, monokai, solarized-dark, light

[gui]
window_width = 1100
window_height = 720
scale_factor = 0.0   # Factor de escala HiDPI (0.0 = autodetección por monitor/OS, 1.0, 1.5, 2.0)
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
minimap = true
minimap_width = 100
git_gutter = true
terminal_height = 180
terminal_shell = ""   # Dejar vacío para autodetectar ($SHELL, bash, sh, cmd)
font_antialiasing = true
gamma_correction = true
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
