use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::fs;
use std::path::Path as StdPath;

use crate::api::AppState;
use crate::dto::{
    DeleteResponse, TaskPayload, TaskCreateResponse, ErrorResponse, ExecuteTaskPayload,
};
use crate::model::Task;
use crate::repository::task as repo;
use crate::repository::iteration;
use crate::enumerator::TaskType;
use crate::engine::EngineClient;
use crate::parser::extract_codes;

pub async fn get_tasks_by_use_case(
    State(state): State<AppState>,
    Path(use_case_id): Path<i32>,
) -> Result<Json<Vec<Task>>, (StatusCode, Json<ErrorResponse>)> {

    let tasks = repo::find_all_by_use_case_id(&state.pool, use_case_id)
        .await
        .map_err(|e| {
            eprintln!("DB ERROR (get_tasks_by_use_case): {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Ocorreu um erro inesperado.".into(),
                }),
            )
        })?;

    if tasks.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "BR_0001".into(),
                message: "Não existem tarefas cadastradas para este caso de uso.".into(),
            }),
        ));
    }

    Ok(Json(tasks))
}

pub async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Task>, (StatusCode, Json<ErrorResponse>)> {

    repo::find_by_id(&state.pool, id)
        .await
        .map_err(|e| {
            eprintln!("DB ERROR (get_task): {:?}", e);
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
                message: "Tarefa não encontrada.".into(),
            }),
        ))
}

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

    if let Err(e) = validate_directory_path(&payload.path) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "BR_0002".into(),
                message: format!("Path inválido: {}", e),
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
                message: "Ocorreu um erro inesperado.".into(),
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

    if !TaskType::is_valid(&payload.r#type) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "BR_0002".into(),
                message: "Tipo de tarefa inválido.".into(),
            }),
        ));
    }

    if let Err(e) = validate_directory_path(&payload.path) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "BR_0004".into(),
                message: format!("Path inválido: {}", e),
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
                message: "Ocorreu um erro inesperado.".into(),
            }),
        )
    })?
    .map(Json)
    .ok_or((
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            code: "BR_0001".into(),
            message: "Tarefa não encontrada.".into(),
        }),
    ))
}

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
                    message: "Ocorreu um erro inesperado.".into(),
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

    let deleted = repo::delete(&state.pool, id)
        .await
        .map_err(|e| {
            eprintln!("DB ERROR (delete_task): {:?}", e);
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
                message: "Tarefa não encontrada.".into(),
            }),
        ))
    }
}

pub async fn execute_task(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<ExecuteTaskPayload>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {

    // 1. Busca a task no banco
    let task = repo::find_by_id(&state.pool, id)
        .await
        .map_err(|e| {
            eprintln!("DB ERROR (execute_task): {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Ocorreu um erro inesperado.".into(),
                }),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                code: "BR_0001".into(),
                message: "Tarefa não encontrada.".into(),
            }),
        ))?;

    // 2. Valida se o path está configurado
    if task.path.trim().is_empty() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                code: "BR_0000".into(),
                message: "O path da tarefa não foi definido.".into(),
            }),
        ));
    }

    // 3. Garante que o diretório existe, criando-o se necessário
    let dir_path = StdPath::new(&task.path);

    if !dir_path.exists() {
        fs::create_dir_all(dir_path).map_err(|e| {
            eprintln!("DIR ERROR: failed to create directory {}: {}", dir_path.display(), e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Não foi possível criar um path da tarefa.".into(),
                }),
            )
        })?;
        eprintln!("✅ Directory created: {}", dir_path.display());
    }

    // 4. Chama o engine (LLM)
    let engine = EngineClient::new(state.settings.engine_base_url.clone());

    let raw_content = engine
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
                    message: "Não foi possível gerar o código pela engine.".into(),
                }),
            )
        })?;

    // 5. Processa o retorno do LLM de acordo com o task.type
    match task.r#type.as_str() {
        "JAVA" => {
            let classes = extract_codes(&raw_content).map_err(|e| {
                eprintln!("PARSER ERROR: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        code: "BR_0006".into(),
                        message: "Nenhum código foi gerado pelo modelo.".into(),
                    }),
                )
            })?;

            eprintln!("=== {} JAVA CLASS(ES) EXTRACTED ===", classes.len());

            let mut created_files: Vec<String> = Vec::new();

            for (i, class_code) in classes.iter().enumerate() {
                let class_name = extract_java_class_name(class_code)
                    .unwrap_or_else(|| {
                        eprintln!(
                            "WARN: could not extract class name from code {}, using 'Class{}'",
                            i + 1,
                            i + 1
                        );
                        format!("Class{}", i + 1)
                    });

                let file_path = dir_path.join(format!("{}.java", class_name));

                fs::write(&file_path, class_code).map_err(|e| {
                    eprintln!("FILE ERROR: failed to write {:?}: {}", file_path, e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            code: "BR_0003".into(),
                            message: format!("Erro ao escrever arquivo {}.java: {}", class_name, e),
                        }),
                    )
                })?;

                eprintln!("✅ Java file written: {:?}", file_path);
                created_files.push(class_name);
            }

            eprintln!("📁 Classes created: {:?}", created_files);
        }

        "XML" => {
            let code = extract_single_code(&raw_content).map_err(|e| {
                eprintln!("PARSER ERROR (XML): {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        code: "BR_0006".into(),
                        message: "Nenhum código XML foi gerado pelo modelo.".into(),
                    }),
                )
            })?;

            let final_path = ensure_extension(&task.path, "xml");

            fs::write(&final_path, &code).map_err(|e| {
                eprintln!("FILE ERROR: failed to write {}: {}", final_path, e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        code: "BR_0003".into(),
                        message: format!("Erro ao escrever arquivo: {}", e),
                    }),
                )
            })?;

            eprintln!("✅ XML file written to: {}", final_path);
        }

        "SHELL_SCRIPT" => {
            let code = extract_single_code(&raw_content).map_err(|e| {
                eprintln!("PARSER ERROR (SHELL): {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        code: "BR_0006".into(),
                        message: "Nenhum script foi gerado pelo modelo.".into(),
                    }),
                )
            })?;

            let final_path = ensure_extension(&task.path, "sh");

            fs::write(&final_path, &code).map_err(|e| {
                eprintln!("FILE ERROR: failed to write {}: {}", final_path, e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        code: "BR_0003".into(),
                        message: format!("Erro ao escrever arquivo: {}", e),
                    }),
                )
            })?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = fs::metadata(&final_path) {
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o755);
                    if let Err(e) = fs::set_permissions(&final_path, perms) {
                        eprintln!("WARN: could not make script executable: {}", e);
                    }
                }
            }

            eprintln!("✅ Shell script written to: {}", final_path);
        }

        _ => {
            eprintln!("WARN: tipo de tarefa não mapeado: {}", task.r#type);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Tipo da tarefa não mapeado.".into(),
                }),
            ));
        }
    }

    Ok(StatusCode::CREATED)
}

fn validate_directory_path(path: &str) -> Result<(), String> {

    if path.is_empty() {
        return Err("Path não pode ser vazio".to_string());
    }

    let p = StdPath::new(path);

    if !p.is_absolute() {
        return Err("Path deve ser absoluto".to_string());
    }

    if p.components().any(|c| c == std::path::Component::ParentDir) {
        return Err("Path não pode conter '..'".to_string());
    }

    if let Some(ext) = p.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        if ext == "java" || ext == "xml" || ext == "sh" {
            return Err(
                "Path deve ser um diretório, não deve conter nome de arquivo com extensão"
                    .to_string(),
            );
        }
    }

    Ok(())
}

fn extract_java_class_name(code: &str) -> Option<String> {

    let patterns = [
        "public class ",
        "public interface ",
        "public enum ",
        "class ",
        "interface ",
        "enum ",
    ];

    for pattern in patterns {
        if let Some(pos) = code.find(pattern) {
            let after = &code[pos + pattern.len()..];
            let name: String = after
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                eprintln!("[task] extracted class name: {}", name);
                return Some(name);
            }
        }
    }

    eprintln!("[task] failed to extract class name");
    None
}

fn extract_single_code(raw: &str) -> Result<String, String> {

    let sanitized = raw.trim().replace("\\'", "'");

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&sanitized) {

        if let Some(arr) = value.get("code").and_then(|v| v.as_array()) {
            if let Some(first) = arr.first().and_then(|v| v.as_str()) {
                return Ok(first.to_string());
            }
        }

        if let Some(s) = value.get("code").and_then(|v| v.as_str()) {
            return Ok(s.to_string());
        }
    }

    Err("Could not extract code from LLM output".to_string())
}

fn ensure_extension(path: &str, ext: &str) -> String {

    if path.to_lowercase().ends_with(&format!(".{}", ext)) {
        path.to_string()
    } else {
        format!("{}.{}", path, ext)
    }
}