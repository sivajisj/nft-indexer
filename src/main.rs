mod auth;
mod chain;
mod config;
mod db;
mod events;
mod errors;
mod api;
use alloy::primitives::address;
use axum::{Router, routing::{post, get}};
use tower_http::cors::{CorsLayer, Any};

use config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::from_env();

    let pool = db::connect(&cfg.database_url).await?;


let app = Router::new()
    .route("/auth/nonce", post(auth::get_nonce))
    .route("/auth/verify", post(auth::verify_signature))
.route("/tokens/owned/:address", get(api::owned_tokens))
    .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
    .with_state(pool.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await?;
    println!("API server running on http://localhost:4000");
    let indexer_pool = pool.clone();
    tokio::spawn(async move {
        let provider = chain::build_provider(&cfg.rpc_url).await.unwrap();
        let contract_address = address!("1D24FE1860F4E670aFd65C1B93118A4B4F5c0f54");
        chain::run_indexer(provider, indexer_pool, contract_address, 11155111i64)
            .await
            .unwrap();
    });

    axum::serve(listener, app).await?;

    Ok(())
}
