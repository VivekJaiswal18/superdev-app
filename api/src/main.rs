use poem::{handler, listener::TcpListener, Route, Server};
use dotenv::dotenv;
use std::env;

#[handler]
async fn index() -> &'static str {
    "Welcome to the API!"
}

#[handler]
async fn hello() -> &'static str {
    "Hello, world!"
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
        .at("/", index)
        .at("/hello", hello)
        .at("/health", health);

    Server::new(TcpListener::bind(addr))
        .run(app)
        .await
}
