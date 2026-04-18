use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;

use crate::dto::{DeleteResponse, TaskPayload, TaskCreateResponse, ErrorResponse};
use crate::model::Task;
use crate::repository::task as repo;
use crate::repository::iteration; // 🔥 novo

pub async fn get_tasks_by_use_case(
    State(pool): State<PgPool>,
    Path(use_case_id): Path<i32>,
) -> Result<Json<Vec<Task>>, StatusCode> {
    let tasks = repo::find_all_by_use_case_id(&pool, use_case_id)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_tasks_by_use_case): {:?}", e);
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
) -> Result<(StatusCode, Json<TaskCreateResponse>), StatusCode> {
    let task = repo::insert(
        &pool,
        payload.name,
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
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {

    // 🔍 Verifica se existem iterations associadas
    let has_iterations = iteration::exists_by_task_id(&pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (check_iterations): {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Erro interno.".into(),
                }),
            )
        })?;

    // 🚫 Bloqueia deleção
    if has_iterations {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "BR_0001".into(),
                message: "Não foi possível excluir a tarefa, existe iteração associada.".into(),
            }),
        ));
    }

    // 🗑 Deleção normal
    let deleted = repo::delete(&pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (delete_task): {:?}", e);
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
                message: "Tarefa não encontrada.".into(),
            }),
        ))
    }
}