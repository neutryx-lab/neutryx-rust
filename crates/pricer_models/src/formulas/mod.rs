//! Closed-form pricing formulas for European options.
//!
//! This module provides analytical solutions for option pricing:
//! - Black-Scholes model for lognormal dynamics
//! - Bachelier model for normal dynamics
//! - Garman-Kohlhagen model for FX options
//! - SABR Hagan implied volatility formula
//! - Analytical Greeks (Delta, Gamma, Vega, Theta, Rho)
//!
//! ## Design Principles
//!
//! - **Generic over `T: Float`**: Supports both `f64` and `Dual64` for AD
//! - **AD Compatibility**: Avoids branching for tape consistency
//! - **Numerical Stability**: Uses erfc-based CDF for accuracy

pub mod distributions;
pub mod error;
pub mod sabr_implied_vol;

mod bachelier;
mod black_scholes;
pub mod garman_kohlhagen;

// Re-export main types at module level
pub use bachelier::Bachelier;
pub use black_scholes::BlackScholes;
pub use distributions::{norm_cdf, norm_inv_cdf, norm_pdf};
pub use error::AnalyticalError;
pub use garman_kohlhagen::{fx_call_price, fx_put_price, GarmanKohlhagen, GarmanKohlhagenParams};
pub use sabr_implied_vol::{
    sabr_atm_vol, sabr_implied_vol, sabr_implied_vol_with_floor, SabrImpliedVolError,
    SabrImpliedVolParams,
};
