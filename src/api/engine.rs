use axum::{
    extract::State,
    http::StatusCode,
    Json,
};

use crate::api::AppState;
use crate::engine::EngineClient;
use crate::dto::ErrorResponse;

//
// ==========================
// GET MODELS 🔥
// ==========================
//

pub async fn get_models(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, (StatusCode, Json<ErrorResponse>)> {

    let settings = &state.settings;

    // 1. Criar client do engine
    let engine = EngineClient::new(settings.engine_base_url.clone());

    // 2. Buscar modelos
    let models = engine
        .list_models()
        .await
        .map_err(|e| {
            println!("ENGINE ERROR (get_models): {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    code: "BR_0000".into(),
                    message: "Erro ao buscar modelos do engine.".into(),
                }),
            )
        })?;

    // 3. Retornar apenas lista de IDs
    Ok(Json(models))
}