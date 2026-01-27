//! Market data management context.
//!
//! This module consolidates market data management functionality:
//! - [`MarketProvider`]: Lazy market data resolution with Arc caching
//! - [`IndexedMarket`]: Index-keyed market data container
//! - [`IndexCurveMapper`]: Rate index to curve name mapping
//! - [`TradeIndexRequirements`]: Trade market data requirements trait
//! - [`MarketValidator`]: Market completeness validation
//!
//! # Architecture
//!
//! The context module implements the "Pull-then-Push" execution pattern:
//! - **Pull Phase**: Dependencies are resolved lazily via `MarketProvider`
//! - **Push Phase**: Resolved references are passed to pricing kernels via `IndexedMarket`
//!
//! # Example
//!
//! ```ignore
//! use pricer_models::market::context::{
//!     IndexedMarket, IndexedMarketBuilder, MarketProvider,
//!     MarketValidator, TradeIndexRequirements,
//! };
//! use infra_master::{RateIndex, Date};
//!
//! // Build market with index-keyed data
//! let market = IndexedMarketBuilder::new()
//!     .valuation_date(Date::from_ymd(2025, 1, 15).unwrap())
//!     .with_curve(RateIndex::Sofr, sofr_curve)
//!     .build()?;
//!
//! // Validate trade requirements
//! let validator = MarketValidator::new();
//! validator.validate(&trade, &market)?;
//! ```

mod indexed;
mod provider;
mod requirements;
mod validator;

// Re-export all public types
pub use indexed::{
    DefaultIndexCurveMapper, IndexCurveMapper, IndexedMarket, IndexedMarketBuilder,
};
pub use provider::{MarketProvider, VolCubeProviderKey};
pub use requirements::TradeIndexRequirements;
pub use validator::{MarketValidator, ValidationReport};
