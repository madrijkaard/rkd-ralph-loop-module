use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;

use crate::dto::{DeleteResponse, ProjectPayload, ProjectCreateResponse};
use crate::model::Project;
use crate::repository::project as repo;

pub async fn get_projects(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Project>>, StatusCode> {
    let projects = repo::find_all(&pool)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_projects): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(projects))
}

pub async fn get_project(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Json<Project>, StatusCode> {
    repo::find_by_id(&pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_project): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn create_project(
    State(pool): State<PgPool>,
    Json(payload): Json<ProjectPayload>,
) -> Result<(StatusCode, Json<ProjectCreateResponse>), StatusCode> {
    let project = repo::insert(&pool, payload.name)
        .await
        .map_err(|e| {
            println!("DB ERROR (create_project): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok((StatusCode::CREATED, Json(project)))
}

pub async fn update_project(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(payload): Json<ProjectPayload>,
) -> Result<Json<Project>, StatusCode> {
    repo::update(&pool, id, payload.name)
        .await
        .map_err(|e| {
            println!("DB ERROR (update_project): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn delete_project(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> Result<Json<DeleteResponse>, StatusCode> {
    let deleted = repo::delete(&pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (delete_project): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if deleted {
        Ok(Json(DeleteResponse { deleted: true }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}