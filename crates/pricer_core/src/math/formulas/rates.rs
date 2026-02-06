//! Interest rate calculations from discount factors.
//!
//! This module provides pure mathematical functions for converting
//! between discount factors and various rate conventions.
//!
//! ## Supported Rate Types
//!
//! - **Zero Rate** (continuous compounding): r = -ln(DF) / T
//! - **Simple Forward Rate**: F = (DF₁ / DF₂ - 1) / τ
//! - **Continuous Forward Rate**: f = ln(DF₁ / DF₂) / τ
//!
//! ## Design Principles
//!
//! - **Generic over `T: Float`**: Supports both `f64` and AD types
//! - **AD Compatibility**: Avoids branching for tape consistency
//! - **Numerical Stability**: Handles edge cases appropriately

use num_traits::Float;

/// Computes the continuously-compounded zero rate from a discount factor.
///
/// # Formula
///
/// r = -ln(DF) / T
///
/// # Arguments
///
/// * `df` - Discount factor (must be positive)
/// * `time` - Time to maturity in years
///
/// # Returns
///
/// The continuously-compounded zero rate. Returns zero if time is zero
/// or negative (to avoid division by zero).
///
/// # Examples
///
/// ```
/// use pricer_core::math::formulas::rates::zero_rate_from_df;
///
/// // DF = e^(-0.05 * 1) ≈ 0.9512 → r ≈ 0.05
/// let df = (-0.05_f64).exp();
/// let rate = zero_rate_from_df(df, 1.0);
/// assert!((rate - 0.05).abs() < 1e-10);
/// ```
#[inline]
pub fn zero_rate_from_df<T: Float>(df: T, time: T) -> T {
    if time <= T::zero() {
        T::zero()
    } else {
        -df.ln() / time
    }
}

/// Computes the simply-compounded forward rate from two discount factors.
///
/// # Formula
///
/// F = (DF_start / DF_end - 1) / τ
///
/// where τ = end_time - start_time
///
/// # Arguments
///
/// * `df_start` - Discount factor at start of the period
/// * `df_end` - Discount factor at end of the period
/// * `tau` - Accrual period (year fraction)
///
/// # Returns
///
/// The simply-compounded forward rate for the period.
///
/// # Panics
///
/// Does not panic, but returns infinity or NaN for invalid inputs
/// (zero df_end or zero tau).
///
/// # Examples
///
/// ```
/// use pricer_core::math::formulas::rates::simple_forward_rate;
///
/// // DF(0.5) = 0.975, DF(1.0) = 0.95, tau = 0.5
/// // F = (0.975 / 0.95 - 1) / 0.5 ≈ 0.0526
/// let rate = simple_forward_rate(0.975_f64, 0.95, 0.5);
/// assert!((rate - 0.05263).abs() < 0.001);
/// ```
#[inline]
pub fn simple_forward_rate<T: Float>(df_start: T, df_end: T, tau: T) -> T {
    (df_start / df_end - T::one()) / tau
}

/// Computes the continuously-compounded forward rate from two discount factors.
///
/// # Formula
///
/// f = ln(DF_start / DF_end) / τ
///
/// where τ = end_time - start_time
///
/// # Arguments
///
/// * `df_start` - Discount factor at start of the period
/// * `df_end` - Discount factor at end of the period
/// * `tau` - Accrual period (year fraction)
///
/// # Returns
///
/// The continuously-compounded forward rate for the period.
///
/// # Examples
///
/// ```
/// use pricer_core::math::formulas::rates::continuous_forward_rate;
///
/// // DF(0.5) = e^(-0.04*0.5), DF(1.0) = e^(-0.05*1.0)
/// // Instantaneous forward rate between 0.5 and 1.0 years
/// let df_start = (-0.04_f64 * 0.5).exp();
/// let df_end = (-0.05_f64 * 1.0).exp();
/// let tau = 0.5;
/// let rate = continuous_forward_rate(df_start, df_end, tau);
/// // f = (0.05*1.0 - 0.04*0.5) / 0.5 = 0.06
/// assert!((rate - 0.06).abs() < 1e-10);
/// ```
#[inline]
pub fn continuous_forward_rate<T: Float>(df_start: T, df_end: T, tau: T) -> T {
    (df_start / df_end).ln() / tau
}

/// Computes the discount factor from a continuously-compounded zero rate.
///
/// # Formula
///
/// DF = e^(-r·T)
///
/// # Arguments
///
/// * `rate` - Continuously-compounded zero rate
/// * `time` - Time to maturity in years
///
/// # Returns
///
/// The discount factor.
///
/// # Examples
///
/// ```
/// use pricer_core::math::formulas::rates::df_from_zero_rate;
///
/// let df = df_from_zero_rate(0.05, 1.0);
/// assert!((df - (-0.05_f64).exp()).abs() < 1e-10);
/// ```
#[inline]
pub fn df_from_zero_rate<T: Float>(rate: T, time: T) -> T { (-rate * time).exp() }

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_zero_rate_from_df() {
        // DF = e^(-0.05 * 1) → r = 0.05
        let df = (-0.05_f64).exp();
        let rate = zero_rate_from_df(df, 1.0);
        assert_relative_eq!(rate, 0.05, epsilon = 1e-10);
    }

    #[test]
    fn test_zero_rate_from_df_two_years() {
        // DF = e^(-0.04 * 2) → r = 0.04
        let df = (-0.04_f64 * 2.0).exp();
        let rate = zero_rate_from_df(df, 2.0);
        assert_relative_eq!(rate, 0.04, epsilon = 1e-10);
    }

    #[test]
    fn test_zero_rate_from_df_zero_time() {
        // Time = 0 should return 0 to avoid division by zero
        let rate = zero_rate_from_df(0.95, 0.0);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn test_zero_rate_from_df_negative_time() {
        // Negative time should return 0
        let rate = zero_rate_from_df(0.95, -1.0);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn test_simple_forward_rate() {
        // DF_start = 0.975, DF_end = 0.95, tau = 0.5
        // F = (0.975 / 0.95 - 1) / 0.5 = 0.052631...
        let rate = simple_forward_rate(0.975, 0.95, 0.5);
        let expected = (0.975 / 0.95 - 1.0) / 0.5;
        assert_relative_eq!(rate, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_simple_forward_rate_one_year() {
        // Flat 5% curve: DF(0) = 1, DF(1) = e^(-0.05)
        // Simple forward rate from 0 to 1
        let df_start = 1.0_f64;
        let df_end = (-0.05_f64).exp();
        let tau = 1.0;
        let rate = simple_forward_rate(df_start, df_end, tau);
        // F = (1 / e^(-0.05) - 1) / 1 = e^0.05 - 1 ≈ 0.05127
        let expected = 0.05_f64.exp() - 1.0;
        assert_relative_eq!(rate, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_continuous_forward_rate() {
        // Flat 5% curve: continuous forward should be 5%
        let df_start = (-0.05_f64 * 0.5).exp();
        let df_end = (-0.05_f64 * 1.0).exp();
        let tau = 0.5;
        let rate = continuous_forward_rate(df_start, df_end, tau);
        assert_relative_eq!(rate, 0.05, epsilon = 1e-10);
    }

    #[test]
    fn test_continuous_forward_rate_upward_slope() {
        // Upward sloping: r(0.5) = 4%, r(1.0) = 5%
        // DF(0.5) = e^(-0.04*0.5), DF(1.0) = e^(-0.05*1.0)
        let df_start = (-0.04_f64 * 0.5).exp();
        let df_end = (-0.05_f64 * 1.0).exp();
        let tau = 0.5;
        let rate = continuous_forward_rate(df_start, df_end, tau);
        // f = ln(DF_start/DF_end) / tau = (0.05*1.0 - 0.04*0.5) / 0.5 = 0.06
        assert_relative_eq!(rate, 0.06, epsilon = 1e-10);
    }

    #[test]
    fn test_df_from_zero_rate() {
        let df = df_from_zero_rate(0.05, 1.0);
        let expected = (-0.05_f64).exp();
        assert_relative_eq!(df, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_df_from_zero_rate_two_years() {
        let df = df_from_zero_rate(0.04, 2.0);
        let expected = (-0.04_f64 * 2.0).exp();
        assert_relative_eq!(df, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_roundtrip_zero_rate() {
        // df → rate → df should be identity
        let original_df = 0.95_f64;
        let time = 1.0;
        let rate = zero_rate_from_df(original_df, time);
        let recovered_df = df_from_zero_rate(rate, time);
        assert_relative_eq!(original_df, recovered_df, epsilon = 1e-10);
    }
}
