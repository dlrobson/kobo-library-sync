//! A simple web server using Axum framework

use kobo_server::App;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    App::run().await
}
