//! Interactive modal prompt for search, file saving/opening, and command palette.
//! Zero third-party dependencies, purely standard library Rust.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    Find,
    SaveAs,
    OpenFile,
    CommandPalette,
}

impl PromptKind {
    pub fn label(&self) -> &'static str {
        match self {
            PromptKind::Find => "Find: ",
            PromptKind::SaveAs => "Save As: ",
            PromptKind::OpenFile => "Open: ",
            PromptKind::CommandPalette => "Command: ",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptModal {
    pub kind: PromptKind,
    pub input: String,
    pub cursor_col: usize,
}

impl PromptModal {
    pub fn new(kind: PromptKind) -> Self {
        Self {
            kind,
            input: String::new(),
            cursor_col: 0,
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        let char_count = self.input.chars().count();
        let idx = self.cursor_col.min(char_count);

        let mut new_input = String::with_capacity(self.input.len() + 4);
        for (i, c) in self.input.chars().enumerate() {
            if i == idx {
                new_input.push(ch);
            }
            new_input.push(c);
        }
        if idx == char_count {
            new_input.push(ch);
        }

        self.input = new_input;
        self.cursor_col += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor_col == 0 || self.input.is_empty() {
            return;
        }

        let delete_idx = self.cursor_col - 1;
        self.input = self
            .input
            .chars()
            .enumerate()
            .filter(|(i, _)| *i != delete_idx)
            .map(|(_, c)| c)
            .collect();

        self.cursor_col -= 1;
    }

    pub fn formatted_display(&self) -> String {
        format!("{}{}", self.kind.label(), self.input)
    }
}
