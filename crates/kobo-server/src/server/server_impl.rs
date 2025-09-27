//! A simple web server using Axum framework

use std::net::SocketAddr;

use axum::{
    Router,
    middleware,
};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::server::{
    kobo_store_fallback::kobo_store_fallback,
    request_logging,
    server_state::ServerState,
};

/// Server struct that manages the Axum server lifecycle
pub struct Server {
    /// The address the server is bound to
    address: SocketAddr,
    /// Cancellation token for graceful shutdown
    cancellation_token: CancellationToken,
    /// Handle to the server task
    handle: JoinHandle<anyhow::Result<()>>,
}

impl Server {
    /// Creates a new server instance and starts it
    pub async fn start(
        port: u16,
        cancellation_token: CancellationToken,
        enable_request_logging: bool,
        enable_response_logging: bool,
    ) -> anyhow::Result<Self> {
        let app = create_router(enable_request_logging, enable_response_logging);
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
        let address = listener.local_addr()?;

        let cancellation_token_clone = cancellation_token.clone();
        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    cancellation_token_clone.cancelled().await;
                })
                .await
                .map_err(Into::into)
        });

        Ok(Self {
            address,
            cancellation_token,
            handle: server_handle,
        })
    }

    /// Gets the address the server is bound to
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Gracefully shuts down the server
    pub async fn shutdown(self) -> anyhow::Result<()> {
        self.cancellation_token.cancel();
        self.handle.await?
    }
}

/// Creates and configures the Axum router
fn create_router(enable_request_logging: bool, enable_response_logging: bool) -> Router {
    let mut router = Router::new().fallback(kobo_store_fallback);

    if enable_request_logging {
        router = router.layer(middleware::from_fn(request_logging::log_requests));
    }

    if enable_response_logging {
        router = router.layer(middleware::from_fn(request_logging::log_responses));
    }
    router.with_state(ServerState::new())
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{
            Request,
            StatusCode,
        },
    };
    use http_body_util::BodyExt as _;
    use tower::ServiceExt as _;
    use tracing_test::traced_test;

    use super::*;

    #[tokio::test]
    async fn test_hello_world() {
        let app = create_router(false, false);

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"Hello, World!");
    }

    #[tokio::test]
    async fn test_not_found() {
        let app = create_router(false, false);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(body.is_empty());
    }

    #[tokio::test]
    #[traced_test]
    async fn test_app_with_logging_enabled() {
        let app = create_router(true, true);

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"Hello, World!");

        // Check that logging occurred
        assert!(logs_contain("Incoming request: GET /"));
    }
}
