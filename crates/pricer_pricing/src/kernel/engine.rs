//! Linear pricing engine for PricingKernel IR.
//!
//! This module provides `LinearEngine` which executes the branchless
//! pricing formula on `PricingKernel` data structures.
//!
//! # SIMD Optimisation
//!
//! The pricing loop is designed to be SIMD-friendly:
//! - Sequential array access (cache-friendly)
//! - No data-dependent branching
//! - Unified formula for fixed/floating (branchless)
//! - f64 operations suitable for AVX2/AVX-512
//!
//! To verify SIMD vectorisation, compile with:
//! ```bash
//! RUSTFLAGS="-C target-cpu=native -C opt-level=3" cargo build --release
//! ```
//!
//! And inspect assembly with:
//! ```bash
//! cargo asm pricer_pricing::kernel::engine::LinearEngine::price
//! ```
//!
//! Expected SIMD instructions: vfmadd*, vmulpd, vaddpd

use pricer_core::ir::PricingKernel;
use rayon::prelude::*;

use super::{context::KernelContext, provider::CurveProvider};

/// Days per year constant for time calculations (ACT/365).
const DAYS_PER_YEAR: f64 = 365.0;

/// Converts days from epoch to time in years relative to valuation date.
///
/// # Arguments
///
/// * `days` - Days from epoch
/// * `valuation_date_days` - Valuation date as days from epoch
///
/// # Returns
///
/// Time in years (can be negative for past dates).
///
/// # Example
///
/// ```ignore
/// // If valuation date is day 18262 (2020-01-01)
/// // and payment date is day 18627 (2021-01-01)
/// let t = days_to_years(18627, 18262);
/// assert!((t - 1.0).abs() < 0.01); // ~1 year
/// ```
#[inline]
pub fn days_to_years(days: i32, valuation_date_days: i32) -> f64 {
    (days - valuation_date_days) as f64 / DAYS_PER_YEAR
}

/// Converts time in years to days from valuation date.
///
/// Inverse of `days_to_years`.
#[inline]
pub fn years_to_days(years: f64, valuation_date_days: i32) -> i32 {
    valuation_date_days + (years * DAYS_PER_YEAR) as i32
}

/// SIMD-friendly pricing engine for linear products.
///
/// `LinearEngine` prices `PricingKernel` IR using a branchless unified
/// formula that works for both fixed and floating cashflows:
///
/// ```text
/// PV = Σᵢ (L_i × αᵢ + βᵢ) × Nᵢ × τᵢ × DFᵢ × FXᵢ
/// ```
///
/// Where:
/// - `L_i`: Forward rate (0.0 for fixed via dummy index)
/// - `αᵢ`: Gearing (0.0 for fixed, 1.0+ for floating)
/// - `βᵢ`: Spread (fixed rate for fixed, spread for floating)
/// - `Nᵢ`: Notional (signed: positive = receive, negative = pay)
/// - `τᵢ`: Year fraction
/// - `DFᵢ`: Discount factor to payment date
/// - `FXᵢ`: FX rate (1.0 for single currency via dummy)
///
/// # Example
///
/// ```ignore
/// use pricer_pricing::kernel::{LinearEngine, KernelContext, FlatCurveProvider};
/// use pricer_core::ir::PricingKernel;
///
/// let kernel = /* compiled kernel */;
/// let curves = FlatCurveProvider::new(0.05, 0.03);
/// let context = KernelContext::new(&curves);
///
/// let npv = LinearEngine::price(&kernel, &context);
/// println!("NPV: {}", npv);
/// ```
pub struct LinearEngine;

impl LinearEngine {
    /// Prices a `PricingKernel` and returns the net present value.
    ///
    /// # Arguments
    ///
    /// * `kernel` - Compiled kernel from `TradeCompiler`
    /// * `context` - Market data context
    ///
    /// # Returns
    ///
    /// Net present value of all cashflows in the kernel.
    /// Positive = net receive, negative = net pay.
    ///
    /// # Algorithm
    ///
    /// ```text
    /// for each cashflow i:
    ///     fwd = context.forward_rate(fwd_index_ids[i], fixing_dates[i])
    ///     rate = fwd * gearings[i] + spreads[i]
    ///     amount = notionals[i] * year_fractions[i] * rate
    ///     df = context.discount_factor(discount_curve_ids[i], payment_dates[i])
    ///     fx = context.fx_rate(fx_index_ids[i])
    ///     npv += amount * df * fx
    /// ```
    pub fn price<P: CurveProvider>(kernel: &PricingKernel, context: &KernelContext<'_, P>) -> f64 {
        let n = kernel.len();
        if n == 0 {
            return 0.0;
        }

        let mut npv = 0.0;

        // Sequential loop - SIMD-friendly with no data-dependent branching
        for i in 0..n {
            // Get forward rate (0.0 for fixed legs via dummy index)
            let fwd = context.forward_rate(kernel.fwd_index_ids[i], kernel.fixing_dates[i]);

            // Unified formula: rate = L * α + β
            // - Fixed: 0.0 * 0.0 + fixed_rate = fixed_rate
            // - Floating: L * 1.0 + spread = L + spread
            let rate = fwd * kernel.gearings[i] + kernel.spreads[i];

            // Cash amount = N × τ × rate
            let amount = kernel.notionals[i] * kernel.year_fractions[i] * rate;

            // Discount factor to payment date
            let df = context.discount_factor(kernel.discount_curve_ids[i], kernel.payment_dates[i]);

            // FX rate (1.0 for single currency via dummy)
            let fx = context.fx_rate(kernel.fx_index_ids[i]);

            // Accumulate PV
            npv += amount * df * fx;
        }

        npv
    }

    /// Prices a kernel and returns per-cashflow present values.
    ///
    /// Useful for cashflow decomposition, sensitivity analysis,
    /// and debugging.
    ///
    /// # Arguments
    ///
    /// * `kernel` - Compiled kernel
    /// * `context` - Market data context
    ///
    /// # Returns
    ///
    /// Vector of present values, one per cashflow.
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
    ///
    /// Useful for validation and cash flow analysis.
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

    // =========================================================================
    // Batch Evaluation (Task 12.3: Rayon Parallelisation)
    // =========================================================================

    /// Prices a batch of kernels sequentially.
    ///
    /// Useful for comparison with parallel batch pricing.
    ///
    /// # Arguments
    ///
    /// * `kernels` - Slice of kernels to price
    /// * `context` - Market data context (shared across all kernels)
    ///
    /// # Returns
    ///
    /// Vector of NPVs, one per kernel.
    pub fn price_batch<P: CurveProvider>(
        kernels: &[PricingKernel],
        context: &KernelContext<'_, P>,
    ) -> Vec<f64> {
        kernels.iter().map(|k| Self::price(k, context)).collect()
    }

    /// Prices a batch of kernels in parallel using Rayon.
    ///
    /// Distributes kernel pricing across available CPU cores.
    /// Optimal for large batches (>100 kernels).
    ///
    /// # Arguments
    ///
    /// * `kernels` - Slice of kernels to price
    /// * `context` - Market data context (must be Sync for parallel access)
    ///
    /// # Returns
    ///
    /// Vector of NPVs, one per kernel (same order as input).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let kernels: Vec<PricingKernel> = /* compiled kernels */;
    /// let context = KernelContext::new(&curves);
    ///
    /// // Parallel pricing
    /// let npvs = LinearEngine::price_batch_parallel(&kernels, &context);
    /// ```
    pub fn price_batch_parallel<P: CurveProvider + Sync>(
        kernels: &[PricingKernel],
        context: &KernelContext<'_, P>,
    ) -> Vec<f64>
    where
        P: Sync,
    {
        kernels
            .par_iter()
            .map(|k| Self::price(k, context))
            .collect()
    }

    /// Sums batch NPVs in parallel.
    ///
    /// More efficient than collecting to Vec when only total is needed.
    ///
    /// # Arguments
    ///
    /// * `kernels` - Slice of kernels to price
    /// * `context` - Market data context
    ///
    /// # Returns
    ///
    /// Sum of all NPVs.
    pub fn price_batch_sum_parallel<P: CurveProvider + Sync>(
        kernels: &[PricingKernel],
        context: &KernelContext<'_, P>,
    ) -> f64
    where
        P: Sync,
    {
        kernels.par_iter().map(|k| Self::price(k, context)).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::{super::provider::FlatCurveProvider, *};

    fn create_fixed_kernel() -> PricingKernel {
        // 2 fixed cashflows: 5% rate, 1M notional, semi-annual
        PricingKernel::new(
            vec![365, 730],                 // payment_dates (1Y, 2Y from epoch)
            vec![0, 365],                   // fixing_dates (not used for fixed)
            vec![0.5, 0.5],                 // year_fractions
            vec![1_000_000.0, 1_000_000.0], // notionals
            vec![0.05, 0.05],               // spreads (= fixed rate)
            vec![0.0, 0.0],                 // gearings (0 = fixed)
            vec![0, 0],                     // currency_ids
            vec![0, 0],                     // discount_curve_ids
            vec![0, 0],                     // fwd_index_ids (0 = dummy)
            vec![0, 0],                     // fx_index_ids (0 = no FX)
        )
        .expect("Valid kernel")
    }

    fn create_floating_kernel() -> PricingKernel {
        // 2 floating cashflows: SOFR + 100bp, 1M notional
        PricingKernel::new(
            vec![365, 730],                 // payment_dates
            vec![0, 365],                   // fixing_dates
            vec![0.5, 0.5],                 // year_fractions
            vec![1_000_000.0, 1_000_000.0], // notionals
            vec![0.01, 0.01],               // spreads (100bp)
            vec![1.0, 1.0],                 // gearings (1.0 = floating)
            vec![0, 0],                     // currency_ids
            vec![0, 0],                     // discount_curve_ids
            vec![1, 1],                     // fwd_index_ids (1 = real index)
            vec![0, 0],                     // fx_index_ids
        )
        .expect("Valid kernel")
    }

    fn create_swap_kernel() -> PricingKernel {
        // Swap: receive fixed 5%, pay floating SOFR
        PricingKernel::new(
            vec![365, 365, 730, 730], // payment_dates (interleaved)
            vec![0, 0, 365, 365],     // fixing_dates
            vec![0.5, 0.5, 0.5, 0.5], // year_fractions
            vec![1_000_000.0, -1_000_000.0, 1_000_000.0, -1_000_000.0], /* notionals (receiver
                                       * fixed, payer
                                       * floating) */
            vec![0.05, 0.0, 0.05, 0.0], // spreads
            vec![0.0, 1.0, 0.0, 1.0],   // gearings
            vec![0, 0, 0, 0],           // currency_ids
            vec![0, 0, 0, 0],           // discount_curve_ids
            vec![0, 1, 0, 1],           // fwd_index_ids
            vec![0, 0, 0, 0],           // fx_index_ids
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
        let provider = FlatCurveProvider::new(0.05, 0.03); // 5% discount rate
        let context = KernelContext::new(&provider);

        let npv = LinearEngine::price(&kernel, &context);

        // Expected:
        // CF1: 1M * 0.5 * 0.05 * exp(-0.05 * 1) = 25000 * 0.9512 ≈ 23780
        // CF2: 1M * 0.5 * 0.05 * exp(-0.05 * 2) = 25000 * 0.9048 ≈ 22620
        // Total ≈ 46400

        assert!(npv > 45000.0, "NPV should be around 46000, got {npv}");
        assert!(npv < 48000.0, "NPV should be around 46000, got {npv}");
    }

    #[test]
    fn test_price_floating_kernel() {
        let kernel = create_floating_kernel();
        let provider = FlatCurveProvider::new(0.05, 0.03); // fwd = 3%
        let context = KernelContext::new(&provider);

        let npv = LinearEngine::price(&kernel, &context);

        // Expected:
        // Rate = 3% + 1% = 4%
        // CF1: 1M * 0.5 * 0.04 * exp(-0.05 * 1) = 20000 * 0.9512 ≈ 19024
        // CF2: 1M * 0.5 * 0.04 * exp(-0.05 * 2) = 20000 * 0.9048 ≈ 18096
        // Total ≈ 37120

        assert!(npv > 36000.0, "NPV should be around 37000, got {npv}");
        assert!(npv < 38000.0, "NPV should be around 37000, got {npv}");
    }

    #[test]
    fn test_price_swap_kernel() {
        let kernel = create_swap_kernel();
        // Set forward rate = 5% so swap is at par
        let provider = FlatCurveProvider::new(0.05, 0.05);
        let context = KernelContext::new(&provider);

        let npv = LinearEngine::price(&kernel, &context);

        // At par swap (fixed rate = forward rate): NPV ≈ 0
        assert!(
            npv.abs() < 1000.0,
            "At-par swap should have NPV ≈ 0, got {npv}"
        );
    }

    #[test]
    fn test_price_decomposed() {
        let kernel = create_fixed_kernel();
        let provider = FlatCurveProvider::new(0.05, 0.03);
        let context = KernelContext::new(&provider);

        let pvs = LinearEngine::price_decomposed(&kernel, &context);

        assert_eq!(pvs.len(), 2);

        // Each PV should be positive (receiving fixed)
        assert!(pvs[0] > 0.0);
        assert!(pvs[1] > 0.0);

        // Sum should equal total NPV
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

        // Expected: 2 × (1M × 0.5 × 5%) = 50000
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
        // Payer leg (negative notional)
        let kernel = PricingKernel::new(
            vec![365],
            vec![0],
            vec![0.5],
            vec![-1_000_000.0], // Payer
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

        // Payer should have negative NPV
        assert!(npv < 0.0, "Payer NPV should be negative, got {npv}");
    }

    #[test]
    fn test_price_zero_notional() {
        let kernel = PricingKernel::new(
            vec![365],
            vec![0],
            vec![0.5],
            vec![0.0], // Zero notional
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

    // === Task 5.2: days_to_years helper tests ===

    #[test]
    fn test_days_to_years_one_year() {
        let valuation = 18262; // 2020-01-01
        let payment = 18262 + 365; // 2021-01-01
        let years = super::days_to_years(payment, valuation);
        assert!((years - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_days_to_years_half_year() {
        let valuation = 18262;
        let payment = valuation + 183; // ~6 months
        let years = super::days_to_years(payment, valuation);
        assert!((years - 0.5013698630).abs() < 0.01); // 183/365
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
        let original_days = 18627; // 1 year later
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

    // === Task 12.3: Batch pricing tests ===

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

        // Verify each NPV matches individual pricing
        for (i, kernel) in kernels.iter().enumerate() {
            let expected = LinearEngine::price(kernel, &context);
            assert!((npvs[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_price_batch_parallel_consistency() {
        // Create a batch of identical kernels
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
        // Mix of different kernel types
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
