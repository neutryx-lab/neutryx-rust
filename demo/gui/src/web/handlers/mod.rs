//! Modular HTTP handlers for the FrictionalBank Web API (v2).
//!
//! This module organises API handlers by domain:
//!
//! - [`health`]: Health checks, metrics, and system endpoints
//! - [`exposure`]: Exposure metrics
//! - [`portfolio`]: Portfolio management
//! - [`risk`]: XVA risk metrics
//! - [`jobs`]: Async job management
//! - [`graphs`]: Computation graph visualisation
//! - [`pricing`]: Instrument pricing (options, IRS, bootstrap)
//! - [`greeks`]: Greek calculations
//! - [`scenarios`]: Scenario analysis
//! - [`benchmarks`]: Performance benchmarks
//!
//! ## Migration Status
//!
//! All handlers are now available through this module. The legacy `handlers.rs`
//! is kept for backward compatibility and will be removed once migration is
//! complete.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use demo_gui::web::handlers::{health, exposure, portfolio, risk, jobs, graphs, pricing, greeks};
//!
//! // Use the new modular handlers
//! let router = Router::new()
//!     .route("/api/health", get(health::health))
//!     .route("/api/metrics", get(health::get_metrics))
//!     .route("/api/exposure", get(exposure::get_exposure))
//!     .route("/api/portfolio", get(portfolio::get_portfolio))
//!     .route("/api/risk", get(risk::get_risk_metrics))
//!     .route("/api/graph", get(graphs::get_graph))
//!     .route("/api/v1/jobs", get(jobs::list_jobs))
//!     .route("/api/price", post(pricing::price_instrument))
//!     .route("/api/greeks/compare", post(greeks::greeks_compare));
//! ```

pub mod benchmarks;
pub mod types;
pub mod config;
pub mod curves;
pub mod events;
pub mod exposure;
pub mod fxcurve;
pub mod fxvol;
pub mod generic_pricer;
pub mod graphs;
pub mod greeks;
pub mod health;
pub mod irvol;
pub mod jobs;
pub mod market;
pub mod portfolio;
pub mod pricer_graph;
pub mod pricing;
pub mod risk;
pub mod risk_engine;
pub mod scenario_analysis;
pub mod scenarios;
pub mod trades;
pub mod volcube;

// =============================================================================
// Re-exports for convenience
// =============================================================================

// Exposure module
pub use exposure::{get_exposure, ExposurePoint, ExposureResponse};

// Graphs module - all types are now defined in graphs.rs
pub use graphs::{
    generate_sample_graph, generate_sample_portfolio_graph, get_graph, get_instrument_graph,
    get_portfolio_graph, CachedGraph, CachedPortfolioGraph, GraphCache, GraphEdgeResponse,
    GraphErrorResponse, GraphMetadataResponse, GraphNodeResponse, GraphQueryParams, GraphResponse,
    InstrumentGraphMetadata, InstrumentGraphNode, InstrumentGraphQueryParams,
    InstrumentGraphResponse, PortfolioGraphCache, PortfolioGraphMetadataResponse,
    PortfolioGraphNodeResponse, PortfolioGraphQueryParams, PortfolioGraphResponse,
};

// Health module
pub use health::{
    get_index, get_metrics, health, serve_index_with_config, ApiResponseTimes, HealthResponse,
    MetricsResponse,
};

// Jobs module
pub use jobs::{get_job_status, list_jobs, JobErrorResponse, JobListResponse, JobPathParams};

// Portfolio module
pub use portfolio::{
    get_portfolio, get_portfolio_trades, price_portfolio, sample_trades, PortfolioResponse,
    PortfolioTradeSummary, PortfolioTradesResponse, PortfolioTradesStats, PriceRequest,
    PriceRequestItem, TradeData,
};

// Risk module - handlers from risk.rs, types from types module
pub use risk::{get_risk_metrics, risk_aad, risk_bump, risk_compare, RiskMetricsResponse};
pub use types::{
    RiskAadResponse, RiskBumpResponse, RiskCompareResponse, RiskMethodResult, RiskRequest,
};

// Pricing module - handlers from pricing.rs, types from types module
pub use pricing::{bootstrap_curve, price_instrument, price_irs};
pub use types::{
    BootstrapRequest, BootstrapResponse, EquityOptionParams, FxOptionParams, GreeksData,
    InstrumentParams, InstrumentType, IrsBootstrapErrorResponse, IrsParams, IrsPricingRequest,
    IrsPricingResponse, MarketDataConfig, MarketDataSource, PricingErrorResponse, PricingRequest,
    PricingResponse,
};

// Greeks module - handlers from greeks.rs, types from types module
pub use greeks::{
    get_greeks_heatmap, get_greeks_timeseries, greeks_bucket_dv01, greeks_compare,
    greeks_first_order, greeks_second_order,
};
pub use types::{
    BucketDv01Request, BucketDv01Response, GreeksCalculationRequest, GreeksCalculationResponse,
    GreeksCompareRequest, GreeksCompareResponse, GreeksDiff, GreeksHeatmapRequest,
    GreeksHeatmapResponse, GreeksMethodResult, GreeksTimeseriesRequest, GreeksTimeseriesResponse,
    SecondOrderGreeksRequest, SecondOrderGreeksResponse,
};

// Scenarios module
pub use scenarios::{run_scenario, ScenarioRequest, ScenarioResponse};

// Benchmarks module
pub use benchmarks::{get_speed_comparison, SpeedComparisonResponse};

// Other handlers are accessed via handlers::<module>::<function>
// e.g., handlers::scenario_analysis::get_scenario_presets
