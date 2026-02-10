//! Numeric constant conversion utilities.
//!
//! Provides safe conversion from primitive numeric literals to generic types.
//! These functions encapsulate the `unwrap()` calls that are safe for constant
//! values, satisfying the `clippy::unwrap_used` lint while maintaining clarity.

use num_traits::Float;

/// Converts an `f64` literal to a generic `Float` type.
///
/// # Panics
///
/// Panics if the conversion fails (should never occur for f32/f64/Dual).
#[inline]
#[allow(clippy::unwrap_used)]
#[must_use]
pub fn from_f64<T: Float>(value: f64) -> T { T::from(value).unwrap() }

/// Converts an `i32` literal to a generic `Float` type.
///
/// # Panics
///
/// Panics if the conversion fails.
#[inline]
#[allow(clippy::unwrap_used)]
#[must_use]
pub fn from_i32<T: Float>(value: i32) -> T { T::from(value).unwrap() }

/// Converts a `usize` literal to a generic `Float` type.
///
/// # Panics
///
/// Panics if the conversion fails.
#[inline]
#[allow(clippy::unwrap_used)]
#[must_use]
pub fn from_usize<T: Float>(value: usize) -> T { T::from(value).unwrap() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_f64() {
        let two: f64 = from_f64(2.0);
        assert!((two - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_from_i32() {
        let five: f64 = from_i32(5);
        assert!((five - 5.0).abs() < f64::EPSILON);
    }
}
