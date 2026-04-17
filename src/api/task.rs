use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;

use crate::dto::{DeleteResponse, TaskPayload};
use crate::model::Task;
use crate::repository::task as repo;

pub async fn get_tasks(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Task>>, StatusCode> {
    let tasks = repo::find_all(&pool)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_tasks): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(tasks))
}

pub async fn get_task(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Json<Task>, StatusCode> {
    repo::find_by_id(&pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_task): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn create_task(
    State(pool): State<PgPool>,
    Json(payload): Json<TaskPayload>,
) -> Result<(StatusCode, Json<Task>), StatusCode> {
    let task = repo::insert(
        &pool,
        payload.name,
        payload.sequence,
        payload.r#type,
        payload.path,
        payload.prompt,
        payload.use_case_id,
    )
    .await
    .map_err(|e| {
        println!("DB ERROR (create_task): {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((StatusCode::CREATED, Json(task)))
}

pub async fn update_task(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(payload): Json<TaskPayload>,
) -> Result<Json<Task>, StatusCode> {
    repo::update(
        &pool,
        id,
        payload.name,
        payload.sequence,
        payload.r#type,
        payload.path,
        payload.prompt,
        payload.use_case_id,
    )
    .await
    .map_err(|e| {
        println!("DB ERROR (update_task): {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .map(Json)
    .ok_or(StatusCode::NOT_FOUND)
}

pub async fn delete_task(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Json<DeleteResponse>, StatusCode> {
    let deleted = repo::delete(&pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (delete_task): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if deleted {
        Ok(Json(DeleteResponse { deleted: true }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}