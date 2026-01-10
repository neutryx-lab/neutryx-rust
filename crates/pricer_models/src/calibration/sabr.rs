//! SABR model calibration.
//!
//! # Requirement: 6.2
//!
//! This module provides calibration infrastructure for the SABR
//! stochastic volatility model using the Hagan formula for
//! implied volatility.
//!
//! ## Calibration Approach
//!
//! SABR parameters are calibrated by fitting the Hagan approximation
//! formula to market implied volatilities. The objective is to minimise
//! the sum of squared differences between model and market vols.
//!
//! ## Hagan Formula
//!
//! The SABR implied volatility is approximated by:
//! ```text
//! σ_B(K) = α/D(F,K) * z/χ(z) * (1 + ... )
//! ```
//! where z is related to the log-moneyness.
//!
//! ## Beta Handling
//!
//! Beta can be:
//! - Fixed at a specific value (common choice: 0.5 or 1.0)
//! - Calibrated along with other parameters

use pricer_core::traits::calibration::{
    CalibrationConfig, CalibrationResult, Calibrator, Constraint, ParameterBounds,
};

use super::{ModelCalibrator, ModelCalibratorConfig};

/// SABR smile point for calibration.
///
/// Represents a single strike/volatility observation.
#[derive(Debug, Clone, PartialEq)]
pub struct SABRSmilePoint {
    /// Strike price (or rate).
    pub strike: f64,
    /// Market implied volatility.
    pub implied_vol: f64,
    /// Weight for this point in calibration.
    pub weight: f64,
}

impl SABRSmilePoint {
    /// Create a new smile point.
    pub fn new(strike: f64, implied_vol: f64) -> Self {
        Self {
            strike,
            implied_vol,
            weight: 1.0,
        }
    }

    /// Set the weight.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

/// SABR calibration data.
///
/// # Requirement: 6.2
///
/// Contains market data for SABR calibration including
/// ATM volatility and smile points.
#[derive(Debug, Clone)]
pub struct SABRCalibrationData {
    /// Forward price/rate.
    pub forward: f64,
    /// Time to expiry (years).
    pub expiry: f64,
    /// ATM implied volatility.
    pub atm_vol: f64,
    /// Smile points (off-ATM strikes and vols).
    pub smile_points: Vec<SABRSmilePoint>,
    /// Fixed beta value (None means calibrate beta).
    pub fixed_beta: Option<f64>,
}

impl SABRCalibrationData {
    /// Create new calibration data.
    pub fn new(forward: f64, expiry: f64, atm_vol: f64) -> Self {
        Self {
            forward,
            expiry,
            atm_vol,
            smile_points: Vec::new(),
            fixed_beta: None,
        }
    }

    /// Set fixed beta value.
    pub fn with_fixed_beta(mut self, beta: f64) -> Self {
        self.fixed_beta = Some(beta);
        self
    }

    /// Add a smile point.
    pub fn add_smile_point(&mut self, strike: f64, vol: f64) {
        self.smile_points.push(SABRSmilePoint::new(strike, vol));
    }

    /// Add a weighted smile point.
    pub fn add_weighted_smile_point(&mut self, strike: f64, vol: f64, weight: f64) {
        self.smile_points
            .push(SABRSmilePoint::new(strike, vol).with_weight(weight));
    }

    /// Total number of points (ATM + smile).
    pub fn len(&self) -> usize {
        1 + self.smile_points.len()
    }

    /// Check if there's only ATM data.
    pub fn is_atm_only(&self) -> bool {
        self.smile_points.is_empty()
    }

    /// Validate the data.
    pub fn validate(&self) -> Result<(), String> {
        if self.forward <= 0.0 {
            return Err("Forward must be positive".to_string());
        }
        if self.expiry <= 0.0 {
            return Err("Expiry must be positive".to_string());
        }
        if self.atm_vol <= 0.0 {
            return Err("ATM volatility must be positive".to_string());
        }
        if let Some(beta) = self.fixed_beta {
            if beta < 0.0 || beta > 1.0 {
                return Err("Beta must be in [0, 1]".to_string());
            }
        }
        for (i, point) in self.smile_points.iter().enumerate() {
            if point.strike <= 0.0 {
                return Err(format!("Smile point {}: strike must be positive", i));
            }
            if point.implied_vol <= 0.0 {
                return Err(format!("Smile point {}: vol must be positive", i));
            }
        }
        Ok(())
    }
}

/// SABR parameter indices for calibration.
///
/// When beta is fixed:
/// - params[0] = alpha
/// - params[1] = rho
/// - params[2] = nu
///
/// When beta is calibrated:
/// - params[0] = alpha
/// - params[1] = beta
/// - params[2] = rho
/// - params[3] = nu
#[derive(Debug, Clone, Copy)]
pub struct SABRParamIndex;

impl SABRParamIndex {
    /// Alpha index (always 0).
    pub const ALPHA: usize = 0;
    /// Beta index (when calibrated).
    pub const BETA: usize = 1;
    /// Rho index (depends on beta mode).
    pub fn rho(fixed_beta: bool) -> usize {
        if fixed_beta { 1 } else { 2 }
    }
    /// Nu index (depends on beta mode).
    pub fn nu(fixed_beta: bool) -> usize {
        if fixed_beta { 2 } else { 3 }
    }
    /// Number of parameters.
    pub fn count(fixed_beta: bool) -> usize {
        if fixed_beta { 3 } else { 4 }
    }
}

/// SABR model calibrator.
///
/// # Requirement: 6.2
///
/// Calibrates SABR model parameters to market implied volatility
/// data using Levenberg-Marquardt optimisation with the Hagan
/// approximation formula.
///
/// ## Parameter Bounds
///
/// - alpha > 0 (initial volatility)
/// - beta in [0, 1] (CEV exponent)
/// - rho in (-1, 1) (correlation)
/// - nu > 0 (vol-of-vol)
#[derive(Debug, Clone)]
pub struct SABRCalibrator {
    /// Underlying model calibrator.
    calibrator: ModelCalibrator,
    /// Fixed beta value (None means calibrate).
    fixed_beta: Option<f64>,
}

impl Default for SABRCalibrator {
    fn default() -> Self {
        Self::new()
    }
}

impl SABRCalibrator {
    /// Create a new SABR calibrator (calibrates all params including beta).
    pub fn new() -> Self {
        let config = ModelCalibratorConfig::default().with_bounds(vec![
            ParameterBounds::new(0.001, 2.0),     // alpha: (0, 2]
            ParameterBounds::unit_interval(),     // beta: [0, 1]
            ParameterBounds::new(-0.999, 0.999),  // rho: (-1, 1)
            ParameterBounds::new(0.001, 2.0),     // nu: (0, 2]
        ]);

        Self {
            calibrator: ModelCalibrator::new(config),
            fixed_beta: None,
        }
    }

    /// Create a calibrator with fixed beta.
    pub fn with_fixed_beta(beta: f64) -> Self {
        let config = ModelCalibratorConfig::default().with_bounds(vec![
            ParameterBounds::new(0.001, 2.0),     // alpha: (0, 2]
            ParameterBounds::new(-0.999, 0.999),  // rho: (-1, 1)
            ParameterBounds::new(0.001, 2.0),     // nu: (0, 2]
        ]);

        Self {
            calibrator: ModelCalibrator::new(config),
            fixed_beta: Some(beta),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: ModelCalibratorConfig, fixed_beta: Option<f64>) -> Self {
        Self {
            calibrator: ModelCalibrator::new(config),
            fixed_beta,
        }
    }

    /// Get the fixed beta value if any.
    pub fn fixed_beta(&self) -> Option<f64> {
        self.fixed_beta
    }

    /// Compute SABR implied volatility using Hagan formula.
    pub fn implied_vol(
        forward: f64,
        strike: f64,
        expiry: f64,
        alpha: f64,
        beta: f64,
        rho: f64,
        nu: f64,
    ) -> f64 {
        sabr_hagan_vol(forward, strike, expiry, alpha, beta, rho, nu)
    }

    /// Extract full parameters from calibration vector.
    fn extract_params(&self, params: &[f64], data: &SABRCalibrationData) -> (f64, f64, f64, f64) {
        let beta = self.fixed_beta.or(data.fixed_beta).unwrap_or(params[SABRParamIndex::BETA]);
        let fixed = self.fixed_beta.is_some() || data.fixed_beta.is_some();

        let alpha = params[SABRParamIndex::ALPHA];
        let rho = params[SABRParamIndex::rho(fixed)];
        let nu = params[SABRParamIndex::nu(fixed)];

        (alpha, beta, rho, nu)
    }
}

impl Calibrator for SABRCalibrator {
    type MarketData = SABRCalibrationData;
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

        let forward = market_data.forward;
        let expiry = market_data.expiry;
        let atm_vol = market_data.atm_vol;
        let smile_points = market_data.smile_points.clone();
        let fixed_beta = self.fixed_beta.or(market_data.fixed_beta);

        let residuals = move |params: &[f64]| {
            let (alpha, beta, rho, nu) = if let Some(beta_val) = fixed_beta {
                (params[0], beta_val, params[1], params[2])
            } else {
                (params[0], params[1], params[2], params[3])
            };

            let mut resids = Vec::with_capacity(1 + smile_points.len());

            // ATM residual (higher weight for ATM)
            let atm_model_vol = sabr_hagan_vol(forward, forward, expiry, alpha, beta, rho, nu);
            resids.push(2.0 * (atm_model_vol - atm_vol));

            // Smile residuals
            for point in &smile_points {
                let model_vol = sabr_hagan_vol(forward, point.strike, expiry, alpha, beta, rho, nu);
                resids.push(point.weight * (model_vol - point.implied_vol));
            }

            resids
        };

        self.calibrator
            .calibrate_with_residuals(residuals, initial_params)
    }

    fn objective_function(
        &self,
        params: &Self::ModelParams,
        market_data: &Self::MarketData,
    ) -> Vec<f64> {
        let (alpha, beta, rho, nu) = self.extract_params(params, market_data);
        let forward = market_data.forward;
        let expiry = market_data.expiry;

        let mut resids = Vec::with_capacity(1 + market_data.smile_points.len());

        // ATM residual
        let atm_model = sabr_hagan_vol(forward, forward, expiry, alpha, beta, rho, nu);
        resids.push(2.0 * (atm_model - market_data.atm_vol));

        // Smile residuals
        for point in &market_data.smile_points {
            let model_vol = sabr_hagan_vol(forward, point.strike, expiry, alpha, beta, rho, nu);
            resids.push(point.weight * (model_vol - point.implied_vol));
        }

        resids
    }

    fn constraints(&self) -> Vec<Constraint> {
        if self.fixed_beta.is_some() {
            vec![
                Constraint::positive(0),              // alpha > 0
                Constraint::bounds(1, -0.999, 0.999), // rho in (-1, 1)
                Constraint::positive(2),              // nu > 0
            ]
        } else {
            vec![
                Constraint::positive(0),              // alpha > 0
                Constraint::bounds(1, 0.0, 1.0),      // beta in [0, 1]
                Constraint::bounds(2, -0.999, 0.999), // rho in (-1, 1)
                Constraint::positive(3),              // nu > 0
            ]
        }
    }
}

/// Convenience function to calibrate SABR model.
///
/// # Requirement: 6.2
///
/// # Arguments
///
/// * `market_data` - Market implied volatility data
/// * `initial_params` - Initial parameter guess
///   - With fixed beta: [alpha, rho, nu]
///   - Without fixed beta: [alpha, beta, rho, nu]
///
/// # Returns
///
/// Calibration result with optimised parameters.
///
/// # Example
///
/// ```
/// use pricer_models::calibration::sabr::{calibrate_sabr, SABRCalibrationData};
///
/// let mut data = SABRCalibrationData::new(0.03, 1.0, 0.20);
/// data.add_smile_point(0.02, 0.25);  // Low strike, higher vol
/// data.add_smile_point(0.04, 0.22);  // High strike, higher vol
///
/// // With fixed beta = 0.5
/// let data = data.with_fixed_beta(0.5);
/// let initial = vec![0.04, -0.3, 0.4];  // alpha, rho, nu
/// let result = calibrate_sabr(&data, initial);
///
/// if result.converged {
///     println!("Calibrated alpha: {}", result.params[0]);
/// }
/// ```
pub fn calibrate_sabr(
    market_data: &SABRCalibrationData,
    initial_params: Vec<f64>,
) -> CalibrationResult<Vec<f64>> {
    let calibrator = if market_data.fixed_beta.is_some() {
        SABRCalibrator::with_fixed_beta(market_data.fixed_beta.unwrap())
    } else {
        SABRCalibrator::new()
    };
    calibrator.calibrate(market_data, initial_params, &CalibrationConfig::default())
}

/// Calibrate SABR with explicit fixed beta.
///
/// # Arguments
///
/// * `market_data` - Market data (fixed_beta field is ignored)
/// * `beta` - Fixed beta value
/// * `initial_params` - Initial guess [alpha, rho, nu]
pub fn calibrate_sabr_fixed_beta(
    market_data: &SABRCalibrationData,
    beta: f64,
    initial_params: Vec<f64>,
) -> CalibrationResult<Vec<f64>> {
    let calibrator = SABRCalibrator::with_fixed_beta(beta);
    calibrator.calibrate(market_data, initial_params, &CalibrationConfig::default())
}

// =============================================================================
// Hagan formula implementation
// =============================================================================

/// SABR implied volatility using Hagan formula.
///
/// This is the standard approximation from Hagan et al. (2002).
fn sabr_hagan_vol(
    forward: f64,
    strike: f64,
    expiry: f64,
    alpha: f64,
    beta: f64,
    rho: f64,
    nu: f64,
) -> f64 {
    let eps = 1e-10;

    // Handle near-ATM case
    if (forward - strike).abs() < eps * forward {
        return sabr_atm_vol(forward, expiry, alpha, beta, rho, nu);
    }

    let one_minus_beta = 1.0 - beta;
    let log_fk = (forward / strike).ln();
    let fk_mid = (forward * strike).powf(one_minus_beta / 2.0);

    // z = (nu/alpha) * (FK)^((1-beta)/2) * ln(F/K)
    let z = (nu / alpha) * fk_mid * log_fk;

    // chi(z) = ln[(sqrt(1-2*rho*z+z^2)+z-rho)/(1-rho)]
    let sqrt_term = (1.0 - 2.0 * rho * z + z * z).sqrt();
    let chi_z = ((sqrt_term + z - rho) / (1.0 - rho)).ln();

    // Handle chi(z) near zero
    let z_over_chi = if chi_z.abs() < eps {
        1.0
    } else {
        z / chi_z
    };

    // Denominator from log-moneyness expansion
    let log_fk_2 = log_fk * log_fk;
    let log_fk_4 = log_fk_2 * log_fk_2;
    let one_minus_beta_2 = one_minus_beta * one_minus_beta;
    let one_minus_beta_4 = one_minus_beta_2 * one_minus_beta_2;

    let denom = 1.0
        + one_minus_beta_2 * log_fk_2 / 24.0
        + one_minus_beta_4 * log_fk_4 / 1920.0;

    // Higher-order corrections
    let term1 = one_minus_beta_2 * alpha * alpha / (24.0 * fk_mid * fk_mid);
    let term2 = 0.25 * rho * beta * nu * alpha / fk_mid;
    let term3 = (2.0 - 3.0 * rho * rho) * nu * nu / 24.0;

    let higher_order = 1.0 + (term1 + term2 + term3) * expiry;

    // Final volatility
    (alpha / fk_mid) * z_over_chi * higher_order / denom
}

/// SABR ATM volatility approximation.
fn sabr_atm_vol(
    forward: f64,
    expiry: f64,
    alpha: f64,
    beta: f64,
    rho: f64,
    nu: f64,
) -> f64 {
    let one_minus_beta = 1.0 - beta;
    let f_pow = forward.powf(one_minus_beta);

    // Base ATM vol
    let vol_0 = alpha / f_pow;

    // Higher-order ATM corrections
    let term1 = one_minus_beta * one_minus_beta * alpha * alpha / (24.0 * f_pow * f_pow);
    let term2 = 0.25 * rho * beta * nu * alpha / f_pow;
    let term3 = (2.0 - 3.0 * rho * rho) * nu * nu / 24.0;

    vol_0 * (1.0 + (term1 + term2 + term3) * expiry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sabr_smile_point() {
        let point = SABRSmilePoint::new(0.02, 0.25);
        assert_eq!(point.strike, 0.02);
        assert_eq!(point.implied_vol, 0.25);
        assert_eq!(point.weight, 1.0);

        let weighted = point.with_weight(2.0);
        assert_eq!(weighted.weight, 2.0);
    }

    #[test]
    fn test_sabr_calibration_data() {
        let mut data = SABRCalibrationData::new(0.03, 1.0, 0.20);
        assert_eq!(data.forward, 0.03);
        assert_eq!(data.expiry, 1.0);
        assert_eq!(data.atm_vol, 0.20);
        assert!(data.is_atm_only());
        assert_eq!(data.len(), 1);

        data.add_smile_point(0.02, 0.25);
        data.add_smile_point(0.04, 0.22);
        assert!(!data.is_atm_only());
        assert_eq!(data.len(), 3);
    }

    #[test]
    fn test_sabr_calibration_data_validation() {
        let data = SABRCalibrationData::new(-0.03, 1.0, 0.20);
        assert!(data.validate().is_err());

        let data = SABRCalibrationData::new(0.03, -1.0, 0.20);
        assert!(data.validate().is_err());

        let data = SABRCalibrationData::new(0.03, 1.0, 0.20).with_fixed_beta(1.5);
        assert!(data.validate().is_err());

        let data = SABRCalibrationData::new(0.03, 1.0, 0.20);
        assert!(data.validate().is_ok());
    }

    #[test]
    fn test_sabr_param_index() {
        assert_eq!(SABRParamIndex::ALPHA, 0);
        assert_eq!(SABRParamIndex::BETA, 1);

        assert_eq!(SABRParamIndex::rho(true), 1);
        assert_eq!(SABRParamIndex::rho(false), 2);

        assert_eq!(SABRParamIndex::nu(true), 2);
        assert_eq!(SABRParamIndex::nu(false), 3);

        assert_eq!(SABRParamIndex::count(true), 3);
        assert_eq!(SABRParamIndex::count(false), 4);
    }

    #[test]
    fn test_sabr_calibrator_new() {
        let calibrator = SABRCalibrator::new();
        assert!(calibrator.fixed_beta().is_none());
    }

    #[test]
    fn test_sabr_calibrator_fixed_beta() {
        let calibrator = SABRCalibrator::with_fixed_beta(0.5);
        assert_eq!(calibrator.fixed_beta(), Some(0.5));
    }

    #[test]
    fn test_sabr_hagan_vol_atm() {
        // ATM should return positive vol
        let vol = sabr_hagan_vol(0.03, 0.03, 1.0, 0.04, 0.5, -0.3, 0.4);
        assert!(vol > 0.0);
        assert!(vol < 1.0);
    }

    #[test]
    fn test_sabr_hagan_vol_smile() {
        let forward = 0.03;
        let expiry = 1.0;
        let alpha = 0.04;
        let beta = 0.5;
        let rho = -0.3;
        let nu = 0.4;

        let vol_atm = sabr_hagan_vol(forward, forward, expiry, alpha, beta, rho, nu);
        let vol_low = sabr_hagan_vol(forward, 0.02, expiry, alpha, beta, rho, nu);
        let vol_high = sabr_hagan_vol(forward, 0.04, expiry, alpha, beta, rho, nu);

        // All should be positive
        assert!(vol_atm > 0.0);
        assert!(vol_low > 0.0);
        assert!(vol_high > 0.0);

        // Typical skew: low strikes have higher vol with negative rho
        // (this depends on parameters, so just check they're different)
        assert!((vol_low - vol_atm).abs() > 0.001);
    }

    #[test]
    fn test_sabr_implied_vol_matches_internal() {
        let vol1 = SABRCalibrator::implied_vol(0.03, 0.025, 1.0, 0.04, 0.5, -0.3, 0.4);
        let vol2 = sabr_hagan_vol(0.03, 0.025, 1.0, 0.04, 0.5, -0.3, 0.4);
        assert!((vol1 - vol2).abs() < 1e-15);
    }

    #[test]
    fn test_calibrate_sabr_fixed_beta_synthetic() {
        // Generate synthetic data from known SABR params
        let true_alpha = 0.04;
        let true_beta = 0.5;
        let true_rho = -0.3;
        let true_nu = 0.4;
        let forward = 0.03;
        let expiry = 1.0;

        let atm_vol = sabr_hagan_vol(forward, forward, expiry, true_alpha, true_beta, true_rho, true_nu);

        let mut data = SABRCalibrationData::new(forward, expiry, atm_vol)
            .with_fixed_beta(true_beta);

        // Add smile points
        for strike in [0.02, 0.025, 0.035, 0.04] {
            let vol = sabr_hagan_vol(forward, strike, expiry, true_alpha, true_beta, true_rho, true_nu);
            data.add_smile_point(strike, vol);
        }

        // Perturbed initial guess
        let initial = vec![0.05, -0.1, 0.3]; // alpha, rho, nu

        let result = calibrate_sabr(&data, initial);

        // Should converge and recover approximately correct parameters
        assert!(result.converged, "Calibration did not converge");
        assert!((result.params[0] - true_alpha).abs() < 0.01, "Alpha mismatch");
        assert!((result.params[1] - true_rho).abs() < 0.1, "Rho mismatch");
        assert!((result.params[2] - true_nu).abs() < 0.1, "Nu mismatch");
    }

    #[test]
    fn test_calibrate_sabr_full_synthetic() {
        // Calibrate all 4 parameters
        let true_alpha = 0.04;
        let true_beta = 0.5;
        let true_rho = -0.3;
        let true_nu = 0.4;
        let forward = 0.03;
        let expiry = 1.0;

        let atm_vol = sabr_hagan_vol(forward, forward, expiry, true_alpha, true_beta, true_rho, true_nu);

        let mut data = SABRCalibrationData::new(forward, expiry, atm_vol);

        // Add more smile points for full calibration
        for strike in [0.015, 0.02, 0.025, 0.035, 0.04, 0.045] {
            let vol = sabr_hagan_vol(forward, strike, expiry, true_alpha, true_beta, true_rho, true_nu);
            data.add_smile_point(strike, vol);
        }

        // Perturbed initial guess
        let initial = vec![0.05, 0.4, -0.1, 0.3]; // alpha, beta, rho, nu

        let result = calibrate_sabr(&data, initial);

        // Should converge
        assert!(result.converged, "Calibration did not converge");
        // Parameters should be in valid ranges
        assert!(result.params[0] > 0.0); // alpha
        assert!(result.params[1] >= 0.0 && result.params[1] <= 1.0); // beta
        assert!(result.params[2] > -1.0 && result.params[2] < 1.0); // rho
        assert!(result.params[3] > 0.0); // nu
    }

    #[test]
    fn test_calibrate_sabr_atm_only() {
        // With only ATM, calibration is underdetermined but should still run
        let data = SABRCalibrationData::new(0.03, 1.0, 0.20).with_fixed_beta(0.5);
        let initial = vec![0.04, -0.3, 0.4];

        let result = calibrate_sabr(&data, initial);

        // May not converge well but should return valid params
        assert!(result.params[0] > 0.0);
    }

    #[test]
    fn test_sabr_constraints() {
        let calibrator_full = SABRCalibrator::new();
        assert_eq!(calibrator_full.constraints().len(), 4);

        let calibrator_fixed = SABRCalibrator::with_fixed_beta(0.5);
        assert_eq!(calibrator_fixed.constraints().len(), 3);
    }

    #[test]
    fn test_calibrate_sabr_fixed_beta_fn() {
        let mut data = SABRCalibrationData::new(0.03, 1.0, 0.20);
        data.add_smile_point(0.02, 0.25);

        let initial = vec![0.04, -0.3, 0.4];
        let result = calibrate_sabr_fixed_beta(&data, 0.5, initial);

        assert!(result.params[0] > 0.0);
    }

    #[test]
    fn test_sabr_hagan_vol_beta_extremes() {
        let forward = 0.03;
        let expiry = 1.0;
        let alpha = 0.04;
        let rho = -0.3;
        let nu = 0.4;

        // beta = 0 (Normal SABR)
        let vol_beta0 = sabr_hagan_vol(forward, 0.025, expiry, alpha, 0.0, rho, nu);
        assert!(vol_beta0 > 0.0);

        // beta = 1 (Lognormal SABR)
        let vol_beta1 = sabr_hagan_vol(forward, 0.025, expiry, alpha, 1.0, rho, nu);
        assert!(vol_beta1 > 0.0);
    }
}
