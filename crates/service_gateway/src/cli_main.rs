//! Neutryx CLI - Command Line Operations for XVA Pricing
//!
//! This is the operational entry point for the Neutryx XVA pricing library.
//!
//! # Commands
//!
//! - `neutryx calibrate` - Calibrate model parameters from market data
//! - `neutryx price --portfolio <file>` - Price a portfolio of trades
//! - `neutryx report` - Generate risk reports
//! - `neutryx check` - Check system configuration
//! - `neutryx demo` - Run architecture demonstration

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> anyhow::Result<()> {
    // Initialise tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = service_gateway::cli::Cli::parse();
    service_gateway::cli::run(cli)?;

    Ok(())
}
