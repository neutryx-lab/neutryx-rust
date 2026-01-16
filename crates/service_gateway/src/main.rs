//! Neutryx Server - gRPC/REST API for XVA Pricing
//!
//! This is the production integration point for the Neutryx XVA pricing library.
//!
//! # Architecture
//!
//! As part of the **S**ervice layer in the A-I-P-S architecture, this crate
//! provides network interfaces for microservice deployment.
//!
//! # Endpoints
//!
//! ## REST (Axum)
//! - `POST /api/v1/price` - Price a single instrument
//! - `POST /api/v1/price/batch` - Price a portfolio
//! - `POST /api/v1/calibrate` - Calibrate model parameters
//! - `GET /api/v1/health` - Health check
//!
//! ## gRPC (Tonic)
//! - `PricingService.PriceInstrument` - Price a single instrument
//! - `PricingService.PricePortfolio` - Price a portfolio (streaming)
//! - `CalibrationService.Calibrate` - Calibrate model parameters

use std::net::SocketAddr;

use anyhow::Result;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod rest;

pub use error::ServerError;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialise tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Neutryx Server...");

    // Load configuration
    let config = config::ServerConfig::from_env()?;

    info!("Configuration loaded");
    info!("  REST enabled: {}", config.rest_enabled);
    info!("  gRPC enabled: {}", config.grpc_enabled);

    // Start REST server
    #[cfg(feature = "rest")]
    if config.rest_enabled {
        let addr: SocketAddr = config.rest_addr.parse()?;
        info!("Starting REST server on {}", addr);

        let app = rest::create_router();

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
    }

    // gRPC server requires tonic integration (feature-gated).
    #[cfg(feature = "grpc")]
    if config.grpc_enabled {
        info!("gRPC server: feature enabled but implementation pending");
    }

    Ok(())
}
