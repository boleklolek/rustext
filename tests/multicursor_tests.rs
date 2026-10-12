use rustext::core::{Cursor, CursorSet, Language, SearchEngine, SyntaxHighlighter, TextBuffer, TokenType};
use rustext::editor::Editor;

#[test]
fn test_multicursor_simultaneous_typing() {
    let buf = TextBuffer::from_str("a = 1;\nb = 1;\nc = 1;");
    let mut editor = Editor::new(buf);

    // Set cursors at the start of each line
    editor.cursors.set_single(0); // at 'a'
    editor.cursors.add_cursor(Cursor::new(7)); // at 'b'
    editor.cursors.add_cursor(Cursor::new(14)); // at 'c'

    assert_eq!(editor.cursors.cursors().len(), 3);

    // Type "pub " at all cursors
    for ch in "pub ".chars() {
        editor.insert_char_multicursor(ch);
    }

    assert_eq!(
        editor.buffer.to_string(),
        "pub a = 1;\npub b = 1;\npub c = 1;"
    );

    // Undo the 4 characters typed
    for _ in 0..4 {
        editor.history.undo(&mut editor.buffer);
    }

    assert_eq!(editor.buffer.to_string(), "a = 1;\nb = 1;\nc = 1;");
}

#[test]
fn test_syntax_tokenizer_rust() {
    let line = "pub fn calculate(val: u32) -> String { // comment";
    let tokens = SyntaxHighlighter::tokenize_line(line, Language::Rust);

    let keywords: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == TokenType::Keyword)
        .map(|t| t.text.as_str())
        .collect();

    assert!(keywords.contains(&"pub"));
    assert!(keywords.contains(&"fn"));

    let types: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == TokenType::Type)
        .map(|t| t.text.as_str())
        .collect();

    assert!(types.contains(&"u32"));
    assert!(types.contains(&"String"));

    let comments: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == TokenType::Comment)
        .collect();

    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].text, "// comment");
}

#[test]
fn test_search_and_expansion() {
    let buf = TextBuffer::from_str("apple orange apple banana apple");
    let mut cs = CursorSet::single(0);

    // Select first "apple"
    SearchEngine::select_next_occurrence(&buf, &mut cs);
    assert_eq!(cs.cursors().len(), 1);
    assert_eq!(cs.cursors()[0].selection_range(), 0..5);

    // Select second "apple"
    SearchEngine::select_next_occurrence(&buf, &mut cs);
    assert_eq!(cs.cursors().len(), 2);
    assert_eq!(cs.cursors()[1].selection_range(), 13..18);

    // Select third "apple"
    SearchEngine::select_next_occurrence(&buf, &mut cs);
    assert_eq!(cs.cursors().len(), 3);
    assert_eq!(cs.cursors()[2].selection_range(), 26..31);
}
