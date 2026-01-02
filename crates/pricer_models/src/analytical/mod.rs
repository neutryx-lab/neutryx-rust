//! Analytical pricing formulas for European options.
//!
//! This module provides closed-form solutions for option pricing:
//! - Black-Scholes model for lognormal dynamics
//! - Bachelier model for normal dynamics
//! - Analytical Greeks (Delta, Gamma, Vega, Theta, Rho)
//!
//! ## Design Principles
//!
//! - **Generic over `T: Float`**: Supports both `f64` and `Dual64` for AD
//! - **AD Compatibility**: Avoids branching for tape consistency
//! - **Numerical Stability**: Uses erfc-based CDF for accuracy

pub mod bachelier;
pub mod black_scholes;
pub mod distributions;
pub mod error;

// Re-export main types at module level
pub use bachelier::Bachelier;
pub use black_scholes::{BlackScholes, Greeks};
pub use distributions::{norm_cdf, norm_pdf};
pub use error::AnalyticalError;
