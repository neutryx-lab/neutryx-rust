//! Neutryx Server - gRPC/REST API for XVA Pricing.

use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
#[cfg(not(feature = "demo"))]
use service_gateway::WsAppState;
use service_gateway::{rest, AppState, GraphAppState};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting Neutryx Server...");

    let config = service_gateway::config::ServerConfig::load()?;

    info!("Configuration loaded");
    info!("  REST enabled: {}", config.rest_enabled);
    info!("  gRPC enabled: {}", config.grpc_enabled);

    #[cfg(feature = "rest")]
    if config.rest_enabled {
        let addr: SocketAddr = config.rest_addr.parse()?;
        info!("Starting REST server on {}", addr);

        let app_state = Arc::new(AppState::new());
        info!("AppState initialised with curve cache (max 100) and fxvol cache (max 20)");

        let graph_state = match GraphAppState::new_with_sample(50, 5) {
            Ok(state) => {
                info!(
                    "Loaded {} FpML trades from demo/data/trades/fpml/",
                    state.trade_count()
                );
                Arc::new(state)
            }
            Err(e) => {
                tracing::warn!("Failed to create graph state: {}. Using v2 router only.", e);
                let app = rest::create_router_with_state(app_state);
                let listener = tokio::net::TcpListener::bind(addr).await?;
                axum::serve(listener, app).await?;
                return Ok(());
            }
        };

        #[cfg(feature = "demo")]
        let app = {
            let _ = graph_state;
            info!("Demo GUI mode enabled - serving static files from demo/gui/dist/");
            info!("Demo API endpoints available at /api/*");
            rest::create_demo_router(app_state)
        };

        #[cfg(not(feature = "demo"))]
        let app = {
            let ws_state = Arc::new(WsAppState::new(graph_state));
            info!("WebSocket endpoint enabled at /ws");
            info!("API v2 endpoints available at /api/v2/*");
            info!("Legacy v1 endpoints available at /api/v1/*");
            rest::create_full_router(app_state, ws_state)
        };

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
    }

    #[cfg(feature = "grpc")]
    if config.grpc_enabled {
        info!("gRPC server: feature enabled but implementation pending");
    }

    Ok(())
}
