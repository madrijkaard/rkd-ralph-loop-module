mod api;
mod config;
mod db;
mod dto;
mod enumerator;
mod model;
mod repository;
mod engine;

use db::init_db;
use api::AppState;

#[tokio::main]
async fn main() {
    // 1. Carrega configurações
    let settings = config::load_config();

    // 2. Inicializa pool do banco
    let pool = init_db(&settings.database_url).await;

    // 3. Monta o estado global da aplicação
    let state = AppState {
        pool,
        settings,
    };

    // 4. Cria rotas com state
    let app = api::routes(state);

    // 5. Sobe servidor
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("Server running on http://localhost:3000");

    axum::serve(listener, app).await.unwrap();
}