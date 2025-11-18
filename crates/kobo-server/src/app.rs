//! Application entry point and orchestration.

use std::{
    net::SocketAddr,
    sync::{Mutex, PoisonError},
};

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use crate::{
    command_line_arguments::CommandLineArguments,
    server::{Server, ServerBuilder},
};

/// The main application struct that orchestrates the entire application lifecycle.
pub struct App {
    // Cancellation token to signal shutdown
    cancellation_token: CancellationToken,
    // Optional server instance
    server: Mutex<Option<Server>>,
    // Started flag to indicate if the server is running
    server_started: CancellationToken,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Creates a new instance of the application.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cancellation_token: CancellationToken::new(),
            server: Mutex::new(None),
            server_started: CancellationToken::new(),
        }
    }

    /// Initialize and run the application
    ///
    /// # Errors
    ///
    /// If the server fails to start.
    pub async fn run(&self, command_line_arguments: CommandLineArguments) -> Result<()> {
        self.start_server(command_line_arguments).await?;
        self.wait_for_shutdown_signal().await;
        self.shutdown().await?;

        Ok(())
    }

    /// Starts the server and stores it in the app state
    ///
    /// # Errors
    ///
    /// If the server fails to start.
    async fn start_server(&self, command_line_arguments: CommandLineArguments) -> Result<()> {
        let server = ServerBuilder::new(self.cancellation_token.clone())
            .port(command_line_arguments.port)
            .frontend_url(
                command_line_arguments
                    .frontend_url
                    .unwrap_or_else(|| format!("http://localhost:{}", command_line_arguments.port)),
            )
            .enable_request_logging(command_line_arguments.enable_request_logging)
            .enable_response_logging(command_line_arguments.enable_response_logging)
            .build()
            .await?;

        tracing::info!("Server started on http://{}", server.address());
        *self.server.lock().unwrap_or_else(PoisonError::into_inner) = Some(server);

        self.server_started.cancel();

        Ok(())
    }

    /// Waits for a shutdown signal (Ctrl+C or cancellation token)
    async fn wait_for_shutdown_signal(&self) {
        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                if let Err(e) = result {
                    tracing::error!("Failed to listen for shutdown signal: {e}");
                    return;
                }
            }
            () = self.cancellation_token.cancelled() => ()
        }

        tracing::info!("Shutdown signal received");
    }

    /// Waits until the server is running.
    pub async fn wait_until_running(&self) {
        self.server_started.cancelled().await;
    }

    /// Gets the server's bound address if the server is running
    pub fn server_address(&self) -> Option<SocketAddr> {
        self.server
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map(Server::address)
    }

    /// Gracefully shuts down the application.
    ///
    /// # Errors
    ///
    /// If the server fails to shut down cleanly.
    pub async fn shutdown(&self) -> Result<()> {
        self.cancellation_token.cancel();
        let server = self
            .server
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();

        if let Some(server) = server {
            server.shutdown().await?;
        }

        Ok(())
    }
}
