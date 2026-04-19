use serde::{Deserialize, Serialize};
use std::time::Duration;

//
// ==========================
// REQUEST STRUCTS
// ==========================
//

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    temperature: f32,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

//
// ==========================
// RESPONSE STRUCTS (CHAT)
// ==========================
//

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

//
// ==========================
// RESPONSE STRUCTS (MODELS)
// ==========================
//

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<Model>,
}

#[derive(Deserialize)]
struct Model {
    id: String,
}

//
// ==========================
// ENGINE CLIENT
// ==========================
//

pub struct EngineClient {
    base_url: String,
    client: reqwest::Client,
}

impl EngineClient {
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");

        Self { base_url, client }
    }

    /// ==========================
    /// CHAT COMPLETIONS (FIXED)
    /// ==========================
    pub async fn generate(
        &self,
        system_content: String,
        user_content: String,
        model: String,
    ) -> Result<String, EngineError> {

        let url = format!("{}/v1/chat/completions", self.base_url);

        let request = ChatRequest {
            model,
            temperature: 0.0,
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_content,
                },
                Message {
                    role: "user".to_string(),
                    content: user_content,
                },
            ],
        };

        let response = self
            .client
            .post(url)
            .json(&request)
            .send()
            .await
            .map_err(EngineError::Http)?
            .error_for_status()
            .map_err(EngineError::Http)?;

        let body: ChatResponse = response
            .json()
            .await
            .map_err(EngineError::Parse)?;

        // 1. pega resposta bruta do LLM
        let raw_content = body
            .choices
            .get(0)
            .ok_or(EngineError::EmptyResponse)?
            .message
            .content
            .clone();

        eprintln!("=== RAW LLM RESPONSE ===");
        eprintln!("{}", raw_content);
        eprintln!("=== END RAW RESPONSE ===");

        // 2. Extrai e formata o código
        let code = extract_code_safely(&raw_content);

        eprintln!("=== FINAL CODE LENGTH: {} ===", code.len());
        if code.len() > 0 {
            let preview_start = &code[..code.len().min(200)];
            let preview_end = &code[code.len().saturating_sub(200)..];
            eprintln!("First 200 chars:\n{}", preview_start);
            eprintln!("...");
            eprintln!("Last 200 chars:\n{}", preview_end);
        }

        Ok(code)
    }

    /// ==========================
    /// LIST MODELS
    /// ==========================
    pub async fn list_models(&self) -> Result<Vec<String>, EngineError> {

        let url = format!("{}/v1/models", self.base_url);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(EngineError::Http)?
            .error_for_status()
            .map_err(EngineError::Http)?;

        let body: ModelsResponse = response
            .json()
            .await
            .map_err(EngineError::Parse)?;

        Ok(body.data.into_iter().map(|m| m.id).collect())
    }
}

//
// ==========================
// SAFE EXTRACTION (ULTRA ROBUST VERSION)
// ==========================
//

/// Extrai e limpa o código do LLM de forma segura
fn extract_code_safely(content: &str) -> String {
    let trimmed = content.trim();
    
    // TENTATIVA 1: Parse JSON com serde_json (método mais confiável)
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(code) = find_code_field(&value) {
            eprintln!("=== EXTRACTED VIA SERDE_JSON ===");
            return clean_final_code(code);
        }
    }
    
    // TENTATIVA 2: Extração manual do campo "code"
    if let Some(code) = manual_extract_code(trimmed) {
        eprintln!("=== EXTRACTED VIA MANUAL PARSING ===");
        return clean_final_code(code);
    }
    
    // TENTATIVA 3: Regex simples para capturar o conteúdo de "code"
    let re = regex::Regex::new(r#""code"\s*:\s*"((?:[^"\\]|\\.)*)""#).unwrap();
    if let Some(caps) = re.captures(trimmed) {
        if let Some(matched) = caps.get(1) {
            let extracted = matched.as_str();
            eprintln!("=== EXTRACTED WITH SIMPLE REGEX ===");
            
            let unescaped = agressive_unescape(extracted);
            if looks_like_code(&unescaped) {
                return clean_final_code(unescaped);
            }
        }
    }
    
    // TENTATIVA 4: Procura por padrão de código Java/XML/Shell
    if let Some(code) = extract_code_by_heuristic(trimmed) {
        eprintln!("=== EXTRACTED VIA HEURISTIC ===");
        return clean_final_code(code);
    }
    
    // FALLBACK: Aplica unescape em tudo
    eprintln!("=== USING FALLBACK UNESCAPE ===");
    clean_final_code(agressive_unescape(trimmed))
}

/// Limpeza final do código extraído
fn clean_final_code(mut code: String) -> String {
    // Remove artefatos JSON do final: }", "}, }\n", etc.
    while code.ends_with("}\"") || code.ends_with("\"}") || code.ends_with("}\n\"") || code.ends_with("}\r\n\"") {
        if code.ends_with("}\r\n\"") {
            code.pop(); // remove "
            code.pop(); // remove \n
            code.pop(); // remove \r
            code.pop(); // remove }
        } else if code.ends_with("}\n\"") {
            code.pop(); // remove "
            code.pop(); // remove \n
            code.pop(); // remove }
        } else if code.ends_with("}\"") {
            code.pop(); // remove "
            code.pop(); // remove }
        } else if code.ends_with("\"}") {
            code.pop(); // remove }
            code.pop(); // remove "
        }
    }
    
    // Remove aspas duplas no final se existirem e não fizerem parte do código
    while code.ends_with('"') {
        let quote_count = code.matches('"').count();
        let brace_count = code.matches('{').count();
        // Só remove se houver mais aspas que chaves (indicando aspas extras do JSON)
        if quote_count > brace_count * 2 {
            code.pop();
        } else {
            break;
        }
    }
    
    // Remove quebras de linha e espaços extras no final
    while code.ends_with('\n') || code.ends_with('\r') || code.ends_with(' ') {
        code.pop();
    }
    
    // Garante que o arquivo termina com uma quebra de linha
    if !code.ends_with('\n') {
        code.push('\n');
    }
    
    code
}

/// Extração manual do campo "code" sem usar regex complexo
fn manual_extract_code(content: &str) -> Option<String> {
    // Encontra a posição de "code":
    let code_field_start = content.find("\"code\"")?;
    
    // Encontra os dois pontos depois de "code"
    let after_code = &content[code_field_start..];
    let colon_pos = after_code.find(':')?;
    
    // Encontra a primeira aspa depois dos dois pontos
    let after_colon = &after_code[colon_pos + 1..];
    let first_quote = after_colon.find('"')?;
    
    // O conteúdo começa depois da primeira aspa
    let content_start = code_field_start + colon_pos + 1 + first_quote + 1;
    let mut chars = content[content_start..].chars();
    
    let mut result = String::new();
    let mut escaped = false;
    let mut brace_depth = 0;
    let mut in_string = false;
    let mut char_count = 0;
    
    while let Some(c) = chars.next() {
        char_count += 1;
        
        if escaped {
            result.push(c);
            escaped = false;
            continue;
        }
        
        if c == '\\' {
            escaped = true;
            result.push(c);
            continue;
        }
        
        // Rastreamento de strings e chaves para saber quando o JSON termina
        if c == '"' && !escaped {
            in_string = !in_string;
        }
        
        if !in_string {
            if c == '{' {
                brace_depth += 1;
            } else if c == '}' {
                brace_depth -= 1;
                // Se voltamos ao nível 0, o objeto JSON principal terminou
                if brace_depth == 0 && char_count > 10 {
                    // Não adiciona a chave final
                    break;
                }
            }
        }
        
        result.push(c);
        
        // Se encontramos uma aspa não escapada e já temos conteúdo suficiente
        // e não estamos dentro de uma string, pode ser o fim do valor "code"
        if c == '"' && !escaped && !in_string && brace_depth == 0 && result.len() > 50 {
            // Remove a última aspa
            result.pop();
            break;
        }
    }
    
    if !result.is_empty() {
        let unescaped = agressive_unescape(&result);
        if looks_like_code(&unescaped) {
            Some(unescaped)
        } else {
            Some(agressive_unescape(&result))
        }
    } else {
        None
    }
}

/// Unescape agressivo que resolve múltiplas camadas
fn agressive_unescape(s: &str) -> String {
    let mut result = s.to_string();
    let mut changed = true;
    let max_iterations = 10;
    let mut iteration = 0;
    
    while changed && iteration < max_iterations {
        changed = false;
        iteration += 1;
        
        let mut new_result = String::with_capacity(result.len());
        let mut chars = result.chars();
        
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => {
                        new_result.push('\n');
                        changed = true;
                    }
                    Some('r') => {
                        new_result.push('\r');
                        changed = true;
                    }
                    Some('t') => {
                        new_result.push('\t');
                        changed = true;
                    }
                    Some('"') => {
                        new_result.push('"');
                        changed = true;
                    }
                    Some('\\') => {
                        new_result.push('\\');
                        changed = true;
                    }
                    Some('\'') => {
                        new_result.push('\'');
                        changed = true;
                    }
                    Some('/') => {
                        new_result.push('/');
                        changed = true;
                    }
                    Some('b') => {
                        new_result.push('\x08');
                        changed = true;
                    }
                    Some('f') => {
                        new_result.push('\x0C');
                        changed = true;
                    }
                    Some('u') => {
                        let mut hex = String::new();
                        for _ in 0..4 {
                            if let Some(h) = chars.next() {
                                hex.push(h);
                            }
                        }
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = char::from_u32(code) {
                                new_result.push(ch);
                            } else {
                                new_result.push_str(&format!("\\u{}", hex));
                            }
                        } else {
                            new_result.push_str(&format!("\\u{}", hex));
                        }
                        changed = true;
                    }
                    Some(other) => {
                        new_result.push('\\');
                        new_result.push(other);
                    }
                    None => {
                        new_result.push('\\');
                        break;
                    }
                }
            } else {
                new_result.push(c);
            }
        }
        
        result = new_result;
    }
    
    result
}

/// Verifica se o conteúdo parece código válido
fn looks_like_code(content: &str) -> bool {
    let content = content.trim();
    
    // Java
    if content.contains("package ") || 
       content.contains("import java") || 
       content.contains("public class ") ||
       content.contains("private class ") ||
       content.contains("public interface ") {
        return true;
    }
    
    // XML
    if content.starts_with("<?xml") || 
       (content.starts_with("<") && content.contains("</") && content.ends_with(">")) {
        return true;
    }
    
    // Shell
    if content.starts_with("#!/bin/") {
        return true;
    }
    
    false
}

/// Procura recursivamente pelo campo "code" em um Value
fn find_code_field(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            // Procura diretamente pelo campo "code"
            if let Some(serde_json::Value::String(code)) = map.get("code") {
                eprintln!("Found 'code' field directly");
                return Some(agressive_unescape(code));
            }
            
            // Procura recursivamente em objetos aninhados
            for (key, v) in map.iter() {
                if let Some(code) = find_code_field(v) {
                    eprintln!("Found 'code' field inside nested object '{}'", key);
                    return Some(code);
                }
            }
            None
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                if let Some(code) = find_code_field(v) {
                    eprintln!("Found 'code' field inside array at index {}", i);
                    return Some(code);
                }
            }
            None
        }
        serde_json::Value::String(s) => {
            // Tenta parsear a string como JSON
            if let Ok(inner) = serde_json::from_str::<serde_json::Value>(s) {
                find_code_field(&inner)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Extrai código baseado em heurísticas
fn extract_code_by_heuristic(content: &str) -> Option<String> {
    let content = content.trim();
    
    // Procura por "package " ou "import " - início típico de arquivo Java
    if let Some(idx) = content.find("package ") {
        return Some(content[idx..].to_string());
    }
    
    if let Some(idx) = content.find("import java") {
        return Some(content[idx..].to_string());
    }
    
    if let Some(idx) = content.find("public class ") {
        return Some(content[idx..].to_string());
    }
    
    // XML
    if let Some(idx) = content.find("<?xml") {
        return Some(content[idx..].to_string());
    }
    
    // Shell script
    if let Some(idx) = content.find("#!/bin/") {
        return Some(content[idx..].to_string());
    }
    
    None
}

//
// ==========================
// ERROR HANDLING
// ==========================
//

#[derive(Debug)]
pub enum EngineError {
    Http(reqwest::Error),
    Parse(reqwest::Error),
    EmptyResponse,
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Http(e) => write!(f, "HTTP error: {}", e),
            EngineError::Parse(e) => write!(f, "Parse error: {}", e),
            EngineError::EmptyResponse => write!(f, "Empty response from engine"),
        }
    }
}

impl std::error::Error for EngineError {}