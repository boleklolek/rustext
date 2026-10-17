//! Subsequence Fuzzy Matcher with boundary scoring and index tracking.
//! Zero third-party dependencies, purely standard library Rust.

/// Result of a successful fuzzy match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzyMatch {
    pub score: i32,
    /// 0-indexed character positions within `target` where pattern characters matched.
    pub matched_indices: Vec<usize>,
}

/// Performs intelligent fuzzy matching of `pattern` against `target`.
/// Returns `Some(FuzzyMatch)` with calculated relevance score and matched character indices,
/// or `None` if `pattern` is not a subsequence of `target`.
pub fn fuzzy_match(pattern: &str, target: &str) -> Option<FuzzyMatch> {
    if pattern.is_empty() {
        return Some(FuzzyMatch {
            score: 0,
            matched_indices: Vec::new(),
        });
    }

    if target.is_empty() {
        return None;
    }

    let p_chars: Vec<char> = pattern.chars().collect();
    let t_chars: Vec<char> = target.chars().collect();

    let has_uppercase = pattern.chars().any(|c| c.is_uppercase());
    let p_lower: Vec<char> = if has_uppercase {
        p_chars.clone()
    } else {
        pattern.to_lowercase().chars().collect()
    };
    let t_lower: Vec<char> = if has_uppercase {
        t_chars.clone()
    } else {
        target.to_lowercase().chars().collect()
    };

    // Fast O(N) subsequence check
    if !is_subsequence_slice(&p_lower, &t_lower) {
        return None;
    }

    // Generate candidate alignments
    let mut best_indices: Option<Vec<usize>> = None;
    let mut best_score = i32::MIN;

    // Alignment 1: Reverse-greedy (favors filename / end of path)
    if let Some(rev) = reverse_greedy_alignment(&p_lower, &t_lower) {
        let sc = score_alignment(&p_chars, &t_chars, &rev);
        if sc > best_score {
            best_score = sc;
            best_indices = Some(rev);
        }
    }

    // Alignment 2: Forward-greedy (favors beginning of string)
    if let Some(fwd) = forward_greedy_alignment(&p_lower, &t_lower) {
        let sc = score_alignment(&p_chars, &t_chars, &fwd);
        if sc > best_score {
            best_score = sc;
            best_indices = Some(fwd);
        }
    }

    // Alignment 3: Filename-anchored forward greedy
    let filename_start = t_chars
        .iter()
        .rposition(|&c| c == '/' || c == '\\')
        .map(|idx| idx + 1)
        .unwrap_or(0);

    if filename_start > 0 && filename_start < t_chars.len() {
        if let Some(fn_align) = filename_anchored_alignment(&p_lower, &t_lower, filename_start) {
            let sc = score_alignment(&p_chars, &t_chars, &fn_align);
            if sc > best_score {
                best_score = sc;
                best_indices = Some(fn_align);
            }
        }
    }

    best_indices.map(|matched_indices| FuzzyMatch {
        score: best_score,
        matched_indices,
    })
}

/// Checks whether `sub` is a subsequence of `target`.
#[inline]
fn is_subsequence_slice(sub: &[char], target: &[char]) -> bool {
    let mut t_iter = target.iter();
    for &sc in sub {
        if !t_iter.any(|&tc| tc == sc) {
            return false;
        }
    }
    true
}

/// Matches pattern characters from right to left (end of target).
fn reverse_greedy_alignment(pattern: &[char], target: &[char]) -> Option<Vec<usize>> {
    let mut indices = Vec::with_capacity(pattern.len());
    let mut t_idx = target.len();

    for &pc in pattern.iter().rev() {
        if t_idx == 0 {
            return None;
        }
        let mut found = false;
        while t_idx > 0 {
            t_idx -= 1;
            if target[t_idx] == pc {
                indices.push(t_idx);
                found = true;
                break;
            }
        }
        if !found {
            return None;
        }
    }

    indices.reverse();
    Some(indices)
}

/// Matches pattern characters from left to right (start of target).
fn forward_greedy_alignment(pattern: &[char], target: &[char]) -> Option<Vec<usize>> {
    let mut indices = Vec::with_capacity(pattern.len());
    let mut t_idx = 0;

    for &pc in pattern {
        let mut found = false;
        while t_idx < target.len() {
            let cur = t_idx;
            t_idx += 1;
            if target[cur] == pc {
                indices.push(cur);
                found = true;
                break;
            }
        }
        if !found {
            return None;
        }
    }

    Some(indices)
}

/// Matches pattern anchoring first at or after `filename_start`.
fn filename_anchored_alignment(pattern: &[char], target: &[char], filename_start: usize) -> Option<Vec<usize>> {
    if !is_subsequence_slice(pattern, &target[filename_start..]) {
        return None;
    }
    let mut indices = Vec::with_capacity(pattern.len());
    let mut t_idx = filename_start;

    for &pc in pattern {
        let mut found = false;
        while t_idx < target.len() {
            let cur = t_idx;
            t_idx += 1;
            if target[cur] == pc {
                indices.push(cur);
                found = true;
                break;
            }
        }
        if !found {
            return None;
        }
    }

    Some(indices)
}

/// Scores a concrete alignment of matched indices.
fn score_alignment(pattern: &[char], target: &[char], indices: &[usize]) -> i32 {
    let n = target.len();
    let m = pattern.len();
    if m == 0 {
        return 0;
    }

    let filename_start = target
        .iter()
        .rposition(|&c| c == '/' || c == '\\')
        .map(|idx| idx + 1)
        .unwrap_or(0);

    let mut score = 0;
    let mut prev_idx: Option<usize> = None;
    let mut consecutive_count = 0;

    for (p_i, &t_i) in indices.iter().enumerate() {
        let mut char_score = 15; // Base points per matched char

        // 1. Inside filename bonus
        if t_i >= filename_start {
            char_score += 30;
        }

        // 2. Word / delimiter / transition boundary bonus
        if t_i == 0 || t_i == filename_start {
            char_score += 55; // First character of path or filename
        } else {
            let prev_c = target[t_i - 1];
            if prev_c == '/' || prev_c == '\\' || prev_c == '_' || prev_c == '-' || prev_c == '.' || prev_c == ' ' {
                char_score += 45; // Word boundary match
            } else if prev_c.is_lowercase() && target[t_i].is_uppercase() {
                char_score += 40; // CamelCase boundary match
            }
        }

        // 3. Consecutive match bonus
        if let Some(prev) = prev_idx {
            if t_i == prev + 1 {
                consecutive_count += 1;
                char_score += 25 + consecutive_count * 15;
            } else {
                consecutive_count = 0;
                let gap = t_i - prev - 1;
                char_score -= (gap as i32).min(10) * 3; // Gap penalty
            }
        }

        // 4. Exact case match bonus
        if pattern[p_i] == target[t_i] {
            char_score += 5;
        }

        score += char_score;
        prev_idx = Some(t_i);
    }

    // 5. Full filename exact or prefix match
    let filename_len = n.saturating_sub(filename_start);
    if indices.first().copied() == Some(filename_start) {
        if m == filename_len {
            score += 250; // Exact filename match (e.g. "app.rs")
        } else {
            score += 70; // Prefix match of filename
        }
    }

    // 6. Path length penalty (prefer concise paths)
    score -= (n as i32) / 2;

    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match_empty() {
        let m = fuzzy_match("", "src/main.rs");
        assert!(m.is_some());
        assert_eq!(m.unwrap().score, 0);

        let m2 = fuzzy_match("main", "");
        assert!(m2.is_none());
    }

    #[test]
    fn test_fuzzy_match_subsequence() {
        assert!(fuzzy_match("main", "src/main.rs").is_some());
        assert!(fuzzy_match("xyz", "src/main.rs").is_none());
    }

    #[test]
    fn test_fuzzy_match_indices() {
        let m = fuzzy_match("mrs", "src/main.rs").unwrap();
        // 'm' at 4, 'r' at 9, 's' at 10
        assert_eq!(m.matched_indices, vec![4, 9, 10]);
    }

    #[test]
    fn test_fuzzy_match_prefers_filename_boundary() {
        let m1 = fuzzy_match("app", "src/application/app.rs").unwrap();
        // Should anchor to app.rs, not application
        assert_eq!(m1.matched_indices, vec![16, 17, 18]);
    }

    #[test]
    fn test_fuzzy_match_ranking_precision() {
        let m_exact = fuzzy_match("app.rs", "src/editor/app.rs").unwrap();
        let m_fuzzy = fuzzy_match("app.rs", "src/appearance_options.rs").unwrap();
        assert!(m_exact.score > m_fuzzy.score);
    }
}
