//! A simple web server using Axum framework

use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/path", get(|| async { "Hello, World!" }))
        .layer(axum::middleware::from_fn(print_request_response));

    tracing::info!("Listening on http://0.0.0.0:3000");

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn print_request_response(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<impl axum::response::IntoResponse, (axum::http::StatusCode, String)> {
    tracing::debug!("Incoming request: {} {}", req.method(), req.uri());

    Ok(next.run(req).await)
}
