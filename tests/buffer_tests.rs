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

#[test]
fn test_large_file_utf8_fast_navigation() {
    // Generate 15,000 lines of UTF-8 text containing accents and unicode
    let mut large_text = String::with_capacity(1_500_000);
    for i in 0..15_000 {
        large_text.push_str(&format!("Línea número {} con caracteres españoles: á, é, í, ó, ú, ñ, ¡hola! 🦀\n", i));
    }

    let start_load = std::time::Instant::now();
    let buf = TextBuffer::from_str(&large_text);
    let dur_load = start_load.elapsed();
    assert!(dur_load.as_millis() < 500, "Load took too long: {:?}", dur_load);
    assert_eq!(buf.len_lines(), 15_001);

    // Test line length and line extraction at line 14,000 (deep into the file)
    assert_eq!(buf.line_len_chars(14_000), "Línea número 14000 con caracteres españoles: á, é, í, ó, ú, ñ, ¡hola! 🦀".chars().count());
    let deep_line = buf.line(14_000).expect("Should find line 14000");
    assert!(deep_line.contains("14000 con caracteres"));

    // Simulate scrolling through 100 frames, reading 40 visible lines per frame (4,000 line reads total)
    let start_scroll = std::time::Instant::now();
    for frame in 0..100 {
        let scroll_row = frame * 120; // jump through file
        for r in 0..40 {
            let l_idx = scroll_row + r;
            let _ = buf.line(l_idx);
        }
    }
    let dur_scroll = start_scroll.elapsed();
    // 4,000 line reads across 15,000 multi-byte lines should take < 150ms in debug, < 15ms in release
    assert!(dur_scroll.as_millis() < 150, "Scrolling 100 frames took too long: {:?}", dur_scroll);
}

