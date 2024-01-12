use axum::{routing::get, Router};

pub mod application_monitor;

mod application;
mod deposit;
mod error;
mod node;
mod task;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/deposit", get(deposit::get))
        .route("/node", get(node::get))
        .route("/task", get(task::get))
        .route("/application", get(application::get));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
