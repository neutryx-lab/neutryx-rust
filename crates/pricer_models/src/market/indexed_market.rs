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
#[derive(Debug)]
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
    fallback_curve_set: Option<CurveSet<T>>,

    /// Optional index mapper for CurveSet fallback.
    index_mapper: Option<Arc<dyn IndexCurveMapper + Send + Sync>>,
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

        // Try fallback via CurveSet + IndexMapper
        if let (Some(curve_set), Some(mapper)) = (&self.fallback_curve_set, &self.index_mapper) {
            let curve_name = mapper.map_to_curve(index)?;
            if let Some(curve) = curve_set.get_curve(curve_name) {
                return Ok(curve);
            }
        }

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
        self.volcubes.get(&index).ok_or(MarketDataError::IndexNotFound {
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
    ///
    /// Checks both direct storage and fallback CurveSet.
    #[must_use]
    pub fn has_curve(&self, index: RateIndex) -> bool {
        if self.curves.contains_key(&index) {
            return true;
        }

        // Check fallback
        if let (Some(curve_set), Some(mapper)) = (&self.fallback_curve_set, &self.index_mapper) {
            if let Ok(curve_name) = mapper.map_to_curve(index) {
                return curve_set.get_curve(curve_name).is_some();
            }
        }

        false
    }

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
    pub fn available_fx_pairs(&self) -> Vec<CurrencyPair> { self.fx_curves.keys().copied().collect() }

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
#[derive(Default)]
pub struct IndexedMarketBuilder<T: Float> {
    valuation_date: Option<Date>,
    curves: HashMap<RateIndex, Arc<dyn YieldCurve<T> + Send + Sync>>,
    volcubes: HashMap<RateIndex, VolCube<T>>,
    fx_curves: HashMap<CurrencyPair, Arc<dyn FxCurve<T> + Send + Sync>>,
    fx_vol_surfaces: HashMap<CurrencyPair, Arc<dyn VolatilitySurface<T> + Send + Sync>>,
    fallback_curve_set: Option<CurveSet<T>>,
    index_mapper: Option<Arc<dyn IndexCurveMapper + Send + Sync>>,
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
    use super::*;
    use crate::market::curves::FlatCurve;
    use crate::market::surfaces::FlatVol;
    use infra_master::Currency;

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
        match result.unwrap_err() {
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
