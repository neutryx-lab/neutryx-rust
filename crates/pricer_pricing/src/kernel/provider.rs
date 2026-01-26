//! Market data provider traits for pricing kernels.
//!
//! This module defines the `CurveProvider` trait that abstracts market data
//! access for the pricing kernel. Implementations provide discount factors
//! and forward rates based on curve IDs and dates.

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
    /// * `curve_id` - Discount curve ID (from `PricingKernel::discount_curve_ids`)
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
    /// * `fwd_index_id` - Forward index ID (from `PricingKernel::fwd_index_ids`)
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
/// # #[cfg(feature = "l1l2-integration")]
/// # {
/// use pricer_pricing::kernel::FlatCurveProvider;
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
    /// * `discount_rate` - Continuous compound discount rate (e.g., 0.05 for 5%)
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

    fn fx_rate(&self, fx_id: u16) -> f64 {
        // ID 0 is dummy (no FX conversion)
        if fx_id == 0 {
            1.0
        } else {
            1.0 // Flat provider assumes single currency
        }
    }

    fn valuation_date_days(&self) -> i32 {
        self.valuation_date_days
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
}
