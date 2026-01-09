//! Integration tests for Dual number type and smoothing functions.
//!
//! Task 3.2: Verify that DualNumber type is accessible and basic operations work.
//! Note: num_dual::Dual64 does not implement num_traits::Float directly,
//! so smoothing functions require a wrapper or different approach.
//! This test file validates the DualNumber type alias and basic operations.

#![cfg(feature = "num-dual-mode")]

use approx::assert_relative_eq;
use pricer_core::types::dual::DualNumber;

/// Test that DualNumber type alias is accessible.
#[test]
fn test_dual_number_type_accessible() {
    let dual = DualNumber::new(3.0, 1.0);
    assert_eq!(dual.re, 3.0);
    assert_eq!(dual.eps, 1.0);
}

/// Test DualNumber basic arithmetic operations.
#[test]
fn test_dual_number_arithmetic() {
    let a = DualNumber::new(2.0, 1.0); // a = 2, da/da = 1
    let b = DualNumber::new(3.0, 0.0); // b = 3, db/da = 0

    // Addition: d(a+b)/da = 1
    let sum = a + b;
    assert_relative_eq!(sum.re, 5.0, epsilon = 1e-10);
    assert_relative_eq!(sum.eps, 1.0, epsilon = 1e-10);

    // Subtraction: d(a-b)/da = 1
    let diff = a - b;
    assert_relative_eq!(diff.re, -1.0, epsilon = 1e-10);
    assert_relative_eq!(diff.eps, 1.0, epsilon = 1e-10);

    // Multiplication: d(a*b)/da = b = 3
    let prod = a * b;
    assert_relative_eq!(prod.re, 6.0, epsilon = 1e-10);
    assert_relative_eq!(prod.eps, 3.0, epsilon = 1e-10);

    // Division: d(a/b)/da = 1/b = 1/3
    let quot = a / b;
    assert_relative_eq!(quot.re, 2.0 / 3.0, epsilon = 1e-10);
    assert_relative_eq!(quot.eps, 1.0 / 3.0, epsilon = 1e-10);
}

/// Test DualNumber exponential function.
#[test]
fn test_dual_number_exp() {
    use num_dual::DualNum;

    let a = DualNumber::new(2.0, 1.0);
    let exp_a = a.exp();

    let expected_exp = 2.0_f64.exp();
    assert_relative_eq!(exp_a.re, expected_exp, epsilon = 1e-10);
    // d(exp(a))/da = exp(a)
    assert_relative_eq!(exp_a.eps, expected_exp, epsilon = 1e-10);
}

/// Test DualNumber natural logarithm function.
#[test]
fn test_dual_number_ln() {
    use num_dual::DualNum;

    let a = DualNumber::new(2.0, 1.0);
    let ln_a = a.ln();

    let expected_ln = 2.0_f64.ln();
    assert_relative_eq!(ln_a.re, expected_ln, epsilon = 1e-10);
    // d(ln(a))/da = 1/a = 0.5
    assert_relative_eq!(ln_a.eps, 0.5, epsilon = 1e-10);
}

/// Test DualNumber square root function.
#[test]
fn test_dual_number_sqrt() {
    use num_dual::DualNum;

    let a = DualNumber::new(4.0, 1.0);
    let sqrt_a = a.sqrt();

    assert_relative_eq!(sqrt_a.re, 2.0, epsilon = 1e-10);
    // d(sqrt(a))/da = 1/(2*sqrt(a)) = 1/4 = 0.25
    assert_relative_eq!(sqrt_a.eps, 0.25, epsilon = 1e-10);
}

/// Test DualNumber power function.
#[test]
fn test_dual_number_powf() {
    use num_dual::DualNum;

    let a = DualNumber::new(3.0, 1.0);
    let p = 2.0;
    let pow_a = a.powf(p);

    assert_relative_eq!(pow_a.re, 9.0, epsilon = 1e-10);
    // d(a^p)/da = p * a^(p-1) = 2 * 3^1 = 6
    assert_relative_eq!(pow_a.eps, 6.0, epsilon = 1e-10);
}

/// Test DualNumber sine function.
#[test]
fn test_dual_number_sin() {
    use num_dual::DualNum;
    use std::f64::consts::PI;

    let a = DualNumber::new(PI / 6.0, 1.0); // 30 degrees
    let sin_a = a.sin();

    assert_relative_eq!(sin_a.re, 0.5, epsilon = 1e-10);
    // d(sin(a))/da = cos(a) = cos(PI/6) = sqrt(3)/2
    let expected_cos = (PI / 6.0).cos();
    assert_relative_eq!(sin_a.eps, expected_cos, epsilon = 1e-10);
}

/// Test DualNumber cosine function.
#[test]
fn test_dual_number_cos() {
    use num_dual::DualNum;
    use std::f64::consts::PI;

    let a = DualNumber::new(PI / 3.0, 1.0); // 60 degrees
    let cos_a = a.cos();

    assert_relative_eq!(cos_a.re, 0.5, epsilon = 1e-10);
    // d(cos(a))/da = -sin(a) = -sin(PI/3) = -sqrt(3)/2
    let expected_neg_sin = -(PI / 3.0).sin();
    assert_relative_eq!(cos_a.eps, expected_neg_sin, epsilon = 1e-10);
}

/// Test chain rule with DualNumber.
#[test]
fn test_dual_number_chain_rule() {
    use num_dual::DualNum;

    // f(x) = exp(x^2), f'(x) = 2x * exp(x^2)
    let x = DualNumber::new(1.0, 1.0);
    let x_squared = x * x;
    let result = x_squared.exp();

    // f(1) = exp(1)
    let e = 1.0_f64.exp();
    assert_relative_eq!(result.re, e, epsilon = 1e-10);

    // f'(1) = 2 * 1 * exp(1) = 2e
    assert_relative_eq!(result.eps, 2.0 * e, epsilon = 1e-10);
}

/// Test composition of multiple operations with DualNumber.
#[test]
fn test_dual_number_complex_expression() {
    use num_dual::DualNum;

    // f(x) = (x + 1) * ln(x), f'(x) = ln(x) + (x+1)/x
    let x = DualNumber::new(2.0, 1.0);
    let x_plus_1 = x + DualNumber::new(1.0, 0.0);
    let ln_x = x.ln();
    let result = x_plus_1 * ln_x;

    // f(2) = 3 * ln(2)
    let ln_2 = 2.0_f64.ln();
    assert_relative_eq!(result.re, 3.0 * ln_2, epsilon = 1e-10);

    // f'(2) = ln(2) + 3/2 = ln(2) + 1.5
    let expected_deriv = ln_2 + 1.5;
    assert_relative_eq!(result.eps, expected_deriv, epsilon = 1e-10);
}

/// Test that DualNumber correctly handles derivative with respect to second variable.
#[test]
fn test_dual_number_derivative_wrt_second_var() {

    // f(x, y) = x * y^2, df/dy = 2xy
    let x = DualNumber::new(3.0, 0.0); // dx/dy = 0
    let y = DualNumber::new(2.0, 1.0); // dy/dy = 1

    let y_squared = y * y;
    let result = x * y_squared;

    // f(3, 2) = 3 * 4 = 12
    assert_relative_eq!(result.re, 12.0, epsilon = 1e-10);

    // df/dy at (3, 2) = 2 * 3 * 2 = 12
    assert_relative_eq!(result.eps, 12.0, epsilon = 1e-10);
}

/// Test DualNumber with zero value.
#[test]
fn test_dual_number_zero_value() {
    let a = DualNumber::new(0.0, 1.0);
    let b = DualNumber::new(5.0, 0.0);

    // 0 + 5 = 5
    let sum = a + b;
    assert_relative_eq!(sum.re, 5.0, epsilon = 1e-10);
    assert_relative_eq!(sum.eps, 1.0, epsilon = 1e-10);

    // 0 * 5 = 0
    let prod = a * b;
    assert_relative_eq!(prod.re, 0.0, epsilon = 1e-10);
    // d(0*b)/da = b = 5
    assert_relative_eq!(prod.eps, 5.0, epsilon = 1e-10);
}

/// Test DualNumber negation.
#[test]
fn test_dual_number_negation() {
    let a = DualNumber::new(3.0, 1.0);
    let neg_a = -a;

    assert_relative_eq!(neg_a.re, -3.0, epsilon = 1e-10);
    assert_relative_eq!(neg_a.eps, -1.0, epsilon = 1e-10);
}

/// Test creating DualNumber from f64.
#[test]
fn test_dual_number_from_f64() {
    let val: f64 = 5.0;
    let dual = DualNumber::from(val);

    assert_relative_eq!(dual.re, 5.0, epsilon = 1e-10);
    assert_relative_eq!(dual.eps, 0.0, epsilon = 1e-10);
}

/// Test that all DualNumber operations return finite values for valid inputs.
#[test]
fn test_dual_number_operations_finite() {
    use num_dual::DualNum;

    let x = DualNumber::new(2.0, 1.0);
    let y = DualNumber::new(3.0, 0.5);

    // Arithmetic
    assert!((x + y).re.is_finite());
    assert!((x - y).re.is_finite());
    assert!((x * y).re.is_finite());
    assert!((x / y).re.is_finite());

    // Mathematical functions
    assert!(x.exp().re.is_finite());
    assert!(x.ln().re.is_finite());
    assert!(x.sqrt().re.is_finite());
    assert!(x.sin().re.is_finite());
    assert!(x.cos().re.is_finite());
    assert!(x.powf(2.0).re.is_finite());
}
