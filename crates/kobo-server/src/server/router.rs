//! The Router module for the Kobo server, defining routes and middleware.

use axum::{Router, middleware};

use crate::server::{
    kobo_store_fallback::kobo_store_fallback, request_logging, server_state::ServerState,
};

/// Creates and configures the Axum router
pub fn create_router(enable_request_logging: bool, enable_response_logging: bool) -> Router {
    let mut router = Router::new().fallback(kobo_store_fallback);

    if enable_request_logging {
        router = router.layer(middleware::from_fn(request_logging::log_requests));
    }

    if enable_response_logging {
        router = router.layer(middleware::from_fn(request_logging::log_responses));
    }
    router.with_state(ServerState::new())
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt as _;
    use tracing_test::traced_test;

    use super::*;

    const TEST_BODY: &str = "test body";

    fn build_request() -> Request<Body> {
        Request::builder()
            .uri("/")
            .body(Body::new(Body::from(TEST_BODY)))
            .expect("failed to build request")
    }

    #[tokio::test]
    async fn request_without_routes_falls_back_to_kobo_store() {
        let router = create_router(false, false);

        let response = router
            .oneshot(build_request())
            .await
            .expect("service should return a response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    #[traced_test]
    async fn request_logging_layer_logs_requests() {
        let router = create_router(true, false);

        let response = router
            .oneshot(build_request())
            .await
            .expect("service should return a response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(logs_contain("Incoming Request"));
        assert!(logs_contain(TEST_BODY));
    }

    #[tokio::test]
    #[traced_test]
    async fn response_logging_layer_logs_responses() {
        let router = create_router(false, true);

        let response = router
            .oneshot(build_request())
            .await
            .expect("service should return a response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(logs_contain("Outgoing Response"));
    }
}
