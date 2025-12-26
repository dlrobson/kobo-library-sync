//! Shared application state for the Kobo server.

pub use implementation::ServerState;

mod implementation {
    use std::sync::Arc;

    use anyhow::Result;
    use axum::body::Body;
    use hyper_util::{client::legacy::Client, rt::TokioExecutor};

    use crate::server::state::client::{HttpsConnector, KoboClient};

    /// Shared application state
    #[derive(Clone)]
    pub struct ServerState {
        /// HTTP client to forward requests to Kobo API
        pub client: Arc<dyn KoboClient>,
        /// The Frontend URL that devices should point to (scheme + authority)
        pub frontend_url: String,
    }

    impl ServerState {
        /// Start building a `ServerState` with a required frontend URL (scheme + host\[:port\]).
        pub fn builder<T: Into<String>>(frontend_url: T) -> ServerStateBuilder {
            ServerStateBuilder {
                frontend_url: frontend_url.into(),
                client: None,
            }
        }
    }

    /// Builder for `ServerState`.
    pub struct ServerStateBuilder {
        frontend_url: String,
        client: Option<Arc<dyn KoboClient>>,
    }

    impl ServerStateBuilder {
        /// Provide a custom HTTP client (e.g. test stub).
        #[cfg(test)]
        pub fn client(mut self, client: Arc<dyn KoboClient>) -> Self {
            self.client = Some(client);
            self
        }

        /// Build the `ServerState`.
        pub fn build(self) -> Result<ServerState> {
            let frontend_url = self.frontend_url;

            let client = if let Some(client) = self.client {
                client
            } else {
                let connector = hyper_rustls::HttpsConnectorBuilder::new()
                    .with_webpki_roots()
                    .https_only()
                    .enable_http1()
                    .enable_http2()
                    .build();
                let client: Client<HttpsConnector, Body> =
                    Client::builder(TokioExecutor::new()).build(connector);
                let client: Arc<dyn KoboClient> = Arc::new(client);
                client
            };

            Ok(ServerState {
                client,
                frontend_url,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_sets_frontend_url() {
        let state = ServerState::builder("https://example.test")
            .build()
            .unwrap();
        assert_eq!(state.frontend_url, "https://example.test");
    }

    #[test]
    fn builder_defaults_frontend_url() {
        // No implicit default anymore; test explicit usage
        let state = ServerState::builder("http://localhost:1234")
            .build()
            .unwrap();
        assert_eq!(state.frontend_url, "http://localhost:1234");
    }
}
