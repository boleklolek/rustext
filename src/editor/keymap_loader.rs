//! Declarative Keymap parser, custom shortcut profiles, and modal keybinding engine (Vim, Emacs, Default).
//! Zero third-party dependencies, purely standard library Rust.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use crate::editor::keymap::EditorAction;
use crate::ui::terminal::Key;

/// Maximum size allowed for a .keymap file (512 KB) to prevent DoS (CWE-400).
pub const MAX_KEYMAP_FILE_SIZE: u64 = 512 * 1024;

/// Modal editing state for modal profiles (such as Vim).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModalMode {
    #[default]
    Normal,
    Insert,
    Visual,
}

impl ModalMode {
    pub fn badge_name(&self) -> &'static str {
        match self {
            ModalMode::Normal => "NORMAL",
            ModalMode::Insert => "INSERT",
            ModalMode::Visual => "VISUAL",
        }
    }

    pub fn is_insert(&self) -> bool {
        matches!(self, ModalMode::Insert)
    }
}

/// Metadata header for a keymap configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapMeta {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub is_modal: bool,
}

impl Default for KeymapMeta {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "Modern Standard".to_string(),
            author: "Rustext Team".to_string(),
            version: "1.0".to_string(),
            is_modal: false,
        }
    }
}

/// A parsed key chord, either single or two-step sequence (e.g. `Ctrl+x Ctrl+s` or `g g`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyChord {
    Single(Key),
    Sequence(Key, Key),
}

/// A custom keymap definition loaded from a .keymap file or built-in profile.
#[derive(Debug, Clone, PartialEq)]
pub struct KeymapProfile {
    pub meta: KeymapMeta,
    pub bindings: BTreeMap<KeyChord, EditorAction>,
    pub normal_bindings: BTreeMap<KeyChord, EditorAction>,
}

impl KeymapProfile {
    pub fn new(meta: KeymapMeta) -> Self {
        Self {
            meta,
            bindings: BTreeMap::new(),
            normal_bindings: BTreeMap::new(),
        }
    }
}

/// Runtime engine that handles stateful key resolution (prefix sequences and modal states).
#[derive(Debug, Clone)]
pub struct KeymapEngine {
    pub profile: KeymapProfile,
    pub modal_mode: ModalMode,
    pub pending_chord: Option<Key>,
}

impl KeymapEngine {
    pub fn new(profile: KeymapProfile) -> Self {
        let is_modal = profile.meta.is_modal;
        Self {
            profile,
            modal_mode: if is_modal { ModalMode::Normal } else { ModalMode::Insert },
            pending_chord: None,
        }
    }

    /// Resets any pending two-step prefix chord.
    pub fn reset_pending(&mut self) {
        self.pending_chord = None;
    }

    /// Switches modal mode (e.g. Normal -> Insert or Insert -> Normal).
    pub fn set_modal_mode(&mut self, mode: ModalMode) {
        self.modal_mode = mode;
        self.pending_chord = None;
    }

    /// Replaces the active keymap profile and resets engine state.
    pub fn set_profile(&mut self, profile: KeymapProfile) {
        let is_modal = profile.meta.is_modal;
        self.profile = profile;
        self.modal_mode = if is_modal { ModalMode::Normal } else { ModalMode::Insert };
        self.pending_chord = None;
    }

    /// Resolves an incoming key event.
    /// Returns `Some(action)` if a command is resolved, or `None` if waiting for sequence suffix or unhandled.
    pub fn resolve_key(&mut self, key: Key) -> Option<EditorAction> {
        // 1. If we have a pending prefix chord, check two-step sequence first
        if let Some(prefix) = self.pending_chord.take() {
            let chord = KeyChord::Sequence(prefix.clone(), key.clone());
            if self.profile.meta.is_modal && self.modal_mode == ModalMode::Normal {
                if let Some(act) = self.profile.normal_bindings.get(&chord) {
                    return Some(act.clone());
                }
            }
            if let Some(act) = self.profile.bindings.get(&chord) {
                return Some(act.clone());
            }
            // Sequence not found, continue to process `key` as standalone below
        }

        // 2. Check if `key` matches any sequence prefix
        let is_prefix = if self.profile.meta.is_modal && self.modal_mode == ModalMode::Normal {
            self.profile.normal_bindings.keys().any(|k| matches!(k, KeyChord::Sequence(p, _) if p == &key))
        } else {
            self.profile.bindings.keys().any(|k| matches!(k, KeyChord::Sequence(p, _) if p == &key))
        };

        if is_prefix {
            self.pending_chord = Some(key);
            return None; // Wait for suffix key
        }

        // 3. Resolve single chord
        let chord = KeyChord::Single(key.clone());

        // In modal mode Normal
        if self.profile.meta.is_modal {
            match self.modal_mode {
                ModalMode::Normal => {
                    // Check mode switches
                    if let Key::Char('i') = key {
                        self.set_modal_mode(ModalMode::Insert);
                        return Some(EditorAction::None);
                    }
                    if let Key::Char('a') = key {
                        self.set_modal_mode(ModalMode::Insert);
                        return Some(EditorAction::MoveRight { select: false });
                    }
                    if let Key::Char('v') = key {
                        self.set_modal_mode(ModalMode::Visual);
                        return Some(EditorAction::None);
                    }
                    if let Key::Esc = key {
                        return Some(EditorAction::CollapseCursors);
                    }

                    if let Some(act) = self.profile.normal_bindings.get(&chord) {
                        return Some(act.clone());
                    }
                }
                ModalMode::Visual => {
                    if let Key::Esc = key {
                        self.set_modal_mode(ModalMode::Normal);
                        return Some(EditorAction::CollapseCursors);
                    }
                    // Visual motions
                    if let Some(act) = self.profile.normal_bindings.get(&chord) {
                        // Transform movements to selection
                        let visual_act = match act {
                            EditorAction::MoveLeft { .. } => EditorAction::MoveLeft { select: true },
                            EditorAction::MoveRight { .. } => EditorAction::MoveRight { select: true },
                            EditorAction::MoveUp { .. } => EditorAction::MoveUp { select: true },
                            EditorAction::MoveDown { .. } => EditorAction::MoveDown { select: true },
                            EditorAction::MoveLineStart { .. } => EditorAction::MoveLineStart { select: true },
                            EditorAction::MoveLineEnd { .. } => EditorAction::MoveLineEnd { select: true },
                            other => other.clone(),
                        };
                        return Some(visual_act);
                    }
                }
                ModalMode::Insert => {
                    if let Key::Esc = key {
                        self.set_modal_mode(ModalMode::Normal);
                        return Some(EditorAction::MoveLeft { select: false });
                    }
                }
            }
        }

        // Check general profile bindings
        if let Some(act) = self.profile.bindings.get(&chord) {
            return Some(act.clone());
        }

        // Fallback to standard resolve_key if not handled by profile
        Some(crate::editor::keymap::Keymap::resolve_key(key))
    }
}

/// Parses key chord string representations (e.g. `Ctrl+s`, `Alt+Up`, `Ctrl+x Ctrl+s`, `g g`).
pub fn parse_key_chord(s: &str) -> Option<KeyChord> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() == 1 {
        parse_single_key(parts[0]).map(KeyChord::Single)
    } else if parts.len() == 2 {
        let k1 = parse_single_key(parts[0])?;
        let k2 = parse_single_key(parts[1])?;
        Some(KeyChord::Sequence(k1, k2))
    } else {
        None
    }
}

/// Parses a single key representation like `Ctrl+s`, `Alt+Down`, `Enter`, `F12`, or `h`.
pub fn parse_single_key(s: &str) -> Option<Key> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Check special single characters
    if trimmed.chars().count() == 1 {
        let c = trimmed.chars().next().unwrap();
        return Some(Key::Char(c));
    }

    let lower = trimmed.to_lowercase();
    match lower.as_str() {
        "enter" | "return" => return Some(Key::Enter),
        "backspace" => return Some(Key::Backspace),
        "tab" => return Some(Key::Tab),
        "backtab" => return Some(Key::BackTab),
        "esc" | "escape" => return Some(Key::Esc),
        "space" => return Some(Key::Char(' ')),
        "up" => return Some(Key::Up),
        "down" => return Some(Key::Down),
        "left" => return Some(Key::Left),
        "right" => return Some(Key::Right),
        "home" => return Some(Key::Home),
        "end" => return Some(Key::End),
        "pageup" | "pgup" => return Some(Key::PageUp),
        "pagedown" | "pgdn" => return Some(Key::PageDown),
        "delete" | "del" => return Some(Key::Delete),
        _ => {}
    }

    // Function keys F1..=F12
    if lower.starts_with('f') && lower.len() >= 2 {
        if let Ok(num) = lower[1..].parse::<u8>() {
            if (1..=12).contains(&num) {
                return Some(Key::F(num));
            }
        }
    }

    // Combinations with '+' (e.g. Ctrl+Shift+P, Ctrl+s, Alt+Up)
    let subkeys: Vec<&str> = trimmed.split('+').map(|p| p.trim()).collect();
    if subkeys.len() >= 2 {
        let mut is_ctrl = false;
        let mut is_alt = false;
        let mut is_shift = false;
        let mut primary_str = "";

        for part in subkeys {
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => is_ctrl = true,
                "alt" => is_alt = true,
                "shift" => is_shift = true,
                _ => primary_str = part,
            }
        }

        if is_ctrl && is_shift && primary_str.chars().count() == 1 {
            let c = primary_str.chars().next().unwrap();
            return Some(Key::CtrlShift(c));
        }

        if is_ctrl && !is_alt && !is_shift {
            if primary_str.chars().count() == 1 {
                let c = primary_str.chars().next().unwrap().to_ascii_lowercase();
                return Some(Key::Ctrl(c));
            }
            if primary_str.eq_ignore_ascii_case("space") {
                return Some(Key::Ctrl(' '));
            }
        }

        if is_alt && !is_ctrl && !is_shift {
            match primary_str.to_lowercase().as_str() {
                "up" => return Some(Key::AltUp),
                "down" => return Some(Key::AltDown),
                "left" => return Some(Key::AltLeft),
                "right" => return Some(Key::AltRight),
                _ => {
                    if primary_str.chars().count() == 1 {
                        let c = primary_str.chars().next().unwrap();
                        return Some(Key::Alt(c));
                    }
                }
            }
        }

        if is_shift && is_alt {
            match primary_str.to_lowercase().as_str() {
                "up" => return Some(Key::ShiftAltUp),
                "down" => return Some(Key::ShiftAltDown),
                _ => {}
            }
        }
    }

    None
}

/// Parses an action identifier string into an `EditorAction`.
pub fn parse_action_name(name: &str) -> Option<EditorAction> {
    let lower = name.trim().to_lowercase().replace('-', "_");
    match lower.as_str() {
        "save" => Some(EditorAction::Save),
        "quit" => Some(EditorAction::Quit),
        "undo" => Some(EditorAction::Undo),
        "redo" => Some(EditorAction::Redo),
        "cut" => Some(EditorAction::Cut),
        "copy" => Some(EditorAction::Copy),
        "paste" => Some(EditorAction::Paste),
        "delete" => Some(EditorAction::Delete),
        "backspace" => Some(EditorAction::Backspace),
        "new_tab" => Some(EditorAction::NewTab),
        "close_tab" => Some(EditorAction::CloseTab),
        "next_tab" => Some(EditorAction::NextTab),
        "prev_tab" => Some(EditorAction::PrevTab),
        "open_file" => Some(EditorAction::OpenOpenFilePrompt),
        "open_folder" => Some(EditorAction::OpenFolderPrompt),
        "open_recent_files" => Some(EditorAction::OpenRecentFilesPrompt),
        "file_finder" => Some(EditorAction::OpenFileFinderPrompt),
        "command_palette" => Some(EditorAction::OpenCommandPalette),
        "search" => Some(EditorAction::OpenSearchPrompt),
        "replace" => Some(EditorAction::OpenReplacePrompt),
        "global_search" => Some(EditorAction::OpenGlobalSearchPrompt),
        "toggle_minimap" => Some(EditorAction::ToggleMinimap),
        "toggle_sidebar" => Some(EditorAction::ToggleSidebar),
        "toggle_terminal" => Some(EditorAction::ToggleTerminal),
        "toggle_split_vertical" => Some(EditorAction::ToggleSplitVertical),
        "toggle_split_horizontal" => Some(EditorAction::ToggleSplitHorizontal),
        "switch_split_pane" => Some(EditorAction::SwitchSplitPane),
        "close_split" => Some(EditorAction::CloseSplit),
        "move_left" => Some(EditorAction::MoveLeft { select: false }),
        "move_right" => Some(EditorAction::MoveRight { select: false }),
        "move_up" => Some(EditorAction::MoveUp { select: false }),
        "move_down" => Some(EditorAction::MoveDown { select: false }),
        "move_line_start" => Some(EditorAction::MoveLineStart { select: false }),
        "move_line_end" => Some(EditorAction::MoveLineEnd { select: false }),
        "move_line_up" => Some(EditorAction::MoveLineUp),
        "move_line_down" => Some(EditorAction::MoveLineDown),
        "duplicate_line_down" => Some(EditorAction::DuplicateLineDown),
        "select_next_occurrence" => Some(EditorAction::SelectNextOccurrence),
        "add_cursor_above" => Some(EditorAction::AddCursorAbove),
        "add_cursor_below" => Some(EditorAction::AddCursorBelow),
        "collapse_cursors" | "cancel" => Some(EditorAction::CollapseCursors),
        "toggle_comment" => Some(EditorAction::ToggleComment),
        "toggle_word_wrap" => Some(EditorAction::ToggleWordWrap),
        "toggle_indent_guides" => Some(EditorAction::ToggleIndentGuides),
        "toggle_fold" => Some(EditorAction::ToggleFold),
        "fold_all" => Some(EditorAction::FoldAll),
        "unfold_all" => Some(EditorAction::UnfoldAll),
        "jump_back" => Some(EditorAction::JumpBack),
        "jump_forward" => Some(EditorAction::JumpForward),
        "format_document" => Some(EditorAction::FormatDocument),
        "check_document" => Some(EditorAction::CheckDocument),
        "lsp_definition" => Some(EditorAction::LspGoToDefinition),
        "lsp_references" => Some(EditorAction::LspFindReferences),
        "lsp_hover" => Some(EditorAction::LspHover),
        "lsp_rename" => Some(EditorAction::LspRename),
        "lsp_code_actions" => Some(EditorAction::LspCodeActions),
        "trigger_autocomplete" => Some(EditorAction::TriggerAutocomplete),
        _ => None,
    }
}

/// Parses .keymap file format string into a `KeymapProfile`.
pub fn parse_keymap_str(content: &str) -> KeymapProfile {
    let mut profile = KeymapProfile::new(KeymapMeta::default());
    let mut current_section = "";

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = &trimmed[1..trimmed.len() - 1];
            continue;
        }

        if let Some((raw_key, raw_val)) = trimmed.split_once('=') {
            let key = raw_key.trim();
            let val = raw_val.trim();

            match current_section {
                "meta" => match key {
                    "id" => profile.meta.id = val.to_string(),
                    "name" => profile.meta.name = val.to_string(),
                    "author" => profile.meta.author = val.to_string(),
                    "version" => profile.meta.version = val.to_string(),
                    "modal" => profile.meta.is_modal = val.eq_ignore_ascii_case("true") || val == "1",
                    _ => {}
                },
                "bindings" => {
                    if let Some(chord) = parse_key_chord(key) {
                        if let Some(act) = parse_action_name(val) {
                            profile.bindings.insert(chord, act);
                        }
                    }
                }
                "normal" => {
                    if let Some(chord) = parse_key_chord(key) {
                        if let Some(act) = parse_action_name(val) {
                            profile.normal_bindings.insert(chord, act);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    profile
}

/// Validates that a keymap name contains only safe alphanumeric, dash, or underscore characters (max 64 bytes)
/// to defend against Path Traversal attacks (CWE-22).
pub fn is_safe_keymap_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Loads a keymap by name, searching user directory (`~/.rustext/keymaps/`) and official built-ins.
pub fn load_keymap_profile(name: &str) -> KeymapProfile {
    let trimmed = name.to_lowercase().trim().to_string();

    // 1. Check user keymaps directory
    if is_safe_keymap_name(&trimmed) {
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            let user_path = PathBuf::from(home)
                .join(".rustext")
                .join("keymaps")
                .join(format!("{}.keymap", trimmed));
            if user_path.is_file() {
                if let Ok(meta) = fs::metadata(&user_path) {
                    if meta.len() <= MAX_KEYMAP_FILE_SIZE {
                        if let Ok(content) = fs::read_to_string(&user_path) {
                            return parse_keymap_str(&content);
                        }
                    }
                }
            }
        }
    }

    // 2. Built-in templates
    match trimmed.as_str() {
        "vim" => parse_keymap_str(include_str!("../../assets/keymaps/vim.keymap")),
        "emacs" => parse_keymap_str(include_str!("../../assets/keymaps/emacs.keymap")),
        _ => parse_keymap_str(include_str!("../../assets/keymaps/default.keymap")),
    }
}

/// Returns a list of all available keymap profiles (built-in and user created).
pub fn available_keymaps() -> Vec<KeymapMeta> {
    let mut list = vec![
        KeymapMeta {
            id: "default".into(),
            name: "Modern Standard (Sublime/VS Code)".into(),
            author: "Rustext Team".into(),
            version: "1.0".into(),
            is_modal: false,
        },
        KeymapMeta {
            id: "vim".into(),
            name: "Vim Modal".into(),
            author: "Rustext Team".into(),
            version: "1.0".into(),
            is_modal: true,
        },
        KeymapMeta {
            id: "emacs".into(),
            name: "Emacs Classic".into(),
            author: "Rustext Team".into(),
            version: "1.0".into(),
            is_modal: false,
        },
    ];

    // Discover user keymaps in ~/.rustext/keymaps/
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let dir = PathBuf::from(home).join(".rustext").join("keymaps");
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map(|e| e == "keymap").unwrap_or(false) {
                    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                        let id = stem.to_string();
                        if !list.iter().any(|k| k.id == id) && is_safe_keymap_name(&id) {
                            if let Ok(content) = fs::read_to_string(&p) {
                                let profile = parse_keymap_str(&content);
                                list.push(profile.meta);
                            }
                        }
                    }
                }
            }
        }
    }

    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_and_sequence_chords() {
        assert_eq!(parse_key_chord("Ctrl+s"), Some(KeyChord::Single(Key::Ctrl('s'))));
        assert_eq!(parse_key_chord("Alt+Up"), Some(KeyChord::Single(Key::AltUp)));
        assert_eq!(
            parse_key_chord("Ctrl+x Ctrl+s"),
            Some(KeyChord::Sequence(Key::Ctrl('x'), Key::Ctrl('s')))
        );
        assert_eq!(
            parse_key_chord("g g"),
            Some(KeyChord::Sequence(Key::Char('g'), Key::Char('g')))
        );
    }

    #[test]
    fn test_builtin_keymaps_parse() {
        let default_km = load_keymap_profile("default");
        assert_eq!(default_km.meta.id, "default");
        assert!(!default_km.bindings.is_empty());

        let vim_km = load_keymap_profile("vim");
        assert_eq!(vim_km.meta.id, "vim");
        assert!(vim_km.meta.is_modal);

        let emacs_km = load_keymap_profile("emacs");
        assert_eq!(emacs_km.meta.id, "emacs");
        assert!(!emacs_km.bindings.is_empty());
    }

    #[test]
    fn test_keymap_engine_emacs_sequence() {
        let emacs_km = load_keymap_profile("emacs");
        let mut engine = KeymapEngine::new(emacs_km);

        // Sending Ctrl+x should yield None (pending sequence)
        let r1 = engine.resolve_key(Key::Ctrl('x'));
        assert_eq!(r1, None);
        assert_eq!(engine.pending_chord, Some(Key::Ctrl('x')));

        // Sending Ctrl+s should resolve to Save
        let r2 = engine.resolve_key(Key::Ctrl('s'));
        assert_eq!(r2, Some(EditorAction::Save));
        assert_eq!(engine.pending_chord, None);
    }

    #[test]
    fn test_keymap_engine_vim_modes() {
        let vim_km = load_keymap_profile("vim");
        let mut engine = KeymapEngine::new(vim_km);
        assert_eq!(engine.modal_mode, ModalMode::Normal);

        // 'h' in Normal moves left
        let act = engine.resolve_key(Key::Char('h'));
        assert_eq!(act, Some(EditorAction::MoveLeft { select: false }));

        // 'i' enters insert mode
        let _ = engine.resolve_key(Key::Char('i'));
        assert_eq!(engine.modal_mode, ModalMode::Insert);

        // Esc returns to Normal mode
        let _ = engine.resolve_key(Key::Esc);
        assert_eq!(engine.modal_mode, ModalMode::Normal);
    }
}
