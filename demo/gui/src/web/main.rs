//! Web dashboard entry point.
//!
//! Usage: cargo run -p demo_gui --features web --bin demo-web

use demo_gui::web;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "demo_gui=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Server address
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    println!();
    println!("  FrictionalBank Web Dashboard");
    println!("  ============================");
    println!();
    println!("  Starting server at http://{}", addr);
    println!();
    println!("  API Endpoints:");
    println!("    GET  /api/health     - Health check");
    println!("    GET  /api/portfolio  - Portfolio data");
    println!("    POST /api/portfolio  - Price portfolio");
    println!("    GET  /api/exposure   - Exposure metrics");
    println!("    GET  /api/risk       - Risk metrics");
    println!("    WS   /api/ws         - WebSocket updates");
    println!();
    println!("  Press Ctrl+C to stop the server");
    println!();

    web::run_server(addr).await
}
