//! Rustext Binary Entrypoint.
//! Pure standard-library Rust with zero external crates.

use std::env;
use std::path::PathBuf;

use rustext::core::TextBuffer;
use rustext::editor::Editor;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_version() {
    println!("rustext {} (GNU AGPLv3 - Copyleft Libre)", VERSION);
}

fn print_help() {
    println!(
        concat!(
            "Rustext: Editor de Código Ultraligero y Agnóstico en Rust Puro (Std-Only)\n\n",
            "USO:\n",
            "    rustext [OPCIONES] [ARCHIVO]\n\n",
            "OPCIONES:\n",
            "    -g, --gui        Abrir en ventana gráfica nativa (Linux Wayland/X11, Windows, macOS)\n",
            "    -t, --tui        Abrir en modo terminal TUI (por defecto)\n",
            "    -v, --version    Mostrar información de versión y licencia\n",
            "    -h, --help       Mostrar esta ayuda\n\n",
            "VARIABLES DE ENTORNO:\n",
            "    RUSTEXT_MODE=gui                 Abrir en modo ventana gráfica por defecto\n",
            "    RUSTEXT_BACKEND=wayland|x11      Forzar backend gráfico específico en Linux\n"
        )
    );
}

#[cfg(target_os = "linux")]
fn ensure_desktop_integration() {
    if let Ok(home) = env::var("HOME") {
        let app_dir = PathBuf::from(&home).join(".local/share/applications");
        let icon_dir = PathBuf::from(&home).join(".local/share/icons/hicolor/scalable/apps");
        let desktop_file = app_dir.join("rustext.desktop");
        let icon_file = icon_dir.join("rustext.svg");

        let mut changed = false;
        if !desktop_file.exists() {
            let _ = std::fs::create_dir_all(&app_dir);
            let _ = std::fs::write(&desktop_file, include_str!("../assets/rustext.desktop"));
            changed = true;
        }

        if !icon_file.exists() {
            let _ = std::fs::create_dir_all(&icon_dir);
            let _ = std::fs::write(&icon_file, include_str!("../assets/logo.svg"));
            changed = true;
        }

        if changed {
            let _ = std::process::Command::new("update-desktop-database").arg(&app_dir).status();
            let _ = std::process::Command::new("kbuildsycoca6").arg("--noincremental").status();
            let _ = std::process::Command::new("gtk-update-icon-cache")
                .arg("-f")
                .arg("-t")
                .arg(PathBuf::from(&home).join(".local/share/icons/hicolor"))
                .status();
        }

        if let Ok(current_exe) = env::current_exe() {
            let cargo_bin = PathBuf::from(&home).join(".cargo/bin");
            let target_link = cargo_bin.join("rustext");
            if cargo_bin.exists() && !target_link.exists() {
                #[cfg(unix)]
                let _ = std::os::unix::fs::symlink(&current_exe, &target_link);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    ensure_desktop_integration();

    // Install panic hook to restore terminal state if an unhandled panic occurs (SEC-06)
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        use std::io::Write;
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(b"\x1b[?25h\x1b[0m\x1b[?1049l\r\n");
        let _ = stdout.flush();
        #[cfg(unix)]
        let _ = std::process::Command::new("stty").arg("sane").status();
        default_panic(info);
    }));

    let raw_args: Vec<String> = env::args().skip(1).collect();

    if raw_args.iter().any(|a| a == "-v" || a == "--version") {
        print_version();
        return Ok(());
    }

    if raw_args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return Ok(());
    }

    let env_gui = env::var("RUSTEXT_MODE")
        .map(|v| v.eq_ignore_ascii_case("gui"))
        .unwrap_or(false);

    let mut force_gui = env_gui;
    let mut force_tui = false;
    let mut file_arg: Option<String> = None;

    for arg in &raw_args {
        match arg.as_str() {
            "-g" | "--gui" => force_gui = true,
            "-t" | "--tui" => force_tui = true,
            other if !other.starts_with('-') && file_arg.is_none() => {
                file_arg = Some(other.to_string());
            }
            _ => {}
        }
    }

    let use_gui = force_gui && !force_tui;

    let buffer = if let Some(ref file_str) = file_arg {
        let path = PathBuf::from(file_str);
        if path.exists() {
            TextBuffer::open(&path)?
        } else {
            let mut buf = TextBuffer::new();
            buf.set_file_path(path);
            buf
        }
    } else {
        // Welcome buffer
        let welcome = concat!(
            "// ====================================================================\n",
            "//  Rustext: Editor de Código Ultraligero en Rust Puro (Std-Only)\n",
            "//  Cero dependencias externas. Latencia mínima. Multicursores.\n",
            "//  Soporte Agnóstico: Linux (X11), Windows (Win32), macOS (Cocoa)\n",
            "// ====================================================================\n\n",
            "fn main() {\n",
            "    println!(\"¡Bienvenido a Rustext!\");\n",
            "}\n\n",
            "// Atajos principales:\n",
            "//   Ctrl+S : Guardar archivo\n",
            "//   Ctrl+F : Buscar texto\n",
            "//   Ctrl+D : Seleccionar siguiente ocurrencia (Multicursor)\n",
            "//   Ctrl+K : Añadir cursor arriba\n",
            "//   Ctrl+J : Añadir cursor abajo\n",
            "//   Ctrl+T : Nueva pestaña\n",
            "//   Ctrl+W : Cerrar pestaña\n",
            "//   Ctrl+O : Abrir archivo\n",
            "//   Ctrl+Z : Deshacer (Undo)\n",
            "//   Ctrl+Y : Rehacer (Redo)\n",
            "//   Ctrl+P : Ir a archivo (Fuzzy File Finder)\n",
            "//   Ctrl+Shift+P : Paleta de comandos\n",
            "//   Esc    : Colapsar a un solo cursor / Cancelar prompt\n",
            "//   Ctrl+Q : Salir\n"
        );
        TextBuffer::from_str(welcome)
    };

    let mut editor = Editor::new(buffer);

    if file_arg.is_none() && editor.config.editor.restore_session {
        editor.restore_session();
    }

    if use_gui {
        match editor.run_gui() {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("[Rustext] Fallback a modo TUI debido a: {}", e);
                editor.run()?;
                Ok(())
            }
        }
    } else {
        editor.run()?;
        Ok(())
    }
}
