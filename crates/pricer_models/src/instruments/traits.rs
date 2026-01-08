//! Instrument trait definitions.
//!
//! This module provides the core [`InstrumentTrait`] that all financial instruments
//! must implement, ensuring a unified interface across asset classes.
//!
//! # Design Philosophy
//!
//! The trait is designed for **static dispatch** using enum-based polymorphism
//! to ensure Enzyme AD compatibility at LLVM level. Do NOT use `Box<dyn InstrumentTrait>`.
//!
//! # Layer Boundaries
//!
//! This trait is in L2 (pricer_models) and provides:
//! - Instrument metadata (maturity, currency, notional)
//! - Simple payoff calculation (spot-based)
//! - Classification helpers (path dependency)
//!
//! Full pricing with market data and Greeks is handled in L3 (pricer_pricing).

use num_traits::Float;
use pricer_core::types::Currency;

/// Core trait for all financial instruments.
///
/// Provides a unified interface for instrument properties and basic payoff
/// calculations. This trait is intentionally minimal to avoid L3 dependencies.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Required Methods
///
/// - [`payoff`](InstrumentTrait::payoff) - Compute payoff at given spot price
/// - [`expiry`](InstrumentTrait::expiry) - Time to expiry in years
/// - [`currency`](InstrumentTrait::currency) - Settlement currency
///
/// # Provided Methods
///
/// - [`notional`](InstrumentTrait::notional) - Notional amount (default: 1.0)
/// - [`is_path_dependent`](InstrumentTrait::is_path_dependent) - Requires MC simulation
///
/// # Examples
///
/// ```
/// use pricer_models::instruments::InstrumentTrait;
/// use pricer_core::types::Currency;
/// use num_traits::Float;
///
/// struct SimpleCall<T: Float> {
///     strike: T,
///     expiry: T,
/// }
///
/// impl<T: Float> InstrumentTrait<T> for SimpleCall<T> {
///     fn payoff(&self, spot: T) -> T {
///         (spot - self.strike).max(T::zero())
///     }
///
///     fn expiry(&self) -> T {
///         self.expiry
///     }
///
///     fn currency(&self) -> Currency {
///         Currency::USD
///     }
/// }
///
/// let call = SimpleCall { strike: 100.0_f64, expiry: 1.0 };
/// assert!((call.payoff(110.0) - 10.0).abs() < 1e-10);
/// ```
pub trait InstrumentTrait<T: Float> {
    /// Compute the payoff at given spot price.
    ///
    /// For path-dependent instruments, this may represent the terminal payoff
    /// or a placeholder. Full path-dependent pricing requires Monte Carlo.
    ///
    /// # Arguments
    ///
    /// * `spot` - Current or terminal spot price
    ///
    /// # Returns
    ///
    /// Payoff value (may be negative for short positions)
    fn payoff(&self, spot: T) -> T;

    /// Return the time to expiry in years.
    ///
    /// For swaps and other multi-period instruments, this should return
    /// the final maturity date.
    fn expiry(&self) -> T;

    /// Return the settlement currency.
    fn currency(&self) -> Currency;

    /// Return the notional amount.
    ///
    /// Default implementation returns 1.0 for options where notional
    /// is implicit in the number of contracts.
    #[inline]
    fn notional(&self) -> T {
        T::one()
    }

    /// Return whether this instrument requires path-dependent pricing.
    ///
    /// Path-dependent instruments (Asian, Barrier, Lookback, etc.) require
    /// Monte Carlo simulation and cannot be priced with simple closed-form
    /// solutions.
    ///
    /// Default implementation returns `false`.
    #[inline]
    fn is_path_dependent(&self) -> bool {
        false
    }

    /// Return a human-readable instrument type name.
    ///
    /// Used for logging and error messages.
    fn type_name(&self) -> &'static str {
        "Unknown"
    }
}

/// Cashflow structure for instruments with scheduled payments.
///
/// Represents a single cashflow with payment date and amount.
/// Used by interest rate instruments, swaps, and bonds.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cashflow<T: Float> {
    /// Payment date as time in years from valuation date.
    pub payment_time: T,
    /// Cashflow amount (positive = receive, negative = pay).
    pub amount: T,
    /// Currency of the cashflow.
    pub currency: Currency,
}

impl<T: Float> Cashflow<T> {
    /// Create a new cashflow.
    ///
    /// # Arguments
    ///
    /// * `payment_time` - Time to payment in years
    /// * `amount` - Cashflow amount
    /// * `currency` - Payment currency
    #[inline]
    pub fn new(payment_time: T, amount: T, currency: Currency) -> Self {
        Self {
            payment_time,
            amount,
            currency,
        }
    }

    /// Return the present value of this cashflow given a discount factor.
    ///
    /// # Arguments
    ///
    /// * `discount_factor` - Discount factor to payment date
    #[inline]
    pub fn present_value(&self, discount_factor: T) -> T {
        self.amount * discount_factor
    }
}

/// Trait for instruments with scheduled cashflows.
///
/// Extended trait for interest rate products and other instruments
/// with deterministic cashflow schedules.
pub trait CashflowInstrument<T: Float>: InstrumentTrait<T> {
    /// Return all scheduled cashflows.
    ///
    /// Cashflows should be ordered by payment date.
    fn cashflows(&self) -> Vec<Cashflow<T>>;

    /// Return the number of cashflows.
    #[inline]
    fn num_cashflows(&self) -> usize {
        self.cashflows().len()
    }

    /// Return the total undiscounted cashflow amount.
    #[inline]
    fn total_cashflow(&self) -> T {
        self.cashflows()
            .iter()
            .fold(T::zero(), |acc, cf| acc + cf.amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // InstrumentTrait Tests
    // ========================================

    struct TestCall<T: Float> {
        strike: T,
        expiry_time: T,
        notional_amount: T,
    }

    impl<T: Float> InstrumentTrait<T> for TestCall<T> {
        fn payoff(&self, spot: T) -> T {
            (spot - self.strike).max(T::zero()) * self.notional_amount
        }

        fn expiry(&self) -> T {
            self.expiry_time
        }

        fn currency(&self) -> Currency {
            Currency::USD
        }

        fn notional(&self) -> T {
            self.notional_amount
        }

        fn type_name(&self) -> &'static str {
            "TestCall"
        }
    }

    struct TestPut<T: Float> {
        strike: T,
        expiry_time: T,
    }

    impl<T: Float> InstrumentTrait<T> for TestPut<T> {
        fn payoff(&self, spot: T) -> T {
            (self.strike - spot).max(T::zero())
        }

        fn expiry(&self) -> T {
            self.expiry_time
        }

        fn currency(&self) -> Currency {
            Currency::EUR
        }
    }

    struct TestAsian<T: Float> {
        strike: T,
        expiry_time: T,
    }

    impl<T: Float> InstrumentTrait<T> for TestAsian<T> {
        fn payoff(&self, spot: T) -> T {
            // Placeholder - real Asian uses average
            (spot - self.strike).max(T::zero())
        }

        fn expiry(&self) -> T {
            self.expiry_time
        }

        fn currency(&self) -> Currency {
            Currency::USD
        }

        fn is_path_dependent(&self) -> bool {
            true
        }

        fn type_name(&self) -> &'static str {
            "TestAsian"
        }
    }

    #[test]
    fn test_call_payoff_itm() {
        let call = TestCall {
            strike: 100.0_f64,
            expiry_time: 1.0,
            notional_amount: 1.0,
        };
        let payoff = call.payoff(110.0);
        assert!((payoff - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_call_payoff_otm() {
        let call = TestCall {
            strike: 100.0_f64,
            expiry_time: 1.0,
            notional_amount: 1.0,
        };
        let payoff = call.payoff(90.0);
        assert!(payoff.abs() < 1e-10);
    }

    #[test]
    fn test_call_payoff_atm() {
        let call = TestCall {
            strike: 100.0_f64,
            expiry_time: 1.0,
            notional_amount: 1.0,
        };
        let payoff = call.payoff(100.0);
        assert!(payoff.abs() < 1e-10);
    }

    #[test]
    fn test_call_with_notional() {
        let call = TestCall {
            strike: 100.0_f64,
            expiry_time: 1.0,
            notional_amount: 1000.0,
        };
        let payoff = call.payoff(110.0);
        assert!((payoff - 10000.0).abs() < 1e-10);
    }

    #[test]
    fn test_put_payoff_itm() {
        let put = TestPut {
            strike: 100.0_f64,
            expiry_time: 0.5,
        };
        let payoff = put.payoff(90.0);
        assert!((payoff - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_put_payoff_otm() {
        let put = TestPut {
            strike: 100.0_f64,
            expiry_time: 0.5,
        };
        let payoff = put.payoff(110.0);
        assert!(payoff.abs() < 1e-10);
    }

    #[test]
    fn test_expiry() {
        let call = TestCall {
            strike: 100.0_f64,
            expiry_time: 1.5,
            notional_amount: 1.0,
        };
        assert!((call.expiry() - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_currency() {
        let call = TestCall {
            strike: 100.0_f64,
            expiry_time: 1.0,
            notional_amount: 1.0,
        };
        assert_eq!(call.currency(), Currency::USD);

        let put = TestPut {
            strike: 100.0_f64,
            expiry_time: 0.5,
        };
        assert_eq!(put.currency(), Currency::EUR);
    }

    #[test]
    fn test_default_notional() {
        let put = TestPut {
            strike: 100.0_f64,
            expiry_time: 0.5,
        };
        assert!((put.notional() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_custom_notional() {
        let call = TestCall {
            strike: 100.0_f64,
            expiry_time: 1.0,
            notional_amount: 5000.0,
        };
        assert!((call.notional() - 5000.0).abs() < 1e-10);
    }

    #[test]
    fn test_is_path_dependent_default() {
        let call = TestCall {
            strike: 100.0_f64,
            expiry_time: 1.0,
            notional_amount: 1.0,
        };
        assert!(!call.is_path_dependent());
    }

    #[test]
    fn test_is_path_dependent_asian() {
        let asian = TestAsian {
            strike: 100.0_f64,
            expiry_time: 1.0,
        };
        assert!(asian.is_path_dependent());
    }

    #[test]
    fn test_type_name() {
        let call = TestCall {
            strike: 100.0_f64,
            expiry_time: 1.0,
            notional_amount: 1.0,
        };
        assert_eq!(call.type_name(), "TestCall");

        let asian = TestAsian {
            strike: 100.0_f64,
            expiry_time: 1.0,
        };
        assert_eq!(asian.type_name(), "TestAsian");
    }

    #[test]
    fn test_with_f32() {
        let call = TestCall {
            strike: 100.0_f32,
            expiry_time: 1.0_f32,
            notional_amount: 1.0_f32,
        };
        let payoff = call.payoff(110.0_f32);
        assert!((payoff - 10.0_f32).abs() < 1e-5);
    }

    // ========================================
    // Cashflow Tests
    // ========================================

    #[test]
    fn test_cashflow_new() {
        let cf = Cashflow::new(0.5_f64, 1000.0, Currency::USD);
        assert!((cf.payment_time - 0.5).abs() < 1e-10);
        assert!((cf.amount - 1000.0).abs() < 1e-10);
        assert_eq!(cf.currency, Currency::USD);
    }

    #[test]
    fn test_cashflow_present_value() {
        let cf = Cashflow::new(0.5_f64, 1000.0, Currency::USD);
        let pv = cf.present_value(0.975);
        assert!((pv - 975.0).abs() < 1e-10);
    }

    #[test]
    fn test_cashflow_negative_amount() {
        let cf = Cashflow::new(1.0_f64, -500.0, Currency::EUR);
        assert!((cf.amount - (-500.0)).abs() < 1e-10);
        let pv = cf.present_value(0.95);
        assert!((pv - (-475.0)).abs() < 1e-10);
    }

    #[test]
    fn test_cashflow_clone() {
        let cf1 = Cashflow::new(0.5_f64, 1000.0, Currency::USD);
        let cf2 = cf1.clone();
        assert_eq!(cf1, cf2);
    }

    // ========================================
    // CashflowInstrument Tests
    // ========================================

    struct TestSwap {
        fixed_rate: f64,
        notional: f64,
        payment_times: Vec<f64>,
    }

    impl InstrumentTrait<f64> for TestSwap {
        fn payoff(&self, _spot: f64) -> f64 {
            // Swaps don't have spot-based payoff
            0.0
        }

        fn expiry(&self) -> f64 {
            *self.payment_times.last().unwrap_or(&0.0)
        }

        fn currency(&self) -> Currency {
            Currency::USD
        }

        fn notional(&self) -> f64 {
            self.notional
        }

        fn type_name(&self) -> &'static str {
            "TestSwap"
        }
    }

    impl CashflowInstrument<f64> for TestSwap {
        fn cashflows(&self) -> Vec<Cashflow<f64>> {
            self.payment_times
                .iter()
                .map(|&t| {
                    let amount = self.notional * self.fixed_rate * 0.5; // semi-annual
                    Cashflow::new(t, amount, Currency::USD)
                })
                .collect()
        }
    }

    #[test]
    fn test_cashflow_instrument_cashflows() {
        let swap = TestSwap {
            fixed_rate: 0.05,
            notional: 1_000_000.0,
            payment_times: vec![0.5, 1.0, 1.5, 2.0],
        };
        let cfs = swap.cashflows();
        assert_eq!(cfs.len(), 4);
        // Each payment is notional * rate * 0.5 = 1M * 0.05 * 0.5 = 25,000
        assert!((cfs[0].amount - 25_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_cashflow_instrument_num_cashflows() {
        let swap = TestSwap {
            fixed_rate: 0.05,
            notional: 1_000_000.0,
            payment_times: vec![0.5, 1.0, 1.5, 2.0],
        };
        assert_eq!(swap.num_cashflows(), 4);
    }

    #[test]
    fn test_cashflow_instrument_total_cashflow() {
        let swap = TestSwap {
            fixed_rate: 0.05,
            notional: 1_000_000.0,
            payment_times: vec![0.5, 1.0, 1.5, 2.0],
        };
        let total = swap.total_cashflow();
        // 4 payments of 25,000 each = 100,000
        assert!((total - 100_000.0).abs() < 1e-6);
    }
}
