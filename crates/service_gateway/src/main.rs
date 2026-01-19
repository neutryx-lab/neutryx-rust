//! Neutryx Server - gRPC/REST API for XVA Pricing
//!
//! This is the production integration point for the Neutryx XVA pricing
//! library.
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
//! - `GET /api/v1/portfolio/graph` - Get Portfolio computation graph
//! - `GET /api/v1/portfolio/trades` - List Portfolio trades
//! - `GET /health` - Health check
//!
//! ## WebSocket
//! - `GET /ws` - Real-time graph updates (select_trades, subgraph_update)
//!
//! ## gRPC (Tonic)
//! - `PricingService.PriceInstrument` - Price a single instrument
//! - `PricingService.PricePortfolio` - Price a portfolio (streaming)
//! - `CalibrationService.Calibrate` - Calibrate model parameters

use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod rest;

pub use error::ServerError;
pub use rest::{GraphAppState, WsAppState};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialise tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Neutryx Server...");

    // Load configuration from unified settings
    let config = config::ServerConfig::load()?;

    info!("Configuration loaded");
    info!("  REST enabled: {}", config.rest_enabled);
    info!("  gRPC enabled: {}", config.grpc_enabled);

    // Start REST server
    #[cfg(feature = "rest")]
    if config.rest_enabled {
        let addr: SocketAddr = config.rest_addr.parse()?;
        info!("Starting REST server on {}", addr);

        // Initialise graph state with sample portfolio
        let graph_state = match GraphAppState::new_with_sample(50, 5) {
            Ok(state) => {
                info!(
                    "Sample portfolio created with {} trades",
                    state.portfolio.trade_count()
                );
                Arc::new(state)
            }
            Err(e) => {
                tracing::warn!("Failed to create graph state: {}. Using basic router.", e);
                // Fall back to basic router without graph endpoints
                let app = rest::create_router();
                let listener = tokio::net::TcpListener::bind(addr).await?;
                axum::serve(listener, app).await?;
                return Ok(());
            }
        };

        // Create WebSocket state wrapping graph state
        let ws_state = Arc::new(WsAppState::new(graph_state));
        info!("WebSocket endpoint enabled at /ws");

        let app = rest::create_router_with_ws_state(ws_state);

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
