use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::fs;

use crate::api::AppState;
use crate::dto::{
    DeleteResponse, TaskPayload, TaskCreateResponse, ErrorResponse, ExecuteTaskPayload
};
use crate::model::Task;
use crate::repository::task as repo;
use crate::repository::iteration;
use crate::enumerator::TaskType;
use crate::engine::EngineClient;

pub async fn get_tasks_by_use_case(
    State(state): State<AppState>,
    Path(use_case_id): Path<i32>,
) -> Result<Json<Vec<Task>>, StatusCode> {

    let pool = &state.pool;

    let tasks = repo::find_all_by_use_case_id(pool, use_case_id)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_tasks_by_use_case): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(tasks))
}

pub async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Task>, StatusCode> {

    let pool = &state.pool;

    repo::find_by_id(pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (get_task): {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn create_task(
    State(state): State<AppState>,
    Json(payload): Json<TaskPayload>,
) -> Result<(StatusCode, Json<TaskCreateResponse>), (StatusCode, Json<ErrorResponse>)> {

    let pool = &state.pool;

    if !TaskType::is_valid(&payload.r#type) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "BR_0002".into(),
                message: "O tipo informado para a tarefa está inválido.".into(),
            }),
        ));
    }

    let task = repo::insert(
        pool,
        payload.name,
        payload.r#type,
        payload.path,
        payload.system_prompt,
        payload.user_prompt,
        payload.use_case_id,
    )
    .await
    .map_err(|e| {
        println!("DB ERROR (create_task): {:?}", e);
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

pub async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<TaskPayload>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {

    let pool = &state.pool;

    if !TaskType::is_valid(&payload.r#type) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "BR_0002".into(),
                message: "O tipo informado para a tarefa está inválido.".into(),
            }),
        ));
    }

    repo::update(
        pool,
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
        println!("DB ERROR (update_task): {:?}", e);
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

pub async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ErrorResponse>)> {

    let pool = &state.pool;

    let has_iterations = iteration::exists_by_task_id(pool, id)
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

    if has_iterations {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "BR_0001".into(),
                message: "Não foi possível excluir a tarefa, existe iteração associada.".into(),
            }),
        ));
    }

    let deleted = repo::delete(pool, id)
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

//
// ==========================
// EXECUTE TASK 🔥
// ==========================
//

pub async fn execute_task(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<ExecuteTaskPayload>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {

    let pool = &state.pool;
    let settings = &state.settings;

    // 1. Buscar task
    let task = repo::find_by_id(pool, id)
        .await
        .map_err(|e| {
            println!("DB ERROR (execute_task): {:?}", e);
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

    // 2. Criar client
    let engine = EngineClient::new(settings.engine_base_url.clone());

    // 3. Executar engine usando prompts da task
    let content = engine
        .generate(
            task.system_prompt.clone(),
            task.user_prompt.clone(),
            payload.model,
        )
        .await
        .map_err(|e| {
            println!("ENGINE ERROR: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Erro ao executar engine.".into(),
                }),
            )
        })?;

    // 4. Se for JAVA → criar arquivo
    if task.r#type == "JAVA" {
        fs::write(&task.path, content)
            .map_err(|e| {
                println!("FILE ERROR: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        code: "BR_0003".into(),
                        message: "Erro ao escrever arquivo.".into(),
                    }),
                )
            })?;
    }

    Ok(StatusCode::CREATED)
}