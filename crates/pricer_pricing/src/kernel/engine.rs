//! Linear pricing engine for PricingKernel IR.

use pricer_core::kernel::PricingKernel;
use rayon::prelude::*;

use super::{context::KernelContext, provider::CurveProvider};

const DAYS_PER_YEAR: f64 = 365.0;

/// Converts days from epoch to time in years relative to valuation date.
#[inline]
pub fn days_to_years(days: i32, valuation_date_days: i32) -> f64 {
    (days - valuation_date_days) as f64 / DAYS_PER_YEAR
}

/// Converts time in years to days from valuation date.
#[inline]
pub fn years_to_days(years: f64, valuation_date_days: i32) -> i32 {
    valuation_date_days + (years * DAYS_PER_YEAR) as i32
}

/// SIMD-friendly pricing engine for linear products.
pub struct LinearEngine;

impl LinearEngine {
    /// Prices a `PricingKernel` and returns the net present value.
    pub fn price<P: CurveProvider>(kernel: &PricingKernel, context: &KernelContext<'_, P>) -> f64 {
        let n = kernel.len();
        if n == 0 {
            return 0.0;
        }

        let mut npv = 0.0;

        for i in 0..n {
            let fwd = context.forward_rate(kernel.fwd_index_ids[i], kernel.fixing_dates[i]);
            let rate = fwd * kernel.gearings[i] + kernel.spreads[i];
            let amount = kernel.notionals[i] * kernel.year_fractions[i] * rate;
            let df = context.discount_factor(kernel.discount_curve_ids[i], kernel.payment_dates[i]);
            let fx = context.fx_rate(kernel.fx_index_ids[i]);
            npv += amount * df * fx;
        }

        npv
    }

    /// Prices a kernel and returns per-cashflow present values.
    pub fn price_decomposed<P: CurveProvider>(
        kernel: &PricingKernel,
        context: &KernelContext<'_, P>,
    ) -> Vec<f64> {
        let n = kernel.len();
        let mut pvs = Vec::with_capacity(n);

        for i in 0..n {
            let fwd = context.forward_rate(kernel.fwd_index_ids[i], kernel.fixing_dates[i]);
            let rate = fwd * kernel.gearings[i] + kernel.spreads[i];
            let amount = kernel.notionals[i] * kernel.year_fractions[i] * rate;
            let df = context.discount_factor(kernel.discount_curve_ids[i], kernel.payment_dates[i]);
            let fx = context.fx_rate(kernel.fx_index_ids[i]);
            pvs.push(amount * df * fx);
        }

        pvs
    }

    /// Returns the total undiscounted cash amount.
    pub fn undiscounted_amount<P: CurveProvider>(
        kernel: &PricingKernel,
        context: &KernelContext<'_, P>,
    ) -> f64 {
        let n = kernel.len();
        let mut total = 0.0;

        for i in 0..n {
            let fwd = context.forward_rate(kernel.fwd_index_ids[i], kernel.fixing_dates[i]);
            let rate = fwd * kernel.gearings[i] + kernel.spreads[i];
            let amount = kernel.notionals[i] * kernel.year_fractions[i] * rate;
            total += amount;
        }

        total
    }

    /// Prices a batch of kernels sequentially.
    pub fn price_batch<P: CurveProvider>(
        kernels: &[PricingKernel],
        context: &KernelContext<'_, P>,
    ) -> Vec<f64> {
        kernels.iter().map(|k| Self::price(k, context)).collect()
    }

    /// Prices a batch of kernels in parallel using Rayon.
    pub fn price_batch_parallel<P: CurveProvider + Sync>(
        kernels: &[PricingKernel],
        context: &KernelContext<'_, P>,
    ) -> Vec<f64> {
        kernels
            .par_iter()
            .map(|k| Self::price(k, context))
            .collect()
    }

    /// Sums batch NPVs in parallel.
    pub fn price_batch_sum_parallel<P: CurveProvider + Sync>(
        kernels: &[PricingKernel],
        context: &KernelContext<'_, P>,
    ) -> f64 {
        kernels.par_iter().map(|k| Self::price(k, context)).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::{super::provider::FlatCurveProvider, *};

    fn create_fixed_kernel() -> PricingKernel {
        PricingKernel::new(
            vec![365, 730],
            vec![0, 365],
            vec![0.5, 0.5],
            vec![1_000_000.0, 1_000_000.0],
            vec![0.05, 0.05],
            vec![0.0, 0.0],
            vec![0, 0],
            vec![0, 0],
            vec![0, 0],
            vec![0, 0],
        )
        .expect("Valid kernel")
    }

    fn create_floating_kernel() -> PricingKernel {
        PricingKernel::new(
            vec![365, 730],
            vec![0, 365],
            vec![0.5, 0.5],
            vec![1_000_000.0, 1_000_000.0],
            vec![0.01, 0.01],
            vec![1.0, 1.0],
            vec![0, 0],
            vec![0, 0],
            vec![1, 1],
            vec![0, 0],
        )
        .expect("Valid kernel")
    }

    fn create_swap_kernel() -> PricingKernel {
        PricingKernel::new(
            vec![365, 365, 730, 730],
            vec![0, 0, 365, 365],
            vec![0.5, 0.5, 0.5, 0.5],
            vec![1_000_000.0, -1_000_000.0, 1_000_000.0, -1_000_000.0],
            vec![0.05, 0.0, 0.05, 0.0],
            vec![0.0, 1.0, 0.0, 1.0],
            vec![0, 0, 0, 0],
            vec![0, 0, 0, 0],
            vec![0, 1, 0, 1],
            vec![0, 0, 0, 0],
        )
        .expect("Valid kernel")
    }

    #[test]
    fn test_price_empty_kernel() {
        let kernel = PricingKernel::empty();
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);
        let npv = LinearEngine::price(&kernel, &context);
        assert!((npv - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_price_fixed_kernel() {
        let kernel = create_fixed_kernel();
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);
        let npv = LinearEngine::price(&kernel, &context);
        assert!(npv > 45000.0, "NPV should be around 46000, got {npv}");
        assert!(npv < 48000.0, "NPV should be around 46000, got {npv}");
    }

    #[test]
    fn test_price_floating_kernel() {
        let kernel = create_floating_kernel();
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);
        let npv = LinearEngine::price(&kernel, &context);
        assert!(npv > 36000.0, "NPV should be around 37000, got {npv}");
        assert!(npv < 38000.0, "NPV should be around 37000, got {npv}");
    }

    #[test]
    fn test_price_swap_kernel() {
        let kernel = create_swap_kernel();
        let provider = FlatCurveProvider::new(0.05, 0.05);
        let context = KernelContext::new(&provider);
        let npv = LinearEngine::price(&kernel, &context);
        assert!(
            npv.abs() < 1000.0,
            "At-par swap should have NPV ~ 0, got {npv}"
        );
    }

    #[test]
    fn test_price_decomposed() {
        let kernel = create_fixed_kernel();
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);
        let pvs = LinearEngine::price_decomposed(&kernel, &context);
        assert_eq!(pvs.len(), 2);
        assert!(pvs[0] > 0.0);
        assert!(pvs[1] > 0.0);
        let total: f64 = pvs.iter().sum();
        let npv = LinearEngine::price(&kernel, &context);
        assert!((total - npv).abs() < 1e-10);
    }

    #[test]
    fn test_undiscounted_amount() {
        let kernel = create_fixed_kernel();
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);
        let amount = LinearEngine::undiscounted_amount(&kernel, &context);
        assert!((amount - 50000.0).abs() < 1e-10);
    }

    #[test]
    fn test_price_with_valuation_date() {
        let kernel = create_fixed_kernel();
        let provider = FlatCurveProvider::new(0.05, 0.03).with_valuation_date(0);
        let context = KernelContext::new(&provider);
        let npv = LinearEngine::price(&kernel, &context);
        assert!(npv > 0.0, "NPV should be positive");
    }

    #[test]
    fn test_price_direction_sign() {
        let kernel = PricingKernel::new(
            vec![365],
            vec![0],
            vec![0.5],
            vec![-1_000_000.0],
            vec![0.05],
            vec![0.0],
            vec![0],
            vec![0],
            vec![0],
            vec![0],
        )
        .expect("Valid kernel");

        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);
        let npv = LinearEngine::price(&kernel, &context);
        assert!(npv < 0.0, "Payer NPV should be negative, got {npv}");
    }

    #[test]
    fn test_price_zero_notional() {
        let kernel = PricingKernel::new(
            vec![365],
            vec![0],
            vec![0.5],
            vec![0.0],
            vec![0.05],
            vec![0.0],
            vec![0],
            vec![0],
            vec![0],
            vec![0],
        )
        .expect("Valid kernel");

        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);
        let npv = LinearEngine::price(&kernel, &context);
        assert!((npv - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_days_to_years_one_year() {
        let valuation = 18262;
        let payment = 18262 + 365;
        let years = super::days_to_years(payment, valuation);
        assert!((years - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_days_to_years_half_year() {
        let valuation = 18262;
        let payment = valuation + 183;
        let years = super::days_to_years(payment, valuation);
        assert!((years - 0.5013698630).abs() < 0.01);
    }

    #[test]
    fn test_days_to_years_same_date() {
        let valuation = 18262;
        let years = super::days_to_years(valuation, valuation);
        assert!((years - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_days_to_years_past_date() {
        let valuation = 18262;
        let past = valuation - 365;
        let years = super::days_to_years(past, valuation);
        assert!((years - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_years_to_days_roundtrip() {
        let valuation = 18262;
        let original_days = 18627;
        let years = super::days_to_years(original_days, valuation);
        let back_to_days = super::years_to_days(years, valuation);
        assert_eq!(back_to_days, original_days);
    }

    #[test]
    fn test_years_to_days_two_years() {
        let valuation = 18262;
        let days = super::years_to_days(2.0, valuation);
        assert_eq!(days, valuation + 730);
    }

    #[test]
    fn test_price_batch_empty() {
        let kernels: Vec<PricingKernel> = vec![];
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);
        let npvs = LinearEngine::price_batch(&kernels, &context);
        assert!(npvs.is_empty());
    }

    #[test]
    fn test_price_batch_single() {
        let kernels = vec![create_fixed_kernel()];
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);
        let npvs = LinearEngine::price_batch(&kernels, &context);
        let single_npv = LinearEngine::price(&kernels[0], &context);
        assert_eq!(npvs.len(), 1);
        assert!((npvs[0] - single_npv).abs() < 1e-10);
    }

    #[test]
    fn test_price_batch_multiple() {
        let kernels = vec![
            create_fixed_kernel(),
            create_floating_kernel(),
            create_swap_kernel(),
        ];
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);
        let npvs = LinearEngine::price_batch(&kernels, &context);
        assert_eq!(npvs.len(), 3);
        for (i, kernel) in kernels.iter().enumerate() {
            let expected = LinearEngine::price(kernel, &context);
            assert!((npvs[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_price_batch_parallel_consistency() {
        let kernels: Vec<PricingKernel> = (0..100).map(|_| create_fixed_kernel()).collect();
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);
        let sequential = LinearEngine::price_batch(&kernels, &context);
        let parallel = LinearEngine::price_batch_parallel(&kernels, &context);
        assert_eq!(sequential.len(), parallel.len());
        for (seq, par) in sequential.iter().zip(parallel.iter()) {
            assert!(
                (seq - par).abs() < 1e-10,
                "Parallel result should match sequential"
            );
        }
    }

    #[test]
    fn test_price_batch_sum_parallel() {
        let kernels: Vec<PricingKernel> = (0..50).map(|_| create_fixed_kernel()).collect();
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);
        let sum_parallel = LinearEngine::price_batch_sum_parallel(&kernels, &context);
        let sum_sequential: f64 = LinearEngine::price_batch(&kernels, &context).iter().sum();
        assert!((sum_parallel - sum_sequential).abs() < 1e-8);
    }

    #[test]
    fn test_price_batch_parallel_mixed_types() {
        let mut kernels = Vec::new();
        for _ in 0..10 {
            kernels.push(create_fixed_kernel());
            kernels.push(create_floating_kernel());
            kernels.push(create_swap_kernel());
        }
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);
        let sequential = LinearEngine::price_batch(&kernels, &context);
        let parallel = LinearEngine::price_batch_parallel(&kernels, &context);
        assert_eq!(sequential.len(), parallel.len());
        for (seq, par) in sequential.iter().zip(parallel.iter()) {
            assert!((seq - par).abs() < 1e-10);
        }
    }
}
