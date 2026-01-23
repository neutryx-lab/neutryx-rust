//! Generic Pricer Engine.
//!
//! This module provides a unified pricing API for financial instruments
//! with support for:
//! - Single trade pricing with `GenericPricer::get_pv()`
//! - Batch pricing with `BatchPricer::price_batch()`
//! - Greeks calculation with `GenericPricer::get_greeks()`
//!
//! # Architecture
//!
//! The Generic Pricer follows the 3-stage rocket pattern:
//! 1. **Definition** (L2): Model, instrument, and market data types
//! 2. **Linking** (Stage 2): Market data resolution
//! 3. **Execution** (Stage 3): Pure computation kernel
//!
//! # Design Decisions
//!
//! - `GenericPricer`: Concrete struct (not trait) - single implementation suffices
//! - `PricingResult`: f64 fixed - AD only needed for `get_greeks()`
//! - `reporting_currency`: Required argument for `get_pv()` (risk calculation prerequisite)
//! - Currency breakdown: Leg-level only (no `CurrencyBreakdown` - Enzyme AD compatibility)
//!
//! # Example
//!
//! ```rust,ignore
//! use pricer_pricing::generic_pricer::{GenericPricer, ModelConfig, PricerConfig};
//!
//! let pricer = GenericPricer::new(market, model_config, pricer_config);
//! let result = pricer.get_pv(&trade, valuation_date, Currency::USD)?;
//!
//! // Access results at different levels
//! let total_pv = result.total_pv;
//! let legs = result.by_leg();
//! let cashflows = result.by_cashflow();
//! ```

mod error;

pub use error::{ConfigError, PricingError};
