//! High-performance workspace file discovery and fuzzy ranking engine.
//! Zero third-party dependencies, purely standard library Rust.

use std::fs;
use std::path::{Path, PathBuf};
use super::fuzzy::fuzzy_match;

/// Discovered file candidate in the workspace tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFile {
    /// Full path on disk.
    pub path: PathBuf,
    /// Relative path for display (e.g. "src/editor/app.rs").
    pub relative_path: String,
    /// Basename of the file (e.g. "app.rs").
    pub file_name: String,
    /// Lowercase file extension (e.g. "rs").
    pub extension: String,
    /// Whether this file is currently open in an active tab.
    pub is_open: bool,
    /// Whether this file was recently opened.
    pub is_recent: bool,
}

/// A matched and scored file result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFinderMatch {
    pub file: ProjectFile,
    pub score: i32,
    /// Indices within `relative_path` where query characters matched.
    pub matched_indices: Vec<usize>,
}

pub struct FileFinder;

impl FileFinder {
    /// Scans the workspace directory tree recursively and returns all project files.
    pub fn scan_workspace(
        root_dir: &Path,
        open_paths: &[PathBuf],
        recent_paths: &[PathBuf],
    ) -> Vec<ProjectFile> {
        let mut files = Vec::with_capacity(512);
        let max_files = 25000;

        Self::scan_dir_recursive(root_dir, root_dir, 0, open_paths, recent_paths, max_files, &mut files);

        // Sort so that files in root and shallower directories appear first deterministically
        files.sort_by(|a, b| {
            let a_depth = a.relative_path.chars().filter(|&c| c == '/' || c == '\\').count();
            let b_depth = b.relative_path.chars().filter(|&c| c == '/' || c == '\\').count();
            a_depth.cmp(&b_depth).then_with(|| a.relative_path.cmp(&b.relative_path))
        });

        files
    }

    fn scan_dir_recursive(
        root_dir: &Path,
        current_dir: &Path,
        depth: usize,
        open_paths: &[PathBuf],
        recent_paths: &[PathBuf],
        max_files: usize,
        files: &mut Vec<ProjectFile>,
    ) {
        if depth > 32 || files.len() >= max_files {
            return;
        }

        let entries = match fs::read_dir(current_dir) {
            Ok(rd) => rd,
            Err(_) => return,
        };

        let mut subdirs = Vec::new();
        let mut local_files = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };

            // Filter out build artifacts, VCS, dependencies, and hidden folders
            if matches!(
                name,
                ".git"
                    | "target"
                    | "node_modules"
                    | ".idea"
                    | ".vscode"
                    | ".cache"
                    | "build"
                    | "dist"
                    | ".DS_Store"
                    | "__pycache__"
                    | ".next"
                    | ".svn"
                    | ".hg"
                    | ".gradle"
            ) {
                continue;
            }

            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            // Defense against directory symlink recursion (SEC-04 / CWE-59)
            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                subdirs.push(path);
            } else if file_type.is_file() {
                local_files.push(path);
            }
        }

        // Add local files
        for p in local_files {
            if files.len() >= max_files {
                break;
            }

            let file_name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            let extension = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();

            let relative_path = p
                .strip_prefix(root_dir)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");

            let is_open = open_paths.iter().any(|op| op == &p);
            let is_recent = recent_paths.iter().any(|rp| rp == &p);

            files.push(ProjectFile {
                path: p,
                relative_path,
                file_name,
                extension,
                is_open,
                is_recent,
            });
        }

        // Traverse subdirectories
        for dir in subdirs {
            Self::scan_dir_recursive(root_dir, &dir, depth + 1, open_paths, recent_paths, max_files, files);
        }
    }

    /// Filters and ranks workspace project files against a user query.
    pub fn filter_and_rank(
        files: &[ProjectFile],
        query: &str,
        max_results: usize,
    ) -> Vec<FileFinderMatch> {
        let trimmed = query.trim();

        if trimmed.is_empty() {
            // When query is empty: show currently open files first, then recent files, then other project files
            let mut results: Vec<FileFinderMatch> = files
                .iter()
                .map(|file| {
                    let mut score = 0;
                    if file.is_open {
                        score += 200;
                    }
                    if file.is_recent {
                        score += 100;
                    }
                    FileFinderMatch {
                        file: file.clone(),
                        score,
                        matched_indices: Vec::new(),
                    }
                })
                .collect();

            results.sort_by(|a, b| {
                b.score
                    .cmp(&a.score)
                    .then_with(|| a.file.relative_path.cmp(&b.file.relative_path))
            });

            results.truncate(max_results);
            return results;
        }

        let mut matched = Vec::with_capacity(files.len().min(128));

        for file in files {
            // Match against relative path (e.g. "src/editor/app.rs")
            if let Some(mut m) = fuzzy_match(trimmed, &file.relative_path) {
                // Priority bonuses
                if file.is_open {
                    m.score += 120;
                }
                if file.is_recent {
                    m.score += 60;
                }

                matched.push(FileFinderMatch {
                    file: file.clone(),
                    score: m.score,
                    matched_indices: m.matched_indices,
                });
            }
        }

        // Sort descending by relevance score, then alphabetically by relative path
        matched.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.file.relative_path.cmp(&b.file.relative_path))
        });

        matched.truncate(max_results);
        matched
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_finder_empty_query_ordering() {
        let files = vec![
            ProjectFile {
                path: PathBuf::from("/p/src/main.rs"),
                relative_path: "src/main.rs".to_string(),
                file_name: "main.rs".to_string(),
                extension: "rs".to_string(),
                is_open: false,
                is_recent: false,
            },
            ProjectFile {
                path: PathBuf::from("/p/src/lib.rs"),
                relative_path: "src/lib.rs".to_string(),
                file_name: "lib.rs".to_string(),
                extension: "rs".to_string(),
                is_open: true,
                is_recent: false,
            },
        ];

        let results = FileFinder::filter_and_rank(&files, "", 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].file.relative_path, "src/lib.rs"); // Open file prioritized
    }

    #[test]
    fn test_file_finder_query_fuzzy_matching() {
        let files = vec![
            ProjectFile {
                path: PathBuf::from("/p/src/editor/app.rs"),
                relative_path: "src/editor/app.rs".to_string(),
                file_name: "app.rs".to_string(),
                extension: "rs".to_string(),
                is_open: false,
                is_recent: false,
            },
            ProjectFile {
                path: PathBuf::from("/p/src/window/win32.rs"),
                relative_path: "src/window/win32.rs".to_string(),
                file_name: "win32.rs".to_string(),
                extension: "rs".to_string(),
                is_open: false,
                is_recent: false,
            },
        ];

        let results = FileFinder::filter_and_rank(&files, "apprs", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file.relative_path, "src/editor/app.rs");
    }
}
