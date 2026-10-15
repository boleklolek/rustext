//! Live Git Integration for Rustext.
//! Provides Git branch detection, working tree status parsing, and live gutter diffs.
//! 100% pure standard library Rust with zero third-party dependencies.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Working tree status of a file in the Git repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitFileStatus {
    Modified,
    Untracked,
    Added,
    Deleted,
    Renamed,
}

impl GitFileStatus {
    /// Single-character badge for UI display ('M', 'U', 'A', 'D', 'R').
    pub fn badge_char(&self) -> char {
        match self {
            Self::Modified => 'M',
            Self::Untracked => 'U',
            Self::Added => 'A',
            Self::Deleted => 'D',
            Self::Renamed => 'R',
        }
    }

    /// Color hex code for UI display.
    pub fn color(&self) -> u32 {
        match self {
            Self::Modified => 0x00E0AF68, // Amber / Yellow
            Self::Untracked => 0x009ECE6A, // Green
            Self::Added => 0x0073DACA,     // Cyan / Teal
            Self::Deleted => 0x00F7768E,   // Red
            Self::Renamed => 0x00BB9AF7,   // Purple
        }
    }
}

/// Diff state of a single line in the active document relative to Git HEAD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitLineStatus {
    None,
    Added,
    Modified,
    DeletedAbove,
}

impl GitLineStatus {
    /// Color hex for the gutter diff indicator bar.
    pub fn color(&self) -> u32 {
        match self {
            Self::None => 0x00000000,
            Self::Added => 0x009ECE6A,        // Green
            Self::Modified => 0x007AA2F7,     // Blue / Cyan
            Self::DeletedAbove => 0x00F7768E, // Red
        }
    }
}

/// Results of a line-by-line diff against Git HEAD.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitGutterDiff {
    pub lines: Vec<GitLineStatus>,
    pub added_count: usize,
    pub modified_count: usize,
    pub deleted_count: usize,
}

impl GitGutterDiff {
    pub fn is_empty(&self) -> bool {
        self.added_count == 0 && self.modified_count == 0 && self.deleted_count == 0
    }

    pub fn status_for_line(&self, line_idx: usize) -> GitLineStatus {
        if line_idx < self.lines.len() {
            self.lines[line_idx]
        } else {
            GitLineStatus::None
        }
    }
}

/// Computes line-by-line diff using Longest Common Subsequence (LCS).
pub fn compute_gutter_diff(old_text: &str, new_text: &str) -> GitGutterDiff {
    let old_lines: Vec<&str> = if old_text.is_empty() {
        Vec::new()
    } else {
        old_text.split('\n').collect()
    };
    let new_lines: Vec<&str> = if new_text.is_empty() {
        Vec::new()
    } else {
        new_text.split('\n').collect()
    };

    let n = new_lines.len();
    let m = old_lines.len();

    if n == 0 {
        return GitGutterDiff {
            lines: Vec::new(),
            added_count: 0,
            modified_count: 0,
            deleted_count: m,
        };
    }

    if m == 0 {
        return GitGutterDiff {
            lines: vec![GitLineStatus::Added; n],
            added_count: n,
            modified_count: 0,
            deleted_count: 0,
        };
    }

    // 1. Identify common prefix
    let mut prefix_len = 0;
    while prefix_len < m && prefix_len < n && old_lines[prefix_len] == new_lines[prefix_len] {
        prefix_len += 1;
    }

    // 2. Identify common suffix
    let mut suffix_len = 0;
    while suffix_len < (m - prefix_len)
        && suffix_len < (n - prefix_len)
        && old_lines[m - 1 - suffix_len] == new_lines[n - 1 - suffix_len]
    {
        suffix_len += 1;
    }

    let mut result_lines = vec![GitLineStatus::None; n];

    let mid_old_len = m - prefix_len - suffix_len;
    let mid_new_len = n - prefix_len - suffix_len;

    let mut added = 0;
    let mut modified = 0;
    let mut deleted = 0;

    if mid_old_len == 0 && mid_new_len > 0 {
        // Pure additions in the middle
        for i in 0..mid_new_len {
            result_lines[prefix_len + i] = GitLineStatus::Added;
            added += 1;
        }
    } else if mid_old_len > 0 && mid_new_len == 0 {
        // Pure deletions in the middle
        if prefix_len < n {
            result_lines[prefix_len] = GitLineStatus::DeletedAbove;
        } else if n > 0 {
            result_lines[n - 1] = GitLineStatus::DeletedAbove;
        }
        deleted += mid_old_len;
    } else if mid_old_len > 0 && mid_new_len > 0 {
        // Compute LCS on the middle portion
        // Cap matrix size for safety
        if mid_old_len * mid_new_len <= 2_500_000 {
            let old_slice = &old_lines[prefix_len..m - suffix_len];
            let new_slice = &new_lines[prefix_len..n - suffix_len];

            // DP table for LCS
            let mut dp = vec![0u32; (mid_old_len + 1) * (mid_new_len + 1)];
            let stride = mid_new_len + 1;

            for i in 0..mid_old_len {
                for j in 0..mid_new_len {
                    if old_slice[i] == new_slice[j] {
                        dp[(i + 1) * stride + (j + 1)] = dp[i * stride + j] + 1;
                    } else {
                        let left = dp[(i + 1) * stride + j];
                        let up = dp[i * stride + (j + 1)];
                        dp[(i + 1) * stride + (j + 1)] = left.max(up);
                    }
                }
            }

            // Backtrack to find edits
            let mut i = mid_old_len;
            let mut j = mid_new_len;
            let mut is_new_matched = vec![false; mid_new_len];
            let mut is_old_matched = vec![false; mid_old_len];

            while i > 0 && j > 0 {
                if old_slice[i - 1] == new_slice[j - 1] {
                    is_new_matched[j - 1] = true;
                    is_old_matched[i - 1] = true;
                    i -= 1;
                    j -= 1;
                } else if dp[i * stride + (j - 1)] >= dp[(i - 1) * stride + j] {
                    j -= 1;
                } else {
                    i -= 1;
                }
            }

            // Map unmatched lines into Added / Modified / DeletedAbove
            let mut old_idx = 0;
            let mut new_idx = 0;

            while old_idx < mid_old_len || new_idx < mid_new_len {
                if old_idx < mid_old_len && new_idx < mid_new_len && is_old_matched[old_idx] && is_new_matched[new_idx] {
                    // Both lines match
                    old_idx += 1;
                    new_idx += 1;
                } else {
                    // Count consecutive unmatched old and new
                    let start_new = new_idx;
                    while new_idx < mid_new_len && !is_new_matched[new_idx] {
                        new_idx += 1;
                    }
                    let count_new = new_idx - start_new;

                    let start_old = old_idx;
                    while old_idx < mid_old_len && !is_old_matched[old_idx] {
                        old_idx += 1;
                    }
                    let count_old = old_idx - start_old;

                    if count_old > 0 && count_new > 0 {
                        // Replacement: lines replaced are Modified, excess are Added
                        let mod_count = count_new.min(count_old);
                        for k in 0..mod_count {
                            result_lines[prefix_len + start_new + k] = GitLineStatus::Modified;
                            modified += 1;
                        }
                        if count_new > count_old {
                            for k in mod_count..count_new {
                                result_lines[prefix_len + start_new + k] = GitLineStatus::Added;
                                added += 1;
                            }
                        } else if count_old > count_new {
                            deleted += count_old - count_new;
                        }
                    } else if count_new > 0 {
                        // Pure addition
                        for k in 0..count_new {
                            result_lines[prefix_len + start_new + k] = GitLineStatus::Added;
                            added += 1;
                        }
                    } else if count_old > 0 {
                        // Pure deletion
                        deleted += count_old;
                        let target_idx = prefix_len + start_new;
                        if target_idx < n {
                            result_lines[target_idx] = GitLineStatus::DeletedAbove;
                        }
                    }
                }
            }
        } else {
            // Fast fallback for gigantic edits: mark difference region as modified
            for i in 0..mid_new_len {
                result_lines[prefix_len + i] = GitLineStatus::Modified;
                modified += 1;
            }
        }
    }

    GitGutterDiff {
        lines: result_lines,
        added_count: added,
        modified_count: modified,
        deleted_count: deleted,
    }
}

/// Git manager responsible for branch detection, repository root finding,
/// file status tracking, and per-document gutter diffs.
#[derive(Debug, Clone)]
pub struct GitManager {
    pub git_root: Option<PathBuf>,
    pub branch: Option<String>,
    pub file_statuses: HashMap<PathBuf, GitFileStatus>,
    pub cached_diffs: HashMap<PathBuf, GitGutterDiff>,
    pub is_enabled: bool,
}

impl GitManager {
    /// Creates a new GitManager starting search from `root_dir`.
    pub fn new(root_dir: &Path) -> Self {
        let git_root = Self::find_git_root(root_dir);
        let branch = git_root.as_deref().and_then(Self::detect_branch);

        let mut mgr = Self {
            git_root,
            branch,
            file_statuses: HashMap::new(),
            cached_diffs: HashMap::new(),
            is_enabled: true,
        };

        mgr.refresh_statuses();
        mgr
    }

    /// Finds `.git` directory or file by walking upwards from `start_dir`.
    pub fn find_git_root(start_dir: &Path) -> Option<PathBuf> {
        let mut cur = if start_dir.is_file() {
            start_dir.parent()?.to_path_buf()
        } else {
            start_dir.to_path_buf()
        };

        loop {
            let git_dir = cur.join(".git");
            if git_dir.exists() {
                return cur.canonicalize().ok().or(Some(cur));
            }
            if !cur.pop() {
                break;
            }
        }
        None
    }

    /// Detects current branch name from `.git/HEAD` without spawning child processes.
    pub fn detect_branch(git_root: &Path) -> Option<String> {
        let head_path = git_root.join(".git").join("HEAD");
        if let Ok(content) = fs::read_to_string(&head_path) {
            let trimmed = content.trim();
            if let Some(branch_ref) = trimmed.strip_prefix("ref: refs/heads/") {
                return Some(branch_ref.to_string());
            }
            if trimmed.len() >= 7 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
                // Detached HEAD: return short SHA
                return Some(trimmed[..7].to_string());
            }
        }

        // Fallback to git CLI if .git is a worktree or complex reference
        let out = Command::new("git")
            .arg("-C")
            .arg(git_root)
            .args(["branch", "--show-current"])
            .output()
            .ok()?;
        if out.status.success() {
            let b = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !b.is_empty() {
                return Some(b);
            }
        }

        None
    }

    /// Refreshes branch name and file statuses from the repository.
    pub fn refresh_statuses(&mut self) {
        let root = match &self.git_root {
            Some(r) => r.clone(),
            None => return,
        };

        self.branch = Self::detect_branch(&root);
        self.file_statuses.clear();

        // Run `git status --porcelain=v1 -uall`
        let output = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["status", "--porcelain=v1", "-uall"])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    if line.len() < 4 {
                        continue;
                    }
                    let status_code = &line[0..2];
                    let rel_path = line[3..].trim();
                    // Handle renamed paths "old -> new"
                    let clean_path = if let Some(idx) = rel_path.find(" -> ") {
                        &rel_path[idx + 4..]
                    } else {
                        rel_path
                    };

                    let full_path = root.join(clean_path);
                    let canon_path = full_path.canonicalize().unwrap_or(full_path);

                    let status = if status_code.contains('M') {
                        GitFileStatus::Modified
                    } else if status_code.contains('?') {
                        GitFileStatus::Untracked
                    } else if status_code.contains('A') {
                        GitFileStatus::Added
                    } else if status_code.contains('D') {
                        GitFileStatus::Deleted
                    } else if status_code.contains('R') {
                        GitFileStatus::Renamed
                    } else {
                        GitFileStatus::Modified
                    };

                    self.file_statuses.insert(canon_path, status);
                }
            }
        }
    }

    /// Retrieves committed file content from Git index or HEAD.
    pub fn get_head_file_content(&self, abs_path: &Path) -> Option<String> {
        let root = self.git_root.as_ref()?;
        let rel_path = abs_path.strip_prefix(root).ok()?;
        let rel_str = rel_path.to_str()?;

        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["show", &format!(":{}", rel_str)])
            .output()
            .ok()?;

        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            None
        }
    }

    /// Computes and caches gutter diff for an open file.
    pub fn diff_document(&mut self, file_path: Option<&Path>, current_text: &str) -> GitGutterDiff {
        if !self.is_enabled {
            return GitGutterDiff::default();
        }

        let path = match file_path {
            Some(p) => p,
            None => {
                // Unsaved untitled buffer: all lines are Added if non-empty
                return compute_gutter_diff("", current_text);
            }
        };

        let head_content = self.get_head_file_content(path).unwrap_or_default();
        let diff = compute_gutter_diff(&head_content, current_text);

        let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.cached_diffs.insert(canon, diff.clone());

        diff
    }

    /// Returns the Git status of a specific file for sidebar badge display.
    pub fn status_for_path(&self, path: &Path) -> Option<GitFileStatus> {
        let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        self.file_statuses.get(&canon).copied()
    }

    /// Returns formatted branch string (e.g. "main" or "feat/ui").
    pub fn branch_name(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    /// Formats a concise Git summary for the status bar (e.g. "git: main (+3 ~1)").
    pub fn status_bar_summary(&self, active_diff: Option<&GitGutterDiff>) -> String {
        let branch = self.branch.as_deref().unwrap_or("sin git");
        if let Some(diff) = active_diff {
            if !diff.is_empty() {
                let mut parts = Vec::new();
                if diff.added_count > 0 {
                    parts.push(format!("+{}", diff.added_count));
                }
                if diff.modified_count > 0 {
                    parts.push(format!("~{}", diff.modified_count));
                }
                if diff.deleted_count > 0 {
                    parts.push(format!("-{}", diff.deleted_count));
                }
                return format!(" {} ({})", branch, parts.join(" "));
            }
        }
        format!(" {}", branch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_diff_identical() {
        let text = "line 1\nline 2\nline 3";
        let diff = compute_gutter_diff(text, text);
        assert_eq!(diff.added_count, 0);
        assert_eq!(diff.modified_count, 0);
        assert_eq!(diff.deleted_count, 0);
        assert_eq!(diff.lines, vec![GitLineStatus::None; 3]);
    }

    #[test]
    fn test_compute_diff_added_lines() {
        let old = "line 1\nline 3";
        let new = "line 1\nline 2\nline 3";
        let diff = compute_gutter_diff(old, new);
        assert_eq!(diff.added_count, 1);
        assert_eq!(diff.modified_count, 0);
        assert_eq!(diff.lines[0], GitLineStatus::None);
        assert_eq!(diff.lines[1], GitLineStatus::Added);
        assert_eq!(diff.lines[2], GitLineStatus::None);
    }

    #[test]
    fn test_compute_diff_modified_line() {
        let old = "fn main() {\n    println!(\"old\");\n}";
        let new = "fn main() {\n    println!(\"new\");\n}";
        let diff = compute_gutter_diff(old, new);
        assert_eq!(diff.modified_count, 1);
        assert_eq!(diff.added_count, 0);
        assert_eq!(diff.lines[0], GitLineStatus::None);
        assert_eq!(diff.lines[1], GitLineStatus::Modified);
        assert_eq!(diff.lines[2], GitLineStatus::None);
    }

    #[test]
    fn test_compute_diff_deleted_lines() {
        let old = "line 1\nline 2\nline 3";
        let new = "line 1\nline 3";
        let diff = compute_gutter_diff(old, new);
        assert_eq!(diff.deleted_count, 1);
        assert_eq!(diff.lines[0], GitLineStatus::None);
        assert_eq!(diff.lines[1], GitLineStatus::DeletedAbove);
    }

    #[test]
    fn test_compute_diff_empty_old() {
        let new = "fn hello() {}\nlet x = 10;";
        let diff = compute_gutter_diff("", new);
        assert_eq!(diff.added_count, 2);
        assert_eq!(diff.lines, vec![GitLineStatus::Added, GitLineStatus::Added]);
    }

    #[test]
    fn test_git_file_status_badges_and_colors() {
        assert_eq!(GitFileStatus::Modified.badge_char(), 'M');
        assert_eq!(GitFileStatus::Untracked.badge_char(), 'U');
        assert_eq!(GitFileStatus::Added.badge_char(), 'A');
        assert_eq!(GitFileStatus::Deleted.badge_char(), 'D');
        assert_eq!(GitFileStatus::Renamed.badge_char(), 'R');
    }

    #[test]
    fn test_git_manager_creation_and_branch() {
        let cur = std::env::current_dir().unwrap();
        let mgr = GitManager::new(&cur);
        assert!(mgr.git_root.is_some());
        assert_eq!(mgr.branch.as_deref(), Some("main"));
    }
}
