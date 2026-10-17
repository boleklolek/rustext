//! Session and Workspace State Persistence (Hot Exit, Tabs & Recent Files).
//! Pure standard-library Rust with zero third-party dependencies.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Snapshot of a single open tab in a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabSession {
    pub file_path: Option<PathBuf>,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub scroll_row: usize,
    pub scroll_col: usize,
    pub is_dirty: bool,
    pub draft_path: Option<PathBuf>,
}

impl Default for TabSession {
    fn default() -> Self {
        Self {
            file_path: None,
            cursor_line: 0,
            cursor_col: 0,
            scroll_row: 0,
            scroll_col: 0,
            is_dirty: false,
            draft_path: None,
        }
    }
}

/// Workspace session state holding open tabs, window layout, and recent files.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionState {
    pub open_tabs: Vec<TabSession>,
    pub active_tab_idx: usize,
    pub split_active: bool,
    pub split_direction: String,
    pub split_secondary_idx: usize,
    pub split_active_pane: usize,
    pub split_ratio: f32,
    pub sidebar_visible: bool,
    pub sidebar_root: Option<PathBuf>,
    pub recent_files: Vec<PathBuf>,
    pub window_width: usize,
    pub window_height: usize,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            open_tabs: Vec::new(),
            active_tab_idx: 0,
            split_active: false,
            split_direction: "none".to_string(),
            split_secondary_idx: 0,
            split_active_pane: 0,
            split_ratio: 0.5,
            sidebar_visible: true,
            sidebar_root: None,
            recent_files: Vec::new(),
            window_width: 1024,
            window_height: 640,
        }
    }
}

impl SessionState {
    /// Serializes session state into an INI formatted string.
    pub fn to_ini_string(&self) -> String {
        let mut out = String::with_capacity(2048);

        out.push_str("; Rustext Session File (session.ini)\n");
        out.push_str("[session]\n");
        out.push_str(&format!("active_tab = {}\n", self.active_tab_idx));
        out.push_str(&format!("split_active = {}\n", self.split_active));
        out.push_str(&format!("split_direction = {}\n", self.split_direction));
        out.push_str(&format!("split_secondary_idx = {}\n", self.split_secondary_idx));
        out.push_str(&format!("split_active_pane = {}\n", self.split_active_pane));
        out.push_str(&format!("split_ratio = {:.2}\n", self.split_ratio));
        out.push_str(&format!("sidebar_visible = {}\n", self.sidebar_visible));
        if let Some(ref root) = self.sidebar_root {
            out.push_str(&format!("sidebar_root = {}\n", root.display()));
        }
        out.push_str(&format!("window_width = {}\n", self.window_width));
        out.push_str(&format!("window_height = {}\n", self.window_height));
        out.push_str(&format!("tab_count = {}\n", self.open_tabs.len()));

        out.push_str("\n[tabs]\n");
        for (i, tab) in self.open_tabs.iter().enumerate() {
            if let Some(ref path) = tab.file_path {
                out.push_str(&format!("tab_{}_path = {}\n", i, path.display()));
            } else {
                out.push_str(&format!("tab_{}_path = \n", i));
            }
            out.push_str(&format!("tab_{}_line = {}\n", i, tab.cursor_line));
            out.push_str(&format!("tab_{}_col = {}\n", i, tab.cursor_col));
            out.push_str(&format!("tab_{}_scroll_row = {}\n", i, tab.scroll_row));
            out.push_str(&format!("tab_{}_scroll_col = {}\n", i, tab.scroll_col));
            out.push_str(&format!("tab_{}_dirty = {}\n", i, tab.is_dirty));
            if let Some(ref draft) = tab.draft_path {
                out.push_str(&format!("tab_{}_draft = {}\n", i, draft.display()));
            } else {
                out.push_str(&format!("tab_{}_draft = \n", i));
            }
        }

        out.push_str("\n[recent_files]\n");
        out.push_str(&format!("count = {}\n", self.recent_files.len()));
        for (i, path) in self.recent_files.iter().enumerate() {
            out.push_str(&format!("file_{} = {}\n", i, path.display()));
        }

        out
    }

    /// Parses an INI string into a `SessionState` instance.
    pub fn parse(content: &str) -> Self {
        let mut state = Self::default();
        let mut current_section = String::new();
        let mut tab_count = 0;
        let mut tabs_map: std::collections::HashMap<usize, TabSession> = std::collections::HashMap::new();
        let mut recent_map: std::collections::HashMap<usize, PathBuf> = std::collections::HashMap::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current_section = trimmed[1..trimmed.len() - 1].trim().to_lowercase();
                continue;
            }

            let mut parts = trimmed.splitn(2, '=');
            let key = parts.next().unwrap_or("").trim().to_lowercase();
            let val = parts.next().unwrap_or("").trim();
            let val_unquoted = val.trim_matches('"').trim_matches('\'').trim();

            match current_section.as_str() {
                "session" => match key.as_str() {
                    "active_tab" => {
                        if let Ok(n) = val_unquoted.parse::<usize>() {
                            state.active_tab_idx = n;
                        }
                    }
                    "split_active" => {
                        state.split_active = val_unquoted.eq_ignore_ascii_case("true") || val_unquoted == "1";
                    }
                    "split_direction" => {
                        state.split_direction = val_unquoted.to_string();
                    }
                    "split_secondary_idx" => {
                        if let Ok(n) = val_unquoted.parse::<usize>() {
                            state.split_secondary_idx = n;
                        }
                    }
                    "split_active_pane" => {
                        if let Ok(n) = val_unquoted.parse::<usize>() {
                            state.split_active_pane = n;
                        }
                    }
                    "split_ratio" => {
                        if let Ok(r) = val_unquoted.parse::<f32>() {
                            state.split_ratio = r.clamp(0.15, 0.85);
                        }
                    }
                    "sidebar_visible" => {
                        state.sidebar_visible = val_unquoted.eq_ignore_ascii_case("true") || val_unquoted == "1";
                    }
                    "sidebar_root" => {
                        if !val_unquoted.is_empty() {
                            state.sidebar_root = Some(PathBuf::from(val_unquoted));
                        }
                    }
                    "window_width" => {
                        if let Ok(w) = val_unquoted.parse::<usize>() {
                            state.window_width = w.clamp(320, 7680);
                        }
                    }
                    "window_height" => {
                        if let Ok(h) = val_unquoted.parse::<usize>() {
                            state.window_height = h.clamp(200, 4320);
                        }
                    }
                    "tab_count" => {
                        if let Ok(n) = val_unquoted.parse::<usize>() {
                            tab_count = n;
                        }
                    }
                    _ => {}
                },
                "tabs" => {
                    // key format: tab_{idx}_{field}
                    if key.starts_with("tab_") {
                        let sub = &key[4..];
                        if let Some(pos) = sub.find('_') {
                            if let Ok(idx) = sub[..pos].parse::<usize>() {
                                let field = &sub[pos + 1..];
                                let entry = tabs_map.entry(idx).or_default();
                                match field {
                                    "path" => {
                                        if !val_unquoted.is_empty() {
                                            entry.file_path = Some(PathBuf::from(val_unquoted));
                                        }
                                    }
                                    "line" => {
                                        if let Ok(l) = val_unquoted.parse::<usize>() {
                                            entry.cursor_line = l;
                                        }
                                    }
                                    "col" => {
                                        if let Ok(c) = val_unquoted.parse::<usize>() {
                                            entry.cursor_col = c;
                                        }
                                    }
                                    "scroll_row" => {
                                        if let Ok(r) = val_unquoted.parse::<usize>() {
                                            entry.scroll_row = r;
                                        }
                                    }
                                    "scroll_col" => {
                                        if let Ok(c) = val_unquoted.parse::<usize>() {
                                            entry.scroll_col = c;
                                        }
                                    }
                                    "dirty" => {
                                        entry.is_dirty = val_unquoted.eq_ignore_ascii_case("true") || val_unquoted == "1";
                                    }
                                    "draft" => {
                                        if !val_unquoted.is_empty() {
                                            let p = PathBuf::from(val_unquoted);
                                            let safe_ext = p.extension().and_then(|e| e.to_str()) == Some("draft");
                                            let no_traversal = !val_unquoted.contains("..") && !val_unquoted.contains('\0');
                                            if safe_ext && no_traversal {
                                                entry.draft_path = Some(p);
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                "recent_files" => {
                    if key.starts_with("file_") {
                        if let Ok(idx) = key[5..].parse::<usize>() {
                            if !val_unquoted.is_empty() {
                                recent_map.insert(idx, PathBuf::from(val_unquoted));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Assemble tabs in order
        let total_tabs = tab_count.max(tabs_map.keys().copied().max().map(|m| m + 1).unwrap_or(0));
        for i in 0..total_tabs {
            if let Some(tab) = tabs_map.remove(&i) {
                state.open_tabs.push(tab);
            }
        }

        // Assemble recent files in order
        let recent_total = recent_map.keys().copied().max().map(|m| m + 1).unwrap_or(0);
        for i in 0..recent_total {
            if let Some(p) = recent_map.remove(&i) {
                state.recent_files.push(p);
            }
        }

        state
    }
}

/// Manages loading and saving sessions and recovery drafts on disk.
#[derive(Debug, Clone)]
pub struct SessionManager {
    pub base_dir: PathBuf,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    /// Creates a new SessionManager with the default system directory (`~/.rustext`).
    pub fn new() -> Self {
        let base_dir = Self::default_session_dir();
        Self { base_dir }
    }

    /// Creates a SessionManager with a custom directory (useful for unit testing).
    pub fn with_dir(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Determines the standard session directory.
    pub fn default_session_dir() -> PathBuf {
        let local = PathBuf::from(".rustext");
        if local.exists() {
            return local;
        }

        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from);

        if let Some(h) = home {
            return h.join(".rustext");
        }

        local
    }

    /// Canonical path to `session.ini`.
    pub fn session_file_path(&self) -> PathBuf {
        self.base_dir.join("session.ini")
    }

    /// Directory for storing temporary unsaved drafts (*Hot Exit*).
    pub fn drafts_dir(&self) -> PathBuf {
        self.base_dir.join("drafts")
    }

    /// Loads the saved session state from disk, if it exists and is valid.
    pub fn load_session(&self) -> Option<SessionState> {
        let path = self.session_file_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                let state = SessionState::parse(&content);
                return Some(state);
            }
        }
        None
    }

    /// Persists session state to `session.ini`.
    pub fn save_session(&self, state: &SessionState) -> io::Result<()> {
        let path = self.session_file_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, state.to_ini_string())
    }

    /// Saves an unsaved draft buffer to disk.
    pub fn save_draft(&self, id: &str, content: &str) -> io::Result<PathBuf> {
        let dir = self.drafts_dir();
        fs::create_dir_all(&dir)?;
        let file_path = dir.join(format!("{}.draft", id));
        fs::write(&file_path, content)?;
        Ok(file_path)
    }

    /// Checks whether a draft path is safely confined within the drafts directory.
    pub fn is_safe_draft_path(&self, path: &Path) -> bool {
        let drafts_dir = self.drafts_dir();
        if let Ok(canon_dir) = drafts_dir.canonicalize() {
            if let Ok(canon_path) = path.canonicalize() {
                return canon_path.starts_with(&canon_dir)
                    && canon_path.extension().and_then(|e| e.to_str()) == Some("draft");
            }
        }
        path.starts_with(&drafts_dir) && path.extension().and_then(|e| e.to_str()) == Some("draft")
    }

    /// Reads an unsaved draft buffer from disk.
    pub fn read_draft(&self, path: &Path) -> io::Result<String> {
        if !self.is_safe_draft_path(path) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Draft path traversal denied: file outside drafts directory (SEC-05)",
            ));
        }
        fs::read_to_string(path)
    }

    /// Removes an existing draft file.
    pub fn remove_draft(&self, path: &Path) {
        if self.is_safe_draft_path(path) {
            let _ = fs::remove_file(path);
        }
    }

    /// Clears all existing draft recovery files.
    pub fn clear_all_drafts(&self) {
        let dir = self.drafts_dir();
        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
        }
    }

    /// Adds a file to the recent files list, deduplicating and maintaining order.
    pub fn add_recent_file(recent_files: &mut Vec<PathBuf>, path: PathBuf, max_count: usize) {
        if !path.exists() {
            return;
        }

        // Canonicalize if possible, or keep as is
        let norm = path.canonicalize().unwrap_or(path);

        recent_files.retain(|p| p != &norm);
        recent_files.insert(0, norm);
        if recent_files.len() > max_count {
            recent_files.truncate(max_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_ini_serialization_and_parsing() {
        let mut state = SessionState::default();
        state.active_tab_idx = 1;
        state.split_active = true;
        state.split_direction = "vertical".to_string();
        state.split_secondary_idx = 1;
        state.split_active_pane = 1;
        state.split_ratio = 0.65;
        state.sidebar_visible = true;
        state.sidebar_root = Some(PathBuf::from("/home/user/project"));
        state.window_width = 1200;
        state.window_height = 800;

        let tab0 = TabSession {
            file_path: Some(PathBuf::from("/home/user/project/main.rs")),
            cursor_line: 15,
            cursor_col: 4,
            scroll_row: 5,
            scroll_col: 0,
            is_dirty: false,
            draft_path: None,
        };
        let tab1 = TabSession {
            file_path: None,
            cursor_line: 2,
            cursor_col: 10,
            scroll_row: 0,
            scroll_col: 0,
            is_dirty: true,
            draft_path: Some(PathBuf::from("/home/user/.rustext/drafts/tab1.draft")),
        };
        state.open_tabs.push(tab0);
        state.open_tabs.push(tab1);

        state.recent_files.push(PathBuf::from("/home/user/project/main.rs"));
        state.recent_files.push(PathBuf::from("/home/user/project/lib.rs"));

        let ini_text = state.to_ini_string();
        assert!(ini_text.contains("active_tab = 1"));
        assert!(ini_text.contains("split_active = true"));
        assert!(ini_text.contains("split_direction = vertical"));
        assert!(ini_text.contains("tab_0_path = /home/user/project/main.rs"));
        assert!(ini_text.contains("tab_1_dirty = true"));
        assert!(ini_text.contains("file_0 = /home/user/project/main.rs"));

        // Roundtrip parse
        let parsed = SessionState::parse(&ini_text);
        assert_eq!(parsed.active_tab_idx, 1);
        assert!(parsed.split_active);
        assert_eq!(parsed.split_direction, "vertical");
        assert_eq!(parsed.open_tabs.len(), 2);
        assert_eq!(parsed.open_tabs[0].cursor_line, 15);
        assert_eq!(parsed.open_tabs[0].cursor_col, 4);
        assert!(!parsed.open_tabs[0].is_dirty);
        assert!(parsed.open_tabs[1].is_dirty);
        assert_eq!(parsed.recent_files.len(), 2);
        assert_eq!(parsed.recent_files[0], PathBuf::from("/home/user/project/main.rs"));
    }

    #[test]
    fn test_session_manager_save_load_and_drafts() {
        let temp_dir = std::env::temp_dir().join(format!("rustext_session_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let manager = SessionManager::with_dir(temp_dir.clone());

        let mut state = SessionState::default();
        state.open_tabs.push(TabSession {
            file_path: Some(PathBuf::from("test.txt")),
            cursor_line: 3,
            cursor_col: 2,
            scroll_row: 0,
            scroll_col: 0,
            is_dirty: false,
            draft_path: None,
        });

        // Save session
        manager.save_session(&state).unwrap();
        assert!(manager.session_file_path().exists());

        // Load session
        let loaded = manager.load_session().expect("Session should be loaded");
        assert_eq!(loaded.open_tabs.len(), 1);
        assert_eq!(loaded.open_tabs[0].cursor_line, 3);

        // Draft saving and reading
        let draft_path = manager.save_draft("doc_0", "fn draft_code() {}").unwrap();
        assert!(draft_path.exists());
        let draft_content = manager.read_draft(&draft_path).unwrap();
        assert_eq!(draft_content, "fn draft_code() {}");

        manager.remove_draft(&draft_path);
        assert!(!draft_path.exists());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_add_recent_file_deduplication_and_limit() {
        let temp_dir = std::env::temp_dir().join(format!("rustext_recent_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let _ = fs::create_dir_all(&temp_dir);

        let f1 = temp_dir.join("f1.rs");
        let f2 = temp_dir.join("f2.rs");
        let f3 = temp_dir.join("f3.rs");
        let _ = fs::write(&f1, "1");
        let _ = fs::write(&f2, "2");
        let _ = fs::write(&f3, "3");

        let mut recents = Vec::new();
        SessionManager::add_recent_file(&mut recents, f1.clone(), 2);
        SessionManager::add_recent_file(&mut recents, f2.clone(), 2);
        assert_eq!(recents.len(), 2);

        // Adding f1 again brings it to the front
        SessionManager::add_recent_file(&mut recents, f1.clone(), 2);
        assert_eq!(recents.len(), 2);
        assert_eq!(recents[0], f1.canonicalize().unwrap());

        // Adding f3 truncates to limit 2
        SessionManager::add_recent_file(&mut recents, f3.clone(), 2);
        assert_eq!(recents.len(), 2);
        assert_eq!(recents[0], f3.canonicalize().unwrap());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
