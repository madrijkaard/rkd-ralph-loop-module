use serde::{Serialize, Deserialize};
use chrono::NaiveDateTime;

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Project {
    pub id: i32,
    pub name: String,
    pub created_date: NaiveDateTime,
    pub last_modified_date: NaiveDateTime,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct UseCase {
    pub id: i32,
    pub name: String,
    pub specification: String,
    pub created_date: NaiveDateTime,
    pub last_modified_date: NaiveDateTime,
    pub project_id: i32,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: i32,
    pub name: String,
    pub sequence: i32,
    pub r#type: String,
    pub path: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub created_date: NaiveDateTime,
    pub last_modified_date: NaiveDateTime,
    pub use_case_id: i32,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Iteration {
    pub id: i32,
    pub created_date: NaiveDateTime,
    pub last_modified_date: NaiveDateTime,
    pub task_id: i32,
}