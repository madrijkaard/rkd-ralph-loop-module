use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::api::AppState;
use crate::dto::{DeleteResponse, IterationPayload, IterationCreateResponse};
use crate::model::Iteration;
use crate::repository::iteration as repo;

pub async fn get_iterations_by_task(
    State(state): State<AppState>,
    Path(task_id): Path<i32>,
) -> Result<Json<Vec<Iteration>>, StatusCode> {

    let pool = &state.pool;

    let iterations = repo::find_all_by_task_id(pool, task_id)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_iterations_by_task): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(iterations))
}

pub async fn get_iteration(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Iteration>, StatusCode> {

    let pool = &state.pool;

    repo::find_by_id(pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_iteration): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn create_iteration(
    State(state): State<AppState>,
    Json(payload): Json<IterationPayload>,
) -> Result<(StatusCode, Json<IterationCreateResponse>), StatusCode> {

    let pool = &state.pool;

    let iteration = repo::insert(pool, payload.task_id)
        .await
        .map_err(|e| {
            println!("DB ERROR (create_iteration): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok((StatusCode::CREATED, Json(iteration)))
}

pub async fn update_iteration(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<IterationPayload>,
) -> Result<Json<Iteration>, StatusCode> {

    let pool = &state.pool;

    repo::update(pool, id, payload.task_id)
        .await
        .map_err(|e| {
            println!("DB ERROR (update_iteration): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn delete_iteration(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<DeleteResponse>, StatusCode> {

    let pool = &state.pool;

    let deleted = repo::delete(pool, id)
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