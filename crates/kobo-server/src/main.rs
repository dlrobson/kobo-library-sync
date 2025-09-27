//! A simple web server using Axum framework

use kobo_server::{
    App,
    CommandLineArguments,
};
use tracing_subscriber::{
    layer::SubscriberExt as _,
    util::SubscriberInitExt as _,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let command_line_arguments = CommandLineArguments::parse_arguments();
    initialize_logging(&command_line_arguments.log_level);
    let app = App::new();
    app.run(command_line_arguments).await
}

/// Initialize the logging subsystem with the specified log level.
fn initialize_logging(log_level: &str) {
    let mut parse_error: Option<String> = None;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::builder()
                .parse(log_level)
                .inspect_err(|e| {
                    parse_error = Some(format!("Failed to parse log level '{log_level}': {e}"));
                })
                .unwrap_or_default(),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    if let Some(message) = parse_error {
        tracing::error!("{message}");
    }
}
