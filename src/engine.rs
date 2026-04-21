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
            .timeout(Duration::from_secs(240))
            .build()
            .expect("Failed to build HTTP client");

        Self { base_url, client }
    }

    /// ==========================
    /// CHAT COMPLETIONS (UPDATED)
    /// Returns Vec<String> with all extracted codes
    /// ==========================
    pub async fn generate(
        &self,
        system_content: String,
        user_content: String,
        model: String,
    ) -> Result<Vec<String>, EngineError> {

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

        // 2. Extrai todos os códigos do JSON
        let codes = extract_codes_from_json(&raw_content);

        eprintln!("=== EXTRACTED {} CODE(S) ===", codes.len());
        for (i, code) in codes.iter().enumerate() {
            eprintln!("Code {} - length: {}", i+1, code.len());
            if code.len() > 0 {
                let preview_start = &code[..code.len().min(200)];
                eprintln!("Code {} preview: {}...", i+1, preview_start);
            }
        }

        Ok(codes)
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
// DIRECT CODE EXTRACTION (MOST ROBUST)
// ==========================
//

/// Extrai códigos diretamente do JSON sem fazer parse completo
/// Esta é a abordagem mais robusta para JSON malformado
fn extract_codes_directly(content: &str) -> Vec<String> {
    // Procura pelo padrão "code": [ ... ]
    let code_pattern = r#""code"\s*:\s*\["#;
    let re = match regex::Regex::new(code_pattern) {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };
    
    if let Some(matched) = re.find(content) {
        let start_pos = matched.end();
        
        // Encontra o fechamento do array
        let mut bracket_count = 1;
        let mut end_pos = start_pos;
        let chars: Vec<char> = content.chars().collect();
        
        for i in start_pos..chars.len() {
            match chars[i] {
                '[' => bracket_count += 1,
                ']' => {
                    bracket_count -= 1;
                    if bracket_count == 0 {
                        end_pos = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        
        if end_pos > start_pos {
            let array_content = &content[start_pos..end_pos];
            let all_strings = extract_all_json_strings(array_content);
            
            // Primeiro pega o tamanho, DEPOIS faz o filtro
            let total_count = all_strings.len();
            
            let valid_codes: Vec<String> = all_strings
                .into_iter()
                .filter(|s| {
                    s.contains("package ") || 
                    (s.contains("public class") && s.contains("class ")) ||
                    (s.contains("import ") && s.contains(";")) ||
                    s.contains("public interface") ||
                    s.contains("public enum")
                })
                .collect();
            
            eprintln!("Filtered from {} to {} valid Java codes", total_count, valid_codes.len());
            return valid_codes;
        }
    }
    
    Vec::new()
}

/// Extrai TODAS as strings de um conteúdo que deveria ser um array JSON
/// Versão melhorada que não quebra strings com aspas internas
fn extract_all_json_strings(array_content: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escape_next = false;
    let chars: Vec<char> = array_content.chars().collect();
    let mut i = 0;
    let mut string_count = 0;
    
    while i < chars.len() {
        let c = chars[i];
        
        if escape_next {
            current.push(c);
            escape_next = false;
            i += 1;
            continue;
        }
        
        match c {
            '\\' => {
                current.push(c);
                escape_next = true;
                i += 1;
            }
            '"' => {
                if !in_string {
                    // Início da string
                    in_string = true;
                    current.clear();
                    string_count += 1;
                    eprintln!("Starting string #{} at position {}", string_count, i);
                } else {
                    // Fim da string - verifica se não é aspa escapada
                    // Olha para trás para ver se a aspa não é escapada
                    let mut j = i as i32 - 1;
                    let mut backslash_count = 0;
                    while j >= 0 && chars[j as usize] == '\\' {
                        backslash_count += 1;
                        j -= 1;
                    }
                    
                    // Se o número de backslashes for par, é fim de string
                    if backslash_count % 2 == 0 {
                        in_string = false;
                        if !current.is_empty() {
                            eprintln!("Ending string #{} - length: {}", string_count, current.len());
                            result.push(current.clone());
                        }
                        current.clear();
                    } else {
                        // É aspa escapada, continua
                        current.push(c);
                    }
                }
                i += 1;
            }
            _ => {
                if in_string {
                    current.push(c);
                }
                i += 1;
            }
        }
    }
    
    eprintln!("Total raw strings extracted: {}", result.len());
    
    // Agora aplica unescape e limpeza em cada string
    let mut cleaned_result = Vec::new();
    for (idx, s) in result.iter().enumerate() {
        eprintln!("Cleaning string #{} - raw length: {}", idx + 1, s.len());
        let unescaped = agressive_unescape(s);
        let cleaned = clean_final_code(unescaped);
        
        // Verifica se parece código Java válido
        let is_java = cleaned.contains("public class") || 
                      cleaned.contains("package ") ||
                      (cleaned.contains("import ") && cleaned.contains(";"));
        
        if is_java && cleaned.len() > 100 {
            eprintln!("String #{} is valid Java code (length: {})", idx + 1, cleaned.len());
            cleaned_result.push(cleaned);
        } else {
            eprintln!("String #{} skipped (not Java code or too short)", idx + 1);
        }
    }
    
    cleaned_result
}

//
// ==========================
// JSON FIXING FUNCTIONS
// ==========================
//

/// Tenta corrigir JSON malformado usando contagem de colchetes
fn try_fix_malformed_json(content: &str) -> String {
    // Se o JSON já é válido, retorna ele mesmo
    if serde_json::from_str::<serde_json::Value>(content).is_ok() {
        return content.to_string();
    }
    
    eprintln!("Attempting to fix malformed JSON...");
    
    // Procura pela posição do "code": [
    let code_pattern = r#""code"\s*:\s*\["#;
    let re_code_start = match regex::Regex::new(code_pattern) {
        Ok(re) => re,
        Err(_) => return content.to_string(),
    };
    
    if let Some(matched) = re_code_start.find(content) {
        let start_pos = matched.end();
        
        // Conta colchetes para achar o final do array
        let mut bracket_count = 1;
        let mut end_pos = start_pos;
        let chars: Vec<char> = content.chars().collect();
        
        for i in start_pos..chars.len() {
            match chars[i] {
                '[' => bracket_count += 1,
                ']' => {
                    bracket_count -= 1;
                    if bracket_count == 0 {
                        end_pos = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        
        if end_pos > start_pos {
            let array_content = &content[start_pos..end_pos];
            let strings = extract_all_json_strings(array_content);
            
            if !strings.is_empty() {
                let fixed = serde_json::json!({ "code": strings });
                eprintln!("✅ Fixed JSON with {} strings", strings.len());
                return fixed.to_string();
            }
        }
    }
    
    content.to_string()
}

//
// ==========================
// EXTRACT CODES FROM JSON (ANY ATTRIBUTE NAME)
// ==========================
//

/// Extrai todos os códigos do JSON do LLM
/// Espera um objeto com UM ÚNICO atributo, cujo valor é um array de strings
/// Exemplo: { "classes": ["code1", "code2", "code3"] }
fn extract_codes_from_json(content: &str) -> Vec<String> {
    let trimmed = content.trim();
    
    // 🔥 TENTATIVA 1: Extração direta (mais robusta para JSON malformado)
    eprintln!("=== TRYING DIRECT ARRAY EXTRACTION ===");
    let direct_codes = extract_codes_directly(trimmed);
    if !direct_codes.is_empty() {
        eprintln!("=== EXTRACTED {} CODE(S) VIA DIRECT EXTRACTION ===", direct_codes.len());
        return direct_codes;
    }
    
    // TENTATIVA 2: Tenta corrigir JSON malformado e parsear
    let fixed_content = try_fix_malformed_json(trimmed);
    match serde_json::from_str::<serde_json::Value>(&fixed_content) {
        Ok(value) => {
            if let Some(codes) = extract_first_attribute_array(&value) {
                eprintln!("=== EXTRACTED {} CODE(S) VIA SERDE_JSON ===", codes.len());
                return codes;
            }
        }
        Err(e) => {
            eprintln!("WARN: serde_json parse failed: {}", e);
            eprintln!("First 300 chars of content: {}", &trimmed[..trimmed.len().min(300)]);
        }
    }
    
    // TENTATIVA 3: Fallback - tenta extrair array via regex
    if let Some(codes) = manual_extract_array(trimmed) {
        eprintln!("=== EXTRACTED {} CODE(S) VIA MANUAL PARSING ===", codes.len());
        return codes;
    }
    
    // TENTATIVA 4: Extrair código de blocos markdown ```java ... ```
    let markdown_codes = extract_markdown_code_blocks(trimmed);
    if !markdown_codes.is_empty() {
        eprintln!("=== EXTRACTED {} CODE(S) VIA MARKDOWN PARSING ===", markdown_codes.len());
        return markdown_codes;
    }
    
    // TENTATIVA 5: Se não conseguir extrair array, tenta como código único (backward compatibility)
    eprintln!("=== NO ARRAY FOUND, TRYING SINGLE CODE FALLBACK ===");
    let single_code = extract_single_code_fallback(trimmed);
    if !single_code.is_empty() {
        vec![single_code]
    } else {
        Vec::new()
    }
}

/// Pega o primeiro atributo do objeto JSON e espera que seja um array de strings
fn extract_first_attribute_array(value: &serde_json::Value) -> Option<Vec<String>> {
    match value {
        serde_json::Value::Object(map) => {
            // Pega o primeiro atributo (qualquer nome)
            if let Some((key, array_value)) = map.iter().next() {
                eprintln!("Found first attribute: '{}'", key);
                
                // Verifica se é um array
                if let Some(arr) = array_value.as_array() {
                    let mut codes = Vec::new();
                    for (i, item) in arr.iter().enumerate() {
                        // TENTATIVA 1: item é string direta
                        if let Some(code_str) = item.as_str() {
                            eprintln!("Extracting direct string code {} from array", i+1);
                            let cleaned = clean_final_code(agressive_unescape(code_str));
                            codes.push(cleaned);
                        }
                        // TENTATIVA 2: item é objeto com campo "codigo"
                        else if let Some(obj) = item.as_object() {
                            if let Some(code_str) = obj.get("codigo").and_then(|v| v.as_str()) {
                                eprintln!("Extracting 'codigo' field from object {} in array", i+1);
                                let cleaned = clean_final_code(agressive_unescape(code_str));
                                codes.push(cleaned);
                            } else if let Some(code_str) = obj.get("code").and_then(|v| v.as_str()) {
                                eprintln!("Extracting 'code' field from object {} in array", i+1);
                                let cleaned = clean_final_code(agressive_unescape(code_str));
                                codes.push(cleaned);
                            }
                        }
                    }
                    
                    if !codes.is_empty() {
                        return Some(codes);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Extrai array via regex quando o JSON está mal formatado (sem look-ahead)
fn manual_extract_array(content: &str) -> Option<Vec<String>> {
    // Regex sem look-ahead (suportado no Rust)
    let re = match regex::Regex::new(r#""(\w+)"\s*:\s*\[(.*?)\]"#) {
        Ok(re) => re,
        Err(e) => {
            eprintln!("ERROR: Failed to compile regex: {}", e);
            return None;
        }
    };
    
    if let Some(caps) = re.captures(content) {
        if let Some(array_content) = caps.get(2) {
            let items = extract_strings_from_array(array_content.as_str());
            if !items.is_empty() {
                eprintln!("Manual array extraction found {} items", items.len());
                return Some(items);
            }
        }
    }
    
    None
}

/// Extrai strings de um array em formato texto
fn extract_strings_from_array(array_text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let re = match regex::Regex::new(r#""((?:[^"\\]|\\.)*)""#) {
        Ok(re) => re,
        Err(e) => {
            eprintln!("ERROR: Failed to compile string extraction regex: {}", e);
            return result;
        }
    };
    
    for caps in re.captures_iter(array_text) {
        if let Some(matched) = caps.get(1) {
            let extracted = matched.as_str();
            let unescaped = agressive_unescape(extracted);
            result.push(clean_final_code(unescaped));
        }
    }
    
    result
}

/// Extrai código de blocos markdown como ```java ... ```
fn extract_markdown_code_blocks(content: &str) -> Vec<String> {
    let re = match regex::Regex::new(r"```(?:java)?\s*\n(.*?)```") {
        Ok(re) => re,
        Err(e) => {
            eprintln!("ERROR: Failed to compile markdown regex: {}", e);
            return Vec::new();
        }
    };
    
    let mut codes = Vec::new();
    
    for caps in re.captures_iter(content) {
        if let Some(code) = caps.get(1) {
            let extracted = code.as_str().trim().to_string();
            if !extracted.is_empty() && (extracted.contains("class ") || extracted.contains("package ")) {
                eprintln!("Found Java code block in markdown");
                let cleaned = clean_final_code(agressive_unescape(&extracted));
                codes.push(cleaned);
            }
        }
    }
    
    codes
}

/// Fallback para extrair código único (comportamento antigo)
fn extract_single_code_fallback(content: &str) -> String {
    // Tenta encontrar qualquer campo "code"
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
        if let Some(code) = find_code_field_recursive(&value) {
            return clean_final_code(code);
        }
    }
    
    // Tenta heuristicas
    if let Some(code) = extract_code_by_heuristic(content) {
        return clean_final_code(code);
    }
    
    // Último fallback
    clean_final_code(agressive_unescape(content))
}

/// Procura recursivamente pelo campo "code" (backward compatibility)
fn find_code_field_recursive(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(code)) = map.get("code") {
                eprintln!("Found 'code' field (backward compatibility)");
                return Some(agressive_unescape(code));
            }
            
            for v in map.values() {
                if let Some(code) = find_code_field_recursive(v) {
                    return Some(code);
                }
            }
            None
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                if let Some(code) = find_code_field_recursive(v) {
                    return Some(code);
                }
            }
            None
        }
        serde_json::Value::String(s) => {
            if let Ok(inner) = serde_json::from_str::<serde_json::Value>(s) {
                find_code_field_recursive(&inner)
            } else {
                None
            }
        }
        _ => None,
    }
}

//
// ==========================
// CODE CLEANING FUNCTIONS
// ==========================
//

/// Limpeza final do código extraído - VERSÃO SIMPLIFICADA (NÃO CORTA NADA)
fn clean_final_code(code: String) -> String {
    eprintln!("=== CLEANING CODE (minimal) ===");
    eprintln!("Original length: {}", code.len());
    
    let debug_len = code.len();
    let start_idx = if debug_len > 100 { debug_len - 100 } else { 0 };
    eprintln!("Last 100 chars before: \"{}\"", &code[start_idx..].escape_debug());
    
    // 🔥 NÃO REMOVE NADA DO CONTEÚDO - apenas sanitiza se necessário
    
    // Remove apenas se houver um trailing quote NO FINAL DO ARQUIVO
    let mut result = code;
    
    // Remove apenas uma aspa no final se existir
    if result.ends_with('"') {
        result.pop();
        eprintln!("Removed single trailing quote");
    }
    
    // Garante que termina com newline
    if !result.ends_with('\n') {
        result.push('\n');
        eprintln!("Added trailing newline");
    }
    
    eprintln!("After cleaning - length: {}", result.len());
    let after_len = result.len();
    let start_idx_after = if after_len > 100 { after_len - 100 } else { 0 };
    eprintln!("Last 100 chars after: \"{}\"", &result[start_idx_after..].escape_debug());
    eprintln!("=== END CLEANING ===");
    
    result
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

/// Extrai código baseado em heurísticas
fn extract_code_by_heuristic(content: &str) -> Option<String> {
    let content = content.trim();
    
    if let Some(idx) = content.find("package ") {
        return Some(content[idx..].to_string());
    }
    
    if let Some(idx) = content.find("import java") {
        return Some(content[idx..].to_string());
    }
    
    if let Some(idx) = content.find("public class ") {
        return Some(content[idx..].to_string());
    }
    
    if let Some(idx) = content.find("<?xml") {
        return Some(content[idx..].to_string());
    }
    
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