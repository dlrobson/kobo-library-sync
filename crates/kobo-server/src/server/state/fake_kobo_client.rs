//! Client abstraction for making requests to the Kobo API.

pub use implementation::FakeKoboClient;

mod implementation {
    use std::collections::VecDeque;
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use anyhow::{Result, anyhow};
    use axum::http::{HeaderMap, Method, Uri};
    use axum::{body::Body, extract::Request};
    use http_body_util::BodyExt as _;
    use hyper::Response;

    use crate::server::state::client::KoboClient;

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
    pub struct FakeKoboClient {
        responses: Mutex<VecDeque<Result<Response<Body>>>>,
        recorded_requests: Mutex<Vec<RecordedRequest>>,
    }

    impl FakeKoboClient {
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
    impl KoboClient for FakeKoboClient {
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

mod tests {
    use axum::body::Body;
    use axum::extract::Request;
    use axum::http::{HeaderValue, Method, StatusCode, Uri};
    use hyper::Response;

    use crate::server::state::client::KoboClient as _;
    use crate::server::state::fake_kobo_client::FakeKoboClient;

    #[test]
    fn new_creates_empty_stub_client() {
        let client = FakeKoboClient::new();
        assert!(client.recorded_requests().is_empty());
    }

    #[test]
    fn default_creates_empty_stub_client() {
        let client = FakeKoboClient::default();
        assert!(client.recorded_requests().is_empty());
    }

    #[test]
    fn recorded_requests_returns_empty_vec_initially() {
        let client = FakeKoboClient::new();

        let requests = client.recorded_requests();

        assert!(requests.is_empty());
    }

    #[tokio::test]
    async fn request_records_method() {
        let client = FakeKoboClient::new();
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();
        client.enqueue_response(response);

        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com")
            .body(Body::empty())
            .unwrap();

        let _unused = client.request(request).await;
        let recorded = &client.recorded_requests()[0];

        assert_eq!(recorded.method, Method::POST);
    }

    #[tokio::test]
    async fn request_records_uri() {
        let client = FakeKoboClient::new();
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();
        client.enqueue_response(response);

        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/path")
            .body(Body::empty())
            .unwrap();

        let _unused = client.request(request).await;
        let recorded = &client.recorded_requests()[0];

        assert_eq!(recorded.uri, Uri::from_static("https://example.com/path"));
    }

    #[tokio::test]
    async fn request_records_headers() {
        let client = FakeKoboClient::new();
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();
        client.enqueue_response(response);

        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        let _unused = client.request(request).await;
        let recorded = &client.recorded_requests()[0];

        assert_eq!(
            recorded.headers.get("Content-Type"),
            Some(&HeaderValue::from_static("application/json"))
        );
    }

    #[tokio::test]
    async fn request_records_body() {
        let client = FakeKoboClient::new();
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();
        client.enqueue_response(response);

        let body_content = "test body content";
        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.com")
            .body(Body::from(body_content))
            .unwrap();

        let _unused = client.request(request).await;
        let recorded = &client.recorded_requests()[0];

        assert_eq!(recorded.body, body_content.as_bytes());
    }

    #[tokio::test]
    async fn request_returns_enqueued_response() {
        let client = FakeKoboClient::new();
        let response = Response::builder()
            .status(StatusCode::CREATED)
            .body(Body::from("test response"))
            .unwrap();
        client.enqueue_response(response);

        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com")
            .body(Body::empty())
            .unwrap();

        let result = client.request(request).await.unwrap();

        assert_eq!(result.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn request_returns_enqueued_error() {
        let client = FakeKoboClient::new();
        let error = anyhow::anyhow!("Test error message");
        client.enqueue_error(error);

        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com")
            .body(Body::empty())
            .unwrap();

        let result = client.request(request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn request_error_contains_expected_message() {
        let client = FakeKoboClient::new();
        let error = anyhow::anyhow!("Custom error message");
        client.enqueue_error(error);

        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com")
            .body(Body::empty())
            .unwrap();

        let result = client.request(request).await;

        assert_eq!(result.unwrap_err().to_string(), "Custom error message");
    }

    #[tokio::test]
    async fn request_returns_error_when_no_response_queued() {
        let client = FakeKoboClient::new();

        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com")
            .body(Body::empty())
            .unwrap();

        let result = client.request(request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn request_no_response_queued_has_expected_message() {
        let client = FakeKoboClient::new();

        let request = Request::builder()
            .method(Method::GET)
            .uri("https://example.com")
            .body(Body::empty())
            .unwrap();

        let result = client.request(request).await;

        assert_eq!(
            result.unwrap_err().to_string(),
            "No stubbed response configured"
        );
    }

    #[tokio::test]
    async fn multiple_requests_first_response_has_correct_status() {
        let client = FakeKoboClient::new();
        let response1 = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();
        let response2 = Response::builder()
            .status(StatusCode::CREATED)
            .body(Body::empty())
            .unwrap();

        client.enqueue_response(response1);
        client.enqueue_response(response2);

        let request1 = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/1")
            .body(Body::empty())
            .unwrap();

        let result1 = client.request(request1).await.unwrap();

        assert_eq!(result1.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn multiple_requests_second_response_has_correct_status() {
        let client = FakeKoboClient::new();
        let response1 = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();
        let response2 = Response::builder()
            .status(StatusCode::CREATED)
            .body(Body::empty())
            .unwrap();

        client.enqueue_response(response1);
        client.enqueue_response(response2);

        let request1 = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/1")
            .body(Body::empty())
            .unwrap();
        let request2 = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/2")
            .body(Body::empty())
            .unwrap();

        let _unused = client.request(request1).await;
        let result2 = client.request(request2).await.unwrap();

        assert_eq!(result2.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn multiple_requests_all_get_recorded() {
        let client = FakeKoboClient::new();
        let response1 = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();
        let response2 = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();

        client.enqueue_response(response1);
        client.enqueue_response(response2);

        let request1 = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/1")
            .body(Body::empty())
            .unwrap();
        let request2 = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/2")
            .body(Body::empty())
            .unwrap();

        let _unused = client.request(request1).await;
        let _unused = client.request(request2).await;

        assert_eq!(client.recorded_requests().len(), 2);
    }

    #[tokio::test]
    async fn recorded_requests_maintains_order() {
        let client = FakeKoboClient::new();
        let response1 = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();
        let response2 = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();

        client.enqueue_response(response1);
        client.enqueue_response(response2);

        let request1 = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/1")
            .body(Body::empty())
            .unwrap();
        let request2 = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/2")
            .body(Body::empty())
            .unwrap();

        let _unused = client.request(request1).await;
        let _unused = client.request(request2).await;

        let logged_requests = client.recorded_requests();

        assert_eq!(logged_requests[0].method, Method::GET);
    }

    #[tokio::test]
    async fn recorded_requests_second_request_has_correct_method() {
        let client = FakeKoboClient::new();
        let response1 = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();
        let response2 = Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap();

        client.enqueue_response(response1);
        client.enqueue_response(response2);

        let request1 = Request::builder()
            .method(Method::GET)
            .uri("https://example.com/1")
            .body(Body::empty())
            .unwrap();
        let request2 = Request::builder()
            .method(Method::POST)
            .uri("https://example.com/2")
            .body(Body::empty())
            .unwrap();

        let _unused = client.request(request1).await;
        let _unused = client.request(request2).await;

        let logged_requests = client.recorded_requests();

        assert_eq!(logged_requests[1].method, Method::POST);
    }
}
