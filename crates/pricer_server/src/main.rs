//! Neutryx Pricer Server
//!
//! REST API server for XVA pricing library.

use clap::Parser;
use pricer_server::config::{build_config, CliArgs as ConfigCliArgs, ConfigError};
use pricer_server::server::Server;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

fn init_tracing(log_level: &str) {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let cli_args: ConfigCliArgs = args.into();
    let config = build_config(&cli_args)?;

    // Initialize tracing
    init_tracing(config.log_level.as_filter_str());

    tracing::info!("Neutryx Pricer Server v{}", pricer_server::VERSION);
    tracing::info!(
        host = %config.host,
        port = %config.port,
        log_level = %config.log_level,
        environment = %config.environment,
        kernel_enabled = %config.kernel_enabled,
        api_key_required = %config.api_key_required,
        rate_limit_rpm = %config.rate_limit_rpm,
        "Server configuration loaded"
    );

    // Create and start the server
    let server = Server::new(config);
    tracing::info!(address = %server.socket_addr(), "Starting server");

    server.run().await?;

    Ok(())
}
