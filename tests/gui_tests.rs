use rustext::core::TextBuffer;
use rustext::editor::Document;
use rustext::gfx::{Framebuffer, GfxRenderer, ThemeColors};
use rustext::ui::prompt::{PromptKind, PromptModal};

#[test]
fn test_framebuffer_allocation_and_resize() {
    let mut fb = Framebuffer::new(320, 240);
    assert_eq!(fb.width, 320);
    assert_eq!(fb.height, 240);
    assert_eq!(fb.pixels.len(), 320 * 240);

    fb.clear(0x00FF0000); // Red
    assert_eq!(fb.get_pixel(10, 10), 0x00FF0000);

    // Resize framebuffer
    fb.resize(640, 480);
    assert_eq!(fb.width, 640);
    assert_eq!(fb.height, 480);
    assert_eq!(fb.pixels.len(), 640 * 480);
    // Preserved existing region
    assert_eq!(fb.get_pixel(10, 10), 0x00FF0000);
}

#[test]
fn test_gfx_renderer_multi_tabs_and_prompt() {
    let mut fb = Framebuffer::new(800, 600);
    let doc = Document::new(TextBuffer::from_str("pub fn test_gui() -> bool {\n    true\n}"));
    let renderer = GfxRenderer::new();
    let tabs = vec!["tab1.rs".to_string(), "tab2.rs".to_string()];

    // Render normal state with menu bar
    let menu = rustext::editor::MenuBar::new();
    renderer.render(&mut fb, &doc, &tabs, 0, Some("Normal Mode"), None, Some(&menu));

    // Pixel at menu bar header should have menu colors
    let menu_pixel = fb.get_pixel(12, 10);
    assert_ne!(menu_pixel, 0x00000000);

    // Render with search modal
    let mut modal = PromptModal::new(PromptKind::Find);
    modal.insert_char('t');
    modal.insert_char('e');
    modal.insert_char('s');
    modal.insert_char('t');

    renderer.render(&mut fb, &doc, &tabs, 1, None, Some(&modal), Some(&menu));

    // Floating prompt modal center should have modal background color
    let prompt_box_pixel = fb.get_pixel(400, 55);
    assert_eq!(prompt_box_pixel, 0x00252526);
}

#[test]
fn test_menu_bar_interactions() {
    use rustext::editor::{EditorAction, MenuBar};

    let mut menu = MenuBar::new();
    assert_eq!(menu.is_open(), false);

    // Click on "Archivo" header (x = 12, y = 10)
    let action = menu.handle_mouse_click(12, 10, 8);
    assert_eq!(action, None);
    assert_eq!(menu.is_open(), true);
    assert_eq!(menu.active_category, Some(0));

    // Click on "Guardar" item (x = 20, y = 24 + 2 + 2*22 + 5 = 75)
    let action = menu.handle_mouse_click(20, 75, 8);
    assert_eq!(action, Some(EditorAction::Save));
    assert_eq!(menu.is_open(), false);
}

#[test]
fn test_theme_colors() {
    let theme = ThemeColors::modern_dark();
    assert_eq!(theme.background, 0x001E1E1E);
    assert_eq!(theme.status_bar_bg, 0x00007ACC);
    assert_eq!(theme.cursor, 0x00528BFF);
}

#[test]
fn test_menu_bar_boundary_and_underflow_safety() {
    use rustext::editor::{EditorAction, MenuBar};

    let mut menu = MenuBar::new();

    // Open first category
    menu.handle_mouse_click(12, 10, 8);
    assert_eq!(menu.is_open(), true);
    assert_eq!(menu.active_category, Some(0));

    // Test exact boundary values of y that previously triggered underflow
    // y = 24 (menu.height), y = 25 (menu.height + 1), y = 26 (menu.height + 2)
    for y in [0, 23, 24, 25, 26, 27, 200, 1000] {
        for x in [0, 8, 20, 260, 500] {
            // Must not panic on any mouse move coordinates!
            menu.handle_mouse_move(x, y, 8);
        }
    }

    // Ensure hovering on padding (y = 24, 25) clears or doesn't select an item
    menu.active_category = Some(0);
    menu.handle_mouse_move(20, 24, 8);
    assert_eq!(menu.hovered_item, None);
    menu.handle_mouse_move(20, 25, 8);
    assert_eq!(menu.hovered_item, None);

    // Hover on first item (y = 26 + 10 = 36) of category 0 ("Archivo", drop_x = 8..268)
    menu.handle_mouse_move(20, 36, 8);
    assert_eq!(menu.hovered_item, Some(0));

    // Clicking exactly at y = 24 or y = 25 inside dropdown area shouldn't crash or trigger item 0
    let action = menu.handle_mouse_click(20, 24, 8);
    assert_eq!(action, None);
    assert_eq!(menu.is_open(), true);

    let action = menu.handle_mouse_click(20, 25, 8);
    assert_eq!(action, None);
    assert_eq!(menu.is_open(), true);

    // Clicking separator should return None and keep menu open or not trigger action
    // "Archivo" items: 0: Nueva, 1: Abrir, 2: Guardar, 3: Guardar Como, 4: Separator
    let sep_y = 24 + 2 + 4 * 22 + 5;
    let action = menu.handle_mouse_click(20, sep_y, 8);
    assert_eq!(action, None);
    assert_eq!(menu.is_open(), true);

    // Re-open category 0 and test clicking item 0 ("Nueva Pestaña")
    let action = menu.handle_mouse_click(20, 36, 8);
    assert_eq!(action, Some(EditorAction::NewTab));
    assert_eq!(menu.is_open(), false);

    // Re-open menu to test clicking outside dropdown closes it
    menu.handle_mouse_click(12, 10, 8);
    assert_eq!(menu.is_open(), true);
    let action = menu.handle_mouse_click(400, 200, 8);
    assert_eq!(action, None);
    assert_eq!(menu.is_open(), false);
}

#[test]
fn test_about_dialog_rendering_and_mode() {
    use rustext::editor::{Editor, EditorAction, EditorMode};
    use rustext::ui::terminal::Key;

    let mut editor = Editor::new(rustext::core::TextBuffer::from_str("hello world"));
    assert_eq!(editor.mode, EditorMode::Normal);

    // Open About Dialog via action
    editor.execute_action(EditorAction::OpenAboutDialog);
    assert_eq!(editor.mode, EditorMode::AboutDialog);

    // Render About Dialog in Framebuffer
    let mut fb = Framebuffer::new(800, 600);
    let renderer = GfxRenderer::new();
    renderer.render_about_dialog(&mut fb, false, false);

    // Pixel in the dialog header area should have dark titlebar color
    let header_pixel = fb.get_pixel(400, (600 - 450) / 2 + 10);
    assert_eq!(header_pixel, 0x002D2D2D);

    // Pixel in body should have body color 0x001E1E1E
    let body_pixel = fb.get_pixel(400, (600 - 450) / 2 + 100);
    assert_ne!(body_pixel, 0x00000000);

    // Esc closes dialog
    editor.handle_key(Key::Esc);
    assert_eq!(editor.mode, EditorMode::Normal);

    // F1 re-opens dialog
    editor.handle_key(Key::F(1));
    assert_eq!(editor.mode, EditorMode::AboutDialog);

    // Enter closes dialog
    editor.handle_key(Key::Enter);
    assert_eq!(editor.mode, EditorMode::Normal);
}

#[test]
fn test_command_palette_floating_window_and_commands() {
    use rustext::editor::EditorAction;
    use rustext::ui::prompt::{PromptKind, PromptModal};

    let mut modal = PromptModal::new(PromptKind::CommandPalette);
    let all_cmds = modal.matching_commands();
    assert!(all_cmds.len() >= 15);

    // Check that "Acerca de Rustext" is in standard commands
    let about_cmd = all_cmds.iter().find(|c| c.name == "Acerca de Rustext");
    assert!(about_cmd.is_some());
    assert_eq!(about_cmd.unwrap().action, EditorAction::OpenAboutDialog);

    // Filter by typing "acerca"
    modal.insert_char('a');
    modal.insert_char('c');
    modal.insert_char('e');
    modal.insert_char('r');
    modal.insert_char('c');
    modal.insert_char('a');
    let filtered = modal.matching_commands();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "Acerca de Rustext");

    // Render command palette window
    let mut fb = Framebuffer::new(800, 600);
    let renderer = GfxRenderer::new();
    renderer.render_prompt_overlay_with_hover(&mut fb, &modal, false);

    // Title bar of floating window
    let header_pixel = fb.get_pixel(400, 28 + 10);
    assert_eq!(header_pixel, 0x002D2D2D);
}

#[test]
fn test_file_dialog_navigation_and_saving() {
    use rustext::editor::Editor;
    use rustext::gfx::{Framebuffer, GfxRenderer};
    use rustext::ui::prompt::{expand_tilde, get_quick_bookmarks, PromptKind, PromptModal};
    use std::fs;

    // 1. Bookmarks & Tilde expansion
    let bookmarks = get_quick_bookmarks();
    assert!(!bookmarks.is_empty(), "Quick bookmarks should detect at least project directory");

    let tilde_path = expand_tilde("~/prueba.txt");
    assert!(!tilde_path.to_string_lossy().starts_with('~'));

    // 2. Initialize file dialog modal
    let cur_dir = std::env::current_dir().unwrap();
    let mut modal = PromptModal::new_with_dir(PromptKind::SaveAs, cur_dir.clone(), Some("nuevo.txt"));
    assert!(modal.is_file_dialog());
    assert_eq!(modal.input, "nuevo.txt");
    assert!(!modal.dir_entries.is_empty(), "Current dir should contain entries");

    // Test selection navigation
    modal.select_next_entry(10);
    assert_eq!(modal.selected_entry_idx, 1);
    modal.select_prev_entry();
    assert_eq!(modal.selected_entry_idx, 0);

    // Test navigation up and back
    let initial_dir = modal.current_dir.clone();
    modal.navigate_up();
    assert_ne!(modal.current_dir, initial_dir);
    modal.navigate_to(initial_dir.clone());
    assert_eq!(modal.current_dir, initial_dir);

    // 3. Render file dialog in Framebuffer
    let mut fb = Framebuffer::new(800, 600);
    let renderer = GfxRenderer::new();
    renderer.render_file_dialog(&mut fb, &modal, false);

    // Pixel in dialog header should be dark titlebar color
    let box_y = (600 - 480) / 2;
    let header_pixel = fb.get_pixel(400, box_y + 10);
    assert_eq!(header_pixel, 0x002D2D2D);

    // 4. Test recursive directory creation and saving in arbitrary folder
    let temp_base = std::env::temp_dir().join("rustext_save_test");
    let nested_target = temp_base.join("carpetas").join("subcarpetas").join("archivo_guardado.rs");
    if temp_base.exists() {
        let _ = fs::remove_dir_all(&temp_base);
    }

    let mut editor = Editor::new(rustext::core::TextBuffer::from_str("fn test_func() -> bool { true }"));
    
    // Simulate SaveAs modal pointing to the deep nested path
    let mut save_modal = PromptModal::new_with_dir(PromptKind::SaveAs, temp_base.clone(), None);
    save_modal.input = nested_target.to_string_lossy().to_string();
    editor.commit_prompt(&save_modal);

    // Verify file and parent directories were created
    assert!(nested_target.exists(), "Target file must be saved successfully");
    let content = fs::read_to_string(&nested_target).unwrap();
    assert_eq!(content, "fn test_func() -> bool { true }");

    // Test opening the file back into editor
    editor.open_file(&nested_target);
    assert_eq!(editor.doc().buffer.to_string(), "fn test_func() -> bool { true }");

    // Clean up
    let _ = fs::remove_dir_all(&temp_base);
}

#[test]
fn test_extension_manager_conf_parsing_and_execution() {
    use rustext::plugin::{ExtensionInfo, ExtensionManager};
    use std::path::PathBuf;

    let conf = r#"
[extension]
id = formateador_json
name = Formateador JSON
version = 1.2.0
author = Bolek
description = Formatea y embellece texto JSON
enabled = true
on_save = false
exec = python3 format.py
args = --indent 4

[commands]
cmd_format = Formatear JSON | Ctrl+Alt+F
cmd_minify = Minificar JSON
"#;

    let info = ExtensionInfo::from_conf_str(conf, PathBuf::from("/tmp/ext_test"));
    assert_eq!(info.id, "formateador_json");
    assert_eq!(info.name, "Formateador JSON");
    assert_eq!(info.version, "1.2.0");
    assert_eq!(info.author, "Bolek");
    assert_eq!(info.description, "Formatea y embellece texto JSON");
    assert!(info.enabled);
    assert!(!info.on_save);
    assert_eq!(info.commands.len(), 2);
    assert_eq!(info.commands[0].id, "cmd_format");
    assert_eq!(info.commands[0].label, "Formatear JSON");
    assert_eq!(info.commands[0].shortcut, "Ctrl+Alt+F");
    assert_eq!(info.commands[1].id, "cmd_minify");
    assert_eq!(info.commands[1].label, "Minificar JSON");

    // Test ExtensionManager in isolation
    let mut mgr = ExtensionManager {
        extensions: vec![info],
        base_dirs: Vec::new(),
    };

    let base_count = mgr.get_active_commands().len();
    assert_eq!(base_count, 2);

    // Toggle disabled
    let new_state = mgr.toggle_extension(0);
    assert!(!new_state);
    assert_eq!(mgr.get_active_commands().len(), 0);

    // Toggle enabled again
    let new_state2 = mgr.toggle_extension(0);
    assert!(new_state2);
    assert_eq!(mgr.get_active_commands().len(), 2);
}

#[test]
fn test_extension_dialog_rendering_and_editor_mode() {
    use rustext::editor::app::{Editor, EditorMode};
    use rustext::editor::keymap::EditorAction;
    use rustext::gfx::{Framebuffer, GfxRenderer};
    use rustext::ui::terminal::Key;

    let mut editor = Editor::new(rustext::core::TextBuffer::from_str("Hello World"));

    // 1. Open Extensions Manager via action
    editor.execute_action(EditorAction::OpenExtensionsManager);
    assert_eq!(editor.mode, EditorMode::ExtensionsManager { selected_idx: 0, scroll_offset: 0 });

    // 2. Test keyboard navigation in Extensions Manager
    editor.handle_key(Key::Down);
    if let EditorMode::ExtensionsManager { selected_idx, .. } = editor.mode {
        if editor.extension_manager.extensions.len() > 1 {
            assert_eq!(selected_idx, 1);
        }
    }

    editor.handle_key(Key::Up);
    if let EditorMode::ExtensionsManager { selected_idx, .. } = editor.mode {
        assert_eq!(selected_idx, 0);
    }

    // Toggle with Space
    if !editor.extension_manager.extensions.is_empty() {
        let initial_state = editor.extension_manager.extensions[0].enabled;
        editor.handle_key(Key::Char(' '));
        assert_eq!(editor.extension_manager.extensions[0].enabled, !initial_state);
        // Toggle back
        editor.handle_key(Key::Char(' '));
        assert_eq!(editor.extension_manager.extensions[0].enabled, initial_state);
    }

    // Exit with Esc
    editor.handle_key(Key::Esc);
    assert_eq!(editor.mode, EditorMode::Normal);

    // 3. Render extensions dialog in Framebuffer
    let mut fb = Framebuffer::new(800, 600);
    let renderer = GfxRenderer::new();
    renderer.render_extensions_dialog(&mut fb, &editor.extension_manager, 0, 0, false);

    // Dialog titlebar pixel
    let win_y = (600 - 480) / 2;
    let header_pixel = fb.get_pixel(400, win_y + 10);
    assert_eq!(header_pixel, 0x002D2D2D);
}

#[test]
fn test_scrollbar_geometry_and_proportions() {
    let renderer = GfxRenderer::new();
    let fb_w = 800;
    let fb_h = 600;

    // 1. Short document (10 lines)
    let short_doc = Document::new(TextBuffer::from_str("line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10"));
    let geom_short = renderer.scrollbar_geometry(fb_w, fb_h, &short_doc, true);

    assert_eq!(geom_short.x, fb_w - renderer.scrollbar_width);
    assert_eq!(geom_short.width, renderer.scrollbar_width);
    assert_eq!(geom_short.y, renderer.menu_bar_height + renderer.tab_bar_height);
    // Since visible_lines exceeds 10 lines, max_scroll should be 0 and thumb fills the track
    assert_eq!(geom_short.max_scroll, 0);
    assert_eq!(geom_short.thumb_y, geom_short.y);
    assert_eq!(geom_short.thumb_height, geom_short.height);

    // 2. Large document (1000 lines)
    let thousand_lines = (0..1000).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
    let mut long_doc = Document::new(TextBuffer::from_str(&thousand_lines));
    let geom_long_top = renderer.scrollbar_geometry(fb_w, fb_h, &long_doc, true);

    assert!(geom_long_top.max_scroll > 900);
    assert!(geom_long_top.thumb_height >= 20);
    assert!(geom_long_top.thumb_height < geom_long_top.height);
    assert_eq!(geom_long_top.thumb_y, geom_long_top.y);

    // Hit test checks
    assert!(geom_long_top.contains_point(795, 100));
    assert!(!geom_long_top.contains_point(780, 100)); // outside track (to the left)
    assert!(geom_long_top.thumb_contains_point(795, geom_long_top.y + 5));

    // Scroll down to 50%
    long_doc.scroll_row = geom_long_top.max_scroll / 2;
    let geom_long_mid = renderer.scrollbar_geometry(fb_w, fb_h, &long_doc, true);
    assert!(geom_long_mid.thumb_y > geom_long_top.thumb_y);

    // Bidirectional translation: row_for_thumb_y
    let calculated_row = geom_long_mid.row_for_thumb_y(geom_long_mid.thumb_y);
    let diff = (calculated_row as isize - long_doc.scroll_row as isize).abs();
    assert!(diff <= 2, "Calculated row {} should be close to actual scroll row {}", calculated_row, long_doc.scroll_row);
}

#[test]
fn test_scrollbar_rendering_in_framebuffer() {
    let renderer = GfxRenderer::new();
    let mut fb = Framebuffer::new(800, 600);

    let thousand_lines = (0..500).map(|i| format!("fn function_{}() -> i32 {{ {} }}", i, i)).collect::<Vec<_>>().join("\n");
    let doc = Document::new(TextBuffer::from_str(&thousand_lines));
    let tabs = vec!["big_file.rs".to_string()];

    // 1. Normal state rendering
    renderer.render_with_scrollbar_state(&mut fb, &doc, &tabs, 0, None, None, None, false, false);

    let geom = renderer.scrollbar_geometry(800, 600, &doc, false);
    // Divider line at geom.x
    let divider_pixel = fb.get_pixel(geom.x, geom.y + 10);
    assert_eq!(divider_pixel, renderer.colors.scrollbar_border);

    // Thumb pixel inside thumb
    let thumb_pixel = fb.get_pixel(geom.x + 4, geom.thumb_y + 5);
    assert_eq!(thumb_pixel, renderer.colors.scrollbar_thumb);

    // Track pixel below thumb
    let track_pixel = fb.get_pixel(geom.x + 4, geom.thumb_y + geom.thumb_height + 20);
    assert_eq!(track_pixel, renderer.colors.scrollbar_bg);

    // 2. Hover state rendering
    renderer.render_with_scrollbar_state(&mut fb, &doc, &tabs, 0, None, None, None, true, false);
    let hovered_thumb_pixel = fb.get_pixel(geom.x + 4, geom.thumb_y + 5);
    assert_eq!(hovered_thumb_pixel, renderer.colors.scrollbar_thumb_hover);

    // 3. Dragging state rendering
    renderer.render_with_scrollbar_state(&mut fb, &doc, &tabs, 0, None, None, None, true, true);
    let dragging_thumb_pixel = fb.get_pixel(geom.x + 4, geom.thumb_y + 5);
    assert_eq!(dragging_thumb_pixel, renderer.colors.scrollbar_thumb_active);
}

#[test]
fn test_scrollbar_interaction_page_up_down_and_drag() {
    let renderer = GfxRenderer::new();
    let hundred_lines = (0..200).map(|i| format!("Line number {}", i)).collect::<Vec<_>>().join("\n");
    let mut doc = Document::new(TextBuffer::from_str(&hundred_lines));
    let geom = renderer.scrollbar_geometry(800, 600, &doc, true);

    assert_eq!(doc.scroll_row, 0);

    // Click below thumb -> Page Down
    let click_y_below = geom.thumb_y + geom.thumb_height + 50;
    assert!(geom.contains_point(geom.x + 5, click_y_below));
    assert!(!geom.thumb_contains_point(geom.x + 5, click_y_below));

    let visible = geom.visible_lines.max(1);
    doc.scroll_row = (doc.scroll_row + visible).min(geom.max_scroll);
    assert_eq!(doc.scroll_row, visible);

    // Click above thumb -> Page Up
    let new_geom = renderer.scrollbar_geometry(800, 600, &doc, true);
    assert!(new_geom.thumb_y > geom.thumb_y);
    let click_y_above = geom.thumb_y + 2;
    assert!(click_y_above < new_geom.thumb_y);

    doc.scroll_row = doc.scroll_row.saturating_sub(visible);
    assert_eq!(doc.scroll_row, 0);

    // Drag simulation
    let drag_target_thumb_y = new_geom.y + (new_geom.height.saturating_sub(new_geom.thumb_height)) / 2;
    let dragged_row = new_geom.row_for_thumb_y(drag_target_thumb_y);
    assert!(dragged_row > 0 && dragged_row < geom.max_scroll);
}

#[test]
fn test_find_and_replace_interactive_flow() {
    use rustext::editor::{Editor, EditorAction, EditorMode};

    let mut editor = Editor::new(TextBuffer::from_str("foo bar foo baz foo"));
    assert_eq!(editor.mode, EditorMode::Normal);

    // 1. Open Replace Prompt via action
    editor.execute_action(EditorAction::OpenReplacePrompt);
    assert_eq!(matches!(editor.mode, EditorMode::ModalPrompt(ref m) if m.kind == PromptKind::Replace), true);

    // 2. Set search and replace query
    if let EditorMode::ModalPrompt(ref mut modal) = editor.mode {
        modal.input = "foo".to_string();
        modal.cursor_col = 3;
        modal.replace_input = "qux".to_string();
        modal.replace_cursor_col = 3;
    }

    // 3. Find next match
    editor.find_next_match("foo");
    let primary = editor.doc().cursors.primary();
    assert!(primary.has_selection());
    let range = primary.selection_range();
    assert_eq!(range.start, 0);
    assert_eq!(range.end, 3);

    // 4. Replace current match
    editor.replace_current_match("foo", "qux");
    assert_eq!(editor.doc().buffer.to_string(), "qux bar foo baz foo");
    // Should automatically select next match (at offset 8..11)
    let next_primary = editor.doc().cursors.primary();
    assert!(next_primary.has_selection());
    let next_range = next_primary.selection_range();
    assert_eq!(next_range.start, 8);
    assert_eq!(next_range.end, 11);

    // 5. Replace all remaining matches
    editor.replace_all_matches("foo", "qux");
    assert_eq!(editor.doc().buffer.to_string(), "qux bar qux baz qux");

    // 6. Test atomic undo
    editor.execute_action(EditorAction::Undo);
    assert_eq!(editor.doc().buffer.to_string(), "qux bar foo baz foo");

    editor.execute_action(EditorAction::Undo);
    assert_eq!(editor.doc().buffer.to_string(), "foo bar foo baz foo");
}

#[test]
fn test_find_replace_dialog_rendering() {
    let mut fb = Framebuffer::new(800, 600);
    let renderer = GfxRenderer::new();

    let mut modal = PromptModal::new(PromptKind::Replace);
    modal.input = "rustext".to_string();
    modal.replace_input = "editor".to_string();
    modal.active_field = 1;

    renderer.render_prompt_overlay_with_hover(&mut fb, &modal, false);

    let box_w = 800.min(540).max(380);
    let box_x = (800 - box_w) / 2;
    let box_y = 28;

    // Header background pixel (left of text)
    let header_pixel = fb.get_pixel(box_x + 4, box_y + 4);
    assert_eq!(header_pixel, 0x002D2D2D);

    // Field 2 (Reemplazar) focused border should be blue (0x00007ACC)
    let field2_border_pixel = fb.get_pixel(box_x + 12 + 100 - 1, box_y + 24 + 1 + 12 + 24 + 10);
    assert_eq!(field2_border_pixel, 0x00007ACC);

    // Button pixel
    let btn_pixel = fb.get_pixel(box_x + 12 + 5, box_y + 24 + 1 + 12 + 24 + 10 + 24 + 16 + 5);
    assert_ne!(btn_pixel, 0x00000000);
}

#[test]
fn test_find_replace_field_switching_and_editing() {
    let mut modal = PromptModal::new(PromptKind::Replace);
    assert_eq!(modal.active_field, 0);

    // Type into Search field
    modal.insert_char('f');
    modal.insert_char('i');
    modal.insert_char('n');
    modal.insert_char('d');
    assert_eq!(modal.input, "find");
    assert_eq!(modal.cursor_col, 4);

    // Toggle to Replace field
    modal.toggle_active_field();
    assert_eq!(modal.active_field, 1);

    // Type into Replace field
    modal.insert_char('r');
    modal.insert_char('e');
    modal.insert_char('p');
    modal.insert_char('l');
    assert_eq!(modal.replace_input, "repl");
    assert_eq!(modal.replace_cursor_col, 4);

    // Backspace in Replace field
    modal.backspace();
    assert_eq!(modal.replace_input, "rep");
    assert_eq!(modal.replace_cursor_col, 3);
    assert_eq!(modal.input, "find"); // Find field remains untouched

    // Formatted display
    let display = modal.formatted_display();
    assert!(display.contains("find"));
    assert!(display.contains("rep"));
}

#[test]
fn test_config_ini_parsing_and_serialization() {
    use rustext::editor::config::EditorConfig;

    let default_ini = EditorConfig::default_ini_content();
    let config = EditorConfig::parse(default_ini);
    assert_eq!(config.editor.tab_size, 4);
    assert!(config.editor.auto_indent);
    assert!(config.editor.auto_close_brackets);
    assert!(config.editor.auto_surround);
    assert!(config.editor.show_line_numbers);
    assert_eq!(config.theme.name, "dark");
    assert_eq!(config.gui.window_width, 1024);
    assert_eq!(config.gui.window_height, 640);
    assert!(config.gui.menu_bar);
    assert!(config.gui.status_bar);
    assert!(config.gui.scrollbar);

    // Custom configuration
    let custom_ini = r#"
[editor]
tab_size = 8
auto_indent = false
auto_close_brackets = 0
auto_surround = no
show_line_numbers = off

[theme]
name = monokai

[gui]
window_width = 1440
window_height = 900
menu_bar = off
status_bar = false
scrollbar = 0
"#;
    let custom_cfg = EditorConfig::parse(custom_ini);
    assert_eq!(custom_cfg.editor.tab_size, 8);
    assert!(!custom_cfg.editor.auto_indent);
    assert!(!custom_cfg.editor.auto_close_brackets);
    assert!(!custom_cfg.editor.auto_surround);
    assert!(!custom_cfg.editor.show_line_numbers);
    assert_eq!(custom_cfg.theme.name, "monokai");
    assert_eq!(custom_cfg.gui.window_width, 1440);
    assert_eq!(custom_cfg.gui.window_height, 900);
    assert!(!custom_cfg.gui.menu_bar);
    assert!(!custom_cfg.gui.status_bar);
    assert!(!custom_cfg.gui.scrollbar);

    // Test serialization roundtrip
    let serialized = custom_cfg.to_ini_string();
    let roundtrip_cfg = EditorConfig::parse(&serialized);
    assert_eq!(custom_cfg, roundtrip_cfg);

    // Clamping test
    let extreme_ini = r#"
[editor]
tab_size = 999
[gui]
window_width = 50
window_height = 10000
"#;
    let clamped = EditorConfig::parse(extreme_ini);
    assert_eq!(clamped.editor.tab_size, 16);
    assert_eq!(clamped.gui.window_width, 320);
    assert_eq!(clamped.gui.window_height, 4320);
}

#[test]
fn test_theme_variants_and_switching() {
    use rustext::editor::config::EditorTheme;
    use rustext::gfx::ThemeColors;
    use rustext::ui::Theme;

    let dark = ThemeColors::by_name("dark");
    let monokai = ThemeColors::by_name("monokai");
    let solarized = ThemeColors::by_name("solarized");
    let light = ThemeColors::by_name("light");

    assert_eq!(dark.background, 0x001E1E1E);
    assert_eq!(monokai.background, 0x00272822);
    assert_eq!(solarized.background, 0x00002B36);
    assert_eq!(light.background, 0x00FFFFFF);

    // Verify ANSI TUI themes
    let ansi_dark = Theme::by_name("dark");
    let ansi_light = Theme::by_name("light");
    assert_ne!(ansi_dark.status_bar_bg, ansi_light.status_bar_bg);

    // Enum conversion
    assert_eq!(EditorTheme::from_name("monokai"), EditorTheme::Monokai);
    assert_eq!(EditorTheme::from_name("solarized_dark"), EditorTheme::SolarizedDark);
    assert_eq!(EditorTheme::from_name("light"), EditorTheme::Light);
    assert_eq!(EditorTheme::from_name("unknown"), EditorTheme::Dark);
}

#[test]
fn test_editor_actions_config_and_theme() {
    use rustext::core::TextBuffer;
    use rustext::editor::app::Editor;
    use rustext::editor::config::{EditorConfig, EditorTheme};
    use rustext::editor::keymap::EditorAction;

    let mut editor = Editor::new(TextBuffer::from_str("fn main() {}"));

    // Set theme via action
    editor.execute_action(EditorAction::SetTheme(EditorTheme::Monokai));
    assert_eq!(editor.config.theme.name, "monokai");

    // Open config file into tab
    let temp_conf = std::env::temp_dir().join("test_rustext_config.ini");
    std::fs::write(&temp_conf, EditorConfig::default_ini_content()).unwrap();
    editor.open_file(&temp_conf);
    assert_eq!(editor.documents.len(), 2);
    assert!(editor.doc().buffer.file_path().unwrap().ends_with("test_rustext_config.ini"));
    let _ = std::fs::remove_file(&temp_conf);

    // Toggle line numbers in renderer
    let mut fb = rustext::gfx::Framebuffer::new(800, 600);
    let mut renderer = rustext::gfx::GfxRenderer::new();
    renderer.show_line_numbers = false;
    let tabs = vec!["main.rs".to_string()];
    renderer.render(&mut fb, editor.doc(), &tabs, 0, None, None, None);
    assert_eq!(renderer.show_line_numbers, false);
}

#[test]
fn test_snippet_template_parsing() {
    use rustext::editor::snippet::ExpandedSnippet;

    let template = "fn ${1:my_func}(${2:arg: i32}) -> ${3:bool} {\n    ${0:true}\n}";
    let expanded = ExpandedSnippet::parse(template);

    assert_eq!(
        expanded.text,
        "fn my_func(arg: i32) -> bool {\n    true\n}"
    );

    // Verify tabstops parsed in proper order
    assert_eq!(expanded.tabstops.len(), 4);
    assert_eq!(expanded.tabstops[0].index, 1);
    assert_eq!(expanded.tabstops[0].placeholder, "my_func");
    assert_eq!(expanded.tabstops[0].len, 7);

    assert_eq!(expanded.tabstops[1].index, 2);
    assert_eq!(expanded.tabstops[1].placeholder, "arg: i32");
    assert_eq!(expanded.tabstops[1].len, 8);

    assert_eq!(expanded.tabstops[2].index, 3);
    assert_eq!(expanded.tabstops[2].placeholder, "bool");
    assert_eq!(expanded.tabstops[2].len, 4);

    assert_eq!(expanded.tabstops[3].index, 0);
    assert_eq!(expanded.tabstops[3].placeholder, "true");
    assert_eq!(expanded.tabstops[3].len, 4);
}

#[test]
fn test_snippet_indentation_preservation() {
    use rustext::core::syntax::Language;
    use rustext::editor::snippet::Snippet;

    let snip = Snippet::new(
        "fn",
        "Function",
        "fn ${1:name}() {\n    ${0}\n}",
        Language::Rust,
    );

    let expanded = snip.expand("    ");
    assert_eq!(
        expanded.text,
        "fn name() {\n        \n    }"
    );
}

#[test]
fn test_snippet_expansion_and_tab_navigation() {
    use rustext::core::TextBuffer;
    use rustext::editor::app::Editor;
    use rustext::ui::terminal::Key;

    let mut editor = Editor::new(TextBuffer::new());
    editor.doc_mut().language = rustext::core::syntax::Language::Rust;

    // 1. Type "fn" and press Tab
    editor.handle_key(Key::Char('f'));
    editor.handle_key(Key::Char('n'));
    assert_eq!(editor.doc().buffer.to_string(), "fn");

    editor.handle_key(Key::Tab);

    // Should have expanded "fn" snippet
    assert!(editor.doc().buffer.to_string().starts_with("fn name("));
    assert!(editor.snippet_session.is_some());

    // Primary cursor should currently select "name"
    assert!(editor.doc().cursors.primary().has_selection());
    let range = editor.doc().cursors.primary().selection_range();
    let selected_text = editor.doc().buffer.get_text_range(range.start, range.end);
    assert_eq!(selected_text, "name");

    // 2. Type function name to replace "name"
    for c in "calcular_total".chars() {
        editor.handle_key(Key::Char(c));
    }
    assert!(editor.doc().buffer.to_string().starts_with("fn calcular_total("));

    // 3. Press Tab to advance to params tabstop
    editor.handle_key(Key::Tab);
    assert!(editor.snippet_session.is_some());
    let range2 = editor.doc().cursors.primary().selection_range();
    let selected_params = editor.doc().buffer.get_text_range(range2.start, range2.end);
    assert_eq!(selected_params, "params");

    // 4. Type params
    for c in "x: i32, y: i32".chars() {
        editor.handle_key(Key::Char(c));
    }

    // 5. Press Tab to advance to return type
    editor.handle_key(Key::Tab);
    let range3 = editor.doc().cursors.primary().selection_range();
    let selected_ret = editor.doc().buffer.get_text_range(range3.start, range3.end);
    assert_eq!(selected_ret, "Result<(), Box<dyn std::error::Error>>");

    // 6. Press Tab to advance to body ($0)
    editor.handle_key(Key::Tab);
    let range4 = editor.doc().cursors.primary().selection_range();
    let selected_body = editor.doc().buffer.get_text_range(range4.start, range4.end);
    assert_eq!(selected_body, "todo!()");

    // 7. Press Tab after final tabstop to finish session
    editor.handle_key(Key::Tab);
    assert!(editor.snippet_session.is_none());
}

#[test]
fn test_snippet_registry_and_languages() {
    use rustext::core::syntax::Language;
    use rustext::editor::snippet::SnippetRegistry;

    assert!(SnippetRegistry::find(Language::Rust, "fn").is_some());
    assert!(SnippetRegistry::find(Language::Rust, "struct").is_some());
    assert!(SnippetRegistry::find(Language::Python, "def").is_some());
    assert!(SnippetRegistry::find(Language::Python, "class").is_some());
    assert!(SnippetRegistry::find(Language::C, "main").is_some());
    assert!(SnippetRegistry::find(Language::JavaScript, "afn").is_some());
    assert!(SnippetRegistry::find(Language::Markdown, "link").is_some());

    // Non-existent trigger
    assert!(SnippetRegistry::find(Language::Rust, "non_existent_keyword").is_none());
}

#[test]
fn test_diagnostic_collection_operations() {
    use std::path::PathBuf;
    use rustext::editor::diagnostic::{Diagnostic, DiagnosticSeverity, DiagnosticCollection};

    let mut collection = DiagnosticCollection::new();
    assert!(collection.is_empty());

    collection.add(Diagnostic::new(
        Some(PathBuf::from("src/main.rs")),
        10,
        5,
        DiagnosticSeverity::Error,
        "cannot borrow `*self` as mutable more than once at a time".to_string(),
        Some("E0499".to_string()),
    ));

    collection.add(Diagnostic::new(
        Some(PathBuf::from("src/main.rs")),
        5,
        1,
        DiagnosticSeverity::Warning,
        "unused variable `x`".to_string(),
        Some("unused_variables".to_string()),
    ));

    assert_eq!(collection.len(), 2);
    assert!(collection.has_errors());
    assert!(collection.has_warnings());

    // Sorted by line (line 5 first, then line 10)
    assert_eq!(collection.all()[0].line, 5);
    assert_eq!(collection.all()[1].line, 10);

    // Query by line
    assert!(collection.for_line(5).is_some());
    assert_eq!(collection.for_line(5).unwrap().severity, DiagnosticSeverity::Warning);
    assert!(collection.for_line(10).is_some());
    assert_eq!(collection.for_line(10).unwrap().severity, DiagnosticSeverity::Error);
    assert!(collection.for_line(99).is_none());

    // Navigation: next and previous
    let next = collection.next_diagnostic(2);
    assert_eq!(next.unwrap().line, 5);

    let next2 = collection.next_diagnostic(5);
    assert_eq!(next2.unwrap().line, 10);

    let prev = collection.prev_diagnostic(8);
    assert_eq!(prev.unwrap().line, 5);

    // Display string format
    let disp = collection.for_line(10).unwrap().to_display_string();
    assert!(disp.contains("Error [E0499]"));
    assert!(disp.contains("L10:5"));
}

#[test]
fn test_cargo_json_diagnostic_parsing() {
    use rustext::editor::diagnostic::{DiagnosticCollection, DiagnosticSeverity};

    let sample_cargo_json = r#"{"reason":"compiler-artifact","package_id":"rustext 0.1.0"}
{"reason":"compiler-message","package_id":"rustext 0.1.0","message":{"rendered":"error[E0499]: cannot borrow...","code":{"code":"E0499"},"level":"error","message":"cannot borrow `*self` as mutable more than once at a time","spans":[{"column_end":13,"column_start":9,"file_name":"src/editor/app.rs","is_primary":true,"line_end":1645,"line_start":1645}]}}
{"reason":"build-finished","success":false}
"#;

    let diags = DiagnosticCollection::parse_cargo_json(sample_cargo_json);
    assert_eq!(diags.len(), 1);
    let diag = &diags[0];
    assert_eq!(diag.severity, DiagnosticSeverity::Error);
    assert_eq!(diag.line, 1645);
    assert_eq!(diag.col, 9);
    assert_eq!(diag.code.as_deref(), Some("E0499"));
    assert!(diag.message.contains("cannot borrow"));

    // Test standard compiler line parser
    let standard_err = "src/main.rs:42:15: error: expected `;`, found `}`\n";
    let parsed_std = DiagnosticCollection::parse_standard_errors(standard_err, None);
    assert_eq!(parsed_std.len(), 1);
    assert_eq!(parsed_std[0].line, 42);
    assert_eq!(parsed_std[0].col, 15);
    assert_eq!(parsed_std[0].severity, DiagnosticSeverity::Error);
}

#[test]
fn test_editor_actions_formatting_and_diagnostics() {
    use rustext::core::TextBuffer;
    use rustext::editor::app::Editor;
    use rustext::editor::keymap::EditorAction;
    use rustext::editor::diagnostic::{Diagnostic, DiagnosticSeverity};

    let mut editor = Editor::new(TextBuffer::from_str("line 1\nline 2\nline 3\nline 4\nline 5\n"));

    editor.diagnostics.add(Diagnostic::new(
        None,
        4,
        2,
        DiagnosticSeverity::Error,
        "mismatched types".to_string(),
        Some("E0308".to_string()),
    ));

    // Jump to next diagnostic
    editor.execute_action(EditorAction::NextDiagnostic);
    let pos = editor.doc().buffer.char_to_position(editor.doc().cursors.primary().head);
    assert_eq!(pos.line, 3); // line 4 is 0-indexed 3
    assert_eq!(pos.col, 1);

    // Status message should reflect the diagnostic
    assert!(editor.status_message.is_some());
    let (msg, _) = editor.status_message.as_ref().unwrap();
    assert!(msg.contains("Error [E0308]"));
    assert!(msg.contains("mismatched types"));

    // Check menu bar has Herramientas
    assert!(editor.menu_bar.categories.iter().any(|c| c.name == "Herramientas"));
}

#[test]
fn test_config_format_and_check_on_save() {
    use rustext::editor::config::EditorConfig;

    let ini_text = r#"
[editor]
tab_size = 2
auto_indent = true
auto_close_brackets = true
auto_surround = true
show_line_numbers = true
format_on_save = true
check_on_save = false

[theme]
name = monokai
"#;

    let config = EditorConfig::parse(ini_text);
    assert_eq!(config.editor.tab_size, 2);
    assert!(config.editor.format_on_save);
    assert!(!config.editor.check_on_save);

    let serialized = config.to_ini_string();
    assert!(serialized.contains("format_on_save = true"));
    assert!(serialized.contains("check_on_save = false"));
}





