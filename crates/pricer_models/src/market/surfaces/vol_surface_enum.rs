//! Volatility surface enum for dynamic dispatch.
//!
//! This module provides `VolSurfaceEnum<T>`, an enum wrapper around
//! different volatility surface implementations that provides a unified
//! interface via the `VolatilitySurface` trait.

use num_traits::Float;

use super::{FlatVol, FxVolatilitySurface, InterpolatedVolSurface, VolatilitySurface};
use crate::market::error::MarketDataError;

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
pub enum VolSurfaceEnum<T: Float> {
    /// Flat volatility surface with constant vol.
    Flat(FlatVol<T>),

    /// Grid-based interpolated volatility surface.
    Interpolated(InterpolatedVolSurface<T>),

    /// FX delta-expiry based volatility surface.
    FxSurface(FxVolatilitySurface<T>),
}

impl<T: Float> VolatilitySurface<T> for VolSurfaceEnum<T> {
    fn volatility(&self, strike: T, expiry: T) -> Result<T, MarketDataError> {
        match self {
            VolSurfaceEnum::Flat(s) => s.volatility(strike, expiry),
            VolSurfaceEnum::Interpolated(s) => s.volatility(strike, expiry),
            VolSurfaceEnum::FxSurface(s) => s.volatility(strike, expiry),
        }
    }

    fn strike_domain(&self) -> (T, T) {
        match self {
            VolSurfaceEnum::Flat(s) => s.strike_domain(),
            VolSurfaceEnum::Interpolated(s) => s.strike_domain(),
            VolSurfaceEnum::FxSurface(s) => s.strike_domain(),
        }
    }

    fn expiry_domain(&self) -> (T, T) {
        match self {
            VolSurfaceEnum::Flat(s) => s.expiry_domain(),
            VolSurfaceEnum::Interpolated(s) => s.expiry_domain(),
            VolSurfaceEnum::FxSurface(s) => s.expiry_domain(),
        }
    }
}

impl<T: Float> VolSurfaceEnum<T> {
    /// Create a flat volatility surface.
    ///
    /// # Arguments
    ///
    /// * `sigma` - The constant implied volatility
    #[inline]
    pub fn flat(sigma: T) -> Self { Self::Flat(FlatVol::new(sigma)) }

    /// Check if this is a flat surface.
    #[inline]
    pub fn is_flat(&self) -> bool { matches!(self, Self::Flat(_)) }

    /// Check if this is an interpolated surface.
    #[inline]
    pub fn is_interpolated(&self) -> bool { matches!(self, Self::Interpolated(_)) }

    /// Check if this is an FX surface.
    #[inline]
    pub fn is_fx(&self) -> bool { matches!(self, Self::FxSurface(_)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Construction Tests
    // ========================================

    #[test]
    fn test_flat_construction() {
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::flat(0.20);
        assert!(surface.is_flat());
        assert!(!surface.is_interpolated());
        assert!(!surface.is_fx());
    }

    #[test]
    fn test_from_flat_vol() {
        let flat = FlatVol::new(0.25_f64);
        let surface: VolSurfaceEnum<f64> = VolSurfaceEnum::Flat(flat);
        assert!(surface.is_flat());
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
}
