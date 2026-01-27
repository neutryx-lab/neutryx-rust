//! Analytical pricing formulas for European options.
//!
//! **DEPRECATED**: This module has been renamed to [`formulas`](crate::formulas).
//! Please update your imports to use `pricer_models::formulas` instead.
//!
//! This module will be removed in a future version.

#![deprecated(
    since = "0.2.0",
    note = "Use `pricer_models::formulas` instead. This module will be removed in a future version."
)]

// Re-export everything from formulas for backward compatibility
#[allow(deprecated)]
pub use crate::formulas::{
    norm_cdf, norm_inv_cdf, norm_pdf, AnalyticalError, Bachelier, BlackScholes, GarmanKohlhagen,
    GarmanKohlhagenParams,
};

#[allow(deprecated)]
pub use crate::formulas::garman_kohlhagen::{fx_call_price, fx_put_price};

/// Deprecated module for error types.
///
/// Use [`crate::formulas::error`] instead.
#[deprecated(
    since = "0.2.0",
    note = "Use `pricer_models::formulas::error` instead."
)]
pub mod error {
    pub use crate::formulas::error::*;
}

/// Deprecated module for distribution functions.
///
/// Use [`pricer_core::math::distributions`] instead.
#[deprecated(
    since = "0.2.0",
    note = "Use `pricer_core::math::distributions` instead."
)]
pub mod distributions {
    pub use pricer_core::math::distributions::{norm_cdf, norm_inv_cdf, norm_pdf};
}

/// Deprecated module for Garman-Kohlhagen FX pricing.
///
/// Use [`crate::formulas::garman_kohlhagen`] instead.
#[deprecated(
    since = "0.2.0",
    note = "Use `pricer_models::formulas::garman_kohlhagen` instead."
)]
pub mod garman_kohlhagen {
    pub use crate::formulas::garman_kohlhagen::*;
}
