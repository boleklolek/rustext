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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspTextEdit {
    pub range: LspRange,
    pub new_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspWorkspaceEdit {
    pub changes: HashMap<PathBuf, Vec<LspTextEdit>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspCodeAction {
    pub title: String,
    pub kind: Option<String>,
    pub edit: Option<LspWorkspaceEdit>,
    pub is_preferred: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    File = 1,
    Module = 2,
    Namespace = 3,
    Package = 4,
    Class = 5,
    Method = 6,
    Property = 7,
    Field = 8,
    Constructor = 9,
    Enum = 10,
    Interface = 11,
    Function = 12,
    Variable = 13,
    Constant = 14,
    String = 15,
    Number = 16,
    Boolean = 17,
    Array = 18,
    Object = 19,
    Key = 20,
    Null = 21,
    EnumMember = 22,
    Struct = 23,
    Event = 24,
    Operator = 25,
    TypeParameter = 26,
    Unknown = 99,
}

impl SymbolKind {
    pub fn from_u64(n: u64) -> Self {
        match n {
            1 => SymbolKind::File,
            2 => SymbolKind::Module,
            3 => SymbolKind::Namespace,
            4 => SymbolKind::Package,
            5 => SymbolKind::Class,
            6 => SymbolKind::Method,
            7 => SymbolKind::Property,
            8 => SymbolKind::Field,
            9 => SymbolKind::Constructor,
            10 => SymbolKind::Enum,
            11 => SymbolKind::Interface,
            12 => SymbolKind::Function,
            13 => SymbolKind::Variable,
            14 => SymbolKind::Constant,
            15 => SymbolKind::String,
            16 => SymbolKind::Number,
            17 => SymbolKind::Boolean,
            18 => SymbolKind::Array,
            19 => SymbolKind::Object,
            20 => SymbolKind::Key,
            21 => SymbolKind::Null,
            22 => SymbolKind::EnumMember,
            23 => SymbolKind::Struct,
            24 => SymbolKind::Event,
            25 => SymbolKind::Operator,
            26 => SymbolKind::TypeParameter,
            _ => SymbolKind::Unknown,
        }
    }

    pub fn badge_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "[fn]",
            SymbolKind::Method => "[method]",
            SymbolKind::Struct => "[struct]",
            SymbolKind::Class => "[class]",
            SymbolKind::Interface => "[trait]",
            SymbolKind::Enum => "[enum]",
            SymbolKind::EnumMember => "[variant]",
            SymbolKind::Variable => "[var]",
            SymbolKind::Constant => "[const]",
            SymbolKind::Field | SymbolKind::Property => "[field]",
            SymbolKind::Module | SymbolKind::Namespace | SymbolKind::Package => "[mod]",
            SymbolKind::TypeParameter => "[type]",
            _ => "[sym]",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDocumentSymbol {
    pub name: String,
    pub detail: String,
    pub kind: SymbolKind,
    pub range: LspRange,
    pub selection_range: LspRange,
    pub depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlayHintKind {
    Other = 0,
    Type = 1,
    Parameter = 2,
}

impl InlayHintKind {
    pub fn from_u64(n: u64) -> Self {
        match n {
            1 => InlayHintKind::Type,
            2 => InlayHintKind::Parameter,
            _ => InlayHintKind::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspInlayHint {
    pub position: LspPosition,
    pub label: String,
    pub kind: InlayHintKind,
    pub padding_left: bool,
    pub padding_right: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspReferenceItem {
    pub path: PathBuf,
    pub range: LspRange,
    pub line_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticTokensLegend {
    pub token_types: Vec<String>,
    pub token_modifiers: Vec<String>,
}

impl Default for SemanticTokensLegend {
    fn default() -> Self {
        Self {
            token_types: vec![
                "comment".to_string(),
                "keyword".to_string(),
                "string".to_string(),
                "number".to_string(),
                "regexp".to_string(),
                "operator".to_string(),
                "namespace".to_string(),
                "type".to_string(),
                "struct".to_string(),
                "class".to_string(),
                "interface".to_string(),
                "enum".to_string(),
                "enumMember".to_string(),
                "typeParameter".to_string(),
                "function".to_string(),
                "method".to_string(),
                "property".to_string(),
                "macro".to_string(),
                "variable".to_string(),
                "parameter".to_string(),
                "modifier".to_string(),
            ],
            token_modifiers: vec![
                "declaration".to_string(),
                "definition".to_string(),
                "readonly".to_string(),
                "static".to_string(),
                "deprecated".to_string(),
                "abstract".to_string(),
                "async".to_string(),
                "modification".to_string(),
                "documentation".to_string(),
                "defaultLibrary".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LspEvent {
    ServerStarted { server_name: String },
    Initialized { legend: Option<SemanticTokensLegend> },
    PublishDiagnostics { file: PathBuf, diagnostics: Vec<Diagnostic> },
    CompletionResponse { request_id: u64, items: Vec<LspCompletionItem> },
    HoverResponse { request_id: u64, hover: Option<LspHover> },
    SignatureHelpResponse { request_id: u64, signature_help: Option<LspSignatureHelp> },
    DefinitionResponse { request_id: u64, location: Option<LspLocation> },
    ReferencesResponse { request_id: u64, references: Vec<LspLocation> },
    RenameResponse { request_id: u64, edit: Option<LspWorkspaceEdit> },
    CodeActionResponse { request_id: u64, actions: Vec<LspCodeAction> },
    FormattingResponse { request_id: u64, edits: Vec<LspTextEdit> },
    DocumentSymbolsResponse { request_id: u64, file: Option<PathBuf>, symbols: Vec<LspDocumentSymbol> },
    InlayHintsResponse { request_id: u64, file: Option<PathBuf>, hints: Vec<LspInlayHint> },
    SemanticTokensRawResponse { request_id: u64, raw_data: Vec<u64> },
    SemanticTokensResponse {
        request_id: u64,
        file: Option<PathBuf>,
        tokens: HashMap<usize, Vec<crate::core::syntax::SemanticTokenSpan>>,
    },
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
    pub server_legend: Option<SemanticTokensLegend>,
    pub pending_semantic_requests: HashMap<u64, PathBuf>,
    pub pending_requests: Arc<std::sync::Mutex<HashMap<u64, String>>>,
    pub pending_doc_requests: Arc<std::sync::Mutex<HashMap<u64, PathBuf>>>,
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
            server_legend: None,
            pending_semantic_requests: HashMap::new(),
            pending_requests: Arc::new(std::sync::Mutex::new(HashMap::new())),
            pending_doc_requests: Arc::new(std::sync::Mutex::new(HashMap::new())),
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

        let parts: Vec<&str> = server_cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Comando LSP no especificado".to_string());
        }
        let mut cmd = Command::new(parts[0]);
        cmd.args(&parts[1..]);
        let mut child = cmd
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
        let pending_req_clone = self.pending_requests.clone();
        let pending_doc_clone = self.pending_doc_requests.clone();
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
                        Self::dispatch_incoming_json(&json, &ev_tx_clone, &pending_req_clone, &pending_doc_clone);
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
        self.pending_semantic_requests.clear();
        if let Ok(mut map) = self.pending_requests.lock() {
            map.clear();
        }
        if let Ok(mut map) = self.pending_doc_requests.lock() {
            map.clear();
        }

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
                            LspEvent::Initialized { legend } => {
                                self.status = ServerStatus::Ready;
                                if legend.is_some() {
                                    self.server_legend = legend.clone();
                                }
                            }
                            LspEvent::SemanticTokensRawResponse { request_id, raw_data } => {
                                let file = self.pending_semantic_requests.remove(request_id);
                                let legend = self.server_legend.clone().unwrap_or_default();
                                let tokens = Self::decode_semantic_tokens(raw_data, &legend);
                                events.push(LspEvent::SemanticTokensResponse {
                                    request_id: *request_id,
                                    file,
                                    tokens,
                                });
                                continue;
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

    /// Injects an LSP event into the client event queue for deterministic test verification.
    pub fn push_event_for_test(&mut self, event: LspEvent) {
        let (tx, rx) = channel::<LspEvent>();
        let _ = tx.send(event);
        self.event_rx = Some(rx);
    }

    /// Injects multiple LSP events into the client event queue for deterministic test verification.
    pub fn push_events_for_test(&mut self, events: Vec<LspEvent>) {
        let (tx, rx) = channel::<LspEvent>();
        for ev in events {
            let _ = tx.send(ev);
        }
        self.event_rx = Some(rx);
    }

    fn send_raw_json(&mut self, json_str: &str) {
        if let Some(tx) = &self.stdin_tx {
            let framed = format!("Content-Length: {}\r\n\r\n{}", json_str.len(), json_str);
            let _ = tx.send(framed);
        }
    }

    pub fn send_request(&mut self, method: &str, params: JsonValue) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut map) = self.pending_requests.lock() {
            map.insert(id, method.to_string());
        }
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
                        "references".to_string(),
                        JsonValue::Object(vec![("dynamicRegistration".to_string(), JsonValue::Bool(false))]),
                    ),
                    (
                        "rename".to_string(),
                        JsonValue::Object(vec![
                            ("dynamicRegistration".to_string(), JsonValue::Bool(false)),
                            ("prepareSupport".to_string(), JsonValue::Bool(false)),
                        ]),
                    ),
                    (
                        "codeAction".to_string(),
                        JsonValue::Object(vec![
                            ("dynamicRegistration".to_string(), JsonValue::Bool(false)),
                            ("codeActionLiteralSupport".to_string(), JsonValue::Object(vec![(
                                "codeAction".to_string(),
                                JsonValue::Object(vec![(
                                    "kind".to_string(),
                                    JsonValue::Object(vec![(
                                        "valueSet".to_string(),
                                        JsonValue::Array(vec![
                                            JsonValue::String("".to_string()),
                                            JsonValue::String("quickfix".to_string()),
                                            JsonValue::String("refactor".to_string()),
                                            JsonValue::String("refactor.extract".to_string()),
                                            JsonValue::String("refactor.inline".to_string()),
                                            JsonValue::String("refactor.rewrite".to_string()),
                                            JsonValue::String("source".to_string()),
                                            JsonValue::String("source.organizeImports".to_string()),
                                        ]),
                                    )]),
                                )]),
                            )])),
                        ]),
                    ),
                    (
                        "formatting".to_string(),
                        JsonValue::Object(vec![("dynamicRegistration".to_string(), JsonValue::Bool(false))]),
                    ),
                    (
                        "documentSymbol".to_string(),
                        JsonValue::Object(vec![
                            ("dynamicRegistration".to_string(), JsonValue::Bool(false)),
                            ("hierarchicalDocumentSymbolSupport".to_string(), JsonValue::Bool(true)),
                        ]),
                    ),
                    (
                        "inlayHint".to_string(),
                        JsonValue::Object(vec![("dynamicRegistration".to_string(), JsonValue::Bool(false))]),
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
                    (
                        "semanticTokens".to_string(),
                        JsonValue::Object(vec![
                            ("dynamicRegistration".to_string(), JsonValue::Bool(false)),
                            (
                                "requests".to_string(),
                                JsonValue::Object(vec![("full".to_string(), JsonValue::Bool(true))]),
                            ),
                            (
                                "tokenTypes".to_string(),
                                JsonValue::Array(
                                    vec![
                                        "comment", "keyword", "string", "number", "regexp", "operator",
                                        "namespace", "type", "struct", "class", "interface", "enum",
                                        "enumMember", "typeParameter", "function", "method", "property",
                                        "macro", "variable", "parameter", "modifier",
                                    ]
                                    .into_iter()
                                    .map(|s| JsonValue::String(s.to_string()))
                                    .collect(),
                                ),
                            ),
                            (
                                "tokenModifiers".to_string(),
                                JsonValue::Array(
                                    vec![
                                        "declaration", "definition", "readonly", "static", "deprecated",
                                        "abstract", "async", "modification", "documentation", "defaultLibrary",
                                    ]
                                    .into_iter()
                                    .map(|s| JsonValue::String(s.to_string()))
                                    .collect(),
                                ),
                            ),
                            (
                                "formats".to_string(),
                                JsonValue::Array(vec![JsonValue::String("relative".to_string())]),
                            ),
                        ]),
                    ),
                ]),
            ),
            (
                "workspace".to_string(),
                JsonValue::Object(vec![
                    ("applyEdit".to_string(), JsonValue::Bool(true)),
                    (
                        "workspaceEdit".to_string(),
                        JsonValue::Object(vec![("documentChanges".to_string(), JsonValue::Bool(true))]),
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
        let id = self.send_request("textDocument/completion", params);
        if let Ok(mut map) = self.pending_doc_requests.lock() {
            map.insert(id, path.to_path_buf());
        }
        id
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
        let id = self.send_request("textDocument/hover", params);
        if let Ok(mut map) = self.pending_doc_requests.lock() {
            map.insert(id, path.to_path_buf());
        }
        id
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
        let id = self.send_request("textDocument/definition", params);
        if let Ok(mut map) = self.pending_doc_requests.lock() {
            map.insert(id, path.to_path_buf());
        }
        id
    }

    pub fn request_references(&mut self, path: &Path, line: usize, col: usize, include_decl: bool) -> u64 {
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
            (
                "context".to_string(),
                JsonValue::Object(vec![
                    ("includeDeclaration".to_string(), JsonValue::Bool(include_decl)),
                ]),
            ),
        ]);
        let id = self.send_request("textDocument/references", params);
        if let Ok(mut map) = self.pending_doc_requests.lock() {
            map.insert(id, path.to_path_buf());
        }
        id
    }

    pub fn request_rename(&mut self, path: &Path, line: usize, col: usize, new_name: &str) -> u64 {
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
            ("newName".to_string(), JsonValue::String(new_name.to_string())),
        ]);
        let id = self.send_request("textDocument/rename", params);
        if let Ok(mut map) = self.pending_doc_requests.lock() {
            map.insert(id, path.to_path_buf());
        }
        id
    }

    pub fn request_code_actions(&mut self, path: &Path, range: LspRange, diagnostics: &[Diagnostic]) -> u64 {
        let uri = path_to_uri(path);
        let diags_json: Vec<JsonValue> = diagnostics
            .iter()
            .map(|d| {
                let mut entries = vec![
                    ("message".to_string(), JsonValue::String(d.message.clone())),
                    (
                        "severity".to_string(),
                        JsonValue::Number(match d.severity {
                            DiagnosticSeverity::Error => 1.0,
                            DiagnosticSeverity::Warning => 2.0,
                            DiagnosticSeverity::Information => 3.0,
                            DiagnosticSeverity::Hint => 4.0,
                        }),
                    ),
                    (
                        "range".to_string(),
                        JsonValue::Object(vec![
                            (
                                "start".to_string(),
                                JsonValue::Object(vec![
                                    ("line".to_string(), JsonValue::Number(d.line.saturating_sub(1) as f64)),
                                    ("character".to_string(), JsonValue::Number(d.col.saturating_sub(1) as f64)),
                                ]),
                            ),
                            (
                                "end".to_string(),
                                JsonValue::Object(vec![
                                    ("line".to_string(), JsonValue::Number(d.line.saturating_sub(1) as f64)),
                                    (
                                        "character".to_string(),
                                        JsonValue::Number(d.end_col.unwrap_or(d.col).saturating_sub(1) as f64),
                                    ),
                                ]),
                            ),
                        ]),
                    ),
                ];
                if let Some(code) = &d.code {
                    entries.push(("code".to_string(), JsonValue::String(code.clone())));
                }
                JsonValue::Object(entries)
            })
            .collect();

        let params = JsonValue::Object(vec![
            (
                "textDocument".to_string(),
                JsonValue::Object(vec![("uri".to_string(), JsonValue::String(uri))]),
            ),
            (
                "range".to_string(),
                JsonValue::Object(vec![
                    (
                        "start".to_string(),
                        JsonValue::Object(vec![
                            ("line".to_string(), JsonValue::Number(range.start.line as f64)),
                            ("character".to_string(), JsonValue::Number(range.start.character as f64)),
                        ]),
                    ),
                    (
                        "end".to_string(),
                        JsonValue::Object(vec![
                            ("line".to_string(), JsonValue::Number(range.end.line as f64)),
                            ("character".to_string(), JsonValue::Number(range.end.character as f64)),
                        ]),
                    ),
                ]),
            ),
            (
                "context".to_string(),
                JsonValue::Object(vec![("diagnostics".to_string(), JsonValue::Array(diags_json))]),
            ),
        ]);
        let id = self.send_request("textDocument/codeAction", params);
        if let Ok(mut map) = self.pending_doc_requests.lock() {
            map.insert(id, path.to_path_buf());
        }
        id
    }

    pub fn request_formatting(&mut self, path: &Path, tab_size: usize, insert_spaces: bool) -> u64 {
        let uri = path_to_uri(path);
        let params = JsonValue::Object(vec![
            (
                "textDocument".to_string(),
                JsonValue::Object(vec![("uri".to_string(), JsonValue::String(uri))]),
            ),
            (
                "options".to_string(),
                JsonValue::Object(vec![
                    ("tabSize".to_string(), JsonValue::Number(tab_size as f64)),
                    ("insertSpaces".to_string(), JsonValue::Bool(insert_spaces)),
                ]),
            ),
        ]);
        let id = self.send_request("textDocument/formatting", params);
        if let Ok(mut map) = self.pending_doc_requests.lock() {
            map.insert(id, path.to_path_buf());
        }
        id
    }

    pub fn request_document_symbols(&mut self, path: &Path) -> u64 {
        let uri = path_to_uri(path);
        let params = JsonValue::Object(vec![(
            "textDocument".to_string(),
            JsonValue::Object(vec![("uri".to_string(), JsonValue::String(uri))]),
        )]);
        let id = self.send_request("textDocument/documentSymbol", params);
        if let Ok(mut map) = self.pending_doc_requests.lock() {
            map.insert(id, path.to_path_buf());
        }
        id
    }

    pub fn request_inlay_hints(&mut self, path: &Path, range: LspRange) -> u64 {
        let uri = path_to_uri(path);
        let params = JsonValue::Object(vec![
            (
                "textDocument".to_string(),
                JsonValue::Object(vec![("uri".to_string(), JsonValue::String(uri))]),
            ),
            (
                "range".to_string(),
                JsonValue::Object(vec![
                    (
                        "start".to_string(),
                        JsonValue::Object(vec![
                            ("line".to_string(), JsonValue::Number(range.start.line as f64)),
                            ("character".to_string(), JsonValue::Number(range.start.character as f64)),
                        ]),
                    ),
                    (
                        "end".to_string(),
                        JsonValue::Object(vec![
                            ("line".to_string(), JsonValue::Number(range.end.line as f64)),
                            ("character".to_string(), JsonValue::Number(range.end.character as f64)),
                        ]),
                    ),
                ]),
            ),
        ]);
        let id = self.send_request("textDocument/inlayHint", params);
        if let Ok(mut map) = self.pending_doc_requests.lock() {
            map.insert(id, path.to_path_buf());
        }
        id
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
        let id = self.send_request("textDocument/signatureHelp", params);
        if let Ok(mut map) = self.pending_doc_requests.lock() {
            map.insert(id, path.to_path_buf());
        }
        id
    }

    pub fn request_semantic_tokens(&mut self, path: &Path) -> u64 {
        let uri = path_to_uri(path);
        let params = JsonValue::Object(vec![(
            "textDocument".to_string(),
            JsonValue::Object(vec![("uri".to_string(), JsonValue::String(uri))]),
        )]);
        let id = self.send_request("textDocument/semanticTokens/full", params);
        self.pending_semantic_requests.insert(id, path.to_path_buf());
        if let Ok(mut map) = self.pending_doc_requests.lock() {
            map.insert(id, path.to_path_buf());
        }
        id
    }

    fn dispatch_incoming_json(
        json: &JsonValue,
        event_tx: &Sender<LspEvent>,
        pending_requests: &Arc<std::sync::Mutex<HashMap<u64, String>>>,
        pending_doc_requests: &Arc<std::sync::Mutex<HashMap<u64, PathBuf>>>,
    ) {
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
            let method_opt = pending_requests.lock().ok().and_then(|mut m| m.remove(&id));
            let file_opt = pending_doc_requests.lock().ok().and_then(|mut m| m.remove(&id));

            // Check if response to 'initialize'
            if method_opt.as_deref() == Some("initialize")
                || (method_opt.is_none() && json.get_path(&["result", "capabilities"]).is_some())
            {
                if let Some(result) = json.get("result") {
                    if let Some(caps) = result.get("capabilities") {
                        let legend = caps.get_path(&["semanticTokensProvider", "legend"]).map(|leg| {
                            let types = leg
                                .get("tokenTypes")
                                .and_then(|v| v.as_array())
                                .map(|arr| arr.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                                .unwrap_or_default();
                            let mods = leg
                                .get("tokenModifiers")
                                .and_then(|v| v.as_array())
                                .map(|arr| arr.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                                .unwrap_or_default();
                            SemanticTokensLegend {
                                token_types: types,
                                token_modifiers: mods,
                            }
                        });
                        let _ = event_tx.send(LspEvent::Initialized { legend });
                        return;
                    }
                }
            }

            match method_opt.as_deref() {
                Some("textDocument/references") => {
                    if let Some(references) = Self::try_parse_references_response(json) {
                        let _ = event_tx.send(LspEvent::ReferencesResponse {
                            request_id: id,
                            references,
                        });
                        return;
                    }
                }
                Some("textDocument/rename") => {
                    if let Some(edit) = Self::try_parse_rename_response(json) {
                        let _ = event_tx.send(LspEvent::RenameResponse {
                            request_id: id,
                            edit,
                        });
                        return;
                    }
                }
                Some("textDocument/codeAction") => {
                    if let Some(actions) = Self::try_parse_code_actions_response(json) {
                        let _ = event_tx.send(LspEvent::CodeActionResponse {
                            request_id: id,
                            actions,
                        });
                        return;
                    }
                }
                Some("textDocument/formatting") => {
                    if let Some(edits) = Self::try_parse_formatting_response(json) {
                        let _ = event_tx.send(LspEvent::FormattingResponse {
                            request_id: id,
                            edits,
                        });
                        return;
                    }
                }
                Some("textDocument/documentSymbol") => {
                    if let Some(symbols) = Self::try_parse_document_symbols_response(json) {
                        let _ = event_tx.send(LspEvent::DocumentSymbolsResponse {
                            request_id: id,
                            file: file_opt,
                            symbols,
                        });
                        return;
                    }
                }
                Some("textDocument/inlayHint") => {
                    if let Some(hints) = Self::try_parse_inlay_hints_response(json) {
                        let _ = event_tx.send(LspEvent::InlayHintsResponse {
                            request_id: id,
                            file: file_opt,
                            hints,
                        });
                        return;
                    }
                }
                Some("textDocument/completion") => {
                    if let Some(items) = Self::try_parse_completion_response(json) {
                        let _ = event_tx.send(LspEvent::CompletionResponse {
                            request_id: id,
                            items,
                        });
                        return;
                    }
                }
                Some("textDocument/hover") => {
                    if let Some(hover) = Self::try_parse_hover_response(json) {
                        let _ = event_tx.send(LspEvent::HoverResponse {
                            request_id: id,
                            hover,
                        });
                        return;
                    }
                }
                Some("textDocument/definition") => {
                    if let Some(loc) = Self::try_parse_definition_response(json) {
                        let _ = event_tx.send(LspEvent::DefinitionResponse {
                            request_id: id,
                            location: loc,
                        });
                        return;
                    }
                }
                Some("textDocument/signatureHelp") => {
                    if let Some(sig) = Self::try_parse_signature_help_response(json) {
                        let _ = event_tx.send(LspEvent::SignatureHelpResponse {
                            request_id: id,
                            signature_help: sig,
                        });
                        return;
                    }
                }
                Some("textDocument/semanticTokens/full") => {
                    if let Some(data) = Self::try_parse_semantic_tokens_response(json) {
                        let _ = event_tx.send(LspEvent::SemanticTokensRawResponse {
                            request_id: id,
                            raw_data: data,
                        });
                        return;
                    }
                }
                _ => {
                    // Fallback heuristics
                    if let Some(sig) = Self::try_parse_signature_help_response(json) {
                        let _ = event_tx.send(LspEvent::SignatureHelpResponse {
                            request_id: id,
                            signature_help: sig,
                        });
                        return;
                    }
                    if let Some(items) = Self::try_parse_completion_response(json) {
                        let _ = event_tx.send(LspEvent::CompletionResponse {
                            request_id: id,
                            items,
                        });
                        return;
                    }
                    if let Some(hover) = Self::try_parse_hover_response(json) {
                        let _ = event_tx.send(LspEvent::HoverResponse {
                            request_id: id,
                            hover,
                        });
                        return;
                    }
                    if let Some(loc) = Self::try_parse_definition_response(json) {
                        let _ = event_tx.send(LspEvent::DefinitionResponse {
                            request_id: id,
                            location: loc,
                        });
                        return;
                    }
                    if let Some(data) = Self::try_parse_semantic_tokens_response(json) {
                        let _ = event_tx.send(LspEvent::SemanticTokensRawResponse {
                            request_id: id,
                            raw_data: data,
                        });
                    }
                }
            }
        }
    }

    fn try_parse_semantic_tokens_response(json: &JsonValue) -> Option<Vec<u64>> {
        let result = json.get("result")?;
        let data_arr = result.get("data")?.as_array()?;
        let mut data = Vec::with_capacity(data_arr.len());
        for item in data_arr {
            data.push(item.as_u64()?);
        }
        Some(data)
    }

    pub fn decode_semantic_tokens(
        data: &[u64],
        legend: &SemanticTokensLegend,
    ) -> HashMap<usize, Vec<crate::core::syntax::SemanticTokenSpan>> {
        let mut result = HashMap::new();
        if data.len() < 5 {
            return result;
        }

        let mut current_line: usize = 0;
        let mut current_col: usize = 0;

        let chunk_count = data.len() / 5;
        for i in 0..chunk_count {
            let delta_line = data[5 * i + 0] as usize;
            let delta_start_char = data[5 * i + 1] as usize;
            let length = data[5 * i + 2] as usize;
            let token_type_idx = data[5 * i + 3] as usize;
            let token_modifiers = data[5 * i + 4];

            if delta_line > 0 {
                current_line += delta_line;
                current_col = delta_start_char;
            } else {
                current_col += delta_start_char;
            }

            let type_name = legend
                .token_types
                .get(token_type_idx)
                .map(|s| s.as_str())
                .unwrap_or("");

            let token_type = match type_name {
                "function" => crate::core::syntax::TokenType::Function,
                "method" => crate::core::syntax::TokenType::Method,
                "struct" | "class" | "interface" | "enum" | "union" | "typeAlias" => {
                    crate::core::syntax::TokenType::Struct
                }
                "trait" => crate::core::syntax::TokenType::Trait,
                "macro" => crate::core::syntax::TokenType::Macro,
                "parameter" => crate::core::syntax::TokenType::Parameter,
                "variable" => {
                    let is_constant = legend
                        .token_modifiers
                        .iter()
                        .enumerate()
                        .any(|(bit, mod_name)| {
                            (mod_name == "readonly" || mod_name == "static")
                                && (token_modifiers & (1 << bit)) != 0
                        });
                    if is_constant {
                        crate::core::syntax::TokenType::Constant
                    } else {
                        crate::core::syntax::TokenType::Identifier
                    }
                }
                "property" | "field" => crate::core::syntax::TokenType::Field,
                "type" | "typeParameter" => crate::core::syntax::TokenType::Type,
                "keyword" | "modifier" => crate::core::syntax::TokenType::Keyword,
                "string" => crate::core::syntax::TokenType::StringLiteral,
                "number" => crate::core::syntax::TokenType::Number,
                "comment" => crate::core::syntax::TokenType::Comment,
                "operator" => crate::core::syntax::TokenType::Operator,
                _ => crate::core::syntax::TokenType::Identifier,
            };

            let span = crate::core::syntax::SemanticTokenSpan::new(
                current_col,
                current_col + length,
                token_type,
            );
            result.entry(current_line).or_insert_with(Vec::new).push(span);
        }

        result
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

    pub fn parse_location(val: &JsonValue) -> Option<LspLocation> {
        let uri = val.get("uri").or_else(|| val.get("targetUri"))?.as_str()?;
        let path = uri_to_path(uri)?;
        let range_val = val.get("range").or_else(|| val.get("targetSelectionRange"))?;

        let start_pos = range_val.get("start")?;
        let s_line = start_pos.get("line")?.as_u64()? as usize;
        let s_char = start_pos.get("character")?.as_u64()? as usize;

        let end_pos = range_val.get("end")?;
        let e_line = end_pos.get("line")?.as_u64()? as usize;
        let e_char = end_pos.get("character")?.as_u64()? as usize;

        Some(LspLocation {
            path,
            range: LspRange {
                start: LspPosition { line: s_line, character: s_char },
                end: LspPosition { line: e_line, character: e_char },
            },
        })
    }

    pub fn parse_range(val: &JsonValue) -> Option<LspRange> {
        let start_pos = val.get("start")?;
        let s_line = start_pos.get("line")?.as_u64()? as usize;
        let s_char = start_pos.get("character")?.as_u64()? as usize;
        let end_pos = val.get("end")?;
        let e_line = end_pos.get("line")?.as_u64()? as usize;
        let e_char = end_pos.get("character")?.as_u64()? as usize;
        Some(LspRange {
            start: LspPosition { line: s_line, character: s_char },
            end: LspPosition { line: e_line, character: e_char },
        })
    }

    pub fn try_parse_references_response(json: &JsonValue) -> Option<Vec<LspLocation>> {
        let result = json.get("result")?;
        if result == &JsonValue::Null {
            return Some(Vec::new());
        }
        let arr = result.as_array()?;
        let mut locs = Vec::with_capacity(arr.len());
        for item in arr {
            if let Some(loc) = Self::parse_location(item) {
                locs.push(loc);
            }
        }
        Some(locs)
    }

    pub fn try_parse_rename_response(json: &JsonValue) -> Option<Option<LspWorkspaceEdit>> {
        let result = json.get("result")?;
        if result == &JsonValue::Null {
            return Some(None);
        }
        let ws_edit = Self::try_parse_workspace_edit(result)?;
        Some(Some(ws_edit))
    }

    pub fn try_parse_workspace_edit(val: &JsonValue) -> Option<LspWorkspaceEdit> {
        let mut changes_map: HashMap<PathBuf, Vec<LspTextEdit>> = HashMap::new();

        // 1. Changes map: { "uri": [ TextEdit, ... ] }
        if let Some(changes_obj) = val.get("changes").and_then(|v| v.as_object()) {
            for (uri_str, edits_val) in changes_obj {
                if let Some(path) = uri_to_path(uri_str) {
                    if let Some(edits_arr) = edits_val.as_array() {
                        let mut edits = Vec::new();
                        for edit_json in edits_arr {
                            if let Some(te) = Self::try_parse_text_edit(edit_json) {
                                edits.push(te);
                            }
                        }
                        changes_map.entry(path).or_default().extend(edits);
                    }
                }
            }
        }

        // 2. Document changes array: [ { textDocument: { uri: "..." }, edits: [ ... ] }, ... ]
        if let Some(doc_changes) = val.get("documentChanges").and_then(|v| v.as_array()) {
            for change_item in doc_changes {
                if let Some(uri_str) = change_item.get_path(&["textDocument", "uri"]).and_then(|u| u.as_str()) {
                    if let Some(path) = uri_to_path(uri_str) {
                        if let Some(edits_arr) = change_item.get("edits").and_then(|e| e.as_array()) {
                            let mut edits = Vec::new();
                            for edit_json in edits_arr {
                                if let Some(te) = Self::try_parse_text_edit(edit_json) {
                                    edits.push(te);
                                }
                            }
                            changes_map.entry(path).or_default().extend(edits);
                        }
                    }
                }
            }
        }

        Some(LspWorkspaceEdit { changes: changes_map })
    }

    pub fn try_parse_text_edit(val: &JsonValue) -> Option<LspTextEdit> {
        let range_val = val.get("range")?;
        let range = Self::parse_range(range_val)?;
        let new_text = val.get("newText")?.as_str()?.to_string();
        Some(LspTextEdit { range, new_text })
    }

    pub fn try_parse_code_actions_response(json: &JsonValue) -> Option<Vec<LspCodeAction>> {
        let result = json.get("result")?;
        if result == &JsonValue::Null {
            return Some(Vec::new());
        }
        let arr = result.as_array()?;
        let mut actions = Vec::with_capacity(arr.len());
        for item in arr {
            let title = match item.get("title").and_then(|t| t.as_str()) {
                Some(t) => t.to_string(),
                None => continue,
            };
            let kind = item.get("kind").and_then(|k| k.as_str()).map(String::from);
            let is_preferred = item.get("isPreferred").and_then(|b| b.as_bool()).unwrap_or(false);
            let edit = item.get("edit").and_then(Self::try_parse_workspace_edit);

            actions.push(LspCodeAction {
                title,
                kind,
                edit,
                is_preferred,
            });
        }
        Some(actions)
    }

    pub fn try_parse_formatting_response(json: &JsonValue) -> Option<Vec<LspTextEdit>> {
        let result = json.get("result")?;
        if result == &JsonValue::Null {
            return Some(Vec::new());
        }
        let arr = result.as_array()?;
        let mut edits = Vec::with_capacity(arr.len());
        for item in arr {
            if let Some(te) = Self::try_parse_text_edit(item) {
                edits.push(te);
            }
        }
        Some(edits)
    }

    pub fn try_parse_document_symbols_response(json: &JsonValue) -> Option<Vec<LspDocumentSymbol>> {
        let result = json.get("result")?;
        if result == &JsonValue::Null {
            return Some(Vec::new());
        }
        let arr = result.as_array()?;
        let mut symbols = Vec::new();
        for item in arr {
            Self::parse_document_symbol_recursive(item, 0, &mut symbols);
        }
        Some(symbols)
    }

    fn parse_document_symbol_recursive(val: &JsonValue, depth: usize, out: &mut Vec<LspDocumentSymbol>) {
        let name = match val.get("name").and_then(|n| n.as_str()) {
            Some(n) => n.to_string(),
            None => return,
        };
        let detail = val.get("detail").and_then(|d| d.as_str()).unwrap_or("").to_string();
        let kind_num = val.get("kind").and_then(|k| k.as_u64()).unwrap_or(0);
        let kind = SymbolKind::from_u64(kind_num);

        let (range, selection_range) = if let Some(r_val) = val.get("range") {
            let r = Self::parse_range(r_val).unwrap_or(LspRange {
                start: LspPosition { line: 0, character: 0 },
                end: LspPosition { line: 0, character: 0 },
            });
            let sel = val.get("selectionRange").and_then(Self::parse_range).unwrap_or(r);
            (r, sel)
        } else if let Some(loc) = val.get("location") {
            let r = loc.get("range").and_then(Self::parse_range).unwrap_or(LspRange {
                start: LspPosition { line: 0, character: 0 },
                end: LspPosition { line: 0, character: 0 },
            });
            (r, r)
        } else {
            return;
        };

        out.push(LspDocumentSymbol {
            name,
            detail,
            kind,
            range,
            selection_range,
            depth,
        });

        if let Some(children) = val.get("children").and_then(|c| c.as_array()) {
            for child in children {
                Self::parse_document_symbol_recursive(child, depth + 1, out);
            }
        }
    }

    pub fn try_parse_inlay_hints_response(json: &JsonValue) -> Option<Vec<LspInlayHint>> {
        let result = json.get("result")?;
        if result == &JsonValue::Null {
            return Some(Vec::new());
        }
        let arr = result.as_array()?;
        let mut hints = Vec::with_capacity(arr.len());
        for item in arr {
            let pos_val = item.get("position")?;
            let line = pos_val.get("line")?.as_u64()? as usize;
            let character = pos_val.get("character")?.as_u64()? as usize;

            let label = if let Some(s) = item.get("label").and_then(|l| l.as_str()) {
                s.to_string()
            } else if let Some(parts) = item.get("label").and_then(|l| l.as_array()) {
                let mut buf = String::new();
                for p in parts {
                    if let Some(v) = p.get("value").and_then(|v| v.as_str()) {
                        buf.push_str(v);
                    }
                }
                buf
            } else {
                continue;
            };

            let kind_num = item.get("kind").and_then(|k| k.as_u64()).unwrap_or(0);
            let kind = InlayHintKind::from_u64(kind_num);
            let padding_left = item.get("paddingLeft").and_then(|b| b.as_bool()).unwrap_or(false);
            let padding_right = item.get("paddingRight").and_then(|b| b.as_bool()).unwrap_or(false);

            hints.push(LspInlayHint {
                position: LspPosition { line, character },
                label,
                kind,
                padding_left,
                padding_right,
            });
        }
        Some(hints)
    }
}

/// Applies reverse-order LSP `TextEdit`s to a `TextBuffer` guaranteeing zero offset drift.
pub fn apply_text_edits(buffer: &mut crate::core::buffer::TextBuffer, mut edits: Vec<LspTextEdit>) {
    if edits.is_empty() {
        return;
    }
    // Sort in reverse order: latest line and character first
    edits.sort_by(|a, b| {
        b.range
            .start
            .line
            .cmp(&a.range.start.line)
            .then_with(|| b.range.start.character.cmp(&a.range.start.character))
    });

    for edit in edits {
        let start_char = lsp_pos_to_char(buffer, edit.range.start);
        let end_char = lsp_pos_to_char(buffer, edit.range.end);
        if start_char < end_char {
            buffer.delete_range(start_char, end_char);
        }
        if !edit.new_text.is_empty() {
            buffer.insert(start_char, &edit.new_text);
        }
    }
}

/// Converts an LSP 0-indexed position into a buffer character offset with robust line boundary safety.
pub fn lsp_pos_to_char(buffer: &crate::core::buffer::TextBuffer, pos: LspPosition) -> usize {
    if buffer.len_lines() == 0 {
        return 0;
    }
    if pos.line >= buffer.len_lines() {
        return buffer.len_chars();
    }
    let line_start = buffer.line_starts()[pos.line];
    let next_line_start = if pos.line + 1 < buffer.len_lines() {
        buffer.line_starts()[pos.line + 1]
    } else {
        buffer.len_chars()
    };
    (line_start + pos.character).min(next_line_start).min(buffer.len_chars())
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

    #[test]
    fn test_decode_semantic_tokens() {
        let legend = SemanticTokensLegend::default();
        // Encoded 5-tuples:
        // Token 1: line 0, col 3, len 4 (type 14 = "function", mod 0)
        // Token 2: line 0 (delta 0), col +5 (col 8), len 3 (type 8 = "struct", mod 0)
        // Token 3: line 2 (delta 2), col 4, len 5 (type 17 = "macro", mod 0)
        let data = vec![
            0, 3, 4, 14, 0,
            0, 5, 3, 8, 0,
            2, 4, 5, 17, 0,
        ];
        let decoded = LspClient::decode_semantic_tokens(&data, &legend);
        assert_eq!(decoded.len(), 2);

        let line0 = &decoded[&0];
        assert_eq!(line0.len(), 2);
        assert_eq!(line0[0].start_col, 3);
        assert_eq!(line0[0].end_col, 7);
        assert_eq!(line0[0].token_type, crate::core::syntax::TokenType::Function);

        assert_eq!(line0[1].start_col, 8);
        assert_eq!(line0[1].end_col, 11);
        assert_eq!(line0[1].token_type, crate::core::syntax::TokenType::Struct);

        let line2 = &decoded[&2];
        assert_eq!(line2.len(), 1);
        assert_eq!(line2[0].start_col, 4);
        assert_eq!(line2[0].end_col, 9);
        assert_eq!(line2[0].token_type, crate::core::syntax::TokenType::Macro);
    }

    #[test]
    fn test_parse_references_response() {
        let json_str = r#"{
            "id": 10,
            "result": [
                {
                    "uri": "file:///workspace/src/main.rs",
                    "range": {
                        "start": { "line": 4, "character": 8 },
                        "end": { "line": 4, "character": 16 }
                    }
                },
                {
                    "uri": "file:///workspace/src/lib.rs",
                    "range": {
                        "start": { "line": 12, "character": 4 },
                        "end": { "line": 12, "character": 12 }
                    }
                }
            ]
        }"#;
        let parsed = JsonValue::parse(json_str).unwrap();
        let refs = LspClient::try_parse_references_response(&parsed).unwrap();
        assert_eq!(refs.len(), 2);
        assert!(refs[0].path.ends_with("src/main.rs"));
        assert_eq!(refs[0].range.start.line, 4);
        assert_eq!(refs[0].range.start.character, 8);
        assert!(refs[1].path.ends_with("src/lib.rs"));
        assert_eq!(refs[1].range.start.line, 12);
    }

    #[test]
    fn test_parse_rename_response_both_formats() {
        // Format A: changes
        let json_a = r#"{
            "id": 11,
            "result": {
                "changes": {
                    "file:///workspace/src/app.rs": [
                        {
                            "range": {
                                "start": { "line": 1, "character": 4 },
                                "end": { "line": 1, "character": 7 }
                            },
                            "newText": "new_var"
                        }
                    ]
                }
            }
        }"#;
        let parsed_a = JsonValue::parse(json_a).unwrap();
        let edit_a = LspClient::try_parse_rename_response(&parsed_a).unwrap().unwrap();
        assert_eq!(edit_a.changes.len(), 1);
        let path = edit_a.changes.keys().next().unwrap();
        assert!(path.ends_with("src/app.rs"));
        assert_eq!(edit_a.changes[path][0].new_text, "new_var");

        // Format B: documentChanges
        let json_b = r#"{
            "id": 12,
            "result": {
                "documentChanges": [
                    {
                        "textDocument": { "uri": "file:///workspace/src/mod.rs", "version": 2 },
                        "edits": [
                            {
                                "range": {
                                    "start": { "line": 10, "character": 0 },
                                    "end": { "line": 10, "character": 3 }
                                },
                                "newText": "let"
                            }
                        ]
                    }
                ]
            }
        }"#;
        let parsed_b = JsonValue::parse(json_b).unwrap();
        let edit_b = LspClient::try_parse_rename_response(&parsed_b).unwrap().unwrap();
        assert_eq!(edit_b.changes.len(), 1);
        let b_path = edit_b.changes.keys().next().unwrap();
        assert!(b_path.ends_with("src/mod.rs"));
        assert_eq!(edit_b.changes[b_path][0].new_text, "let");
    }

    #[test]
    fn test_parse_code_actions_response() {
        let json_str = r#"{
            "id": 13,
            "result": [
                {
                    "title": "Import std::collections::HashMap",
                    "kind": "quickfix",
                    "isPreferred": true
                },
                {
                    "title": "Extract function",
                    "kind": "refactor.extract"
                }
            ]
        }"#;
        let parsed = JsonValue::parse(json_str).unwrap();
        let actions = LspClient::try_parse_code_actions_response(&parsed).unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].title, "Import std::collections::HashMap");
        assert_eq!(actions[0].kind.as_deref(), Some("quickfix"));
        assert!(actions[0].is_preferred);
        assert_eq!(actions[1].title, "Extract function");
        assert_eq!(actions[1].kind.as_deref(), Some("refactor.extract"));
        assert!(!actions[1].is_preferred);
    }

    #[test]
    fn test_parse_formatting_response() {
        let json_str = r#"{
            "id": 14,
            "result": [
                {
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 0, "character": 0 }
                    },
                    "newText": "    "
                }
            ]
        }"#;
        let parsed = JsonValue::parse(json_str).unwrap();
        let edits = LspClient::try_parse_formatting_response(&parsed).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "    ");
    }

    #[test]
    fn test_parse_document_symbols_hierarchical() {
        let json_str = r#"{
            "id": 15,
            "result": [
                {
                    "name": "App",
                    "kind": 23,
                    "range": {
                        "start": { "line": 10, "character": 0 },
                        "end": { "line": 30, "character": 1 }
                    },
                    "children": [
                        {
                            "name": "run",
                            "kind": 6,
                            "range": {
                                "start": { "line": 15, "character": 4 },
                                "end": { "line": 20, "character": 5 }
                            }
                        }
                    ]
                }
            ]
        }"#;
        let parsed = JsonValue::parse(json_str).unwrap();
        let symbols = LspClient::try_parse_document_symbols_response(&parsed).unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "App");
        assert_eq!(symbols[0].kind, SymbolKind::Struct);
        assert_eq!(symbols[0].depth, 0);

        assert_eq!(symbols[1].name, "run");
        assert_eq!(symbols[1].kind, SymbolKind::Method);
        assert_eq!(symbols[1].depth, 1);
    }

    #[test]
    fn test_parse_inlay_hints_response() {
        let json_str = r#"{
            "id": 16,
            "result": [
                {
                    "position": { "line": 3, "character": 12 },
                    "label": ": i32",
                    "kind": 1,
                    "paddingLeft": true
                }
            ]
        }"#;
        let parsed = JsonValue::parse(json_str).unwrap();
        let hints = LspClient::try_parse_inlay_hints_response(&parsed).unwrap();
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].label, ": i32");
        assert_eq!(hints[0].kind, InlayHintKind::Type);
        assert_eq!(hints[0].position.line, 3);
        assert_eq!(hints[0].position.character, 12);
        assert!(hints[0].padding_left);
    }

    #[test]
    fn test_apply_text_edits_reverse_order_precision() {
        use crate::core::buffer::TextBuffer;
        let mut buf = TextBuffer::from_str("hello world\nfoo bar\nbaz qux\n");

        // Edits out of order: one at line 2, one at line 0, one at line 1
        let edits = vec![
            LspTextEdit {
                range: LspRange {
                    start: LspPosition { line: 0, character: 6 },
                    end: LspPosition { line: 0, character: 11 },
                },
                new_text: "Rustext".to_string(),
            },
            LspTextEdit {
                range: LspRange {
                    start: LspPosition { line: 2, character: 0 },
                    end: LspPosition { line: 2, character: 3 },
                },
                new_text: "super".to_string(),
            },
            LspTextEdit {
                range: LspRange {
                    start: LspPosition { line: 1, character: 4 },
                    end: LspPosition { line: 1, character: 7 },
                },
                new_text: "baz".to_string(),
            },
        ];

        apply_text_edits(&mut buf, edits);
        assert_eq!(buf.to_string(), "hello Rustext\nfoo baz\nsuper qux\n");
    }
}
