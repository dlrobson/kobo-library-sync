//! Server implementation for the Kobo store.

mod client;
mod kobo_store_fallback;
mod request_logging;
mod router;
mod server_impl;
mod server_state;

#[cfg(test)]
use client::stub_kobo_client::StubKoboClient;
pub use server_impl::Server;
