//! Client abstraction for making requests to the Kobo API.

use anyhow::Result;
use axum::{body::Body, extract::Request};
use http_body_util::BodyExt as _;
use hyper::Response;
use hyper_tls::HttpsConnector;
use hyper_util::client::legacy::{Client, connect::HttpConnector};

/// Trait representing a client capable of forwarding requests to the Kobo API.
#[async_trait::async_trait]
pub trait KoboClient: Send + Sync {
    /// Forwards a request to the Kobo API and returns the response.
    async fn request(&self, request: Request) -> Result<Response<Body>>;
}

#[async_trait::async_trait]
impl KoboClient for Client<HttpsConnector<HttpConnector>, Body> {
    async fn request(&self, request: Request) -> Result<Response<Body>> {
        let response = Client::request(self, request).await?;
        let (parts, body) = response.into_parts();
        Ok(Response::from_parts(
            parts,
            Body::from_stream(body.into_data_stream()),
        ))
    }
}

#[cfg(test)]
pub mod stub_kobo_client {
    use std::collections::VecDeque;
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use anyhow::anyhow;
    use axum::http::{HeaderMap, Method, Uri};

    use super::*;

    /// Representation of a request captured by the [`StubKoboClient`].
    #[derive(Clone, Debug)]
    pub struct RecordedRequest {
        pub method: Method,
        pub uri: Uri,
        pub headers: HeaderMap,
        pub body: Vec<u8>,
    }

    /// Test-only stub client that returns predefined responses and records requests.
    #[derive(Debug, Default)]
    pub struct StubKoboClient {
        responses: Mutex<VecDeque<Result<Response<Body>>>>,
        recorded_requests: Mutex<Vec<RecordedRequest>>,
    }

    impl StubKoboClient {
        pub fn new() -> Self {
            Self::default()
        }

        fn get_responses_lock(&self) -> MutexGuard<'_, VecDeque<Result<Response<Body>>>> {
            self.responses
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
        }

        fn get_recorded_requests_lock(&self) -> MutexGuard<'_, Vec<RecordedRequest>> {
            self.recorded_requests
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
        }

        pub fn enqueue_response(&self, response: Response<Body>) {
            self.get_responses_lock().push_back(Ok(response));
        }

        pub fn enqueue_error(&self, error: anyhow::Error) {
            self.get_responses_lock().push_back(Err(error));
        }

        pub fn recorded_requests(&self) -> Vec<RecordedRequest> {
            self.get_recorded_requests_lock().clone()
        }
    }

    #[async_trait::async_trait]
    impl KoboClient for StubKoboClient {
        async fn request(&self, request: Request) -> Result<Response<Body>> {
            let (parts, body) = request.into_parts();
            let body_bytes = body.collect().await?.to_bytes();

            let recorded = RecordedRequest {
                method: parts.method.clone(),
                uri: parts.uri.clone(),
                headers: parts.headers.clone(),
                body: body_bytes.to_vec(),
            };

            self.get_recorded_requests_lock().push(recorded);

            self.get_responses_lock()
                .pop_front()
                .unwrap_or_else(|| Err(anyhow!("No stubbed response configured")))
        }
    }
}
