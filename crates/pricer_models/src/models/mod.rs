//! Stochastic process models for Monte Carlo simulation.
//!
//! **DEPRECATED**: This module has been renamed to [`stochastic`](crate::stochastic).
//! Please update your imports to use `pricer_models::stochastic` instead.
//!
//! This module will be removed in a future version.
//!
//! Note: The SABR stochastic SDE implementation has been removed. For SABR
//! implied volatility calculations, use [`crate::formulas::sabr_implied_vol`].

#![deprecated(
    since = "0.2.0",
    note = "Use `pricer_models::stochastic` instead. This module will be removed in a future version."
)]

// Re-export everything from stochastic for backward compatibility
#[allow(deprecated)]
pub use crate::stochastic::{
    EquityModel, FxModel, HybridModel, ModelError, ModelParams, ModelState, RatesModel,
    SingleState, StochasticModel, StochasticModelEnum, StochasticState, TwoFactorState,
};

#[cfg(feature = "equity")]
#[allow(deprecated)]
pub use crate::stochastic::{GBMModel, GBMParams, HestonError, HestonModel, HestonParams};

#[cfg(feature = "rates")]
#[allow(deprecated)]
pub use crate::stochastic::{CIRModel, CIRParams, HullWhiteModel, HullWhiteParams, ThetaFunction};

#[cfg(feature = "exotic")]
#[allow(deprecated)]
pub use crate::stochastic::{
    CholeskyFactor, CorrelatedModels, CorrelationError, CorrelationMatrix,
};

/// Deprecated module for error types.
///
/// Use [`crate::stochastic::error`] instead.
#[deprecated(
    since = "0.2.0",
    note = "Use `pricer_models::stochastic::error` instead."
)]
pub mod error {
    pub use crate::stochastic::error::*;
}

/// Deprecated module for stochastic model enum.
///
/// Use [`crate::stochastic::model_enum`] instead.
#[deprecated(
    since = "0.2.0",
    note = "Use `pricer_models::stochastic::model_enum` instead."
)]
pub mod model_enum {
    pub use crate::stochastic::model_enum::*;
}

/// Deprecated module for stochastic traits.
///
/// Use [`crate::stochastic::stochastic`] instead.
#[deprecated(
    since = "0.2.0",
    note = "Use `pricer_models::stochastic::stochastic` instead."
)]
pub mod stochastic {
    pub use crate::stochastic::stochastic::*;
}

/// Deprecated module for validation utilities.
///
/// Use [`crate::stochastic::validation`] instead.
#[deprecated(
    since = "0.2.0",
    note = "Use `pricer_models::stochastic::validation` instead."
)]
pub mod validation {
    pub use crate::stochastic::validation::*;
}

/// Deprecated module for GBM model.
///
/// Use [`crate::stochastic::gbm`] instead.
#[cfg(feature = "equity")]
#[deprecated(since = "0.2.0", note = "Use `pricer_models::stochastic::gbm` instead.")]
pub mod gbm {
    pub use crate::stochastic::gbm::*;
}

/// Deprecated module for Heston model.
///
/// Use [`crate::stochastic::heston`] instead.
#[cfg(feature = "equity")]
#[deprecated(
    since = "0.2.0",
    note = "Use `pricer_models::stochastic::heston` instead."
)]
pub mod heston {
    pub use crate::stochastic::heston::*;
}

/// Deprecated module for Hull-White model.
///
/// Use [`crate::stochastic::hull_white`] instead.
#[cfg(feature = "rates")]
#[deprecated(
    since = "0.2.0",
    note = "Use `pricer_models::stochastic::hull_white` instead."
)]
pub mod hull_white {
    pub use crate::stochastic::hull_white::*;
}

/// Deprecated module for CIR model.
///
/// Use [`crate::stochastic::cir`] instead.
#[cfg(feature = "rates")]
#[deprecated(since = "0.2.0", note = "Use `pricer_models::stochastic::cir` instead.")]
pub mod cir {
    pub use crate::stochastic::cir::*;
}

/// Deprecated module for correlated models.
///
/// Use [`crate::stochastic::correlated`] instead.
#[cfg(feature = "exotic")]
#[deprecated(
    since = "0.2.0",
    note = "Use `pricer_models::stochastic::correlated` instead."
)]
pub mod correlated {
    pub use crate::stochastic::correlated::*;
}

/// SABR stochastic model (DEPRECATED).
///
/// The SABR stochastic SDE implementation has been removed.
/// For SABR implied volatility calculations, use
/// [`crate::formulas::sabr_implied_vol`] instead.
#[cfg(feature = "equity")]
#[deprecated(
    since = "0.2.0",
    note = "SABR stochastic SDE is removed. Use `pricer_models::formulas::sabr_implied_vol` for implied vol calculations."
)]
pub mod sabr {
    // SABR stochastic SDE has been removed.
    // For backward compatibility, we provide empty type aliases
    // pointing to the formulas module.

    /// SABR parameters for implied volatility (use formulas module directly).
    #[deprecated(
        since = "0.2.0",
        note = "Use `pricer_models::formulas::sabr_implied_vol::SabrImpliedVolParams` instead."
    )]
    pub type SABRParams<T> = crate::formulas::sabr_implied_vol::SabrImpliedVolParams<T>;

    /// SABR error type (use formulas module directly).
    #[deprecated(
        since = "0.2.0",
        note = "Use `pricer_models::formulas::sabr_implied_vol::SabrImpliedVolError` instead."
    )]
    pub type SABRError = crate::formulas::sabr_implied_vol::SabrImpliedVolError;
}

// Re-export deprecated SABR types at module level for backward compatibility
#[cfg(feature = "equity")]
#[allow(deprecated)]
pub use sabr::{SABRError, SABRParams};
