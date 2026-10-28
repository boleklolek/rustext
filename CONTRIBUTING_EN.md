# 🤝 Contributing to Rustext (`CONTRIBUTING.md`)

Thank you for your interest in contributing to **Rustext**! This document outlines our core architectural principles, development workflow, and quality standards that ensure Rustext stays ultra-fast, secure, and completely dependency-free.

---

## 🏛️ 1. Golden Rule: Zero External Dependencies (`std::*` Only)

The foundational, non-negotiable principle of **Rustext** is that the `[dependencies]` section of [`Cargo.toml`](Cargo.toml) **must remain completely empty**.

- **No third-party crates** (neither in `[dependencies]`, `[dev-dependencies]`, nor `[build-dependencies]`).
- Every algorithm, protocol (LSP, JSON, INI, Wayland wire protocol, SFNT TrueType, Flexbox), and data structure must be implemented in **pure Rust using exclusively the standard library (`std`)**.
- Operating system interactions (Wayland, X11, Win32, Cocoa) are done directly via system IPC sockets or minimal FFI bindings to pre-installed OS libraries.

---

## 🛡️ 2. Code & Security Standards

1. **Rust Edition**: The project uses **Rust 2024 Edition**.
2. **Memory Safety (`Safe Rust`)**:
   - The entire core editor (`src/core/`, `src/editor/`, `src/gfx/`, `src/syntax/`, `src/lsp/`, `src/ui/`) must be written in **100% Safe Rust**.
   - `unsafe` blocks are strictly restricted to windowing FFI boundaries (`src/window/`) and essential POSIX/system calls, and must be scoped, audited, and documented.
3. **DoS and Path Traversal Resistance**:
   - Any reading of configuration files, themes, or drafts must enforce size limits and canonical path confinement (see [`SECURITY.md`](SECURITY.md) / [`SECURITY_EN.md`](SECURITY_EN.md)).
   - Never use regular expressions with exponential backtracking or spawn intermediate shells (`sh -c`) for subprocess execution; always pass vector argument slices to `std::process::Command::new()`.
4. **Zero Unnecessary Allocations on Hot Paths**:
   - Viewport rendering and key event handling must remain $\mathcal{O}(1)$ relative to total file size and prevent memory leaks (never use `Box::leak` in UI cycles).

---

## 🛠️ 3. Development & Testing Environment

### Prerequisites
- Official **stable Rust compiler** (`rustc >= 1.85` supporting `edition = "2024"`).
- On Linux: X11 development headers (`libx11-dev` on Debian/Ubuntu) for the fallback X11 backend.

### Core Commands

```bash
# 1. Verify compilation without errors or warnings:
cargo check

# 2. Run the complete test suite (248+ unit and integration tests):
cargo test --tests

# 3. Run comparative benchmarks and memory profiling:
cargo run --release --bin benchmark_comparison
cargo run --release --bin memory_profile
```

All contributions must pass 100% of the existing tests on Linux, Windows, and macOS, and include new tests in [`tests/`](tests/) covering any new feature or bug fix.

---

## 🎨 4. Contributing Themes (`.theme`) and Extensions

You do not need to modify the graphics engine to contribute new themes or extensions:
- **Color Themes**: Create a `<name>.theme` file following the format in [`assets/themes/`](assets/themes/) and instructions in [`MANUAL_TEMAS_Y_EXTENSIONES.md`](MANUAL_TEMAS_Y_EXTENSIONES.md).
- **Snippets & Languages**: You can extend language tokenizers in `src/syntax/` or the snippet registry in `src/editor/snippet.rs`, with corresponding tests in `tests/multicursor_tests.rs`.

---

## 🔄 5. Pull Request (PR) Workflow

1. **Fork** the repository and create a descriptive branch off `main`:
   - `feat/feature-name` for new features.
   - `fix/bug-fix-description` for bug fixes.
   - `docs/documentation-update` for manuals and guides.
   - `sec/security-enhancement` for defensive improvements.
2. Ensure `cargo test --tests` passes with 0 failures and 0 warnings.
3. If your change adds a keybinding, configuration setting, or visible user feature, update [`README.md`](README.md), [`README_EN.md`](README_EN.md), and [`MANUAL.md`](MANUAL.md).
4. Submit the Pull Request clearly describing the problem solved, performance/memory footprint, and verification steps.

---

## 📄 License of Contributions

By contributing to this repository, you agree that your code will be licensed under the project's license: **GNU Affero General Public License v3.0 or later (`AGPL-3.0-or-later`)** (see [`LICENSE`](LICENSE)).
