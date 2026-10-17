//! Language Server Protocol (LSP) Client for real-time diagnostics, completion, and navigation.
//! Pure standard-library Rust with zero third-party dependencies (`std::*`).
//!
//! Implements JSON-RPC 2.0 framing and protocol handling over standard I/O (stdin/stdout)
//! for language servers like `rust-analyzer`, `pylsp`, and `clangd`.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::Arc;
use std::thread;

use crate::core::syntax::Language;
use crate::editor::autocomplete::CompletionKind;
use crate::editor::diagnostic::{Diagnostic, DiagnosticSeverity};

// ============================================================================
// Lightweight Pure-Rust JSON AST & Parser (Zero Dependencies)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn get_path(&self, keys: &[&str]) -> Option<&JsonValue> {
        let mut curr = self;
        for &k in keys {
            curr = curr.get(k)?;
        }
        Some(curr)
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            JsonValue::Number(n) => Some(*n as i64),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            JsonValue::Number(n) if *n >= 0.0 => Some(*n as u64),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            JsonValue::Array(arr) => Some(arr.as_slice()),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, JsonValue)]> {
        match self {
            JsonValue::Object(entries) => Some(entries.as_slice()),
            _ => None,
        }
    }

    pub fn to_json_string(&self) -> String {
        let mut out = String::new();
        self.serialize_into(&mut out);
        out
    }

    fn serialize_into(&self, out: &mut String) {
        match self {
            JsonValue::Null => out.push_str("null"),
            JsonValue::Bool(true) => out.push_str("true"),
            JsonValue::Bool(false) => out.push_str("false"),
            JsonValue::Number(n) => {
                if n.fract() == 0.0 && !n.is_nan() && !n.is_infinite() {
                    out.push_str(&format!("{:.0}", n));
                } else {
                    out.push_str(&n.to_string());
                }
            }
            JsonValue::String(s) => {
                out.push('"');
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        other => {
                            if (other as u32) < 0x20 {
                                out.push_str(&format!("\\u{:04x}", other as u32));
                            } else {
                                out.push(other);
                            }
                        }
                    }
                }
                out.push('"');
            }
            JsonValue::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.serialize_into(out);
                }
                out.push(']');
            }
            JsonValue::Object(entries) => {
                out.push('{');
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push('"');
                    out.push_str(k);
                    out.push_str("\":");
                    v.serialize_into(out);
                }
                out.push('}');
            }
        }
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        let chars: Vec<char> = input.chars().collect();
        let mut idx = 0;
        skip_ws(&chars, &mut idx);
        let val = parse_value(&chars, &mut idx)?;
        skip_ws(&chars, &mut idx);
        if idx < chars.len() {
            return Err(format!("Trailing characters after JSON value at index {}", idx));
        }
        Ok(val)
    }
}

fn skip_ws(chars: &[char], idx: &mut usize) {
    while *idx < chars.len() && chars[*idx].is_whitespace() {
        *idx += 1;
    }
}

fn parse_value(chars: &[char], idx: &mut usize) -> Result<JsonValue, String> {
    skip_ws(chars, idx);
    if *idx >= chars.len() {
        return Err("Unexpected end of JSON input".to_string());
    }

    match chars[*idx] {
        'n' => parse_literal(chars, idx, "null", JsonValue::Null),
        't' => parse_literal(chars, idx, "true", JsonValue::Bool(true)),
        'f' => parse_literal(chars, idx, "false", JsonValue::Bool(false)),
        '"' => parse_string(chars, idx).map(JsonValue::String),
        '[' => parse_array(chars, idx),
        '{' => parse_object(chars, idx),
        '-' | '0'..='9' => parse_number(chars, idx),
        other => Err(format!("Unexpected character '{}' at position {}", other, *idx)),
    }
}

fn parse_literal(chars: &[char], idx: &mut usize, expected: &str, result: JsonValue) -> Result<JsonValue, String> {
    for expected_c in expected.chars() {
        if *idx >= chars.len() || chars[*idx] != expected_c {
            return Err(format!("Expected '{}' at index {}", expected, *idx));
        }
        *idx += 1;
    }
    Ok(result)
}

fn parse_string(chars: &[char], idx: &mut usize) -> Result<String, String> {
    if *idx >= chars.len() || chars[*idx] != '"' {
        return Err("Expected string opening quote".to_string());
    }
    *idx += 1;
    let mut out = String::new();

    while *idx < chars.len() {
        let c = chars[*idx];
        *idx += 1;
        match c {
            '"' => return Ok(out),
            '\\' => {
                if *idx >= chars.len() {
                    return Err("Unterminated escape sequence in string".to_string());
                }
                let esc = chars[*idx];
                *idx += 1;
                match esc {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'b' => out.push('\u{0008}'),
                    'f' => out.push('\u{000C}'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'u' => {
                        if *idx + 4 > chars.len() {
                            return Err("Incomplete \\u hex escape in string".to_string());
                        }
                        let hex_str: String = chars[*idx..*idx + 4].iter().collect();
                        *idx += 4;
                        if let Ok(code) = u32::from_str_radix(&hex_str, 16) {
                            if let Some(ch) = char::from_u32(code) {
                                out.push(ch);
                            }
                        }
                    }
                    other => out.push(other),
                }
            }
            other => out.push(other),
        }
    }
    Err("Unterminated string literal".to_string())
}

fn parse_number(chars: &[char], idx: &mut usize) -> Result<JsonValue, String> {
    let start = *idx;
    if *idx < chars.len() && chars[*idx] == '-' {
        *idx += 1;
    }
    while *idx < chars.len() && chars[*idx].is_ascii_digit() {
        *idx += 1;
    }
    if *idx < chars.len() && chars[*idx] == '.' {
        *idx += 1;
        while *idx < chars.len() && chars[*idx].is_ascii_digit() {
            *idx += 1;
        }
    }
    if *idx < chars.len() && (chars[*idx] == 'e' || chars[*idx] == 'E') {
        *idx += 1;
        if *idx < chars.len() && (chars[*idx] == '+' || chars[*idx] == '-') {
            *idx += 1;
        }
        while *idx < chars.len() && chars[*idx].is_ascii_digit() {
            *idx += 1;
        }
    }

    let slice: String = chars[start..*idx].iter().collect();
    match slice.parse::<f64>() {
        Ok(n) => Ok(JsonValue::Number(n)),
        Err(e) => Err(format!("Invalid number '{}': {}", slice, e)),
    }
}

fn parse_array(chars: &[char], idx: &mut usize) -> Result<JsonValue, String> {
    if *idx >= chars.len() || chars[*idx] != '[' {
        return Err("Expected array opening bracket".to_string());
    }
    *idx += 1;
    skip_ws(chars, idx);

    let mut items = Vec::new();
    if *idx < chars.len() && chars[*idx] == ']' {
        *idx += 1;
        return Ok(JsonValue::Array(items));
    }

    loop {
        let val = parse_value(chars, idx)?;
        items.push(val);
        skip_ws(chars, idx);

        if *idx >= chars.len() {
            return Err("Unterminated array".to_string());
        }

        match chars[*idx] {
            ',' => {
                *idx += 1;
                skip_ws(chars, idx);
            }
            ']' => {
                *idx += 1;
                return Ok(JsonValue::Array(items));
            }
            other => return Err(format!("Expected ',' or ']' in array, found '{}'", other)),
        }
    }
}

fn parse_object(chars: &[char], idx: &mut usize) -> Result<JsonValue, String> {
    if *idx >= chars.len() || chars[*idx] != '{' {
        return Err("Expected object opening brace".to_string());
    }
    *idx += 1;
    skip_ws(chars, idx);

    let mut entries = Vec::new();
    if *idx < chars.len() && chars[*idx] == '}' {
        *idx += 1;
        return Ok(JsonValue::Object(entries));
    }

    loop {
        skip_ws(chars, idx);
        let key = parse_string(chars, idx)?;
        skip_ws(chars, idx);

        if *idx >= chars.len() || chars[*idx] != ':' {
            return Err(format!("Expected ':' after key \"{}\"", key));
        }
        *idx += 1;

        let val = parse_value(chars, idx)?;
        entries.push((key, val));
        skip_ws(chars, idx);

        if *idx >= chars.len() {
            return Err("Unterminated object".to_string());
        }

        match chars[*idx] {
            ',' => {
                *idx += 1;
                skip_ws(chars, idx);
            }
            '}' => {
                *idx += 1;
                return Ok(JsonValue::Object(entries));
            }
            other => return Err(format!("Expected ',' or '}}' in object, found '{}'", other)),
        }
    }
}

// ============================================================================
// URI Encoding / Decoding Utilities
// ============================================================================

/// Converts a local filesystem path into an LSP `file://` URI.
pub fn path_to_uri(path: &Path) -> String {
    let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let path_str = abs_path.to_string_lossy();
    if cfg!(windows) {
        format!("file:///{}", path_str.replace('\\', "/"))
    } else {
        format!("file://{}", path_str)
    }
}

/// Converts an LSP `file://` URI into a local filesystem path.
pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
    if !uri.starts_with("file://") {
        return None;
    }
    let without_scheme = &uri[7..];
    let path_str = if cfg!(windows) && without_scheme.starts_with('/') {
        &without_scheme[1..]
    } else {
        without_scheme
    };

    // Percent decoding
    let mut decoded = String::with_capacity(path_str.len());
    let chars: Vec<char> = path_str.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' && i + 2 < chars.len() {
            let hex: String = chars[i + 1..=i + 2].iter().collect();
            if let Ok(b) = u8::from_str_radix(&hex, 16) {
                decoded.push(b as char);
                i += 3;
                continue;
            }
        }
        decoded.push(chars[i]);
        i += 1;
    }

    Some(PathBuf::from(decoded))
}

// ============================================================================
// LSP Data Models
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LspPosition {
    pub line: usize,      // 0-indexed in LSP
    pub character: usize, // 0-indexed in LSP
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspLocation {
    pub path: PathBuf,
    pub range: LspRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspHover {
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspParameterInformation {
    pub label: String,
    pub documentation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspSignatureInformation {
    pub label: String,
    pub documentation: String,
    pub parameters: Vec<LspParameterInformation>,
    pub active_parameter: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspSignatureHelp {
    pub signatures: Vec<LspSignatureInformation>,
    pub active_signature: usize,
    pub active_parameter: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspCompletionItem {
    pub label: String,
    pub detail: String,
    pub documentation: String,
    pub kind: CompletionKind,
    pub insert_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LspEvent {
    ServerStarted { server_name: String },
    Initialized,
    PublishDiagnostics { file: PathBuf, diagnostics: Vec<Diagnostic> },
    CompletionResponse { request_id: u64, items: Vec<LspCompletionItem> },
    HoverResponse { request_id: u64, hover: Option<LspHover> },
    SignatureHelpResponse { request_id: u64, signature_help: Option<LspSignatureHelp> },
    DefinitionResponse { request_id: u64, location: Option<LspLocation> },
    ServerExited { code: Option<i32> },
    Error { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerStatus {
    Disconnected,
    Connecting,
    Ready,
    Error,
}

// ============================================================================
// LSP Client Engine
// ============================================================================

pub struct LspClient {
    pub server_name: String,
    pub root_dir: PathBuf,
    pub status: ServerStatus,
    pub active_language: Language,
    next_id: Arc<AtomicU64>,
    stdin_tx: Option<Sender<String>>,
    event_rx: Option<Receiver<LspEvent>>,
    child: Option<Child>,
    open_docs: HashMap<PathBuf, u64>,
}

impl LspClient {
    pub fn new() -> Self {
        Self {
            server_name: "rust-analyzer".to_string(),
            root_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            status: ServerStatus::Disconnected,
            active_language: Language::Rust,
            next_id: Arc::new(AtomicU64::new(1)),
            stdin_tx: None,
            event_rx: None,
            child: None,
            open_docs: HashMap::new(),
        }
    }

    pub fn is_running(&self) -> bool {
        self.status == ServerStatus::Ready || self.status == ServerStatus::Connecting
    }

    /// Spawns the specified LSP server binary and initiates connection.
    pub fn start(&mut self, server_cmd: &str, root_dir: &Path, lang: Language) -> Result<(), String> {
        self.stop();

        self.server_name = server_cmd.to_string();
        self.root_dir = root_dir.to_path_buf();
        self.active_language = lang;
        self.status = ServerStatus::Connecting;

        let mut child = Command::new(server_cmd)
            .current_dir(root_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("No se pudo iniciar '{}': {}", server_cmd, e))?;

        let stdin = child.stdin.take().ok_or("Failed to open child stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open child stdout")?;

        let (stdin_tx, stdin_rx) = channel::<String>();
        let (event_tx, event_rx) = channel::<LspEvent>();

        // Background Writer Thread
        thread::spawn(move || {
            let mut writer = stdin;
            while let Ok(msg) = stdin_rx.recv() {
                if writer.write_all(msg.as_bytes()).is_err() || writer.flush().is_err() {
                    break;
                }
            }
        });

        // Background Reader Thread (parses Content-Length framing & JSON-RPC)
        let ev_tx_clone = event_tx.clone();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut header_buf = String::new();

            loop {
                header_buf.clear();
                let mut content_length: Option<usize> = None;

                // Read headers until empty line "\r\n"
                loop {
                    header_buf.clear();
                    if reader.read_line(&mut header_buf).unwrap_or(0) == 0 {
                        let _ = ev_tx_clone.send(LspEvent::ServerExited { code: None });
                        return;
                    }

                    let line = header_buf.trim();
                    if line.is_empty() {
                        break;
                    }

                    if line.to_lowercase().starts_with("content-length:") {
                        if let Some(val_str) = line.split(':').nth(1) {
                            content_length = val_str.trim().parse::<usize>().ok();
                        }
                    }
                }

                let length = match content_length {
                    Some(len) => len,
                    None => continue,
                };

                let mut body_buf = vec![0u8; length];
                if reader.read_exact(&mut body_buf).is_err() {
                    break;
                }

                if let Ok(body_str) = String::from_utf8(body_buf) {
                    if let Ok(json) = JsonValue::parse(&body_str) {
                        Self::dispatch_incoming_json(&json, &ev_tx_clone);
                    }
                }
            }
        });

        self.stdin_tx = Some(stdin_tx);
        self.event_rx = Some(event_rx);
        self.child = Some(child);

        let _ = event_tx.send(LspEvent::ServerStarted {
            server_name: self.server_name.clone(),
        });

        // Send LSP Initialize Request
        self.send_initialize(root_dir);

        Ok(())
    }

    /// Shuts down the LSP connection and terminates child process.
    pub fn stop(&mut self) {
        if self.is_running() {
            self.send_notification("exit", JsonValue::Object(Vec::new()));
        }

        self.status = ServerStatus::Disconnected;
        self.stdin_tx = None;
        self.event_rx = None;
        self.open_docs.clear();

        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// Non-blocking poll for incoming LSP events from background thread.
    pub fn poll_events(&mut self) -> Vec<LspEvent> {
        let mut events = Vec::new();
        if let Some(rx) = &self.event_rx {
            loop {
                match rx.try_recv() {
                    Ok(ev) => {
                        match &ev {
                            LspEvent::Initialized => {
                                self.status = ServerStatus::Ready;
                            }
                            LspEvent::ServerExited { .. } => {
                                self.status = ServerStatus::Disconnected;
                            }
                            LspEvent::Error { .. } => {
                                self.status = ServerStatus::Error;
                            }
                            _ => {}
                        }
                        events.push(ev);
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        self.status = ServerStatus::Disconnected;
                        break;
                    }
                }
            }
        }
        events
    }

    fn send_raw_json(&mut self, json_str: &str) {
        if let Some(tx) = &self.stdin_tx {
            let framed = format!("Content-Length: {}\r\n\r\n{}", json_str.len(), json_str);
            let _ = tx.send(framed);
        }
    }

    pub fn send_request(&mut self, method: &str, params: JsonValue) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonValue::Object(vec![
            ("jsonrpc".to_string(), JsonValue::String("2.0".to_string())),
            ("id".to_string(), JsonValue::Number(id as f64)),
            ("method".to_string(), JsonValue::String(method.to_string())),
            ("params".to_string(), params),
        ]);
        self.send_raw_json(&req.to_json_string());
        id
    }

    pub fn send_notification(&mut self, method: &str, params: JsonValue) {
        let notif = JsonValue::Object(vec![
            ("jsonrpc".to_string(), JsonValue::String("2.0".to_string())),
            ("method".to_string(), JsonValue::String(method.to_string())),
            ("params".to_string(), params),
        ]);
        self.send_raw_json(&notif.to_json_string());
    }

    fn send_initialize(&mut self, root_dir: &Path) {
        let root_uri = path_to_uri(root_dir);

        let capabilities = JsonValue::Object(vec![
            (
                "textDocument".to_string(),
                JsonValue::Object(vec![
                    (
                        "hover".to_string(),
                        JsonValue::Object(vec![(
                            "contentFormat".to_string(),
                            JsonValue::Array(vec![
                                JsonValue::String("markdown".to_string()),
                                JsonValue::String("plaintext".to_string()),
                            ]),
                        )]),
                    ),
                    (
                        "completion".to_string(),
                        JsonValue::Object(vec![(
                            "completionItem".to_string(),
                            JsonValue::Object(vec![(
                                "snippetSupport".to_string(),
                                JsonValue::Bool(false),
                            )]),
                        )]),
                    ),
                    (
                        "definition".to_string(),
                        JsonValue::Object(vec![(
                            "dynamicRegistration".to_string(),
                            JsonValue::Bool(false),
                        )]),
                    ),
                    (
                        "publishDiagnostics".to_string(),
                        JsonValue::Object(vec![(
                            "relatedInformation".to_string(),
                            JsonValue::Bool(false),
                        )]),
                    ),
                    (
                        "signatureHelp".to_string(),
                        JsonValue::Object(vec![(
                            "signatureInformation".to_string(),
                            JsonValue::Object(vec![(
                                "documentationFormat".to_string(),
                                JsonValue::Array(vec![
                                    JsonValue::String("plaintext".to_string()),
                                    JsonValue::String("markdown".to_string()),
                                ]),
                            )]),
                        )]),
                    ),
                ]),
            ),
        ]);

        let params = JsonValue::Object(vec![
            ("processId".to_string(), JsonValue::Number(std::process::id() as f64)),
            ("rootUri".to_string(), JsonValue::String(root_uri)),
            ("capabilities".to_string(), capabilities),
        ]);

        self.send_request("initialize", params);
    }

    pub fn notify_did_open(&mut self, path: &Path, language_id: &str, text: &str) {
        if !self.is_running() {
            return;
        }
        let uri = path_to_uri(path);
        let params = JsonValue::Object(vec![(
            "textDocument".to_string(),
            JsonValue::Object(vec![
                ("uri".to_string(), JsonValue::String(uri)),
                ("languageId".to_string(), JsonValue::String(language_id.to_string())),
                ("version".to_string(), JsonValue::Number(1.0)),
                ("text".to_string(), JsonValue::String(text.to_string())),
            ]),
        )]);
        self.send_notification("textDocument/didOpen", params);
        self.open_docs.insert(path.to_path_buf(), 1);
    }

    pub fn notify_did_change(&mut self, path: &Path, text: &str) {
        if !self.is_running() {
            return;
        }
        let version = self.open_docs.entry(path.to_path_buf()).or_insert(1);
        *version += 1;
        let cur_ver = *version;

        let uri = path_to_uri(path);
        let params = JsonValue::Object(vec![
            (
                "textDocument".to_string(),
                JsonValue::Object(vec![
                    ("uri".to_string(), JsonValue::String(uri)),
                    ("version".to_string(), JsonValue::Number(cur_ver as f64)),
                ]),
            ),
            (
                "contentChanges".to_string(),
                JsonValue::Array(vec![JsonValue::Object(vec![(
                    "text".to_string(),
                    JsonValue::String(text.to_string()),
                )])]),
            ),
        ]);
        self.send_notification("textDocument/didChange", params);
    }

    pub fn notify_did_save(&mut self, path: &Path) {
        if !self.is_running() {
            return;
        }
        let uri = path_to_uri(path);
        let params = JsonValue::Object(vec![(
            "textDocument".to_string(),
            JsonValue::Object(vec![("uri".to_string(), JsonValue::String(uri))]),
        )]);
        self.send_notification("textDocument/didSave", params);
    }

    pub fn notify_did_close(&mut self, path: &Path) {
        if !self.is_running() {
            return;
        }
        let uri = path_to_uri(path);
        let params = JsonValue::Object(vec![(
            "textDocument".to_string(),
            JsonValue::Object(vec![("uri".to_string(), JsonValue::String(uri))]),
        )]);
        self.send_notification("textDocument/didClose", params);
        self.open_docs.remove(path);
    }

    pub fn request_completion(&mut self, path: &Path, line: usize, col: usize) -> u64 {
        let uri = path_to_uri(path);
        let params = JsonValue::Object(vec![
            (
                "textDocument".to_string(),
                JsonValue::Object(vec![("uri".to_string(), JsonValue::String(uri))]),
            ),
            (
                "position".to_string(),
                JsonValue::Object(vec![
                    ("line".to_string(), JsonValue::Number(line as f64)),
                    ("character".to_string(), JsonValue::Number(col as f64)),
                ]),
            ),
        ]);
        self.send_request("textDocument/completion", params)
    }

    pub fn request_hover(&mut self, path: &Path, line: usize, col: usize) -> u64 {
        let uri = path_to_uri(path);
        let params = JsonValue::Object(vec![
            (
                "textDocument".to_string(),
                JsonValue::Object(vec![("uri".to_string(), JsonValue::String(uri))]),
            ),
            (
                "position".to_string(),
                JsonValue::Object(vec![
                    ("line".to_string(), JsonValue::Number(line as f64)),
                    ("character".to_string(), JsonValue::Number(col as f64)),
                ]),
            ),
        ]);
        self.send_request("textDocument/hover", params)
    }

    pub fn request_definition(&mut self, path: &Path, line: usize, col: usize) -> u64 {
        let uri = path_to_uri(path);
        let params = JsonValue::Object(vec![
            (
                "textDocument".to_string(),
                JsonValue::Object(vec![("uri".to_string(), JsonValue::String(uri))]),
            ),
            (
                "position".to_string(),
                JsonValue::Object(vec![
                    ("line".to_string(), JsonValue::Number(line as f64)),
                    ("character".to_string(), JsonValue::Number(col as f64)),
                ]),
            ),
        ]);
        self.send_request("textDocument/definition", params)
    }

    pub fn request_signature_help(&mut self, path: &Path, line: usize, col: usize) -> u64 {
        let uri = path_to_uri(path);
        let params = JsonValue::Object(vec![
            (
                "textDocument".to_string(),
                JsonValue::Object(vec![("uri".to_string(), JsonValue::String(uri))]),
            ),
            (
                "position".to_string(),
                JsonValue::Object(vec![
                    ("line".to_string(), JsonValue::Number(line as f64)),
                    ("character".to_string(), JsonValue::Number(col as f64)),
                ]),
            ),
        ]);
        self.send_request("textDocument/signatureHelp", params)
    }

    fn dispatch_incoming_json(json: &JsonValue, event_tx: &Sender<LspEvent>) {
        // 1. Check for notifications (method present, no id)
        if let Some(method) = json.get("method").and_then(|v| v.as_str()) {
            match method {
                "textDocument/publishDiagnostics" => {
                    if let Some(params) = json.get("params") {
                        if let Some((file, diags)) = Self::parse_publish_diagnostics(params) {
                            let _ = event_tx.send(LspEvent::PublishDiagnostics {
                                file,
                                diagnostics: diags,
                            });
                        }
                    }
                }
                "window/showMessage" | "window/logMessage" => {
                    // Handled silently or logged
                }
                _ => {}
            }
            return;
        }

        // 2. Check for request responses (id present)
        if let Some(id) = json.get("id").and_then(|v| v.as_u64()) {
            // Check if response to 'initialize'
            if let Some(result) = json.get("result") {
                if result.get("capabilities").is_some() {
                    let _ = event_tx.send(LspEvent::Initialized);
                    return;
                }
            }

            // Signature Help Response
            if let Some(sig) = Self::try_parse_signature_help_response(json) {
                let _ = event_tx.send(LspEvent::SignatureHelpResponse {
                    request_id: id,
                    signature_help: sig,
                });
                return;
            }

            // Completion Response
            if let Some(items) = Self::try_parse_completion_response(json) {
                let _ = event_tx.send(LspEvent::CompletionResponse {
                    request_id: id,
                    items,
                });
                return;
            }

            // Hover Response
            if let Some(hover) = Self::try_parse_hover_response(json) {
                let _ = event_tx.send(LspEvent::HoverResponse {
                    request_id: id,
                    hover,
                });
                return;
            }

            // Definition Response
            if let Some(loc) = Self::try_parse_definition_response(json) {
                let _ = event_tx.send(LspEvent::DefinitionResponse {
                    request_id: id,
                    location: loc,
                });
            }
        }
    }

    fn parse_publish_diagnostics(params: &JsonValue) -> Option<(PathBuf, Vec<Diagnostic>)> {
        let uri = params.get("uri")?.as_str()?;
        let path = uri_to_path(uri)?;
        let raw_diags = params.get("diagnostics")?.as_array()?;

        let mut diags = Vec::with_capacity(raw_diags.len());
        for item in raw_diags {
            let msg = item.get("message")?.as_str()?.to_string();
            let range = item.get("range")?;
            let start = range.get("start")?;
            let line_0 = start.get("line")?.as_u64()? as usize;
            let col_0 = start.get("character")?.as_u64()? as usize;

            let end_col = range
                .get("end")
                .and_then(|e| e.get("character"))
                .and_then(|c| c.as_u64())
                .map(|c| c as usize + 1);

            let sev_num = item.get("severity").and_then(|s| s.as_u64()).unwrap_or(1);
            let severity = match sev_num {
                1 => DiagnosticSeverity::Error,
                2 => DiagnosticSeverity::Warning,
                3 => DiagnosticSeverity::Information,
                _ => DiagnosticSeverity::Hint,
            };

            let code = item.get("code").and_then(|c| {
                if let Some(s) = c.as_str() {
                    Some(s.to_string())
                } else if let Some(n) = c.as_i64() {
                    Some(n.to_string())
                } else {
                    None
                }
            });

            let mut d = Diagnostic::new(
                Some(path.clone()),
                line_0 + 1,
                col_0 + 1,
                severity,
                msg,
                code,
            );
            if let Some(ec) = end_col {
                d = d.with_end_col(ec);
            }
            diags.push(d);
        }

        Some((path, diags))
    }

    fn try_parse_completion_response(json: &JsonValue) -> Option<Vec<LspCompletionItem>> {
        let result = json.get("result")?;
        let raw_items = if let Some(items) = result.as_array() {
            items
        } else if let Some(items) = result.get("items").and_then(|v| v.as_array()) {
            items
        } else {
            return None;
        };

        let mut list = Vec::with_capacity(raw_items.len());
        for item in raw_items {
            let label = item.get("label")?.as_str()?.to_string();
            let detail = item.get("detail").and_then(|d| d.as_str()).unwrap_or("").to_string();
            let kind_id = item.get("kind").and_then(|k| k.as_u64()).unwrap_or(0);

            let kind = match kind_id {
                2 | 3 => CompletionKind::Function,
                7 | 22 => CompletionKind::Type,
                14 => CompletionKind::Keyword,
                15 => CompletionKind::Snippet,
                _ => CompletionKind::Word,
            };

            let insert_text = item.get("insertText").and_then(|it| it.as_str()).map(|s| s.to_string());
            let documentation = if let Some(d) = item.get("documentation") {
                if let Some(s) = d.as_str() {
                    s.to_string()
                } else if let Some(v) = d.get("value").and_then(|val| val.as_str()) {
                    v.to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            list.push(LspCompletionItem {
                label,
                detail,
                documentation,
                kind,
                insert_text,
            });
        }
        Some(list)
    }

    fn try_parse_signature_help_response(json: &JsonValue) -> Option<Option<LspSignatureHelp>> {
        let result = json.get("result")?;
        if result == &JsonValue::Null {
            return Some(None);
        }

        let signatures_arr = result.get("signatures")?.as_array()?;
        let active_signature = result.get("activeSignature").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let active_parameter = result.get("activeParameter").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let mut signatures = Vec::with_capacity(signatures_arr.len());
        for sig in signatures_arr {
            let label = sig.get("label")?.as_str()?.to_string();
            let documentation = if let Some(d) = sig.get("documentation") {
                if let Some(s) = d.as_str() {
                    s.to_string()
                } else if let Some(v) = d.get("value").and_then(|val| val.as_str()) {
                    v.to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let sig_active_param = sig.get("activeParameter").and_then(|v| v.as_u64()).map(|v| v as usize);

            let mut parameters = Vec::new();
            if let Some(param_arr) = sig.get("parameters").and_then(|p| p.as_array()) {
                for p in param_arr {
                    let p_label = if let Some(s) = p.get("label").and_then(|l| l.as_str()) {
                        s.to_string()
                    } else {
                        String::new()
                    };
                    let p_doc = p.get("documentation").and_then(|d| d.as_str()).unwrap_or("").to_string();
                    parameters.push(LspParameterInformation {
                        label: p_label,
                        documentation: p_doc,
                    });
                }
            }

            signatures.push(LspSignatureInformation {
                label,
                documentation,
                parameters,
                active_parameter: sig_active_param,
            });
        }

        if signatures.is_empty() {
            Some(None)
        } else {
            Some(Some(LspSignatureHelp {
                signatures,
                active_signature,
                active_parameter,
            }))
        }
    }

    fn try_parse_hover_response(json: &JsonValue) -> Option<Option<LspHover>> {
        let result = json.get("result")?;
        if result == &JsonValue::Null {
            return Some(None);
        }

        let contents = result.get("contents")?;
        let text = if let Some(s) = contents.as_str() {
            s.to_string()
        } else if let Some(val) = contents.get("value").and_then(|v| v.as_str()) {
            val.to_string()
        } else if let Some(arr) = contents.as_array() {
            let mut parts = Vec::new();
            for item in arr {
                if let Some(s) = item.as_str() {
                    parts.push(s);
                } else if let Some(val) = item.get("value").and_then(|v| v.as_str()) {
                    parts.push(val);
                }
            }
            parts.join("\n")
        } else {
            return Some(None);
        };

        Some(Some(LspHover { contents: text }))
    }

    fn try_parse_definition_response(json: &JsonValue) -> Option<Option<LspLocation>> {
        let result = json.get("result")?;
        if result == &JsonValue::Null {
            return Some(None);
        }

        let target = if let Some(arr) = result.as_array() {
            arr.first()?
        } else {
            result
        };

        let uri = target.get("uri").or_else(|| target.get("targetUri"))?.as_str()?;
        let path = uri_to_path(uri)?;
        let range_val = target.get("range").or_else(|| target.get("targetSelectionRange"))?;

        let start_pos = range_val.get("start")?;
        let s_line = start_pos.get("line")?.as_u64()? as usize;
        let s_char = start_pos.get("character")?.as_u64()? as usize;

        let end_pos = range_val.get("end")?;
        let e_line = end_pos.get("line")?.as_u64()? as usize;
        let e_char = end_pos.get("character")?.as_u64()? as usize;

        Some(Some(LspLocation {
            path,
            range: LspRange {
                start: LspPosition { line: s_line, character: s_char },
                end: LspPosition { line: e_line, character: e_char },
            },
        }))
    }
}

impl Default for LspClient {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        self.stop();
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_parser_primitives_and_escapes() {
        let json_str = r#"{"name":"Rustext \"LSP\"","version":1.0,"active":true,"tags":["std",null]}"#;
        let parsed = JsonValue::parse(json_str).unwrap();

        assert_eq!(parsed.get("name").and_then(|v| v.as_str()), Some("Rustext \"LSP\""));
        assert_eq!(parsed.get("version").and_then(|v| v.as_f64()), Some(1.0));
        assert_eq!(parsed.get("active").and_then(|v| v.as_bool()), Some(true));

        let tags = parsed.get("tags").and_then(|v| v.as_array()).unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].as_str(), Some("std"));
        assert_eq!(tags[1], JsonValue::Null);
    }

    #[test]
    fn test_uri_roundtrip_conversion() {
        let p = PathBuf::from("/home/user/project/src/main.rs");
        let uri = path_to_uri(&p);
        assert!(uri.starts_with("file://"));
        assert!(uri.ends_with("/src/main.rs"));

        let decoded = uri_to_path(&uri).unwrap();
        assert!(decoded.ends_with("src/main.rs"));
    }

    #[test]
    fn test_parse_publish_diagnostics() {
        let json_str = r#"{
            "uri": "file:///workspace/src/lib.rs",
            "diagnostics": [
                {
                    "range": {
                        "start": { "line": 9, "character": 4 },
                        "end": { "line": 9, "character": 16 }
                    },
                    "severity": 1,
                    "code": "E0308",
                    "message": "mismatched types: expected u32, found &str"
                }
            ]
        }"#;

        let parsed = JsonValue::parse(json_str).unwrap();
        let (file, diags) = LspClient::parse_publish_diagnostics(&parsed).unwrap();

        assert!(file.ends_with("src/lib.rs"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, 10);
        assert_eq!(diags[0].col, 5);
        assert_eq!(diags[0].end_col, Some(17));
        assert_eq!(diags[0].severity, DiagnosticSeverity::Error);
        assert_eq!(diags[0].code.as_deref(), Some("E0308"));
    }

    #[test]
    fn test_parse_completion_items() {
        let json_str = r#"{
            "id": 42,
            "result": {
                "items": [
                    { "label": "clone", "detail": "fn(&self) -> Self", "kind": 2 },
                    { "label": "String", "detail": "struct std::string::String", "kind": 7 }
                ]
            }
        }"#;

        let parsed = JsonValue::parse(json_str).unwrap();
        let items = LspClient::try_parse_completion_response(&parsed).unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].label, "clone");
        assert_eq!(items[0].kind, CompletionKind::Function);
        assert_eq!(items[1].label, "String");
        assert_eq!(items[1].kind, CompletionKind::Type);
    }
}
