//! Unified market environment for pricing operations.
//!
//! [`MarketEnvironment`] aggregates all market data (discount curves, forward
//! curves, FX spots, FX forward curves, and volatility surfaces) required by a
//! pricing kernel into a single, immutable snapshot.
//!
//! # Design
//!
//! * **Keyed by domain types** — discount curves by [`Currency`], forward
//!   curves by [`CurveName`], FX data by [`CurrencyPair`].
//! * **Enum dispatch** — uses [`CurveEnum`] and [`FxCurveEnum`] for zero-cost
//!   polymorphism, keeping the structure Enzyme-friendly.
//! * **Provider-compatible** — exposes accessor methods whose signatures mirror
//!   `CurveProvider` / `SpotProvider` in pricer\_pricing, allowing a thin
//!   adapter in that crate without introducing a reverse dependency.
//!
//! # Example
//!
//! ```
//! use pricer_models::market_env::MarketEnvironmentBuilder;
//! use pricer_models::market::{CurveEnum, CurveName};
//! use infra_domain::market::Currency;
//! use infra_domain::time::Date;
//!
//! let env = MarketEnvironmentBuilder::new(Date::from_ymd(2024, 6, 15).unwrap())
//!     .with_discount_curve(Currency::USD, CurveEnum::flat(0.05))
//!     .with_forward_curve(CurveName::Sofr, CurveEnum::flat(0.045))
//!     .build();
//!
//! assert_eq!(env.valuation_date(), Date::from_ymd(2024, 6, 15).unwrap());
//! assert!(env.discount_curve(&Currency::USD).is_some());
//! ```

use std::collections::HashMap;

use infra_domain::{
    market::{Currency, CurrencyPair},
    time::Date,
};

use crate::{
    market::{
        curves::YieldCurve, fx_curves::FxCurve, CurveEnum, CurveName, FxCurveEnum, MarketDataError,
    },
    vol_surface::{VolSurface, VolSurfaceEnum},
};

// ---------------------------------------------------------------------------
// MarketEnvironment
// ---------------------------------------------------------------------------

/// Immutable snapshot of all market data required for pricing.
///
/// Constructed via [`MarketEnvironmentBuilder`].
#[derive(Debug, Clone)]
pub struct MarketEnvironment {
    valuation_date: Date,
    discount_curves: HashMap<Currency, CurveEnum<f64>>,
    forward_curves: HashMap<CurveName, CurveEnum<f64>>,
    fx_spots: HashMap<CurrencyPair, f64>,
    fx_curves: HashMap<CurrencyPair, FxCurveEnum<f64>>,
    vol_surfaces: HashMap<String, VolSurfaceEnum<f64>>,
    /// Generic spot prices keyed by identifier (e.g. equity ticker, commodity
    /// code).
    spot_prices: HashMap<String, f64>,
}

impl MarketEnvironment {
    /// Returns the valuation date for this environment.
    pub fn valuation_date(&self) -> Date { self.valuation_date }

    // -- Discount curves ---------------------------------------------------

    /// Returns a reference to the discount curve for the given currency, if
    /// present.
    pub fn discount_curve(&self, currency: &Currency) -> Option<&CurveEnum<f64>> {
        self.discount_curves.get(currency)
    }

    /// Returns the discount factor for `currency` at year-fraction `t`.
    ///
    /// This mirrors `CurveProvider::discount_factor` semantics for use by a
    /// thin adapter in `pricer_pricing`.
    pub fn discount_factor(&self, currency: &Currency, t: f64) -> Result<f64, MarketDataError> {
        self.discount_curves
            .get(currency)
            .ok_or_else(|| MarketDataError::CurveNotFound {
                name: format!("discount:{}", currency),
            })?
            .discount_factor(t)
    }

    // -- Forward curves ----------------------------------------------------

    /// Returns a reference to the forward curve for the given name, if
    /// present.
    pub fn forward_curve(&self, name: &CurveName) -> Option<&CurveEnum<f64>> {
        self.forward_curves.get(name)
    }

    /// Returns the forward rate on the named curve between `t1` and `t2`.
    ///
    /// Mirrors `CurveProvider::forward_rate` semantics.
    pub fn forward_rate(&self, name: &CurveName, t1: f64, t2: f64) -> Result<f64, MarketDataError> {
        self.forward_curves
            .get(name)
            .ok_or_else(|| MarketDataError::CurveNotFound {
                name: format!("forward:{:?}", name),
            })?
            .forward_rate(t1, t2)
    }

    // -- FX -----------------------------------------------------------------

    /// Returns the spot FX rate for the given currency pair, if present.
    ///
    /// Supports triangulation via the inverse pair: if the direct pair is
    /// absent but the inverse is stored, the reciprocal is returned.
    pub fn fx_rate(&self, from: Currency, to: Currency) -> Option<f64> {
        if from == to {
            return Some(1.0);
        }
        let pair = CurrencyPair::new(from, to);
        if let Some(&rate) = self.fx_spots.get(&pair) {
            return Some(rate);
        }
        // Try inverse pair.
        let inverse = pair.inverse();
        self.fx_spots.get(&inverse).map(|&rate| 1.0 / rate)
    }

    /// Returns a reference to the FX forward curve for the given pair, if
    /// present.
    pub fn fx_curve(&self, pair: &CurrencyPair) -> Option<&FxCurveEnum<f64>> {
        self.fx_curves.get(pair)
    }

    /// Returns the FX forward rate for `pair` at year-fraction `t`.
    pub fn fx_forward(&self, pair: &CurrencyPair, t: f64) -> Result<f64, MarketDataError> {
        self.fx_curves
            .get(pair)
            .ok_or_else(|| MarketDataError::CurveNotFound {
                name: format!("fx:{}", pair),
            })?
            .forward_rate(t)
    }

    // -- Vol surfaces ------------------------------------------------------

    /// Returns a reference to the volatility surface keyed by `key`, if
    /// present.
    pub fn vol_surface(&self, key: &str) -> Option<&VolSurfaceEnum<f64>> {
        self.vol_surfaces.get(key)
    }

    /// Returns the implied volatility from the named surface.
    pub fn implied_vol(
        &self,
        key: &str,
        strike: f64,
        expiry: f64,
        forward: f64,
    ) -> Result<f64, MarketDataError> {
        let surface = self
            .vol_surfaces
            .get(key)
            .ok_or_else(|| MarketDataError::CurveNotFound {
                name: format!("vol:{}", key),
            })?;
        surface.implied_vol(strike, expiry, forward).map_err(|e| {
            MarketDataError::InterpolationFailed {
                reason: e.to_string(),
            }
        })
    }

    // -- Collection accessors ----------------------------------------------

    /// Returns a reference to all discount curves.
    pub fn discount_curves(&self) -> &HashMap<Currency, CurveEnum<f64>> { &self.discount_curves }

    /// Returns a reference to all forward curves.
    pub fn forward_curves(&self) -> &HashMap<CurveName, CurveEnum<f64>> { &self.forward_curves }

    /// Returns a reference to all FX spot rates.
    pub fn fx_spots(&self) -> &HashMap<CurrencyPair, f64> { &self.fx_spots }

    /// Returns a reference to all FX forward curves.
    pub fn fx_curves_map(&self) -> &HashMap<CurrencyPair, FxCurveEnum<f64>> { &self.fx_curves }

    /// Returns a reference to all volatility surfaces.
    pub fn vol_surfaces(&self) -> &HashMap<String, VolSurfaceEnum<f64>> { &self.vol_surfaces }

    // -- Spot prices -------------------------------------------------------

    /// Returns the spot price for the given key (e.g. equity ticker), if
    /// present.
    pub fn spot_price(&self, key: &str) -> Option<f64> {
        self.spot_prices.get(key).copied()
    }

    /// Returns a reference to all spot prices.
    pub fn spot_prices(&self) -> &HashMap<String, f64> { &self.spot_prices }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Fluent builder for [`MarketEnvironment`].
///
/// Only `valuation_date` is mandatory; all other fields default to empty
/// collections.
///
/// # Example
///
/// ```
/// use pricer_models::market_env::MarketEnvironmentBuilder;
/// use pricer_models::market::CurveEnum;
/// use infra_domain::market::Currency;
/// use infra_domain::time::Date;
///
/// let env = MarketEnvironmentBuilder::new(Date::from_ymd(2024, 1, 1).unwrap())
///     .with_discount_curve(Currency::USD, CurveEnum::flat(0.05))
///     .with_discount_curve(Currency::EUR, CurveEnum::flat(0.03))
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct MarketEnvironmentBuilder {
    valuation_date: Date,
    discount_curves: HashMap<Currency, CurveEnum<f64>>,
    forward_curves: HashMap<CurveName, CurveEnum<f64>>,
    fx_spots: HashMap<CurrencyPair, f64>,
    fx_curves: HashMap<CurrencyPair, FxCurveEnum<f64>>,
    vol_surfaces: HashMap<String, VolSurfaceEnum<f64>>,
    spot_prices: HashMap<String, f64>,
}

impl MarketEnvironmentBuilder {
    /// Creates a new builder with the given valuation date.
    #[must_use]
    pub fn new(valuation_date: Date) -> Self {
        Self {
            valuation_date,
            discount_curves: HashMap::new(),
            forward_curves: HashMap::new(),
            fx_spots: HashMap::new(),
            fx_curves: HashMap::new(),
            vol_surfaces: HashMap::new(),
            spot_prices: HashMap::new(),
        }
    }

    /// Adds a discount curve for the given currency.
    #[must_use]
    pub fn with_discount_curve(mut self, currency: Currency, curve: CurveEnum<f64>) -> Self {
        self.discount_curves.insert(currency, curve);
        self
    }

    /// Adds a forward curve with the given name.
    #[must_use]
    pub fn with_forward_curve(mut self, name: CurveName, curve: CurveEnum<f64>) -> Self {
        self.forward_curves.insert(name, curve);
        self
    }

    /// Adds an FX spot rate.
    #[must_use]
    pub fn with_fx_spot(mut self, pair: CurrencyPair, rate: f64) -> Self {
        self.fx_spots.insert(pair, rate);
        self
    }

    /// Adds an FX forward curve.
    #[must_use]
    pub fn with_fx_curve(mut self, pair: CurrencyPair, curve: FxCurveEnum<f64>) -> Self {
        self.fx_curves.insert(pair, curve);
        self
    }

    /// Adds a volatility surface with the given key.
    #[must_use]
    pub fn with_vol_surface(
        mut self,
        key: impl Into<String>,
        surface: VolSurfaceEnum<f64>,
    ) -> Self {
        self.vol_surfaces.insert(key.into(), surface);
        self
    }

    /// Adds a generic spot price (equity ticker, commodity code, etc.).
    #[must_use]
    pub fn with_spot_price(mut self, key: impl Into<String>, price: f64) -> Self {
        self.spot_prices.insert(key.into(), price);
        self
    }

    /// Consumes the builder and produces a [`MarketEnvironment`].
    #[must_use]
    pub fn build(self) -> MarketEnvironment {
        MarketEnvironment {
            valuation_date: self.valuation_date,
            discount_curves: self.discount_curves,
            forward_curves: self.forward_curves,
            fx_spots: self.fx_spots,
            fx_curves: self.fx_curves,
            vol_surfaces: self.vol_surfaces,
            spot_prices: self.spot_prices,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_date() -> Date { Date::from_ymd(2024, 6, 15).unwrap() }

    #[test]
    fn test_empty_environment() {
        let env = MarketEnvironmentBuilder::new(sample_date()).build();
        assert_eq!(env.valuation_date(), sample_date());
        assert!(env.discount_curves().is_empty());
        assert!(env.forward_curves().is_empty());
        assert!(env.fx_spots().is_empty());
        assert!(env.vol_surfaces().is_empty());
    }

    #[test]
    fn test_discount_curve_lookup() {
        let env = MarketEnvironmentBuilder::new(sample_date())
            .with_discount_curve(Currency::USD, CurveEnum::flat(0.05))
            .build();

        assert!(env.discount_curve(&Currency::USD).is_some());
        assert!(env.discount_curve(&Currency::EUR).is_none());

        let df = env.discount_factor(&Currency::USD, 1.0).unwrap();
        let expected = (-0.05_f64).exp();
        assert!((df - expected).abs() < 1e-10);
    }

    #[test]
    fn test_discount_curve_not_found() {
        let env = MarketEnvironmentBuilder::new(sample_date()).build();
        let result = env.discount_factor(&Currency::GBP, 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_forward_curve_lookup() {
        let env = MarketEnvironmentBuilder::new(sample_date())
            .with_forward_curve(CurveName::Sofr, CurveEnum::flat(0.045))
            .build();

        assert!(env.forward_curve(&CurveName::Sofr).is_some());
        assert!(env.forward_curve(&CurveName::Euribor).is_none());

        let fwd = env.forward_rate(&CurveName::Sofr, 0.5, 1.0).unwrap();
        assert!(fwd.is_finite());
    }

    #[test]
    fn test_fx_spot_direct() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let env = MarketEnvironmentBuilder::new(sample_date())
            .with_fx_spot(pair, 1.10)
            .build();

        assert_eq!(env.fx_rate(Currency::EUR, Currency::USD), Some(1.10));
    }

    #[test]
    fn test_fx_spot_inverse() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let env = MarketEnvironmentBuilder::new(sample_date())
            .with_fx_spot(pair, 1.10)
            .build();

        let inverse = env.fx_rate(Currency::USD, Currency::EUR).unwrap();
        assert!((inverse - 1.0 / 1.10).abs() < 1e-10);
    }

    #[test]
    fn test_fx_same_currency() {
        let env = MarketEnvironmentBuilder::new(sample_date()).build();
        assert_eq!(env.fx_rate(Currency::USD, Currency::USD), Some(1.0));
    }

    #[test]
    fn test_fx_not_found() {
        let env = MarketEnvironmentBuilder::new(sample_date()).build();
        assert!(env.fx_rate(Currency::EUR, Currency::JPY).is_none());
    }

    #[test]
    fn test_vol_surface_lookup() {
        let surface = VolSurfaceEnum::<f64>::flat(0.20).unwrap();
        let env = MarketEnvironmentBuilder::new(sample_date())
            .with_vol_surface("EURUSD", surface)
            .build();

        assert!(env.vol_surface("EURUSD").is_some());
        assert!(env.vol_surface("USDJPY").is_none());

        let vol = env.implied_vol("EURUSD", 100.0, 1.0, 100.0).unwrap();
        assert!((vol - 0.20).abs() < 1e-10);
    }

    #[test]
    fn test_builder_chaining() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let surface = VolSurfaceEnum::<f64>::flat(0.18).unwrap();

        let env = MarketEnvironmentBuilder::new(sample_date())
            .with_discount_curve(Currency::USD, CurveEnum::flat(0.05))
            .with_discount_curve(Currency::EUR, CurveEnum::flat(0.03))
            .with_forward_curve(CurveName::Sofr, CurveEnum::flat(0.045))
            .with_forward_curve(CurveName::Estr, CurveEnum::flat(0.025))
            .with_fx_spot(pair, 1.10)
            .with_vol_surface("EURUSD", surface)
            .build();

        assert_eq!(env.discount_curves().len(), 2);
        assert_eq!(env.forward_curves().len(), 2);
        assert_eq!(env.fx_spots().len(), 1);
        assert_eq!(env.vol_surfaces().len(), 1);
    }

    #[test]
    fn test_fx_curve_forward() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let fx_curve = FxCurveEnum::irp_flat(1.10, 0.05, 0.03, pair);

        let env = MarketEnvironmentBuilder::new(sample_date())
            .with_fx_curve(pair, fx_curve)
            .build();

        assert!(env.fx_curve(&pair).is_some());

        let fwd = env.fx_forward(&pair, 1.0).unwrap();
        let expected = 1.10 * (0.02_f64).exp();
        assert!((fwd - expected).abs() < 1e-8);
    }
}
