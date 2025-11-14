//! HTTP body utilities for handling compression and encoding.
//!
//! This module provides shared functionality for working with HTTP bodies,
//! including gzip compression/decompression and encoding detection.

pub use implementation::{
    buffer_body, decode_response_body, encode_response_body, is_gzip_encoded, read_response_body,
};
#[cfg(test)]
pub use implementation::{compress_gzip, decompress_gzip};

mod implementation {
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
    /// # Errors
    ///
    /// Returns a tuple containing an HTTP status code and error message if the body cannot be read.
    pub async fn buffer_body<B>(body: B) -> Result<Bytes, (StatusCode, String)>
    where
        B: HttpBody<Data = Bytes>,
        B::Error: std::fmt::Display,
    {
        let collected_bytes = body.collect().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to read body: {e}"),
            )
        })?;

        Ok(collected_bytes.to_bytes())
    }

    /// Decompresses gzip-encoded bytes to a string.
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
    /// # Errors
    ///
    /// Returns a `StatusCode` error if the body cannot be read.
    pub async fn read_response_body(
        response: Response,
    ) -> Result<(hyper::http::response::Parts, Bytes), StatusCode> {
        let (parts, body) = response.into_parts();
        let bytes = buffer_body(body).await.map_err(|(status, _)| status)?;

        Ok((parts, bytes))
    }

    /// Checks if a response is gzip-encoded based on its headers.
    pub fn is_gzip_encoded(headers: &hyper::HeaderMap) -> bool {
        headers.get("content-encoding").is_some_and(|v| v == "gzip")
    }

    /// Decodes a response body based on its encoding type.
    ///
    /// # Errors
    ///
    /// Returns a `StatusCode` error if decoding fails.
    pub fn decode_response_body(
        bytes: &Bytes,
        is_gzipped: bool,
    ) -> Result<Cow<'_, str>, StatusCode> {
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
            Ok(Body::from(text.to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use axum::{
        body::{Body, Bytes},
        http::{HeaderName, HeaderValue, Response},
    };
    use flate2::{Compression, write::GzEncoder};
    use hyper::{HeaderMap, StatusCode};

    use crate::server::utils::http_body::{
        buffer_body, compress_gzip, decode_response_body, decompress_gzip, encode_response_body,
        is_gzip_encoded, read_response_body,
    };

    // Test data
    const TEST_TEXT: &str = "Hello, World! This is a test string for compression.";
    const EMPTY_TEXT: &str = "";

    /// Helper function to create gzipped bytes
    fn create_gzipped_bytes(text: &str) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(text.as_bytes()).unwrap();
        encoder.finish().unwrap()
    }

    #[tokio::test]
    async fn test_buffer_body_success() {
        let body = Body::from(TEST_TEXT);

        let bytes = buffer_body(body).await.unwrap();

        assert_eq!(bytes, TEST_TEXT.as_bytes());
    }

    #[tokio::test]
    async fn test_buffer_body_empty() {
        let body = Body::empty();

        let bytes = buffer_body(body).await.unwrap();

        assert!(bytes.is_empty());
    }

    #[test]
    fn test_decompress_gzip_success() {
        let compressed = create_gzipped_bytes(TEST_TEXT);
        let bytes = Bytes::from(compressed);

        let text = decompress_gzip(&bytes).unwrap();

        assert_eq!(text, TEST_TEXT);
    }

    #[test]
    fn test_decompress_gzip_empty() {
        let compressed = create_gzipped_bytes(EMPTY_TEXT);
        let bytes = Bytes::from(compressed);

        let text = decompress_gzip(&bytes).unwrap();

        assert_eq!(text, EMPTY_TEXT);
    }

    #[test]
    fn test_decompress_gzip_invalid_data() {
        let invalid_data = b"not gzipped data";
        let bytes = Bytes::from(&invalid_data[..]);

        let result = decompress_gzip(&bytes);

        assert!(result.is_err());
    }

    #[test]
    fn test_compress_gzip_success() {
        let compressed = compress_gzip(TEST_TEXT).unwrap();
        let bytes = Bytes::from(compressed);
        let decompressed = decompress_gzip(&bytes).unwrap();

        assert_eq!(decompressed, TEST_TEXT);
    }

    #[test]
    fn test_compress_gzip_empty() {
        let compressed = compress_gzip(EMPTY_TEXT).unwrap();
        let bytes = Bytes::from(compressed);
        let decompressed = decompress_gzip(&bytes).unwrap();

        assert_eq!(decompressed, EMPTY_TEXT);
    }

    #[tokio::test]
    async fn test_read_response_body_status_ok() {
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(TEST_TEXT))
            .unwrap();

        let (parts, _bytes) = read_response_body(response).await.unwrap();

        assert_eq!(parts.status, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_read_response_body_bytes_match() {
        let response = Response::builder().body(Body::from(TEST_TEXT)).unwrap();

        let (_parts, bytes) = read_response_body(response).await.unwrap();

        assert_eq!(bytes, TEST_TEXT.as_bytes());
    }

    #[tokio::test]
    async fn test_read_response_body_with_headers() {
        let response = Response::builder()
            .header("content-type", "application/json")
            .body(Body::from(TEST_TEXT))
            .unwrap();

        let (parts, _bytes) = read_response_body(response).await.unwrap();

        assert_eq!(
            parts.headers.get("content-type").unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_is_gzip_encoded_true() {
        let headers = HeaderMap::from_iter([(
            HeaderName::from_static("content-encoding"),
            HeaderValue::from_static("gzip"),
        )]);

        let result = is_gzip_encoded(&headers);

        assert!(result);
    }

    #[test]
    fn test_is_gzip_encoded_false_no_header() {
        let headers = HeaderMap::new();

        let result = is_gzip_encoded(&headers);

        assert!(!result);
    }

    #[test]
    fn test_is_gzip_encoded_false_different_encoding() {
        let headers = HeaderMap::from_iter([(
            HeaderName::from_static("content-encoding"),
            HeaderValue::from_static("deflate"),
        )]);

        let result = is_gzip_encoded(&headers);

        assert!(!result);
    }

    #[test]
    fn test_is_gzip_encoded_false_multiple_encodings() {
        let mut headers = HeaderMap::new();
        headers.insert("content-encoding", "br, gzip".parse().unwrap());

        let result = is_gzip_encoded(&headers);

        assert!(!result);
    }

    #[test]
    fn test_decode_response_body_plain_text() {
        let bytes = Bytes::from(TEST_TEXT);

        let result = decode_response_body(&bytes, false);

        assert!(result.is_ok());
    }

    #[test]
    fn test_decode_response_body_plain_text_content() {
        let bytes = Bytes::from(TEST_TEXT);

        let decoded = decode_response_body(&bytes, false).unwrap();

        assert_eq!(decoded, TEST_TEXT);
    }

    #[test]
    fn test_decode_response_body_gzipped() {
        let compressed = create_gzipped_bytes(TEST_TEXT);
        let bytes = Bytes::from(compressed);

        let result = decode_response_body(&bytes, true);

        assert!(result.is_ok());
    }

    #[test]
    fn test_decode_response_body_gzipped_content() {
        let compressed = create_gzipped_bytes(TEST_TEXT);
        let bytes = Bytes::from(compressed);

        let decoded = decode_response_body(&bytes, true).unwrap();

        assert_eq!(decoded, TEST_TEXT);
    }

    #[test]
    fn test_decode_response_body_invalid_utf8() {
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        let bytes = Bytes::from(invalid_utf8);

        let result = decode_response_body(&bytes, false);

        assert!(result.is_ok());
    }

    #[test]
    fn test_decode_response_body_invalid_utf8_uses_replacement_chars() {
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        let bytes = Bytes::from(invalid_utf8);

        let decoded = decode_response_body(&bytes, false).unwrap();

        assert!(decoded.contains('�')); // Replacement character
    }

    #[test]
    fn test_decode_response_body_gzip_error() {
        let invalid_gzip = Bytes::from("not gzipped");

        let result = decode_response_body(&invalid_gzip, true);

        assert!(result.is_err());
    }

    #[test]
    fn test_decode_response_body_gzip_error_status() {
        let invalid_gzip = Bytes::from("not gzipped");

        let error = decode_response_body(&invalid_gzip, true).unwrap_err();

        assert_eq!(error, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_encode_response_body_plain() {
        let result = encode_response_body(TEST_TEXT, false);

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_encode_response_body_plain_content() {
        let body = encode_response_body(TEST_TEXT, false).unwrap();
        let bytes = buffer_body(body).await.unwrap();

        assert_eq!(bytes, TEST_TEXT.as_bytes());
    }

    #[tokio::test]
    async fn test_encode_response_body_compressed() {
        let result = encode_response_body(TEST_TEXT, true);

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_encode_response_body_compressed_content() {
        let body = encode_response_body(TEST_TEXT, true).unwrap();
        let bytes = buffer_body(body).await.unwrap();
        let decompressed = decompress_gzip(&bytes).unwrap();

        assert_eq!(decompressed, TEST_TEXT);
    }

    #[tokio::test]
    async fn test_encode_response_body_empty_compressed() {
        let result = encode_response_body(EMPTY_TEXT, true);

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_encode_response_body_empty_compressed_content() {
        let body = encode_response_body(EMPTY_TEXT, true).unwrap();
        let bytes = buffer_body(body).await.unwrap();
        let decompressed = decompress_gzip(&bytes).unwrap();

        assert_eq!(decompressed, EMPTY_TEXT);
    }

    #[test]
    fn test_round_trip_compression_test_text() {
        let compressed = compress_gzip(TEST_TEXT).unwrap();
        let bytes = Bytes::from(compressed);
        let decompressed = decompress_gzip(&bytes).unwrap();

        assert_eq!(decompressed, TEST_TEXT);
    }

    #[test]
    fn test_round_trip_compression_empty_text() {
        let compressed = compress_gzip(EMPTY_TEXT).unwrap();
        let bytes = Bytes::from(compressed);
        let decompressed = decompress_gzip(&bytes).unwrap();

        assert_eq!(decompressed, EMPTY_TEXT);
    }

    #[test]
    fn test_round_trip_compression_single_char() {
        let original_text = "a";

        let compressed = compress_gzip(original_text).unwrap();
        let bytes = Bytes::from(compressed);
        let decompressed = decompress_gzip(&bytes).unwrap();

        assert_eq!(decompressed, original_text);
    }

    #[test]
    fn test_round_trip_compression_unicode() {
        let original_text = "🚀";

        let compressed = compress_gzip(original_text).unwrap();
        let bytes = Bytes::from(compressed);
        let decompressed = decompress_gzip(&bytes).unwrap();

        assert_eq!(decompressed, original_text);
    }
}
