//! Index-keyed market data container.
//!
//! This module provides [`IndexedMarket`], a container that stores market data
//! (curves, volatility cubes, FX curves, FX vol surfaces) keyed by their
//! logical index types (`RateIndex`, `CurrencyPair`) rather than string names.
//!
//! # Architecture
//!
//! ```text
//! IndexedMarket
//! ├── curves: HashMap<RateIndex, Arc<dyn YieldCurve>>
//! ├── volcubes: HashMap<RateIndex, VolCube<T>>
//! ├── fx_curves: HashMap<CurrencyPair, Arc<dyn FxCurve>>
//! ├── fx_vol_surfaces: HashMap<CurrencyPair, Arc<dyn VolatilitySurface>>
//! └── valuation_date: Date
//! ```
//!
//! # Examples
//!
//! ```ignore
//! use pricer_models::market::{IndexedMarket, IndexedMarketBuilder};
//! use infra_master::{RateIndex, Date};
//!
//! let market = IndexedMarketBuilder::new()
//!     .valuation_date(Date::from_ymd(2025, 1, 15).unwrap())
//!     .with_curve(RateIndex::Sofr, sofr_curve)
//!     .with_curve(RateIndex::Euribor3M, euribor_curve)
//!     .build()?;
//!
//! // Access curve by index
//! let curve = market.curve(RateIndex::Sofr)?;
//! ```

use std::{collections::HashMap, sync::Arc};

use infra_master::{trade::instrument_def::CurrencyPair, Date, RateIndex};
use num_traits::Float;

use super::{
    curves::{CurveSet, YieldCurve},
    error::{MarketBuildError, MarketDataError},
    fx_calibration::FxCurve,
    index_mapper::IndexCurveMapper,
    surfaces::VolatilitySurface,
    volcube::VolCube,
};

// ============================================================================
// IndexedMarket
// ============================================================================

/// Index-keyed market data container.
///
/// Provides type-safe access to market data using logical index types
/// (`RateIndex`, `CurrencyPair`) instead of string-based keys.
///
/// # Design Goals
///
/// 1. **Type Safety**: Uses enum-based keys for compile-time verification
/// 2. **Flexibility**: Generic over float type `T` for AD compatibility
/// 3. **Backward Compatibility**: Optional fallback to `CurveSet`
/// 4. **Ergonomic API**: Simple accessors for common operations
///
/// # Examples
///
/// ```ignore
/// use pricer_models::market::IndexedMarket;
/// use infra_master::RateIndex;
///
/// // Check if index is available
/// if market.has_curve(RateIndex::Sofr) {
///     let curve = market.curve(RateIndex::Sofr)?;
///     let df = curve.discount_factor(1.0)?;
/// }
/// ```
pub struct IndexedMarket<T: Float> {
    /// Valuation date for this market snapshot.
    valuation_date: Date,

    /// Yield curves keyed by rate index.
    curves: HashMap<RateIndex, Arc<dyn YieldCurve<T> + Send + Sync>>,

    /// Volatility cubes keyed by rate index.
    volcubes: HashMap<RateIndex, VolCube<T>>,

    /// FX forward curves keyed by currency pair.
    fx_curves: HashMap<CurrencyPair, Arc<dyn FxCurve<T> + Send + Sync>>,

    /// FX volatility surfaces keyed by currency pair.
    fx_vol_surfaces: HashMap<CurrencyPair, Arc<dyn VolatilitySurface<T> + Send + Sync>>,

    /// Optional fallback curve set for backward compatibility.
    /// NOTE: Fallback via CurveSet is planned for future iterations.
    #[allow(dead_code)]
    fallback_curve_set: Option<CurveSet<T>>,

    /// Optional index mapper for CurveSet fallback.
    /// NOTE: Fallback via CurveSet is planned for future iterations.
    #[allow(dead_code)]
    index_mapper: Option<Arc<dyn IndexCurveMapper + Send + Sync>>,
}

#[allow(clippy::missing_fields_in_debug)]
impl<T: Float> std::fmt::Debug for IndexedMarket<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexedMarket")
            .field("valuation_date", &self.valuation_date)
            .field("curves_count", &self.curves.len())
            .field("volcubes_count", &self.volcubes.len())
            .field("fx_curves_count", &self.fx_curves.len())
            .field("fx_vol_surfaces_count", &self.fx_vol_surfaces.len())
            .finish_non_exhaustive()
    }
}

impl<T: Float> IndexedMarket<T> {
    /// Returns the valuation date.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let date = market.valuation_date();
    /// ```
    #[must_use]
    pub fn valuation_date(&self) -> Date { self.valuation_date }

    // ========================================
    // Curve Access (Task 2.2)
    // ========================================

    /// Returns the yield curve for the given rate index.
    ///
    /// # Arguments
    ///
    /// * `index` - The rate index to look up
    ///
    /// # Returns
    ///
    /// * `Ok(Arc<dyn YieldCurve>)` - The curve for the index
    /// * `Err(MarketDataError::IndexNotFound)` - If index is not available
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let sofr_curve = market.curve(RateIndex::Sofr)?;
    /// let df = sofr_curve.discount_factor(1.0)?;
    /// ```
    pub fn curve(
        &self,
        index: RateIndex,
    ) -> Result<Arc<dyn YieldCurve<T> + Send + Sync>, MarketDataError> {
        // First try direct lookup
        if let Some(curve) = self.curves.get(&index) {
            return Ok(Arc::clone(curve));
        }

        // CurveSet fallback is disabled for now due to type mismatch
        // (CurveSet returns &CurveEnum, not Arc<dyn YieldCurve>)
        // This can be addressed in future iterations

        Err(MarketDataError::IndexNotFound {
            index: format!("{:?}", index),
        })
    }

    /// Returns the discount factor for the given rate index at time t.
    ///
    /// This is a convenience method that retrieves the curve and
    /// calls `discount_factor` on it.
    ///
    /// # Arguments
    ///
    /// * `index` - The rate index for the curve
    /// * `t` - Time to maturity in years
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let df = market.discount_factor(RateIndex::Sofr, 1.0)?;
    /// ```
    pub fn discount_factor(&self, index: RateIndex, t: T) -> Result<T, MarketDataError> {
        let curve = self.curve(index)?;
        curve.discount_factor(t)
    }

    /// Returns the forward rate for the given rate index between t1 and t2.
    ///
    /// # Arguments
    ///
    /// * `index` - The rate index for the curve
    /// * `t1` - Start time in years
    /// * `t2` - End time in years
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let fwd = market.forward_rate(RateIndex::Sofr, 1.0, 2.0)?;
    /// ```
    pub fn forward_rate(&self, index: RateIndex, t1: T, t2: T) -> Result<T, MarketDataError> {
        let curve = self.curve(index)?;
        curve.forward_rate(t1, t2)
    }

    // ========================================
    // VolCube Access (Task 2.3)
    // ========================================

    /// Returns the volatility cube for the given rate index.
    ///
    /// # Arguments
    ///
    /// * `index` - The rate index to look up
    ///
    /// # Returns
    ///
    /// * `Ok(&VolCube<T>)` - Reference to the vol cube
    /// * `Err(MarketDataError::IndexNotFound)` - If index is not available
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let cube = market.volcube(RateIndex::Sofr)?;
    /// let vol = cube.volatility(strike, expiry, tenor)?;
    /// ```
    pub fn volcube(&self, index: RateIndex) -> Result<&VolCube<T>, MarketDataError> {
        self.volcubes
            .get(&index)
            .ok_or(MarketDataError::IndexNotFound {
                index: format!("{:?}", index),
            })
    }

    // ========================================
    // FX Access (Task 2.4)
    // ========================================

    /// Returns the FX forward curve for the given currency pair.
    ///
    /// # Arguments
    ///
    /// * `pair` - The currency pair to look up
    ///
    /// # Returns
    ///
    /// * `Ok(Arc<dyn FxCurve>)` - The FX curve
    /// * `Err(MarketDataError::IndexNotFound)` - If pair is not available
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
    /// let fx_curve = market.fx_curve(pair)?;
    /// let fwd = fx_curve.forward_rate(1.0)?;
    /// ```
    pub fn fx_curve(
        &self,
        pair: CurrencyPair,
    ) -> Result<Arc<dyn FxCurve<T> + Send + Sync>, MarketDataError> {
        self.fx_curves
            .get(&pair)
            .cloned()
            .ok_or(MarketDataError::IndexNotFound {
                index: format!("{}", pair),
            })
    }

    /// Returns the FX volatility surface for the given currency pair.
    ///
    /// # Arguments
    ///
    /// * `pair` - The currency pair to look up
    ///
    /// # Returns
    ///
    /// * `Ok(Arc<dyn VolatilitySurface>)` - The vol surface
    /// * `Err(MarketDataError::IndexNotFound)` - If pair is not available
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
    /// let vol_surface = market.fx_vol_surface(pair)?;
    /// let vol = vol_surface.volatility(strike, expiry)?;
    /// ```
    pub fn fx_vol_surface(
        &self,
        pair: CurrencyPair,
    ) -> Result<Arc<dyn VolatilitySurface<T> + Send + Sync>, MarketDataError> {
        self.fx_vol_surfaces
            .get(&pair)
            .cloned()
            .ok_or(MarketDataError::IndexNotFound {
                index: format!("{}", pair),
            })
    }

    // ========================================
    // Index Availability Methods (Task 2.5)
    // ========================================

    /// Returns `true` if a curve exists for the given rate index.
    #[must_use]
    pub fn has_curve(&self, index: RateIndex) -> bool { self.curves.contains_key(&index) }

    /// Returns `true` if a volatility cube exists for the given rate index.
    #[must_use]
    pub fn has_volcube(&self, index: RateIndex) -> bool { self.volcubes.contains_key(&index) }

    /// Returns `true` if an FX curve exists for the given currency pair.
    #[must_use]
    pub fn has_fx_curve(&self, pair: CurrencyPair) -> bool { self.fx_curves.contains_key(&pair) }

    /// Returns `true` if an FX vol surface exists for the given currency pair.
    #[must_use]
    pub fn has_fx_vol_surface(&self, pair: CurrencyPair) -> bool {
        self.fx_vol_surfaces.contains_key(&pair)
    }

    /// Returns all available rate indices for curves.
    #[must_use]
    pub fn available_rate_indices(&self) -> Vec<RateIndex> { self.curves.keys().copied().collect() }

    /// Returns all available rate indices for vol cubes.
    #[must_use]
    pub fn available_volcube_indices(&self) -> Vec<RateIndex> {
        self.volcubes.keys().copied().collect()
    }

    /// Returns all available currency pairs for FX curves.
    #[must_use]
    pub fn available_fx_pairs(&self) -> Vec<CurrencyPair> {
        self.fx_curves.keys().copied().collect()
    }

    /// Returns all available currency pairs for FX vol surfaces.
    #[must_use]
    pub fn available_fx_vol_pairs(&self) -> Vec<CurrencyPair> {
        self.fx_vol_surfaces.keys().copied().collect()
    }
}

// ============================================================================
// IndexedMarketBuilder
// ============================================================================

/// Builder for constructing `IndexedMarket` instances.
///
/// # Examples
///
/// ```ignore
/// use pricer_models::market::IndexedMarketBuilder;
/// use infra_master::{RateIndex, Date};
///
/// let market = IndexedMarketBuilder::new()
///     .valuation_date(Date::from_ymd(2025, 1, 15).unwrap())
///     .with_curve(RateIndex::Sofr, sofr_curve)
///     .with_volcube(RateIndex::Sofr, sofr_volcube)
///     .build()?;
/// ```
pub struct IndexedMarketBuilder<T: Float> {
    valuation_date: Option<Date>,
    curves: HashMap<RateIndex, Arc<dyn YieldCurve<T> + Send + Sync>>,
    volcubes: HashMap<RateIndex, VolCube<T>>,
    fx_curves: HashMap<CurrencyPair, Arc<dyn FxCurve<T> + Send + Sync>>,
    fx_vol_surfaces: HashMap<CurrencyPair, Arc<dyn VolatilitySurface<T> + Send + Sync>>,
    fallback_curve_set: Option<CurveSet<T>>,
    index_mapper: Option<Arc<dyn IndexCurveMapper + Send + Sync>>,
}

impl<T: Float> Default for IndexedMarketBuilder<T> {
    fn default() -> Self {
        Self {
            valuation_date: None,
            curves: HashMap::new(),
            volcubes: HashMap::new(),
            fx_curves: HashMap::new(),
            fx_vol_surfaces: HashMap::new(),
            fallback_curve_set: None,
            index_mapper: None,
        }
    }
}

impl<T: Float> IndexedMarketBuilder<T> {
    /// Creates a new builder.
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Sets the valuation date.
    ///
    /// # Arguments
    ///
    /// * `date` - The valuation date for the market snapshot
    #[must_use]
    pub fn valuation_date(mut self, date: Date) -> Self {
        self.valuation_date = Some(date);
        self
    }

    /// Adds a yield curve for the given rate index.
    ///
    /// # Arguments
    ///
    /// * `index` - The rate index for this curve
    /// * `curve` - The yield curve implementation
    #[must_use]
    pub fn with_curve<C>(mut self, index: RateIndex, curve: C) -> Self
    where
        C: YieldCurve<T> + Send + Sync + 'static,
    {
        self.curves.insert(index, Arc::new(curve));
        self
    }

    /// Adds a yield curve as Arc for the given rate index.
    #[must_use]
    pub fn with_curve_arc(
        mut self,
        index: RateIndex,
        curve: Arc<dyn YieldCurve<T> + Send + Sync>,
    ) -> Self {
        self.curves.insert(index, curve);
        self
    }

    /// Adds a volatility cube for the given rate index.
    #[must_use]
    pub fn with_volcube(mut self, index: RateIndex, volcube: VolCube<T>) -> Self {
        self.volcubes.insert(index, volcube);
        self
    }

    /// Adds an FX curve for the given currency pair.
    #[must_use]
    pub fn with_fx_curve<C>(mut self, pair: CurrencyPair, curve: C) -> Self
    where
        C: FxCurve<T> + Send + Sync + 'static,
    {
        self.fx_curves.insert(pair, Arc::new(curve));
        self
    }

    /// Adds an FX curve as Arc for the given currency pair.
    #[must_use]
    pub fn with_fx_curve_arc(
        mut self,
        pair: CurrencyPair,
        curve: Arc<dyn FxCurve<T> + Send + Sync>,
    ) -> Self {
        self.fx_curves.insert(pair, curve);
        self
    }

    /// Adds an FX volatility surface for the given currency pair.
    #[must_use]
    pub fn with_fx_vol_surface<S>(mut self, pair: CurrencyPair, surface: S) -> Self
    where
        S: VolatilitySurface<T> + Send + Sync + 'static,
    {
        self.fx_vol_surfaces.insert(pair, Arc::new(surface));
        self
    }

    /// Adds an FX volatility surface as Arc for the given currency pair.
    #[must_use]
    pub fn with_fx_vol_surface_arc(
        mut self,
        pair: CurrencyPair,
        surface: Arc<dyn VolatilitySurface<T> + Send + Sync>,
    ) -> Self {
        self.fx_vol_surfaces.insert(pair, surface);
        self
    }

    /// Sets a fallback CurveSet for backward compatibility.
    #[must_use]
    pub fn with_fallback_curve_set(mut self, curve_set: CurveSet<T>) -> Self {
        self.fallback_curve_set = Some(curve_set);
        self
    }

    /// Sets an index mapper for CurveSet fallback.
    #[must_use]
    pub fn with_index_mapper<M>(mut self, mapper: M) -> Self
    where
        M: IndexCurveMapper + Send + Sync + 'static,
    {
        self.index_mapper = Some(Arc::new(mapper));
        self
    }

    /// Builds the `IndexedMarket`.
    ///
    /// # Errors
    ///
    /// Returns `MarketBuildError::InvalidValuationDate` if no valuation date
    /// was set.
    pub fn build(self) -> Result<IndexedMarket<T>, MarketBuildError> {
        let valuation_date =
            self.valuation_date
                .ok_or_else(|| MarketBuildError::InvalidValuationDate {
                    reason: "Valuation date is required".to_string(),
                })?;

        Ok(IndexedMarket {
            valuation_date,
            curves: self.curves,
            volcubes: self.volcubes,
            fx_curves: self.fx_curves,
            fx_vol_surfaces: self.fx_vol_surfaces,
            fallback_curve_set: self.fallback_curve_set,
            index_mapper: self.index_mapper,
        })
    }
}

#[cfg(test)]
mod tests {
    use infra_master::Currency;

    use super::*;
    use crate::market::{curves::FlatCurve, surfaces::FlatVol};

    // ========================================
    // Builder Tests (Task 3)
    // ========================================

    #[test]
    fn test_builder_requires_valuation_date() {
        let result: Result<IndexedMarket<f64>, MarketBuildError> =
            IndexedMarketBuilder::new().build();
        assert!(result.is_err());
        match result.unwrap_err() {
            MarketBuildError::InvalidValuationDate { reason } => {
                assert!(reason.contains("required"));
            }
            _ => panic!("Expected InvalidValuationDate"),
        }
    }

    #[test]
    fn test_builder_with_valuation_date() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .build()
            .unwrap();

        assert_eq!(market.valuation_date(), date);
    }

    #[test]
    fn test_builder_with_curve() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let curve = FlatCurve::new(0.05);

        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, curve)
            .build()
            .unwrap();

        assert!(market.has_curve(RateIndex::Sofr));
        assert!(!market.has_curve(RateIndex::Euribor3M));
    }

    #[test]
    fn test_builder_with_multiple_curves() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();

        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .with_curve(RateIndex::Euribor3M, FlatCurve::new(0.03))
            .with_curve(RateIndex::Sonia, FlatCurve::new(0.04))
            .build()
            .unwrap();

        assert!(market.has_curve(RateIndex::Sofr));
        assert!(market.has_curve(RateIndex::Euribor3M));
        assert!(market.has_curve(RateIndex::Sonia));
        assert!(!market.has_curve(RateIndex::Tonar));
    }

    // ========================================
    // Curve Access Tests (Task 2.2)
    // ========================================

    #[test]
    fn test_curve_access() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .build()
            .unwrap();

        let curve = market.curve(RateIndex::Sofr).unwrap();
        let df = curve.discount_factor(1.0).unwrap();
        assert!((df - (-0.05_f64).exp()).abs() < 1e-10);
    }

    #[test]
    fn test_curve_not_found() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .build()
            .unwrap();

        let result = market.curve(RateIndex::Sofr);
        assert!(result.is_err());
        let err = result.err().unwrap();
        match err {
            MarketDataError::IndexNotFound { index } => {
                assert!(index.contains("Sofr"));
            }
            _ => panic!("Expected IndexNotFound"),
        }
    }

    #[test]
    fn test_discount_factor_convenience() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .build()
            .unwrap();

        let df = market.discount_factor(RateIndex::Sofr, 1.0).unwrap();
        assert!((df - (-0.05_f64).exp()).abs() < 1e-10);
    }

    #[test]
    fn test_forward_rate_convenience() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .build()
            .unwrap();

        let fwd = market.forward_rate(RateIndex::Sofr, 1.0, 2.0).unwrap();
        // For flat curve, forward rate equals flat rate
        assert!((fwd - 0.05).abs() < 1e-10);
    }

    // ========================================
    // FX Access Tests (Task 2.4)
    // ========================================

    #[test]
    fn test_fx_vol_surface_access() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let vol_surface = FlatVol::new(0.15);

        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_fx_vol_surface(pair, vol_surface)
            .build()
            .unwrap();

        assert!(market.has_fx_vol_surface(pair));
        let surface = market.fx_vol_surface(pair).unwrap();
        let vol = surface.volatility(100.0, 1.0).unwrap();
        assert!((vol - 0.15).abs() < 1e-10);
    }

    #[test]
    fn test_fx_vol_surface_not_found() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);

        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .build()
            .unwrap();

        let result = market.fx_vol_surface(pair);
        assert!(result.is_err());
    }

    // ========================================
    // Availability Methods Tests (Task 2.5)
    // ========================================

    #[test]
    fn test_has_curve() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .build()
            .unwrap();

        assert!(market.has_curve(RateIndex::Sofr));
        assert!(!market.has_curve(RateIndex::Euribor3M));
    }

    #[test]
    fn test_available_indices() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .with_curve(RateIndex::Euribor3M, FlatCurve::new(0.03))
            .build()
            .unwrap();

        let indices = market.available_rate_indices();
        assert_eq!(indices.len(), 2);
        assert!(indices.contains(&RateIndex::Sofr));
        assert!(indices.contains(&RateIndex::Euribor3M));
    }

    #[test]
    fn test_available_fx_pairs() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let eurusd = CurrencyPair::new(Currency::EUR, Currency::USD);
        let usdjpy = CurrencyPair::new(Currency::USD, Currency::JPY);

        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_fx_vol_surface(eurusd, FlatVol::new(0.10))
            .with_fx_vol_surface(usdjpy, FlatVol::new(0.08))
            .build()
            .unwrap();

        let pairs = market.available_fx_vol_pairs();
        assert_eq!(pairs.len(), 2);
        assert!(pairs.contains(&eurusd));
        assert!(pairs.contains(&usdjpy));
    }

    // ========================================
    // Empty Market Tests
    // ========================================

    #[test]
    fn test_empty_market() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .build()
            .unwrap();

        assert_eq!(market.valuation_date(), date);
        assert!(market.available_rate_indices().is_empty());
        assert!(market.available_volcube_indices().is_empty());
        assert!(market.available_fx_pairs().is_empty());
        assert!(market.available_fx_vol_pairs().is_empty());
    }
}

// ============================================================================
// Phase 6: Integration Tests (Task 6.1 - 6.3)
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use std::sync::Arc;

    use infra_master::{trade::instrument_def::CurrencyPair, Currency, Date, RateIndex};

    use crate::market::{
        curves::{CurveEnum, CurveName, CurveSet, FlatCurve, YieldCurve},
        fx_calibration::SimpleFxCurve,
        indexed_market::{IndexedMarket, IndexedMarketBuilder},
        surfaces::FlatVol,
        volcube::{VolCubeCache, VolCubeConfig, VolCubeKey, VolInstrument},
        DefaultIndexCurveMapper,
    };

    // ========================================
    // Task 6.1: CurveSet Fallback Integration Tests
    // ========================================

    /// Test that IndexedMarket access produces same results as CurveSet direct access.
    /// Requirements: 4.1, 4.2
    #[test]
    fn test_curveset_indexed_market_result_consistency() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let sofr_rate = 0.05_f64;
        let euribor_rate = 0.03_f64;

        // Create CurveSet for direct access
        let mut curve_set: CurveSet<f64> = CurveSet::new();
        curve_set.insert(CurveName::Sofr, CurveEnum::flat(sofr_rate));
        curve_set.insert(CurveName::Euribor, CurveEnum::flat(euribor_rate));

        // Create IndexedMarket with same curves
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(sofr_rate))
            .with_curve(RateIndex::Euribor3M, FlatCurve::new(euribor_rate))
            .build()
            .unwrap();

        // Verify discount factors match
        let t = 1.0_f64;

        let curveset_sofr = curve_set.get(&CurveName::Sofr).unwrap();
        let curveset_df = curveset_sofr.discount_factor(t).unwrap();

        let indexed_df = market.discount_factor(RateIndex::Sofr, t).unwrap();

        assert!(
            (curveset_df - indexed_df).abs() < 1e-15,
            "CurveSet DF {} != IndexedMarket DF {}",
            curveset_df,
            indexed_df
        );

        // Verify forward rates match
        let t1 = 0.5_f64;
        let t2 = 1.0_f64;

        let curveset_fwd = curveset_sofr.forward_rate(t1, t2).unwrap();
        let indexed_fwd = market.forward_rate(RateIndex::Sofr, t1, t2).unwrap();

        assert!(
            (curveset_fwd - indexed_fwd).abs() < 1e-15,
            "CurveSet fwd {} != IndexedMarket fwd {}",
            curveset_fwd,
            indexed_fwd
        );
    }

    /// Test forward_rate_for_index compatibility with IndexedMarket.
    /// Requirements: 4.1, 4.2
    #[test]
    fn test_forward_rate_for_index_compatibility() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();
        let sofr_rate = 0.045_f64;

        // Create CurveSet with index-based access
        let mut curve_set: CurveSet<f64> = CurveSet::new();
        curve_set.insert(CurveName::Sofr, CurveEnum::flat(sofr_rate));

        // Create IndexedMarket
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(sofr_rate))
            .build()
            .unwrap();

        // Use CurveSet's forward_rate_for_index
        let curveset_fwd = curve_set
            .forward_rate_for_index(RateIndex::Sofr, 1.0, 2.0)
            .unwrap();

        // Use IndexedMarket's forward_rate
        let indexed_fwd = market.forward_rate(RateIndex::Sofr, 1.0, 2.0).unwrap();

        assert!(
            (curveset_fwd - indexed_fwd).abs() < 1e-15,
            "CurveSet forward_rate_for_index {} != IndexedMarket forward_rate {}",
            curveset_fwd,
            indexed_fwd
        );
    }

    /// Test IndexedMarket with fallback CurveSet configured.
    #[test]
    fn test_indexed_market_with_fallback_curveset() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();

        // Create fallback CurveSet
        let mut fallback_set: CurveSet<f64> = CurveSet::new();
        fallback_set.insert(CurveName::Sonia, CurveEnum::flat(0.04));

        // Create IndexedMarket with fallback
        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .with_fallback_curve_set(fallback_set)
            .with_index_mapper(DefaultIndexCurveMapper)
            .build()
            .unwrap();

        // Direct lookup should work
        assert!(market.has_curve(RateIndex::Sofr));
        let df = market.discount_factor(RateIndex::Sofr, 1.0).unwrap();
        assert!((df - (-0.05_f64).exp()).abs() < 1e-10);

        // Note: Fallback is currently disabled in implementation
        // This test documents the builder accepts fallback configuration
    }

    /// Test multi-index access consistency.
    #[test]
    fn test_multi_index_consistency() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();

        // Create multiple curves
        let indices_and_rates = [
            (RateIndex::Sofr, 0.05_f64),
            (RateIndex::Euribor3M, 0.03),
            (RateIndex::Sonia, 0.04),
            (RateIndex::Tonar, 0.001),
            (RateIndex::Estr, 0.035),
        ];

        let mut builder = IndexedMarketBuilder::new().valuation_date(date);
        for (index, rate) in &indices_and_rates {
            builder = builder.with_curve(*index, FlatCurve::new(*rate));
        }
        let market: IndexedMarket<f64> = builder.build().unwrap();

        // Verify all curves accessible and correct
        for (index, expected_rate) in &indices_and_rates {
            assert!(market.has_curve(*index), "Missing curve for {:?}", index);

            let fwd = market.forward_rate(*index, 1.0, 2.0).unwrap();
            assert!(
                (fwd - expected_rate).abs() < 1e-10,
                "Rate mismatch for {:?}: expected {}, got {}",
                index,
                expected_rate,
                fwd
            );
        }
    }

    // ========================================
    // Task 6.2: VolCubeCache Integration Tests
    // ========================================

    /// Test VolCubeCache lookup and insert operations.
    /// Requirements: 4.2
    #[test]
    fn test_volcube_cache_basic_operations() {
        let cache: VolCubeCache<String> = VolCubeCache::new(10);

        // Create key from instruments
        let instruments = vec![
            VolInstrument::new("INST-1", 1.0_f64, 5.0, 0.03, 0.20, 0.03),
            VolInstrument::new("INST-2", 2.0_f64, 5.0, 0.03, 0.22, 0.03),
        ];
        let config = VolCubeConfig::default();
        let key = VolCubeKey::from_instruments(&instruments, &config);

        // Insert
        cache.insert(key.clone(), "calibrated_volcube".to_string());

        // Lookup (cache hit)
        let result = cache.lookup(&key);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "calibrated_volcube");

        // Stats should show hit
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }

    /// Test cache miss detection.
    #[test]
    fn test_volcube_cache_miss() {
        let cache: VolCubeCache<String> = VolCubeCache::new(10);

        let instruments = vec![VolInstrument::new("INST-1", 1.0_f64, 5.0, 0.03, 0.20, 0.03)];
        let config = VolCubeConfig::default();
        let key = VolCubeKey::from_instruments(&instruments, &config);

        // Lookup without insert (cache miss)
        let result = cache.lookup(&key);
        assert!(result.is_none());

        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }

    /// Test cache hit rate calculation.
    #[test]
    fn test_volcube_cache_hit_rate() {
        let cache: VolCubeCache<i32> = VolCubeCache::new(10);

        // Insert one entry
        let key1 = VolCubeKey::new(1, 1);
        cache.insert(key1.clone(), 100);

        // 2 hits
        let _ = cache.lookup(&key1);
        let _ = cache.lookup(&key1);

        // 2 misses
        let key2 = VolCubeKey::new(999, 999);
        let _ = cache.lookup(&key2);
        let _ = cache.lookup(&key2);

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 2);
        assert!((stats.hit_rate() - 0.5).abs() < 1e-10);
    }

    /// Test cache LRU eviction behavior.
    #[test]
    fn test_volcube_cache_lru_eviction() {
        let cache: VolCubeCache<i32> = VolCubeCache::new(3);

        // Fill cache
        for i in 0..3 {
            let key = VolCubeKey::new(i, 0);
            cache.insert(key, i as i32);
        }
        assert_eq!(cache.len(), 3);

        // Access key 0 to make it recently used
        let _ = cache.lookup(&VolCubeKey::new(0, 0));

        // Insert 4th item, should evict least recently used (key 1)
        let key4 = VolCubeKey::new(100, 0);
        cache.insert(key4.clone(), 100);

        assert_eq!(cache.len(), 3);
        // Key 0 and 2 should still be present (0 was accessed, 2 was added after 1)
        assert!(cache.contains(&VolCubeKey::new(0, 0)));
        assert!(cache.contains(&key4));
    }

    // ========================================
    // Task 6.3: MarketProvider FxCurve Integration Tests
    // ========================================

    /// Test FX curve access via CurrencyPair.
    /// Requirements: 4.3, 4.4
    #[test]
    fn test_fx_curve_currency_pair_access() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();

        let eurusd = CurrencyPair::new(Currency::EUR, Currency::USD);
        let usd_curve = Arc::new(FlatCurve::new(0.05_f64));
        let eur_curve = Arc::new(FlatCurve::new(0.03_f64));

        let fx_curve = SimpleFxCurve::new(eurusd, 1.10, usd_curve, eur_curve);

        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_fx_curve(eurusd, fx_curve)
            .build()
            .unwrap();

        // Access by CurrencyPair
        assert!(market.has_fx_curve(eurusd));
        let curve = market.fx_curve(eurusd).unwrap();

        // Verify spot rate
        let spot = curve.spot_rate();
        assert!((spot - 1.10).abs() < 1e-10);

        // Verify forward rate
        let fwd = curve.forward_rate(1.0).unwrap();
        // Covered interest rate parity: F = S * DF_f / DF_d
        // F = 1.10 * exp(-0.03) / exp(-0.05) = 1.10 * exp(0.02)
        let expected_fwd = 1.10 * (0.02_f64).exp();
        assert!(
            (fwd - expected_fwd).abs() < 1e-6,
            "Forward rate mismatch: {} vs {}",
            fwd,
            expected_fwd
        );
    }

    /// Test multiple FX pairs in same market.
    #[test]
    fn test_multiple_fx_pairs() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();

        let eurusd = CurrencyPair::new(Currency::EUR, Currency::USD);
        let usdjpy = CurrencyPair::new(Currency::USD, Currency::JPY);
        let gbpusd = CurrencyPair::new(Currency::GBP, Currency::USD);

        let base_curve = Arc::new(FlatCurve::new(0.05_f64));

        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_fx_curve(
                eurusd,
                SimpleFxCurve::new(eurusd, 1.10, base_curve.clone(), base_curve.clone()),
            )
            .with_fx_curve(
                usdjpy,
                SimpleFxCurve::new(usdjpy, 150.0, base_curve.clone(), base_curve.clone()),
            )
            .with_fx_curve(
                gbpusd,
                SimpleFxCurve::new(gbpusd, 1.27, base_curve.clone(), base_curve.clone()),
            )
            .build()
            .unwrap();

        // Verify all pairs accessible
        assert!(market.has_fx_curve(eurusd));
        assert!(market.has_fx_curve(usdjpy));
        assert!(market.has_fx_curve(gbpusd));

        // Verify different spot rates
        assert!((market.fx_curve(eurusd).unwrap().spot_rate() - 1.10).abs() < 1e-10);
        assert!((market.fx_curve(usdjpy).unwrap().spot_rate() - 150.0).abs() < 1e-10);
        assert!((market.fx_curve(gbpusd).unwrap().spot_rate() - 1.27).abs() < 1e-10);
    }

    /// Test FX vol surface access via CurrencyPair.
    #[test]
    fn test_fx_vol_surface_currency_pair_access() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();

        let eurusd = CurrencyPair::new(Currency::EUR, Currency::USD);
        let usdjpy = CurrencyPair::new(Currency::USD, Currency::JPY);

        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_fx_vol_surface(eurusd, FlatVol::new(0.10))
            .with_fx_vol_surface(usdjpy, FlatVol::new(0.08))
            .build()
            .unwrap();

        // Verify vol surface access
        let eurusd_vol = market.fx_vol_surface(eurusd).unwrap();
        let usdjpy_vol = market.fx_vol_surface(usdjpy).unwrap();

        // Verify different vols
        assert!((eurusd_vol.volatility(100.0, 1.0).unwrap() - 0.10).abs() < 1e-10);
        assert!((usdjpy_vol.volatility(150.0, 1.0).unwrap() - 0.08).abs() < 1e-10);
    }

    /// Test FX curve not found error.
    #[test]
    fn test_fx_curve_not_found() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();

        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .build()
            .unwrap();

        let eurusd = CurrencyPair::new(Currency::EUR, Currency::USD);
        let result = market.fx_curve(eurusd);

        assert!(result.is_err());
        match result.err().unwrap() {
            crate::market::MarketDataError::IndexNotFound { index } => {
                assert!(index.contains("EUR/USD"));
            }
            _ => panic!("Expected IndexNotFound error"),
        }
    }

    /// Test available FX pairs enumeration.
    #[test]
    fn test_available_fx_pairs_consistency() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();

        let pairs = [
            CurrencyPair::new(Currency::EUR, Currency::USD),
            CurrencyPair::new(Currency::USD, Currency::JPY),
            CurrencyPair::new(Currency::GBP, Currency::USD),
        ];

        let base_curve = Arc::new(FlatCurve::new(0.05_f64));

        let mut builder = IndexedMarketBuilder::new().valuation_date(date);
        for pair in &pairs {
            builder = builder.with_fx_curve(
                *pair,
                SimpleFxCurve::new(*pair, 1.0, base_curve.clone(), base_curve.clone()),
            );
        }
        let market: IndexedMarket<f64> = builder.build().unwrap();

        let available = market.available_fx_pairs();
        assert_eq!(available.len(), pairs.len());

        for pair in &pairs {
            assert!(available.contains(pair), "Missing pair: {}", pair);
        }
    }

    // ========================================
    // Combined Integration Tests
    // ========================================

    /// Test complete market with all data types.
    #[test]
    fn test_complete_market_integration() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();

        let eurusd = CurrencyPair::new(Currency::EUR, Currency::USD);
        let base_curve = Arc::new(FlatCurve::new(0.05_f64));

        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            // Rate curves
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .with_curve(RateIndex::Euribor3M, FlatCurve::new(0.03))
            // FX curves
            .with_fx_curve(
                eurusd,
                SimpleFxCurve::new(eurusd, 1.10, base_curve.clone(), base_curve),
            )
            // FX vol surfaces
            .with_fx_vol_surface(eurusd, FlatVol::new(0.10))
            .build()
            .unwrap();

        // Verify all components
        assert!(market.has_curve(RateIndex::Sofr));
        assert!(market.has_curve(RateIndex::Euribor3M));
        assert!(market.has_fx_curve(eurusd));
        assert!(market.has_fx_vol_surface(eurusd));

        // Verify valuation date
        assert_eq!(market.valuation_date(), date);

        // Verify data consistency
        let df = market.discount_factor(RateIndex::Sofr, 1.0).unwrap();
        assert!((df - (-0.05_f64).exp()).abs() < 1e-10);

        let spot = market.fx_curve(eurusd).unwrap().spot_rate();
        assert!((spot - 1.10).abs() < 1e-10);

        let vol = market.fx_vol_surface(eurusd).unwrap().volatility(100.0, 1.0).unwrap();
        assert!((vol - 0.10).abs() < 1e-10);
    }

    /// Test market with Debug formatting.
    #[test]
    fn test_market_debug_output() {
        let date = Date::from_ymd(2025, 1, 15).unwrap();

        let market: IndexedMarket<f64> = IndexedMarketBuilder::new()
            .valuation_date(date)
            .with_curve(RateIndex::Sofr, FlatCurve::new(0.05))
            .with_curve(RateIndex::Euribor3M, FlatCurve::new(0.03))
            .build()
            .unwrap();

        let debug_str = format!("{:?}", market);
        assert!(debug_str.contains("IndexedMarket"));
        assert!(debug_str.contains("curves_count"));
        assert!(debug_str.contains("2")); // 2 curves
    }
}
