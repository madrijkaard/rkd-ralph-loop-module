use serde::{Deserialize, Serialize};
use std::collections::HashMap;

//
// ==========================
// REQUEST (CHAT COMPLETIONS)
// ==========================
//

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub temperature: f32,
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

//
// ==========================
// RESPONSE (CHAT COMPLETIONS)
// ==========================
//

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
pub struct ResponseMessage {
    pub content: String,
}

//
// ==========================
// RESPONSE (MODELS)
// ==========================
//

#[derive(Debug, Deserialize)]
pub struct ModelsResponse {
    pub data: Vec<Model>,
}

#[derive(Debug, Deserialize)]
pub struct Model {
    pub id: String,
}

//
// ==========================
// DOMAIN RESPONSE (LLM OUTPUT)
// ==========================
//
// NOVO FORMATO: O LLM retorna um objeto com UM ÚNICO atributo,
// cujo valor é um ARRAY de strings contendo os códigos.
//
// Exemplo 1: { "classes": ["public class A {}", "public class B {}"] }
// Exemplo 2: { "code": ["public class Main {}", "public class Helper {}"] }
// Exemplo 3: { "files": ["...", "...", "..."] }
//
// O nome do atributo pode ser QUALQUER UM (classes, code, files, etc.)
// O importante é que o valor seja um array de strings.
//

// Esta struct NÃO é mais usada diretamente, pois o nome do campo é dinâmico.
// Mantida apenas para backward compatibility (será removida em versões futuras)
#[derive(Debug, Deserialize, Serialize)]
#[deprecated(note = "Use LLMResponse enum or dynamic HashMap instead")]
pub struct CodeResponse {
    pub code: String,
}

// Nova representação para respostas dinâmicas do LLM
// Permite qualquer nome de campo, desde que o valor seja um array de strings
#[derive(Debug, Deserialize, Serialize)]
pub struct LLMResponse {
    // Usamos HashMap para capturar qualquer nome de campo
    #[serde(flatten)]
    pub data: HashMap<String, Vec<String>>,
}

impl LLMResponse {
    /// Retorna o primeiro array encontrado no response (ignorando o nome do campo)
    pub fn get_first_array(&self) -> Option<&Vec<String>> {
        self.data.values().next()
    }
    
    /// Retorna todos os códigos do primeiro array encontrado
    pub fn extract_codes(&self) -> Vec<String> {
        self.get_first_array()
            .map(|arr| arr.clone())
            .unwrap_or_else(Vec::new)
    }
    
    /// Verifica se a resposta contém algum array
    pub fn has_array(&self) -> bool {
        !self.data.is_empty() && self.data.values().any(|v| !v.is_empty())
    }
    
    /// Retorna o nome do primeiro campo encontrado
    pub fn get_first_field_name(&self) -> Option<&String> {
        self.data.keys().next()
    }
}

// Implementação de Default para facilitar criação
impl Default for LLMResponse {
    fn default() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}

// Para casos onde a resposta do LLM é apenas uma string (código único)
#[derive(Debug, Deserialize, Serialize)]
pub struct SingleCodeResponse {
    pub code: String,
}

// Enum unificada para representar todos os possíveis formatos de resposta
#[derive(Debug)]
pub enum UnifiedResponse {
    /// Múltiplos códigos em array
    Multiple(Vec<String>),
    /// Código único (string)
    Single(String),
    /// Resposta vazia ou inválida
    Empty,
}

impl UnifiedResponse {
    /// Extrai os códigos para um Vec<String>
    pub fn into_codes(self) -> Vec<String> {
        match self {
            UnifiedResponse::Multiple(codes) => codes,
            UnifiedResponse::Single(code) => vec![code],
            UnifiedResponse::Empty => Vec::new(),
        }
    }
    
    /// Retorna a quantidade de códigos
    pub fn len(&self) -> usize {
        match self {
            UnifiedResponse::Multiple(codes) => codes.len(),
            UnifiedResponse::Single(_) => 1,
            UnifiedResponse::Empty => 0,
        }
    }
    
    /// Verifica se está vazio
    pub fn is_empty(&self) -> bool {
        matches!(self, UnifiedResponse::Empty) || self.len() == 0
    }
}