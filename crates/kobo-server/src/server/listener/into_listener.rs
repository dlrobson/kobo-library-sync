//! Listener abstraction for configurable server listeners.

use std::net::SocketAddr;

use axum::serve::Listener;

/// Trait for types that can be converted into a listener for the server.
/// This allows abstracting over different listener types (TCP, fake, etc.)
#[async_trait::async_trait]
pub trait IntoListener {
    type Listener: Listener<Addr = SocketAddr> + Send + 'static;

    /// Convert this value into a listener that can accept connections.
    ///
    /// # Errors
    /// May return an error if listener creation fails (e.g., port binding fails).
    async fn into_listener(self, port: u16) -> anyhow::Result<Self::Listener>
    where
        <Self::Listener as Listener>::Io: Send + Unpin + 'static;
}

pub struct TokioTcpListener;

/// Implementation for u16 port numbers - creates a TCP listener bound to the port.
#[async_trait::async_trait]
impl IntoListener for TokioTcpListener {
    type Listener = tokio::net::TcpListener;

    async fn into_listener(self, port: u16) -> anyhow::Result<Self::Listener> {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
        Ok(listener)
    }
}
