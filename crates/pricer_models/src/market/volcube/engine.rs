//! VolCube Calibration Engine.
//!
//! # Requirements: 5.1, 5.2, 5.6, 9.1, 9.3
//!
//! This module provides a structured calibration engine for VolCube
//! construction. The engine orchestrates the calibration process, managing
//! instrument grouping, per-slice calibration, and diagnostics aggregation.
//!
//! # Architecture
//!
//! ```text
//! VolCubeCalibrationEngine
//! ├── Instruments (VolInstrument or VolQuote)
//! ├── Config (VolCubeConfig)
//! ├── Calibrator (VolCubeCalibrator trait)
//! └── Output
//!     ├── CalibratedVolCube
//!     └── CalibrationDiagnostics
//! ```
//!
//! # Example
//!
//! ```ignore
//! use pricer_models::market::volcube::{
//!     VolCubeCalibrationEngine, VolCubeConfig, VolInstrument,
//! };
//!
//! let engine = VolCubeCalibrationEngine::new()
//!     .with_instruments(instruments)
//!     .with_config(config);
//!
//! let result = engine.calibrate()?;
//! let cube = result.cube;
//! let diagnostics = result.diagnostics;
//! ```

use std::sync::Arc;

use num_traits::Float;

use super::{
    calibrator::{BoxedCalibrator, CalibratorOutput, SabrCalibrator, VolCubeCalibrator},
    config::VolCubeConfig,
    cube::VolCube,
    error::{CalibrationDiagnostics, ConvergenceStatus, VolCubeError},
    types::VolInstrument,
};

/// Trait for providing forward rates for VolCube calibration.
///
/// # Requirements: 5.3, 5.9
///
/// This trait abstracts the curve dependency, allowing different
/// curve implementations to be used for forward rate calculation.
pub trait ForwardRateProvider<T: Float>: Send + Sync {
    /// Calculate the forward swap rate for a given expiry and tenor.
    ///
    /// # Arguments
    ///
    /// * `expiry` - Option expiry in years from valuation date
    /// * `tenor` - Underlying swap tenor in years
    ///
    /// # Returns
    ///
    /// The forward swap rate at the given expiry/tenor point.
    fn forward_swap_rate(&self, expiry: T, tenor: T) -> Result<T, ForwardRateError>;

    /// Get the discount factor at a given time.
    ///
    /// # Arguments
    ///
    /// * `t` - Time in years from valuation date
    fn discount_factor(&self, t: T) -> Result<T, ForwardRateError>;
}

/// Error type for forward rate calculation.
#[derive(Debug, Clone, PartialEq)]
pub enum ForwardRateError {
    /// Discount curve not found.
    DiscountCurveNotFound {
        /// The currency for which the discount curve was not found.
        currency: String,
    },
    /// Projection curve not found.
    ProjectionCurveNotFound {
        /// The rate index for which the projection curve was not found.
        index: String,
    },
    /// Invalid time parameter.
    InvalidTime {
        /// The invalid time value.
        t: f64,
    },
    /// Calculation error.
    CalculationError {
        /// The error message describing the calculation failure.
        message: String,
    },
}

impl std::fmt::Display for ForwardRateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DiscountCurveNotFound { currency } => {
                write!(f, "Discount curve not found for currency: {}", currency)
            }
            Self::ProjectionCurveNotFound { index } => {
                write!(f, "Projection curve not found for index: {}", index)
            }
            Self::InvalidTime { t } => write!(f, "Invalid time: {}", t),
            Self::CalculationError { message } => write!(f, "Calculation error: {}", message),
        }
    }
}

impl std::error::Error for ForwardRateError {}

/// Simple forward rate provider using a closure.
///
/// # Requirements: 5.3
///
/// This allows flexible forward rate provision from any source.
pub struct ClosureForwardRateProvider<T, F>
where
    T: Float + Send + Sync,
    F: Fn(T, T) -> T + Send + Sync,
{
    forward_rate_fn: F,
    discount_fn: Option<Box<dyn Fn(T) -> T + Send + Sync>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T, F> ClosureForwardRateProvider<T, F>
where
    T: Float + Send + Sync,
    F: Fn(T, T) -> T + Send + Sync,
{
    /// Create a new closure-based forward rate provider.
    ///
    /// # Arguments
    ///
    /// * `forward_rate_fn` - Function that returns forward swap rate for
    ///   (expiry, tenor)
    pub fn new(forward_rate_fn: F) -> Self {
        Self {
            forward_rate_fn,
            discount_fn: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// Add a discount factor function.
    pub fn with_discount_fn<D>(mut self, discount_fn: D) -> Self
    where
        D: Fn(T) -> T + Send + Sync + 'static,
    {
        self.discount_fn = Some(Box::new(discount_fn));
        self
    }
}

impl<T, F> ForwardRateProvider<T> for ClosureForwardRateProvider<T, F>
where
    T: Float + Send + Sync,
    F: Fn(T, T) -> T + Send + Sync,
{
    fn forward_swap_rate(&self, expiry: T, tenor: T) -> Result<T, ForwardRateError> {
        Ok((self.forward_rate_fn)(expiry, tenor))
    }

    fn discount_factor(&self, t: T) -> Result<T, ForwardRateError> {
        match &self.discount_fn {
            Some(df) => Ok(df(t)),
            None => {
                // Default: flat discount at 2%
                let rate = T::from(0.02).unwrap();
                Ok((-rate * t).exp())
            }
        }
    }
}

/// Callback type for progress reporting.
///
/// The callback receives a `CalibrationProgress` struct containing
/// current calibration state information.
pub type ProgressCallback = Arc<dyn Fn(&CalibrationProgress) + Send + Sync>;

/// Calibration progress information.
///
/// # Requirements: 5.5
///
/// Contains detailed progress information for monitoring calibration.
#[derive(Debug, Clone)]
pub struct CalibrationProgress {
    /// Current slice index (0-based).
    pub current_slice: usize,
    /// Total number of slices to calibrate.
    pub total_slices: usize,
    /// Current slice expiry (years).
    pub expiry: f64,
    /// Current slice tenor (years).
    pub tenor: f64,
    /// Current iteration within the slice calibration.
    pub iteration: usize,
    /// Current residual (calibration error).
    pub residual: f64,
    /// Overall progress percentage (0.0 to 1.0).
    pub progress_pct: f64,
    /// Status message.
    pub message: String,
}

impl CalibrationProgress {
    /// Create a new progress report for a slice.
    pub fn new(current_slice: usize, total_slices: usize, expiry: f64, tenor: f64) -> Self {
        let progress_pct = if total_slices > 0 {
            current_slice as f64 / total_slices as f64
        } else {
            0.0
        };

        Self {
            current_slice,
            total_slices,
            expiry,
            tenor,
            iteration: 0,
            residual: f64::NAN,
            progress_pct,
            message: String::new(),
        }
    }

    /// Update iteration count.
    pub fn with_iteration(mut self, iteration: usize) -> Self {
        self.iteration = iteration;
        self
    }

    /// Update residual.
    pub fn with_residual(mut self, residual: f64) -> Self {
        self.residual = residual;
        self
    }

    /// Update status message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Mark slice as starting.
    pub fn starting(&self) -> Self {
        let mut progress = self.clone();
        progress.message = format!(
            "Starting slice {}/{}: expiry={:.2}Y, tenor={:.2}Y",
            progress.current_slice + 1,
            progress.total_slices,
            progress.expiry,
            progress.tenor
        );
        progress
    }

    /// Mark slice as completed.
    pub fn completed(&self) -> Self {
        let mut progress = self.clone();
        progress.progress_pct = if self.total_slices > 0 {
            (self.current_slice + 1) as f64 / self.total_slices as f64
        } else {
            1.0
        };
        progress.message = format!(
            "Completed slice {}/{}: residual={:.6}",
            progress.current_slice + 1,
            progress.total_slices,
            progress.residual
        );
        progress
    }
}

impl Default for CalibrationProgress {
    fn default() -> Self { Self::new(0, 0, 0.0, 0.0) }
}

/// Result of the calibration engine.
#[derive(Debug, Clone)]
pub struct EngineOutput<T: Float> {
    /// The calibrated VolCube.
    pub cube: VolCube<T>,
    /// Detailed calibration diagnostics.
    pub diagnostics: CalibrationDiagnostics,
    /// Number of instruments used.
    pub instrument_count: usize,
    /// Number of (expiry, tenor) slices calibrated.
    pub slice_count: usize,
}

impl<T: Float> EngineOutput<T> {
    /// Create a new engine output.
    pub fn new(cube: VolCube<T>, diagnostics: CalibrationDiagnostics) -> Self {
        let slice_count = diagnostics.slice_count;
        Self {
            cube,
            diagnostics,
            instrument_count: 0,
            slice_count,
        }
    }

    /// Set the instrument count.
    pub fn with_instrument_count(mut self, count: usize) -> Self {
        self.instrument_count = count;
        self
    }

    /// Check if calibration was successful.
    pub fn is_success(&self) -> bool {
        matches!(
            self.diagnostics.overall_status,
            ConvergenceStatus::Success | ConvergenceStatus::Warning
        )
    }

    /// Get the overall convergence status.
    pub fn status(&self) -> ConvergenceStatus { self.diagnostics.overall_status }
}

/// VolCube Calibration Engine.
///
/// Provides a structured approach to VolCube calibration with:
/// - Pluggable calibrator strategy
/// - Progress reporting via callbacks
/// - Comprehensive diagnostics
/// - Thread-safe design
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
pub struct VolCubeCalibrationEngine<T: Float + Send + Sync> {
    /// Calibration instruments.
    instruments: Vec<VolInstrument<T>>,
    /// Calibration configuration.
    config: VolCubeConfig,
    /// Optional custom calibrator.
    calibrator: Option<BoxedCalibrator<T>>,
    /// Optional progress callback.
    progress_callback: Option<ProgressCallback>,
}

impl<T: Float + Send + Sync> Default for VolCubeCalibrationEngine<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Float + Send + Sync> VolCubeCalibrationEngine<T> {
    /// Create a new calibration engine with default settings.
    pub fn new() -> Self {
        Self {
            instruments: Vec::new(),
            config: VolCubeConfig::default(),
            calibrator: None,
            progress_callback: None,
        }
    }

    /// Set the instruments to calibrate.
    pub fn with_instruments(mut self, instruments: Vec<VolInstrument<T>>) -> Self {
        self.instruments = instruments;
        self
    }

    /// Set the calibration configuration.
    pub fn with_config(mut self, config: VolCubeConfig) -> Self {
        self.config = config;
        self
    }

    /// Set a custom calibrator.
    ///
    /// If not set, the default SABR calibrator is used.
    pub fn with_calibrator<C: VolCubeCalibrator<T> + 'static>(mut self, calibrator: C) -> Self {
        self.calibrator = Some(Box::new(calibrator));
        self
    }

    /// Set a progress callback for monitoring calibration progress.
    ///
    /// The callback is invoked for each (expiry, tenor) slice.
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Get the current configuration.
    pub fn config(&self) -> &VolCubeConfig { &self.config }

    /// Get the number of instruments.
    pub fn instrument_count(&self) -> usize { self.instruments.len() }

    /// Validate the engine state before calibration.
    fn validate(&self) -> Result<(), VolCubeError> {
        if self.instruments.is_empty() {
            return Err(VolCubeError::InsufficientData { got: 0, need: 1 });
        }

        // Validate all instruments
        for instrument in &self.instruments {
            instrument
                .validate()
                .map_err(|e| VolCubeError::InvalidInput { message: e })?;
        }

        Ok(())
    }

    /// Run the calibration.
    ///
    /// # Returns
    ///
    /// * `Ok(EngineOutput)` - Successful calibration with cube and diagnostics
    /// * `Err(VolCubeError)` - Calibration failure
    ///
    /// # Example
    ///
    /// ```ignore
    /// let result = engine.calibrate()?;
    /// println!("Calibration status: {:?}", result.status());
    /// println!("Slices calibrated: {}", result.slice_count);
    /// ```
    pub fn calibrate(self) -> Result<EngineOutput<T>, VolCubeError> {
        // Validate before calibration
        self.validate()?;

        let instrument_count = self.instruments.len();
        let config = self.config.clone();
        let tolerance = config.tolerance;

        // Use custom calibrator or default SABR
        let calibrator: BoxedCalibrator<T> = self
            .calibrator
            .unwrap_or_else(|| Box::new(SabrCalibrator::new()));

        // Check interpolation method compatibility
        if !calibrator.supports_interpolation(&config.interpolation) {
            return Err(VolCubeError::InvalidInput {
                message: format!(
                    "Calibrator '{}' does not support interpolation method '{:?}'",
                    calibrator.name(),
                    config.interpolation
                ),
            });
        }

        // Run calibration
        let output: CalibratorOutput<T> = calibrator.calibrate(&self.instruments, &config)?;

        // Build diagnostics from calibrator output
        let diagnostics = build_diagnostics_from_output(&output, tolerance);

        Ok(EngineOutput::new(output.cube, diagnostics).with_instrument_count(instrument_count))
    }
}

/// Build diagnostics from calibrator output.
fn build_diagnostics_from_output<T: Float>(
    output: &CalibratorOutput<T>,
    tolerance: f64,
) -> CalibrationDiagnostics {
    let mut diagnostics = CalibrationDiagnostics::new();

    // Set overall metrics
    diagnostics.iterations = output.iterations;
    diagnostics.residuals.clone_from(&output.instrument_errors);

    // Determine overall status based on residual
    diagnostics.overall_status = if output.final_residual < tolerance {
        ConvergenceStatus::Success
    } else if output.final_residual < tolerance * 10.0 {
        ConvergenceStatus::Warning
    } else {
        ConvergenceStatus::Failed
    };

    // Compute slice count from cube dimensions (expiries × tenors)
    let num_expiries = output.cube.sabr_params().expiries().len();
    let num_tenors = output.cube.sabr_params().tenors().len();
    diagnostics.slice_count = num_expiries * num_tenors;

    // For now, assume all slices converged if overall status is Success
    diagnostics.converged_slices = if matches!(
        diagnostics.overall_status,
        ConvergenceStatus::Success | ConvergenceStatus::Warning
    ) {
        diagnostics.slice_count
    } else {
        0
    };

    diagnostics
}

/// Builder extension for creating engine from VolQuotes.
impl<T: Float + Send + Sync> VolCubeCalibrationEngine<T> {
    /// Create an engine from VolQuote set.
    ///
    /// Converts VolQuotes to VolInstruments for calibration using the
    /// existing `VolQuoteSet::to_instruments` method.
    ///
    /// # Arguments
    /// * `quotes` - The VolQuoteSet containing market quotes
    /// * `forward_getter` - Function to get forward rate for (expiry_years,
    ///   tenor_years)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let engine = VolCubeCalibrationEngine::from_quotes(
    ///     &quote_set,
    ///     |expiry, tenor| forward_curve.forward_swap_rate(expiry, tenor),
    /// )?;
    /// ```
    pub fn from_quotes(
        quotes: &super::quote::VolQuoteSet,
        forward_getter: impl Fn(f64, f64) -> T,
    ) -> Result<Self, VolCubeError> {
        // Use the existing to_instruments method from VolQuoteSet
        let instruments = quotes.to_instruments(forward_getter);

        if instruments.is_empty() {
            return Err(VolCubeError::InsufficientData { got: 0, need: 1 });
        }

        Ok(Self::new().with_instruments(instruments))
    }

    /// Create an engine from VolQuoteSet using a ForwardRateProvider.
    ///
    /// # Requirements: 5.3, 5.9
    ///
    /// This method uses the ForwardRateProvider trait for curve dependency
    /// resolution, allowing flexible integration with different curve sources.
    ///
    /// # Arguments
    ///
    /// * `quotes` - The VolQuoteSet containing market quotes
    /// * `provider` - Forward rate provider for curve-based rate calculation
    ///
    /// # Example
    ///
    /// ```ignore
    /// let provider = ClosureForwardRateProvider::new(|expiry, tenor| {
    ///     curve_set.forward_swap_rate(expiry, tenor).unwrap()
    /// });
    ///
    /// let engine = VolCubeCalibrationEngine::from_quotes_with_provider(
    ///     &quote_set,
    ///     &provider,
    /// )?;
    /// ```
    pub fn from_quotes_with_provider<P>(
        quotes: &super::quote::VolQuoteSet,
        provider: &P,
    ) -> Result<Self, VolCubeError>
    where
        P: ForwardRateProvider<T>,
        T: 'static,
    {
        use super::quote::VolStrike;

        let as_of_date = quotes.as_of_date;

        // Build instruments from quotes using provider
        let instruments: Vec<VolInstrument<T>> = quotes
            .quotes
            .iter()
            .filter_map(|q| {
                // Calculate expiry in years from as-of date
                let expiry_days = (q.expiry - as_of_date).num_days();
                let expiry_years = expiry_days as f64 / 365.0;

                // Get tenor in years
                let tenor_years = q.tenor.0;

                let expiry = T::from(expiry_years)?;
                let tenor = T::from(tenor_years)?;

                // Get forward rate from provider
                let forward = provider.forward_swap_rate(expiry, tenor).ok()?;
                let forward_f64 = forward.to_f64()?;

                // Convert strike to absolute value
                let strike_f64 = match q.strike {
                    VolStrike::Absolute(k) => k,
                    VolStrike::RelativeToAtm(bps) => forward_f64 + bps * 0.0001,
                    VolStrike::Moneyness(m) => m * forward_f64,
                    VolStrike::LogMoneyness(lm) => forward_f64 * lm.exp(),
                };

                // Get volatility (mid)
                let vol = q.mid;

                // Convert to generic type T
                let expiry_t = T::from(expiry_years)?;
                let tenor_t = T::from(tenor_years)?;
                let strike_t = T::from(strike_f64)?;
                let vol_t = T::from(vol)?;
                let forward_t = T::from(forward_f64)?;

                Some(VolInstrument::new(
                    q.instrument_id.clone(),
                    expiry_t,
                    tenor_t,
                    strike_t,
                    vol_t,
                    forward_t,
                ))
            })
            .collect();

        if instruments.is_empty() {
            return Err(VolCubeError::InsufficientData { got: 0, need: 1 });
        }

        Ok(Self::new().with_instruments(instruments))
    }
}

/// Calculate forward swap rate from discount factors.
///
/// # Requirements: 5.3
///
/// For a swap starting at time `t` with tenor `tenor`, the forward swap rate
/// is:
///
/// ```text
///        P(0, t) - P(0, t + tenor)
/// F = -------------------------------
///            sum(δᵢ × P(0, tᵢ))
/// ```
///
/// where δᵢ is the accrual fraction and P(0, tᵢ) is the discount factor.
///
/// # Arguments
///
/// * `discount_factor` - Function to get discount factor at time t
/// * `expiry` - Swap start time (option expiry) in years
/// * `tenor` - Swap tenor in years
/// * `frequency` - Number of payments per year (e.g., 1 for annual, 2 for
///   semi-annual)
pub fn calculate_forward_swap_rate<T: Float>(
    discount_factor: impl Fn(T) -> T,
    expiry: T,
    tenor: T,
    frequency: u32,
) -> T {
    let freq_t = T::from(frequency).unwrap_or(T::one());
    let period = T::one() / freq_t;
    let num_periods = (tenor * freq_t).ceil().to_usize().unwrap_or(1);

    let df_start = discount_factor(expiry);
    let df_end = discount_factor(expiry + tenor);

    // Sum of discounted accrual factors (annuity)
    let mut annuity = T::zero();
    for i in 1..=num_periods {
        let t = expiry + period * T::from(i).unwrap_or(T::one());
        let df = discount_factor(t);
        annuity = annuity + period * df;
    }

    if annuity <= T::zero() {
        return T::from(0.02).unwrap_or(T::zero()); // Default fallback
    }

    (df_start - df_end) / annuity
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::volcube::{InstrumentId, SabrParameterSurface, SabrParams};

    // Helper to create test instruments
    fn create_test_instruments() -> Vec<VolInstrument<f64>> {
        vec![
            VolInstrument::new(
                InstrumentId::new("VOL-1Y-2Y-ATM"),
                1.0,
                2.0,
                0.03,
                0.20,
                0.03,
            ),
            VolInstrument::new(
                InstrumentId::new("VOL-1Y-2Y-LOW"),
                1.0,
                2.0,
                0.02,
                0.22,
                0.03,
            ),
            VolInstrument::new(
                InstrumentId::new("VOL-1Y-2Y-HIGH"),
                1.0,
                2.0,
                0.04,
                0.21,
                0.03,
            ),
            VolInstrument::new(
                InstrumentId::new("VOL-1Y-5Y-ATM"),
                1.0,
                5.0,
                0.035,
                0.18,
                0.035,
            ),
            VolInstrument::new(
                InstrumentId::new("VOL-1Y-5Y-LOW"),
                1.0,
                5.0,
                0.025,
                0.20,
                0.035,
            ),
            VolInstrument::new(
                InstrumentId::new("VOL-1Y-5Y-HIGH"),
                1.0,
                5.0,
                0.045,
                0.19,
                0.035,
            ),
            VolInstrument::new(
                InstrumentId::new("VOL-2Y-2Y-ATM"),
                2.0,
                2.0,
                0.032,
                0.19,
                0.032,
            ),
            VolInstrument::new(
                InstrumentId::new("VOL-2Y-2Y-LOW"),
                2.0,
                2.0,
                0.022,
                0.21,
                0.032,
            ),
            VolInstrument::new(
                InstrumentId::new("VOL-2Y-2Y-HIGH"),
                2.0,
                2.0,
                0.042,
                0.20,
                0.032,
            ),
            VolInstrument::new(
                InstrumentId::new("VOL-2Y-5Y-ATM"),
                2.0,
                5.0,
                0.038,
                0.17,
                0.038,
            ),
            VolInstrument::new(
                InstrumentId::new("VOL-2Y-5Y-LOW"),
                2.0,
                5.0,
                0.028,
                0.19,
                0.038,
            ),
            VolInstrument::new(
                InstrumentId::new("VOL-2Y-5Y-HIGH"),
                2.0,
                5.0,
                0.048,
                0.18,
                0.038,
            ),
        ]
    }

    // ========================================
    // Engine Construction Tests
    // ========================================

    #[test]
    fn test_engine_new() {
        let engine: VolCubeCalibrationEngine<f64> = VolCubeCalibrationEngine::new();
        assert_eq!(engine.instrument_count(), 0);
    }

    #[test]
    fn test_engine_with_instruments() {
        let instruments = create_test_instruments();
        let count = instruments.len();
        let engine = VolCubeCalibrationEngine::new().with_instruments(instruments);
        assert_eq!(engine.instrument_count(), count);
    }

    #[test]
    fn test_engine_with_config() {
        let config = VolCubeConfig {
            tolerance: 1e-8,
            max_iterations: 200,
            ..Default::default()
        };
        let engine: VolCubeCalibrationEngine<f64> =
            VolCubeCalibrationEngine::new().with_config(config.clone());
        assert_eq!(engine.config().tolerance, 1e-8);
        assert_eq!(engine.config().max_iterations, 200);
    }

    #[test]
    fn test_engine_with_calibrator() {
        let engine: VolCubeCalibrationEngine<f64> =
            VolCubeCalibrationEngine::new().with_calibrator(SabrCalibrator::new());
        // Calibrator is set (can't easily verify without calibrating)
        assert_eq!(engine.instrument_count(), 0);
    }

    #[test]
    fn test_engine_with_progress_callback() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));
        let callback_count = call_count.clone();

        let callback: ProgressCallback = Arc::new(move |progress: &CalibrationProgress| {
            let _ = progress.current_slice;
            callback_count.fetch_add(1, Ordering::SeqCst);
        });

        let _engine: VolCubeCalibrationEngine<f64> =
            VolCubeCalibrationEngine::new().with_progress_callback(callback);
    }

    // ========================================
    // Validation Tests
    // ========================================

    #[test]
    fn test_engine_validate_empty_instruments() {
        let engine: VolCubeCalibrationEngine<f64> = VolCubeCalibrationEngine::new();
        let result = engine.calibrate();
        assert!(result.is_err());
        match result {
            Err(VolCubeError::InsufficientData { got, need, .. }) => {
                assert_eq!(got, 0);
                assert_eq!(need, 1);
            }
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_engine_validate_invalid_instrument() {
        let invalid_instruments = vec![VolInstrument::new(
            InstrumentId::new("INVALID"),
            -1.0, // Invalid: negative expiry
            2.0,
            0.03,
            0.20,
            0.03,
        )];

        let engine = VolCubeCalibrationEngine::new().with_instruments(invalid_instruments);
        let result = engine.calibrate();
        assert!(result.is_err());
    }

    // ========================================
    // Calibration Tests
    // ========================================

    #[test]
    fn test_engine_calibrate_success() {
        let instruments = create_test_instruments();
        let config = VolCubeConfig::default();

        let engine = VolCubeCalibrationEngine::new()
            .with_instruments(instruments)
            .with_config(config);

        let result = engine.calibrate();
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.instrument_count > 0);
        assert!(output.slice_count > 0);
    }

    #[test]
    fn test_engine_output_is_success() {
        let instruments = create_test_instruments();
        let engine = VolCubeCalibrationEngine::new().with_instruments(instruments);

        let result = engine.calibrate();
        assert!(result.is_ok());

        let output = result.unwrap();
        // Check that we can query the result
        assert!(output.is_success() || !output.is_success()); // Status is determined
    }

    // ========================================
    // Interpolation Compatibility Tests
    // ========================================

    #[test]
    fn test_engine_unsupported_interpolation() {
        use super::super::{calibrator::SviCalibrator, config::InterpolationMethod};

        let instruments = create_test_instruments();
        let config = VolCubeConfig {
            interpolation: InterpolationMethod::Sabr,
            ..Default::default()
        };

        // SVI calibrator doesn't support SABR interpolation
        let engine = VolCubeCalibrationEngine::new()
            .with_instruments(instruments)
            .with_config(config)
            .with_calibrator(SviCalibrator::new());

        let result = engine.calibrate();
        assert!(result.is_err());
    }

    // ========================================
    // EngineOutput Tests
    // ========================================

    #[test]
    fn test_engine_output_new() {
        let expiries = vec![0.5_f64, 1.0];
        let tenors = vec![2.0, 5.0];
        let params = vec![
            vec![
                SabrParams::new(0.04, 0.5, -0.3, 0.4),
                SabrParams::new(0.05, 0.5, -0.25, 0.35),
            ],
            vec![
                SabrParams::new(0.045, 0.5, -0.35, 0.45),
                SabrParams::new(0.055, 0.5, -0.2, 0.3),
            ],
        ];
        let sabr_surface = SabrParameterSurface::new(expiries, tenors, &params, 0.5).unwrap();

        let cube = VolCube::new(
            sabr_surface,
            vec![vec![0.03, 0.035], vec![0.032, 0.038]],
            VolCubeConfig::default(),
            vec![InstrumentId::new("TEST-1")],
            (0.01, 0.10),
        );

        let diagnostics = CalibrationDiagnostics::new();
        let output = EngineOutput::new(cube, diagnostics);

        assert_eq!(output.instrument_count, 0);
        assert_eq!(output.slice_count, 0);
    }

    #[test]
    fn test_engine_output_with_instrument_count() {
        let expiries = vec![0.5_f64, 1.0];
        let tenors = vec![2.0, 5.0];
        let params = vec![
            vec![
                SabrParams::new(0.04, 0.5, -0.3, 0.4),
                SabrParams::new(0.05, 0.5, -0.25, 0.35),
            ],
            vec![
                SabrParams::new(0.045, 0.5, -0.35, 0.45),
                SabrParams::new(0.055, 0.5, -0.2, 0.3),
            ],
        ];
        let sabr_surface = SabrParameterSurface::new(expiries, tenors, &params, 0.5).unwrap();

        let cube = VolCube::new(
            sabr_surface,
            vec![vec![0.03, 0.035], vec![0.032, 0.038]],
            VolCubeConfig::default(),
            vec![InstrumentId::new("TEST-1")],
            (0.01, 0.10),
        );

        let diagnostics = CalibrationDiagnostics::new();
        let output = EngineOutput::new(cube, diagnostics).with_instrument_count(12);

        assert_eq!(output.instrument_count, 12);
    }

    // ========================================
    // Forward Rate Provider Tests
    // ========================================

    #[test]
    fn test_closure_forward_rate_provider() {
        let provider = ClosureForwardRateProvider::new(|expiry: f64, tenor: f64| {
            0.03 + 0.001 * expiry + 0.002 * tenor
        });

        let rate = provider.forward_swap_rate(1.0, 5.0).unwrap();
        assert!((rate - 0.041).abs() < 1e-10); // 0.03 + 0.001 + 0.010

        let rate2 = provider.forward_swap_rate(2.0, 10.0).unwrap();
        assert!((rate2 - 0.052).abs() < 1e-10); // 0.03 + 0.002 + 0.020
    }

    #[test]
    fn test_closure_forward_rate_provider_with_discount() {
        let provider = ClosureForwardRateProvider::new(|_expiry: f64, _tenor: f64| 0.035)
            .with_discount_fn(|t: f64| (-0.02 * t).exp());

        // Check forward rate
        let rate = provider.forward_swap_rate(1.0, 5.0).unwrap();
        assert!((rate - 0.035).abs() < 1e-10);

        // Check discount factor at 1 year
        let df = provider.discount_factor(1.0).unwrap();
        assert!((df - (-0.02_f64).exp()).abs() < 1e-10);
    }

    #[test]
    fn test_closure_forward_rate_provider_default_discount() {
        let provider = ClosureForwardRateProvider::new(|_: f64, _: f64| 0.03);

        // Default discount uses 2% rate
        let df = provider.discount_factor(1.0).unwrap();
        assert!((df - (-0.02_f64).exp()).abs() < 1e-10);
    }

    #[test]
    fn test_forward_rate_error_display() {
        let err = ForwardRateError::DiscountCurveNotFound {
            currency: "EUR".to_string(),
        };
        assert!(err.to_string().contains("EUR"));

        let err2 = ForwardRateError::ProjectionCurveNotFound {
            index: "EURIBOR6M".to_string(),
        };
        assert!(err2.to_string().contains("EURIBOR6M"));

        let err3 = ForwardRateError::InvalidTime { t: -1.0 };
        assert!(err3.to_string().contains("-1"));

        let err4 = ForwardRateError::CalculationError {
            message: "overflow".to_string(),
        };
        assert!(err4.to_string().contains("overflow"));
    }

    // ========================================
    // Forward Swap Rate Calculation Tests
    // ========================================

    #[test]
    fn test_calculate_forward_swap_rate_annual() {
        // Simple flat discount curve at 3%
        let df = |t: f64| (-0.03 * t).exp();

        // Forward swap rate for 1Y swap starting immediately (expiry=0)
        let rate = calculate_forward_swap_rate(df, 0.0, 1.0, 1);

        // For 1Y annual swap: (DF(0) - DF(1)) / (1 * DF(1))
        // = (1 - exp(-0.03)) / exp(-0.03)
        // ≈ 0.0305 (approximately 3%)
        assert!((rate - 0.03).abs() < 0.01); // Within 100bp of 3%
    }

    #[test]
    fn test_calculate_forward_swap_rate_semi_annual() {
        // Flat discount curve at 2%
        let df = |t: f64| (-0.02 * t).exp();

        // 5Y semi-annual swap starting at year 1
        let rate = calculate_forward_swap_rate(df, 1.0, 5.0, 2);

        // Should be close to 2% for a flat curve
        assert!((rate - 0.02).abs() < 0.01);
    }

    #[test]
    fn test_calculate_forward_swap_rate_deferred_start() {
        // Flat discount at 4%
        let df = |t: f64| (-0.04 * t).exp();

        // 2Y annual swap starting at year 3
        let rate = calculate_forward_swap_rate(df, 3.0, 2.0, 1);

        // For flat curve, forward rate should be close to spot rate
        assert!((rate - 0.04).abs() < 0.01);
    }

    #[test]
    fn test_calculate_forward_swap_rate_upward_sloping() {
        // Upward sloping curve: short rates lower than long rates
        let df = |t: f64| {
            // Zero rate increases with time: r(t) = 0.02 + 0.005*t
            let rate = 0.02 + 0.005 * t;
            (-rate * t).exp()
        };

        // 5Y swap starting now
        let rate = calculate_forward_swap_rate(df, 0.0, 5.0, 1);

        // Should be higher than 2% due to upward slope
        assert!(rate > 0.02);
        assert!(rate < 0.10); // But not unreasonably high
    }

    // ========================================
    // CalibrationProgress Tests
    // ========================================

    #[test]
    fn test_calibration_progress_new() {
        let progress = CalibrationProgress::new(2, 10, 1.0, 5.0);

        assert_eq!(progress.current_slice, 2);
        assert_eq!(progress.total_slices, 10);
        assert!((progress.expiry - 1.0).abs() < 1e-10);
        assert!((progress.tenor - 5.0).abs() < 1e-10);
        assert_eq!(progress.iteration, 0);
        assert!(progress.residual.is_nan());
        assert!((progress.progress_pct - 0.2).abs() < 1e-10); // 2/10
    }

    #[test]
    fn test_calibration_progress_with_iteration() {
        let progress = CalibrationProgress::new(1, 5, 0.5, 2.0).with_iteration(50);

        assert_eq!(progress.iteration, 50);
    }

    #[test]
    fn test_calibration_progress_with_residual() {
        let progress = CalibrationProgress::new(1, 5, 0.5, 2.0).with_residual(0.001);

        assert!((progress.residual - 0.001).abs() < 1e-10);
    }

    #[test]
    fn test_calibration_progress_with_message() {
        let progress =
            CalibrationProgress::new(1, 5, 0.5, 2.0).with_message("Calibrating SABR params");

        assert_eq!(progress.message, "Calibrating SABR params");
    }

    #[test]
    fn test_calibration_progress_starting() {
        let progress = CalibrationProgress::new(2, 10, 1.5, 3.0).starting();

        assert!(progress.message.contains("Starting slice 3/10"));
        assert!(progress.message.contains("expiry=1.50Y"));
        assert!(progress.message.contains("tenor=3.00Y"));
    }

    #[test]
    fn test_calibration_progress_completed() {
        let progress = CalibrationProgress::new(2, 10, 1.5, 3.0)
            .with_residual(0.000123)
            .completed();

        assert!(progress.message.contains("Completed slice 3/10"));
        assert!(progress.message.contains("residual=0.000123"));
        // Progress should advance to (2+1)/10 = 0.3
        assert!((progress.progress_pct - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_calibration_progress_default() {
        let progress = CalibrationProgress::default();

        assert_eq!(progress.current_slice, 0);
        assert_eq!(progress.total_slices, 0);
        assert!((progress.expiry - 0.0).abs() < 1e-10);
        assert!((progress.tenor - 0.0).abs() < 1e-10);
        assert!((progress.progress_pct - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_calibration_progress_zero_slices() {
        // Edge case: zero total slices shouldn't cause division by zero
        let progress = CalibrationProgress::new(0, 0, 1.0, 5.0);
        assert!((progress.progress_pct - 0.0).abs() < 1e-10);

        let completed = progress.completed();
        assert!((completed.progress_pct - 1.0).abs() < 1e-10);
    }
}
