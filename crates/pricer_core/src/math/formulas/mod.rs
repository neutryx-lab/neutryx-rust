//! Closed-form pricing formulas.
//!
//! This module provides pure mathematical implementations of classical
//! option pricing formulas:
//!
//! - [`generalised_bsm`] - Generalised Black-Scholes-Merton with cost-of-carry
//! - [`black_scholes`] - Black-Scholes model (delegates to GeneralisedBSM, b = r)
//! - [`bachelier`] - Bachelier model for normal dynamics
//! - [`garman_kohlhagen`] - Garman-Kohlhagen FX model (delegates to GeneralisedBSM, b = rd - rf)
//! - [`forward`] - Forward contract pricing
//! - [`rates`] - Interest rate calculations from discount factors
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
pub mod generalised_bsm;
pub mod rates;
pub mod sabr;

// Re-export main types at module level
pub use bachelier::Bachelier;
pub use black_scholes::BlackScholes;
pub use generalised_bsm::GeneralisedBSM;
pub use error::FormulaError;
pub use forward::{forward_price, forward_pv, Forward, ForwardParams};
pub use fx_delta::{delta_to_strike, strike_to_delta};
pub use garman_kohlhagen::{fx_call_price, fx_put_price, GarmanKohlhagen, GarmanKohlhagenParams};
pub use rates::{
    continuous_forward_rate, df_from_zero_rate, simple_forward_rate, zero_rate_from_df,
};
pub use sabr::{
    sabr_atm_vol, sabr_implied_vol, sabr_implied_vol_with_floor, SabrImpliedVolError,
    SabrImpliedVolParams,
};
