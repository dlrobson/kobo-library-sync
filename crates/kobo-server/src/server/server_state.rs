use hyper_tls::HttpsConnector;
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};

/// Shared application state
#[derive(Clone)]
pub struct ServerState {
    /// HTTP client to forward requests to Kobo API
    pub client: Client<HttpsConnector<HttpConnector>, axum::body::Body>,
}

impl ServerState {
    /// Creates a new instance of the application state
    pub fn new() -> Self {
        Self {
            client: Client::builder(TokioExecutor::new()).build(HttpsConnector::new()),
        }
    }
}
