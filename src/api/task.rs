use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::fs;

use crate::api::AppState;
use crate::dto::{
    DeleteResponse, TaskPayload, TaskCreateResponse, ErrorResponse, ExecuteTaskPayload,
};
use crate::model::Task;
use crate::repository::task as repo;
use crate::repository::iteration;
use crate::enumerator::TaskType;
use crate::engine::EngineClient;

//
// ==========================
// GET TASKS BY USE CASE
// ==========================
//

pub async fn get_tasks_by_use_case(
    State(state): State<AppState>,
    Path(use_case_id): Path<i32>,
) -> Result<Json<Vec<Task>>, StatusCode> {
    let tasks = repo::find_all_by_use_case_id(&state.pool, use_case_id)
        .await
        .map_err(|e| {
            eprintln!("DB ERROR (get_tasks_by_use_case): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(tasks))
}

//
// ==========================
// GET TASK
// ==========================
//

pub async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Task>, StatusCode> {
    repo::find_by_id(&state.pool, id)
        .await
        .map_err(|e| {
            eprintln!("DB ERROR (get_task): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

//
// ==========================
// CREATE TASK
// ==========================
//

pub async fn create_task(
    State(state): State<AppState>,
    Json(payload): Json<TaskPayload>,
) -> Result<(StatusCode, Json<TaskCreateResponse>), (StatusCode, Json<ErrorResponse>)> {
    if !TaskType::is_valid(&payload.r#type) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "BR_0002".into(),
                message: "Tipo de tarefa inválido.".into(),
            }),
        ));
    }

    let task = repo::insert(
        &state.pool,
        payload.name,
        payload.r#type,
        payload.path,
        payload.system_prompt,
        payload.user_prompt,
        payload.use_case_id,
    )
    .await
    .map_err(|e| {
        eprintln!("DB ERROR (create_task): {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                code: "BR_0000".into(),
                message: "Erro interno.".into(),
            }),
        )
    })?;

    Ok((StatusCode::CREATED, Json(task)))
}

//
// ==========================
// UPDATE TASK
// ==========================
//

pub async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<TaskPayload>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {
    if !TaskType::is_valid(&payload.r#type) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "BR_0002".into(),
                message: "Tipo de tarefa inválido.".into(),
            }),
        ));
    }

    repo::update(
        &state.pool,
        id,
        payload.name,
        payload.r#type,
        payload.path,
        payload.system_prompt,
        payload.user_prompt,
        payload.use_case_id,
    )
    .await
    .map_err(|e| {
        eprintln!("DB ERROR (update_task): {:?}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                code: "BR_0000".into(),
                message: "Erro interno.".into(),
            }),
        )
    })?
    .map(Json)
    .ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            code: "BR_0002".into(),
            message: "Tarefa não encontrada.".into(),
        }),
    ))
}

//
// ==========================
// DELETE TASK
// ==========================
//

pub async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {
    let has_iterations = iteration::exists_by_task_id(&state.pool, id)
        .await
        .map_err(|e| {
            eprintln!("DB ERROR (check_iterations): {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Erro interno.".into(),
                }),
            )
        })?;

    if has_iterations {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "BR_0001".into(),
                message: "Existe iteração associada à tarefa.".into(),
            }),
        ));
    }

    let deleted = repo::delete(&state.pool, id)
        .await
        .map_err(|e| {
            eprintln!("DB ERROR (delete_task): {:?}", e);
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

//
// ==========================
// EXECUTE TASK (ENGINE FIXED)
// ==========================
//

pub async fn execute_task(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<ExecuteTaskPayload>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let task = repo::find_by_id(&state.pool, id)
        .await
        .map_err(|e| {
            eprintln!("DB ERROR (execute_task): {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Erro interno.".into(),
                }),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "BR_0002".into(),
                message: "Tarefa não encontrada.".into(),
            }),
        ))?;

    let engine = EngineClient::new(state.settings.engine_base_url.clone());

    let generated_code = engine
        .generate(
            task.system_prompt.clone(),
            task.user_prompt.clone(),
            payload.model,
        )
        .await
        .map_err(|e| {
            eprintln!("ENGINE ERROR: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Erro ao executar engine.".into(),
                }),
            )
        })?;

    match task.r#type.as_str() {
        "JAVA" => {
            fs::write(&task.path, generated_code)
                .map_err(|e| {
                    eprintln!("FILE ERROR: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            code: "BR_0003".into(),
                            message: "Erro ao escrever arquivo.".into(),
                        }),
                    )
                })?;
        }
        _ => {
            eprintln!("WARN: tipo de tarefa não suportado para execução: {}", task.r#type);
        }
    }

    Ok(StatusCode::CREATED)
}