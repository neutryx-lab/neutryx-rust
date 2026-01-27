//! FX Market Builder for end-to-end market construction.
//!
//! This module provides orchestration for building complete FX market data
//! including discount curves, FX forward curves, and volatility surfaces.

use std::sync::Arc;

use chrono::NaiveDate;
use infra_master::{trade::instrument_def::CurrencyPair, Currency};
use num_traits::Float;
use thiserror::Error;

use super::{
    config::FxVolSurfaceConfig,
    lazy_surface::LazyFxVolSurface,
    surface::CalibratedFxVolSurface,
    vol_builder::{CalibrationDiagnostics, CalibrationError, FxVolSurfaceBuilder, VolQuote},
    FxCurve, SimpleFxCurve,
};
use crate::market::{
    calibration::bootstrapping::{BootstrapInstrument, CurveEngine, CurveEngineError},
    YieldCurve,
};

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during FX market construction.
#[derive(Debug, Clone, Error)]
pub enum FxMarketError {
    /// Failed to build domestic discount curve.
    #[error("Failed to build domestic curve: {reason}")]
    DomesticCurveFailed {
        /// Reason for failure.
        reason: String,
    },

    /// Failed to build foreign discount curve.
    #[error("Failed to build foreign curve: {reason}")]
    ForeignCurveFailed {
        /// Reason for failure.
        reason: String,
    },

    /// Failed to build FX forward curve.
    #[error("Failed to build FX curve: {reason}")]
    FxCurveFailed {
        /// Reason for failure.
        reason: String,
    },

    /// Failed to calibrate volatility surface.
    #[error("Failed to calibrate vol surface: {reason}")]
    VolSurfaceFailed {
        /// Reason for failure.
        reason: String,
    },

    /// Missing required input.
    #[error("Missing required input: {field}")]
    MissingInput {
        /// Name of the missing field.
        field: String,
    },

    /// Invalid configuration.
    #[error("Invalid configuration: {message}")]
    InvalidConfiguration {
        /// Description of the configuration error.
        message: String,
    },
}

impl From<CurveEngineError> for FxMarketError {
    fn from(e: CurveEngineError) -> Self {
        FxMarketError::DomesticCurveFailed {
            reason: e.to_string(),
        }
    }
}

impl From<CalibrationError> for FxMarketError {
    fn from(e: CalibrationError) -> Self {
        FxMarketError::VolSurfaceFailed {
            reason: e.to_string(),
        }
    }
}

// ============================================================================
// FxMarket Result Type
// ============================================================================

/// Complete FX market data structure.
///
/// Contains all calibrated market components needed for FX option pricing:
/// - Domestic and foreign discount curves
/// - FX forward curve
/// - Optional volatility surface
///
/// # Example
///
/// ```ignore
/// let market = FxMarketBuilder::new(CurrencyPair::eurusd())
///     .with_reference_date(ref_date)
///     .with_spot_rate(1.0850)
///     .with_domestic_instruments(ois_instruments)
///     .with_foreign_instruments(eur_instruments)
///     .build()?;
///
/// // Access market components
/// let fwd = market.fx_curve().forward_rate(0.5)?;
/// let vol = market.vol_surface().map(|s| s.volatility(1.10, 0.25));
/// ```
#[derive(Clone)]
pub struct FxMarket<T: Float> {
    /// Currency pair.
    currency_pair: CurrencyPair,
    /// Reference date for all curves.
    reference_date: NaiveDate,
    /// Domestic currency discount curve.
    domestic_curve: Arc<dyn YieldCurve<T> + Send + Sync>,
    /// Foreign currency discount curve.
    foreign_curve: Arc<dyn YieldCurve<T> + Send + Sync>,
    /// FX forward curve.
    fx_curve: Arc<dyn FxCurve<T> + Send + Sync>,
    /// Optional volatility surface.
    vol_surface: Option<CalibratedFxVolSurface<T>>,
    /// Calibration diagnostics.
    diagnostics: FxMarketDiagnostics,
}

impl<T: Float + Send + Sync + 'static> FxMarket<T> {
    /// Returns the currency pair.
    #[must_use]
    pub fn currency_pair(&self) -> CurrencyPair { self.currency_pair }

    /// Returns the reference date.
    #[must_use]
    pub fn reference_date(&self) -> NaiveDate { self.reference_date }

    /// Returns the domestic discount curve.
    #[must_use]
    pub fn domestic_curve(&self) -> &Arc<dyn YieldCurve<T> + Send + Sync> { &self.domestic_curve }

    /// Returns the foreign discount curve.
    #[must_use]
    pub fn foreign_curve(&self) -> &Arc<dyn YieldCurve<T> + Send + Sync> { &self.foreign_curve }

    /// Returns the FX forward curve.
    #[must_use]
    pub fn fx_curve(&self) -> &Arc<dyn FxCurve<T> + Send + Sync> { &self.fx_curve }

    /// Returns the volatility surface if available.
    #[must_use]
    pub fn vol_surface(&self) -> Option<&CalibratedFxVolSurface<T>> { self.vol_surface.as_ref() }

    /// Returns the calibration diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &FxMarketDiagnostics { &self.diagnostics }

    /// Returns the spot rate.
    #[must_use]
    pub fn spot_rate(&self) -> T { self.fx_curve.spot_rate() }

    /// Returns the forward rate at a given time.
    pub fn forward_rate(&self, t: T) -> Result<T, super::FxCurveError> {
        self.fx_curve.forward_rate(t)
    }
}

// ============================================================================
// Diagnostics
// ============================================================================

/// Diagnostics from FX market construction.
#[derive(Debug, Clone, Default)]
pub struct FxMarketDiagnostics {
    /// Domestic curve construction iterations.
    pub domestic_iterations: usize,
    /// Foreign curve construction iterations.
    pub foreign_iterations: usize,
    /// Volatility surface calibration diagnostics.
    pub vol_diagnostics: Option<CalibrationDiagnostics>,
    /// Total build time in milliseconds.
    pub build_time_ms: u64,
}

// ============================================================================
// FxMarketBuilder
// ============================================================================

/// Builder for constructing complete FX market data.
///
/// Orchestrates the construction of:
/// 1. Domestic OIS discount curve
/// 2. Foreign OIS discount curve
/// 3. FX forward curve (from curves or FX swaps)
/// 4. Optional volatility surface
///
/// # Construction Order
///
/// The build process follows a strict dependency chain:
/// 1. Domestic curve (independent)
/// 2. Foreign curve (independent, can parallel with domestic)
/// 3. FX curve (depends on both discount curves)
/// 4. Vol surface (depends on FX curve)
///
/// # Example
///
/// ```ignore
/// let market = FxMarketBuilder::new(CurrencyPair::eurusd())
///     .with_reference_date(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
///     .with_spot_rate(1.0850)
///     .with_domestic_instruments(usd_ois)
///     .with_foreign_instruments(eur_ois)
///     .with_vol_quotes(vol_quotes)
///     .build()?;
/// ```
pub struct FxMarketBuilder<T: Float> {
    /// Currency pair.
    currency_pair: CurrencyPair,
    /// Reference date.
    reference_date: Option<NaiveDate>,
    /// Spot FX rate.
    spot_rate: Option<T>,
    /// Pre-built domestic curve (if provided).
    prebuilt_domestic: Option<Arc<dyn YieldCurve<T> + Send + Sync>>,
    /// Pre-built foreign curve (if provided).
    prebuilt_foreign: Option<Arc<dyn YieldCurve<T> + Send + Sync>>,
    /// Domestic OIS instruments for bootstrapping.
    domestic_instruments: Vec<BootstrapInstrument<T>>,
    /// Foreign OIS instruments for bootstrapping.
    foreign_instruments: Vec<BootstrapInstrument<T>>,
    /// Volatility quotes.
    vol_quotes: Vec<VolQuote<T>>,
    /// Volatility surface configuration.
    vol_config: FxVolSurfaceConfig,
    /// Whether to use lazy evaluation for vol surface.
    lazy_vol_surface: bool,
}

impl<T: Float + Send + Sync + 'static> FxMarketBuilder<T> {
    /// Creates a new FX market builder.
    pub fn new(currency_pair: CurrencyPair) -> Self {
        Self {
            currency_pair,
            reference_date: None,
            spot_rate: None,
            prebuilt_domestic: None,
            prebuilt_foreign: None,
            domestic_instruments: Vec::new(),
            foreign_instruments: Vec::new(),
            vol_quotes: Vec::new(),
            vol_config: FxVolSurfaceConfig::default(),
            lazy_vol_surface: false,
        }
    }

    /// Creates a builder for EURUSD.
    pub fn eurusd() -> Self { Self::new(CurrencyPair::new(Currency::EUR, Currency::USD)) }

    /// Creates a builder for USDJPY.
    pub fn usdjpy() -> Self { Self::new(CurrencyPair::new(Currency::USD, Currency::JPY)) }

    /// Sets the reference date.
    pub fn with_reference_date(mut self, date: NaiveDate) -> Self {
        self.reference_date = Some(date);
        self
    }

    /// Sets the spot FX rate.
    pub fn with_spot_rate(mut self, spot: T) -> Self {
        self.spot_rate = Some(spot);
        self
    }

    /// Sets a pre-built domestic discount curve.
    ///
    /// Use this when you have an existing curve and don't need to bootstrap.
    pub fn with_prebuilt_domestic_curve(
        mut self,
        curve: Arc<dyn YieldCurve<T> + Send + Sync>,
    ) -> Self {
        self.prebuilt_domestic = Some(curve);
        self
    }

    /// Sets a pre-built foreign discount curve.
    ///
    /// Use this when you have an existing curve and don't need to bootstrap.
    pub fn with_prebuilt_foreign_curve(
        mut self,
        curve: Arc<dyn YieldCurve<T> + Send + Sync>,
    ) -> Self {
        self.prebuilt_foreign = Some(curve);
        self
    }

    /// Sets domestic OIS instruments for bootstrapping.
    pub fn with_domestic_instruments(mut self, instruments: Vec<BootstrapInstrument<T>>) -> Self {
        self.domestic_instruments = instruments;
        self
    }

    /// Sets foreign OIS instruments for bootstrapping.
    pub fn with_foreign_instruments(mut self, instruments: Vec<BootstrapInstrument<T>>) -> Self {
        self.foreign_instruments = instruments;
        self
    }

    /// Adds a domestic OIS instrument.
    pub fn add_domestic_ois(mut self, maturity: T, rate: T) -> Self {
        self.domestic_instruments
            .push(BootstrapInstrument::ois(maturity, rate));
        self
    }

    /// Adds a foreign OIS instrument.
    pub fn add_foreign_ois(mut self, maturity: T, rate: T) -> Self {
        self.foreign_instruments
            .push(BootstrapInstrument::ois(maturity, rate));
        self
    }

    /// Sets volatility quotes.
    pub fn with_vol_quotes(mut self, quotes: Vec<VolQuote<T>>) -> Self {
        self.vol_quotes = quotes;
        self
    }

    /// Adds a volatility quote.
    pub fn add_vol_quote(mut self, quote: VolQuote<T>) -> Self {
        self.vol_quotes.push(quote);
        self
    }

    /// Sets the volatility surface configuration.
    pub fn with_vol_config(mut self, config: FxVolSurfaceConfig) -> Self {
        self.vol_config = config;
        self
    }

    /// Enables lazy evaluation for the volatility surface.
    pub fn with_lazy_vol_surface(mut self, lazy: bool) -> Self {
        self.lazy_vol_surface = lazy;
        self
    }

    /// Builds just the discount curves.
    ///
    /// Returns (domestic_curve, foreign_curve).
    pub fn build_discount_curves(
        &self,
    ) -> Result<
        (
            Arc<dyn YieldCurve<T> + Send + Sync>,
            Arc<dyn YieldCurve<T> + Send + Sync>,
        ),
        FxMarketError,
    > {
        let domestic = self.build_domestic_curve()?;
        let foreign = self.build_foreign_curve()?;
        Ok((domestic, foreign))
    }

    /// Builds just the FX forward curve.
    ///
    /// Requires discount curves to be available (either pre-built or via
    /// instruments).
    pub fn build_fx_curve(&self) -> Result<Arc<dyn FxCurve<T> + Send + Sync>, FxMarketError> {
        let _reference_date = self
            .reference_date
            .ok_or_else(|| FxMarketError::MissingInput {
                field: "reference_date".to_string(),
            })?;

        let spot = self.spot_rate.ok_or_else(|| FxMarketError::MissingInput {
            field: "spot_rate".to_string(),
        })?;

        let (domestic, foreign) = self.build_discount_curves()?;

        // Use SimpleFxCurve for interest rate parity based forward curve
        let fx_curve = SimpleFxCurve::new(self.currency_pair, spot, domestic, foreign);

        Ok(Arc::new(fx_curve))
    }

    /// Builds just the volatility surface.
    ///
    /// Requires FX curve to be available.
    pub fn build_vol_surface(
        &self,
    ) -> Result<(CalibratedFxVolSurface<T>, CalibrationDiagnostics), FxMarketError> {
        if self.vol_quotes.is_empty() {
            return Err(FxMarketError::MissingInput {
                field: "vol_quotes".to_string(),
            });
        }

        let reference_date = self
            .reference_date
            .ok_or_else(|| FxMarketError::MissingInput {
                field: "reference_date".to_string(),
            })?;

        let fx_curve = self.build_fx_curve()?;

        let mut builder = FxVolSurfaceBuilder::new(self.currency_pair)
            .with_reference_date(reference_date)
            .with_fx_curve(fx_curve)
            .with_config(self.vol_config.clone());

        for quote in &self.vol_quotes {
            builder = builder.add_quotes(vec![quote.clone()]);
        }

        let (surface, diag) = builder.build()?;
        Ok((surface, diag))
    }

    /// Builds the complete FX market.
    pub fn build(self) -> Result<FxMarket<T>, FxMarketError> {
        let start = std::time::Instant::now();

        let reference_date = self
            .reference_date
            .ok_or_else(|| FxMarketError::MissingInput {
                field: "reference_date".to_string(),
            })?;

        let spot = self.spot_rate.ok_or_else(|| FxMarketError::MissingInput {
            field: "spot_rate".to_string(),
        })?;

        // Step 1: Build discount curves
        let domestic = self.build_domestic_curve()?;
        let foreign = self.build_foreign_curve()?;

        // Step 2: Build FX forward curve using interest rate parity
        let fx_curve = SimpleFxCurve::new(
            self.currency_pair,
            spot,
            Arc::clone(&domestic),
            Arc::clone(&foreign),
        );

        let fx_curve: Arc<dyn FxCurve<T> + Send + Sync> = Arc::new(fx_curve);

        // Step 3: Build volatility surface (if quotes provided)
        let (vol_surface, vol_diagnostics) = if !self.vol_quotes.is_empty() {
            let mut builder = FxVolSurfaceBuilder::new(self.currency_pair)
                .with_reference_date(reference_date)
                .with_fx_curve(Arc::clone(&fx_curve))
                .with_config(self.vol_config.clone());

            for quote in &self.vol_quotes {
                builder = builder.add_quotes(vec![quote.clone()]);
            }

            let (surface, diag) = builder.build()?;
            (Some(surface), Some(diag))
        } else {
            (None, None)
        };

        let build_time = start.elapsed().as_millis() as u64;

        Ok(FxMarket {
            currency_pair: self.currency_pair,
            reference_date,
            domestic_curve: domestic,
            foreign_curve: foreign,
            fx_curve,
            vol_surface,
            diagnostics: FxMarketDiagnostics {
                domestic_iterations: 0, // TODO: Get from curve engine
                foreign_iterations: 0,
                vol_diagnostics,
                build_time_ms: build_time,
            },
        })
    }

    /// Builds a lazy FX market with deferred vol surface calibration.
    pub fn build_lazy(self) -> Result<(FxMarket<T>, Option<LazyFxVolSurface<T>>), FxMarketError> {
        let start = std::time::Instant::now();

        let reference_date = self
            .reference_date
            .ok_or_else(|| FxMarketError::MissingInput {
                field: "reference_date".to_string(),
            })?;

        let spot = self.spot_rate.ok_or_else(|| FxMarketError::MissingInput {
            field: "spot_rate".to_string(),
        })?;

        // Build discount curves
        let domestic = self.build_domestic_curve()?;
        let foreign = self.build_foreign_curve()?;

        // Build FX forward curve using interest rate parity
        let fx_curve = SimpleFxCurve::new(
            self.currency_pair,
            spot,
            Arc::clone(&domestic),
            Arc::clone(&foreign),
        );

        let fx_curve: Arc<dyn FxCurve<T> + Send + Sync> = Arc::new(fx_curve);

        // Create lazy vol surface builder (if quotes provided)
        let lazy_surface = if !self.vol_quotes.is_empty() {
            let mut builder = FxVolSurfaceBuilder::new(self.currency_pair)
                .with_reference_date(reference_date)
                .with_fx_curve(Arc::clone(&fx_curve))
                .with_config(self.vol_config.clone());

            for quote in &self.vol_quotes {
                builder = builder.add_quotes(vec![quote.clone()]);
            }

            Some(LazyFxVolSurface::new(builder))
        } else {
            None
        };

        let build_time = start.elapsed().as_millis() as u64;

        let market = FxMarket {
            currency_pair: self.currency_pair,
            reference_date,
            domestic_curve: domestic,
            foreign_curve: foreign,
            fx_curve,
            vol_surface: None, // Lazy - not yet calibrated
            diagnostics: FxMarketDiagnostics {
                domestic_iterations: 0,
                foreign_iterations: 0,
                vol_diagnostics: None,
                build_time_ms: build_time,
            },
        };

        Ok((market, lazy_surface))
    }

    /// Builds the domestic discount curve.
    fn build_domestic_curve(&self) -> Result<Arc<dyn YieldCurve<T> + Send + Sync>, FxMarketError> {
        if let Some(ref curve) = self.prebuilt_domestic {
            return Ok(Arc::clone(curve));
        }

        if self.domestic_instruments.is_empty() {
            return Err(FxMarketError::MissingInput {
                field: "domestic_instruments or prebuilt_domestic_curve".to_string(),
            });
        }

        // Use CurveEngine to bootstrap
        let engine = CurveEngine::<T>::new();
        let result = engine
            .build_curve_from_instruments(&self.domestic_instruments)
            .map_err(|e| FxMarketError::DomesticCurveFailed {
                reason: e.to_string(),
            })?;

        Ok(Arc::new(result.curve) as Arc<dyn YieldCurve<T> + Send + Sync>)
    }

    /// Builds the foreign discount curve.
    fn build_foreign_curve(&self) -> Result<Arc<dyn YieldCurve<T> + Send + Sync>, FxMarketError> {
        if let Some(ref curve) = self.prebuilt_foreign {
            return Ok(Arc::clone(curve));
        }

        if self.foreign_instruments.is_empty() {
            return Err(FxMarketError::MissingInput {
                field: "foreign_instruments or prebuilt_foreign_curve".to_string(),
            });
        }

        // Use CurveEngine to bootstrap
        let engine = CurveEngine::<T>::new();
        let result = engine
            .build_curve_from_instruments(&self.foreign_instruments)
            .map_err(|e| FxMarketError::ForeignCurveFailed {
                reason: e.to_string(),
            })?;

        Ok(Arc::new(result.curve) as Arc<dyn YieldCurve<T> + Send + Sync>)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::curves::FlatCurve;

    fn make_prebuilt_curves() -> (
        Arc<dyn YieldCurve<f64> + Send + Sync>,
        Arc<dyn YieldCurve<f64> + Send + Sync>,
    ) {
        let domestic = Arc::new(FlatCurve::new(0.05));
        let foreign = Arc::new(FlatCurve::new(0.03));
        (domestic, foreign)
    }

    #[test]
    fn test_fx_market_builder_new() {
        let builder = FxMarketBuilder::<f64>::eurusd();
        assert_eq!(
            builder.currency_pair,
            CurrencyPair::new(Currency::EUR, Currency::USD)
        );
    }

    #[test]
    fn test_fx_market_builder_with_prebuilt_curves() {
        let (domestic, foreign) = make_prebuilt_curves();
        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let market = FxMarketBuilder::<f64>::eurusd()
            .with_reference_date(ref_date)
            .with_spot_rate(1.0850)
            .with_prebuilt_domestic_curve(domestic)
            .with_prebuilt_foreign_curve(foreign)
            .build();

        assert!(market.is_ok());
        let market = market.unwrap();
        assert_eq!(market.reference_date(), ref_date);
        assert!((market.spot_rate() - 1.0850).abs() < 1e-10);
    }

    #[test]
    fn test_fx_market_forward_rate() {
        let (domestic, foreign) = make_prebuilt_curves();
        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let market = FxMarketBuilder::<f64>::eurusd()
            .with_reference_date(ref_date)
            .with_spot_rate(1.0850)
            .with_prebuilt_domestic_curve(domestic)
            .with_prebuilt_foreign_curve(foreign)
            .build()
            .unwrap();

        let fwd = market.forward_rate(1.0);
        assert!(fwd.is_ok());
        // F = S * DF_f / DF_d
        // With flat curves: F ≈ 1.0850 * exp(-0.03) / exp(-0.05) ≈ 1.0850 *
        // exp(0.02)
    }

    #[test]
    fn test_fx_market_missing_reference_date() {
        let (domestic, foreign) = make_prebuilt_curves();

        let result = FxMarketBuilder::<f64>::eurusd()
            .with_spot_rate(1.0850)
            .with_prebuilt_domestic_curve(domestic)
            .with_prebuilt_foreign_curve(foreign)
            .build();

        assert!(matches!(result, Err(FxMarketError::MissingInput { .. })));
    }

    #[test]
    fn test_fx_market_missing_spot() {
        let (domestic, foreign) = make_prebuilt_curves();
        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let result = FxMarketBuilder::<f64>::eurusd()
            .with_reference_date(ref_date)
            .with_prebuilt_domestic_curve(domestic)
            .with_prebuilt_foreign_curve(foreign)
            .build();

        assert!(matches!(result, Err(FxMarketError::MissingInput { .. })));
    }

    #[test]
    fn test_fx_market_with_vol_quotes() {
        let (domestic, foreign) = make_prebuilt_curves();
        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2024, 4, 15).unwrap();

        let market = FxMarketBuilder::<f64>::eurusd()
            .with_reference_date(ref_date)
            .with_spot_rate(1.0850)
            .with_prebuilt_domestic_curve(domestic)
            .with_prebuilt_foreign_curve(foreign)
            .add_vol_quote(VolQuote::atm(expiry, 0.08))
            .build();

        assert!(market.is_ok());
        let market = market.unwrap();
        assert!(market.vol_surface().is_some());
    }

    #[test]
    fn test_fx_market_lazy_build() {
        let (domestic, foreign) = make_prebuilt_curves();
        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let expiry = NaiveDate::from_ymd_opt(2024, 4, 15).unwrap();

        let result = FxMarketBuilder::<f64>::eurusd()
            .with_reference_date(ref_date)
            .with_spot_rate(1.0850)
            .with_prebuilt_domestic_curve(domestic)
            .with_prebuilt_foreign_curve(foreign)
            .add_vol_quote(VolQuote::atm(expiry, 0.08))
            .build_lazy();

        assert!(result.is_ok());
        let (market, lazy_surface) = result.unwrap();

        // Vol surface not yet calibrated in market
        assert!(market.vol_surface().is_none());

        // Lazy surface available
        assert!(lazy_surface.is_some());
        let lazy = lazy_surface.unwrap();
        assert!(!lazy.is_calibrated());

        // Trigger calibration
        let _ = lazy.force_calibrate();
        assert!(lazy.is_calibrated());
    }

    #[test]
    fn test_fx_market_diagnostics() {
        let (domestic, foreign) = make_prebuilt_curves();
        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let market = FxMarketBuilder::<f64>::eurusd()
            .with_reference_date(ref_date)
            .with_spot_rate(1.0850)
            .with_prebuilt_domestic_curve(domestic)
            .with_prebuilt_foreign_curve(foreign)
            .build()
            .unwrap();

        let _diag = market.diagnostics();
    }

    #[test]
    fn test_fx_market_error_display() {
        let err = FxMarketError::MissingInput {
            field: "spot_rate".to_string(),
        };
        assert!(err.to_string().contains("spot_rate"));
    }

    #[test]
    fn test_build_discount_curves_only() {
        let (domestic, foreign) = make_prebuilt_curves();
        let ref_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let builder = FxMarketBuilder::<f64>::eurusd()
            .with_reference_date(ref_date)
            .with_spot_rate(1.0850)
            .with_prebuilt_domestic_curve(domestic)
            .with_prebuilt_foreign_curve(foreign);

        let result = builder.build_discount_curves();
        assert!(result.is_ok());
    }
}
