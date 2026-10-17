//! Code Folding Engine.
//! Provides syntax-aware and indentation-aware code block folding.
//! Pure standard-library Rust with zero third-party dependencies (`std::*`).

use crate::core::buffer::TextBuffer;
use crate::core::syntax::{Language, SyntaxHighlighter, TokenType};

/// Represents a collapsible block of code spanning from `start_line` to `end_line`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldRegion {
    /// 0-indexed line where the block begins (visible line with collapse indicator).
    pub start_line: usize,
    /// 0-indexed line where the block ends.
    pub end_line: usize,
    /// Whether this region is currently folded (lines `start_line + 1 ..= end_line` hidden).
    pub is_folded: bool,
}

/// Manages folding regions, state persistence, and visibility queries for a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldingManager {
    pub regions: Vec<FoldRegion>,
    pub enabled: bool,
    last_version: u64,
}

impl Default for FoldingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FoldingManager {
    /// Creates a new folding manager with code folding enabled.
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            enabled: true,
            last_version: u64::MAX,
        }
    }

    /// Recomputes fold regions for the buffer if the buffer version has changed.
    /// Preserves existing folded states across edits whenever possible.
    pub fn recompute_regions(&mut self, buffer: &TextBuffer, language: Language) {
        let current_version = buffer.version();
        if current_version == self.last_version {
            return;
        }

        // Remember existing folded lines to preserve state
        let previously_folded: Vec<usize> = self
            .regions
            .iter()
            .filter(|r| r.is_folded)
            .map(|r| r.start_line)
            .collect();

        let total_lines = buffer.len_lines();
        if total_lines < 2 {
            self.regions.clear();
            self.last_version = current_version;
            return;
        }

        let mut candidate_regions: Vec<FoldRegion> = Vec::new();

        // 1. Syntax Brace Matching (for Rust, C, JavaScript, JSON, CSS, etc.)
        let brace_langs = matches!(
            language,
            Language::Rust
                | Language::C
                | Language::JavaScript
                | Language::Json
                | Language::Css
                | Language::Html
        );

        if brace_langs {
            let mut brace_stack: Vec<usize> = Vec::new();
            for line_idx in 0..total_lines {
                let line_cow = buffer.line_cow(line_idx).unwrap_or(std::borrow::Cow::Borrowed(""));
                let tokens = SyntaxHighlighter::tokenize_line(&line_cow, language);

                for span in &tokens {
                    // Only match braces outside strings and comments
                    if span.token_type != TokenType::Comment && span.token_type != TokenType::StringLiteral {
                        for ch in span.text.chars() {
                            match ch {
                                '{' => brace_stack.push(line_idx),
                                '}' => {
                                    if let Some(start_line) = brace_stack.pop().filter(|&s| line_idx > s) {
                                        candidate_regions.push(FoldRegion {
                                            start_line,
                                            end_line: line_idx,
                                            is_folded: previously_folded.contains(&start_line),
                                        });
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        // 2. Indentation-based folding (for Python, YAML, Markdown, Shell, or fallback)
        let needs_indent_folding = candidate_regions.is_empty() || matches!(language, Language::Python | Language::Yaml);

        if needs_indent_folding {
            let mut line_indents: Vec<Option<usize>> = Vec::with_capacity(total_lines);
            for line_idx in 0..total_lines {
                let line_cow = buffer.line_cow(line_idx).unwrap_or(std::borrow::Cow::Borrowed(""));
                let trimmed = line_cow.trim_start();
                if trimmed.is_empty() {
                    line_indents.push(None); // Empty line
                } else {
                    let mut spaces = 0;
                    for ch in line_cow.chars() {
                        if ch == ' ' {
                            spaces += 1;
                        } else if ch == '\t' {
                            spaces += 4;
                        } else {
                            break;
                        }
                    }
                    line_indents.push(Some(spaces));
                }
            }

            // O(N) Stack-based indentation folding
            // Stack entries: (parent_line, parent_indent, last_child_line)
            let mut stack: Vec<(usize, usize, usize)> = Vec::new();

            for line_idx in 0..total_lines {
                if let Some(indent) = line_indents[line_idx] {
                    // While current line has less or equal indentation than stack top:
                    while let Some(&(start_line, stack_indent, last_child)) = stack.last() {
                        if indent <= stack_indent {
                            stack.pop();
                            if last_child > start_line {
                                candidate_regions.push(FoldRegion {
                                    start_line,
                                    end_line: last_child,
                                    is_folded: previously_folded.contains(&start_line),
                                });
                            }
                        } else {
                            break;
                        }
                    }

                    // Update last_child of all enclosing parent scopes
                    for parent in stack.iter_mut() {
                        parent.2 = line_idx;
                    }

                    // Push current line as potential parent for lines with deeper indentation
                    stack.push((line_idx, indent, line_idx));
                }
            }

            // Close remaining open regions at EOF
            while let Some((start_line, _, last_child)) = stack.pop() {
                if last_child > start_line {
                    candidate_regions.push(FoldRegion {
                        start_line,
                        end_line: last_child,
                        is_folded: previously_folded.contains(&start_line),
                    });
                }
            }
        }

        // Deduplicate: if multiple regions start on the same line, pick the largest (outermost)
        candidate_regions.sort_by(|a, b| {
            if a.start_line == b.start_line {
                b.end_line.cmp(&a.end_line)
            } else {
                a.start_line.cmp(&b.start_line)
            }
        });

        let mut deduped: Vec<FoldRegion> = Vec::with_capacity(candidate_regions.len());
        for region in candidate_regions {
            if let Some(last) = deduped.last_mut().filter(|l| l.start_line == region.start_line) {
                // Keep the outermost region
                if region.end_line > last.end_line {
                    last.end_line = region.end_line;
                }
                continue;
            }
            deduped.push(region);
        }

        self.regions = deduped;
        self.last_version = current_version;
    }

    /// Returns `Some(true)` if `line` starts a folded region,
    /// `Some(false)` if `line` starts an unfolded region,
    /// or `None` if `line` is not a fold start.
    pub fn is_fold_start(&self, line: usize) -> Option<bool> {
        if !self.enabled {
            return None;
        }
        for r in &self.regions {
            if r.start_line == line {
                return Some(r.is_folded);
            }
        }
        None
    }

    /// Checks if a line is hidden inside any active folded region.
    pub fn is_line_hidden(&self, line: usize) -> bool {
        if !self.enabled {
            return false;
        }
        for r in &self.regions {
            if r.is_folded && line > r.start_line && line <= r.end_line {
                return true;
            }
        }
        false
    }

    /// Returns the end line if `line` is the start of a currently folded block.
    pub fn fold_end_for_line(&self, line: usize) -> Option<usize> {
        if !self.enabled {
            return None;
        }
        for r in &self.regions {
            if r.start_line == line && r.is_folded {
                return Some(r.end_line);
            }
        }
        None
    }

    /// Toggles the fold state at `line` (or the enclosing block if on an inner line).
    /// Returns `true` if a fold region state was changed.
    pub fn toggle_fold_at_line(&mut self, line: usize) -> bool {
        if !self.enabled {
            return false;
        }

        // 1. Direct match on start_line
        for r in &mut self.regions {
            if r.start_line == line {
                r.is_folded = !r.is_folded;
                return true;
            }
        }

        // 2. Enclosing region match (innermost block containing line)
        let mut best_idx = None;
        let mut min_span = usize::MAX;
        for (i, r) in self.regions.iter().enumerate() {
            if line >= r.start_line && line <= r.end_line {
                let span = r.end_line.saturating_sub(r.start_line);
                if span < min_span {
                    min_span = span;
                    best_idx = Some(i);
                }
            }
        }

        if let Some(idx) = best_idx {
            self.regions[idx].is_folded = !self.regions[idx].is_folded;
            true
        } else {
            false
        }
    }

    /// Folds the block at `line` or enclosing `line`.
    pub fn fold_at_line(&mut self, line: usize) -> bool {
        if !self.enabled {
            return false;
        }
        for r in &mut self.regions {
            if r.start_line == line {
                if !r.is_folded {
                    r.is_folded = true;
                    return true;
                }
                return false;
            }
        }
        // Enclosing
        for r in &mut self.regions {
            if line >= r.start_line && line <= r.end_line && !r.is_folded {
                r.is_folded = true;
                return true;
            }
        }
        false
    }

    /// Unfolds the block at `line` or enclosing `line`.
    pub fn unfold_at_line(&mut self, line: usize) -> bool {
        if !self.enabled {
            return false;
        }
        let mut changed = false;
        for r in &mut self.regions {
            if (r.start_line == line || (line >= r.start_line && line <= r.end_line)) && r.is_folded {
                r.is_folded = false;
                changed = true;
            }
        }
        changed
    }

    /// Folds all collapsible blocks in the document.
    pub fn fold_all(&mut self) {
        if !self.enabled {
            return;
        }
        for r in &mut self.regions {
            r.is_folded = true;
        }
    }

    /// Unfolds all collapsible blocks in the document.
    pub fn unfold_all(&mut self) {
        for r in &mut self.regions {
            r.is_folded = false;
        }
    }

    /// Unfolds any collapsed regions that hide `line`, ensuring `line` is fully visible.
    /// Returns `true` if any region was unfolded.
    pub fn unfold_to_reveal(&mut self, line: usize) -> bool {
        let mut revealed = false;
        for r in &mut self.regions {
            if r.is_folded && line > r.start_line && line <= r.end_line {
                r.is_folded = false;
                revealed = true;
            }
        }
        revealed
    }

    /// Computes the next visible line when moving down from `current_line`.
    pub fn next_visible_line(&self, current_line: usize, total_lines: usize) -> usize {
        if !self.enabled || self.regions.is_empty() {
            return (current_line + 1).min(total_lines.saturating_sub(1));
        }

        let mut l = current_line + 1;
        while l < total_lines {
            if self.fold_end_for_line(l).is_some() {
                // If l is itself a folded start, it is visible!
                return l;
            }
            if self.is_line_hidden(l) {
                // Jump to the end of the folding region hiding this line
                for r in &self.regions {
                    if r.is_folded && l > r.start_line && l <= r.end_line {
                        l = r.end_line + 1;
                        break;
                    }
                }
            } else {
                return l;
            }
        }
        current_line.min(total_lines.saturating_sub(1))
    }

    /// Computes the previous visible line when moving up from `current_line`.
    pub fn prev_visible_line(&self, current_line: usize) -> usize {
        if !self.enabled || current_line == 0 || self.regions.is_empty() {
            return current_line.saturating_sub(1);
        }

        let mut l = current_line - 1;
        while l > 0 && self.is_line_hidden(l) {
            for r in &self.regions {
                if r.is_folded && l > r.start_line && l <= r.end_line {
                    l = r.start_line;
                    return l;
                }
            }
            l = l.saturating_sub(1);
        }
        l
    }

    /// Returns the number of currently hidden lines due to active folds.
    pub fn hidden_line_count(&self) -> usize {
        if !self.enabled {
            return 0;
        }
        let mut hidden = 0;
        for r in &self.regions {
            if r.is_folded {
                hidden += r.end_line.saturating_sub(r.start_line);
            }
        }
        hidden
    }

    /// Maps a screen row offset from `scroll_row` to the corresponding visible buffer line index.
    pub fn visible_line_for_screen_row(
        &self,
        scroll_row: usize,
        screen_row: usize,
        total_lines: usize,
    ) -> Option<usize> {
        if total_lines == 0 {
            return None;
        }
        let mut cur = scroll_row.min(total_lines.saturating_sub(1));
        while cur > 0 && self.is_line_hidden(cur) {
            cur = self.prev_visible_line(cur);
        }

        for _ in 0..screen_row {
            let next = self.next_visible_line(cur, total_lines);
            if next <= cur {
                return None;
            }
            cur = next;
        }
        if cur < total_lines {
            Some(cur)
        } else {
            None
        }
    }

    /// Returns the 0-indexed line numbers of active scope headers that enclose `scroll_row`
    /// but have scrolled above the visible viewport (`start_line < scroll_row <= end_line`).
    ///
    /// The scopes are sorted ascending by `start_line` (from outermost scope to innermost scope)
    /// and capped at `max_lines`.
    pub fn sticky_lines(&self, scroll_row: usize, max_lines: usize) -> Vec<usize> {
        if !self.enabled || max_lines == 0 || scroll_row == 0 {
            return Vec::new();
        }

        let mut scopes: Vec<&FoldRegion> = self
            .regions
            .iter()
            .filter(|r| !r.is_folded && r.start_line < scroll_row && r.end_line >= scroll_row)
            .collect();

        // Sort by start_line ascending (outermost to innermost).
        // If two regions share the same start_line, pick the larger end_line (outermost).
        scopes.sort_by(|a, b| {
            if a.start_line == b.start_line {
                b.end_line.cmp(&a.end_line)
            } else {
                a.start_line.cmp(&b.start_line)
            }
        });

        let mut result = Vec::with_capacity(max_lines.min(scopes.len()));
        for r in scopes {
            if !result.contains(&r.start_line) {
                result.push(r.start_line);
                if result.len() == max_lines {
                    break;
                }
            }
        }
        result
    }
}
