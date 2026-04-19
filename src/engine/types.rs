use serde::{Deserialize, Serialize};

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
// Isso representa o JSON interno esperado do LLM:
// {
//   "code": "...."
// }

#[derive(Debug, Deserialize, Serialize)]
pub struct CodeResponse {
    pub code: String,
}