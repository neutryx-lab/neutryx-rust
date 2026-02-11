//! Market data provider traits for pricing kernels.

use std::sync::Arc;

use pricer_models::market::curves::YieldCurve;

/// Trait for providing market data to pricing kernels.
pub trait CurveProvider {
    /// Returns the discount factor for a given curve and date.
    fn discount_factor(&self, curve_id: u8, days_from_epoch: i32) -> f64;

    /// Returns the forward rate for a given index and fixing date.
    fn forward_rate(&self, fwd_index_id: u16, fixing_days: i32, tenor_days: i32) -> f64;

    /// Returns the FX rate for currency conversion.
    fn fx_rate(&self, fx_id: u16) -> f64;

    /// Returns the current valuation date as days from epoch.
    fn valuation_date_days(&self) -> i32;
}

/// A simple flat curve provider for testing and demonstration.
#[derive(Debug, Clone)]
pub struct FlatCurveProvider {
    discount_rate: f64,
    forward_rate: f64,
    valuation_date_days: i32,
}

impl FlatCurveProvider {
    /// Creates a new flat curve provider.
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
        let days_to_payment = days_from_epoch - self.valuation_date_days;
        if days_to_payment <= 0 {
            return 1.0;
        }

        let t = days_to_payment as f64 / 365.0;
        (-self.discount_rate * t).exp()
    }

    fn forward_rate(&self, fwd_index_id: u16, _fixing_days: i32, _tenor_days: i32) -> f64 {
        if fwd_index_id == 0 {
            0.0
        } else {
            self.forward_rate
        }
    }

    fn fx_rate(&self, _fx_id: u16) -> f64 { 1.0 }

    fn valuation_date_days(&self) -> i32 { self.valuation_date_days }
}

/// Adapter that provides `CurveProvider` interface for `IndexedMarket`.
pub struct IndexedMarketAdapter {
    discount_curves: Vec<Option<Arc<dyn YieldCurve<f64> + Send + Sync>>>,
    forward_curves: Vec<Option<Arc<dyn YieldCurve<f64> + Send + Sync>>>,
    fx_rates: Vec<f64>,
    valuation_date_days: i32,
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

        1.0
    }

    fn forward_rate(&self, fwd_index_id: u16, fixing_days: i32, tenor_days: i32) -> f64 {
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

        0.0
    }

    fn fx_rate(&self, fx_id: u16) -> f64 {
        if fx_id == 0 {
            return 1.0;
        }

        let idx = fx_id as usize;
        if idx < self.fx_rates.len() {
            return self.fx_rates[idx];
        }

        1.0
    }

    fn valuation_date_days(&self) -> i32 { self.valuation_date_days }
}

/// Builder for constructing `IndexedMarketAdapter` instances.
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
            forward_curves: vec![None],
            fx_rates: vec![1.0],
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

        let df_today = provider.discount_factor(0, 0);
        assert!((df_today - 1.0).abs() < 1e-10);

        let df_1y = provider.discount_factor(0, 365);
        let expected = (-0.05_f64).exp();
        assert!(
            (df_1y - expected).abs() < 1e-6,
            "Expected {expected}, got {df_1y}"
        );

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

        let df_val = provider.discount_factor(0, 100);
        assert!((df_val - 1.0).abs() < 1e-10);

        let df_1y = provider.discount_factor(0, 465);
        let expected = (-0.05_f64).exp();
        assert!((df_1y - expected).abs() < 1e-6);
    }

    #[test]
    fn test_flat_provider_forward_rate() {
        let provider = FlatCurveProvider::new(0.05, 0.03);

        let fwd_dummy = provider.forward_rate(0, 100, 90);
        assert!((fwd_dummy - 0.0).abs() < 1e-10);

        let fwd_real = provider.forward_rate(1, 100, 90);
        assert!((fwd_real - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_flat_provider_fx_rate() {
        let provider = FlatCurveProvider::new(0.05, 0.03);

        let fx_dummy = provider.fx_rate(0);
        assert!((fx_dummy - 1.0).abs() < 1e-10);

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

        let df_0 = provider.discount_factor(0, 365);
        let df_1 = provider.discount_factor(1, 365);
        let df_255 = provider.discount_factor(255, 365);

        assert!((df_0 - df_1).abs() < 1e-10);
        assert!((df_0 - df_255).abs() < 1e-10);
    }

    #[test]
    fn test_flat_provider_negative_days() {
        let provider = FlatCurveProvider::new(0.05, 0.03).with_valuation_date(100);

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

    use pricer_models::market::FlatCurve;

    #[test]
    fn test_indexed_market_adapter_builder_default() {
        let adapter = IndexedMarketAdapterBuilder::new().build();
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

        let df_today = adapter.discount_factor(0, 0);
        assert!((df_today - 1.0).abs() < 1e-10);

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

        let df_usd = adapter.discount_factor(0, 365);
        let expected_usd = (-0.05_f64).exp();
        assert!((df_usd - expected_usd).abs() < 1e-6);

        let df_eur = adapter.discount_factor(1, 365);
        let expected_eur = (-0.03_f64).exp();
        assert!((df_eur - expected_eur).abs() < 1e-6);
    }

    #[test]
    fn test_indexed_market_adapter_forward_rate_dummy() {
        let adapter = IndexedMarketAdapterBuilder::new().build();
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

        let fwd = adapter.forward_rate(1, 0, 90);
        assert!(
            (fwd - 0.04).abs() < 0.001,
            "Forward rate from flat curve should be ~0.04, got {fwd}"
        );
    }

    #[test]
    fn test_indexed_market_adapter_fx_rate_dummy() {
        let adapter = IndexedMarketAdapterBuilder::new().build();
        let fx = adapter.fx_rate(0);
        assert!((fx - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_indexed_market_adapter_fx_rate() {
        let adapter = IndexedMarketAdapterBuilder::new()
            .add_fx_rate(1, 1.10)
            .add_fx_rate(2, 0.85)
            .build();

        assert!((adapter.fx_rate(1) - 1.10).abs() < 1e-10);
        assert!((adapter.fx_rate(2) - 0.85).abs() < 1e-10);
    }

    #[test]
    fn test_indexed_market_adapter_missing_curve_fallback() {
        let adapter = IndexedMarketAdapterBuilder::new().build();
        let df = adapter.discount_factor(99, 365);
        assert!((df - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_indexed_market_adapter_missing_forward_fallback() {
        let adapter = IndexedMarketAdapterBuilder::new().build();
        let fwd = adapter.forward_rate(99, 100, 90);
        assert!((fwd - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_indexed_market_adapter_missing_fx_fallback() {
        let adapter = IndexedMarketAdapterBuilder::new().build();
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

        let df_val = adapter.discount_factor(0, 100);
        assert!((df_val - 1.0).abs() < 1e-10);

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

        let df_past = adapter.discount_factor(0, 50);
        assert!((df_past - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_indexed_market_adapter_default_tenor() {
        let curve = Arc::new(FlatCurve::new(0.04));
        let adapter = IndexedMarketAdapterBuilder::new()
            .valuation_date_days(0)
            .default_tenor_days(180)
            .add_forward_curve(1, curve)
            .build();

        let fwd = adapter.forward_rate(1, 0, 0);
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
