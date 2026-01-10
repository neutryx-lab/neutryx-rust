//! Interest rate stochastic models.
//!
//! This module provides stochastic models for interest rate processes:
//! - [`HullWhiteModel`]: Hull-White one-factor model for short rate dynamics
//! - [`CIRModel`]: Cox-Ingersoll-Ross model with mean reversion (future implementation)
//!
//! # Feature Flag
//!
//! This module is available when the `rates` feature is enabled.
//!
//! # Models
//!
//! ## Hull-White 1F
//!
//! The Hull-White model describes short rate dynamics with mean reversion:
//! ```text
//! dr(t) = [theta(t) - a * r(t)] * dt + sigma * dW(t)
//! ```
//!
//! ## CIR Model
//!
//! The CIR model ensures non-negative rates (when Feller condition holds):
//! ```text
//! dr(t) = a * (b - r(t)) * dt + sigma * sqrt(r(t)) * dW(t)
//! ```

pub mod cir;
pub mod hull_white;

// Re-export main types
pub use cir::{CIRModel, CIRParams};
pub use hull_white::{HullWhiteModel, HullWhiteParams, ThetaFunction};
