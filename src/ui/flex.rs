//! Flexible Box Layout Engine (Flexbox) for Rustext.
//! Pure standard-library Rust with zero third-party dependencies.
//! Implements multi-line wrapping, flex grow, flex shrink, alignment, justification, gaps, and recursive nesting.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Dimension {
    Auto,
    Points(f32),
    Percent(f32),
}

impl Dimension {
    #[inline]
    pub fn resolve(&self, parent_size: f32) -> Option<f32> {
        match *self {
            Dimension::Auto => None,
            Dimension::Points(pts) => Some(pts.max(0.0)),
            Dimension::Percent(pct) => Some((parent_size * (pct / 100.0)).max(0.0)),
        }
    }

    #[inline]
    pub fn resolve_or(&self, parent_size: f32, default: f32) -> f32 {
        self.resolve(parent_size).unwrap_or(default)
    }

    #[inline]
    pub fn is_auto(&self) -> bool {
        matches!(self, Dimension::Auto)
    }
}

impl Default for Dimension {
    fn default() -> Self {
        Dimension::Auto
    }
}

/// 4-edge spacing representation for padding, margins, and borders.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Edges {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Edges {
    pub const fn zero() -> Self {
        Self { left: 0.0, right: 0.0, top: 0.0, bottom: 0.0 }
    }

    pub const fn all(val: f32) -> Self {
        Self { left: val, right: val, top: val, bottom: val }
    }

    pub const fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }

    pub const fn trbl(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self { top, right, bottom, left }
    }

    #[inline]
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    #[inline]
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

/// 2D rectangular layout box with spatial query helpers.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    pub const fn zero() -> Self {
        Self { x: 0.0, y: 0.0, width: 0.0, height: 0.0 }
    }

    #[inline]
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    #[inline]
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    #[inline]
    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.right() && py >= self.y && py <= self.bottom()
    }

    #[inline]
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

impl FlexDirection {
    #[inline]
    pub fn is_row(self) -> bool {
        matches!(self, FlexDirection::Row | FlexDirection::RowReverse)
    }

    #[inline]
    pub fn is_column(self) -> bool {
        matches!(self, FlexDirection::Column | FlexDirection::ColumnReverse)
    }

    #[inline]
    pub fn is_reverse(self) -> bool {
        matches!(self, FlexDirection::RowReverse | FlexDirection::ColumnReverse)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignItems {
    FlexStart,
    FlexEnd,
    Center,
    #[default]
    Stretch,
    Baseline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignSelf {
    #[default]
    Auto,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    Baseline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    Stretch,
}

/// Comprehensive Flexbox styling rules.
#[derive(Debug, Clone, PartialEq)]
pub struct FlexStyle {
    pub direction: FlexDirection,
    pub wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_self: AlignSelf,
    pub align_content: AlignContent,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub max_width: Dimension,
    pub min_height: Dimension,
    pub max_height: Dimension,
    pub padding: Edges,
    pub margin: Edges,
    pub gap: f32,
    pub row_gap: Option<f32>,
    pub col_gap: Option<f32>,
}

impl Default for FlexStyle {
    fn default() -> Self {
        Self {
            direction: FlexDirection::Row,
            wrap: FlexWrap::NoWrap,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,
            align_self: AlignSelf::Auto,
            align_content: AlignContent::FlexStart,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::Auto,
            width: Dimension::Auto,
            height: Dimension::Auto,
            min_width: Dimension::Auto,
            max_width: Dimension::Auto,
            min_height: Dimension::Auto,
            max_height: Dimension::Auto,
            padding: Edges::zero(),
            margin: Edges::zero(),
            gap: 0.0,
            row_gap: None,
            col_gap: None,
        }
    }
}

impl FlexStyle {
    #[inline]
    pub fn col_gap(&self) -> f32 {
        self.col_gap.unwrap_or(self.gap)
    }

    #[inline]
    pub fn row_gap(&self) -> f32 {
        self.row_gap.unwrap_or(self.gap)
    }
}

/// Internal item metadata during flex line calculation.
#[derive(Debug, Clone)]
struct FlexLineItem {
    child_idx: usize,
    main_size: f32,
    cross_size: f32,
    main_margin: f32,
    cross_margin: f32,
    flex_grow: f32,
    flex_shrink: f32,
    align_self: AlignSelf,
}

/// A line of flex items for single or multi-line flex containers.
#[derive(Debug, Default)]
struct FlexLine {
    items: Vec<FlexLineItem>,
    line_cross_size: f32,
}

/// Node within the hierarchical flexible box layout tree.
#[derive(Debug, Clone)]
pub struct FlexNode {
    pub id: usize,
    pub style: FlexStyle,
    pub children: Vec<FlexNode>,
    pub layout_rect: Rect,
    pub is_dirty: bool,
}

impl FlexNode {
    /// Creates a new FlexNode with default flex styling.
    pub fn new(id: usize) -> Self {
        Self {
            id,
            style: FlexStyle::default(),
            children: Vec::new(),
            layout_rect: Rect::zero(),
            is_dirty: true,
        }
    }

    /// Builder helper to set the entire style.
    pub fn with_style(mut self, style: FlexStyle) -> Self {
        self.style = style;
        self.is_dirty = true;
        self
    }

    /// Builder helper to set direction.
    pub fn with_direction(mut self, direction: FlexDirection) -> Self {
        self.style.direction = direction;
        self.is_dirty = true;
        self
    }

    /// Builder helper to set flex wrap.
    pub fn with_wrap(mut self, wrap: FlexWrap) -> Self {
        self.style.wrap = wrap;
        self.is_dirty = true;
        self
    }

    /// Builder helper to set justify content.
    pub fn with_justify_content(mut self, justify: JustifyContent) -> Self {
        self.style.justify_content = justify;
        self.is_dirty = true;
        self
    }

    /// Builder helper to set align items.
    pub fn with_align_items(mut self, align: AlignItems) -> Self {
        self.style.align_items = align;
        self.is_dirty = true;
        self
    }

    /// Builder helper to set flex grow.
    pub fn with_flex_grow(mut self, grow: f32) -> Self {
        self.style.flex_grow = grow;
        self.is_dirty = true;
        self
    }

    /// Builder helper to set flex shrink.
    pub fn with_flex_shrink(mut self, shrink: f32) -> Self {
        self.style.flex_shrink = shrink;
        self.is_dirty = true;
        self
    }

    /// Builder helper to set flex basis.
    pub fn with_flex_basis(mut self, basis: Dimension) -> Self {
        self.style.flex_basis = basis;
        self.is_dirty = true;
        self
    }

    /// Builder helper to set width and height in points.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.style.width = Dimension::Points(width);
        self.style.height = Dimension::Points(height);
        self.is_dirty = true;
        self
    }

    /// Builder helper to set width.
    pub fn with_width(mut self, width: Dimension) -> Self {
        self.style.width = width;
        self.is_dirty = true;
        self
    }

    /// Builder helper to set height.
    pub fn with_height(mut self, height: Dimension) -> Self {
        self.style.height = height;
        self.is_dirty = true;
        self
    }

    /// Builder helper to set padding.
    pub fn with_padding(mut self, padding: Edges) -> Self {
        self.style.padding = padding;
        self.is_dirty = true;
        self
    }

    /// Builder helper to set margin.
    pub fn with_margin(mut self, margin: Edges) -> Self {
        self.style.margin = margin;
        self.is_dirty = true;
        self
    }

    /// Builder helper to set gap.
    pub fn with_gap(mut self, gap: f32) -> Self {
        self.style.gap = gap;
        self.is_dirty = true;
        self
    }

    /// Appends a child node.
    pub fn add_child(&mut self, child: FlexNode) {
        self.children.push(child);
        self.is_dirty = true;
    }

    /// Appends multiple children.
    pub fn with_children(mut self, children: Vec<FlexNode>) -> Self {
        self.children = children;
        self.is_dirty = true;
        self
    }

    /// Marks the node and its ancestors as dirty for relayout.
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    /// Computes the complete flexbox layout across the tree using available parent dimensions.
    pub fn compute_layout(&mut self, available_width: f32, available_height: f32) {
        self.compute_layout_at(0.0, 0.0, available_width, available_height);
    }

    /// Internal flexbox root layout algorithm.
    pub fn compute_layout_at(
        &mut self,
        origin_x: f32,
        origin_y: f32,
        parent_width: f32,
        parent_height: f32,
    ) {
        // 1. Resolve container dimensions
        let resolved_width = self.style.width.resolve(parent_width);
        let resolved_height = self.style.height.resolve(parent_height);

        let mut container_w = resolved_width.unwrap_or(parent_width).max(0.0);
        let mut container_h = resolved_height.unwrap_or(parent_height).max(0.0);

        // Clamp by min / max constraints
        if let Some(min_w) = self.style.min_width.resolve(parent_width) {
            container_w = container_w.max(min_w);
        }
        if let Some(max_w) = self.style.max_width.resolve(parent_width) {
            container_w = container_w.min(max_w);
        }
        if let Some(min_h) = self.style.min_height.resolve(parent_height) {
            container_h = container_h.max(min_h);
        }
        if let Some(max_h) = self.style.max_height.resolve(parent_height) {
            container_h = container_h.min(max_h);
        }

        // Apply margins to positioning
        let x = origin_x + self.style.margin.left;
        let y = origin_y + self.style.margin.top;

        self.layout_rect = Rect::new(x, y, container_w, container_h);

        self.layout_children(resolved_width.is_none(), resolved_height.is_none());
    }

    /// Resolves the layout of children within this container's layout_rect.
    pub fn layout_children(&mut self, auto_w: bool, auto_h: bool) {
        if self.children.is_empty() {
            self.is_dirty = false;
            return;
        }

        let is_row = self.style.direction.is_row();
        let is_reverse = self.style.direction.is_reverse();

        let container_w = self.layout_rect.width;
        let container_h = self.layout_rect.height;
        let x = self.layout_rect.x;
        let y = self.layout_rect.y;

        // 2. Compute inner content area subtracting padding
        let inner_w = (container_w - self.style.padding.horizontal()).max(0.0);
        let inner_h = (container_h - self.style.padding.vertical()).max(0.0);
        let inner_x = x + self.style.padding.left;
        let inner_y = y + self.style.padding.top;

        let (inner_main, inner_cross, gap) = if is_row {
            (inner_w, inner_h, self.style.col_gap())
        } else {
            (inner_h, inner_w, self.style.row_gap())
        };

        // 3. Measure children's hypothetical main and cross sizes
        let mut line_items = Vec::with_capacity(self.children.len());
        for (idx, child) in self.children.iter().enumerate() {
            let child_w = child.style.width.resolve(inner_w);
            let child_h = child.style.height.resolve(inner_h);

            let basis = child.style.flex_basis.resolve(if is_row { inner_w } else { inner_h });

            let (main_size, cross_size, main_margin, cross_margin) = if is_row {
                let m_size = basis.or(child_w).unwrap_or(0.0);
                let c_size = child_h.unwrap_or(0.0);
                (m_size, c_size, child.style.margin.horizontal(), child.style.margin.vertical())
            } else {
                let m_size = basis.or(child_h).unwrap_or(0.0);
                let c_size = child_w.unwrap_or(0.0);
                (m_size, c_size, child.style.margin.vertical(), child.style.margin.horizontal())
            };

            let align = if child.style.align_self != AlignSelf::Auto {
                child.style.align_self
            } else {
                match self.style.align_items {
                    AlignItems::FlexStart => AlignSelf::FlexStart,
                    AlignItems::FlexEnd => AlignSelf::FlexEnd,
                    AlignItems::Center => AlignSelf::Center,
                    AlignItems::Stretch => AlignSelf::Stretch,
                    AlignItems::Baseline => AlignSelf::Baseline,
                }
            };

            line_items.push(FlexLineItem {
                child_idx: idx,
                main_size,
                cross_size,
                main_margin,
                cross_margin,
                flex_grow: child.style.flex_grow,
                flex_shrink: child.style.flex_shrink,
                align_self: align,
            });
        }

        // 4. Collect items into flex lines (respecting FlexWrap)
        let mut lines: Vec<FlexLine> = Vec::new();
        let mut current_line = FlexLine::default();
        let mut current_line_main = 0.0f32;

        for item in line_items {
            let item_outer_main = item.main_size + item.main_margin;
            let gap_needed = if current_line.items.is_empty() { 0.0 } else { gap };

            let would_overflow = current_line_main + gap_needed + item_outer_main > inner_main;
            let can_wrap = self.style.wrap != FlexWrap::NoWrap;

            if can_wrap && would_overflow && !current_line.items.is_empty() {
                lines.push(current_line);
                current_line = FlexLine::default();
                current_line_main = 0.0;
            }

            let effective_gap = if current_line.items.is_empty() { 0.0 } else { gap };
            current_line_main += effective_gap + item_outer_main;
            current_line.line_cross_size = current_line.line_cross_size.max(item.cross_size + item.cross_margin);
            current_line.items.push(item);
        }

        if !current_line.items.is_empty() {
            lines.push(current_line);
        }

        // If container cross size is fixed and there's 1 line with Stretch, expand line cross size
        if lines.len() == 1 && (if is_row { !auto_h } else { !auto_w }) {
            lines[0].line_cross_size = lines[0].line_cross_size.max(inner_cross);
        }

        // 5. Layout lines across cross axis
        let total_lines_cross: f32 = lines.iter().map(|l| l.line_cross_size).sum();
        let total_cross_gaps = if lines.len() > 1 {
            (lines.len() - 1) as f32 * if is_row { self.style.row_gap() } else { self.style.col_gap() }
        } else {
            0.0
        };

        let free_cross_space = (inner_cross - (total_lines_cross + total_cross_gaps)).max(0.0);
        let mut cross_cursor = 0.0f32;

        // AlignContent distribution
        let (initial_cross_offset, line_cross_spacing) = match self.style.align_content {
            AlignContent::FlexEnd => (free_cross_space, 0.0),
            AlignContent::Center => (free_cross_space / 2.0, 0.0),
            AlignContent::SpaceBetween => {
                if lines.len() > 1 {
                    (0.0, free_cross_space / (lines.len() - 1) as f32)
                } else {
                    (0.0, 0.0)
                }
            }
            AlignContent::SpaceAround => {
                if !lines.is_empty() {
                    let per_line = free_cross_space / lines.len() as f32;
                    (per_line / 2.0, per_line)
                } else {
                    (0.0, 0.0)
                }
            }
            AlignContent::Stretch => {
                // Distribute extra space among lines if auto
                let extra_per_line = if !lines.is_empty() { free_cross_space / lines.len() as f32 } else { 0.0 };
                for line in &mut lines {
                    line.line_cross_size += extra_per_line;
                }
                (0.0, 0.0)
            }
            AlignContent::FlexStart => (0.0, 0.0),
        };

        if self.style.wrap == FlexWrap::WrapReverse {
            // Reverse line cross positions
            lines.reverse();
        }

        cross_cursor += initial_cross_offset;

        // 6. Resolve main axis distribution per line (grow, shrink, justify)
        for line in &mut lines {
            let num_items = line.items.len();
            if num_items == 0 {
                continue;
            }

            let gaps_main = (num_items - 1) as f32 * gap;
            let sum_main_basis: f32 = line.items.iter().map(|it| it.main_size + it.main_margin).sum();
            let free_main = inner_main - (sum_main_basis + gaps_main);

            // Grow or Shrink
            if free_main > 0.0 {
                let total_grow: f32 = line.items.iter().map(|it| it.flex_grow).sum();
                if total_grow > 0.0 {
                    for it in &mut line.items {
                        if it.flex_grow > 0.0 {
                            let share = (it.flex_grow / total_grow) * free_main;
                            it.main_size += share;
                        }
                    }
                }
            } else if free_main < 0.0 {
                let total_shrink_scaled: f32 = line.items.iter().map(|it| it.flex_shrink * it.main_size).sum();
                if total_shrink_scaled > 0.0 {
                    for it in &mut line.items {
                        if it.flex_shrink > 0.0 {
                            let reduction = (-free_main) * (it.flex_shrink * it.main_size) / total_shrink_scaled;
                            it.main_size = (it.main_size - reduction).max(0.0);
                        }
                    }
                }
            }

            // Recalculate remaining main free space after grow/shrink
            let final_sum_main: f32 = line.items.iter().map(|it| it.main_size + it.main_margin).sum();
            let remaining_free_main = (inner_main - (final_sum_main + gaps_main)).max(0.0);

            let (mut main_cursor, item_spacing) = match self.style.justify_content {
                JustifyContent::FlexEnd => (remaining_free_main, 0.0),
                JustifyContent::Center => (remaining_free_main / 2.0, 0.0),
                JustifyContent::SpaceBetween => {
                    if num_items > 1 {
                        (0.0, remaining_free_main / (num_items - 1) as f32)
                    } else {
                        (0.0, 0.0)
                    }
                }
                JustifyContent::SpaceAround => {
                    let per_item = remaining_free_main / num_items as f32;
                    (per_item / 2.0, per_item)
                }
                JustifyContent::SpaceEvenly => {
                    let step = remaining_free_main / (num_items + 1) as f32;
                    (step, step)
                }
                JustifyContent::FlexStart => (0.0, 0.0),
            };

            // Order items if reversed
            let ordered_indices: Vec<usize> = if is_reverse {
                (0..num_items).rev().collect()
            } else {
                (0..num_items).collect()
            };

            for &idx in &ordered_indices {
                let it = &line.items[idx];
                let child = &mut self.children[it.child_idx];

                // Determine cross size and offset according to align_self / align_items
                let item_cross_size = match it.align_self {
                    AlignSelf::Stretch => {
                        // If child doesn't have an explicit cross size, stretch to fill line
                        let has_explicit_cross = if is_row {
                            child.style.height.resolve(inner_h).is_some()
                        } else {
                            child.style.width.resolve(inner_w).is_some()
                        };
                        if has_explicit_cross {
                            it.cross_size
                        } else {
                            (line.line_cross_size - it.cross_margin).max(0.0)
                        }
                    }
                    _ => it.cross_size,
                };

                let cross_offset = match it.align_self {
                    AlignSelf::FlexEnd => line.line_cross_size - (item_cross_size + it.cross_margin),
                    AlignSelf::Center => (line.line_cross_size - (item_cross_size + it.cross_margin)) / 2.0,
                    _ => 0.0,
                };

                let (final_x, final_y, final_w, final_h) = if is_row {
                    let item_x = inner_x + main_cursor + child.style.margin.left;
                    let item_y = inner_y + cross_cursor + cross_offset + child.style.margin.top;
                    (item_x, item_y, it.main_size, item_cross_size)
                } else {
                    let item_x = inner_x + cross_cursor + cross_offset + child.style.margin.left;
                    let item_y = inner_y + main_cursor + child.style.margin.top;
                    (item_x, item_y, item_cross_size, it.main_size)
                };

                // Assign layout rectangle and recurse down child hierarchy
                child.layout_rect = Rect::new(final_x, final_y, final_w, final_h);
                if !child.children.is_empty() {
                    child.layout_children(false, false);
                } else {
                    child.is_dirty = false;
                }

                let spacing = if matches!(self.style.justify_content, JustifyContent::SpaceEvenly) {
                    item_spacing
                } else {
                    0.0
                };
                let between = if matches!(self.style.justify_content, JustifyContent::SpaceBetween | JustifyContent::SpaceAround) {
                    item_spacing
                } else {
                    0.0
                };

                main_cursor += it.main_size + it.main_margin + gap + between + spacing;
            }

            let cross_gap = if is_row { self.style.row_gap() } else { self.style.col_gap() };
            cross_cursor += line.line_cross_size + cross_gap + line_cross_spacing;
        }

        // 7. Auto sizing (shrink-to-fit) if width or height was Auto
        if auto_w && !self.children.is_empty() {
            let max_r = self.children.iter().map(|c| c.layout_rect.right() + c.style.margin.right).fold(0.0f32, f32::max);
            self.layout_rect.width = (max_r - x + self.style.padding.right).max(0.0);
        }
        if auto_h && !self.children.is_empty() {
            let max_b = self.children.iter().map(|c| c.layout_rect.bottom() + c.style.margin.bottom).fold(0.0f32, f32::max);
            self.layout_rect.height = (max_b - y + self.style.padding.bottom).max(0.0);
        }

        self.is_dirty = false;
    }
}
