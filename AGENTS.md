# AGENTS.md — Guía para agentes de IA y colaboradores automatizados

Instrucciones para trabajar en **Rustext** (editor nativo en Rust puro, `std` únicamente). Complementa a [`CONTRIBUTING.md`](CONTRIBUTING.md) y [`SECURITY.md`](SECURITY.md). Comunicación y documentación del proyecto: **español**.

## Reglas no negociables

1. **Cero dependencias**: `[dependencies]` en `Cargo.toml` debe seguir vacío (tampoco `dev-` ni `build-dependencies`). Nada de crates de terceros.
2. **Safe Rust en el núcleo**: `unsafe` solo en la frontera con el sistema (`src/window/`, FFI/libc necesario), acotado y justificado.
3. **Seguridad**: valida límites de tamaño y confina rutas canónicas al leer temas, borradores o configuración. Ejecuta subprocesos con `Command::new()` y argumentos vectoriales, nunca con `sh -c`. No uses `Box::leak` en ciclos de interfaz.
4. **Rendimiento**: el renderizado del viewport y el manejo de teclas deben ser $O(1)$ respecto al tamaño del archivo.
5. **Rust 2024 Edition**: las llamadas a funciones `unsafe` dentro de `unsafe fn` necesitan un bloque `unsafe { }` explícito.
6. **No modifiques archivos no relacionados** ni borres comentarios o docstrings ajenos a tu cambio.

## Estructura

| Ruta | Contenido |
| :--- | :--- |
| `src/core/` | Piece Table, búfer, cursores, historial, búsqueda, portapapeles |
| `src/editor/` | `Editor` (`app.rs` es muy grande), LSP, Git, sesión, menús, snippets |
| `src/gfx/` | Framebuffer, renderizador por software, fuentes TTF, temas (`theme_loader.rs`) |
| `src/window/` | Backends nativos: `wayland.rs`, `x11.rs`, `linux.rs`, `win32.rs`, `macos.rs` |
| `src/ui/` | TUI, Flexbox, primitivas reactivas, widgets retenidos |
| `src/plugin/` | Extensiones externas por IPC |
| `assets/themes/` | Temas oficiales `.theme` (no hace falta tocar `renderer.rs` para añadir temas) |
| `installer/` | Empaquetado: Windows (`.ps1`, WiX `.wxs`) y Linux (`build_deb.sh`) |
| `tests/` | `gui_tests.rs` (la mayoría), `buffer_tests.rs`, `layout_tests.rs`, `multicursor_tests.rs` |

## Comandos

```bash
cargo check                      # compilación sin errores ni warnings nuevos
cargo test --tests               # suite completa (más de 250 tests)
cargo test --test gui_tests -- <nombre>   # un test concreto
cargo run --release -- --gui     # GUI nativa;  sin --gui = modo TUI
```

## Trampas conocidas

- **`test_wayland_socket_discovery_and_availability` abre una ventana real** en el compositor de la sesión. Si falla, depende del entorno; mira el mensaje de error (ahora incluye el `wl_display.error` real).
- **Wayland: los IDs de objeto del cliente deben ser secuenciales sin huecos**, en orden de creación. Los globales opcionales (`wl_seat`, `zxdg_decoration_manager_v1`; este último no existe en GNOME/Mutter) solo consumen ID si están anunciados. Un hueco hace que el compositor cierre la conexión.
- **`win32.rs` y `macos.rs` no compilan en Linux** por su `#![cfg(target_os = ...)]`. Para comprobar tipos desde Linux, desactiva temporalmente ese `cfg` (y el de `pub mod win32;` en `src/window/mod.rs`), ejecuta `cargo check` y **restaura ambos archivos**. No dejes el `cfg` desactivado en un commit.
- **El backend gráfico de macOS es un esqueleto** (`--gui` no abre ventana); el TUI y el portapapeles (`pbcopy`/`pbpaste`) sí funcionan.
- **La versión se lee de `Cargo.toml`** vía `env!("CARGO_PKG_VERSION")`. Al subirla, actualiza también `installer/windows/rustext.wxs`, los badges de `README.md`, `README_EN.md`, `MANUAL.md`, `MANUAL_TEMAS_Y_EXTENSIONES.md` y `CHANGELOG.md`.
- **No imprimas `git remote -v` ni la URL del remoto**: puede contener un token de acceso. No lo copies a archivos, logs ni mensajes.

## Antes de dar un cambio por terminado

1. `cargo test --tests` pasa sin fallos; si añades comportamiento, añade tests en `tests/`.
2. Si cambias algo visible (atajo, opción de `config.ini`, función), actualiza `README.md`, `README_EN.md` y `MANUAL.md`, y añade una entrada en `CHANGELOG.md` (sección **Sin publicar**).
3. Un cambio de interfaz que no hayas visto en pantalla debe describirse como **no probado visualmente**.

## Commits y releases

- Mensajes en español con prefijo convencional: `feat(...)`, `fix(...)`, `docs(...)`, `ci(...)`, `chore(...)`.
- Un push a `main` ejecuta el CI (Linux, Windows, macOS ARM e Intel). Una etiqueta `v*` ejecuta el release y publica `.tar.gz`, `.zip`, `.msi`, `.deb` y `SHA256SUMS.txt`.
- No muevas ni fuerces etiquetas ya publicadas sin que el usuario lo pida.
