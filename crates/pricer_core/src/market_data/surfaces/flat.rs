//! Flat volatility surface implementation.

use super::VolatilitySurface;
use crate::market_data::error::MarketDataError;
use num_traits::Float;

/// Flat volatility surface with constant implied volatility.
///
/// A simple volatility surface where the same volatility applies to all
/// strike and expiry combinations. Useful for prototyping, testing, and
/// scenarios where smile/skew effects are negligible.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Example
///
/// ```
/// use pricer_core::market_data::surfaces::{VolatilitySurface, FlatVol};
///
/// let surface = FlatVol::new(0.20_f64);
///
/// // Volatility is constant for all strikes and expiries
/// assert_eq!(surface.volatility(80.0, 0.5).unwrap(), 0.20);
/// assert_eq!(surface.volatility(100.0, 1.0).unwrap(), 0.20);
/// assert_eq!(surface.volatility(120.0, 2.0).unwrap(), 0.20);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatVol<T: Float> {
    /// The constant implied volatility
    sigma: T,
}

impl<T: Float> FlatVol<T> {
    /// Construct a flat volatility surface.
    ///
    /// # Arguments
    ///
    /// * `sigma` - The constant implied volatility
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::surfaces::FlatVol;
    ///
    /// let surface = FlatVol::new(0.25_f64);
    /// assert_eq!(surface.sigma(), 0.25);
    /// ```
    #[inline]
    pub fn new(sigma: T) -> Self {
        Self { sigma }
    }

    /// Return the constant volatility.
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::surfaces::FlatVol;
    ///
    /// let surface = FlatVol::new(0.30_f64);
    /// assert_eq!(surface.sigma(), 0.30);
    /// ```
    #[inline]
    pub fn sigma(&self) -> T {
        self.sigma
    }
}

impl<T: Float> VolatilitySurface<T> for FlatVol<T> {
    /// Return the implied volatility for given strike and expiry.
    ///
    /// For a flat surface, returns the constant volatility regardless
    /// of strike and expiry (as long as they are positive).
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price (must be > 0)
    /// * `expiry` - Time to expiry in years (must be > 0)
    ///
    /// # Returns
    ///
    /// * `Ok(sigma)` - The constant volatility
    /// * `Err(MarketDataError::InvalidStrike)` - If strike <= 0
    /// * `Err(MarketDataError::InvalidExpiry)` - If expiry <= 0
    fn volatility(&self, strike: T, expiry: T) -> Result<T, MarketDataError> {
        if strike <= T::zero() {
            return Err(MarketDataError::InvalidStrike {
                strike: strike.to_f64().unwrap_or(0.0),
            });
        }
        if expiry <= T::zero() {
            return Err(MarketDataError::InvalidExpiry {
                expiry: expiry.to_f64().unwrap_or(0.0),
            });
        }
        Ok(self.sigma)
    }

    /// Return the valid strike domain.
    ///
    /// For a flat surface, any positive strike is valid.
    #[inline]
    fn strike_domain(&self) -> (T, T) {
        (T::zero(), T::infinity())
    }

    /// Return the valid expiry domain.
    ///
    /// For a flat surface, any positive expiry is valid.
    #[inline]
    fn expiry_domain(&self) -> (T, T) {
        (T::zero(), T::infinity())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Construction Tests
    // ========================================

    #[test]
    fn test_new() {
        let surface = FlatVol::new(0.20_f64);
        assert_eq!(surface.sigma(), 0.20);
    }

    #[test]
    fn test_new_high_vol() {
        let surface = FlatVol::new(0.80_f64);
        assert_eq!(surface.sigma(), 0.80);
    }

    #[test]
    fn test_new_low_vol() {
        let surface = FlatVol::new(0.05_f64);
        assert_eq!(surface.sigma(), 0.05);
    }

    #[test]
    fn test_clone() {
        let surface = FlatVol::new(0.20_f64);
        let cloned = surface.clone();
        assert_eq!(surface.sigma(), cloned.sigma());
    }

    #[test]
    fn test_copy() {
        let surface = FlatVol::new(0.20_f64);
        let copied = surface;
        assert_eq!(surface.sigma(), copied.sigma());
    }

    #[test]
    fn test_debug() {
        let surface = FlatVol::new(0.20_f64);
        let debug_str = format!("{:?}", surface);
        assert!(debug_str.contains("FlatVol"));
    }

    // ========================================
    // Volatility Lookup Tests
    // ========================================

    #[test]
    fn test_volatility_constant() {
        let surface = FlatVol::new(0.25_f64);

        // Various strikes and expiries should all return 0.25
        let test_cases = [
            (80.0, 0.25),
            (90.0, 0.5),
            (100.0, 1.0),
            (110.0, 2.0),
            (120.0, 5.0),
        ];

        for (strike, expiry) in test_cases {
            let vol = surface.volatility(strike, expiry).unwrap();
            assert_eq!(vol, 0.25);
        }
    }

    #[test]
    fn test_volatility_invalid_strike_zero() {
        let surface = FlatVol::new(0.20_f64);
        let result = surface.volatility(0.0, 1.0);

        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::InvalidStrike { strike } => assert_eq!(strike, 0.0),
            _ => panic!("Expected InvalidStrike error"),
        }
    }

    #[test]
    fn test_volatility_invalid_strike_negative() {
        let surface = FlatVol::new(0.20_f64);
        let result = surface.volatility(-100.0, 1.0);

        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::InvalidStrike { strike } => assert_eq!(strike, -100.0),
            _ => panic!("Expected InvalidStrike error"),
        }
    }

    #[test]
    fn test_volatility_invalid_expiry_zero() {
        let surface = FlatVol::new(0.20_f64);
        let result = surface.volatility(100.0, 0.0);

        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::InvalidExpiry { expiry } => assert_eq!(expiry, 0.0),
            _ => panic!("Expected InvalidExpiry error"),
        }
    }

    #[test]
    fn test_volatility_invalid_expiry_negative() {
        let surface = FlatVol::new(0.20_f64);
        let result = surface.volatility(100.0, -1.0);

        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::InvalidExpiry { expiry } => assert_eq!(expiry, -1.0),
            _ => panic!("Expected InvalidExpiry error"),
        }
    }

    // ========================================
    // Domain Tests
    // ========================================

    #[test]
    fn test_strike_domain() {
        let surface = FlatVol::new(0.20_f64);
        let (k_min, k_max) = surface.strike_domain();
        assert_eq!(k_min, 0.0);
        assert!(k_max.is_infinite());
    }

    #[test]
    fn test_expiry_domain() {
        let surface = FlatVol::new(0.20_f64);
        let (t_min, t_max) = surface.expiry_domain();
        assert_eq!(t_min, 0.0);
        assert!(t_max.is_infinite());
    }

    // ========================================
    // Generic Type Tests
    // ========================================

    #[test]
    fn test_with_f32() {
        let surface = FlatVol::new(0.25_f32);
        let vol = surface.volatility(100.0_f32, 1.0_f32).unwrap();
        assert_eq!(vol, 0.25_f32);
    }

    // ========================================
    // AD Compatibility Tests
    // ========================================

    #[cfg(feature = "num-dual-mode")]
    mod ad_tests {
        use super::*;
        use num_dual::*;

        #[test]
        fn test_flat_vol_with_dual64() {
            // Create surface with Dual64 volatility
            let sigma = Dual64::from(0.20).derivative();
            let surface = FlatVol::new(sigma);

            let strike = Dual64::from(100.0);
            let expiry = Dual64::from(1.0);
            let vol = surface.volatility(strike, expiry).unwrap();

            // Value should be 0.20
            assert!((vol.re() - 0.20).abs() < 1e-10);

            // Derivative w.r.t. sigma should be 1.0
            assert!((vol.eps[0] - 1.0).abs() < 1e-10);
        }

        #[test]
        fn test_vega_propagation() {
            // This test verifies that volatility sensitivities propagate correctly
            let sigma = Dual64::from(0.25).derivative();
            let surface = FlatVol::new(sigma);

            let strike = Dual64::from(100.0);
            let expiry = Dual64::from(0.5);
            let vol = surface.volatility(strike, expiry).unwrap();

            // Vega coefficient: dV/dσ = dV/dvol * dvol/dσ = dV/dvol * 1
            // So the derivative of vol w.r.t. sigma is 1.0
            assert!((vol.eps[0] - 1.0).abs() < 1e-10);
        }
    }
}
