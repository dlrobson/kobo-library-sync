//! The Router module for the Kobo server, defining routes and middleware.

pub use implementation::create_router;

mod implementation {
    use axum::{Router, middleware, routing::get};
    use tower::{Layer as _, ServiceBuilder};
    use tower_http::normalize_path::{NormalizePath, NormalizePathLayer};

    use crate::server::{
        middleware::request_logging,
        routes::{initialization::initialization_handler, kobo_store_request::kobo_store_request},
        state::server_state::ServerState,
    };

    /// Creates and configures the Axum router with default server state.
    pub fn create_router(
        enable_request_logging: bool,
        enable_response_logging: bool,
        server_state: ServerState,
    ) -> NormalizePath<Router<()>> {
        let router = Router::new()
            .route("/v1/initialization", get(initialization_handler))
            .fallback(kobo_store_request)
            .layer(
                ServiceBuilder::new()
                    .option_layer(
                        enable_request_logging
                            .then(|| middleware::from_fn(request_logging::log_requests)),
                    )
                    .option_layer(
                        enable_response_logging
                            .then(|| middleware::from_fn(request_logging::log_responses)),
                    ),
            )
            .with_state(server_state);

        // Removes double leading slashes and removes trailing slashes. The Kobo device
        // always sends a double leading slash in its requests (e.g.,
        // "//library"), so we normalize the path to ensure consistent
        // routing.
        NormalizePathLayer::trim_trailing_slash().layer(router)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{body::Body, http::Request};
    use tower::ServiceExt as _;

    use super::*;
    use crate::server::state::{fake_kobo_client::FakeKoboClient, server_state::ServerState};

    #[tokio::test]
    async fn multiple_leading_and_trailing_slashes_are_normalized_by_layer() {
        let stub = Arc::new(FakeKoboClient::new());
        let state = ServerState::builder("http://frontend.test")
            .client(stub.clone())
            .build();
        stub.enqueue_response(
            axum::http::Response::builder()
                .status(200)
                .body(Body::from("ok"))
                .unwrap(),
        );
        let router = create_router(false, false, state);

        let request = Request::builder()
            .uri("////some/path///")
            .body(Body::empty())
            .unwrap();

        let response = router
            .oneshot(request)
            .await
            .expect("router should respond");
        assert_eq!(response.status(), 200);
        // We can't directly read normalized path from response, but normalization allowed fallback
        // to match and stub to be consumed. Indirect evidence: stub saw request with normalized
        // path.
        let recorded = stub.recorded_requests();
        let forwarded = recorded.first().expect("expected forwarded request");
        assert_eq!(forwarded.uri.path(), "/some/path");
    }
}
