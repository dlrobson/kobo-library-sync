//! A simple web server using Axum framework

use axum::{Router, routing::get};

/// Creates and configures the Axum router
fn create_app() -> Router {
    Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .layer(axum::middleware::from_fn(log_incoming_request))
}

/// Starts the server on the specified port
pub async fn start_server(port: u16) -> anyhow::Result<()> {
    let app = create_app();

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    tracing::info!("Listening on http://0.0.0.0:{port}");
    axum::serve(listener, app).await.map_err(Into::into)
}

async fn log_incoming_request(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<impl axum::response::IntoResponse, (axum::http::StatusCode, String)> {
    tracing::debug!("Incoming request: {} {}", req.method(), req.uri());

    Ok(next.run(req).await)
}
