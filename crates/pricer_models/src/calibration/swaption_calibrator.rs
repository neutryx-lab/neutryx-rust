//! Swaption volatility surface calibration.
//!
//! This module provides calibration of model parameters to swaption
//! volatility surfaces.

use pricer_core::traits::calibration::{
    CalibrationConfig, CalibrationResult, Calibrator, Constraint, ParameterBounds,
};

use super::{ModelCalibrator, ModelCalibratorConfig};

/// Type of volatility used in calibration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolatilityType {
    /// Log-normal (Black76) volatility.
    LogNormal,
    /// Normal (Bachelier) volatility.
    Normal,
}

impl Default for VolatilityType {
    fn default() -> Self {
        Self::LogNormal
    }
}

/// A single point on the swaption volatility surface.
#[derive(Debug, Clone, PartialEq)]
pub struct SwaptionMarketPoint {
    /// Option expiry in years.
    pub expiry: f64,
    /// Underlying swap tenor in years.
    pub tenor: f64,
    /// Strike (as an absolute rate, e.g., 0.03 for 3%).
    pub strike: f64,
    /// Market volatility.
    pub volatility: f64,
    /// Weight for this point in calibration.
    pub weight: f64,
}

impl SwaptionMarketPoint {
    /// Create a new market point.
    pub fn new(expiry: f64, tenor: f64, strike: f64, volatility: f64) -> Self {
        Self {
            expiry,
            tenor,
            strike,
            volatility,
            weight: 1.0,
        }
    }

    /// Set the weight.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

/// Swaption market data for calibration.
#[derive(Debug, Clone)]
pub struct SwaptionMarketData {
    /// Market points (expiry, tenor, strike, vol).
    pub points: Vec<SwaptionMarketPoint>,
    /// Volatility type.
    pub vol_type: VolatilityType,
    /// Forward swap rates for each point.
    pub forwards: Vec<f64>,
    /// Annuities for each point.
    pub annuities: Vec<f64>,
}

impl SwaptionMarketData {
    /// Create new swaption market data.
    pub fn new(vol_type: VolatilityType) -> Self {
        Self {
            points: Vec::new(),
            vol_type,
            forwards: Vec::new(),
            annuities: Vec::new(),
        }
    }

    /// Add a market point.
    pub fn add_point(&mut self, point: SwaptionMarketPoint, forward: f64, annuity: f64) {
        self.points.push(point);
        self.forwards.push(forward);
        self.annuities.push(annuity);
    }

    /// Number of points.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

/// Swaption calibrator for model parameters.
///
/// This calibrator fits model parameters to match market swaption
/// implied volatilities.
///
/// # Model Types
///
/// The calibrator can be used with different pricing models:
/// - Hull-White 1F: Calibrate mean reversion and volatility
/// - SABR: Calibrate alpha, beta, rho, nu
///
/// # Example
///
/// ```ignore
/// use pricer_models::calibration::{SwaptionCalibrator, SwaptionMarketData, VolatilityType};
///
/// let market_data = SwaptionMarketData::new(VolatilityType::LogNormal);
/// // ... add market points ...
///
/// let calibrator = SwaptionCalibrator::new_sabr();
/// let result = calibrator.calibrate(&market_data, initial_params);
/// ```
#[derive(Debug, Clone)]
pub struct SwaptionCalibrator {
    /// Underlying model calibrator.
    calibrator: ModelCalibrator,
    /// Parameter names for debugging.
    param_names: Vec<String>,
    /// Model volatility function.
    model_type: SwaptionModelType,
}

/// Type of model to calibrate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwaptionModelType {
    /// Hull-White 1-factor model.
    HullWhite1F,
    /// SABR model.
    Sabr,
    /// Flat volatility (for testing).
    FlatVol,
}

impl SwaptionCalibrator {
    /// Create a new swaption calibrator.
    pub fn new(config: ModelCalibratorConfig, model_type: SwaptionModelType) -> Self {
        let param_names = match model_type {
            SwaptionModelType::HullWhite1F => vec!["mean_reversion".into(), "volatility".into()],
            SwaptionModelType::Sabr => {
                vec!["alpha".into(), "beta".into(), "rho".into(), "nu".into()]
            }
            SwaptionModelType::FlatVol => vec!["vol".into()],
        };

        Self {
            calibrator: ModelCalibrator::new(config),
            param_names,
            model_type,
        }
    }

    /// Create a Hull-White 1F calibrator.
    pub fn new_hull_white() -> Self {
        let config = ModelCalibratorConfig::default().with_bounds(vec![
            ParameterBounds::positive(), // mean_reversion > 0
            ParameterBounds::positive(), // volatility > 0
        ]);

        Self::new(config, SwaptionModelType::HullWhite1F)
    }

    /// Create a SABR calibrator.
    pub fn new_sabr() -> Self {
        let config = ModelCalibratorConfig::default().with_bounds(vec![
            ParameterBounds::positive(),     // alpha > 0
            ParameterBounds::unit_interval(), // beta in [0, 1]
            ParameterBounds::new(-0.999, 0.999), // rho in (-1, 1)
            ParameterBounds::positive(),     // nu > 0
        ]);

        Self::new(config, SwaptionModelType::Sabr)
    }

    /// Create a flat volatility calibrator (for testing).
    pub fn new_flat_vol() -> Self {
        let config = ModelCalibratorConfig::default()
            .with_bounds(vec![ParameterBounds::positive()]);

        Self::new(config, SwaptionModelType::FlatVol)
    }

    /// Get parameter names.
    pub fn param_names(&self) -> &[String] {
        &self.param_names
    }

    /// Get model type.
    pub fn model_type(&self) -> SwaptionModelType {
        self.model_type
    }

    /// Compute model volatility for given parameters and market point.
    fn model_vol(&self, params: &[f64], point: &SwaptionMarketPoint, forward: f64) -> f64 {
        match self.model_type {
            SwaptionModelType::FlatVol => params[0],
            SwaptionModelType::HullWhite1F => {
                // Hull-White implied vol approximation
                let kappa = params[0]; // mean reversion
                let sigma = params[1]; // volatility
                let t = point.expiry;

                // Approximate Black volatility from HW1F
                // sigma_black ≈ sigma * sqrt((1 - exp(-2*kappa*t)) / (2*kappa*t))
                let factor = if kappa * t > 1e-6 {
                    ((1.0 - (-2.0 * kappa * t).exp()) / (2.0 * kappa * t)).sqrt()
                } else {
                    1.0
                };
                sigma * factor
            }
            SwaptionModelType::Sabr => {
                // SABR implied volatility (Hagan approximation)
                let alpha = params[0];
                let beta = params[1];
                let rho = params[2];
                let nu = params[3];

                sabr_implied_vol(forward, point.strike, point.expiry, alpha, beta, rho, nu)
            }
        }
    }
}

impl Calibrator for SwaptionCalibrator {
    type MarketData = SwaptionMarketData;
    type ModelParams = Vec<f64>;

    fn calibrate(
        &self,
        market_data: &Self::MarketData,
        initial_params: Self::ModelParams,
        _config: &CalibrationConfig,
    ) -> CalibrationResult<Self::ModelParams> {
        if market_data.is_empty() {
            return CalibrationResult::not_converged(
                initial_params,
                0,
                f64::INFINITY,
                "No market data provided".to_string(),
            );
        }

        let residuals = |params: &[f64]| {
            market_data
                .points
                .iter()
                .zip(&market_data.forwards)
                .map(|(point, &forward)| {
                    let model_vol = self.model_vol(params, point, forward);
                    point.weight * (model_vol - point.volatility)
                })
                .collect()
        };

        self.calibrator.calibrate_with_residuals(residuals, initial_params)
    }

    fn objective_function(
        &self,
        params: &Self::ModelParams,
        market_data: &Self::MarketData,
    ) -> Vec<f64> {
        market_data
            .points
            .iter()
            .zip(&market_data.forwards)
            .map(|(point, &forward)| {
                let model_vol = self.model_vol(params, point, forward);
                point.weight * (model_vol - point.volatility)
            })
            .collect()
    }

    fn constraints(&self) -> Vec<Constraint> {
        match self.model_type {
            SwaptionModelType::FlatVol => vec![Constraint::positive(0)],
            SwaptionModelType::HullWhite1F => {
                vec![Constraint::positive(0), Constraint::positive(1)]
            }
            SwaptionModelType::Sabr => vec![
                Constraint::positive(0), // alpha > 0
                Constraint::bounds(1, 0.0, 1.0), // beta in [0, 1]
                Constraint::bounds(2, -0.999, 0.999), // rho in (-1, 1)
                Constraint::positive(3), // nu > 0
            ],
        }
    }
}

/// SABR implied volatility using Hagan approximation.
///
/// This is a simplified implementation for demonstration.
/// Production use would require more robust handling of edge cases.
fn sabr_implied_vol(forward: f64, strike: f64, expiry: f64, alpha: f64, beta: f64, rho: f64, nu: f64) -> f64 {
    let eps = 1e-10;

    // Handle ATM case
    if (forward - strike).abs() < eps {
        let f_mid = forward.powf(1.0 - beta);
        let vol = alpha / f_mid;

        // ATM correction
        let term1 = ((1.0 - beta).powi(2) / 24.0) * alpha.powi(2) / f_mid.powi(2);
        let term2 = 0.25 * rho * beta * nu * alpha / f_mid;
        let term3 = (2.0 - 3.0 * rho.powi(2)) / 24.0 * nu.powi(2);

        return vol * (1.0 + (term1 + term2 + term3) * expiry);
    }

    // Non-ATM case
    let log_fk = (forward / strike).ln();
    let f_mid = (forward * strike).powf((1.0 - beta) / 2.0);

    let z = (nu / alpha) * f_mid * log_fk;
    let x_z = ((1.0 - 2.0 * rho * z + z.powi(2)).sqrt() + z - rho) / (1.0 - rho);

    let x_z_ratio = if x_z.abs() < eps {
        1.0
    } else {
        z / x_z.ln()
    };

    let vol_0 = alpha / f_mid * x_z_ratio;

    // Higher order corrections
    let term1 = ((1.0 - beta).powi(2) / 24.0) * log_fk.powi(2);
    let term2 = ((1.0 - beta).powi(4) / 1920.0) * log_fk.powi(4);
    let denominator = 1.0 + term1 + term2;

    let num1 = ((1.0 - beta).powi(2) / 24.0) * alpha.powi(2) / f_mid.powi(2);
    let num2 = 0.25 * rho * beta * nu * alpha / f_mid;
    let num3 = (2.0 - 3.0 * rho.powi(2)) / 24.0 * nu.powi(2);
    let numerator = 1.0 + (num1 + num2 + num3) * expiry;

    vol_0 * numerator / denominator
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_market_data() -> SwaptionMarketData {
        let mut data = SwaptionMarketData::new(VolatilityType::LogNormal);

        // Add ATM swaptions with different expiries
        let vol = 0.20; // 20% flat vol
        for expiry in [0.5, 1.0, 2.0, 3.0, 5.0] {
            let point = SwaptionMarketPoint::new(expiry, 5.0, 0.03, vol);
            data.add_point(point, 0.03, 1.0);
        }

        data
    }

    #[test]
    fn test_swaption_market_point() {
        let point = SwaptionMarketPoint::new(1.0, 5.0, 0.03, 0.20);
        assert_eq!(point.expiry, 1.0);
        assert_eq!(point.tenor, 5.0);
        assert_eq!(point.strike, 0.03);
        assert_eq!(point.volatility, 0.20);
        assert_eq!(point.weight, 1.0);

        let weighted = point.with_weight(2.0);
        assert_eq!(weighted.weight, 2.0);
    }

    #[test]
    fn test_swaption_market_data() {
        let data = create_test_market_data();
        assert_eq!(data.len(), 5);
        assert!(!data.is_empty());
    }

    #[test]
    fn test_flat_vol_calibrator() {
        let calibrator = SwaptionCalibrator::new_flat_vol();
        assert_eq!(calibrator.model_type(), SwaptionModelType::FlatVol);
        assert_eq!(calibrator.param_names().len(), 1);
    }

    #[test]
    fn test_hull_white_calibrator() {
        let calibrator = SwaptionCalibrator::new_hull_white();
        assert_eq!(calibrator.model_type(), SwaptionModelType::HullWhite1F);
        assert_eq!(calibrator.param_names().len(), 2);
    }

    #[test]
    fn test_sabr_calibrator() {
        let calibrator = SwaptionCalibrator::new_sabr();
        assert_eq!(calibrator.model_type(), SwaptionModelType::Sabr);
        assert_eq!(calibrator.param_names().len(), 4);
    }

    #[test]
    fn test_calibrate_flat_vol() {
        let calibrator = SwaptionCalibrator::new_flat_vol();
        let market_data = create_test_market_data();

        let result = calibrator.calibrate(
            &market_data,
            vec![0.15], // Initial guess: 15%
            &CalibrationConfig::default(),
        );

        assert!(result.converged);
        // Should converge to 20%
        assert!((result.params[0] - 0.20).abs() < 0.01);
    }

    #[test]
    fn test_calibrate_empty_data() {
        let calibrator = SwaptionCalibrator::new_flat_vol();
        let market_data = SwaptionMarketData::new(VolatilityType::LogNormal);

        let result = calibrator.calibrate(
            &market_data,
            vec![0.15],
            &CalibrationConfig::default(),
        );

        assert!(!result.converged);
    }

    #[test]
    fn test_objective_function() {
        let calibrator = SwaptionCalibrator::new_flat_vol();
        let market_data = create_test_market_data();

        let residuals = calibrator.objective_function(&vec![0.20], &market_data);

        // At correct vol, residuals should be near zero
        for r in &residuals {
            assert!(r.abs() < 1e-10);
        }
    }

    #[test]
    fn test_constraints() {
        let flat_calibrator = SwaptionCalibrator::new_flat_vol();
        assert_eq!(flat_calibrator.constraints().len(), 1);

        let hw_calibrator = SwaptionCalibrator::new_hull_white();
        assert_eq!(hw_calibrator.constraints().len(), 2);

        let sabr_calibrator = SwaptionCalibrator::new_sabr();
        assert_eq!(sabr_calibrator.constraints().len(), 4);
    }

    #[test]
    fn test_sabr_implied_vol_atm() {
        let forward = 0.03;
        let strike = 0.03;
        let expiry = 1.0;
        let alpha = 0.04;
        let beta = 0.5;
        let rho = -0.3;
        let nu = 0.4;

        let vol = sabr_implied_vol(forward, strike, expiry, alpha, beta, rho, nu);

        // Should produce a reasonable volatility
        assert!(vol > 0.0);
        assert!(vol < 1.0); // Less than 100%
    }

    #[test]
    fn test_sabr_implied_vol_otm() {
        let forward = 0.03;
        let strike = 0.04; // OTM
        let expiry = 1.0;
        let alpha = 0.04;
        let beta = 0.5;
        let rho = -0.3;
        let nu = 0.4;

        let vol = sabr_implied_vol(forward, strike, expiry, alpha, beta, rho, nu);

        // Should produce a reasonable volatility
        assert!(vol > 0.0);
        assert!(vol < 1.0);
    }

    #[test]
    fn test_volatility_type_default() {
        let vol_type = VolatilityType::default();
        assert_eq!(vol_type, VolatilityType::LogNormal);
    }

    #[test]
    fn test_calibrate_sabr_smoke() {
        // Smoke test for SABR calibration
        let calibrator = SwaptionCalibrator::new_sabr();

        // Create market data with SABR-like smile
        let mut data = SwaptionMarketData::new(VolatilityType::LogNormal);

        let forward = 0.03;
        let expiry = 1.0;

        // Add points with a smile pattern
        for (strike, vol) in [(0.02, 0.25), (0.025, 0.22), (0.03, 0.20), (0.035, 0.21), (0.04, 0.24)]
        {
            let point = SwaptionMarketPoint::new(expiry, 5.0, strike, vol);
            data.add_point(point, forward, 1.0);
        }

        // Initial SABR params: alpha, beta, rho, nu
        let initial = vec![0.05, 0.5, -0.2, 0.3];

        let result = calibrator.calibrate(&data, initial, &CalibrationConfig::fast());

        // Just check that it runs and produces reasonable params
        assert!(result.params[0] > 0.0); // alpha > 0
        assert!(result.params[1] >= 0.0 && result.params[1] <= 1.0); // beta in [0, 1]
        assert!(result.params[2] > -1.0 && result.params[2] < 1.0); // rho in (-1, 1)
        assert!(result.params[3] > 0.0); // nu > 0
    }
}
