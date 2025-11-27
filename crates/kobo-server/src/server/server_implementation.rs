//! Server builder module for configurable server construction.

pub use self::implementation::{Server, ServerBuilder};
mod implementation {
    use std::net::SocketAddr;

    use axum::{ServiceExt, body::Body, serve::Listener};
    use tokio::task::JoinHandle;
    use tokio_util::sync::CancellationToken;

    use crate::server::{
        listener::{IntoListener, TokioTcpListener},
        router::create_router,
        state::server_state::ServerState,
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

    /// Builder for configuring and creating Server instances.
    pub struct ServerBuilder<L> {
        listener_builder: L,
        cancellation_token: CancellationToken,
        port: u16,
        frontend_url: String,
        enable_request_logging: bool,
        enable_response_logging: bool,
    }

    impl ServerBuilder<TokioTcpListener> {
        /// Creates a new `ServerBuilder` with default configuration.
        ///
        /// # Arguments
        /// * `frontend_url` - The frontend URL to use for URL rewriting
        pub fn new(cancellation_token: CancellationToken) -> Self {
            Self {
                listener_builder: TokioTcpListener,
                cancellation_token,
                port: 8080,
                frontend_url: "http://localhost:8080".to_owned(),
                enable_request_logging: false,
                enable_response_logging: false,
            }
        }
    }

    impl<L> ServerBuilder<L> {
        /// Enables request logging middleware.
        pub fn enable_request_logging(mut self, enable: bool) -> Self {
            self.enable_request_logging = enable;
            self
        }

        /// Enables response logging middleware.
        pub fn enable_response_logging(mut self, enable: bool) -> Self {
            self.enable_response_logging = enable;
            self
        }

        /// Sets the frontend URL for URL rewriting.
        ///
        /// # Arguments
        /// * `frontend_url` - The frontend URL to use
        pub fn frontend_url<T: Into<String>>(mut self, frontend_url: T) -> Self {
            self.frontend_url = frontend_url.into();
            self
        }

        /// Sets the port for the server to bind to.
        ///
        /// # Arguments
        /// * `port` - The port number to bind the server to
        pub fn port(mut self, port: u16) -> Self {
            self.port = port;
            self
        }

        /// Sets the listener builder for the server.
        ///
        /// # Arguments
        /// * `listener_builder` - The listener builder to use for creating the listener
        #[cfg(test)]
        pub fn listener_builder<N>(self, listener_builder: N) -> ServerBuilder<N> {
            ServerBuilder {
                listener_builder,
                cancellation_token: self.cancellation_token,
                port: self.port,
                frontend_url: self.frontend_url,
                enable_request_logging: self.enable_request_logging,
                enable_response_logging: self.enable_response_logging,
            }
        }

        /// Builds and starts the server with the configured options.
        ///
        /// # Arguments
        /// * `cancellation_token` - Token for graceful shutdown coordination
        ///
        /// # Errors
        /// Returns an error if the server fails to start.
        pub async fn build(self) -> anyhow::Result<Server>
        where
            L: IntoListener + Send,
            <L::Listener as Listener>::Io: Send + Unpin + 'static,
        {
            let listener = self.listener_builder.into_listener(self.port).await?;
            let app_state = ServerState::builder(self.frontend_url).build()?;
            let app = create_router(
                self.enable_request_logging,
                self.enable_response_logging,
                app_state,
            );
            let address = listener.local_addr()?;

            let cancellation_token_clone = self.cancellation_token.clone();
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

            Ok(Server {
                address,
                cancellation_token: self.cancellation_token,
                handle: server_handle,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::server::listener::FakeListenerBuilder;

    // Helper function to create a basic server builder for testing
    fn create_test_server_builder() -> ServerBuilder<FakeListenerBuilder> {
        ServerBuilder::new(CancellationToken::new()).listener_builder(FakeListenerBuilder)
    }

    #[tokio::test]
    async fn server_starts_with_fake_listener() {
        let server = create_test_server_builder().build().await;
        assert!(server.is_ok());
    }

    #[tokio::test]
    async fn server_returns_correct_ip_address() {
        let server = create_test_server_builder().build().await.unwrap();
        assert_eq!(server.address().ip().to_string(), "0.0.0.0");
    }

    #[tokio::test]
    async fn server_shutdown_completes_successfully() {
        let server = create_test_server_builder().build().await.unwrap();
        let shutdown_result = server.shutdown().await;
        assert!(shutdown_result.is_ok());
    }
}
