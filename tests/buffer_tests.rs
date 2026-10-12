use rustext::core::{PieceTable, TextBuffer, Position};

#[test]
fn test_large_insertions_and_lines() {
    let mut buf = TextBuffer::new();
    for i in 0..100 {
        buf.insert_at_pos(Position::new(i, 0), &format!("Line number {}\n", i));
    }

    assert_eq!(buf.len_lines(), 101);
    assert_eq!(buf.line(0), Some("Line number 0".to_string()));
    assert_eq!(buf.line(99), Some("Line number 99".to_string()));
}

#[test]
fn test_piece_table_stress() {
    let mut pt = PieceTable::from_str("Initial");
    for i in 0..500 {
        pt.insert(i % (pt.len_chars() + 1), "x");
    }
    assert_eq!(pt.len_chars(), 507);
    for _ in 0..100 {
        pt.delete(0, 1);
    }
    assert_eq!(pt.len_chars(), 407);
}
