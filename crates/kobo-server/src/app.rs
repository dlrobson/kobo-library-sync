//! Application entry point and orchestration.

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

use crate::{command_line_arguments::CommandLineArguments, server::start_server};

/// The main application struct that orchestrates the entire application lifecycle.
pub struct App;

impl App {
    /// Initialize and run the application
    ///
    /// # Errors
    ///
    /// If the server fails to start.
    pub async fn run() -> Result<()> {
        let cli = CommandLineArguments::parse();

        Self::initialize_logging(&cli.log_level);

        start_server(cli.port).await
    }

    /// Initialize the logging subsystem with the specified log level.
    fn initialize_logging(log_level: &str) {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::builder()
                    .parse(log_level)
                    .unwrap_or_default(),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
}
