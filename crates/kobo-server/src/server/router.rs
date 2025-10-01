//! The Router module for the Kobo server, defining routes and middleware.

use axum::{Router, middleware, routing::get};
use tower::ServiceBuilder;
use tower_http::normalize_path::NormalizePathLayer;

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
) -> Router {
    Router::new()
        .route("/v1/initialization", get(initialization_handler))
        .fallback(kobo_store_request)
        .layer(
            ServiceBuilder::new()
                // Removes double leading slashes and removes trailing slashes. The Kobo device
                // always sends a double leading slash in its requests (e.g.,
                // "//library"), so we normalize the path to ensure consistent
                // routing.
                .layer(NormalizePathLayer::trim_trailing_slash())
                .option_layer(
                    enable_request_logging
                        .then(|| middleware::from_fn(request_logging::log_requests)),
                )
                .option_layer(
                    enable_response_logging
                        .then(|| middleware::from_fn(request_logging::log_responses)),
                ),
        )
        .with_state(server_state)
}
