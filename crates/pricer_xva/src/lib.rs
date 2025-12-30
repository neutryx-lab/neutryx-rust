//! # Pricer XVA (L4: Application)
//!
//! Portfolio aggregation, XVA calculations, and parallelisation.
//!
//! This crate provides:
//! - Portfolio and trade structures with netting sets
//! - Counterparty credit parameters
//! - Exposure aggregation (EE, EPE, PFE)
//! - CVA, DVA, FVA calculations
//! - Structure of Arrays (SoA) for cache efficiency
//! - Rayon-based parallelisation for Greeks computation
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │            pricer_xva (L4)              │
//! ├─────────────────────────────────────────┤
//! │  portfolio/  - Trade, Counterparty,    │
//! │               NettingSet, Portfolio     │
//! │  exposure/   - EE, EPE, PFE metrics    │
//! │  xva/        - CVA, DVA, FVA           │
//! │  soa/        - Structure of Arrays     │
//! │  parallel/   - Rayon utilities         │
//! └─────────────────────────────────────────┘
//!          ↓
//! ┌─────────────────────────────────────────┐
//! │           pricer_kernel (L3)           │
//! │  Monte Carlo engine with Enzyme AD     │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## Performance
//!
//! - Uses SoA layout for SIMD-friendly memory access
//! - Parallel computation with Rayon (target: >80% efficiency on 8+ cores)
//! - Batch processing for large portfolios
//!
//! ## Example
//!
//! ```
//! use pricer_xva::portfolio::{
//!     PortfolioBuilder, Trade, TradeId, Counterparty, CounterpartyId,
//!     NettingSet, NettingSetId, CreditParams,
//! };
//! use pricer_core::types::Currency;
//! use pricer_models::instruments::{
//!     Instrument, VanillaOption, InstrumentParams, PayoffType, ExerciseStyle,
//! };
//!
//! // Build a portfolio
//! let credit = CreditParams::new(0.02, 0.4).unwrap();
//! let counterparty = Counterparty::new(CounterpartyId::new("CP001"), credit);
//!
//! let netting_set = NettingSet::new(
//!     NettingSetId::new("NS001"),
//!     CounterpartyId::new("CP001"),
//! );
//!
//! let params = InstrumentParams::new(100.0, 1.0, 1.0).unwrap();
//! let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
//!
//! let trade = Trade::new(
//!     TradeId::new("T001"),
//!     Instrument::Vanilla(call),
//!     Currency::USD,
//!     CounterpartyId::new("CP001"),
//!     NettingSetId::new("NS001"),
//!     1_000_000.0,
//! );
//!
//! let portfolio = PortfolioBuilder::new()
//!     .add_counterparty(counterparty)
//!     .add_netting_set(netting_set)
//!     .add_trade(trade)
//!     .build()
//!     .unwrap();
//!
//! assert_eq!(portfolio.trade_count(), 1);
//! ```

#![warn(missing_docs)]

pub mod exposure;
pub mod parallel;
pub mod portfolio;
pub mod soa;
pub mod xva;

// Re-export commonly used types
pub use exposure::ExposureCalculator;
pub use parallel::{ParallelConfig, DEFAULT_BATCH_SIZE};
pub use portfolio::{
    CollateralAgreement, Counterparty, CounterpartyId, CreditParams, CreditRating, NettingSet,
    NettingSetId, Portfolio, PortfolioBuilder, PortfolioError, Trade, TradeBuilder, TradeId,
};
pub use soa::{ExposureSoA, TradeSoA};
