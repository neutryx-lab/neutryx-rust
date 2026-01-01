//! Interpolated volatility surface implementation.

use super::VolatilitySurface;
use crate::market_data::error::MarketDataError;
use crate::math::interpolators::BilinearInterpolator;
use num_traits::Float;

/// Interpolated volatility surface using grid data.
///
/// Stores a 2D grid of volatilities indexed by strike and expiry,
/// and uses bilinear interpolation for lookups between grid points.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Grid Layout
///
/// The grid is organised as `vols[expiry_idx][strike_idx]`:
/// - Rows correspond to expiry slices
/// - Columns correspond to strikes within each slice
///
/// # Example
///
/// ```
/// use pricer_core::market_data::surfaces::{VolatilitySurface, InterpolatedVolSurface};
///
/// let strikes = [90.0, 100.0, 110.0];
/// let expiries = [0.25, 0.5, 1.0];
/// let vols = [
///     &[0.22, 0.20, 0.21][..],  // 0.25Y expiry
///     &[0.23, 0.21, 0.22][..],  // 0.5Y expiry
///     &[0.24, 0.22, 0.23][..],  // 1.0Y expiry
/// ];
///
/// let surface = InterpolatedVolSurface::new(&strikes, &expiries, &vols, false).unwrap();
/// let vol = surface.volatility(95.0, 0.375).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct InterpolatedVolSurface<T: Float> {
    /// Sorted strike prices
    strikes: Vec<T>,
    /// Sorted expiry times
    expiries: Vec<T>,
    /// Volatility grid: `vols[expiry_idx][strike_idx]`
    vols: Vec<Vec<T>>,
    /// Whether to allow flat extrapolation
    allow_extrapolation: bool,
}

impl<T: Float> InterpolatedVolSurface<T> {
    /// Construct an interpolated volatility surface from grid data.
    ///
    /// # Arguments
    ///
    /// * `strikes` - Sorted strike prices (at least 2 points)
    /// * `expiries` - Sorted expiry times (at least 2 points)
    /// * `vols` - Volatility grid: `vols[expiry_idx][strike_idx]`
    /// * `allow_extrapolation` - Whether to allow flat extrapolation beyond grid
    ///
    /// # Returns
    ///
    /// * `Ok(InterpolatedVolSurface)` - Successfully constructed surface
    /// * `Err(MarketDataError::InsufficientData)` - Fewer than 2 points on an axis
    /// * `Err(MarketDataError::Interpolation)` - Grid dimensions mismatch
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::surfaces::InterpolatedVolSurface;
    ///
    /// let strikes = [90.0, 100.0, 110.0];
    /// let expiries = [0.5, 1.0];
    /// let vols = [
    ///     &[0.22, 0.20, 0.21][..],
    ///     &[0.23, 0.21, 0.22][..],
    /// ];
    ///
    /// let surface = InterpolatedVolSurface::new(&strikes, &expiries, &vols, true).unwrap();
    /// ```
    pub fn new(
        strikes: &[T],
        expiries: &[T],
        vols: &[&[T]],
        allow_extrapolation: bool,
    ) -> Result<Self, MarketDataError> {
        // Validate minimum sizes
        if strikes.len() < 2 {
            return Err(MarketDataError::InsufficientData {
                got: strikes.len(),
                need: 2,
            });
        }
        if expiries.len() < 2 {
            return Err(MarketDataError::InsufficientData {
                got: expiries.len(),
                need: 2,
            });
        }

        // Validate grid dimensions
        if vols.len() != expiries.len() {
            return Err(MarketDataError::InsufficientData {
                got: vols.len(),
                need: expiries.len(),
            });
        }

        for (i, row) in vols.iter().enumerate() {
            if row.len() != strikes.len() {
                return Err(MarketDataError::InsufficientData {
                    got: row.len(),
                    need: strikes.len(),
                });
            }
            // Validate volatilities are positive
            for &v in row.iter() {
                if v <= T::zero() {
                    return Err(MarketDataError::InvalidStrike {
                        strike: v.to_f64().unwrap_or(0.0),
                    });
                }
            }
            // Validate expiries are positive
            if expiries[i] <= T::zero() {
                return Err(MarketDataError::InvalidExpiry {
                    expiry: expiries[i].to_f64().unwrap_or(0.0),
                });
            }
        }

        // Validate strikes are positive and sorted
        for i in 0..strikes.len() {
            if strikes[i] <= T::zero() {
                return Err(MarketDataError::InvalidStrike {
                    strike: strikes[i].to_f64().unwrap_or(0.0),
                });
            }
            if i > 0 && strikes[i] <= strikes[i - 1] {
                return Err(MarketDataError::InvalidStrike {
                    strike: strikes[i].to_f64().unwrap_or(0.0),
                });
            }
        }

        // Validate expiries are sorted
        for i in 1..expiries.len() {
            if expiries[i] <= expiries[i - 1] {
                return Err(MarketDataError::InvalidExpiry {
                    expiry: expiries[i].to_f64().unwrap_or(0.0),
                });
            }
        }

        let vols_vec: Vec<Vec<T>> = vols.iter().map(|row| row.to_vec()).collect();

        Ok(Self {
            strikes: strikes.to_vec(),
            expiries: expiries.to_vec(),
            vols: vols_vec,
            allow_extrapolation,
        })
    }

    /// Return whether extrapolation is allowed.
    #[inline]
    pub fn allow_extrapolation(&self) -> bool {
        self.allow_extrapolation
    }

    /// Perform bilinear interpolation with optional extrapolation.
    fn interpolate(&self, strike: T, expiry: T) -> Result<T, MarketDataError> {
        let (k_min, k_max) = self.strike_domain();
        let (t_min, t_max) = self.expiry_domain();

        // Clamp to domain if extrapolation allowed
        let k = if self.allow_extrapolation {
            if strike < k_min {
                k_min
            } else if strike > k_max {
                k_max
            } else {
                strike
            }
        } else {
            if strike < k_min || strike > k_max {
                return Err(MarketDataError::OutOfBounds {
                    x: strike.to_f64().unwrap_or(0.0),
                    min: k_min.to_f64().unwrap_or(0.0),
                    max: k_max.to_f64().unwrap_or(0.0),
                });
            }
            strike
        };

        let t = if self.allow_extrapolation {
            if expiry < t_min {
                t_min
            } else if expiry > t_max {
                t_max
            } else {
                expiry
            }
        } else {
            if expiry < t_min || expiry > t_max {
                return Err(MarketDataError::OutOfBounds {
                    x: expiry.to_f64().unwrap_or(0.0),
                    min: t_min.to_f64().unwrap_or(0.0),
                    max: t_max.to_f64().unwrap_or(0.0),
                });
            }
            expiry
        };

        // Build BilinearInterpolator
        // Note: BilinearInterpolator expects xs (expiries), ys (strikes), zs[i][j] = vol(expiry_i, strike_j)
        let zs_refs: Vec<&[T]> = self.vols.iter().map(|v| v.as_slice()).collect();
        let interp = BilinearInterpolator::new(&self.expiries, &self.strikes, &zs_refs)?;

        // Interpolate: (x=expiry, y=strike) -> vol
        let vol = interp.interpolate(t, k)?;
        Ok(vol)
    }
}

impl<T: Float> VolatilitySurface<T> for InterpolatedVolSurface<T> {
    /// Return the implied volatility for given strike and expiry.
    ///
    /// Uses bilinear interpolation on the volatility grid.
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price (must be > 0)
    /// * `expiry` - Time to expiry in years (must be > 0)
    ///
    /// # Returns
    ///
    /// * `Ok(sigma)` - Interpolated implied volatility
    /// * `Err(MarketDataError::InvalidStrike)` - If strike <= 0
    /// * `Err(MarketDataError::InvalidExpiry)` - If expiry <= 0
    /// * `Err(MarketDataError::OutOfBounds)` - If outside domain and extrapolation disabled
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

        self.interpolate(strike, expiry)
    }

    /// Return the valid strike domain.
    #[inline]
    fn strike_domain(&self) -> (T, T) {
        (self.strikes[0], self.strikes[self.strikes.len() - 1])
    }

    /// Return the valid expiry domain.
    #[inline]
    fn expiry_domain(&self) -> (T, T) {
        (self.expiries[0], self.expiries[self.expiries.len() - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a simple test surface
    fn create_test_surface() -> InterpolatedVolSurface<f64> {
        let strikes = [90.0, 100.0, 110.0];
        let expiries = [0.25, 0.5, 1.0];
        // Smile pattern: higher vol at wings
        let vols = [
            &[0.22, 0.20, 0.21][..], // 0.25Y
            &[0.23, 0.21, 0.22][..], // 0.5Y
            &[0.24, 0.22, 0.23][..], // 1.0Y
        ];
        InterpolatedVolSurface::new(&strikes, &expiries, &vols, false).unwrap()
    }

    // ========================================
    // Construction Tests
    // ========================================

    #[test]
    fn test_new_valid() {
        let surface = create_test_surface();
        assert_eq!(surface.strike_domain(), (90.0, 110.0));
        assert_eq!(surface.expiry_domain(), (0.25, 1.0));
    }

    #[test]
    fn test_new_insufficient_strikes() {
        let strikes = [100.0_f64];
        let expiries = [0.5, 1.0];
        let vols = [&[0.20][..], &[0.21][..]];

        let result = InterpolatedVolSurface::new(&strikes, &expiries, &vols, false);
        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::InsufficientData { got, need } => {
                assert_eq!(got, 1);
                assert_eq!(need, 2);
            }
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_new_insufficient_expiries() {
        let strikes = [90.0, 100.0, 110.0];
        let expiries = [0.5_f64];
        let vols = [&[0.22, 0.20, 0.21][..]];

        let result = InterpolatedVolSurface::new(&strikes, &expiries, &vols, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_new_grid_dimension_mismatch() {
        let strikes = [90.0, 100.0, 110.0];
        let expiries = [0.5, 1.0];
        let vols = [&[0.22, 0.20][..], &[0.23, 0.21][..]]; // Wrong strike count

        let result = InterpolatedVolSurface::new(&strikes, &expiries, &vols, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_new_negative_strike() {
        let strikes = [-90.0, 100.0, 110.0];
        let expiries = [0.5, 1.0];
        let vols = [&[0.22, 0.20, 0.21][..], &[0.23, 0.21, 0.22][..]];

        let result = InterpolatedVolSurface::new(&strikes, &expiries, &vols, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_new_unsorted_strikes() {
        let strikes = [100.0, 90.0, 110.0];
        let expiries = [0.5, 1.0];
        let vols = [&[0.22, 0.20, 0.21][..], &[0.23, 0.21, 0.22][..]];

        let result = InterpolatedVolSurface::new(&strikes, &expiries, &vols, false);
        assert!(result.is_err());
    }

    // ========================================
    // Interpolation Tests
    // ========================================

    #[test]
    fn test_volatility_at_grid_points() {
        let surface = create_test_surface();

        // At exact grid points, should return grid values
        assert!((surface.volatility(90.0, 0.25).unwrap() - 0.22).abs() < 1e-10);
        assert!((surface.volatility(100.0, 0.5).unwrap() - 0.21).abs() < 1e-10);
        assert!((surface.volatility(110.0, 1.0).unwrap() - 0.23).abs() < 1e-10);
    }

    #[test]
    fn test_volatility_interpolated() {
        let surface = create_test_surface();

        // At midpoint, should interpolate smoothly
        let vol = surface.volatility(95.0, 0.375).unwrap();
        assert!(vol > 0.0 && vol < 1.0);
    }

    #[test]
    fn test_volatility_out_of_bounds_strike() {
        let surface = create_test_surface();

        let result = surface.volatility(80.0, 0.5);
        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::OutOfBounds { .. } => {}
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_volatility_out_of_bounds_expiry() {
        let surface = create_test_surface();

        let result = surface.volatility(100.0, 0.1);
        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::OutOfBounds { .. } => {}
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_volatility_with_extrapolation() {
        let strikes = [90.0, 100.0, 110.0];
        let expiries = [0.25, 0.5, 1.0];
        let vols = [
            &[0.22, 0.20, 0.21][..],
            &[0.23, 0.21, 0.22][..],
            &[0.24, 0.22, 0.23][..],
        ];
        let surface = InterpolatedVolSurface::new(&strikes, &expiries, &vols, true).unwrap();

        // Outside domain should extrapolate (clamp to boundary)
        let vol_low_strike = surface.volatility(80.0, 0.5).unwrap();
        let vol_high_strike = surface.volatility(120.0, 0.5).unwrap();
        let vol_low_expiry = surface.volatility(100.0, 0.1).unwrap();
        let vol_high_expiry = surface.volatility(100.0, 2.0).unwrap();

        assert!(vol_low_strike > 0.0);
        assert!(vol_high_strike > 0.0);
        assert!(vol_low_expiry > 0.0);
        assert!(vol_high_expiry > 0.0);
    }

    #[test]
    fn test_volatility_invalid_strike() {
        let surface = create_test_surface();

        let result = surface.volatility(0.0, 0.5);
        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::InvalidStrike { .. } => {}
            _ => panic!("Expected InvalidStrike error"),
        }
    }

    #[test]
    fn test_volatility_invalid_expiry() {
        let surface = create_test_surface();

        let result = surface.volatility(100.0, 0.0);
        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::InvalidExpiry { .. } => {}
            _ => panic!("Expected InvalidExpiry error"),
        }
    }

    // ========================================
    // Domain Tests
    // ========================================

    #[test]
    fn test_strike_domain() {
        let surface = create_test_surface();
        assert_eq!(surface.strike_domain(), (90.0, 110.0));
    }

    #[test]
    fn test_expiry_domain() {
        let surface = create_test_surface();
        assert_eq!(surface.expiry_domain(), (0.25, 1.0));
    }

    // ========================================
    // Generic Type Tests
    // ========================================

    #[test]
    fn test_with_f32() {
        let strikes = [90.0_f32, 100.0, 110.0];
        let expiries = [0.5_f32, 1.0];
        let vols = [&[0.22_f32, 0.20, 0.21][..], &[0.23_f32, 0.21, 0.22][..]];

        let surface = InterpolatedVolSurface::new(&strikes, &expiries, &vols, false).unwrap();
        let vol = surface.volatility(100.0_f32, 0.75_f32).unwrap();
        assert!(vol > 0.0 && vol < 1.0);
    }
}
