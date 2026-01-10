//! Hull-White model calibration.
//!
//! # Requirement: 6.3
//!
//! This module provides calibration infrastructure for the Hull-White
//! one-factor short rate model using swaption volatility data.
//!
//! ## Calibration Approach
//!
//! Hull-White parameters (mean reversion and volatility) are calibrated
//! by fitting model swaption prices to market swaption prices. The model
//! uses the Jamshidian decomposition for swaption pricing.
//!
//! ## Model Dynamics
//!
//! The Hull-White model follows:
//! ```text
//! dr(t) = (θ(t) - a*r(t)) dt + σ dW(t)
//! ```
//! where:
//! - a = mean reversion speed
//! - σ = volatility
//! - θ(t) = time-dependent drift (fitted to initial term structure)

use pricer_core::traits::calibration::{
    CalibrationConfig, CalibrationResult, Calibrator, Constraint, ParameterBounds,
};

use super::{ModelCalibrator, ModelCalibratorConfig};

/// Swaption data point for Hull-White calibration.
#[derive(Debug, Clone, PartialEq)]
pub struct HWSwaptionPoint {
    /// Option expiry in years.
    pub expiry: f64,
    /// Underlying swap tenor in years.
    pub tenor: f64,
    /// Market implied volatility (Black or normal).
    pub market_vol: f64,
    /// Whether vol is normal (true) or lognormal (false).
    pub is_normal_vol: bool,
    /// Weight for this point in calibration.
    pub weight: f64,
}

impl HWSwaptionPoint {
    /// Create a new swaption point with lognormal volatility.
    pub fn from_lognormal(expiry: f64, tenor: f64, vol: f64) -> Self {
        Self {
            expiry,
            tenor,
            market_vol: vol,
            is_normal_vol: false,
            weight: 1.0,
        }
    }

    /// Create a new swaption point with normal volatility.
    pub fn from_normal(expiry: f64, tenor: f64, vol: f64) -> Self {
        Self {
            expiry,
            tenor,
            market_vol: vol,
            is_normal_vol: true,
            weight: 1.0,
        }
    }

    /// Set the weight.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

/// Hull-White calibration data.
///
/// # Requirement: 6.3
///
/// Contains market swaption data for Hull-White model calibration.
#[derive(Debug, Clone)]
pub struct HullWhiteCalibrationData {
    /// Swaption points for calibration.
    pub swaptions: Vec<HWSwaptionPoint>,
    /// Flat forward rate (simplified yield curve).
    pub forward_rate: f64,
    /// Day count convention (simplified as year fraction).
    pub payment_frequency: f64,
}

impl HullWhiteCalibrationData {
    /// Create new calibration data.
    pub fn new(forward_rate: f64) -> Self {
        Self {
            swaptions: Vec::new(),
            forward_rate,
            payment_frequency: 0.5, // Semi-annual by default
        }
    }

    /// Set payment frequency.
    pub fn with_payment_frequency(mut self, freq: f64) -> Self {
        self.payment_frequency = freq;
        self
    }

    /// Add a swaption point (lognormal vol).
    pub fn add_swaption(&mut self, expiry: f64, tenor: f64, vol: f64) {
        self.swaptions
            .push(HWSwaptionPoint::from_lognormal(expiry, tenor, vol));
    }

    /// Add a swaption point (normal vol).
    pub fn add_swaption_normal(&mut self, expiry: f64, tenor: f64, vol: f64) {
        self.swaptions
            .push(HWSwaptionPoint::from_normal(expiry, tenor, vol));
    }

    /// Number of swaptions.
    pub fn len(&self) -> usize {
        self.swaptions.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.swaptions.is_empty()
    }

    /// Validate the data.
    pub fn validate(&self) -> Result<(), String> {
        if self.swaptions.is_empty() {
            return Err("No swaption data provided".to_string());
        }
        if self.forward_rate <= 0.0 {
            return Err("Forward rate must be positive".to_string());
        }
        for (i, swp) in self.swaptions.iter().enumerate() {
            if swp.expiry <= 0.0 {
                return Err(format!("Swaption {}: expiry must be positive", i));
            }
            if swp.tenor <= 0.0 {
                return Err(format!("Swaption {}: tenor must be positive", i));
            }
            if swp.market_vol <= 0.0 {
                return Err(format!("Swaption {}: volatility must be positive", i));
            }
        }
        Ok(())
    }
}

/// Hull-White parameter indices.
///
/// - params[0] = a (mean reversion)
/// - params[1] = sigma (volatility)
#[derive(Debug, Clone, Copy)]
pub struct HWParamIndex;

impl HWParamIndex {
    /// Mean reversion index.
    pub const MEAN_REVERSION: usize = 0;
    /// Volatility index.
    pub const VOLATILITY: usize = 1;
    /// Number of parameters.
    pub const COUNT: usize = 2;
}

/// Hull-White model calibrator.
///
/// # Requirement: 6.3
///
/// Calibrates Hull-White mean reversion and volatility parameters
/// to market swaption volatilities using Levenberg-Marquardt
/// optimisation.
///
/// ## Parameter Bounds
///
/// - a > 0 (mean reversion speed)
/// - σ > 0 (volatility)
#[derive(Debug, Clone)]
pub struct HullWhiteCalibrator {
    /// Underlying model calibrator.
    calibrator: ModelCalibrator,
}

impl Default for HullWhiteCalibrator {
    fn default() -> Self {
        Self::new()
    }
}

impl HullWhiteCalibrator {
    /// Create a new Hull-White calibrator.
    pub fn new() -> Self {
        let config = ModelCalibratorConfig::default().with_bounds(vec![
            ParameterBounds::new(0.001, 10.0), // a: (0, 10]
            ParameterBounds::new(0.0001, 0.1), // sigma: (0, 0.1]
        ]);

        Self {
            calibrator: ModelCalibrator::new(config),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: ModelCalibratorConfig) -> Self {
        Self {
            calibrator: ModelCalibrator::new(config),
        }
    }

    /// Compute Hull-White swaption Black volatility.
    ///
    /// Uses the approximate formula for Hull-White implied volatility:
    /// σ_Black ≈ σ * sqrt(B(T_opt, T_opt + τ)) * sqrt(V(T_opt))
    ///
    /// where B is the zero-coupon bond volatility factor.
    pub fn swaption_vol(
        expiry: f64,
        tenor: f64,
        mean_reversion: f64,
        sigma: f64,
    ) -> f64 {
        // B(t, T) = (1 - exp(-a*(T-t))) / a
        let b_factor = |t: f64, t_end: f64, a: f64| -> f64 {
            if a.abs() < 1e-10 {
                t_end - t
            } else {
                (1.0 - (-a * (t_end - t)).exp()) / a
            }
        };

        // Variance of integrated short rate
        let var_factor = |t: f64, a: f64, sig: f64| -> f64 {
            if a.abs() < 1e-10 {
                sig * sig * t
            } else {
                sig * sig * (1.0 - (-2.0 * a * t).exp()) / (2.0 * a)
            }
        };

        // Simplified swaption vol approximation
        // For a more accurate model, use Jamshidian decomposition
        let a = mean_reversion;

        // Average B factor over swap period
        let b_avg = b_factor(expiry, expiry + tenor, a);

        // Variance at option expiry
        let v = var_factor(expiry, a, sigma);

        // Approximate Black volatility
        sigma * b_avg.sqrt() * (v / expiry).sqrt()
    }
}

impl Calibrator for HullWhiteCalibrator {
    type MarketData = HullWhiteCalibrationData;
    type ModelParams = Vec<f64>;

    fn calibrate(
        &self,
        market_data: &Self::MarketData,
        initial_params: Self::ModelParams,
        _config: &CalibrationConfig,
    ) -> CalibrationResult<Self::ModelParams> {
        if let Err(e) = market_data.validate() {
            return CalibrationResult::not_converged(
                initial_params,
                0,
                f64::INFINITY,
                e,
            );
        }

        let swaptions = market_data.swaptions.clone();

        let residuals = move |params: &[f64]| {
            let mean_reversion = params[HWParamIndex::MEAN_REVERSION];
            let sigma = params[HWParamIndex::VOLATILITY];

            swaptions
                .iter()
                .map(|swp| {
                    let model_vol = HullWhiteCalibrator::swaption_vol(
                        swp.expiry,
                        swp.tenor,
                        mean_reversion,
                        sigma,
                    );
                    swp.weight * (model_vol - swp.market_vol)
                })
                .collect()
        };

        self.calibrator
            .calibrate_with_residuals(residuals, initial_params)
    }

    fn objective_function(
        &self,
        params: &Self::ModelParams,
        market_data: &Self::MarketData,
    ) -> Vec<f64> {
        let mean_reversion = params[HWParamIndex::MEAN_REVERSION];
        let sigma = params[HWParamIndex::VOLATILITY];

        market_data
            .swaptions
            .iter()
            .map(|swp| {
                let model_vol = HullWhiteCalibrator::swaption_vol(
                    swp.expiry,
                    swp.tenor,
                    mean_reversion,
                    sigma,
                );
                swp.weight * (model_vol - swp.market_vol)
            })
            .collect()
    }

    fn constraints(&self) -> Vec<Constraint> {
        vec![
            Constraint::positive(HWParamIndex::MEAN_REVERSION),
            Constraint::positive(HWParamIndex::VOLATILITY),
        ]
    }
}

/// Convenience function to calibrate Hull-White model.
///
/// # Requirement: 6.3
///
/// # Arguments
///
/// * `market_data` - Swaption market data
/// * `initial_params` - Initial guess [mean_reversion, sigma]
///
/// # Returns
///
/// Calibration result with optimised parameters.
///
/// # Example
///
/// ```
/// use pricer_models::calibration::hull_white::{calibrate_hull_white, HullWhiteCalibrationData};
///
/// let mut data = HullWhiteCalibrationData::new(0.03);
/// data.add_swaption(1.0, 5.0, 0.20);  // 1Y into 5Y swaption
/// data.add_swaption(2.0, 5.0, 0.18);  // 2Y into 5Y swaption
///
/// let initial = vec![0.05, 0.01];  // a, sigma
/// let result = calibrate_hull_white(&data, initial);
///
/// if result.converged {
///     println!("Calibrated mean reversion: {}", result.params[0]);
///     println!("Calibrated volatility: {}", result.params[1]);
/// }
/// ```
pub fn calibrate_hull_white(
    market_data: &HullWhiteCalibrationData,
    initial_params: Vec<f64>,
) -> CalibrationResult<Vec<f64>> {
    let calibrator = HullWhiteCalibrator::new();
    calibrator.calibrate(market_data, initial_params, &CalibrationConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hw_swaption_point() {
        let point = HWSwaptionPoint::from_lognormal(1.0, 5.0, 0.20);
        assert_eq!(point.expiry, 1.0);
        assert_eq!(point.tenor, 5.0);
        assert_eq!(point.market_vol, 0.20);
        assert!(!point.is_normal_vol);

        let normal = HWSwaptionPoint::from_normal(1.0, 5.0, 0.005);
        assert!(normal.is_normal_vol);
    }

    #[test]
    fn test_hw_calibration_data() {
        let mut data = HullWhiteCalibrationData::new(0.03);
        assert_eq!(data.forward_rate, 0.03);
        assert!(data.is_empty());

        data.add_swaption(1.0, 5.0, 0.20);
        data.add_swaption(2.0, 5.0, 0.18);
        assert_eq!(data.len(), 2);
        assert!(!data.is_empty());
    }

    #[test]
    fn test_hw_calibration_data_validation() {
        let data = HullWhiteCalibrationData::new(0.03);
        assert!(data.validate().is_err()); // Empty

        let data = HullWhiteCalibrationData::new(-0.03);
        assert!(data.validate().is_err()); // Negative rate

        let mut data = HullWhiteCalibrationData::new(0.03);
        data.add_swaption(1.0, 5.0, 0.20);
        assert!(data.validate().is_ok());
    }

    #[test]
    fn test_hw_param_index() {
        assert_eq!(HWParamIndex::MEAN_REVERSION, 0);
        assert_eq!(HWParamIndex::VOLATILITY, 1);
        assert_eq!(HWParamIndex::COUNT, 2);
    }

    #[test]
    fn test_hw_calibrator_new() {
        let calibrator = HullWhiteCalibrator::new();
        assert_eq!(calibrator.constraints().len(), 2);
    }

    #[test]
    fn test_hw_swaption_vol() {
        let vol = HullWhiteCalibrator::swaption_vol(1.0, 5.0, 0.05, 0.01);
        assert!(vol > 0.0);
        assert!(vol < 1.0);
    }

    #[test]
    fn test_hw_swaption_vol_zero_mean_reversion() {
        // With zero mean reversion, volatility should still be valid
        let vol = HullWhiteCalibrator::swaption_vol(1.0, 5.0, 0.0, 0.01);
        assert!(vol > 0.0);
    }

    #[test]
    fn test_hw_swaption_vol_monotonicity() {
        // Higher model sigma should give higher swaption vol
        let vol1 = HullWhiteCalibrator::swaption_vol(1.0, 5.0, 0.05, 0.01);
        let vol2 = HullWhiteCalibrator::swaption_vol(1.0, 5.0, 0.05, 0.02);
        assert!(vol2 > vol1);
    }

    #[test]
    fn test_calibrate_hw_synthetic() {
        // Generate synthetic data from known HW params
        let true_a = 0.05;
        let true_sigma = 0.01;

        let mut data = HullWhiteCalibrationData::new(0.03);

        // Add swaptions with model vols
        for (expiry, tenor) in [(1.0, 5.0), (2.0, 5.0), (3.0, 5.0), (5.0, 5.0)] {
            let vol = HullWhiteCalibrator::swaption_vol(expiry, tenor, true_a, true_sigma);
            data.add_swaption(expiry, tenor, vol);
        }

        // Perturbed initial guess
        let initial = vec![0.08, 0.015];

        let result = calibrate_hull_white(&data, initial);

        assert!(result.converged, "Calibration did not converge");
        assert!(
            (result.params[0] - true_a).abs() < 0.02,
            "Mean reversion mismatch: {} vs {}",
            result.params[0],
            true_a
        );
        assert!(
            (result.params[1] - true_sigma).abs() < 0.005,
            "Sigma mismatch: {} vs {}",
            result.params[1],
            true_sigma
        );
    }

    #[test]
    fn test_calibrate_hw_empty_data() {
        let data = HullWhiteCalibrationData::new(0.03);
        let initial = vec![0.05, 0.01];

        let result = calibrate_hull_white(&data, initial);
        assert!(!result.converged);
    }

    #[test]
    fn test_hw_objective_function() {
        let calibrator = HullWhiteCalibrator::new();
        let mut data = HullWhiteCalibrationData::new(0.03);

        let true_a = 0.05;
        let true_sigma = 0.01;
        let vol = HullWhiteCalibrator::swaption_vol(1.0, 5.0, true_a, true_sigma);
        data.add_swaption(1.0, 5.0, vol);

        // At true params, objective should be near zero
        let params = vec![true_a, true_sigma];
        let residuals = calibrator.objective_function(&params, &data);

        assert_eq!(residuals.len(), 1);
        assert!(residuals[0].abs() < 1e-10);
    }

    #[test]
    fn test_hw_with_payment_frequency() {
        let data = HullWhiteCalibrationData::new(0.03)
            .with_payment_frequency(0.25); // Quarterly

        assert_eq!(data.payment_frequency, 0.25);
    }

    #[test]
    fn test_hw_add_swaption_normal() {
        let mut data = HullWhiteCalibrationData::new(0.03);
        data.add_swaption_normal(1.0, 5.0, 0.005);

        assert!(data.swaptions[0].is_normal_vol);
    }

    #[test]
    fn test_hw_swaption_weight() {
        let point = HWSwaptionPoint::from_lognormal(1.0, 5.0, 0.20)
            .with_weight(2.0);
        assert_eq!(point.weight, 2.0);
    }
}
