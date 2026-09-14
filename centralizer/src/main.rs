mod api;
mod state;

use std::env;
use std::time::Instant;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let api_key = env::var("X-API-Key").expect("FATAL: X-API-Key must be set in .env");

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let shared_data = api::SharedData {
        state: state::new_state(),
        start_time: Instant::now(),
        api_key,
    };

    let app = api::router(shared_data);

    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("Centralizer listening on {}", addr);
    
    // Axum server
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .await
        .unwrap();
}
