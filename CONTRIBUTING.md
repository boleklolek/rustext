# 🤝 Guía de Contribución a Rustext (`CONTRIBUTING.md`)

¡Gracias por tu interés en contribuir a **Rustext**! Este documento describe las reglas arquitectónicas fundamentales, el flujo de trabajo de desarrollo y los estándares de calidad que mantienen a Rustext ultrarrápido, seguro y libre de dependencias.

---

## 🏛️ 1. Regla de Oro: Cero Dependencias Externas (`std::*` Only)

El principio fundacional e innegociable de **Rustext** es que la sección `[dependencies]` de [`Cargo.toml`](Cargo.toml) **debe permanecer completamente vacía**.

- **No se aceptan crates de terceros** (ni en `[dependencies]`, ni en `[dev-dependencies]`, ni en `[build-dependencies]`).
- Todo algoritmo, protocolo (LSP, JSON, INI, Wayland wire protocol, SFNT TrueType, Flexbox) o estructura de datos debe implementarse en **Rust puro utilizando exclusivamente la biblioteca estándar (`std`)**.
- Las interacciones con el sistema operativo (Wayland, X11, Win32, Cocoa) se realizan directamente mediante sockets IPC del sistema o enlaces FFI mínimos a las bibliotecas nativas preinstaladas del sistema operativo.

---

## 🛡️ 2. Estándares de Código y Seguridad

1. **Edición de Rust**: El proyecto utiliza **Rust 2024 Edition**.
2. **Seguridad de Memoria (`Safe Rust`)**:
   - Todo el núcleo del editor (`src/core/`, `src/editor/`, `src/gfx/`, `src/syntax/`, `src/lsp/`, `src/ui/`) debe escribirse en **100% Safe Rust**.
   - Los bloques `unsafe` están estrictamente restringidos a las fronteras FFI del sistema de ventanas (`src/window/`) y llamadas `libc`/POSIX indispensables, y deben estar debidamente justificados y acotados.
3. **Resistencia a Denegación de Servicio (DoS) y Path Traversal**:
   - Cualquier lectura de archivos de configuración, temas o borradores debe validar límites de tamaño y confinar rutas canónicas (ver [`SECURITY.md`](SECURITY.md)).
   - Nunca utilices expresiones regulares con retroceso exponencial ni invoques shells intermedios (`sh -c`) al ejecutar subprocesos; utiliza siempre argumentos vectoriales directos en `std::process::Command::new()`.
4. **Cero Asignaciones Innecesarias en el Camino Crítico**:
   - El renderizado del viewport y el manejo de teclas deben mantener complejidad $\mathcal{O}(1)$ respecto al tamaño total del archivo y evitar fugas de memoria (nunca usar `Box::leak` en ciclos de interfaz).

---

## 🛠️ 3. Entorno de Desarrollo y Pruebas

### Requisitos
- Compilador oficial **Rust estable** (`rustc >= 1.85` con soporte para `edition = "2024"`).
- En Linux: encabezados de desarrollo X11 (`libx11-dev` en Debian/Ubuntu) para el enlace dinámico de respaldo X11.

### Comandos Principales

```bash
# 1. Verificar compilación sin errores ni warnings:
cargo check

# 2. Ejecutar la suite completa de pruebas (166+ tests unitarios y de integración):
cargo test --tests

# 3. Ejecutar benchmarks comparativos y perfilado de memoria:
cargo run --release --bin benchmark_comparison
cargo run --release --bin memory_profile
```

Toda contribución de código debe pasar el 100% de los tests existentes en Linux, Windows y macOS, e incluir nuevas pruebas en [`tests/`](tests/) que cubran la funcionalidad añadida o el bug corregido.

---

## 🎨 4. Contribuir con Temas (`.theme`) y Extensiones

No es necesario modificar el motor gráfico para aportar nuevos temas visuales o extensiones:
- **Temas Visuales**: Crea un archivo `<nombre>.theme` siguiendo la especificación de [`assets/themes/`](assets/themes/) y las instrucciones detalladas en [`MANUAL_TEMAS_Y_EXTENSIONES.md`](MANUAL_TEMAS_Y_EXTENSIONES.md).
- **Snippets y Lenguajes**: Puedes ampliar las definiciones léxicas en `src/syntax/` o el registro de snippets en `src/editor/snippet.rs` acompañados de sus respectivos tests en `tests/multicursor_tests.rs`.

---

## 🔄 5. Flujo de Pull Requests (PR)

1. Haz un **Fork** del repositorio y crea una rama descriptiva desde `main`:
   - `feat/nombre-funcionalidad` para nuevas características.
   - `fix/descripcion-correccion` para corrección de errores.
   - `docs/mejora-documentacion` para manuales y guías.
   - `sec/mitigacion-seguridad` para mejoras defensivas.
2. Asegúrate de que `cargo test --tests` pase sin fallos ni advertencias del compilador.
3. Si tu cambio añade un atajo de teclado, opción en `config.ini` o característica visible para el usuario, actualiza [`README.md`](README.md), [`README_EN.md`](README_EN.md) y [`MANUAL.md`](MANUAL.md).
4. Abre el Pull Request describiendo claramente el problema resuelto, el impacto en memoria/rendimiento y cómo fue verificado.

---

## 📄 Licencia de las Contribuciones

Al enviar una contribución a este repositorio, aceptas que tu código sea distribuido bajo la misma licencia del proyecto: **GNU Affero General Public License v3.0 or later (`AGPL-3.0-or-later`)** (ver [`LICENSE`](LICENSE)).
