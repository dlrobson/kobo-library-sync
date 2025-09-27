//! A simple web server using Axum framework

use kobo_server::{App, CommandLineArguments};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command_line_arguments = CommandLineArguments::parse_arguments();
    let app = App::new();

    app.run(command_line_arguments).await
}
