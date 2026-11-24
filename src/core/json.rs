//! Lightweight Pure-Rust JSON AST & Parser (Zero Dependencies).
//!
//! Provides JSON data types and parsing for LSP and configuration.

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
