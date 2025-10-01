//! Server components for the Kobo proxy application.

mod middleware;
mod router;
mod routes;
mod server_impl;
mod state;
mod utils;

pub use server_impl::Server;
#[cfg(test)]
use state::client::stub_kobo_client::StubKoboClient;
