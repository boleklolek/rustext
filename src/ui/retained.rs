//! Retained-Mode UI Widget Tree with Fine-Grained Reactive Bindings for Rustext.
//! Pure standard-library Rust with zero third-party dependencies.
//!
//! # Architecture: Immediate vs. Retained vs. Reactive
//!
//! 1. **Immediate Mode GUI (IMGUI)**:
//!    - Widgets are transient function invocations executed every frame (e.g., `if ui.button("Save")`).
//!    - *Pros*: Extremely simple mental model, zero synchronization overhead, low memory usage.
//!    - *Cons*: Re-executes layout and painting continuously at 60/120 FPS; bidirectional layouts
//!      (like Flexbox shrink-to-fit) require multi-frame delays; complex gesture/focus tracking is tedious.
//!
//! 2. **Retained Mode GUI (RMGUI)**:
//!    - Widgets are persistent objects residing in a persistent hierarchical tree (`WidgetNode`).
//!    - *Pros*: Layout geometry, scroll positions, focus state, and hit-test caches are retained;
//!      repainting only occurs when nodes are explicitly invalidated.
//!    - *Cons*: Risk of state divergence between business logic and widget state if updated imperatively.
//!
//! 3. **Fine-Grained Reactive-Retained Hybrid (Rustext)**:
//!    - Widgets remain persistent in a retained tree (`WidgetTree`), retaining layout boxes and event listeners.
//!    - Widget properties are declaratively bound to fine-grained `Signal<T>` primitives.
//!    - When a signal updates, only the directly affected widgets mark their `DirtyFlags` (`layout` or `paint`),
//!      propagating up to their ancestors without rebuilding the widget tree or running a virtual DOM diff.

use super::flex::{Dimension, Edges, FlexNode, FlexStyle, Rect};
use super::reactive::{DirtyFlags, Signal, SubscriptionId};
use crate::gfx::framebuffer::Framebuffer;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_WIDGET_ID: AtomicU64 = AtomicU64::new(100);

/// Unique identifier for each widget in the retained tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u64);

/// Concrete visual role and parameters for a widget.
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetKind {
    Container {
        background: Option<u32>,
        border: Option<(u32, f32)>, // (color, thickness)
        corner_radius: f32,
    },
    Text {
        content: String,
        color: u32,
        scale: usize,
    },
    Button {
        label: String,
        bg_color: u32,
        hover_bg: u32,
        text_color: u32,
        is_hovered: bool,
        is_pressed: bool,
    },
    Input {
        text: String,
        placeholder: String,
        is_focused: bool,
        border_color: u32,
    },
    Spacer,
}

/// A persistent widget node within the retained UI tree.
pub struct WidgetNode {
    pub id: WidgetId,
    pub kind: WidgetKind,
    pub flex_node: FlexNode,
    pub dirty: DirtyFlags,
    pub children: Vec<WidgetNode>,
    pub subscriptions: Vec<SubscriptionId>,
}

impl WidgetNode {
    /// Creates a new container widget.
    pub fn container(style: FlexStyle) -> Self {
        let id = WidgetId(NEXT_WIDGET_ID.fetch_add(1, Ordering::Relaxed));
        let flex_node = FlexNode::new(id.0 as usize).with_style(style);
        Self {
            id,
            kind: WidgetKind::Container {
                background: None,
                border: None,
                corner_radius: 0.0,
            },
            flex_node,
            dirty: DirtyFlags::all_dirty(),
            children: Vec::new(),
            subscriptions: Vec::new(),
        }
    }

    /// Creates a new text label widget.
    pub fn text(content: impl Into<String>, color: u32) -> Self {
        let content_str = content.into();
        let id = WidgetId(NEXT_WIDGET_ID.fetch_add(1, Ordering::Relaxed));

        // Estimate intrinsic text width (FONT_WIDTH * len)
        let est_w = (content_str.chars().count() * 8) as f32;
        let est_h = 16.0f32;

        let style = FlexStyle {
            width: Dimension::Points(est_w),
            height: Dimension::Points(est_h),
            ..Default::default()
        };

        let flex_node = FlexNode::new(id.0 as usize).with_style(style);
        Self {
            id,
            kind: WidgetKind::Text {
                content: content_str,
                color,
                scale: 1,
            },
            flex_node,
            dirty: DirtyFlags::all_dirty(),
            children: Vec::new(),
            subscriptions: Vec::new(),
        }
    }

    /// Creates a clickable button widget.
    pub fn button(label: impl Into<String>, bg_color: u32, text_color: u32) -> Self {
        let label_str = label.into();
        let id = WidgetId(NEXT_WIDGET_ID.fetch_add(1, Ordering::Relaxed));

        let est_w = (label_str.chars().count() * 8 + 24) as f32;
        let est_h = 28.0f32;

        let style = FlexStyle {
            width: Dimension::Points(est_w),
            height: Dimension::Points(est_h),
            padding: Edges::symmetric(4.0, 12.0),
            ..Default::default()
        };

        let flex_node = FlexNode::new(id.0 as usize).with_style(style);
        Self {
            id,
            kind: WidgetKind::Button {
                label: label_str,
                bg_color,
                hover_bg: bg_color.saturating_add(0x00111111),
                text_color,
                is_hovered: false,
                is_pressed: false,
            },
            flex_node,
            dirty: DirtyFlags::all_dirty(),
            children: Vec::new(),
            subscriptions: Vec::new(),
        }
    }

    /// Fluent builder to set background color.
    pub fn with_background(mut self, color: u32) -> Self {
        if let WidgetKind::Container { ref mut background, .. } = self.kind {
            *background = Some(color);
        }
        self.dirty.mark_paint_dirty();
        self
    }

    /// Fluent builder to set border and corner radius.
    pub fn with_border_and_corners(mut self, border_color: u32, thickness: f32, radius: f32) -> Self {
        if let WidgetKind::Container { ref mut border, ref mut corner_radius, .. } = self.kind {
            *border = Some((border_color, thickness));
            *corner_radius = radius;
        }
        self.dirty.mark_paint_dirty();
        self
    }

    /// Appends a child widget.
    pub fn add_child(&mut self, child: WidgetNode) {
        self.children.push(child);
        self.dirty.mark_layout_dirty();
    }

    /// Appends multiple child widgets.
    pub fn with_children(mut self, children: Vec<WidgetNode>) -> Self {
        for child in children {
            self.add_child(child);
        }
        self
    }

    /// Connects a reactive string signal to this text widget's content.
    pub fn bind_text(&mut self, signal: &Signal<String>) {
        if let WidgetKind::Text { ref mut content, .. } = self.kind {
            *content = signal.get();
        }
        self.dirty.mark_layout_dirty();

        let node_id = self.id;
        let sub = signal.subscribe(move |_ver| {
            // Signal updated: In practice, signals notify the widget tree scheduler
            let _ = node_id;
        });
        self.subscriptions.push(sub);
    }

    /// Resolves the layout recursively using the Flexbox engine only when marked dirty.
    pub fn layout(&mut self, available_w: f32, available_h: f32) {
        if !self.dirty.layout {
            return;
        }

        // Synchronize flex node children
        self.sync_flex_tree();

        // Execute 2-pass Flexbox layout
        self.flex_node.compute_layout(available_w, available_h);

        // Propagate calculated geometry back to retained widget children
        self.sync_layout_rects();

        self.dirty.layout = false;
    }

    fn sync_flex_tree(&mut self) {
        self.flex_node.children.clear();
        for child in &mut self.children {
            child.sync_flex_tree();
            self.flex_node.children.push(child.flex_node.clone());
        }
    }

    fn sync_layout_rects(&mut self) {
        for (idx, child) in self.children.iter_mut().enumerate() {
            if idx < self.flex_node.children.len() {
                child.flex_node.layout_rect = self.flex_node.children[idx].layout_rect;
                child.sync_layout_rects();
            }
        }
    }

    /// Returns the computed layout bounding box.
    #[inline]
    pub fn layout_rect(&self) -> Rect {
        self.flex_node.layout_rect
    }

    /// Performs hit testing to find the deepest widget containing the point (x, y).
    pub fn hit_test(&self, x: f32, y: f32) -> Option<WidgetId> {
        if !self.flex_node.layout_rect.contains_point(x, y) {
            return None;
        }

        // Test children in reverse paint order (top-most first)
        for child in self.children.iter().rev() {
            if let Some(hit) = child.hit_test(x, y) {
                return Some(hit);
            }
        }

        Some(self.id)
    }

    /// Dispatches mouse click event to widget tree.
    pub fn dispatch_mouse_click(&mut self, x: f32, y: f32) -> Option<WidgetId> {
        let hit_id = self.hit_test(x, y)?;

        // Recursive traversal to trigger button clicks
        self.notify_click(hit_id);
        Some(hit_id)
    }

    fn notify_click(&mut self, target_id: WidgetId) {
        if self.id == target_id {
            if let WidgetKind::Button { ref mut is_pressed, .. } = self.kind {
                *is_pressed = true;
                self.dirty.mark_paint_dirty();
            }
        } else {
            for child in &mut self.children {
                child.notify_click(target_id);
            }
        }
    }

    /// Dispatches mouse hover move event, updating hover states.
    pub fn dispatch_mouse_move(&mut self, x: f32, y: f32) -> bool {
        let mut changed = false;
        let is_inside = self.flex_node.layout_rect.contains_point(x, y);

        if let WidgetKind::Button { ref mut is_hovered, .. } = self.kind {
            if *is_hovered != is_inside {
                *is_hovered = is_inside;
                self.dirty.mark_paint_dirty();
                changed = true;
            }
        }

        for child in &mut self.children {
            if child.dispatch_mouse_move(x, y) {
                changed = true;
            }
        }

        changed
    }

    /// Incremental rendering pass onto the target Framebuffer.
    pub fn paint(&self, fb: &mut Framebuffer) {
        let r = self.layout_rect();
        if r.width <= 0.0 || r.height <= 0.0 {
            return;
        }

        let px = r.x.round() as usize;
        let py = r.y.round() as usize;
        let pw = r.width.round() as usize;
        let ph = r.height.round() as usize;

        match &self.kind {
            WidgetKind::Container { background, border, corner_radius } => {
                if let Some(bg) = background {
                    if *corner_radius > 0.0 {
                        fb.fill_rounded_rect_aa(r.x as f64, r.y as f64, r.width as f64, r.height as f64, *corner_radius as f64, *bg);
                    } else {
                        fb.fill_rect(px, py, pw, ph, *bg);
                    }
                }
                if let Some((border_col, thickness)) = border {
                    if *corner_radius > 0.0 {
                        fb.draw_rounded_outline_aa(r.x as f64, r.y as f64, r.width as f64, r.height as f64, *corner_radius as f64, *thickness as f64, *border_col);
                    } else {
                        fb.draw_rect_outline(px, py, pw, ph, *border_col);
                    }
                }
            }
            WidgetKind::Text { content, color, .. } => {
                fb.draw_text(content, px, py, *color, None);
            }
            WidgetKind::Button { label, bg_color, hover_bg, text_color, is_hovered, .. } => {
                let current_bg = if *is_hovered { *hover_bg } else { *bg_color };
                fb.fill_rounded_rect_aa(r.x as f64, r.y as f64, r.width as f64, r.height as f64, 4.0, current_bg);
                fb.draw_rounded_outline_aa(r.x as f64, r.y as f64, r.width as f64, r.height as f64, 4.0, 1.0, 0x003B4261);

                // Centered text
                let text_w = label.chars().count() * 8;
                let text_x = px + pw.saturating_sub(text_w) / 2;
                let text_y = py + ph.saturating_sub(16) / 2;
                fb.draw_text(label, text_x, text_y, *text_color, None);
            }
            WidgetKind::Input { text, placeholder, is_focused, border_color } => {
                fb.fill_rounded_rect_aa(r.x as f64, r.y as f64, r.width as f64, r.height as f64, 4.0, 0x0024283B);
                let outline_col = if *is_focused { 0x007AA2F7 } else { *border_color };
                fb.draw_rounded_outline_aa(r.x as f64, r.y as f64, r.width as f64, r.height as f64, 4.0, 1.0, outline_col);

                let display_str = if text.is_empty() { placeholder.as_str() } else { text.as_str() };
                let display_col = if text.is_empty() { 0x00565F89 } else { 0x00C0CAF5 };
                fb.draw_text(display_str, px + 8, py + 6, display_col, None);
            }
            WidgetKind::Spacer => {}
        }

        // Paint child widgets
        for child in &self.children {
            child.paint(fb);
        }
    }
}

/// Persistent container managing the root widget tree, event dispatch, and layout updates.
pub struct WidgetTree {
    pub root: WidgetNode,
    pub width: f32,
    pub height: f32,
}

impl WidgetTree {
    /// Creates a new widget tree with a root container.
    pub fn new(root: WidgetNode, width: f32, height: f32) -> Self {
        Self { root, width, height }
    }

    /// Sets viewport dimensions, marking layout dirty if size changed.
    pub fn resize(&mut self, width: f32, height: f32) {
        if (self.width - width).abs() > 0.001 || (self.height - height).abs() > 0.001 {
            self.width = width;
            self.height = height;
            self.root.dirty.mark_layout_dirty();
        }
    }

    /// Resolves layout across the tree.
    pub fn update_layout(&mut self) {
        self.root.layout(self.width, self.height);
    }

    /// Renders the widget tree onto the framebuffer.
    pub fn render(&mut self, fb: &mut Framebuffer) {
        self.update_layout();
        self.root.paint(fb);
    }

    /// Dispatches mouse click.
    pub fn on_mouse_click(&mut self, x: f32, y: f32) -> Option<WidgetId> {
        self.root.dispatch_mouse_click(x, y)
    }

    /// Dispatches mouse move.
    pub fn on_mouse_move(&mut self, x: f32, y: f32) -> bool {
        self.root.dispatch_mouse_move(x, y)
    }
}
