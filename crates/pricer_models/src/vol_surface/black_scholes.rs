//! Flat Black-Scholes volatility surface.
//!
//! The simplest possible vol surface: a single constant volatility
//! parameter (sigma) returned for every strike, expiry, and forward.

use pricer_core::traits::Float;

use super::{VolSurface, VolSurfaceError};

/// A constant (flat) Black-Scholes volatility surface.
///
/// Implied and local volatilities are identical and equal to `sigma`
/// regardless of the query point.
#[derive(Clone, Debug)]
pub struct BlackScholesVol<T: Float> {
    sigma: T,
}

impl<T: Float> BlackScholesVol<T> {
    /// Creates a new flat vol surface.
    ///
    /// Returns an error if `sigma` is not strictly positive.
    pub fn new(sigma: T) -> Result<Self, VolSurfaceError> {
        if sigma <= T::zero() {
            return Err(VolSurfaceError::InvalidInput(
                "sigma must be strictly positive".to_string(),
            ));
        }
        Ok(Self { sigma })
    }

    /// Returns the constant volatility parameter.
    pub fn sigma(&self) -> T {
        self.sigma
    }
}

impl<T: Float> VolSurface<T> for BlackScholesVol<T> {
    /// Returns the constant sigma for any strike / expiry / forward.
    fn implied_vol(&self, _strike: T, _expiry: T, _forward: T) -> Result<T, VolSurfaceError> {
        Ok(self.sigma)
    }

    /// Local vol coincides with implied vol for a flat surface.
    fn local_vol(&self, _strike: T, _expiry: T, _forward: T) -> Result<T, VolSurfaceError> {
        Ok(self.sigma)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_new_valid() {
        let surface = BlackScholesVol::new(0.2_f64);
        assert!(surface.is_ok());
        assert_relative_eq!(surface.unwrap().sigma(), 0.2);
    }

    #[test]
    fn test_new_zero_sigma_rejected() {
        let surface = BlackScholesVol::new(0.0_f64);
        assert!(matches!(surface, Err(VolSurfaceError::InvalidInput(_))));
    }

    #[test]
    fn test_new_negative_sigma_rejected() {
        let surface = BlackScholesVol::new(-0.1_f64);
        assert!(matches!(surface, Err(VolSurfaceError::InvalidInput(_))));
    }

    #[test]
    fn test_implied_vol_constant() {
        let surface = BlackScholesVol::new(0.25_f64).unwrap();
        let vol = surface.implied_vol(100.0, 1.0, 100.0).unwrap();
        assert_relative_eq!(vol, 0.25);

        // Different strike / expiry / forward should still return sigma
        let vol2 = surface.implied_vol(120.0, 0.5, 95.0).unwrap();
        assert_relative_eq!(vol2, 0.25);
    }

    #[test]
    fn test_local_vol_constant() {
        let surface = BlackScholesVol::new(0.30_f64).unwrap();
        let vol = surface.local_vol(110.0, 2.0, 105.0).unwrap();
        assert_relative_eq!(vol, 0.30);
    }

    #[test]
    fn test_atm_vol_delegates_to_implied() {
        let surface = BlackScholesVol::new(0.18_f64).unwrap();
        let vol = surface.atm_vol(1.0, 100.0).unwrap();
        assert_relative_eq!(vol, 0.18);
    }
}
