//! Kernel context for runtime market data access.
//!
//! This module provides `KernelContext` which holds references to market
//! data providers during pricing.

use super::provider::CurveProvider;

/// Runtime context for pricing kernel execution.
///
/// `KernelContext` provides the bridge between the `PricingKernel` IR
/// and the market data needed for pricing. It holds a reference to a
/// `CurveProvider` that supplies discount factors and forward rates.
///
/// # Lifetime
///
/// The context holds a reference to the curve provider, so it must not
/// outlive the provider. This design avoids copying market data.
///
/// # Examples
///
/// ```ignore
/// use pricer_pricing::kernel::{CurveProvider, KernelContext, FlatCurveProvider};
///
/// let curves = FlatCurveProvider::new(0.05, 0.03);
/// let context = KernelContext::new(&curves);
///
/// // Context can be used for pricing
/// let df = context.discount_factor(0, 365);
/// ```
#[derive(Debug)]
pub struct KernelContext<'a, P: CurveProvider> {
    /// Reference to the curve provider.
    provider: &'a P,
    /// Default tenor in days for forward rate calculations.
    default_tenor_days: i32,
}

impl<'a, P: CurveProvider> KernelContext<'a, P> {
    /// Creates a new kernel context with the given curve provider.
    ///
    /// Uses a default tenor of 90 days (3 months) for forward rate
    /// calculations when not specified.
    #[must_use]
    pub fn new(provider: &'a P) -> Self {
        Self {
            provider,
            default_tenor_days: 90,
        }
    }

    /// Creates a context with a custom default tenor.
    ///
    /// # Arguments
    ///
    /// * `provider` - Curve provider reference
    /// * `default_tenor_days` - Default tenor for forward rates (days)
    #[must_use]
    pub fn with_tenor(provider: &'a P, default_tenor_days: i32) -> Self {
        Self {
            provider,
            default_tenor_days,
        }
    }

    /// Returns the discount factor for a given curve and date.
    ///
    /// Delegates to the underlying curve provider.
    #[inline]
    pub fn discount_factor(&self, curve_id: u8, days_from_epoch: i32) -> f64 {
        self.provider.discount_factor(curve_id, days_from_epoch)
    }

    /// Returns the forward rate for a given index and fixing date.
    ///
    /// Uses the default tenor unless overridden.
    #[inline]
    pub fn forward_rate(&self, fwd_index_id: u16, fixing_days: i32) -> f64 {
        self.provider
            .forward_rate(fwd_index_id, fixing_days, self.default_tenor_days)
    }

    /// Returns the forward rate with explicit tenor.
    #[inline]
    pub fn forward_rate_with_tenor(
        &self,
        fwd_index_id: u16,
        fixing_days: i32,
        tenor_days: i32,
    ) -> f64 {
        self.provider.forward_rate(fwd_index_id, fixing_days, tenor_days)
    }

    /// Returns the FX rate for currency conversion.
    #[inline]
    pub fn fx_rate(&self, fx_id: u16) -> f64 {
        self.provider.fx_rate(fx_id)
    }

    /// Returns the valuation date as days from epoch.
    #[inline]
    pub fn valuation_date_days(&self) -> i32 {
        self.provider.valuation_date_days()
    }

    /// Returns a reference to the underlying curve provider.
    #[inline]
    #[must_use]
    pub fn provider(&self) -> &P {
        self.provider
    }

    /// Returns the default tenor in days.
    #[inline]
    #[must_use]
    pub fn default_tenor_days(&self) -> i32 {
        self.default_tenor_days
    }
}

#[cfg(test)]
mod tests {
    use super::super::provider::FlatCurveProvider;
    use super::*;

    #[test]
    fn test_kernel_context_new() {
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);

        assert_eq!(context.default_tenor_days(), 90);
    }

    #[test]
    fn test_kernel_context_with_tenor() {
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::with_tenor(&provider, 180);

        assert_eq!(context.default_tenor_days(), 180);
    }

    #[test]
    fn test_kernel_context_discount_factor() {
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);

        let df = context.discount_factor(0, 365);
        let expected = (-0.05_f64).exp();
        assert!((df - expected).abs() < 1e-6);
    }

    #[test]
    fn test_kernel_context_forward_rate() {
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);

        // Dummy index returns 0.0
        let fwd_dummy = context.forward_rate(0, 100);
        assert!((fwd_dummy - 0.0).abs() < 1e-10);

        // Real index returns forward rate
        let fwd_real = context.forward_rate(1, 100);
        assert!((fwd_real - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_kernel_context_fx_rate() {
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);

        let fx = context.fx_rate(0);
        assert!((fx - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_kernel_context_valuation_date() {
        let provider = FlatCurveProvider::new(0.05, 0.03).with_valuation_date(18262);
        let context = KernelContext::new(&provider);

        assert_eq!(context.valuation_date_days(), 18262);
    }

    #[test]
    fn test_kernel_context_provider() {
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);

        // Can access provider through context
        let df = context.provider().discount_factor(0, 365);
        let expected = (-0.05_f64).exp();
        assert!((df - expected).abs() < 1e-6);
    }
}
