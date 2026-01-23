//! Volatility surface enum for dynamic dispatch.
//!
//! This module provides `VolSurfaceEnum<T>`, an enum wrapper around
//! different volatility surface implementations that provides a unified
//! interface via the `VolatilitySurface` trait.

use num_traits::Float;

use super::{
    FlatVol, FxVolatilitySurface, InterpolatedVolSurface, VolCubeSlice, VolatilitySurface,
};
use crate::market::{error::MarketDataError, volcube::VolCube};

/// Enum wrapper for volatility surface implementations.
///
/// Provides a unified interface over different volatility surface types
/// via the `VolatilitySurface` trait, enabling dynamic dispatch when needed.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Example
///
/// ```
/// use pricer_models::market::surfaces::{VolSurfaceEnum, FlatVol, VolatilitySurface};
///
/// // Create a flat vol surface wrapped in enum
/// let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::Flat(FlatVol::new(0.20));
///
/// // Access via trait
/// let vol = surface.volatility(100.0, 1.0).unwrap();
/// assert!((vol - 0.20).abs() < 1e-10);
/// ```
#[derive(Debug, Clone)]
pub enum VolSurfaceEnum<T: Float + Send + Sync> {
    /// Flat volatility surface with constant vol.
    Flat(FlatVol<T>),

    /// Grid-based interpolated volatility surface.
    Interpolated(InterpolatedVolSurface<T>),

    /// FX delta-expiry based volatility surface.
    FxSurface(FxVolatilitySurface<T>),

    /// 3D VolCube slice at a fixed tenor.
    ///
    /// Adapts a 3D volatility cube to a 2D surface interface by fixing
    /// the tenor dimension.
    CubeSlice(Box<VolCubeSlice<T>>),
}

impl<T: Float + Send + Sync> VolatilitySurface<T> for VolSurfaceEnum<T> {
    fn volatility(&self, strike: T, expiry: T) -> Result<T, MarketDataError> {
        match self {
            VolSurfaceEnum::Flat(s) => s.volatility(strike, expiry),
            VolSurfaceEnum::Interpolated(s) => s.volatility(strike, expiry),
            VolSurfaceEnum::FxSurface(s) => s.volatility(strike, expiry),
            VolSurfaceEnum::CubeSlice(s) => s.volatility(strike, expiry),
        }
    }

    fn strike_domain(&self) -> (T, T) {
        match self {
            VolSurfaceEnum::Flat(s) => s.strike_domain(),
            VolSurfaceEnum::Interpolated(s) => s.strike_domain(),
            VolSurfaceEnum::FxSurface(s) => s.strike_domain(),
            VolSurfaceEnum::CubeSlice(s) => s.strike_domain(),
        }
    }

    fn expiry_domain(&self) -> (T, T) {
        match self {
            VolSurfaceEnum::Flat(s) => s.expiry_domain(),
            VolSurfaceEnum::Interpolated(s) => s.expiry_domain(),
            VolSurfaceEnum::FxSurface(s) => s.expiry_domain(),
            VolSurfaceEnum::CubeSlice(s) => s.expiry_domain(),
        }
    }
}

impl<T: Float + Send + Sync> VolSurfaceEnum<T> {
    /// Create a flat volatility surface.
    ///
    /// # Arguments
    ///
    /// * `sigma` - The constant implied volatility
    #[inline]
    pub fn flat(sigma: T) -> Self { Self::Flat(FlatVol::new(sigma)) }

    /// Create a cube slice surface from a VolCube at a specific tenor.
    ///
    /// # Arguments
    ///
    /// * `cube` - The 3D VolCube
    /// * `tenor` - The tenor to slice at (years)
    #[inline]
    pub fn cube_slice(cube: VolCube<T>, tenor: T) -> Self {
        Self::CubeSlice(Box::new(VolCubeSlice::new(cube, tenor)))
    }

    /// Check if this is a flat surface.
    #[inline]
    pub fn is_flat(&self) -> bool { matches!(self, Self::Flat(_)) }

    /// Check if this is an interpolated surface.
    #[inline]
    pub fn is_interpolated(&self) -> bool { matches!(self, Self::Interpolated(_)) }

    /// Check if this is an FX surface.
    #[inline]
    pub fn is_fx(&self) -> bool { matches!(self, Self::FxSurface(_)) }

    /// Check if this is a cube slice.
    #[inline]
    pub fn is_cube_slice(&self) -> bool { matches!(self, Self::CubeSlice(_)) }

    /// Get the underlying VolCubeSlice if this is a CubeSlice variant.
    #[inline]
    pub fn as_cube_slice(&self) -> Option<&VolCubeSlice<T>> {
        match self {
            Self::CubeSlice(s) => Some(s),
            _ => None,
        }
    }
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
    fn test_flat_construction() {
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::flat(0.20);
        assert!(surface.is_flat());
        assert!(!surface.is_interpolated());
        assert!(!surface.is_fx());
        assert!(!surface.is_cube_slice());
    }

    #[test]
    fn test_from_flat_vol() {
        let flat = FlatVol::new(0.25_f64);
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::Flat(flat);
        assert!(surface.is_flat());
    }

    #[test]
    fn test_cube_slice_construction() {
        let cube = create_test_cube();
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::cube_slice(cube, 5.0);
        assert!(surface.is_cube_slice());
        assert!(!surface.is_flat());
        assert!(!surface.is_interpolated());
        assert!(!surface.is_fx());
    }

    #[test]
    fn test_as_cube_slice() {
        let cube = create_test_cube();
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::cube_slice(cube, 5.0);

        let slice = surface.as_cube_slice();
        assert!(slice.is_some());
        assert_eq!(slice.unwrap().tenor(), 5.0);
    }

    #[test]
    fn test_as_cube_slice_returns_none_for_flat() {
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::flat(0.20);
        assert!(surface.as_cube_slice().is_none());
    }

    // ========================================
    // Volatility Lookup Tests
    // ========================================

    #[test]
    fn test_flat_volatility() {
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::flat(0.20);
        let vol = surface.volatility(100.0, 1.0).unwrap();
        assert!((vol - 0.20).abs() < 1e-10);
    }

    #[test]
    fn test_flat_volatility_constant() {
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::flat(0.25);

        // All strikes and expiries should return same vol
        for strike in [80.0, 100.0, 120.0] {
            for expiry in [0.25, 0.5, 1.0, 2.0] {
                let vol = surface.volatility(strike, expiry).unwrap();
                assert!((vol - 0.25).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_volatility_invalid_strike() {
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::flat(0.20);
        let result = surface.volatility(0.0, 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_volatility_invalid_expiry() {
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::flat(0.20);
        let result = surface.volatility(100.0, 0.0);
        assert!(result.is_err());
    }

    // ========================================
    // Domain Tests
    // ========================================

    #[test]
    fn test_flat_strike_domain() {
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::flat(0.20);
        let (k_min, k_max) = surface.strike_domain();
        assert_eq!(k_min, 0.0);
        assert!(k_max.is_infinite());
    }

    #[test]
    fn test_flat_expiry_domain() {
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::flat(0.20);
        let (t_min, t_max) = surface.expiry_domain();
        assert_eq!(t_min, 0.0);
        assert!(t_max.is_infinite());
    }

    // ========================================
    // Clone Tests
    // ========================================

    #[test]
    fn test_clone() {
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::flat(0.20);
        let cloned = surface.clone();

        let vol1 = surface.volatility(100.0, 1.0).unwrap();
        let vol2 = cloned.volatility(100.0, 1.0).unwrap();
        assert_eq!(vol1, vol2);
    }

    // ========================================
    // Debug Tests
    // ========================================

    #[test]
    fn test_debug() {
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::flat(0.20);
        let debug_str = format!("{:?}", surface);
        assert!(debug_str.contains("Flat"));
    }

    #[test]
    fn test_cube_slice_debug() {
        let cube = create_test_cube();
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::cube_slice(cube, 5.0);
        let debug_str = format!("{:?}", surface);
        assert!(debug_str.contains("CubeSlice"));
    }

    // ========================================
    // CubeSlice Volatility Tests
    // ========================================

    #[test]
    fn test_cube_slice_volatility() {
        let cube = create_test_cube();
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::cube_slice(cube, 5.0);

        // Get vol via enum interface
        let vol = surface.volatility(0.03, 0.75).unwrap();
        assert!(vol > 0.0);
        assert!(vol < 1.0);
    }

    #[test]
    fn test_cube_slice_volatility_invalid_strike() {
        let cube = create_test_cube();
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::cube_slice(cube, 5.0);

        let result = surface.volatility(0.0, 0.75);
        assert!(result.is_err());
    }

    #[test]
    fn test_cube_slice_volatility_invalid_expiry() {
        let cube = create_test_cube();
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::cube_slice(cube, 5.0);

        let result = surface.volatility(0.03, 0.0);
        assert!(result.is_err());
    }

    // ========================================
    // CubeSlice Domain Tests
    // ========================================

    #[test]
    fn test_cube_slice_strike_domain() {
        let cube = create_test_cube();
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::cube_slice(cube, 5.0);

        let (k_min, k_max) = surface.strike_domain();
        assert_eq!(k_min, 0.01);
        assert_eq!(k_max, 0.10);
    }

    #[test]
    fn test_cube_slice_expiry_domain() {
        let cube = create_test_cube();
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::cube_slice(cube, 5.0);

        let (t_min, t_max) = surface.expiry_domain();
        assert_eq!(t_min, 0.5);
        assert_eq!(t_max, 1.0);
    }

    // ========================================
    // CubeSlice Clone Tests
    // ========================================

    #[test]
    fn test_cube_slice_clone() {
        let cube = create_test_cube();
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::cube_slice(cube, 5.0);
        let cloned = surface.clone();

        let vol1 = surface.volatility(0.03, 0.75).unwrap();
        let vol2 = cloned.volatility(0.03, 0.75).unwrap();
        assert_eq!(vol1, vol2);
    }
}
