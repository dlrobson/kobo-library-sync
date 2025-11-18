use std::net::{Ipv4Addr, SocketAddr};

use axum::serve::Listener;
use tokio::io::{AsyncRead, AsyncWrite};

pub struct FakeIo;

impl AsyncRead for FakeIo {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        unimplemented!()
    }
}

impl AsyncWrite for FakeIo {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        unimplemented!()
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        unimplemented!()
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        unimplemented!()
    }
}

pub struct FakeListener {
    port: u16,
}

/// Types that can listen for connections.
impl Listener for FakeListener {
    /// The listener's IO type.
    type Io = FakeIo;

    /// The listener's address type.
    type Addr = SocketAddr;

    /// Accept a new incoming connection to this listener.
    ///
    /// If the underlying accept call can return an error, this function must
    /// take care of logging and retrying.
    fn accept(&mut self) -> impl Future<Output = (Self::Io, Self::Addr)> + Send {
        async move {
            // Implementation of accepting a connection goes here
            std::future::pending::<()>().await;
            unreachable!()
        }
    }

    /// Returns the local address that this listener is bound to.
    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        let ipv4_addr = Ipv4Addr::new(0, 0, 0, 0);
        Ok(SocketAddr::new(std::net::IpAddr::V4(ipv4_addr), self.port))
    }
}

impl FakeListener {
    /// Creates a new FakeListener instance.
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}
