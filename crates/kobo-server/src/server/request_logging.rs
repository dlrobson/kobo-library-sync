//! Request and response logging middleware for the Axum server.
//!
//! This module provides middleware functions that can be used to log incoming
//! HTTP requests and outgoing HTTP responses, including their headers and body
//! content. The middleware supports both plain text and gzip-compressed content.

use std::io::Read as _;

use anyhow::Result;
use axum::{
    body::{Body, Bytes, HttpBody},
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use flate2::read::GzDecoder;
use http_body_util::BodyExt as _;

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
/// * HTTP 400 Bad Request if the request body cannot be read.
/// * HTTP 500 Internal Server Error if the request body cannot be read or decoded.
pub async fn log_requests(
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, (hyper::StatusCode, String)> {
    let (parts, body) = request.into_parts();
    let bytes = buffer(body).await?;

    match body_display(
        &bytes,
        &EncodingType::from(parts.headers.get("content-encoding")),
    ) {
        Ok(body) => tracing::info!(
            method = %parts.method,
            uri = %parts.uri,
            headers = ?parts.headers,
            body = %body,
            "Incoming Request"
        ),
        Err(e) => {
            tracing::error!("Failed to decode request body: {e}");
            return Err((hyper::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    }
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
/// Returns an
/// * HTTP 400 Bad Request if the response body cannot be read.
/// * HTTP 500 Internal Server Error if the response body cannot be read or decoded.
pub async fn log_responses(
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, (hyper::StatusCode, String)> {
    let res = next.run(request).await;

    let (parts, body) = res.into_parts();
    let bytes = buffer(body).await?;
    let encoding_type: EncodingType = parts.headers.get("content-encoding").into();
    match body_display(&bytes, &encoding_type) {
        Ok(body) => tracing::info!(
            status = %parts.status,
            headers = ?parts.headers,
            body = %body,
            "Outgoing Response"
        ),
        Err(e) => {
            tracing::error!("Failed to decode response body: {e}");
            return Err((hyper::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    }

    let res = Response::from_parts(parts, Body::from(bytes));

    Ok(res)
}

/// Buffers the entire HTTP body into memory for inspection.
///
/// This helper function collects all bytes from an HTTP body stream,
/// allowing the body content to be logged while preserving it for
/// further processing.
///
/// # Returns
///
/// Returns the body content as `Bytes`.
///
/// # Errors
///
/// Returns an HTTP 400 Bad Request error if the body cannot be read.
async fn buffer<B>(body: B) -> Result<Bytes, (hyper::StatusCode, String)>
where
    B: HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err((
                hyper::StatusCode::BAD_REQUEST,
                format!("failed to read body: {err}"),
            ));
        }
    };

    Ok(bytes)
}

/// Represents the encoding type of HTTP body content.
#[derive(PartialEq)]
enum EncodingType {
    /// Gzip-compressed content
    Gzip,
    /// Plain text content
    Plain,
}

impl From<Option<&hyper::header::HeaderValue>> for EncodingType {
    /// Converts an HTTP Content-Encoding header value to an `EncodingType`.
    ///
    /// # Returns
    ///
    /// Returns `EncodingType::Gzip` if the header indicates gzip encoding,
    /// otherwise returns `EncodingType::Plain`.
    fn from(value: Option<&hyper::header::HeaderValue>) -> Self {
        match value {
            Some(v) if v == hyper::header::HeaderValue::from_static("gzip") => EncodingType::Gzip,
            _ => EncodingType::Plain,
        }
    }
}

/// Converts HTTP body bytes to a displayable string, handling different encodings.
///
/// This function decodes the body content based on its encoding type and
/// returns a displayable representation suitable for logging.
///
/// # Returns
///
/// Returns a boxed `Display` trait object containing the decoded body content.
///
/// # Errors
///
/// Returns an error if gzip-compressed content cannot be decompressed or
/// if the decompressed content is not valid UTF-8.
fn body_display<'a>(
    bytes: &'a Bytes,
    encoding_type: &EncodingType,
) -> Result<Box<dyn std::fmt::Display + 'a>> {
    match encoding_type {
        EncodingType::Gzip => {
            let mut gz = GzDecoder::new(&bytes[..]);
            let mut s = String::new();
            gz.read_to_string(&mut s)?;

            Ok(Box::new(s))
        }
        EncodingType::Plain => Ok(Box::new(String::from_utf8_lossy(bytes))),
    }
}
