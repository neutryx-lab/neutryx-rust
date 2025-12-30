//! Common instrument parameters.
//!
//! This module provides shared parameter structures for
//! financial instruments with validation.

use num_traits::Float;

use super::error::InstrumentError;

/// Common parameters shared across instrument types.
///
/// Contains strike price, expiry time, and notional amount with
/// validation ensuring all values are positive.
///
/// # Type Parameters
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Examples
/// ```
/// use pricer_models::instruments::InstrumentParams;
///
/// let params = InstrumentParams::new(100.0_f64, 1.0, 1_000_000.0).unwrap();
/// assert_eq!(params.strike(), 100.0);
/// assert_eq!(params.expiry(), 1.0);
/// assert_eq!(params.notional(), 1_000_000.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstrumentParams<T: Float> {
    strike: T,
    expiry: T,
    notional: T,
}

impl<T: Float> InstrumentParams<T> {
    /// Creates new instrument parameters with validation.
    ///
    /// # Arguments
    /// * `strike` - Strike price (must be positive)
    /// * `expiry` - Time to expiry in years (must be positive)
    /// * `notional` - Notional amount (must be positive)
    ///
    /// # Returns
    /// `Ok(InstrumentParams)` if all parameters are valid,
    /// `Err(InstrumentError)` if any parameter is non-positive.
    ///
    /// # Examples
    /// ```
    /// use pricer_models::instruments::InstrumentParams;
    ///
    /// // Valid parameters
    /// let params = InstrumentParams::new(100.0_f64, 1.0, 1_000_000.0);
    /// assert!(params.is_ok());
    ///
    /// // Invalid strike
    /// let invalid = InstrumentParams::new(-100.0_f64, 1.0, 1_000_000.0);
    /// assert!(invalid.is_err());
    /// ```
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_valid_params() {
        let params = InstrumentParams::new(100.0_f64, 1.0, 1_000_000.0).unwrap();
        assert_eq!(params.strike(), 100.0);
        assert_eq!(params.expiry(), 1.0);
        assert_eq!(params.notional(), 1_000_000.0);
    }

    #[test]
    fn test_new_invalid_strike_negative() {
        let result = InstrumentParams::new(-100.0_f64, 1.0, 1_000_000.0);
        match result {
            Err(InstrumentError::InvalidStrike { strike }) => {
                assert_eq!(strike, -100.0);
            }
            _ => panic!("Expected InvalidStrike error"),
        }
    }

    #[test]
    fn test_new_invalid_strike_zero() {
        let result = InstrumentParams::new(0.0_f64, 1.0, 1_000_000.0);
        assert!(matches!(result, Err(InstrumentError::InvalidStrike { .. })));
    }

    #[test]
    fn test_new_invalid_expiry_negative() {
        let result = InstrumentParams::new(100.0_f64, -1.0, 1_000_000.0);
        match result {
            Err(InstrumentError::InvalidExpiry { expiry }) => {
                assert_eq!(expiry, -1.0);
            }
            _ => panic!("Expected InvalidExpiry error"),
        }
    }

    #[test]
    fn test_new_invalid_expiry_zero() {
        let result = InstrumentParams::new(100.0_f64, 0.0, 1_000_000.0);
        assert!(matches!(result, Err(InstrumentError::InvalidExpiry { .. })));
    }

    #[test]
    fn test_new_invalid_notional_negative() {
        let result = InstrumentParams::new(100.0_f64, 1.0, -1_000_000.0);
        match result {
            Err(InstrumentError::InvalidNotional { notional }) => {
                assert_eq!(notional, -1_000_000.0);
            }
            _ => panic!("Expected InvalidNotional error"),
        }
    }

    #[test]
    fn test_new_invalid_notional_zero() {
        let result = InstrumentParams::new(100.0_f64, 1.0, 0.0);
        assert!(matches!(
            result,
            Err(InstrumentError::InvalidNotional { .. })
        ));
    }

    #[test]
    fn test_f32_compatibility() {
        let params = InstrumentParams::new(100.0_f32, 1.0_f32, 1_000_000.0_f32).unwrap();
        assert_eq!(params.strike(), 100.0_f32);
    }

    #[test]
    fn test_clone_and_equality() {
        let params1 = InstrumentParams::new(100.0_f64, 1.0, 1_000_000.0).unwrap();
        let params2 = params1;
        assert_eq!(params1, params2);
    }

    #[test]
    fn test_debug() {
        let params = InstrumentParams::new(100.0_f64, 1.0, 1_000_000.0).unwrap();
        let debug_str = format!("{:?}", params);
        assert!(debug_str.contains("InstrumentParams"));
        assert!(debug_str.contains("strike"));
        assert!(debug_str.contains("expiry"));
        assert!(debug_str.contains("notional"));
    }

    // AD compatibility test with Dual64
    #[test]
    fn test_dual64_compatibility() {
        use num_dual::Dual64;

        let strike = Dual64::new(100.0, 0.0);
        let expiry = Dual64::new(1.0, 0.0);
        let notional = Dual64::new(1_000_000.0, 0.0);

        let params = InstrumentParams::new(strike, expiry, notional).unwrap();
        assert_eq!(params.strike().re, 100.0);
        assert_eq!(params.expiry().re, 1.0);
        assert_eq!(params.notional().re, 1_000_000.0);
    }
}
