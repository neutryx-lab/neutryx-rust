//! Swap contract definitions.
//!
//! This module provides interest rate swap structures
//! for fixed-for-floating rate exchanges.

use num_traits::Float;
use pricer_core::types::Currency;

use super::error::InstrumentError;

/// Payment frequency for swap contracts.
///
/// Defines the standard payment frequencies for interest rate swaps.
///
/// # Variants
/// - `Annual`: Once per year (frequency = 1)
/// - `SemiAnnual`: Twice per year (frequency = 2)
/// - `Quarterly`: Four times per year (frequency = 4)
/// - `Monthly`: Twelve times per year (frequency = 12)
///
/// # Examples
/// ```
/// use pricer_models::instruments::PaymentFrequency;
///
/// let freq = PaymentFrequency::Quarterly;
/// assert_eq!(freq.periods_per_year(), 4);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaymentFrequency {
    /// Annual payments (once per year)
    Annual,
    /// Semi-annual payments (twice per year)
    SemiAnnual,
    /// Quarterly payments (four times per year)
    Quarterly,
    /// Monthly payments (twelve times per year)
    Monthly,
}

impl PaymentFrequency {
    /// Returns the number of payment periods per year.
    ///
    /// # Examples
    /// ```
    /// use pricer_models::instruments::PaymentFrequency;
    ///
    /// assert_eq!(PaymentFrequency::Annual.periods_per_year(), 1);
    /// assert_eq!(PaymentFrequency::SemiAnnual.periods_per_year(), 2);
    /// assert_eq!(PaymentFrequency::Quarterly.periods_per_year(), 4);
    /// assert_eq!(PaymentFrequency::Monthly.periods_per_year(), 12);
    /// ```
    #[inline]
    pub fn periods_per_year(&self) -> u32 {
        match self {
            PaymentFrequency::Annual => 1,
            PaymentFrequency::SemiAnnual => 2,
            PaymentFrequency::Quarterly => 4,
            PaymentFrequency::Monthly => 12,
        }
    }

    /// Returns the year fraction for one payment period.
    ///
    /// # Examples
    /// ```
    /// use pricer_models::instruments::PaymentFrequency;
    ///
    /// let annual = PaymentFrequency::Annual.period_fraction::<f64>();
    /// assert!((annual - 1.0).abs() < 1e-10);
    ///
    /// let quarterly = PaymentFrequency::Quarterly.period_fraction::<f64>();
    /// assert!((quarterly - 0.25).abs() < 1e-10);
    /// ```
    #[inline]
    pub fn period_fraction<T: Float>(&self) -> T {
        T::one() / T::from(self.periods_per_year()).unwrap()
    }
}

/// Interest rate swap contract.
///
/// Represents a fixed-for-floating interest rate swap with specified
/// payment schedule and notional amount.
///
/// # Type Parameters
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Examples
/// ```
/// use pricer_models::instruments::{Swap, PaymentFrequency};
/// use pricer_core::types::Currency;
///
/// // Create a 5-year quarterly swap
/// let payment_dates: Vec<f64> = (1..=20).map(|i| 0.25 * i as f64).collect();
/// let swap = Swap::new(
///     1_000_000.0,
///     0.02,
///     payment_dates,
///     PaymentFrequency::Quarterly,
///     Currency::USD,
/// ).unwrap();
///
/// assert_eq!(swap.notional(), 1_000_000.0);
/// assert_eq!(swap.fixed_rate(), 0.02);
/// ```
#[derive(Debug, Clone)]
pub struct Swap<T: Float> {
    notional: T,
    fixed_rate: T,
    payment_dates: Vec<T>,
    frequency: PaymentFrequency,
    currency: Currency,
}

impl<T: Float> Swap<T> {
    /// Creates a new swap contract.
    ///
    /// # Arguments
    /// * `notional` - Notional principal amount (must be positive)
    /// * `fixed_rate` - Fixed interest rate (can be positive or negative)
    /// * `payment_dates` - Vector of payment times in years (must be sorted ascending)
    /// * `frequency` - Payment frequency
    /// * `currency` - Currency denomination
    ///
    /// # Returns
    /// `Ok(Swap)` if parameters are valid,
    /// `Err(InstrumentError)` otherwise.
    ///
    /// # Errors
    /// - `InvalidNotional`: If notional is non-positive
    /// - `InvalidParameter`: If payment_dates are not sorted or empty
    ///
    /// # Examples
    /// ```
    /// use pricer_models::instruments::{Swap, PaymentFrequency};
    /// use pricer_core::types::Currency;
    ///
    /// let dates = vec![0.5, 1.0, 1.5, 2.0];
    /// let swap = Swap::new(1_000_000.0, 0.025, dates, PaymentFrequency::SemiAnnual, Currency::EUR);
    /// assert!(swap.is_ok());
    ///
    /// // Empty payment dates
    /// let invalid = Swap::new(1_000_000.0, 0.025, vec![], PaymentFrequency::Annual, Currency::USD);
    /// assert!(invalid.is_err());
    /// ```
    pub fn new(
        notional: T,
        fixed_rate: T,
        payment_dates: Vec<T>,
        frequency: PaymentFrequency,
        currency: Currency,
    ) -> Result<Self, InstrumentError> {
        let zero = T::zero();

        // Validate notional is positive
        if notional <= zero {
            return Err(InstrumentError::InvalidNotional {
                notional: notional.to_f64().unwrap_or(f64::NAN),
            });
        }

        // Validate payment dates are not empty
        if payment_dates.is_empty() {
            return Err(InstrumentError::InvalidParameter {
                message: "Payment dates must not be empty".to_string(),
            });
        }

        // Validate payment dates are sorted in ascending order
        for i in 1..payment_dates.len() {
            if payment_dates[i] <= payment_dates[i - 1] {
                return Err(InstrumentError::InvalidParameter {
                    message: "Payment dates must be sorted in ascending order".to_string(),
                });
            }
        }

        // Validate all payment dates are positive
        if payment_dates[0] <= zero {
            return Err(InstrumentError::InvalidParameter {
                message: "All payment dates must be positive".to_string(),
            });
        }

        Ok(Self {
            notional,
            fixed_rate,
            payment_dates,
            frequency,
            currency,
        })
    }

    /// Returns the notional principal amount.
    #[inline]
    pub fn notional(&self) -> T {
        self.notional
    }

    /// Returns the fixed interest rate.
    #[inline]
    pub fn fixed_rate(&self) -> T {
        self.fixed_rate
    }

    /// Returns a reference to the payment dates.
    #[inline]
    pub fn payment_dates(&self) -> &[T] {
        &self.payment_dates
    }

    /// Returns the payment frequency.
    #[inline]
    pub fn frequency(&self) -> PaymentFrequency {
        self.frequency
    }

    /// Returns the currency denomination.
    #[inline]
    pub fn currency(&self) -> Currency {
        self.currency
    }

    /// Returns the number of remaining payment periods.
    #[inline]
    pub fn num_periods(&self) -> usize {
        self.payment_dates.len()
    }

    /// Returns the maturity (last payment date).
    ///
    /// # Panics
    /// This method assumes payment_dates is non-empty, which is enforced
    /// by the constructor.
    #[inline]
    pub fn maturity(&self) -> T {
        // Safe because constructor validates non-empty payment_dates
        *self.payment_dates.last().unwrap()
    }

    /// Calculates the fixed leg cash flow for a single period.
    ///
    /// # Arguments
    /// * `year_fraction` - The accrual period as a fraction of a year
    ///
    /// # Returns
    /// notional * fixed_rate * year_fraction
    #[inline]
    pub fn fixed_leg_cashflow(&self, year_fraction: T) -> T {
        self.notional * self.fixed_rate * year_fraction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_payment_frequency_periods_per_year() {
        assert_eq!(PaymentFrequency::Annual.periods_per_year(), 1);
        assert_eq!(PaymentFrequency::SemiAnnual.periods_per_year(), 2);
        assert_eq!(PaymentFrequency::Quarterly.periods_per_year(), 4);
        assert_eq!(PaymentFrequency::Monthly.periods_per_year(), 12);
    }

    #[test]
    fn test_payment_frequency_period_fraction() {
        assert_relative_eq!(
            PaymentFrequency::Annual.period_fraction::<f64>(),
            1.0,
            epsilon = 1e-10
        );
        assert_relative_eq!(
            PaymentFrequency::SemiAnnual.period_fraction::<f64>(),
            0.5,
            epsilon = 1e-10
        );
        assert_relative_eq!(
            PaymentFrequency::Quarterly.period_fraction::<f64>(),
            0.25,
            epsilon = 1e-10
        );
        assert_relative_eq!(
            PaymentFrequency::Monthly.period_fraction::<f64>(),
            1.0 / 12.0,
            epsilon = 1e-10
        );
    }

    #[test]
    fn test_payment_frequency_clone_copy() {
        let freq1 = PaymentFrequency::Quarterly;
        let freq2 = freq1; // Copy
        assert_eq!(freq1, freq2);
    }

    #[test]
    fn test_payment_frequency_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(PaymentFrequency::Annual);
        set.insert(PaymentFrequency::Quarterly);
        set.insert(PaymentFrequency::Annual); // Duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_new_valid_swap() {
        let dates = vec![0.5_f64, 1.0, 1.5, 2.0];
        let swap = Swap::new(
            1_000_000.0,
            0.025,
            dates,
            PaymentFrequency::SemiAnnual,
            Currency::USD,
        )
        .unwrap();

        assert_eq!(swap.notional(), 1_000_000.0);
        assert_eq!(swap.fixed_rate(), 0.025);
        assert_eq!(swap.payment_dates().len(), 4);
        assert_eq!(swap.frequency(), PaymentFrequency::SemiAnnual);
        assert_eq!(swap.currency(), Currency::USD);
        assert_eq!(swap.num_periods(), 4);
        assert_eq!(swap.maturity(), 2.0);
    }

    #[test]
    fn test_new_invalid_notional_negative() {
        let dates = vec![0.5_f64, 1.0];
        let result = Swap::new(
            -1_000_000.0,
            0.025,
            dates,
            PaymentFrequency::SemiAnnual,
            Currency::USD,
        );
        assert!(matches!(
            result,
            Err(InstrumentError::InvalidNotional { .. })
        ));
    }

    #[test]
    fn test_new_invalid_notional_zero() {
        let dates = vec![0.5_f64, 1.0];
        let result = Swap::new(
            0.0,
            0.025,
            dates,
            PaymentFrequency::SemiAnnual,
            Currency::USD,
        );
        assert!(matches!(
            result,
            Err(InstrumentError::InvalidNotional { .. })
        ));
    }

    #[test]
    fn test_new_empty_payment_dates() {
        let result = Swap::new(
            1_000_000.0_f64,
            0.025,
            vec![],
            PaymentFrequency::SemiAnnual,
            Currency::USD,
        );
        match result {
            Err(InstrumentError::InvalidParameter { message }) => {
                assert!(message.contains("empty"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[test]
    fn test_new_unsorted_payment_dates() {
        let dates = vec![1.0_f64, 0.5, 1.5]; // Not sorted
        let result = Swap::new(
            1_000_000.0,
            0.025,
            dates,
            PaymentFrequency::SemiAnnual,
            Currency::USD,
        );
        match result {
            Err(InstrumentError::InvalidParameter { message }) => {
                assert!(message.contains("sorted") || message.contains("ascending"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[test]
    fn test_new_duplicate_payment_dates() {
        let dates = vec![0.5_f64, 1.0, 1.0, 1.5]; // Duplicate
        let result = Swap::new(
            1_000_000.0,
            0.025,
            dates,
            PaymentFrequency::SemiAnnual,
            Currency::USD,
        );
        assert!(matches!(
            result,
            Err(InstrumentError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn test_new_non_positive_payment_date() {
        let dates = vec![-0.5_f64, 0.5, 1.0];
        let result = Swap::new(
            1_000_000.0,
            0.025,
            dates,
            PaymentFrequency::SemiAnnual,
            Currency::USD,
        );
        match result {
            Err(InstrumentError::InvalidParameter { message }) => {
                assert!(message.contains("positive"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[test]
    fn test_new_zero_payment_date() {
        let dates = vec![0.0_f64, 0.5, 1.0];
        let result = Swap::new(
            1_000_000.0,
            0.025,
            dates,
            PaymentFrequency::SemiAnnual,
            Currency::USD,
        );
        assert!(matches!(
            result,
            Err(InstrumentError::InvalidParameter { .. })
        ));
    }

    #[test]
    fn test_negative_fixed_rate() {
        // Negative rates are allowed (common in some markets)
        let dates = vec![0.5_f64, 1.0];
        let swap = Swap::new(
            1_000_000.0,
            -0.01,
            dates,
            PaymentFrequency::SemiAnnual,
            Currency::EUR,
        )
        .unwrap();
        assert_eq!(swap.fixed_rate(), -0.01);
    }

    #[test]
    fn test_fixed_leg_cashflow() {
        let dates = vec![0.5_f64, 1.0];
        let swap = Swap::new(
            1_000_000.0,
            0.04, // 4%
            dates,
            PaymentFrequency::SemiAnnual,
            Currency::USD,
        )
        .unwrap();

        // For semi-annual: cashflow = 1M * 0.04 * 0.5 = 20,000
        let cashflow = swap.fixed_leg_cashflow(0.5);
        assert_relative_eq!(cashflow, 20_000.0, epsilon = 1e-10);
    }

    #[test]
    fn test_maturity() {
        let dates = vec![0.25_f64, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0];
        let swap = Swap::new(
            1_000_000.0,
            0.03,
            dates,
            PaymentFrequency::Quarterly,
            Currency::GBP,
        )
        .unwrap();

        assert_relative_eq!(swap.maturity(), 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_different_currencies() {
        let dates = vec![1.0_f64];

        let usd_swap = Swap::new(1_000_000.0, 0.02, dates.clone(), PaymentFrequency::Annual, Currency::USD).unwrap();
        assert_eq!(usd_swap.currency(), Currency::USD);

        let eur_swap = Swap::new(1_000_000.0, 0.02, dates.clone(), PaymentFrequency::Annual, Currency::EUR).unwrap();
        assert_eq!(eur_swap.currency(), Currency::EUR);

        let jpy_swap = Swap::new(100_000_000.0, 0.001, dates.clone(), PaymentFrequency::Annual, Currency::JPY).unwrap();
        assert_eq!(jpy_swap.currency(), Currency::JPY);
    }

    #[test]
    fn test_f32_compatibility() {
        let dates = vec![0.5_f32, 1.0];
        let swap = Swap::new(
            1_000_000.0_f32,
            0.025,
            dates,
            PaymentFrequency::SemiAnnual,
            Currency::USD,
        )
        .unwrap();
        assert!((swap.notional() - 1_000_000.0_f32).abs() < 1.0);
    }

    #[test]
    fn test_clone() {
        let dates = vec![0.5_f64, 1.0];
        let swap1 = Swap::new(
            1_000_000.0,
            0.025,
            dates,
            PaymentFrequency::SemiAnnual,
            Currency::USD,
        )
        .unwrap();
        let swap2 = swap1.clone();
        assert_eq!(swap1.notional(), swap2.notional());
        assert_eq!(swap1.fixed_rate(), swap2.fixed_rate());
        assert_eq!(swap1.payment_dates(), swap2.payment_dates());
    }

    #[test]
    fn test_debug() {
        let dates = vec![0.5_f64, 1.0];
        let swap = Swap::new(
            1_000_000.0,
            0.025,
            dates,
            PaymentFrequency::SemiAnnual,
            Currency::USD,
        )
        .unwrap();
        let debug_str = format!("{:?}", swap);
        assert!(debug_str.contains("Swap"));
        assert!(debug_str.contains("notional"));
        assert!(debug_str.contains("SemiAnnual"));
    }

    // AD compatibility test with Dual64
    #[test]
    fn test_dual64_compatibility() {
        use num_dual::Dual64;

        let payment_dates = vec![
            Dual64::new(0.5, 0.0),
            Dual64::new(1.0, 0.0),
            Dual64::new(1.5, 0.0),
            Dual64::new(2.0, 0.0),
        ];

        let swap = Swap::new(
            Dual64::new(1_000_000.0, 0.0),
            Dual64::new(0.04, 1.0), // Track derivative w.r.t. rate
            payment_dates,
            PaymentFrequency::SemiAnnual,
            Currency::USD,
        )
        .unwrap();

        // Fixed leg cashflow with rate derivative tracking
        let year_fraction = Dual64::new(0.5, 0.0);
        let cashflow = swap.fixed_leg_cashflow(year_fraction);

        // Cashflow = notional * rate * year_fraction = 1M * 0.04 * 0.5 = 20,000
        assert!((cashflow.re - 20_000.0).abs() < 1e-6);

        // d(cashflow)/d(rate) = notional * year_fraction = 1M * 0.5 = 500,000
        assert!((cashflow.eps - 500_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_dual64_notional_sensitivity() {
        use num_dual::Dual64;

        let payment_dates = vec![
            Dual64::new(0.5, 0.0),
            Dual64::new(1.0, 0.0),
        ];

        let swap = Swap::new(
            Dual64::new(1_000_000.0, 1.0), // Track derivative w.r.t. notional
            Dual64::new(0.04, 0.0),
            payment_dates,
            PaymentFrequency::SemiAnnual,
            Currency::USD,
        )
        .unwrap();

        let year_fraction = Dual64::new(0.5, 0.0);
        let cashflow = swap.fixed_leg_cashflow(year_fraction);

        // d(cashflow)/d(notional) = rate * year_fraction = 0.04 * 0.5 = 0.02
        assert!((cashflow.eps - 0.02).abs() < 1e-10);
    }
}
