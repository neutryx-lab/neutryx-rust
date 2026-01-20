//! Forward contract definitions.

use num_traits::Float;

use super::error::InstrumentError;

/// Trade direction for forward contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Long position (buy underlying)
    Long,
    /// Short position (sell underlying)
    Short,
}

impl Direction {
    /// Returns whether this is a long position.
    #[inline]
    pub fn is_long(&self) -> bool { matches!(self, Direction::Long) }

    /// Returns whether this is a short position.
    #[inline]
    pub fn is_short(&self) -> bool { matches!(self, Direction::Short) }
}

/// Forward contract instrument.
#[derive(Debug, Clone, Copy)]
pub struct Forward<T: Float> {
    strike: T,
    expiry: T,
    notional: T,
    direction: Direction,
}

impl<T: Float> Forward<T> {
    /// Creates a new forward contract.
    pub fn new(
        strike: T,
        expiry: T,
        notional: T,
        direction: Direction,
    ) -> Result<Self, InstrumentError> {
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
            direction,
        })
    }

    /// Calculates the payoff at expiry for a given spot price.
    #[inline]
    pub fn payoff(&self, spot: T) -> T {
        match self.direction {
            Direction::Long => self.notional * (spot - self.strike),
            Direction::Short => self.notional * (self.strike - spot),
        }
    }

    /// Returns the strike (delivery) price.
    #[inline]
    pub fn strike(&self) -> T { self.strike }

    /// Returns the time to expiry.
    #[inline]
    pub fn expiry(&self) -> T { self.expiry }

    /// Returns the notional amount.
    #[inline]
    pub fn notional(&self) -> T { self.notional }

    /// Returns the direction (Long or Short).
    #[inline]
    pub fn direction(&self) -> Direction { self.direction }

    /// Returns whether this is a long forward.
    #[inline]
    pub fn is_long(&self) -> bool { self.direction.is_long() }

    /// Returns whether this is a short forward.
    #[inline]
    pub fn is_short(&self) -> bool { self.direction.is_short() }
}
