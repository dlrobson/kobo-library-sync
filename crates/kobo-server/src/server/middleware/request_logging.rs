//! Request and response logging middleware for the Axum server.
//!
//! This module provides middleware functions that can be used to log incoming
//! HTTP requests and outgoing HTTP responses, including their headers and body
//! content. The middleware supports both plain text and gzip-compressed content.

pub use implementation::{log_requests, log_responses};

mod implementation {
    use std::borrow::Cow;

    use anyhow::Result;
    use axum::{
        body::Body,
        extract::Request,
        middleware::Next,
        response::{IntoResponse, Response},
    };
    use hyper::StatusCode;

    use crate::server::utils::http_body::{buffer_body, decode_response_body, is_gzip_encoded};

    /// Logs an incoming HTTP request (method, URI, headers, body; gzip-aware).
    pub async fn log_requests(
        request: Request,
        next: Next,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let (parts, body) = request.into_parts();
        let bytes = buffer_body(body).await?;

        let is_gzipped = is_gzip_encoded(&parts.headers);
        let body_repr = decode_response_body(&bytes, is_gzipped).unwrap_or_else(|e| {
            tracing::warn!("Failed to decode request body: {e}");
            Cow::Owned("<unprintable body>".into())
        });

        tracing::info!(
            method = %parts.method,
            uri = %parts.uri,
            headers = ?parts.headers,
            body = %body_repr,
            "Incoming Request"
        );
        let req = Request::from_parts(parts, Body::from(bytes));

        Ok(next.run(req).await)
    }

    /// Logs an outgoing HTTP response (status, headers, body; gzip-aware).
    pub async fn log_responses(
        request: Request,
        next: Next,
    ) -> Result<impl IntoResponse, (StatusCode, String)> {
        let res = next.run(request).await;

        let (parts, body) = res.into_parts();
        let bytes = buffer_body(body).await?;
        let is_gzipped = is_gzip_encoded(&parts.headers);

        let body_repr = decode_response_body(&bytes, is_gzipped).unwrap_or_else(|e| {
            tracing::warn!("Failed to decode response body: {e}");
            Cow::Owned("<unprintable body>".into())
        });

        tracing::info!(
            status = %parts.status,
            headers = ?parts.headers,
            body = %body_repr,
            "Outgoing Response"
        );

        let res = Response::from_parts(parts, Body::from(bytes));

        Ok(res)
    }
}
#[cfg(test)]
mod tests {
    use std::io::Write as _;
    use std::sync::Arc;

    use axum::{
        body::Body,
        http::{Request, Response},
    };
    use flate2::{Compression, write::GzEncoder};
    use tower::ServiceExt as _;
    use tracing_test::traced_test;

    use crate::server::{
        router::create_router,
        state::{fake_kobo_client::FakeKoboClient, server_state::ServerState},
    };

    const TEST_BODY: &str = "test body";
    const TEST_RESPONSE: &str = "stubbed response";

    fn build_request() -> Request<Body> {
        Request::builder()
            .uri("/")
            .body(Body::from(TEST_BODY))
            .expect("failed to build request")
    }

    fn gzip_bytes(input: &str) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(input.as_bytes())
            .expect("failed to write gzip contents");
        encoder.finish().expect("failed to finish gzip encoding")
    }

    #[tokio::test]
    #[traced_test]
    async fn request_logging_layer_logs_requests() {
        let stub = Arc::new(FakeKoboClient::new());
        let state = ServerState::builder("http://frontend.test")
            .client(stub.clone())
            .build()
            .unwrap();
        let router = create_router(true, false, state);

        stub.enqueue_response(
            Response::builder()
                .body(Body::from(TEST_RESPONSE))
                .expect("failed to build stub response"),
        );

        let _request = router
            .oneshot(build_request())
            .await
            .expect("service should return a response");

        assert!(logs_contain(TEST_BODY));
    }

    #[tokio::test]
    #[traced_test]
    async fn response_logging_layer_logs_responses() {
        let stub = Arc::new(FakeKoboClient::new());
        let state = ServerState::builder("http://frontend.test")
            .client(stub.clone())
            .build()
            .unwrap();
        let router = create_router(false, true, state);

        stub.enqueue_response(
            Response::builder()
                .body(Body::from(TEST_RESPONSE))
                .expect("failed to build stub response"),
        );

        let _request = router
            .oneshot(build_request())
            .await
            .expect("service should return a response");

        assert!(logs_contain(TEST_RESPONSE));
    }

    #[tokio::test]
    #[traced_test]
    async fn response_logging_layer_handles_gzip_body() {
        let stub = Arc::new(FakeKoboClient::new());
        let state = ServerState::builder("http://frontend.test")
            .client(stub.clone())
            .build()
            .unwrap();
        let router = create_router(false, true, state);

        let gzip_body = gzip_bytes(TEST_RESPONSE);
        stub.enqueue_response(
            Response::builder()
                .header("content-encoding", "gzip")
                .body(Body::from(gzip_body))
                .expect("failed to build stub response"),
        );

        let _request = router
            .oneshot(build_request())
            .await
            .expect("service should return a response");

        assert!(logs_contain(TEST_RESPONSE));
    }
}
