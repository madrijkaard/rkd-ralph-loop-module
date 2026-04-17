use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;

use crate::dto::{DeleteResponse, IterationPayload};
use crate::model::Iteration;
use crate::repository::iteration as repo;

pub async fn get_iterations(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Iteration>>, StatusCode> {
    let iterations = repo::find_all(&pool)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_iterations): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(iterations))
}

pub async fn get_iteration(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Json<Iteration>, StatusCode> {
    repo::find_by_id(&pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_iteration): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn create_iteration(
    State(pool): State<PgPool>,
    Json(payload): Json<IterationPayload>,
) -> Result<(StatusCode, Json<Iteration>), StatusCode> {
    let iteration = repo::insert(&pool, payload.task_id)
        .await
        .map_err(|e| {
            println!("DB ERROR (create_iteration): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok((StatusCode::CREATED, Json(iteration)))
}

pub async fn update_iteration(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(payload): Json<IterationPayload>,
) -> Result<Json<Iteration>, StatusCode> {
    repo::update(&pool, id, payload.task_id)
        .await
        .map_err(|e| {
            println!("DB ERROR (update_iteration): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn delete_iteration(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Json<DeleteResponse>, StatusCode> {
    let deleted = repo::delete(&pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (delete_iteration): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if deleted {
        Ok(Json(DeleteResponse { deleted: true }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}