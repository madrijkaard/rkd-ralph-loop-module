use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[derive(Deserialize)]
pub struct ProjectPayload {
    pub name: String,
}

#[derive(Deserialize)]
pub struct UseCasePayload {
    pub name: String,
    pub specification: String,
    pub project_id: i32,
}

#[derive(Deserialize)]
pub struct TaskPayload {
    pub name: String,
    pub r#type: String,
    pub path: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub use_case_id: i32,
}

#[derive(Deserialize)]
pub struct IterationPayload {
    pub task_id: i32,
}

//
// ==========================
// EXECUTE TASK 🔥
// ==========================
//

#[derive(Deserialize)]
pub struct ExecuteTaskPayload {
    pub model: String,
}

//
// ==========================
// GENERIC RESPONSES
// ==========================
//

#[derive(Serialize)]
pub struct DeleteResponse {
    pub deleted: bool,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

//
// ==========================
// CREATE RESPONSES
// ==========================
//

#[derive(Serialize, sqlx::FromRow)]
pub struct ProjectCreateResponse {
    pub id: i32,
    pub name: String,
    pub created_date: NaiveDateTime,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct UseCaseCreateResponse {
    pub id: i32,
    pub name: String,
    pub specification: String,
    pub created_date: NaiveDateTime,
    pub project_id: i32,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct TaskCreateResponse {
    pub id: i32,
    pub name: String,
    pub sequence: i32,
    pub r#type: String,
    pub path: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub created_date: NaiveDateTime,
    pub use_case_id: i32,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct IterationCreateResponse {
    pub id: i32,
    pub created_date: NaiveDateTime,
    pub task_id: i32,
}