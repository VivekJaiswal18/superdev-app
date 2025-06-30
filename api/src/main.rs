use dotenv::dotenv;
use poem::{
    handler, listener::TcpListener, web::Json, Route, Server, Result, error::InternalServerError
};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::env;
use base58::ToBase58;

#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
struct KeypairResponse {
    pubkey: String,
    secret: String,
}

#[handler]
async fn generate_keypair() -> Result<Json<ApiResponse<KeypairResponse>>> {
    let keypair = Keypair::new();
    let response = ApiResponse {
        success: true,
        data: Some(KeypairResponse {
            pubkey: keypair.pubkey().to_string(),
            secret: (&keypair.to_bytes()[..]).to_base58(),
        }),
        error: None,
    };
    Ok(Json(response))
}

#[handler]
async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    dotenv().ok();
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("127.0.0.1:{}", port);
    let app = Route::new()
        .at("/health", health)
        .at("/keypair", generate_keypair);
    println!("🚀 Solana HTTP Server starting on {}", addr);
    Server::new(TcpListener::bind(addr))
        .run(app)
        .await
}
