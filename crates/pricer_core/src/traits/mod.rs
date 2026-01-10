//! Core traits for priceable instruments, differentiation, and calibration.
//!
//! This module defines fundamental abstractions for:
//! - Generic floating-point operations (`Float` trait)
//! - Price calculation (`Priceable` trait)
//! - Gradient computation (`Differentiable` trait)
//! - Model calibration (`Calibrator` trait)
//!
//! All traits are designed for static dispatch (enum-based) to ensure
//! compatibility with Enzyme AD optimisation at LLVM level.
//!
//! ## Important
//! Do NOT use `Box<dyn Trait>` dynamic dispatch with these traits,
//! as it is incompatible with Enzyme's LLVM-level optimisation.

/// Generic floating-point trait for numeric computations.
///
/// This trait provides a unified interface for both standard floating-point
/// types (f64, f32) and automatic differentiation types (DualNumber).
///
/// # Type Safety
/// All implementing types must support:
/// - Arithmetic operations (+, -, *, /)
/// - Comparisons (PartialOrd)
/// - Mathematical functions (exp, ln, sqrt, etc.)
/// - Copy and Clone semantics
///
/// # Examples
/// ```
/// use pricer_core::traits::Float;
///
/// fn compute_discount<T: Float>(rate: T, time: T) -> T {
///     (-rate * time).exp()
/// }
///
/// let discount_f64: f64 = compute_discount(0.05, 1.0);
/// assert!((discount_f64 - 0.951229).abs() < 1e-5);
/// ```
pub use num_traits::Float;

pub mod calibration;
pub mod priceable;
pub mod risk;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_float_trait_with_f64() {
        // Test that f64 satisfies Float trait
        fn generic_sqrt<T: Float>(x: T) -> T {
            x.sqrt()
        }

        let result = generic_sqrt(4.0_f64);
        assert_eq!(result, 2.0);
    }

    #[test]
    fn test_float_trait_arithmetic() {
        // Test arithmetic operations through Float trait
        fn generic_quadratic<T: Float>(a: T, b: T, c: T, x: T) -> T {
            a * x * x + b * x + c
        }

        let result = generic_quadratic(2.0_f64, 3.0, 1.0, 5.0);
        assert_eq!(result, 66.0); // 2*25 + 3*5 + 1 = 66
    }

    #[test]
    fn test_float_trait_mathematical_functions() {
        // Test mathematical functions
        fn generic_exp<T: Float>(x: T) -> T {
            x.exp()
        }

        let result = generic_exp(0.0_f64);
        assert_eq!(result, 1.0);

        let result2 = generic_exp(1.0_f64);
        assert!((result2 - std::f64::consts::E).abs() < 1e-10);
    }
}
