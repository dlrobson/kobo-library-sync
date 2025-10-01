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
    /// Creates a new instance of the application state with a production HTTP client
    pub fn new() -> Self {
        let client: Client<HttpsConnector<HttpConnector>, Body> =
            Client::builder(TokioExecutor::new()).build(HttpsConnector::new());

        let client: Arc<dyn KoboClient> = Arc::new(client);

        Self { client }
    }

    #[cfg(test)]
    /// Creates a new instance with a stub client for testing
    /// Returns both the `ServerState` and the `StubKoboClient` for test access
    pub fn new_null() -> (Self, Arc<crate::server::StubKoboClient>) {
        use crate::server::StubKoboClient;
        let stub = Arc::new(StubKoboClient::new());
        let client: Arc<dyn KoboClient> = stub.clone();
        (Self { client }, stub)
    }
}
