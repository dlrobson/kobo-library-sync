//! Client abstraction for making requests to the Kobo API.

pub use implementation::{HttpsConnector, KoboClient};

mod implementation {
    use anyhow::Result;
    use axum::{body::Body, extract::Request};
    use http_body_util::BodyExt as _;
    use hyper::Response;
    use hyper_util::client::legacy::Client;

    /// HTTPS connector using rustls
    pub type HttpsConnector =
        hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>;

    /// Trait representing a client capable of forwarding requests to the Kobo API.
    #[async_trait::async_trait]
    pub trait KoboClient: Send + Sync {
        /// Forwards a request to the Kobo API and returns the response.
        async fn request(&self, request: Request) -> Result<Response<Body>>;
    }

    #[async_trait::async_trait]
    impl KoboClient for Client<HttpsConnector, Body> {
        async fn request(&self, request: Request) -> Result<Response<Body>> {
            let response = Client::request(self, request).await?;
            let (parts, body) = response.into_parts();
            Ok(Response::from_parts(
                parts,
                Body::from_stream(body.into_data_stream()),
            ))
        }
    }
}
