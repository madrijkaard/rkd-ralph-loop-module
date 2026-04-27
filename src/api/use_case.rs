use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::api::AppState;
use crate::dto::{DeleteResponse, UseCasePayload, UseCaseCreateResponse, ErrorResponse};
use crate::model::UseCase;
use crate::repository::use_case as repo;
use crate::repository::task;

pub async fn get_use_cases_by_project(
    State(state): State<AppState>,
    Path(project_id): Path<i32>,
) -> Result<Json<Vec<UseCase>>, (StatusCode, Json<ErrorResponse>)> {

    let pool = &state.pool;

    let use_cases = repo::find_all_by_project_id(pool, project_id)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_use_cases_by_project): {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Ocorreu um erro inesperado.".into(),
                }),
            )
        })?;

    if use_cases.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "BR_0001".into(),
                message: "Não existem casos de uso cadastrados para este projeto.".into(),
            }),
        ));
    }

    Ok(Json(use_cases))
}

pub async fn get_use_case(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<UseCase>, (StatusCode, Json<ErrorResponse>)> {

    let pool = &state.pool;

    repo::find_by_id(pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_use_case): {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Ocorreu um erro inesperado.".into(),
                }),
            )
        })?
        .map(Json)
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "BR_0001".into(),
                message: "Caso de uso não encontrado.".into(),
            }),
        ))
}

pub async fn create_use_case(
    State(state): State<AppState>,
    Json(payload): Json<UseCasePayload>,
) -> Result<(StatusCode, Json<UseCaseCreateResponse>), (StatusCode, Json<ErrorResponse>)> {

    let pool = &state.pool;

    let use_case = repo::insert(
        pool,
        payload.name,
        payload.specification,
        payload.project_id,
    )
    .await
    .map_err(|e| {
        println!("DB ERROR (create_use_case): {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                code: "BR_0000".into(),
                message: "Ocorreu um erro inesperado.".into(),
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(use_case)))
}

pub async fn update_use_case(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UseCasePayload>,
) -> Result<Json<UseCase>, (StatusCode, Json<ErrorResponse>)> {

    let pool = &state.pool;

    repo::update(
        pool,
        id,
        payload.name,
        payload.specification,
        payload.project_id,
    )
    .await
    .map_err(|e| {
        println!("DB ERROR (update_use_case): {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                code: "BR_0000".into(),
                message: "Ocorreu um erro inesperado.".into(),
            }),
        )
    })?
    .map(Json)
    .ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            code: "BR_0001".into(),
            message: "Caso de uso não encontrado.".into(),
        }),
    ))
}

pub async fn delete_use_case(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {

    let pool = &state.pool;

    let has_tasks = task::exists_by_use_case_id(pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (check_tasks): {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Ocorreu um erro inesperado.".into(),
                }),
            )
        })?;

    if has_tasks {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "BR_0001".into(),
                message: "Não foi possível excluir o caso de uso, existe tarefa associada.".into(),
            }),
        ));
    }

    let deleted = repo::delete(pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (delete_use_case): {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Ocorreu um erro inesperado.".into(),
                }),
            )
        })?;

    if deleted {
        Ok(Json(DeleteResponse { deleted: true }))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "BR_0001".into(),
                message: "Caso de uso não encontrado.".into(),
            }),
        ))
    }
}