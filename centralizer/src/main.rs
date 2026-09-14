mod api;
mod cache;
mod renderer;
mod state;

use std::env;
use std::time::Instant;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // Extraemos la llave usando el estandar de guiones bajos
    let api_key = env::var("X_API_KEY").expect("FATAL: X_API_KEY must be set in .env");

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let shared_data = api::SharedData {
        state: state::new_state(),
        cache: cache::new_cache(),
        start_time: Instant::now(),
        api_key,
    };

    let app = api::router(shared_data);

    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("Centralizer listening on {}", addr);

    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .await
        .unwrap();
}