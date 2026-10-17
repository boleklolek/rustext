//! Intelligent Autocomplete Engine with floating candidate matching and ranking.
//! Zero third-party dependencies, purely standard library Rust.

use std::collections::HashSet;
use crate::core::buffer::{Position, TextBuffer};
use crate::core::syntax::SyntaxHighlighter;
use crate::editor::document::Document;
use crate::editor::snippet::SnippetRegistry;

/// Classification category of a completion suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword,
    Type,
    Function,
    Snippet,
    Word,
}

impl CompletionKind {
    /// Short 4-char badge displayed inside category chip.
    pub const fn badge(&self) -> &'static str {
        match self {
            CompletionKind::Keyword => "[kw]",
            CompletionKind::Type => "[type]",
            CompletionKind::Function => "[fn]",
            CompletionKind::Snippet => "[snip]",
            CompletionKind::Word => "[word]",
        }
    }

    /// Primary accent color hex for the chip category.
    pub const fn badge_color(&self) -> u32 {
        match self {
            CompletionKind::Keyword => 0x00BB9AF7,  // Lavender / Magenta
            CompletionKind::Type => 0x002AC3DE,     // Cyan
            CompletionKind::Function => 0x007AA2F7, // Accent Blue
            CompletionKind::Snippet => 0x00FF9E64,  // Orange / Amber
            CompletionKind::Word => 0x007DCFFF,     // Soft Slate Blue
        }
    }
}

/// A single completion candidate displayed in the popup list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub detail: String,
    pub documentation: String,
    pub kind: CompletionKind,
    pub score: i32,
    pub insert_text: Option<String>,
}

impl CompletionItem {
    pub fn new(label: impl Into<String>, detail: impl Into<String>, kind: CompletionKind) -> Self {
        Self {
            label: label.into(),
            detail: detail.into(),
            documentation: String::new(),
            kind,
            score: 0,
            insert_text: None,
        }
    }

    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.documentation = doc.into();
        self
    }

    pub fn with_insert_text(mut self, text: impl Into<String>) -> Self {
        self.insert_text = Some(text.into());
        self
    }
}

/// State tracking the active autocomplete popup session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutocompleteState {
    pub is_active: bool,
    pub prefix: String,
    pub items: Vec<CompletionItem>,
    pub selected_idx: usize,
    pub scroll_offset: usize,
    pub anchor_char: usize,
    pub max_visible: usize,
}

impl Default for AutocompleteState {
    fn default() -> Self {
        Self::new()
    }
}

impl AutocompleteState {
    pub fn new() -> Self {
        Self {
            is_active: false,
            prefix: String::new(),
            items: Vec::new(),
            selected_idx: 0,
            scroll_offset: 0,
            anchor_char: 0,
            max_visible: 7,
        }
    }

    /// Opens the autocomplete menu with a candidate list and prefix.
    pub fn open(&mut self, prefix: String, anchor_char: usize, items: Vec<CompletionItem>) {
        if items.is_empty() {
            self.close();
            return;
        }
        self.is_active = true;
        self.prefix = prefix;
        self.anchor_char = anchor_char;
        self.items = items;
        self.selected_idx = 0;
        self.scroll_offset = 0;
    }

    /// Closes and resets the autocomplete state.
    pub fn close(&mut self) {
        self.is_active = false;
        self.prefix.clear();
        self.items.clear();
        self.selected_idx = 0;
        self.scroll_offset = 0;
        self.anchor_char = 0;
    }

    /// Merges new items (e.g. from LSP completion) without duplicates.
    pub fn merge_items(&mut self, new_items: Vec<CompletionItem>) {
        if new_items.is_empty() {
            return;
        }
        for item in new_items {
            if !self.items.iter().any(|existing| existing.label == item.label) {
                self.items.push(item);
            }
        }
        if !self.items.is_empty() {
            self.is_active = true;
        }
    }

    /// Cycles forward to the next candidate (with wrap-around).
    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        if self.selected_idx + 1 < self.items.len() {
            self.selected_idx += 1;
        } else {
            self.selected_idx = 0;
        }
        self.adjust_scroll();
    }

    /// Cycles backward to the previous candidate (with wrap-around).
    pub fn select_prev(&mut self) {
        if self.items.is_empty() {
            return;
        }
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
        } else {
            self.selected_idx = self.items.len().saturating_sub(1);
        }
        self.adjust_scroll();
    }

    /// Returns the currently highlighted completion candidate.
    pub fn selected_item(&self) -> Option<&CompletionItem> {
        self.items.get(self.selected_idx)
    }

    /// Keeps `scroll_offset` synchronized with `selected_idx`.
    pub fn adjust_scroll(&mut self) {
        let max_vis = self.max_visible.max(1);
        if self.selected_idx < self.scroll_offset {
            self.scroll_offset = self.selected_idx;
        } else if self.selected_idx >= self.scroll_offset + max_vis {
            self.scroll_offset = self.selected_idx + 1 - max_vis;
        }
    }

    /// Checks if a character belongs to an identifier name.
    #[inline]
    pub fn is_identifier_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }

    /// Extracts the identifier prefix immediately preceding cursor `head`.
    /// Returns `(prefix_string, start_char_offset)`.
    pub fn extract_word_prefix(buffer: &TextBuffer, head: usize) -> (String, usize) {
        if head == 0 {
            return (String::new(), 0);
        }
        let mut start = head;
        while start > 0 {
            let prev_idx = start - 1;
            if let Some(ch) = buffer.char_at(prev_idx) {
                if Self::is_identifier_char(ch) {
                    start -= 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        let prefix = buffer.get_text_range(start, head);
        (prefix, start)
    }

    /// Collects all candidate suggestions across snippets, keywords, types, and buffer identifiers.
    pub fn collect_candidates(
        doc: &Document,
        other_docs: &[Document],
        _cursor_head: usize,
    ) -> Vec<CompletionItem> {
        let mut candidates = Vec::with_capacity(128);
        let mut seen = HashSet::new();

        // 1. Snippets (highest priority for templated structures)
        for snip in SnippetRegistry::list_for_language(doc.language) {
            if seen.insert(snip.trigger.to_string()) {
                candidates.push(CompletionItem::new(
                    snip.trigger,
                    snip.description,
                    CompletionKind::Snippet,
                ));
            }
        }

        // 2. Language Keywords
        for &kw in SyntaxHighlighter::keywords(doc.language) {
            if seen.insert(kw.to_string()) {
                candidates.push(CompletionItem::new(kw, "Keyword", CompletionKind::Keyword));
            }
        }

        // 3. Language Built-in Types
        for &ty in SyntaxHighlighter::types(doc.language) {
            if seen.insert(ty.to_string()) {
                candidates.push(CompletionItem::new(ty, "Type", CompletionKind::Type));
            }
        }

        // 4. Identifiers & functions from current document buffer
        let cursor_positions: Vec<Position> = doc.cursors.cursors().iter().map(|c| doc.buffer.char_to_position(c.head)).collect();
        Self::collect_buffer_identifiers(&doc.buffer, &cursor_positions, &mut seen, &mut candidates);

        // 5. Identifiers from other open tab buffers (limit to 500 lines per tab for instant speed)
        for other in other_docs {
            if other.buffer.file_path() != doc.buffer.file_path() {
                Self::collect_buffer_identifiers(&other.buffer, &[], &mut seen, &mut candidates);
            }
        }

        candidates
    }

    /// Scans a buffer for identifier words and potential function calls.
    fn collect_buffer_identifiers(
        buffer: &TextBuffer,
        cursor_positions: &[Position],
        seen: &mut HashSet<String>,
        candidates: &mut Vec<CompletionItem>,
    ) {
        let total_lines = buffer.len_lines();
        if total_lines == 0 {
            return;
        }

        let (start_line, end_line) = if total_lines <= 2500 {
            (0, total_lines)
        } else {
            let primary_line = cursor_positions.first().map(|p| p.line).unwrap_or(0);
            let s = primary_line.saturating_sub(800);
            let e = (primary_line + 800).min(total_lines);
            (s, e)
        };

        for line_idx in start_line..end_line {
            if let Some(line) = buffer.line(line_idx) {
                let chars: Vec<char> = line.chars().collect();
                let len = chars.len();
                let mut i = 0;

                while i < len {
                    if Self::is_identifier_char(chars[i]) && !chars[i].is_ascii_digit() {
                        let word_start = i;
                        while i < len && Self::is_identifier_char(chars[i]) {
                            i += 1;
                        }
                        let word_str: String = chars[word_start..i].iter().collect();

                        // Skip the word token currently being typed under any active cursor
                        if cursor_positions.iter().any(|pos| line_idx == pos.line && word_start <= pos.col && i >= pos.col) {
                            continue;
                        }

                        if word_str.len() >= 2 && seen.insert(word_str.clone()) {
                            // Check if followed by '(' -> function/method
                            let mut is_fn = false;
                            let mut peek = i;
                            while peek < len && chars[peek].is_whitespace() {
                                peek += 1;
                            }
                            if peek < len && chars[peek] == '(' {
                                is_fn = true;
                            }

                            let (detail, kind) = if is_fn {
                                ("Function", CompletionKind::Function)
                            } else {
                                ("Identifier", CompletionKind::Word)
                            };

                            candidates.push(CompletionItem::new(word_str, detail, kind));
                        }
                    } else {
                        i += 1;
                    }
                }
            }
        }
    }

    /// Filters and ranks candidate items against the user prefix.
    pub fn filter_and_rank(candidates: &[CompletionItem], prefix: &str) -> Vec<CompletionItem> {
        let prefix_clean = prefix.trim();
        let prefix_lower = prefix_clean.to_lowercase();

        let mut scored_items: Vec<CompletionItem> = Vec::with_capacity(candidates.len());

        for c in candidates {
            let label = &c.label;
            let label_lower = label.to_lowercase();

            let score = if prefix_clean.is_empty() {
                // No prefix: order by kind base priority
                match c.kind {
                    CompletionKind::Snippet => 40,
                    CompletionKind::Keyword => 30,
                    CompletionKind::Type => 20,
                    CompletionKind::Function => 15,
                    CompletionKind::Word => 10,
                }
            } else if label == prefix_clean {
                if c.kind == CompletionKind::Word {
                    // Do not suggest replacing an identical word identifier with itself
                    continue;
                }
                1000
            } else if label.starts_with(prefix_clean) {
                500 - (label.len() as i32 - prefix_clean.len() as i32)
            } else if label_lower.starts_with(&prefix_lower) {
                350 - (label.len() as i32 - prefix_clean.len() as i32)
            } else if let Some(pos) = label_lower.find(&prefix_lower) {
                200 - (pos as i32 * 10) - (label.len() as i32)
            } else if is_subsequence(&prefix_lower, &label_lower) {
                100 - (label.len() as i32)
            } else {
                continue;
            };

            let kind_bonus = match c.kind {
                CompletionKind::Snippet => 25,
                CompletionKind::Keyword => 20,
                CompletionKind::Type => 15,
                CompletionKind::Function => 10,
                CompletionKind::Word => 5,
            };

            let mut item = c.clone();
            item.score = score + kind_bonus;
            scored_items.push(item);
        }

        // Sort descending by score, then alphabetically
        scored_items.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.label.cmp(&b.label)));

        // Truncate to top 35 results for optimal UI performance
        scored_items.truncate(35);
        scored_items
    }
}

/// Checks if `sub` is a fuzzy subsequence of `target`.
fn is_subsequence(sub: &str, target: &str) -> bool {
    let mut target_chars = target.chars();
    for sc in sub.chars() {
        if !target_chars.any(|tc| tc == sc) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_word_prefix() {
        let mut buf = TextBuffer::new();
        buf.insert(0, "let my_variable = 42;");

        // Cursor at end of "my_variable" (index 15)
        let (prefix, start) = AutocompleteState::extract_word_prefix(&buf, 15);
        assert_eq!(prefix, "my_variable");
        assert_eq!(start, 4);

        // Cursor at middle of "let" (index 2)
        let (prefix, start) = AutocompleteState::extract_word_prefix(&buf, 2);
        assert_eq!(prefix, "le");
        assert_eq!(start, 0);

        // Cursor right after space (index 4)
        let (prefix, start) = AutocompleteState::extract_word_prefix(&buf, 4);
        assert_eq!(prefix, "");
        assert_eq!(start, 4);
    }

    #[test]
    fn test_candidate_scoring_and_ranking() {
        let candidates = vec![
            CompletionItem::new("struct", "Definición", CompletionKind::Snippet),
            CompletionItem::new("str", "Type", CompletionKind::Type),
            CompletionItem::new("String", "Type", CompletionKind::Type),
            CompletionItem::new("stream_read", "Identifier", CompletionKind::Word),
            CompletionItem::new("unwrap_or_default", "Function", CompletionKind::Function),
        ];

        let ranked = AutocompleteState::filter_and_rank(&candidates, "str");
        assert!(!ranked.is_empty());
        // Exact prefix "str" or "struct" should be at top
        assert_eq!(ranked[0].label, "str");
        assert!(ranked.iter().any(|item| item.label == "struct"));
        assert!(ranked.iter().any(|item| item.label == "String"));
        assert!(ranked.iter().any(|item| item.label == "stream_read"));
    }

    #[test]
    fn test_autocomplete_navigation_and_scroll() {
        let mut state = AutocompleteState::new();
        state.max_visible = 3;

        let items = vec![
            CompletionItem::new("a", "det", CompletionKind::Word),
            CompletionItem::new("b", "det", CompletionKind::Word),
            CompletionItem::new("c", "det", CompletionKind::Word),
            CompletionItem::new("d", "det", CompletionKind::Word),
            CompletionItem::new("e", "det", CompletionKind::Word),
        ];

        state.open("a".to_string(), 0, items);
        assert_eq!(state.selected_idx, 0);
        assert_eq!(state.scroll_offset, 0);

        state.select_next();
        assert_eq!(state.selected_idx, 1);
        assert_eq!(state.scroll_offset, 0);

        state.select_next();
        assert_eq!(state.selected_idx, 2);
        assert_eq!(state.scroll_offset, 0);

        state.select_next();
        assert_eq!(state.selected_idx, 3);
        assert_eq!(state.scroll_offset, 1);

        // Wrap around backward
        state.select_prev();
        state.select_prev();
        state.select_prev();
        state.select_prev();
        assert_eq!(state.selected_idx, 4);
        assert_eq!(state.scroll_offset, 2);
    }
}
