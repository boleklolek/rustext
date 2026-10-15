//! Split editor state and pane management.
//! Zero third-party dependencies, purely standard library Rust.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    None,
    Vertical,   // Side-by-side (Left & Right)
    Horizontal, // Stacked (Top & Bottom)
}

#[derive(Debug, Clone)]
pub struct SplitState {
    pub direction: SplitDirection,
    pub active_pane: usize,       // 0 = Primary, 1 = Secondary
    pub primary_doc_idx: usize,   // Index into editor.documents for the primary pane
    pub secondary_doc_idx: usize, // Index into editor.documents for the secondary pane
    pub ratio: f32,               // Default 0.5 (equal split)
}

impl Default for SplitState {
    fn default() -> Self {
        Self::new()
    }
}

impl SplitState {
    /// Creates a new split state in single-pane mode.
    pub fn new() -> Self {
        Self {
            direction: SplitDirection::None,
            active_pane: 0,
            primary_doc_idx: 0,
            secondary_doc_idx: 0,
            ratio: 0.5,
        }
    }

    /// Returns whether the split view is active.
    #[inline]
    pub fn is_split(&self) -> bool {
        self.direction != SplitDirection::None
    }

    /// Toggles vertical split mode (Side-by-side).
    pub fn toggle_vertical(&mut self, total_docs: usize, current_active: usize) {
        if self.direction == SplitDirection::Vertical {
            self.direction = SplitDirection::None;
            self.active_pane = 0;
        } else {
            self.direction = SplitDirection::Vertical;
            self.active_pane = 1;
            self.primary_doc_idx = current_active;
            self.secondary_doc_idx = if total_docs > 1 {
                (current_active + 1) % total_docs
            } else {
                current_active
            };
        }
    }

    /// Toggles horizontal split mode (Stacked).
    pub fn toggle_horizontal(&mut self, total_docs: usize, current_active: usize) {
        if self.direction == SplitDirection::Horizontal {
            self.direction = SplitDirection::None;
            self.active_pane = 0;
        } else {
            self.direction = SplitDirection::Horizontal;
            self.active_pane = 1;
            self.primary_doc_idx = current_active;
            self.secondary_doc_idx = if total_docs > 1 {
                (current_active + 1) % total_docs
            } else {
                current_active
            };
        }
    }

    /// Closes the secondary pane and restores single-pane mode.
    pub fn close_split(&mut self) {
        self.direction = SplitDirection::None;
        self.active_pane = 0;
    }

    /// Switches focus to the opposite pane.
    pub fn switch_pane(&mut self) -> usize {
        if self.is_split() {
            self.active_pane = if self.active_pane == 0 { 1 } else { 0 };
        }
        self.active_pane
    }

    /// Sets the focused pane (0 or 1).
    pub fn set_active_pane(&mut self, pane: usize) {
        if pane <= 1 {
            self.active_pane = pane;
        }
    }

    /// Returns which document index should be considered active given the fallback primary document index.
    #[inline]
    pub fn active_doc_index(&self, fallback_primary: usize) -> usize {
        if self.is_split() {
            if self.active_pane == 1 {
                self.secondary_doc_idx
            } else {
                self.primary_doc_idx
            }
        } else {
            fallback_primary
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_state_transitions() {
        let mut split = SplitState::new();
        assert!(!split.is_split());
        assert_eq!(split.active_pane, 0);

        // Toggle vertical with 2 docs (active = 0)
        split.toggle_vertical(2, 0);
        assert!(split.is_split());
        assert_eq!(split.direction, SplitDirection::Vertical);
        assert_eq!(split.active_pane, 1);
        assert_eq!(split.secondary_doc_idx, 1);
        assert_eq!(split.active_doc_index(0), 1);

        // Switch pane
        split.switch_pane();
        assert_eq!(split.active_pane, 0);
        assert_eq!(split.active_doc_index(0), 0);

        // Toggle vertical again should close
        split.toggle_vertical(2, 0);
        assert!(!split.is_split());
        assert_eq!(split.direction, SplitDirection::None);
        assert_eq!(split.active_pane, 0);
    }

    #[test]
    fn test_split_horizontal_and_close() {
        let mut split = SplitState::new();
        split.toggle_horizontal(3, 1);
        assert!(split.is_split());
        assert_eq!(split.direction, SplitDirection::Horizontal);
        assert_eq!(split.secondary_doc_idx, 2);

        split.close_split();
        assert!(!split.is_split());
        assert_eq!(split.direction, SplitDirection::None);
        assert_eq!(split.active_pane, 0);
    }
}
