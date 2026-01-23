//! Modular HTTP handlers for the FrictionalBank Web API (v2).
//!
//! This module organises API handlers by domain:
//!
//! - [`health`]: Health checks, metrics, and system endpoints
//! - [`exposure`]: Exposure metrics
//! - [`portfolio`]: Portfolio management
//! - [`risk`]: XVA risk metrics
//! - [`jobs`]: Async job management
//!
//! ## Migration Status
//!
//! This module is part of an ongoing migration from the monolithic `handlers.rs`
//! to a more maintainable modular structure. Once migration is complete, this
//! module will be renamed to `handlers` and the legacy module will be removed.
//!
//! ### Migrated
//! - `health` - Health check, metrics, index page
//! - `exposure` - Exposure profile
//! - `portfolio` - Portfolio data, pricing, trades list
//! - `risk` - XVA risk metrics (basic handler only)
//! - `jobs` - Job status and listing
//!
//! ### Not Yet Migrated (in legacy handlers.rs)
//! - Pricing handlers (`price_instrument`, `bootstrap_curve`, `price_irs`)
//! - Advanced risk handlers (`risk_bump`, `risk_aad`, `risk_compare`)
//! - Greeks handlers (`greeks_compare`, `greeks_first_order`, etc.)
//! - Graph handlers (`get_graph`, `get_speed_comparison`)
//! - Portfolio graph (`get_portfolio_graph`)
//! - Scenario handler (`run_scenario`)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use demo_gui::web::handlers_v2::{health, exposure, portfolio, risk, jobs};
//!
//! // Use the new modular handlers
//! let router = Router::new()
//!     .route("/api/health", get(health::health))
//!     .route("/api/metrics", get(health::get_metrics))
//!     .route("/api/exposure", get(exposure::get_exposure))
//!     .route("/api/portfolio", get(portfolio::get_portfolio))
//!     .route("/api/risk", get(risk::get_risk_metrics))
//!     .route("/api/v1/jobs", get(jobs::list_jobs));
//! ```

pub mod exposure;
pub mod health;
pub mod jobs;
pub mod portfolio;
pub mod risk;

// =============================================================================
// Re-exports for convenience
// =============================================================================

// Health module
pub use health::{
    get_index, get_metrics, health, serve_index_with_config, ApiResponseTimes, HealthResponse,
    MetricsResponse,
};

// Exposure module
pub use exposure::{get_exposure, ExposurePoint, ExposureResponse};

// Portfolio module
pub use portfolio::{
    get_portfolio, get_portfolio_trades, price_portfolio, sample_trades, PortfolioResponse,
    PortfolioTradeSummary, PortfolioTradesResponse, PortfolioTradesStats, PriceRequest,
    PriceRequestItem, TradeData,
};

// Risk module
pub use risk::{get_risk_metrics, RiskMetricsResponse};

// Jobs module
pub use jobs::{get_job_status, list_jobs, JobErrorResponse, JobListResponse, JobPathParams};
