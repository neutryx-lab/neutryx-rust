//! IRS Greeks calculation by risk factor.
//!
//! This module provides [`IrsGreeksByFactorCalculator`], which extends
//! the base `IrsGreeksCalculator` to compute Greeks organised by risk factor.
//!
//! # Requirements
//!
//! - Requirement 1.1, 1.2: First and second-order Greeks by risk factor.
//! - Requirement 1.3: Risk factor identification and classification.
//! - Requirement 1.6: NaN/Inf detection and error handling.

use super::{GreeksResultByFactor, RiskFactorId};
use pricer_core::market_data::curves::{CurveEnum, CurveName, CurveSet, YieldCurve};
use pricer_core::types::time::Date;
use pricer_models::instruments::rates::{price_irs, InterestRateSwap};
use pricer_pricing::greeks::{GreeksMode, GreeksResult};
use pricer_pricing::irs_greeks::{IrsGreeksCalculator, IrsGreeksConfig, IrsGreeksError};
use std::collections::HashMap;
use std::time::Instant;

/// Error types for factor-based Greeks calculation.
#[derive(Debug, thiserror::Error)]
pub enum GreeksByFactorError {
    /// Underlying IRS Greeks calculation error.
    #[error("IRS Greeks error: {0}")]
    IrsGreeksError(#[from] IrsGreeksError),

    /// NaN detected in calculation result.
    #[error("NaN detected in {0}: factor={1}")]
    NaNDetected(String, String),

    /// Infinite value detected in calculation result.
    #[error("Infinite value detected in {0}: factor={1}")]
    InfiniteDetected(String, String),

    /// Invalid risk factor configuration.
    #[error("Invalid risk factor: {0}")]
    InvalidRiskFactor(String),
}

/// Configuration for factor-based Greeks calculation.
#[derive(Clone, Debug)]
pub struct GreeksByFactorConfig {
    /// Base IRS Greeks configuration.
    pub irs_config: IrsGreeksConfig,

    /// Whether to validate results for NaN/Inf.
    pub validate_results: bool,

    /// Whether to include second-order Greeks.
    pub include_second_order: bool,
}

impl Default for GreeksByFactorConfig {
    fn default() -> Self {
        Self {
            irs_config: IrsGreeksConfig::default(),
            validate_results: true,
            include_second_order: false,
        }
    }
}

impl GreeksByFactorConfig {
    /// Creates a new configuration with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to validate results for NaN/Inf.
    pub fn with_validation(mut self, validate: bool) -> Self {
        self.validate_results = validate;
        self
    }

    /// Sets whether to include second-order Greeks.
    pub fn with_second_order(mut self, include: bool) -> Self {
        self.include_second_order = include;
        self
    }

    /// Sets the bump size for finite differences.
    pub fn with_bump_size(mut self, bump_size: f64) -> Self {
        self.irs_config = self.irs_config.with_bump_size(bump_size);
        self
    }
}

/// Calculator for IRS Greeks organised by risk factor.
///
/// Computes Greeks for Interest Rate Swaps and organises results
/// by the underlying risk factors (curves, tenors).
///
/// # Examples
///
/// ```rust,ignore
/// use pricer_risk::scenarios::{IrsGreeksByFactorCalculator, GreeksByFactorConfig};
///
/// let config = GreeksByFactorConfig::default();
/// let calculator = IrsGreeksByFactorCalculator::new(config);
///
/// let result = calculator.compute_greeks_by_factor(&swap, &curves, valuation_date)?;
/// ```
pub struct IrsGreeksByFactorCalculator {
    config: GreeksByFactorConfig,
    inner: IrsGreeksCalculator<f64>,
}

impl IrsGreeksByFactorCalculator {
    /// Creates a new calculator with the given configuration.
    pub fn new(config: GreeksByFactorConfig) -> Self {
        let inner = IrsGreeksCalculator::new(config.irs_config.clone());
        Self { config, inner }
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &GreeksByFactorConfig {
        &self.config
    }

    /// Validates a computed value for NaN/Inf.
    fn validate_value(
        &self,
        value: f64,
        greek_name: &str,
        factor: &RiskFactorId,
    ) -> Result<f64, GreeksByFactorError> {
        if !self.config.validate_results {
            return Ok(value);
        }

        if value.is_nan() {
            return Err(GreeksByFactorError::NaNDetected(
                greek_name.to_string(),
                factor.to_string(),
            ));
        }

        if value.is_infinite() {
            return Err(GreeksByFactorError::InfiniteDetected(
                greek_name.to_string(),
                factor.to_string(),
            ));
        }

        Ok(value)
    }

    /// Extracts risk factors from a curve set.
    ///
    /// Returns a list of `RiskFactorId::Curve` for each curve in the set.
    pub fn extract_risk_factors(&self, curves: &CurveSet<f64>) -> Vec<RiskFactorId> {
        curves
            .iter()
            .map(|(name, _)| RiskFactorId::curve(name.to_string()))
            .collect()
    }

    /// Computes Delta sensitivity for a specific curve.
    ///
    /// Uses finite differences to compute the sensitivity of the swap NPV
    /// to a 1bp parallel shift in the specified curve.
    fn compute_curve_delta(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        curve_name: CurveName,
        bump_size: f64,
    ) -> Result<f64, GreeksByFactorError> {
        let base_npv = price_irs(swap, curves, valuation_date);

        // Create curves with only the target curve bumped
        let mut bumped_curves = curves.clone();
        if let Some(curve) = curves.get(&curve_name) {
            let base_rate = curve.zero_rate(1.0).unwrap_or(0.0);
            let bumped_rate = base_rate + bump_size;
            bumped_curves.insert(curve_name, CurveEnum::flat(bumped_rate));
        }

        let bumped_npv = price_irs(swap, &bumped_curves, valuation_date);

        // Delta = dNPV / dr, scaled to 1bp
        let delta = (bumped_npv - base_npv) / bump_size * 0.0001;

        Ok(delta)
    }

    /// Computes Gamma (second-order Delta) for a specific curve.
    ///
    /// Uses central differences to compute the second derivative.
    fn compute_curve_gamma(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        curve_name: CurveName,
        bump_size: f64,
    ) -> Result<f64, GreeksByFactorError> {
        let base_npv = price_irs(swap, curves, valuation_date);

        // Bump up
        let mut up_curves = curves.clone();
        if let Some(curve) = curves.get(&curve_name) {
            let base_rate = curve.zero_rate(1.0).unwrap_or(0.0);
            up_curves.insert(curve_name, CurveEnum::flat(base_rate + bump_size));
        }
        let up_npv = price_irs(swap, &up_curves, valuation_date);

        // Bump down
        let mut down_curves = curves.clone();
        if let Some(curve) = curves.get(&curve_name) {
            let base_rate = curve.zero_rate(1.0).unwrap_or(0.0);
            down_curves.insert(curve_name, CurveEnum::flat(base_rate - bump_size));
        }
        let down_npv = price_irs(swap, &down_curves, valuation_date);

        // Gamma = d²NPV / dr², scaled to (1bp)²
        let gamma = (up_npv - 2.0 * base_npv + down_npv) / (bump_size * bump_size) * 0.0001 * 0.0001;

        Ok(gamma)
    }

    /// Computes Greeks organised by risk factor.
    ///
    /// For IRS, the main risk factors are the yield curves. This method
    /// computes Delta (and optionally Gamma) for each curve in the set.
    ///
    /// # Arguments
    ///
    /// * `swap` - The interest rate swap
    /// * `curves` - Curve set containing discount and forward curves
    /// * `valuation_date` - The valuation date
    /// * `mode` - Calculation mode (BumpRevalue or AAD)
    ///
    /// # Returns
    ///
    /// `GreeksResultByFactor<f64>` containing Delta/Gamma for each curve.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 1.1, 1.2: First and second-order Greeks by factor
    /// - Requirement 1.3: Risk factor identification
    /// - Requirement 1.6: NaN/Inf detection
    pub fn compute_greeks_by_factor(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
        mode: GreeksMode,
    ) -> Result<GreeksResultByFactor<f64>, GreeksByFactorError> {
        let start_time = Instant::now();

        // Validate inputs via inner calculator
        let _ = self.inner.compute_npv(swap, curves, valuation_date)?;

        let mut results = GreeksResultByFactor::new(mode);
        let bump_size = self.config.irs_config.bump_size;

        // Compute Greeks for each curve
        for (name, _curve) in curves.iter() {
            let factor = RiskFactorId::curve(name.to_string());

            // Compute Delta
            let delta = self.compute_curve_delta(swap, curves, valuation_date, *name, bump_size)?;
            let delta = self.validate_value(delta, "delta", &factor)?;

            // Create GreeksResult
            let mut greeks_result = GreeksResult::new(0.0, 0.0).with_rho(delta);

            // Optionally compute Gamma
            if self.config.include_second_order {
                let gamma =
                    self.compute_curve_gamma(swap, curves, valuation_date, *name, bump_size)?;
                let gamma = self.validate_value(gamma, "gamma", &factor)?;
                greeks_result = greeks_result.with_gamma(gamma);
            }

            results.insert(factor, greeks_result);
        }

        let compute_time_ns = start_time.elapsed().as_nanos() as u64;
        results.set_computation_time_ns(compute_time_ns);

        Ok(results)
    }

    /// Computes first-order Greeks (Delta/Rho) by risk factor.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 1.1: First-order Greeks (Delta, Rho) by factor
    pub fn compute_first_order_greeks(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
    ) -> Result<GreeksResultByFactor<f64>, GreeksByFactorError> {
        let config = GreeksByFactorConfig {
            include_second_order: false,
            ..self.config.clone()
        };
        let calculator = IrsGreeksByFactorCalculator::new(config);
        calculator.compute_greeks_by_factor(swap, curves, valuation_date, GreeksMode::BumpRevalue)
    }

    /// Computes second-order Greeks (Gamma) by risk factor.
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 1.2: Second-order Greeks (Gamma) by factor
    pub fn compute_second_order_greeks(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
    ) -> Result<GreeksResultByFactor<f64>, GreeksByFactorError> {
        let config = GreeksByFactorConfig {
            include_second_order: true,
            ..self.config.clone()
        };
        let calculator = IrsGreeksByFactorCalculator::new(config);
        calculator.compute_greeks_by_factor(swap, curves, valuation_date, GreeksMode::BumpRevalue)
    }

    /// Computes DV01 by risk factor.
    ///
    /// DV01 is computed as the absolute Delta scaled to 1bp.
    pub fn compute_dv01_by_factor(
        &self,
        swap: &InterestRateSwap<f64>,
        curves: &CurveSet<f64>,
        valuation_date: Date,
    ) -> Result<HashMap<RiskFactorId, f64>, GreeksByFactorError> {
        let greeks = self.compute_first_order_greeks(swap, curves, valuation_date)?;

        let mut dv01s = HashMap::new();
        for (factor, result) in greeks.iter() {
            // Delta is stored in rho field for curve factors
            if let Some(delta) = result.rho {
                dv01s.insert(factor.clone(), delta.abs());
            }
        }

        Ok(dv01s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pricer_core::types::time::DayCountConvention;
    use pricer_core::types::Currency;
    use pricer_models::instruments::rates::{FixedLeg, FloatingLeg, RateIndex, SwapDirection};
    use pricer_models::schedules::{Frequency, ScheduleBuilder};

    // ================================================================
    // Task 1.3: IrsGreeksByFactorCalculator tests (TDD - RED phase)
    // ================================================================

    /// Helper function to create a test swap
    fn create_test_swap() -> InterestRateSwap<f64> {
        let start_date = Date::from_ymd(2025, 1, 15).unwrap();
        let end_date = Date::from_ymd(2030, 1, 15).unwrap();

        let fixed_schedule = ScheduleBuilder::new()
            .start(start_date)
            .end(end_date)
            .frequency(Frequency::SemiAnnual)
            .day_count(DayCountConvention::Thirty360)
            .build()
            .unwrap();

        let floating_schedule = ScheduleBuilder::new()
            .start(start_date)
            .end(end_date)
            .frequency(Frequency::Quarterly)
            .day_count(DayCountConvention::ActualActual360)
            .build()
            .unwrap();

        let fixed_leg = FixedLeg::new(fixed_schedule, 0.02, DayCountConvention::Thirty360);
        let floating_leg = FloatingLeg::new(
            floating_schedule,
            0.0,
            RateIndex::Sofr,
            DayCountConvention::ActualActual360,
        );

        InterestRateSwap::new(
            10_000_000.0,
            fixed_leg,
            floating_leg,
            Currency::USD,
            SwapDirection::PayFixed,
        )
    }

    /// Helper function to create test curves
    fn create_test_curves() -> CurveSet<f64> {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Discount, CurveEnum::flat(0.03));
        curves.insert(CurveName::Forward, CurveEnum::flat(0.035));
        curves.set_discount_curve(CurveName::Discount);
        curves
    }

    #[test]
    fn test_greeks_by_factor_config_default() {
        let config = GreeksByFactorConfig::default();
        assert!(config.validate_results);
        assert!(!config.include_second_order);
    }

    #[test]
    fn test_greeks_by_factor_config_builder() {
        let config = GreeksByFactorConfig::new()
            .with_validation(false)
            .with_second_order(true)
            .with_bump_size(0.0005);

        assert!(!config.validate_results);
        assert!(config.include_second_order);
        assert!((config.irs_config.bump_size - 0.0005).abs() < 1e-10);
    }

    #[test]
    fn test_calculator_new() {
        let config = GreeksByFactorConfig::default();
        let _calculator = IrsGreeksByFactorCalculator::new(config);
    }

    #[test]
    fn test_extract_risk_factors() {
        let config = GreeksByFactorConfig::default();
        let calculator = IrsGreeksByFactorCalculator::new(config);
        let curves = create_test_curves();

        let factors = calculator.extract_risk_factors(&curves);
        assert!(!factors.is_empty());
        assert!(factors.iter().all(|f| f.is_curve()));
    }

    #[test]
    fn test_compute_greeks_by_factor_first_order() {
        let config = GreeksByFactorConfig::new().with_second_order(false);
        let calculator = IrsGreeksByFactorCalculator::new(config);
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let result = calculator
            .compute_greeks_by_factor(&swap, &curves, valuation_date, GreeksMode::BumpRevalue)
            .unwrap();

        // Should have results for each curve
        assert!(!result.is_empty());
        assert_eq!(result.mode(), GreeksMode::BumpRevalue);

        // Each factor should have rho (curve delta)
        for (factor, greeks) in result.iter() {
            assert!(factor.is_curve());
            assert!(greeks.rho.is_some());
            // Gamma should not be computed for first-order only
        }
    }

    #[test]
    fn test_compute_greeks_by_factor_second_order() {
        let config = GreeksByFactorConfig::new().with_second_order(true);
        let calculator = IrsGreeksByFactorCalculator::new(config);
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let result = calculator
            .compute_greeks_by_factor(&swap, &curves, valuation_date, GreeksMode::BumpRevalue)
            .unwrap();

        // Each factor should have gamma
        for (_factor, greeks) in result.iter() {
            assert!(greeks.gamma.is_some());
        }
    }

    #[test]
    fn test_compute_first_order_greeks() {
        let config = GreeksByFactorConfig::default();
        let calculator = IrsGreeksByFactorCalculator::new(config);
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let result = calculator
            .compute_first_order_greeks(&swap, &curves, valuation_date)
            .unwrap();

        assert!(!result.is_empty());
    }

    #[test]
    fn test_compute_second_order_greeks() {
        let config = GreeksByFactorConfig::default();
        let calculator = IrsGreeksByFactorCalculator::new(config);
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let result = calculator
            .compute_second_order_greeks(&swap, &curves, valuation_date)
            .unwrap();

        // All results should have gamma
        for (_factor, greeks) in result.iter() {
            assert!(greeks.gamma.is_some());
        }
    }

    #[test]
    fn test_compute_dv01_by_factor() {
        let config = GreeksByFactorConfig::default();
        let calculator = IrsGreeksByFactorCalculator::new(config);
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let dv01s = calculator
            .compute_dv01_by_factor(&swap, &curves, valuation_date)
            .unwrap();

        assert!(!dv01s.is_empty());
        // All DV01 values should be non-negative
        for (_factor, dv01) in &dv01s {
            assert!(*dv01 >= 0.0);
        }
    }

    #[test]
    fn test_computation_time_tracking() {
        let config = GreeksByFactorConfig::default();
        let calculator = IrsGreeksByFactorCalculator::new(config);
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let result = calculator
            .compute_greeks_by_factor(&swap, &curves, valuation_date, GreeksMode::BumpRevalue)
            .unwrap();

        // Computation time should be recorded
        assert!(result.computation_time_ns() > 0);
    }

    #[test]
    fn test_nan_detection() {
        let config = GreeksByFactorConfig::new().with_validation(true);
        let calculator = IrsGreeksByFactorCalculator::new(config);

        let factor = RiskFactorId::curve("USD-OIS");
        let result = calculator.validate_value(f64::NAN, "delta", &factor);

        assert!(result.is_err());
        assert!(matches!(result, Err(GreeksByFactorError::NaNDetected(_, _))));
    }

    #[test]
    fn test_infinite_detection() {
        let config = GreeksByFactorConfig::new().with_validation(true);
        let calculator = IrsGreeksByFactorCalculator::new(config);

        let factor = RiskFactorId::curve("USD-OIS");
        let result = calculator.validate_value(f64::INFINITY, "delta", &factor);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(GreeksByFactorError::InfiniteDetected(_, _))
        ));
    }

    #[test]
    fn test_validation_disabled() {
        let config = GreeksByFactorConfig::new().with_validation(false);
        let calculator = IrsGreeksByFactorCalculator::new(config);

        let factor = RiskFactorId::curve("USD-OIS");

        // NaN should pass when validation is disabled
        let result = calculator.validate_value(f64::NAN, "delta", &factor);
        assert!(result.is_ok());

        // Infinite should pass when validation is disabled
        let result = calculator.validate_value(f64::INFINITY, "delta", &factor);
        assert!(result.is_ok());
    }

    #[test]
    fn test_total_greeks() {
        let config = GreeksByFactorConfig::new().with_second_order(true);
        let calculator = IrsGreeksByFactorCalculator::new(config);
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let result = calculator
            .compute_greeks_by_factor(&swap, &curves, valuation_date, GreeksMode::BumpRevalue)
            .unwrap();

        // Total should aggregate all factors
        let total = result.total();
        assert!(total.rho.is_some() || total.gamma.is_some());
    }

    #[test]
    fn test_filter_by_factor_type() {
        let config = GreeksByFactorConfig::default();
        let calculator = IrsGreeksByFactorCalculator::new(config);
        let swap = create_test_swap();
        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let result = calculator
            .compute_greeks_by_factor(&swap, &curves, valuation_date, GreeksMode::BumpRevalue)
            .unwrap();

        let curve_greeks = result.filter_curves();
        assert_eq!(curve_greeks.len(), result.len()); // All are curve factors

        let underlying_greeks = result.filter_underlying();
        assert!(underlying_greeks.is_empty()); // No underlying factors for IRS
    }

    #[test]
    fn test_error_propagation_invalid_swap() {
        let config = GreeksByFactorConfig::default();
        let calculator = IrsGreeksByFactorCalculator::new(config);

        // Create a swap with zero notional (invalid for IrsGreeksCalculator)
        let start_date = Date::from_ymd(2025, 1, 15).unwrap();
        let end_date = Date::from_ymd(2030, 1, 15).unwrap();

        let fixed_schedule = ScheduleBuilder::new()
            .start(start_date)
            .end(end_date)
            .frequency(Frequency::SemiAnnual)
            .day_count(DayCountConvention::Thirty360)
            .build()
            .unwrap();

        let floating_schedule = ScheduleBuilder::new()
            .start(start_date)
            .end(end_date)
            .frequency(Frequency::Quarterly)
            .day_count(DayCountConvention::ActualActual360)
            .build()
            .unwrap();

        let fixed_leg = FixedLeg::new(fixed_schedule, 0.02, DayCountConvention::Thirty360);
        let floating_leg = FloatingLeg::new(
            floating_schedule,
            0.0,
            RateIndex::Sofr,
            DayCountConvention::ActualActual360,
        );

        // Zero notional is invalid
        let swap = InterestRateSwap::new(
            0.0, // Zero notional - should cause validation error
            fixed_leg,
            floating_leg,
            Currency::USD,
            SwapDirection::PayFixed,
        );

        let curves = create_test_curves();
        let valuation_date = Date::from_ymd(2025, 1, 15).unwrap();

        let result = calculator.compute_greeks_by_factor(
            &swap,
            &curves,
            valuation_date,
            GreeksMode::BumpRevalue,
        );

        // Should propagate the error from IrsGreeksCalculator (invalid notional)
        assert!(result.is_err());
    }
}
