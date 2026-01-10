//! Forward mode automatic differentiation types.
//!
//! This module provides `ForwardAD<T>`, a type that carries both primal values
//! and tangent (derivative) values for forward mode automatic differentiation.
//!
//! # Forward Mode AD
//!
//! Forward mode AD propagates tangent values alongside primal values through
//! the computation graph. This is efficient when computing derivatives with
//! respect to a single input variable (e.g., Delta = ∂V/∂S).
//!
//! # Requirements Coverage
//!
//! - Requirement 2.3: tangent seed → Delta output
//! - Requirement 2.4: constant (非微分) と variable (微分対象) のコンストラクタ
//! - Requirement 2.5: 単一変数微分の O(1) 追加コスト
//!
//! # Usage
//!
//! ```rust
//! use pricer_pricing::enzyme::forward::ForwardAD;
//!
//! // Create a variable with tangent seed = 1.0
//! let spot = ForwardAD::variable(100.0);
//!
//! // Create a constant (no derivative)
//! let strike = ForwardAD::constant(100.0);
//!
//! // Arithmetic operations propagate tangents
//! let diff = spot - strike;
//! assert_eq!(diff.primal(), 0.0);
//! assert_eq!(diff.tangent(), 1.0); // d(spot - strike)/d(spot) = 1
//! ```

use num_traits::Float;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Forward mode AD value carrying primal and tangent.
///
/// `ForwardAD<T>` represents a value `x` along with its derivative `dx/dy`
/// with respect to some input variable `y`. The tangent value propagates
/// through arithmetic operations according to the chain rule.
///
/// # Type Parameter
///
/// * `T` - Floating point type (typically `f64`)
///
/// # Fields
///
/// * `primal` - The actual value
/// * `tangent` - The derivative with respect to the seeded input
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::enzyme::forward::ForwardAD;
///
/// // Compute f(x) = x^2 and f'(x) = 2x at x = 3
/// let x = ForwardAD::variable(3.0);
/// let y = x * x;
///
/// assert_eq!(y.primal(), 9.0);
/// assert_eq!(y.tangent(), 6.0); // d(x^2)/dx = 2x = 6
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForwardAD<T: Float> {
    primal: T,
    tangent: T,
}

impl<T: Float> ForwardAD<T> {
    /// Creates a new ForwardAD value with explicit primal and tangent.
    ///
    /// # Arguments
    ///
    /// * `primal` - The value
    /// * `tangent` - The derivative (tangent) value
    #[inline]
    pub fn new(primal: T, tangent: T) -> Self {
        Self { primal, tangent }
    }

    /// Creates a variable with tangent seed = 1.0.
    ///
    /// Use this for the input variable you want to differentiate with respect to.
    ///
    /// # Arguments
    ///
    /// * `value` - The input value
    ///
    /// # Returns
    ///
    /// `ForwardAD` with `tangent = 1.0`, indicating this is the differentiation variable.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::enzyme::forward::ForwardAD;
    ///
    /// let spot = ForwardAD::variable(100.0);
    /// assert_eq!(spot.tangent(), 1.0);
    /// ```
    #[inline]
    pub fn variable(value: T) -> Self {
        Self {
            primal: value,
            tangent: T::one(),
        }
    }

    /// Creates a constant with tangent = 0.0.
    ///
    /// Use this for parameters that are not being differentiated.
    ///
    /// # Arguments
    ///
    /// * `value` - The constant value
    ///
    /// # Returns
    ///
    /// `ForwardAD` with `tangent = 0.0`, indicating this is not differentiated.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use pricer_pricing::enzyme::forward::ForwardAD;
    ///
    /// let strike = ForwardAD::constant(100.0);
    /// assert_eq!(strike.tangent(), 0.0);
    /// ```
    #[inline]
    pub fn constant(value: T) -> Self {
        Self {
            primal: value,
            tangent: T::zero(),
        }
    }

    /// Creates a variable with a custom tangent seed.
    ///
    /// # Arguments
    ///
    /// * `value` - The input value
    /// * `seed` - The tangent seed (typically 1.0)
    #[inline]
    pub fn with_tangent(value: T, seed: T) -> Self {
        Self {
            primal: value,
            tangent: seed,
        }
    }

    /// Returns the primal (actual) value.
    #[inline]
    pub fn primal(&self) -> T {
        self.primal
    }

    /// Returns the tangent (derivative) value.
    #[inline]
    pub fn tangent(&self) -> T {
        self.tangent
    }

    /// Extracts both primal and tangent as a tuple.
    #[inline]
    pub fn into_parts(self) -> (T, T) {
        (self.primal, self.tangent)
    }
}

// =============================================================================
// Arithmetic Operations (Forward Mode Chain Rule)
// =============================================================================

impl<T: Float> Add for ForwardAD<T> {
    type Output = Self;

    /// Addition: d(a + b)/dx = da/dx + db/dx
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            primal: self.primal + rhs.primal,
            tangent: self.tangent + rhs.tangent,
        }
    }
}

impl<T: Float> Sub for ForwardAD<T> {
    type Output = Self;

    /// Subtraction: d(a - b)/dx = da/dx - db/dx
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            primal: self.primal - rhs.primal,
            tangent: self.tangent - rhs.tangent,
        }
    }
}

impl<T: Float> Mul for ForwardAD<T> {
    type Output = Self;

    /// Multiplication (product rule): d(a × b)/dx = a × db/dx + b × da/dx
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            primal: self.primal * rhs.primal,
            tangent: self.primal * rhs.tangent + self.tangent * rhs.primal,
        }
    }
}

impl<T: Float> Div for ForwardAD<T> {
    type Output = Self;

    /// Division (quotient rule): d(a/b)/dx = (b × da/dx - a × db/dx) / b²
    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        let denom = rhs.primal * rhs.primal;
        Self {
            primal: self.primal / rhs.primal,
            tangent: (rhs.primal * self.tangent - self.primal * rhs.tangent) / denom,
        }
    }
}

impl<T: Float> Neg for ForwardAD<T> {
    type Output = Self;

    /// Negation: d(-a)/dx = -da/dx
    #[inline]
    fn neg(self) -> Self::Output {
        Self {
            primal: -self.primal,
            tangent: -self.tangent,
        }
    }
}

// =============================================================================
// Mathematical Functions (Forward Mode Chain Rule)
// =============================================================================

impl<T: Float> ForwardAD<T> {
    /// Square root: d(√a)/dx = (1/(2√a)) × da/dx
    #[inline]
    pub fn sqrt(self) -> Self {
        let sqrt_val = self.primal.sqrt();
        let two = T::from(2.0).unwrap();
        Self {
            primal: sqrt_val,
            tangent: self.tangent / (two * sqrt_val),
        }
    }

    /// Exponential: d(exp(a))/dx = exp(a) × da/dx
    #[inline]
    pub fn exp(self) -> Self {
        let exp_val = self.primal.exp();
        Self {
            primal: exp_val,
            tangent: exp_val * self.tangent,
        }
    }

    /// Natural logarithm: d(ln(a))/dx = (1/a) × da/dx
    #[inline]
    pub fn ln(self) -> Self {
        Self {
            primal: self.primal.ln(),
            tangent: self.tangent / self.primal,
        }
    }

    /// Power function: d(a^n)/dx = n × a^(n-1) × da/dx
    #[inline]
    pub fn powf(self, n: T) -> Self {
        let pow_val = self.primal.powf(n);
        Self {
            primal: pow_val,
            tangent: n * self.primal.powf(n - T::one()) * self.tangent,
        }
    }

    /// Sine: d(sin(a))/dx = cos(a) × da/dx
    #[inline]
    pub fn sin(self) -> Self {
        Self {
            primal: self.primal.sin(),
            tangent: self.primal.cos() * self.tangent,
        }
    }

    /// Cosine: d(cos(a))/dx = -sin(a) × da/dx
    #[inline]
    pub fn cos(self) -> Self {
        Self {
            primal: self.primal.cos(),
            tangent: -self.primal.sin() * self.tangent,
        }
    }

    /// Absolute value: d|a|/dx = sign(a) × da/dx
    #[inline]
    pub fn abs(self) -> Self {
        let sign = if self.primal >= T::zero() {
            T::one()
        } else {
            -T::one()
        };
        Self {
            primal: self.primal.abs(),
            tangent: sign * self.tangent,
        }
    }

    /// Maximum of two values (smooth approximation not needed for primal)
    #[inline]
    pub fn max(self, other: Self) -> Self {
        if self.primal >= other.primal {
            self
        } else {
            other
        }
    }

    /// Minimum of two values
    #[inline]
    pub fn min(self, other: Self) -> Self {
        if self.primal <= other.primal {
            self
        } else {
            other
        }
    }
}

// =============================================================================
// Scalar Operations (for convenience)
// =============================================================================

impl<T: Float> Add<T> for ForwardAD<T> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: T) -> Self::Output {
        Self {
            primal: self.primal + rhs,
            tangent: self.tangent,
        }
    }
}

impl<T: Float> Sub<T> for ForwardAD<T> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: T) -> Self::Output {
        Self {
            primal: self.primal - rhs,
            tangent: self.tangent,
        }
    }
}

impl<T: Float> Mul<T> for ForwardAD<T> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: T) -> Self::Output {
        Self {
            primal: self.primal * rhs,
            tangent: self.tangent * rhs,
        }
    }
}

impl<T: Float> Div<T> for ForwardAD<T> {
    type Output = Self;

    #[inline]
    fn div(self, rhs: T) -> Self::Output {
        Self {
            primal: self.primal / rhs,
            tangent: self.tangent / rhs,
        }
    }
}

// =============================================================================
// Default Implementation
// =============================================================================

impl<T: Float> Default for ForwardAD<T> {
    fn default() -> Self {
        Self::constant(T::zero())
    }
}

// =============================================================================
// Conversion from f64
// =============================================================================

impl From<f64> for ForwardAD<f64> {
    fn from(value: f64) -> Self {
        Self::constant(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_variable_creation() {
        let x = ForwardAD::variable(3.0);
        assert_eq!(x.primal(), 3.0);
        assert_eq!(x.tangent(), 1.0);
    }

    #[test]
    fn test_constant_creation() {
        let c = ForwardAD::constant(5.0);
        assert_eq!(c.primal(), 5.0);
        assert_eq!(c.tangent(), 0.0);
    }

    #[test]
    fn test_with_tangent() {
        let x = ForwardAD::with_tangent(3.0, 2.0);
        assert_eq!(x.primal(), 3.0);
        assert_eq!(x.tangent(), 2.0);
    }

    #[test]
    fn test_addition() {
        let a = ForwardAD::new(2.0, 1.0);
        let b = ForwardAD::new(3.0, 0.5);
        let c = a + b;
        assert_eq!(c.primal(), 5.0);
        assert_eq!(c.tangent(), 1.5);
    }

    #[test]
    fn test_subtraction() {
        let a = ForwardAD::new(5.0, 1.0);
        let b = ForwardAD::new(3.0, 0.5);
        let c = a - b;
        assert_eq!(c.primal(), 2.0);
        assert_eq!(c.tangent(), 0.5);
    }

    #[test]
    fn test_multiplication() {
        // f(x) = x * 2, f'(x) = 2 at x = 3
        let x = ForwardAD::variable(3.0);
        let two = ForwardAD::constant(2.0);
        let y = x * two;
        assert_eq!(y.primal(), 6.0);
        assert_eq!(y.tangent(), 2.0);
    }

    #[test]
    fn test_square() {
        // f(x) = x^2, f'(x) = 2x at x = 3
        let x = ForwardAD::variable(3.0);
        let y = x * x;
        assert_eq!(y.primal(), 9.0);
        assert_eq!(y.tangent(), 6.0);
    }

    #[test]
    fn test_division() {
        // f(x) = x / 2, f'(x) = 0.5
        let x = ForwardAD::variable(4.0);
        let two = ForwardAD::constant(2.0);
        let y = x / two;
        assert_eq!(y.primal(), 2.0);
        assert_eq!(y.tangent(), 0.5);
    }

    #[test]
    fn test_negation() {
        let x = ForwardAD::variable(3.0);
        let y = -x;
        assert_eq!(y.primal(), -3.0);
        assert_eq!(y.tangent(), -1.0);
    }

    #[test]
    fn test_sqrt() {
        // f(x) = sqrt(x), f'(x) = 1/(2*sqrt(x)) at x = 4
        let x = ForwardAD::variable(4.0);
        let y = x.sqrt();
        assert_eq!(y.primal(), 2.0);
        assert_eq!(y.tangent(), 0.25); // 1/(2*2) = 0.25
    }

    #[test]
    fn test_exp() {
        // f(x) = exp(x), f'(x) = exp(x) at x = 0
        let x = ForwardAD::variable(0.0);
        let y = x.exp();
        assert_relative_eq!(y.primal(), 1.0);
        assert_relative_eq!(y.tangent(), 1.0);
    }

    #[test]
    fn test_ln() {
        // f(x) = ln(x), f'(x) = 1/x at x = 2
        let x = ForwardAD::variable(2.0);
        let y = x.ln();
        assert_relative_eq!(y.primal(), 2.0_f64.ln());
        assert_relative_eq!(y.tangent(), 0.5);
    }

    #[test]
    fn test_powf() {
        // f(x) = x^3, f'(x) = 3x^2 at x = 2
        let x = ForwardAD::variable(2.0);
        let y = x.powf(3.0);
        assert_relative_eq!(y.primal(), 8.0);
        assert_relative_eq!(y.tangent(), 12.0); // 3 * 2^2 = 12
    }

    #[test]
    fn test_sin() {
        // f(x) = sin(x), f'(x) = cos(x) at x = 0
        let x = ForwardAD::variable(0.0);
        let y = x.sin();
        assert_relative_eq!(y.primal(), 0.0);
        assert_relative_eq!(y.tangent(), 1.0);
    }

    #[test]
    fn test_cos() {
        // f(x) = cos(x), f'(x) = -sin(x) at x = 0
        let x = ForwardAD::variable(0.0);
        let y = x.cos();
        assert_relative_eq!(y.primal(), 1.0);
        assert_relative_eq!(y.tangent(), 0.0);
    }

    #[test]
    fn test_chain_rule() {
        // f(x) = (x + 1)^2 at x = 2
        // f(2) = 9, f'(x) = 2(x + 1), f'(2) = 6
        let x = ForwardAD::variable(2.0);
        let one = ForwardAD::constant(1.0);
        let y = (x + one) * (x + one);
        assert_eq!(y.primal(), 9.0);
        assert_eq!(y.tangent(), 6.0);
    }

    #[test]
    fn test_complex_expression() {
        // f(x) = x * exp(-x^2) at x = 1
        // f'(x) = exp(-x^2) * (1 - 2x^2)
        // f(1) = exp(-1), f'(1) = exp(-1) * (1 - 2) = -exp(-1)
        let x = ForwardAD::variable(1.0);
        let x_squared = x * x;
        let exp_neg = (-x_squared).exp();
        let y = x * exp_neg;

        let expected_primal = (-1.0_f64).exp();
        let expected_tangent = expected_primal * (-1.0);

        assert_relative_eq!(y.primal(), expected_primal, epsilon = 1e-10);
        assert_relative_eq!(y.tangent(), expected_tangent, epsilon = 1e-10);
    }

    #[test]
    fn test_scalar_operations() {
        let x = ForwardAD::variable(3.0);

        let y1 = x + 2.0;
        assert_eq!(y1.primal(), 5.0);
        assert_eq!(y1.tangent(), 1.0);

        let y2 = x - 1.0;
        assert_eq!(y2.primal(), 2.0);
        assert_eq!(y2.tangent(), 1.0);

        let y3 = x * 2.0;
        assert_eq!(y3.primal(), 6.0);
        assert_eq!(y3.tangent(), 2.0);

        let y4 = x / 3.0;
        assert_eq!(y4.primal(), 1.0);
        assert_relative_eq!(y4.tangent(), 1.0 / 3.0);
    }

    #[test]
    fn test_into_parts() {
        let x = ForwardAD::new(3.0, 2.0);
        let (p, t) = x.into_parts();
        assert_eq!(p, 3.0);
        assert_eq!(t, 2.0);
    }

    #[test]
    fn test_default() {
        let x: ForwardAD<f64> = ForwardAD::default();
        assert_eq!(x.primal(), 0.0);
        assert_eq!(x.tangent(), 0.0);
    }

    #[test]
    fn test_from_f64() {
        let x: ForwardAD<f64> = 5.0.into();
        assert_eq!(x.primal(), 5.0);
        assert_eq!(x.tangent(), 0.0);
    }
}
