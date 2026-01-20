//! Numeric conversion utilities.
//!
//! This module provides helper functions for converting between numeric types
//! while maintaining compatibility with generic `Float` bounds. These functions
//! mirror the ones in `pricer_core::math::numeric` but are available without
//! the `l1l2-integration` feature.

use num_traits::Float;

/// Converts an `f64` value to a generic `Float` type.
///
/// # Arguments
///
/// * `value` - The `f64` value to convert
///
/// # Returns
///
/// The value converted to type `T`
///
/// # Panics
///
/// Panics if the conversion fails (e.g., if the value is out of range for `T`)
#[inline]
pub fn from_f64<T: Float>(value: f64) -> T { T::from(value).unwrap() }

/// Converts a `usize` value to a generic `Float` type.
///
/// # Arguments
///
/// * `value` - The `usize` value to convert
///
/// # Returns
///
/// The value converted to type `T`
///
/// # Panics
///
/// Panics if the conversion fails (e.g., if the value is out of range for `T`)
#[inline]
pub fn from_usize<T: Float>(value: usize) -> T { T::from(value).unwrap() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_f64() {
        let value: f64 = from_f64(3.14);
        assert!((value - 3.14).abs() < 1e-10);

        let value: f32 = from_f64(2.5);
        assert!((value - 2.5).abs() < 1e-6);
    }

    #[test]
    fn test_from_usize() {
        let value: f64 = from_usize(42);
        assert!((value - 42.0).abs() < 1e-10);

        let value: f32 = from_usize(100);
        assert!((value - 100.0).abs() < 1e-6);
    }
}
