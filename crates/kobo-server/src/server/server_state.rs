//! Shared application state for the Kobo server.

use std::sync::Arc;

use axum::body::Body;
use hyper_tls::HttpsConnector;
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};

use super::client::KoboClient;

/// Shared application state
#[derive(Clone)]
pub struct ServerState {
    /// HTTP client to forward requests to Kobo API
    pub client: Arc<dyn KoboClient>,
}

impl ServerState {
    /// Creates a new instance of the application state
    pub fn new() -> Self {
        let client: Client<HttpsConnector<HttpConnector>, Body> =
            Client::builder(TokioExecutor::new()).build(HttpsConnector::new());

        Self {
            client: Arc::new(client),
        }
    }

    /// Creates a new instance with the provided client implementation.
    #[cfg(test)]
    pub fn with_client(client: Arc<dyn KoboClient>) -> Self {
        Self { client }
    }
}
