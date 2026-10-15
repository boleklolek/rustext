# 📖 Manual de Usuario y Referencia Técnica de Rustext

**Rustext** es un editor de texto y código fuente nativo de alto rendimiento, escrito **100% en Rust puro utilizando exclusivamente la biblioteca estándar (`std`)**, sin dependencias externas ni crates de terceros.

Este manual ofrece una guía exhaustiva de operación, arquitectura interna, atajos de teclado, configuración, herramientas de desarrollo y los resultados del **profiling de memoria y benchmarks de rendimiento**.

---

## 📑 Tabla de Contenidos

1. [Filosofía y Arquitectura](#1-filosofía-y-arquitectura)
2. [Instalación y Compilación](#2-instalación-y-compilación)
3. [Guía de la Interfaz Gráfica (Desktop GUI) y TUI](#3-guía-de-la-interfaz-gráfica-desktop-gui-y-tui)
4. [Árbol de Proyecto y Explorador Lateral](#4-árbol-de-proyecto-y-explorador-lateral)
5. [Motor de Edición y Multicursor](#5-motor-de-edición-y-multicursor)
6. [Sistema de Snippets e Indentación Inteligente](#6-sistema-de-snippets-e-indentación-inteligente)
7. [Diagnósticos en Vivo, Linter y Formato de Código](#7-diagnósticos-en-vivo-linter-y-formato-de-código)
8. [Paleta de Comandos y Atajos de Teclado](#8-paleta-de-comandos-y-atajos-de-teclado)
9. [Sistema de Configuración (`config.ini`)](#9-sistema-de-configuración-configini)
10. [Sistema de Plugins y Extensiones](#10-sistema-de-plugins-y-extensiones)
11. [Perfil de Memoria y Benchmarks de Rendimiento](#11-perfil-de-memoria-y-benchmarks-de-rendimiento)
12. [Preguntas Frecuentes y Solución de Problemas](#12-preguntas-frecuentes-y-solución-de-problemas)

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
   - **Linux**: Interacción directa con el servidor X11 a través de `libX11.so` mediante FFI dinámico estándar.
   - **Windows**: Creación de ventana Win32 nativa (`user32.dll`, `gdi32.dll`) y dibujo mediante `StretchDIBits`.
   - **macOS**: Creación de ventana Cocoa / AppKit a través del runtime estándar de Objective-C (`objc_msgSend`).
   - **Renderizador de Software**: Framebuffer ARGB de 32 bits con primitivas 2D aceleradas, esquinas redondeadas anti-aliased, sombras proyectadas suaves, píldoras informativas y fuente bitmap monoespaciada embebida (8×16 px).

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

# 4. Ejecutar el Profile de Memoria y Benchmarks:
cargo run --release --bin memory_profile
```

---

## 3. Guía de la Interfaz Gráfica (Desktop GUI) y TUI

La interfaz gráfica de Rustext está diseñada con una estética moderna, limpia y minimalista inspirada en Tokyo Night y Catppuccin:

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ Archivo   Editar   Selección   Ver   Preferencias   Extensiones   Herramientas   Ayuda │  ← Barra de Menús Nativa
├──────────────────┬─────────────────────────────────────────────────────────────────────┤
│ PROYECTO [+] ↺ × │  1 main.rs ×    2 config.ini ×   [+]                                │  ← Barra de Pestañas
├──────────────────┼─────────────────────────────────────────────────────────────────────┤
│ ▼ src            │  1 │ pub fn main() {                                             ▲  │
│   • lib.rs       │  2 │     let config = EditorConfig::load();                      │  │
│   • main.rs      │  3 │     println!("Rustext iniciado");                           █  │  ← Scrollbar Interactivo
│ ▶ editor         │  4 │ }                                                           │  │
│ ▶ gfx            │  ~ │                                                             ▼  │
│ • Cargo.toml     │  ~ │                                                                │
├──────────────────┴─────────────────────────────────────────────────────────────────────┤
│ [ EDIT ]  src/main.rs  |  Línea 3, Col 5  |  UTF-8  |  Rust  |  Cursores: 1            │  ← Barra de Estado
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### Componentes de la Interfaz:
- **Barra de Menús Superior**: Desplegables accesibles mediante clic o atajos `Alt+A` (Archivo), `Alt+E` (Editar), `Alt+V` (Ver), etc.
- **Barra de Pestañas (Tab Bar)**: Muestra los búferes abiertos, botón de cerrar `×`, botón de nueva pestaña `[+]` y advertencia de cambios sin guardar `[+]`.
- **Barra Lateral (File Explorer)**: Árbol interactivo de archivos y carpetas del proyecto a la izquierda.
- **Gutter y Líneas de Código**: Números de línea alineados, indicador de advertencia/error de diagnóstico y lienzo de código.
- **Scrollbar Vertical Interactivo**: Pista de desplazamiento con pulgar redondeado arrastrable y soporte para clics de página arriba/abajo.
- **Barra de Estado Inferior**: Píldoras con modo de edición, nombre de archivo, posición del cursor, codificación UTF-8, lenguaje y contador de cursores.

---

## 4. Árbol de Proyecto y Explorador Lateral

El **Explorador Lateral (File Explorer Sidebar)** permite gestionar proyectos completos sin salir del editor.

### Características:
1. **Escaneo Perezoso (Lazy Scanning)**: Solo lee del disco las carpetas que el usuario decide expandir, evitando consumo de memoria en proyectos con miles de ficheros.
2. **Filtro Automático**: Oculta automáticamente directorios ruidosos o de compilación (`.git`, `target`, `node_modules`, `.DS_Store`, `.idea`, `.vscode`).
3. **Pistas de Color por Extensión**:
   - Naranja: Archivos `.rs`.
   - Cian: Archivos `.toml`, `.ini`, `.json`, `.yaml`.
   - Verde: Archivos `.md`, `.txt`, `.py`.
   - Púrpura: Archivos `.c`, `.cpp`, `.h`.
   - Acento: Archivos web `.html`, `.css`, `.js`, `.ts`.
4. **Flechas Jerárquicas**: Glifos bitmap embebidos `▼` para directorios abiertos y `▶` para carpetas cerradas.

### Controles del Explorador:
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

---

## 5. Motor de Edición y Multicursor

Rustext incluye un motor multicursor vectorial de primera clase:

### Operaciones Multicursor:
- `Ctrl + D`: **Selección Inteligente / Siguiente Ocurrencia**:
  - Si no hay selección, selecciona la palabra bajo el cursor.
  - Si se presiona nuevamente, busca la siguiente ocurrencia en el archivo y añade un cursor adicional en ella.
- `Ctrl + K`: Añadir un cursor en la misma columna de la línea superior.
- `Ctrl + J`: Añadir un cursor en la misma columna de la línea inferior.
- `Esc`: Colapsa todos los cursores al cursor principal.

### Sincronización y Desplazamiento Vectorial:
Cuando múltiples cursores escriben simultáneamente:
- Todas las inserciones se calculan en paralelo.
- El motor calcula el desplazamiento acumulativo (`offset shift`) para que los cursores posteriores mantengan su alineación exacta.
- Los cursores que colisionan o se solapan se fusionan automáticamente sin duplicaciones.

### Transacciones Atómicas de Deshacer / Rehacer (`Ctrl+Z` / `Ctrl+Y`):
- Todas las operaciones realizadas por 10 o 50 cursores en una pulsación se empaquetan en una única transacción atómica.
- Al pulsar `Ctrl+Z`, se revierte el texto y se restauran las posiciones exactas de todos los cursores previos.

### Auto-Cierre y Envoltorio de Pares (Auto-Surround):
- Al escribir `(`, `[`, `{`, `"`, `'`, el editor inserta automáticamente el carácter de cierre.
- Al presionar `Backspace` dentro de un par vacío (`|`), ambos caracteres se eliminan.
- **Envoltorio de Selección**: Si selecciona texto y presiona `(` o `"`, el texto seleccionado se envuelve automáticamente entre los delimitadores.

---

## 6. Sistema de Snippets e Indentación Inteligente

Rustext incluye un motor de plantillas y snippets sin dependencias externas:

### Snippets Integrados:
Escriba el prefijo y presione `Tab`:
- `fn` $\rightarrow$ `fn nombre(args) -> Retorno {\n    \n}`
- `struct` $\rightarrow$ `pub struct Nombre {\n    campo: Tipo,\n}`
- `for` $\rightarrow$ `for item in iter {\n    \n}`
- `match` $\rightarrow$ `match valor {\n    Patron => {},\n    _ => {}\n}`
- `test` $\rightarrow$ `#[test]\nfn test_nombre() {\n    assert!(true);\n}`

### Navegación de Placeholders:
- Presione `Tab` para avanzar al siguiente campo del snippet.
- Presione `Shift + Tab` para regresar al campo anterior.

---

## 7. Diagnósticos en Vivo, Linter y Formato de Código

Rustext incluye herramientas de verificación y corrección de código integradas:

### 1. Comprobación de Errores y Linter (`F7`):
- **Rust**: Ejecuta `cargo check --message-format=json` de forma transparente y mapea los errores y advertencias línea por línea en el buffer.
- **Python**: Valida sintaxis con `python -m py_compile` o `pyflakes`.
- **C/C++**: Ejecuta `clang -fsyntax-only`.
- **Visualización**: Las líneas con error se señalan en el gutter con insignias de color:
  - `[E]` Rojo: Error de compilación.
  - `[W]` Amarillo: Advertencia (Warning).
  - `[I]` Azul: Información.

### 2. Navegación entre Errores:
- `F8`: Salta al **siguiente error o advertencia** en el archivo.
- `Shift + F8`: Salta al **error anterior**.

### 3. Formateo Automático de Código (`Ctrl + Shift + I`):
Alinea el archivo utilizando el formateador del lenguaje disponible en el sistema operativo:
- Rust: `rustfmt`
- Python: `black` o `autopep8`
- C/C++: `clang-format`
- Web / JS / TS / JSON / Markdown: `prettier`
- Go: `gofmt`

### 4. Automatización al Guardar:
Configure en `config.ini` `format_on_save = true` o `check_on_save = true` para formatear y verificar el código automáticamente al presionar `Ctrl + S`.

---

## 8. Paleta de Comandos y Atajos de Teclado

Presione `Ctrl + P` para abrir la **Paleta de Comandos Flotante**. Permite buscar y ejecutar cualquier acción del editor mediante filtrado difuso.

### Tabla Completa de Atajos de Teclado:

| Atajo de Teclado | Acción | Menú Correspondiente |
| :--- | :--- | :--- |
| `Ctrl + S` | Guardar archivo actual | Archivo > Guardar |
| `Ctrl + O` | Abrir diálogo de archivo | Archivo > Abrir Archivo... |
| `Ctrl + T` | Abrir nueva pestaña | Archivo > Nueva Pestaña |
| `Ctrl + W` | Cerrar pestaña actual | Archivo > Cerrar Pestaña |
| `Ctrl + Q` | Salir de Rustext | Archivo > Salir |
| `Ctrl + Z` | Deshacer (Undo) | Editar > Deshacer |
| `Ctrl + Y` | Rehacer (Redo) | Editar > Rehacer |
| `Ctrl + X` | Cortar selección o línea | Editar > Cortar |
| `Ctrl + C` | Copiar selección o línea | Editar > Copiar |
| `Ctrl + V` | Pegar desde el portapapeles | Editar > Pegar |
| `Ctrl + F` | Buscar en el documento | Editar > Buscar... |
| `Ctrl + H` | Buscar y Reemplazar | Editar > Reemplazar... |
| `Ctrl + D` | Seleccionar siguiente ocurrencia | Selección > Siguiente Ocurrencia |
| `Ctrl + K` | Añadir cursor arriba | Selección > Añadir Cursor Arriba |
| `Ctrl + J` | Añadir cursor abajo | Selección > Añadir Cursor Abajo |
| `Esc` | Cursor único / Cancelar modal | Selección > Cursor Único |
| `Ctrl + B` | Alternar Explorador Lateral | Ver > Explorador Lateral |
| `PageUp` | Cambiar a pestaña anterior | Ver > Pestaña Anterior |
| `PageDown` | Cambiar a pestaña siguiente | Ver > Pestaña Siguiente |
| `Ctrl + P` | Abrir Paleta de Comandos | Ver > Paleta de Comandos... |
| `Ctrl + Shift + X`| Administrador de Extensiones | Extensiones > Administrador... |
| `Ctrl + Shift + I`| Formatear Documento | Herramientas > Formatear Documento |
| `F7` | Comprobar Errores (Check / Lint)| Herramientas > Comprobar Errores |
| `F8` | Saltar al Siguiente Error | Herramientas > Siguiente Error |
| `Shift + F8` | Saltar al Error Anterior | Herramientas > Error Anterior |
| `F1` | Ventana Acerca de Rustext | Ayuda > Acerca de Rustext |

---

## 9. Sistema de Configuración (`config.ini`)

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
format_on_save = false
check_on_save = false

[theme]
# Opciones: tokyo-night, catppuccin-mocha, one-dark, modern-dark, monokai, solarized-dark, light, high-contrast
name = tokyo-night

[gui]
window_width = 1100
window_height = 720
font_scale = 1
line_spacing = 2
cursor_style = bar       # Opciones: bar, block, underline
cursor_blink = true
rounded_corners = true
shadows = true
scrollbar = true
menu_bar = true
sidebar = true
sidebar_width = 220
```

---

## 10. Sistema de Plugins y Extensiones

Rustext proporciona una arquitectura de extensiones extensible sin sacrificar la filosofía de cero dependencias externas.

### 1. Extensiones en el Proceso (`Plugin` Trait):
Permite interceptar eventos del editor:
```rust
pub trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn on_init(&mut self, ctx: &mut PluginContext) {}
    fn on_before_save(&mut self, ctx: &mut PluginContext) {}
    fn on_after_save(&mut self, ctx: &mut PluginContext) {}
    fn on_command(&mut self, command: &str, ctx: &mut PluginContext) -> bool { false }
}
```

### 2. Extensiones por Archivo de Manifiesto (`extension.conf`):
En el directorio `~/.config/rustext/extensions/` cree una subcarpeta con un archivo `extension.conf`:
```ini
[extension]
id = mi_herramienta
name = Mi Herramienta Personalizada
version = 1.0.0
author = Bolek

[command.contar]
title = Contar Palabras del Documento
executable = wc
args = -w
```
Al presionar `Ctrl + P`, el comando estará disponible instantáneamente en la Paleta de Comandos.

---

## 11. Perfil de Memoria y Benchmarks de Rendimiento

Para validar el rendimiento y la estabilidad de Rustext bajo cargas extremas, se diseñó un binario dedicado de profiling y benchmarking: [`src/bin/memory_profile.rs`](file:///home/bolek/Proyectos/rustext/src/bin/memory_profile.rs).

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

## 12. Preguntas Frecuentes y Solución de Problemas

### ¿Por qué Rustext no utiliza bibliotecas de ventana como `winit` o `SDL`?
Para eliminar cientos de dependencias de compilación y asegurar que el binario compile en cualquier máquina en 2 segundos sin requerir gestores de paquetes externos.

### ¿Cómo fuerzo el modo terminal si estoy en un entorno con servidor gráfico?
Ejecute simplemente `./target/release/rustext` sin banderas, o `./target/release/rustext --tui`. El modo gráfico se activa con `-g` o `--gui`.

### ¿Cómo cambio el tema visual?
Abra el menú superior **Preferencias > Tema** o agregue `name = catppuccin-mocha` en su archivo `config.ini`.
