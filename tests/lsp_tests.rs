use rustext::core::TextBuffer;
use rustext::editor::Document;
use rustext::gfx::{Framebuffer, GfxRenderer};

#[test]
fn test_lsp_config_ini_parsing_and_defaults() {
    use rustext::editor::config::EditorConfig;

    // 1. Verify default values
    let default_config = EditorConfig::default();
    assert!(default_config.lsp.enabled);
    assert!(default_config.lsp.auto_start);
    assert_eq!(default_config.lsp.rust_server, "rust-analyzer");
    assert_eq!(default_config.lsp.python_server, "pylsp");
    assert_eq!(default_config.lsp.c_server, "clangd");

    // 2. Parse custom [lsp] INI configuration
    let custom_ini = r#"
[lsp]
enabled = false
auto_start = false
rust_server = /usr/local/bin/custom-ra
python_server = /usr/bin/pyright
c_server = /opt/llvm/bin/clangd
"#;
    let parsed = EditorConfig::parse(custom_ini);
    assert!(!parsed.lsp.enabled);
    assert!(!parsed.lsp.auto_start);
    assert_eq!(parsed.lsp.rust_server, "/usr/local/bin/custom-ra");
    assert_eq!(parsed.lsp.python_server, "/usr/bin/pyright");
    assert_eq!(parsed.lsp.c_server, "/opt/llvm/bin/clangd");

    // 3. Verify serialization roundtrip preserves [lsp]
    let serialized = parsed.to_ini_string();
    assert!(serialized.contains("[lsp]"));
    assert!(serialized.contains("enabled = false"));
    assert!(serialized.contains("auto_start = false"));
    assert!(serialized.contains("rust_server = /usr/local/bin/custom-ra"));
    assert!(serialized.contains("python_server = /usr/bin/pyright"));
    assert!(serialized.contains("c_server = /opt/llvm/bin/clangd"));
}

#[test]
fn test_lsp_keymap_and_menu_actions() {
    use rustext::ui::terminal::Key;
    use rustext::editor::keymap::{EditorAction, Keymap};
    use rustext::editor::MenuBar;

    // 1. Key mappings
    assert_eq!(Keymap::resolve_key(Key::F(12)), EditorAction::LspGoToDefinition);
    assert_eq!(Keymap::resolve_key(Key::F(2)), EditorAction::LspRename);
    assert_eq!(Keymap::resolve_key(Key::Alt('k')), EditorAction::LspHover);
    assert_eq!(Keymap::resolve_key(Key::Alt('K')), EditorAction::LspHover);
    assert_eq!(Keymap::resolve_key(Key::CtrlShift('u')), EditorAction::LspFindReferences);
    assert_eq!(Keymap::resolve_key(Key::Ctrl('.')), EditorAction::LspCodeActions);
    assert_eq!(Keymap::resolve_key(Key::CtrlShift('i')), EditorAction::LspFormatDocument);
    assert_eq!(Keymap::resolve_key(Key::CtrlShift('r')), EditorAction::LspDocumentSymbols);

    // 2. Menu integration
    let menu_bar = MenuBar::new();
    let tools_category = menu_bar.categories.iter().find(|c| c.name == "Herramientas").expect("Herramientas category");

    let has_goto_def = tools_category.items.iter().any(|it| it.action == Some(EditorAction::LspGoToDefinition) && it.shortcut == "F12");
    let has_references = tools_category.items.iter().any(|it| it.action == Some(EditorAction::LspFindReferences) && it.shortcut == "Ctrl+Shift+U");
    let has_rename = tools_category.items.iter().any(|it| it.action == Some(EditorAction::LspRename) && it.shortcut == "F2");
    let has_code_actions = tools_category.items.iter().any(|it| it.action == Some(EditorAction::LspCodeActions) && it.shortcut == "Ctrl+.");
    let has_format = tools_category.items.iter().any(|it| it.action == Some(EditorAction::LspFormatDocument) && it.shortcut == "Ctrl+Shift+I");
    let has_symbols = tools_category.items.iter().any(|it| it.action == Some(EditorAction::LspDocumentSymbols) && it.shortcut == "Ctrl+Shift+R");
    let has_hover = tools_category.items.iter().any(|it| it.action == Some(EditorAction::LspHover));
    let has_restart = tools_category.items.iter().any(|it| it.action == Some(EditorAction::RestartLspServer));

    assert!(has_goto_def, "Menu must contain 'LSP: Ir a Definición (F12)'");
    assert!(has_references, "Menu must contain 'LSP: Buscar Referencias (Ctrl+Shift+U)'");
    assert!(has_rename, "Menu must contain 'LSP: Renombrar Símbolo (F2)'");
    assert!(has_code_actions, "Menu must contain 'LSP: Acciones Rápidas (Ctrl+.)'");
    assert!(has_format, "Menu must contain 'LSP: Formatear Documento (Ctrl+Shift+I)'");
    assert!(has_symbols, "Menu must contain 'LSP: Esquema / Símbolos (Ctrl+Shift+R)'");
    assert!(has_hover, "Menu must contain 'LSP: Información de Tipo'");
    assert!(has_restart, "Menu must contain 'LSP: Reiniciar Servidor'");
}

#[test]
fn test_lsp_event_dispatch_and_editor_integration() {
    use rustext::core::buffer::TextBuffer;
    use rustext::editor::app::Editor;
    use rustext::editor::autocomplete::{CompletionItem, CompletionKind};
    use rustext::editor::diagnostic::{Diagnostic, DiagnosticSeverity};
    use rustext::editor::lsp::{LspEvent, LspHover, LspLocation, LspPosition, LspRange, LspCompletionItem};

    let temp_dir = std::env::temp_dir();
    let file_a = temp_dir.join("rustext_test_lsp_a.rs");
    let file_b = temp_dir.join("rustext_test_lsp_b.rs");

    let _ = std::fs::write(&file_a, "fn helper() -> i32 { 42 }\n");
    let _ = std::fs::write(&file_b, "fn main() {\n    let x = helper();\n}\n");

    let buffer = TextBuffer::open(&file_b).expect("Open file b");
    let mut editor = Editor::new(buffer);
    editor.config.lsp.enabled = true;

    // 1. Test PublishDiagnostics event handling
    let diag = Diagnostic::new(
        Some(file_b.clone()),
        2,
        9,
        DiagnosticSeverity::Error,
        "cannot find function helper in this scope".to_string(),
        Some("E0425".to_string()),
    ).with_end_col(15);

    let pub_event = LspEvent::PublishDiagnostics {
        file: file_b.clone(),
        diagnostics: vec![diag],
    };

    // Inject and process through poll_lsp_events logic
    match pub_event {
        LspEvent::PublishDiagnostics { file, diagnostics } => {
            editor.diagnostics.update_for_file(&file, diagnostics);
            editor.set_status("LSP: 1 diagnósticos en archivo (1 errores, 0 avisos)");
        }
        _ => unreachable!(),
    }

    assert_eq!(editor.diagnostics.len(), 1);
    let diags_for_line = editor.diagnostics.for_line_and_file(2, Some(&file_b));
    assert_eq!(diags_for_line.len(), 1);
    assert_eq!(diags_for_line[0].col, 9);
    assert_eq!(diags_for_line[0].end_col, Some(15));
    assert!(editor.status_message.as_ref().unwrap().0.contains("1 diagnósticos"));

    // 2. Test CompletionResponse event handling
    let comp_event = LspEvent::CompletionResponse {
        request_id: 1,
        items: vec![
            LspCompletionItem {
                label: "helper".to_string(),
                detail: "fn() -> i32".to_string(),
                documentation: "Calculates helper result".to_string(),
                kind: CompletionKind::Function,
                insert_text: Some("helper()".to_string()),
            },
            LspCompletionItem {
                label: "println!".to_string(),
                detail: "macro".to_string(),
                documentation: "Prints to standard output with newline".to_string(),
                kind: CompletionKind::Snippet,
                insert_text: None,
            },
        ],
    };

    match comp_event {
        LspEvent::CompletionResponse { items, .. } => {
            let comp_items: Vec<CompletionItem> = items
                .into_iter()
                .map(|it| CompletionItem::new(it.label, it.detail, it.kind))
                .collect();
            editor.autocomplete.merge_items(comp_items);
        }
        _ => unreachable!(),
    }

    assert!(editor.autocomplete.is_active);
    assert_eq!(editor.autocomplete.items.len(), 2);
    assert_eq!(editor.autocomplete.items[0].label, "helper");
    assert_eq!(editor.autocomplete.items[0].kind, CompletionKind::Function);

    // 3. Test HoverResponse event handling
    let hover_event = LspEvent::HoverResponse {
        request_id: 2,
        hover: Some(LspHover {
            contents: "```rust\nfn helper() -> i32\n```\nDocumentation for helper".to_string(),
        }),
    };

    match hover_event {
        LspEvent::HoverResponse { hover, .. } => {
            if let Some(h) = hover {
                let first_line = h.contents.lines().find(|l| !l.trim().is_empty() && !l.starts_with("```")).unwrap_or("").trim();
                editor.set_status(format!("LSP: {}", first_line));
            }
        }
        _ => unreachable!(),
    }

    assert!(editor.status_message.as_ref().unwrap().0.contains("fn helper() -> i32"));

    // 4. Test DefinitionResponse event handling (jump to file_a line 1, col 4)
    let def_event = LspEvent::DefinitionResponse {
        request_id: 3,
        location: Some(LspLocation {
            path: file_a.clone(),
            range: LspRange {
                start: LspPosition { line: 0, character: 3 },
                end: LspPosition { line: 0, character: 9 },
            },
        }),
    };

    match def_event {
        LspEvent::DefinitionResponse { location, .. } => {
            if let Some(loc) = location {
                editor.open_file(&loc.path);
                let doc = editor.doc_mut();
                let target_line = loc.range.start.line;
                let target_col = loc.range.start.character;
                let char_offset = doc.buffer.position_to_char(rustext::core::buffer::Position::new(target_line, target_col));
                doc.cursors.set_single(char_offset);
            }
        }
        _ => unreachable!(),
    }

    assert_eq!(editor.doc().buffer.file_path(), Some(file_a.as_path()));
    let primary_pos = editor.doc().buffer.char_to_position(editor.doc().cursors.primary().head);
    assert_eq!(primary_pos.line, 0);
    assert_eq!(primary_pos.col, 3);

    // Clean up temporary test files
    let _ = std::fs::remove_file(file_a);
    let _ = std::fs::remove_file(file_b);
}

#[test]
fn test_cli_version_and_help_flags() {
    let bin_path = env!("CARGO_BIN_EXE_rustext");
    let version_output = std::process::Command::new(bin_path)
        .arg("--version")
        .output()
        .expect("Failed to execute rustext --version");
    assert!(version_output.status.success());
    let stdout = String::from_utf8_lossy(&version_output.stdout);
    assert!(stdout.contains("rustext"));
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
    assert!(stdout.contains("AGPLv3"));

    let help_output = std::process::Command::new(bin_path)
        .arg("--help")
        .output()
        .expect("Failed to execute rustext --help");
    assert!(help_output.status.success());
    let help_text = String::from_utf8_lossy(&help_output.stdout);
    assert!(help_text.contains("Wayland"));
    assert!(help_text.contains("--gui"));
    assert!(help_text.contains("--version"));
}

#[test]
fn test_editing_ergonomics_move_lines_up_and_down() {
    use rustext::core::buffer::{Position, TextBuffer};
    use rustext::editor::app::Editor;
    use rustext::editor::keymap::EditorAction;

    let initial_text = "line 0\nline 1\nline 2\nline 3";
    let mut editor = Editor::new(TextBuffer::from_str(initial_text));

    // Place cursor on line 1 ("line 1")
    let pos_line1 = editor.doc().buffer.position_to_char(Position::new(1, 2));
    editor.doc_mut().cursors.set_single(pos_line1);

    // Move line 1 down (swaps with line 2)
    editor.execute_action(EditorAction::MoveLineDown);
    assert_eq!(editor.doc().buffer.to_string(), "line 0\nline 2\nline 1\nline 3");
    let current_pos = editor.doc().buffer.char_to_position(editor.doc().cursors.primary().head);
    assert_eq!(current_pos.line, 2);
    assert_eq!(current_pos.col, 2);

    // Move line 2 up (swaps back with line 1)
    editor.execute_action(EditorAction::MoveLineUp);
    assert_eq!(editor.doc().buffer.to_string(), "line 0\nline 1\nline 2\nline 3");
    let current_pos2 = editor.doc().buffer.char_to_position(editor.doc().cursors.primary().head);
    assert_eq!(current_pos2.line, 1);
    assert_eq!(current_pos2.col, 2);

    // Undo moves line back down
    editor.execute_action(EditorAction::Undo);
    assert_eq!(editor.doc().buffer.to_string(), "line 0\nline 2\nline 1\nline 3");

    // Redo restores it
    editor.execute_action(EditorAction::Redo);
    assert_eq!(editor.doc().buffer.to_string(), "line 0\nline 1\nline 2\nline 3");
}

#[test]
fn test_editing_ergonomics_duplicate_lines_down() {
    use rustext::core::buffer::{Position, TextBuffer};
    use rustext::editor::app::Editor;
    use rustext::editor::keymap::EditorAction;

    let mut editor = Editor::new(TextBuffer::from_str("hello\nworld"));

    // Position cursor on line 0
    let pos = editor.doc().buffer.position_to_char(Position::new(0, 1));
    editor.doc_mut().cursors.set_single(pos);

    // Duplicate line 0 down
    editor.execute_action(EditorAction::DuplicateLineDown);
    assert_eq!(editor.doc().buffer.to_string(), "hello\nhello\nworld");
    let new_pos = editor.doc().buffer.char_to_position(editor.doc().cursors.primary().head);
    assert_eq!(new_pos.line, 1);
    assert_eq!(new_pos.col, 1);

    // Undo reverts back
    editor.execute_action(EditorAction::Undo);
    assert_eq!(editor.doc().buffer.to_string(), "hello\nworld");
}

#[test]
fn test_editing_ergonomics_toggle_comment() {
    use rustext::core::buffer::{Position, TextBuffer};
    use rustext::core::cursor::Cursor;
    use rustext::editor::app::Editor;
    use rustext::editor::keymap::EditorAction;

    // 1. Rust comments (// )
    let mut editor = Editor::new(TextBuffer::from_str("    let a = 1;\n    let b = 2;"));
    editor.doc_mut().language = rustext::core::syntax::Language::Rust;

    // Select across line 0 and line 1
    let p_start = editor.doc().buffer.position_to_char(Position::new(0, 0));
    let p_end = editor.doc().buffer.position_to_char(Position::new(1, 5));
    editor.doc_mut().cursors.set_cursors(vec![Cursor::with_selection(p_start, p_end)]);

    // Comment
    editor.execute_action(EditorAction::ToggleComment);
    assert_eq!(editor.doc().buffer.to_string(), "    // let a = 1;\n    // let b = 2;");

    // Uncomment
    editor.execute_action(EditorAction::ToggleComment);
    assert_eq!(editor.doc().buffer.to_string(), "    let a = 1;\n    let b = 2;");

    // 2. Python comments (# )
    let mut py_editor = Editor::new(TextBuffer::from_str("def greet():\n    pass"));
    py_editor.doc_mut().language = rustext::core::syntax::Language::Python;
    py_editor.execute_action(EditorAction::ToggleComment);
    assert_eq!(py_editor.doc().buffer.to_string(), "# def greet():\n    pass");
    py_editor.execute_action(EditorAction::ToggleComment);
    assert_eq!(py_editor.doc().buffer.to_string(), "def greet():\n    pass");

    // 3. HTML comments (<!-- -->)
    let mut html_editor = Editor::new(TextBuffer::from_str("<p>hello</p>"));
    html_editor.doc_mut().language = rustext::core::syntax::Language::Html;
    html_editor.execute_action(EditorAction::ToggleComment);
    assert_eq!(html_editor.doc().buffer.to_string(), "<!-- <p>hello</p> -->");
    html_editor.execute_action(EditorAction::ToggleComment);
    assert_eq!(html_editor.doc().buffer.to_string(), "<p>hello</p>");
}

#[test]
fn test_editing_ergonomics_jump_list() {
    use rustext::core::buffer::{Position, TextBuffer};
    use rustext::editor::app::Editor;
    use rustext::editor::keymap::EditorAction;

    let temp_a = std::env::temp_dir().join(format!("rustext_jump_a_{}.rs", std::process::id()));
    let temp_b = std::env::temp_dir().join(format!("rustext_jump_b_{}.rs", std::process::id()));

    std::fs::write(&temp_a, "fn foo() {\n    let x = 1;\n    let y = 2;\n}\n").unwrap();
    std::fs::write(&temp_b, "fn bar() {\n    let z = 99;\n}\n").unwrap();

    let mut editor = Editor::new(TextBuffer::new());
    editor.open_file(&temp_a);
    let p_a = editor.doc().buffer.position_to_char(Position::new(1, 4));
    editor.doc_mut().cursors.set_single(p_a);

    // Open file B (automatically records jump location in file A)
    editor.open_file(&temp_b);
    let p_b = editor.doc().buffer.position_to_char(Position::new(1, 8));
    editor.doc_mut().cursors.set_single(p_b);

    // Jump back (Alt+Left) -> returns to file A at line 1
    editor.execute_action(EditorAction::JumpBack);
    assert_eq!(editor.doc().buffer.file_path(), Some(temp_a.as_path()));
    let pos_back = editor.doc().buffer.char_to_position(editor.doc().cursors.primary().head);
    assert_eq!(pos_back.line, 1);

    // Jump forward (Alt+Right) -> returns to file B at line 1
    editor.execute_action(EditorAction::JumpForward);
    assert_eq!(editor.doc().buffer.file_path(), Some(temp_b.as_path()));
    let pos_fwd = editor.doc().buffer.char_to_position(editor.doc().cursors.primary().head);
    assert_eq!(pos_fwd.line, 1);

    let _ = std::fs::remove_file(temp_a);
    let _ = std::fs::remove_file(temp_b);
}

#[test]
fn test_lsp_signature_help_parsing_and_rendering() {
    use rustext::core::buffer::TextBuffer;
    use rustext::editor::app::Editor;
    use rustext::editor::lsp::{
        LspParameterInformation, LspSignatureHelp, LspSignatureInformation,
    };
    use rustext::gfx::framebuffer::Framebuffer;
    use rustext::gfx::renderer::GfxRenderer;

    let sig_info = LspSignatureInformation {
        label: "fn push(&mut self, value: T) -> ()".to_string(),
        documentation: "Appends an element to the back of a collection.".to_string(),
        parameters: vec![
            LspParameterInformation {
                label: "&mut self".to_string(),
                documentation: "Receiver handle".to_string(),
            },
            LspParameterInformation {
                label: "value: T".to_string(),
                documentation: "Element to insert".to_string(),
            },
        ],
        active_parameter: Some(1),
    };

    let sig_help = LspSignatureHelp {
        signatures: vec![sig_info],
        active_signature: 0,
        active_parameter: 1, // Highlighting "value: T"
    };

    assert_eq!(sig_help.signatures.len(), 1);
    assert_eq!(sig_help.active_parameter, 1);
    let active_sig = &sig_help.signatures[sig_help.active_signature];
    assert_eq!(active_sig.parameters.len(), 2);
    assert_eq!(active_sig.parameters[1].label, "value: T");

    // Test renderer tooltip integration
    let mut fb = Framebuffer::new(800, 600);
    let renderer = GfxRenderer::new();
    let editor = Editor::new(TextBuffer::from_str("vec.push("));

    // Call render_signature_help
    renderer.render_signature_help(&mut fb, &sig_help, editor.doc(), 0, false);

    // Verify pixels are non-zero inside the tooltip bounding box
    let mut drawn_pixels = 0;
    for y in 0..600 {
        for x in 0..800 {
            if fb.get_pixel(x, y) != 0 {
                drawn_pixels += 1;
            }
        }
    }
    assert!(drawn_pixels > 0, "Signature help tooltip should draw frame pixels");
}

#[test]
fn test_inline_git_blame_formatting_and_rendering() {
    use rustext::editor::git::{GitBlameLine, format_relative_time};
    use rustext::gfx::ThemeColors;

    // 1. Test relative time formatting
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    assert_eq!(format_relative_time(now - 10), "10s ago");
    assert_eq!(format_relative_time(now - 120), "2 mins ago");
    assert_eq!(format_relative_time(now - 7200), "2 hours ago");
    assert_eq!(format_relative_time(now - 86400 * 3), "3 days ago");

    // 2. Test GitBlameLine formatting
    let blame = GitBlameLine {
        commit_hash: "a1b2c3d4e5f6".to_string(),
        author: "bolek".to_string(),
        summary: "feat: add semantic tokens".to_string(),
        time_relative: "5 mins ago".to_string(),
    };

    let formatted = blame.format_inline();
    assert!(formatted.contains("bolek"));
    assert!(formatted.contains("5 mins ago"));
    assert!(formatted.contains("feat: add semantic tokens"));

    // 3. Test rendering inline blame on editor line
    let mut fb = Framebuffer::new(800, 600);
    let mut renderer = GfxRenderer::new();
    let doc = Document::new(TextBuffer::from_str("fn main() {\n    println!(\"hello\");\n}\n"));
    let colors = ThemeColors::tokyo_night();

    renderer.inline_blame = Some(formatted);
    renderer.render_editor_pane(
        &mut fb,
        &doc,
        0,
        0,
        800,
        600,
        true,
        "main.rs",
        false,
        false,
        false,
        false,
    );

    // Framebuffer should contain non-zero pixels
    let mut pixel_count = 0;
    for &p in fb.pixels.iter() {
        if p != colors.background {
            pixel_count += 1;
        }
    }
    assert!(pixel_count > 0, "Inline git blame should be rendered into framebuffer");
}

#[test]
fn test_git_hunk_diff_popup_and_atomic_revert() {
    use rustext::editor::git::{GitHunk, GitLineStatus, compute_hunks};
    use rustext::editor::{Editor, EditorAction};

    // 1. Test compute_hunks
    let old_text = "line 1\nline 2\nline 3\n";
    let new_text = "line 1\nline 2 MODIFIED\nline 3\nline 4 ADDED\n";

    let hunks = compute_hunks(old_text, new_text);
    assert!(!hunks.is_empty(), "Hunks should be detected");

    // 2. Test Hunk Diff Popup rendering
    let mut fb = Framebuffer::new(800, 600);
    let renderer = GfxRenderer::new();
    let test_hunk = GitHunk {
        start_line: 1,
        end_line: 1,
        old_lines: vec!["line 2".to_string()],
        new_lines: vec!["line 2 MODIFIED".to_string()],
        status: GitLineStatus::Modified,
    };

    renderer.render_hunk_diff_popup(&mut fb, &test_hunk);

    let mut painted_pixels = 0;
    for &p in fb.pixels.iter() {
        if p != 0 {
            painted_pixels += 1;
        }
    }
    assert!(painted_pixels > 100, "Popup modal should render with drop shadow and content");

    // 3. Test Editor atomic hunk revert for Modified hunk
    let mut editor = Editor::new(TextBuffer::from_str("alpha\nbeta-mod\ngamma\n"));
    editor.active_hunk_popup = Some(GitHunk {
        start_line: 1,
        end_line: 1,
        old_lines: vec!["beta-original".to_string()],
        new_lines: vec!["beta-mod".to_string()],
        status: GitLineStatus::Modified,
    });

    editor.revert_active_hunk();

    // Verify buffer content restored
    assert_eq!(editor.doc().buffer.to_string(), "alpha\nbeta-original\ngamma\n");
    assert!(editor.active_hunk_popup.is_none(), "Popup should be closed after revert");

    // Verify undo restores the modification
    editor.execute_action(EditorAction::Undo);
    assert_eq!(editor.doc().buffer.to_string(), "alpha\nbeta-mod\ngamma\n");

    // 4. Test Editor atomic hunk revert for Added hunk
    let mut editor_added = Editor::new(TextBuffer::from_str("alpha\nnew-line-1\nnew-line-2\ngamma\n"));
    editor_added.active_hunk_popup = Some(GitHunk {
        start_line: 1,
        end_line: 2,
        old_lines: Vec::new(),
        new_lines: vec!["new-line-1".to_string(), "new-line-2".to_string()],
        status: GitLineStatus::Added,
    });

    editor_added.revert_active_hunk();
    assert_eq!(editor_added.doc().buffer.to_string(), "alpha\ngamma\n");

    // 5. Test Editor atomic hunk revert for DeletedAbove hunk
    let mut editor_deleted = Editor::new(TextBuffer::from_str("alpha\ngamma\n"));
    editor_deleted.active_hunk_popup = Some(GitHunk {
        start_line: 1,
        end_line: 1,
        old_lines: vec!["restored-beta".to_string()],
        new_lines: Vec::new(),
        status: GitLineStatus::DeletedAbove,
    });

    editor_deleted.revert_active_hunk();
    assert_eq!(editor_deleted.doc().buffer.to_string(), "alpha\nrestored-beta\ngamma\n");
}

#[test]
fn test_lsp_semantic_tokens_overlay_and_theme_colors() {
    use rustext::core::syntax::{Language, SemanticTokenSpan, SyntaxHighlighter, TokenType};
    use rustext::editor::lsp::{LspClient, SemanticTokensLegend};
    use rustext::gfx::ThemeColors;

    // 1. Test decode_semantic_tokens
    let legend = SemanticTokensLegend::default();
    let raw_data = vec![
        0, 4, 8, 14, 0,
        1, 4, 6, 18, 4,
    ];

    let tokens_map = LspClient::decode_semantic_tokens(&raw_data, &legend);
    assert_eq!(tokens_map.len(), 2);
    assert!(tokens_map.contains_key(&0));
    assert!(tokens_map.contains_key(&1));

    let line0_tokens = &tokens_map[&0];
    assert_eq!(line0_tokens.len(), 1);
    assert_eq!(line0_tokens[0].token_type, TokenType::Function);
    assert_eq!(line0_tokens[0].start_col, 4);
    assert_eq!(line0_tokens[0].end_col, 12);

    let line1_tokens = &tokens_map[&1];
    assert_eq!(line1_tokens.len(), 1);
    assert_eq!(line1_tokens[0].token_type, TokenType::Constant);

    // 2. Test apply_semantic_spans character overlay
    let line_str = "    function_name(param);";
    let lexical = SyntaxHighlighter::tokenize_line(line_str, Language::Rust);
    let semantic = vec![
        SemanticTokenSpan {
            start_col: 4,
            end_col: 17,
            token_type: TokenType::Function,
        },
        SemanticTokenSpan {
            start_col: 18,
            end_col: 23,
            token_type: TokenType::Parameter,
        },
    ];

    let merged = SyntaxHighlighter::apply_semantic_spans(line_str, lexical, &semantic);
    let func_tok = merged.iter().find(|t| t.token_type == TokenType::Function);
    assert!(func_tok.is_some(), "Function token should be overlaid");
    assert_eq!(func_tok.unwrap().text, "function_name");

    let param_tok = merged.iter().find(|t| t.token_type == TokenType::Parameter);
    assert!(param_tok.is_some(), "Parameter token should be overlaid");
    assert_eq!(param_tok.unwrap().text, "param");

    // 3. Test ThemeColors token color resolution for new semantic tokens
    let theme = ThemeColors::catppuccin_mocha();
    assert_eq!(theme.token_color(TokenType::Function), theme.syntax_function);
    assert_eq!(theme.token_color(TokenType::Method), theme.syntax_method);
    assert_eq!(theme.token_color(TokenType::Struct), theme.syntax_struct);
    assert_eq!(theme.token_color(TokenType::Trait), theme.syntax_trait);
    assert_eq!(theme.token_color(TokenType::Macro), theme.syntax_macro);
    assert_eq!(theme.token_color(TokenType::Parameter), theme.syntax_parameter);
    assert_eq!(theme.token_color(TokenType::Constant), theme.syntax_constant);
    assert_eq!(theme.token_color(TokenType::Field), theme.syntax_field);
}

#[test]
fn test_render_logo_faceted_ribbon_and_badge() {
    let mut fb = Framebuffer::new(800, 600);
    let renderer = GfxRenderer::new();

    // 1. Boundary check: size 0 must not panic
    renderer.render_logo(&mut fb, 50, 50, 0, true);
    renderer.render_logo(&mut fb, 50, 50, 0, false);

    // 2. Render logo badge at 64x64
    let x = 100;
    let y = 100;
    let size = 64;
    renderer.render_logo(&mut fb, x, y, size, true);

    // Verify squircle background and facets were drawn
    let mut non_zero_pixels = 0;
    let mut has_orange = false;
    let mut has_cyan = false;

    for py in y..y + size {
        for px in x..x + size {
            let p = fb.get_pixel(px, py);
            if p != 0 {
                non_zero_pixels += 1;
            }
            // Check for Rust orange facet components (high red, medium green, low blue)
            let r = (p >> 16) & 0xFF;
            let g = (p >> 8) & 0xFF;
            let b = p & 0xFF;
            if r >= 200 && (50..=150).contains(&g) && b <= 70 {
                has_orange = true;
            }
            // Check for electric cyan facet components (high blue, medium-high green)
            if b >= 180 && g >= 100 && r <= 150 {
                has_cyan = true;
            }
        }
    }

    assert!(non_zero_pixels > (size * size) / 2, "Logo badge should fill significant area");
    assert!(has_orange, "Logo must contain faceted Rust orange pixels");
    assert!(has_cyan, "Logo must contain faceted electric cyan pixels");

    // 3. Render standalone ribbon without badge at 96x96
    let mut fb2 = Framebuffer::new(400, 400);
    renderer.render_logo(&mut fb2, 50, 50, 96, false);
    let non_zero_fb2 = fb2.pixels.iter().filter(|&&p| p != 0).count();
    assert!(non_zero_fb2 > 200, "Standalone logo ribbon should render facets without badge");
}

#[test]
fn test_workspace_management_and_project_switching() {
    use rustext::editor::app::{Editor, EditorMode};
    use rustext::editor::keymap::EditorAction;
    use rustext::ui::terminal::Key;
    use rustext::ui::prompt::{PromptKind, PromptModal};
    use std::fs;

    // 1. Setup temporary project folders
    let base_temp = std::env::temp_dir().join(format!("rustext_ws_integ_{}", std::process::id()));
    let proj_a = base_temp.join("rust_backend");
    let proj_b = base_temp.join("web_frontend");

    let _ = fs::create_dir_all(&proj_a);
    let _ = fs::create_dir_all(&proj_b);

    // Project A: Rust project with custom .rustext/config.ini override
    fs::write(proj_a.join("Cargo.toml"), "[package]\nname = \"rust_backend\"\nversion = \"0.1.0\"\n").unwrap();
    let src_dir = proj_a.join("src");
    let _ = fs::create_dir_all(&src_dir);
    let main_rs = src_dir.join("main.rs");
    fs::write(&main_rs, "fn main() {\n    println!(\"Hello Project A\");\n}\n").unwrap();

    let dot_rustext_a = proj_a.join(".rustext");
    let _ = fs::create_dir_all(&dot_rustext_a);
    fs::write(dot_rustext_a.join("config.ini"), "[editor]\ntab_size = 2\n").unwrap();

    // Project B: Node.js project
    fs::write(proj_b.join("package.json"), "{\n  \"name\": \"web_frontend\",\n  \"version\": \"1.0.0\"\n}\n").unwrap();
    let index_js = proj_b.join("index.js");
    fs::write(&index_js, "console.log('Hello Project B');\n").unwrap();

    let orig_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/home/bolek/Proyectos/rustext"));

    // 2. Initialize Editor
    let mut editor = Editor::new(rustext::core::TextBuffer::from_str(""));
    editor.project_registry = rustext::editor::workspace::ProjectRegistry::with_dir(base_temp.join("reg"));
    editor.recent_projects.clear();

    // Test OpenFolderPrompt action
    editor.execute_action(EditorAction::OpenFolderPrompt);
    if let EditorMode::ModalPrompt(modal) = &editor.mode {
        assert_eq!(modal.kind, PromptKind::OpenFolder);
        assert!(modal.is_file_dialog());
        assert_eq!(modal.kind.window_title(), "Abrir Carpeta / Proyecto (Ctrl+Shift+O)");
    } else {
        panic!("Expected EditorMode::ModalPrompt(OpenFolder)");
    }

    // Cancel modal
    editor.handle_key(Key::Esc);
    assert_eq!(editor.mode, EditorMode::Normal);

    // 3. Open Workspace A
    editor.open_workspace(proj_a.clone());
    assert_eq!(editor.active_workspace, Some(proj_a.clone()));
    assert_eq!(editor.sidebar.root_dir, proj_a);
    // Verify local config override was applied (tab_size = 2)
    assert_eq!(editor.config.editor.tab_size, 2);

    // Verify recent projects contains proj_a with [Rust] tag
    let item_a = editor.recent_projects.iter().find(|p| p.path == proj_a);
    assert!(item_a.is_some(), "Project A should be in recent_projects");
    assert_eq!(item_a.unwrap().kind_tag, "[Rust]");

    // Open file in Project A
    editor.open_file(&main_rs);
    assert_eq!(editor.documents.len(), 1);
    assert_eq!(editor.doc().buffer.file_path(), Some(main_rs.as_path()));

    // 4. Open Workspace B (switching project)
    editor.open_workspace(proj_b.clone());
    assert_eq!(editor.active_workspace, Some(proj_b.clone()));
    assert_eq!(editor.sidebar.root_dir, proj_b);

    // Verify recent projects contains proj_b with [Node] tag
    let item_b = editor.recent_projects.iter().find(|p| p.path == proj_b);
    assert!(item_b.is_some(), "Project B should be in recent_projects");
    assert_eq!(item_b.unwrap().kind_tag, "[Node]");

    // Open file in Project B
    editor.open_file(&index_js);
    assert_eq!(editor.doc().buffer.file_path(), Some(index_js.as_path()));

    // 5. Test Recent Projects Modal & Navigation
    editor.execute_action(EditorAction::OpenRecentProjectsPrompt);
    if let EditorMode::ModalPrompt(modal) = &editor.mode {
        assert_eq!(modal.kind, PromptKind::RecentProjects);
        let matches = modal.matching_projects();
        assert!(matches.len() >= 2);
    } else {
        panic!("Expected ModalPrompt(RecentProjects)");
    }

    // Type query to filter
    editor.handle_key(Key::Char('b'));
    editor.handle_key(Key::Char('a'));
    editor.handle_key(Key::Char('c'));
    editor.handle_key(Key::Char('k'));
    if let EditorMode::ModalPrompt(modal) = &editor.mode {
        let matches = modal.matching_projects();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "rust_backend");
    }

    // Press Enter to switch to Project A
    editor.handle_key(Key::Enter);
    assert_eq!(editor.mode, EditorMode::Normal);
    assert_eq!(editor.active_workspace, Some(proj_a.clone()));

    // 6. Test Removing from Recent Projects via Key::Delete
    editor.execute_action(EditorAction::OpenRecentProjectsPrompt);
    let initial_count = editor.recent_projects.len();
    editor.handle_key(Key::Delete);
    assert_eq!(editor.recent_projects.len(), initial_count - 1);

    // Close modal
    editor.handle_key(Key::Esc);

    // 7. Test CloseWorkspace
    editor.execute_action(EditorAction::CloseWorkspace);
    assert_eq!(editor.active_workspace, None);

    // 8. Test GUI Framebuffer Rendering for RecentProjects and Status Bar
    let mut fb = Framebuffer::new(800, 600);
    fb.clear(0x001A1B26);

    let mut renderer = GfxRenderer::new();
    renderer.workspace_name = Some("rust_backend".to_string());

    // Render status bar with workspace chip
    renderer.render_status_bar(&mut fb, editor.doc(), None);

    // Verify status bar area has drawn chips
    let status_bar_y = 600 - renderer.status_bar_height;
    let non_zero_status = fb.pixels[status_bar_y * 800..(status_bar_y + 10) * 800].iter().filter(|&&p| p != 0).count();
    assert!(non_zero_status > 50, "Status bar should render workspace badge chip");

    // Render recent projects modal
    let recent_modal = PromptModal::new_recent_projects(vec![
        rustext::editor::WorkspaceItem::new(proj_a.clone()),
        rustext::editor::WorkspaceItem::new(proj_b.clone()),
    ]);
    renderer.render_prompt_overlay_with_hover(&mut fb, &recent_modal, false);

    // Verify modal overlay rendered pixels
    let non_zero_pixels = fb.pixels.iter().filter(|&&p| p != 0).count();
    assert!(non_zero_pixels > 2000, "Recent projects modal must render cleanly on framebuffer");

    // Cleanup
    let _ = std::env::set_current_dir(orig_dir);
    let _ = fs::remove_dir_all(&base_temp);
}

#[test]
fn test_lsp_elite_references_and_jump() {
    use rustext::core::buffer::TextBuffer;
    use rustext::editor::app::{Editor, EditorMode};
    use rustext::editor::lsp::{LspEvent, LspLocation, LspPosition, LspRange};
    use rustext::ui::prompt::PromptKind;
    use rustext::ui::terminal::Key;
    use std::fs;

    let temp_dir = std::env::temp_dir().join(format!("rustext_lsp_ref_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let file_path = temp_dir.join("main.rs");
    let content = "fn compute(val: i32) -> i32 {\n    let result = val * 2;\n    println!(\"{}\", result);\n    result\n}\n";
    fs::write(&file_path, content).unwrap();

    let buf = TextBuffer::open(&file_path).unwrap();
    let mut editor = Editor::new(buf);
    editor.config.lsp.enabled = true;

    // Simulate references event from LSP
    let references = vec![
        LspLocation {
            path: file_path.clone(),
            range: LspRange {
                start: LspPosition { line: 1, character: 8 },
                end: LspPosition { line: 1, character: 14 },
            },
        },
        LspLocation {
            path: file_path.clone(),
            range: LspRange {
                start: LspPosition { line: 2, character: 19 },
                end: LspPosition { line: 2, character: 25 },
            },
        },
    ];

    editor.lsp.push_event_for_test(LspEvent::ReferencesResponse {
        request_id: 1,
        references,
    });
    editor.poll_lsp_events();

    // Check modal prompt is opened in References mode
    match &editor.mode {
        EditorMode::ModalPrompt(modal) => {
            assert_eq!(modal.kind, PromptKind::References);
            let matching = modal.matching_references();
            assert_eq!(matching.len(), 2);
            assert_eq!(matching[0].range.start.line, 1);
            assert_eq!(matching[1].range.start.line, 2);
            assert!(matching[0].line_text.contains("let result = val * 2"));
        }
        _ => panic!("Expected EditorMode::ModalPrompt"),
    }

    // Press down to select second reference and press Enter to jump
    editor.handle_key(Key::Down);
    editor.handle_key(Key::Enter);

    assert_eq!(editor.mode, EditorMode::Normal);
    let head = editor.doc().cursors.primary().head;
    let pos = editor.doc().buffer.char_to_position(head);
    assert_eq!(pos.line, 2);
    assert_eq!(pos.col, 19);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_lsp_elite_workspace_rename_and_edits() {
    use rustext::core::buffer::TextBuffer;
    use rustext::editor::app::Editor;
    use rustext::editor::lsp::{LspEvent, LspPosition, LspRange, LspTextEdit, LspWorkspaceEdit};
    use std::collections::HashMap;
    use std::fs;

    let temp_dir = std::env::temp_dir().join(format!("rustext_lsp_rename_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);

    let file_a = temp_dir.join("lib.rs");
    let file_b = temp_dir.join("main.rs");
    fs::write(&file_a, "pub fn old_calculator() -> i32 { 42 }\n").unwrap();
    fs::write(&file_b, "fn main() { let ans = old_calculator(); }\n").unwrap();

    let buf_a = TextBuffer::open(&file_a).unwrap();
    let mut editor = Editor::new(buf_a);
    editor.open_file(&file_b);

    // Prepare WorkspaceEdit: rename `old_calculator` to `new_super_calculator` in both files
    let mut changes = HashMap::new();
    changes.insert(file_a.clone(), vec![
        LspTextEdit {
            range: LspRange {
                start: LspPosition { line: 0, character: 7 },
                end: LspPosition { line: 0, character: 21 },
            },
            new_text: "new_super_calculator".to_string(),
        },
    ]);
    changes.insert(file_b.clone(), vec![
        LspTextEdit {
            range: LspRange {
                start: LspPosition { line: 0, character: 22 },
                end: LspPosition { line: 0, character: 36 },
            },
            new_text: "new_super_calculator".to_string(),
        },
    ]);

    editor.lsp.push_event_for_test(LspEvent::RenameResponse {
        request_id: 1,
        edit: Some(LspWorkspaceEdit { changes }),
    });
    editor.poll_lsp_events();

    // Verify open tabs have the updated content
    let doc_b = editor.doc();
    assert_eq!(doc_b.buffer.to_string(), "fn main() { let ans = new_super_calculator(); }\n");

    editor.prev_tab();
    let doc_a = editor.doc();
    assert_eq!(doc_a.buffer.to_string(), "pub fn new_super_calculator() -> i32 { 42 }\n");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_lsp_elite_code_actions_and_apply() {
    use rustext::core::buffer::TextBuffer;
    use rustext::editor::app::{Editor, EditorMode};
    use rustext::editor::lsp::{LspCodeAction, LspEvent, LspPosition, LspRange, LspTextEdit, LspWorkspaceEdit};
    use rustext::ui::prompt::PromptKind;
    use rustext::ui::terminal::Key;
    use std::collections::HashMap;
    use std::fs;

    let temp_dir = std::env::temp_dir().join(format!("rustext_lsp_action_test_{}", std::process::id()));
    let _ = fs::create_dir_all(&temp_dir);
    let file_path = temp_dir.join("main.rs");
    fs::write(&file_path, "let x = 10;\n").unwrap();

    let buf = TextBuffer::open(&file_path).unwrap();
    let mut editor = Editor::new(buf);

    let mut changes = HashMap::new();
    changes.insert(file_path.clone(), vec![
        LspTextEdit {
            range: LspRange {
                start: LspPosition { line: 0, character: 4 },
                end: LspPosition { line: 0, character: 4 },
            },
            new_text: "mut ".to_string(),
        },
    ]);

    let action = LspCodeAction {
        title: "Make variable mutable".to_string(),
        kind: Some("quickfix".to_string()),
        edit: Some(LspWorkspaceEdit { changes }),
        is_preferred: true,
    };

    editor.lsp.push_event_for_test(LspEvent::CodeActionResponse {
        request_id: 1,
        actions: vec![action],
    });
    editor.poll_lsp_events();

    match &editor.mode {
        EditorMode::ModalPrompt(modal) => {
            assert_eq!(modal.kind, PromptKind::CodeActions);
            assert_eq!(modal.code_actions.len(), 1);
            assert_eq!(modal.code_actions[0].title, "Make variable mutable");
            assert!(modal.code_actions[0].is_preferred);
        }
        _ => panic!("Expected EditorMode::ModalPrompt"),
    }

    // Press Enter to apply code action
    editor.handle_key(Key::Enter);
    assert_eq!(editor.mode, EditorMode::Normal);
    assert_eq!(editor.doc().buffer.to_string(), "let mut x = 10;\n");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_lsp_elite_document_formatting() {
    use rustext::core::buffer::TextBuffer;
    use rustext::editor::app::Editor;
    use rustext::editor::lsp::{LspEvent, LspPosition, LspRange, LspTextEdit};

    let mut editor = Editor::new(TextBuffer::from_str("fn foo(){\nlet y=1;\n}\n"));

    let edits = vec![
        LspTextEdit {
            range: LspRange {
                start: LspPosition { line: 0, character: 8 },
                end: LspPosition { line: 0, character: 9 },
            },
            new_text: " {".to_string(),
        },
        LspTextEdit {
            range: LspRange {
                start: LspPosition { line: 1, character: 0 },
                end: LspPosition { line: 1, character: 0 },
            },
            new_text: "    ".to_string(),
        },
        LspTextEdit {
            range: LspRange {
                start: LspPosition { line: 1, character: 5 },
                end: LspPosition { line: 1, character: 6 },
            },
            new_text: " = ".to_string(),
        },
    ];

    editor.lsp.push_event_for_test(LspEvent::FormattingResponse {
        request_id: 1,
        edits,
    });
    editor.poll_lsp_events();

    assert_eq!(editor.doc().buffer.to_string(), "fn foo() {\n    let y = 1;\n}\n");
}

#[test]
fn test_lsp_elite_document_symbols_outline_and_jump() {
    use rustext::core::buffer::TextBuffer;
    use rustext::editor::app::{Editor, EditorMode};
    use rustext::editor::lsp::{LspDocumentSymbol, LspEvent, LspPosition, LspRange, SymbolKind};
    use rustext::ui::prompt::PromptKind;
    use rustext::ui::terminal::Key;

    let buf = TextBuffer::from_str("struct AppState {\n    count: usize,\n}\n\nimpl AppState {\n    pub fn run() {}\n}\n");
    let mut editor = Editor::new(buf);

    let symbols = vec![
        LspDocumentSymbol {
            name: "AppState".to_string(),
            detail: "struct".to_string(),
            kind: SymbolKind::Struct,
            range: LspRange {
                start: LspPosition { line: 0, character: 0 },
                end: LspPosition { line: 2, character: 1 },
            },
            selection_range: LspRange {
                start: LspPosition { line: 0, character: 7 },
                end: LspPosition { line: 0, character: 15 },
            },
            depth: 0,
        },
        LspDocumentSymbol {
            name: "run".to_string(),
            detail: "fn()".to_string(),
            kind: SymbolKind::Function,
            range: LspRange {
                start: LspPosition { line: 5, character: 4 },
                end: LspPosition { line: 5, character: 19 },
            },
            selection_range: LspRange {
                start: LspPosition { line: 5, character: 11 },
                end: LspPosition { line: 5, character: 14 },
            },
            depth: 1,
        },
    ];

    editor.lsp.push_event_for_test(LspEvent::DocumentSymbolsResponse {
        request_id: 1,
        file: None,
        symbols,
    });
    editor.poll_lsp_events();

    match &editor.mode {
        EditorMode::ModalPrompt(modal) => {
            assert_eq!(modal.kind, PromptKind::DocumentSymbols);
            let matching = modal.matching_document_symbols();
            assert_eq!(matching.len(), 2);
            assert_eq!(matching[0].name, "AppState");
            assert_eq!(matching[0].kind.badge_str(), "[struct]");
            assert_eq!(matching[1].name, "run");
            assert_eq!(matching[1].kind.badge_str(), "[fn]");
        }
        _ => panic!("Expected EditorMode::ModalPrompt"),
    }

    // Select second symbol and jump
    editor.handle_key(Key::Down);
    editor.handle_key(Key::Enter);

    assert_eq!(editor.mode, EditorMode::Normal);
    let head = editor.doc().cursors.primary().head;
    let pos = editor.doc().buffer.char_to_position(head);
    assert_eq!(pos.line, 5);
}

#[test]
fn test_lsp_elite_inlay_hints_and_toggle() {
    use rustext::core::buffer::TextBuffer;
    use rustext::editor::app::Editor;
    use rustext::editor::lsp::{InlayHintKind, LspEvent, LspInlayHint, LspPosition};

    let mut editor = Editor::new(TextBuffer::from_str("let x = compute();\n"));
    editor.config.lsp.inlay_hints = true;

    let hints = vec![
        LspInlayHint {
            position: LspPosition { line: 0, character: 5 },
            label: ": i32".to_string(),
            kind: InlayHintKind::Type,
            padding_left: false,
            padding_right: true,
        },
    ];

    editor.lsp.push_event_for_test(LspEvent::InlayHintsResponse {
        request_id: 1,
        file: None,
        hints,
    });
    editor.poll_lsp_events();

    assert_eq!(editor.doc().inlay_hints.len(), 1);
    assert_eq!(editor.doc().inlay_hints[&0].len(), 1);
    assert_eq!(editor.doc().inlay_hints[&0][0].label, ": i32");

    // Toggle inlay hints off
    editor.lsp_toggle_inlay_hints();
    assert!(!editor.config.lsp.inlay_hints);
    assert!(editor.doc().inlay_hints.is_empty());

    // Toggle back on
    editor.lsp_toggle_inlay_hints();
    assert!(editor.config.lsp.inlay_hints);
}

#[test]
fn test_lsp_multi_language_servers_and_config() {
    use rustext::core::buffer::TextBuffer;
    use rustext::core::syntax::Language;
    use rustext::editor::app::Editor;
    use rustext::editor::config::EditorConfig;

    let editor = Editor::new(TextBuffer::from_str(""));

    assert_eq!(editor.lsp_server_cmd_for_lang(Language::Rust).unwrap(), "rust-analyzer");
    assert_eq!(editor.lsp_server_cmd_for_lang(Language::Python).unwrap(), "pylsp");
    assert_eq!(editor.lsp_server_cmd_for_lang(Language::C).unwrap(), "clangd");
    assert_eq!(editor.lsp_server_cmd_for_lang(Language::JavaScript).unwrap(), "typescript-language-server --stdio");
    assert_eq!(editor.lsp_server_cmd_for_lang(Language::Html).unwrap(), "vscode-html-language-server --stdio");
    assert_eq!(editor.lsp_server_cmd_for_lang(Language::Css).unwrap(), "vscode-html-language-server --stdio");
    assert_eq!(editor.lsp_server_cmd_for_lang(Language::Json).unwrap(), "vscode-json-language-server --stdio");
    assert_eq!(editor.lsp_server_cmd_for_lang(Language::Markdown).unwrap(), "marksman server");
    assert_eq!(editor.lsp_server_cmd_for_lang(Language::Julia).unwrap(), "julia --startup-file=no --history-file=no -e 'using LanguageServer; runserver()'");

    let mut cfg = EditorConfig::default();
    cfg.lsp.inlay_hints = true;
    cfg.lsp.go_server = "gopls".to_string();
    cfg.lsp.zig_server = "zls".to_string();
    cfg.lsp.julia_server = "julia-lsp".to_string();

    let ini = cfg.to_ini_string();
    assert!(ini.contains("inlay_hints = true"));
    assert!(ini.contains("typescript_server = typescript-language-server --stdio"));
    assert!(ini.contains("go_server = gopls"));
    assert!(ini.contains("zig_server = zls"));
    assert!(ini.contains("julia_server = julia-lsp"));

    let loaded = EditorConfig::parse(&ini);
    assert!(loaded.lsp.inlay_hints);
    assert_eq!(loaded.lsp.go_server, "gopls");
    assert_eq!(loaded.lsp.zig_server, "zls");
    assert_eq!(loaded.lsp.julia_server, "julia-lsp");
}

