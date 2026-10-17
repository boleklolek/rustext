//! Project-wide global search engine across file trees.
//! Zero third-party dependencies, purely standard library Rust.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

/// A single matched occurrence inside a project file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalSearchMatch {
    /// Full path to the file on disk.
    pub file_path: PathBuf,
    /// Relative path display string (e.g. "src/editor/app.rs").
    pub relative_path: String,
    /// 1-indexed line number where the match occurs.
    pub line_number: usize,
    /// 1-indexed character column offset within the line.
    pub col: usize,
    /// Display snippet of the line (trimmed or windowed for long lines).
    pub line_text: String,
    /// Character offset of the match start inside `line_text` for UI highlighting.
    pub match_char_offset: usize,
    /// Character length of the match.
    pub match_char_len: usize,
}

/// Aggregated search results and metrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalSearchResults {
    /// Searched query.
    pub query: String,
    /// All matched occurrences found.
    pub matches: Vec<GlobalSearchMatch>,
    /// Number of files scanned.
    pub total_files_scanned: usize,
    /// Number of unique files containing matches.
    pub matching_files_count: usize,
    /// Case-sensitivity flag used.
    pub case_sensitive: bool,
    /// True if search stopped early due to hit limit.
    pub truncated: bool,
}

impl GlobalSearchResults {
    pub fn empty(query: String, case_sensitive: bool) -> Self {
        Self {
            query,
            matches: Vec::new(),
            total_files_scanned: 0,
            matching_files_count: 0,
            case_sensitive,
            truncated: false,
        }
    }
}

/// Project search utility.
pub struct GlobalSearchEngine;

impl GlobalSearchEngine {
    /// Recursively scans `root_dir` for all occurrences of `query`.
    pub fn search_project(
        root_dir: &Path,
        query: &str,
        case_sensitive: bool,
        max_matches: usize,
    ) -> GlobalSearchResults {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            return GlobalSearchResults::empty(query.to_string(), case_sensitive);
        }

        let mut matches = Vec::new();
        let mut total_files_scanned = 0;
        let mut matching_files = HashSet::new();
        let mut truncated = false;

        let query_chars: Vec<char> = trimmed_query.chars().collect();
        let query_lower: Vec<char> = trimmed_query.to_lowercase().chars().collect();

        Self::scan_dir_recursive(
            root_dir,
            root_dir,
            0,
            &query_chars,
            &query_lower,
            case_sensitive,
            max_matches,
            &mut matches,
            &mut total_files_scanned,
            &mut matching_files,
            &mut truncated,
        );

        let matching_files_count = matching_files.len();

        GlobalSearchResults {
            query: query.to_string(),
            matches,
            total_files_scanned,
            matching_files_count,
            case_sensitive,
            truncated,
        }
    }

    fn scan_dir_recursive(
        root_dir: &Path,
        current_dir: &Path,
        depth: usize,
        query_chars: &[char],
        query_lower: &[char],
        case_sensitive: bool,
        max_matches: usize,
        matches: &mut Vec<GlobalSearchMatch>,
        total_files_scanned: &mut usize,
        matching_files: &mut HashSet<PathBuf>,
        truncated: &mut bool,
    ) {
        // Limit recursion depth to 32 to prevent stack overflow from deep trees (SEC-04)
        if *truncated || depth > 32 {
            return;
        }

        let entries = match fs::read_dir(current_dir) {
            Ok(rd) => rd,
            Err(_) => return,
        };

        let mut subdirs = Vec::new();
        let mut files = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };

            // Ignore common build artifacts, version control, and temporary folders
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
            ) {
                continue;
            }

            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            // Do not follow symlinks to avoid cyclical directory links and infinite recursion (SEC-04 / CWE-59)
            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                subdirs.push(path);
            } else if file_type.is_file() {
                files.push(path);
            }
        }

        // Sort paths for deterministic results
        files.sort();
        subdirs.sort();

        for file_path in files {
            if *truncated {
                return;
            }

            *total_files_scanned += 1;
            Self::search_file(
                root_dir,
                &file_path,
                query_chars,
                query_lower,
                case_sensitive,
                max_matches,
                matches,
                matching_files,
                truncated,
            );
        }

        for subdir in subdirs {
            Self::scan_dir_recursive(
                root_dir,
                &subdir,
                depth + 1,
                query_chars,
                query_lower,
                case_sensitive,
                max_matches,
                matches,
                total_files_scanned,
                matching_files,
                truncated,
            );
        }
    }

    fn search_file(
        root_dir: &Path,
        file_path: &Path,
        query_chars: &[char],
        query_lower: &[char],
        case_sensitive: bool,
        max_matches: usize,
        matches: &mut Vec<GlobalSearchMatch>,
        matching_files: &mut HashSet<PathBuf>,
        truncated: &mut bool,
    ) {
        // Skip files larger than 10 MB
        if let Ok(meta) = fs::metadata(file_path) {
            if meta.len() > 10 * 1024 * 1024 {
                return;
            }
        }

        let mut file = match File::open(file_path) {
            Ok(f) => f,
            Err(_) => return,
        };

        // Binary check: read first 512 bytes
        let mut sample = [0u8; 512];
        let bytes_read = match file.read(&mut sample) {
            Ok(n) => n,
            Err(_) => return,
        };
        if sample[..bytes_read].contains(&0) {
            return; // Skip binary files
        }

        // Re-open buffered reader for line streaming
        let file = match File::open(file_path) {
            Ok(f) => f,
            Err(_) => return,
        };
        let reader = BufReader::new(file);

        let relative_path = file_path
            .strip_prefix(root_dir)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        let q_len = query_chars.len();
        if q_len == 0 {
            return;
        }

        for (line_idx, line_res) in reader.lines().enumerate() {
            let line_str = match line_res {
                Ok(l) => l,
                Err(_) => continue,
            };

            let line_number = line_idx + 1;
            let line_chars: Vec<char> = line_str.chars().collect();
            let total_chars = line_chars.len();
            if total_chars < q_len {
                continue;
            }

            let mut col_matches = Vec::new();

            if case_sensitive {
                for i in 0..=(total_chars - q_len) {
                    if line_chars[i..i + q_len] == query_chars[..] {
                        col_matches.push(i);
                    }
                }
            } else {
                let line_lower: Vec<char> = line_str.to_lowercase().chars().collect();
                if line_lower.len() >= q_len {
                    for i in 0..=(line_lower.len() - q_len) {
                        if line_lower[i..i + q_len] == query_lower[..] {
                            col_matches.push(i);
                        }
                    }
                }
            }

            for char_idx in col_matches {
                let col = char_idx + 1;

                // Format display snippet
                let (line_text, match_char_offset) = if total_chars <= 120 {
                    let lead_spaces = line_chars.iter().take_while(|c| c.is_whitespace()).count();
                    let trimmed: String = line_chars[lead_spaces..].iter().collect();
                    let offset = char_idx.saturating_sub(lead_spaces);
                    (trimmed, offset)
                } else {
                    // Long line windowing around match
                    let win_start = char_idx.saturating_sub(25);
                    let win_end = (char_idx + q_len + 45).min(total_chars);
                    let prefix = if win_start > 0 { "..." } else { "" };
                    let suffix = if win_end < total_chars { "..." } else { "" };
                    let inner: String = line_chars[win_start..win_end].iter().collect();
                    let text = format!("{}{}{}", prefix, inner, suffix);
                    let offset = prefix.chars().count() + char_idx.saturating_sub(win_start);
                    (text, offset)
                };

                matches.push(GlobalSearchMatch {
                    file_path: file_path.to_path_buf(),
                    relative_path: relative_path.clone(),
                    line_number,
                    col,
                    line_text,
                    match_char_offset,
                    match_char_len: q_len,
                });

                matching_files.insert(file_path.to_path_buf());

                if matches.len() >= max_matches {
                    *truncated = true;
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_global_search_basic() {
        let temp_dir = std::env::temp_dir().join(format!("rustext_test_gs_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let file1 = temp_dir.join("main.rs");
        let file2 = temp_dir.join("lib.rs");
        let ignored_dir = temp_dir.join(".git");
        let _ = fs::create_dir_all(&ignored_dir);
        let ignored_file = ignored_dir.join("config");

        fs::write(&file1, "fn main() {\n    let answer = 42;\n    println!(\"{}\", answer);\n}\n").unwrap();
        fs::write(&file2, "pub const ANSWER_VALUE: i32 = 42;\n").unwrap();
        fs::write(&ignored_file, "answer = 42\n").unwrap();

        // Search case-insensitive
        let res = GlobalSearchEngine::search_project(&temp_dir, "answer", false, 100);
        assert_eq!(res.matches.len(), 3);
        assert_eq!(res.matching_files_count, 2);
        assert!(!res.truncated);

        // Search case-sensitive
        let res_sens = GlobalSearchEngine::search_project(&temp_dir, "ANSWER", true, 100);
        assert_eq!(res_sens.matches.len(), 1);
        assert_eq!(res_sens.matches[0].relative_path, "lib.rs");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_global_search_empty_and_max_limit() {
        let temp_dir = std::env::temp_dir().join(format!("rustext_test_limit_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let file = temp_dir.join("test.txt");
        fs::write(&file, "foo\nfoo\nfoo\nfoo\nfoo\n").unwrap();

        // Empty query
        let empty_res = GlobalSearchEngine::search_project(&temp_dir, "", false, 100);
        assert!(empty_res.matches.is_empty());

        // Max limit truncation
        let limit_res = GlobalSearchEngine::search_project(&temp_dir, "foo", false, 3);
        assert_eq!(limit_res.matches.len(), 3);
        assert!(limit_res.truncated);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
