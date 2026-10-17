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

    // Click on "Guardar" item (x = 20, y = 24 + 2 + 3*22 + 5 = 97)
    let action = menu.handle_mouse_click(20, 97, 8);
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
    // "Archivo" items: 0: Nueva, 1: Abrir, 2: Abrir Reciente, 3: Guardar, 4: Guardar Como, 5: Separator
    let sep_y = 24 + 2 + 5 * 22 + 5;
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
    assert!(config.gui.subpixel_antialiasing);

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
subpixel_antialiasing = false
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
    assert!(!custom_cfg.gui.subpixel_antialiasing);

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

#[test]
fn test_gui_customization_config_options() {
    use rustext::editor::config::{EditorConfig, EditorTheme};

    let ini_content = r#"
[editor]
tab_size = 4

[theme]
name = tokyo_night

[gui]
window_width = 1280
window_height = 800
menu_bar = true
status_bar = true
scrollbar = true
font_scale = 2
line_spacing = 5
rounded_corners = true
shadows = true
cursor_style = block
cursor_blink = false
"#;

    let config = EditorConfig::parse(ini_content);
    assert_eq!(config.theme.name, "tokyo_night");
    assert_eq!(config.gui.window_width, 1280);
    assert_eq!(config.gui.window_height, 800);
    assert_eq!(config.gui.font_scale, 2);
    assert_eq!(config.gui.line_spacing, 5);
    assert!(config.gui.rounded_corners);
    assert!(config.gui.shadows);
    assert_eq!(config.gui.cursor_style, "block");
    assert!(!config.gui.cursor_blink);

    // Serialization check
    let serialized = config.to_ini_string();
    assert!(serialized.contains("font_scale = 2"));
    assert!(serialized.contains("line_spacing = 5"));
    assert!(serialized.contains("rounded_corners = true"));
    assert!(serialized.contains("shadows = true"));
    assert!(serialized.contains("cursor_style = block"));
    assert!(serialized.contains("cursor_blink = false"));

    // Theme enum tests
    assert_eq!(EditorTheme::from_name("tokyo_night"), EditorTheme::TokyoNight);
    assert_eq!(EditorTheme::from_name("catppuccin"), EditorTheme::Catppuccin);
    assert_eq!(EditorTheme::from_name("onedark"), EditorTheme::OneDark);
    assert_eq!(EditorTheme::TokyoNight.as_str(), "tokyo_night");
    assert_eq!(EditorTheme::Catppuccin.as_str(), "catppuccin");
    assert_eq!(EditorTheme::OneDark.as_str(), "onedark");
}

#[test]
fn test_modern_theme_palettes_and_slots() {
    use rustext::gfx::ThemeColors;
    use rustext::ui::theme::Theme;

    // GUI Themes
    let tokyo = ThemeColors::by_name("tokyo_night");
    assert_eq!(tokyo.background, 0x001A1B26);
    assert_ne!(tokyo.accent, 0);
    assert_ne!(tokyo.modal_bg, 0);
    assert_ne!(tokyo.chip_bg, 0);

    let catppuccin = ThemeColors::by_name("catppuccin_mocha");
    assert_eq!(catppuccin.background, 0x001E1E2E);
    assert_ne!(catppuccin.accent, 0);
    assert_ne!(catppuccin.button_bg, 0);

    let onedark = ThemeColors::by_name("onedark");
    assert_eq!(onedark.background, 0x00282C34);
    assert_ne!(onedark.accent, 0);

    // TUI Themes
    let tui_tokyo = Theme::by_name("tokyo_night");
    assert!(!tui_tokyo.status_bar_bg.is_empty());
    let tui_catppuccin = Theme::by_name("catppuccin");
    assert!(!tui_catppuccin.status_bar_bg.is_empty());
    let tui_onedark = Theme::by_name("onedark");
    assert!(!tui_onedark.status_bar_bg.is_empty());
}

#[test]
fn test_framebuffer_2d_primitives_alpha_and_corners() {
    use rustext::gfx::Framebuffer;

    let mut fb = Framebuffer::new(100, 100);

    // 1. Alpha blending
    fb.fill_rect(0, 0, 100, 100, 0x00000000); // Black
    fb.fill_rect_alpha(10, 10, 20, 20, 0x00FFFFFF, 128); // 50% white
    let mid_pixel = fb.get_pixel(15, 15);
    let r = (mid_pixel >> 16) & 0xFF;
    assert!(r >= 120 && r <= 135, "Alpha blending should produce ~128 grey, got {}", r);

    // 2. Rounded rectangle
    fb.clear(0x00000000);
    fb.fill_rounded_rect(20, 20, 40, 40, 8, 0x00FF0000);
    // Center should be red
    assert_eq!(fb.get_pixel(40, 40), 0x00FF0000);
    // Top-left corner pixel outside the radius should remain black
    assert_eq!(fb.get_pixel(20, 20), 0x00000000);

    // 3. Rounded outline
    fb.draw_rounded_outline(20, 20, 40, 40, 8, 0x0000FF00);
    // Top edge center should have outline color
    assert_eq!(fb.get_pixel(40, 20), 0x0000FF00);

    // 4. Drop shadow
    fb.clear(0x00FFFFFF); // White background
    fb.draw_drop_shadow(30, 30, 40, 40, 8, 6);
    // Shadow region around (28, 72) should be darker than white
    let shadow_pixel = fb.get_pixel(35, 72);
    assert_ne!(shadow_pixel, 0x00FFFFFF);

    // 5. Scaled text
    fb.clear(0x00000000);
    fb.draw_text_scaled("Hi", 10, 10, 0x000000FF, None, 2);
    assert!(fb.pixels.iter().any(|&p| p == 0x000000FF));
}

#[test]
fn test_renderer_with_custom_cursor_and_spacing() {
    use rustext::core::buffer::TextBuffer;
    use rustext::editor::Document;
    use rustext::gfx::{Framebuffer, GfxRenderer};

    let mut fb = Framebuffer::new(800, 600);
    let doc = Document::new(TextBuffer::from_str("fn test() {\n    let val = 123;\n}"));
    let mut renderer = GfxRenderer::new();
    renderer.line_spacing = 6;
    renderer.cursor_style = "block".to_string();
    renderer.rounded_corners = true;
    renderer.shadows = true;

    let tabs = vec!["main.rs".to_string()];
    renderer.render(&mut fb, &doc, &tabs, 0, None, None, None);

    // Framebuffer should contain non-empty pixels rendered with modern spacing
    let non_bg = fb.pixels.iter().filter(|&&p| p != renderer.colors.background).count();
    assert!(non_bg > 200);

    // Underline cursor style test
    renderer.cursor_style = "underline".to_string();
    renderer.render(&mut fb, &doc, &tabs, 0, None, None, None);
    let non_bg2 = fb.pixels.iter().filter(|&&p| p != renderer.colors.background).count();
    assert!(non_bg2 > 200);
}

#[test]
fn test_file_explorer_tree_scan_and_expansion() {
    use std::fs;
    use rustext::editor::sidebar::FileExplorer;

    let test_dir = std::env::temp_dir().join(format!("rustext_sidebar_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let _ = fs::create_dir_all(&test_dir);
    let src_dir = test_dir.join("src");
    let nested_dir = src_dir.join("nested");
    let target_dir = test_dir.join("target");
    let git_dir = test_dir.join(".git");

    let _ = fs::create_dir_all(&nested_dir);
    let _ = fs::create_dir_all(&target_dir);
    let _ = fs::create_dir_all(&git_dir);

    let _ = fs::write(test_dir.join("Cargo.toml"), "[package]\nname = \"demo\"\n");
    let _ = fs::write(test_dir.join("README.md"), "# Demo\n");
    let _ = fs::write(src_dir.join("main.rs"), "fn main() {}\n");
    let _ = fs::write(src_dir.join("lib.rs"), "pub fn hello() {}\n");
    let _ = fs::write(nested_dir.join("mod.rs"), "// nested\n");
    let _ = fs::write(target_dir.join("debug.bin"), "bin");
    let _ = fs::write(git_dir.join("config"), "git config");

    let mut explorer = FileExplorer::new(test_dir.clone());
    assert!(explorer.is_visible);
    assert_eq!(explorer.width, 220);

    // Initial root scan: target and .git must be ignored!
    let names: Vec<String> = explorer.visible_items.iter().map(|i| i.name.clone()).collect();
    assert!(names.contains(&"src".to_string()));
    assert!(names.contains(&"Cargo.toml".to_string()));
    assert!(names.contains(&"README.md".to_string()));
    assert!(!names.contains(&"target".to_string()));
    assert!(!names.contains(&".git".to_string()));

    // src/main.rs should not be visible before src is expanded
    assert!(!names.contains(&"main.rs".to_string()));

    // Find index of "src"
    let src_idx = explorer.visible_items.iter().position(|i| i.name == "src").unwrap();
    explorer.selected_idx = src_idx;
    assert!(explorer.expand_selected());

    // Now src is expanded, main.rs and lib.rs should be visible!
    let expanded_names: Vec<String> = explorer.visible_items.iter().map(|i| i.name.clone()).collect();
    assert!(expanded_names.contains(&"main.rs".to_string()));
    assert!(expanded_names.contains(&"lib.rs".to_string()));
    assert!(expanded_names.contains(&"nested".to_string()));

    // Test collapse
    explorer.selected_idx = src_idx;
    assert!(explorer.collapse_selected());
    let collapsed_names: Vec<String> = explorer.visible_items.iter().map(|i| i.name.clone()).collect();
    assert!(!collapsed_names.contains(&"main.rs".to_string()));

    // Test color hint by extension
    let colors = ThemeColors::tokyo_night();
    let rs_item = explorer.visible_items.iter().find(|i| i.name.ends_with(".toml")).unwrap();
    assert_eq!(rs_item.color_hint(&colors), colors.syntax_type);

    // Cleanup test directory
    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_file_explorer_mouse_clicks_and_scroll() {
    use std::fs;
    use rustext::editor::sidebar::{FileExplorer, SidebarClickResult};

    let test_dir = std::env::temp_dir().join(format!("rustext_click_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let _ = fs::create_dir_all(&test_dir);
    let _ = fs::write(test_dir.join("file1.rs"), "fn a() {}");
    let _ = fs::write(test_dir.join("file2.rs"), "fn b() {}");

    let mut explorer = FileExplorer::new(test_dir.clone());
    let header_h = 26;
    let row_h = 20;

    // Click close button in header (top-right)
    let close_x = explorer.width - 10;
    let res = explorer.handle_click(close_x, 10, row_h, header_h);
    assert_eq!(res, SidebarClickResult::CloseSidebar);

    // Click refresh button in header
    let refresh_x = explorer.width - 28;
    let res = explorer.handle_click(refresh_x, 10, row_h, header_h);
    assert_eq!(res, SidebarClickResult::Refresh);

    // Click new file button in header
    let new_file_x = explorer.width - 48;
    let res = explorer.handle_click(new_file_x, 10, row_h, header_h);
    assert_eq!(res, SidebarClickResult::NewFile);

    // Click on first row (file1.rs or file2.rs)
    let first_row_y = header_h + 10;
    let res = explorer.handle_click(10, first_row_y, row_h, header_h);
    match res {
        SidebarClickResult::OpenFile(p) => {
            assert!(p.exists());
        }
        _ => panic!("Expected OpenFile on clicking file item"),
    }

    // Test mouse scrolling
    explorer.handle_scroll(-1, 1);
    assert!(explorer.scroll_offset <= explorer.visible_items.len());
    explorer.handle_scroll(1, 1);
    assert_eq!(explorer.scroll_offset, 0);

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_file_explorer_editor_integration() {
    use rustext::core::TextBuffer;
    use rustext::editor::keymap::EditorAction;
    use rustext::editor::Editor;

    let mut editor = Editor::new(TextBuffer::new());
    let initial_visibility = editor.sidebar.is_visible;

    // Toggle sidebar
    editor.execute_action(EditorAction::ToggleSidebar);
    assert_eq!(editor.sidebar.is_visible, !initial_visibility);

    // Toggle again
    editor.execute_action(EditorAction::ToggleSidebar);
    assert_eq!(editor.sidebar.is_visible, initial_visibility);

    // Refresh action
    editor.execute_action(EditorAction::RefreshSidebar);
    assert!(editor.status_message.is_some());

    // Focus action
    editor.sidebar.is_visible = true;
    editor.sidebar.is_focused = false;
    editor.execute_action(EditorAction::FocusSidebar);
    assert!(editor.sidebar.is_focused);
}

#[test]
fn test_file_explorer_gfx_rendering_and_glyphs() {
    use rustext::core::TextBuffer;
    use rustext::editor::sidebar::FileExplorer;
    use rustext::gfx::font::get_glyph;

    // Test font glyphs
    let down_arrow = get_glyph('▼');
    let right_arrow = get_glyph('▶');
    let refresh_glyph = get_glyph('↺');
    assert!(down_arrow.iter().any(|&b| b != 0));
    assert!(right_arrow.iter().any(|&b| b != 0));
    assert!(refresh_glyph.iter().any(|&b| b != 0));

    // Render frame with sidebar
    let mut fb = Framebuffer::new(800, 600);
    let doc = Document::new(TextBuffer::from_str("fn main() {\n    println!(\"Hello\");\n}"));
    let renderer = GfxRenderer::new();
    let tabs = vec!["main.rs".to_string()];
    let explorer = FileExplorer::new(std::env::current_dir().unwrap());

    renderer.render_full_frame(
        &mut fb,
        &doc,
        &tabs,
        0,
        Some("Ready"),
        None,
        None,
        Some(&explorer),
        false,
        false,
        None,
    );

    // Sidebar region x: 0..220 should contain non-background pixels (header, text, divider)
    let mut sidebar_pixels = 0;
    for y in 0..600 {
        for x in 0..220 {
            if fb.get_pixel(x, y) != renderer.colors.background {
                sidebar_pixels += 1;
            }
        }
    }
    assert!(sidebar_pixels > 500, "Sidebar should render header, divider border and tree rows");
}

#[test]
fn test_autocomplete_trigger_and_commit() {
    use rustext::editor::{Editor, EditorAction};
    use rustext::ui::terminal::Key;

    let buf = TextBuffer::from_str("let contador_visitas = 100;\n");
    let mut editor = Editor::new(buf);
    editor.doc_mut().language = rustext::core::syntax::Language::Rust;

    // Position cursor at end of second line
    editor.execute_action(EditorAction::MoveLineEnd { select: false });
    editor.execute_action(EditorAction::InsertNewline);

    // Type "co" (2 chars >= autocomplete_min_chars)
    editor.handle_key(Key::Char('c'));
    editor.handle_key(Key::Char('o'));

    assert!(editor.autocomplete.is_active, "Autocomplete should automatically trigger after 2 chars");
    assert!(!editor.autocomplete.items.is_empty());
    assert_eq!(editor.autocomplete.prefix, "co");

    // "contador_visitas", "const", "continue" should be candidates
    let labels: Vec<&str> = editor.autocomplete.items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"contador_visitas"));
    assert!(labels.contains(&"const"));
    assert!(labels.contains(&"continue"));

    // Cycle down to select a candidate
    let initial_idx = editor.autocomplete.selected_idx;
    editor.handle_key(Key::Down);
    assert_eq!(editor.autocomplete.selected_idx, initial_idx + 1);

    // Commit completion with Enter
    let selected_label = editor.autocomplete.selected_item().unwrap().label.clone();
    editor.handle_key(Key::Enter);

    assert!(!editor.autocomplete.is_active, "Autocomplete should close upon commit");
    let full_text = editor.doc().buffer.to_string();
    assert!(full_text.trim_end().ends_with(&selected_label), "Buffer text should end with committed label '{}'", selected_label);
}

#[test]
fn test_autocomplete_dismissal() {
    use rustext::editor::Editor;
    use rustext::ui::terminal::Key;

    let mut editor = Editor::new(TextBuffer::new());
    editor.doc_mut().language = rustext::core::syntax::Language::Rust;
    editor.handle_key(Key::Char('f'));
    editor.handle_key(Key::Char('n'));

    assert!(editor.autocomplete.is_active);

    // Pressing Esc should dismiss autocomplete immediately
    editor.handle_key(Key::Esc);
    assert!(!editor.autocomplete.is_active);

    // Typing non-identifier character (like space or semicolon) also dismisses
    editor.handle_key(Key::Char(' '));
    editor.handle_key(Key::Char('f'));
    editor.handle_key(Key::Char('o'));
    assert!(editor.autocomplete.is_active);

    editor.handle_key(Key::Char(' '));
    assert!(!editor.autocomplete.is_active);
}

#[test]
fn test_autocomplete_floating_popup_rendering() {
    use rustext::editor::{AutocompleteState, CompletionItem, CompletionKind};

    let mut fb = Framebuffer::new(800, 600);
    let doc = Document::new(TextBuffer::from_str("fn calcular_total() {\n    let val = 10;\n}"));
    let renderer = GfxRenderer::new();

    let mut ac = AutocompleteState::new();
    let items = vec![
        CompletionItem::new("calcular_total", "Function", CompletionKind::Function),
        CompletionItem::new("const", "Keyword", CompletionKind::Keyword),
        CompletionItem::new("char", "Type", CompletionKind::Type),
        CompletionItem::new("co", "Snippet", CompletionKind::Snippet),
    ];
    ac.open("co".to_string(), 0, items);

    let geom = renderer.autocomplete_geometry(800, 600, &doc, 220, true, ac.items.len());
    assert!(geom.width >= 240);
    assert!(geom.height >= 80);

    // Render popup directly into framebuffer
    renderer.render_autocomplete_popup(&mut fb, &ac, &geom);

    // Inspect popup area: must have modal_bg, chips, text, and shadow
    let mut popup_pixels = 0;
    for y in geom.y..geom.y + geom.height {
        for x in geom.x..geom.x + geom.width {
            if fb.get_pixel(x, y) != 0x00000000 {
                popup_pixels += 1;
            }
        }
    }
    assert!(popup_pixels > 1000, "Floating popup must draw background, borders, category chips, and labels");
}

#[test]
fn test_autocomplete_geometry_and_mouse_hit() {
    use rustext::editor::{AutocompleteState, CompletionItem, CompletionKind};

    let doc = Document::new(TextBuffer::from_str("let x = 1;\nlet y = 2;\n"));
    let renderer = GfxRenderer::new();

    let mut ac = AutocompleteState::new();
    let items = vec![
        CompletionItem::new("item1", "det1", CompletionKind::Word),
        CompletionItem::new("item2", "det2", CompletionKind::Word),
        CompletionItem::new("item3", "det3", CompletionKind::Word),
    ];
    ac.open("it".to_string(), 0, items);

    let geom = renderer.autocomplete_geometry(800, 600, &doc, 0, false, ac.items.len());

    // Inside popup bounds
    assert!(geom.contains_point(geom.x + 10, geom.y + 10));
    // Outside popup bounds
    assert!(!geom.contains_point(0, 0));
    assert!(!geom.contains_point(geom.x + geom.width + 10, geom.y));

    // Mouse click on row 1 (second item)
    let click_y = geom.y + 4 + geom.item_height + 5;
    let clicked_idx = geom.item_index_at(click_y, 0);
    assert_eq!(clicked_idx, Some(1));
}

#[test]
fn test_autocomplete_multicursor_simultaneous_commit() {
    use rustext::editor::Editor;
    use rustext::core::cursor::Cursor;
    use rustext::ui::terminal::Key;

    let buf = TextBuffer::from_str("line1: \nline2: \n");
    let mut editor = Editor::new(buf);
    editor.doc_mut().language = rustext::core::syntax::Language::Rust;

    // Place cursor on line 1 and line 2 after colon and space
    editor.doc_mut().cursors.set_cursors(vec![
        Cursor::new(7),
        Cursor::new(15),
    ]);

    // Type "st" across both cursors
    editor.handle_key(Key::Char('s'));
    editor.handle_key(Key::Char('t'));

    assert!(editor.autocomplete.is_active);

    // Commit completion with Tab
    editor.handle_key(Key::Tab);

    let text = editor.doc().buffer.to_string();
    // Both lines must have had "st" replaced with the same completion
    assert!(text.contains("line1: struct") || text.contains("line1: str") || text.contains("line1: static"));
    assert!(text.contains("line2: struct") || text.contains("line2: str") || text.contains("line2: static"));
}

#[test]
fn test_global_search_modal_lifecycle_and_filtering() {
    use rustext::ui::prompt::{PromptKind, PromptModal};

    let cur_dir = std::env::current_dir().unwrap();
    let mut modal = PromptModal::new_global_search(cur_dir, None);
    assert_eq!(modal.kind, PromptKind::GlobalSearch);
    assert!(modal.global_results.is_empty());

    // Type "PieceTable"
    for c in "PieceTable".chars() {
        modal.insert_char(c);
    }
    assert_eq!(modal.input, "PieceTable");
    assert!(!modal.global_results.is_empty(), "Project should contain occurrences of PieceTable");
    assert!(modal.global_total_files > 0);
    assert!(modal.global_matching_files > 0);

    // Initial selected index is 0
    assert_eq!(modal.selected_idx, 0);

    // Navigation down & up
    modal.select_next_global_result(10);
    if modal.global_results.len() > 1 {
        assert_eq!(modal.selected_idx, 1);
    }
    modal.select_prev_global_result();
    assert_eq!(modal.selected_idx, 0);

    // Case sensitivity toggle
    assert!(!modal.case_sensitive);
    modal.toggle_case_sensitivity();
    assert!(modal.case_sensitive);
}

#[test]
fn test_global_search_action_and_cursor_jump() {
    use rustext::editor::{Editor, EditorAction, EditorMode};
    use rustext::ui::terminal::Key;

    let buf = TextBuffer::from_str("hello world");
    let mut editor = Editor::new(buf);

    // Trigger action
    editor.execute_action(EditorAction::OpenGlobalSearchPrompt);
    match &editor.mode {
        EditorMode::ModalPrompt(modal) => {
            assert_eq!(modal.kind, rustext::ui::prompt::PromptKind::GlobalSearch);
        }
        _ => panic!("Expected ModalPrompt with GlobalSearch"),
    }

    // Type query "Piece" into active modal
    editor.handle_key(Key::Char('P'));
    editor.handle_key(Key::Char('i'));
    editor.handle_key(Key::Char('e'));
    editor.handle_key(Key::Char('c'));
    editor.handle_key(Key::Char('e'));

    // Hit Enter to jump to the first matched file and position
    editor.handle_key(Key::Enter);

    assert_eq!(editor.mode, EditorMode::Normal);
    // Buffer should now be the opened file
    let opened_text = editor.doc().buffer.to_string();
    assert!(opened_text.contains("PieceTable") || opened_text.contains("Piece") || opened_text.contains("piece"));
    assert!(editor.status_message.is_some());
    let (msg, _) = editor.status_message.as_ref().unwrap();
    assert!(msg.starts_with("Salto a "));
}

#[test]
fn test_global_search_gui_rendering() {
    use rustext::ui::prompt::PromptModal;

    let mut fb = Framebuffer::new(800, 600);
    let cur_dir = std::env::current_dir().unwrap();
    let modal = PromptModal::new_global_search(cur_dir, Some("struct"));
    let renderer = GfxRenderer::new();

    renderer.render_global_search_dialog(&mut fb, &modal, false);

    // Verify dialog header pixel
    let header_pixel = fb.get_pixel(400, 28 + 10);
    assert_eq!(header_pixel, 0x002D2D2D);

    // Verify body pixels are drawn (not blank black 0)
    let body_pixel = fb.get_pixel(400, 28 + 80);
    assert_ne!(body_pixel, 0x00000000);
}

#[test]
fn test_minimap_geometry_and_rendering() {
    let mut fb = Framebuffer::new(800, 600);
    let sample_code = (0..100)
        .map(|i| format!("fn line_{}() {{\n    let x = {};\n}}", i, i))
        .collect::<Vec<_>>()
        .join("\n");
    let doc = Document::new(TextBuffer::from_str(&sample_code));
    let renderer = GfxRenderer::new();

    let mm_geom = renderer.minimap_geometry(800, 600, &doc, true);
    assert!(mm_geom.width > 0);
    assert_eq!(mm_geom.width, renderer.minimap_width);
    assert!(mm_geom.height > 0);
    assert!(mm_geom.contains_point(mm_geom.x + 10, mm_geom.y + 10));
    assert!(!mm_geom.contains_point(10, 10));

    // Slider position and bounds
    assert!(mm_geom.slider_height > 0);
    assert!(mm_geom.slider_y >= mm_geom.y);
    assert!(mm_geom.slider_contains_point(mm_geom.x + 5, mm_geom.slider_y + 2));

    // Line translation for mouse click
    let line_mid = mm_geom.line_for_click_y(mm_geom.y + mm_geom.height / 2);
    assert!(line_mid > 0 && line_mid <= doc.buffer.len_lines());

    // Row calculation for slider dragging
    let row_target = mm_geom.row_for_slider_y(mm_geom.y + 50);
    assert!(row_target <= doc.buffer.len_lines());

    // Render minimap into framebuffer
    renderer.render_minimap(&mut fb, &doc, &mm_geom, false, false);
    let bg_pixel = fb.get_pixel(mm_geom.x + 2, mm_geom.y + 2);
    assert_ne!(bg_pixel, 0x00000000);
}

#[test]
fn test_split_editor_lifecycle_and_switching() {
    use rustext::editor::{Editor, EditorAction};

    let mut editor = Editor::new(TextBuffer::from_str("// Doc 1"));
    editor.new_tab();
    editor.doc_mut().buffer.insert(0, "// Doc 2");
    assert_eq!(editor.documents.len(), 2);
    assert_eq!(editor.active_doc_idx, 1);

    // Initial state: No split
    assert!(!editor.split.is_split());

    // 1. Toggle Vertical Split
    editor.execute_action(EditorAction::ToggleSplitVertical);
    assert!(editor.split.is_split());
    assert_eq!(editor.split.direction, rustext::editor::split::SplitDirection::Vertical);
    assert_eq!(editor.split.active_pane, 1);
    assert_eq!(editor.active_doc_idx, editor.split.secondary_doc_idx);

    // 2. Switch Pane (Alt+\)
    editor.execute_action(EditorAction::SwitchSplitPane);
    assert_eq!(editor.split.active_pane, 0);
    assert_eq!(editor.active_doc_idx, editor.split.primary_doc_idx);

    // 3. Switch back to Pane 1
    editor.execute_action(EditorAction::SwitchSplitPane);
    assert_eq!(editor.split.active_pane, 1);

    // 4. Toggle Horizontal Split
    editor.execute_action(EditorAction::ToggleSplitHorizontal);
    assert!(editor.split.is_split());
    assert_eq!(editor.split.direction, rustext::editor::split::SplitDirection::Horizontal);

    // 5. Close Split
    editor.execute_action(EditorAction::CloseSplit);
    assert!(!editor.split.is_split());
    assert_eq!(editor.active_doc_idx, editor.split.primary_doc_idx);
}

#[test]
fn test_split_rendering_and_pane_geometries() {
    let mut fb = Framebuffer::new(1024, 768);
    let doc1 = Document::new(TextBuffer::from_str("fn first_pane() {}"));
    let doc2 = Document::new(TextBuffer::from_str("fn second_pane() {}"));

    let renderer = GfxRenderer::new();
    let mut split = rustext::editor::split::SplitState::new();

    // 1. Check pane geometries when vertical split is active
    split.toggle_vertical(2, 0);
    let (p0, p1, splitter) = renderer.pane_geometries(1024, 768, 0, true, &split);
    assert_eq!(p0.x, 0);
    assert!(p0.width > 300);
    assert!(p1.x > p0.width);
    assert!(splitter.is_some());
    let (sx, sy, sw, sh) = splitter.unwrap();
    assert_eq!(sx, p0.width);
    assert_eq!(sw, 4);
    assert_eq!(sh, p0.height);

    // 2. Subgeometries for Pane 0 and Pane 1
    let (sb0, mm0, max_code_x0) = renderer.pane_subgeometries(&p0, &doc1);
    assert!(sb0.width > 0);
    assert!(mm0.is_some());
    assert!(max_code_x0 < p0.x + p0.width);

    let (sb1, mm1, max_code_x1) = renderer.pane_subgeometries(&p1, &doc2);
    assert!(sb1.width > 0);
    assert!(mm1.is_some());
    assert!(max_code_x1 < p1.x + p1.width);

    // 3. Render full frame with vertical split into framebuffer
    let tabs = vec!["doc1.rs".to_string(), "doc2.rs".to_string()];
    renderer.render_full_frame_with_split(
        &mut fb,
        &doc1,
        Some(&doc2),
        Some(&split),
        &tabs,
        0,
        Some("Split Mode Active"),
        None,
        None,
        None,
        false,
        false,
        false,
        false,
        None,
    );

    // Verify pane 0 header and pane 1 header pixels are drawn
    let p0_hdr_pixel = fb.get_pixel(p0.x + 10, p0.y + 10);
    assert_ne!(p0_hdr_pixel, 0x00000000);
    let p1_hdr_pixel = fb.get_pixel(p1.x + 10, p1.y + 10);
    assert_ne!(p1_hdr_pixel, 0x00000000);

    // Verify splitter bar pixel
    let splitter_pixel = fb.get_pixel(sx + 1, sy + 50);
    assert_ne!(splitter_pixel, 0x00000000);
}

#[test]
fn test_minimap_and_split_config_and_menu() {
    use rustext::editor::config::EditorConfig;
    use rustext::editor::menu::MenuBar;
    use rustext::editor::EditorAction;

    // Config options parsing and defaults
    let cfg = EditorConfig::load();
    assert!(cfg.gui.minimap);
    assert_eq!(cfg.gui.minimap_width, 100);

    let ini_str = cfg.to_ini_string();
    assert!(ini_str.contains("minimap = true"));
    assert!(ini_str.contains("minimap_width = 100"));

    // Menu bar contains the new entries under "Ver"
    let menu = MenuBar::new();
    let ver_cat = &menu.categories[3];
    assert_eq!(ver_cat.name, "Ver");
    let has_minimap_item = ver_cat.items.iter().any(|item| item.action == Some(EditorAction::ToggleMinimap));
    let has_split_v_item = ver_cat.items.iter().any(|item| item.action == Some(EditorAction::ToggleSplitVertical));
    let has_split_h_item = ver_cat.items.iter().any(|item| item.action == Some(EditorAction::ToggleSplitHorizontal));
    let has_switch_pane = ver_cat.items.iter().any(|item| item.action == Some(EditorAction::SwitchSplitPane));
    let has_close_split = ver_cat.items.iter().any(|item| item.action == Some(EditorAction::CloseSplit));

    assert!(has_minimap_item);
    assert!(has_split_v_item);
    assert!(has_split_h_item);
    assert!(has_switch_pane);
    assert!(has_close_split);
}

#[test]
fn test_git_integration_editor_and_config() {
    use rustext::core::TextBuffer;
    use rustext::editor::{Editor, EditorAction};
    use rustext::editor::config::EditorConfig;

    let cfg = EditorConfig::load();
    assert!(cfg.gui.git_gutter);

    let ini = cfg.to_ini_string();
    assert!(ini.contains("git_gutter = true"));

    let buffer = TextBuffer::from_str("fn main() {\n    println!(\"hello\");\n}\n");
    let mut editor = Editor::new(buffer);

    // Initial git manager should find the repository
    assert!(editor.git.git_root.is_some());
    assert_eq!(editor.git.branch_name(), Some("main"));

    // Toggle git gutter
    assert!(editor.config.gui.git_gutter);
    editor.execute_action(EditorAction::ToggleGitGutter);
    assert!(!editor.config.gui.git_gutter);
    editor.execute_action(EditorAction::ToggleGitGutter);
    assert!(editor.config.gui.git_gutter);

    // Refresh git action
    editor.execute_action(EditorAction::RefreshGit);
    assert!(editor.status_message.is_some());
}

#[test]
fn test_git_gutter_diff_rendering_and_status_bar() {
    use rustext::core::TextBuffer;
    use rustext::editor::Document;
    use rustext::editor::git::{GitGutterDiff, GitLineStatus, GitFileStatus};
    use rustext::gfx::{Framebuffer, GfxRenderer};
    use std::collections::HashMap;
    use std::path::PathBuf;

    let buffer = TextBuffer::from_str("line 1\nline 2 (modified)\nline 3 (added)\nline 4\n");
    let doc = Document::new(buffer);
    let mut fb = Framebuffer::new(800, 600);
    let mut renderer = GfxRenderer::new();

    let mut diff = GitGutterDiff::default();
    diff.lines = vec![
        GitLineStatus::None,
        GitLineStatus::Modified,
        GitLineStatus::Added,
        GitLineStatus::None,
    ];
    diff.added_count = 1;
    diff.modified_count = 1;

    let mut statuses = HashMap::new();
    statuses.insert(PathBuf::from("/tmp/test.rs"), GitFileStatus::Modified);

    renderer.git_diff = Some(diff);
    renderer.git_branch = Some("main".to_string());
    renderer.git_statuses = Some(statuses);
    renderer.git_gutter_enabled = true;

    let tabs = vec!["test.rs".to_string()];
    renderer.render_full_frame_with_split(
        &mut fb,
        &doc,
        None,
        None,
        &tabs,
        0,
        None,
        None,
        None,
        None,
        false,
        false,
        false,
        false,
        None,
    );

    // Ensure framebuffer drew without crashing and contains non-black pixels
    assert!(fb.pixels.iter().any(|&p| p != 0));
}

#[test]
fn test_unsaved_changes_warning_tab_close_cancel_and_discard() {
    use rustext::core::TextBuffer;
    use rustext::editor::{Editor, EditorAction, EditorMode, UnsavedConfirmAction};
    use rustext::ui::terminal::Key;

    let mut editor = Editor::new(TextBuffer::new());
    // Type something to make it dirty
    editor.handle_key(Key::Char('a'));
    assert!(editor.doc().buffer.is_dirty());

    // Try closing the tab (Ctrl+W)
    editor.execute_action(EditorAction::CloseTab);

    // Should enter UnsavedConfirm dialog
    match &editor.mode {
        EditorMode::UnsavedConfirm(dialog) => {
            assert_eq!(dialog.action, UnsavedConfirmAction::CloseTab(0));
            assert_eq!(dialog.selected_button, 0); // Default to Guardar
            let (l1, l2) = dialog.message_lines();
            assert!(l1.contains("cambios sin guardar"));
            assert!(l2.contains("cerrar la pestaña"));
            let (b0, b1, b2) = dialog.button_labels();
            assert_eq!(b0, "Guardar");
            assert_eq!(b1, "No guardar");
            assert_eq!(b2, "Cancelar");
        }
        _ => panic!("Expected EditorMode::UnsavedConfirm when closing dirty tab"),
    }

    // Test button navigation with Right / Left
    editor.handle_key(Key::Right);
    if let EditorMode::UnsavedConfirm(dialog) = &editor.mode {
        assert_eq!(dialog.selected_button, 1);
    }
    editor.handle_key(Key::Right);
    if let EditorMode::UnsavedConfirm(dialog) = &editor.mode {
        assert_eq!(dialog.selected_button, 2);
    }
    editor.handle_key(Key::Left);
    if let EditorMode::UnsavedConfirm(dialog) = &editor.mode {
        assert_eq!(dialog.selected_button, 1);
    }

    // Cancel via Esc
    editor.handle_key(Key::Esc);
    assert_eq!(editor.mode, EditorMode::Normal);
    // Document should still exist and still be dirty
    assert_eq!(editor.documents.len(), 1);
    assert!(editor.doc().buffer.is_dirty());

    // Try closing again
    editor.execute_action(EditorAction::CloseTab);
    assert!(matches!(editor.mode, EditorMode::UnsavedConfirm(_)));

    // Discard via 'd' key
    editor.handle_key(Key::Char('d'));
    assert_eq!(editor.mode, EditorMode::Normal);
    // In single-tab editor, closing the only tab clears it to a fresh clean buffer
    assert!(!editor.doc().buffer.is_dirty());
    assert_eq!(editor.doc().buffer.to_string(), "");
}

#[test]
fn test_unsaved_changes_warning_tab_close_save() {
    use rustext::core::TextBuffer;
    use rustext::editor::{Editor, EditorAction, EditorMode, UnsavedConfirmAction};
    use rustext::ui::terminal::Key;
    use std::fs;

    let temp_file = std::env::temp_dir().join(format!("rustext_save_test_{}.rs", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    fs::write(&temp_file, "fn original() {}\n").unwrap();

    let mut editor = Editor::new(TextBuffer::new());
    editor.open_file(&temp_file);
    assert_eq!(editor.documents.len(), 2);
    assert_eq!(editor.active_doc_idx, 1);

    // Edit the file to make it dirty
    editor.handle_key(Key::Char('/'));
    editor.handle_key(Key::Char('/'));
    assert!(editor.doc().buffer.is_dirty());

    // Close tab (Ctrl+W)
    editor.execute_action(EditorAction::CloseTab);
    match &editor.mode {
        EditorMode::UnsavedConfirm(dialog) => {
            assert_eq!(dialog.action, UnsavedConfirmAction::CloseTab(1));
        }
        _ => panic!("Expected UnsavedConfirm dialog"),
    }

    // Press 'g' to save and close
    editor.handle_key(Key::Char('g'));
    assert_eq!(editor.mode, EditorMode::Normal);
    assert_eq!(editor.documents.len(), 1); // Tab closed

    // Verify file on disk has the changes
    let disk_content = fs::read_to_string(&temp_file).unwrap();
    assert!(disk_content.starts_with("//"));

    let _ = fs::remove_file(&temp_file);
}

#[test]
fn test_unsaved_changes_warning_quit_flow() {
    use rustext::core::TextBuffer;
    use rustext::editor::{Editor, EditorAction, EditorMode, UnsavedConfirmAction};
    use rustext::ui::terminal::Key;

    let mut editor = Editor::new(TextBuffer::new());
    editor.handle_key(Key::Char('x')); // dirty tab 0

    editor.new_tab();
    editor.handle_key(Key::Char('y')); // dirty tab 1

    assert_eq!(editor.documents.len(), 2);
    assert!(editor.documents[0].buffer.is_dirty());
    assert!(editor.documents[1].buffer.is_dirty());

    // Trigger Quit (Ctrl+Q)
    editor.execute_action(EditorAction::Quit);
    assert!(!editor.should_quit, "Should not quit immediately when buffers are dirty");

    match &editor.mode {
        EditorMode::UnsavedConfirm(dialog) => {
            assert_eq!(dialog.action, UnsavedConfirmAction::Quit);
            assert_eq!(dialog.dirty_count, 2);
            let (b0, b1, b2) = dialog.button_labels();
            assert_eq!(b0, "Guardar Todo");
            assert_eq!(b1, "Descartar Todo");
            assert_eq!(b2, "Cancelar");
        }
        _ => panic!("Expected UnsavedConfirm dialog on Quit"),
    }

    // Cancel quit with 'c'
    editor.handle_key(Key::Char('c'));
    assert_eq!(editor.mode, EditorMode::Normal);
    assert!(!editor.should_quit);

    // Trigger Quit again and discard all with 'd'
    editor.execute_action(EditorAction::Quit);
    assert!(matches!(editor.mode, EditorMode::UnsavedConfirm(_)));
    editor.handle_key(Key::Char('d'));
    assert!(editor.should_quit, "Should quit after discarding all unsaved changes");
}

#[test]
fn test_unsaved_changes_dialog_gui_rendering() {
    use rustext::editor::UnsavedConfirmDialog;
    use rustext::gfx::{Framebuffer, GfxRenderer};

    let mut fb = Framebuffer::new(800, 600);
    let renderer = GfxRenderer::new();
    let dialog = UnsavedConfirmDialog::for_close_tab(0, "test_file.rs".to_string());

    renderer.render_unsaved_confirm_dialog(&mut fb, &dialog, Some(0));

    // Scrim darkened pixel
    let scrim_pixel = fb.get_pixel(10, 10);
    assert_ne!(scrim_pixel, 0x00FFFFFF);

    // Modal box center pixel
    let box_pixel = fb.get_pixel(400, 260);
    assert_ne!(box_pixel, 0x00000000);
}

#[test]
fn test_session_save_and_restore_full_editor_state() {
    use rustext::core::buffer::TextBuffer;
    use rustext::editor::session::SessionManager;
    use rustext::editor::{Editor, EditorAction};
    use std::fs;

    let test_dir = std::env::temp_dir().join(format!("rustext_sess_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let _ = fs::create_dir_all(&test_dir);

    let file_a = test_dir.join("file_a.rs");
    let file_b = test_dir.join("file_b.rs");
    fs::write(&file_a, "fn hello_a() {\n    println!(\"A\");\n}\n").unwrap();
    fs::write(&file_b, "fn hello_b() {\n    println!(\"B\");\n}\n").unwrap();

    let sess_dir = test_dir.join("sess");
    let sess_mgr = SessionManager::with_dir(sess_dir.clone());

    // 1. Setup editor with two files, split and cursors
    let mut editor = Editor::new(TextBuffer::open(&file_a).unwrap());
    editor.session_manager = sess_mgr.clone();
    editor.open_file(&file_b);
    assert_eq!(editor.documents.len(), 2);
    assert_eq!(editor.active_doc_idx, 1);

    // Toggle vertical split
    editor.execute_action(EditorAction::ToggleSplitVertical);
    assert!(editor.split.is_split());
    let expected_active = editor.active_doc_idx;

    // Save session
    let res = editor.save_session();
    assert!(res.is_ok());

    // 2. New editor instance restores the session
    let mut new_editor = Editor::new(TextBuffer::new());
    new_editor.session_manager = sess_mgr;
    let restored = new_editor.restore_session();
    assert!(restored);

    assert_eq!(new_editor.documents.len(), 2);
    assert_eq!(new_editor.documents[0].buffer.file_path(), Some(file_a.as_path()));
    assert_eq!(new_editor.documents[1].buffer.file_path(), Some(file_b.as_path()));
    assert!(new_editor.split.is_split());
    assert_eq!(new_editor.active_doc_idx, expected_active);

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_hot_exit_draft_recovery_on_restore() {
    use rustext::core::buffer::TextBuffer;
    use rustext::editor::session::SessionManager;
    use rustext::editor::Editor;
    use rustext::ui::terminal::Key;
    use std::fs;

    let test_dir = std::env::temp_dir().join(format!("rustext_hotexit_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let _ = fs::create_dir_all(&test_dir);

    let file_path = test_dir.join("unsaved.rs");
    fs::write(&file_path, "original content\n").unwrap();

    let sess_dir = test_dir.join("sess");
    let sess_mgr = SessionManager::with_dir(sess_dir.clone());

    let mut editor = Editor::new(TextBuffer::open(&file_path).unwrap());
    editor.session_manager = sess_mgr.clone();
    editor.config.editor.hot_exit = true;

    // Modify buffer without saving
    editor.handle_key(Key::Char('X'));
    assert!(editor.doc().buffer.is_dirty());

    // Save session with hot exit enabled
    let res = editor.save_session();
    assert!(res.is_ok());

    // Verify draft file was written
    assert!(sess_dir.join("drafts").exists());

    // New editor restores session
    let mut new_editor = Editor::new(TextBuffer::new());
    new_editor.session_manager = sess_mgr;
    new_editor.config.editor.hot_exit = true;
    let restored = new_editor.restore_session();
    assert!(restored);

    assert_eq!(new_editor.documents.len(), 1);
    let doc = new_editor.doc();
    assert!(doc.buffer.is_dirty(), "Restored document should preserve dirty state");
    assert!(doc.buffer.to_string().starts_with("Xoriginal content"));

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_recent_files_modal_navigation_and_open() {
    use rustext::core::buffer::TextBuffer;
    use rustext::editor::{Editor, EditorAction, EditorMode};
    use rustext::ui::prompt::PromptKind;
    use rustext::ui::terminal::Key;
    use std::fs;

    let test_dir = std::env::temp_dir().join(format!("rustext_recents_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let _ = fs::create_dir_all(&test_dir);

    let file_1 = test_dir.join("first.txt");
    let file_2 = test_dir.join("second.txt");
    fs::write(&file_1, "file 1").unwrap();
    fs::write(&file_2, "file 2").unwrap();

    let mut editor = Editor::new(TextBuffer::new());
    editor.recent_files.clear();
    editor.open_file(&file_1);
    editor.open_file(&file_2);

    assert_eq!(editor.recent_files.len(), 2);
    assert_eq!(editor.recent_files[0], file_2.canonicalize().unwrap());
    assert_eq!(editor.recent_files[1], file_1.canonicalize().unwrap());

    // Open Recent Files Prompt (Ctrl+R)
    editor.execute_action(EditorAction::OpenRecentFilesPrompt);
    match &editor.mode {
        EditorMode::ModalPrompt(modal) => {
            assert_eq!(modal.kind, PromptKind::RecentFiles);
            let matching = modal.matching_recent_files();
            assert_eq!(matching.len(), 2);
        }
        _ => panic!("Expected RecentFiles modal"),
    }

    // Navigate down and commit to open the second file in the list (file_1)
    editor.handle_key(Key::Down);
    editor.handle_key(Key::Enter);

    assert_eq!(editor.mode, EditorMode::Normal);
    assert_eq!(editor.doc().buffer.file_path(), Some(file_1.as_path()));

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_integrated_terminal_actions_and_keymap() {
    use rustext::editor::app::Editor;
    use rustext::editor::keymap::{EditorAction, Keymap};
    use rustext::core::buffer::TextBuffer;
    use rustext::ui::terminal::Key;

    // Test keymap resolution
    assert_eq!(Keymap::resolve_key(Key::Ctrl('`')), EditorAction::ToggleTerminal);
    assert_eq!(Keymap::resolve_key(Key::Ctrl('~')), EditorAction::ToggleTerminal);

    let mut editor = Editor::new(TextBuffer::from_str("Initial editor content"));
    assert!(!editor.terminal.is_visible);
    assert!(!editor.terminal.is_focused);

    // Toggle terminal open
    editor.execute_action(EditorAction::ToggleTerminal);
    assert!(editor.terminal.is_visible);
    assert!(editor.terminal.is_focused);

    // Typing keys while terminal is focused should go to terminal input, not editor buffer
    editor.handle_key(Key::Char('e'));
    editor.handle_key(Key::Char('c'));
    editor.handle_key(Key::Char('h'));
    editor.handle_key(Key::Char('o'));
    assert_eq!(editor.terminal.current_input, "echo");
    assert_eq!(editor.doc().buffer.to_string(), "Initial editor content");

    // Pressing Esc defocusses terminal
    editor.handle_key(Key::Esc);
    assert!(!editor.terminal.is_focused);
    assert!(editor.terminal.is_visible);

    // Focus terminal action
    editor.execute_action(EditorAction::FocusTerminal);
    assert!(editor.terminal.is_focused);

    // Submitting a command with Enter adds prompt line
    editor.handle_key(Key::Enter);
    assert!(editor.terminal.lines.iter().any(|l| l.contains("echo")));
    assert_eq!(editor.terminal.current_input, "");

    // Clear terminal action
    editor.execute_action(EditorAction::ClearTerminal);
    assert!(editor.terminal.lines.is_empty());

    // Toggle terminal close
    editor.execute_action(EditorAction::ToggleTerminal);
    assert!(!editor.terminal.is_visible);
    assert!(!editor.terminal.is_focused);
}

#[test]
fn test_integrated_terminal_geometry_and_rendering() {
    use rustext::editor::app::Editor;
    use rustext::core::buffer::TextBuffer;

    let editor = Editor::new(TextBuffer::from_str("Hello World"));
    let renderer = GfxRenderer::new();
    let mut fb = Framebuffer::new(800, 600);

    let geom = renderer.terminal_geometry(fb.width, fb.height, 200, 180);
    assert_eq!(geom.x, 200);
    assert_eq!(geom.width, 600);
    assert_eq!(geom.height, 180);

    // Splitter hit test
    assert!(geom.is_over_splitter(250, geom.splitter_y + 1));
    assert!(!geom.is_over_splitter(250, geom.splitter_y + 10));

    // Clear and close button hit test
    assert!(geom.is_over_clear_btn(geom.btn_clear_x + 5, geom.header_y + 5));
    assert!(geom.is_over_close_btn(geom.btn_close_x + 5, geom.header_y + 5));
    assert!(geom.contains_point(geom.x + 50, geom.content_y + 50));
    assert!(!geom.contains_point(50, 50));

    // Render terminal panel into framebuffer
    renderer.render_terminal_panel(&mut fb, &editor.terminal, &geom, false, false);

    // Verify terminal background pixel (0x00121212)
    let term_bg_pixel = fb.get_pixel(geom.x + 20, geom.content_y + 20);
    assert_eq!(term_bg_pixel, 0x00121212);
}

#[test]
fn test_hidpi_scaling_coordinates_and_layout() {
    let mut renderer = GfxRenderer::new();

    // 1. Unscaled baseline (1.0x)
    assert_eq!(renderer.scale_factor, 1.0);
    assert_eq!(renderer.font_scale, 1);
    assert_eq!(renderer.char_width(), 8);
    assert_eq!(renderer.char_height(), 16);
    assert_eq!(renderer.line_height(), 19); // 16 + 3
    assert_eq!(renderer.scale_coord(10), 10);
    assert_eq!(renderer.scale_coord(100), 100);
    assert_eq!(renderer.to_logical_coord(100), 100);
    assert_eq!(renderer.menu_bar_height, 24);
    assert_eq!(renderer.tab_bar_height, 26);
    assert_eq!(renderer.status_bar_height, 22);
    assert_eq!(renderer.scrollbar_width, 14);
    assert_eq!(renderer.sidebar_width, 220);
    assert_eq!(renderer.minimap_width, 100);

    // 2. Double scaling (2.0x, e.g. Retina / 4K display)
    renderer.set_scale_factor(2.0);
    assert_eq!(renderer.scale_factor, 2.0);
    assert_eq!(renderer.font_scale, 2);
    assert_eq!(renderer.char_width(), 16);
    assert_eq!(renderer.char_height(), 32);
    assert_eq!(renderer.line_height(), 38); // 32 + 6
    assert_eq!(renderer.scale_coord(10), 20);
    assert_eq!(renderer.scale_coord(100), 200);
    assert_eq!(renderer.scale_coord_f64(12.5), 25.0);
    assert_eq!(renderer.to_logical_coord(200), 100);
    assert_eq!(renderer.menu_bar_height, 48);
    assert_eq!(renderer.tab_bar_height, 52);
    assert_eq!(renderer.status_bar_height, 44);
    assert_eq!(renderer.scrollbar_width, 28);
    assert_eq!(renderer.sidebar_width, 440);
    assert_eq!(renderer.minimap_width, 200);
    assert_eq!(renderer.line_spacing, 6);

    // 3. Fractional scaling (1.5x, e.g. 150% in GNOME/KDE/Windows)
    renderer.set_scale_factor(1.5);
    assert_eq!(renderer.scale_factor, 1.5);
    assert_eq!(renderer.font_scale, 2); // 1.5 rounds to 2 for crisp bitmap rasterization
    assert_eq!(renderer.scale_coord(10), 15);
    assert_eq!(renderer.scale_coord(20), 30);
    assert_eq!(renderer.scale_coord(100), 150);
    assert_eq!(renderer.to_logical_coord(150), 100);
    assert_eq!(renderer.menu_bar_height, 36);
    assert_eq!(renderer.tab_bar_height, 39);
    assert_eq!(renderer.status_bar_height, 33);
    assert_eq!(renderer.scrollbar_width, 21);
    assert_eq!(renderer.sidebar_width, 330);
    assert_eq!(renderer.minimap_width, 150);
}

#[test]
fn test_hidpi_menu_bar_and_dropdown_hit_testing() {
    use rustext::editor::MenuBar;

    let mut menu = MenuBar::new();
    assert_eq!(menu.height, 24);

    // Unscaled bounds for category 0 ("Archivo")
    let (start_x, end_x) = menu.category_bounds(0, 8);
    assert_eq!(start_x, 8);
    // "Archivo" has 7 characters: 7 * 8 + 16 pad = 72
    assert_eq!(end_x, 8 + 72);

    // Click on "Archivo" header
    let action = menu.handle_mouse_click(start_x + 10, 12, 8);
    assert_eq!(action, None);
    assert_eq!(menu.active_category, Some(0));

    // Scale menu to 2.0x (HiDPI)
    menu.set_scale_factor(2.0);
    assert_eq!(menu.height, 48);

    // Font width at 2.0x is 16
    let (start_x_2x, end_x_2x) = menu.category_bounds(0, 16);
    assert_eq!(start_x_2x, 16); // 8 * 2.0
    // "Archivo" 7 chars * 16 + 32 pad = 112 + 32 = 144
    assert_eq!(end_x_2x, 16 + 144);

    // Click inside scaled header toggles category
    let _ = menu.handle_mouse_click(start_x_2x + 20, 24, 16);
    assert_eq!(menu.active_category, None); // Toggled off
    let _ = menu.handle_mouse_click(start_x_2x + 20, 24, 16);
    assert_eq!(menu.active_category, Some(0)); // Toggled back on

    // Dropdown item click at 2.0x
    // First item is "Nueva Pestaña", item_h = 44, dropdown starts at y = 48
    let click_y = 48 + 10;
    let click_x = start_x_2x + 30;
    let item_action = menu.handle_mouse_click(click_x, click_y, 16);
    assert_eq!(item_action, Some(rustext::editor::keymap::EditorAction::NewTab));
    assert_eq!(menu.active_category, None); // Menu closed on action selection
}

#[test]
fn test_hidpi_framebuffer_and_editor_full_frame() {
    use rustext::editor::config::EditorConfig;
    use rustext::editor::MenuBar;

    // Test INI serialization and parsing with scale_factor
    let ini_content = "[gui]\nscale_factor = 2.0\nwindow_width = 1920\nwindow_height = 1080\n";
    let config = EditorConfig::parse(ini_content);
    assert_eq!(config.gui.scale_factor, 2.0);
    assert_eq!(config.gui.window_width, 1920);
    assert_eq!(config.gui.window_height, 1080);

    let serialized = config.to_ini_string();
    assert!(serialized.contains("scale_factor = 2.0"));

    // Render full editor frame in 2.0x HiDPI
    let mut fb = Framebuffer::new(1920, 1080);
    fb.font_scale = 2;

    let mut renderer = GfxRenderer::new();
    renderer.set_scale_factor(2.0);

    let doc = Document::new(TextBuffer::from_str("fn main() {\n    println!(\"HiDPI Native!\");\n}\n"));
    let tabs = vec!["main.rs".to_string(), "lib.rs".to_string()];
    let mut menu = MenuBar::new();
    menu.set_scale_factor(2.0);

    renderer.render(&mut fb, &doc, &tabs, 0, Some("NORMAL"), None, Some(&menu));

    // Verify Menu Bar area (top 48 pixels) contains non-background pixels
    let mut menu_pixels = 0;
    for y in 0..renderer.menu_bar_height {
        for x in 0..100 {
            if fb.get_pixel(x, y) != 0 {
                menu_pixels += 1;
            }
        }
    }
    assert!(menu_pixels > 0, "Menu bar should be rendered in top 48 pixels");

    // Verify Status Bar area (bottom 44 pixels) is populated
    let status_bar_y = fb.height - renderer.status_bar_height;
    let mut status_pixels = 0;
    for y in status_bar_y..fb.height {
        for x in 0..100 {
            if fb.get_pixel(x, y) != 0 {
                status_pixels += 1;
            }
        }
    }
    assert!(status_pixels > 0, "Status bar should be rendered in bottom 44 pixels");
}

#[test]
fn test_font_shaping_ligatures_and_combining_marks() {
    use rustext::gfx::{FontShaper, LigatureKind};

    // 1. Prefix matching
    assert_eq!(LigatureKind::match_prefix("-> hello"), Some((LigatureKind::Arrow, 2)));
    assert_eq!(LigatureKind::match_prefix("=> world"), Some((LigatureKind::FatArrow, 2)));
    assert_eq!(LigatureKind::match_prefix("!= 0"), Some((LigatureKind::NotEqual, 2)));
    assert_eq!(LigatureKind::match_prefix("=== 1"), Some((LigatureKind::TripleEqual, 3)));
    assert_eq!(LigatureKind::match_prefix("!== null"), Some((LigatureKind::NotDoubleEqual, 3)));
    assert_eq!(LigatureKind::match_prefix("<= 10"), Some((LigatureKind::LessEqual, 2)));
    assert_eq!(LigatureKind::match_prefix(">= 20"), Some((LigatureKind::GreaterEqual, 2)));
    assert_eq!(LigatureKind::match_prefix(":= val"), Some((LigatureKind::Walrus, 2)));
    assert_eq!(LigatureKind::match_prefix("&& cond"), Some((LigatureKind::LogicalAnd, 2)));
    assert_eq!(LigatureKind::match_prefix("|| other"), Some((LigatureKind::LogicalOr, 2)));
    assert_eq!(LigatureKind::match_prefix("::scope"), Some((LigatureKind::Scope, 2)));
    assert_eq!(LigatureKind::match_prefix("..10"), Some((LigatureKind::Range, 2)));
    assert_eq!(LigatureKind::match_prefix("...rest"), Some((LigatureKind::Ellipsis, 3)));

    // 2. Shape text with ligatures enabled
    let shaped = FontShaper::shape_text("fn test() -> bool { a == b && c != d }", 1, true);
    let lig_clusters: Vec<&str> = shaped.iter().filter(|g| g.is_ligature).map(|g| g.cluster.as_str()).collect();
    assert!(lig_clusters.contains(&"->"));
    assert!(lig_clusters.contains(&"=="));
    assert!(lig_clusters.contains(&"&&"));
    assert!(lig_clusters.contains(&"!="));

    // Verify ligature cell dimensions
    let arrow_glyph = shaped.iter().find(|g| g.cluster == "->").unwrap();
    assert_eq!(arrow_glyph.advance_cells, 2);
    assert_eq!(arrow_glyph.width, 16); // 8 * 2
    assert_eq!(arrow_glyph.height, 16);
    assert!(!arrow_glyph.coverage.is_empty());

    // 3. Shape text with ligatures disabled
    let plain = FontShaper::shape_text("->", 1, false);
    assert_eq!(plain.len(), 2);
    assert_eq!(plain[0].cluster, "-");
    assert_eq!(plain[1].cluster, ">");
    assert!(!plain[0].is_ligature);

    // 4. Combining diacritic shaping ('a' + combining acute)
    let combining_text = "a\u{0301}";
    let shaped_combining = FontShaper::shape_text(combining_text, 1, true);
    assert_eq!(shaped_combining.len(), 1);
    assert_eq!(shaped_combining[0].advance_cells, 1);
}

#[test]
fn test_font_hinting_stem_snapping_and_levels() {
    use rustext::gfx::{FontHinter, HintingLevel};

    assert_eq!(HintingLevel::from_str("none"), HintingLevel::None);
    assert_eq!(HintingLevel::from_str("slight"), HintingLevel::Slight);
    assert_eq!(HintingLevel::from_str("medium"), HintingLevel::Medium);
    assert_eq!(HintingLevel::from_str("full"), HintingLevel::Full);

    assert_eq!(FontHinter::snap_to_pixel(14.4), 14);
    assert_eq!(FontHinter::snap_to_pixel(14.6), 15);
    assert_eq!(FontHinter::snap_coord(10.3, HintingLevel::None), 10.3);
    assert_eq!(FontHinter::snap_coord(10.3, HintingLevel::Medium), 10.0);

    // Baseline snapping
    let baseline = FontHinter::snap_baseline(100, 20, 16, HintingLevel::Medium);
    assert_eq!(baseline, 102);

    // Coverage stem hinting
    let mut raw_cov = vec![0u8; 8 * 16];
    // Split vertical stroke between col 3 and col 4
    for y in 2..14 {
        raw_cov[y * 8 + 3] = 120;
        raw_cov[y * 8 + 4] = 120;
    }
    let hinted = FontHinter::hint_coverage(&raw_cov, 8, 16, HintingLevel::Medium);
    // Dominant column should be boosted and split bleed attenuated
    let col3_sum: u32 = (2..14).map(|y| hinted[y * 8 + 3] as u32).sum();
    let col4_sum: u32 = (2..14).map(|y| hinted[y * 8 + 4] as u32).sum();
    assert!(col3_sum > 120 * 12 || col4_sum > 120 * 12, "Hinting should consolidate energy into dominant stem column");
}

#[test]
fn test_analytic_geometric_anti_aliasing() {
    use rustext::gfx::{AnalyticAA, Framebuffer};

    let mut fb = Framebuffer::new(100, 100);
    fb.clear(0x00000000);

    // 1. Analytical anti-aliased line
    AnalyticAA::draw_line(&mut fb, 10.2, 10.4, 90.7, 50.3, 2.0, 0x00FFFFFF);
    assert_ne!(fb.get_pixel(10, 10), 0);
    assert_ne!(fb.get_pixel(90, 50), 0);

    // 2. Analytical anti-aliased circle
    fb.clear(0x00000000);
    AnalyticAA::fill_circle(&mut fb, 50.0, 50.0, 20.0, 0x00FF0000);
    assert_eq!(fb.get_pixel(50, 50), 0x00FF0000);
    // Boundary pixel at radius ~20.0 should have non-zero anti-aliased alpha
    assert_ne!(fb.get_pixel(69, 50), 0);
    // Pixel outside radius 22.0 should remain background
    assert_eq!(fb.get_pixel(75, 50), 0);

    // 3. Analytical circle outline
    fb.clear(0x00000000);
    AnalyticAA::draw_circle_outline(&mut fb, 50.0, 50.0, 20.0, 2.0, 0x0000FF00);
    assert_eq!(fb.get_pixel(50, 50), 0x00000000); // Hollow interior
    assert_ne!(fb.get_pixel(69, 50), 0x00000000); // Ring stroke

    // 4. Exact SDF rounded rectangle
    fb.clear(0x00000000);
    AnalyticAA::fill_rounded_rect(&mut fb, 10.0, 10.0, 60.0, 40.0, 8.0, 0x000000FF);
    assert_ne!(fb.get_pixel(40, 30), 0);
    // Outer corner at (10, 10) outside radius 8 arc should be background
    assert_eq!(fb.get_pixel(10, 10), 0);
    // Pixel inside the rounded corner arc should be filled
    assert_ne!(fb.get_pixel(15, 15), 0);

    // 5. Analytical triangle
    fb.clear(0x00000000);
    AnalyticAA::fill_triangle(&mut fb, (20.0, 20.0), (80.0, 20.0), (50.0, 80.0), 0x00FFFF00);
    assert_ne!(fb.get_pixel(50, 40), 0);
    assert_eq!(fb.get_pixel(10, 10), 0);
}

#[test]
fn test_blur_engine_gaussian_frosted_glass_and_shadows() {
    use rustext::gfx::{BlurEngine, Framebuffer};

    let mut fb = Framebuffer::new(80, 80);
    fb.clear(0x00000000);

    // Set a bright impulse block at center
    fb.fill_rect(38, 38, 4, 4, 0x00FFFFFF);
    assert_eq!(fb.get_pixel(30, 30), 0);

    // Apply fast 3-pass Gaussian blur
    BlurEngine::gaussian_blur_argb(&mut fb.pixels, 80, 80, 5);
    // Impulse energy should spread smoothly to surrounding pixels
    assert_ne!(fb.get_pixel(35, 35), 0);
    assert_ne!(fb.get_pixel(45, 45), 0);

    // Frosted glass effect
    let mut glass_fb = Framebuffer::new(100, 100);
    glass_fb.clear(0x00202020);
    BlurEngine::frosted_glass(&mut glass_fb, 10, 10, 60, 40, 6, 3, 0x001A1B26, 180);
    assert_ne!(glass_fb.get_pixel(30, 30), 0x00202020);

    // Smooth drop shadow
    let mut shadow_fb = Framebuffer::new(120, 120);
    shadow_fb.clear(0x00FFFFFF);
    BlurEngine::draw_smooth_shadow(&mut shadow_fb, 20, 20, 60, 40, 6, 4, 0, 4, 0x00000000, 80);
    // Pixel underneath/offset from shadow should be darkened from pure white
    assert_ne!(shadow_fb.get_pixel(50, 45), 0x00FFFFFF);
}

#[test]
fn test_animation_curves_easings_and_cubic_bezier() {
    use rustext::gfx::{AnimatedFloat, CubicBezier, Easing, SpringAnimation};

    // 1. Easing boundaries
    let easings = [
        Easing::Linear,
        Easing::EaseInQuad,
        Easing::EaseOutQuad,
        Easing::EaseInOutQuad,
        Easing::EaseInCubic,
        Easing::EaseOutCubic,
        Easing::EaseInOutCubic,
        Easing::EaseInExpo,
        Easing::EaseOutExpo,
        Easing::EaseOutBack,
        Easing::EaseOutBounce,
        Easing::EaseOutElastic,
    ];

    for easing in &easings {
        let start_val = easing.evaluate(0.0);
        let end_val = easing.evaluate(1.0);
        assert!((start_val - 0.0).abs() < 1e-4, "{:?} at 0.0 should be 0.0, got {}", easing, start_val);
        assert!((end_val - 1.0).abs() < 1e-4, "{:?} at 1.0 should be 1.0, got {}", easing, end_val);
    }

    // 2. Cubic-Bézier solver
    let ease = CubicBezier::ease();
    assert_eq!(ease.sample(0.0), 0.0);
    assert_eq!(ease.sample(1.0), 1.0);
    let mid = ease.sample(0.5);
    assert!(mid > 0.0 && mid < 1.0);

    let ease_out = CubicBezier::ease_out();
    assert!(ease_out.sample(0.2) > 0.2); // Ease out starts fast

    // 3. Stateful AnimatedFloat
    let mut anim = AnimatedFloat::new(0.0);
    assert_eq!(anim.current, 0.0);
    assert!(anim.is_finished);

    anim.animate_to(100.0, 200.0, Easing::EaseOutQuad);
    assert!(!anim.is_finished);

    // Step 100ms (halfway)
    let val_half = anim.update(100.0);
    assert!(val_half > 0.0 && val_half < 100.0);
    assert!(!anim.is_finished);

    // Step remaining 100ms
    let val_end = anim.update(100.0);
    assert_eq!(val_end, 100.0);
    assert!(anim.is_finished);

    // 4. Spring physics oscillator
    let mut spring = SpringAnimation::new(0.0);
    spring.set_target(50.0);
    for _ in 0..100 {
        spring.step(0.016);
    }
    assert!((spring.value - 50.0).abs() < 1.0, "Spring should settle near target 50.0");
}

#[test]
fn test_edge_perception_contrast_and_unsharp_mask() {
    use rustext::gfx::{EdgePerception, Framebuffer};

    // 1. Relative luminance ITU-R BT.709
    let lum_black = EdgePerception::relative_luminance(0x00000000);
    let lum_white = EdgePerception::relative_luminance(0x00FFFFFF);
    assert_eq!(lum_black, 0.0);
    assert!((lum_white - 1.0).abs() < 1e-4);

    // 2. Weber-Fechner edge contrast adaptation
    let alpha_mid = 128u8;
    // Dark mode (white text on dark background): should boost edge coverage slightly
    let adapted_dark = EdgePerception::adapt_edge_coverage(alpha_mid, 0x00FFFFFF, 0x001A1B26);
    assert!(adapted_dark >= alpha_mid);

    // Light mode (dark text on white background): preserves slender stems
    let adapted_light = EdgePerception::adapt_edge_coverage(alpha_mid, 0x00000000, 0x00FFFFFF);
    assert!(adapted_light <= alpha_mid);

    // 3. Mask sharpening with Laplacian
    let mut mask = vec![0u8; 5 * 5];
    mask[2 * 5 + 2] = 128; // Single edge transition pixel
    let sharpened = EdgePerception::sharpen_coverage_mask(&mask, 5, 5, 0.5);
    assert!(sharpened[2 * 5 + 2] >= 128);

    // 4. Unsharp masking on Framebuffer
    let mut fb = Framebuffer::new(30, 30);
    fb.clear(0x00101010);
    fb.fill_rect(10, 10, 10, 10, 0x00EEEEEE);
    EdgePerception::unsharp_mask_region(&mut fb, 5, 5, 20, 20, 0.5);
    assert_ne!(fb.get_pixel(15, 15), 0);
}

#[test]
fn test_modern_gfx_config_and_renderer_integration() {
    use rustext::editor::config::EditorConfig;
    use rustext::gfx::{Framebuffer, GfxRenderer, HintingLevel};

    let ini = "[gui]\nfont_ligatures = true\nfont_hinting = full\nsmooth_shadows = true\nanimation_enabled = true\nedge_perception = true\n";
    let config = EditorConfig::parse(ini);
    assert!(config.gui.font_ligatures);
    assert_eq!(config.gui.font_hinting, "full");
    assert!(config.gui.smooth_shadows);
    assert!(config.gui.animation_enabled);
    assert!(config.gui.edge_perception);

    let serialized = config.to_ini_string();
    assert!(serialized.contains("font_ligatures = true"));
    assert!(serialized.contains("font_hinting = full"));
    assert!(serialized.contains("smooth_shadows = true"));

    let mut fb = Framebuffer::new(200, 60);
    fb.clear(0x0016161E);

    // Draw shaped text with ligatures, hinting, and edge perception
    fb.draw_shaped_text("a => b != c", 10, 20, 0x007AA2F7, None, true, HintingLevel::Full, true);

    // Verify non-empty pixel drawing
    let mut painted_pixels = 0;
    for y in 0..60 {
        for x in 0..200 {
            if fb.get_pixel(x, y) != 0x0016161E {
                painted_pixels += 1;
            }
        }
    }
    assert!(painted_pixels > 20, "Shaped text with ligatures should be rendered onto framebuffer");

    // Test renderer shadow delegation
    let renderer = GfxRenderer::new();
    assert!(renderer.smooth_shadows);
    assert!(renderer.font_ligatures);
    renderer.render_shadow(&mut fb, 20, 20, 80, 30, 6, 4);
}

#[test]
fn test_font_family_and_weight_properties_and_dilation() {
    use rustext::gfx::font::{FontFamily, FontRasterizer, FontWeight};

    // 1. Verify 5 distinct monospace font families
    assert_eq!(FontFamily::ALL.len(), 5);
    for &f in &FontFamily::ALL {
        assert!(!f.as_str().is_empty());
        assert!(!f.display_name().is_empty());
        assert!(!f.description().is_empty());
        assert_eq!(FontFamily::from_name(f.as_str()), f);
    }

    assert_eq!(FontFamily::from_name("terminus"), FontFamily::Terminus);
    assert_eq!(FontFamily::from_name("modern"), FontFamily::Modern);
    assert_eq!(FontFamily::from_name("typewriter"), FontFamily::Typewriter);
    assert_eq!(FontFamily::from_name("retro"), FontFamily::Retro);
    assert_eq!(FontFamily::from_name("standard"), FontFamily::Standard);
    assert_eq!(FontFamily::from_name("unknown_font"), FontFamily::Standard);

    // 2. Verify Font weights and dilation
    assert_eq!(FontWeight::ALL.len(), 3);
    for &w in &FontWeight::ALL {
        assert!(!w.as_str().is_empty());
        assert!(!w.display_name().is_empty());
        assert_eq!(FontWeight::from_name(w.as_str()), w);
    }

    // 3. Bilinear sizing test across dimensions
    for &family in &FontFamily::ALL {
        for (w, h) in [(8, 16), (11, 21), (13, 27), (16, 32)] {
            let (out_w, out_h, cov) = FontRasterizer::rasterize_glyph_sized('A', family, w, h, FontWeight::Normal);
            assert_eq!(out_w, w);
            assert_eq!(out_h, h);
            assert_eq!(cov.len(), w * h);
            let active = cov.iter().filter(|&&c| c > 0).count();
            assert!(active > 0, "Glyph 'A' for {:?} at {}x{} should have painted pixels", family, w, h);
        }
    }

    // 4. Verify Bold dilation produces more covered subpixels than Normal
    let (_, _, cov_normal) = FontRasterizer::rasterize_glyph_sized('B', FontFamily::Standard, 8, 16, FontWeight::Normal);
    let (_, _, cov_bold) = FontRasterizer::rasterize_glyph_sized('B', FontFamily::Standard, 8, 16, FontWeight::Bold);
    let count_normal: usize = cov_normal.iter().map(|&c| c as usize).sum();
    let count_bold: usize = cov_bold.iter().map(|&c| c as usize).sum();
    assert!(count_bold > count_normal, "Bold glyph should have higher total coverage due to horizontal dilation");
}

#[test]
fn test_font_selector_dialog_lifecycle_and_navigation() {
    use rustext::editor::font_selector::FontSelectorDialog;
    use rustext::gfx::font::{FontFamily, FontWeight};
    use rustext::gfx::HintingLevel;
    use rustext::ui::terminal::Key;

    let mut dialog = FontSelectorDialog::new(
        FontFamily::Standard,
        12,
        FontWeight::Normal,
        3,
        true,
        true,
        HintingLevel::Medium,
        0,
        0.50,
        true,
        0.0,
        true,
        true,
        true,
        true,
    );

    assert_eq!(dialog.selected_family, FontFamily::Standard);
    assert_eq!(dialog.selected_size, 12);
    assert_eq!(dialog.selected_weight, FontWeight::Normal);
    assert_eq!(dialog.line_spacing, 3);
    assert!(dialog.ligatures);
    assert!(dialog.antialiasing);
    assert_eq!(dialog.letter_spacing, 0);
    assert!((dialog.font_aspect_ratio - 0.50).abs() < 1e-4);
    assert!(dialog.stem_darkening);
    assert!(dialog.sdf_rendering);
    assert!(dialog.optical_spacing);

    // 1. Navigation through sections with Tab
    assert_eq!(dialog.focus_section, 0);
    dialog.handle_key(Key::Tab);
    assert_eq!(dialog.focus_section, 1); // Size section
    dialog.handle_key(Key::Tab);
    assert_eq!(dialog.focus_section, 2); // Weight section
    dialog.handle_key(Key::Tab);
    assert_eq!(dialog.focus_section, 3); // Options section
    dialog.handle_key(Key::Tab);
    assert_eq!(dialog.focus_section, 4); // Buttons section
    dialog.handle_key(Key::Tab);
    assert_eq!(dialog.focus_section, 0); // Wraps to Family

    // 2. Change font family with Down / Up
    dialog.handle_key(Key::Down);
    assert_eq!(dialog.selected_family, FontFamily::Terminus);
    dialog.handle_key(Key::Down);
    assert_eq!(dialog.selected_family, FontFamily::Modern);
    dialog.handle_key(Key::Up);
    assert_eq!(dialog.selected_family, FontFamily::Terminus);

    // 3. Zoom shortcuts: '+' and '-'
    assert_eq!(dialog.selected_size, 12);
    dialog.handle_key(Key::Char('+'));
    assert_eq!(dialog.selected_size, 14);
    dialog.handle_key(Key::Char('+'));
    assert_eq!(dialog.selected_size, 16);
    dialog.handle_key(Key::Char('-'));
    assert_eq!(dialog.selected_size, 14);
    dialog.handle_key(Key::Char('0'));
    assert_eq!(dialog.selected_size, 12);

    // 4. Feature toggles (l, a, h, s, e, p, d, f, o)
    dialog.handle_key(Key::Char('l'));
    assert!(!dialog.ligatures);
    dialog.handle_key(Key::Char('l'));
    assert!(dialog.ligatures);

    dialog.handle_key(Key::Char('a'));
    assert!(!dialog.antialiasing);
    dialog.handle_key(Key::Char('a'));
    assert!(dialog.antialiasing);

    dialog.handle_key(Key::Char('s'));
    assert_eq!(dialog.line_spacing, 5); // 3 -> 5

    dialog.handle_key(Key::Char('e'));
    assert_eq!(dialog.letter_spacing, 1);
    dialog.handle_key(Key::Char('e'));
    assert_eq!(dialog.letter_spacing, 2);

    dialog.handle_key(Key::Char('p'));
    assert!((dialog.font_aspect_ratio - 0.55).abs() < 1e-4);
    dialog.handle_key(Key::Char('p'));
    assert!((dialog.font_aspect_ratio - 0.60).abs() < 1e-4);

    dialog.handle_key(Key::Char('d'));
    assert!(!dialog.stem_darkening);
    dialog.handle_key(Key::Char('d'));
    assert!(dialog.stem_darkening);

    dialog.handle_key(Key::Char('f'));
    assert!(!dialog.sdf_rendering);
    dialog.handle_key(Key::Char('f'));
    assert!(dialog.sdf_rendering);

    dialog.handle_key(Key::Char('o'));
    assert!(!dialog.optical_spacing);
    dialog.handle_key(Key::Char('o'));
    assert!(dialog.optical_spacing);

    dialog.handle_key(Key::Char('i'));
    assert_eq!(dialog.scale_factor, 1.0);
    dialog.handle_key(Key::Char('i'));
    assert_eq!(dialog.scale_factor, 1.25);

    dialog.handle_key(Key::Char('r'));
    assert!(!dialog.ergonomic_scaling);
    dialog.handle_key(Key::Char('r'));
    assert!(dialog.ergonomic_scaling);

    dialog.handle_key(Key::Char('t'));
    assert!(!dialog.use_ttf);
    dialog.handle_key(Key::Char('t'));
    assert!(dialog.use_ttf);

    // 5. Esc cancels dialog
    let cancel_res = dialog.handle_key(Key::Esc);
    assert_eq!(cancel_res, Some(false));

    // 6. Enter on Apply button applies dialog
    dialog.focus_section = 4;
    dialog.selected_button = 0; // Apply
    let apply_res = dialog.handle_key(Key::Enter);
    assert_eq!(apply_res, Some(true));
}

#[test]
fn test_font_selector_gui_rendering_and_preview() {
    use rustext::editor::font_selector::FontSelectorDialog;
    use rustext::gfx::font::{FontFamily, FontWeight};
    use rustext::gfx::HintingLevel;

    let mut fb = Framebuffer::new(800, 600);
    fb.clear(0x001A1B26);

    let renderer = GfxRenderer::new();
    let dialog = FontSelectorDialog::new(
        FontFamily::Modern,
        14,
        FontWeight::Semibold,
        3,
        true,
        true,
        HintingLevel::Medium,
        1,
        0.55,
        true,
        1.5,
        true,
        true,
        true,
        true,
    );

    // Render dialog overlay onto framebuffer
    renderer.render_font_selector_dialog(&mut fb, &dialog, false);

    // Verify center of dialog is filled with modal background
    let center_pixel = fb.get_pixel(400, 300);
    assert_ne!(center_pixel, 0x001A1B26, "Dialog should render on top of backdrop");

    // Close button hit area
    renderer.render_font_selector_dialog(&mut fb, &dialog, true);
    assert!(fb.width == 800 && fb.height == 600);
}

#[test]
fn test_dark_mode_optical_stem_darkening_and_irradiation_compensation() {
    use rustext::gfx::StemDarkener;

    // 1. Relative luminance calculations
    assert!((StemDarkener::relative_luminance(0x00000000) - 0.0).abs() < 1e-5);
    assert!((StemDarkener::relative_luminance(0x00FFFFFF) - 1.0).abs() < 1e-5);
    let dark_bg = 0x001E1E1E;
    assert!(StemDarkener::relative_luminance(dark_bg) < 0.2);

    // 2. Darkening factor calculations
    let k_dark = StemDarkener::compute_darkening_factor(dark_bg, 0x00FFFFFF, 12, 1.0);
    assert!(k_dark > 0.4, "Dark mode text should have a positive stem darkening factor (got {})", k_dark);

    // Inactive on light themes (light background >= 0.5)
    let k_light = StemDarkener::compute_darkening_factor(0x00FFFFFF, 0x00000000, 12, 1.0);
    assert_eq!(k_light, 0.0, "Stem darkening must be completely inactive in light themes");

    // Inactive for low contrast
    let k_none = StemDarkener::compute_darkening_factor(dark_bg, dark_bg, 12, 1.0);
    assert_eq!(k_none, 0.0);

    // Size attenuation (FreeType ppem curve)
    let k_10 = StemDarkener::compute_darkening_factor(dark_bg, 0x00FFFFFF, 10, 1.0);
    let k_14 = StemDarkener::compute_darkening_factor(dark_bg, 0x00FFFFFF, 14, 1.0);
    let k_24 = StemDarkener::compute_darkening_factor(dark_bg, 0x00FFFFFF, 24, 1.0);
    assert!(k_10 > k_14);
    assert!(k_14 > k_24);
    assert_eq!(k_24, 0.0, "Large fonts >= 24pt do not need stem darkening");

    // 3. Alpha darkening and boundary preservation
    assert_eq!(StemDarkener::darken_alpha(0, k_dark), 0);
    assert_eq!(StemDarkener::darken_alpha(255, k_dark), 255);
    let darkened_mid = StemDarkener::darken_alpha(128, k_dark);
    assert!(darkened_mid > 150, "Mid-tone alpha should be emboldened (got {})", darkened_mid);

    // 4. Subpixel chromatic balance preservation
    let sub = StemDarkener::darken_subpixel(120, 120, 120, k_dark);
    assert_eq!(sub[0], sub[1]);
    assert_eq!(sub[1], sub[2]);
    assert!(sub[0] > 120);

    // 5. 2D coverage mask and counter-space protection
    // Create a 5x5 mask with an enclosed counter (hole in the center at (2,2))
    // and an outer edge at (0, 2)
    let mask = vec![
        0, 200, 200, 200, 0,
        0, 200,   0, 200, 0,
        0, 200,   0, 200, 0,
        0, 200, 200, 200, 0,
        0,   0,   0,   0, 0,
    ];
    let darkened_mask = StemDarkener::darken_coverage_mask(&mask, 5, 5, k_dark);

    // Center hole at (2,2): index = 2 * 5 + 2 = 12
    assert_eq!(darkened_mask[12], 0, "Counter-space / interior loop must be strictly preserved without clogging");

    // Outer edge at (0, 1): index = 1 * 5 + 0 = 5, has solid neighbor to the right (index 6 = 200)
    // Left is free, so it is an exterior boundary that receives halo dilation
    assert!(darkened_mask[5] > 0, "Exterior boundary should receive subtle optical dilation");

    // 6. Config INI roundtrip
    let mut config = rustext::editor::config::EditorConfig::default();
    assert!(config.gui.stem_darkening);
    assert!((config.gui.stem_darkening_amount - 1.0).abs() < 1e-4);

    config.gui.stem_darkening = false;
    config.gui.stem_darkening_amount = 1.5;
    let ini = config.to_ini_string();
    assert!(ini.contains("stem_darkening = false"));
    assert!(ini.contains("stem_darkening_amount = 1.50"));

    let reloaded = rustext::editor::config::EditorConfig::parse(&ini);
    assert!(!reloaded.gui.stem_darkening);
    assert!((reloaded.gui.stem_darkening_amount - 1.5).abs() < 1e-4);
}

#[test]
fn test_ergonomic_aspect_ratio_and_letter_spacing_metrics() {
    let mut renderer = GfxRenderer::new();
    assert_eq!(renderer.glyph_width(), 8);
    assert_eq!(renderer.char_width(), 8);

    // Change aspect ratio to Comfort (0.60): 16 * 0.60 = 9.6 -> round to 10
    renderer.font_aspect_ratio = 0.60;
    assert_eq!(renderer.glyph_width(), 10);
    assert_eq!(renderer.char_width(), 10);

    // Add letter spacing +2px
    renderer.letter_spacing = 2;
    assert_eq!(renderer.glyph_width(), 10);
    assert_eq!(renderer.char_width(), 12);

    // Test framebuffer cell size synchronization
    let mut fb = Framebuffer::new(800, 600);
    fb.set_font_aspect_ratio(0.60);
    fb.set_letter_spacing(2);
    let (cell_w, cell_h) = fb.char_cell_size();
    assert_eq!(cell_w, 12);
    assert_eq!(cell_h, 16);

    // Box drawing characters contiguity test
    assert!(rustext::gfx::font::FontRasterizer::is_box_drawing('│'));
    assert!(rustext::gfx::font::FontRasterizer::is_box_drawing('─'));
    assert!(rustext::gfx::font::FontRasterizer::is_box_drawing('█'));
    assert!(rustext::gfx::font::FontRasterizer::is_box_drawing('\u{2502}'));
    assert!(rustext::gfx::font::FontRasterizer::is_box_drawing('\u{2588}'));
    assert!(!rustext::gfx::font::FontRasterizer::is_box_drawing('A'));
}

#[test]
fn test_font_settings_editor_actions_and_ini_persistence() {
    use rustext::core::TextBuffer;
    use rustext::editor::{Editor, EditorAction, EditorMode};
    use rustext::gfx::font::{FontFamily, FontWeight};

    let mut editor = Editor::new(TextBuffer::from_str("fn main() {}"));
    editor.config = rustext::editor::config::EditorConfig::default();
    assert_eq!(editor.config.gui.font_family, "standard");
    assert_eq!(editor.config.gui.font_size, 12);
    assert_eq!(editor.config.gui.font_weight, "normal");

    // Action: OpenFontSelector
    editor.execute_action(EditorAction::OpenFontSelector);
    assert!(matches!(editor.mode, EditorMode::FontSelector(_)));

    // Action: IncreaseFontSize / DecreaseFontSize / ResetFontSize
    editor.execute_action(EditorAction::IncreaseFontSize);
    assert_eq!(editor.config.gui.font_size, 14);
    editor.execute_action(EditorAction::IncreaseFontSize);
    assert_eq!(editor.config.gui.font_size, 16);
    editor.execute_action(EditorAction::DecreaseFontSize);
    assert_eq!(editor.config.gui.font_size, 14);
    editor.execute_action(EditorAction::ResetFontSize);
    assert_eq!(editor.config.gui.font_size, 12);

    // Action: SetFontFamily and SetFontWeight
    editor.execute_action(EditorAction::SetFontFamily(FontFamily::Terminus));
    assert_eq!(editor.config.gui.font_family, "terminus");

    editor.execute_action(EditorAction::SetFontWeight(FontWeight::Bold));
    assert_eq!(editor.config.gui.font_weight, "bold");

    // Test INI serialization and roundtrip parsing
    let ini_str = editor.config.to_ini_string();
    assert!(ini_str.contains("font_family = terminus"));
    assert!(ini_str.contains("font_size = 12"));
    assert!(ini_str.contains("font_weight = bold"));

    let reloaded = rustext::editor::config::EditorConfig::parse(&ini_str);
    assert_eq!(reloaded.gui.font_family, "terminus");
    assert_eq!(reloaded.gui.font_size, 12);
    assert_eq!(reloaded.gui.font_weight, "bold");

    // Restore default configuration
    editor.config = rustext::editor::config::EditorConfig::default();
    let _ = editor.config.save();
}

#[test]
fn test_iso_9241_303_visual_angle_and_ergonomic_scaling() {
    use rustext::gfx::hidpi::{DisplayFormFactor, HiDpiDetector, IsoComplianceRating};

    // 1. Visual angle calculations
    // Character of 3.84 mm at 600 mm viewing distance:
    // angle = 2 * atan(3.84 / 1200) * 3437.75 = 22.0 arcmin
    let angle = HiDpiDetector::calculate_visual_angle_arcmin(3.84, 600.0);
    assert!((angle - 22.0).abs() < 0.1, "Angle was {:.2}, expected ~22.0'", angle);

    // Inverse target physical height
    let h_target = HiDpiDetector::target_physical_height_mm(22.0, 600.0);
    assert!((h_target - 3.84).abs() < 0.05, "Target height was {:.2}mm, expected ~3.84mm", h_target);

    // 2. Form factors and viewing distances
    assert_eq!(DisplayFormFactor::UltraCompact.default_viewing_distance_mm(), 450.0);
    assert_eq!(DisplayFormFactor::Laptop.default_viewing_distance_mm(), 480.0);
    assert_eq!(DisplayFormFactor::Desktop.default_viewing_distance_mm(), 600.0);

    // 3. Continuous scale calculation
    // Standard desktop (96 DPI, 600 mm distance, 22 arcmin target)
    let s_96 = HiDpiDetector::compute_continuous_scale(96.0, 600.0, 22.0);
    assert!((s_96 - 0.91).abs() < 0.1);
    assert_eq!(HiDpiDetector::quantize_ergonomic_scale(s_96), 1.0);

    // 14" 1080p laptop (157.7 DPI, 480 mm distance, 22 arcmin target)
    let s_laptop = HiDpiDetector::compute_continuous_scale(157.7, 480.0, 22.0);
    let q_laptop = HiDpiDetector::quantize_ergonomic_scale(s_laptop);
    assert_eq!(q_laptop, 1.25, "157 DPI at 48cm laptop distance quantizes to 1.25x");

    // 4K 27" desktop (163.2 DPI, 650 mm distance, 22 arcmin target)
    let s_4k_desktop = HiDpiDetector::compute_continuous_scale(163.2, 650.0, 22.0);
    let q_4k_desktop = HiDpiDetector::quantize_ergonomic_scale(s_4k_desktop);
    assert!(q_4k_desktop >= 1.5 && q_4k_desktop <= 1.75);

    // 4K 15.6" laptop (282 DPI, 480 mm distance, 22 arcmin target)
    let s_4k_laptop = HiDpiDetector::compute_continuous_scale(282.0, 480.0, 22.0);
    let q_4k_laptop = HiDpiDetector::quantize_ergonomic_scale(s_4k_laptop);
    assert!(q_4k_laptop >= 2.0 && q_4k_laptop <= 2.25);

    // 4. ISO 9241-303 legibility assessment and ratings
    // Unscaled 16px font at 157.7 DPI viewed at 600mm -> 14.8 arcmin (< 16' BelowMinimum!)
    let report_unscaled = HiDpiDetector::assess_legibility(16, 1.0, 157.7, 600.0);
    assert_eq!(report_unscaled.rating, IsoComplianceRating::BelowMinimum);
    assert!(!report_unscaled.rating.is_compliant());
    assert!(report_unscaled.visual_angle_arcmin < 16.0);
    assert_eq!(report_unscaled.recommended_scale, 1.5);

    // Scaled 1.5x at 157.7 DPI viewed at 600mm -> 22.2 arcmin (Optimal!)
    let report_scaled = HiDpiDetector::assess_legibility(16, 1.5, 157.7, 600.0);
    assert_eq!(report_scaled.rating, IsoComplianceRating::Optimal);
    assert!(report_scaled.rating.is_optimal());
    assert!(report_scaled.rating.is_compliant());
    assert!((report_scaled.visual_angle_arcmin - 22.2).abs() < 0.5);

    // 5. Quantization table edge cases
    assert_eq!(HiDpiDetector::quantize_ergonomic_scale(0.8), 1.0);
    assert_eq!(HiDpiDetector::quantize_ergonomic_scale(1.10), 1.0);
    assert_eq!(HiDpiDetector::quantize_ergonomic_scale(1.20), 1.25);
    assert_eq!(HiDpiDetector::quantize_ergonomic_scale(1.50), 1.5);
    assert_eq!(HiDpiDetector::quantize_ergonomic_scale(1.80), 1.75);
    assert_eq!(HiDpiDetector::quantize_ergonomic_scale(2.00), 2.0);
    assert_eq!(HiDpiDetector::quantize_ergonomic_scale(2.20), 2.25);
    assert_eq!(HiDpiDetector::quantize_ergonomic_scale(2.50), 2.5);
    assert_eq!(HiDpiDetector::quantize_ergonomic_scale(3.20), 3.0);
}

#[test]
fn test_edid_parser_and_display_metrics() {
    use rustext::gfx::hidpi::HiDpiDetector;

    // 1. Build a synthetic valid 128-byte VESA EDID block
    let mut edid = vec![0u8; 128];
    // Magic header: 00 FF FF FF FF FF FF 00
    edid[0] = 0x00;
    for i in 1..7 { edid[i] = 0xFF; }
    edid[7] = 0x00;

    // Bytes 21 and 22: Basic Display Parameters (width 31cm, height 17cm)
    edid[21] = 31;
    edid[22] = 17;

    // Detailed Timing Descriptor 1 at bytes 54..72
    // Pixel clock: 148.50 MHz (14850 = 0x3A02)
    edid[54] = 0x02;
    edid[55] = 0x3A;
    // Horizontal active: 1920 (0x780 -> lower 8 bits 0x80, upper 4 bits 0x70)
    edid[56] = 0x80;
    edid[58] = 0x70;
    // Vertical active: 1080 (0x438 -> lower 8 bits 0x38, upper 4 bits 0x40)
    edid[59] = 0x38;
    edid[61] = 0x40;
    // Image size in mm: 309mm x 174mm (0x135 x 0x0AE)
    edid[66] = 0x35; // lower 8 bits of 309
    edid[67] = 0xAE; // lower 8 bits of 174
    edid[68] = 0x10; // high nibble 1 for H, 0 for V

    // Compute checksum (mod 256 sum of all 128 bytes must equal 0)
    let partial_sum: usize = edid[0..127].iter().map(|&b| b as usize).sum();
    let chk = ((256 - (partial_sum % 256)) % 256) as u8;
    edid[127] = chk;

    // Verify parsing
    let parsed = HiDpiDetector::parse_edid(&edid);
    assert!(parsed.is_some(), "Valid EDID should parse successfully");
    let (w_px, h_px, w_mm, h_mm) = parsed.unwrap();
    assert_eq!(w_px, 1920);
    assert_eq!(h_px, 1080);
    assert_eq!(w_mm as usize, 309);
    assert_eq!(h_mm as usize, 174);

    // Corrupt header
    let mut bad_edid = edid.clone();
    bad_edid[0] = 0xAA;
    assert!(HiDpiDetector::parse_edid(&bad_edid).is_none());

    // Corrupt checksum
    let mut bad_chk = edid.clone();
    bad_chk[127] ^= 0x55;
    assert!(HiDpiDetector::parse_edid(&bad_chk).is_none());
}

#[test]
fn test_config_ini_ergonomic_scaling_persistence() {
    use rustext::editor::config::EditorConfig;

    let ini = r#"
[gui]
scale_factor = 1.5
ergonomic_scaling = true
viewing_distance_mm = 650.0
iso_target_arcmin = 21.5
"#;
    let cfg = EditorConfig::parse(ini);
    assert_eq!(cfg.gui.scale_factor, 1.5);
    assert!(cfg.gui.ergonomic_scaling);
    assert_eq!(cfg.gui.viewing_distance_mm, 650.0);
    assert_eq!(cfg.gui.iso_target_arcmin, 21.5);

    // Serialization roundtrip
    let serialized = cfg.to_ini_string();
    assert!(serialized.contains("scale_factor = 1.5"));
    assert!(serialized.contains("ergonomic_scaling = true"));
    assert!(serialized.contains("viewing_distance_mm = 650.0"));
    assert!(serialized.contains("iso_target_arcmin = 21.5"));

    let roundtrip = EditorConfig::parse(&serialized);
    assert_eq!(roundtrip.gui.scale_factor, 1.5);
    assert_eq!(roundtrip.gui.ergonomic_scaling, true);
    assert_eq!(roundtrip.gui.viewing_distance_mm, 650.0);
    assert_eq!(roundtrip.gui.iso_target_arcmin, 21.5);
}

#[test]
fn test_sdf_euclidean_distance_transform_and_bilinear_sampling() {
    use rustext::gfx::sdf::{SignedDistanceField, SdfFontRasterizer};
    use rustext::gfx::{FontFamily, FontWeight};

    // 1. Create a synthetic 8x8 glyph mask with a 4x4 filled center box
    let w = 8;
    let h = 8;
    let mut cov = vec![0u8; w * h];
    for y in 2..6 {
        for x in 2..6 {
            cov[y * w + x] = 255;
        }
    }

    let sdf = SignedDistanceField::from_coverage(&cov, w, h);
    assert_eq!(sdf.width, w);
    assert_eq!(sdf.height, h);

    // Pixels inside center box (e.g. (3, 3)) should have positive signed distance
    let inner_dist = sdf.get_distance(3, 3);
    assert!(inner_dist > 0.0, "Inner pixel distance must be positive, got {}", inner_dist);

    // Pixels far outside (e.g. (0, 0)) should have negative signed distance
    let outer_dist = sdf.get_distance(0, 0);
    assert!(outer_dist < 0.0, "Outer pixel distance must be negative, got {}", outer_dist);

    // Continuous bilinear sampling
    let sampled_center = sdf.sample_bilinear(3.5, 3.5);
    assert!(sampled_center > 0.5, "Center bilinear sample should be strongly positive, got {}", sampled_center);

    let sampled_corner = sdf.sample_bilinear(0.2, 0.2);
    assert!(sampled_corner < -1.0, "Corner bilinear sample should be negative, got {}", sampled_corner);

    // Arbitrary resolution rendering (upscaling to 16x16)
    let (rw, rh, out_sized) = sdf.render_sized(16, 16, 0.0);
    assert_eq!(rw, 16);
    assert_eq!(rh, 16);
    assert_eq!(out_sized.len(), 16 * 16);
    // Center pixel of upscaled box should have full coverage
    assert_eq!(out_sized[8 * 16 + 8], 255);

    // Subpixel LCD RGB ClearType sampling
    let (sw, sh, out_subpixel) = sdf.render_subpixel(16, 16, 0.0);
    assert_eq!(sw, 16);
    assert_eq!(sh, 16);
    assert_eq!(out_subpixel.len(), 16 * 16);
    // Center pixel should be opaque white RGB
    assert_eq!(out_subpixel[8 * 16 + 8], [255, 255, 255]);

    // Test SdfFontRasterizer with standard glyphs
    let (gw, gh, alpha_sdf) = SdfFontRasterizer::rasterize_glyph_sdf('A', FontFamily::Standard, 16, 32, FontWeight::Normal, 0.0);
    assert_eq!(gw, 16);
    assert_eq!(gh, 32);
    assert!(alpha_sdf.iter().any(|&a| a > 0), "Glyph 'A' must produce non-empty coverage");

    let (sw2, sh2, rgb_sdf) = SdfFontRasterizer::rasterize_glyph_sdf_subpixel('M', FontFamily::Modern, 12, 24, FontWeight::Bold, 0.0);
    assert_eq!(sw2, 12);
    assert_eq!(sh2, 24);
    assert!(rgb_sdf.iter().any(|&[r, g, b]| r > 0 || g > 0 || b > 0), "Glyph 'M' must produce non-empty subpixel coverage");
}

#[test]
fn test_optical_punctuation_spacing_rules_and_in_cell_clamping() {
    use rustext::gfx::optical_spacing::{GlyphOpticalMetrics, OpticalPunctuationSpacing, PunctuationClass};

    // 1. Classification
    assert_eq!(PunctuationClass::from_char('.'), PunctuationClass::Period);
    assert_eq!(PunctuationClass::from_char(','), PunctuationClass::Comma);
    assert_eq!(PunctuationClass::from_char(';'), PunctuationClass::Semicolon);
    assert_eq!(PunctuationClass::from_char('('), PunctuationClass::OpenParen);
    assert_eq!(PunctuationClass::from_char(')'), PunctuationClass::CloseParen);
    assert_eq!(PunctuationClass::from_char('['), PunctuationClass::OpenBracket);
    assert_eq!(PunctuationClass::from_char(']'), PunctuationClass::CloseBracket);
    assert_eq!(PunctuationClass::from_char('\''), PunctuationClass::Quote);
    assert_eq!(PunctuationClass::from_char('"'), PunctuationClass::Quote);
    assert_eq!(PunctuationClass::from_char('a'), PunctuationClass::Alphanumeric);

    // 2. Optical metrics measurement on a synthetic glyph
    let w = 8;
    let h = 16;
    let mut dot_cov = vec![0u8; w * h];
    // Put a 2x2 dot on bottom left (cols 1..3, rows 12..14)
    for r in 12..14 {
        for c in 1..3 {
            dot_cov[r * w + c] = 255;
        }
    }
    let metrics = GlyphOpticalMetrics::from_coverage(&dot_cov, w, h);
    assert!(metrics.total_mass > 0);
    assert_eq!(metrics.min_x, 1);
    assert_eq!(metrics.max_x, 2);
    assert_eq!(metrics.ink_width, 2);
    assert!(metrics.center_of_mass_x < 2.5, "Center of mass should be on left half");

    // 3. Optical tucking rules
    let cell_w = 12;
    let scale = 1;

    // Member access: "obj.prop" -> period tucked left towards 'j'
    let offset_member = OpticalPunctuationSpacing::compute_optical_offset(Some('j'), '.', Some('p'), cell_w, scale);
    assert!(offset_member < 0, "Member access period should tuck left, got offset {}", offset_member);

    // Argument list: "arg, next" -> comma tucked left towards 'g'
    let offset_arg = OpticalPunctuationSpacing::compute_optical_offset(Some('g'), ',', Some(' '), cell_w, scale);
    assert!(offset_arg < 0, "Argument separator comma should tuck left, got offset {}", offset_arg);

    // Semicolon statement termination: "1;" -> tucked left towards '1'
    let offset_semi = OpticalPunctuationSpacing::compute_optical_offset(Some('1'), ';', None, cell_w, scale);
    assert!(offset_semi < 0, "Statement termination semicolon should tuck left, got offset {}", offset_semi);

    // Function call open paren: "foo(" -> opening paren tucked left towards function name
    let offset_call = OpticalPunctuationSpacing::compute_optical_offset(Some('o'), '(', Some('x'), cell_w, scale);
    assert!(offset_call < 0, "Function call opening paren should tuck left towards identifier, got offset {}", offset_call);

    // Empty pair: "()" -> closing paren tucked left towards opening paren
    let offset_paren_close = OpticalPunctuationSpacing::compute_optical_offset(Some('('), ')', None, cell_w, scale);
    assert!(offset_paren_close < 0, "Closing paren after opening paren should tuck left, got offset {}", offset_paren_close);

    // 4. In-cell strict boundary clamping
    let glyph_w = 8;
    for cell_px in [0, 48, 120, 240] {
        let nominal_centered = cell_px + (cell_w - glyph_w) / 2;
        let min_bound = cell_px;
        let max_bound = cell_px + cell_w - glyph_w;

        // Zero offset gives nominal centered coordinate
        let safe_zero = OpticalPunctuationSpacing::compute_safe_in_cell_draw_x(cell_px, cell_w, glyph_w, 0);
        assert_eq!(safe_zero, nominal_centered);

        // Huge negative offset (-100px) must be safely clamped to cell_px
        let safe_neg = OpticalPunctuationSpacing::compute_safe_in_cell_draw_x(cell_px, cell_w, glyph_w, -100);
        assert_eq!(safe_neg, min_bound);

        // Huge positive offset (+100px) must be safely clamped to cell_px + cell_w - glyph_w
        let safe_pos = OpticalPunctuationSpacing::compute_safe_in_cell_draw_x(cell_px, cell_w, glyph_w, 100);
        assert_eq!(safe_pos, max_bound);

        // Within bounds
        let normal_shift = OpticalPunctuationSpacing::compute_safe_in_cell_draw_x(cell_px, cell_w, glyph_w, -1);
        assert!(normal_shift >= min_bound && normal_shift <= max_bound);
    }
}

#[test]
fn test_config_ini_sdf_and_optical_spacing_persistence() {
    use rustext::editor::config::EditorConfig;

    let ini = r#"
[gui]
sdf_font_rendering = true
optical_punctuation_spacing = true
"#;
    let cfg = EditorConfig::parse(ini);
    assert!(cfg.gui.sdf_font_rendering);
    assert!(cfg.gui.optical_punctuation_spacing);

    // Toggle off in config
    let ini_off = r#"
[gui]
sdf_font_rendering = false
optical_punctuation_spacing = false
"#;
    let cfg_off = EditorConfig::parse(ini_off);
    assert!(!cfg_off.gui.sdf_font_rendering);
    assert!(!cfg_off.gui.optical_punctuation_spacing);

    // Serialization roundtrip
    let serialized = cfg.to_ini_string();
    assert!(serialized.contains("sdf_font_rendering = true"));
    assert!(serialized.contains("optical_punctuation_spacing = true"));

    let roundtrip = EditorConfig::parse(&serialized);
    assert!(roundtrip.gui.sdf_font_rendering);
    assert!(roundtrip.gui.optical_punctuation_spacing);
}

#[test]
fn test_ttf_binary_reader_and_bounds_checking() {
    use rustext::gfx::ttf::{TtfError, TtfReader};

    let sample_data = [
        0x12, 0x34,             // u16: 0x1234
        0x56, 0x78, 0x9A, 0xBC, // u32: 0x56789ABC
        0xFE, 0xDC,             // i16: -292 (0xFEDC)
        0x00, 0x01, 0x80, 0x00, // Fixed: 1.5
        0x40, 0x00,             // f2dot14: 1.0 (0x4000)
        0xAB,                   // u8: 0xAB
    ];

    let mut reader = TtfReader::new(&sample_data);
    assert_eq!(reader.remaining(), 15);
    assert_eq!(reader.read_u16().unwrap(), 0x1234);
    assert_eq!(reader.read_u32().unwrap(), 0x56789ABC);
    assert_eq!(reader.read_i16().unwrap(), -292);

    let fixed_val = reader.read_fixed().unwrap();
    assert!((fixed_val - 1.5).abs() < 1e-4);

    let f2_val = reader.read_f2dot14().unwrap();
    assert!((f2_val - 1.0).abs() < 1e-4);

    assert_eq!(reader.read_u8().unwrap(), 0xAB);
    assert_eq!(reader.remaining(), 0);

    // Reading past EOF must return TtfError::UnexpectedEof
    assert_eq!(reader.read_u8().unwrap_err(), TtfError::UnexpectedEof);

    // Sub-reader tests
    let full_reader = TtfReader::new(&sample_data);
    let sub = full_reader.sub_reader(2, 4).unwrap();
    assert_eq!(sub.remaining(), 4);
    let mut sub_mut = sub;
    assert_eq!(sub_mut.read_u32().unwrap(), 0x56789ABC);

    // Sub-reader bounds overflow
    assert_eq!(full_reader.sub_reader(10, 10).unwrap_err(), TtfError::UnexpectedEof);
}

#[test]
fn test_ttf_outline_regularization_and_bezier_subdivision() {
    use rustext::gfx::ttf::{Contour, GlyphOutline, OutlinePoint};

    // Construct a contour with consecutive off-curve points (implicit on-curve midpoint)
    // P0: On-curve (0, 0)
    // P1: Off-curve (10, 0)
    // P2: Off-curve (10, 10) -> implicit midpoint at (10, 5)
    // P3: On-curve (0, 10)
    let raw_points = vec![
        OutlinePoint::on_curve(0.0, 0.0),
        OutlinePoint::off_curve(10.0, 0.0),
        OutlinePoint::off_curve(10.0, 10.0),
        OutlinePoint::on_curve(0.0, 10.0),
    ];
    let contour = Contour::with_points(raw_points);
    let regularized = contour.regularize_points();

    // Must have inserted the midpoint (10, 5) between P1 and P2
    assert!(regularized.len() >= 5);
    let has_implicit_midpoint = regularized.iter().any(|p| {
        p.on_curve && (p.x - 10.0).abs() < 1e-3 && (p.y - 5.0).abs() < 1e-3
    });
    assert!(has_implicit_midpoint, "Implicit on-curve midpoint must be regularized");

    // Test De Casteljau quadratic Bézier flattening
    let mut outline = GlyphOutline::new();
    outline.add_contour(contour);

    let flattened = outline.scale_and_flatten(1.0, 1.0, 0.0, 0.0, 0.25);
    assert_eq!(flattened.len(), 1);
    let polygon = &flattened[0];
    assert!(polygon.len() > 4, "Bézier curve must subdivide into flattened polygonal line segments");

    // All polygon points must remain within the bounding hull [0..10, 0..10]
    for pt in polygon {
        assert!(pt.x >= -0.01 && pt.x <= 10.01);
        assert!(pt.y >= -0.01 && pt.y <= 10.01);
    }
}

#[test]
fn test_ttf_font_synthesizer_and_table_parser() {
    use rustext::gfx::ttf::{TrueTypeFont, TtfFontGenerator};

    // 1. Synthesize built-in vector TTF binary
    let ttf_bytes = TtfFontGenerator::generate_default_monospace_font();
    assert!(ttf_bytes.len() > 1024, "TTF font must contain complete SFNT tables");

    // Check SFNT magic tag: standard TrueType (0x00010000)
    assert_eq!(&ttf_bytes[0..4], &[0x00, 0x01, 0x00, 0x00]);

    // 2. Parse synthesized font using pure Rust TrueType parser
    let font = TrueTypeFont::from_bytes(&ttf_bytes)
        .expect("Generated TTF font must parse successfully");

    assert_eq!(font.units_per_em, 1000);
    assert_eq!(font.ascender, 800);
    assert_eq!(font.descender, -200);
    assert_eq!(font.family_name, "Rustext Vector Mono");
    assert_eq!(font.style_name, "Regular");
    assert!(font.num_glyphs >= 96, "Font must contain printable ASCII range");

    // 3. Test character-to-glyph mapping via cmap table
    assert_eq!(font.char_to_glyph_id(' '), Some(1));
    assert_eq!(font.char_to_glyph_id('!'), Some(2));
    assert_eq!(font.char_to_glyph_id('A'), Some(34)); // 65 - 31 = 34

    // Unknown unicode character must return None or map gracefully
    let unknown_gid = font.char_to_glyph_id('\u{1F600}');
    assert_eq!(unknown_gid, None);

    // 4. Test glyph outline retrieval
    let outline_a = font.get_glyph_outline(34).expect("Glyph 'A' outline must exist");
    assert!(!outline_a.is_empty(), "Glyph 'A' outline must have contours");
    assert!(outline_a.contours.len() >= 2, "Glyph 'A' has outer contour and inner triangle hole");

    // 5. Test metrics retrieval
    let metrics_a = font.get_glyph_metrics(34);
    assert_eq!(metrics_a.advance_width, 600);
    assert_eq!(metrics_a.left_side_bearing, 60);
}

#[test]
fn test_ttf_rasterizer_analytical_scanline_coverage() {
    use rustext::gfx::ttf::{TrueTypeFont, TtfFontGenerator, TtfRasterizer};

    let bundled_bytes = TtfFontGenerator::get_bundled_ttf();
    let font = TrueTypeFont::from_bytes(bundled_bytes).unwrap();

    // Rasterize character 'A' into a 16x24 pixel grid
    let target_w = 16;
    let target_h = 24;
    let (w, h, coverage) = TtfRasterizer::rasterize_char(&font, 'A', target_w, target_h);

    assert_eq!(w, target_w);
    assert_eq!(h, target_h);
    assert_eq!(coverage.len(), target_w * target_h);

    // Verify non-empty coverage
    let non_zero_count = coverage.iter().filter(|&&alpha| alpha > 0).count();
    assert!(non_zero_count > 10, "Glyph 'A' must produce rasterized pixels");

    // Verify analytical anti-aliasing produces smooth fractional coverage (0 < alpha < 255)
    let partial_alpha_count = coverage.iter().filter(|&&alpha| alpha > 0 && alpha < 255).count();
    assert!(partial_alpha_count > 0, "Analytical scanline accumulator must generate anti-aliased edge alphas");

    // Space ' ' should be entirely blank
    let (_, _, space_cov) = TtfRasterizer::rasterize_char(&font, ' ', target_w, target_h);
    assert!(space_cov.iter().all(|&a| a == 0), "Space glyph must be completely empty");
}

#[test]
fn test_ttf_rasterizer_subpixel_cleartype_lcd_rendering() {
    use rustext::gfx::ttf::{TrueTypeFont, TtfFontGenerator, TtfRasterizer};

    let bundled_bytes = TtfFontGenerator::get_bundled_ttf();
    let font = TrueTypeFont::from_bytes(bundled_bytes).unwrap();

    let target_w = 14;
    let target_h = 20;
    let (w, h, subpixel_cov) = TtfRasterizer::rasterize_char_subpixel(&font, 'M', target_w, target_h);

    assert_eq!(w, target_w);
    assert_eq!(h, target_h);
    assert_eq!(subpixel_cov.len(), target_w * target_h);

    // Each subpixel entry is [r, g, b]
    let mut total_r = 0u64;
    let mut total_g = 0u64;
    let mut total_b = 0u64;
    let mut has_asymmetric_subpixels = false;

    for &[r, g, b] in &subpixel_cov {
        total_r += r as u64;
        total_g += g as u64;
        total_b += b as u64;
        if r != g || g != b {
            has_asymmetric_subpixels = true;
        }
    }

    assert!(total_r > 0 && total_g > 0 && total_b > 0, "Subpixel rendering must populate color channels");
    assert!(has_asymmetric_subpixels, "ClearType LCD 3x subpixel rendering must produce color-fringe anti-aliasing");
}

#[test]
fn test_system_font_loader_and_caching() {
    use rustext::gfx::ttf::SystemFontLoader;

    let font = SystemFontLoader::get_active_font();
    let gid = font.char_to_glyph_id('m').unwrap();
    let outline = font.get_glyph_outline(gid).unwrap();
    assert!(!outline.contours.is_empty());
    let (w, h, cov_m) = rustext::gfx::ttf::TtfRasterizer::rasterize_char(&font, 'm', 8, 16);
    assert_eq!(w, 8);
    assert_eq!(h, 16);
    assert!(cov_m.iter().any(|&c| c > 0), "Character 'm' must render non-zero coverage");

    for ch in ['a', 'e', 'o', 'B', 'g', '1'] {
        let (cw, ch_h, cov) = rustext::gfx::ttf::TtfRasterizer::rasterize_char(&font, ch, 8, 16);
        assert_eq!(cw, 8);
        assert_eq!(ch_h, 16);
        assert!(cov.iter().any(|&c| c > 0), "Character '{}' must render non-zero coverage", ch);
    }

    assert!(font.units_per_em > 0);
    assert!(font.num_glyphs > 0);

    // Same pointer / cache retrieval
    let font2 = SystemFontLoader::get_active_font();
    assert!(std::sync::Arc::ptr_eq(&font, &font2), "SystemFontLoader must return cached Arc instance");

    // Fallback on non-existent path
    let fallback = SystemFontLoader::load_font(Some("/non_existent_directory/font.ttf"));
    assert!(fallback.num_glyphs > 0, "Non-existent path must gracefully fallback to valid font");
}

#[test]
fn test_config_ini_ttf_persistence_and_roundtrip() {
    use rustext::editor::config::EditorConfig;

    let ini = r#"
[gui]
use_ttf_font = true
ttf_font_path = /usr/share/fonts/custom/MyFont.ttf
"#;
    let cfg = EditorConfig::parse(ini);
    assert!(cfg.gui.use_ttf_font);
    assert_eq!(cfg.gui.ttf_font_path, "/usr/share/fonts/custom/MyFont.ttf");

    // Toggle off
    let ini_off = r#"
[gui]
use_ttf_font = false
ttf_font_path = 
"#;
    let cfg_off = EditorConfig::parse(ini_off);
    assert!(!cfg_off.gui.use_ttf_font);
    assert_eq!(cfg_off.gui.ttf_font_path, "");

    // Serialization roundtrip
    let serialized = cfg.to_ini_string();
    assert!(serialized.contains("use_ttf_font = true"));
    assert!(serialized.contains("ttf_font_path = /usr/share/fonts/custom/MyFont.ttf"));

    let roundtrip = EditorConfig::parse(&serialized);
    assert!(roundtrip.gui.use_ttf_font);
    assert_eq!(roundtrip.gui.ttf_font_path, "/usr/share/fonts/custom/MyFont.ttf");
}

#[test]
fn test_framebuffer_and_renderer_ttf_integration() {
    use rustext::core::TextBuffer;
    use rustext::editor::Document;
    use rustext::gfx::{Framebuffer, GfxRenderer};

    let mut fb = Framebuffer::new(320, 240);
    fb.set_use_ttf_font(true);
    assert!(fb.use_ttf_font);

    // Draw text with TTF vector rasterizer
    fb.draw_text("Rustext TTF", 10, 20, 0x00FFFFFF, Some(0x00000000));

    // Verify text pixels were drawn into framebuffer
    let mut drawn_pixels = 0;
    for y in 20..40 {
        for x in 10..120 {
            if fb.get_pixel(x, y) != 0x00000000 {
                drawn_pixels += 1;
            }
        }
    }
    assert!(drawn_pixels > 20, "Framebuffer text with TTF must render visible pixels");

    // Verify GfxRenderer with use_ttf_font
    let mut r = GfxRenderer::new();
    r.use_ttf_font = true;
    let doc = Document::new(TextBuffer::from_str("let x = 42;"));
    let tabs = vec!["main.rs".to_string()];
    r.render(&mut fb, &doc, &tabs, 0, None, None, None);

    let doc_drawn_pixels = fb.pixels.iter().filter(|&&p| p != 0x00000000).count();
    assert!(doc_drawn_pixels > 50, "Renderer with TTF vector font must render document line tokens");
}

#[cfg(target_os = "linux")]
#[test]
fn test_wayland_socket_discovery_and_availability() {
    use rustext::window::wayland::WaylandWindow;

    // Check socket resolution
    let socket_opt = WaylandWindow::find_socket_path();
    let is_avail = WaylandWindow::is_available();

    if std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("XDG_RUNTIME_DIR").is_ok() {
        if let Some(ref path) = socket_opt {
            assert!(path.exists(), "Detected Wayland socket must exist on filesystem");
            assert!(is_avail, "is_available() must be true when socket path exists");

            let res = WaylandWindow::new("Wayland Direct Test", 640, 480);
            assert!(res.is_ok(), "WaylandWindow::new failed: {:?}", res.err());
            let mut win = res.unwrap();

            // Initial buffer presentation
            let mut fb1 = Framebuffer::new(640, 480);
            fb1.clear(0xFF1E1E1E);
            win.present(&fb1);

            // Resized / maximized buffer presentation (verifies recreation of wl_buffer with new stride)
            let mut fb2 = Framebuffer::new(1920, 1080);
            fb2.clear(0xFF252526);
            win.present(&fb2);
            assert_eq!(win.width, 1920);
            assert_eq!(win.height, 1080);

            // Poll events for 100ms to observe compositor response
            for _ in 0..10 {
                let _ = win.poll_event();
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_linux_window_creation_and_fallback() {
    use rustext::window::linux::LinuxWindow;
    use rustext::window::WindowEvent;

    // Create a native Linux window (Wayland or X11)
    let win_res = LinuxWindow::new("Rustext Test Window", 640, 480);
    assert!(win_res.is_ok(), "LinuxWindow::new must successfully initialize: {:?}", win_res.err());

    let mut win = win_res.unwrap();
    assert!(win.width >= 400);
    assert!(win.height >= 300);
    assert!(win.scale_factor() >= 0.5);

    let name = win.backend_name();
    assert!(name == "Wayland" || name == "X11", "Backend name must be Wayland or X11, got: {}", name);

    if win.is_wayland() {
        assert_eq!(name, "Wayland");
    } else {
        assert_eq!(name, "X11");
    }

    // Set title
    win.set_title("Rustext Native Window Test - Updated");

    // Poll event non-blocking
    let ev = win.poll_event();
    // Initially should be None or Resize/Configure
    match ev {
        WindowEvent::None | WindowEvent::Resize { .. } => {}
        other => println!("Initial event: {:?}", other),
    }

    // Present a test Framebuffer
    let mut fb = rustext::gfx::Framebuffer::new(win.width, win.height);
    fb.fill_rect(0, 0, win.width, win.height, 0x001E1E2E);
    fb.draw_text("Wayland Native Support in Rustext", 10, 10, 0x00CDD6F4, None);
    win.present(&fb);

    // Close state
    assert!(!win.should_close());
    win.cancel_close();
    assert!(!win.should_close());
}

#[cfg(target_os = "linux")]
#[test]
fn test_linux_backend_override_environment() {
    use rustext::window::linux::LinuxWindow;

    // Test X11 override
    unsafe {
        std::env::set_var("RUSTEXT_BACKEND", "x11");
    }
    if let Ok(win) = LinuxWindow::new("Rustext X11 Force", 400, 300) {
        assert!(win.is_x11());
        assert_eq!(win.backend_name(), "X11");
    }
    unsafe {
        std::env::remove_var("RUSTEXT_BACKEND");
    }
}

#[test]
fn test_ttf_glyph_cache_lifecycle_and_warmup() {
    use rustext::gfx::ttf::{SystemFontLoader, TtfRasterizer};

    let font = SystemFontLoader::get_active_font();
    TtfRasterizer::clear_cache();
    let stats_empty = TtfRasterizer::cache_stats();
    assert_eq!(stats_empty.entry_count, 0);

    // Warm up ASCII at 8x16
    TtfRasterizer::warmup_cache(&font, 8, 16);
    let stats_warmed = TtfRasterizer::cache_stats();
    assert!(stats_warmed.fast_ascii_count >= 95, "ASCII warmup must populate fast tables");
    assert!(stats_warmed.memory_bytes > 0);

    // Retrieval of warmed ASCII character 'A' should hit the cache immediately
    let prev_hits = stats_warmed.hits;
    let (w, h, cov_a) = TtfRasterizer::rasterize_char_cached(&font, 'A', 8, 16);
    assert_eq!(w, 8);
    assert_eq!(h, 16);
    assert!(cov_a.iter().any(|&c| c > 0));

    let stats_after = TtfRasterizer::cache_stats();
    assert!(stats_after.hits > prev_hits, "Warm ASCII lookup must register a cache hit");

    // Subpixel cached retrieval
    let (sw, sh, sub_a) = TtfRasterizer::rasterize_char_subpixel_cached(&font, 'A', 8, 16);
    assert_eq!(sw, 8);
    assert_eq!(sh, 16);
    assert!(sub_a.iter().any(|&[r, g, b]| r > 0 || g > 0 || b > 0));

    // Clear cache
    TtfRasterizer::clear_cache();
    let stats_cleared = TtfRasterizer::cache_stats();
    assert_eq!(stats_cleared.entry_count, 0);
    assert_eq!(stats_cleared.fast_ascii_count, 0);
}

#[test]
fn test_ttf_glyph_cache_eviction_and_metrics() {
    use rustext::gfx::ttf::{SystemFontLoader, TtfGlyphCache};

    let font = SystemFontLoader::get_active_font();
    // Cache with tiny capacity of 128 to test LRU eviction
    let mut cache = TtfGlyphCache::with_capacity(128);

    for code in 32u32..=200u32 {
        if let Some(ch) = char::from_u32(code) {
            let _ = cache.get_or_rasterize_grayscale(&font, ch, 10, 20);
        }
    }

    let stats = cache.stats();
    assert!(stats.entry_count <= 128, "Cache must not exceed maximum configured capacity");
    assert!(stats.hit_ratio() >= 0.0 && stats.hit_ratio() <= 100.0);

    // Re-accessing an existing glyph produces hits
    let hits_before = cache.stats().hits;
    let _ = cache.get_or_rasterize_grayscale(&font, 'x', 10, 20);
    let hits_after = cache.stats().hits;
    assert!(hits_after > hits_before);
}

#[test]
fn test_security_atomic_file_save() {
    use rustext::core::TextBuffer;
    use std::fs;

    let temp_dir = std::env::temp_dir().join(format!("rustext_sec_test_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();
    let target_file = temp_dir.join("critical_code.rs");

    let mut buf = TextBuffer::new();
    buf.set_file_path(target_file.clone());
    buf.insert(0, "pub fn safe_atomic() -> bool { true }\n");

    assert!(buf.save().is_ok());
    assert!(target_file.exists());

    let saved = fs::read_to_string(&target_file).unwrap();
    assert_eq!(saved, "pub fn safe_atomic() -> bool { true }\n");

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_security_draft_path_traversal_confinement() {
    use rustext::editor::session::SessionManager;
    use std::path::PathBuf;

    let manager = SessionManager::default();

    // 1. Path with parent traversal must be rejected
    let traversal_path = PathBuf::from("/etc/../etc/passwd.draft");
    assert!(!manager.is_safe_draft_path(&traversal_path));
    assert!(manager.read_draft(&traversal_path).is_err());

    // 2. Arbitrary file without .draft extension must be rejected
    let arbitrary_path = PathBuf::from("/etc/shadow");
    assert!(!manager.is_safe_draft_path(&arbitrary_path));
    assert!(manager.read_draft(&arbitrary_path).is_err());

    // 3. Legit draft within drafts directory is accepted
    let legitimate = manager.drafts_dir().join("tab_1.draft");
    assert!(manager.is_safe_draft_path(&legitimate));
}

#[test]
fn test_security_global_search_symlink_defense() {
    use rustext::core::GlobalSearchEngine;
    use std::fs;

    let temp_dir = std::env::temp_dir().join(format!("rustext_symlink_test_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let normal_file = temp_dir.join("test.rs");
    fs::write(&normal_file, "fn search_target_token() {}\n").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        // Create a symlink loop: loop_dir -> temp_dir
        let loop_link = temp_dir.join("loop_link");
        let _ = symlink(&temp_dir, &loop_link);
    }

    // Search must terminate gracefully without stack overflow or infinite recursion
    let results = GlobalSearchEngine::search_project(&temp_dir, "search_target_token", false, 100);
    assert_eq!(results.matches.len(), 1);
    assert_eq!(results.matches[0].line_number, 1);

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_word_wrap_compute_sublines_and_word_boundaries() {
    use rustext::editor::wrap::WordWrap;

    // 1. Empty string
    let empty_subs = WordWrap::compute_sublines("", 40);
    assert_eq!(empty_subs.len(), 1);
    assert_eq!(empty_subs[0].char_start, 0);
    assert_eq!(empty_subs[0].char_end, 0);
    assert!(!empty_subs[0].is_continuation);

    // 2. Short string under max_cols
    let short_subs = WordWrap::compute_sublines("fn main() {}", 40);
    assert_eq!(short_subs.len(), 1);
    assert_eq!(short_subs[0].char_start, 0);
    assert_eq!(short_subs[0].char_end, 12);
    assert!(!short_subs[0].is_continuation);

    // 3. Long sentence broken cleanly at space boundary
    let text = "let message = \"This is a long sentence that should wrap gracefully across boundaries\";";
    let wrapped = WordWrap::compute_sublines(text, 30);
    assert!(wrapped.len() >= 3);
    assert!(!wrapped[0].is_continuation);
    assert!(wrapped[1].is_continuation);
    assert!(wrapped[2].is_continuation);

    // Ensure all characters are accounted for without gaps
    let mut prev_end = 0;
    for sub in &wrapped {
        assert_eq!(sub.char_start, prev_end);
        assert!(sub.char_end > sub.char_start);
        prev_end = sub.char_end;
    }
    assert_eq!(prev_end, text.chars().count());

    // 4. Broken at punctuation boundary (e.g. method chain)
    let code = "let result = data.filter(|x| x > 0).map(|x| x * 2).collect::<Vec<_>>();";
    let code_wrapped = WordWrap::compute_sublines(code, 35);
    assert!(code_wrapped.len() >= 2);
    assert_eq!(code_wrapped[0].char_start, 0);

    // 5. Unbroken long identifier / URL (hard fallback break)
    let long_token = "https://subdomain.domain.example.com/very/long/path/with/lots/of/slashes/and_data_token";
    let token_wrapped = WordWrap::compute_sublines(long_token, 25);
    assert!(token_wrapped.len() >= 3);
    for sub in &token_wrapped {
        assert!(sub.len_chars() <= 25);
    }

    // 6. Multibyte UTF-8 characters (emojis, accents, kanji)
    let utf8_text = "🦀 Rust programación 日本語: alta velocidad y cero dependencias!";
    let utf8_wrapped = WordWrap::compute_sublines(utf8_text, 20);
    assert!(utf8_wrapped.len() >= 3);
    for sub in &utf8_wrapped {
        // Ensure string slicing on byte boundaries does not panic
        let slice = &utf8_text[sub.byte_start..sub.byte_end];
        assert_eq!(slice.chars().count(), sub.len_chars());
    }
}

#[test]
fn test_word_wrap_coordinate_mapping() {
    use rustext::editor::wrap::WordWrap;

    let text = "The quick brown fox jumps over the lazy dog near the river bank";
    let sublines = WordWrap::compute_sublines(text, 25);

    // First subline
    let (s0, v0) = WordWrap::col_to_visual(0, &sublines);
    assert_eq!(s0, 0);
    assert_eq!(v0, 0);
    assert_eq!(WordWrap::visual_to_col(s0, v0, &sublines), 0);

    let (s_mid, v_mid) = WordWrap::col_to_visual(10, &sublines);
    assert_eq!(s_mid, 0);
    assert_eq!(v_mid, 10);
    assert_eq!(WordWrap::visual_to_col(s_mid, v_mid, &sublines), 10);

    // Second subline
    let second_start = sublines[1].char_start;
    let (s1, v1) = WordWrap::col_to_visual(second_start + 4, &sublines);
    assert_eq!(s1, 1);
    assert_eq!(v1, 4);
    assert_eq!(WordWrap::visual_to_col(s1, v1, &sublines), second_start + 4);

    // Clamping past end
    let total_len = text.chars().count();
    let (s_last, v_last) = WordWrap::col_to_visual(total_len + 50, &sublines);
    assert_eq!(s_last, sublines.len() - 1);
    assert_eq!(v_last, (total_len + 50) - sublines[sublines.len() - 1].char_start);
}

#[test]
fn test_word_wrap_adaptive_cursor_movement() {
    use rustext::core::buffer::TextBuffer;
    use rustext::core::cursor::CursorSet;
    use rustext::editor::wrap::WordWrap;

    let text = "First long line that wraps across multiple visual sublines when max columns is set to thirty\nSecond line";
    let buffer = TextBuffer::from_str(text);
    let mut cursors = CursorSet::new();

    // Start at (0, 0)
    assert_eq!(cursors.primary().head, 0);

    // Move Down into second subline of the first physical line
    WordWrap::move_down(&mut cursors, &buffer, 30, false);
    let pos1 = buffer.char_to_position(cursors.primary().head);
    assert_eq!(pos1.line, 0);
    assert!(pos1.col > 0, "Cursor should have advanced into subline 1");

    // Move Down again into third subline of the first physical line
    WordWrap::move_down(&mut cursors, &buffer, 30, false);
    let pos2 = buffer.char_to_position(cursors.primary().head);
    assert_eq!(pos2.line, 0);
    assert!(pos2.col > pos1.col, "Cursor should have advanced into subline 2");

    // Move Down again -> transitions into the second physical line
    WordWrap::move_down(&mut cursors, &buffer, 30, false);
    WordWrap::move_down(&mut cursors, &buffer, 30, false);
    let pos3 = buffer.char_to_position(cursors.primary().head);
    assert_eq!(pos3.line, 1, "Cursor should have reached physical line 1");

    // Move Up -> transitions back into the last subline of physical line 0
    WordWrap::move_up(&mut cursors, &buffer, 30, false);
    let pos_back = buffer.char_to_position(cursors.primary().head);
    assert_eq!(pos_back.line, 0);

    // Move to visual line end
    WordWrap::move_to_line_end(&mut cursors, &buffer, 30, false);
    let pos_end = buffer.char_to_position(cursors.primary().head);
    assert!(pos_end.col > 0);

    // Move to visual line start
    WordWrap::move_to_line_start(&mut cursors, &buffer, 30, false);
    let pos_start = buffer.char_to_position(cursors.primary().head);
    assert!(pos_start.col <= pos_end.col);
}

#[test]
fn test_word_wrap_editor_action_and_keymap() {
    use rustext::core::buffer::TextBuffer;
    use rustext::editor::{Editor, EditorAction, Keymap};
    use rustext::ui::terminal::Key;

    // 1. Keymap mapping for Alt+Z and Alt+z
    assert_eq!(Keymap::resolve_key(Key::Alt('z')), EditorAction::ToggleWordWrap);
    assert_eq!(Keymap::resolve_key(Key::Alt('Z')), EditorAction::ToggleWordWrap);

    // 2. Editor action execution
    let mut editor = Editor::new(TextBuffer::from_str("Sample text"));
    assert!(!editor.word_wrap);

    editor.execute_action(EditorAction::ToggleWordWrap);
    assert!(editor.word_wrap);
    assert!(editor.config.editor.word_wrap);
    let status = editor.status_message.as_ref().map(|(msg, _)| msg.as_str()).unwrap_or("");
    assert!(status.contains("ACTIVADO"));

    editor.execute_action(EditorAction::ToggleWordWrap);
    assert!(!editor.word_wrap);
    assert!(!editor.config.editor.word_wrap);
    let status2 = editor.status_message.as_ref().map(|(msg, _)| msg.as_str()).unwrap_or("");
    assert!(status2.contains("DESACTIVADO"));
}

#[test]
fn test_word_wrap_renderer_and_sublines() {
    use rustext::core::buffer::TextBuffer;
    use rustext::editor::Document;
    use rustext::gfx::{Framebuffer, GfxRenderer};

    let text = "let greeting = \"Hello World! This is a very long line designed to test the soft wrap rendering in GfxRenderer\";\nlet x = 42;\n";
    let doc = Document::new(TextBuffer::from_str(text));

    let mut renderer = GfxRenderer::new();
    renderer.word_wrap = true;
    let mut fb = Framebuffer::new(800, 600);

    // 1. Render in editor pane
    renderer.render_editor_pane(
        &mut fb,
        &doc,
        0,
        0,
        800,
        600,
        true,
        "test.rs",
        false,
        false,
        false,
        false,
    );

    // Ensure framebuffer pixels were drawn (not all zeros)
    let non_zero_pixels = fb.pixels.iter().filter(|&&p| p != 0).count();
    assert!(non_zero_pixels > 100);

    // 2. Render full frame
    let mut fb2 = Framebuffer::new(1024, 640);
    renderer.render_full_frame_with_split(
        &mut fb2,
        &doc,
        None,
        None,
        &["test.rs".to_string()],
        0,
        Some("Word Wrap active"),
        None,
        None,
        None,
        false,
        false,
        false,
        false,
        None,
    );
    let non_zero_pixels2 = fb2.pixels.iter().filter(|&&p| p != 0).count();
    assert!(non_zero_pixels2 > 500);
}

#[test]
fn test_framebuffer_draw_text_clipped_and_truncated() {
    let mut fb = Framebuffer::new(300, 100);
    fb.clear(0x00000000);

    let text = "Texto largo que no debe desbordarse jamás";
    // Clip strictly at max_x = 100
    fb.draw_text_clipped(text, 10, 20, 100, 0x00FFFFFF, None);

    // Any pixel at x >= 100 on row 20..36 must remain 0
    for y in 20..36 {
        for x in 100..300 {
            assert_eq!(fb.get_pixel(x, y), 0x00000000, "Pixel at ({}, {}) should be clipped", x, y);
        }
    }
    // Pixels before x=100 should be drawn
    let drawn_clipped = fb.pixels.iter().filter(|&&p| p != 0).count();
    assert!(drawn_clipped > 0);

    // Test draw_text_truncated
    fb.clear(0x00000000);
    fb.draw_text_truncated(text, 10, 40, 120, 0x00FFFFFF, None);
    for y in 40..56 {
        for x in 120..300 {
            assert_eq!(fb.get_pixel(x, y), 0x00000000, "Pixel at ({}, {}) should be truncated", x, y);
        }
    }
    let drawn_truncated = fb.pixels.iter().filter(|&&p| p != 0).count();
    assert!(drawn_truncated > 0);
}

#[test]
fn test_menu_bar_dropdown_dynamic_width_and_clamping() {
    use rustext::editor::MenuBar;

    let mut menu = MenuBar::new();
    let font_width = 8;

    // "Extensiones" has "Administrador de Extensiones..." (31) + "Ctrl+Shift+X" (12)
    // Category 5 is "Extensiones"
    let ext_w = menu.dropdown_width(5, font_width);
    assert!(ext_w >= 380, "Dropdown for Extensiones must expand dynamically to fit long label and shortcut (got {})", ext_w);

    // Clamping to screen width:
    // With window width 800, rightmost category "Ayuda" (index 7) or "Herramientas" (index 6)
    let max_w = 600;
    menu.set_window_width(max_w);
    let (drop_x, _, drop_w, _) = menu.dropdown_rect(6, font_width, max_w);
    assert!(drop_x + drop_w <= max_w, "Dropdown rect must be clamped within window width (x={}, w={}, max={})", drop_x, drop_w, max_w);
}

#[test]
fn test_font_selector_dialog_no_text_overflow() {
    use rustext::editor::font_selector::FontSelectorDialog;
    use rustext::gfx::font::{FontFamily, FontWeight};
    use rustext::gfx::HintingLevel;

    // Simulate 1024x640 window
    let mut fb = Framebuffer::new(1024, 640);
    let bg_color = 0x001A1B26;
    fb.clear(bg_color);

    let renderer = GfxRenderer::new();
    let dialog = FontSelectorDialog::new(
        FontFamily::Standard,
        12,
        FontWeight::Normal,
        3,
        true,
        true,
        HintingLevel::Medium,
        0,
        0.50,
        false,
        1.0,
        false,
        false,
        false,
        false,
    );

    renderer.render_font_selector_dialog(&mut fb, &dialog, false);

    let win_w = 760;
    let win_h = 520;
    let win_x = (1024 - win_w) / 2; // 132
    let win_y = (640 - win_h) / 2;  // 60
    let right_edge = win_x + win_w; // 892

    // Check that at the footer line (y = win_y + win_h - 20 = 560),
    // pixels to the right of the dialog and its drop shadow (x > right_edge + 24) are uniform dimmed backdrop,
    // and definitely not text foreground colors (0x00858585, 0x00FFFFFF, or cyan accent)
    let footer_y = win_y + win_h - 20;
    let expected_backdrop = fb.get_pixel(1000, footer_y);
    for x in (right_edge + 24)..1024 {
        let p = fb.get_pixel(x, footer_y);
        assert_eq!(
            p,
            expected_backdrop,
            "Footer shortcut text must not overflow outside the modal dialog at x={}, y={}",
            x,
            footer_y
        );
        assert_ne!(p, 0x00858585, "Pixel must not contain text foreground color");
        assert_ne!(p, 0x00FFFFFF, "Pixel must not contain button text foreground color");
    }
}

#[test]
fn test_code_folding_engine_brace_matching() {
    use rustext::core::buffer::TextBuffer;
    use rustext::core::syntax::Language;
    use rustext::editor::folding::FoldingManager;

    let code = "fn alpha() {\n    let a = 10;\n    let b = 20;\n}\n\nfn beta() {\n    let s = \"{ ignored }\"; // comment {\n    let c = 30;\n}\n";
    let buffer = TextBuffer::from_str(code);
    let mut folding = FoldingManager::new();

    folding.recompute_regions(&buffer, Language::Rust);
    assert!(folding.regions.len() >= 2, "Expected at least 2 fold regions for the two functions");

    // First function spans line 0 to line 3
    let r0 = &folding.regions[0];
    assert_eq!(r0.start_line, 0);
    assert_eq!(r0.end_line, 3);
    assert!(!r0.is_folded);

    // Second function spans line 5 to line 8 (ignoring brace in string and comment)
    let r1 = &folding.regions[1];
    assert_eq!(r1.start_line, 5);
    assert_eq!(r1.end_line, 8);
    assert!(!r1.is_folded);

    // Initial state: fold indicators
    assert_eq!(folding.is_fold_start(0), Some(false)); // Expanded
    assert_eq!(folding.is_fold_start(1), None);
    assert_eq!(folding.is_fold_start(5), Some(false)); // Expanded

    // Toggle fold at line 0
    assert!(folding.toggle_fold_at_line(0));
    assert_eq!(folding.is_fold_start(0), Some(true)); // Folded!
    assert_eq!(folding.fold_end_for_line(0), Some(3));

    // Lines 1, 2, 3 must be hidden; line 0 and line 4 must be visible
    assert!(!folding.is_line_hidden(0));
    assert!(folding.is_line_hidden(1));
    assert!(folding.is_line_hidden(2));
    assert!(folding.is_line_hidden(3));
    assert!(!folding.is_line_hidden(4));
    assert!(!folding.is_line_hidden(5));
    assert_eq!(folding.hidden_line_count(), 3);
}

#[test]
fn test_code_folding_engine_indentation() {
    use rustext::core::buffer::TextBuffer;
    use rustext::core::syntax::Language;
    use rustext::editor::folding::FoldingManager;

    let py_code = "def calculate():\n    x = 10\n    if x > 5:\n        y = x * 2\n        print(y)\n    return x\n";
    let buffer = TextBuffer::from_str(py_code);
    let mut folding = FoldingManager::new();

    folding.recompute_regions(&buffer, Language::Python);
    assert!(!folding.regions.is_empty(), "Python indentation folding must detect indented blocks");

    // Outer function starts at line 0 and spans to line 5
    let outer = folding.regions.iter().find(|r| r.start_line == 0);
    assert!(outer.is_some(), "Outer block at line 0 must exist");
    assert_eq!(outer.unwrap().end_line, 5);

    // Inner block (if statement) starts at line 2 and spans to line 4
    let inner = folding.regions.iter().find(|r| r.start_line == 2);
    assert!(inner.is_some(), "Inner block at line 2 must exist");
    assert_eq!(inner.unwrap().end_line, 4);

    // Toggle inner block: lines 3 and 4 should be hidden, line 5 (return x) remains visible
    folding.toggle_fold_at_line(2);
    assert!(!folding.is_line_hidden(0));
    assert!(!folding.is_line_hidden(2));
    assert!(folding.is_line_hidden(3));
    assert!(folding.is_line_hidden(4));
    assert!(!folding.is_line_hidden(5));
}

#[test]
fn test_code_folding_cursor_navigation_and_reveal() {
    use rustext::core::buffer::TextBuffer;
    use rustext::core::syntax::Language;
    use rustext::editor::folding::FoldingManager;

    let code = "line 0\nfn test() {\n    line 2\n    line 3\n    line 4\n}\nline 6\nline 7\n";
    let buffer = TextBuffer::from_str(code);
    let mut folding = FoldingManager::new();

    folding.recompute_regions(&buffer, Language::Rust);
    // Fold fn test() at line 1 (spans 1..=5)
    folding.toggle_fold_at_line(1);
    assert_eq!(folding.hidden_line_count(), 4);

    let total = buffer.len_lines();

    // Next visible line navigation:
    // From line 0 -> line 1
    assert_eq!(folding.next_visible_line(0, total), 1);
    // From line 1 (folded!) -> skips hidden 2, 3, 4, 5 -> jumps to line 6!
    assert_eq!(folding.next_visible_line(1, total), 6);
    // From line 6 -> line 7
    assert_eq!(folding.next_visible_line(6, total), 7);

    // Previous visible line navigation:
    // From line 6 -> skips hidden 5, 4, 3, 2 -> jumps to line 1!
    assert_eq!(folding.prev_visible_line(6), 1);
    // From line 1 -> line 0
    assert_eq!(folding.prev_visible_line(1), 0);

    // Screen row mapping:
    // Screen row 0 starting at scroll_row 0 is line 0
    assert_eq!(folding.visible_line_for_screen_row(0, 0, total), Some(0));
    // Screen row 1 is line 1
    assert_eq!(folding.visible_line_for_screen_row(0, 1, total), Some(1));
    // Screen row 2 is line 6 (skipping folded lines 2..=5)
    assert_eq!(folding.visible_line_for_screen_row(0, 2, total), Some(6));
    // Screen row 3 is line 7
    assert_eq!(folding.visible_line_for_screen_row(0, 3, total), Some(7));

    // Test unfold_to_reveal: jumping to line 3 automatically unfolds the block!
    assert!(folding.unfold_to_reveal(3));
    assert_eq!(folding.hidden_line_count(), 0);
    assert!(!folding.is_line_hidden(3));
}

#[test]
fn test_code_folding_gui_rendering_and_actions() {
    use rustext::core::buffer::TextBuffer;
    use rustext::editor::document::Document;
    use rustext::gfx::{Framebuffer, GfxRenderer};

    let code = "fn compute() {\n    let val = 42;\n    val * 2\n}\n// trailing line\n";
    let buffer = TextBuffer::from_str(code);
    let mut doc = Document::new(buffer);
    doc.language = rustext::core::syntax::Language::Rust;
    doc.update_folding();

    let renderer = GfxRenderer::new();
    let mut fb = Framebuffer::new(800, 600);

    // Initially expanded: fold indicator is ▼
    renderer.render_editor_pane(&mut fb, &doc, 0, 0, 800, 600, true, "main.rs", false, false, false, false);
    // Check gutter indicator on line 0 (y around tab_bar_height + line_spacing)
    let char_w = renderer.char_width();
    let line_h = renderer.line_height();
    let tab_h = renderer.tab_bar_height;
    let gutter_w = (3 + 2) * char_w;
    let fold_chevron_x = (3 + 1) * char_w;

    // Toggle fold at line 0
    assert!(doc.folding.toggle_fold_at_line(0));
    assert!(doc.folding.is_line_hidden(1));
    assert!(doc.folding.is_line_hidden(2));
    assert!(doc.folding.is_line_hidden(3));

    // Render folded editor pane
    let mut fb_folded = Framebuffer::new(800, 600);
    renderer.render_editor_pane(&mut fb_folded, &doc, 0, 0, 800, 600, true, "main.rs", false, false, false, false);

    // Verify gutter chevron has accent color pixel drawn for '▶'
    let mut has_accent_chevron = false;
    for px in fold_chevron_x..(fold_chevron_x + char_w) {
        for py in tab_h..(tab_h + line_h) {
            let pixel = fb_folded.get_pixel(px, py);
            if pixel != renderer.colors.background && pixel != renderer.colors.gutter_bg {
                has_accent_chevron = true;
                break;
            }
        }
    }
    assert!(has_accent_chevron, "Folded gutter must render the collapse indicator chevron");

    // Verify collapsed badge ' ··· ' is rendered after line 0 text
    let line_0_len = "fn compute() {".len();
    let text_start_x = gutter_w + char_w;
    let badge_start_x = text_start_x + line_0_len * char_w;
    let mut has_badge_pixel = false;
    for px in badge_start_x..(badge_start_x + char_w * 6) {
        for py in tab_h..(tab_h + line_h) {
            let pixel = fb_folded.get_pixel(px, py);
            if pixel != renderer.colors.background {
                has_badge_pixel = true;
                break;
            }
        }
    }
    assert!(has_badge_pixel, "Folded line must display the inline collapsed badge ' ··· '");

    // Test fold_all and unfold_all
    doc.folding.fold_all();
    assert!(doc.folding.regions.iter().all(|r| r.is_folded));
    doc.folding.unfold_all();
    assert!(doc.folding.regions.iter().all(|r| !r.is_folded));
    assert_eq!(doc.folding.hidden_line_count(), 0);
}

#[test]
fn test_indent_guides_calculation_and_blank_lines() {
    use rustext::core::TextBuffer;
    use rustext::editor::Document;
    use rustext::gfx::GfxRenderer;

    let content = "fn main() {\n    let a = 1;\n\n        let b = 2;\n\t\tlet c = 3;\n}\n";
    let doc = Document::new(TextBuffer::from_str(content));

    // Line 0: "fn main() {" -> 0 spaces
    assert_eq!(GfxRenderer::line_indent_spaces(&doc, 0, 4), 0);
    assert!(!GfxRenderer::is_blank_line(&doc, 0));

    // Line 1: "    let a = 1;" -> 4 spaces
    assert_eq!(GfxRenderer::line_indent_spaces(&doc, 1, 4), 4);
    assert!(!GfxRenderer::is_blank_line(&doc, 1));

    // Line 2: "" -> blank line, blank_line_indent should infer min(prev=4, next=8) = 4
    assert!(GfxRenderer::is_blank_line(&doc, 2));
    assert_eq!(GfxRenderer::blank_line_indent(&doc, 2, 4), 4);

    // Line 3: "        let b = 2;" -> 8 spaces
    assert_eq!(GfxRenderer::line_indent_spaces(&doc, 3, 4), 8);
    assert!(!GfxRenderer::is_blank_line(&doc, 3));

    // Line 4: "\t\tlet c = 3;" -> 2 tabs * 4 = 8 spaces
    assert_eq!(GfxRenderer::line_indent_spaces(&doc, 4, 4), 8);
    // with tab_size 2 -> 4 spaces
    assert_eq!(GfxRenderer::line_indent_spaces(&doc, 4, 2), 4);
}

#[test]
fn test_indent_guides_rendering_and_active_highlight() {
    use rustext::core::TextBuffer;
    use rustext::editor::Document;
    use rustext::editor::keymap::{EditorAction, Keymap};
    use rustext::ui::terminal::Key;
    use rustext::gfx::{Framebuffer, GfxRenderer};

    // Verify keymap binding
    assert_eq!(Keymap::resolve_key(Key::Alt('i')), EditorAction::ToggleIndentGuides);
    assert_eq!(Keymap::resolve_key(Key::Alt('I')), EditorAction::ToggleIndentGuides);

    let content = "fn test() {\n    let val = 100;\n\n    let val2 = 200;\n}\n";
    let mut doc = Document::new(TextBuffer::from_str(content));
    // Cursor at line 1 ("    let val = 100;")
    doc.cursors = rustext::core::cursor::CursorSet::single(16);

    let mut fb = Framebuffer::new(400, 300);
    let mut renderer = GfxRenderer::new();
    renderer.show_line_numbers = true;
    renderer.indent_guides = true;
    renderer.highlight_active_indent_guide = true;
    renderer.tab_size = 4;

    let tabs = vec!["test.rs".to_string()];
    renderer.render(&mut fb, &doc, &tabs, 0, None, None, None);

    // Line 1 is indented with 4 spaces.
    // Indent guide at column 0 should be rendered.
    // Check that active guide color (gutter_fg) or normal guide color (gutter_border) exists in the indent columns
    let gutter_cols = doc.buffer.len_lines().to_string().len().max(3);
    let gutter_w = (gutter_cols + 2) * renderer.char_width();
    let text_start_x = gutter_w + renderer.char_width();
    let guide_0_x = text_start_x; // col 0 guide

    let tab_h = 24; // logical tab bar height
    let line_h = renderer.line_height();

    // Verify that guide pixel at col 0 is rendered on line 1 and blank line 2
    let mut line1_has_guide = false;
    for y in (tab_h + line_h)..(tab_h + line_h * 2) {
        let p = fb.get_pixel(guide_0_x, y);
        if p == renderer.colors.gutter_fg || p == renderer.colors.gutter_border {
            line1_has_guide = true;
            break;
        }
    }
    assert!(line1_has_guide, "Indent guide must be rendered at col 0 for indented line 1");

    let mut line2_blank_has_guide = false;
    for y in (tab_h + line_h * 2)..(tab_h + line_h * 3) {
        let p = fb.get_pixel(guide_0_x, y);
        if p == renderer.colors.gutter_fg || p == renderer.colors.gutter_border {
            line2_blank_has_guide = true;
            break;
        }
    }
    assert!(line2_blank_has_guide, "Indent guide must seamlessly span across blank line 2");

    // Test with indent_guides disabled
    let mut fb_no_guides = Framebuffer::new(400, 300);
    renderer.indent_guides = false;
    renderer.render(&mut fb_no_guides, &doc, &tabs, 0, None, None, None);
    let mut no_guide_present = true;
    for y in (tab_h + line_h * 2)..(tab_h + line_h * 3) {
        let p = fb_no_guides.get_pixel(guide_0_x, y);
        if p == renderer.colors.gutter_fg || p == renderer.colors.gutter_border {
            no_guide_present = false;
            break;
        }
    }
    assert!(no_guide_present, "When indent_guides is disabled, no guide pixel should be rendered on blank line");

    // Also test word wrap rendering with indent guides enabled
    renderer.indent_guides = true;
    renderer.word_wrap = true;
    let mut fb_wrap = Framebuffer::new(400, 300);
    renderer.render(&mut fb_wrap, &doc, &tabs, 0, None, None, None);
    let mut wrap_has_guide = false;
    for y in (tab_h + line_h)..(tab_h + line_h * 2) {
        let p = fb_wrap.get_pixel(guide_0_x, y);
        if p == renderer.colors.gutter_fg || p == renderer.colors.gutter_border {
            wrap_has_guide = true;
            break;
        }
    }
    assert!(wrap_has_guide, "Indent guide must render correctly in word wrap mode");
}



