# 📦 Changelog — Rustext Release History

All notable changes to **Rustext** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### ✨ Added
- **Modular Keymap System (`.keymap`) and Vim / Emacs Profiles**:
  - Declarative template-based keymap architecture modeled after the theme system: modular template files in `assets/keymaps/` (`default.keymap`, `vim.keymap`, `emacs.keymap`) with support for custom user keymaps in `~/.rustext/keymaps/` and `./.rustext/keymaps/`.
  - Full support for multi-step prefix chord sequences (`KeyChord::Sequence`) such as `Ctrl+X Ctrl+S` in Emacs and `g g` in Vim.
  - Native modal editing states (Normal, Insert, Visual) for Vim mode with seamless transitions (`i`, `a`, `v`, `Esc`) and reactive status bar indicators (`[ NORMAL ]`, `[ INSERT ]`, `[ VISUAL ]`).
  - Persistent `keymap = default` configuration in `config.ini`, dynamic switcher under *Preferences > Keyboard Keymap*, and Command Palette (`Ctrl+Shift+P`) actions.
- **Integrated client-side window controls (CSD) for Wayland**: Native Minimize (`—`), Maximize/Restore (`□`), and Close (`✕`) buttons in the top menu bar with interactive hover styling, dispatching `xdg_toplevel.set_minimized`, `set_maximized`, and `unset_maximized`. Provides complete window control on Wayland compositors that omit server-side decorations (such as GNOME/Mutter).
- **Draggable tab reordering with mouse** in the GUI tab bar: `Editor::move_tab`, `GfxRenderer::tab_rects`, and `GfxRenderer::tab_drop_index`. Active tab and split pane document indexes follow smoothly.
- **Native OS clipboard integration** on Windows (`clip` / `Get-Clipboard`) and macOS (`pbcopy` / `pbpaste`), with `xsel` as a Linux fallback.
- **Win32 Backend**: Keyboard shortcuts `Alt+Letter/Digit/[/\/Arrows`, real system DPI detection via `GetDpiForSystem`, and correct client coordinate mapping for mouse wheel events.

### 🐛 Fixed
- **Wayland on GNOME/Mutter**: Client object IDs are now allocated sequentially and only consumed for globals that are actually advertised (`wl_seat`, `zxdg_decoration_manager_v1`). Previously, an ID gap caused the compositor to terminate the connection with `invalid arguments for wl_compositor#N.create_surface`, preventing the GUI from opening. Socket write failures now report the actual wire protocol error (`wl_display.error`) rather than generic `Broken pipe`.

### 🔧 Changed
- **macOS**: Releases now compile a true universal binary (`aarch64` + `x86_64` using `lipo`, targeting `MACOSX_DEPLOYMENT_TARGET=11.0`) with CI validating both Apple Silicon and Intel runners.
- **Documentation & Badges**: Updated test suite badges and documentation in `README.md` and `README_EN.md` to reflect 248 tests passing. Complete professional English translation of all project technical documentation: `SECURITY_EN.md`, `CONTRIBUTING_EN.md`, `CHANGELOG_EN.md`, `MANUAL_EN.md`, and `MANUAL_THEMES_AND_EXTENSIONS_EN.md`.

---

## [v0.2.0] - 2026-10-21

### ✨ Added
- **Automated Native Cross-Platform Packaging in CI/CD**:
  - **Windows (`.msi` and portable `.zip`)**: WiX Toolset integration (`installer/windows/rustext.wxs`) and PowerShell packaging script (`installer/windows/build.ps1`) with Start Menu shortcuts.
  - **Linux Debian/Ubuntu (`.deb`)**: Package builder script [`installer/linux/build_deb.sh`](installer/linux/build_deb.sh) installing `/usr/bin/rustext`, desktop entry, SVG vector logo, system themes in `/usr/share/rustext/themes/`, and documentation in `/usr/share/doc/rustext/`.
  - Automatic release publishing with cryptographic checksums (`SHA256SUMS.txt`) for Linux (`x86_64`), Windows (`x86_64 MSVC`), and macOS (`apple-darwin`).
- **Dynamic Declarative Theme Engine (`.theme`) Without Recompilation**:
  - Auto-discovery and live hot-reloading of themes from `~/.rustext/themes/` and `./.rustext/themes/`.
  - 8 modular official themes in [`assets/themes/`](assets/themes/): `tokyo-night.theme`, `catppuccin-mocha.theme`, `one-dark.theme`, `modern-dark.theme`, `monokai.theme`, `solarized-dark.theme`, `light.theme`, and `dracula.theme`.
  - Automatic extraction of official templates to `~/.rustext/themes/` on initial launch.
- **Bilingual Documentation & Technical Manuals**:
  - Complete English [`README_EN.md`](README_EN.md) with bilingual badge switcher (`🇪🇸 Español` / `🇬🇧 English`).
  - New [`MANUAL_TEMAS_Y_EXTENSIONES.md`](MANUAL_TEMAS_Y_EXTENSIONES.md) detailing 55+ ARGB theme slots and extension IPC architecture.
  - Standard open-source community guidelines: [`CHANGELOG.md`](CHANGELOG.md), [`CONTRIBUTING.md`](CONTRIBUTING.md), and [`SECURITY.md`](SECURITY.md).

### 🛡️ Security
- **SEC-07 (CWE-377 / CWE-59)**: Exclusive `O_CREAT | O_EXCL` creation with UNIX `0600` permissions for shared memory buffers in `/dev/shm` when `memfd_create` is unavailable.
- **SEC-08 (CWE-22)**: Strict whitelist validation `is_safe_theme_name()` (max 64 bytes) and canonical path confinement `starts_with(canonical_base)` against path traversal attacks.
- **SEC-09 (CWE-400 / CWE-776)**: Hard limit `MAX_THEME_FILE_SIZE = 512 KB` and quota limit of 256 files per directory to guard against DoS via memory exhaustion.
- **SEC-10 (CWE-401)**: Replaced `Box::leak` with `Cow<'static, str>` in `CommandItem`, eliminating memory leaks across Command Palette modal invocations.

### ⚡ Performance & Fixes
- **Theme Parser Optimizations**: Constant-time $\mathcal{O}(1)$ hex parsing using bitwise arithmetic (`hex_nibble`) and removal of intermediate string allocations in `parse_theme_ini`.
- **Win32 Backend (`src/window/win32.rs`)**: Fixed `WM_CHAR` decoding (`raw_byte` and UTF-32 support), added `WM_RBUTTONUP` / `WM_MBUTTONUP` handlers, and enforced explicit `unsafe` blocks for Rust 2024 Edition.
- **GitHub Actions**: Workflows updated to native Node.js 24 actions (`actions/checkout@v7`, `actions/upload-artifact@v7`, `actions/download-artifact@v8`, `softprops/action-gh-release@v3`).

---

## [v0.1.0] - 2026-10-18

### ✨ Initial Release
- **100% Pure Standard Rust (`std::*`) with Zero External Dependencies**:
  - High-performance **Piece Table** data structure with zero-copy chunk iteration and full multibyte UTF-8 support.
  - **Parallel Vectorial Multi-cursor Engine** with automatic offset synchronization and atomic Undo/Redo transactions.
- **Native Desktop GUI & Terminal (TUI) Windowing**:
  - Dependency-free backends for **Wayland** (`wl_shm` + `xdg_shell` + `memfd_create`), **X11** (`libX11.so`), **Windows** (`user32.dll` / `gdi32.dll`), and **macOS** (`AppKit`).
  - 32-bit ARGB software `Framebuffer` with HiDPI support, analytical SDF rounded corners, $\mathcal{O}(1)$ Gaussian blur shadows, and frosted glass effects.
  - Double-buffered flicker-free TUI mode with safe terminal restoration (`TerminalGuard` and `panic::set_hook`).
- **Advanced Typography & Vector TrueType Engine (`.ttf`)**:
  - Pure Rust SFNT binary decoder, analytical trapezoidal scanline rasterizer, subpixel ClearType LCD anti-aliasing, two-level glyph cache (`TtfGlyphCache`), linear sRGB gamma correction (IEC 61966-2-1), programming ligatures, and interactive font selector (`Ctrl + Shift + T`).
- **Flexbox Layout Engine & Fine-Grained Reactive Architecture**:
  - Pure Rust W3C CSS Flexbox layout engine combined with fine-grained reactive primitives (`Signal<T>`, `Computed<T>`, `Effect`) and a retained widget tree with `DirtyFlags`.
- **Developer Tooling & Elite LSP Suite**:
  - Built-in LSP client preconfigured for 10 languages (Rust, Python, C/C++, TypeScript/JS, Go, HTML/CSS, JSON, Markdown, Zig, and Julia) supporting Go to Definition (`F12`), References (`Ctrl+Shift+U`), Workspace Rename (`F2`), Code Actions (`Ctrl+.`), Document Symbols (`Ctrl+Shift+R`), Inlay Hints, Semantic Tokens (`F5`), and Signature Help.
  - Real-time Git integration: Live gutter diff, $\mathcal{O}(1)$ inline git blame, Hunk Diff Popup (`Alt+H`), and atomic hunk revert (`Alt+R`).
  - Syntax minimap (`Ctrl+M`), horizontal/vertical split views (`Ctrl+\`), project file tree (`Ctrl+B`), global search (`Ctrl+Shift+F`), docked terminal panel (`Ctrl+\``), and workspace manager with isolated sessions and Hot Exit.
- **Initial Security Audit (`SEC-01` to `SEC-06`)**:
  - Atomic file saving, extension and draft path confinement, cyclic symlink recursion guards, and secure Wayland clipboard piping.

[v0.2.0]: https://github.com/boleklolek/rustext/compare/v0.1.0...v0.2.0
[v0.1.0]: https://github.com/boleklolek/rustext/releases/tag/v0.1.0
