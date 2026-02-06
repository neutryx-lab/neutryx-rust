//! Swaption volatility cube calibration.
//!
//! This module provides `VolCubeBuilder` for calibrating swaption volatility
//! cubes using slice-wise SABR calibration.

use std::collections::BTreeMap;

use num_traits::Float;

use super::{
    CalibrationError, OrderedFloat, SabrParams, SabrSliceCalibrator, SliceCalibrationConfig,
    SliceCalibrationDiagnostics, SliceCalibrator, VolQuote,
};

// =============================================================================
// VolCubeBuilder
// =============================================================================

/// Builder for swaption volatility cubes.
///
/// Calibrates SABR parameters for each (expiry, tenor) slice independently,
/// then aggregates into a complete parameter cube.
///
/// # Example
///
/// ```ignore
/// use pricer_models::builder::vol::VolCubeBuilder;
///
/// let mut builder = VolCubeBuilder::new();
/// builder.add_quote(1.0, 5.0, 0.03, 0.2, 0.03);  // 1Y expiry, 5Y tenor
/// builder.add_quote(1.0, 5.0, 0.02, 0.22, 0.03);
/// builder.add_quote(1.0, 10.0, 0.03, 0.21, 0.03); // 1Y expiry, 10Y tenor
///
/// let cube = builder.calibrate()?;
/// ```
#[derive(Debug, Clone)]
pub struct VolCubeBuilder<T: Float> {
    /// Quotes organised by (expiry, tenor)
    slices: BTreeMap<(OrderedFloat<T>, OrderedFloat<T>), Vec<VolQuote<T>>>,
    /// Calibration configuration
    config: SliceCalibrationConfig<T>,
    /// Slice calibrator
    calibrator: SabrSliceCalibrator<T>,
}

impl<T: Float> Default for VolCubeBuilder<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Float> VolCubeBuilder<T> {
    /// Creates a new VolCube builder with default configuration.
    pub fn new() -> Self {
        Self {
            slices: BTreeMap::new(),
            config: SliceCalibrationConfig::rates(),
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

    /// Adds quotes for a single (expiry, tenor) slice.
    pub fn add_slice(&mut self, expiry: T, tenor: T, quotes: Vec<VolQuote<T>>) -> &mut Self {
        let key = (OrderedFloat(expiry), OrderedFloat(tenor));
        self.slices.entry(key).or_default().extend(quotes);
        self
    }

    /// Adds a single quote.
    pub fn add_quote(
        &mut self,
        expiry: T,
        tenor: T,
        strike: T,
        volatility: T,
        forward: T,
    ) -> &mut Self {
        let key = (OrderedFloat(expiry), OrderedFloat(tenor));
        self.slices
            .entry(key)
            .or_default()
            .push(VolQuote::new(strike, volatility, forward, expiry));
        self
    }

    /// Calibrates all slices and returns the parameter cube.
    pub fn calibrate(&self) -> Result<VolCubeResult<T>, CalibrationError> {
        if self.slices.is_empty() {
            return Err(CalibrationError::InsufficientData {
                required: 1,
                provided: 0,
            });
        }

        let mut expiries = Vec::new();
        let mut tenors = Vec::new();
        let mut params = BTreeMap::new();
        let mut diagnostics = BTreeMap::new();

        for ((exp, ten), quotes) in &self.slices {
            let result = self.calibrator.calibrate_slice(quotes, &self.config)?;
            params.insert((*exp, *ten), result.params);
            diagnostics.insert((*exp, *ten), result.diagnostics);

            if !expiries.contains(&exp.0) {
                expiries.push(exp.0);
            }
            if !tenors.contains(&ten.0) {
                tenors.push(ten.0);
            }
        }

        // Sort expiries and tenors
        expiries.sort_by(|a, b| a.partial_cmp(b).unwrap());
        tenors.sort_by(|a, b| a.partial_cmp(b).unwrap());

        Ok(VolCubeResult {
            expiries,
            tenors,
            params,
            diagnostics,
        })
    }
}

// =============================================================================
// VolCubeResult
// =============================================================================

/// Result of VolCube calibration.
#[derive(Debug, Clone)]
pub struct VolCubeResult<T: Float> {
    /// Expiry grid points
    pub expiries: Vec<T>,
    /// Tenor grid points
    pub tenors: Vec<T>,
    /// Calibrated SABR parameters indexed by (expiry, tenor)
    pub params: BTreeMap<(OrderedFloat<T>, OrderedFloat<T>), SabrParams<T>>,
    /// Calibration diagnostics indexed by (expiry, tenor)
    pub diagnostics: BTreeMap<(OrderedFloat<T>, OrderedFloat<T>), SliceCalibrationDiagnostics>,
}

impl<T: Float> VolCubeResult<T> {
    /// Gets parameters for a specific (expiry, tenor) point.
    pub fn get(&self, expiry: T, tenor: T) -> Option<&SabrParams<T>> {
        self.params
            .get(&(OrderedFloat(expiry), OrderedFloat(tenor)))
    }

    /// Returns the number of calibrated slices.
    pub fn num_slices(&self) -> usize { self.params.len() }

    /// Gets all expiries.
    pub fn expiries(&self) -> &[T] { &self.expiries }

    /// Gets all tenors.
    pub fn tenors(&self) -> &[T] { &self.tenors }

    /// Gets diagnostics for a specific (expiry, tenor) point.
    pub fn get_diagnostics(&self, expiry: T, tenor: T) -> Option<&SliceCalibrationDiagnostics> {
        self.diagnostics
            .get(&(OrderedFloat(expiry), OrderedFloat(tenor)))
    }

    /// Returns true if all slices converged.
    pub fn all_converged(&self) -> bool { self.diagnostics.values().all(|d| d.converged) }

    /// Returns true if all slices have acceptable fit quality.
    pub fn all_acceptable(&self) -> bool { self.diagnostics.values().all(|d| d.is_acceptable()) }

    /// Returns any warnings from calibration across all slices.
    pub fn warnings(&self) -> Vec<(T, T, &str)> {
        let mut result = Vec::new();
        for ((exp, ten), diag) in &self.diagnostics {
            for warning in &diag.warnings {
                result.push((exp.0, ten.0, warning.as_str()));
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
    fn test_volcube_builder_empty() {
        let builder: VolCubeBuilder<f64> = VolCubeBuilder::new();
        let result = builder.calibrate();
        assert!(result.is_err());
    }

    #[test]
    fn test_volcube_builder_single_slice() {
        let mut builder: VolCubeBuilder<f64> = VolCubeBuilder::new();
        builder.add_quote(1.0, 5.0, 0.03, 0.2, 0.03);
        builder.add_quote(1.0, 5.0, 0.02, 0.22, 0.03);
        builder.add_quote(1.0, 5.0, 0.04, 0.21, 0.03);

        let result = builder.calibrate();
        assert!(result.is_ok());

        let cube = result.unwrap();
        assert_eq!(cube.num_slices(), 1);
        assert!(cube.get(1.0, 5.0).is_some());
    }

    #[test]
    fn test_volcube_builder_multiple_slices() {
        let mut builder: VolCubeBuilder<f64> = VolCubeBuilder::new();

        // 1Y x 5Y (3 quotes minimum for SABR calibration)
        builder.add_quote(1.0, 5.0, 0.03, 0.2, 0.03);
        builder.add_quote(1.0, 5.0, 0.02, 0.22, 0.03);
        builder.add_quote(1.0, 5.0, 0.04, 0.21, 0.03);

        // 1Y x 10Y (3 quotes)
        builder.add_quote(1.0, 10.0, 0.03, 0.19, 0.03);
        builder.add_quote(1.0, 10.0, 0.02, 0.21, 0.03);
        builder.add_quote(1.0, 10.0, 0.04, 0.20, 0.03);

        // 5Y x 5Y (3 quotes)
        builder.add_quote(5.0, 5.0, 0.03, 0.18, 0.03);
        builder.add_quote(5.0, 5.0, 0.02, 0.20, 0.03);
        builder.add_quote(5.0, 5.0, 0.04, 0.19, 0.03);

        let result = builder.calibrate();
        assert!(result.is_ok());

        let cube = result.unwrap();
        assert_eq!(cube.num_slices(), 3);
        assert_eq!(cube.expiries().len(), 2); // 1Y, 5Y
        assert_eq!(cube.tenors().len(), 2); // 5Y, 10Y
    }

    #[test]
    fn test_volcube_builder_with_config() {
        let config = SliceCalibrationConfig::rates();
        let mut builder: VolCubeBuilder<f64> = VolCubeBuilder::with_config(config);

        // Need 3 quotes minimum for SABR calibration
        builder.add_quote(1.0, 5.0, 0.03, 0.2, 0.03);
        builder.add_quote(1.0, 5.0, 0.02, 0.22, 0.03);
        builder.add_quote(1.0, 5.0, 0.04, 0.21, 0.03);

        let result = builder.calibrate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_volcube_result_get() {
        let mut builder: VolCubeBuilder<f64> = VolCubeBuilder::new();
        // 1Y x 5Y (3 quotes)
        builder.add_quote(1.0, 5.0, 0.03, 0.2, 0.03);
        builder.add_quote(1.0, 5.0, 0.02, 0.22, 0.03);
        builder.add_quote(1.0, 5.0, 0.04, 0.21, 0.03);
        // 2Y x 10Y (3 quotes)
        builder.add_quote(2.0, 10.0, 0.03, 0.19, 0.03);
        builder.add_quote(2.0, 10.0, 0.02, 0.21, 0.03);
        builder.add_quote(2.0, 10.0, 0.04, 0.20, 0.03);

        let cube = builder.calibrate().unwrap();

        assert!(cube.get(1.0, 5.0).is_some());
        assert!(cube.get(2.0, 10.0).is_some());
        assert!(cube.get(1.0, 10.0).is_none());
    }

    #[test]
    fn test_volcube_add_slice() {
        let mut builder: VolCubeBuilder<f64> = VolCubeBuilder::new();

        // Need 3 quotes minimum for SABR calibration
        let quotes = vec![
            VolQuote::new(0.03, 0.2, 0.03, 1.0),
            VolQuote::new(0.02, 0.22, 0.03, 1.0),
            VolQuote::new(0.04, 0.21, 0.03, 1.0),
        ];

        builder.add_slice(1.0, 5.0, quotes);

        let cube = builder.calibrate().unwrap();
        assert_eq!(cube.num_slices(), 1);
    }
}
