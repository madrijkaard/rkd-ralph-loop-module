mod api;
mod config;
mod db;
mod dto;
mod enumerator;
mod model;
mod repository;

use db::init_db;

#[tokio::main]
async fn main() {
    let settings = config::load_config();
    let pool = init_db(&settings.database_url).await;

    let app = api::routes(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://localhost:3000");

    axum::serve(listener, app).await.unwrap();
}