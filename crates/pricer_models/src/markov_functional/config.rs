//! Configuration types for the 1F Markov Functional Model.
//!
//! Defines [`MfmConfig`] for model parameterisation, [`MfmVolType`] for
//! volatility convention selection, and [`MfmCalibrationResult`] for
//! collecting calibration outputs.

use pricer_core::{math::numeric::from_f64, traits::Float};

use super::{
    integral_adjuster::IntegralAdjusterNormal, rate_mapping::RateIndexCalibration, MfmError,
};

// ─── Volatility type ────────────────────────────────────────────────

/// Volatility convention used for swaption quotes in MFM calibration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MfmVolType {
    /// Normal (Bachelier) volatility.
    #[default]
    Normal,
    /// Log-normal (Black) volatility.
    Lognormal,
}

// ─── Configuration ──────────────────────────────────────────────────

/// Configuration parameters for the 1F Markov Functional Model.
///
/// Controls the Gaussian grid construction, Newton-Raphson solver
/// tolerances, and the schedule of exercise dates, swap tenors, and
/// payment frequencies used during calibration.
#[derive(Debug, Clone)]
pub struct MfmConfig<T: Float> {
    /// Mean reversion speed of the Gaussian driver process (must be > 0).
    pub mean_reversion: T,
    /// Instantaneous volatility of the Gaussian driver (must be > 0).
    pub volatility: T,
    /// Number of grid points in the Gaussian recombining tree (must be >= 3 and
    /// odd).
    pub num_grid_points: usize,
    /// Number of standard deviations for grid extent.
    pub num_std_devs: T,
    /// Volatility convention for swaption market data.
    pub vol_type: MfmVolType,
    /// Newton-Raphson convergence tolerance.
    pub nr_tolerance: T,
    /// Maximum Newton-Raphson iterations per grid node.
    pub nr_max_iterations: usize,
    /// Exercise times (year fractions) for each calibration instrument.
    pub exercise_times: Vec<T>,
    /// Swap tenors (year fractions) for each calibration instrument.
    pub swap_tenors: Vec<T>,
    /// Payment frequencies (year fractions) for each calibration instrument.
    pub payment_frequencies: Vec<T>,
}

impl<T: Float> Default for MfmConfig<T> {
    fn default() -> Self {
        Self {
            mean_reversion: from_f64(0.01),
            volatility: from_f64(0.01),
            num_grid_points: 41,
            num_std_devs: from_f64(5.0),
            vol_type: MfmVolType::default(),
            nr_tolerance: from_f64(1e-10),
            nr_max_iterations: 100,
            exercise_times: Vec::new(),
            swap_tenors: Vec::new(),
            payment_frequencies: Vec::new(),
        }
    }
}

impl<T: Float> MfmConfig<T> {
    /// Validates the configuration, returning an error if any parameter
    /// is out of its acceptable range.
    ///
    /// # Checks
    /// - `mean_reversion` must be strictly positive.
    /// - `volatility` must be strictly positive.
    /// - `num_grid_points` must be at least 3 and odd.
    /// - `exercise_times`, `swap_tenors`, and `payment_frequencies` must all
    ///   have the same length.
    pub fn validate(&self) -> Result<(), MfmError> {
        let zero: T = from_f64(0.0);

        if self.mean_reversion <= zero {
            return Err(MfmError::InvalidParameter {
                name: "mean_reversion",
                reason: "must be strictly positive".to_string(),
            });
        }

        if self.volatility <= zero {
            return Err(MfmError::InvalidParameter {
                name: "volatility",
                reason: "must be strictly positive".to_string(),
            });
        }

        if self.num_grid_points < 3 {
            return Err(MfmError::InvalidParameter {
                name: "num_grid_points",
                reason: "must be at least 3".to_string(),
            });
        }

        #[allow(unknown_lints, clippy::manual_is_multiple_of)]
        if self.num_grid_points % 2 == 0 {
            return Err(MfmError::InvalidParameter {
                name: "num_grid_points",
                reason: "must be odd".to_string(),
            });
        }

        let n_ex = self.exercise_times.len();
        let n_sw = self.swap_tenors.len();
        let n_pf = self.payment_frequencies.len();

        if n_ex != n_sw || n_ex != n_pf {
            return Err(MfmError::InvalidParameter {
                name: "exercise_times/swap_tenors/payment_frequencies",
                reason: format!(
                    "lengths must match: exercise_times={}, swap_tenors={}, payment_frequencies={}",
                    n_ex, n_sw, n_pf
                ),
            });
        }

        Ok(())
    }
}

// ─── Calibration result ─────────────────────────────────────────────

/// Aggregated result of MFM calibration across all rate indices.
///
/// Contains the calibrated rate mappings for each curve, the integral
/// adjuster state, and diagnostic statistics.
#[derive(Debug, Clone)]
pub struct MfmCalibrationResult<T: Float> {
    /// Calibrated funding-index swap rate mapping.
    pub funding_calibration: RateIndexCalibration<T>,
    /// Calibrated coupon-index swap rate mapping.
    pub coupon_swap_calibration: RateIndexCalibration<T>,
    /// Calibrated coupon-index LIBOR rate mapping.
    pub coupon_libor_calibration: RateIndexCalibration<T>,
    /// Integral adjuster state after calibration.
    pub adjuster: IntegralAdjusterNormal<T>,
    /// Maximum number of Newton-Raphson iterations used across all grid nodes.
    pub max_nr_iterations_used: usize,
    /// Maximum calibration error (absolute) across all exercise dates.
    pub max_calibration_error: T,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_vol_type_is_normal() {
        assert_eq!(MfmVolType::default(), MfmVolType::Normal);
    }

    #[test]
    fn default_config_validates() {
        let config = MfmConfig::<f64>::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn default_config_values() {
        let config = MfmConfig::<f64>::default();
        assert_eq!(config.mean_reversion, 0.01);
        assert_eq!(config.volatility, 0.01);
        assert_eq!(config.num_grid_points, 41);
        assert_eq!(config.num_std_devs, 5.0);
        assert_eq!(config.vol_type, MfmVolType::Normal);
        assert_eq!(config.nr_tolerance, 1e-10);
        assert_eq!(config.nr_max_iterations, 100);
        assert!(config.exercise_times.is_empty());
        assert!(config.swap_tenors.is_empty());
        assert!(config.payment_frequencies.is_empty());
    }

    #[test]
    fn validate_rejects_non_positive_mean_reversion() {
        let config = MfmConfig::<f64> {
            mean_reversion: 0.0,
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert_eq!(
            err,
            MfmError::InvalidParameter {
                name: "mean_reversion",
                reason: "must be strictly positive".to_string(),
            }
        );
    }

    #[test]
    fn validate_rejects_negative_mean_reversion() {
        let config = MfmConfig::<f64> {
            mean_reversion: -0.5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn validate_rejects_non_positive_volatility() {
        let config = MfmConfig::<f64> {
            volatility: 0.0,
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert_eq!(
            err,
            MfmError::InvalidParameter {
                name: "volatility",
                reason: "must be strictly positive".to_string(),
            }
        );
    }

    #[test]
    fn validate_rejects_too_few_grid_points() {
        let config = MfmConfig::<f64> {
            num_grid_points: 2,
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert_eq!(
            err,
            MfmError::InvalidParameter {
                name: "num_grid_points",
                reason: "must be at least 3".to_string(),
            }
        );
    }

    #[test]
    fn validate_rejects_even_grid_points() {
        let config = MfmConfig::<f64> {
            num_grid_points: 40,
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert_eq!(
            err,
            MfmError::InvalidParameter {
                name: "num_grid_points",
                reason: "must be odd".to_string(),
            }
        );
    }

    #[test]
    fn validate_rejects_mismatched_schedule_lengths() {
        let config = MfmConfig::<f64> {
            exercise_times: vec![1.0, 2.0],
            swap_tenors: vec![5.0],
            payment_frequencies: vec![0.5, 0.5],
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        match err {
            MfmError::InvalidParameter { name, .. } => {
                assert_eq!(name, "exercise_times/swap_tenors/payment_frequencies");
            }
            _ => panic!("expected InvalidParameter"),
        }
    }

    #[test]
    fn validate_accepts_matching_schedule_lengths() {
        let config = MfmConfig::<f64> {
            exercise_times: vec![1.0, 2.0, 3.0],
            swap_tenors: vec![5.0, 5.0, 5.0],
            payment_frequencies: vec![0.5, 0.5, 0.5],
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_accepts_min_odd_grid_points() {
        let config = MfmConfig::<f64> {
            num_grid_points: 3,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn vol_type_clone_and_copy() {
        let v1 = MfmVolType::Lognormal;
        let v2 = v1;
        assert_eq!(v1, v2);
    }

    #[test]
    fn config_clone() {
        let config = MfmConfig::<f64> {
            exercise_times: vec![1.0],
            swap_tenors: vec![5.0],
            payment_frequencies: vec![0.5],
            ..Default::default()
        };
        let cloned = config.clone();
        assert_eq!(cloned.exercise_times, vec![1.0]);
        assert_eq!(cloned.mean_reversion, config.mean_reversion);
    }
}
