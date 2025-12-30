//! Volatility surface trait definition.

use crate::market_data::error::MarketDataError;
use num_traits::Float;

/// Generic volatility surface trait for implied volatility lookup.
///
/// All implementations must be generic over `T: Float` for AD compatibility.
/// This ensures the surface can be used with both standard floating-point types
/// (f64, f32) and automatic differentiation types (Dual64).
///
/// # Contract
///
/// - `volatility(strike, expiry)` returns the implied volatility σ(K, T)
/// - `strike_domain()` returns the valid range of strike prices
/// - `expiry_domain()` returns the valid range of expiry times
///
/// # Invariants
///
/// - σ > 0 for all valid (strike, expiry) pairs
/// - Continuity for interpolated implementations
///
/// # Example
///
/// ```
/// use pricer_core::market_data::surfaces::{VolatilitySurface, FlatVol};
///
/// let surface = FlatVol::new(0.20_f64);
///
/// // Get volatility for any strike and expiry
/// let sigma = surface.volatility(100.0, 1.0).unwrap();
/// assert_eq!(sigma, 0.20);
/// ```
pub trait VolatilitySurface<T: Float> {
    /// Return the implied volatility for given strike and expiry.
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price (must be > 0)
    /// * `expiry` - Time to expiry in years (must be > 0)
    ///
    /// # Returns
    ///
    /// * `Ok(sigma)` - Implied volatility
    /// * `Err(MarketDataError::InvalidStrike)` - If strike <= 0
    /// * `Err(MarketDataError::InvalidExpiry)` - If expiry <= 0
    /// * `Err(MarketDataError::OutOfBounds)` - If outside valid domain
    fn volatility(&self, strike: T, expiry: T) -> Result<T, MarketDataError>;

    /// Return the valid strike domain.
    ///
    /// # Returns
    ///
    /// A tuple (K_min, K_max) representing the range of valid strike prices.
    fn strike_domain(&self) -> (T, T);

    /// Return the valid expiry domain.
    ///
    /// # Returns
    ///
    /// A tuple (T_min, T_max) representing the range of valid expiry times.
    fn expiry_domain(&self) -> (T, T);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock implementation for testing
    struct MockVolSurface {
        sigma: f64,
    }

    impl VolatilitySurface<f64> for MockVolSurface {
        fn volatility(&self, strike: f64, expiry: f64) -> Result<f64, MarketDataError> {
            if strike <= 0.0 {
                return Err(MarketDataError::InvalidStrike { strike });
            }
            if expiry <= 0.0 {
                return Err(MarketDataError::InvalidExpiry { expiry });
            }
            Ok(self.sigma)
        }

        fn strike_domain(&self) -> (f64, f64) {
            (0.0, f64::INFINITY)
        }

        fn expiry_domain(&self) -> (f64, f64) {
            (0.0, f64::INFINITY)
        }
    }

    #[test]
    fn test_mock_volatility() {
        let surface = MockVolSurface { sigma: 0.25 };
        let vol = surface.volatility(100.0, 1.0).unwrap();
        assert_eq!(vol, 0.25);
    }

    #[test]
    fn test_mock_invalid_strike() {
        let surface = MockVolSurface { sigma: 0.25 };
        let result = surface.volatility(0.0, 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_invalid_expiry() {
        let surface = MockVolSurface { sigma: 0.25 };
        let result = surface.volatility(100.0, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_domain() {
        let surface = MockVolSurface { sigma: 0.25 };
        let (k_min, _k_max) = surface.strike_domain();
        assert_eq!(k_min, 0.0);

        let (t_min, _t_max) = surface.expiry_domain();
        assert_eq!(t_min, 0.0);
    }
}
