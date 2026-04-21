use serde_json::Value;
use std::error::Error;
use std::collections::HashMap;

/// Extrai TODOS os códigos de um conteúdo vindo do LLM.
///
/// Estratégia:
/// 1. Tenta parsear como JSON válido com um único atributo contendo um array
/// 2. Fallback: regex para capturar qualquer atributo com array
/// 3. Fallback: tenta extrair código único (backward compatibility)
/// 4. Unescape final do conteúdo
pub fn extract_codes(raw: &str) -> Result<Vec<String>, ParserError> {
    let trimmed = raw.trim();
    
    // =========================
    // 1. Tentativa normal (JSON válido com array)
    // =========================
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        if let Some(codes) = extract_first_array(&v) {
            return Ok(codes);
        }
    }

    // =========================
    // 2. Fallback regex (JSON quebrado) - captura qualquer atributo com array
    // =========================
    let re = regex::Regex::new(r#""(\w+)"\s*:\s*\[(.*?)\](?=\s*\})"#)
        .map_err(|_| ParserError::InvalidRegex)?;

    if let Some(caps) = re.captures(trimmed) {
        if let Some(array_content) = caps.get(2) {
            let codes = extract_strings_from_array(array_content.as_str());
            if !codes.is_empty() {
                return Ok(codes);
            }
        }
    }

    // =========================
    // 3. Fallback: tenta extrair código único (backward compatibility)
    // =========================
    if let Ok(single_code) = extract_single_code(trimmed) {
        return Ok(vec![single_code]);
    }

    // =========================
    // Falhou tudo
    // =========================
    Err(ParserError::CodeNotFound)
}

/// Extrai o primeiro array encontrado em um JSON Value
fn extract_first_array(value: &Value) -> Option<Vec<String>> {
    match value {
        Value::Object(map) => {
            // Procura pelo primeiro campo que contenha um array
            for (key, val) in map.iter() {
                if let Some(arr) = val.as_array() {
                    eprintln!("Found array in field: '{}' with {} items", key, arr.len());
                    let mut codes = Vec::new();
                    for item in arr {
                        if let Some(code_str) = item.as_str() {
                            codes.push(unescape(code_str));
                        } else if let Some(obj) = item.as_object() {
                            // Se for objeto, tenta extrair campo "code"
                            if let Some(code) = extract_code_from_object(obj) {
                                codes.push(unescape(&code));
                            }
                        }
                    }
                    if !codes.is_empty() {
                        return Some(codes);
                    }
                }
            }
            
            // Se não encontrou array diretamente, procura recursivamente
            for val in map.values() {
                if let Some(codes) = extract_first_array(val) {
                    return Some(codes);
                }
            }
            None
        }
        Value::Array(arr) => {
            for item in arr {
                if let Some(codes) = extract_first_array(item) {
                    return Some(codes);
                }
            }
            None
        }
        _ => None,
    }
}

/// Extrai strings de um array em formato texto (para fallback de regex)
fn extract_strings_from_array(array_text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let re = regex::Regex::new(r#""((?:[^"\\]|\\.)*)""#).unwrap();
    
    for caps in re.captures_iter(array_text) {
        if let Some(matched) = caps.get(1) {
            let extracted = matched.as_str();
            result.push(unescape(extracted));
        }
    }
    
    result
}

/// Extrai código único (backward compatibility para formato antigo)
fn extract_single_code(raw: &str) -> Result<String, ParserError> {
    // Tenta encontrar campo "code"
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        if let Some(code) = v.get("code").and_then(|c| c.as_str()) {
            return Ok(unescape(code));
        }
    }

    // Fallback regex para campo "code"
    let re = regex::Regex::new(r#""code"\s*:\s*"((?:[^"\\]|\\.)*)""#)
        .map_err(|_| ParserError::InvalidRegex)?;

    if let Some(caps) = re.captures(raw) {
        if let Some(m) = caps.get(1) {
            let extracted = m.as_str();
            return Ok(unescape(extracted));
        }
    }

    // Se não encontrar JSON, assume que o conteúdo todo é o código
    Ok(unescape(raw))
}

/// Extrai código de um objeto JSON (para casos aninhados)
fn extract_code_from_object(obj: &serde_json::Map<String, Value>) -> Option<String> {
    if let Some(Value::String(code)) = obj.get("code") {
        return Some(code.clone());
    }
    
    for value in obj.values() {
        if let Some(obj) = value.as_object() {
            if let Some(code) = extract_code_from_object(obj) {
                return Some(code);
            }
        }
    }
    
    None
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
                    result.push('\\');
                    result.push(other);
                }
                None => {
                    result.push('\\');
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

//
// ==========================
// FUNÇÃO LEGADA (DEPRECATED)
// ==========================
//

#[deprecated(note = "Use extract_codes() instead")]
pub fn extract_code(raw: &str) -> Result<String, ParserError> {
    let codes = extract_codes(raw)?;
    codes.first()
        .cloned()
        .ok_or(ParserError::CodeNotFound)
}

//
// ==========================
// ERROR HANDLING
// ==========================
//

#[derive(Debug)]
pub enum ParserError {
    CodeNotFound,
    InvalidRegex,
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserError::CodeNotFound => write!(f, "No codes found in LLM output"),
            ParserError::InvalidRegex => write!(f, "Invalid regex pattern"),
        }
    }
}

impl Error for ParserError {}