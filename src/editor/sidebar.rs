//! Project File Explorer Sidebar.
//! Pure standard-library Rust with zero third-party dependencies.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Click action triggered by mouse interaction on the sidebar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarClickResult {
    None,
    ToggleFolder,
    OpenFile(PathBuf),
    NewFile,
    Refresh,
    CloseSidebar,
}

/// Represents an item in the flattened visible tree list of the sidebar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileItem {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub depth: usize,
}

impl FileItem {
    /// Returns a subtle color hint for the file type based on theme syntax colors.
    pub fn color_hint(&self, colors: &crate::gfx::ThemeColors) -> u32 {
        if self.is_dir {
            colors.accent
        } else {
            let ext = self.path.extension().and_then(|e| e.to_str()).unwrap_or("");
            match ext.to_lowercase().as_str() {
                "rs" => colors.syntax_number, // orange
                "toml" | "ini" | "json" | "yaml" | "yml" => colors.syntax_type, // cyan
                "md" | "txt" => colors.syntax_string, // green
                "c" | "cpp" | "h" | "hpp" => colors.syntax_keyword, // purple
                "py" => colors.syntax_string,
                "html" | "css" | "js" | "ts" => colors.accent,
                _ => colors.syntax_plain,
            }
        }
    }
}

/// File Explorer state and tree traversal engine.
#[derive(Debug, Clone)]
pub struct FileExplorer {
    pub root_dir: PathBuf,
    pub root_name: String,
    pub expanded_dirs: BTreeSet<PathBuf>,
    pub visible_items: Vec<FileItem>,
    pub selected_idx: usize,
    pub scroll_offset: usize,
    pub is_visible: bool,
    pub width: usize,
    pub is_focused: bool,
}

impl FileExplorer {
    /// Creates a new FileExplorer anchored at `root_dir`.
    pub fn new(root_dir: PathBuf) -> Self {
        let canonical_root = root_dir.canonicalize().unwrap_or(root_dir);
        let root_name = canonical_root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("PROYECTO")
            .to_string();

        let mut expanded_dirs = BTreeSet::new();
        expanded_dirs.insert(canonical_root.clone());

        let mut explorer = Self {
            root_dir: canonical_root,
            root_name,
            expanded_dirs,
            visible_items: Vec::new(),
            selected_idx: 0,
            scroll_offset: 0,
            is_visible: true,
            width: 220,
            is_focused: false,
        };

        explorer.refresh();
        explorer
    }

    /// Re-scans the expanded directories from the filesystem.
    pub fn refresh(&mut self) {
        self.visible_items.clear();
        Self::scan_directory(&self.root_dir, 0, &self.expanded_dirs, &mut self.visible_items);
        if !self.visible_items.is_empty() && self.selected_idx >= self.visible_items.len() {
            self.selected_idx = self.visible_items.len() - 1;
        }
    }

    /// Recursively scans directory contents for expanded paths.
    fn scan_directory(
        dir: &Path,
        depth: usize,
        expanded: &BTreeSet<PathBuf>,
        out: &mut Vec<FileItem>,
    ) {
        let entries = match fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(_) => return,
        };

        let mut items = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            // Filter common build/vcs noise
            if matches!(name.as_str(), ".git" | "target" | "node_modules" | ".DS_Store" | ".idea" | ".vscode") {
                continue;
            }

            let is_dir = path.is_dir();
            items.push((path, name, is_dir));
        }

        // Sort: directories first (alphabetical), then files (alphabetical)
        items.sort_by(|a, b| {
            match (a.2, b.2) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.1.to_lowercase().cmp(&b.1.to_lowercase()),
            }
        });

        for (path, name, is_dir) in items {
            if is_dir {
                let is_exp = expanded.contains(&path);
                out.push(FileItem {
                    path: path.clone(),
                    name,
                    is_dir: true,
                    is_expanded: is_exp,
                    depth,
                });
                if is_exp {
                    Self::scan_directory(&path, depth + 1, expanded, out);
                }
            } else {
                out.push(FileItem {
                    path,
                    name,
                    is_dir: false,
                    is_expanded: false,
                    depth,
                });
            }
        }
    }

    /// Moves the selection up.
    pub fn move_up(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
            if self.selected_idx < self.scroll_offset {
                self.scroll_offset = self.selected_idx;
            }
        }
    }

    /// Moves the selection down.
    pub fn move_down(&mut self, max_visible: usize) {
        if !self.visible_items.is_empty() && self.selected_idx + 1 < self.visible_items.len() {
            self.selected_idx += 1;
            if max_visible > 0 && self.selected_idx >= self.scroll_offset + max_visible {
                self.scroll_offset = self.selected_idx + 1 - max_visible;
            }
        }
    }

    /// Expands the selected folder or enters first child.
    pub fn expand_selected(&mut self) -> bool {
        if let Some(item) = self.visible_items.get(self.selected_idx).cloned() {
            if item.is_dir {
                if !item.is_expanded {
                    self.expanded_dirs.insert(item.path);
                    self.refresh();
                    return true;
                } else if self.selected_idx + 1 < self.visible_items.len() {
                    self.selected_idx += 1;
                    return true;
                }
            }
        }
        false
    }

    /// Collapses the selected folder or jumps to parent directory.
    pub fn collapse_selected(&mut self) -> bool {
        if let Some(item) = self.visible_items.get(self.selected_idx).cloned() {
            if item.is_dir && item.is_expanded {
                self.expanded_dirs.remove(&item.path);
                self.refresh();
                return true;
            } else if item.depth > 0 {
                // Find parent directory (item with depth == item.depth - 1 above current)
                for i in (0..self.selected_idx).rev() {
                    if self.visible_items[i].is_dir && self.visible_items[i].depth == item.depth - 1 {
                        self.selected_idx = i;
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Toggles folder expand/collapse, or returns file path if a file was chosen.
    pub fn toggle_selected(&mut self) -> Option<PathBuf> {
        if let Some(item) = self.visible_items.get(self.selected_idx).cloned() {
            if item.is_dir {
                if item.is_expanded {
                    self.expanded_dirs.remove(&item.path);
                } else {
                    self.expanded_dirs.insert(item.path);
                }
                self.refresh();
                None
            } else {
                Some(item.path)
            }
        } else {
            None
        }
    }

    /// Handles a mouse click inside the sidebar bounds.
    pub fn handle_click(
        &mut self,
        rel_x: usize,
        rel_y: usize,
        row_h: usize,
        header_h: usize,
    ) -> SidebarClickResult {
        if rel_y < header_h {
            // Header buttons: [+] [↺] [×] on top right
            let btn_w = 16;
            let close_x = self.width.saturating_sub(btn_w + 6);
            let refresh_x = close_x.saturating_sub(btn_w + 4);
            let new_file_x = refresh_x.saturating_sub(btn_w + 4);

            if rel_x >= close_x && rel_x < close_x + btn_w {
                return SidebarClickResult::CloseSidebar;
            } else if rel_x >= refresh_x && rel_x < refresh_x + btn_w {
                self.refresh();
                return SidebarClickResult::Refresh;
            } else if rel_x >= new_file_x && rel_x < new_file_x + btn_w {
                return SidebarClickResult::NewFile;
            }
            return SidebarClickResult::None;
        }

        // File tree rows
        let row_idx = (rel_y - header_h) / row_h.max(1) + self.scroll_offset;
        if row_idx < self.visible_items.len() {
            self.selected_idx = row_idx;
            self.is_focused = true;
            if let Some(item) = self.visible_items.get(row_idx).cloned() {
                if item.is_dir {
                    if item.is_expanded {
                        self.expanded_dirs.remove(&item.path);
                    } else {
                        self.expanded_dirs.insert(item.path);
                    }
                    self.refresh();
                    SidebarClickResult::ToggleFolder
                } else {
                    SidebarClickResult::OpenFile(item.path)
                }
            } else {
                SidebarClickResult::None
            }
        } else {
            SidebarClickResult::None
        }
    }

    /// Handles mouse wheel scrolling inside the sidebar.
    pub fn handle_scroll(&mut self, delta: i32, max_visible: usize) {
        if delta < 0 {
            // Scroll down
            let max_scroll = self.visible_items.len().saturating_sub(max_visible);
            if self.scroll_offset < max_scroll {
                self.scroll_offset += 1;
            }
        } else if delta > 0 && self.scroll_offset > 0 {
            // Scroll up
            self.scroll_offset -= 1;
        }
    }
}
