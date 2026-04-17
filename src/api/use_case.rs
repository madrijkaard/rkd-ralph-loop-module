use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;

use crate::dto::{DeleteResponse, UseCasePayload, UseCaseCreateResponse};
use crate::model::UseCase;
use crate::repository::use_case as repo;

pub async fn get_use_cases_by_project(
    State(pool): State<PgPool>,
    Path(project_id): Path<i32>,
) -> Result<Json<Vec<UseCase>>, StatusCode> {
    let use_cases = repo::find_all_by_project_id(&pool, project_id)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_use_cases_by_project): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(use_cases))
}

pub async fn get_use_case(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Json<UseCase>, StatusCode> {
    repo::find_by_id(&pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_use_case): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn create_use_case(
    State(pool): State<PgPool>,
    Json(payload): Json<UseCasePayload>,
) -> Result<(StatusCode, Json<UseCaseCreateResponse>), StatusCode> {
    let use_case = repo::insert(
        &pool,
        payload.name,
        payload.prompt,
        payload.project_id,
    )
    .await
    .map_err(|e| {
        println!("DB ERROR (create_use_case): {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((StatusCode::CREATED, Json(use_case)))
}

pub async fn update_use_case(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(payload): Json<UseCasePayload>,
) -> Result<Json<UseCase>, StatusCode> {
    repo::update(
        &pool,
        id,
        payload.name,
        payload.prompt,
        payload.project_id,
    )
    .await
    .map_err(|e| {
        println!("DB ERROR (update_use_case): {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .map(Json)
    .ok_or(StatusCode::NOT_FOUND)
}

pub async fn delete_use_case(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Json<DeleteResponse>, StatusCode> {
    let deleted = repo::delete(&pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (delete_use_case): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if deleted {
        Ok(Json(DeleteResponse { deleted: true }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}