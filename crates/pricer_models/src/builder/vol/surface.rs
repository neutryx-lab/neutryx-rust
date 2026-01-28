//! FX volatility surface calibration.
//!
//! This module provides `FxVolBuilder` for calibrating FX volatility surfaces
//! using slice-wise SABR calibration.
//!
//! # Example with DeltaVolSlice
//!
//! ```ignore
//! use pricer_models::builder::vol::{FxVolBuilder, DeltaVolSlice};
//! use pricer_models::market::{FxCurveEnum, FxCurve};
//! use infra_master::trade::instrument_def::{FxVolConvention, DeltaType};
//!
//! // Create builder with FX curve and convention
//! let fx_curve = FxCurveEnum::irp_flat(1.10, 0.03, 0.01, pair);
//! let convention = FxVolConvention::eurusd();
//!
//! let mut builder = FxVolBuilder::new()
//!     .with_fx_curve(fx_curve)
//!     .with_convention(convention);
//!
//! // Add delta-quoted slice (RR/BF format)
//! let slice = DeltaVolSlice::new_with_25d(0.10, 0.01, 0.005, 0.25, 1.10);
//! builder.add_delta_vol_slice(slice);
//!
//! let result = builder.calibrate()?;
//! ```

use std::collections::BTreeMap;
use num_traits::Float;

use infra_master::trade::instrument_def::FxVolConvention;
use pricer_core::math::formulas::fx_delta::delta_to_strike;

use super::{
    CalibrationError, DeltaVolSlice, OrderedFloat, SabrParams, SabrSliceCalibrator,
    SliceCalibrationConfig, SliceCalibrationDiagnostics, SliceCalibrator, VolQuote,
};
use crate::market::{FxCurve, FxCurveEnum};

// =============================================================================
// FxVolBuilder
// =============================================================================

/// Builder for FX volatility surfaces.
///
/// Calibrates SABR parameters for each expiry slice independently,
/// then aggregates into a complete parameter surface.
///
/// # Example
///
/// ```ignore
/// use pricer_models::builder::vol::FxVolBuilder;
///
/// let mut builder = FxVolBuilder::new();
/// builder.add_quote(0.25, 1.10, 0.08, 1.10);  // 3M expiry
/// builder.add_quote(0.25, 1.05, 0.085, 1.10);
/// builder.add_quote(1.0, 1.10, 0.10, 1.10);   // 1Y expiry
///
/// let surface = builder.calibrate()?;
/// ```
#[derive(Debug, Clone)]
pub struct FxVolBuilder<T: Float> {
    /// Quotes organised by expiry
    slices: BTreeMap<OrderedFloat<T>, Vec<VolQuote<T>>>,
    /// Calibration configuration
    config: SliceCalibrationConfig<T>,
    /// Slice calibrator
    calibrator: SabrSliceCalibrator<T>,
    /// FX forward curve (optional, for delta-to-strike conversion)
    fx_curve: Option<FxCurveEnum<T>>,
    /// Market convention (optional, for delta-to-strike conversion)
    convention: Option<FxVolConvention>,
}

impl<T: Float> Default for FxVolBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Float> FxVolBuilder<T> {
    /// Creates a new FxVol builder with default configuration.
    pub fn new() -> Self {
        Self {
            slices: BTreeMap::new(),
            config: SliceCalibrationConfig::fx(),
            calibrator: SabrSliceCalibrator::new(),
            fx_curve: None,
            convention: None,
        }
    }

    /// Creates a builder with custom configuration.
    pub fn with_config(config: SliceCalibrationConfig<T>) -> Self {
        Self {
            slices: BTreeMap::new(),
            config,
            calibrator: SabrSliceCalibrator::new(),
            fx_curve: None,
            convention: None,
        }
    }

    /// Sets the FX forward curve for delta-to-strike conversion.
    ///
    /// Required when using `add_delta_vol_slice`.
    pub fn with_fx_curve(mut self, fx_curve: FxCurveEnum<T>) -> Self {
        self.fx_curve = Some(fx_curve);
        self
    }

    /// Sets the market convention for delta-to-strike conversion.
    ///
    /// Required when using `add_delta_vol_slice`.
    pub fn with_convention(mut self, convention: FxVolConvention) -> Self {
        self.convention = Some(convention);
        self
    }

    /// Adds quotes for a single expiry slice.
    pub fn add_slice(&mut self, expiry: T, quotes: Vec<VolQuote<T>>) -> &mut Self {
        let key = OrderedFloat(expiry);
        self.slices.entry(key).or_default().extend(quotes);
        self
    }

    /// Adds a single quote.
    pub fn add_quote(&mut self, expiry: T, strike: T, volatility: T, forward: T) -> &mut Self {
        let key = OrderedFloat(expiry);
        self.slices
            .entry(key)
            .or_default()
            .push(VolQuote::new(strike, volatility, forward, expiry));
        self
    }

    /// Adds a delta-quoted volatility slice (ATM/RR/BF format).
    ///
    /// Converts delta-based quotes to strike-based quotes using the configured
    /// FX curve and convention. Requires `with_fx_curve` and `with_convention`
    /// to be called first.
    ///
    /// # Arguments
    ///
    /// * `slice` - Delta-quoted volatility slice
    ///
    /// # Returns
    ///
    /// * `Ok(&mut Self)` - On success
    /// * `Err(CalibrationError)` - If FX curve or convention not set
    pub fn add_delta_vol_slice(
        &mut self,
        slice: DeltaVolSlice<T>,
    ) -> Result<&mut Self, CalibrationError> {
        let fx_curve = self.fx_curve.as_ref().ok_or_else(|| CalibrationError::MissingInput {
            field: "fx_curve".to_string(),
        })?;
        let convention = self.convention.ok_or_else(|| CalibrationError::MissingInput {
            field: "convention".to_string(),
        })?;

        let spot = fx_curve.spot();
        let expiry = slice.expiry;
        let forward = slice.forward;

        // Get rates from FX curve (derive from forward/spot ratio)
        // F = S × exp((rd - rf) × T) => rd - rf = ln(F/S) / T
        let rate_diff = if expiry > T::zero() {
            (forward / spot).ln() / expiry
        } else {
            T::zero()
        };

        // For simplicity, assume symmetric rates around the diff
        // In production, would get actual rates from the curves
        let domestic_rate = rate_diff;
        let foreign_rate = T::zero();

        let delta_type = convention.delta_type;

        // Convert delta vols to strike quotes
        let delta_vols = slice.to_delta_vols();
        let key = OrderedFloat(expiry);

        for dv in delta_vols {
            // Convert delta (as decimal) to signed delta (positive for call, negative for put)
            let signed_delta = if dv.is_call { dv.delta } else { -dv.delta };

            // Convert delta to strike
            let strike = delta_to_strike(
                signed_delta,
                spot,
                domestic_rate,
                foreign_rate,
                expiry,
                dv.volatility,
                delta_type,
            )
            .map_err(|e| CalibrationError::SolverError {
                message: format!("delta_to_strike failed: {}", e),
            })?;

            self.slices
                .entry(key)
                .or_default()
                .push(VolQuote::new(strike, dv.volatility, forward, expiry));
        }

        Ok(self)
    }

    /// Calibrates all slices and returns the parameter surface.
    pub fn calibrate(&self) -> Result<FxVolResult<T>, CalibrationError> {
        if self.slices.is_empty() {
            return Err(CalibrationError::InsufficientData {
                required: 1,
                provided: 0,
            });
        }

        let mut expiries = Vec::new();
        let mut params = BTreeMap::new();
        let mut diagnostics = BTreeMap::new();

        for (exp, quotes) in &self.slices {
            let result = self.calibrator.calibrate_slice(quotes, &self.config)?;
            params.insert(*exp, result.params);
            diagnostics.insert(*exp, result.diagnostics);
            expiries.push(exp.0);
        }

        // Sort expiries
        expiries.sort_by(|a, b| a.partial_cmp(b).unwrap());

        Ok(FxVolResult { expiries, params, diagnostics })
    }
}

// =============================================================================
// FxVolResult
// =============================================================================

/// Result of FX vol surface calibration.
#[derive(Debug, Clone)]
pub struct FxVolResult<T: Float> {
    /// Expiry grid points
    pub expiries: Vec<T>,
    /// Calibrated SABR parameters indexed by expiry
    pub params: BTreeMap<OrderedFloat<T>, SabrParams<T>>,
    /// Calibration diagnostics for each slice
    pub diagnostics: BTreeMap<OrderedFloat<T>, SliceCalibrationDiagnostics>,
}

impl<T: Float> FxVolResult<T> {
    /// Gets parameters for a specific expiry.
    pub fn get(&self, expiry: T) -> Option<&SabrParams<T>> {
        self.params.get(&OrderedFloat(expiry))
    }

    /// Returns the number of calibrated slices.
    pub fn num_slices(&self) -> usize {
        self.params.len()
    }

    /// Gets all expiries.
    pub fn expiries(&self) -> &[T] {
        &self.expiries
    }

    /// Gets diagnostics for a specific expiry.
    pub fn get_diagnostics(&self, expiry: T) -> Option<&SliceCalibrationDiagnostics> {
        self.diagnostics.get(&OrderedFloat(expiry))
    }

    /// Returns true if all slices converged.
    pub fn all_converged(&self) -> bool {
        self.diagnostics.values().all(|d| d.converged)
    }

    /// Returns true if all slices have acceptable fit quality.
    pub fn all_acceptable(&self) -> bool {
        self.diagnostics.values().all(|d| d.is_acceptable())
    }

    /// Returns any warnings from calibration across all slices.
    pub fn warnings(&self) -> Vec<(T, &str)> {
        let mut result = Vec::new();
        for (exp, diag) in &self.diagnostics {
            for warning in &diag.warnings {
                result.push((exp.0, warning.as_str()));
            }
        }
        result
    }

    /// Returns the maximum RMSE across all slices.
    pub fn max_rmse(&self) -> f64 {
        self.diagnostics
            .values()
            .map(|d| d.rmse)
            .fold(0.0, f64::max)
    }

    /// Returns the total number of iterations across all slices.
    pub fn total_iterations(&self) -> usize {
        self.diagnostics.values().map(|d| d.iterations).sum()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fxvol_builder_empty() {
        let builder: FxVolBuilder<f64> = FxVolBuilder::new();
        let result = builder.calibrate();
        assert!(result.is_err());
    }

    #[test]
    fn test_fxvol_builder_single_slice() {
        let mut builder: FxVolBuilder<f64> = FxVolBuilder::new();
        builder.add_quote(0.25, 1.10, 0.08, 1.10);
        builder.add_quote(0.25, 1.05, 0.085, 1.10);
        builder.add_quote(0.25, 1.15, 0.082, 1.10);

        let result = builder.calibrate();
        assert!(result.is_ok());

        let surface = result.unwrap();
        assert_eq!(surface.num_slices(), 1);
        assert!(surface.get(0.25).is_some());
    }

    #[test]
    fn test_fxvol_builder_multiple_slices() {
        let mut builder: FxVolBuilder<f64> = FxVolBuilder::new();

        // 3M expiry (3 quotes minimum for SABR calibration)
        builder.add_quote(0.25, 1.10, 0.08, 1.10);
        builder.add_quote(0.25, 1.05, 0.085, 1.10);
        builder.add_quote(0.25, 1.15, 0.082, 1.10);

        // 1Y expiry (3 quotes)
        builder.add_quote(1.0, 1.10, 0.10, 1.10);
        builder.add_quote(1.0, 1.05, 0.105, 1.10);
        builder.add_quote(1.0, 1.15, 0.098, 1.10);

        let result = builder.calibrate();
        assert!(result.is_ok());

        let surface = result.unwrap();
        assert_eq!(surface.num_slices(), 2);
        assert_eq!(surface.expiries().len(), 2);
    }

    #[test]
    fn test_fxvol_builder_with_config() {
        let config = SliceCalibrationConfig::fx();
        let mut builder: FxVolBuilder<f64> = FxVolBuilder::with_config(config);

        // Need 3 quotes minimum for SABR calibration
        builder.add_quote(0.5, 1.10, 0.09, 1.10);
        builder.add_quote(0.5, 1.05, 0.095, 1.10);
        builder.add_quote(0.5, 1.15, 0.088, 1.10);

        let result = builder.calibrate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_fxvol_result_get() {
        let mut builder: FxVolBuilder<f64> = FxVolBuilder::new();
        // 0.25Y expiry (3 quotes)
        builder.add_quote(0.25, 1.10, 0.08, 1.10);
        builder.add_quote(0.25, 1.05, 0.085, 1.10);
        builder.add_quote(0.25, 1.15, 0.078, 1.10);
        // 0.5Y expiry (3 quotes)
        builder.add_quote(0.5, 1.10, 0.09, 1.10);
        builder.add_quote(0.5, 1.05, 0.095, 1.10);
        builder.add_quote(0.5, 1.15, 0.088, 1.10);

        let surface = builder.calibrate().unwrap();

        assert!(surface.get(0.25).is_some());
        assert!(surface.get(0.5).is_some());
        assert!(surface.get(1.0).is_none());
    }

    #[test]
    fn test_fxvol_builder_with_fx_curve_and_convention() {
        use infra_master::Currency;
        use infra_master::trade::instrument_def::CurrencyPair;

        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let fx_curve = FxCurveEnum::irp_flat(1.10, 0.03, 0.01, pair);
        let convention = FxVolConvention::eurusd();

        let builder: FxVolBuilder<f64> = FxVolBuilder::new()
            .with_fx_curve(fx_curve)
            .with_convention(convention);

        // Builder should be configured correctly
        assert!(builder.fx_curve.is_some());
        assert!(builder.convention.is_some());
    }

    #[test]
    fn test_fxvol_builder_add_delta_vol_slice_missing_fx_curve() {
        let slice: DeltaVolSlice<f64> = DeltaVolSlice::new_with_25d(0.10, 0.01, 0.005, 0.25, 1.10);

        let mut builder: FxVolBuilder<f64> = FxVolBuilder::new();
        let result = builder.add_delta_vol_slice(slice);

        // Should fail because fx_curve is not set
        assert!(result.is_err());
    }

    #[test]
    fn test_fxvol_builder_add_delta_vol_slice_success() {
        use infra_master::Currency;
        use infra_master::trade::instrument_def::CurrencyPair;

        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let fx_curve = FxCurveEnum::irp_flat(1.10, 0.03, 0.01, pair);
        let convention = FxVolConvention::eurusd();

        let mut builder = FxVolBuilder::new()
            .with_fx_curve(fx_curve)
            .with_convention(convention);

        // Add a delta vol slice with ATM, 25D RR, 25D BF
        let slice: DeltaVolSlice<f64> = DeltaVolSlice::new_with_25d(
            0.10,   // atm
            0.01,   // rr_25d
            0.005,  // bf_25d
            0.25,   // expiry
            1.10,   // forward
        );

        let result = builder.add_delta_vol_slice(slice);
        assert!(result.is_ok());

        // Should have 3 quotes (ATM + 25D call + 25D put)
        let quotes = builder.slices.values().next().unwrap();
        assert_eq!(quotes.len(), 3);
    }

    #[test]
    fn test_fxvol_builder_full_pipeline_with_delta_slices() {
        use infra_master::Currency;
        use infra_master::trade::instrument_def::CurrencyPair;

        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let fx_curve = FxCurveEnum::irp_flat(1.10, 0.03, 0.01, pair);
        let convention = FxVolConvention::eurusd();

        let mut builder = FxVolBuilder::new()
            .with_fx_curve(fx_curve)
            .with_convention(convention);

        // Add delta vol slice
        let slice: DeltaVolSlice<f64> = DeltaVolSlice::new_with_25d(
            0.10,   // atm (10%)
            0.01,   // rr_25d (1%)
            0.005,  // bf_25d (0.5%)
            0.25,   // expiry (3 months)
            1.10,   // forward
        );

        builder.add_delta_vol_slice(slice).unwrap();

        // Calibrate should succeed
        let result = builder.calibrate();
        assert!(result.is_ok());

        let surface = result.unwrap();
        assert_eq!(surface.num_slices(), 1);

        // Check SABR parameters are reasonable
        let params = surface.get(0.25).unwrap();
        assert!(params.alpha > 0.0);
        assert!(params.beta > 0.0 && params.beta <= 1.0);
        assert!(params.rho > -1.0 && params.rho < 1.0);
        assert!(params.nu > 0.0);
    }

    // =========================================================================
    // Diagnostics Tests
    // =========================================================================

    #[test]
    fn test_fxvol_result_diagnostics() {
        let mut builder: FxVolBuilder<f64> = FxVolBuilder::new();
        builder.add_quote(0.25, 1.10, 0.08, 1.10);
        builder.add_quote(0.25, 1.05, 0.085, 1.10);
        builder.add_quote(0.25, 1.15, 0.082, 1.10);

        let surface = builder.calibrate().unwrap();

        // Should have diagnostics for the slice
        let diag = surface.get_diagnostics(0.25);
        assert!(diag.is_some());

        let diag = diag.unwrap();
        assert!(diag.converged);
        assert!(diag.iterations > 0);
        assert_eq!(diag.num_quotes, 3);
    }

    #[test]
    fn test_fxvol_result_all_converged() {
        let mut builder: FxVolBuilder<f64> = FxVolBuilder::new();

        // 3M expiry
        builder.add_quote(0.25, 1.10, 0.08, 1.10);
        builder.add_quote(0.25, 1.05, 0.085, 1.10);
        builder.add_quote(0.25, 1.15, 0.082, 1.10);

        // 1Y expiry
        builder.add_quote(1.0, 1.10, 0.10, 1.10);
        builder.add_quote(1.0, 1.05, 0.105, 1.10);
        builder.add_quote(1.0, 1.15, 0.098, 1.10);

        let surface = builder.calibrate().unwrap();

        // All slices should have converged
        assert!(surface.all_converged());
    }

    #[test]
    fn test_fxvol_result_max_rmse() {
        let mut builder: FxVolBuilder<f64> = FxVolBuilder::new();
        builder.add_quote(0.25, 1.10, 0.08, 1.10);
        builder.add_quote(0.25, 1.05, 0.085, 1.10);
        builder.add_quote(0.25, 1.15, 0.082, 1.10);

        let surface = builder.calibrate().unwrap();

        // Max RMSE should be a small positive number
        let max_rmse = surface.max_rmse();
        assert!(max_rmse >= 0.0);
        assert!(max_rmse < 0.01); // Should be a good fit
    }

    #[test]
    fn test_fxvol_result_total_iterations() {
        let mut builder: FxVolBuilder<f64> = FxVolBuilder::new();

        // Two slices
        builder.add_quote(0.25, 1.10, 0.08, 1.10);
        builder.add_quote(0.25, 1.05, 0.085, 1.10);
        builder.add_quote(0.25, 1.15, 0.082, 1.10);

        builder.add_quote(1.0, 1.10, 0.10, 1.10);
        builder.add_quote(1.0, 1.05, 0.105, 1.10);
        builder.add_quote(1.0, 1.15, 0.098, 1.10);

        let surface = builder.calibrate().unwrap();

        // Total iterations should be positive
        let total = surface.total_iterations();
        assert!(total > 0);
    }

    #[test]
    fn test_fxvol_result_warnings() {
        let mut builder: FxVolBuilder<f64> = FxVolBuilder::new();
        builder.add_quote(0.25, 1.10, 0.08, 1.10);
        builder.add_quote(0.25, 1.05, 0.085, 1.10);
        builder.add_quote(0.25, 1.15, 0.082, 1.10);

        let surface = builder.calibrate().unwrap();

        // Should have no warnings for well-calibrated data
        let warnings = surface.warnings();
        // Converged calibrations should have no warnings
        if surface.all_converged() {
            assert!(warnings.is_empty());
        }
    }
}
