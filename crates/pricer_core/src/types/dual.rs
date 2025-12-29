//! Dual-mode numeric types for automatic differentiation.
//!
//! This module provides a unified interface for numeric types that can be used
//! in both Enzyme (LLVM-level AD) and num-dual (dual number AD) modes.
//!
//! ## Design
//!
//! - **Enzyme mode** (`enzyme-mode` feature): Uses `f64` directly. Enzyme handles
//!   differentiation at LLVM IR level with zero runtime overhead.
//! - **num-dual mode** (`num-dual-mode` feature, default): Uses `num_dual::DualDVec64`
//!   for forward-mode AD. Used for verification and testing.
//!
//! ## Usage
//!
//! ```ignore
//! use pricer_core::types::dual::{Dual, NumericType};
//!
//! fn price_option<T: NumericType>(spot: T, strike: f64) -> T {
//!     let strike_t = T::from_f64(strike);
//!     // ... pricing logic works with both Enzyme and num-dual
//! }
//! ```

use num_traits::{Float, FloatConst, FromPrimitive, NumCast, One, Zero};
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

// ============================================================================
// Feature-gated type definitions
// ============================================================================

/// The concrete dual number type, determined by feature flags.
///
/// - With `num-dual-mode` (default): `num_dual::DualDVec64<f64>`
/// - With `enzyme-mode`: `f64`
#[cfg(feature = "num-dual-mode")]
pub type Dual = num_dual::DualDVec64<f64>;

#[cfg(feature = "enzyme-mode")]
pub type Dual = f64;

// ============================================================================
// NumericType trait
// ============================================================================

/// Core trait for numeric types used in pricing calculations.
///
/// This trait abstracts over both `f64` (Enzyme mode) and `DualDVec64` (num-dual mode),
/// allowing pricing functions to be generic and work with both backends.
///
/// ## Requirements
///
/// Types implementing this trait must support:
/// - Basic arithmetic operations (+, -, *, /, -)
/// - Conversion to/from f64
/// - Special values (zero, one)
/// - Mathematical functions (exp, ln, sqrt, etc.)
///
/// ## Examples
///
/// ```ignore
/// fn european_call_payoff<T: NumericType>(spot: T, strike: f64) -> T {
///     let k = T::from_f64(strike);
///     (spot - k).max(T::zero())
/// }
/// ```
pub trait NumericType:
    Copy
    + Clone
    + fmt::Debug
    + fmt::Display
    + PartialEq
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
    + Zero
    + One
    + Float
    + FloatConst
    + FromPrimitive
    + NumCast
{
    /// Create from f64 value.
    fn from_f64(x: f64) -> Self;

    /// Convert to f64 (extracts primal value for dual numbers).
    fn to_f64(&self) -> f64;

    /// Create a dual number with value x and derivative dx (for num-dual mode).
    /// In Enzyme mode, this just returns x.
    fn with_derivative(x: f64, dx: f64) -> Self;

    /// Get the derivative (for num-dual mode).
    /// In Enzyme mode, this returns 0.0.
    fn derivative(&self) -> f64;
}

// ============================================================================
// Implementation for f64 (Enzyme mode)
// ============================================================================

#[cfg(feature = "enzyme-mode")]
impl NumericType for f64 {
    #[inline(always)]
    fn from_f64(x: f64) -> Self {
        x
    }

    #[inline(always)]
    fn to_f64(&self) -> f64 {
        *self
    }

    #[inline(always)]
    fn with_derivative(x: f64, _dx: f64) -> Self {
        // Enzyme handles derivatives; we only need the primal value
        x
    }

    #[inline(always)]
    fn derivative(&self) -> f64 {
        // In Enzyme mode, derivatives are computed by Enzyme, not stored in the value
        0.0
    }
}

// ============================================================================
// Implementation for DualDVec64 (num-dual mode)
// ============================================================================

#[cfg(feature = "num-dual-mode")]
impl NumericType for num_dual::DualDVec64<f64> {
    #[inline(always)]
    fn from_f64(x: f64) -> Self {
        num_dual::DualDVec64::from_re(x)
    }

    #[inline(always)]
    fn to_f64(&self) -> f64 {
        self.re()
    }

    #[inline(always)]
    fn with_derivative(x: f64, dx: f64) -> Self {
        num_dual::DualDVec64::new(x, dx)
    }

    #[inline(always)]
    fn derivative(&self) -> f64 {
        // For DualDVec64, get first derivative component
        // (assuming single variable differentiation)
        self.eps()[0]
    }
}

// ============================================================================
// Utility functions
// ============================================================================

/// Create a constant (derivative = 0).
#[inline]
pub fn constant<T: NumericType>(x: f64) -> T {
    T::from_f64(x)
}

/// Create a variable with derivative = 1 (for num-dual mode).
#[inline]
pub fn variable<T: NumericType>(x: f64) -> T {
    T::with_derivative(x, 1.0)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "num-dual-mode")]
    use approx::assert_relative_eq;

    #[test]
    fn test_from_f64() {
        let x = Dual::from_f64(5.0);
        assert_eq!(x.to_f64(), 5.0);
    }

    #[test]
    fn test_arithmetic() {
        let x = Dual::from_f64(3.0);
        let y = Dual::from_f64(2.0);

        assert_eq!((x + y).to_f64(), 5.0);
        assert_eq!((x - y).to_f64(), 1.0);
        assert_eq!((x * y).to_f64(), 6.0);
        assert_eq!((x / y).to_f64(), 1.5);
    }

    #[test]
    fn test_math_functions() {
        let x = Dual::from_f64(2.0);

        assert_eq!(x.exp().to_f64(), 2.0_f64.exp());
        assert_eq!(x.ln().to_f64(), 2.0_f64.ln());
        assert_eq!(x.sqrt().to_f64(), 2.0_f64.sqrt());
        assert_eq!(x.powi(3).to_f64(), 8.0);
    }

    #[cfg(feature = "num-dual-mode")]
    #[test]
    fn test_derivative_forward_mode() {
        // Test: f(x) = x^2
        // Expected: f'(x) = 2x
        // At x = 3: f'(3) = 6

        let x = variable::<Dual>(3.0); // x with dx/dx = 1
        let y = x * x; // y = x^2

        assert_relative_eq!(y.to_f64(), 9.0, epsilon = 1e-10);
        assert_relative_eq!(y.derivative(), 6.0, epsilon = 1e-10);
    }

    #[cfg(feature = "num-dual-mode")]
    #[test]
    fn test_derivative_exp() {
        // Test: f(x) = exp(x)
        // Expected: f'(x) = exp(x)
        // At x = 1: f'(1) = e

        let x = variable::<Dual>(1.0);
        let y = x.exp();

        assert_relative_eq!(y.to_f64(), 1.0_f64.exp(), epsilon = 1e-10);
        assert_relative_eq!(y.derivative(), 1.0_f64.exp(), epsilon = 1e-10);
    }

    #[test]
    fn test_zero_one() {
        let zero = Dual::zero();
        let one = Dual::one();

        assert_eq!(zero.to_f64(), 0.0);
        assert_eq!(one.to_f64(), 1.0);
    }

    #[test]
    fn test_constant_vs_variable() {
        let c = constant::<Dual>(5.0);
        let v = variable::<Dual>(5.0);

        assert_eq!(c.to_f64(), 5.0);
        assert_eq!(v.to_f64(), 5.0);

        #[cfg(feature = "num-dual-mode")]
        {
            assert_eq!(c.derivative(), 0.0); // Constant has zero derivative
            assert_eq!(v.derivative(), 1.0); // Variable has derivative = 1
        }
    }
}
