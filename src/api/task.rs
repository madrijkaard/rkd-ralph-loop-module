use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::fs;
use std::path::Path as StdPath;
use regex::Regex;

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

    // Valida se o path é seguro (agora deve ser apenas diretório)
    if let Err(e) = validate_directory_path(&payload.path) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "BR_0004".into(),
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

    // Valida se o path é seguro (agora deve ser apenas diretório)
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
// EXECUTE TASK (UPDATED FOR MULTIPLE FILES)
// ==========================
//

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

    // 2. Valida o path antes de tentar escrever
    if task.path.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                code: "BR_0004".into(),
                message: "Path da tarefa não configurado.".into(),
            }),
        ));
    }

    // 3. Verifica se o diretório existe, se não, tenta criar
    let dir_path = StdPath::new(&task.path);
    if !dir_path.exists() {
        if let Err(e) = fs::create_dir_all(dir_path) {
            eprintln!("DIR ERROR: failed to create directory {}: {}", dir_path.display(), e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0003".into(),
                    message: format!("Não foi possível criar o diretório: {}", e),
                }),
            ));
        }
        eprintln!("✅ Directory created: {}", dir_path.display());
    }

    // 4. Inicializa o engine client
    let engine = EngineClient::new(state.settings.engine_base_url.clone());

    // 5. Gera os códigos via LLM (agora retorna Vec<String>)
    let generated_codes = engine
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
                    message: format!("Erro ao executar engine: {}", e),
                }),
            )
        })?;

    // 6. Verifica se algum código foi gerado
    if generated_codes.is_empty() {
        eprintln!("ERROR: No codes generated from LLM");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                code: "BR_0006".into(),
                message: "Nenhum código foi gerado pelo modelo.".into(),
            }),
        ));
    }

    eprintln!("=== GENERATED {} CODE(S) ===", generated_codes.len());

    // 7. Processa baseado no tipo da task
    match task.r#type.as_str() {
        "JAVA" => {
            let mut created_files = Vec::new();
            
            for (i, code) in generated_codes.iter().enumerate() {
                // Extrai o nome da classe do código Java
                let class_name = match extract_class_name_from_java(code) {
                    Some(name) => name,
                    None => {
                        eprintln!("WARN: Could not extract class name from code {}, using default name 'Class{}'", i+1, i+1);
                        format!("Class{}", i+1)
                    }
                };
                
                // Constrói o path completo do arquivo
                let file_path = dir_path.join(format!("{}.java", class_name));
                let file_path_str = file_path.to_str().ok_or_else(|| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            code: "BR_0003".into(),
                            message: "Path inválido para o arquivo.".into(),
                        }),
                    )
                })?;
                
                // Escreve o arquivo
                fs::write(file_path_str, code)
                    .map_err(|e| {
                        eprintln!("FILE ERROR: failed to write {}: {}", file_path_str, e);
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ErrorResponse {
                                code: "BR_0003".into(),
                                message: format!("Erro ao escrever arquivo {}: {}", class_name, e),
                            }),
                        )
                    })?;
                
                eprintln!("✅ Java file written to: {}", file_path_str);
                created_files.push(class_name);
            }
            
            eprintln!("✅ Total Java files created: {}", created_files.len());
            eprintln!("📁 Classes: {:?}", created_files);
        }
        "XML" => {
            // Para XML, ainda esperamos apenas um arquivo
            if generated_codes.len() > 1 {
                eprintln!("WARN: Received {} codes for XML task, but only first will be used", generated_codes.len());
            }
            
            let final_path = ensure_xml_extension(&task.path);
            
            fs::write(&final_path, &generated_codes[0])
                .map_err(|e| {
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
            // Para Shell, ainda esperamos apenas um arquivo
            if generated_codes.len() > 1 {
                eprintln!("WARN: Received {} codes for Shell task, but only first will be used", generated_codes.len());
            }
            
            let final_path = ensure_sh_extension(&task.path);
            
            fs::write(&final_path, &generated_codes[0])
                .map_err(|e| {
                    eprintln!("FILE ERROR: failed to write {}: {}", final_path, e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            code: "BR_0003".into(),
                            message: format!("Erro ao escrever arquivo: {}", e),
                        }),
                    )
                })?;
            
            // Torna o script executável em sistemas Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = fs::metadata(&final_path) {
                    let mut permissions = metadata.permissions();
                    permissions.set_mode(0o755);
                    if let Err(e) = fs::set_permissions(&final_path, permissions) {
                        eprintln!("WARN: could not make script executable: {}", e);
                    }
                }
            }
            
            eprintln!("✅ Shell script written to: {}", final_path);
        }
        _ => {
            eprintln!("WARN: tipo de tarefa não suportado para execução: {}", task.r#type);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    code: "BR_0005".into(),
                    message: format!("Tipo de tarefa não suportado: {}", task.r#type),
                }),
            ));
        }
    }

    Ok(StatusCode::CREATED)
}

//
// ==========================
// HELPER FUNCTIONS
// ==========================
//

/// Valida se o path é um diretório seguro para escrita
fn validate_directory_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("Path não pode ser vazio".to_string());
    }
    
    let path = StdPath::new(path);
    
    // Verifica se é um path absoluto (recomendado)
    if !path.is_absolute() {
        return Err("Path deve ser absoluto".to_string());
    }
    
    // Previne path traversal (..)
    if path.components().any(|c| c == std::path::Component::ParentDir) {
        return Err("Path não pode conter '..'".to_string());
    }
    
    // Verifica se parece um arquivo (tem extensão)
    if let Some(extension) = path.extension() {
        if extension == "java" || extension == "xml" || extension == "sh" {
            return Err("Path deve ser um diretório, não deve conter nome de arquivo com extensão".to_string());
        }
    }
    
    Ok(())
}

/// Extrai o nome da classe de um código Java
fn extract_class_name_from_java(code: &str) -> Option<String> {
    // Procura por padrões como: "public class NomeClasse", "class NomeClasse", "public interface NomeClasse"
    let patterns = [
        r"public\s+class\s+(\w+)",
        r"class\s+(\w+)",
        r"public\s+interface\s+(\w+)",
        r"interface\s+(\w+)",
        r"public\s+enum\s+(\w+)",
        r"enum\s+(\w+)",
    ];
    
    for pattern in patterns {
        let re = Regex::new(pattern).unwrap();
        if let Some(caps) = re.captures(code) {
            if let Some(class_name) = caps.get(1) {
                let name = class_name.as_str().to_string();
                eprintln!("Extracted class name: {}", name);
                return Some(name);
            }
        }
    }
    
    eprintln!("Failed to extract class name from Java code");
    eprintln!("Code preview: {}", &code[..code.len().min(200)]);
    None
}

/// Garante que o arquivo tem extensão .xml
fn ensure_xml_extension(path: &str) -> String {
    if path.to_lowercase().ends_with(".xml") {
        path.to_string()
    } else {
        format!("{}.xml", path)
    }
}

/// Garante que o arquivo tem extensão .sh
fn ensure_sh_extension(path: &str) -> String {
    if path.to_lowercase().ends_with(".sh") {
        path.to_string()
    } else {
        format!("{}.sh", path)
    }
}