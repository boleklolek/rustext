//! Jump List navigation history (Alt+Left / Alt+Right).
//! Pure standard-library Rust with zero third-party dependencies.

use std::path::PathBuf;

/// A recorded cursor location in the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpLocation {
    pub file_path: Option<PathBuf>,
    pub char_offset: usize,
    pub line: usize,
    pub col: usize,
}

impl JumpLocation {
    pub fn new(file_path: Option<PathBuf>, char_offset: usize, line: usize, col: usize) -> Self {
        Self {
            file_path,
            char_offset,
            line,
            col,
        }
    }
}

/// Manages backward and forward jump locations like VS Code and Vim jumplists.
#[derive(Debug, Clone)]
pub struct JumpList {
    pub backward: Vec<JumpLocation>,
    pub forward: Vec<JumpLocation>,
    pub max_entries: usize,
}

impl Default for JumpList {
    fn default() -> Self {
        Self::new(64)
    }
}

impl JumpList {
    pub fn new(max_entries: usize) -> Self {
        Self {
            backward: Vec::with_capacity(max_entries),
            forward: Vec::with_capacity(max_entries),
            max_entries: max_entries.max(8),
        }
    }

    /// Records a location before jumping away.
    /// Deduplicates if the last entry is at the same file and line.
    pub fn record(&mut self, loc: JumpLocation) {
        if let Some(last) = self.backward.last() {
            if last.file_path == loc.file_path && last.line == loc.line {
                return;
            }
        }
        if self.backward.len() >= self.max_entries {
            self.backward.remove(0);
        }
        self.backward.push(loc);
        self.forward.clear();
    }

    /// Jumps back: pushes current location to forward stack, pops from backward stack.
    pub fn jump_back(&mut self, current: JumpLocation) -> Option<JumpLocation> {
        let prev = self.backward.pop()?;
        self.forward.push(current);
        Some(prev)
    }

    /// Jumps forward: pushes current location to backward stack, pops from forward stack.
    pub fn jump_forward(&mut self, current: JumpLocation) -> Option<JumpLocation> {
        let next = self.forward.pop()?;
        self.backward.push(current);
        Some(next)
    }

    pub fn can_jump_back(&self) -> bool {
        !self.backward.is_empty()
    }

    pub fn can_jump_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    pub fn clear(&mut self) {
        self.backward.clear();
        self.forward.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jump_list_record_and_travel() {
        let mut jl = JumpList::new(10);
        let loc1 = JumpLocation::new(Some(PathBuf::from("a.rs")), 10, 1, 2);
        let loc2 = JumpLocation::new(Some(PathBuf::from("b.rs")), 50, 5, 0);
        let loc3 = JumpLocation::new(Some(PathBuf::from("b.rs")), 120, 12, 4);

        jl.record(loc1.clone());
        jl.record(loc2.clone());

        // From loc3, jump back -> should return loc2
        let back1 = jl.jump_back(loc3.clone()).unwrap();
        assert_eq!(back1, loc2);
        assert!(jl.can_jump_forward());

        // From loc2, jump back -> should return loc1
        let back2 = jl.jump_back(loc2.clone()).unwrap();
        assert_eq!(back2, loc1);

        // Cannot jump back further
        assert!(!jl.can_jump_back());

        // Jump forward -> should return loc2
        let fwd1 = jl.jump_forward(loc1.clone()).unwrap();
        assert_eq!(fwd1, loc2);

        // Jump forward -> should return loc3
        let fwd2 = jl.jump_forward(loc2.clone()).unwrap();
        assert_eq!(fwd2, loc3);
    }

    #[test]
    fn test_jump_list_deduplication() {
        let mut jl = JumpList::new(10);
        let loc1 = JumpLocation::new(Some(PathBuf::from("a.rs")), 10, 1, 2);
        let loc1_duplicate = JumpLocation::new(Some(PathBuf::from("a.rs")), 15, 1, 7);

        jl.record(loc1.clone());
        jl.record(loc1_duplicate);
        assert_eq!(jl.backward.len(), 1);
    }
}
