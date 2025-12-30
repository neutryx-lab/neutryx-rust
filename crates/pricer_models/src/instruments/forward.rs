//! Forward contract definitions.
//!
//! This module provides forward contract structures
//! for linear payoff instruments.

use num_traits::Float;

use super::error::InstrumentError;

/// Trade direction for forward contracts.
///
/// # Variants
/// - `Long`: Buyer of the underlying (profits when price rises)
/// - `Short`: Seller of the underlying (profits when price falls)
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
    pub fn is_long(&self) -> bool {
        matches!(self, Direction::Long)
    }

    /// Returns whether this is a short position.
    #[inline]
    pub fn is_short(&self) -> bool {
        matches!(self, Direction::Short)
    }
}

/// Forward contract instrument.
///
/// A forward contract is a linear derivative with payoff:
/// - Long: notional * (spot - strike)
/// - Short: notional * (strike - spot)
///
/// # Type Parameters
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Examples
/// ```
/// use pricer_models::instruments::{Forward, Direction};
///
/// // Long forward at strike 100
/// let forward = Forward::new(100.0_f64, 1.0, 1_000_000.0, Direction::Long).unwrap();
///
/// // Profit when spot > strike
/// let payoff = forward.payoff(110.0);
/// assert!((payoff - 10_000_000.0).abs() < 1.0); // 1M * (110 - 100)
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Forward<T: Float> {
    strike: T,
    expiry: T,
    notional: T,
    direction: Direction,
}

impl<T: Float> Forward<T> {
    /// Creates a new forward contract.
    ///
    /// # Arguments
    /// * `strike` - Delivery price (must be positive)
    /// * `expiry` - Time to delivery in years (must be positive)
    /// * `notional` - Notional amount (must be positive)
    /// * `direction` - Long or Short
    ///
    /// # Returns
    /// `Ok(Forward)` if parameters are valid,
    /// `Err(InstrumentError)` otherwise.
    ///
    /// # Examples
    /// ```
    /// use pricer_models::instruments::{Forward, Direction};
    ///
    /// let forward = Forward::new(100.0_f64, 1.0, 1_000_000.0, Direction::Long);
    /// assert!(forward.is_ok());
    ///
    /// // Invalid expiry
    /// let invalid = Forward::new(100.0_f64, -1.0, 1_000_000.0, Direction::Long);
    /// assert!(invalid.is_err());
    /// ```
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
    ///
    /// # Arguments
    /// * `spot` - Spot price at expiry
    ///
    /// # Returns
    /// - Long: notional * (spot - strike)
    /// - Short: notional * (strike - spot)
    ///
    /// # Examples
    /// ```
    /// use pricer_models::instruments::{Forward, Direction};
    ///
    /// let long = Forward::new(100.0_f64, 1.0, 1.0, Direction::Long).unwrap();
    /// assert!((long.payoff(110.0) - 10.0).abs() < 0.001);
    ///
    /// let short = Forward::new(100.0_f64, 1.0, 1.0, Direction::Short).unwrap();
    /// assert!((short.payoff(110.0) - (-10.0)).abs() < 0.001);
    /// ```
    #[inline]
    pub fn payoff(&self, spot: T) -> T {
        match self.direction {
            Direction::Long => self.notional * (spot - self.strike),
            Direction::Short => self.notional * (self.strike - spot),
        }
    }

    /// Returns the strike (delivery) price.
    #[inline]
    pub fn strike(&self) -> T {
        self.strike
    }

    /// Returns the time to expiry.
    #[inline]
    pub fn expiry(&self) -> T {
        self.expiry
    }

    /// Returns the notional amount.
    #[inline]
    pub fn notional(&self) -> T {
        self.notional
    }

    /// Returns the direction (Long or Short).
    #[inline]
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// Returns whether this is a long forward.
    #[inline]
    pub fn is_long(&self) -> bool {
        self.direction.is_long()
    }

    /// Returns whether this is a short forward.
    #[inline]
    pub fn is_short(&self) -> bool {
        self.direction.is_short()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_direction_is_long() {
        assert!(Direction::Long.is_long());
        assert!(!Direction::Short.is_long());
    }

    #[test]
    fn test_direction_is_short() {
        assert!(Direction::Short.is_short());
        assert!(!Direction::Long.is_short());
    }

    #[test]
    fn test_new_valid_forward() {
        let forward = Forward::new(100.0_f64, 1.0, 1_000_000.0, Direction::Long).unwrap();
        assert_eq!(forward.strike(), 100.0);
        assert_eq!(forward.expiry(), 1.0);
        assert_eq!(forward.notional(), 1_000_000.0);
        assert_eq!(forward.direction(), Direction::Long);
    }

    #[test]
    fn test_new_invalid_strike() {
        let result = Forward::new(-100.0_f64, 1.0, 1_000_000.0, Direction::Long);
        assert!(matches!(result, Err(InstrumentError::InvalidStrike { .. })));
    }

    #[test]
    fn test_new_invalid_expiry() {
        let result = Forward::new(100.0_f64, -1.0, 1_000_000.0, Direction::Long);
        assert!(matches!(result, Err(InstrumentError::InvalidExpiry { .. })));
    }

    #[test]
    fn test_new_invalid_notional() {
        let result = Forward::new(100.0_f64, 1.0, -1_000_000.0, Direction::Long);
        assert!(matches!(
            result,
            Err(InstrumentError::InvalidNotional { .. })
        ));
    }

    #[test]
    fn test_long_payoff_profit() {
        let forward = Forward::new(100.0_f64, 1.0, 1.0, Direction::Long).unwrap();
        // Spot > Strike: profit
        let payoff = forward.payoff(110.0);
        assert_relative_eq!(payoff, 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_long_payoff_loss() {
        let forward = Forward::new(100.0_f64, 1.0, 1.0, Direction::Long).unwrap();
        // Spot < Strike: loss
        let payoff = forward.payoff(90.0);
        assert_relative_eq!(payoff, -10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_short_payoff_profit() {
        let forward = Forward::new(100.0_f64, 1.0, 1.0, Direction::Short).unwrap();
        // Spot < Strike: profit for short
        let payoff = forward.payoff(90.0);
        assert_relative_eq!(payoff, 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_short_payoff_loss() {
        let forward = Forward::new(100.0_f64, 1.0, 1.0, Direction::Short).unwrap();
        // Spot > Strike: loss for short
        let payoff = forward.payoff(110.0);
        assert_relative_eq!(payoff, -10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_payoff_with_notional() {
        let forward = Forward::new(100.0_f64, 1.0, 1_000_000.0, Direction::Long).unwrap();
        let payoff = forward.payoff(110.0);
        assert_relative_eq!(payoff, 10_000_000.0, epsilon = 1.0);
    }

    #[test]
    fn test_payoff_at_strike() {
        let forward = Forward::new(100.0_f64, 1.0, 1.0, Direction::Long).unwrap();
        let payoff = forward.payoff(100.0);
        assert_relative_eq!(payoff, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_is_long_short() {
        let long = Forward::new(100.0_f64, 1.0, 1.0, Direction::Long).unwrap();
        assert!(long.is_long());
        assert!(!long.is_short());

        let short = Forward::new(100.0_f64, 1.0, 1.0, Direction::Short).unwrap();
        assert!(short.is_short());
        assert!(!short.is_long());
    }

    #[test]
    fn test_f32_compatibility() {
        let forward = Forward::new(100.0_f32, 1.0_f32, 1.0_f32, Direction::Long).unwrap();
        let payoff = forward.payoff(110.0_f32);
        assert!((payoff - 10.0_f32).abs() < 0.001);
    }

    #[test]
    fn test_clone_and_copy() {
        let forward1 = Forward::new(100.0_f64, 1.0, 1.0, Direction::Long).unwrap();
        let forward2 = forward1; // Copy
        assert_eq!(forward1.strike(), forward2.strike());
    }

    #[test]
    fn test_debug() {
        let forward = Forward::new(100.0_f64, 1.0, 1.0, Direction::Long).unwrap();
        let debug_str = format!("{:?}", forward);
        assert!(debug_str.contains("Forward"));
        assert!(debug_str.contains("Long"));
    }

    // AD compatibility test with Dual64
    #[test]
    fn test_dual64_compatibility() {
        use num_dual::Dual64;

        let forward = Forward::new(
            Dual64::new(100.0, 0.0),
            Dual64::new(1.0, 0.0),
            Dual64::new(1.0, 0.0),
            Direction::Long,
        )
        .unwrap();

        // Compute payoff with derivative tracking
        let spot = Dual64::new(110.0, 1.0); // d/dS = 1
        let payoff = forward.payoff(spot);

        // Payoff = notional * (spot - strike) = 1 * (110 - 100) = 10
        assert!((payoff.re - 10.0).abs() < 1e-10);

        // Delta = notional * 1 = 1 (for long forward)
        assert!((payoff.eps - 1.0).abs() < 1e-10);
    }
}
