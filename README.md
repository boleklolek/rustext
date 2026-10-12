# 🦀 Rustext

**Rustext** es un editor de código nativo, ultraligero y de latencia ultra baja, escrito **100% en Rust puro utilizando exclusivamente la biblioteca estándar (`std`)**, sin dependencias externas ni crates de terceros.

Inspirado en los puntos fuertes de **Sublime Text** (rendimiento nativo, inicio instantáneo, memoria mínima) y **Atom / VS Code** (maleabilidad, multicursores, sistema de plugins y extensibilidad).

---

## ⚡ Características Principales

1. **Cero Dependencias Externas (Pure `std` Rust)**:
   - Binario compilado final de apenas **~650 KB**.
   - Tiempo de compilación inferior a **2 segundos**.
   - Sin riesgo de cadena de suministro ni dependencias desactualizadas.

2. **Arquitectura de Búfer Piece Table**:
   - Estructura `PieceTable` con búferes inmutables `Original` y append-only `Add`.
   - Indexación continua de líneas y conversión rápida de coordenadas `(line, col) <-> char_offset`.
   - Soporte total para caracteres y símbolos multibyte UTF-8.

3. **Motor Multicursor Vectorial Paralelo**:
   - Edición simultánea con múltiples cursores en paralelo.
   - Ajuste automático de desplazamientos (`offset shift`) para inserciones, saltos de línea, tabulaciones y borrado (`Backspace`/`Delete`).
   - Selección de la siguiente ocurrencia de la palabra actual con `Ctrl+D` (estilo Sublime Text / VS Code).
   - Añadir cursores en líneas adyacentes (`Ctrl+K` arriba, `Ctrl+J` abajo).
   - Fusión y deduplicación automática de cursores solapados.

4. **Transacciones Atómicas de Deshacer / Rehacer (Undo / Redo)**:
   - Todas las mutaciones paralelas multicursor se agrupan en una única transacción reversible.
   - Restauración exacta de todos los cursores en cada paso de deshacer o rehacer.

5. **Resaltado de Sintaxis sin Crates**:
   - Tokenizador léxico por streaming de alto rendimiento.
   - Soporte integrado para: **Rust**, **Python**, **C/C++**, **JavaScript/TypeScript**, **JSON**, **Markdown** y **TOML**.

6. **Sistema de Extensiones y Plugins**:
   - **Plugins en el proceso**: Trait `Plugin` con ganchos de ciclo de vida (`on_init`, `on_open`, `on_before_save`, `on_after_save`, `on_command`).
   - **Plugins por procesos externos (IPC)**: Ejecución segura de formateadores (como `rustfmt`, `black`, `prettier`) vía tuberías estándar `stdin`/`stdout`.

7. **Interfaz de Terminal (TUI) de Doble Búfer**:
   - Renderizado atómico en una única llamada al sistema para eliminar por completo el parpadeo (*flicker-free*).
   - Control seguro del modo *raw* de la terminal con guardia RAII (`TerminalGuard`) que restaura la terminal incluso en caso de error.
   - Barra de estado informativa (Modo, Archivo, Modificado `[+]`, Línea/Columna, Total líneas/caracteres, Número de cursores, Lenguaje).
   - Modales interactivos para búsqueda (`Ctrl+F`), guardar como y paleta de comandos (`Ctrl+P`).

---

## 📂 Estructura del Código

```text
rustext/
├── Cargo.toml               # Sin dependencias en [dependencies]
├── README.md
├── src/
│   ├── lib.rs               # Biblioteca con re-exportación de componentes
│   ├── main.rs              # Punto de entrada CLI del editor
│   ├── core/                # Estructuras de datos fundamentales
│   │   ├── mod.rs
│   │   ├── piece_table.rs   # Estructura Piece Table
│   │   ├── buffer.rs        # TextBuffer con indexación de líneas y archivo
│   │   ├── cursor.rs        # Cursor y CursorSet (multicursores vectoriales)
│   │   ├── history.rs       # Transacciones y pilas de Undo/Redo
│   │   ├── search.rs        # Búsqueda y expansión de ocurrencias (Ctrl+D)
│   │   └── syntax.rs        # Tokenizador y resaltado sintáctico
│   ├── plugin/              # Arquitectura de plugins
│   │   ├── mod.rs
│   │   ├── traits.rs        # Trait Plugin y PluginContext
│   │   ├── registry.rs      # PluginRegistry y plugins integrados
│   │   └── process.rs       # Ejecutor de procesos externos vía pipes IPC
│   ├── ui/                  # Interfaz de usuario en terminal
│   │   ├── mod.rs
│   │   ├── terminal.rs      # Modo raw, detección de tamaño y lectura de teclas
│   │   ├── theme.rs         # Paleta de colores ANSI
│   │   ├── screen.rs        # Dibujado de pantalla con doble búfer
│   │   └── prompt.rs        # Modal interactivo para búsqueda y comandos
│   └── editor/              # Estado global y orquestación
│       ├── mod.rs
│       ├── keymap.rs        # Mapeo de teclas a acciones
│       └── app.rs           # Bucle principal de eventos y edición
└── tests/                   # Pruebas de integración
    ├── buffer_tests.rs
    └── multicursor_tests.rs
```

---

## ⌨️ Atajos de Teclado

| Atajo | Acción |
| :--- | :--- |
| `Ctrl + S` | Guardar el archivo actual (o solicitar nombre si no tiene) |
| `Ctrl + Q` | Salir del editor |
| `Ctrl + F` | Abrir buscador interactivo |
| `Ctrl + D` | Seleccionar palabra / añadir siguiente ocurrencia como cursor |
| `Ctrl + K` | Añadir cursor en la línea superior |
| `Ctrl + J` | Añadir cursor en la línea inferior |
| `Ctrl + Z` | Deshacer (Undo) |
| `Ctrl + Y` | Rehacer (Redo) |
| `Ctrl + P` | Abrir Paleta de Comandos (`stats`, plugins, etc.) |
| `Flechas` | Navegación de cursores |
| `Home` / `End` | Inicio / fin de la línea actual |
| `Tab` | Indentación (4 espacios por defecto) |
| `Enter` | Salto de línea con auto-indentación según la línea previa |
| `Esc` | Colapsar multicursores a cursor único / Cancelar modal |

---

## 🚀 Compilación y Ejecución

### Compilar y ejecutar pruebas:
```bash
cargo test
```

### Compilar binario optimizado:
```bash
cargo build --release
```

### Iniciar el editor:
```bash
# Abrir editor con buffer de bienvenida
./target/release/rustext

# O abrir un archivo específico
./target/release/rustext src/main.rs
```
