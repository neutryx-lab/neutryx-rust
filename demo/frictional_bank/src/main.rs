//! FrictionalBank Demo Server
//!
//! HTTP server entry point for Cloud Run deployment.

use std::net::SocketAddr;

use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use frictional_bank::prelude::*;
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Application state shared across handlers
#[derive(Clone)]
struct AppState {
    config: DemoConfig,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("frictional_bank=info".parse()?))
        .init();

    tracing::info!("FrictionalBank Demo Server Starting...");

    // Load configuration
    let config = DemoConfig::load_or_default().with_env_override();
    tracing::info!("Demo mode: {:?}", config.mode);

    // Get port from PORT env var (Cloud Run) or default to 8080
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // Create app state
    let state = AppState { config };

    // Build router
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/api/v1/status", get(status_handler))
        .route("/api/v1/workflow/eod", post(eod_workflow_handler))
        .route("/api/v1/workflow/intraday", post(intraday_workflow_handler))
        .route("/api/v1/workflow/stress", post(stress_workflow_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    tracing::info!("Starting HTTP server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Root handler
async fn root_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "service": "FrictionalBank Demo",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running"
    }))
}

/// Health check endpoint for Cloud Run
async fn health_handler() -> impl IntoResponse {
    StatusCode::OK
}

/// Status response
#[derive(Serialize)]
struct StatusResponse {
    mode: String,
    data_dir: String,
    max_trades: Option<usize>,
}

/// Status endpoint
async fn status_handler(State(state): State<AppState>) -> impl IntoResponse {
    Json(StatusResponse {
        mode: format!("{:?}", state.config.mode),
        data_dir: state.config.data_dir.display().to_string(),
        max_trades: state.config.max_trades,
    })
}

/// Workflow response
#[derive(Serialize)]
struct WorkflowResponse {
    success: bool,
    workflow: String,
    message: String,
    duration_ms: u64,
    trades_processed: usize,
}

/// EOD batch workflow handler
async fn eod_workflow_handler(State(state): State<AppState>) -> impl IntoResponse {
    tracing::info!("Starting EOD batch workflow");

    let workflow = EodBatchWorkflow::new();

    match workflow.run(&state.config, None).await {
        Ok(result) => Json(WorkflowResponse {
            success: result.success,
            workflow: "eod_batch".to_string(),
            message: format!("Processed {} trades", result.trades_processed),
            duration_ms: result.duration_ms,
            trades_processed: result.trades_processed,
        }),
        Err(e) => Json(WorkflowResponse {
            success: false,
            workflow: "eod_batch".to_string(),
            message: format!("Error: {}", e),
            duration_ms: 0,
            trades_processed: 0,
        }),
    }
}

/// Intraday workflow request
#[derive(Deserialize)]
struct IntradayRequest {
    #[serde(default = "default_iterations")]
    iterations: usize,
}

fn default_iterations() -> usize {
    5
}

/// Intraday workflow handler
async fn intraday_workflow_handler(
    State(state): State<AppState>,
    Json(req): Json<IntradayRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "Starting intraday workflow with {} iterations",
        req.iterations
    );

    // Use iterations as max_trades for the workflow
    let mut config = state.config.clone();
    config.max_trades = Some(req.iterations);

    let workflow = IntradayWorkflow::new();

    match workflow.run(&config, None).await {
        Ok(result) => Json(WorkflowResponse {
            success: result.success,
            workflow: "intraday".to_string(),
            message: format!("Completed {} updates", result.trades_processed),
            duration_ms: result.duration_ms,
            trades_processed: result.trades_processed,
        }),
        Err(e) => Json(WorkflowResponse {
            success: false,
            workflow: "intraday".to_string(),
            message: format!("Error: {}", e),
            duration_ms: 0,
            trades_processed: 0,
        }),
    }
}

/// Stress test workflow handler
async fn stress_workflow_handler(State(state): State<AppState>) -> impl IntoResponse {
    tracing::info!("Starting stress test workflow");

    let workflow = StressTestWorkflow::new();

    match workflow.run(&state.config, None).await {
        Ok(result) => Json(WorkflowResponse {
            success: result.success,
            workflow: "stress_test".to_string(),
            message: format!("Completed {} scenarios", result.trades_processed),
            duration_ms: result.duration_ms,
            trades_processed: result.trades_processed,
        }),
        Err(e) => Json(WorkflowResponse {
            success: false,
            workflow: "stress_test".to_string(),
            message: format!("Error: {}", e),
            duration_ms: 0,
            trades_processed: 0,
        }),
    }
}
