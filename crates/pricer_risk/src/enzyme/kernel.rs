//! Slice-based pricing kernels for Enzyme AAD.
//!
//! This module provides pricing kernel functions that operate on `&[f64]` slices,
//! enabling Enzyme LLVM-level automatic differentiation without heap allocations
//! in the hot path.
//!
//! # Design Principles
//!
//! 1. **Slice-based inputs**: All inputs are `&[f64]` slices, not structs
//! 2. **No heap allocation**: Kernel functions must not allocate in hot path
//! 3. **Concrete f64 only**: No generics or dual numbers
//! 4. **Enzyme-compatible**: Functions designed for `#[autodiff]` macro
//!
//! # Input Categories
//!
//! - **Active** (`Duplicated`): Inputs we differentiate with respect to
//!   - Example: `rates` for computing DV01
//! - **Const**: Inputs that are not differentiated
//!   - Example: `times`, `notionals`, `year_fractions`
//! - **Output** (`Duplicated`): Where results and gradients are written
//!
//! # Requirements Coverage
//!
//! - 2.1: Active inputs via `&[f64]` slices
//! - 2.2: Const inputs via `&[f64]` slices
//! - 2.3: Output via `&mut f64`
//! - 2.4: No heap allocation in kernel
//! - 2.5: Enzyme accessibility via `#[autodiff]` macro
//! - 2.6: Pure f64 arithmetic
//!
//! # Example
//!
//! ```rust,ignore
//! use pricer_risk::enzyme::kernel::pricing_kernel_irs;
//!
//! let rates = [0.02, 0.03, 0.04];
//! let times = [1.0, 2.0, 5.0];
//! let notionals = [1_000_000.0; 3];
//! let year_fractions = [1.0, 1.0, 3.0];
//! let fixed_rate = 0.025;
//!
//! let mut pv = 0.0;
//! pricing_kernel_irs(&rates, &times, &notionals, &year_fractions, fixed_rate, &mut pv);
//!
//! // pv now contains the present value
//! ```

// Enzyme autodiff imports (only when feature enabled)
#[cfg(feature = "enzyme-ad")]
use std::autodiff::autodiff;

// =============================================================================
// Pricing Kernels
// =============================================================================

/// Interest Rate Swap (IRS) pricing kernel.
///
/// Computes the present value of a floating leg minus fixed leg payments.
/// All discount factors are computed from zero rates.
///
/// # Arguments
///
/// * `rates` - Zero rates at pillar tenors (Active: differentiable)
/// * `times` - Tenor points in years (Const: not differentiable)
/// * `notionals` - Notional amounts per period (Const)
/// * `year_fractions` - Year fractions per period (Const)
/// * `fixed_rate` - Fixed leg rate (Const)
/// * `output` - Output present value (Active: receives adjoint seed)
///
/// # PV Calculation
///
/// For each period i:
/// ```text
/// df[i] = exp(-rates[i] * times[i])
/// floating_cf[i] = notionals[i] * rates[i] * year_fractions[i] * df[i]
/// fixed_cf[i] = notionals[i] * fixed_rate * year_fractions[i] * df[i]
/// pv += floating_cf[i] - fixed_cf[i]
/// ```
///
/// # Requirements Coverage
///
/// - 2.1: `rates` is Active input
/// - 2.2: `times`, `notionals`, `year_fractions`, `fixed_rate` are Const
/// - 2.3: `output` is `&mut f64`
/// - 2.4: No heap allocation (uses only stack operations)
/// - 2.6: Pure f64 arithmetic
///
/// # Panics
///
/// Debug assertions check that all input slices have equal length.
#[cfg(feature = "enzyme-ad")]
#[autodiff(d_pricing_kernel_irs, Reverse, Duplicated, Const, Const, Const, Const, Duplicated)]
pub fn pricing_kernel_irs(
    rates: &[f64],
    times: &[f64],
    notionals: &[f64],
    year_fractions: &[f64],
    fixed_rate: f64,
    output: &mut f64,
) {
    debug_assert_eq!(rates.len(), times.len());
    debug_assert_eq!(rates.len(), notionals.len());
    debug_assert_eq!(rates.len(), year_fractions.len());

    let n = rates.len();
    let mut pv = 0.0;

    // Hot path: no allocation, pure arithmetic
    for i in 0..n {
        // Discount factor from zero rate
        let df = (-rates[i] * times[i]).exp();

        // Floating leg cashflow (receives floating rate)
        let floating_cf = notionals[i] * rates[i] * year_fractions[i] * df;

        // Fixed leg cashflow (pays fixed rate)
        let fixed_cf = notionals[i] * fixed_rate * year_fractions[i] * df;

        // Net cashflow (receive floating, pay fixed)
        pv += floating_cf - fixed_cf;
    }

    *output = pv;
}

/// Fallback implementation when enzyme-ad feature is disabled.
#[cfg(not(feature = "enzyme-ad"))]
pub fn pricing_kernel_irs(
    rates: &[f64],
    times: &[f64],
    notionals: &[f64],
    year_fractions: &[f64],
    fixed_rate: f64,
    output: &mut f64,
) {
    debug_assert_eq!(rates.len(), times.len());
    debug_assert_eq!(rates.len(), notionals.len());
    debug_assert_eq!(rates.len(), year_fractions.len());

    let n = rates.len();
    let mut pv = 0.0;

    for i in 0..n {
        let df = (-rates[i] * times[i]).exp();
        let floating_cf = notionals[i] * rates[i] * year_fractions[i] * df;
        let fixed_cf = notionals[i] * fixed_rate * year_fractions[i] * df;
        pv += floating_cf - fixed_cf;
    }

    *output = pv;
}

/// Simple discount factor computation kernel.
///
/// Computes present value by discounting a single cashflow.
///
/// # Arguments
///
/// * `rate` - Continuously compounded rate (Active)
/// * `time` - Time to maturity in years (Const)
/// * `cashflow` - Future cashflow amount (Const)
/// * `output` - Discounted present value (Active)
///
/// # PV Calculation
///
/// ```text
/// pv = cashflow * exp(-rate * time)
/// ```
#[cfg(feature = "enzyme-ad")]
#[autodiff(d_discount_kernel, Reverse, Duplicated, Const, Const, Duplicated)]
pub fn discount_kernel(rate: &f64, time: f64, cashflow: f64, output: &mut f64) {
    *output = cashflow * (-*rate * time).exp();
}

/// Fallback discount kernel when enzyme-ad is disabled.
#[cfg(not(feature = "enzyme-ad"))]
pub fn discount_kernel(rate: &f64, time: f64, cashflow: f64, output: &mut f64) {
    *output = cashflow * (-*rate * time).exp();
}

/// Bond pricing kernel.
///
/// Computes the present value of a series of coupon payments plus principal.
///
/// # Arguments
///
/// * `rates` - Zero rates at payment dates (Active)
/// * `times` - Payment dates in years (Const)
/// * `coupon_rate` - Annual coupon rate (Const)
/// * `face_value` - Face/principal value (Const)
/// * `output` - Bond present value (Active)
#[cfg(feature = "enzyme-ad")]
#[autodiff(d_bond_pricing_kernel, Reverse, Duplicated, Const, Const, Const, Duplicated)]
pub fn bond_pricing_kernel(
    rates: &[f64],
    times: &[f64],
    coupon_rate: f64,
    face_value: f64,
    output: &mut f64,
) {
    debug_assert_eq!(rates.len(), times.len());
    debug_assert!(!rates.is_empty());

    let n = rates.len();
    let mut pv = 0.0;

    // Coupon payments
    for i in 0..n {
        let df = (-rates[i] * times[i]).exp();
        // Annual coupon payment (assume annual frequency)
        let coupon = face_value * coupon_rate;
        pv += coupon * df;
    }

    // Principal repayment at maturity (last period)
    let df_maturity = (-rates[n - 1] * times[n - 1]).exp();
    pv += face_value * df_maturity;

    *output = pv;
}

/// Fallback bond pricing kernel when enzyme-ad is disabled.
#[cfg(not(feature = "enzyme-ad"))]
pub fn bond_pricing_kernel(
    rates: &[f64],
    times: &[f64],
    coupon_rate: f64,
    face_value: f64,
    output: &mut f64,
) {
    debug_assert_eq!(rates.len(), times.len());
    debug_assert!(!rates.is_empty());

    let n = rates.len();
    let mut pv = 0.0;

    for i in 0..n {
        let df = (-rates[i] * times[i]).exp();
        let coupon = face_value * coupon_rate;
        pv += coupon * df;
    }

    let df_maturity = (-rates[n - 1] * times[n - 1]).exp();
    pv += face_value * df_maturity;

    *output = pv;
}

/// FRA (Forward Rate Agreement) pricing kernel.
///
/// Computes the present value of a FRA.
///
/// # Arguments
///
/// * `rates` - Zero rates: [short_rate, long_rate] (Active)
/// * `times` - Times: [t_short, t_long] (Const)
/// * `notional` - Notional amount (Const)
/// * `fra_rate` - Contracted FRA rate (Const)
/// * `output` - FRA present value (Active)
///
/// # PV Calculation
///
/// ```text
/// df_short = exp(-rates[0] * times[0])
/// df_long = exp(-rates[1] * times[1])
/// forward_rate = (df_short / df_long - 1) / (times[1] - times[0])
/// pv = notional * (forward_rate - fra_rate) * (times[1] - times[0]) * df_long
/// ```
#[cfg(feature = "enzyme-ad")]
#[autodiff(d_fra_pricing_kernel, Reverse, Duplicated, Const, Const, Const, Duplicated)]
pub fn fra_pricing_kernel(
    rates: &[f64],
    times: &[f64],
    notional: f64,
    fra_rate: f64,
    output: &mut f64,
) {
    debug_assert_eq!(rates.len(), 2);
    debug_assert_eq!(times.len(), 2);

    let df_short = (-rates[0] * times[0]).exp();
    let df_long = (-rates[1] * times[1]).exp();

    let accrual = times[1] - times[0];
    let forward_rate = (df_short / df_long - 1.0) / accrual;

    *output = notional * (forward_rate - fra_rate) * accrual * df_long;
}

/// Fallback FRA pricing kernel when enzyme-ad is disabled.
#[cfg(not(feature = "enzyme-ad"))]
pub fn fra_pricing_kernel(
    rates: &[f64],
    times: &[f64],
    notional: f64,
    fra_rate: f64,
    output: &mut f64,
) {
    debug_assert_eq!(rates.len(), 2);
    debug_assert_eq!(times.len(), 2);

    let df_short = (-rates[0] * times[0]).exp();
    let df_long = (-rates[1] * times[1]).exp();

    let accrual = times[1] - times[0];
    let forward_rate = (df_short / df_long - 1.0) / accrual;

    *output = notional * (forward_rate - fra_rate) * accrual * df_long;
}

// =============================================================================
// Finite Difference Fallback (for testing without enzyme-ad)
// =============================================================================

/// Compute gradients using finite differences.
///
/// This function is used when enzyme-ad is not available, providing
/// a fallback for testing and verification.
///
/// # Arguments
///
/// * `kernel` - The kernel function to differentiate
/// * `rates` - Input rates (will be bumped)
/// * `args` - Additional arguments (times, notionals, etc.)
/// * `bump_size` - Finite difference bump size (default: 1e-7)
///
/// # Returns
///
/// Vector of gradients, one per rate
pub fn finite_difference_gradients<F>(
    kernel: F,
    rates: &[f64],
    times: &[f64],
    notionals: &[f64],
    year_fractions: &[f64],
    fixed_rate: f64,
    bump_size: f64,
) -> Vec<f64>
where
    F: Fn(&[f64], &[f64], &[f64], &[f64], f64, &mut f64),
{
    let n = rates.len();
    let mut gradients = vec![0.0; n];
    let mut rates_bumped = rates.to_vec();

    for i in 0..n {
        // Bump up
        rates_bumped[i] = rates[i] + bump_size;
        let mut pv_up = 0.0;
        kernel(
            &rates_bumped,
            times,
            notionals,
            year_fractions,
            fixed_rate,
            &mut pv_up,
        );

        // Bump down
        rates_bumped[i] = rates[i] - bump_size;
        let mut pv_down = 0.0;
        kernel(
            &rates_bumped,
            times,
            notionals,
            year_fractions,
            fixed_rate,
            &mut pv_down,
        );

        // Central difference
        gradients[i] = (pv_up - pv_down) / (2.0 * bump_size);

        // Reset
        rates_bumped[i] = rates[i];
    }

    gradients
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Task 2.1: Pricing kernel tests (Requirements 2.1-2.6)
    // =========================================================================

    #[test]
    fn test_pricing_kernel_irs_basic() {
        // Simple case: flat rates, equal notionals
        let rates = [0.03, 0.03, 0.03];
        let times = [1.0, 2.0, 3.0];
        let notionals = [1_000_000.0, 1_000_000.0, 1_000_000.0];
        let year_fractions = [1.0, 1.0, 1.0];
        let fixed_rate = 0.03; // At-the-money

        let mut pv = 0.0;
        pricing_kernel_irs(
            &rates,
            &times,
            &notionals,
            &year_fractions,
            fixed_rate,
            &mut pv,
        );

        // ATM swap should have PV close to zero
        assert!(
            pv.abs() < 1.0,
            "ATM swap should have near-zero PV, got {}",
            pv
        );
    }

    #[test]
    fn test_pricing_kernel_irs_positive_pv() {
        // Floating rate > fixed rate -> positive PV (receive floating)
        let rates = [0.04, 0.04, 0.04];
        let times = [1.0, 2.0, 3.0];
        let notionals = [1_000_000.0, 1_000_000.0, 1_000_000.0];
        let year_fractions = [1.0, 1.0, 1.0];
        let fixed_rate = 0.02;

        let mut pv = 0.0;
        pricing_kernel_irs(
            &rates,
            &times,
            &notionals,
            &year_fractions,
            fixed_rate,
            &mut pv,
        );

        assert!(pv > 0.0, "Expected positive PV, got {}", pv);
    }

    #[test]
    fn test_pricing_kernel_irs_negative_pv() {
        // Floating rate < fixed rate -> negative PV
        let rates = [0.02, 0.02, 0.02];
        let times = [1.0, 2.0, 3.0];
        let notionals = [1_000_000.0, 1_000_000.0, 1_000_000.0];
        let year_fractions = [1.0, 1.0, 1.0];
        let fixed_rate = 0.04;

        let mut pv = 0.0;
        pricing_kernel_irs(
            &rates,
            &times,
            &notionals,
            &year_fractions,
            fixed_rate,
            &mut pv,
        );

        assert!(pv < 0.0, "Expected negative PV, got {}", pv);
    }

    #[test]
    fn test_discount_kernel_basic() {
        let rate = 0.05;
        let time = 1.0;
        let cashflow = 100.0;

        let mut pv = 0.0;
        discount_kernel(&rate, time, cashflow, &mut pv);

        let expected = 100.0 * (-0.05_f64).exp();
        assert!(
            (pv - expected).abs() < 1e-10,
            "Expected {}, got {}",
            expected,
            pv
        );
    }

    #[test]
    fn test_discount_kernel_zero_rate() {
        let rate = 0.0;
        let time = 1.0;
        let cashflow = 100.0;

        let mut pv = 0.0;
        discount_kernel(&rate, time, cashflow, &mut pv);

        // Zero rate means no discounting
        assert!((pv - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_bond_pricing_kernel_basic() {
        let rates = [0.03, 0.03, 0.03];
        let times = [1.0, 2.0, 3.0];
        let coupon_rate = 0.05;
        let face_value = 100.0;

        let mut pv = 0.0;
        bond_pricing_kernel(&rates, &times, coupon_rate, face_value, &mut pv);

        // Bond with coupon > discount rate should be worth more than face
        assert!(pv > face_value, "Expected PV > {}, got {}", face_value, pv);
    }

    #[test]
    fn test_bond_pricing_kernel_par_bond() {
        // When coupon rate equals discount rate, bond trades at par
        let rates = [0.05, 0.05, 0.05];
        let times = [1.0, 2.0, 3.0];
        let coupon_rate = 0.05;
        let face_value = 100.0;

        let mut pv = 0.0;
        bond_pricing_kernel(&rates, &times, coupon_rate, face_value, &mut pv);

        // Should be close to par (100)
        assert!(
            (pv - face_value).abs() < 5.0,
            "Expected PV close to {}, got {}",
            face_value,
            pv
        );
    }

    #[test]
    fn test_fra_pricing_kernel_basic() {
        let rates = [0.02, 0.03]; // 2% at 3M, 3% at 6M
        let times = [0.25, 0.5]; // 3M and 6M
        let notional = 1_000_000.0;
        let fra_rate = 0.04; // 4% FRA rate

        let mut pv = 0.0;
        fra_pricing_kernel(&rates, &times, notional, fra_rate, &mut pv);

        // Forward rate will be different from FRA rate, so PV != 0
        assert!(pv.abs() > 0.0);
    }

    #[test]
    fn test_fra_pricing_kernel_atm() {
        // When FRA rate equals implied forward rate, PV should be ~0
        let rates = [0.02_f64, 0.02]; // Flat curve
        let times = [0.25_f64, 0.5];
        let notional = 1_000_000.0;

        // Compute implied forward rate
        let df_short = (-rates[0] * times[0]).exp();
        let df_long = (-rates[1] * times[1]).exp();
        let accrual = times[1] - times[0];
        let implied_forward = (df_short / df_long - 1.0) / accrual;

        let mut pv = 0.0;
        fra_pricing_kernel(&rates, &times, notional, implied_forward, &mut pv);

        assert!(
            pv.abs() < 1.0,
            "ATM FRA should have near-zero PV, got {}",
            pv
        );
    }

    // =========================================================================
    // Finite difference gradient tests
    // =========================================================================

    #[test]
    fn test_finite_difference_gradients() {
        let rates = [0.03, 0.03, 0.03];
        let times = [1.0, 2.0, 3.0];
        let notionals = [1_000_000.0, 1_000_000.0, 1_000_000.0];
        let year_fractions = [1.0, 1.0, 1.0];
        let fixed_rate = 0.025;

        let gradients = finite_difference_gradients(
            pricing_kernel_irs,
            &rates,
            &times,
            &notionals,
            &year_fractions,
            fixed_rate,
            1e-7,
        );

        // All gradients should be non-zero (rate sensitivity)
        for (i, &grad) in gradients.iter().enumerate() {
            assert!(
                grad.abs() > 0.0,
                "Gradient {} should be non-zero, got {}",
                i,
                grad
            );
        }
    }

    #[test]
    fn test_gradients_sign() {
        // Higher rates -> lower PV (for receive floating swap when rates < fixed)
        // But we're computing ∂PV/∂rate, which can be positive or negative
        let rates = [0.02, 0.02, 0.02];
        let times = [1.0, 2.0, 3.0];
        let notionals = [1_000_000.0, 1_000_000.0, 1_000_000.0];
        let year_fractions = [1.0, 1.0, 1.0];
        let fixed_rate = 0.02; // ATM

        let gradients = finite_difference_gradients(
            pricing_kernel_irs,
            &rates,
            &times,
            &notionals,
            &year_fractions,
            fixed_rate,
            1e-7,
        );

        // At ATM, sensitivities should be positive (floating leg benefits from rate up)
        for (i, &grad) in gradients.iter().enumerate() {
            assert!(
                grad > 0.0,
                "ATM swap gradient {} should be positive, got {}",
                i,
                grad
            );
        }
    }

    // =========================================================================
    // Edge cases
    // =========================================================================

    #[test]
    fn test_empty_inputs() {
        // Empty inputs should produce zero PV
        let rates: [f64; 0] = [];
        let times: [f64; 0] = [];
        let notionals: [f64; 0] = [];
        let year_fractions: [f64; 0] = [];
        let fixed_rate = 0.03;

        let mut pv = 0.0;
        pricing_kernel_irs(
            &rates,
            &times,
            &notionals,
            &year_fractions,
            fixed_rate,
            &mut pv,
        );

        assert_eq!(pv, 0.0);
    }

    #[test]
    fn test_single_period() {
        let rates = [0.05];
        let times = [1.0];
        let notionals = [1_000_000.0];
        let year_fractions = [1.0];
        let fixed_rate = 0.03;

        let mut pv = 0.0;
        pricing_kernel_irs(
            &rates,
            &times,
            &notionals,
            &year_fractions,
            fixed_rate,
            &mut pv,
        );

        // Manual calculation
        let df = (-0.05_f64).exp();
        let expected = 1_000_000.0 * (0.05 - 0.03) * 1.0 * df;

        assert!(
            (pv - expected).abs() < 1e-6,
            "Expected {}, got {}",
            expected,
            pv
        );
    }
}
