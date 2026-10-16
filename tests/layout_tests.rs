//! Comprehensive test suite for Flexbox Layout Engine and Reactive vs. Retained UI Patterns.
//! Pure standard-library Rust with zero third-party dependencies.

use rustext::gfx::Framebuffer;
use rustext::ui::flex::{
    AlignSelf, Dimension, Edges, FlexDirection, FlexNode, FlexStyle,
    FlexWrap, JustifyContent,
};
use rustext::ui::reactive::{Computed, DirtyFlags, Effect, Signal};
use rustext::ui::retained::{WidgetId, WidgetKind, WidgetNode, WidgetTree};

#[test]
fn test_flexbox_row_grow_and_distribution() {
    // 300px wide row container with 3 items having flex_grow 1.0, 2.0, and 0.0 (fixed 50px)
    let item1 = FlexNode::new(1)
        .with_size(0.0, 40.0)
        .with_flex_grow(1.0);
    let item2 = FlexNode::new(2)
        .with_size(0.0, 40.0)
        .with_flex_grow(2.0);
    let item3 = FlexNode::new(3)
        .with_size(50.0, 40.0)
        .with_flex_grow(0.0);

    let mut root = FlexNode::new(0)
        .with_size(300.0, 100.0)
        .with_direction(FlexDirection::Row)
        .with_children(vec![item1, item2, item3]);

    root.compute_layout(300.0, 100.0);

    // Free space: 300 - 50 = 250px.
    // item1 gets 1/3 of 250 ~ 83.33px.
    // item2 gets 2/3 of 250 ~ 166.67px.
    // item3 remains 50px.
    let r1 = root.children[0].layout_rect;
    let r2 = root.children[1].layout_rect;
    let r3 = root.children[2].layout_rect;

    assert_eq!(r1.x, 0.0);
    assert!((r1.width - 83.33).abs() < 0.5);

    assert!((r2.x - 83.33).abs() < 0.5);
    assert!((r2.width - 166.67).abs() < 0.5);

    assert!((r3.x - 250.0).abs() < 0.5);
    assert_eq!(r3.width, 50.0);
}

#[test]
fn test_flexbox_row_shrink_on_overflow() {
    // 200px container containing two 150px items. Overflow is 100px.
    // item1: basis 150, shrink 1.0 -> scaled basis 150
    // item2: basis 150, shrink 1.0 -> scaled basis 150
    // Each shrinks equally by 50px to 100px.
    let item1 = FlexNode::new(1)
        .with_size(150.0, 40.0)
        .with_flex_shrink(1.0);
    let item2 = FlexNode::new(2)
        .with_size(150.0, 40.0)
        .with_flex_shrink(1.0);

    let mut root = FlexNode::new(0)
        .with_size(200.0, 60.0)
        .with_direction(FlexDirection::Row)
        .with_children(vec![item1, item2]);

    root.compute_layout(200.0, 60.0);

    assert_eq!(root.children[0].layout_rect.width, 100.0);
    assert_eq!(root.children[1].layout_rect.width, 100.0);
    assert_eq!(root.children[1].layout_rect.x, 100.0);
}

#[test]
fn test_flexbox_column_layout_and_gaps() {
    // 100x300 column container with 10px gap between items
    let item1 = FlexNode::new(1).with_size(80.0, 50.0);
    let item2 = FlexNode::new(2).with_size(80.0, 60.0);
    let item3 = FlexNode::new(3).with_size(80.0, 70.0);

    let mut root = FlexNode::new(0)
        .with_size(100.0, 300.0)
        .with_direction(FlexDirection::Column)
        .with_gap(10.0)
        .with_padding(Edges::all(5.0))
        .with_children(vec![item1, item2, item3]);

    root.compute_layout(100.0, 300.0);

    // item 1: y = 5.0, height = 50.0
    // item 2: y = 5.0 + 50.0 + 10.0 = 65.0, height = 60.0
    // item 3: y = 65.0 + 60.0 + 10.0 = 135.0, height = 70.0
    assert_eq!(root.children[0].layout_rect.y, 5.0);
    assert_eq!(root.children[0].layout_rect.height, 50.0);

    assert_eq!(root.children[1].layout_rect.y, 65.0);
    assert_eq!(root.children[1].layout_rect.height, 60.0);

    assert_eq!(root.children[2].layout_rect.y, 135.0);
    assert_eq!(root.children[2].layout_rect.height, 70.0);
}

#[test]
fn test_flexbox_justify_content_variants() {
    // 300px wide container with two 50px items. Free space = 200px.
    let make_container = |justify: JustifyContent| {
        let i1 = FlexNode::new(1).with_size(50.0, 30.0);
        let i2 = FlexNode::new(2).with_size(50.0, 30.0);
        let mut root = FlexNode::new(0)
            .with_size(300.0, 50.0)
            .with_justify_content(justify)
            .with_children(vec![i1, i2]);
        root.compute_layout(300.0, 50.0);
        root
    };

    // 1. FlexStart: x = 0, x = 50
    let start_root = make_container(JustifyContent::FlexStart);
    assert_eq!(start_root.children[0].layout_rect.x, 0.0);
    assert_eq!(start_root.children[1].layout_rect.x, 50.0);

    // 2. Center: remaining 200 / 2 = 100. x = 100, x = 150
    let center_root = make_container(JustifyContent::Center);
    assert_eq!(center_root.children[0].layout_rect.x, 100.0);
    assert_eq!(center_root.children[1].layout_rect.x, 150.0);

    // 3. FlexEnd: remaining 200. x = 200, x = 250
    let end_root = make_container(JustifyContent::FlexEnd);
    assert_eq!(end_root.children[0].layout_rect.x, 200.0);
    assert_eq!(end_root.children[1].layout_rect.x, 250.0);

    // 4. SpaceBetween: i1 at 0, i2 at 250 (300 - 50)
    let sb_root = make_container(JustifyContent::SpaceBetween);
    assert_eq!(sb_root.children[0].layout_rect.x, 0.0);
    assert_eq!(sb_root.children[1].layout_rect.x, 250.0);

    // 5. SpaceAround: 200 / 2 items = 100 per item. half space (50) before/after.
    // i1: 50..100, i2: 200..250
    let sa_root = make_container(JustifyContent::SpaceAround);
    assert_eq!(sa_root.children[0].layout_rect.x, 50.0);
    assert_eq!(sa_root.children[1].layout_rect.x, 200.0);

    // 6. SpaceEvenly: 200 / (2 + 1) = 66.67
    let se_root = make_container(JustifyContent::SpaceEvenly);
    assert!((se_root.children[0].layout_rect.x - 66.67).abs() < 0.5);
    assert!((se_root.children[1].layout_rect.x - 183.33).abs() < 0.5);
}

#[test]
fn test_flexbox_align_items_and_stretch() {
    // 200x100 container.
    // item 1: height 40, align_self Center -> centered at y = 30
    // item 2: height Auto, align_self Stretch -> stretched to height 100, y = 0
    // item 3: height 30, align_self FlexEnd -> y = 70
    let mut i1 = FlexNode::new(1).with_size(50.0, 40.0);
    i1.style.align_self = AlignSelf::Center;

    let mut i2 = FlexNode::new(2).with_width(Dimension::Points(50.0));
    i2.style.align_self = AlignSelf::Stretch;

    let mut i3 = FlexNode::new(3).with_size(50.0, 30.0);
    i3.style.align_self = AlignSelf::FlexEnd;

    let mut root = FlexNode::new(0)
        .with_size(200.0, 100.0)
        .with_direction(FlexDirection::Row)
        .with_children(vec![i1, i2, i3]);

    root.compute_layout(200.0, 100.0);

    assert_eq!(root.children[0].layout_rect.y, 30.0);
    assert_eq!(root.children[0].layout_rect.height, 40.0);

    assert_eq!(root.children[1].layout_rect.y, 0.0);
    assert_eq!(root.children[1].layout_rect.height, 100.0);

    assert_eq!(root.children[2].layout_rect.y, 70.0);
    assert_eq!(root.children[2].layout_rect.height, 30.0);
}

#[test]
fn test_flexbox_wrapping_multiline() {
    // 150px wide container with three 60px wide items and FlexWrap::Wrap
    // Item 1 + Item 2 = 120px fits on Line 1.
    // Item 3 = 60px wraps to Line 2.
    let i1 = FlexNode::new(1).with_size(60.0, 40.0);
    let i2 = FlexNode::new(2).with_size(60.0, 40.0);
    let i3 = FlexNode::new(3).with_size(60.0, 40.0);

    let mut root = FlexNode::new(0)
        .with_size(150.0, 100.0)
        .with_direction(FlexDirection::Row)
        .with_wrap(FlexWrap::Wrap)
        .with_children(vec![i1, i2, i3]);

    root.compute_layout(150.0, 100.0);

    // Line 1:
    assert_eq!(root.children[0].layout_rect.x, 0.0);
    assert_eq!(root.children[0].layout_rect.y, 0.0);
    assert_eq!(root.children[1].layout_rect.x, 60.0);
    assert_eq!(root.children[1].layout_rect.y, 0.0);

    // Line 2:
    assert_eq!(root.children[2].layout_rect.x, 0.0);
    assert_eq!(root.children[2].layout_rect.y, 40.0);
}

#[test]
fn test_flexbox_nested_containers() {
    // Full Application Shell layout:
    // Root: Column (800x600)
    //   - Header (Tab bar): height 30px
    //   - Main Body: flex_grow 1.0 (Row)
    //       - Sidebar: width 200px
    //       - Editor Area: flex_grow 1.0
    //   - Status Bar: height 24px

    let header = FlexNode::new(1).with_size(800.0, 30.0);

    let sidebar = FlexNode::new(2).with_width(Dimension::Points(200.0));
    let editor = FlexNode::new(3).with_flex_grow(1.0);

    let body = FlexNode::new(4)
        .with_direction(FlexDirection::Row)
        .with_flex_grow(1.0)
        .with_children(vec![sidebar, editor]);

    let status_bar = FlexNode::new(5).with_size(800.0, 24.0);

    let mut app = FlexNode::new(0)
        .with_size(800.0, 600.0)
        .with_direction(FlexDirection::Column)
        .with_children(vec![header, body, status_bar]);

    app.compute_layout(800.0, 600.0);

    // Header geometry
    assert_eq!(app.children[0].layout_rect.y, 0.0);
    assert_eq!(app.children[0].layout_rect.height, 30.0);

    // Main Body geometry: 600 - 30 - 24 = 546px high
    let body_rect = app.children[1].layout_rect;
    assert_eq!(body_rect.y, 30.0);
    assert_eq!(body_rect.height, 546.0);

    // Sidebar: x = 0, width = 200, height = 546 (stretched)
    let sidebar_rect = app.children[1].children[0].layout_rect;
    assert_eq!(sidebar_rect.x, 0.0);
    assert_eq!(sidebar_rect.width, 200.0);
    assert_eq!(sidebar_rect.height, 546.0);

    // Editor: x = 200, width = 600 (800 - 200), height = 546
    let editor_rect = app.children[1].children[1].layout_rect;
    assert_eq!(editor_rect.x, 200.0);
    assert_eq!(editor_rect.width, 600.0);
    assert_eq!(editor_rect.height, 546.0);

    // Status bar geometry: y = 30 + 546 = 576, height = 24
    assert_eq!(app.children[2].layout_rect.y, 576.0);
    assert_eq!(app.children[2].layout_rect.height, 24.0);
}

#[test]
fn test_reactive_signal_get_set_and_update() {
    let count = Signal::new(10);
    assert_eq!(count.get(), 10);
    assert_eq!(count.version(), 1);

    // Setting same value should not bump version
    count.set(10);
    assert_eq!(count.version(), 1);

    // Setting new value bumps version
    count.set(25);
    assert_eq!(count.get(), 25);
    assert_eq!(count.version(), 2);

    // In-place update
    count.update(|val| *val += 5);
    assert_eq!(count.get(), 30);
    assert_eq!(count.version(), 3);
}

#[test]
fn test_reactive_computed_memo_lazy_caching() {
    let base = Signal::new(5);
    let base_clone = base.clone();

    let squared = Computed::from_signal(base, move |val| val * val);

    // Initial state: not evaluated yet
    assert_eq!(squared.eval_count(), 0);

    // First read triggers evaluation
    assert_eq!(squared.get(), 25);
    assert_eq!(squared.eval_count(), 1);

    // Subsequent reads hit cache without re-evaluating
    assert_eq!(squared.get(), 25);
    assert_eq!(squared.get(), 25);
    assert_eq!(squared.eval_count(), 1);

    // Mutating dependency signal invalidates memo
    base_clone.set(9);
    // Next read re-evaluates
    assert_eq!(squared.get(), 81);
    assert_eq!(squared.eval_count(), 2);

    // Reading again hits cache
    assert_eq!(squared.get(), 81);
    assert_eq!(squared.eval_count(), 2);
}

#[test]
fn test_reactive_effect_and_dirty_flags() {
    let theme_mode = Signal::new("dark".to_string());
    let mut flags = DirtyFlags::clean();
    assert!(!flags.is_dirty());

    // Connect effect to signal
    let call_count = std::rc::Rc::new(std::cell::RefCell::new(0));
    let count_clone = call_count.clone();
    let _effect = Effect::on_signal(&theme_mode, move || {
        *count_clone.borrow_mut() += 1;
    });

    // Effect ran once on creation
    assert_eq!(*call_count.borrow(), 1);

    // Update signal triggers effect
    theme_mode.set("light".to_string());
    assert_eq!(*call_count.borrow(), 2);

    flags.mark_paint_dirty();
    assert!(flags.paint);
    assert!(!flags.layout);

    flags.mark_layout_dirty();
    assert!(flags.layout);
    assert!(flags.paint); // Layout invalidation forces repaint

    flags.clear();
    assert!(!flags.is_dirty());
}

#[test]
fn test_retained_widget_tree_layout_and_hit_testing() {
    // Construct a retained modal dialog with header, body input, and footer buttons
    let header = WidgetNode::text("Confirm Action", 0x00FFFFFF);

    let input = WidgetNode {
        id: WidgetId(101),
        kind: WidgetKind::Input {
            text: "Initial Query".to_string(),
            placeholder: "Search...".to_string(),
            is_focused: true,
            border_color: 0x007AA2F7,
        },
        flex_node: FlexNode::new(101).with_size(260.0, 32.0),
        dirty: DirtyFlags::all_dirty(),
        children: Vec::new(),
        subscriptions: Vec::new(),
    };

    let btn_cancel = WidgetNode::button("Cancel", 0x003B4261, 0x00C0CAF5);
    let btn_confirm = WidgetNode::button("Confirm", 0x007AA2F7, 0x00FFFFFF);

    let footer = WidgetNode::container(FlexStyle {
        direction: FlexDirection::Row,
        justify_content: JustifyContent::FlexEnd,
        gap: 8.0,
        width: Dimension::Points(260.0),
        height: Dimension::Points(32.0),
        ..Default::default()
    }).with_children(vec![btn_cancel, btn_confirm]);

    let modal = WidgetNode::container(FlexStyle {
        direction: FlexDirection::Column,
        gap: 12.0,
        padding: Edges::all(16.0),
        width: Dimension::Points(300.0),
        height: Dimension::Points(180.0),
        ..Default::default()
    })
    .with_background(0x001F2335)
    .with_border_and_corners(0x003B4261, 1.0, 8.0)
    .with_children(vec![header, input, footer]);

    let mut tree = WidgetTree::new(modal, 400.0, 300.0);
    tree.update_layout();

    let root_rect = tree.root.layout_rect();
    assert_eq!(root_rect.width, 300.0);
    assert_eq!(root_rect.height, 180.0);

    // Hit testing: point (20, 20) is inside root modal container
    let hit_root = tree.on_mouse_click(20.0, 20.0);
    assert!(hit_root.is_some());

    // Hit test outside modal should return None
    let hit_outside = tree.on_mouse_click(350.0, 250.0);
    assert!(hit_outside.is_none());

    // Mouse hover dispatch
    let hover_changed = tree.on_mouse_move(20.0, 20.0);
    assert!(!hover_changed); // Container doesn't change hover state
}

#[test]
fn test_retained_ui_rendering_to_framebuffer() {
    let mut fb = Framebuffer::new(320, 200);
    fb.clear(0x001A1B26);

    let title = WidgetNode::text("Rustext Reactive GUI", 0x007AA2F7);
    let btn = WidgetNode::button("Click Me", 0x0024283B, 0x00C0CAF5);

    let container = WidgetNode::container(FlexStyle {
        direction: FlexDirection::Column,
        padding: Edges::all(12.0),
        gap: 10.0,
        width: Dimension::Points(200.0),
        height: Dimension::Points(120.0),
        ..Default::default()
    })
    .with_background(0x0016161E)
    .with_border_and_corners(0x007AA2F7, 1.0, 6.0)
    .with_children(vec![title, btn]);

    let mut tree = WidgetTree::new(container, 320.0, 200.0);
    tree.render(&mut fb);

    // Check that container was painted inside (non-background pixel at 20, 20)
    let p_container = fb.get_pixel(20, 20);
    assert_ne!(p_container, 0x001A1B26);

    // Check that button text or background was painted inside button region
    let p_btn = fb.get_pixel(30, 60);
    assert_ne!(p_btn, 0x001A1B26);
}
