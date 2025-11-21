//! Application entry point and orchestration.

pub use implementation::App;

mod implementation {
    use std::{
        net::SocketAddr,
        sync::{Mutex, PoisonError},
    };

    use anyhow::Result;
    use axum::serve::Listener;
    use tokio_util::sync::CancellationToken;

    use crate::{
        command_line_arguments::CommandLineArguments,
        server::{
            Server, ServerBuilder,
            listener::{IntoListener, TokioTcpListener},
        },
    };

    /// The main application struct that orchestrates the entire application lifecycle.
    pub struct App<L> {
        // Cancellation token to signal shutdown
        cancellation_token: CancellationToken,
        // Optional server instance
        server: Mutex<Option<Server>>,
        // Started flag to indicate if the server is running
        server_started: CancellationToken,
        // The server builder
        server_builder: Mutex<Option<ServerBuilder<L>>>,
    }

    impl<L> App<L>
    where
        L: IntoListener + Send,
        <L::Listener as Listener>::Io: Send + Unpin + 'static,
    {
        /// Creates a new instance with a custom server builder (useful for testing)
        #[must_use]
        pub fn with_server_builder(server_builder: ServerBuilder<L>) -> Self {
            let cancellation_token = CancellationToken::new();
            Self {
                cancellation_token: cancellation_token.clone(),
                server: Mutex::new(None),
                server_started: CancellationToken::new(),
                server_builder: Mutex::new(Some(server_builder)),
            }
        }

        /// Initialize and run the application
        ///
        /// # Errors
        ///
        /// If the server fails to start.
        pub async fn run(&self) -> Result<()> {
            self.start_server().await?;
            self.wait_for_shutdown_signal().await;
            self.shutdown().await?;

            Ok(())
        }

        /// Starts the server and stores it in the app state
        ///
        /// # Errors
        ///
        /// If the server fails to start.
        async fn start_server(&self) -> Result<()> {
            let Some(server_builder) = self
                .server_builder
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take()
            else {
                return Err(anyhow::anyhow!("Server builder is not set"));
            };
            let server = server_builder.build().await?;

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

    impl App<TokioTcpListener> {
        /// Creates a new instance of the application for production use.
        #[must_use]
        pub fn new(command_line_arguments: CommandLineArguments) -> Self {
            let cancellation_token = CancellationToken::new();
            let server_builder =
                ServerBuilder::new(cancellation_token.clone())
                    .port(command_line_arguments.port)
                    .frontend_url(command_line_arguments.frontend_url.unwrap_or_else(|| {
                        format!("http://localhost:{}", command_line_arguments.port)
                    }))
                    .enable_request_logging(command_line_arguments.enable_request_logging)
                    .enable_response_logging(command_line_arguments.enable_response_logging);

            Self::with_server_builder(server_builder)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::{
        command_line_arguments::CommandLineArguments,
        server::{ServerBuilder, listener::FakeListenerBuilder},
    };

    impl App<FakeListenerBuilder> {
        /// Creates a new instance for testing with a fake listener
        #[must_use]
        pub fn new_for_test() -> Self {
            let cancellation_token = CancellationToken::new();
            let server_builder = ServerBuilder::new(cancellation_token.clone())
                .listener_builder(FakeListenerBuilder);

            Self::with_server_builder(server_builder)
        }
    }

    #[test]
    fn new_creates_app_with_tcp_listener() {
        let args = CommandLineArguments {
            port: 8080,
            frontend_url: Some("http://localhost:8080".to_owned()),
            enable_request_logging: false,
            enable_response_logging: false,
            log_level: "info".to_owned(),
        };

        let app = App::new(args);
        assert!(app.server_address().is_none());
    }

    #[test]
    fn with_server_builder_creates_app() {
        let cancellation_token = CancellationToken::new();
        let server_builder =
            ServerBuilder::new(cancellation_token).listener_builder(FakeListenerBuilder);

        let app = App::with_server_builder(server_builder);
        assert!(app.server_address().is_none());
    }

    #[test]
    fn new_for_test_creates_app_with_fake_listener() {
        let app = App::new_for_test();
        assert!(app.server_address().is_none());
    }

    #[test]
    fn server_address_is_none_initially() {
        let app = App::new_for_test();
        assert!(app.server_address().is_none());
    }

    #[tokio::test]
    async fn shutdown_succeeds_without_starting() {
        let app = App::new_for_test();
        let result = app.shutdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn shutdown_can_be_called_multiple_times() {
        let app = App::new_for_test();
        app.shutdown().await.unwrap();
        let result = app.shutdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_server_address_after_start() {
        let app = Arc::new(App::new_for_test());
        let app_clone = app.clone();
        let run_handle = tokio::spawn(async move { app_clone.run().await });

        app.wait_until_running().await;
        assert!(app.server_address().is_some());

        app.shutdown().await.unwrap();
        run_handle.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_clean_shutdown() {
        let app = Arc::new(App::new_for_test());
        let app_clone = app.clone();
        let run_handle = tokio::spawn(async move { app_clone.run().await });

        app.wait_until_running().await;

        app.shutdown().await.unwrap();
        let run_result = run_handle.await.unwrap();
        assert!(run_result.is_ok());
    }
}
