//! Workspace and Project Management System for Rustext.
//! Pure standard-library Rust with zero third-party dependencies.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::editor::session::{SessionManager, SessionState};

/// Discovered workspace or project candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceItem {
    pub path: PathBuf,
    pub name: String,
    pub kind_tag: &'static str,
    pub tag_color: u32,
}

impl WorkspaceItem {
    /// Creates a new workspace item with automatic project type detection.
    pub fn new(path: PathBuf) -> Self {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        let name = canonical
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("PROYECTO")
            .to_string();

        let (kind_tag, tag_color) = Self::detect_kind(&canonical);

        Self {
            path: canonical,
            name,
            kind_tag,
            tag_color,
        }
    }

    /// Automatically detects project nature from marker files in pure standard library Rust.
    pub fn detect_kind(path: &Path) -> (&'static str, u32) {
        if path.join("Cargo.toml").exists() {
            ("[Rust]", 0x00FF6B35) // Rust orange
        } else if path.join("package.json").exists() {
            ("[Node]", 0x0068A063) // Node green
        } else if path.join("pyproject.toml").exists()
            || path.join("requirements.txt").exists()
            || path.join("setup.py").exists()
        {
            ("[Python]", 0x00FFD43B) // Python yellow
        } else if path.join("CMakeLists.txt").exists()
            || path.join("Makefile").exists()
            || path.join("meson.build").exists()
        {
            ("[C/C++]", 0x0061AFEF) // C/C++ blue
        } else if path.join("go.mod").exists() {
            ("[Go]", 0x0000ADD8) // Go cyan
        } else if path.join(".git").exists() {
            ("[Git]", 0x00C678DD) // Git purple
        } else {
            ("[Carpeta]", 0x00858585) // General folder gray
        }
    }

    /// Displays friendly path substituting user HOME with `~`.
    pub fn display_path(&self) -> String {
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).ok();
        if let Some(h) = home {
            let p_str = self.path.to_string_lossy();
            if p_str.starts_with(&h) {
                return format!("~{}", &p_str[h.len()..]);
            }
        }
        self.path.display().to_string()
    }
}

/// Registry managing recent projects and per-project isolated sessions.
#[derive(Debug, Clone)]
pub struct ProjectRegistry {
    pub base_dir: PathBuf,
}

impl Default for ProjectRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectRegistry {
    /// Creates a new ProjectRegistry anchored at the standard user config path (`~/.rustext`).
    pub fn new() -> Self {
        let base_dir = SessionManager::default_session_dir();
        Self { base_dir }
    }

    /// Creates a ProjectRegistry with a custom base directory (useful for unit testing).
    pub fn with_dir(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Canonical path to `projects.ini`.
    pub fn projects_file_path(&self) -> PathBuf {
        self.base_dir.join("projects.ini")
    }

    /// Directory for isolated per-workspace session INI files.
    pub fn workspaces_dir(&self) -> PathBuf {
        self.base_dir.join("workspaces")
    }

    /// Computes a stable, filesystem-safe filename for a given workspace path.
    pub fn workspace_session_path(&self, workspace_root: &Path) -> PathBuf {
        // Check for local project workspace state first (e.g., .rustext/session.ini)
        let local_workspace = workspace_root.join(".rustext").join("session.ini");
        if local_workspace.exists() {
            return local_workspace;
        }

        let canonical = workspace_root.canonicalize().unwrap_or_else(|_| workspace_root.to_path_buf());
        let folder_name = canonical
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("workspace");

        // Compute 64-bit FNV-1a hash of canonical path for collision-free naming
        let mut hash = 0xcbf29ce484222325u64;
        for byte in canonical.to_string_lossy().as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }

        let safe_name = folder_name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>();

        let filename = format!("{}_{:016x}.ini", safe_name, hash);
        self.workspaces_dir().join(filename)
    }

    /// Loads the list of recent projects from `projects.ini`.
    pub fn load_recent_projects(&self) -> Vec<WorkspaceItem> {
        let path = self.projects_file_path();
        if !path.exists() {
            return Vec::new();
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let mut projects = Vec::new();
        let mut in_projects_section = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let sec = trimmed[1..trimmed.len() - 1].trim().to_lowercase();
                in_projects_section = sec == "recent_projects" || sec == "projects";
                continue;
            }

            if in_projects_section {
                let mut parts = trimmed.splitn(2, '=');
                let _key = parts.next().unwrap_or("").trim();
                let val = parts.next().unwrap_or("").trim();
                let path_str = val.trim_matches('"').trim_matches('\'').trim();

                if !path_str.is_empty() {
                    let pb = PathBuf::from(path_str);
                    if pb.exists() && pb.is_dir() {
                        let item = WorkspaceItem::new(pb);
                        if !projects.iter().any(|p: &WorkspaceItem| p.path == item.path) {
                            projects.push(item);
                        }
                    }
                }
            }
        }

        projects
    }

    /// Persists recent projects into `projects.ini`.
    pub fn save_recent_projects(&self, projects: &[WorkspaceItem]) -> io::Result<()> {
        let path = self.projects_file_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut out = String::with_capacity(1024);
        out.push_str("; Rustext Projects Registry (projects.ini)\n");
        out.push_str("[recent_projects]\n");
        out.push_str(&format!("count = {}\n", projects.len()));

        for (idx, proj) in projects.iter().enumerate() {
            out.push_str(&format!("project_{} = {}\n", idx, proj.path.display()));
        }

        fs::write(&path, out)
    }

    /// Adds a project path to the recent projects list, putting it at the top and deduplicating.
    pub fn add_recent_project(&self, projects: &mut Vec<WorkspaceItem>, path: PathBuf, max_count: usize) {
        let item = WorkspaceItem::new(path);
        projects.retain(|p| p.path != item.path);
        projects.insert(0, item);
        if projects.len() > max_count {
            projects.truncate(max_count);
        }
        let _ = self.save_recent_projects(projects);
    }

    /// Removes a project from recent projects list.
    pub fn remove_recent_project(&self, projects: &mut Vec<WorkspaceItem>, path: &Path) {
        let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        projects.retain(|p| p.path != canon && p.path != path);
        let _ = self.save_recent_projects(projects);
    }

    /// Saves isolated workspace session state for a specific project.
    pub fn save_workspace_session(&self, workspace_root: &Path, state: &SessionState) -> io::Result<()> {
        let path = self.workspace_session_path(workspace_root);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, state.to_ini_string())
    }

    /// Loads isolated workspace session state for a specific project, if previously saved.
    pub fn load_workspace_session(&self, workspace_root: &Path) -> Option<SessionState> {
        let path = self.workspace_session_path(workspace_root);
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                let state = SessionState::parse(&content);
                return Some(state);
            }
        }
        None
    }

    /// Applies optional project-level configuration overrides from `.rustext/config.ini`.
    pub fn apply_project_config(workspace_root: &Path, config: &mut crate::editor::config::EditorConfig) {
        let proj_cfg = workspace_root.join(".rustext").join("config.ini");
        if proj_cfg.exists() {
            if let Ok(content) = fs::read_to_string(&proj_cfg) {
                config.apply_ini_str(&content);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_detection_and_tagging() {
        let temp = std::env::temp_dir().join(format!("rustext_ws_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp);

        // Blank folder
        let item_blank = WorkspaceItem::new(temp.clone());
        assert_eq!(item_blank.kind_tag, "[Carpeta]");

        // Rust project
        let cargo_toml = temp.join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nname = \"dummy\"\n").unwrap();
        let item_rust = WorkspaceItem::new(temp.clone());
        assert_eq!(item_rust.kind_tag, "[Rust]");

        // Cleanup
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_project_registry_roundtrip_and_deduplication() {
        let temp = std::env::temp_dir().join(format!("rustext_reg_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp);

        let p1 = temp.join("project_alpha");
        let p2 = temp.join("project_beta");
        fs::create_dir_all(&p1).unwrap();
        fs::create_dir_all(&p2).unwrap();

        let registry = ProjectRegistry::with_dir(temp.clone());
        let mut list = Vec::new();

        registry.add_recent_project(&mut list, p1.clone(), 10);
        registry.add_recent_project(&mut list, p2.clone(), 10);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].path, p2.canonicalize().unwrap());

        // Re-add p1 moves it to top
        registry.add_recent_project(&mut list, p1.clone(), 10);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].path, p1.canonicalize().unwrap());

        // Reload from disk
        let loaded = registry.load_recent_projects();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].path, p1.canonicalize().unwrap());

        // Removal
        registry.remove_recent_project(&mut list, &p1);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].path, p2.canonicalize().unwrap());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_workspace_session_isolation() {
        let temp = std::env::temp_dir().join(format!("rustext_ws_sess_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp);

        let p1 = temp.join("project_1");
        let p2 = temp.join("project_2");
        fs::create_dir_all(&p1).unwrap();
        fs::create_dir_all(&p2).unwrap();

        let registry = ProjectRegistry::with_dir(temp.clone());

        let mut s1 = SessionState::default();
        s1.active_tab_idx = 3;
        s1.split_active = true;

        let mut s2 = SessionState::default();
        s2.active_tab_idx = 0;
        s2.split_active = false;

        registry.save_workspace_session(&p1, &s1).unwrap();
        registry.save_workspace_session(&p2, &s2).unwrap();

        let l1 = registry.load_workspace_session(&p1).expect("s1 should load");
        let l2 = registry.load_workspace_session(&p2).expect("s2 should load");

        assert_eq!(l1.active_tab_idx, 3);
        assert_eq!(l1.split_active, true);

        assert_eq!(l2.active_tab_idx, 0);
        assert_eq!(l2.split_active, false);

        let _ = fs::remove_dir_all(&temp);
    }
}
