//! Fallback handler for requests to the Kobo store API

use anyhow::Result;
use axum::{
    extract::{Request, State},
    http::{
        Uri,
        uri::{Parts, Scheme},
    },
    response::{IntoResponse as _, Response},
};

use crate::server::{routes::constants::KOBO_API_BASE_URI, state::server_state::ServerState};

/// Generate URI parts for the Kobo API given a path and query string.
fn generate_kobo_uri_parts(path_and_query: &str) -> Result<Parts> {
    let mut parts = Parts::default();
    parts.scheme = Some(Scheme::HTTPS);
    parts.authority = Some(KOBO_API_BASE_URI.parse()?);
    parts.path_and_query = Some(path_and_query.parse()?);
    Ok(parts)
}

/// Generate a full URI for the Kobo API given a path and query string.
fn generate_kobo_uri(path_and_query: &str) -> Result<Uri> {
    let parts = generate_kobo_uri_parts(path_and_query)?;
    Ok(Uri::from_parts(parts)?)
}

/// Fallback handler that forwards requests to the Kobo store API. Intended to
/// be used as an axum fallback handler.
///
/// # Errors
///
/// Returns a `hyper::StatusCode` error if the request could not be forwarded
/// or if the URI is invalid.
pub async fn kobo_store_request(
    server_state: State<ServerState>,
    mut request: Request,
) -> Result<Response, hyper::StatusCode> {
    let path_and_query = if let Some(pq) = request.uri().path_and_query() {
        pq.as_str()
    } else {
        tracing::error!("Request URI missing path and query");
        return Err(hyper::StatusCode::BAD_REQUEST);
    };

    *request.uri_mut() = generate_kobo_uri(path_and_query).map_err(|e| {
        tracing::error!("Invalid URI: {e}");
        hyper::StatusCode::BAD_REQUEST
    })?;

    // Replace the `host` header to match the Kobo API host. Required since
    // we're forwarding the request to a different host.
    request.headers_mut().insert(
        hyper::header::HOST,
        hyper::header::HeaderValue::from_static(KOBO_API_BASE_URI),
    );

    match server_state.client.request(request).await {
        Ok(mut resp) => {
            // Remove `transfer-encoding` header. The Kobo sync hangs if this
            // header is present in the response.
            resp.headers_mut().remove("transfer-encoding");
            Ok(resp.into_response())
        }
        Err(e) => {
            tracing::error!("Error forwarding request: {e}");
            Err(hyper::StatusCode::BAD_GATEWAY)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::anyhow;
    use axum::{
        Router,
        body::Body,
        http::{Request, Response, StatusCode, header::HOST},
    };
    use http_body_util::BodyExt as _;
    use hyper::Method;
    use tower::ServiceExt as _;
    use tower_http::normalize_path::NormalizePath;

    use crate::server::{
        router::create_router,
        state::{client::stub_kobo_client::StubKoboClient, server_state::ServerState},
    };

    const TEST_BODY: &str = "test body";
    const TEST_RESPONSE: &str = "stubbed response";

    fn build_request() -> Request<Body> {
        Request::builder()
            .uri("/")
            .body(Body::from(TEST_BODY))
            .expect("failed to build request")
    }

    fn build_router_with_stub() -> (NormalizePath<Router<()>>, Arc<StubKoboClient>) {
        let stub = Arc::new(StubKoboClient::new());
        let state = ServerState::builder().client(stub.clone()).build();
        (create_router(false, false, state), stub)
    }

    #[tokio::test]
    async fn fallback_returns_stubbed_response() {
        let (router, stub) = build_router_with_stub();
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
        let (router, stub) = build_router_with_stub();
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
        let (router, stub) = build_router_with_stub();
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
        let (router, stub) = build_router_with_stub();
        stub.enqueue_error(anyhow!("stubbed failure"));

        let response = router
            .oneshot(build_request())
            .await
            .expect("service should return a response");

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn multiple_leading_slashes_are_normalized() {
        let (router, stub) = build_router_with_stub();
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
}
