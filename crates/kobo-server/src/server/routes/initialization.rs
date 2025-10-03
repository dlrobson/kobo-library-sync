//! Handler for the initialization route.

use axum::response::Response;

use crate::server::{
    routes::{constants::KOBO_API_URL, kobo_store_request::kobo_store_request},
    state::server_state::ServerState,
    utils::http_body::{
        decode_response_body, encode_response_body, is_gzip_encoded, read_response_body,
    },
};

/// Handler for the `/v1/initialization` endpoint. Forwards to Kobo API and rewrites
/// Kobo API base URLs in the JSON body to the configured frontend URL, preserving
/// gzip encoding if present.
pub async fn initialization_handler(
    state: axum::extract::State<ServerState>,
    request: axum::extract::Request,
) -> Result<Response, hyper::StatusCode> {
    let frontend_url = state.frontend_url.clone();
    let response = kobo_store_request(state, request).await?;
    let (parts, bytes) = read_response_body(response).await?;
    let gz = is_gzip_encoded(&parts.headers);
    let body_text = decode_response_body(&bytes, gz)?;
    let modified = body_text.replace(KOBO_API_URL, frontend_url.as_str());
    let body = encode_response_body(&modified, gz)?;
    Ok(Response::from_parts(parts, body))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::Body,
        http::{Request, Response, StatusCode},
    };
    use http_body_util::BodyExt as _;
    use hyper::Method;
    use tower::ServiceExt as _;

    use crate::server::{
        router::create_router,
        state::{client::stub_kobo_client::StubKoboClient, server_state::ServerState},
        utils::http_body::{compress_gzip, decompress_gzip},
    };

    #[tokio::test]
    async fn test_initialization_handler_replaces_urls_in_gzipped_response() {
        let original_json = r#"{"Resources":{"library_sync":"https://storeapi.kobo.com/v1/library/sync","user_profile":"https://storeapi.kobo.com/v1/user/profile"}}"#;
        let compressed_json = compress_gzip(original_json).expect("Failed to compress JSON");
        let configured_frontend = "http://placeholder.local";
        let stub = Arc::new(StubKoboClient::new());
        let state = ServerState::builder()
            .frontend_url(configured_frontend)
            .client(stub.clone())
            .build();
        let router = create_router(false, false, state);
        stub.enqueue_response(
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json; charset=utf-8")
                .header("content-encoding", "gzip")
                .body(Body::from(compressed_json))
                .expect("Failed to build stub response"),
        );
        let request = Request::builder()
            .method(Method::GET)
            .uri("http://example.com:8080/v1/initialization")
            .body(Body::empty())
            .expect("Failed to build request");
        let response = router
            .oneshot(request)
            .await
            .expect("Service should return a response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("content-encoding").unwrap(), "gzip");
        let (_, body) = response.into_parts();
        let bytes = body
            .collect()
            .await
            .expect("Failed to collect body")
            .to_bytes();
        let decompressed = decompress_gzip(&bytes).expect("Failed to decompress response");
        assert!(!decompressed.contains("https://storeapi.kobo.com"));
        assert!(decompressed.contains(configured_frontend));
        assert!(decompressed.contains(&format!("{configured_frontend}/v1/library/sync")));
        assert!(decompressed.contains(&format!("{configured_frontend}/v1/user/profile")));
    }

    #[tokio::test]
    async fn test_initialization_handler_replaces_urls_in_plain_response() {
        let original_json = r#"{"Resources":{"library_sync":"https://storeapi.kobo.com/v1/library/sync","user_profile":"https://storeapi.kobo.com/v1/user/profile"}}"#;
        let configured_frontend = "https://frontend.example";
        let stub = Arc::new(StubKoboClient::new());
        let state = ServerState::builder()
            .frontend_url(configured_frontend)
            .client(stub.clone())
            .build();
        let router = create_router(false, false, state);
        stub.enqueue_response(
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json; charset=utf-8")
                .body(Body::from(original_json))
                .expect("Failed to build stub response"),
        );
        let request = Request::builder()
            .method(Method::GET)
            .uri("https://localhost:3000/v1/initialization")
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
        assert!(!body_text.contains("https://storeapi.kobo.com"));
        assert!(body_text.contains(configured_frontend));
        assert!(body_text.contains(&format!("{configured_frontend}/v1/library/sync")));
        assert!(body_text.contains(&format!("{configured_frontend}/v1/user/profile")));
    }
}
