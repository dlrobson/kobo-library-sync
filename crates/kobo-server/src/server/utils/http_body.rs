//! HTTP body utilities for handling compression and encoding.
//!
//! This module provides shared functionality for working with HTTP bodies,
//! including gzip compression/decompression and encoding detection.

use std::{
    borrow::Cow,
    io::{Read as _, Write as _},
};

use anyhow::Result;
use axum::{
    body::{Body, Bytes, HttpBody},
    response::Response,
};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use http_body_util::BodyExt as _;
use hyper::StatusCode;

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
/// Returns a tuple containing an HTTP status code and error message if the body cannot be read.
pub async fn buffer_body<B>(body: B) -> Result<Bytes, (StatusCode, String)>
where
    B: HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read body: {err}"),
            ));
        }
    };

    Ok(bytes)
}

/// Decompresses gzip-encoded bytes to a string.
///
/// # Returns
///
/// Returns the decompressed string.
///
/// # Errors
///
/// Returns an error if the gzip content cannot be decompressed or
/// if the decompressed content is not valid UTF-8.
pub fn decompress_gzip(bytes: &Bytes) -> Result<String> {
    let mut gz = GzDecoder::new(&bytes[..]);
    let mut text = String::new();
    gz.read_to_string(&mut text)?;
    Ok(text)
}

/// Compresses a string using gzip encoding.
///
/// # Returns
///
/// Returns the compressed bytes.
///
/// # Errors
///
/// Returns an error if the compression fails.
pub fn compress_gzip(text: &str) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(text.as_bytes())?;
    Ok(encoder.finish()?)
}

/// Reads the body of a response and returns it as bytes.
///
/// # Returns
///
/// Returns the response body as `Bytes`.
///
/// # Errors
///
/// Returns a `StatusCode` error if the body cannot be read.
pub async fn read_response_body(
    response: Response,
) -> Result<(hyper::http::response::Parts, Bytes), StatusCode> {
    let (parts, body) = response.into_parts();

    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            tracing::error!("Failed to read response body: {err}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    Ok((parts, bytes))
}

/// Checks if a response is gzip-encoded based on its headers.
///
/// # Returns
///
/// Returns `true` if the response has a `content-encoding: gzip` header.
pub fn is_gzip_encoded(headers: &hyper::HeaderMap) -> bool {
    headers.get("content-encoding").is_some_and(|v| v == "gzip")
}

/// Decodes a response body based on its encoding type.
///
/// # Returns
///
/// Returns the decoded body as a string.
///
/// # Errors
///
/// Returns a `StatusCode` error if decoding fails.
pub fn decode_response_body(bytes: &Bytes, is_gzipped: bool) -> Result<Cow<'_, str>, StatusCode> {
    if is_gzipped {
        let decompressed = decompress_gzip(bytes).map_err(|err| {
            tracing::error!("Failed to decompress gzip response: {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        Ok(Cow::Owned(decompressed))
    } else {
        Ok(String::from_utf8_lossy(bytes))
    }
}

/// Encodes a string into a response body, optionally compressing it.
///
/// # Returns
///
/// Returns the encoded body.
///
/// # Errors
///
/// Returns a `StatusCode` error if encoding fails.
pub fn encode_response_body(text: &str, should_compress: bool) -> Result<Body, StatusCode> {
    if should_compress {
        let compressed = compress_gzip(text).map_err(|err| {
            tracing::error!("Failed to compress response: {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        Ok(Body::from(compressed))
    } else {
        Ok(Body::from(text.to_string()))
    }
}
