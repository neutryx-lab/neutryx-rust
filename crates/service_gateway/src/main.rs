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
//! ## REST API v2 (New - uses facade APIs)
//! - `POST /api/v2/price` - Price a single instrument
//! - `POST /api/v2/price/batch` - Price a portfolio
//! - `POST /api/v2/curves/build` - Build a yield curve
//! - `POST /api/v2/curves/discount-factor` - Get discount factor
//! - `POST /api/v2/curves/forward-rate` - Get forward rate
//!
//! ## REST API v1 (Legacy - placeholder implementations)
//! - `POST /api/v1/price` - Price a single instrument
//! - `POST /api/v1/price/batch` - Price a portfolio
//! - `GET /api/v1/portfolio/graph` - Get Portfolio computation graph
//! - `GET /api/v1/portfolio/trades` - List Portfolio trades
//! - `GET /health` - Health check
//!
//! ## WebSocket
//! - `GET /ws` - Real-time graph updates (`select_trades`, `subgraph_update`)
//!
//! ## gRPC (Tonic) - Planned
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
mod services;
mod state;

pub use error::ServerError;
pub use rest::{GraphAppState, WsAppState};
pub use state::AppState;

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

        // Create new AppState for v2 endpoints
        let app_state = Arc::new(AppState::new());
        info!(
            "AppState initialised with curve cache (max 100) and fxvol cache (max 20)"
        );

        // Initialise graph state with sample portfolio for v1 compatibility
        let graph_state = match GraphAppState::new_with_sample(50, 5) {
            Ok(state) => {
                info!(
                    "Sample portfolio created with {} trades",
                    state.portfolio.trade_count()
                );
                Arc::new(state)
            }
            Err(e) => {
                tracing::warn!("Failed to create graph state: {}. Using v2 router only.", e);
                // Fall back to v2 router without graph endpoints
                let app = rest::create_router_with_state(app_state);
                let listener = tokio::net::TcpListener::bind(addr).await?;
                axum::serve(listener, app).await?;
                return Ok(());
            }
        };

        // Create WebSocket state wrapping graph state
        let ws_state = Arc::new(WsAppState::new(graph_state));
        info!("WebSocket endpoint enabled at /ws");

        // Use the full router with both v1 and v2 endpoints
        let app = rest::create_full_router(app_state, ws_state);

        info!("API v2 endpoints available at /api/v2/*");
        info!("Legacy v1 endpoints available at /api/v1/*");

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
