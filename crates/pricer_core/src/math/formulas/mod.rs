//! Closed-form pricing formulas.
//!
//! This module provides pure mathematical implementations of classical
//! option pricing formulas:
//!
//! - [`black_scholes`] - Black-Scholes model for lognormal dynamics
//! - [`bachelier`] - Bachelier model for normal dynamics
//! - [`garman_kohlhagen`] - Garman-Kohlhagen model for FX options
//! - [`forward`] - Forward contract pricing
//! - [`sabr`] - SABR Hagan implied volatility formula
//! - [`fx_delta`] - FX Delta-Strike conversion functions
//!
//! ## Design Principles
//!
//! - **Generic over `T: Float`**: Supports both `f64` and AD types
//! - **AD Compatibility**: Avoids branching for tape consistency
//! - **Numerical Stability**: Uses erfc-based CDF for accuracy
//! - **Domain-agnostic**: No dependencies on instrument types

pub mod bachelier;
pub mod black_scholes;
pub mod error;
pub mod forward;
pub mod fx_delta;
pub mod garman_kohlhagen;
pub mod sabr;

// Re-export main types at module level
pub use bachelier::Bachelier;
pub use black_scholes::BlackScholes;
pub use error::FormulaError;
pub use forward::{forward_price, forward_pv, Forward, ForwardParams};
pub use fx_delta::{delta_to_strike, strike_to_delta};
pub use garman_kohlhagen::{fx_call_price, fx_put_price, GarmanKohlhagen, GarmanKohlhagenParams};
pub use sabr::{
    sabr_atm_vol, sabr_implied_vol, sabr_implied_vol_with_floor, SabrImpliedVolError,
    SabrImpliedVolParams,
};
