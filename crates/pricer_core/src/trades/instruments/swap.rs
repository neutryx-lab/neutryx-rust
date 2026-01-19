//! Swap contract definitions.

use num_traits::Float;

use crate::math::numeric::from_i32;

#[allow(deprecated)]
use crate::types::Currency;

use super::error::InstrumentError;

/// Payment frequency for swap contracts.
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
    #[inline]
    pub fn period_fraction<T: Float>(&self) -> T {
        let periods: T = from_i32(self.periods_per_year() as i32);
        T::one() / periods
    }
}

/// Interest rate swap contract.
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
    pub fn new(
        notional: T,
        fixed_rate: T,
        payment_dates: Vec<T>,
        frequency: PaymentFrequency,
        currency: Currency,
    ) -> Result<Self, InstrumentError> {
        let zero = T::zero();

        if notional <= zero {
            return Err(InstrumentError::InvalidNotional {
                notional: notional.to_f64().unwrap_or(f64::NAN),
            });
        }

        if payment_dates.is_empty() {
            return Err(InstrumentError::InvalidParameter {
                message: "Payment dates must not be empty".to_string(),
            });
        }

        for i in 1..payment_dates.len() {
            if payment_dates[i] <= payment_dates[i - 1] {
                return Err(InstrumentError::InvalidParameter {
                    message: "Payment dates must be sorted in ascending order".to_string(),
                });
            }
        }

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
    #[inline]
    #[allow(clippy::unwrap_used)]
    pub fn maturity(&self) -> T {
        *self.payment_dates.last().unwrap()
    }

    /// Calculates the fixed leg cash flow for a single period.
    #[inline]
    pub fn fixed_leg_cashflow(&self, year_fraction: T) -> T {
        self.notional * self.fixed_rate * year_fraction
    }
}
