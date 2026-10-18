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

#[test]
fn test_bracket_pair_colorization_cyclic_depth() {
    use rustext::core::syntax::{SyntaxHighlighter, Language, TokenType};
    use rustext::gfx::ThemeColors;

    // Expression with nested brackets: ([{}])
    let line = "let arr = [( { 42 } )]; // ([{}])";
    let (tokens, final_depth) = SyntaxHighlighter::tokenize_line_with_depth(line, Language::Rust, 0);

    // Final depth should be balanced (0)
    assert_eq!(final_depth, 0);

    // Find all Bracket tokens
    let bracket_tokens: Vec<_> = tokens
        .iter()
        .filter_map(|t| match t.token_type {
            TokenType::Bracket(d) => Some((t.text, d)),
            _ => None,
        })
        .collect();

    // Line has:
    // '[' at depth 0
    // '(' at depth 1
    // '{' at depth 2
    // '}' at depth 2
    // ')' at depth 1
    // ']' at depth 0
    // Note: brackets inside the comment "// ([{}])" are ignored because it's a comment!
    assert_eq!(
        bracket_tokens,
        vec![
            ("[", 0),
            ("(", 1),
            ("{", 2),
            ("}", 2),
            (")", 1),
            ("]", 0),
        ]
    );

    // Brackets inside string literals must also be ignored
    let line_str = "let s = \"([{\"; (ok);";
    let (tokens_str, _) = SyntaxHighlighter::tokenize_line_with_depth(line_str, Language::Rust, 0);
    let str_bracket_tokens: Vec<_> = tokens_str
        .iter()
        .filter_map(|t| match t.token_type {
            TokenType::Bracket(d) => Some((t.text, d)),
            _ => None,
        })
        .collect();

    assert_eq!(str_bracket_tokens, vec![("(", 0), (")", 0)]);

    // Verify ThemeColors mapping for cyclic depth % 3
    let theme = ThemeColors::tokyo_night();
    assert_eq!(theme.token_color(TokenType::Bracket(0)), theme.bracket_0);
    assert_eq!(theme.token_color(TokenType::Bracket(1)), theme.bracket_1);
    assert_eq!(theme.token_color(TokenType::Bracket(2)), theme.bracket_2);
    assert_eq!(theme.token_color(TokenType::Bracket(3)), theme.bracket_0); // cyclic
    assert_eq!(theme.token_color(TokenType::Bracket(4)), theme.bracket_1); // cyclic
}

#[test]
fn test_new_languages_syntax_detection_and_tokenization() {
    use rustext::core::syntax::{Language, SyntaxHighlighter, TokenType};
    use std::path::Path;

    // 1. Language file extension detection
    assert_eq!(Language::from_path(Path::new("index.html")), Language::Html);
    assert_eq!(Language::from_path(Path::new("page.htm")), Language::Html);
    assert_eq!(Language::from_path(Path::new("styles.css")), Language::Css);
    assert_eq!(Language::from_path(Path::new("styles.scss")), Language::Css);
    assert_eq!(Language::from_path(Path::new("docker-compose.yaml")), Language::Yaml);
    assert_eq!(Language::from_path(Path::new("config.yml")), Language::Yaml);
    assert_eq!(Language::from_path(Path::new("deploy.sh")), Language::Shell);
    assert_eq!(Language::from_path(Path::new("setup.bash")), Language::Shell);
    assert_eq!(Language::from_path(Path::new(".zshrc")), Language::Shell);
    assert_eq!(Language::from_path(Path::new(".bashrc")), Language::Shell);
    assert_eq!(Language::from_path(Path::new(".env")), Language::Shell);
    assert_eq!(Language::from_path(Path::new("schema.sql")), Language::Sql);
    assert_eq!(Language::from_path(Path::new("README.md")), Language::Markdown);
    assert_eq!(Language::from_path(Path::new("main.jl")), Language::Julia);
    assert_eq!(Language::from_path(Path::new("script.JL")), Language::Julia);

    // 2. HTML Tokenizer: tags and attributes
    let html_tokens = SyntaxHighlighter::tokenize_line("<div class=\"main\">content</div>", Language::Html);
    assert!(html_tokens.iter().any(|t| t.text == "<div" && t.token_type == TokenType::Keyword));
    assert!(html_tokens.iter().any(|t| t.text == "class" && t.token_type == TokenType::Type));
    assert!(html_tokens.iter().any(|t| t.text == "\"main\"" && t.token_type == TokenType::StringLiteral));

    // 3. CSS Tokenizer: selector, property, hex color
    let css_tokens = SyntaxHighlighter::tokenize_line(".btn { display: flex; color: #ff00aa; }", Language::Css);
    assert!(css_tokens.iter().any(|t| t.text == "display" && t.token_type == TokenType::Keyword));
    assert!(css_tokens.iter().any(|t| t.text == "#ff00aa" && t.token_type == TokenType::Number));

    // 4. YAML Tokenizer: keys and booleans
    let yaml_tokens = SyntaxHighlighter::tokenize_line("port: 8080\nenabled: true", Language::Yaml);
    assert!(yaml_tokens.iter().any(|t| t.text == "8080" && t.token_type == TokenType::Number));
    assert!(yaml_tokens.iter().any(|t| t.text == "true" && t.token_type == TokenType::Keyword));

    // 5. Shell Tokenizer: variables and comments
    let sh_tokens = SyntaxHighlighter::tokenize_line("echo $USER ${HOME} # print user", Language::Shell);
    assert!(sh_tokens.iter().any(|t| t.text == "echo" && t.token_type == TokenType::Keyword));
    assert!(sh_tokens.iter().any(|t| t.text == "$USER" && t.token_type == TokenType::Type));
    assert!(sh_tokens.iter().any(|t| t.text == "${HOME}" && t.token_type == TokenType::Type));
    assert!(sh_tokens.iter().any(|t| t.text == "# print user" && t.token_type == TokenType::Comment));

    // 6. SQL Tokenizer: keywords case-insensitive
    let sql_tokens = SyntaxHighlighter::tokenize_line("SELECT id, name FROM users WHERE active = true;", Language::Sql);
    assert!(sql_tokens.iter().any(|t| t.text == "SELECT" && t.token_type == TokenType::Keyword));
    assert!(sql_tokens.iter().any(|t| t.text == "FROM" && t.token_type == TokenType::Keyword));
    assert!(sql_tokens.iter().any(|t| t.text == "WHERE" && t.token_type == TokenType::Keyword));
    assert!(sql_tokens.iter().any(|t| t.text == "true" && t.token_type == TokenType::Keyword));

    // 7. Markdown Tokenizer: headers and inline code
    let md_header = SyntaxHighlighter::tokenize_line("# Header 1", Language::Markdown);
    assert!(md_header.iter().any(|t| t.text == "#" && t.token_type == TokenType::Keyword));
    assert!(md_header.iter().any(|t| t.text == " Header 1" && t.token_type == TokenType::Type));

    let md_code = SyntaxHighlighter::tokenize_line("Text with `code` block", Language::Markdown);
    assert!(md_code.iter().any(|t| t.text == "`code`" && t.token_type == TokenType::StringLiteral));

    // 8. Julia Tokenizer: function, types, macros, comments, strings and commands
    let jl_tokens = SyntaxHighlighter::tokenize_line("function calc(x::Int64)::Vector{Float64}", Language::Julia);
    assert!(jl_tokens.iter().any(|t| t.text == "function" && t.token_type == TokenType::Keyword));
    assert!(jl_tokens.iter().any(|t| t.text == "Int64" && t.token_type == TokenType::Type));
    assert!(jl_tokens.iter().any(|t| t.text == "Vector" && t.token_type == TokenType::Type));
    assert!(jl_tokens.iter().any(|t| t.text == "Float64" && t.token_type == TokenType::Type));

    let jl_macro_comment = SyntaxHighlighter::tokenize_line("@inline @time foo() #= inline timing =# # line comment", Language::Julia);
    assert!(jl_macro_comment.iter().any(|t| t.text == "@inline" && t.token_type == TokenType::Macro));
    assert!(jl_macro_comment.iter().any(|t| t.text == "@time" && t.token_type == TokenType::Macro));
    assert!(jl_macro_comment.iter().any(|t| t.text == "#= inline timing =#" && t.token_type == TokenType::Comment));
    assert!(jl_macro_comment.iter().any(|t| t.text == "# line comment" && t.token_type == TokenType::Comment));

    let jl_cmd = SyntaxHighlighter::tokenize_line("cmd = `echo hello`", Language::Julia);
    assert!(jl_cmd.iter().any(|t| t.text == "`echo hello`" && t.token_type == TokenType::StringLiteral));
}

#[test]
fn test_snippets_for_new_languages() {
    use rustext::core::syntax::Language;
    use rustext::editor::snippet::SnippetRegistry;

    assert!(SnippetRegistry::find(Language::Html, "html5").is_some());
    assert!(SnippetRegistry::find(Language::Html, "div").is_some());
    assert!(SnippetRegistry::find(Language::Css, "flex").is_some());
    assert!(SnippetRegistry::find(Language::Css, "grid").is_some());
    assert!(SnippetRegistry::find(Language::Yaml, "kv").is_some());
    assert!(SnippetRegistry::find(Language::Shell, "sh").is_some());
    assert!(SnippetRegistry::find(Language::Shell, "func").is_some());
    assert!(SnippetRegistry::find(Language::Sql, "select").is_some());
    assert!(SnippetRegistry::find(Language::Sql, "table").is_some());
    assert!(SnippetRegistry::find(Language::Julia, "fn").is_some());
    assert!(SnippetRegistry::find(Language::Julia, "struct").is_some());
    assert!(SnippetRegistry::find(Language::Julia, "for").is_some());
    assert!(SnippetRegistry::find(Language::Julia, "test").is_some());
}



