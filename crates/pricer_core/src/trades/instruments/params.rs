//! Common instrument parameters.

use num_traits::Float;

use super::error::InstrumentError;

/// Common parameters shared across instrument types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstrumentParams<T: Float> {
    strike: T,
    expiry: T,
    notional: T,
}

impl<T: Float> InstrumentParams<T> {
    /// Creates new instrument parameters with validation.
    pub fn new(strike: T, expiry: T, notional: T) -> Result<Self, InstrumentError> {
        let zero = T::zero();

        if strike <= zero {
            return Err(InstrumentError::InvalidStrike {
                strike: strike.to_f64().unwrap_or(f64::NAN),
            });
        }

        if expiry <= zero {
            return Err(InstrumentError::InvalidExpiry {
                expiry: expiry.to_f64().unwrap_or(f64::NAN),
            });
        }

        if notional <= zero {
            return Err(InstrumentError::InvalidNotional {
                notional: notional.to_f64().unwrap_or(f64::NAN),
            });
        }

        Ok(Self {
            strike,
            expiry,
            notional,
        })
    }

    /// Returns the strike price.
    #[inline]
    pub fn strike(&self) -> T { self.strike }

    /// Returns the time to expiry.
    #[inline]
    pub fn expiry(&self) -> T { self.expiry }

    /// Returns the notional amount.
    #[inline]
    pub fn notional(&self) -> T { self.notional }
}
