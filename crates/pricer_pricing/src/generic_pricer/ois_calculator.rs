//! OIS (Overnight Index Swap) rate calculation utilities.
//!
//! This module provides the `OisCalculator` for computing compounded
//! overnight rates from daily accruals.

use num_traits::Float;

/// Daily accrual record for OIS compounding.
///
/// Represents a single day's contribution to the compounded rate.
#[derive(Debug, Clone, PartialEq)]
pub struct DailyAccrual {
    /// The overnight rate for this day (as a decimal, e.g., 0.035 for 3.5%)
    pub overnight_rate: f64,
    /// Day count fraction for this day (typically 1/360 or 1/365)
    pub day_fraction: f64,
}

impl DailyAccrual {
    /// Creates a new daily accrual record.
    ///
    /// # Arguments
    ///
    /// * `overnight_rate` - The overnight rate (decimal)
    /// * `day_fraction` - The day count fraction
    pub fn new(overnight_rate: f64, day_fraction: f64) -> Self {
        Self {
            overnight_rate,
            day_fraction,
        }
    }
}

/// Calculator for OIS (Overnight Index Swap) rate compounding.
///
/// Provides methods for calculating compounded rates from daily
/// overnight rate observations, following ISDA/SOFR conventions.
///
/// # Example
///
/// ```
/// use pricer_pricing::generic_pricer::{OisCalculator, DailyAccrual};
///
/// // Three days of overnight fixings at 3.5%
/// let accruals = vec![
///     DailyAccrual::new(0.035, 1.0 / 360.0),
///     DailyAccrual::new(0.035, 1.0 / 360.0),
///     DailyAccrual::new(0.035, 1.0 / 360.0),
/// ];
///
/// let compounded_rate = OisCalculator::compound_rate::<f64>(&accruals);
/// let annualized = OisCalculator::annualized_rate(compounded_rate, 3.0 / 360.0);
///
/// // Should be approximately 3.5% (with small compounding effect)
/// assert!((annualized - 0.035).abs() < 1e-4);
/// ```
pub struct OisCalculator;

impl OisCalculator {
    /// Calculates the compounded rate from daily accruals.
    ///
    /// Uses the ISDA compounding formula:
    /// ```text
    /// ∏(1 + r_i × δ_i) - 1
    /// ```
    ///
    /// where:
    /// - r_i is the overnight rate for day i
    /// - δ_i is the day count fraction for day i
    ///
    /// # Arguments
    ///
    /// * `daily_accruals` - Slice of daily accrual records
    ///
    /// # Returns
    ///
    /// The compounded rate (not annualised)
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_pricing::generic_pricer::{OisCalculator, DailyAccrual};
    ///
    /// let accruals = vec![
    ///     DailyAccrual::new(0.035, 1.0 / 360.0),
    ///     DailyAccrual::new(0.036, 1.0 / 360.0),
    /// ];
    ///
    /// let compounded = OisCalculator::compound_rate::<f64>(&accruals);
    /// assert!(compounded > 0.0);
    /// ```
    pub fn compound_rate<T: Float>(daily_accruals: &[DailyAccrual]) -> T {
        if daily_accruals.is_empty() {
            return T::zero();
        }

        let mut product = T::one();
        for accrual in daily_accruals {
            let rate = T::from(accrual.overnight_rate).unwrap_or_else(T::zero);
            let day_fraction = T::from(accrual.day_fraction).unwrap_or_else(T::zero);
            product = product * (T::one() + rate * day_fraction);
        }
        product - T::one()
    }

    /// Converts a compounded rate to an annualised rate.
    ///
    /// # Arguments
    ///
    /// * `compounded_rate` - The compounded rate (from `compound_rate`)
    /// * `total_year_fraction` - Total period year fraction
    ///
    /// # Returns
    ///
    /// The annualised rate
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_pricing::generic_pricer::OisCalculator;
    ///
    /// // 0.5% compounded over 0.5 years => ~1% annualised
    /// let annualized = OisCalculator::annualized_rate::<f64>(0.005, 0.5);
    /// assert!((annualized - 0.01).abs() < 1e-10);
    /// ```
    pub fn annualized_rate<T: Float>(compounded_rate: T, total_year_fraction: T) -> T {
        if total_year_fraction <= T::zero() {
            return T::zero();
        }
        compounded_rate / total_year_fraction
    }

    /// Calculates the compounded rate and returns both the rate and
    /// the cumulative notional at each step.
    ///
    /// This is useful for debugging or detailed cashflow analysis.
    ///
    /// # Arguments
    ///
    /// * `daily_accruals` - Slice of daily accrual records
    /// * `initial_notional` - Starting notional amount
    ///
    /// # Returns
    ///
    /// A vector of (cumulative_notional, compounded_rate_so_far) tuples
    pub fn compound_rate_with_history<T: Float>(
        daily_accruals: &[DailyAccrual],
        initial_notional: T,
    ) -> Vec<(T, T)> {
        let mut history = Vec::with_capacity(daily_accruals.len());
        let mut product = T::one();
        let mut notional = initial_notional;

        for accrual in daily_accruals {
            let rate = T::from(accrual.overnight_rate).unwrap_or_else(T::zero);
            let day_fraction = T::from(accrual.day_fraction).unwrap_or_else(T::zero);
            let growth = T::one() + rate * day_fraction;
            product = product * growth;
            notional = notional * growth;
            history.push((notional, product - T::one()));
        }

        history
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // DailyAccrual Tests
    // ========================================

    #[test]
    fn test_daily_accrual_new() {
        let accrual = DailyAccrual::new(0.035, 1.0 / 360.0);
        assert!((accrual.overnight_rate - 0.035).abs() < 1e-10);
        assert!((accrual.day_fraction - 1.0 / 360.0).abs() < 1e-10);
    }

    #[test]
    fn test_daily_accrual_clone() {
        let accrual = DailyAccrual::new(0.035, 1.0 / 360.0);
        let cloned = accrual.clone();
        assert_eq!(accrual, cloned);
    }

    // ========================================
    // Compound Rate Tests
    // ========================================

    #[test]
    fn test_compound_rate_empty() {
        let accruals: Vec<DailyAccrual> = vec![];
        let rate = OisCalculator::compound_rate::<f64>(&accruals);
        assert!((rate - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_compound_rate_single_day() {
        let accruals = vec![DailyAccrual::new(0.035, 1.0 / 360.0)];
        let rate = OisCalculator::compound_rate::<f64>(&accruals);

        // Single day: (1 + 0.035 / 360) - 1 ≈ 0.035 / 360
        let expected = 0.035 / 360.0;
        assert!((rate - expected).abs() < 1e-12);
    }

    #[test]
    fn test_compound_rate_multiple_days_constant_rate() {
        // 90 days at constant 3.5%
        let accruals: Vec<DailyAccrual> = (0..90)
            .map(|_| DailyAccrual::new(0.035, 1.0 / 360.0))
            .collect();

        let compounded = OisCalculator::compound_rate::<f64>(&accruals);
        let annualized = OisCalculator::annualized_rate(compounded, 90.0 / 360.0);

        // Compounding effect: the annualized rate is slightly higher than 3.5%
        // For 90 days at 3.5%, the compounded rate is (1 + 0.035/360)^90 - 1
        // Annualized: ~3.501% (compounding effect)
        // Using 10bp tolerance to account for the compounding
        assert!(
            annualized > 0.035 && annualized < 0.036,
            "Annualized rate should be close to 3.5%, got {}",
            annualized
        );
    }

    #[test]
    fn test_compound_rate_varying_rates() {
        // Three days with different rates
        let accruals = vec![
            DailyAccrual::new(0.030, 1.0 / 360.0), // 3.0%
            DailyAccrual::new(0.035, 1.0 / 360.0), // 3.5%
            DailyAccrual::new(0.040, 1.0 / 360.0), // 4.0%
        ];

        let compounded = OisCalculator::compound_rate::<f64>(&accruals);

        // Manual calculation:
        // (1 + 0.030/360) * (1 + 0.035/360) * (1 + 0.040/360) - 1
        let expected = (1.0 + 0.030 / 360.0) * (1.0 + 0.035 / 360.0) * (1.0 + 0.040 / 360.0) - 1.0;
        assert!((compounded - expected).abs() < 1e-15);
    }

    #[test]
    fn test_compound_rate_weekend_treatment() {
        // Friday to Monday: 3 days worth of accrual on Monday
        let accruals = vec![
            DailyAccrual::new(0.035, 1.0 / 360.0), // Thursday
            DailyAccrual::new(0.035, 3.0 / 360.0), // Friday (applies over weekend)
            DailyAccrual::new(0.035, 1.0 / 360.0), // Monday
        ];

        let compounded = OisCalculator::compound_rate::<f64>(&accruals);

        // 5 days of interest earned over 3 calendar days
        let expected =
            (1.0 + 0.035 / 360.0) * (1.0 + 3.0 * 0.035 / 360.0) * (1.0 + 0.035 / 360.0) - 1.0;
        assert!((compounded - expected).abs() < 1e-15);
    }

    // ========================================
    // Annualized Rate Tests
    // ========================================

    #[test]
    fn test_annualized_rate_zero_period() {
        let rate = OisCalculator::annualized_rate::<f64>(0.005, 0.0);
        assert!((rate - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_annualized_rate_negative_period() {
        let rate = OisCalculator::annualized_rate::<f64>(0.005, -0.25);
        assert!((rate - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_annualized_rate_normal() {
        // 0.875% compounded over 0.25 years => 3.5% annualized
        let compounded = 0.035 * 0.25; // Simplified
        let annualized = OisCalculator::annualized_rate::<f64>(compounded, 0.25);
        assert!((annualized - 0.035).abs() < 1e-10);
    }

    #[test]
    fn test_annualized_rate_full_year() {
        // 3.5% compounded over 1 year => 3.5% annualized
        let annualized = OisCalculator::annualized_rate::<f64>(0.035, 1.0);
        assert!((annualized - 0.035).abs() < 1e-10);
    }

    // ========================================
    // Compound Rate With History Tests
    // ========================================

    #[test]
    fn test_compound_rate_with_history() {
        let accruals = vec![
            DailyAccrual::new(0.035, 1.0 / 360.0),
            DailyAccrual::new(0.035, 1.0 / 360.0),
            DailyAccrual::new(0.035, 1.0 / 360.0),
        ];

        let history = OisCalculator::compound_rate_with_history::<f64>(&accruals, 1_000_000.0);

        assert_eq!(history.len(), 3);

        // Each day should increase the notional
        assert!(history[0].0 > 1_000_000.0);
        assert!(history[1].0 > history[0].0);
        assert!(history[2].0 > history[1].0);

        // Final compounded rate should match compound_rate function
        let expected_rate = OisCalculator::compound_rate::<f64>(&accruals);
        assert!((history[2].1 - expected_rate).abs() < 1e-15);
    }

    // ========================================
    // Integration Tests
    // ========================================

    #[test]
    fn test_sofr_quarterly_payment() {
        // Simulate a quarterly SOFR payment with 90 days
        let accruals: Vec<DailyAccrual> = (0..90)
            .map(|_| DailyAccrual::new(0.0525, 1.0 / 360.0)) // 5.25% SOFR
            .collect();

        let compounded = OisCalculator::compound_rate::<f64>(&accruals);
        let annualized = OisCalculator::annualized_rate(compounded, 90.0 / 360.0);

        // Annualized rate should be close to the input rate
        assert!((annualized - 0.0525).abs() < 1e-3);

        // Payment calculation for $10M notional
        let notional = 10_000_000.0_f64;
        let payment = notional * compounded;

        // Expected: ~$10M * 5.25% * 0.25 = ~$131,250
        assert!((payment - 131_250.0).abs() < 1000.0);
    }
}
