//! FX volatility surface calibration.
//!
//! This module provides `FxVolBuilder` for calibrating FX volatility surfaces
//! using slice-wise SABR calibration.

use std::collections::BTreeMap;
use num_traits::Float;

use super::{
    CalibrationError, OrderedFloat, SabrParams, SabrSliceCalibrator,
    SliceCalibrationConfig, SliceCalibrator, VolQuote,
};

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
        }
    }

    /// Creates a builder with custom configuration.
    pub fn with_config(config: SliceCalibrationConfig<T>) -> Self {
        Self {
            slices: BTreeMap::new(),
            config,
            calibrator: SabrSliceCalibrator::new(),
        }
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
            .push(VolQuote::new(strike, volatility, forward));
        self
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

        for (exp, quotes) in &self.slices {
            let calibrated = self.calibrator.calibrate_slice(quotes, &self.config)?;
            params.insert(*exp, calibrated);
            expiries.push(exp.0);
        }

        // Sort expiries
        expiries.sort_by(|a, b| a.partial_cmp(b).unwrap());

        Ok(FxVolResult { expiries, params })
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

        // 3M expiry
        builder.add_quote(0.25, 1.10, 0.08, 1.10);
        builder.add_quote(0.25, 1.05, 0.085, 1.10);

        // 1Y expiry
        builder.add_quote(1.0, 1.10, 0.10, 1.10);
        builder.add_quote(1.0, 1.05, 0.105, 1.10);

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

        builder.add_quote(0.5, 1.10, 0.09, 1.10);

        let result = builder.calibrate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_fxvol_result_get() {
        let mut builder: FxVolBuilder<f64> = FxVolBuilder::new();
        builder.add_quote(0.25, 1.10, 0.08, 1.10);
        builder.add_quote(0.5, 1.10, 0.09, 1.10);

        let surface = builder.calibrate().unwrap();

        assert!(surface.get(0.25).is_some());
        assert!(surface.get(0.5).is_some());
        assert!(surface.get(1.0).is_none());
    }
}
