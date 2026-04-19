use serde_json::Value;
use std::error::Error;

/// Extrai o campo `code` de um conteúdo vindo do LLM.
///
/// Estratégia:
/// 1. tenta parsear como JSON válido
/// 2. fallback: regex para capturar "code": "..."
/// 3. unescape final do conteúdo
pub fn extract_code(raw: &str) -> Result<String, ParserError> {
    // =========================
    // 1. Tentativa normal (JSON válido)
    // =========================
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        if let Some(code) = v.get("code").and_then(|c| c.as_str()) {
            return Ok(unescape(code));
        }
    }

    // =========================
    // 2. Fallback regex (JSON quebrado)
    // =========================
    let re = regex::Regex::new(r#""code"\s*:\s*"((?:\\.|[^"\\])*)""#)
        .map_err(|_| ParserError::InvalidRegex)?;

    if let Some(caps) = re.captures(raw) {
        if let Some(m) = caps.get(1) {
            let extracted = m.as_str();
            return Ok(unescape(extracted));
        }
    }

    // =========================
    // Falhou tudo
    // =========================
    Err(ParserError::CodeNotFound)
}

/// Remove escapes típicos de JSON string vindo do LLM
fn unescape(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some('r') => result.push('\r'),
                Some(other) => {
                    result.push(other);
                }
                None => break,
            }
        } else {
            result.push(c);
        }
    }

    result
}

#[derive(Debug)]
pub enum ParserError {
    CodeNotFound,
    InvalidRegex,
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::CodeNotFound => write!(f, "field 'code' not found in LLM output"),
            ParserError::InvalidRegex => write!(f, "invalid regex pattern"),
        }
    }
}

impl Error for ParserError {}