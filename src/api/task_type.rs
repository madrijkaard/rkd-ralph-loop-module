use axum::{http::StatusCode, Json};

use crate::enumerator::TaskType;

pub async fn get_task_types() -> Result<Json<Vec<&'static str>>, StatusCode> {
    Ok(Json(TaskType::values()))
}