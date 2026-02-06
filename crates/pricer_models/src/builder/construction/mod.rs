//! Curve construction module.
//!
//! This module provides a high-level facade for building yield curves from
//! definition registries and market data.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                      CurveConstructionEngine                        │
//! │                                                                     │
//! │  ┌─────────────────────┐     ┌──────────────────────────────────┐  │
//! │  │  DefinitionRegistry │     │        MarketRateSet             │  │
//! │  │  (infra_domain)     │     │        (infra_domain)            │  │
//! │  │                     │     │                                  │  │
//! │  │  - Instruments      │     │  - RateId → MarketRate           │  │
//! │  │  - RateIndices      │     │  - Bid/Ask/Mid quotes            │  │
//! │  │  - Curves           │     │                                  │  │
//! │  └──────────┬──────────┘     └───────────────┬──────────────────┘  │
//! │             │                                │                     │
//! │             ▼                                ▼                     │
//! │       ┌─────────────────────────────────────────────────────┐      │
//! │       │                    Converter                         │      │
//! │       │  InstrumentDefinition + rate → MarketInstrument<T>  │      │
//! │       └────────────────────────┬────────────────────────────┘      │
//! │                                │                                   │
//! │                                ▼                                   │
//! │                    ┌─────────────────────────┐                     │
//! │                    │    CurveBootstrapper    │                     │
//! │                    │    (pricer_models)      │                     │
//! │                    └────────────┬────────────┘                     │
//! │                                 │                                  │
//! │                                 ▼                                  │
//! │                    ┌─────────────────────────┐                     │
//! │                    │   BootstrappedCurve<T>  │                     │
//! │                    └─────────────────────────┘                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! use pricer_models::builder::construction::{CurveConstructionEngine, ConstructionConfig};
//! use infra_domain::market::{DefinitionRegistry, MarketRateSet};
//!
//! // 1. Load definitions from JSON or build programmatically
//! let registry = DefinitionRegistry::new();
//! // ... register instruments, rate indices, curves ...
//!
//! // 2. Load market rates from external source
//! let market_rates = MarketRateSet::new();
//! // ... insert rates ...
//!
//! // 3. Build the curve
//! let engine = CurveConstructionEngine::new(
//!     ConstructionConfig::new(1e-10)
//!         .with_max_iterations(100)
//!         .with_strict_mode(true)
//! );
//!
//! let result = engine.build::<f64>(&registry, &market_rates, "USD-SOFR-Discount")?;
//!
//! println!("Built curve with {} instruments", result.instruments_used);
//! println!("5Y discount factor: {:.6}", result.curve.discount_factor(5.0)?);
//! ```

mod converter;
mod engine;
mod error;

pub use converter::{definition_to_instrument, ReferenceDate};
pub use engine::{ConstructionConfig, ConstructionResult, CurveConstructionEngine};
pub use error::ConstructionError;
