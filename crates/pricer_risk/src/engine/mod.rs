//! Risk Engine module.
//!
//! Provides the unified [`RiskEngine`] facade for Greeks and risk calculations.
//!
//! # Overview
//!
//! The Risk Engine is the primary entry point for computing sensitivities
//! (Greeks) on financial instruments. It supports:
//!
//! - **AAD (Automatic Adjoint Differentiation)**: Fast gradient computation
//!   using Enzyme LLVM-level AD (requires `enzyme-ad` feature)
//! - **Bump-and-Revalue**: Traditional finite difference method
//! - **Parallel Processing**: Rayon-based parallelisation for large portfolios
//! - **Configuration-Driven**: Flexible configuration via TOML/JSON
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      RiskEngine                              │
//! ├─────────────────────────────────────────────────────────────┤
//! │  compute_greeks()         - Single trade calculation         │
//! │  compute_portfolio_greeks() - Portfolio calculation          │
//! └─────────────────────────────────────────────────────────────┘
//!                              ↓
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      RiskConfig                              │
//! │  (from infra_config)                                         │
//! ├─────────────────────────────────────────────────────────────┤
//! │  greeks_method:    AAD | Bump                                │
//! │  bump_sizes:       rate, vol, spot bumps                     │
//! │  target_greeks:    Delta, Gamma, Vega, etc.                  │
//! │  second_order_mode: Parallel | Serial                        │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use pricer_risk::engine::{RiskEngine, RiskEngineConfig};
//! use infra_config::{RiskConfig, GreeksMethod, GreekType};
//!
//! // Create configuration
//! let risk_config = RiskConfig {
//!     greeks_method: GreeksMethod::Bump,
//!     target_greeks: vec![GreekType::Delta, GreekType::Gamma, GreekType::Vega],
//!     ..Default::default()
//! };
//!
//! // Create engine
//! let engine = RiskEngine::new(RiskEngineConfig::new(risk_config));
//!
//! // Compute Greeks for a single trade
//! let result = engine.compute_greeks("T001", || {
//!     // Your pricing logic here
//!     Ok(GreeksResult::new(100.0, 0.01)
//!         .with_delta(0.5)
//!         .with_gamma(0.02))
//! })?;
//!
//! println!("Trade: {}", result.trade_id);
//! println!("PV: {}", result.pv);
//! println!("Delta: {:?}", result.greeks.delta);
//! ```
//!
//! # Requirements Coverage
//!
//! - Requirement 5.1: RiskEngine facade
//! - Requirement 5.2: compute_greeks() method
//! - Requirement 5.3: AAD/Bump mode selection
//! - Requirement 5.4: Risk factor identification
//! - Requirement 5.5: RiskResult with metrics

mod engine;
mod error;
mod result;

pub use engine::{RiskEngine, RiskEngineConfig, ScenarioGreeksResult, ScenarioPortfolioResult};
pub use error::{PartialGreeksResult, RiskError};
pub use result::{
    AggregatedGreeks, ComputedGreeks, ExecutionStats, FailedCalculation, PerformanceMetrics,
    PortfolioRiskResult, RiskResult,
};
