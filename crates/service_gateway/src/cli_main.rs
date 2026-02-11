//! Neutryx CLI - Command Line Operations for XVA Pricing.

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = service_gateway::cli::Cli::parse();
    service_gateway::cli::run(cli)?;

    Ok(())
}
