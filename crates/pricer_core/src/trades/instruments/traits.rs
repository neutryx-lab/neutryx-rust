//! Instrument trait definitions.

use num_traits::Float;

#[allow(deprecated)]
use crate::types::Currency;

/// Core trait for all financial instruments.
pub trait InstrumentTrait<T: Float> {
    /// Compute the payoff at given spot price.
    fn payoff(&self, spot: T) -> T;

    /// Return the time to expiry in years.
    fn expiry(&self) -> T;

    /// Return the settlement currency.
    fn currency(&self) -> Currency;

    /// Return the notional amount.
    #[inline]
    fn notional(&self) -> T { T::one() }

    /// Return whether this instrument requires path-dependent pricing.
    #[inline]
    fn is_path_dependent(&self) -> bool { false }

    /// Return a human-readable instrument type name.
    fn type_name(&self) -> &'static str { "Unknown" }
}

/// Cashflow structure for instruments with scheduled payments.
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
    #[inline]
    pub fn new(payment_time: T, amount: T, currency: Currency) -> Self {
        Self {
            payment_time,
            amount,
            currency,
        }
    }

    /// Return the present value of this cashflow given a discount factor.
    #[inline]
    pub fn present_value(&self, discount_factor: T) -> T { self.amount * discount_factor }
}

/// Trait for instruments with scheduled cashflows.
pub trait CashflowInstrument<T: Float>: InstrumentTrait<T> {
    /// Return all scheduled cashflows.
    fn cashflows(&self) -> Vec<Cashflow<T>>;

    /// Return the number of cashflows.
    #[inline]
    fn num_cashflows(&self) -> usize { self.cashflows().len() }

    /// Return the total undiscounted cashflow amount.
    #[inline]
    fn total_cashflow(&self) -> T {
        self.cashflows()
            .iter()
            .fold(T::zero(), |acc, cf| acc + cf.amount)
    }
}
