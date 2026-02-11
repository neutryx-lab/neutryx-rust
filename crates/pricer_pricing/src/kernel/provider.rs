//! Market data provider traits for pricing kernels.
//!
//! This module defines the `CurveProvider` trait that abstracts market data
//! access for the pricing kernel. Implementations provide discount factors
//! and forward rates based on curve IDs and dates.
//!
//! # Providers
//!
//! - [`FlatCurveProvider`]: Simple flat rate provider for testing
//! - [`MarketProvider`]: Reference implementation using `IndexedMarket`

use std::sync::Arc;

use pricer_models::market::curves::YieldCurve;

/// Trait for providing market data to pricing kernels.
///
/// `CurveProvider` abstracts the market data access layer, allowing
/// the pricing engine to be independent of specific curve implementations.
///
/// # ID Conventions
///
/// - `curve_id = 0`: Default discounting curve
/// - `fwd_index_id = 0`: Dummy forward returning 0.0 (for fixed legs)
/// - `fx_id = 0`: Dummy FX rate returning 1.0 (single currency)
///
/// # Examples
///
/// ```ignore
/// use pricer_pricing::kernel::CurveProvider;
///
/// struct MyCurveProvider { /* curves */ }
///
/// impl CurveProvider for MyCurveProvider {
///     fn discount_factor(&self, curve_id: u8, days_from_epoch: i32) -> f64 {
///         // Look up curve and calculate discount factor
///         todo!()
///     }
///
///     fn forward_rate(&self, fwd_index_id: u16, fixing_days: i32, tenor_days: i32) -> f64 {
///         // Look up forward curve and calculate rate
///         todo!()
///     }
///
///     fn fx_rate(&self, fx_id: u16) -> f64 {
///         if fx_id == 0 { 1.0 } else { /* look up */ todo!() }
///     }
/// }
/// ```
pub trait CurveProvider {
    /// Returns the discount factor for a given curve and date.
    ///
    /// # Arguments
    ///
    /// * `curve_id` - Discount curve ID (from
    ///   `PricingKernel::discount_curve_ids`)
    /// * `days_from_epoch` - Payment date as days from epoch
    ///
    /// # Returns
    ///
    /// Discount factor DF(t) where t is the time to payment date.
    /// Should return 1.0 for today (days = 0).
    fn discount_factor(&self, curve_id: u8, days_from_epoch: i32) -> f64;

    /// Returns the forward rate for a given index and fixing date.
    ///
    /// # Arguments
    ///
    /// * `fwd_index_id` - Forward index ID (from
    ///   `PricingKernel::fwd_index_ids`)
    ///   - 0 = dummy index returning 0.0
    /// * `fixing_days` - Fixing date as days from epoch
    /// * `tenor_days` - Tenor in days (e.g., 90 for 3M)
    ///
    /// # Returns
    ///
    /// Forward rate L(t_fix, t_fix + tenor) as a decimal (e.g., 0.05 for 5%).
    /// Returns 0.0 for `fwd_index_id == 0` (dummy for fixed legs).
    fn forward_rate(&self, fwd_index_id: u16, fixing_days: i32, tenor_days: i32) -> f64;

    /// Returns the FX rate for currency conversion.
    ///
    /// # Arguments
    ///
    /// * `fx_id` - FX index ID (from `PricingKernel::fx_index_ids`)
    ///   - 0 = dummy FX returning 1.0
    ///
    /// # Returns
    ///
    /// FX rate for converting cashflow to base currency.
    /// Returns 1.0 for `fx_id == 0` (single currency, no conversion).
    fn fx_rate(&self, fx_id: u16) -> f64;

    /// Returns the current valuation date as days from epoch.
    ///
    /// Used to determine which cashflows are in the future.
    fn valuation_date_days(&self) -> i32;
}

/// A simple flat curve provider for testing and demonstration.
///
/// All discount factors are computed from a single flat rate,
/// and all forward rates return the same value.
///
/// # Examples
///
/// ```
/// # {
/// use pricer_pricing::kernel::{CurveProvider, FlatCurveProvider};
///
/// // Create provider with 5% discount rate and 3% forward rate
/// let provider = FlatCurveProvider::new(0.05, 0.03);
///
/// // Discount factor for 1 year (365 days) from today
/// let df = provider.discount_factor(0, 365);
/// assert!((df - 0.9512).abs() < 0.01); // exp(-0.05 * 1)
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct FlatCurveProvider {
    /// Flat continuously compounded discount rate.
    discount_rate: f64,
    /// Flat forward rate.
    forward_rate: f64,
    /// Valuation date as days from epoch.
    valuation_date_days: i32,
}

impl FlatCurveProvider {
    /// Creates a new flat curve provider.
    ///
    /// # Arguments
    ///
    /// * `discount_rate` - Continuous compound discount rate (e.g., 0.05 for
    ///   5%)
    /// * `forward_rate` - Flat forward rate (e.g., 0.03 for 3%)
    #[must_use]
    pub fn new(discount_rate: f64, forward_rate: f64) -> Self {
        Self {
            discount_rate,
            forward_rate,
            valuation_date_days: 0,
        }
    }

    /// Creates a provider with a specific valuation date.
    #[must_use]
    pub fn with_valuation_date(mut self, valuation_date_days: i32) -> Self {
        self.valuation_date_days = valuation_date_days;
        self
    }

    /// Sets the valuation date.
    pub fn set_valuation_date(&mut self, valuation_date_days: i32) {
        self.valuation_date_days = valuation_date_days;
    }
}

impl CurveProvider for FlatCurveProvider {
    fn discount_factor(&self, _curve_id: u8, days_from_epoch: i32) -> f64 {
        // Calculate time in years from valuation date
        let days_to_payment = days_from_epoch - self.valuation_date_days;
        if days_to_payment <= 0 {
            return 1.0;
        }

        let t = days_to_payment as f64 / 365.0;
        (-self.discount_rate * t).exp()
    }

    fn forward_rate(&self, fwd_index_id: u16, _fixing_days: i32, _tenor_days: i32) -> f64 {
        // ID 0 is dummy (fixed leg)
        if fwd_index_id == 0 {
            0.0
        } else {
            self.forward_rate
        }
    }

    fn fx_rate(&self, _fx_id: u16) -> f64 {
        // Flat provider assumes single currency, always return 1.0
        1.0
    }

    fn valuation_date_days(&self) -> i32 { self.valuation_date_days }
}

// =============================================================================
// IndexedMarketAdapter
// =============================================================================

/// Adapter that provides `CurveProvider` interface for `IndexedMarket`.
///
/// This struct bridges the `IndexedMarket<f64>` API (which uses `RateIndex`
/// keys) to the `CurveProvider` trait (which uses numeric IDs). It stores
/// arrays of curves indexed by their numeric IDs.
///
/// # ID Mapping
///
/// The adapter requires a pre-configured mapping from:
/// - `curve_id` → Discount curve
/// - `fwd_index_id` → Forward rate curve
/// - `fx_id` → FX rate
///
/// ID 0 is reserved for dummy values (0.0 for forwards, 1.0 for FX).
///
/// # Example
///
/// ```ignore
/// use pricer_pricing::kernel::{IndexedMarketAdapter, IndexedMarketAdapterBuilder};
/// use pricer_models::market::{IndexedMarket, FlatCurve};
/// use infra_domain::market::RateIndex;
///
/// let market = /* construct IndexedMarket<f64> */;
/// let adapter = IndexedMarketAdapterBuilder::new()
///     .valuation_date_days(18262)
///     .add_discount_curve(0, market.curve(RateIndex::Sofr).unwrap())
///     .add_forward_curve(1, market.curve(RateIndex::Sofr).unwrap())
///     .build();
/// ```
pub struct IndexedMarketAdapter {
    /// Discount curves indexed by curve_id.
    discount_curves: Vec<Option<Arc<dyn YieldCurve<f64> + Send + Sync>>>,
    /// Forward curves indexed by fwd_index_id.
    forward_curves: Vec<Option<Arc<dyn YieldCurve<f64> + Send + Sync>>>,
    /// FX spot rates indexed by fx_id.
    fx_rates: Vec<f64>,
    /// Valuation date as days from epoch.
    valuation_date_days: i32,
    /// Default tenor in days for forward rate calculations.
    default_tenor_days: i32,
}

impl std::fmt::Debug for IndexedMarketAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexedMarketAdapter")
            .field("discount_curves_count", &self.discount_curves.len())
            .field("forward_curves_count", &self.forward_curves.len())
            .field("fx_rates_count", &self.fx_rates.len())
            .field("valuation_date_days", &self.valuation_date_days)
            .field("default_tenor_days", &self.default_tenor_days)
            .finish()
    }
}

impl IndexedMarketAdapter {
    /// Creates a new adapter with the given configuration.
    ///
    /// Use `IndexedMarketAdapterBuilder` for a more ergonomic construction.
    #[must_use]
    pub fn new(
        discount_curves: Vec<Option<Arc<dyn YieldCurve<f64> + Send + Sync>>>,
        forward_curves: Vec<Option<Arc<dyn YieldCurve<f64> + Send + Sync>>>,
        fx_rates: Vec<f64>,
        valuation_date_days: i32,
    ) -> Self {
        Self {
            discount_curves,
            forward_curves,
            fx_rates,
            valuation_date_days,
            default_tenor_days: 90,
        }
    }

    /// Sets the default tenor for forward rate calculations.
    #[must_use]
    pub fn with_default_tenor(mut self, tenor_days: i32) -> Self {
        self.default_tenor_days = tenor_days;
        self
    }

    /// Converts days from epoch to time in years from valuation date.
    #[inline]
    fn days_to_years(&self, days_from_epoch: i32) -> f64 {
        let days_to_payment = days_from_epoch - self.valuation_date_days;
        if days_to_payment <= 0 {
            0.0
        } else {
            days_to_payment as f64 / 365.0
        }
    }
}

impl CurveProvider for IndexedMarketAdapter {
    fn discount_factor(&self, curve_id: u8, days_from_epoch: i32) -> f64 {
        let t = self.days_to_years(days_from_epoch);
        if t <= 0.0 {
            return 1.0;
        }

        let curve_idx = curve_id as usize;
        if curve_idx < self.discount_curves.len() {
            if let Some(ref curve) = self.discount_curves[curve_idx] {
                return curve.discount_factor(t).unwrap_or(1.0);
            }
        }

        // Fallback: return 1.0 (no discounting)
        1.0
    }

    fn forward_rate(&self, fwd_index_id: u16, fixing_days: i32, tenor_days: i32) -> f64 {
        // ID 0 is dummy (fixed leg)
        if fwd_index_id == 0 {
            return 0.0;
        }

        let idx = fwd_index_id as usize;
        if idx < self.forward_curves.len() {
            if let Some(ref curve) = self.forward_curves[idx] {
                let t1 = self.days_to_years(fixing_days);
                let tenor = if tenor_days > 0 {
                    tenor_days as f64 / 365.0
                } else {
                    self.default_tenor_days as f64 / 365.0
                };
                let t2 = t1 + tenor;
                return curve.forward_rate(t1, t2).unwrap_or(0.0);
            }
        }

        // Fallback: return 0.0
        0.0
    }

    fn fx_rate(&self, fx_id: u16) -> f64 {
        // ID 0 is dummy (no FX conversion)
        if fx_id == 0 {
            return 1.0;
        }

        let idx = fx_id as usize;
        if idx < self.fx_rates.len() {
            return self.fx_rates[idx];
        }

        // Fallback: return 1.0 (no conversion)
        1.0
    }

    fn valuation_date_days(&self) -> i32 { self.valuation_date_days }
}

// =============================================================================
// IndexedMarketAdapterBuilder
// =============================================================================

/// Builder for constructing `IndexedMarketAdapter` instances.
///
/// # Example
///
/// ```ignore
/// use pricer_pricing::kernel::IndexedMarketAdapterBuilder;
///
/// let adapter = IndexedMarketAdapterBuilder::new()
///     .valuation_date_days(18262) // 2020-01-01
///     .add_discount_curve(0, sofr_curve.clone())
///     .add_discount_curve(1, estr_curve.clone())
///     .add_forward_curve(1, sofr_curve)
///     .add_fx_rate(1, 1.10) // EUR/USD
///     .build();
/// ```
#[derive(Default)]
pub struct IndexedMarketAdapterBuilder {
    discount_curves: Vec<Option<Arc<dyn YieldCurve<f64> + Send + Sync>>>,
    forward_curves: Vec<Option<Arc<dyn YieldCurve<f64> + Send + Sync>>>,
    fx_rates: Vec<f64>,
    valuation_date_days: i32,
    default_tenor_days: i32,
}

impl IndexedMarketAdapterBuilder {
    /// Creates a new builder with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            discount_curves: Vec::new(),
            forward_curves: vec![None], // Reserve index 0 for dummy
            fx_rates: vec![1.0],        // Reserve index 0 for dummy (1.0)
            valuation_date_days: 0,
            default_tenor_days: 90,
        }
    }

    /// Sets the valuation date as days from epoch.
    #[must_use]
    pub fn valuation_date_days(mut self, days: i32) -> Self {
        self.valuation_date_days = days;
        self
    }

    /// Sets the default tenor for forward rate calculations.
    #[must_use]
    pub fn default_tenor_days(mut self, days: i32) -> Self {
        self.default_tenor_days = days;
        self
    }

    /// Adds a discount curve at the specified curve_id.
    ///
    /// Curves are stored by curve_id for O(1) lookup.
    #[must_use]
    pub fn add_discount_curve(
        mut self,
        curve_id: u8,
        curve: Arc<dyn YieldCurve<f64> + Send + Sync>,
    ) -> Self {
        let idx = curve_id as usize;
        if idx >= self.discount_curves.len() {
            self.discount_curves.resize(idx + 1, None);
        }
        self.discount_curves[idx] = Some(curve);
        self
    }

    /// Adds a forward curve at the specified fwd_index_id.
    ///
    /// Note: Index 0 is reserved for dummy (returns 0.0).
    #[must_use]
    pub fn add_forward_curve(
        mut self,
        fwd_index_id: u16,
        curve: Arc<dyn YieldCurve<f64> + Send + Sync>,
    ) -> Self {
        let idx = fwd_index_id as usize;
        if idx >= self.forward_curves.len() {
            self.forward_curves.resize(idx + 1, None);
        }
        self.forward_curves[idx] = Some(curve);
        self
    }

    /// Adds an FX rate at the specified fx_id.
    ///
    /// Note: Index 0 is reserved for dummy (returns 1.0).
    #[must_use]
    pub fn add_fx_rate(mut self, fx_id: u16, rate: f64) -> Self {
        let idx = fx_id as usize;
        if idx >= self.fx_rates.len() {
            self.fx_rates.resize(idx + 1, 1.0);
        }
        self.fx_rates[idx] = rate;
        self
    }

    /// Builds the `IndexedMarketAdapter`.
    #[must_use]
    pub fn build(self) -> IndexedMarketAdapter {
        IndexedMarketAdapter {
            discount_curves: self.discount_curves,
            forward_curves: self.forward_curves,
            fx_rates: self.fx_rates,
            valuation_date_days: self.valuation_date_days,
            default_tenor_days: self.default_tenor_days,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_provider_new() {
        let provider = FlatCurveProvider::new(0.05, 0.03);
        assert!((provider.discount_rate - 0.05).abs() < 1e-10);
        assert!((provider.forward_rate - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_flat_provider_discount_factor() {
        let provider = FlatCurveProvider::new(0.05, 0.03);

        // Today: DF = 1.0
        let df_today = provider.discount_factor(0, 0);
        assert!((df_today - 1.0).abs() < 1e-10);

        // 1 year (365 days): DF = exp(-0.05 * 1) ≈ 0.9512
        let df_1y = provider.discount_factor(0, 365);
        let expected = (-0.05_f64).exp();
        assert!(
            (df_1y - expected).abs() < 1e-6,
            "Expected {expected}, got {df_1y}"
        );

        // 2 years: DF = exp(-0.05 * 2) ≈ 0.9048
        let df_2y = provider.discount_factor(0, 730);
        let expected_2y = (-0.10_f64).exp();
        assert!(
            (df_2y - expected_2y).abs() < 1e-6,
            "Expected {expected_2y}, got {df_2y}"
        );
    }

    #[test]
    fn test_flat_provider_discount_factor_with_valuation_date() {
        let provider = FlatCurveProvider::new(0.05, 0.03).with_valuation_date(100);

        // Payment at day 100 (valuation date): DF = 1.0
        let df_val = provider.discount_factor(0, 100);
        assert!((df_val - 1.0).abs() < 1e-10);

        // Payment at day 465 (365 days from valuation): DF = exp(-0.05)
        let df_1y = provider.discount_factor(0, 465);
        let expected = (-0.05_f64).exp();
        assert!((df_1y - expected).abs() < 1e-6);
    }

    #[test]
    fn test_flat_provider_forward_rate() {
        let provider = FlatCurveProvider::new(0.05, 0.03);

        // Dummy index (0) returns 0.0
        let fwd_dummy = provider.forward_rate(0, 100, 90);
        assert!((fwd_dummy - 0.0).abs() < 1e-10);

        // Real index returns forward rate
        let fwd_real = provider.forward_rate(1, 100, 90);
        assert!((fwd_real - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_flat_provider_fx_rate() {
        let provider = FlatCurveProvider::new(0.05, 0.03);

        // Dummy FX (0) returns 1.0
        let fx_dummy = provider.fx_rate(0);
        assert!((fx_dummy - 1.0).abs() < 1e-10);

        // Real FX also returns 1.0 (flat provider is single currency)
        let fx_real = provider.fx_rate(1);
        assert!((fx_real - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_flat_provider_valuation_date() {
        let provider = FlatCurveProvider::new(0.05, 0.03).with_valuation_date(18262);

        assert_eq!(provider.valuation_date_days(), 18262);
    }

    #[test]
    fn test_flat_provider_curve_id_ignored() {
        let provider = FlatCurveProvider::new(0.05, 0.03);

        // Different curve IDs should give same result (flat provider)
        let df_0 = provider.discount_factor(0, 365);
        let df_1 = provider.discount_factor(1, 365);
        let df_255 = provider.discount_factor(255, 365);

        assert!((df_0 - df_1).abs() < 1e-10);
        assert!((df_0 - df_255).abs() < 1e-10);
    }

    #[test]
    fn test_flat_provider_negative_days() {
        let provider = FlatCurveProvider::new(0.05, 0.03).with_valuation_date(100);

        // Payment in the past: DF = 1.0
        let df_past = provider.discount_factor(0, 50);
        assert!((df_past - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_flat_provider_clone() {
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let cloned = provider.clone();

        assert!((cloned.discount_rate - 0.05).abs() < 1e-10);
        assert!((cloned.forward_rate - 0.03).abs() < 1e-10);
    }

    // =========================================================================
    // IndexedMarketAdapter Tests
    // =========================================================================

    use pricer_models::market::FlatCurve;

    #[test]
    fn test_indexed_market_adapter_builder_default() {
        let adapter = IndexedMarketAdapterBuilder::new().build();

        // Should have valuation date of 0
        assert_eq!(adapter.valuation_date_days(), 0);
    }

    #[test]
    fn test_indexed_market_adapter_valuation_date() {
        let adapter = IndexedMarketAdapterBuilder::new()
            .valuation_date_days(18262)
            .build();

        assert_eq!(adapter.valuation_date_days(), 18262);
    }

    #[test]
    fn test_indexed_market_adapter_discount_factor() {
        let curve = Arc::new(FlatCurve::new(0.05));
        let adapter = IndexedMarketAdapterBuilder::new()
            .valuation_date_days(0)
            .add_discount_curve(0, curve)
            .build();

        // Today: DF = 1.0
        let df_today = adapter.discount_factor(0, 0);
        assert!((df_today - 1.0).abs() < 1e-10);

        // 1 year (365 days): DF = exp(-0.05 * 1)
        let df_1y = adapter.discount_factor(0, 365);
        let expected = (-0.05_f64).exp();
        assert!(
            (df_1y - expected).abs() < 1e-6,
            "Expected {expected}, got {df_1y}"
        );
    }

    #[test]
    fn test_indexed_market_adapter_multiple_curves() {
        let curve_usd = Arc::new(FlatCurve::new(0.05));
        let curve_eur = Arc::new(FlatCurve::new(0.03));

        let adapter = IndexedMarketAdapterBuilder::new()
            .valuation_date_days(0)
            .add_discount_curve(0, curve_usd)
            .add_discount_curve(1, curve_eur)
            .build();

        // USD curve (5%)
        let df_usd = adapter.discount_factor(0, 365);
        let expected_usd = (-0.05_f64).exp();
        assert!((df_usd - expected_usd).abs() < 1e-6);

        // EUR curve (3%)
        let df_eur = adapter.discount_factor(1, 365);
        let expected_eur = (-0.03_f64).exp();
        assert!((df_eur - expected_eur).abs() < 1e-6);
    }

    #[test]
    fn test_indexed_market_adapter_forward_rate_dummy() {
        let adapter = IndexedMarketAdapterBuilder::new().build();

        // Dummy index (0) should return 0.0
        let fwd = adapter.forward_rate(0, 100, 90);
        assert!((fwd - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_indexed_market_adapter_forward_rate() {
        let curve = Arc::new(FlatCurve::new(0.04));
        let adapter = IndexedMarketAdapterBuilder::new()
            .valuation_date_days(0)
            .add_forward_curve(1, curve)
            .build();

        // Forward rate from flat curve should be approximately the rate
        let fwd = adapter.forward_rate(1, 0, 90);
        // For flat curve, forward rate = (DF(t1) / DF(t2) - 1) / tau
        // With continuous compounding: ~rate
        assert!(
            (fwd - 0.04).abs() < 0.001,
            "Forward rate from flat curve should be ~0.04, got {fwd}"
        );
    }

    #[test]
    fn test_indexed_market_adapter_fx_rate_dummy() {
        let adapter = IndexedMarketAdapterBuilder::new().build();

        // Dummy FX (0) should return 1.0
        let fx = adapter.fx_rate(0);
        assert!((fx - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_indexed_market_adapter_fx_rate() {
        let adapter = IndexedMarketAdapterBuilder::new()
            .add_fx_rate(1, 1.10) // EUR/USD
            .add_fx_rate(2, 0.85) // GBP/USD
            .build();

        assert!((adapter.fx_rate(1) - 1.10).abs() < 1e-10);
        assert!((adapter.fx_rate(2) - 0.85).abs() < 1e-10);
    }

    #[test]
    fn test_indexed_market_adapter_missing_curve_fallback() {
        let adapter = IndexedMarketAdapterBuilder::new().build();

        // Missing curve should return 1.0 (no discounting)
        let df = adapter.discount_factor(99, 365);
        assert!((df - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_indexed_market_adapter_missing_forward_fallback() {
        let adapter = IndexedMarketAdapterBuilder::new().build();

        // Missing forward curve should return 0.0
        let fwd = adapter.forward_rate(99, 100, 90);
        assert!((fwd - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_indexed_market_adapter_missing_fx_fallback() {
        let adapter = IndexedMarketAdapterBuilder::new().build();

        // Missing FX rate should return 1.0 (no conversion)
        let fx = adapter.fx_rate(99);
        assert!((fx - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_indexed_market_adapter_with_valuation_date() {
        let curve = Arc::new(FlatCurve::new(0.05));
        let adapter = IndexedMarketAdapterBuilder::new()
            .valuation_date_days(100)
            .add_discount_curve(0, curve)
            .build();

        // Payment at valuation date: DF = 1.0
        let df_val = adapter.discount_factor(0, 100);
        assert!((df_val - 1.0).abs() < 1e-10);

        // Payment 365 days after valuation (day 465)
        let df_1y = adapter.discount_factor(0, 465);
        let expected = (-0.05_f64).exp();
        assert!((df_1y - expected).abs() < 1e-6);
    }

    #[test]
    fn test_indexed_market_adapter_past_payment() {
        let curve = Arc::new(FlatCurve::new(0.05));
        let adapter = IndexedMarketAdapterBuilder::new()
            .valuation_date_days(100)
            .add_discount_curve(0, curve)
            .build();

        // Payment in the past (before valuation): DF = 1.0
        let df_past = adapter.discount_factor(0, 50);
        assert!((df_past - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_indexed_market_adapter_default_tenor() {
        let curve = Arc::new(FlatCurve::new(0.04));
        let adapter = IndexedMarketAdapterBuilder::new()
            .valuation_date_days(0)
            .default_tenor_days(180) // 6 months
            .add_forward_curve(1, curve)
            .build();

        // Forward rate with 0 tenor should use default (180 days)
        let fwd = adapter.forward_rate(1, 0, 0);
        // Should be approximately 0.04
        assert!(fwd > 0.0, "Forward rate should be positive");
    }

    #[test]
    fn test_indexed_market_adapter_with_default_tenor_constructor() {
        let adapter = IndexedMarketAdapterBuilder::new()
            .build()
            .with_default_tenor(180);

        assert_eq!(adapter.default_tenor_days, 180);
    }
}
