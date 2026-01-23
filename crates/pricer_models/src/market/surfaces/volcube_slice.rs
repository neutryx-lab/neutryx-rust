//! VolCube slice adapter for 2D VolatilitySurface compatibility.
//!
//! # Requirements: 8.3, 8.4, 8.5
//!
//! This module provides `VolCubeSlice<T>`, an adapter that wraps a
//! `VolCube<T>` and presents it as a 2D `VolatilitySurface` by fixing
//! the tenor dimension.

use num_traits::Float;

use super::VolatilitySurface;
use crate::market::{
    error::MarketDataError,
    volcube::{VolCube, VolatilityCube},
};

/// Adapter for using a 3D VolCube as a 2D VolatilitySurface.
///
/// Fixes the tenor dimension at a specific value, allowing the cube
/// to be used through the standard `VolatilitySurface` interface.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Example
///
/// ```ignore
/// use pricer_models::market::surfaces::{VolCubeSlice, VolatilitySurface};
/// use pricer_models::market::volcube::VolCube;
///
/// let cube: VolCube<f64> = /* ... */;
/// let tenor = 5.0; // 5-year underlying tenor
/// let slice = VolCubeSlice::new(cube, tenor);
///
/// // Access as 2D surface: volatility(strike, expiry)
/// let vol = slice.volatility(0.03, 1.0).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct VolCubeSlice<T: Float> {
    /// The underlying 3D VolCube.
    cube: VolCube<T>,
    /// Fixed tenor for this slice.
    tenor: T,
}

impl<T: Float + Send + Sync> VolCubeSlice<T> {
    /// Create a new VolCubeSlice at a specific tenor.
    ///
    /// # Arguments
    ///
    /// * `cube` - The 3D VolCube to slice
    /// * `tenor` - The fixed tenor value (years)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let slice = VolCubeSlice::new(cube, 5.0);
    /// ```
    #[inline]
    pub fn new(cube: VolCube<T>, tenor: T) -> Self { Self { cube, tenor } }

    /// Get the fixed tenor for this slice.
    #[inline]
    pub fn tenor(&self) -> T { self.tenor }

    /// Get a reference to the underlying VolCube.
    #[inline]
    pub fn cube(&self) -> &VolCube<T> { &self.cube }

    /// Consume the slice and return the underlying VolCube.
    #[inline]
    pub fn into_cube(self) -> VolCube<T> { self.cube }

    /// Get the tenor domain from the underlying cube.
    #[inline]
    pub fn tenor_domain(&self) -> (T, T) { self.cube.tenor_domain() }
}

impl<T: Float + Send + Sync> VolatilitySurface<T> for VolCubeSlice<T> {
    /// Get volatility for given strike and expiry at the fixed tenor.
    ///
    /// Maps the 2D interface `volatility(strike, expiry)` to the 3D cube's
    /// `volatility(expiry, tenor, strike)`.
    ///
    /// # Arguments
    ///
    /// * `strike` - Strike price (must be > 0)
    /// * `expiry` - Time to expiry in years (must be > 0)
    ///
    /// # Returns
    ///
    /// * `Ok(sigma)` - Implied volatility at (expiry, fixed_tenor, strike)
    /// * `Err(...)` - If parameters are invalid or outside domain
    fn volatility(&self, strike: T, expiry: T) -> Result<T, MarketDataError> {
        // Map 2D (strike, expiry) to 3D (expiry, tenor, strike)
        self.cube.volatility(expiry, self.tenor, strike)
    }

    /// Return the valid strike domain from the underlying cube.
    #[inline]
    fn strike_domain(&self) -> (T, T) { self.cube.strike_domain() }

    /// Return the valid expiry domain from the underlying cube.
    #[inline]
    fn expiry_domain(&self) -> (T, T) { self.cube.expiry_domain() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::market::volcube::{InstrumentId, SabrParameterSurface, SabrParams, VolCubeConfig};

    fn create_test_cube() -> VolCube<f64> {
        let expiries = vec![0.5, 1.0];
        let tenors = vec![2.0, 5.0];
        let beta = 0.5;

        let params = vec![
            vec![
                SabrParams::new(0.04, beta, -0.3, 0.4),
                SabrParams::new(0.05, beta, -0.25, 0.35),
            ],
            vec![
                SabrParams::new(0.045, beta, -0.35, 0.45),
                SabrParams::new(0.055, beta, -0.2, 0.3),
            ],
        ];

        let sabr_surface = SabrParameterSurface::new(expiries, tenors, &params, beta).unwrap();

        let forwards = vec![vec![0.03, 0.035], vec![0.032, 0.038]];

        let config = VolCubeConfig::default();
        let source_instruments = vec![InstrumentId::new("INST-1"), InstrumentId::new("INST-2")];
        let strike_domain = (0.01, 0.10);

        VolCube::new(
            sabr_surface,
            forwards,
            config,
            source_instruments,
            strike_domain,
        )
    }

    // ========================================
    // Construction Tests
    // ========================================

    #[test]
    fn test_volcube_slice_new() {
        let cube = create_test_cube();
        let slice = VolCubeSlice::new(cube, 5.0);
        assert_eq!(slice.tenor(), 5.0);
    }

    #[test]
    fn test_volcube_slice_cube_access() {
        let cube = create_test_cube();
        let slice = VolCubeSlice::new(cube, 5.0);

        // Can access underlying cube
        let cube_ref = slice.cube();
        assert_eq!(cube_ref.source_instruments().len(), 2);
    }

    #[test]
    fn test_volcube_slice_into_cube() {
        let cube = create_test_cube();
        let slice = VolCubeSlice::new(cube, 5.0);
        let recovered = slice.into_cube();
        assert_eq!(recovered.source_instruments().len(), 2);
    }

    // ========================================
    // Volatility Lookup Tests
    // ========================================

    #[test]
    fn test_volcube_slice_volatility() {
        let cube = create_test_cube();
        let slice = VolCubeSlice::new(cube, 5.0);

        // Get vol via 2D interface
        let vol = slice.volatility(0.03, 0.75).unwrap();
        assert!(vol > 0.0);
        assert!(vol < 1.0);
    }

    #[test]
    fn test_volcube_slice_volatility_matches_cube() {
        let cube = create_test_cube();
        let tenor = 3.5;
        let expiry = 0.75;
        let strike = 0.03;

        // Get vol directly from cube
        let vol_cube = cube.volatility(expiry, tenor, strike).unwrap();

        // Get vol via slice
        let slice = VolCubeSlice::new(cube, tenor);
        let vol_slice = slice.volatility(strike, expiry).unwrap();

        assert!((vol_cube - vol_slice).abs() < 1e-12);
    }

    #[test]
    fn test_volcube_slice_volatility_invalid_strike() {
        let cube = create_test_cube();
        let slice = VolCubeSlice::new(cube, 5.0);

        let result = slice.volatility(0.0, 0.75);
        assert!(result.is_err());
    }

    #[test]
    fn test_volcube_slice_volatility_invalid_expiry() {
        let cube = create_test_cube();
        let slice = VolCubeSlice::new(cube, 5.0);

        let result = slice.volatility(0.03, 0.0);
        assert!(result.is_err());
    }

    // ========================================
    // Domain Tests
    // ========================================

    #[test]
    fn test_volcube_slice_strike_domain() {
        let cube = create_test_cube();
        let slice = VolCubeSlice::new(cube, 5.0);

        let (k_min, k_max) = slice.strike_domain();
        assert_eq!(k_min, 0.01);
        assert_eq!(k_max, 0.10);
    }

    #[test]
    fn test_volcube_slice_expiry_domain() {
        let cube = create_test_cube();
        let slice = VolCubeSlice::new(cube, 5.0);

        let (t_min, t_max) = slice.expiry_domain();
        assert_eq!(t_min, 0.5);
        assert_eq!(t_max, 1.0);
    }

    #[test]
    fn test_volcube_slice_tenor_domain() {
        let cube = create_test_cube();
        let slice = VolCubeSlice::new(cube, 5.0);

        let (ten_min, ten_max) = slice.tenor_domain();
        assert_eq!(ten_min, 2.0);
        assert_eq!(ten_max, 5.0);
    }

    // ========================================
    // Clone Tests
    // ========================================

    #[test]
    fn test_volcube_slice_clone() {
        let cube = create_test_cube();
        let slice = VolCubeSlice::new(cube, 5.0);
        let cloned = slice.clone();

        let vol1 = slice.volatility(0.03, 0.75).unwrap();
        let vol2 = cloned.volatility(0.03, 0.75).unwrap();
        assert_eq!(vol1, vol2);
    }

    // ========================================
    // Debug Tests
    // ========================================

    #[test]
    fn test_volcube_slice_debug() {
        let cube = create_test_cube();
        let slice = VolCubeSlice::new(cube, 5.0);
        let debug_str = format!("{:?}", slice);
        assert!(debug_str.contains("VolCubeSlice"));
    }

    // ========================================
    // Different Tenor Tests
    // ========================================

    #[test]
    fn test_volcube_slice_different_tenors() {
        let cube = create_test_cube();
        let strike = 0.03;
        let expiry = 0.75;

        let slice_2y = VolCubeSlice::new(cube.clone(), 2.0);
        let slice_5y = VolCubeSlice::new(cube, 5.0);

        let vol_2y = slice_2y.volatility(strike, expiry).unwrap();
        let vol_5y = slice_5y.volatility(strike, expiry).unwrap();

        // Different tenors should give different vols
        assert!(vol_2y > 0.0);
        assert!(vol_5y > 0.0);
        // They may be different due to SABR surface structure
    }
}
