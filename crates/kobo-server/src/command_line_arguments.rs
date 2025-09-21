//! Contains the command line arguments for the kobo-server application.

use clap::Parser;

/// Command line arguments for the kobo-server application.
#[derive(Clone, Debug, Default, Parser)]
#[command(author, version, about, long_about = None)]
pub struct CommandLineArguments {
    /// The log level for the application.
    #[arg(short, long, default_value = "info")]
    pub log_level: String,

    /// The port to listen on.
    #[arg(short, long, default_value_t = 3000)]
    pub port: u16,

    /// Enable request logging middleware.
    #[arg(long, default_value_t = false)]
    pub enable_request_logging: bool,
}

impl CommandLineArguments {
    /// Parse the command line arguments. Implemented manually to avoid exposing
    /// the `clap` dependency in the public API.
    #[must_use] 
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr as _;

    #[test]
    fn test_default_log_level_is_valid() {
        let args = CommandLineArguments::parse_from(["kobo-server"]);
        assert_eq!(args.log_level, "info");

        assert!(
            tracing::Level::from_str(&args.log_level).is_ok(),
            "Log level should be valid"
        );
    }
}
