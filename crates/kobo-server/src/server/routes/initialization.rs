//! Handler for the initialization route.

use axum::{
    extract::{Request, State},
    response::Response,
};
use hyper::StatusCode;

use crate::server::{
    routes::{constants::KOBO_API_URL, kobo_store_request::kobo_store_request},
    state::server_state::ServerState,
    utils::http_body::{
        decode_response_body, encode_response_body, is_gzip_encoded, read_response_body,
    },
};

/// Handler for the `/v1/initialization` endpoint. This is the initialization route for Kobo
/// devices. It forwards the request to the Kobo API, modifies the response to replace Kobo URLs
/// with the server's URL,
///
/// # Errors
///
/// Returns a `StatusCode` error if any step in the request forwarding or response processing fails.
pub async fn initialization_handler(
    state: State<ServerState>,
    request: Request,
) -> Result<Response, hyper::StatusCode> {
    let request_url = request
        .uri()
        .scheme_str()
        .zip(request.uri().authority())
        .map(|(scheme, authority)| format!("{scheme}://{authority}"))
        .ok_or_else(|| {
            tracing::error!("Failed to extract scheme and authority from request URI");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = kobo_store_request(state, request).await?;

    let (parts, bytes) = read_response_body(response).await?;
    let is_gzipped = is_gzip_encoded(&parts.headers);
    let body_text = decode_response_body(&bytes, is_gzipped)?;

    let modified_text = body_text.replace(KOBO_API_URL, request_url.as_str());
    let final_body = encode_response_body(&modified_text, is_gzipped)?;
    Ok(Response::from_parts(parts, final_body))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, Response, StatusCode},
    };
    use http_body_util::BodyExt as _;
    use hyper::Method;
    use tower::ServiceExt as _;

    use crate::server::{
        router::create_router,
        state::server_state::ServerState,
        utils::http_body::{compress_gzip, decompress_gzip},
    };

    #[tokio::test]
    async fn test_initialization_handler_replaces_urls_in_gzipped_response() {
        // Create a sample JSON response with Kobo URLs
        let original_json = r#"{"Resources":{"library_sync":"https://storeapi.kobo.com/v1/library/sync","user_profile":"https://storeapi.kobo.com/v1/user/profile"}}"#;

        // Compress the original JSON
        let compressed_json = compress_gzip(original_json).expect("Failed to compress JSON");

        // Set up the test environment
        let (state, stub) = ServerState::new_null();
        let router = create_router(false, false, state);

        // Configure the stub to return a gzipped response
        stub.enqueue_response(
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json; charset=utf-8")
                .header("content-encoding", "gzip")
                .body(Body::from(compressed_json))
                .expect("Failed to build stub response"),
        );

        // Make a request to the initialization endpoint with a specific host
        let test_host = "example.com:8080";
        let request = Request::builder()
            .method(Method::GET)
            .uri(format!("http://{test_host}/v1/initialization"))
            .body(Body::empty())
            .expect("Failed to build request");

        let response = router
            .oneshot(request)
            .await
            .expect("Service should return a response");

        // Verify the response
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("content-encoding").unwrap(), "gzip");

        let (_, body) = response.into_parts();
        let bytes = body
            .collect()
            .await
            .expect("Failed to collect body")
            .to_bytes();
        let decompressed = decompress_gzip(&bytes).expect("Failed to decompress response");

        // Verify that Kobo URLs have been replaced with the request host
        assert!(!decompressed.contains("https://storeapi.kobo.com"));
        assert!(decompressed.contains(&format!("http://{test_host}")));

        // Verify specific URL replacements
        let expected_library_sync = format!("http://{test_host}/v1/library/sync");
        let expected_user_profile = format!("http://{test_host}/v1/user/profile");
        assert!(decompressed.contains(&expected_library_sync));
        assert!(decompressed.contains(&expected_user_profile));
    }

    #[tokio::test]
    async fn test_initialization_handler_replaces_urls_in_plain_response() {
        // Create a sample JSON response with Kobo URLs
        let original_json = r#"{"Resources":{"library_sync":"https://storeapi.kobo.com/v1/library/sync","user_profile":"https://storeapi.kobo.com/v1/user/profile"}}"#;

        // Set up the test environment
        let (state, stub) = ServerState::new_null();
        let router = create_router(false, false, state);

        // Configure the stub to return a plain text response
        stub.enqueue_response(
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json; charset=utf-8")
                .body(Body::from(original_json))
                .expect("Failed to build stub response"),
        );

        // Make a request to the initialization endpoint with a specific host
        let test_host = "localhost:3000";
        let request = Request::builder()
            .method(Method::GET)
            .uri(format!("https://{test_host}/v1/initialization"))
            .body(Body::empty())
            .expect("Failed to build request");

        let response = router
            .oneshot(request)
            .await
            .expect("Service should return a response");

        // Verify the response
        assert_eq!(response.status(), StatusCode::OK);
        let (_, body) = response.into_parts();
        let bytes = body
            .collect()
            .await
            .expect("Failed to collect body")
            .to_bytes();
        let body_text = String::from_utf8(bytes.to_vec()).expect("Failed to decode response");

        // Verify that Kobo URLs have been replaced with the request host
        assert!(!body_text.contains("https://storeapi.kobo.com"));
        assert!(body_text.contains(&format!("https://{test_host}")));

        // Verify specific URL replacements
        let expected_library_sync = format!("https://{test_host}/v1/library/sync");
        let expected_user_profile = format!("https://{test_host}/v1/user/profile");
        assert!(body_text.contains(&expected_library_sync));
        assert!(body_text.contains(&expected_user_profile));
    }

    #[tokio::test]
    async fn test_initialization_handler_handles_missing_scheme_or_authority() {
        // Set up the test environment
        let (state, _stub) = ServerState::new_null();
        let router = create_router(false, false, state);

        // Make a request without proper scheme/authority (relative URI)
        let request = Request::builder()
            .method(Method::GET)
            .uri("/v1/initialization") // No scheme or authority
            .body(Body::empty())
            .expect("Failed to build request");

        let response = router
            .oneshot(request)
            .await
            .expect("Service should return a response");

        // Should return an internal server error due to missing scheme/authority
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_initialization_handler_preserves_different_schemes() {
        let original_json =
            r#"{"Resources":{"library_sync":"https://storeapi.kobo.com/v1/library/sync"}}"#;

        // Test with HTTPS scheme
        let (state, stub) = ServerState::new_null();
        let router = create_router(false, false, state.clone());

        stub.enqueue_response(
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json; charset=utf-8")
                .body(Body::from(original_json))
                .expect("Failed to build stub response"),
        );

        let request = Request::builder()
            .method(Method::GET)
            .uri("https://secure.example.com:443/v1/initialization")
            .body(Body::empty())
            .expect("Failed to build request");

        let response = router
            .oneshot(request)
            .await
            .expect("Service should return a response");

        assert_eq!(response.status(), StatusCode::OK);
        let (_, body) = response.into_parts();
        let bytes = body
            .collect()
            .await
            .expect("Failed to collect body")
            .to_bytes();
        let body_text = String::from_utf8(bytes.to_vec()).expect("Failed to decode response");

        // Should use HTTPS scheme in replacement
        assert!(body_text.contains("https://secure.example.com:443/v1/library/sync"));
        assert!(!body_text.contains("https://storeapi.kobo.com"));
    }
}
