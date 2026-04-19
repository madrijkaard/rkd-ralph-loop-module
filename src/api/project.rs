use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::api::AppState;
use crate::dto::{DeleteResponse, ProjectPayload, ProjectCreateResponse, ErrorResponse};
use crate::model::Project;
use crate::repository::project as repo;
use crate::repository::use_case;

pub async fn get_projects(
    State(state): State<AppState>,
) -> Result<Json<Vec<Project>>, StatusCode> {

    let pool = &state.pool;

    let projects = repo::find_all(pool)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_projects): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(projects))
}

pub async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Project>, StatusCode> {

    let pool = &state.pool;

    repo::find_by_id(pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_project): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn create_project(
    State(state): State<AppState>,
    Json(payload): Json<ProjectPayload>,
) -> Result<(StatusCode, Json<ProjectCreateResponse>), StatusCode> {

    let pool = &state.pool;

    let project = repo::insert(pool, payload.name)
        .await
        .map_err(|e| {
            println!("DB ERROR (create_project): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok((StatusCode::CREATED, Json(project)))
}

pub async fn update_project(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<ProjectPayload>,
) -> Result<Json<Project>, StatusCode> {

    let pool = &state.pool;

    repo::update(pool, id, payload.name)
        .await
        .map_err(|e| {
            println!("DB ERROR (update_project): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn delete_project(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {

    let pool = &state.pool;

    let has_use_cases = use_case::exists_by_project_id(pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (check_use_cases): {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Erro interno.".into(),
                }),
            )
        })?;

    if has_use_cases {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "BR_0001".into(),
                message: "Não foi possível excluir o projeto, existe caso de uso associado.".into(),
            }),
        ));
    }

    let deleted = repo::delete(pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (delete_project): {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Erro interno.".into(),
                }),
            )
        })?;

    if deleted {
        Ok(Json(DeleteResponse { deleted: true }))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "BR_0002".into(),
                message: "Projeto não encontrado.".into(),
            }),
        ))
    }
}