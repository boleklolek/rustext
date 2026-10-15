use rustext::core::{Cursor, CursorSet, Language, SearchEngine, SyntaxHighlighter, TextBuffer, TokenType};
use rustext::editor::{Editor, EditorMode};
use rustext::ui::Key;

#[test]
fn test_multicursor_simultaneous_typing() {
    let buf = TextBuffer::from_str("a = 1;\nb = 1;\nc = 1;");
    let mut editor = Editor::new(buf);

    // Set cursors at the start of each line
    editor.doc_mut().cursors.set_single(0); // at 'a'
    editor.doc_mut().cursors.add_cursor(Cursor::new(7)); // at 'b'
    editor.doc_mut().cursors.add_cursor(Cursor::new(14)); // at 'c'

    assert_eq!(editor.doc().cursors.cursors().len(), 3);

    // Type "pub " at all cursors
    for ch in "pub ".chars() {
        editor.insert_char_multicursor(ch);
    }

    assert_eq!(
        editor.doc().buffer.to_string(),
        "pub a = 1;\npub b = 1;\npub c = 1;"
    );

    // Undo the 4 characters typed
    for _ in 0..4 {
        let doc = editor.doc_mut();
        doc.history.undo(&mut doc.buffer);
    }

    assert_eq!(editor.doc().buffer.to_string(), "a = 1;\nb = 1;\nc = 1;");
}

#[test]
fn test_clipboard_and_tabs() {
    let buf = TextBuffer::from_str("hello world");
    let mut editor = Editor::new(buf);

    // Select "world"
    editor.doc_mut().cursors.set_single(6);
    editor.doc_mut().cursors.cursors_mut()[0] = Cursor::with_selection(6, 11);

    // Copy to clipboard
    editor.copy();
    assert_eq!(editor.clipboard.get_text(), "world");

    // Open a new tab
    editor.new_tab();
    assert_eq!(editor.documents.len(), 2);
    assert_eq!(editor.active_doc_idx, 1);

    // Paste in new tab
    editor.paste();
    assert_eq!(editor.doc().buffer.to_string(), "world");

    // Switch back to tab 0
    editor.prev_tab();
    assert_eq!(editor.active_doc_idx, 0);
    assert_eq!(editor.doc().buffer.to_string(), "hello world");

    // Close tab 1 (prompts confirmation for unsaved changes, discard with 'd')
    editor.next_tab();
    editor.close_tab();
    assert!(matches!(editor.mode, EditorMode::UnsavedConfirm(_)));
    editor.handle_key(Key::Char('d'));
    assert_eq!(editor.documents.len(), 1);
    assert_eq!(editor.doc().buffer.to_string(), "hello world");
}

#[test]
fn test_syntax_tokenizer_rust() {
    let line = "pub fn calculate(val: u32) -> String { // comment";
    let tokens = SyntaxHighlighter::tokenize_line(line, Language::Rust);

    let keywords: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == TokenType::Keyword)
        .map(|t| t.text)
        .collect();

    assert!(keywords.contains(&"pub"));
    assert!(keywords.contains(&"fn"));

    let types: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == TokenType::Type)
        .map(|t| t.text)
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

#[test]
fn test_auto_close_brackets_and_pairs() {
    use rustext::editor::Editor;

    let mut editor = Editor::new(TextBuffer::new());

    // 1. Typing '(' should auto-close '()' and put cursor between them
    editor.insert_char_multicursor('(');
    assert_eq!(editor.doc().buffer.to_string(), "()");
    assert_eq!(editor.doc().cursors.primary().head, 1);

    // 2. Overtyping ')' should not duplicate, but advance cursor past ')'
    editor.insert_char_multicursor(')');
    assert_eq!(editor.doc().buffer.to_string(), "()");
    assert_eq!(editor.doc().cursors.primary().head, 2);

    // 3. Typing '[' produces '[]'
    editor.insert_char_multicursor('[');
    assert_eq!(editor.doc().buffer.to_string(), "()[]");
    assert_eq!(editor.doc().cursors.primary().head, 3);

    // 4. Typing '{' produces '{}'
    editor.insert_char_multicursor('{');
    assert_eq!(editor.doc().buffer.to_string(), "()[{}]");
    assert_eq!(editor.doc().cursors.primary().head, 4);

    // 5. Typing '"' produces '""'
    editor.insert_char_multicursor('"');
    assert_eq!(editor.doc().buffer.to_string(), "()[{\"\"}]");
    assert_eq!(editor.doc().cursors.primary().head, 5);
}

#[test]
fn test_surround_selection() {
    use rustext::editor::Editor;

    let mut editor = Editor::new(TextBuffer::from_str("palabra"));
    // Select "palabra" (0..7)
    editor.doc_mut().cursors.cursors_mut()[0] = rustext::core::Cursor::with_selection(0, 7);

    // Typing '(' should surround with '(' and ')'
    editor.insert_char_multicursor('(');
    assert_eq!(editor.doc().buffer.to_string(), "(palabra)");
    assert_eq!(editor.doc().cursors.primary().selection_range(), 1..8);

    // Typing '[' should surround again
    editor.insert_char_multicursor('[');
    assert_eq!(editor.doc().buffer.to_string(), "([palabra])");
    assert_eq!(editor.doc().cursors.primary().selection_range(), 2..9);
}

#[test]
fn test_smart_backspace_pairs_and_dedent() {
    use rustext::editor::Editor;

    let mut editor = Editor::new(TextBuffer::new());

    // Type '(' -> '()'
    editor.insert_char_multicursor('(');
    assert_eq!(editor.doc().buffer.to_string(), "()");
    assert_eq!(editor.doc().cursors.primary().head, 1);

    // Backspace should delete BOTH brackets
    editor.backspace_multicursor();
    assert_eq!(editor.doc().buffer.to_string(), "");
    assert_eq!(editor.doc().cursors.primary().head, 0);

    // Test smart tab dedent (4 spaces deleted at once)
    editor.insert_tab_multicursor();
    assert_eq!(editor.doc().buffer.to_string(), "    ");
    assert_eq!(editor.doc().cursors.primary().head, 4);

    editor.backspace_multicursor();
    assert_eq!(editor.doc().buffer.to_string(), "");
    assert_eq!(editor.doc().cursors.primary().head, 0);
}

#[test]
fn test_auto_indent_newline_expansion() {
    use rustext::editor::Editor;

    // 1. Enter between braces: {|} -> splits across 3 lines with indented body
    let mut editor = Editor::new(TextBuffer::from_str("{}"));
    editor.doc_mut().cursors.set_single(1);
    editor.insert_newline_multicursor();
    assert_eq!(editor.doc().buffer.to_string(), "{\n    \n}");
    assert_eq!(editor.doc().cursors.primary().head, 6);

    // 2. Enter after opening brace: "fn test() {" -> new line with +4 indentation
    let mut editor2 = Editor::new(TextBuffer::from_str("fn test() {"));
    editor2.doc_mut().cursors.set_single(11);
    editor2.insert_newline_multicursor();
    assert_eq!(editor2.doc().buffer.to_string(), "fn test() {\n    ");
    assert_eq!(editor2.doc().cursors.primary().head, 16);

    // 3. Enter on indented line maintains indent: "    let x = 1;"
    let mut editor3 = Editor::new(TextBuffer::from_str("    let x = 1;"));
    editor3.doc_mut().cursors.set_single(14);
    editor3.insert_newline_multicursor();
    assert_eq!(editor3.doc().buffer.to_string(), "    let x = 1;\n    ");
    assert_eq!(editor3.doc().cursors.primary().head, 19);

    // 4. Typing '}' on empty indented line dedents automatically
    editor3.insert_char_multicursor('}');
    assert_eq!(editor3.doc().buffer.to_string(), "    let x = 1;\n}");
    assert_eq!(editor3.doc().cursors.primary().head, 16);
}

#[test]
fn test_multicursor_auto_close_and_surround() {
    use rustext::editor::Editor;
    use rustext::core::Cursor;

    let mut editor = Editor::new(TextBuffer::from_str("line1\nline2\nline3"));
    // Place cursors at end of line 1 (5), line 2 (11), line 3 (17)
    editor.doc_mut().cursors.set_cursors(vec![
        Cursor::new(5),
        Cursor::new(11),
        Cursor::new(17),
    ]);

    // Type '(' across all 3 cursors simultaneously
    editor.insert_char_multicursor('(');
    assert_eq!(editor.doc().buffer.to_string(), "line1()\nline2()\nline3()");

    // Backspace across all 3 cursors simultaneously deletes both brackets
    editor.backspace_multicursor();
    assert_eq!(editor.doc().buffer.to_string(), "line1\nline2\nline3");
}


