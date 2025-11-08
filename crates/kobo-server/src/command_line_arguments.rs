//! Contains the command line arguments for the kobo-server application.

use clap::Parser;

/// Command line arguments for the kobo-server application.
#[derive(Clone, Debug, Default, Parser)]
#[command(author, version, about, long_about = None)]
pub struct CommandLineArguments {
    /// The log level for the application.
    #[arg(short, long, default_value = "info", env)]
    pub log_level: String,
    /// The port to listen on.
    #[arg(short, long, default_value_t = 8089, env)]
    pub port: u16,
    /// The front end URL the application will be accessed from. This is used
    /// to generate URLs in responses to Kobo devices.
    #[arg(short, long, env)]
    pub frontend_url: Option<String>,
    /// Enable request logging middleware.
    #[arg(short = 'q', long, default_value_t = false, env)]
    pub enable_request_logging: bool,
    /// Enable response logging middleware.
    #[arg(short = 'r', long, default_value_t = false, env)]
    pub enable_response_logging: bool,
}

impl CommandLineArguments {
    /// Parse the command line arguments. Implemented manually to avoid exposing
    /// the `clap` dependency in the public API.
    #[must_use]
    pub fn parse_arguments() -> Self {
        <Self as Parser>::parse()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;

    #[test]
    fn test_default_log_level_is_valid() {
        let args = CommandLineArguments::parse_from(["kobo-server"]);
        assert!(tracing::Level::from_str(&args.log_level).is_ok());
    }
}
