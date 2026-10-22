# 📖 Manual de Usuario y Referencia Técnica de Rustext

[![Version](https://img.shields.io/badge/version-v0.2.0-blue.svg)](Cargo.toml)
[![CI/CD](https://github.com/boleklolek/rustext/actions/workflows/ci.yml/badge.svg)](https://github.com/boleklolek/rustext/actions)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL_3.0-blue.svg)](LICENSE)
[![Lines of Code](https://img.shields.io/badge/Lines_of_Code-~52k%20LoC-informational.svg)](src/)
[![Rust](https://img.shields.io/badge/Rust-2024%20Edition-red.svg?logo=rust)](Cargo.toml)
[![Dependencies](https://img.shields.io/badge/Dependencies-0%20(Pure%20std)-brightgreen.svg)](Cargo.toml)
[![Tests](https://img.shields.io/badge/Tests-166%20passed-success.svg)](tests/)
[![Platforms](https://img.shields.io/badge/Platforms-Linux%20(Wayland%20%2F%20X11)%20%7C%20Windows%20%7C%20macOS-orange.svg)](#)
[![LSP](https://img.shields.io/badge/LSP-10%20Languages%20%7C%20Elite%20Suite-blueviolet.svg)](#)

**Rustext** es un editor de texto y código fuente nativo de ultra-baja latencia y alto rendimiento, licenciado bajo la **GNU Affero General Public License v3.0 (AGPLv3)**, escrito **100% en Rust puro utilizando exclusivamente la biblioteca estándar (`std`)**, sin dependencias externas ni crates de terceros.

Este manual ofrece una guía exhaustiva de operación, arquitectura interna, atajos de teclado, configuración, herramientas de desarrollo y los resultados del **profiling de memoria y benchmarks de rendimiento**.

---

## 📑 Tabla de Contenidos

1. [Filosofía y Arquitectura](#1-filosofía-y-arquitectura)
2. [Instalación y Compilación](#2-instalación-y-compilación)
3. [Guía de la Interfaz Gráfica (Desktop GUI) y TUI](#3-guía-de-la-interfaz-gráfica-desktop-gui-y-tui)
   - [3.1 Diálogo de Protección de Cambios sin Guardar](#31-diálogo-de-protección-de-cambios-sin-guardar)
   - [3.2 Restauración de Sesión, Modo Hot Exit y Archivos Recientes](#32-restauración-de-sesión-modo-hot-exit-y-archivos-recientes)
   - [3.3 Terminal Integrado Nativo Acoplado](#33-terminal-integrado-nativo-acoplado-ctrl---ctrl-)
   - [3.4 Rasterizador de Fuentes Tipográficas y Corrección Gamma (sRGB IEC 61966-2-1)](#34-rasterizador-de-fuentes-tipográficas-y-corrección-gamma-srgb-iec-61966-2-1)
   - [3.5 Soporte HiDPI Nativo y Escalado de Coordenadas del Monitor](#35-soporte-hidpi-nativo-y-escalado-de-coordenadas-del-monitor)
   - [3.6 Font Shaping, Hinting, Antialiasing Analítico y Efectos Visuales](#36-font-shaping-hinting-antialiasing-analítico-y-efectos-visuales)
   - [3.7 Selector de Fuentes Interactivo y Sistema Tipográfico Avanzado](#37-selector-de-fuentes-interactivo-y-sistema-tipográfico-avanzado)
   - [3.8 Backend Nativo Wayland para Linux (wl_shm + xdg_shell + memfd_create)](#38-backend-nativo-wayland-para-linux-wl_shm--xdg_shell--memfd_create)
   - [3.9 Motor Vectorial TrueType (.ttf), Anti-Aliasing Subpíxel y Caché Tipográfica](#39-motor-vectorial-truetype-ttf-anti-aliasing-subpíxel-y-caché-tipográfica)
   - [3.10 Identidad Visual Oficial: "The Faceted Ribbon R" y Desktop Launcher](#310-identidad-visual-oficial-the-faceted-ribbon-r-y-desktop-launcher)
4. [Árbol de Proyecto, Espacios de Trabajo y Gestión de Proyectos](#4-árbol-de-proyecto-espacios-de-trabajo-y-gestión-de-proyectos)
   - [4.1 Árbol de Proyecto y Explorador Lateral (Sidebar)](#41-árbol-de-proyecto-y-explorador-lateral)
   - [4.2 Espacios de Trabajo y Archivos .rustext-workspace](#42-espacios-de-trabajo-y-archivos-rustext-workspace)
   - [4.3 Aislamiento de Sesiones por Proyecto (session_hash.ini)](#43-aislamiento-de-sesiones-por-proyecto-session_hashini)
   - [4.4 Conmutación Ágil y Proyectos Recientes](#44-conmutación-ágil-y-proyectos-recientes)
5. [División de Vistas (Split Editor) y Minimapa Gráfico](#5-división-de-vistas-split-editor-y-minimapa-gráfico)
6. [Integración Git en Vivo (Git Gutter & Control de Versiones)](#6-integración-git-en-vivo-git-gutter--control-de-versiones)
7. [Motor de Edición, Multicursor, Soft Wrap y Code Folding](#7-motor-de-edición-multicursor-soft-wrap-y-code-folding)
   - [7.1 Operaciones Multicursor](#71-operaciones-multicursor)
   - [7.2 Sincronización y Desplazamiento Vectorial](#72-sincronización-y-desplazamiento-vectorial)
   - [7.3 Transacciones Atómicas de Deshacer / Rehacer](#73-transacciones-atómicas-de-deshacer--rehacer)
   - [7.4 Auto-Cierre y Envoltorio de Pares (Auto-Surround)](#74-auto-cierre-y-envoltorio-de-pares-auto-surround)
   - [7.5 Ajuste de Línea Envolvente (Soft Wrap / Word Wrap, Alt+Z)](#75-ajuste-de-línea-envolvente-soft-wrap--word-wrap-altz)
   - [7.6 Plegado de Código Estructurado (Code Folding, Alt+[, Alt+0, Alt+J)](#76-plegado-de-código-estructurado-code-folding-alt-alt0-altj)
8. [Autocompletado Contextual Inteligente](#8-autocompletado-contextual-inteligente)
9. [Búsqueda Global en Todo el Proyecto](#9-búsqueda-global-en-todo-el-proyecto)
10. [Sistema de Snippets, Sintaxis y Bracket Pair Colorization](#10-sistema-de-snippets-sintaxis-y-bracket-pair-colorization)
11. [Diagnósticos en Vivo, Ondulaciones Wavy Squiggles y Cliente LSP](#11-diagnósticos-en-vivo-ondulaciones-wavy-squiggles-y-cliente-lsp)
12. [Búsqueda Rápida de Archivos (Ctrl+P), Paleta de Comandos y Atajos](#12-búsqueda-rápida-de-archivos-ctrlp-paleta-de-comandos-y-atajos)
13. [Sistema de Configuración (`config.ini`)](#13-sistema-de-configuración-configini)
14. [Sistema de Plugins, Temas y Extensiones](#14-sistema-de-plugins-temas-y-extensiones)
15. [Perfil de Memoria y Benchmarks Comparativos](#15-perfil-de-memoria-y-benchmarks-comparativos)
16. [Preguntas Frecuentes y Solución de Problemas](#16-preguntas-frecuentes-y-solución-de-problemas)
17. [Motor de Cajas Flexibles (Flexbox) y Paradigmas de UI (Reactivo vs. Retained)](#17-motor-de-cajas-flexibles-flexbox-y-paradigmas-de-ui-reactivo-vs-retained)
18. [Auditoría Integral de Seguridad, Modelo de Amenazas y Directrices Defensivas](#18-auditoría-integral-de-seguridad-modelo-de-amenazas-y-directrices-defensivas)
19. [Licencia y Términos de Distribución (GNU AGPLv3)](#19-licencia-y-términos-de-distribución-gnu-agplv3)

---

## 1. Filosofía y Arquitectura

Rustext fue diseñado bajo tres principios inquebrantables:

1. **Cero Dependencias de Terceros (`Cargo.toml` vacío)**:
   - Todo el editor está construido únicamente sobre `std::*`.
   - No utiliza crates como `syn`, `crossterm`, `winit`, `serde` o `regex`.
   - Garantiza tiempos de compilación de **~1.5 a 3 segundos**, tamaño binario mínimo (**~600 KB**) y cero vulnerabilidades de cadena de suministro.

2. **Estructura de Datos Piece Table**:
   - El búfer de edición está respaldado por [`PieceTable`](file:///home/bolek/Proyectos/rustext/src/core/piece_table.rs).
   - El contenido cargado inicialmente se almacena en una cadena inmutable `Original`. Todas las adiciones se realizan en un búfer append-only `Add`.
   - Las ediciones se representan mediante fragmentos (`Piece` de solo 32 bytes) que apuntan a rangos en los búferes.
   - Las inserciones y eliminaciones no copian bloques enteros de memoria, permitiendo editar archivos masivos con latencia mínima.

3. **Motor Gráfico de Software y Ventanas Nativas Multiplataforma**:
   - **Linux**: Soporte dual nativo para **Wayland (`wl_shm` + `xdg_shell` + `memfd`)** y **X11 (`libX11.so`)**. Detección automática del socket Wayland mediante comunicación binaria directa sobre `UnixStream` sin dependencias externas, con fallback instantáneo y transparente a X11.
   - **Windows**: Creación de ventana Win32 nativa (`user32.dll`, `gdi32.dll`) y dibujo mediante `StretchDIBits`.
   - **macOS**: Creación de ventana Cocoa / AppKit a través del runtime estándar de Objective-C (`objc_msgSend`).
   - **Renderizador de Software**: Framebuffer ARGB de 32 bits con primitivas 2D aceleradas, esquinas redondeadas anti-aliased, sombras proyectadas suaves, píldoras informativas, rasterizador de fuentes con anti-aliasing subpixel y composición gamma físicamente correcta (sRGB IEC 61966-2-1 en luz lineal).

---

## 2. Instalación y Compilación

### Requisitos
- Compilador de Rust (versión estable 1.80 o superior / Rust 2024).
- Sistema Operativo: Linux (X11 / Wayland), Windows 10/11 o macOS.

### Compilar desde el Código Fuente

```bash
# Clonar el repositorio
git clone https://github.com/boleklolek/rustext.git
cd rustext

# Compilar en modo Release con optimizaciones máximas
cargo build --release
```

El binario ejecutable se generará en `./target/release/rustext`.

### Modos de Ejecución

```bash
# 1. Modo GUI de Escritorio (Ventana Gráfica Nativa):
./target/release/rustext --gui
./target/release/rustext -g src/main.rs

# 2. Modo Terminal (TUI) para servidores o SSH:
./target/release/rustext
./target/release/rustext src/main.rs

# 3. Ejecutar pruebas unitarias e integración:
cargo test

# 4. Ejecutar el Benchmark Comparativo con Editores:
cargo run --release --bin benchmark_comparison

# 5. Ejecutar el Profile de Memoria y Benchmarks Extremos:
cargo run --release --bin memory_profile
```

---

## 3. Guía de la Interfaz Gráfica (Desktop GUI) y TUI

La interfaz gráfica de Rustext está diseñada con una estética moderna, limpia y minimalista inspirada en Tokyo Night y Catppuccin:

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

### Componentes de la Interfaz:
- **Barra de Menús Superior**: Desplegables accesibles mediante clic o atajos `Alt+A` (Archivo), `Alt+E` (Editar), `Alt+V` (Ver), etc.
- **Barra de Pestañas (Tab Bar)**: Muestra los búferes abiertos, botón de cerrar `×`, botón de nueva pestaña `[+]` y advertencia de cambios sin guardar `[+]`.
- **Barra Lateral (File Explorer)**: Árbol interactivo de archivos y carpetas del proyecto a la izquierda (`Ctrl + B`).
- **Gutter y Líneas de Código**: Números de línea alineados, indicador de advertencia/error de diagnóstico y lienzo de código.
- **Minimapa Gráfico de Código**: Columna lateral con micro-renderizado del código y deslizador translúcido de viewport para navegación visual instantánea (`Ctrl + M`).
- **Scrollbar Vertical Interactivo**: Pista de desplazamiento con pulgar redondeado arrastrable y soporte para clics de página arriba/abajo.
- **Paneles Divididos (Split Editor)**: División vertical (`Ctrl + \`) u horizontal (`Ctrl + Shift + \`) con barra separadora (splitter) arrastrable y paneles independientes.
- **Barra de Estado Inferior**: Píldoras con modo de edición, nombre de archivo, posición del cursor, codificación UTF-8, lenguaje, contador de cursores e indicador de panel activo (`Panel: 1/2`).

---

### 3.1 Diálogo de Protección de Cambios sin Guardar

Rustext incorpora un sistema integral de prevención contra pérdidas involuntarias de datos que supervisa el estado de modificación (`is_dirty`) de todos los búferes abiertos en tiempo de ejecución:

1. **Puntos de Intercepción Segura**:
   - **Cierre de Pestaña**: Al cerrar la pestaña activa con `Ctrl + W`, la opción de menú `Archivo > Cerrar Pestaña`, o al hacer clic sobre el botón `×` en la barra de pestañas.
   - **Salida del Editor**: Al intentar salir de la aplicación mediante `Ctrl + Q`, la opción de menú `Archivo > Salir`, la paleta de comandos o el botón de cierre nativo de la ventana del sistema operativo (`[X]` en X11/Win32/macOS).
   - **Cancelación Nativa de Cierre del SO**: En modo GUI, el bucle de eventos intercepta la señal de cierre de la ventana (`WM_DELETE_WINDOW` / `WM_CLOSE`), congela el ciclo de destrucción mediante `cancel_close()` y presenta el diálogo modal sobre el lienzo gráfico.

2. **Acciones Disponibles en el Diálogo**:
   - **[Guardar] / [Guardar Todo]**: Guarda el archivo modificado inmediatamente en el disco. Si el archivo es nuevo y aún no tiene ruta asignada (`[Sin título]`), despliega automáticamente el diálogo interactivo de *Guardar Como*. Si la acción es salir y hay múltiples archivos modificados, guarda todos secuencialmente antes de salir.
   - **[No guardar] / [Descartar Todo]**: Descarta los cambios en memoria y procede de inmediato a cerrar la pestaña o terminar la sesión de Rustext.
   - **[Cancelar]**: Aborta de forma segura la intención de cerrar o salir, restaurando el foco y dejando todos los cursores, selecciones y contenidos intactos.

3. **Ergonomía de Teclado y Ratón**:
   - **Atajos Nemónicos Instantáneos**:
     - `G` o `S`: Guardar inmediatamente.
     - `D` o `N`: Descartar / No guardar inmediatamente.
     - `Esc` o `C`: Cancelar y volver al editor.
   - **Navegación por Teclas**: Use `←` / `→` o `Tab` / `Shift + Tab` para alternar el foco entre los 3 botones, y `Enter` o `Espacio` para activarlo.
   - **Ratón en GUI**: Efectos visuales de iluminación (*hover*) al pasar sobre los botones, clic directo para confirmar y botón `×` superior derecho para cancelar.
   - **Modo Terminal TUI**: Muestra el distintivo `[CONFIRM]` en la barra de estado y los botones interactivos formateados en la línea de mensajes inferior.

---

### 3.2 Restauración de Sesión, Modo Hot Exit y Archivos Recientes

Rustext implementa un subsistema robusto de persistencia de espacio de trabajo que garantiza que su flujo de trabajo nunca se interrumpa:

1. **Persistencia Automática de Sesión (`session.ini`)**:
   - Al cerrar Rustext o salir, si `auto_save_session = true`, el estado completo se escribe en `~/.rustext/session.ini`.
   - **Información preservada**:
     - Lista secuencial de todas las pestañas abiertas.
     - Coordenadas de cursor individuales (línea y columna) por pestaña.
     - Posición de desplazamiento vertical (`scroll_row`) y horizontal (`scroll_col`).
     - Estado de división de pantalla (*Split Editor*): dirección (vertical/horizontal), pestaña secundaria y proporción exacta del divisor (`split_ratio`).
     - Visibilidad y carpeta raíz del árbol del explorador lateral.
     - Índice de pestaña y panel activo.
   - Al volver a iniciar `rustext` sin argumentos, si `restore_session = true`, el editor recrea idénticamente el espacio de trabajo tal como lo dejó.

2. **Modo Hot Exit y Borradores de Recuperación (`~/.rustext/drafts/*.draft`)**:
   - Con `hot_exit = true`, los búferes que contengan modificaciones sin guardar se salvan atómicamente como borradores temporales en `~/.rustext/drafts/tab_{idx}.draft` al salir del editor.
   - Al reabrir Rustext, el contenido de los borradores se recarga en memoria con el indicador de archivo modificado (`*` / `[+]`) y la ruta original asignada.
   - Los borradores se limpian automáticamente tras guardar el archivo en disco o tras descartar los cambios.

3. **Selector Flotante de Archivos Recientes (`Ctrl + R`)**:
   - Acceda mediante el atajo de teclado `Ctrl + R` o el menú `Archivo > Abrir Archivo Reciente...`.
   - Despliega una ventana flotante con filtro difuso en tiempo real.
   - Los nombres de archivo se destacan en blanco brillante y las rutas completas en gris atenuado.
   - Navegue con `↑` / `↓` y presione `Enter` para abrir de inmediato el archivo, o haga clic directo con el ratón.
   - El historial de archivos recientes se deduplica de manera inteligente y su longitud máxima es configurable mediante `max_recent_files = 20`.

---

### 3.3 Terminal Integrado Nativo Acoplado (`Ctrl + \`` / `Ctrl + ~`)

Rustext incluye un **Terminal Integrado Nativo** acoplado en la parte inferior de la ventana, permitiendo compilar proyectos, ejecutar suites de pruebas, comandos de control de versiones o interactuar con el sistema operativo sin abandonar el editor:

1. **Arquitectura y Proceso Asíncrono**:
   - **Cero Dependencias Externas**: Implementado utilizando exclusivamente las primitivas estándar `std::process::{Command, Stdio, Child}`, hilos nativos (`std::thread`) y canales mpsc (`std::sync::mpsc`).
   - **Detección Automática de Shell**: Inspecciona dinámicamente la variable de entorno `$SHELL`, seleccionando `/bin/bash` o `/bin/sh` en Linux/macOS, o `cmd.exe`/PowerShell en Windows. Permite especificar un shell personalizado en `config.ini` (`terminal_shell = "/bin/zsh"`).
   - **Lectura Bidireccional Asíncrona**: Hilos dedicados en segundo plano capturan continuamente `stdout` y `stderr` sin bloquear la fluidez gráfica del editor a 60 FPS.
   - **Limpieza de Códigos ANSI**: Filtro inteligente de secuencias de escape ANSI para presentar texto limpio y legible.

2. **Controles e Interacción de Usuario**:
   - **Alternar Visibilidad**: Presione `Ctrl + \`` o `Ctrl + ~` en cualquier momento, o seleccione `Ver > Terminal Integrado` en el menú.
   - **Foco Inteligente**: Al abrirse, el foco se sitúa de inmediato en la línea de comando del terminal (`❯ `). Para devolver el foco al búfer del documento activo, presione `Esc` o haga clic en el lienzo de código.
   - **Historial de Comandos**: Navegue por los comandos previamente ejecutados con las teclas `↑` y `↓`.
   - **Atajos Nemónicos de Terminal**:
     - `Enter`: Envía el comando actual al stdin del proceso hijo y lo registra en el historial. Comandos como `clear` o `cls` limpian inmediatamente la pantalla.
     - `Ctrl + C`: Envía interrupción visual y reinicia el prompt actual.
     - `Ctrl + L`: Limpia de inmediato el búfer de salida del terminal.
     - `PageUp` / `PageDown`: Desplaza el historial de salida hacia arriba o hacia abajo.
   - **Desplazamiento con Ratón**: La rueda del ratón realiza scroll automático por las líneas de salida al estar situada sobre el panel del terminal.
   - **Redimensionamiento en Vivo (Splitter)**: La barra superior divisoria puede arrastrarse fluidamente hacia arriba o abajo con el ratón para ajustar la altura del terminal (entre 60 px y la mitad de la ventana).
   - **Botones de Cabecera**:
     - `[Limpiar]`: Vacía el búfer de scrollback y restablece la vista.
     - `[x]`: Oculta el panel de terminal restaurando todo el espacio a la vista de código.

3. **Opciones de Configuración (`config.ini`)**:
   ```ini
   [gui]
   terminal_height = 180       # Altura inicial en píxeles del panel de terminal
   terminal_shell = ""         # Ruta a shell específico (dejar vacío para autodetección)
   ```

---

### 3.4 Rasterizador de Fuentes Tipográficas y Corrección Gamma (sRGB IEC 61966-2-1)

El renderizado tipográfico sobre fondos oscuros (típico de temas modernos como Tokyo Night o Catppuccin) presenta un desafío físico fundamental: **la no linealidad de la respuesta lumínica de las pantallas**.

#### 1. El Problema del Blending Ingenuo y el "Font Thinning":
- La inmensa mayoría de monitores y pantallas operan en el espacio de color **sRGB**, el cual posee una curva de transferencia no lineal ($\gamma \approx 2.2$).
- En un rasterizador ingenuo que realiza composición alfa lineal directamente sobre los valores numéricos sRGB:
  $$\text{dst}_{\text{srgb}} = \text{dst} \times (1 - \alpha) + \text{fg} \times \alpha$$
- Si se dibuja un trazo blanco (`fg = 255`) sobre un fondo negro (`dst = 0`) con un píxel de borde al 50% de cobertura ($\alpha = 128$), el resultado numérico en sRGB es $128$.
- Sin embargo, un monitor sRGB que recibe un valor de $128$ emite en realidad **apenas el 21.8% de los fotones** $\left((128/255)^{2.2} \approx 0.218\right)$, no el 50%.
- **Consecuencia**: El texto se percibe excesivamente delgado (*font thinning*), pierde cuerpo o peso óptico (*stem weight*), y aparecen halos oscuros alrededor de curvas y diagonales.

#### 2. La Solución Física de Rustext: Luz Lineal según IEC 61966-2-1
Rustext implementa una canalización matemática estricta de composición en espacio de luz lineal:
1. **Des-gamma (sRGB a Lineal)**: Transforma los valores de color de entrada a energía lumínica física real utilizando la función de transferencia sRGB de la norma IEC 61966-2-1:
   $$C_{\text{linear}} = \begin{cases} \dfrac{C_{\text{srgb}}}{12.92} & C_{\text{srgb}} \le 0.04045 \\ \left(\dfrac{C_{\text{srgb}} + 0.055}{1.055}\right)^{2.4} & C_{\text{srgb}} > 0.04045 \end{cases}$$
2. **Composición de Fotones**: Realiza el cálculo de cobertura alfa en espacio lineal:
   $$\text{dst}_{\text{linear}} = \text{dst}_{\text{linear}} \times (1 - \alpha) + \text{fg}_{\text{linear}} \times \alpha$$
3. **Re-gamma (Lineal a sRGB)**: Convierte el flujo luminoso resultante de vuelta a la codificación sRGB para el framebuffer:
   $$C_{\text{srgb}} = \begin{cases} 12.92 \times C_{\text{linear}} & C_{\text{linear}} \le 0.0031308 \\ 1.055 \times C_{\text{linear}}^{1/2.4} - 0.055 & C_{\text{linear}} > 0.0031308 \end{cases}$$
Con esta corrección, un 50% de luz lineal mapea a un valor sRGB de ~186, emitiendo exactamente la mitad de los fotones y preservando fielmente la densidad tipográfica original del glifo.

#### 3. Tablas Precomputadas $\mathcal{O}(1)$ de Ultra-Baja Latencia (Caché L1)
Para evitar calcular potencias en punto flotante (`powf`) en cada píxel renderizado, el módulo [`src/gfx/gamma.rs`](src/gfx/gamma.rs) precomputa tablas estáticas inmutables en tiempo de compilación (`.rodata`):
- `SRGB_TO_LINEAR_LUT: [u16; 256]`: Mapea directamente valores sRGB de 8 bits (`0..=255`) a 12 bits de luminancia lineal (`0..=4095`). Tamaño: 512 bytes.
- `LINEAR_TO_SRGB_LUT: [u8; 4096]`: Mapea valores lineales de 12 bits (`0..=4095`) de vuelta a sRGB de 8 bits (`0..=255`). Tamaño: 4096 bytes (4 KB).
- **Huella de memoria total**: Menos de 4.6 KB, cabiendo completamente en la memoria caché L1 de datos de cualquier CPU moderna. Esto garantiza tiempos de acceso instantáneos en $\mathcal{O}(1)$ y cero fallos de caché (*cache misses*).

#### 4. Suavizado Morfológico Espacial y Escalado Bilineal (`FontRasterizer`)
El rasterizador tipográfico en [`src/gfx/font.rs`](src/gfx/font.rs) aporta:
- **Cálculo de Cobertura Espacial**: Analiza la vecindad 2D de píxeles para generar gradientes de cobertura suaves en contornos curvos (ej. 'o', 'C') y diagonales (ej. 'A', 'v', '/').
- **Preservación Geométrica de Caracteres de Caja (Box-Drawing)**: Los caracteres de marcos, bordes y barras (`│`, `─`, `┌`, `└`, `█`, etc.) son detectados automáticamente (`is_box_drawing`) y se renderizan con opacidad total y bordes rectos nítidos, evitando emborronamientos en la interfaz.
- **Escalador Bilineal Continuo**: Para factores de ampliación (`font_scale >= 2`), `rasterize_glyph_scaled` muestrea bilinealmente las matrices de cobertura a resolución continua, eliminando por completo los escalones dentados (*staircase effect*).
- **Caché Estática Zero-Allocation**: Los glifos ASCII y caracteres especiales frecuentes se almacenan pre-rasterizados en `ASCII_AA_CACHE` mediante `OnceLock`, logrando más de **550 FPS** de rendimiento continuo en CPU pura.

---

### 3.5 Soporte HiDPI Nativo y Escalado de Coordenadas del Monitor

Rustext implementa una arquitectura nativa de escalado de coordenadas para pantallas de alta densidad de píxeles (HiDPI / 4K / Apple Retina).

#### 1. Detección Multiplataforma del Factor de Escala
El subsistema de ventanas detecta de forma automática y precisa el factor de escala que reporta el monitor mediante llamadas al sistema operativo y variables de entorno estándar:
- **Linux (X11 / Wayland)**:
  1. Detección física por DPI del servidor X11: calcula el DPI promediado horizontal y verticalmente a partir de la resolución en píxeles y dimensiones físicas en milímetros reportadas por la pantalla:
     $$\text{DPI} = \frac{1}{2} \left( \frac{\text{width\_px} \times 25.4}{\text{width\_mm}} + \frac{\text{height\_px} \times 25.4}{\text{height\_mm}} \right), \quad \text{scale} = \frac{\text{DPI}}{96.0}$$
  2. Inspección de recursos `Xft.dpi` mediante `XGetDefault(display, "Xft", "dpi")`.
  3. Variables de entorno estándar del escritorio: `RUSTEXT_SCALE_FACTOR`, `GDK_SCALE`, `QT_SCALE_FACTOR` y `XCURSOR_SIZE` (ej. cursor de 48px mapea a factor 2.0x).
- **Windows (Win32)**:
  - Invoca dinámicamente `GetDpiForWindow` o `GetDpiForSystem` desde `user32.dll` comparado contra `USER_DEFAULT_SCREEN_DPI (96)`, o variable `RUSTEXT_SCALE_FACTOR`.
- **macOS (AppKit / Cocoa)**:
  - Consulta `backingScaleFactor` de la ventana activa (2.0x para pantallas Retina nativas) o variable de entorno `RUSTEXT_SCALE_FACTOR`.

#### 2. Multiplicación Rigurosa de Coordenadas Lógicas
Para garantizar nitidez visual y proporciones perfectas en pantallas 4K y pantallas Retina sin alterar el código de diseño lógico:
- Las dimensiones de referencia lógicas se multiplican por el factor de escala:
  $$\text{coord}_{\text{física}} = \text{round}(\text{coord}_{\text{lógica}} \times \text{scale\_factor})$$
- Componentes escalados matemáticamente:
  - **Ventana**: Dimensiones iniciales multiplicadas por el factor de escala.
  - **Menú y Pestañas**: Altura de cabeceras, márgenes, padding y anchos de columna.
  - **Barra de Estado y Terminal**: Altura del panel, tamaño del splitter de redimensionamiento y botones `[Limpiar]` / `[x]`.
  - **Scrollbars y Minimapas**: Ancho de banda de desplazamiento (`scrollbar_width`), ancho del minimapa (`minimap_width`) y pulgares deslizantes.
  - **Márgenes y Diálogos Flotantes**: Búsqueda Rápida de Archivos (`Ctrl+P`), Paleta de Comandos (`Ctrl+Shift+P`), Búsqueda Global (`Ctrl+Shift+F`), Buscar y Reemplazar (`Ctrl+H`), Selector de Archivos Recientes (`Ctrl+R`), Diálogo de confirmación de cambios no guardados y Ventanas de Extensiones y Acerca de.
  - **Tipografía y Cursores**: Tamaño del glifo tipográfico (`char_width = 8 * font_scale`, `char_height = 16 * font_scale`), separación interlineal (`line_spacing`) y grosor del cursor.

#### 3. Mapeo Bidireccional de Coordenadas de Entrada (Hit-Testing)
Los clics del ratón, desplazamientos y detecciones de hover se calculan con exactitud de píxel físico:
- Cada widget y modal utiliza las mismas fórmulas escaladas tanto en la fase de renderizado en el `Framebuffer` como en la fase de hit-testing de eventos (`app.rs`).
- La función `renderer.to_logical_coord(physical_px)` permite traducir posiciones de subpíxel/píxel de vuelta al espacio de coordenadas lógicas cuando se requiere.

#### 4. Opciones de Configuración (`config.ini`)
```ini
[gui]
# Factor de escala HiDPI: 0.0 para autodetección por monitor/OS; o fijar valores como 1.0, 1.5, 2.0
scale_factor = 0.0
```

---

### 3.6 Font Shaping, Hinting, Antialiasing Analítico y Efectos Visuales

Rustext incorpora un subsistema de renderizado tipográfico y geométrico avanzado de nivel profesional, escrito íntegramente en Rust estándar sin bibliotecas externas.

#### 1. Font Shaping y Ligaduras de Programación (`src/gfx/shaping.rs`)
- **Ligaduras Continuas de Código**: Detección léxica y rasterizado vectorizado continuo para los operadores de programación más frecuentes:
  - Flechas: `->` (flecha delgada continua), `=>` (flecha gruesa fat-arrow doble)
  - Comparadores: `==` (doble barra extendida), `!=` (desigualdad con barra transversal), `===` (triple barra), `!==` (triple barra tachada), `<=` (menor o igual), `>=` (mayor o igual)
  - Asignación y bloques: `:=` (walrus operator con dos puntos centrados), `&&` (doble ampersand balanceado), `||` (barras verticales paralelas), `::` (resolución de ámbito alineada)
  - Rangos y comentarios: `..` (rango doble punto), `...` (puntos suspensivos en 3 celdas), `/*` y `*/` (apertura y cierre de bloque), `++` e `--` (incremento y decremento)
- **Preservación Estricta de la Cuadrícula Monoespaciada**: Cada ligadura ocupa exactamente $N \times W$ píxeles (donde $N$ es la cantidad de caracteres originales y $W$ el ancho de columna), garantizando que el cursor, selecciones multicursor y columnas de código no se desalineen.
- **Composición de Marcas Diacríticas Unicode**: Combina caracteres base con marcas diacríticas (`\u{0300}`..=`\u{036F}`) en glifos compuestos directos (`á`, `é`, `í`, `ó`, `ú`, `ñ`, `ü`, etc.).

#### 2. Alineación al Píxel (Font Hinting - `src/gfx/hinting.rs`)
- **Grid-Fitting y Stem Snapping**: Analiza la distribución de energía del trazo en el glifo y bloquea los tallos verticales y travesaños horizontales a la cuadrícula de píxeles enteros. Evita el desenfoque producido cuando un trazo cae al 50% entre dos columnas contiguas.
- **Alineación de Línea Base**: Fija la línea base tipográfica en coordenadas enteras exactas del framebuffer.
- **Control de Contraste Mínimo**: Refuerza los píxeles centrales de trazos delgados para evitar que se desvanezcan en fondos oscuros.
- **Niveles Configurables**:
  - `none`: Cobertura continua en coma flotante.
  - `slight`: Alineación de línea base y métricas verticales.
  - `medium`: Alineación de línea base y ajuste de tallos verticales (Recomendado).
  - `full`: Bloqueo completo de cuadrícula en ambos ejes.

#### 3. Antialiasing Geométrico Analítico (Exact SDF - `src/gfx/analytic_aa.rs`)
- **Cobertura Subpíxel Exacta sin Supersampling**:
  - **Líneas Analíticas (Xiaolin Wu)**: Trazado de líneas con coordenadas y espesores continuos (`width: f64`), calculando la distancia perpendicular exacta de cada píxel al segmento.
  - **Círculos y Anillos**: Evaluación analítica de cobertura $\text{clamp}(0.5 + r - d, 0.0, 1.0)$, produciendo bordes 100% lisos sin dientes de sierra.
  - **Rectángulos Redondeados por SDF 2D Box**: Cálculo exacto mediante campo de distancia con signo:
    $$q = (|p - c| - (\text{half\_size} - r))^+, \quad d = \|q\| + \min(\max(q_x, q_y), 0) - r$$
    $$\text{cobertura} = \text{clamp}(0.5 - d, 0.0, 1.0)$$
    Proporciona esquinas redondeadas matemáticas perfectas con antialiasing analítico para modales, chips de sintaxis y pestañas.
  - **Triángulos Analíticos**: Cobertura evaluada mediante distancias de borde con signo.

#### 4. Sombras Suaves y Desenfoque Gaussiano $\mathcal{O}(1)$ (`src/gfx/blur.rs`)
- **Desenfoque Separable en Tiempo Constante**:
  - Por el Teorema del Límite Central, 3 pasadas horizontales y verticales de filtro de caja (*box blur*) convergen exactamente a un desenfoque Gaussiano 2D.
  - Implementación con ventana móvil acumulativa: **coste computacional $\mathcal{O}(1)$ por píxel** sin importar la amplitud del radio de desenfoque.
- **Efecto Frosted Glass (Acrílico Translúcido)**:
  - Extrae y desenfoca la región inferior del framebuffer, aplica un tinte acrílico translúcido y añade un destello especular superior de 1 píxel para una estética moderna de profundidad de escritorio.
- **Sombras Suaves Físicamente Atenuadas**:
  - Genera una máscara SDF de la forma del widget, aplica el desenfoque Gaussiano separable y compone la sombra con atenuación y desplazamiento configurable.

#### 5. Curvas de Animación y Físicas de Resorte (`src/gfx/animation.rs`)
- **Familia de Funciones Easing**:
  - `Linear`, `EaseInQuad`, `EaseOutQuad`, `EaseInOutQuad`, `EaseInCubic`, `EaseOutCubic`, `EaseInOutCubic`, `EaseInQuart`, `EaseOutQuart`, `EaseInExpo`, `EaseOutExpo`, `EaseOutBack`, `EaseOutBounce`, `EaseOutElastic`.
- **Solver Cubic-Bézier estilo CSS**:
  - Curvas paramétricas `cubic-bezier(x1, y1, x2, y2)` con resolución iterativa de raíces mediante el método de Newton-Raphson y subdivisión binaria de respaldo.
- **Oscilador Físico de Resorte (Spring Physics)**:
  - Simula oscilaciones dinámicas naturales ($m \ddot{x} + c \dot{x} + k x = 0$) para animaciones fluidas del cursor y deslizamiento de scroll.

#### 6. Percepción de Bordes y Contraste Adaptativo (`src/gfx/edge_perception.rs`)
- **Ley de Weber-Fechner y Norma ITU-R BT.709**:
  - La percepción visual humana del contraste de bordes es no lineal. El motor calcula la luminancia relativa del texto y el fondo para modular la curva S de contraste en los bordes.
  - Neutraliza la ilusión de irradiación visual (engrosamiento percibido o adelgazamiento de texto blanco sobre fondos oscuros como Tokyo Night o Catppuccin).
- **Filtro Unsharp Masking Laplaciano**:
  - Operador espacial diferencial que realza suavemente las altas frecuencias en los contornos tipográficos y elementos de interfaz.

#### 7. Configuración en `config.ini`
```ini
[gui]
font_ligatures = true     # Activar ligaduras de programación continuas (->, =>, !=, ==, etc.)
font_hinting = medium     # Nivel de alineación a la rejilla: none, slight, medium, full
smooth_shadows = true     # Sombras suaves gaussianas y efecto frosted glass acrílico
animation_enabled = true  # Activar animaciones y curvas de aceleración (easing)
edge_perception = true    # Adaptación de contraste perceptual de bordes (Weber-Fechner)
```

### 3.7 Selector de Fuentes Interactivo y Sistema Tipográfico Avanzado

Rustext incorpora un completo subsistema tipográfico y un diálogo modal interactivo de selección de fuentes (`Ctrl + Shift + T` o menú `Preferencias > Tipografía...`) que permite personalizar al vuelo la experiencia visual de lectura y programación sin dependencias externas ni consumo de recursos.

#### 1. Cinco Familias Tipográficas Monoespaciadas Integradas (`FontFamily`)
El motor cuenta con 5 fuentes diseñadas minuciosamente para programación, cada una con un carácter e identidad visual únicos:
- **`Standard Mono` (`standard`)**: Familia equilibrada, neutra y limpia, optimizada para sesiones de desarrollo prolongadas con mínima fatiga visual.
- **`Terminus Console` (`terminus`)**: Alto contraste geométrico, cero con barra oblicua interior (`0` tachado), `1` con pedestal y gancho superior, `l` con gancho inferior curvado, e `I` mayúscula con serifas transversales simétricas; rescata la estética clásica de consolas UNIX/Linux.
- **`Modern Coding Sans` (`modern`)**: Diseño neo-grotesco contemporáneo de aperturas abiertas, cero con punto central distintivo, letras curvas suaves (`a`, `g`, `t`, `f`) y trazado minimalista.
- **`Typewriter Slab` (`typewriter`)**: Tipografía de estilo mecanográfico editorial con serifas gruesas cuadradas (*slab serifs*) en letras clave (`A`, `H`, `M`, `i`, `l`, `r`).
- **`Retro Pixel 8-Bit` (`retro`)**: Estilo arcade retro y computación clásica de los años 80; desactiva el suavizado analítico forzando píxeles nítidos y bloques rectos de alto impacto.

#### 2. Escalado Bilineal Continuo y Tamaños de Fuente (`Font Size`)
A diferencia de los rasterizadores de bitmap rígidos, Rustext implementa **muestreo subpíxel bilineal continuo**:
$$\text{ancho\_celda} = \text{round}\left(\text{tamaño} \cdot \frac{8.0}{12.0}\right) \cdot \text{font\_scale}, \quad \text{alto\_celda} = \text{round}\left(\text{tamaño} \cdot \frac{16.0}{12.0}\right) \cdot \text{font\_scale}$$
- Tamaños estándar seleccionables: **`10, 11, 12, 14, 16, 18, 20, 24 pt`**.
- Mapea cualquier tamaño a la cuadrícula de cobertura continua mediante interpolación bilineal:
  $$u = \frac{x + 0.5}{W} \cdot 8.0 - 0.5, \quad v = \frac{y + 0.5}{H} \cdot 16.0 - 0.5$$
  proporcionando caracteres perfectamente proporcionados y legibles a cualquier resolución sin artefactos de escalera ni deformaciones.

#### 3. Simulación Física de Grosores de Trazo (`FontWeight`)
- **`Normal`**: Cobertura estándar derivada del contorno base.
- **`Semibold`**: Aumento adaptativo de opacidad en bordes de subpíxel (+35 en la máscara de cobertura) para mayor peso visual en pantallas de alta densidad.
- **`Bold` (Negrita)**: **Dilatación morfológica horizontal analítica**:
  $$\text{bmp}_{\text{bold}}[y] = \text{bmp}[y] \mid (\text{bmp}[y] \gg 1)$$
  Engrosa los tallos verticales exactamente en 1 píxel físico antes del cómputo de cobertura y blending lineal, produciendo un resaltado contundente y natural.

#### 4. Diálogo Flotante de Selección y Vista Previa en Vivo (`FontSelectorDialog`)
El modal flotante está organizado en un diseño ergonómico de dos columnas:
- **Columna Izquierda (Controles)**:
  - Tarjetas de selección para las 5 familias tipográficas con nombre y descripción.
  - Píldoras de selección directa de tamaño de fuente (`10` a `24 pt`).
  - Botones de selección de peso de trazo (`Normal`, `Semibold`, `Bold`).
  - Chips de alternancia de ligaduras, anti-aliasing e interlineado vertical (`1px, 3px, 5px, 8px`).
- **Columna Derecha (Live Preview Box)**:
  - Cuadro de previsualización con código Rust sintácticamente coloreado (`fn benchmark_render()`, comentarios, palabras clave, cadenas con ligaduras `->`, `!=`, `=>`, `==`).
  - Se re-rasteriza en tiempo real al seleccionar cualquier opción, permitiendo evaluar el resultado antes de confirmar.
- **Navegación e Interacción Total**:
  - **Teclado**: `Tab` para alternar entre secciones (Familias, Tamaños, Grosores, Opciones, Botones), `↑`/`↓` para seleccionar, `←`/`→` para ajustar tamaños o grosores, `+`/`-`/`0` para zoom rápido, `l` para ligaduras, `a` para anti-alias, `s` para interlineado, `Enter` para aplicar y `Esc` para cancelar.
  - **Ratón**: Clic directo en cualquier tarjeta, píldora, chip o botón con respuesta hover visual; rueda del ratón (`ScrollUp`/`ScrollDown`) para incrementar o decrementar el tamaño de fuente interactivamente.

#### 5. Atajos Rápidos de Zoom
Rustext permite modificar el tamaño tipográfico en cualquier momento durante la edición:
- `Ctrl + +` o `Ctrl + =`: Aumentar tamaño de fuente (siguiente peldaño en la escala).
- `Ctrl + -`: Disminuir tamaño de fuente (peldaño anterior en la escala).
- `Ctrl + 0`: Restablecer tamaño de fuente inmediatamente al valor predeterminado (**12 pt**).
- `Ctrl + Shift + T`: Abrir el diálogo modal interactivo de tipografía.

#### 6. Persistencia en `config.ini`
```ini
[gui]
font_family = standard   # standard, terminus, modern, typewriter, retro
font_size = 12            # 10, 11, 12, 14, 16, 18, 20, 24 pt
font_weight = normal      # normal, semibold, bold
line_spacing = 3          # Interlineado vertical en píxeles: 1, 3, 5, 8
font_ligatures = true     # Ligaduras de programación
font_antialiasing = true  # Anti-aliasing espacial
font_hinting = medium     # none, slight, medium, full
```

---

### 3.8 Backend Nativo Wayland para Linux (`wl_shm` + `xdg_shell` + `memfd_create`)

A partir de esta versión, Rustext incorpora soporte nativo para el protocolo **Wayland** en Linux sin depender de `libwayland-client.so`, bibliotecas C ni ningún crate externo:

#### 1. Arquitectura Socket-Direct de Cero Dependencias:
- **Comunicación Binaria Pura (`AF_UNIX`)**: Rustext se conecta directamente al socket del compositor Wayland (`$WAYLAND_DISPLAY` o `$XDG_RUNTIME_DIR/wayland-0` / `wayland-1`) utilizando el cliente de sockets estándar `std::os::unix::net::UnixStream`.
- **Serialización y Deserialización en Rust Puro**: Implementa de forma nativa los paquetes de cabecera de 8 bytes `[object_id, (opcode) | (size << 16)]` y los argumentos de 32 bits, cadenas con relleno nulo a 4 bytes y números en punto fijo 24.8.
- **Memoria Compartida Anónima (`memfd_create`)**: El búfer gráfico de la pantalla se respalda mediante un descriptor de archivo en memoria creado con `memfd_create` (con fallback dinámico a `/dev/shm`), mapeado con `mmap` y transferido al compositor (`wl_shm_pool`) mediante mensajes de control `SCM_RIGHTS` en llamadas nativas `sendmsg`.

#### 2. Protocolo de Ventanas de Escritorio (`xdg_shell`):
- **Ciclo de Vida `xdg_wm_base`**: Establece la superficie toplevel (`xdg_surface` y `xdg_toplevel`), gestiona el título de la ventana y responde puntualmente a los eventos `ping(serial)` con `pong(serial)` para garantizar que el compositor nunca marque la aplicación como bloqueada o colgada.
- **Sincronización `ack_configure`**: Acepta dinámicamente configuraciones de tamaño del compositor (tiling managers como Sway, Hyprland, o escritorios flotantes como KDE Plasma y GNOME) y adapta el framebuffer al vuelo.

#### 3. Control de Entrada (Asientos, Puntero y Teclado):
- **`wl_pointer`**: Mapeo de coordenadas continuas de ratón con precisión subpíxel, botones izquierdo (272), derecho (273) y central (274), y eventos de eje de rueda de desplazamiento vertical.
- **`wl_keyboard`**: Decodificación de códigos de escaneo estándar del kernel Linux (`evdev`), estados de teclas y combinaciones de modificadores simultáneos (`Shift`, `Ctrl`, `Alt`, `Ctrl+Shift`).

#### 4. Doble Búfer Real (Double Buffering) y Sincronización con el Compositor:
- **Doble Búfer en Memoria Compartida**: Asigna dos búferes independientes (`buffer_ids: [u32; 2]`) alternando ranuras en el pool de memoria (`slot_size * 2`), permitiendo que el compositor lea el cuadro actual mientras Rustext dibuja el siguiente sin interferencias ni *tearing*.
- **Decoraciones del Servidor (`zxdg_decoration_manager_v1`)**: Negocia automáticamente con el gestor de ventanas (KWin en KDE Plasma 6, Mutter en GNOME) para suministrar la barra de título, botones de minimizar/maximizar/cerrar, marcos de redimensionamiento y sombras externas nativas del entorno de escritorio.
- **Coalescencia Inteligente de Eventos de Ratón**: Fusiona ráfagas de eventos consecutivos `wl_pointer.motion` en la cola interna para evitar saturar el ciclo de renderizado, garantizando un movimiento del cursor extremadamente suave a altas tasas de refresco (120 Hz, 144 Hz, 240 Hz).
- **Opacidad Absoluta**: Garantiza el canal alfa de cada píxel en `0xFF` (`| 0xFF000000`) para evitar transparencias accidentales en pipelines de composición acelerada por GPU (OpenGL/Vulkan).

#### 5. Detección Automática y Fallback Transparente a X11:
- Al arrancar en Linux, [`LinuxWindow::new`](file:///home/bolek/Proyectos/rustext/src/window/linux.rs) comprueba en primer lugar si existe un socket de compositor Wayland y si la conexión es exitosa.
- Si Wayland está disponible, Rustext se ejecuta de manera nativa sin necesidad de XWayland, eliminando intermediarios y reduciendo la latencia de presentación a menos de 1 milisegundo.
- Si Wayland no está disponible o el usuario se encuentra en un entorno X11 puro, el editor conmuta instantáneamente al backend [`X11Window`](file:///home/bolek/Proyectos/rustext/src/window/x11.rs).
- **Control Manual por Variable de Entorno**:
  ```bash
  # Forzar ejecución nativa en Wayland:
  RUSTEXT_BACKEND=wayland ./target/release/rustext --gui

  # Forzar ejecución en X11 / XWayland:
  RUSTEXT_BACKEND=x11 ./target/release/rustext --gui
  ```

---

### 3.9 Motor Vectorial TrueType (.ttf), Anti-Aliasing Subpíxel y Caché Tipográfica

Rustext integra un motor tipográfico vectorial completo para fuentes **TrueType (.ttf)** escrito **100% en Rust puro sin librerías externas** (sin `FreeType`, `HarfBuzz` ni `stb_truetype`), combinando decodificación binaria SFNT segura, rasterización analítica continua y una arquitectura de caché tipográfica de dos niveles.

#### 1. Decodificador Binario SFNT/TrueType Seguro:
- **`TtfReader`**: Lector binario estructurado con ordenamiento big-endian y validación estricta de límites contra ataques de desbordamiento de búfer (*buffer overflow*) y tipografías maliciosas o corruptas (*font fuzzing*).
- **Tablas SFNT Parseadas**:
  - `head`: Bounding box global del glifo, factor `units_per_em`, y formato de tabla de localización (`index_to_loc_format` corto o largo).
  - `hhea` / `hmtx`: Métrica horizontal, ascender, descender, line gap, ancho de avance (`advance_width`) y *left side bearing* (`lsb`).
  - `maxp`: Cantidad total de glifos en el archivo de fuente.
  - `cmap` (Formato 4): Mapeo eficiente de rangos de caracteres Unicode (BMP) a identificadores de glifo (`glyph_id`).
  - `loca` / `glyf`: Extracción directa de contornos vectoriales para glifos simples y compuestos (descomposición recursiva de componentes referenciados con matrices de transformación 2D afines).

#### 2. Reconstrucción de Contornos y Subdivisión Bézier Adaptativa:
- **Puntos On-Curve y Off-Curve**: Reconstruye bucles poligonales cerrados respetando los puntos de control cuadráticos del estándar TrueType.
- **Inserción Analítica de Puntos Medios**: Detecta pares consecutivos de puntos de control off-curve e inserta automáticamente un punto medio virtual on-curve entre ellos.
- **Subdivisión Recursiva de Curvas Bézier**: Evalúa la desviación euclidiana en el punto medio de la curva $B(0.5)$ respecto a la cuerda entre extremos; subdivide recursivamente hasta alcanzar una tolerancia geométrica de **0.25 píxeles físicos** o una profundidad máxima de 10 niveles, eliminando cualquier quiebre visual en las curvas.

#### 3. Rasterizador Analítico de Área y Devanado Fraccional:
- **Algoritmo Trapezoidal Analítico**: Basado en la formulación de líneas de escaneo continuas de Nicolas Rougier y FreeType. Para cada segmento de recta, recorta contra las filas de píxeles y calcula el área trapezoidal con signo y el devanado fraccional (`CellAccumulator`).
- **Anti-Aliasing Continuo sin Supersampling**: Produce una máscara de cobertura espacial de 8 bits (0..=255) continua sin el costo computacional de muestreo por fuerza bruta.

#### 4. Anti-Aliasing Subpíxel LCD ClearType (RGB) con Filtro FIR:
- **Muestreo 3x Horizontal**: Triplica la resolución horizontal del contorno alineándolo con las bandas físicas de subpíxeles rojo, verde y azul del panel LCD.
- **Filtro Paso-Bajo FIR Simétrico de 5 Tomas (`[1, 4, 6, 4, 1] / 16`)**: Aplica una convolución discreta de bajo consumo que elimina la aberración cromática (*color fringing*), maximizando la nitidez aparente del texto en pantallas estándar.
- **Composición Fotométricamente Exacta**: Fusión de canales en espacio de luz lineal con corrección gamma IEC 61966-2-1 (`gamma = 2.2`).

#### 5. Arquitectura de Caché Tipográfica de Dos Niveles (`TtfGlyphCache`):
Para lograr tasas de refresco superiores a **1,000 FPS** sostenidos en la interfaz gráfica, Rustext utiliza una caché tipográfica de dos niveles en memoria:
- **Nivel 1 (Fast ASCII Direct Table)**:
  - Tablas de arrays planos `[Option<Arc<[u8]>>; 128]` (escala de grises) y `[Option<Arc<[[u8; 3]]>>; 128]` (subpíxel LCD).
  - Acceso en tiempo constante $\mathcal{O}(1)$ por índice directo de carácter (`ch as usize`) con **cero costo de hashing**, recuperando la cobertura del glifo en menos de **5 nanosegundos**.
- **Nivel 2 (Associative Bounded LRU Cache)**:
  - Mapa asociativo para caracteres Unicode internacionales (cirílico, griego, CJK, operadores matemáticos, dibujo de marcos y cajas).
  - Desalojo LRU por lotes del 25% más antiguo al alcanzar el límite configurable de 4,096 glifos, impidiendo fugas de memoria en sesiones prolongadas.
- **Pre-calentamiento (*Warmup*) al Arranque**:
  - Pre-rasteriza en segundo plano los 95 caracteres ASCII imprimibles (`32..=126`) tan pronto se configura la ventana, garantizando que el primer fotograma se dibuje sin micro-tirones.
- **Invalidación Segura**:
  - Al conmutar la fuente activa vía `SystemFontLoader::set_active_font()`, la caché se purga automáticamente de forma atómica.

#### 6. Autodescubrimiento del Sistema y Sintetizador Tipográfico Embebido:
- **`SystemFontLoader`**: Escanea automáticamente las rutas estándar del sistema operativo buscando las mejores fuentes monoespaciadas instaladas:
  - `/usr/share/fonts/liberation/LiberationMono-Regular.ttf`
  - `/usr/share/fonts/noto/NotoSansMono-Regular.ttf`
  - `/usr/share/fonts/TTF/DejaVuSansMono.ttf`
  - `/usr/share/fonts/TTF/VeraMono.ttf`, `MesloLGSNerdFontMono-Regular.ttf`, `Hack-Regular.ttf`
  - `~/.local/share/fonts/*.ttf`
- **Sintetizador Tipográfico Nativo (Built-in Fallback)**: Si el sistema carece de fuentes instaladas o se ejecuta en un contenedor minimalista (*scratch*), Rustext sintetiza en memoria una tipografía TrueType SFNT completa y válida que incluye todos los caracteres ASCII con contornos vectoriales matemáticos, asegurando que el editor siempre funcione de manera autónoma.

#### 7. Configuración en `config.ini`:
```ini
[gui]
# Habilitar el motor vectorial TrueType
use_ttf_font = true

# Ruta opcional a una fuente TTF personalizada (dejar vacío para autodetección del sistema)
ttf_font_path = /usr/share/fonts/liberation/LiberationMono-Regular.ttf

# Activar anti-aliasing subpíxel LCD ClearType (RGB)
subpixel_antialiasing = true
```

---

### 3.10 Identidad Visual Oficial: "The Faceted Ribbon R" y Desktop Launcher

Rustext adopta una identidad visual distintiva inspirada en la velocidad, el minimalismo geométrico y el renderizado nativo: **Concept 1: The Faceted Ribbon R**.

1. **Renderizado Geométrico Analítico**:
   - Dibuja una cinta continua facetada en 3D que conforma la letra "R" mediante polígonos analíticos con gradientes lineales y sombreado de luz incidente.
   - Tonos naranja Rust profundo (`#E05D44`, `#F74C00`), azul cibernético (`#7AA2F7`) y acentos titanio.
   - Integrado de forma nativa en la ventana flotante de información (*Acerca de*, `F1`), en los encabezados de diálogo y en la barra de título.

2. **Integración con el Escritorio de Linux (Wayland y X11)**:
   - Al ejecutarse en entornos Linux con Wayland o X11, Rustext detecta automáticamente si el lanzador de escritorio (`~/.local/share/applications/rustext.desktop`) y los iconos en alta resolución (`~/.local/share/icons/hicolor/...`) están presentes.
   - Si no existen o están desactualizados, los genera e instala automáticamente de forma silenciosa e idempotente, resolviendo el clásico problema de Wayland donde la ventana muestra un icono genérico ("W").
   - Define el `app_id` `rustext` en el protocolo `xdg_toplevel` de Wayland y `WM_CLASS` en X11 para un anclaje perfecto en muelles (*docks* como GNOME Dash, KDE Latte/Taskbar, Sway/Waybar).

---

## 4. Árbol de Proyecto, Espacios de Trabajo y Gestión de Proyectos

### 4.1 Árbol de Proyecto y Explorador Lateral (Sidebar)

El **Explorador Lateral (File Explorer Sidebar)** permite gestionar proyectos completos sin salir del editor.

#### Características:
1. **Escaneo Perezoso (Lazy Scanning)**: Solo lee del disco las carpetas que el usuario decide expandir, evitando consumo de memoria en proyectos con miles de ficheros.
2. **Filtro Automático**: Oculta automáticamente directorios ruidosos o de compilación (`.git`, `target`, `node_modules`, `.DS_Store`, `.idea`, `.vscode`).
3. **Pistas de Color por Extensión**:
   - Naranja: Archivos `.rs`.
   - Cian: Archivos `.toml`, `.ini`, `.json`, `.yaml`.
   - Verde: Archivos `.md`, `.txt`, `.py`.
   - Púrpura: Archivos `.c`, `.cpp`, `.h`, `.sql`, `.jl` (Julia).
   - Acento: Archivos web `.html`, `.css`, `.js`, `.ts`.
4. **Flechas Jerárquicas**: Glifos bitmap embebidos `▼` para directorios abiertos y `▶` para carpetas cerradas.

#### Controles del Explorador:
- `Ctrl + B`: Alternar visibilidad de la barra lateral (Mostrar / Ocultar).
- **Clic en Archivo**: Abre el archivo en una pestaña (o conmuta a ella si ya está abierta).
- **Clic en Carpeta**: Expande o contrae la carpeta.
- **Botón `[+]` (Header)**: Abre una nueva pestaña vacía.
- **Botón `[↺]` (Header)**: Fuerza el re-escaneo del árbol de archivos en el disco.
- **Botón `[×]` (Header)**: Cierra la barra lateral.
- **Navegación por Teclado**: Con foco en el explorador:
  - `↑` / `↓`: Mover selección arriba y abajo.
  - `→` / `Enter`: Expandir carpeta o abrir archivo seleccionado.
  - `←`: Contraer carpeta o saltar a la carpeta contenedora.
  - `Esc`: Devolver el foco al editor de código.

### 4.2 Espacios de Trabajo y Archivos `.rustext-workspace`

Rustext soporta la gestión integral de espacios de trabajo multiproyecto sin dependencias externas:
- **Abrir Carpeta de Proyecto (`Ctrl + Shift + O`)**: Abre un diálogo interactivo para seleccionar cualquier directorio del sistema como raíz del proyecto.
- **Archivos de Espacio de Trabajo (`.rustext-workspace`)**: Permite declarar configuraciones, exclusiones y directorios compartidos para repositorios individuales o monorrepositorios.
- Guardar espacio de trabajo actual con el menú **Archivo > Guardar Espacio de Trabajo Como...**.

### 4.3 Aislamiento de Sesiones por Proyecto (`session_<hash>.ini`)

A diferencia de los editores convencionales que mezclan el estado de todas las carpetas en un único archivo de sesión global:
- Rustext computa un hash unívoco para la ruta canónica de cada proyecto.
- Almacena el estado completo en `~/.rustext/session_<hash>.ini`: pestañas abiertas, cursores exactos, desplazamientos de scroll y paneles divididos.
- Al alternar entre proyectos, las pestañas y el estado del proyecto anterior se guardan atómicamente y se restauran las del nuevo proyecto sin interferencias.

### 4.4 Conmutación Ágil y Proyectos Recientes

- **Cambiar Espacio de Trabajo (`Ctrl + Shift + P` / `Ctrl + Alt + W`)**: Despliega un selector difuso rápido para saltar entre proyectos abiertos recientemente.
- Menú **Archivo > Proyectos Recientes**: Historial ordenado por fecha de acceso con eliminación automática de rutas inexistentes.
- La barra de estado inferior y la cabecera del explorador lateral reflejan en tiempo real el nombre del espacio de trabajo activo.

## 5. División de Vistas (Split Editor) y Minimapa Gráfico

Rustext ofrece dos herramientas visuales avanzadas para desarrolladores que gestionan bases de código extensas o requieren comparar y editar múltiples áreas de un proyecto en paralelo:

---

### 5.1 Minimapa Gráfico de Código (`Ctrl + M`)

El **Minimapa Gráfico** proporciona una vista panorámica en miniatura de todo el código fuente del documento activo:

1. **Micro-Renderizado de Sintaxis**:
   - Cada línea de código se proyecta en una fila de micro-píxeles compactos (bloques de 2×1 px).
   - Preserva fielmente la sangría (indentación) y espaciado de los bloques de código.
   - Aplica el resaltado de sintaxis en tiempo real según el lexer del lenguaje (palabras clave, literales numéricos, cadenas, comentarios, identificadores y operadores).
2. **Deslizador de Viewport Translúcido**:
   - Una ventana deslizante semi-transparente resalta el rango exacto de líneas actualmente visibles en el área de edición principal.
   - Cuenta con bordes superior e inferior contrastados para una clara delimitación visual.
3. **Navegación Interactiva con Ratón**:
   - **Clic Directo**: Al hacer clic en cualquier sección del minimapa, el viewport salta inmediatamente centrando la línea seleccionada.
   - **Arrastre del Deslizador**: Haga clic y arrastre sobre el cuadro translúcido para desplazarse continuamente por el archivo a 60 FPS sin tirones.
4. **Configuración y Personalización**:
   - Puede activarse o desactivarse con `Ctrl + M` o desde el menú **Ver > Minimapa**.
   - En `config.ini`, establezca `minimap = true` o `minimap = false`, y ajuste su ancho con `minimap_width = 100` (en píxeles).

---

### 5.2 División de Vistas y Paneles Independientes (Split Editor)

El sistema de **Split Editor** permite dividir el lienzo de trabajo en dos paneles independientes para comparar código, editar dos secciones lejanas del mismo archivo o inspeccionar documentación y código en simultáneo:

1. **Modos de División**:
   - **División Vertical (`Ctrl + \`)**: Dispone dos paneles lado a lado (ideal para pantallas panorámicas 16:9 o ultra-wide).
   - **División Horizontal (`Ctrl + Shift + \`)**: Apila dos paneles uno sobre otro (ideal para comparar funciones contiguas o pantallas verticales).
   - Conmutación directa desde los atajos de teclado o a través del menú **Ver > Dividir Verticalmente / Dividir Horizontalmente**.
2. **Conmutación de Foco y Navegación**:
   - **Atajo `Alt + \`**: Alterna instantáneamente el foco entre el Panel 1 y el Panel 2.
   - **Clic del Ratón**: Hacer clic en cualquier elemento de un panel (cabecera, gutter, código, scrollbar o minimapa) le transfiere el foco activo de inmediato.
   - **Indicadores Visuales**:
     - La cabecera del panel activo muestra el distintivo `[ACTIVO]` con tipografía resaltada.
     - La barra de estado inferior refleja el panel enfocado mediante la píldora `Panel: 1/2` o `Panel: 2/2`.
3. **Barra de División (Splitter) Arrastrable**:
   - Una línea divisoria interactiva separa ambos paneles.
   - Al posicionar el puntero del ratón sobre el splitter y arrastrarlo, el usuario puede ajustar dinámicamente la proporción del espacio asignado a cada panel.
   - **Límites de Seguridad**: El ratio está restringido al rango `[0.15, 0.85]`, garantizando que ninguno de los paneles colapse a tamaño cero.
4. **Independencia Total de Vistas y Búferes**:
   - Cada panel mantiene su propio archivo activo (pueden ser dos archivos distintos de la lista de pestañas o el mismo documento abierto dos veces).
   - **Desplazamiento Desacoplado**: Al navegar o hacer scroll en un panel, el otro conserva inalterada su posición de lectura.
   - **Cursores y Selecciones Aisladas**: Las operaciones multicursor, selecciones de texto y búsquedas locales se ejecutan en el panel que tiene el foco.
   - **Rueda del Ratón**: El scroll vertical con la rueda del ratón afecta automáticamente al panel que se encuentra bajo el cursor del ratón.
5. **Cierre y Reversión**:
   - Puede cerrar la vista dividida en cualquier momento con el menú **Ver > Cerrar División**, desde la Paleta de Comandos (`Ctrl + Shift + P`), o pulsando nuevamente el atajo de la división activa (`Ctrl + \` o `Ctrl + Shift + \`).
   - Al cerrar la división, todas las modificaciones permanecen seguras en memoria y las pestañas se sincronizan sin pérdidas.

---

## 6. Integración Git en Vivo (Git Gutter & Control de Versiones)

Rustext cuenta con un subsistema nativo de control de versiones Git que opera de manera asíncrona y transparente sin dependencias externas:

### Características Principales:
1. **Detección Automática del Repositorio y Rama Activa**:
   - Localiza la raíz del repositorio buscando el directorio `.git` hacia arriba en el árbol de carpetas.
   - Lee `.git/HEAD` directamente para resolver de forma instantánea el nombre de la rama activa (o commit desprendido) sin lanzar procesos externos ni bloquear el hilo de interfaz.
   - Muestra la rama activa y el resumen de cambios en la barra de estado inferior: `git: main (+3 ~1)`.

2. **Git Gutter (Indicadores de Diferencias Línea por Línea)**:
   - Compara en memoria el contenido actual del búfer contra la versión en el índice de Git (`git show :<ruta>`).
   - Algoritmo LCS (*Longest Common Subsequence*) optimizado implementado de forma pura sobre `std::*`.
   - Dibuja una barra vertical coloreada en el margen izquierdo (gutter) del editor:
     - 🟩 **Verde (`Added`)**: Líneas nuevas agregadas respecto al índice.
     - 🟦 **Azul (`Modified`)**: Líneas existentes modificadas.
     - 🟥 **Rojo (`DeletedAbove`)**: Marcador triangular que indica líneas eliminadas inmediatamente encima.
   - Puede activarse o desactivarse con `git_gutter = true/false` en `config.ini` o mediante el comando *"Git: Alternar Diff en Gutter"* en la Paleta de Comandos.

3. **Insignias de Estado en el Explorador Lateral (File Explorer)**:
   - Al lado derecho de cada archivo en la barra lateral se muestra una insignia de estado coloreada:
     - 🟨 **`M` (Modificado)**: Archivo con modificaciones en el árbol de trabajo.
     - 🟩 **`U` (Sin Rastrear / Untracked)**: Archivo nuevo aún no añadido al índice.
     - 🟦 **`A` (Agregado / Staged)**: Archivo agregado a staging.
     - 🟥 **`D` (Eliminado)**: Archivo marcado para eliminación.
     - 🟪 **`R` (Renombrado)**: Archivo renombrado.

4. **Inline Git Blame en Tiempo Real**:
   - Muestra una anotación atenuada (*ghost text*) de baja luminosidad a la derecha del código en la línea donde se encuentra el cursor principal.
   - Formato compacto: `autor, tiempo_relativo • mensaje_del_commit` (por ejemplo: `bolek, 2 hours ago • feat: add semantic tokens`).
   - El tiempo transcurrido se calcula de forma humana relativa (`just now`, `Xs ago`, `X mins ago`, `X hours ago`, `X days ago`, `X months ago`, `X years ago`).
   - **Rendimiento $\mathcal{O}(1)$ Cero Latencia**: Almacena los resultados en `blame_cache: HashMap<(PathBuf, usize), Option<GitBlameLine>>`. Al mover el cursor dentro de la misma línea o en fotogramas sucesivos, la resolución es instantánea en memoria sin ejecutar subprocesos, permitiendo un renderizado a más de 500 FPS.

5. **Ventana Flotante de Git Hunk Diff (`Alt + H`) y Reversión Atómica (`Alt + R`)**:
   - Al presionar `Alt + H` (o desde el menú **Herramientas > Inspeccionar Git Hunk Diff**), el editor analiza el hunk o bloque de diferencias contiguo en la posición del cursor respecto a `Git HEAD`.
   - Abre un diálogo flotante centrado con fondo translúcido y sombra proyectada que muestra:
     - Cabecera con el rango de líneas exacto (` Git Diff Hunk (Líneas X..=Y)`).
     - Líneas eliminadas o versión previa en Git HEAD (`- ` con fondo rojo translúcido).
     - Líneas actuales modificadas o añadidas (`+ ` con fondo verde translúcido).
   - **Reversión Atómica (`Alt + R`)**:
     - Al presionar `Alt + R` o hacer clic en el botón `[Revertir Hunk (Alt+R)]`, el editor revierte atómicamente el bloque a su estado en Git HEAD:
       - **Líneas Añadidas**: Se eliminan limpiamente del búfer y se reajustan los cursores.
       - **Líneas Modificadas**: Se reemplazan por el contenido original de Git HEAD.
       - **Líneas Eliminadas**: Se reinsertan en la posición exacta original.
     - La acción se registra como una `Transaction` en el historial de `History`, por lo que puede deshacerse en cualquier momento con `Ctrl + Z`.
     - Se notifica inmediatamente al servidor LSP del cambio y se actualizan los indicadores del gutter.
   - Presionar `Esc` o `Alt + H` cierra el modal instantáneamente.

6. **Actualización Reactiva**:
   - Se refresca automáticamente al guardar cualquier archivo (`Ctrl + S`).
   - Actualización manual instantánea mediante el atajo `Ctrl + Shift + G` o el menú **Herramientas > Actualizar Estado Git**.

---

## 7. Motor de Edición, Multicursor, Soft Wrap y Code Folding

Rustext incluye un motor de edición avanzado de ultra-baja latencia respaldado por un Piece Table $\mathcal{O}(\log N)$, multicursor vectorial y navegación visual de sublíneas:

### 7.1 Operaciones Multicursor
- `Ctrl + D`: **Selección Inteligente / Siguiente Ocurrencia**:
  - Si no hay selección, selecciona la palabra bajo el cursor.
  - Si se presiona nuevamente, busca la siguiente ocurrencia en el archivo y añade un cursor adicional en ella.
- `Ctrl + K`: Añadir un cursor en la misma columna de la línea superior.
- `Ctrl + J`: Añadir un cursor en la misma columna de la línea inferior.
- `Esc`: Colapsa todos los cursores al cursor principal.

### 7.2 Sincronización y Desplazamiento Vectorial
Cuando múltiples cursores escriben simultáneamente:
- Todas las inserciones se calculan en paralelo.
- El motor calcula el desplazamiento acumulativo (`offset shift`) para que los cursores posteriores mantengan su alineación exacta.
- Los cursores que colisionan o se solapan se fusionan automáticamente sin duplicaciones.

### 7.3 Transacciones Atómicas de Deshacer / Rehacer (`Ctrl+Z` / `Ctrl+Y`)
- Todas las operaciones realizadas por 10 o 50 cursores en una pulsación se empaquetan en una única transacción atómica.
- Al pulsar `Ctrl+Z`, se revierte el texto y se restauran las posiciones exactas de todos los cursores previos.

### 7.4 Auto-Cierre y Envoltorio de Pares (Auto-Surround)
- Al escribir `(`, `[`, `{`, `"`, `'`, el editor inserta automáticamente el carácter de cierre.
- Al presionar `Backspace` dentro de un par vacío (`|`), ambos caracteres se eliminan.
- **Envoltorio de Selección**: Si selecciona texto y presiona `(` o `"`, el texto seleccionado se envuelve automáticamente entre los delimitadores.

### 7.5 Ajuste de Línea Envolvente (Soft Wrap / Word Wrap, Alt+Z)
El sistema de **Ajuste de Línea Envolvente (Soft Wrap)** formatea dinámicamente las líneas que sobrepasan el ancho visible de la ventana en sublíneas virtuales sin insertar saltos de línea físicos ni mutar el búfer de texto:
- **Activación Inmediata**: Conmute con `Alt + Z` o desde el menú **Ver > Ajuste de Línea (Word Wrap)**. Se persiste en `config.ini` bajo `[editor] word_wrap = true/false`.
- **Segmentación por Límites de Palabras**: Rompe visualmente el texto respetando espacios en blanco y delimitadores léxicos, evitando partir identificadores o palabras a mitad de sílaba a menos que una palabra individual exceda el ancho total del viewport.
- **Navegación Vertical por Sublíneas Virtuales**: Las flechas `↑` y `↓` navegan intuitivamente a través de las sublíneas envueltas de la misma línea lógica antes de pasar a la línea anterior o siguiente.
- **Mapeo Geométrico de Clics de Ratón**: El algoritmo de hit-testing calcula la sublínea y columna visuales del clic, mapeándolas con precisión exacta $\mathcal{O}(1)$ al offset de carácter real en el Piece Table.
- **Integración Transparente**: La numeración de líneas en el gutter permanece fija en la primera sublínea (las sublíneas subsecuentes muestran un gutter limpio), y las ondulaciones de diagnóstico (wavy squiggles) se segmentan adecuadamente por sublínea.

### 7.6 Plegado de Código Estructurado (Code Folding, Alt+[, Alt+0, Alt+J)
El motor de **Code Folding** permite colapsar y expandir regiones jerárquicas de código fuente para maximizar la legibilidad en archivos densos:
- **Detección Políglota de Regiones**:
  - **Basado en Delimitadores / Llaves `{ ... }`**: Para Rust, C/C++, JavaScript, TypeScript, Go y lenguajes similares, calcula el emparejamiento de llaves ignorando llaves dentro de cadenas de texto y comentarios.
  - **Basado en Indentación**: Para Python y YAML, detecta cambios en el nivel de sangrado de bloques subsiguientes.
- **Indicadores Visuales en el Gutter**: Dibuja chevrons interactivos en el margen izquierdo:
  - `▼` (Triángulo hacia abajo): Bloque expandido listo para ser colapsado.
  - `▶` (Triángulo hacia la derecha): Bloque plegado.
  - Al hacer clic con el ratón directamente sobre el chevron del gutter, el bloque se pliega o despliega al instante.
- **Atajos de Teclado**:
  - `Alt + [` / `Ctrl + Shift + [` / `F9`: Alternar el plegado del bloque bajo la posición actual del cursor.
  - `Alt + 0`: Plegar todas las regiones del archivo.
  - `Alt + J`: Desplegar todas las regiones del archivo.
- **Despliegue Inteligente por Navegación (`unfold_to_reveal`)**: Si el usuario navega con el cursor hacia una línea oculta, o si un salto de Búsqueda Global (`Ctrl + Shift + F`), siguiente error (`F8`) o "Ir a Definición" de LSP (`F12`) aterriza en una línea plegada, Rustext expande automáticamente el bloque contenedor para garantizar visibilidad inmediata.
- **Sincronización con Encabezados Fijos (Sticky Scroll)**: Las cabeceras de bloques plegados se gestionan limpiamente para evitar redundancias visuales en la zona de scroll pegajoso.

---

## 8. Autocompletado Contextual Inteligente

Rustext implementa un motor de autocompletado en tiempo real sin dependencias externas ni servidores pesados:

### Características:
1. **Activación Inmediata (`Ctrl + Espacio`)**: Despliega un menú emergente anclado exactamente bajo el cursor de texto (o sobre él si el cursor se encuentra cerca del borde inferior de la ventana). También accesible mediante el menú **Editar > Autocompletar**.
2. **Candidatos de Doble Fuente**:
   - **Palabras Clave de Lenguaje**: Palabras reservadas y tipos primitivos según el lenguaje del archivo actual (Rust, Python, C/C++, Go, JavaScript, TypeScript, Markdown, TOML/INI).
   - **Identificadores del Proyecto**: Extrae en vivo identificadores léxicos (funciones, variables, structs, métodos, constantes) de todos los búferes y pestañas abiertas en la sesión.
3. **Scoring y Ranking Inteligente**:
   - Prioriza coincidencias exactas por prefijo sobre coincidencias insensibles a mayúsculas/minúsculas.
   - Ordena por longitud de sugerencia y orden alfabético para una selección natural y rápida.
4. **Diseño Visual**:
   - Marco redondeado con sombra, píldora con indicador de posición `[1/N]`, etiquetas de tipo de sugerencia (`[KW]` palabra clave vs `[ID]` identificador de código).
   - Scrollbar interno interactivo cuando existen más de 8 coincidencias.
5. **Compatibilidad Multicursor**:
   - Si se tienen múltiples cursores activos (`Ctrl + D`, `Ctrl + K/J`), al confirmar la sugerencia seleccionada, el autocompletado se aplica simultáneamente en todos los cursores, preservando los desplazamientos de cada uno.

### Controles de Autocompletado:
- `↑` / `↓`: Navegar hacia arriba o abajo en la lista de candidatos.
- `Tab` / `Enter`: Insertar y confirmar la sugerencia seleccionada.
- `Esc`: Cerrar el menú flotante sin modificar el código.
- **Ratón**: Hover interactivo para resaltar opciones y clic para confirmar e insertar de inmediato.

---

## 9. Búsqueda Global en Todo el Proyecto

La **Búsqueda Global en Todo el Proyecto (`Ctrl + Shift + F`)** permite localizar al instante cualquier cadena de texto o símbolo en todos los archivos del árbol del proyecto.

### Características:
1. **Escaneo Recursivo Ultrarrápido (`GlobalSearchEngine`)**:
   - Recorre recursivamente todas las carpetas a partir de la raíz del proyecto (`root_dir`).
   - **Filtrado Inteligente de Carpetas**: Omite directorios ruidosos o de compilación (`.git`, `target`, `node_modules`, `.cache`, `.vscode`, `.idea`, `dist`, `build`, etc.).
   - **Detección Automática de Binarios**: Inspecciona la cabecera de los archivos; si contiene bytes nulos (`\0`), se descarta automáticamente para no procesar ejecutables ni imágenes.
   - **Límites de Seguridad**: Salta archivos mayores a 10 MB y topa los resultados a 2,000 coincidencias para garantizar 60 FPS estables.
2. **Búsqueda Reactiva en Tiempo Real**:
   - Al tipear en el campo de texto, los resultados se filtran y actualizan inmediatamente en el siguiente fotograma.
3. **Sensibilidad a Mayúsculas/Minúsculas**:
   - Botón interactivo `[Aa]` en la cabecera del diálogo modal.
   - Atajo de teclado `Alt + C` para alternar entre búsqueda exacta o insensible.
4. **Resaltado y Snippets**:
   - Cada fila muestra la insignia de archivo y número de línea (`src/core/mod.rs:42`), separador visual y el snippet del código.
   - La coincidencia exacta dentro de la línea de código se resalta en color ámbar/dorado brillante (`0xFFD700`).
   - Barra de estado con resumen en vivo: `N coincidencias en M archivos`.
5. **Navegación y Apertura Inmediata**:
   - Al pulsar `Enter` o hacer **clic con el ratón** sobre un resultado:
     - El archivo se abre en una pestaña (conmutando a ella si ya estaba abierta).
     - El cursor se posiciona exactamente en la línea y columna del término buscado.
     - El área visible (viewport) se centra verticalmente en la coincidencia.
     - El diálogo modal de búsqueda se cierra automáticamente.

---

## 10. Sistema de Snippets, Sintaxis y Bracket Pair Colorization

Rustext incluye un motor de resaltado léxico sin dependencias externas con soporte especializado para **Bracket Pair Colorization** y plantillas de código (*snippets*):

### 10.1 Coloreado de Paréntesis (*Bracket Pair Colorization*)
- **Profundidad Anidada Cíclica**:
  - Los delimitadores `()`, `[]` y `{}` se analizan en tiempo real calculando la profundidad de anidamiento (`depth`).
  - La profundidad se proyecta cíclicamente con módulo 3 (`depth % 3`) asignando colores de alto contraste (`bracket_0`, `bracket_1`, `bracket_2`).
- **Paletas cromáticas integradas**:
  - **Tokyo Night**: Dorado `#E0AF68`, Púrpura `#BB9AF7`, Azul/Cian `#7AA2F7`.
  - **Catppuccin Mocha**: Amarillo `#F9E2AF`, Malva `#CBA6F7`, Zafiro `#74C7EC`.
  - **One Dark**: Amarillo `#E5C07B`, Púrpura `#C678DD`, Cian `#56B6C2`.
  - **Monokai**: Amarillo `#E6DB74`, Magenta `#F92672`, Cian `#66D9EF`.
  - **Modo Consola TUI**: Códigos de escape ANSI 24-bit (`\x1b[1;33m`, `\x1b[1;35m`, `\x1b[1;36m`).
- **Inmunidad a Cadenas y Comentarios**:
  - Los paréntesis, corchetes o llaves dentro de literales de cadena (`"()"`), plantillas o comentarios (`// []`, `/* {} */`, `# ()`) se ignoran estrictamente para no alterar la profundidad estructural del código.

### 10.2 Lenguajes Sintácticos Soportados
Rustext detecta automáticamente el lenguaje por extensión o nombre:
- **Rust** (`.rs`)
- **Python** (`.py`)
- **C / C++** (`.c`, `.h`, `.cpp`, `.hpp`, `.cc`, `.cxx`)
- **JavaScript / TypeScript** (`.js`, `.ts`, `.jsx`, `.tsx`, `.mjs`, `.cjs`)
- **HTML / XML** (`.html`, `.htm`, `.xhtml`, `.xml`, `.svg`)
- **CSS / SCSS** (`.css`, `.scss`, `.sass`, `.less`)
- **JSON** (`.json`)
- **YAML** (`.yaml`, `.yml`)
- **TOML** (`.toml`)
- **Shell / Bash** (`.sh`, `.bash`, `.zsh`, `.fish`, `.env`, `.bashrc`, `.profile`, `Dockerfile`)
- **SQL** (`.sql`)
- **Markdown** (`.md`, `.markdown`)
- **Julia** (`.jl`, `.JL`): Resaltador sintáctico especializado con soporte para comentarios de una línea (`#`), comentarios de bloque multilínea (`#= ... =#`), literales de cadena (`"..."`), caracteres (`'c'`), comandos de sistema entre comillas invertidas (`` `cmd` ``), macros nativas clasificadas como `TokenType::Macro` (`@inline`, `@time`, `@threads`, `@test`, etc.), palabras clave y tipos primitivos de la biblioteca base de Julia.

### 10.3 Catálogo de Snippets Integrados:
Escriba el disparador y presione `Tab` para expandirlo:

| Lenguaje | Trigger | Descripción | Expansión |
| :--- | :--- | :--- | :--- |
| **Rust** | `fn` | Función con Result | `fn name(params) -> Result<(), ...> { ... }` |
| **Rust** | `pfn` | Función pública | `pub fn name(params) -> () { ... }` |
| **Rust** | `struct` | Estructura con derive | `#[derive(Debug, Clone)] pub struct Name { ... }` |
| **Rust** | `enum` | Enumeración con derive | `#[derive(Debug, Clone, PartialEq, Eq)] pub enum Name { ... }` |
| **Rust** | `impl` | Bloque implementación | `impl Type { ... }` |
| **Rust** | `match` | Expresión match | `match expr { pattern => expr, _ => todo!() }` |
| **Rust** | `test` | Test unitario | `#[test] fn test_name() { assert!(true); }` |
| **Julia** | `fn` | Definición de función | `function name(args)\n ... \nend` |
| **Julia** | `fun` | Función de una línea | `name(args) = expr` |
| **Julia** | `struct` | Estructura inmutable | `struct Name\n ... \nend` |
| **Julia** | `mstruct` | Estructura mutable | `mutable struct Name\n ... \nend` |
| **Julia** | `for` | Bucle for iterativo | `for i in 1:n\n ... \nend` |
| **Julia** | `while` | Bucle condicional | `while condition\n ... \nend` |
| **Julia** | `if` / `ifelse` | Condicional if / else | `if condition\n ... \nelse\n ... \nend` |
| **Julia** | `try` | Manejo de excepciones | `try\n ... \ncatch e\n ... \nend` |
| **Julia** | `mod` | Definición de módulo | `module ModuleName\nexport sym\n ... \nend` |
| **Julia** | `macro` | Definición de macro | `macro name(args)\n ... \nend` |
| **Julia** | `pl` / `println`| Salida estándar | `println("message")` |
| **Julia** | `test` | Suite de pruebas Testset | `@testset "test_name" begin\n @test true\nend` |
| **Python** | `def` | Función con docstring | `def name(args): """Docstring""" pass` |
| **Python** | `class` | Definición de clase | `class ClassName: def __init__(self): pass` |
| **Python** | `main` | Bloque de entrada | `if __name__ == "__main__": main()` |
| **C / C++** | `main` | Función main | `int main(int argc, char *argv[]) { return 0; }` |
| **C / C++** | `inc` | Directiva include | `#include <stdio.h>` |
| **JS / TS** | `fn` | Función estándar | `function name(params) { ... }` |
| **JS / TS** | `afn` | Función flecha (arrow) | `const name = (params) => { ... };` |
| **JS / TS** | `clg` | Imprimir en consola | `console.log(...);` |
| **HTML** | `html5` | Estructura HTML5 | Documento HTML completo con head y body |
| **HTML** | `div` | Contenedor div | `<div class="container">...</div>` |
| **HTML** | `a` | Enlace hipertexto | `<a href="#">enlace</a>` |
| **CSS** | `flex` | Flexbox centrado | `display: flex; justify-content: center; ...` |
| **CSS** | `grid` | Grid responsivo | `display: grid; grid-template-columns: ...` |
| **CSS** | `media` | Media query responsive | `@media (max-width: 768px) { ... }` |
| **YAML** | `kv` | Par clave: valor | `key: value` |
| **YAML** | `item` | Elemento de lista | `- name: value` |
| **Shell** | `sh` | Shebang seguro Bash | `#!/usr/bin/env bash set -euo pipefail` |
| **Shell** | `func` | Función Shell | `name() { echo "done"; }` |
| **Shell** | `if` | Condicional Shell | `if [ condition ]; then ... fi` |
| **SQL** | `select` | Consulta SELECT | `SELECT * FROM table_name WHERE condition;` |
| **SQL** | `table` | Sentencia CREATE TABLE | `CREATE TABLE table_name (id SERIAL PRIMARY KEY, ...);` |
| **SQL** | `insert` | Sentencia INSERT INTO | `INSERT INTO table_name (columns) VALUES (values);` |
| **Markdown** | `code` | Bloque de código | ````rust ... ```` |
| **Markdown** | `link` | Enlace markdown | `[texto](url)` |

### Navegación de Placeholders:
- Presione `Tab` para avanzar al siguiente campo del snippet.
- Presione `Shift + Tab` para regresar al campo anterior.

---

## 11. Diagnósticos en Vivo, Linter y Formato de Código

Rustext incluye herramientas de verificación y corrección de código integradas:

### 1. Comprobación de Errores y Linter (`F7`):
- **Rust**: Ejecuta `cargo check --message-format=json` de forma transparente y mapea los errores y advertencias línea por línea en el buffer.
- **Python**: Valida sintaxis con `python -m py_compile` o `pyflakes`.
- **C/C++**: Ejecuta `clang -fsyntax-only`.
- **Shell**: Valida sintaxis de scripts con `bash -n`.
- **Visualización**: Las líneas con error se señalan en el gutter con insignias de color:
  - `[E]` Rojo: Error de compilación.
  - `[W]` Amarillo: Advertencia (Warning).
  - `[I]` Azul: Información.

### 2. Navegación entre Errores:
- `F8`: Salta al **siguiente error o advertencia** en el archivo.
- `Shift + F8`: Salta al **error anterior**.

### 3. Formateo Automático de Código (`Ctrl + Shift + I`):
Alinea el archivo utilizando el servidor LSP activo o el formateador del lenguaje disponible en el sistema operativo:
- **Rust**: LSP (`rust-analyzer`) o `rustfmt`
- **Julia**: `JuliaFormatter.jl` / `julia -e "using JuliaFormatter; print(format_text(read(stdin, String)))"`
- **Python**: LSP (`pylsp`) o `black` / `ruff format`
- **C/C++**: LSP (`clangd`) o `clang-format`
- **Web / HTML / CSS / JS / TS / JSON / YAML / Markdown**: LSP o `prettier`
- **Shell / Bash**: `shfmt`
- **Go**: LSP (`gopls`) o `gofmt`
- **Zig**: LSP (`zls`)

### 4. Ondulaciones Wavy Squiggles de Diagnóstico:
Rustext dibuja subrayados ondulados realistas (sinusoidales/zigzag) bajo el rango exacto de columnas donde el compilador o servidor de lenguaje reporta un problema:
- **Rojo (`0xFFFF5555`)**: Errores sintácticos o de compilación (`DiagnosticSeverity::Error`).
- **Amarillo / Ámbar (`0xFFFFB86C`)**: Advertencias del linter o compilador (`DiagnosticSeverity::Warning`).
- **Azul Claro (`0xFF8BE9FD`)**: Sugerencias informativas (`DiagnosticSeverity::Info`).
- **Soporte Completo**: Las ondulaciones se calculan y adaptan matemáticamente a resoluciones HiDPI, ajuste de línea envolvente (Soft Wrap), paneles divididos (Split Editor) y encabezados fijos de Sticky Scroll.

### 5. Suite LSP de Élite (Language Server Protocol) y Multi-Lenguaje:
Rustext implementa un cliente **LSP (Language Server Protocol) de nivel IDE conforme a JSON-RPC 2.0**, construido 100% con la biblioteca estándar de Rust (`std::*`), con **cero dependencias externas**:

#### Capacidades Implementadas:
1. **Buscar Todas las Referencias (`textDocument/references`, `Ctrl + Shift + U`)**:
   - Envía la posición del cursor y consulta todas las ocurrencias del símbolo a través de todo el espacio de trabajo.
   - Ventana modal interactiva con insignias de archivo y línea, y **vista previa del snippet** de código donde aparece la referencia.
   - Navegación por teclado (`↑`/`↓`, `PageUp`/`PageDown`, `Enter`, `Esc`) o clic de ratón con salto inmediato y centrado de pantalla.

2. **Renombrado de Símbolos en Todo el Workspace (`textDocument/rename`, `F2`)**:
   - Abre un prompt modal interactivo con el nombre del identificador actual precargado.
   - Procesa transacciones atómicas `WorkspaceEdit` (`changes` y `documentChanges`) aplicando las ediciones en orden inverso por offsets.
   - Actualiza de forma segura pestañas abiertas en memoria y archivos cerrados en el disco sin pérdida de offsets ni corrupción.

3. **Acciones Rápidas y Quick-Fixes (`textDocument/codeAction`, `Ctrl + .`)**:
   - Envía los diagnósticos activos del cursor al servidor para solicitar correcciones automáticas y refactorizaciones.
   - Modal flotante con insignias temáticas: `💡 [QuickFix]`, `🛠️ [Refactor]`, `📦 [Source]` y `★ [Preferido]`.
   - Aplica inmediatamente las ediciones asociadas a la acción confirmada.

4. **Formateo de Documento con LSP (`textDocument/formatting`, `Ctrl + Shift + I`)**:
   - Solicita el formateo estructurado del búfer al servidor de lenguaje.
   - Procesa la lista de `TextEdit[]` en orden inverso por posición para evitar desplazamientos, manteniendo intactos los cursores y el historial atómico de deshacer.

5. **Esquema de Símbolos del Documento / Outline (`textDocument/documentSymbol`, `Ctrl + Shift + R`)**:
   - Modal difuso interactivo para explorar la estructura y jerarquía del código.
   - Soporte para estructuras jerárquicas y planas con insignias sintácticas distintivas (`[fn]`, `[struct]`, `[enum]`, `[trait]`, `[method]`, `[mod]`, `[const]`, etc.).
   - Filtrado en tiempo real con salto inmediato a la declaración del símbolo.

6. **Inlay Hints (`textDocument/inlayHint`)**:
   - Consulta y renderiza anotaciones visuales en línea para nombres de parámetros (`param:`) y tipos inferidos (`: Type`).
   - Diseño estético de micro-píldora coloreada (`#7AA2F7` para tipos, `#BB9AF7` para parámetros).
   - **Aislamiento geométrico estricto**: Se renderizan visualmente sin desplazar los índices del Piece Table ni las columnas de caracteres, preservando la exactitud del cursor, selecciones multicursor y clics de ratón.
   - Conmutable en tiempo real desde el menú **Herramientas > Alternar Inlay Hints** y configurable en `config.ini` (`inlay_hints = true`).

7. **Resaltado Semántico de Compilador (`Semantic Tokens`, `textDocument/semanticTokens/full`, `F5`)**:
   - Negocia de forma dinámica capacidades semánticas con el servidor LSP durante el handshake `initialize`, parseando la leyenda provista (`tokenTypes` y `tokenModifiers`).
   - Decodifica secuencias relativas de deltas en 5-tuplas: `[deltaLine, deltaStartChar, length, tokenType, tokenModifiers]`.
   - Superpone clasificaciones semánticas precisas del compilador (funciones, métodos, structs, traits, macros, parámetros, constantes y campos) sobre el análisis léxico con paletas adaptadas a los 8 temas.

8. **Firma de Métodos Flotante (`Signature Help`, `textDocument/signatureHelp`, `Ctrl + Shift + Espacio`)**:
   - Muestra en tiempo real la firma del método o función que se está invocando con resaltado en negrita del parámetro activo y documentación asociada. Se activa automáticamente al escribir `(` o `,`.

9. **Navegación Semántica y Consulta de Tipos**:
   - **Ir a Definición (`F12`, `textDocument/definition`)**: Salta de inmediato al archivo y coordenadas exactas de la declaración.
   - **Hover (`Alt + K`, `textDocument/hover`)**: Muestra la firma de tipo, parámetros y documentación en la barra de estado.

10. **Ecosistema de 10 Servidores de Lenguaje Soportados**:
    - **Rust**: `rust-analyzer`
    - **Julia**: `julia --startup-file=no --history-file=no -e 'using LanguageServer; runserver()'`
    - **Python**: `pylsp`
    - **C / C++**: `clangd`
    - **TypeScript / JavaScript**: `typescript-language-server --stdio`
    - **Go**: `gopls`
    - **HTML / CSS**: `vscode-html-language-server --stdio`
    - **JSON**: `vscode-json-language-server --stdio`
    - **Markdown**: `marksman server`
    - **Zig**: `zls`
    - Totalmente configurables en la sección `[lsp]` de `config.ini`.

11. **Sincronización Incremental de Documentos**: Envía notificaciones automáticas `didOpen`, `didChange`, `didSave` y `didClose` para mantener el servidor sincronizado en tiempo real con las transacciones del editor.

### 6. Automatización al Guardar:
Configure en `config.ini` `format_on_save = true` o `check_on_save = true` para formatear y verificar el código automáticamente al presionar `Ctrl + S`.

---

## 12. Búsqueda Rápida de Archivos, Paleta de Comandos y Atajos de Teclado

Rustext ofrece un sistema dual de navegación y productividad instantánea:
- **Búsqueda Rápida de Archivos (Fuzzy File Finder, `Ctrl + P`)**: Permite localizar y abrir cualquier archivo en el espacio de trabajo de manera inmediata mediante un motor difuso multialineamiento. Muestra insignias de tipo de archivo (`[RS]`, `[MD]`, `[TOML]`, etc.), resalta en color dorado los caracteres coincidentes en el nombre y en la ruta de directorios, y prioriza pestañas actualmente abiertas e historial de archivos recientes.
- **Paleta de Comandos (`Ctrl + Shift + P`)**: Permite buscar y ejecutar cualquier acción interna o comando de extensión mediante filtrado difuso.
- **Transición Dinámica**: Al presionar `Ctrl + P`, escribir el prefijo `>` transforma al instante el modal en la Paleta de Comandos sin cerrar la ventana.

### Tabla Completa de Atajos de Teclado:

| Atajo de Teclado | Acción | Menú Correspondiente |
| :--- | :--- | :--- |
| `Ctrl + P` | **Ir a Archivo (Fuzzy File Finder)** | Ver > Ir a Archivo... |
| `Ctrl + Shift + P`| **Abrir Paleta de Comandos** | Ver > Paleta de Comandos... |
| `Ctrl + S` | Guardar archivo actual | Archivo > Guardar |
| `Ctrl + O` | Abrir diálogo de archivo | Archivo > Abrir Archivo... |
| `Ctrl + Shift + O` | **Abrir Carpeta de Proyecto / Workspace** | Archivo > Abrir Carpeta de Proyecto... |
| `Ctrl + Shift + P` / `Ctrl + Alt + W` | **Cambiar Espacio de Trabajo / Proyectos Recientes** | Archivo > Cambiar Espacio de Trabajo... |
| `Ctrl + R` | **Abrir Archivo Reciente...** | Archivo > Abrir Archivo Reciente... |
| `Ctrl + T` | Abrir nueva pestaña | Archivo > Nueva Pestaña |
| `Ctrl + W` | Cerrar pestaña actual | Archivo > Cerrar Pestaña |
| `Ctrl + Q` | Salir de Rustext | Archivo > Salir |
| `Ctrl + Z` | Deshacer (Undo) | Editar > Deshacer |
| `Ctrl + Y` | Rehacer (Redo) | Editar > Rehacer |
| `Ctrl + X` | Cortar selección o línea | Editar > Cortar |
| `Ctrl + C` | Copiar selección o línea | Editar > Copiar |
| `Ctrl + V` | Pegar desde el portapapeles | Editar > Pegar |
| `Alt + Up` | **Mover Línea Arriba** | Editar > Mover Línea Arriba |
| `Alt + Down` | **Mover Línea Abajo** | Editar > Mover Línea Abajo |
| `Shift + Alt + Down` / `Ctrl + Shift + D` | **Duplicar Línea Abajo** | Editar > Duplicar Línea Abajo |
| `Ctrl + /` / `Ctrl + _` | **Alternar Comentario de Línea** | Editar > Alternar Comentario |
| `Ctrl + F` | Buscar en el documento | Editar > Buscar... |
| `Ctrl + H` | Buscar y Reemplazar | Editar > Reemplazar... |
| `Ctrl + Shift + F`| **Búsqueda Global en el Proyecto** | Editar > Buscar en el Proyecto... |
| `Ctrl + Espacio` | **Autocompletado Contextual Inteligente** | Editar > Autocompletar |
| `Ctrl + D` | Seleccionar siguiente ocurrencia | Selección > Siguiente Ocurrencia |
| `Ctrl + K` | Añadir cursor arriba | Selección > Añadir Cursor Arriba |
| `Ctrl + J` | Añadir cursor abajo | Selección > Añadir Cursor Abajo |
| `Esc` | Cursor único / Cancelar modal | Selección > Cursor Único |
| `Ctrl + B` | Alternar Explorador Lateral | Ver > Explorador Lateral |
| `Alt + Left` | **Saltar Atrás (Pila de Navegación Jump List)** | Ver > Saltar Atrás |
| `Alt + Right` | **Saltar Adelante (Pila de Navegación Jump List)** | Ver > Saltar Adelante |
| `Alt + Z` | **Alternar Ajuste de Línea (Word Wrap)** | Ver > Ajuste de Línea (Word Wrap) |
| `Alt + I` | **Alternar Guías de Indentación** | Ver > Guías de Indentación |
| `Alt + [` / `F9` | **Plegar/Desplegar Bloque (Toggle Fold)** | Ver > Plegar/Desplegar Bloque |
| `Alt + 0` | **Plegar Todo (Fold All)** | Ver > Plegar Todo |
| `Alt + J` | **Desplegar Todo (Unfold All)** | Ver > Desplegar Todo |
| `Ctrl + M` | **Alternar Minimapa Gráfico** | Ver > Minimapa |
| `Ctrl + \`` / `Ctrl + ~` | **Alternar Terminal Integrado Nativo** | Ver > Terminal Integrado |
| `Ctrl + \` | **Alternar División Vertical (Split)** | Ver > Dividir Verticalmente |
| `Ctrl + Shift + \`| **Alternar División Horizontal** | Ver > Dividir Horizontalmente |
| `Alt + \` | **Cambiar Foco de Panel Dividido** | Ver > Cambiar Foco de Panel |
| `PageUp` | Cambiar a pestaña anterior | Ver > Pestaña Anterior |
| `PageDown` | Cambiar a pestaña siguiente | Ver > Pestaña Siguiente |
| `Ctrl + Shift + T`| **Selector de Fuentes y Tipografía (Modal con Vista Previa)** | Preferencias > Tipografía... |
| `Ctrl + +` / `Ctrl + =` | **Aumentar Tamaño de Fuente (Zoom In)** | Preferencias > Aumentar Tamaño |
| `Ctrl + -` | **Disminuir Tamaño de Fuente (Zoom Out)** | Preferencias > Disminuir Tamaño |
| `Ctrl + 0` | **Restablecer Tamaño de Fuente (Reset Zoom a 12pt)** | Preferencias > Restablecer Tamaño |
| `Ctrl + Shift + X`| Administrador de Extensiones | Extensiones > Administrador... |
| `Ctrl + Shift + G`| **Actualizar Estado Git (Git Gutter)** | Herramientas > Actualizar Estado Git |
| `Alt + H` | **Git: Inspeccionar Git Hunk Diff** | Herramientas > Inspeccionar Git Hunk Diff |
| `Alt + R` | **Git: Revertir Git Hunk Diff** | Herramientas > Revertir Git Hunk Diff |
| `Ctrl + Shift + U`| **LSP: Buscar Referencias en el Workspace** | Herramientas > Buscar Referencias |
| `F2` | **LSP: Renombrar Símbolo en Todo el Workspace** | Herramientas > Renombrar Símbolo |
| `Ctrl + .` | **LSP: Acciones Rápidas y Quick-Fixes** | Herramientas > Acciones Rápidas (Code Actions) |
| `Ctrl + Shift + R`| **LSP: Esquema / Símbolos del Documento** | Herramientas > Esquema / Símbolos del Documento |
| `Ctrl + Shift + I`| **LSP / Formateo: Formatear Documento** | Herramientas > Formatear Documento |
| `F12` | **LSP: Ir a Definición** | Herramientas > LSP: Ir a Definición |
| `Alt + K` | **LSP: Información de Tipo (Hover)** | Herramientas > LSP: Información de Tipo (Hover) |
| `Ctrl + Shift + Espacio` | **LSP: Firma de Métodos (Signature Help)** | Herramientas > LSP: Firma de Métodos |
| `F5` | **LSP: Refrescar Tokens Semánticos** | Herramientas > Refrescar Tokens Semánticos (LSP) |
| `F7` | Comprobar Errores (Check / Lint)| Herramientas > Comprobar Errores |
| `F8` | Saltar al Siguiente Error | Herramientas > Siguiente Error |
| `Shift + F8` | Saltar al Error Anterior | Herramientas > Error Anterior |
| `F1` | **Ventana Acerca de Rustext (Logotipo Faceted Ribbon R)** | Ayuda > Acerca de Rustext |

---

## 13. Sistema de Configuración (`config.ini`)

Rustext almacena su configuración en un archivo INI estándar:
- **Linux / macOS**: `~/.config/rustext/config.ini`
- **Windows**: `%APPDATA%\rustext\config.ini`

El archivo se recarga en caliente de forma automática cuando se guarda.

### Ejemplo Completo de `config.ini`:

```ini
[editor]
tab_size = 4
auto_indent = true
auto_close_brackets = true
auto_surround = true
show_line_numbers = true
word_wrap = false            # Ajuste de línea envolvente (Soft Wrap, conmutable con Alt+Z)
format_on_save = false
check_on_save = false
restore_session = true       # Restaurar pestañas, cursores y split al iniciar
auto_save_session = true     # Guardar estado de sesión automáticamente al salir
hot_exit = false             # Preservar borradores de cambios no guardados al salir (Hot Exit)
max_recent_files = 20        # Número máximo de archivos recientes en historial

[theme]
# Opciones integradas: tokyo-night, catppuccin-mocha, one-dark, modern-dark, monokai, solarized-dark, light, dracula
# O cualquier archivo de tema personalizado (.theme) ubicado en ~/.rustext/themes/ o ./.rustext/themes/
name = tokyo-night

[gui]
window_width = 1100
window_height = 720
scale_factor = 0.0        # Factor de escala HiDPI (0.0 = autodetección por monitor/OS, 1.0, 1.5, 2.0)
font_scale = 1
font_family = standard   # standard, terminus, modern, typewriter, retro
font_size = 12           # 10, 11, 12, 14, 16, 18, 20, 24 pt
font_weight = normal     # normal, semibold, bold
line_spacing = 3         # Interlineado vertical en píxeles: 1, 3, 5, 8
cursor_style = bar       # Opciones: bar, block, underline
cursor_blink = true
rounded_corners = true
shadows = true
scrollbar = true
menu_bar = true
sidebar = true
sidebar_width = 220
minimap = true           # Mostrar minimapa gráfico de código lateral
minimap_width = 100      # Ancho del minimapa en píxeles (default: 100)
git_gutter = true        # Mostrar diff de líneas en el gutter (+ verde, ~ azul, ▲ rojo)
font_antialiasing = true # Suavizado de fuentes tipográficas con anti-aliasing espacial
gamma_correction = true  # Composición física de color con corrección gamma IEC 61966-2-1
font_ligatures = true    # Activar ligaduras de programación continuas (->, =>, !=, ==, etc.)
font_hinting = medium    # Nivel de alineación a la rejilla de píxeles: none, slight, medium, full
smooth_shadows = true    # Sombras suaves gaussianas O(1) y efecto frosted glass acrílico
animation_enabled = true # Animaciones fluidas, easing y físicas de resorte
edge_perception = true   # Ajuste adaptativo de contraste perceptual de bordes (Weber-Fechner)

[lsp]
enabled = true           # Integración con servidores LSP (Language Server Protocol)
auto_start = true        # Iniciar automáticamente el servidor según el lenguaje activo
rust_server = rust-analyzer  # Binario del servidor de lenguaje para Rust
python_server = pylsp        # Binario del servidor de lenguaje para Python
c_server = clangd            # Binario del servidor de lenguaje para C / C++
typescript_server = typescript-language-server --stdio # TypeScript / JavaScript
go_server = gopls            # Binario del servidor para Go
html_css_server = vscode-html-language-server --stdio  # HTML / CSS
json_server = vscode-json-language-server --stdio      # JSON
markdown_server = marksman server # Markdown
zig_server = zls             # Zig
julia_server = julia --startup-file=no --history-file=no -e 'using LanguageServer; runserver()' # Julia
inlay_hints = true           # Mostrar sugerencias de tipo y parámetros en línea (Inlay Hints)
```

---

## 14. Sistema de Plugins, Temas y Extensiones

Rustext proporciona una arquitectura extensible sin sacrificar la filosofía de cero dependencias externas ni arriesgar la estabilidad del editor.

> [!NOTE]
> Para una guía detallada paso a paso con tutoriales en Python, Bash y Node.js, consulte el [**Manual de Creación de Temas y Extensiones**](file:///home/bolek/Proyectos/rustext/MANUAL_TEMAS_Y_EXTENSIONES.md).

### 1. Administrador Visual de Extensiones (`Ctrl + Shift + X`)
* Presione `Ctrl + Shift + X` (o acceda desde **Extensiones > Administrador de Extensiones...**).
* **`Espacio`**: Activar / Desactivar la extensión seleccionada (se persiste en disco automáticamente).
* **`R`**: Recargar todas las extensiones en caliente desde disco.
* **`N`**: Crear una extensión de ejemplo lista para modificar.

### 2. Extensiones por Manifiesto (`extension.conf`):
Rustext descubre extensiones automáticamente en:
* **Global de usuario**: `~/.rustext/extensions/<nombre_extension>/`
* **Local de espacio de trabajo**: `./.rustext/extensions/<nombre_extension>/`

Estructura de `extension.conf`:
```ini
id = mi_herramienta
name = Mi Herramienta Personalizada
version = 1.0.0
author = Desarrollador
description = Transforma o formatea texto vía subprocesos aislados
enabled = true
commands = mi_comando:Ejecutar Mi Herramienta
exec = python3
args = run.py
on_save = false
```

Al abrir la Paleta de Comandos (`Ctrl + P`), aparecerá automáticamente como:
`[Ext] Ejecutar Mi Herramienta`

### 3. Extensiones en el Proceso (`Plugin` Trait):
Para integraciones ultra-rápidas en el código nativo:
```rust
pub trait Plugin {
    fn name(&self) -> &str;
    fn on_init(&mut self, ctx: &mut PluginContext) {}
    fn on_open(&mut self, ctx: &mut PluginContext, path: &Option<PathBuf>) {}
    fn on_before_save(&mut self, ctx: &mut PluginContext, content: &mut String) {}
    fn on_after_save(&mut self, ctx: &mut PluginContext) {}
    fn on_command(&mut self, ctx: &mut PluginContext, command: &str) -> bool { false }
}
```

### 4. Sistema Dinámico de Temas (`.theme`) y Recarga en Caliente

Rustext incluye un motor declarativo de temas que permite personalizar cualquier color del editor y del resaltado sintáctico sin necesidad de recompilar ni editar el código fuente:

* **8 Paletas Oficiales Modularizadas**: Los temas oficiales se encuentran modularizados en archivos independientes dentro de [`assets/themes/`](assets/themes/):
  - `tokyo-night.theme`: Azul profundo, violetas y cian neón.
  - `catppuccin-mocha.theme`: Tonos pastel cálidos sobre fondo chocolate oscuro (`#1e1e2e`).
  - `one-dark.theme`: Estilo clásico Atom / One Dark Pro equilibrado.
  - `modern-dark.theme`: Oscuro contemporáneo minimalista (`#1e1e1e`).
  - `monokai.theme`: Alto contraste con amarillo canario, magenta y verde lima.
  - `solarized-dark.theme`: Clásico azul verdoso (`#002b36`) con contrastes matemáticos Ethan Schoonover.
  - `light.theme`: Fondo blanco nítido tipo GitHub Light para entornos luminosos.
  - `dracula.theme`: Fondo gótico (`#282A36`) con acentos púrpura vampírico, verde y cian.

* **Auto-Despliegue de Plantillas en Primer Arranque**: Al iniciarse por primera vez, si el directorio de usuario `~/.rustext/themes/` está vacío, Rustext copia automáticamente las 8 plantillas oficiales desde los recursos integrados del binario, dejándolas listas para su inspección y modificación directa.

* **Prioridad de Anulación Dinámica (Override)**: Si existe un archivo `.theme` con el mismo nombre en `./.rustext/themes/` (workspace) o en `~/.rustext/themes/` (usuario), Rustext lo carga con máxima prioridad, reemplazando el tema del binario al instante sin alterar el ejecutable.

* **Sintaxis Declarativa de Temas (`.theme` / `.ini`)**:
  - Utiliza sintaxis INI estándar con secciones opcionales `[meta]`, `[ui]`, `[syntax]` y `[brackets]`.
  - Soporta valores hexadecimales flexibles: `#RRGGBB`, `0xRRGGBB`, `#RGB` y comentarios en línea con `;` o ` #`.
  - La propiedad `type = dark` o `type = light` determina los colores de respaldo automáticos para cualquier propiedad omitida.

* **Selección y Recarga Inmediata**:
  - **Paleta de Comandos (`Ctrl + P`)**: Escriba `Tema:` para ver y aplicar tanto los temas oficiales como los descubiertos dinámicamente en disco (etiquetados como `(Personalizado)`).
  - **Menú Gráfico**: Vaya a **Preferencias > Tema** para conmutar rápidamente.
  - **Recarga en Caliente**: Modifique `name = <tema>` en `~/.config/rustext/config.ini`; Rustext recarga y repinta el tema al guardar sin reiniciar el editor.

> [!TIP]
> Para consultar la especificación completa de todos los slots cromáticos, ejemplos listos para copiar y la guía de creación de temas, consulte el [**Manual de Creación de Temas y Extensiones**](file:///home/bolek/Proyectos/rustext/MANUAL_TEMAS_Y_EXTENSIONES.md).

---

## 15. Perfil de Memoria y Benchmarks Comparativos

Para validar el rendimiento, la latencia y la huella en memoria de Rustext frente a los editores más representativos de la industria, se diseñaron dos suites integradas:
1. **Benchmark Comparativo en Vivo contra Otros Editores** ([`src/bin/benchmark_comparison.rs`](file:///home/bolek/Proyectos/rustext/src/bin/benchmark_comparison.rs)): Mide tiempos de arranque en frío, consumo de RAM física (`VmRSS`), throughput de carga de archivos masivos, latencia de edición por pulsación y huella binaria en disco frente a VS Code (Code-OSS), Sublime Text 4, Vim y Micro.
2. **Profiling de Memoria Heap y RAM Física bajo Cargas Extremas** ([`src/bin/memory_profile.rs`](file:///home/bolek/Proyectos/rustext/src/bin/memory_profile.rs)): Instrumenta el asignador global de memoria estándar `TrackingAllocator` para medir byte por byte el heap y las llamadas al kernel (`/proc/self/status`) con archivos desde 1 KB hasta 100 MB.

```bash
# 1. Ejecutar el Benchmark Comparativo contra otros editores:
cargo run --release --bin benchmark_comparison

# 2. Ejecutar el Profiling Exhaustivo de Memoria Heap y Cargas Masivas:
cargo run --release --bin memory_profile
```

### 🏆 Tabla Comparativa: Rustext vs Editores de la Industria (Mediciones Reales en Linux x86_64)

| Métrica / Atributo | **Rustext** | **VS Code (Code-OSS)** | **Sublime Text 4** | **Vim 9.2** | **Micro** | **Helix 24.07** |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Tecnología Base** | **Rust puro (`std::*`)** | Electron / Node / Chromium | C++ nativo / Python | C nativo | Go runtime | Rust / tree-sitter |
| **Dependencias Externas**| **0 (Cero crates)** | >1,000 dependencias npm | Cerrado / Propietario | libc, ncurses | Go stdlib + deps | ~140 crates |
| **Tamaño Binario / Disco**| **~688 KB** | **~824 MB** (~1,200x) | ~45 MB | ~6.4 MB | ~13.0 MB | ~24.5 MB |
| **Inicio en Frío (Cold Start)**| **0.7 µs - 2.5 ms** | 1,500 - 2,500 ms (~1,000x) | 80 - 150 ms | 8 - 15 ms | 20 - 45 ms | 12 - 25 ms |
| **Memoria RAM en Reposo**| **2.6 - 3.1 MB** (36 MB GUI) | ~350 - 450 MB | ~65 - 85 MB | ~9.4 MB | ~22.3 MB | ~18.5 MB |
| **Memoria con Archivo 25 MB**| **29.0 MB** | >600 MB (latencia alta) | ~60 MB | ~38 MB | ~95 MB | ~85 MB |
| **Throughput Carga (25 MB)** | **488.6 MB/s** | ~25 MB/s (alerta archivo grande) | ~180 MB/s | ~120 MB/s | ~65 MB/s | ~210 MB/s |
| **Latencia de Inserción**| **< 20 µs** | ~10 - 25 ms | < 1 ms | < 1 ms | < 2 ms | < 1 ms |
| **Rendimiento Viewport**| **~95 a 1,400+ FPS** | 60 FPS (tope VSync) | 60 - 120 FPS | N/A (TUI) | N/A (TUI) | N/A (TUI) |
| **Tiempo Compilación Limpia**| **~2.0 segundos** | Varios minutos | N/A (cerrado) | ~30 segundos | ~15 segundos | ~1 min 45 s |

---

### Metodología de Medición:
- **Heap Asignado**: Asignador atómico estándar `TrackingAllocator` que intercepta `alloc`, `realloc` y `dealloc` con precisión de 1 byte.
- **Memoria Física del SO**: Lectura directa de `VmRSS` y `VmHWM` desde el kernel de Linux (`/proc/self/status`).
- **Temporizador**: `std::time::Instant` de precisión nanosegundo.
- **Entorno de Prueba**: Linux x86_64, compilación `--release` (optimizaciones `opt-level = 3`).

### 1. Resultados de Uso de Memoria (Heap vs RAM Física):

| Categoría | Escenario | Tamaño Archivo | Líneas | Memoria Heap Neta | Memoria Heap Pico | Ratio Memoria (Heap / Archivo) | VmRSS SO (RAM Física) |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Archivos Pequeños** | Script / Config (1 KB) | 1.00 KB | 21 | **1.28 KB** | 1.28 KB | **1.28x** | 2.67 MB |
| | Módulo Típico (10 KB) | 10.00 KB | 204 | **12.03 KB** | 12.03 KB | **1.20x** | 2.80 MB |
| | Módulo Extenso (50 KB) | 50.00 KB | 1,013 | **58.03 KB** | 58.03 KB | **1.16x** | 4.64 MB |
| **Uso Normal** | Archivo Mediano (500 KB) | 500.00 KB | 10,131 | **628.03 KB** | 628.03 KB | **1.26x** | 4.78 MB |
| | Bundle / Dataset (2 MB) | 2.00 MB | 41,497 | **2.50 MB** | 2.50 MB | **1.25x** | 8.30 MB |
| | Dump de Código (10 MB) | 10.00 MB | 207,481 | **12.00 MB** | 12.00 MB | **1.20x** | 24.40 MB |
| **Archivos Enormes** | Log / DB Dump (25 MB) | 25.00 MB | 518,702 | **29.00 MB** | 29.00 MB | **1.16x** | 58.23 MB |
| | Servidor / Dump (50 MB) | 50.00 MB | 1,036,678 | **58.00 MB** | 58.00 MB | **1.16x** | 112.60 MB |
| | **Extremo (100 MB)** | **100.00 MB** | **2,053,953** | **116.00 MB** | **116.00 MB** | **1.16x** | **220.55 MB** |

### 2. Resultados de Latencias y Tiempos de Respuesta:

| Escenario | Tamaño | Tiempo Carga + Indexado | Throughput | Búsqueda Coordenadas | Edición 1 Carácter | 10 Cursores Simultáneos | Renderizado Viewport |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **1 KB** | 1.00 KB | **7.1 µs** | 136.9 MB/s | 9.1 ns | 8.1 µs | 18.9 µs | 599.6 µs (~1,667 FPS) |
| **10 KB** | 10.00 KB | **23.1 µs** | 422.9 MB/s | 13.8 ns | 23.8 µs | 162.2 µs | 806.6 µs (~1,240 FPS) |
| **50 KB** | 50.00 KB | **79.1 µs** | 617.3 MB/s | 18.1 ns | 125.7 µs | 845.2 µs | 928.0 µs (~1,077 FPS) |
| **500 KB** | 500.00 KB | **809.8 µs** | 603.0 MB/s | 24.9 ns | 1.18 ms | 7.89 ms | 1.35 ms (~740 FPS) |
| **2 MB** | 2.00 MB | **8.32 ms** | 240.4 MB/s | 75.9 ns | 6.37 ms | 33.46 ms | 621.1 µs (~1,610 FPS) |
| **10 MB** | 10.00 MB | **24.38 ms** | 410.2 MB/s | 348.8 ns | 24.07 ms | 163.24 ms | 647.4 µs (~1,544 FPS) |
| **25 MB** | 25.00 MB | **55.20 ms** | 452.9 MB/s | 365.2 ns | 44.62 ms | 363.49 ms | 542.0 µs (~1,845 FPS) |
| **50 MB** | 50.00 MB | **103.89 ms** | 481.3 MB/s | 476.6 ns | 78.63 ms | 799.55 ms | 651.5 µs (~1,535 FPS) |
| **100 MB** | **100.00 MB** | **258.67 ms** | **386.6 MB/s** | **556.7 ns** | **249.66 ms** | **1.51 s** | **638.0 µs (~1,567 FPS)** |

### 3. Prueba de Sesión Realista Multi-Pestaña (5 Pestañas Abiertas):
- Pestañas activas: `main.rs` (15 KB), `config.ini` (2 KB), `parser.rs` (45 KB), `large_module.rs` (250 KB), `data_dump.json` (1.00 MB).
- **Memoria Heap Neta de las 5 Pestañas**: **1.64 MB** (~336 KB por pestaña promedio).
- **Memoria Física del Proceso Completo**: **36.30 MB** (incluye framebuffer gráfico 800×600 a 32-bit de 1.9 MB, glifos, menús, árbol de proyecto e historial).
- **Tiempo de Dibujado Completo**: **1,128 µs** (equivalente a **~886 FPS** sostenidos con interfaz completa).

---

## 16. Preguntas Frecuentes y Solución de Problemas

### ¿Por qué Rustext no utiliza bibliotecas de ventana como `winit` o `SDL`?
Para eliminar cientos de dependencias de compilación y asegurar que el binario compile en cualquier máquina en 2 segundos sin requerir gestores de paquetes externos.

### ¿Cómo fuerzo el modo terminal si estoy en un entorno con servidor gráfico?
Ejecute simplemente `./target/release/rustext` sin banderas, o `./target/release/rustext --tui`. El modo gráfico se activa con `-g` o `--gui`.

### ¿Cómo cambio el tema visual?
Abra el menú superior **Preferencias > Tema** o agregue `name = catppuccin-mocha` en su archivo `config.ini`.

---

## 17. Motor de Cajas Flexibles (Flexbox) y Paradigmas de UI (Reactivo vs. Retained)

Rustext incorpora un motor de diseño de interfaces modular y declarativo implementado íntegramente en **100% Rust estándar (`std::*`)**, sin librerías externas.

### 17.1 El Motor de Cajas Flexibles (Flexbox Layout Engine)

Inspirado en la especificación W3C CSS Flexible Box Layout, el módulo [`rustext::ui::flex`](file:///home/bolek/Proyectos/rustext/src/ui/flex.rs) desacopla la geometría rígida de coordenadas fijas, permitiendo que los componentes de la interfaz se adapten fluidamente a cualquier resolución o factor de escala HiDPI.

#### 1. Modelo Bidimensional de Ejes:
- **Eje Principal (*Main Axis*)**: Dirección a lo largo de la cual se apilan los elementos flexibles:
  - `FlexDirection::Row`: Izquierda a derecha.
  - `FlexDirection::RowReverse`: Derecha a izquierda.
  - `FlexDirection::Column`: Arriba a abajo.
  - `FlexDirection::ColumnReverse`: Abajo a arriba.
- **Eje Transversal (*Cross Axis*)**: Perpendicular al eje principal (vertical en `Row`, horizontal en `Column`).

#### 2. Distribución y Flexibilidad:
- **`flex_grow`**: Proporción en la que un elemento absorbe el espacio positivo libre sobrante en el contenedor:
  $$\text{espacio\_extra}_i = \text{espacio\_libre} \cdot \frac{\text{grow}_i}{\sum \text{grow}}$$
- **`flex_shrink`**: Proporción en la que un elemento se comprime ante un desbordamiento negativo:
  $$\text{reducción}_i = \text{desbordamiento} \cdot \frac{\text{shrink}_i \cdot \text{basis}_i}{\sum (\text{shrink}_k \cdot \text{basis}_k)}$$
- **`flex_basis`**: Dimensión base inicial antes de aplicar distribución (`Auto`, `Points(f32)`, `Percent(f32)`).
- **`gap` / `row_gap` / `col_gap`**: Separación física entre elementos adyacentes sin colisión de márgenes.

#### 3. Justificación y Alineación:
- **`JustifyContent`** (Eje Principal): `FlexStart`, `Center`, `FlexEnd`, `SpaceBetween`, `SpaceAround`, `SpaceEvenly`.
- **`AlignItems`** y **`AlignSelf`** (Eje Transversal): `Stretch` (estiramiento automático a la altura/anchura de la línea), `Center`, `FlexStart`, `FlexEnd`, `Baseline`.
- **`FlexWrap`** y **`AlignContent`**: Envoltura en múltiples líneas flexibles (`NoWrap`, `Wrap`, `WrapReverse`) y distribución vertical de las líneas resultantes.

---

### 17.2 Patrones de Interfaz: Immediate vs. Retained vs. Reactivo

| Característica | Immediate Mode GUI (IMGUI) | Retained Mode GUI (RMGUI) | Fine-Grained Reactive (Reactivo) | Enfoque Híbrido de Rustext |
| :--- | :--- | :--- | :--- | :--- |
| **Persistencia del Árbol** | No existe. Se reconstruye y reejecuta cada frame. | Árbol pesado y persistente de objetos en memoria. | Grafo de dependencias de señales; componentes como closures. | **Árbol Retenido (`WidgetTree`) persistente**. |
| **Frecuencia de Cómputo** | Constante a 60/120 FPS independientemente de cambios. | Por invalidación manual o eventos de sistema. | Por cambio de versión en señales individuales ($\mathcal{O}(1)$). | **Invalidación selectiva reactiva con `DirtyFlags`**. |
| **Gestión de Foco y Scroll** | Requiere trucos de cache/ID stack por cuadro. | Nativa y retenida en el nodo del widget. | Enlazada a señales de estado de la aplicación. | **Retenida en el widget y sincronizada por señales**. |
| **Complejidad de Layout** | Difícil para dos pasadas (shrink-to-fit requiere delay). | Natural, cacheado en nodos persistentes. | Recalculado cuando cambian dimensiones observadas. | **Resolución Flexbox de 2 pasadas cacheada**. |
| **Reconciliación** | Inexistente (siempre redibuja). | Inexistente (mutación imperativa en árbol). | Inexistente (sin Virtual DOM; reacciona a nivel de dato). | **Cero Virtual DOM; sin diffing recursivo de árboles**. |

#### La Síntesis Arquitectónica en Rustext:
1. **Primitivas Reactivas de Grano Fino** ([`rustext::ui::reactive`](file:///home/bolek/Proyectos/rustext/src/ui/reactive.rs)):
   - `Signal<T>`: Contenedor reactivo con versionado de épocas y lista de suscriptores directos.
   - `Computed<T>` (*Memos*): Valores derivados cacheados que se evalúan perezosamente (*lazy*) únicamente cuando la versión de la señal origen avanza.
   - `Effect`: Disparador de efectos colaterales automáticos ante mutaciones de estado.
2. **Árbol de Widgets Retenido** ([`rustext::ui::retained`](file:///home/bolek/Proyectos/rustext/src/ui/retained.rs)):
   - `WidgetNode` y `WidgetTree`: Mantienen identidades persistentes (`WidgetId`), estado de hover, foco y rectángulos de layout (`Rect`).
   - Los widgets enlazan sus propiedades a señales reactivas (`bind_text`, etc.).
   - Cuando una señal cambia, sólo los nodos dependientes marcan `dirty.layout` o `dirty.paint` y lo propagan hacia arriba, garantizando que el pipeline gráfico sólo procese lo estrictamente necesario sin pérdida de rendimiento.

---

## 18. Auditoría Integral de Seguridad, Modelo de Amenazas y Directrices Defensivas

Rustext ha sido sometido a una auditoría estricta de seguridad de software y modelado de amenazas, evaluando su código base frente a los estándares de la industria (taxonomía CWE de MITRE y directrices OWASP). A continuación se documenta el análisis detallado de la postura defensiva, la matriz de hallazgos técnicos y las recomendaciones de endurecimiento (*hardening*).

### 18.1 Postura de Seguridad y Análisis de Cadena de Suministro (*Supply Chain Security*)

Uno de los vectores de ataque más críticos en el desarrollo de software moderno es el compromiso de la cadena de suministro (*supply chain attacks*), ejemplificado por incidentes notorios en ecosistemas de paquetes como npm (p. ej., `event-stream`), PyPI o crates.io (ataques de *typosquatting*, robo de tokens de publicación o inyección de código malicioso en dependencias transitivas).

1. **Superficie de Ataque Nula en Dependencias Externas**:
   - `Cargo.toml` de Rustext no contiene dependencias (`dependencies` está vacío).
   - El archivo `Cargo.lock` no gestiona ningún árbol de paquetes externos ni crates transitivos.
   - La superficie de ataque externa por terceros es exactamente del **0.0%**. Ninguna actualización inadvertida de dependencias de terceros puede introducir puertas traseras, código ofuscado o llamadas de telemetría no consentidas.

2. **Seguridad de Memoria por Construcción (*Memory Safety*)**:
   - La totalidad del núcleo editorial ([`PieceTable`](file:///home/bolek/Proyectos/rustext/src/core/piece_table.rs), [`Buffer`](file:///home/bolek/Proyectos/rustext/src/core/buffer.rs), [`Highlighter`](file:///home/bolek/Proyectos/rustext/src/syntax/mod.rs), [`FlexEngine`](file:///home/bolek/Proyectos/rustext/src/ui/flex.rs), [`ReactiveEngine`](file:///home/bolek/Proyectos/rustext/src/ui/reactive.rs)) está escrita en **100% Safe Rust**.
   - Quedan erradicadas por definición del sistema de tipos y borrow checker clases enteras de vulnerabilidades críticas:
     - Desbordamientos de búfer en memoria (*Buffer Overflows* - CWE-120).
     - Uso tras liberación (*Use-After-Free* - CWE-416).
     - Liberaciones dobles (*Double Free* - CWE-415).
     - Punteros nulos o colgantes (*Null/Dangling Pointer Dereference* - CWE-476).
     - Condiciones de carrera de datos (*Data Races* - CWE-362).
   - Las escasas secciones `unsafe` del proyecto están estrictamente limitadas y aisladas en las capas de binding del sistema de ventanas ([`x11.rs`](file:///home/bolek/Proyectos/rustext/src/gui/x11.rs), [`win32.rs`](file:///home/bolek/Proyectos/rustext/src/gui/win32.rs), [`macos.rs`](file:///home/bolek/Proyectos/rustext/src/gui/macos.rs)), donde cada llamada FFI está envuelta en guardas seguras con verificación explícita de punteros nulos.

---

### 18.2 Matriz Exhaustiva de Hallazgos y Vulnerabilidades Analizadas

Durante la auditoría de código se identificaron 10 puntos de interés técnico clasificados por severidad, impacto potencial y taxonomía CWE:

| ID | Severidad | Debilidad / Vulnerabilidad | CWE | Vector de Ataque Teórico | Impacto Potencial |
| :---: | :---: | :--- | :---: | :--- | :--- |
| **SEC-01** | **Media** | Carga no confiable de extensiones de espacio de trabajo | [CWE-829](https://cwe.mitre.org/data/definitions/829.html)<br>[CWE-94](https://cwe.mitre.org/data/definitions/94.html) | Clonar un repositorio público que contenga un script malicioso en `./.rustext/extensions/`. Al abrir el proyecto, Rustext descubre y expone comandos del script. | Ejecución remota de código (RCE) en el contexto del usuario si se dispara el comando sin advertencia previa. |
| **SEC-02** | **Media** | Guardado de archivos no atómico y ventana de truncamiento | [CWE-377](https://cwe.mitre.org/data/definitions/377.html)<br>[CWE-459](https://cwe.mitre.org/data/definitions/459.html) | Invocación de `File::create(path)` seguida de escritura. Si el proceso sufre un corte de energía, disco lleno o `kill -9` tras el `truncate(0)` pero antes del fin de escritura, el archivo se destruye. | Pérdida irreparable de datos o corrupción de archivos fuente críticos del usuario. |
| **SEC-03** | **Baja** | Inyección de argumentos CLI en portapapeles Wayland | [CWE-88](https://cwe.mitre.org/data/definitions/88.html) | Al copiar texto en Wayland, Rustext invoca `Command::new("wl-copy").arg(text)`. Si el texto excede `ARG_MAX` (~2MB en Linux) o comienza con prefijos de opciones CLI (`--`), puede fallar o alterar flags de `wl-copy`. | Denegación de servicio en portapapeles o ejecución inesperada de banderas de la utilidad de sistema. |
| **SEC-04** | **Baja** | Agotamiento de recursos por bucles de enlaces simbólicos | [CWE-674](https://cwe.mitre.org/data/definitions/674.html)<br>[CWE-59](https://cwe.mitre.org/data/definitions/59.html) | En `GlobalSearchEngine`, la recursión de directorios con `read_dir` puede seguir symlinks recursivos hacia carpetas padre o cíclicas (p. ej. `link -> .`). | Consumo desmedido de descriptores de archivo, desbordamiento de pila o congelamiento de la interfaz (DoS local). |
| **SEC-05** | **Baja** | Deserialización de rutas de borrador sin validación de contención | [CWE-22](https://cwe.mitre.org/data/definitions/22.html) | Un archivo de sesión manipulado (`session.ini`) con una clave `draft_path = "../../../../etc/shadow"` podría inducir lectura o escritura involuntaria de archivos fuera del directorio de borradores. | Travesía de directorios (*Path Traversal*) local ante sesiones alteradas manualmente o por malware. |
| **SEC-06** | **Info** | Consola en modo crudo (*raw*) tras pánicos no recuperados | [CWE-755](https://cwe.mitre.org/data/definitions/755.html) | Si un pánico inesperado ocurre dentro del bucle de eventos TUI sin alcanzar el bloque destructor RAII (`Drop`), la terminal queda sin eco local y en modo raw. | Molestia operacional que obliga al usuario a escribir a ciegas el comando `reset` o cerrar la terminal. |
| **SEC-07** | **Baja** | Creación insegura de archivos temporales en memoria compartida (shm) | [CWE-377](https://cwe.mitre.org/data/definitions/377.html)<br>[CWE-59](https://cwe.mitre.org/data/definitions/59.html) | En el backend Wayland, crear buffers `wl_shm` con nombres predecibles en `/dev/shm` susceptible a enlaces simbólicos maliciosos. | Condición de carrera de enlace simbólico (*symlink race*) o colisión de nombres de archivo temporal. |
| **SEC-08** | **Media** | Travesía de directorios en el cargador dinámico de temas | [CWE-22](https://cwe.mitre.org/data/definitions/22.html) | Un nombre de tema malicioso en `config.ini` o entrada interactiva con secuencias de escape (`../../../../etc/passwd`) o ruta absoluta que evade el directorio de temas. | Lectura no autorizada de archivos del sistema o sondeo de existencia de ficheros arbitrarios en el host. |
| **SEC-09** | **Baja** | Agotamiento de memoria por lectura no acotada de temas y bombas de directorio | [CWE-400](https://cwe.mitre.org/data/definitions/400.html)<br>[CWE-776](https://cwe.mitre.org/data/definitions/776.html) | Colocar archivos `.theme` masivos (>100 MB) o symlinks a dispositivos especiales (`/dev/zero`, `/dev/urandom`) o miles de archivos falsos en `~/.rustext/themes/`. | Consumo excesivo de memoria RAM, congelamiento del bucle de eventos o cuelgue por OOM (Denegación de Servicio local). |
| **SEC-10** | **Baja** | Fuga acumulativa de memoria en la Paleta de Comandos por cadenas filtradas | [CWE-401](https://cwe.mitre.org/data/definitions/401.html) | Invocar `Box::leak` para convertir dinámicamente nombres de extensiones y temas en `&'static str` cada vez que se abre la Paleta de Comandos (`Ctrl+P`). | Crecimiento no acotado del heap en sesiones prolongadas con uso intensivo de la paleta. |

---

### 18.3 Mitigaciones y Directrices Defensivas de Implementación

Para cada uno de los vectores identificados, se han formalizado las siguientes directrices y mecanismos de mitigación en Rust puro:

#### 1. Confinamiento de Extensiones y Modelo *Workspace Trust* (SEC-01)
- **Principio de menor privilegio**: Desactivar el descubrimiento automático de extensiones en rutas relativas del proyecto (`./.rustext/extensions`) a menos que el usuario marque explícitamente el directorio de trabajo como "Espacio Confiable" (*Trusted Workspace*).
- Por defecto, las extensiones sólo se cargan desde el directorio global de usuario (`~/.rustext/extensions`), cuyos permisos de escritura pertenecen exclusivamente al propietario de la cuenta.

#### 2. Guardado Atómico Resistente a Fallos de Energía (*Atomic Save*) (SEC-02)
- En lugar de abrir el archivo original con `File::create(target)` (que invoca inmediatamente `O_TRUNC` reduciendo el archivo a 0 bytes), implementar el patrón atómico de reemplazo POSIX:
  1. Crear un archivo temporal en el mismo sistema de archivos: `path.with_extension("tmp.XXXXXX")`.
  2. Escribir el contenido completo del búfer en el archivo temporal y sincronizar metadatos con `sync_all()`.
  3. Ejecutar `std::fs::rename(temp_path, target_path)`.
- En sistemas POSIX y Windows NTFS, `rename` es una operación atómica a nivel de sistema de ficheros: el archivo original se mantiene 100% íntegro hasta que la nueva versión lo sustituye de un solo golpe.

#### 3. Comunicación por Tuberías Estándar (*Stdio Pipes*) en Portapapeles (SEC-03)
- Evitar pasar cadenas de texto arbitrarias como argumentos de línea de comandos en `Command::new("wl-copy").arg(...)`.
- En su lugar, canalizar el contenido a través de `stdin`:
  ```rust
  let mut child = Command::new("wl-copy")
      .stdin(Stdio::piped())
      .spawn()?;
  if let Some(mut stdin) = child.stdin.take() {
      stdin.write_all(text.as_bytes())?;
  }
  child.wait()?;
  ```
- Esto garantiza compatibilidad con búferes de cualquier tamaño (sin topar con el límite `ARG_MAX` del kernel) y neutraliza cualquier inyección de opciones tipo `--clear`.

#### 4. Detección de Ciclos y Control de Profundidad en Búsqueda Global (SEC-04)
- En [`GlobalSearchEngine`](file:///home/bolek/Proyectos/rustext/src/core/global_search.rs), emplear `fs::symlink_metadata()` en lugar de `fs::metadata()` para identificar enlaces simbólicos.
- No seguir enlaces simbólicos hacia directorios por defecto, o mantener un conjunto `HashSet<(u64, u64)>` con los identificadores `(dev, ino)` de Linux/Unix para detectar y cortar inmediatamente cualquier grafo cíclico.
- Establecer un límite máximo estricto de recursión (p. ej. profundidad máxima de 32 subcarpetas).

#### 5. Saneamiento de Rutas Canónicas en Sesiones (SEC-05)
- Al restaurar archivos de borrador desde `session.ini`, validar que la ruta canónica (`std::fs::canonicalize`) comience estrictamente con el prefijo del directorio de datos de la aplicación (`~/.rustext/drafts/`). Si la ruta intenta salir mediante secuencias `../`, descartar el archivo y emitir una advertencia defensiva.

#### 6. Gancho Global de Pánico (*Panic Hook*) para Restauración de Terminal (SEC-06)
- Configurar un manejador con `std::panic::set_hook()` al inicio de la aplicación en modo TUI:
  ```rust
  let default_hook = std::panic::take_hook();
  std::panic::set_hook(Box::new(move |info| {
      // Restaurar la consola inmediatamente antes de reportar el error
      let _ = std::io::stdout().write_all(b"\x1B[?1049l\x1B[?25h");
      default_hook(info);
  }));
  ```

#### 7. Creación Exclusiva de Búferes en Memoria Compartida Wayland (SEC-07)
- Uso de `memfd_create` o apertura con banderas atómicas `O_CREAT | O_EXCL` y permisos restringidos `0600` en `/dev/shm`, asegurando que ningún proceso no privilegiado pueda pre-crear enlaces simbólicos para secuestrar o interceptar búferes de renderizado.

#### 8. Saneamiento Estricto y Verificación de Contención de Temas (SEC-08)
- Validación defensiva de nombres de archivo mediante `is_safe_theme_name()`: rechazo inmediato de secuencias de travesía (`..`), separadores de directorio (`/`, `\`), caracteres nulos (`\0`) y nombres superiores a 64 bytes.
- Verificación canónica de contención (`starts_with(canonical_base)`): garantiza que ningún enlace simbólico (*symlink*) o resolución relativa dentro de `~/.rustext/themes/` pueda acceder a archivos fuera de los directorios autorizados.

#### 9. Límites de Tamaño y Cuotas de Escaneo para Temas (SEC-09)
- Límite máximo de tamaño de archivo (`MAX_THEME_FILE_SIZE = 512 KB`): verificación de metadatos previo a la lectura en memoria para rechazar ficheros sobredimensionados, tuberías o dispositivos virtuales (`/dev/zero`).
- Cuota de escaneo de directorios (`MAX_THEME_SCAN_LIMIT = 256`): limita la cantidad de entradas procesadas por directorio para prevenir congelamientos de la interfaz ante carpetas con decenas de miles de archivos.

#### 10. Erradicación de Fugas de Memoria en la Paleta de Comandos vía `Cow<'static, str>` (SEC-10)
- Reemplazo del uso de `Box::leak` para cadenas dinámicas por `std::borrow::Cow<'static, str>` en `CommandItem`.
- Los comandos nativos e integrados continúan utilizando punteros estáticos prestados `Cow::Borrowed(&'static str)` sin costo de asignación dinámica, mientras que los comandos generados por extensiones y temas externos se gestionan como `Cow::Owned(String)`, liberándose automáticamente de la memoria al cerrar el modal.

---

### 18.4 Comparativa Multidimensional de Postura de Seguridad frente a Editores de la Industria

| Dimensión de Seguridad | **Rustext** | **VS Code (Code-OSS)** | **Sublime Text 4** | **Vim 9.2** | **Helix 24.07** |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Seguridad de Memoria** | **Total (Safe Rust)** | Depende de V8 / Chromium (C++) | C++ nativo (riesgo latente de UAF/overflow) | C nativo (historial de vulnerabilidades CVE) | **Total (Safe Rust)** |
| **Riesgo en Cadena de Suministro** | **Nulo (0 dependencias)** | Crítico (>1,000 paquetes npm en build/runtime) | Mínimo (cerrado, código interno C++) | Bajo (libc del sistema, ncurses) | Moderado (~140 crates externos en build) |
| **Superficie de Ataque por Extensiones** | **Mínima (Scripts acotados / IPC)** | Muy Alta (Marketplace abierto, plugins con Node/Electron) | Moderada (APIs de Python embebido) | Moderada (scripts Vimscript/Lua sin sandbox) | Ninguna (no tiene sistema de plugins) |
| **Confinamiento de Espacio de Trabajo (*Workspace Trust*)** | **En desarrollo (Directiva SEC-01)** | Implementado nativamente | No implementado (ejecuta plugins de carpeta) | No implementado (`modeline` ha tenido múltiples CVEs) | N/A (sin plugins locales) |
| **Inmunidad a ReDoS (Expresiones Regulares)** | **Inmune (Matchers deterministas $\mathcal{O}(N)$)** | Vulnerable en regexes de extensiones JS | Inmune en sintaxis nativa (Oniguruma / C++) | Moderada (motor regex clásico con backtracking) | Inmune (Tree-sitter grammars) |
| **Guardado Atómico de Archivos** | **Implementado (SEC-02: atomic save vía tmp + rename)** | Implementado (atomic save vía tmp rename) | Implementado (atomic save vía tmp rename) | Implementado (`set backupcopy=...`) | Implementado (atomic write) |
| **Telemetría y Recolección de Datos** | **0.0% (Cero llamadas de red / Cero telemetría)** | Telemetría integrada por defecto (desactivable) | Comprobación de licencias y updates remotos | 0.0% | 0.0% |

---

## 19. Licencia y Términos de Distribución (GNU AGPLv3)

Rustext se distribuye como software libre de código abierto bajo los términos de la **GNU Affero General Public License Version 3 (GNU AGPLv3)**.

### Principios y Garantías:
- **Libertad y Transparencia**: Todo usuario, desarrollador u organización tiene el derecho inalienable de ejecutar, estudiar, auditar, modificar y redistribuir el código fuente para cualquier propósito.
- **Copyleft Fuerte en Red (Cláusula Affero)**: Si modifica Rustext o lo integra como parte de una solución o servicio accesible remotamente a través de una red o servidor, está legalmente obligado a poner a disposición de los usuarios de dicha red el código fuente completo correspondiente bajo esta misma licencia GNU AGPLv3.
- **Cero Dependencias y Pureza Legal**: Al no incorporar ningún crate externo ni librerías de terceros (100% Rust `std`), Rustext garantiza una procedencia limpia y auditada, libre de vulnerabilidades de licencias encubiertas, componentes propietarios, DRM o telemetría.
- **Términos Completos**: Para consultar el texto oficial completo e íntegro de la licencia, revise el archivo [`LICENSE`](LICENSE) ubicado en la raíz del repositorio o visite <https://www.gnu.org/licenses/agpl-3.0.html>.



