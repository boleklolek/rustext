//! Rustext Binary Entrypoint.
//! Pure standard-library Rust with zero external crates.

use std::env;
use std::path::PathBuf;

use rustext::core::TextBuffer;
use rustext::editor::Editor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    let buffer = if args.len() > 1 {
        let file_arg = &args[1];
        let path = PathBuf::from(file_arg);

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
            "//   Ctrl+Z : Deshacer (Undo)\n",
            "//   Ctrl+Y : Rehacer (Redo)\n",
            "//   Ctrl+P : Paleta de comandos\n",
            "//   Esc    : Colapsar a un solo cursor / Cancelar prompt\n",
            "//   Ctrl+Q : Salir\n"
        );
        TextBuffer::from_str(welcome)
    };

    let mut editor = Editor::new(buffer);
    editor.run()?;

    Ok(())
}
