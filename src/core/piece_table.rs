//! Piece Table implementation for high-performance text buffer manipulation.
//! Zero third-party dependencies, purely standard library Rust.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferSource {
    Original,
    Add,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub source: BufferSource,
    pub start: usize,    // byte offset within the source buffer
    pub length: usize,   // byte length
    pub char_len: usize, // character count
}

#[derive(Debug, Clone)]
pub struct PieceTable {
    original: String,
    add: String,
    pieces: Vec<Piece>,
    total_chars: usize,
}

impl PieceTable {
    /// Creates a new empty PieceTable.
    pub fn new() -> Self {
        Self {
            original: String::new(),
            add: String::new(),
            pieces: Vec::new(),
            total_chars: 0,
        }
    }

    /// Creates a PieceTable from initial text.
    pub fn from_str(text: &str) -> Self {
        if text.is_empty() {
            return Self::new();
        }

        let char_len = text.chars().count();
        let piece = Piece {
            source: BufferSource::Original,
            start: 0,
            length: text.len(),
            char_len,
        };

        Self {
            original: text.to_string(),
            add: String::new(),
            pieces: vec![piece],
            total_chars: char_len,
        }
    }

    /// Returns the total number of characters in the buffer.
    pub fn len_chars(&self) -> usize {
        self.total_chars
    }

    /// Checks if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.total_chars == 0
    }

    /// Returns the total byte length across all pieces.
    pub fn len_bytes(&self) -> usize {
        self.pieces.iter().map(|p| p.length).sum()
    }

    /// Returns the number of pieces currently in the table.
    pub fn piece_count(&self) -> usize {
        self.pieces.len()
    }

    /// Provides zero-copy iterator over all string slices in the piece table.
    pub fn chunks(&self) -> impl Iterator<Item = &str> {
        self.pieces.iter().map(|piece| self.piece_text(piece))
    }

    /// Helper to get a string slice reference for a given piece.
    fn piece_text<'a>(&'a self, piece: &Piece) -> &'a str {
        match piece.source {
            BufferSource::Original => &self.original[piece.start..piece.start + piece.length],
            BufferSource::Add => &self.add[piece.start..piece.start + piece.length],
        }
    }

    /// Finds byte offset within a piece corresponding to a character offset relative to the piece start.
    fn byte_offset_for_char(&self, piece: &Piece, char_offset: usize) -> usize {
        if char_offset == 0 {
            return 0;
        }
        if char_offset >= piece.char_len {
            return piece.length;
        }

        let text = self.piece_text(piece);
        let mut count = 0;
        for (byte_idx, _) in text.char_indices() {
            if count == char_offset {
                return byte_idx;
            }
            count += 1;
        }
        piece.length
    }

    /// Inserts text at the specified character offset.
    pub fn insert(&mut self, char_idx: usize, text: &str) {
        if text.is_empty() {
            return;
        }

        let char_idx = char_idx.min(self.total_chars);
        let text_char_len = text.chars().count();
        let text_byte_len = text.len();

        let add_start = self.add.len();
        self.add.push_str(text);

        let new_piece = Piece {
            source: BufferSource::Add,
            start: add_start,
            length: text_byte_len,
            char_len: text_char_len,
        };

        self.total_chars += text_char_len;

        if self.pieces.is_empty() {
            self.pieces.push(new_piece);
            return;
        }

        if char_idx == 0 {
            self.pieces.insert(0, new_piece);
            return;
        }

        if char_idx == self.total_chars - text_char_len {
            self.pieces.push(new_piece);
            return;
        }

        // Find piece where char_idx falls
        let mut accum_chars = 0;
        for i in 0..self.pieces.len() {
            let piece = self.pieces[i];
            let next_accum = accum_chars + piece.char_len;

            if char_idx == accum_chars {
                self.pieces.insert(i, new_piece);
                return;
            }

            if char_idx < next_accum {
                // Split piece at char_idx - accum_chars
                let split_char = char_idx - accum_chars;
                let split_byte = self.byte_offset_for_char(&piece, split_char);

                let left = Piece {
                    source: piece.source,
                    start: piece.start,
                    length: split_byte,
                    char_len: split_char,
                };

                let right = Piece {
                    source: piece.source,
                    start: piece.start + split_byte,
                    length: piece.length - split_byte,
                    char_len: piece.char_len - split_char,
                };

                self.pieces.splice(i..=i, [left, new_piece, right]);
                return;
            }

            accum_chars = next_accum;
        }

        self.pieces.push(new_piece);
    }

    /// Deletes a range of characters [start_char..end_char].
    pub fn delete(&mut self, start_char: usize, end_char: usize) {
        let start_char = start_char.min(self.total_chars);
        let end_char = end_char.min(self.total_chars);

        if start_char >= end_char || self.pieces.is_empty() {
            return;
        }

        let delete_count = end_char - start_char;
        self.total_chars -= delete_count;

        let mut new_pieces = Vec::with_capacity(self.pieces.len());
        let mut accum_chars = 0;

        for piece in &self.pieces {
            let piece_start = accum_chars;
            let piece_end = accum_chars + piece.char_len;
            accum_chars = piece_end;

            // Case 1: Piece is completely before the deletion range
            if piece_end <= start_char {
                new_pieces.push(*piece);
                continue;
            }

            // Case 2: Piece is completely after the deletion range
            if piece_start >= end_char {
                new_pieces.push(*piece);
                continue;
            }

            // Case 3: Deletion covers the entire piece -> drop it
            if piece_start >= start_char && piece_end <= end_char {
                continue;
            }

            // Case 4: Deletion starts inside this piece and ends inside this piece (piece split into 2)
            if piece_start < start_char && piece_end > end_char {
                let left_chars = start_char - piece_start;
                let left_bytes = self.byte_offset_for_char(piece, left_chars);
                let left = Piece {
                    source: piece.source,
                    start: piece.start,
                    length: left_bytes,
                    char_len: left_chars,
                };

                let right_skip_chars = end_char - piece_start;
                let right_skip_bytes = self.byte_offset_for_char(piece, right_skip_chars);
                let right = Piece {
                    source: piece.source,
                    start: piece.start + right_skip_bytes,
                    length: piece.length - right_skip_bytes,
                    char_len: piece.char_len - right_skip_chars,
                };

                new_pieces.push(left);
                new_pieces.push(right);
                continue;
            }

            // Case 5: Deletion starts inside this piece and extends past it (keep left part)
            if piece_start < start_char && piece_end > start_char {
                let left_chars = start_char - piece_start;
                let left_bytes = self.byte_offset_for_char(piece, left_chars);
                let left = Piece {
                    source: piece.source,
                    start: piece.start,
                    length: left_bytes,
                    char_len: left_chars,
                };
                new_pieces.push(left);
                continue;
            }

            // Case 6: Deletion starts before this piece and ends inside it (keep right part)
            if piece_start < end_char && piece_end > end_char {
                let right_skip_chars = end_char - piece_start;
                let right_skip_bytes = self.byte_offset_for_char(piece, right_skip_chars);
                let right = Piece {
                    source: piece.source,
                    start: piece.start + right_skip_bytes,
                    length: piece.length - right_skip_bytes,
                    char_len: piece.char_len - right_skip_chars,
                };
                new_pieces.push(right);
                continue;
            }
        }

        self.pieces = new_pieces;
    }

    /// Extracts a string slice for the character range [start..end].
    pub fn slice_chars(&self, start: usize, end: usize) -> String {
        let start = start.min(self.total_chars);
        let end = end.min(self.total_chars);

        if start >= end {
            return String::new();
        }

        let mut result = String::with_capacity((end - start) * 2);
        let mut accum_chars = 0;

        for piece in &self.pieces {
            let piece_start = accum_chars;
            let piece_end = accum_chars + piece.char_len;
            accum_chars = piece_end;

            if piece_end <= start || piece_start >= end {
                continue;
            }

            let text = self.piece_text(piece);
            let piece_slice_start = if start > piece_start {
                start - piece_start
            } else {
                0
            };
            let piece_slice_end = if end < piece_end {
                end - piece_start
            } else {
                piece.char_len
            };

            let byte_start = self.byte_offset_for_char(piece, piece_slice_start);
            let byte_end = self.byte_offset_for_char(piece, piece_slice_end);

            result.push_str(&text[byte_start..byte_end]);
        }

        result
    }

    /// Returns the complete buffer content as a String.
    pub fn to_string(&self) -> String {
        let total_bytes = self.len_bytes();
        let mut output = String::with_capacity(total_bytes);
        for piece in &self.pieces {
            output.push_str(self.piece_text(piece));
        }
        output
    }

    /// Returns the character at the specified index.
    pub fn char_at(&self, char_idx: usize) -> Option<char> {
        if char_idx >= self.total_chars {
            return None;
        }

        let mut accum_chars = 0;
        for piece in &self.pieces {
            let next_accum = accum_chars + piece.char_len;
            if char_idx < next_accum {
                let offset_in_piece = char_idx - accum_chars;
                let text = self.piece_text(piece);
                return text.chars().nth(offset_in_piece);
            }
            accum_chars = next_accum;
        }
        None
    }
}

impl Default for PieceTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_table() {
        let pt = PieceTable::new();
        assert_eq!(pt.len_chars(), 0);
        assert_eq!(pt.to_string(), "");
    }

    #[test]
    fn test_insertions() {
        let mut pt = PieceTable::from_str("Hello World");
        assert_eq!(pt.len_chars(), 11);
        assert_eq!(pt.to_string(), "Hello World");

        pt.insert(5, ", Rust");
        assert_eq!(pt.to_string(), "Hello, Rust World");
        assert_eq!(pt.len_chars(), 17);

        pt.insert(0, "[START] ");
        assert_eq!(pt.to_string(), "[START] Hello, Rust World");

        let len = pt.len_chars();
        pt.insert(len, " [END]");
        assert_eq!(pt.to_string(), "[START] Hello, Rust World [END]");
    }

    #[test]
    fn test_deletions() {
        let mut pt = PieceTable::from_str("0123456789");
        pt.delete(3, 7); // deletes "3456"
        assert_eq!(pt.to_string(), "012789");
        assert_eq!(pt.len_chars(), 6);

        // Delete from start
        pt.delete(0, 3);
        assert_eq!(pt.to_string(), "789");

        // Delete to end
        pt.delete(1, 3);
        assert_eq!(pt.to_string(), "7");

        pt.delete(0, 1);
        assert_eq!(pt.to_string(), "");
        assert_eq!(pt.len_chars(), 0);
    }

    #[test]
    fn test_utf8_multibyte() {
        let mut pt = PieceTable::from_str("Hola Mundo 🦀");
        assert_eq!(pt.len_chars(), 12);

        pt.insert(10, " feliz");
        assert_eq!(pt.to_string(), "Hola Mundo feliz 🦀");
        assert_eq!(pt.char_at(17), Some('🦀'));

        pt.delete(10, 16);
        assert_eq!(pt.to_string(), "Hola Mundo 🦀");
    }

    #[test]
    fn test_slice_chars() {
        let pt = PieceTable::from_str("The quick brown fox jumps over the lazy dog");
        assert_eq!(pt.slice_chars(4, 9), "quick");
        assert_eq!(pt.slice_chars(16, 19), "fox");
    }
}
