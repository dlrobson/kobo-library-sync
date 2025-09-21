//! Application entry point and orchestration.

use std::{
    net::SocketAddr,
    sync::{
        Mutex,
        PoisonError,
    },
};

use anyhow::Result;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{
    layer::SubscriberExt as _,
    util::SubscriberInitExt as _,
};

use crate::{
    command_line_arguments::CommandLineArguments,
    server::Server,
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
        Self::initialize_logging(&command_line_arguments.log_level);

        let server = Server::start(
            command_line_arguments.port,
            self.cancellation_token.clone(),
            command_line_arguments.enable_request_logging,
        )
        .await?;
        tracing::info!("Server started on http://{}", server.address());
        *self.server.lock().unwrap_or_else(PoisonError::into_inner) = Some(server);
        self.server_started.cancel();

        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                match result {
                    Ok(()) => tracing::info!("Shutdown signal received"),
                    Err(e) => tracing::error!("Failed to listen for shutdown signal: {e}"),
                }
            }
            () = self.cancellation_token.cancelled() => {
                tracing::info!("Cancellation token cancelled");
            }
        }

        // Shutdown the server
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
    pub fn shutdown(&self) {
        self.cancellation_token.cancel();
    }

    /// Initialize the logging subsystem with the specified log level.
    fn initialize_logging(log_level: &str) {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::builder()
                    .parse(log_level)
                    .unwrap_or_default(),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
}
