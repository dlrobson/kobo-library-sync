//! The Router module for the Kobo server, defining routes and middleware.

use axum::{Router, middleware};
use tower::ServiceBuilder;
use tower_http::normalize_path::NormalizePathLayer;

use crate::server::{
    middleware::{kobo_store_fallback::kobo_store_fallback, request_logging},
    state::server_state::ServerState,
};

/// Creates and configures the Axum router with default server state.
pub fn create_router(enable_request_logging: bool, enable_response_logging: bool) -> Router {
    create_router_with_state(
        enable_request_logging,
        enable_response_logging,
        ServerState::new(),
    )
}

/// Creates and configures the Axum router using the provided state.
fn create_router_with_state(
    enable_request_logging: bool,
    enable_response_logging: bool,
    state: ServerState,
) -> Router {
    let router = Router::new().fallback(kobo_store_fallback).layer(
        ServiceBuilder::new()
            // Removes double leading slashes and removes trailing slashes. The Kobo device always
            // sends a double leading slash in its requests (e.g., "//library"), so we normalize the path to ensure consistent routing.
            .layer(NormalizePathLayer::trim_trailing_slash())
            .option_layer(
                enable_request_logging.then(|| middleware::from_fn(request_logging::log_requests)),
            )
            .option_layer(
                enable_response_logging
                    .then(|| middleware::from_fn(request_logging::log_responses)),
            ),
    );

    router.with_state(state)
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;
    use std::sync::Arc;

    use anyhow::anyhow;
    use axum::{
        body::Body,
        http::{Request, Response, StatusCode, header::HOST},
    };
    use flate2::{Compression, write::GzEncoder};
    use http_body_util::BodyExt as _;
    use hyper::Method;
    use tower::ServiceExt as _;
    use tracing_test::traced_test;

    use super::*;
    use crate::server::StubKoboClient;
    use crate::server::state::{client::KoboClient, server_state::ServerState};

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

    fn build_router_with_stub(
        enable_request_logging: bool,
        enable_response_logging: bool,
    ) -> (Router, Arc<StubKoboClient>) {
        let stub = Arc::new(StubKoboClient::new());
        let client: Arc<dyn KoboClient> = stub.clone();
        let state = ServerState::with_client(client);
        (
            create_router_with_state(enable_request_logging, enable_response_logging, state),
            stub,
        )
    }

    mod request_logging_tests {
        use super::*;
        #[tokio::test]
        #[traced_test]
        async fn request_logging_layer_logs_requests() {
            let (router, stub) = build_router_with_stub(true, false);
            stub.enqueue_response(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from(TEST_RESPONSE))
                    .expect("failed to build stub response"),
            );

            let response = router
                .oneshot(build_request())
                .await
                .expect("service should return a response");

            assert_eq!(response.status(), StatusCode::OK);
            assert!(logs_contain("Incoming Request"));
            assert!(logs_contain(TEST_BODY));
        }

        #[tokio::test]
        #[traced_test]
        async fn response_logging_layer_logs_responses() {
            let (router, stub) = build_router_with_stub(false, true);
            stub.enqueue_response(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from(TEST_RESPONSE))
                    .expect("failed to build stub response"),
            );

            let response = router
                .oneshot(build_request())
                .await
                .expect("service should return a response");

            assert_eq!(response.status(), StatusCode::OK);
            assert!(logs_contain("Outgoing Response"));
            assert!(logs_contain(TEST_RESPONSE));
        }

        #[tokio::test]
        #[traced_test]
        async fn response_logging_layer_handles_gzip_body() {
            let (router, stub) = build_router_with_stub(false, true);
            let gzip_body = gzip_bytes(TEST_RESPONSE);
            stub.enqueue_response(
                Response::builder()
                    .status(StatusCode::OK)
                    .header("content-encoding", "gzip")
                    .body(Body::from(gzip_body))
                    .expect("failed to build stub response"),
            );

            let response = router
                .oneshot(build_request())
                .await
                .expect("service should return a response");

            assert_eq!(response.status(), StatusCode::OK);
            assert!(logs_contain("Outgoing Response"));
            assert!(logs_contain(TEST_RESPONSE));
        }
    }

    mod kobo_store_fallback_tests {
        use super::*;

        #[tokio::test]
        async fn fallback_returns_stubbed_response() {
            let (router, stub) = build_router_with_stub(false, false);
            stub.enqueue_response(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from(TEST_RESPONSE))
                    .expect("failed to build stub response"),
            );

            let response = router
                .oneshot(build_request())
                .await
                .expect("service should return a response");

            assert_eq!(response.status(), StatusCode::OK);
            let body = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(&body[..], TEST_RESPONSE.as_bytes());
        }

        #[tokio::test]
        async fn fallback_overwrites_host_header_for_forwarded_requests() {
            let (router, stub) = build_router_with_stub(false, false);
            stub.enqueue_response(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from(TEST_RESPONSE))
                    .expect("failed to build stub response"),
            );

            let response = router
                .oneshot(build_request())
                .await
                .expect("service should return a response");

            assert_eq!(response.status(), StatusCode::OK);

            let recorded = stub.recorded_requests();
            let forwarded = recorded.first().expect("expected a recorded request");

            assert_eq!(
                forwarded.uri.authority().unwrap().as_str(),
                "storeapi.kobo.com"
            );
            assert_eq!(forwarded.uri.scheme_str(), Some("https"));
            let host_header = forwarded
                .headers
                .get(HOST)
                .expect("host header should be set");
            assert_eq!(host_header, "storeapi.kobo.com");
            assert_eq!(forwarded.method, Method::GET);
            assert_eq!(forwarded.body, TEST_BODY.as_bytes());
        }

        #[tokio::test]
        async fn fallback_removes_transfer_encoding_header() {
            let (router, stub) = build_router_with_stub(false, false);
            stub.enqueue_response(
                Response::builder()
                    .status(StatusCode::OK)
                    .header("transfer-encoding", "chunked")
                    .body(Body::from(TEST_RESPONSE))
                    .expect("failed to build stub response"),
            );

            let response = router
                .oneshot(build_request())
                .await
                .expect("service should return a response");

            assert_eq!(response.status(), StatusCode::OK);
            assert!(response.headers().get("transfer-encoding").is_none());
        }

        #[tokio::test]
        async fn fallback_returns_bad_gateway_when_client_errors() {
            let (router, stub) = build_router_with_stub(false, false);
            stub.enqueue_error(anyhow!("stubbed failure"));

            let response = router
                .oneshot(build_request())
                .await
                .expect("service should return a response");

            assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        }

        #[tokio::test]
        async fn multiple_leading_slashes_are_normalized() {
            let (router, stub) = build_router_with_stub(false, false);
            stub.enqueue_response(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from(TEST_RESPONSE))
                    .expect("failed to build stub response"),
            );

            let request = Request::builder()
                .uri("///some/path")
                .body(Body::from(TEST_BODY))
                .expect("failed to build request");

            let response = router
                .oneshot(request)
                .await
                .expect("service should return a response");

            assert_eq!(response.status(), StatusCode::OK);

            let recorded = stub.recorded_requests();
            let forwarded = recorded.first().expect("expected a recorded request");

            assert_eq!(forwarded.uri.path(), "/some/path");
        }

        #[tokio::test]
        async fn trailing_slash_is_removed() {
            let (router, stub) = build_router_with_stub(false, false);
            stub.enqueue_response(
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from(TEST_RESPONSE))
                    .expect("failed to build stub response"),
            );

            let request = Request::builder()
                .uri("/some/path//")
                .body(Body::from(TEST_BODY))
                .expect("failed to build request");

            let response = router
                .oneshot(request)
                .await
                .expect("service should return a response");

            assert_eq!(response.status(), StatusCode::OK);

            let recorded = stub.recorded_requests();
            let forwarded = recorded.first().expect("expected a recorded request");

            assert_eq!(forwarded.uri.path(), "/some/path");
        }
    }
}
