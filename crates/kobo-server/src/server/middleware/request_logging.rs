//! Request and response logging middleware for the Axum server.
//!
//! This module provides middleware functions that can be used to log incoming
//! HTTP requests and outgoing HTTP responses, including their headers and body
//! content. The middleware supports both plain text and gzip-compressed content.

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

/// Middleware function that logs incoming HTTP requests.
///
/// This middleware captures and logs the HTTP method, URI, headers, and body
/// of incoming requests. It supports both plain text and gzip-compressed bodies.
///
/// # Returns
///
/// Returns the response from the next handler in the chain.
///
/// # Errors
///
/// Returns an
/// * HTTP 500 Internal Server Error if the request body cannot be read (e.g., IO/buffering error).
pub async fn log_requests(
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (parts, body) = request.into_parts();
    let bytes = buffer_body(body).await?;

    let is_gzipped = is_gzip_encoded(&parts.headers);
    let body_repr = match decode_response_body(&bytes, is_gzipped) {
        Ok(body) => body,
        Err(e) => {
            tracing::warn!("Failed to decode request body: {e}");
            Cow::Owned("<unprintable body>".into())
        }
    };

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

/// Middleware function that logs outgoing HTTP responses.
///
/// This middleware captures and logs the HTTP status code, headers, and body
/// of outgoing responses. It supports both plain text and gzip-compressed bodies.
///
/// # Returns
///
/// Returns the response after logging it.
///
/// # Errors
///
/// Returns a
/// * HTTP 500 Internal Server Error if the response body cannot be read (e.g., IO/buffering error).
pub async fn log_responses(
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let res = next.run(request).await;

    let (parts, body) = res.into_parts();
    let bytes = buffer_body(body).await?;
    let is_gzipped = is_gzip_encoded(&parts.headers);

    let body_repr = match decode_response_body(&bytes, is_gzipped) {
        Ok(body) => body,
        Err(e) => {
            tracing::warn!("Failed to decode response body: {e}");
            Cow::Owned("<unprintable body>".into())
        }
    };

    tracing::info!(
        status = %parts.status,
        headers = ?parts.headers,
        body = %body_repr,
        "Outgoing Response"
    );

    let res = Response::from_parts(parts, Body::from(bytes));

    Ok(res)
}
