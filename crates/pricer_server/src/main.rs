//! Neutryx Pricer Server
//!
//! REST API server for XVA pricing library.

use clap::Parser;
use pricer_server::config::{build_config, CliArgs as ConfigCliArgs, ConfigError};
use std::path::PathBuf;

/// Neutryx Pricer Server - REST API for XVA pricing
#[derive(Parser, Debug)]
#[command(name = "pricer_server")]
#[command(version, about, long_about = None)]
struct Args {
    /// Configuration file path (TOML format)
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Host address to bind to
    #[arg(long, env = "PRICER_SERVER_HOST")]
    host: Option<String>,

    /// Port to listen on
    #[arg(short, long, env = "PRICER_SERVER_PORT")]
    port: Option<u16>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, env = "PRICER_LOG_LEVEL")]
    log_level: Option<String>,
}

impl From<Args> for ConfigCliArgs {
    fn from(args: Args) -> Self {
        ConfigCliArgs {
            config_file: args.config,
            host: args.host,
            port: args.port,
            log_level: args.log_level,
        }
    }
}

fn main() -> Result<(), ConfigError> {
    let args = Args::parse();
    let cli_args: ConfigCliArgs = args.into();
    let config = build_config(&cli_args)?;

    println!("Neutryx Pricer Server v{}", pricer_server::VERSION);
    println!("Configuration:");
    println!("  Host: {}", config.host);
    println!("  Port: {}", config.port);
    println!("  Log Level: {}", config.log_level);
    println!("  Environment: {}", config.environment);
    println!("  Kernel Integration: {}", config.kernel_enabled);
    println!("  API Key Required: {}", config.api_key_required);
    println!("  Rate Limit: {} rpm", config.rate_limit_rpm);

    // TODO: Start Axum server (Task 2.1)
    println!("\nServer not yet implemented. Configuration loaded successfully.");

    Ok(())
}
