//! Jump list and navigation operations.
//! Zero third-party dependencies, pure standard library Rust.

use super::app::Editor;

impl Editor {
    pub fn current_jump_location(&self) -> crate::editor::jump_list::JumpLocation {
        let doc = self.doc();
        let head = doc.cursors.primary().head;
        let pos = doc.buffer.char_to_position(head);
        crate::editor::jump_list::JumpLocation::new(
            doc.buffer.file_path().map(|p| p.to_path_buf()),
            head,
            pos.line,
            pos.col,
        )
    }

    /// Records current position before a major jump.
    pub fn record_jump_location(&mut self) {
        let loc = self.current_jump_location();
        self.jump_list.record(loc);
    }

    /// Navigates to a JumpLocation safely.
    pub fn navigate_to_jump_location(&mut self, target: &crate::editor::jump_list::JumpLocation) {
        if let Some(ref path) = target.file_path {
            self.open_file_internal(path, false);
        }
        let doc = self.doc_mut();
        let total_lines = doc.buffer.len_lines();
        let line = target.line.min(total_lines.saturating_sub(1));
        let line_len = doc.buffer.line_len(line);
        let col = target.col.min(line_len);
        let offset = doc.buffer.position_to_char(crate::core::buffer::Position::new(line, col));
        doc.cursors.set_single(offset);
    }

    /// Jumps back in navigation history (Alt+Left).
    pub fn jump_back(&mut self) {
        let current = self.current_jump_location();
        if let Some(target) = self.jump_list.jump_back(current) {
            self.navigate_to_jump_location(&target);
            self.set_status("Navegado hacia atrás en la pila de saltos");
        } else {
            self.set_status("No hay saltos anteriores en la pila");
        }
    }

    /// Jumps forward in navigation history (Alt+Right).
    pub fn jump_forward(&mut self) {
        let current = self.current_jump_location();
        if let Some(target) = self.jump_list.jump_forward(current) {
            self.navigate_to_jump_location(&target);
            self.set_status("Navegado hacia adelante en la pila de saltos");
        } else {
            self.set_status("No hay saltos posteriores en la pila");
        }
    }
}
