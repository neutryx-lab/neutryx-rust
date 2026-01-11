//! FrictionalBank Demo CLI
//!
//! Entry point for running FrictionalBank demo scenarios.

use anyhow::Result;
use frictional_bank::prelude::*;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("frictional_bank=info".parse()?))
        .init();

    tracing::info!("FrictionalBank Demo Starting...");

    // Load configuration
    let config = DemoConfig::load_or_default();
    tracing::info!("Demo mode: {:?}", config.mode);

    // TODO: Implement workflow execution based on CLI args
    tracing::info!("FrictionalBank Demo Ready");

    Ok(())
}
