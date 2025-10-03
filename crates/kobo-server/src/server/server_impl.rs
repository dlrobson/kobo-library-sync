//! A simple web server using Axum framework

use std::net::SocketAddr;

use axum::{ServiceExt, body::Body};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::server::{router::create_router, state::server_state::ServerState};

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
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to start.
    pub async fn start(
        port: u16,
        frontend_url: String,
        cancellation_token: CancellationToken,
        enable_request_logging: bool,
        enable_response_logging: bool,
    ) -> anyhow::Result<Self> {
        let app_state = ServerState::builder().frontend_url(frontend_url).build();
        let app = create_router(enable_request_logging, enable_response_logging, app_state);
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
        let address = listener.local_addr()?;

        let cancellation_token_clone = cancellation_token.clone();
        let server_handle = tokio::spawn(async move {
            axum::serve(
                listener,
                ServiceExt::<hyper::Request<Body>>::into_make_service(app),
            )
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
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to shut down cleanly.
    pub async fn shutdown(self) -> anyhow::Result<()> {
        self.cancellation_token.cancel();
        self.handle.await?
    }
}
