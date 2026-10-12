//! Atomic Undo/Redo transaction engine for multi-cursor and buffer mutations.
//! Zero third-party dependencies, purely standard library Rust.

use super::buffer::TextBuffer;
use super::cursor::Cursor;

/// Represents an elementary reversible buffer operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOp {
    Insert { offset: usize, text: String },
    Delete { offset: usize, text: String },
}

impl EditOp {
    pub fn invert(&self) -> Self {
        match self {
            EditOp::Insert { offset, text } => EditOp::Delete {
                offset: *offset,
                text: text.clone(),
            },
            EditOp::Delete { offset, text } => EditOp::Insert {
                offset: *offset,
                text: text.clone(),
            },
        }
    }
}

/// A group of atomic operations executed together (e.g. multi-cursor edits).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub ops: Vec<EditOp>,
    pub cursors_before: Vec<Cursor>,
    pub cursors_after: Vec<Cursor>,
}

impl Transaction {
    pub fn new(cursors_before: Vec<Cursor>) -> Self {
        Self {
            ops: Vec::new(),
            cursors_before,
            cursors_after: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

/// Undo and Redo manager with grouped transactions.
#[derive(Debug, Clone, Default)]
pub struct History {
    undo_stack: Vec<Transaction>,
    redo_stack: Vec<Transaction>,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Records a completed transaction onto the undo stack and clears redo history.
    pub fn record(&mut self, transaction: Transaction) {
        if !transaction.is_empty() {
            self.undo_stack.push(transaction);
            self.redo_stack.clear();
        }
    }

    /// Undoes the most recent transaction, updating the buffer and returning the restored cursors.
    pub fn undo(&mut self, buffer: &mut TextBuffer) -> Option<Vec<Cursor>> {
        let transaction = self.undo_stack.pop()?;

        // Apply operations in reverse order
        for op in transaction.ops.iter().rev() {
            match op {
                EditOp::Insert { offset, text } => {
                    let char_count = text.chars().count();
                    buffer.delete_range(*offset, *offset + char_count);
                }
                EditOp::Delete { offset, text } => {
                    buffer.insert(*offset, text);
                }
            }
        }

        let restored_cursors = transaction.cursors_before.clone();
        self.redo_stack.push(transaction);
        Some(restored_cursors)
    }

    /// Redoes the most recently undone transaction, updating the buffer and returning restored cursors.
    pub fn redo(&mut self, buffer: &mut TextBuffer) -> Option<Vec<Cursor>> {
        let transaction = self.redo_stack.pop()?;

        // Apply operations in normal order
        for op in &transaction.ops {
            match op {
                EditOp::Insert { offset, text } => {
                    buffer.insert(*offset, text);
                }
                EditOp::Delete { offset, text } => {
                    let char_count = text.chars().count();
                    buffer.delete_range(*offset, *offset + char_count);
                }
            }
        }

        let restored_cursors = transaction.cursors_after.clone();
        self.undo_stack.push(transaction);
        Some(restored_cursors)
    }

    /// Returns the number of undoable actions available.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Returns the number of redoable actions available.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_redo_cycle() {
        let mut buf = TextBuffer::from_str("Hello");
        let mut history = History::new();

        let mut tx = Transaction::new(vec![Cursor::new(5)]);
        buf.insert(5, " World");
        tx.ops.push(EditOp::Insert {
            offset: 5,
            text: " World".to_string(),
        });
        tx.cursors_after = vec![Cursor::new(11)];
        history.record(tx);

        assert_eq!(buf.to_string(), "Hello World");

        // Undo
        let cursors = history.undo(&mut buf).unwrap();
        assert_eq!(buf.to_string(), "Hello");
        assert_eq!(cursors[0].head, 5);

        // Redo
        let cursors_redo = history.redo(&mut buf).unwrap();
        assert_eq!(buf.to_string(), "Hello World");
        assert_eq!(cursors_redo[0].head, 11);
    }
}
