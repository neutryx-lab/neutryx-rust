//! # Demo GUI
//!
//! Terminal and Web dashboard for the FrictionalBank demo.
//!
//! ## TUI Mode
//! Uses ratatui for rendering and crossterm for terminal handling.
//!
//! ### Screens
//! - **Dashboard**: Portfolio summary and risk metrics overview
//! - **Portfolio**: Trade list with PV and Greeks
//! - **Risk**: CVA, DVA, FVA, Exposure display
//! - **TradeBlotter**: Selected trade details
//! - **Chart**: Exposure time series chart
//!
//! ## Web Mode (feature: `web`)
//! Uses axum for REST API and WebSocket support.
//!
//! ### Endpoints
//! - `GET /api/health` - Health check
//! - `GET /api/portfolio` - Portfolio data
//! - `GET /api/exposure` - Exposure metrics
//! - `GET /api/risk` - Risk metrics
//! - `WS /api/ws` - Real-time updates

pub mod api_client;
pub mod app;
pub mod screens;

#[cfg(feature = "web")]
pub mod web;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::api_client::ApiClient;
    pub use crate::app::{ExposureTimeSeries, Screen, TuiApp};
}
