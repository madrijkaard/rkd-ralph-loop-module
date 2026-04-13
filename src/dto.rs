use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ProjectPayload {
    pub name: String,
}

#[derive(Deserialize)]
pub struct UseCasePayload {
    pub name: String,
    pub prompt: String,
    pub project_id: i32,
}

#[derive(Deserialize)]
pub struct TaskPayload {
    pub name: String,
    pub sequence: i32,
    pub r#type: String,
    pub path: String,
    pub prompt: String,
    pub use_case_id: i32,
}

#[derive(Deserialize)]
pub struct IterationPayload {
    pub task_id: i32,
}

#[derive(Serialize)]
pub struct DeleteResponse {
    pub deleted: bool,
}