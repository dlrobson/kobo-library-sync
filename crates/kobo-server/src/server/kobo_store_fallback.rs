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

use crate::server::server_state::ServerState;

/// Base URI for the Kobo API.
const KOBO_API_BASE_URI: &str = "storeapi.kobo.com";

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
pub async fn kobo_store_fallback(
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
    request.headers_mut().remove("host");
    request.headers_mut().append(
        "host",
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
