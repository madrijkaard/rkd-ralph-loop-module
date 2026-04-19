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
// RESPONSE STRUCTS (MODELS) 🔥
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
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to build HTTP client");

        Self { base_url, client }
    }

    /// ==========================
    /// CHAT COMPLETIONS
    /// ==========================
    ///
    /// Retorna apenas o campo `content`
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

        let content = body
            .choices
            .get(0)
            .ok_or(EngineError::EmptyResponse)?
            .message
            .content
            .clone();

        Ok(content)
    }

    /// ==========================
    /// LIST MODELS 🔥
    /// ==========================
    ///
    /// Retorna apenas os IDs dos modelos
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

        let models = body
            .data
            .into_iter()
            .map(|m| m.id)
            .collect();

        Ok(models)
    }
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