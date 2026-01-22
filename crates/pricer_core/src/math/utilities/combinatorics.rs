//! Combinatorial functions.
//!
//! This module provides factorial, binomial coefficient, and related
//! combinatorial functions.

use num_traits::Float;

/// Computes the factorial of n (n!).
///
/// Uses the iterative formula: n! = 1 * 2 * 3 * ... * n
///
/// For n = 0, returns 1 (by convention 0! = 1).
///
/// # Arguments
///
/// * `n` - The non-negative integer
///
/// # Returns
///
/// The factorial n! as a float
///
/// # Precision
///
/// For large n (> 20 for f64), precision degrades due to floating-point
/// limitations. Consider using `log_gamma(n + 1)` for large values.
///
/// # Example
///
/// ```
/// use pricer_core::math::utilities::factorial;
///
/// assert_eq!(factorial::<f64>(0), 1.0);
/// assert_eq!(factorial::<f64>(1), 1.0);
/// assert_eq!(factorial::<f64>(5), 120.0);
/// assert_eq!(factorial::<f64>(10), 3628800.0);
/// ```
#[inline]
pub fn factorial<T: Float>(n: usize) -> T {
    if n == 0 || n == 1 {
        return T::one();
    }

    let mut result = T::one();
    for i in 2..=n {
        result = result * T::from(i).unwrap();
    }
    result
}

/// Computes the falling factorial (Pochhammer symbol) x^(n).
///
/// x^(n) = x * (x-1) * (x-2) * ... * (x-n+1)
///
/// This is the number of ways to choose n items from x items
/// where order matters (permutations).
///
/// # Arguments
///
/// * `x` - The starting value
/// * `n` - The number of terms
///
/// # Returns
///
/// The falling factorial x^(n)
///
/// # Example
///
/// ```
/// use pricer_core::math::utilities::falling_factorial;
///
/// // 5^(3) = 5 * 4 * 3 = 60
/// assert_eq!(falling_factorial(5.0_f64, 3), 60.0);
///
/// // x^(0) = 1 by convention
/// assert_eq!(falling_factorial(10.0_f64, 0), 1.0);
/// ```
#[inline]
pub fn falling_factorial<T: Float>(x: T, n: usize) -> T {
    if n == 0 {
        return T::one();
    }

    let mut result = x;
    for i in 1..n {
        result = result * (x - T::from(i).unwrap());
    }
    result
}

/// Computes the binomial coefficient C(n, k) = n! / (k! * (n-k)!).
///
/// Uses the multiplicative formula to avoid overflow:
/// C(n, k) = (n/1) * ((n-1)/2) * ... * ((n-k+1)/k)
///
/// Also uses the symmetry C(n, k) = C(n, n-k) to minimise computation.
///
/// # Arguments
///
/// * `n` - Total number of items
/// * `k` - Number of items to choose
///
/// # Returns
///
/// The binomial coefficient as a float
///
/// # Example
///
/// ```
/// use pricer_core::math::utilities::binomial;
///
/// assert_eq!(binomial::<f64>(5, 0), 1.0);
/// assert_eq!(binomial::<f64>(5, 1), 5.0);
/// assert_eq!(binomial::<f64>(5, 2), 10.0);
/// assert_eq!(binomial::<f64>(5, 5), 1.0);
/// assert_eq!(binomial::<f64>(10, 3), 120.0);
/// ```
#[inline]
pub fn binomial<T: Float>(n: usize, k: usize) -> T {
    // Handle edge cases
    if k > n {
        return T::zero();
    }
    if k == 0 || k == n {
        return T::one();
    }

    // Use symmetry: C(n, k) = C(n, n-k)
    let k = k.min(n - k);

    // Multiplicative formula to avoid overflow
    let mut result = T::one();
    for i in 0..k {
        result = result * T::from(n - i).unwrap() / T::from(i + 1).unwrap();
    }
    result
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    // ==========================================================================
    // factorial tests
    // ==========================================================================

    #[test]
    fn test_factorial_small() {
        assert_eq!(factorial::<f64>(0), 1.0);
        assert_eq!(factorial::<f64>(1), 1.0);
        assert_eq!(factorial::<f64>(2), 2.0);
        assert_eq!(factorial::<f64>(3), 6.0);
        assert_eq!(factorial::<f64>(4), 24.0);
        assert_eq!(factorial::<f64>(5), 120.0);
    }

    #[test]
    fn test_factorial_medium() {
        assert_eq!(factorial::<f64>(10), 3628800.0);
        assert_eq!(factorial::<f64>(12), 479001600.0);
    }

    #[test]
    fn test_factorial_large() {
        // 20! = 2432902008176640000
        let f20 = factorial::<f64>(20);
        assert_relative_eq!(f20, 2432902008176640000.0, epsilon = 1.0);
    }

    // ==========================================================================
    // falling_factorial tests
    // ==========================================================================

    #[test]
    fn test_falling_factorial_zero_terms() {
        assert_eq!(falling_factorial(5.0_f64, 0), 1.0);
        assert_eq!(falling_factorial(100.0_f64, 0), 1.0);
    }

    #[test]
    fn test_falling_factorial_one_term() {
        assert_eq!(falling_factorial(5.0_f64, 1), 5.0);
        assert_eq!(falling_factorial(10.0_f64, 1), 10.0);
    }

    #[test]
    fn test_falling_factorial_multiple_terms() {
        // 5^(3) = 5 * 4 * 3 = 60
        assert_eq!(falling_factorial(5.0_f64, 3), 60.0);

        // 10^(4) = 10 * 9 * 8 * 7 = 5040
        assert_eq!(falling_factorial(10.0_f64, 4), 5040.0);
    }

    #[test]
    fn test_falling_factorial_equals_factorial_at_n() {
        // n^(n) = n!
        for n in 1..=10 {
            let ff = falling_factorial(n as f64, n);
            let fact = factorial::<f64>(n);
            assert_relative_eq!(ff, fact, epsilon = 1e-10);
        }
    }

    // ==========================================================================
    // binomial tests
    // ==========================================================================

    #[test]
    fn test_binomial_edge_cases() {
        // C(n, 0) = 1
        for n in 0..=10 {
            assert_eq!(binomial::<f64>(n, 0), 1.0);
        }

        // C(n, n) = 1
        for n in 0..=10 {
            assert_eq!(binomial::<f64>(n, n), 1.0);
        }
    }

    #[test]
    fn test_binomial_k_greater_than_n() {
        assert_eq!(binomial::<f64>(5, 6), 0.0);
        assert_eq!(binomial::<f64>(0, 1), 0.0);
    }

    #[test]
    fn test_binomial_small() {
        // Pascal's triangle row 5
        assert_eq!(binomial::<f64>(5, 0), 1.0);
        assert_eq!(binomial::<f64>(5, 1), 5.0);
        assert_eq!(binomial::<f64>(5, 2), 10.0);
        assert_eq!(binomial::<f64>(5, 3), 10.0);
        assert_eq!(binomial::<f64>(5, 4), 5.0);
        assert_eq!(binomial::<f64>(5, 5), 1.0);
    }

    #[test]
    fn test_binomial_symmetry() {
        // C(n, k) = C(n, n-k)
        for n in 0..=15 {
            for k in 0..=n {
                let c1 = binomial::<f64>(n, k);
                let c2 = binomial::<f64>(n, n - k);
                assert_relative_eq!(c1, c2, epsilon = 1e-10);
            }
        }
    }

    #[test]
    fn test_binomial_pascal_identity() {
        // C(n, k) = C(n-1, k-1) + C(n-1, k)
        for n in 2..=15 {
            for k in 1..n {
                let lhs = binomial::<f64>(n, k);
                let rhs = binomial::<f64>(n - 1, k - 1) + binomial::<f64>(n - 1, k);
                assert_relative_eq!(lhs, rhs, epsilon = 1e-10);
            }
        }
    }

    #[test]
    fn test_binomial_medium() {
        // C(10, 3) = 120
        assert_eq!(binomial::<f64>(10, 3), 120.0);

        // C(10, 5) = 252
        assert_eq!(binomial::<f64>(10, 5), 252.0);

        // C(20, 10) = 184756
        assert_relative_eq!(binomial::<f64>(20, 10), 184756.0, epsilon = 1e-6);
    }

    #[test]
    fn test_binomial_relation_to_factorial() {
        // C(n, k) = n! / (k! * (n-k)!)
        for n in 0..=12 {
            for k in 0..=n {
                let c_direct = binomial::<f64>(n, k);
                let c_factorial =
                    factorial::<f64>(n) / (factorial::<f64>(k) * factorial::<f64>(n - k));
                assert_relative_eq!(c_direct, c_factorial, epsilon = 1e-10);
            }
        }
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn prop_factorial_positive(n in 0_usize..20) {
            let f = factorial::<f64>(n);
            prop_assert!(f > 0.0);
        }

        #[test]
        fn prop_factorial_increasing(n in 2_usize..20) {
            // factorial(n) > factorial(n-1) for n >= 2
            // Note: factorial(1) = factorial(0) = 1
            let f_n = factorial::<f64>(n);
            let f_n_minus_1 = factorial::<f64>(n - 1);
            prop_assert!(f_n > f_n_minus_1);
        }

        #[test]
        fn prop_binomial_non_negative(n in 0_usize..20, k in 0_usize..25) {
            let c = binomial::<f64>(n, k);
            prop_assert!(c >= 0.0);
        }

        #[test]
        fn prop_binomial_row_sum(n in 0_usize..15) {
            // Sum of C(n, k) for k = 0..n equals 2^n
            let mut sum = 0.0_f64;
            for k in 0..=n {
                sum += binomial::<f64>(n, k);
            }
            let expected = 2.0_f64.powi(n as i32);
            prop_assert!((sum - expected).abs() < 1e-8);
        }
    }
}
