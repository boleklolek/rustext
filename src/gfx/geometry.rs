//! UI Geometries and bounding boxes for interactive editor components.

/// Geometry and bounding box for editor scrollbars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollbarGeometry {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub thumb_y: usize,
    pub thumb_height: usize,
    pub visible_lines: usize,
    pub total_lines: usize,
    pub max_scroll: usize,
}

impl ScrollbarGeometry {
    /// Returns whether the point (x, y) is inside the scrollbar track.
    pub fn contains_point(&self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// Returns whether the point (x, y) is inside the scrollbar draggable thumb.
    pub fn thumb_contains_point(&self, x: usize, y: usize) -> bool {
        self.contains_point(x, y) && y >= self.thumb_y && y < self.thumb_y + self.thumb_height
    }

    /// Translates a target thumb Y pixel position into the corresponding document scroll row.
    pub fn row_for_thumb_y(&self, target_thumb_y: usize) -> usize {
        let scrollable_h = self.height.saturating_sub(self.thumb_height);
        if scrollable_h == 0 || self.max_scroll == 0 {
            0
        } else {
            let clamped_y = target_thumb_y.clamp(self.y, self.y + scrollable_h);
            let rel_y = clamped_y.saturating_sub(self.y);
            let ratio = rel_y as f32 / scrollable_h as f32;
            (ratio * self.max_scroll as f32).round() as usize
        }
    }
}

/// Geometry and bounding box for the floating autocomplete popup menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutocompleteGeometry {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub item_height: usize,
    pub visible_items: usize,
    pub total_items: usize,
}

impl AutocompleteGeometry {
    /// Returns whether the point (px, py) is inside the autocomplete popup container.
    pub fn contains_point(&self, px: usize, py: usize) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }

    /// Translates a mouse click Y pixel position into an item index within the candidate list.
    pub fn item_index_at(&self, py: usize, scroll_offset: usize) -> Option<usize> {
        let pad_y = 4;
        if py < self.y + pad_y || py >= self.y + self.height.saturating_sub(pad_y) || self.item_height == 0 {
            return None;
        }
        let row = (py - self.y - pad_y) / self.item_height;
        let idx = scroll_offset + row;
        if idx < self.total_items {
            Some(idx)
        } else {
            None
        }
    }
}

/// Geometry and bounding box for the interactive minimap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinimapGeometry {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub slider_y: usize,
    pub slider_height: usize,
    pub visible_lines: usize,
    pub total_lines: usize,
}

impl MinimapGeometry {
    /// Returns whether the point (x, y) is inside the minimap container.
    pub fn contains_point(&self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// Returns whether the point (x, y) is inside the viewport highlight slider.
    pub fn slider_contains_point(&self, x: usize, y: usize) -> bool {
        self.contains_point(x, y) && y >= self.slider_y && y < self.slider_y + self.slider_height
    }

    /// Translates a mouse click Y pixel coordinate into a document line index.
    pub fn line_for_click_y(&self, click_y: usize) -> usize {
        if self.height == 0 || self.total_lines == 0 {
            0
        } else {
            let rel_y = click_y.saturating_sub(self.y);
            (rel_y * self.total_lines) / self.height
        }
    }

    /// Translates a target slider Y pixel position into the corresponding document scroll row.
    pub fn row_for_slider_y(&self, target_slider_y: usize) -> usize {
        let scrollable_h = self.height.saturating_sub(self.slider_height);
        let max_scroll = self.total_lines.saturating_sub(self.visible_lines);
        if scrollable_h == 0 || max_scroll == 0 {
            0
        } else {
            let clamped_y = target_slider_y.clamp(self.y, self.y + scrollable_h);
            let rel_y = clamped_y.saturating_sub(self.y);
            let ratio = rel_y as f32 / scrollable_h as f32;
            (ratio * max_scroll as f32).round() as usize
        }
    }
}

/// Geometry and coordinates for a split editor pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneGeometry {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub is_active: bool,
}

impl PaneGeometry {
    pub fn contains_point(&self, x: usize, y: usize) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}
