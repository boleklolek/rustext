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

    // Render normal state
    renderer.render(&mut fb, &doc, &tabs, 0, Some("Normal Mode"), None);

    // Pixel at top tab bar should have tab colors
    let tab_pixel = fb.get_pixel(20, 10);
    assert_ne!(tab_pixel, 0x00000000);

    // Render with search modal
    let mut modal = PromptModal::new(PromptKind::Find);
    modal.insert_char('t');
    modal.insert_char('e');
    modal.insert_char('s');
    modal.insert_char('t');

    renderer.render(&mut fb, &doc, &tabs, 1, None, Some(&modal));

    // Floating prompt modal center should have modal background color
    let prompt_pixel = fb.get_pixel(400, 55);
    assert_eq!(prompt_pixel, 0x00252526);
}

#[test]
fn test_theme_colors() {
    let theme = ThemeColors::modern_dark();
    assert_eq!(theme.background, 0x001E1E1E);
    assert_eq!(theme.status_bar_bg, 0x00007ACC);
    assert_eq!(theme.cursor, 0x00528BFF);
}
