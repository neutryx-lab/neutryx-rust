//! 1-Factor Non-Parametric Markov Functional Model (1F-MFM).
//!
//! Provides calibration and pricing infrastructure for callable inverse
//! floaters (CIF) and other structured interest rate products using a
//! Gaussian recombining tree with non-parametric rate mapping.
//!
//! ## Module Organisation
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`config`] | Calibration configuration and result types |
//! | [`rate_mapping`] | Rate index mapping for multi-curve calibration |
//! | [`vol_cube`] | Swaption volatility cube abstraction |
//! | [`integral_adjuster`] | Gaussian numeraire integral corrections |
//! | [`model`] | Core MFM model implementation |
//! | [`cif_evaluator`] | Callable inverse floater evaluator |

pub mod cif_evaluator;
pub mod config;
pub mod integral_adjuster;
pub mod model;
pub mod rate_mapping;
pub mod vol_cube;

pub use config::{MfmCalibrationResult, MfmConfig, MfmVolType};
pub use integral_adjuster::IntegralAdjusterNormal;
pub use model::MarkovFunctionalNonParametric1F;
pub use rate_mapping::{CalibratedSlice, MfmRateIndex, RateIndexCalibration};
pub use vol_cube::{
    FlatSwaptionVolCube, SabrSwaptionVolCube, SwaptionVolCube, SwaptionVolCubeEnum,
};

// ─── Error type ─────────────────────────────────────────────────────

/// Errors arising from Markov Functional Model operations.
#[derive(Debug, Clone, PartialEq)]
pub enum MfmError {
    /// Calibration failed at a specific exercise date.
    CalibrationFailed { exercise_idx: usize, reason: String },
    /// Newton-Raphson solver failed to converge at a specific grid node.
    NewtonRaphsonFailed {
        exercise_idx: usize,
        grid_idx: usize,
    },
    /// An invalid parameter was supplied.
    InvalidParameter { name: &'static str, reason: String },
    /// Market data error.
    MarketData(String),
    /// Volatility surface error.
    VolSurface(String),
    /// Numerical instability detected during calibration.
    NumericalInstability(String),
}

impl std::fmt::Display for MfmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CalibrationFailed {
                exercise_idx,
                reason,
            } => {
                write!(
                    f,
                    "calibration failed at exercise index {}: {}",
                    exercise_idx, reason
                )
            }
            Self::NewtonRaphsonFailed {
                exercise_idx,
                grid_idx,
            } => {
                write!(
                    f,
                    "Newton-Raphson failed to converge at exercise index {}, grid index {}",
                    exercise_idx, grid_idx
                )
            }
            Self::InvalidParameter { name, reason } => {
                write!(f, "invalid parameter '{}': {}", name, reason)
            }
            Self::MarketData(msg) => {
                write!(f, "market data error: {}", msg)
            }
            Self::VolSurface(msg) => {
                write!(f, "volatility surface error: {}", msg)
            }
            Self::NumericalInstability(msg) => {
                write!(f, "numerical instability: {}", msg)
            }
        }
    }
}

impl std::error::Error for MfmError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_calibration_failed() {
        let e = MfmError::CalibrationFailed {
            exercise_idx: 3,
            reason: "swap rate outside bounds".to_string(),
        };
        assert_eq!(
            format!("{}", e),
            "calibration failed at exercise index 3: swap rate outside bounds"
        );
    }

    #[test]
    fn error_display_newton_raphson_failed() {
        let e = MfmError::NewtonRaphsonFailed {
            exercise_idx: 2,
            grid_idx: 15,
        };
        assert_eq!(
            format!("{}", e),
            "Newton-Raphson failed to converge at exercise index 2, grid index 15"
        );
    }

    #[test]
    fn error_display_invalid_parameter() {
        let e = MfmError::InvalidParameter {
            name: "mean_reversion",
            reason: "must be positive".to_string(),
        };
        assert_eq!(
            format!("{}", e),
            "invalid parameter 'mean_reversion': must be positive"
        );
    }

    #[test]
    fn error_display_market_data() {
        let e = MfmError::MarketData("missing discount curve".to_string());
        assert_eq!(
            format!("{}", e),
            "market data error: missing discount curve"
        );
    }

    #[test]
    fn error_display_vol_surface() {
        let e = MfmError::VolSurface("negative implied vol".to_string());
        assert_eq!(
            format!("{}", e),
            "volatility surface error: negative implied vol"
        );
    }

    #[test]
    fn error_display_numerical_instability() {
        let e = MfmError::NumericalInstability("annuity near zero".to_string());
        assert_eq!(format!("{}", e), "numerical instability: annuity near zero");
    }

    #[test]
    fn error_implements_std_error() {
        let e = MfmError::MarketData("test".to_string());
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn error_clone_and_eq() {
        let e1 = MfmError::NewtonRaphsonFailed {
            exercise_idx: 1,
            grid_idx: 5,
        };
        let e2 = e1.clone();
        assert_eq!(e1, e2);
    }
}
