//! Rustext Binary Entrypoint.
//! Pure standard-library Rust with zero external crates.

use std::env;
use std::path::PathBuf;

use rustext::core::TextBuffer;
use rustext::editor::Editor;

fn print_help() {
    println!(
        concat!(
            "Rustext: Editor de Código Ultraligero y Agnóstico en Rust Puro (Std-Only)\n\n",
            "USO:\n",
            "    rustext [OPCIONES] [ARCHIVO]\n\n",
            "OPCIONES:\n",
            "    -g, --gui        Abrir en ventana gráfica nativa (Linux X11 / Win32 / macOS Cocoa)\n",
            "    -t, --tui        Abrir en modo terminal TUI (por defecto)\n",
            "    -h, --help       Mostrar esta ayuda\n\n",
            "VARIABLES DE ENTORNO:\n",
            "    RUSTEXT_MODE=gui Abrir en modo ventana gráfica por defecto\n"
        )
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_args: Vec<String> = env::args().skip(1).collect();

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

    let buffer = if let Some(file_str) = file_arg {
        let path = PathBuf::from(&file_str);
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
            "//   Ctrl+P : Paleta de comandos\n",
            "//   Esc    : Colapsar a un solo cursor / Cancelar prompt\n",
            "//   Ctrl+Q : Salir\n"
        );
        TextBuffer::from_str(welcome)
    };

    let mut editor = Editor::new(buffer);

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
