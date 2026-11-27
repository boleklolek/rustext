//! JSON-RPC 2.0 and structured IPC protocol for Rustext plugins.
//! Zero third-party dependencies, purely standard library Rust.

use std::collections::HashMap;
use std::path::PathBuf;
use crate::core::json::JsonValue;

/// Structured request sent from Rustext to an external extension.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionIpcRequest {
    pub id: u64,
    pub method: String,
    pub command: String,
    pub file_path: Option<PathBuf>,
    pub buffer_text: String,
    pub selection_text: Option<String>,
    pub line_count: usize,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub language: String,
    pub params: HashMap<String, String>,
}

impl ExtensionIpcRequest {
    /// Serializes this request into standard JSON-RPC 2.0 payload format.
    pub fn to_json_string(&self) -> String {
        let mut root_obj = Vec::new();
        root_obj.push(("jsonrpc".to_string(), JsonValue::String("2.0".to_string())));
        root_obj.push(("id".to_string(), JsonValue::Number(self.id as f64)));
        root_obj.push(("method".to_string(), JsonValue::String(self.method.clone())));

        let mut params_obj = Vec::new();
        params_obj.push(("command".to_string(), JsonValue::String(self.command.clone())));
        if let Some(ref p) = self.file_path {
            params_obj.push(("file_path".to_string(), JsonValue::String(p.to_string_lossy().to_string())));
        } else {
            params_obj.push(("file_path".to_string(), JsonValue::Null));
        }

        params_obj.push(("buffer_text".to_string(), JsonValue::String(self.buffer_text.clone())));
        if let Some(ref s) = self.selection_text {
            params_obj.push(("selection_text".to_string(), JsonValue::String(s.clone())));
        } else {
            params_obj.push(("selection_text".to_string(), JsonValue::Null));
        }

        params_obj.push(("line_count".to_string(), JsonValue::Number(self.line_count as f64)));
        params_obj.push(("cursor_line".to_string(), JsonValue::Number(self.cursor_line as f64)));
        params_obj.push(("cursor_col".to_string(), JsonValue::Number(self.cursor_col as f64)));
        params_obj.push(("language".to_string(), JsonValue::String(self.language.clone())));

        let mut custom_params = Vec::new();
        for (k, v) in &self.params {
            custom_params.push((k.clone(), JsonValue::String(v.clone())));
        }
        params_obj.push(("custom".to_string(), JsonValue::Object(custom_params)));

        root_obj.push(("params".to_string(), JsonValue::Object(params_obj)));

        JsonValue::Object(root_obj).to_json_string()
    }
}

/// An action or modification returned by a plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionAction {
    /// Replace the selected text (or full buffer) with new text.
    ReplaceText(String),
    /// Replace the entire buffer content unconditionally.
    ReplaceBuffer(String),
    /// Set a status message in the editor bottom bar.
    SetStatus(String),
    /// Open a new bottom/side informational panel with lines of content.
    OpenPanel {
        title: String,
        lines: Vec<String>,
    },
    /// Append text to terminal output or clipboard.
    SetClipboard(String),
    /// No change to editor buffer.
    None,
}

/// Structured response received from an extension.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionIpcResponse {
    pub id: u64,
    pub actions: Vec<ExtensionAction>,
    pub status: Option<String>,
    pub error: Option<String>,
}

impl ExtensionIpcResponse {
    /// Parses an IPC response from raw process output (supporting either JSON-RPC or plain text fallback).
    pub fn parse(raw_output: &str) -> Self {
        let trimmed = raw_output.trim();

        // 1. Try parsing as JSON / JSON-RPC
        if (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        {
            if let Ok(json) = JsonValue::parse(trimmed) {
                if let Some(resp) = Self::from_json_value(&json) {
                    return resp;
                }
            }
        }

        // 2. Fallback: treat raw output directly as ReplaceText transformation
        Self {
            id: 0,
            actions: vec![ExtensionAction::ReplaceText(raw_output.to_string())],
            status: None,
            error: None,
        }
    }

    fn from_json_value(json: &JsonValue) -> Option<Self> {
        let obj = match json {
            JsonValue::Object(entries) => entries,
            _ => return None,
        };

        let mut id = 0u64;
        let mut status = None;
        let mut error = None;
        let mut actions = Vec::new();

        for (k, v) in obj {
            match k.as_str() {
                "id" => {
                    if let Some(n) = v.as_f64() {
                        id = n as u64;
                    }
                }
                "error" => {
                    if let Some(s) = v.as_str() {
                        error = Some(s.to_string());
                    } else if let Some(err_obj) = v.as_object() {
                        if let Some(msg) = err_obj.iter().find(|(k, _)| k == "message").and_then(|(_, v)| v.as_str()) {
                            error = Some(msg.to_string());
                        }
                    }
                }
                "result" => {
                    Self::parse_result_into_actions(v, &mut actions, &mut status);
                }
                "actions" => {
                    Self::parse_actions_array(v, &mut actions);
                }
                "status" => {
                    if let Some(s) = v.as_str() {
                        status = Some(s.to_string());
                    }
                }
                _ => {}
            }
        }

        Some(Self {
            id,
            actions,
            status,
            error,
        })
    }

    fn parse_result_into_actions(result: &JsonValue, actions: &mut Vec<ExtensionAction>, status: &mut Option<String>) {
        match result {
            JsonValue::String(s) => {
                actions.push(ExtensionAction::ReplaceText(s.clone()));
            }
            JsonValue::Object(entries) => {
                for (k, v) in entries {
                    match k.as_str() {
                        "text" | "replace_text" => {
                            if let Some(s) = v.as_str() {
                                actions.push(ExtensionAction::ReplaceText(s.to_string()));
                            }
                        }
                        "replace_buffer" => {
                            if let Some(s) = v.as_str() {
                                actions.push(ExtensionAction::ReplaceBuffer(s.to_string()));
                            }
                        }
                        "status" => {
                            if let Some(s) = v.as_str() {
                                *status = Some(s.to_string());
                            }
                        }
                        "clipboard" => {
                            if let Some(s) = v.as_str() {
                                actions.push(ExtensionAction::SetClipboard(s.to_string()));
                            }
                        }
                        "panel" => {
                            if let Some(panel_obj) = v.as_object() {
                                let mut title = "Panel de Extensión".to_string();
                                let mut lines = Vec::new();
                                for (pk, pv) in panel_obj {
                                    if pk == "title" {
                                        if let Some(t) = pv.as_str() {
                                            title = t.to_string();
                                        }
                                    } else if pk == "lines" {
                                        if let Some(arr) = pv.as_array() {
                                            for item in arr {
                                                if let Some(line) = item.as_str() {
                                                    lines.push(line.to_string());
                                                }
                                            }
                                        }
                                    } else if pk == "content" {
                                        if let Some(c) = pv.as_str() {
                                            for l in c.lines() {
                                                lines.push(l.to_string());
                                            }
                                        }
                                    }
                                }
                                actions.push(ExtensionAction::OpenPanel { title, lines });
                            }
                        }
                        _ => {}
                    }
                }
            }
            JsonValue::Array(items) => {
                for item in items {
                    Self::parse_result_into_actions(item, actions, status);
                }
            }
            _ => {}
        }
    }

    fn parse_actions_array(actions_val: &JsonValue, actions: &mut Vec<ExtensionAction>) {
        if let Some(arr) = actions_val.as_array() {
            for item in arr {
                if let Some(obj) = item.as_object() {
                    let mut action_type = "";
                    let mut content = "";
                    let mut title = "Panel de Extensión";
                    let mut lines_vec = Vec::new();

                    for (k, v) in obj {
                        match k.as_str() {
                            "type" | "action" => {
                                if let Some(s) = v.as_str() {
                                    action_type = s;
                                }
                            }
                            "text" | "content" => {
                                if let Some(s) = v.as_str() {
                                    content = s;
                                }
                            }
                            "title" => {
                                if let Some(s) = v.as_str() {
                                    title = s;
                                }
                            }
                            "lines" => {
                                if let Some(larr) = v.as_array() {
                                    for l in larr {
                                        if let Some(ls) = l.as_str() {
                                            lines_vec.push(ls.to_string());
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    match action_type {
                        "replace_text" | "replace" => {
                            actions.push(ExtensionAction::ReplaceText(content.to_string()));
                        }
                        "replace_buffer" => {
                            actions.push(ExtensionAction::ReplaceBuffer(content.to_string()));
                        }
                        "set_status" | "status" => {
                            actions.push(ExtensionAction::SetStatus(content.to_string()));
                        }
                        "panel" | "open_panel" => {
                            if lines_vec.is_empty() && !content.is_empty() {
                                lines_vec = content.lines().map(|s| s.to_string()).collect();
                            }
                            actions.push(ExtensionAction::OpenPanel {
                                title: title.to_string(),
                                lines: lines_vec,
                            });
                        }
                        "clipboard" => {
                            actions.push(ExtensionAction::SetClipboard(content.to_string()));
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
