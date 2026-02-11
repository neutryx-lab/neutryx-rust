//! Kernel context for runtime market data access.

use super::provider::CurveProvider;

/// Runtime context for pricing kernel execution.
#[derive(Debug)]
pub struct KernelContext<'a, P: CurveProvider> {
    provider: &'a P,
    default_tenor_days: i32,
}

impl<'a, P: CurveProvider> KernelContext<'a, P> {
    /// Creates a new kernel context with the given curve provider.
    #[must_use]
    pub fn new(provider: &'a P) -> Self {
        Self {
            provider,
            default_tenor_days: 90,
        }
    }

    /// Creates a context with a custom default tenor.
    #[must_use]
    pub fn with_tenor(provider: &'a P, default_tenor_days: i32) -> Self {
        Self {
            provider,
            default_tenor_days,
        }
    }

    /// Returns the discount factor for a given curve and date.
    #[inline]
    pub fn discount_factor(&self, curve_id: u8, days_from_epoch: i32) -> f64 {
        self.provider.discount_factor(curve_id, days_from_epoch)
    }

    /// Returns the forward rate for a given index and fixing date.
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
        self.provider
            .forward_rate(fwd_index_id, fixing_days, tenor_days)
    }

    /// Returns the FX rate for currency conversion.
    #[inline]
    pub fn fx_rate(&self, fx_id: u16) -> f64 { self.provider.fx_rate(fx_id) }

    /// Returns the valuation date as days from epoch.
    #[inline]
    pub fn valuation_date_days(&self) -> i32 { self.provider.valuation_date_days() }

    /// Returns a reference to the underlying curve provider.
    #[inline]
    #[must_use]
    pub fn provider(&self) -> &P { self.provider }

    /// Returns the default tenor in days.
    #[inline]
    #[must_use]
    pub fn default_tenor_days(&self) -> i32 { self.default_tenor_days }
}

#[cfg(test)]
mod tests {
    use super::{super::provider::FlatCurveProvider, *};

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
        let fwd_dummy = context.forward_rate(0, 100);
        assert!((fwd_dummy - 0.0).abs() < 1e-10);
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
        let df = context.provider().discount_factor(0, 365);
        let expected = (-0.05_f64).exp();
        assert!((df - expected).abs() < 1e-6);
    }
}
