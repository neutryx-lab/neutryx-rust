//! Bivariate normal distribution functions.
//!
//! This module provides the cumulative distribution function (CDF) for the
//! standard bivariate normal distribution, which is essential for pricing
//! compound options, spread options, and other correlation-dependent products.
//!
//! ## Algorithm
//!
//! The implementation uses Gauss-Legendre quadrature based on the Drezner
//! (1978) algorithm, which provides good accuracy with computational
//! efficiency.
//!
//! ## AD Compatibility
//!
//! The function is generic over `T: Float` to support automatic differentiation
//! through dual numbers.
//!
//! ## Reference
//!
//! Drezner, Z. (1978). Computation of the bivariate normal integral.
//! Mathematics of Computation, 32(141), 277-279.

use num_traits::Float;

use super::{normal::norm_cdf, DistributionError};

/// Bivariate normal cumulative distribution function.
///
/// Computes P(X ≤ a, Y ≤ b) where (X, Y) follows a standard bivariate normal
/// distribution with correlation coefficient ρ.
///
/// # Arguments
///
/// * `a` - Upper bound for the first variable
/// * `b` - Upper bound for the second variable
/// * `rho` - Correlation coefficient in [-1, 1]
///
/// # Returns
///
/// The probability P(X ≤ a, Y ≤ b) in the range [0, 1]
///
/// # Errors
///
/// Returns [`DistributionError::InvalidCorrelation`] if ρ is outside [-1, 1].
///
/// # Precision
///
/// Absolute error < 2e-7 for most inputs
///
/// # Example
///
/// ```
/// use pricer_core::math::distributions::bivariate_norm_cdf;
///
/// // Independent case (ρ = 0)
/// let p = bivariate_norm_cdf(0.0_f64, 0.0_f64, 0.0_f64).unwrap();
/// assert!((p - 0.25).abs() < 1e-6);  // Φ(0) * Φ(0) = 0.5 * 0.5
///
/// // Perfect positive correlation
/// let p = bivariate_norm_cdf(1.0_f64, 1.0_f64, 1.0_f64).unwrap();
/// // Should equal Φ(min(a, b)) = Φ(1)
/// ```
#[allow(clippy::excessive_precision)]
pub fn bivariate_norm_cdf<T: Float>(a: T, b: T, rho: T) -> Result<T, DistributionError> {
    let rho_f64 = rho.to_f64().unwrap();

    // Validate correlation coefficient
    if rho_f64 < -1.0 || rho_f64 > 1.0 {
        return Err(DistributionError::InvalidCorrelation { rho: rho_f64 });
    }

    let zero = T::zero();
    let one = T::one();

    // Handle special cases
    let a_f64 = a.to_f64().unwrap();
    let b_f64 = b.to_f64().unwrap();

    // Perfect positive correlation: BVN(a, b, 1) = Φ(min(a, b))
    if rho_f64 >= 1.0 - 1e-15 {
        let min_ab = if a_f64 < b_f64 { a } else { b };
        return Ok(norm_cdf(min_ab));
    }

    // Perfect negative correlation: BVN(a, b, -1) = max(0, Φ(a) + Φ(b) - 1)
    if rho_f64 <= -1.0 + 1e-15 {
        let sum = norm_cdf(a) + norm_cdf(b) - one;
        return Ok(if sum > zero { sum } else { zero });
    }

    // Zero correlation: BVN(a, b, 0) = Φ(a) * Φ(b)
    if rho_f64.abs() < 1e-15 {
        return Ok(norm_cdf(a) * norm_cdf(b));
    }

    // Use the core algorithm for general case
    // The West algorithm uses h = -a, k = -b convention internally
    let result = bivariate_normal_core(-a_f64, -b_f64, rho_f64);

    // Convert back to generic type and ensure valid range
    let result_t = T::from(result).unwrap();
    let clamped = if result_t < zero {
        zero
    } else if result_t > one {
        one
    } else {
        result_t
    };

    Ok(clamped)
}

/// Core bivariate normal CDF computation using Gauss-Legendre quadrature.
///
/// Based on Drezner (1978) with improvements from West (2004).
fn bivariate_normal_core(dh: f64, dk: f64, r: f64) -> f64 {
    // Gauss-Legendre 10-point quadrature weights and abscissae
    const W: [f64; 10] = [
        0.017_614_007_139_152_12,
        0.040_601_429_800_386_94,
        0.062_672_048_334_109_06,
        0.083_276_741_576_704_75,
        0.101_930_119_817_240_4,
        0.118_194_531_961_518_4,
        0.131_688_638_449_176_6,
        0.142_096_109_318_382_1,
        0.149_172_986_472_603_7,
        0.152_753_387_130_725_9,
    ];

    const X: [f64; 10] = [
        0.981_560_634_246_719_3,
        0.904_117_256_370_474_9,
        0.769_902_674_194_304_7,
        0.587_317_954_286_617_4,
        0.367_831_498_998_180_2,
        0.125_233_408_511_468_9,
        -0.125_233_408_511_468_9,
        -0.367_831_498_998_180_2,
        -0.587_317_954_286_617_4,
        -0.769_902_674_194_304_7,
    ];

    let twopi = 2.0 * std::f64::consts::PI;

    // Handle extreme values - use limiting behaviour
    // Note: dh, dk are the negatives of the original a, b
    // So dh > 8 means a < -8 (original argument very negative)
    // And dh < -8 means a > 8 (original argument very positive)
    if dh > 8.0 || dk > 8.0 {
        // Original a or b very negative -> probability near 0
        return 0.0;
    }

    if dh < -8.0 && dk < -8.0 {
        // Original a and b both very positive -> probability near 1
        return 1.0;
    }

    if dh < -8.0 {
        // Original a very positive, use Φ(-dk) = Φ(b)
        return norm_cdf(-dk);
    }

    if dk < -8.0 {
        // Original b very positive, use Φ(-dh) = Φ(a)
        return norm_cdf(-dh);
    }

    let mut bvn = 0.0;

    if r.abs() < 0.925 {
        // Standard case: use simple quadrature
        let hs = f64::midpoint(dh * dh, dk * dk);
        let asr = r.asin();

        for i in 0..10 {
            let sn = (asr * (1.0 + X[i]) / 2.0).sin();
            bvn += W[i] * (sn * dh * dk / (1.0 - sn * sn) - hs / (1.0 - sn * sn)).exp();
        }

        bvn = bvn * asr / (2.0 * twopi);
        bvn += norm_cdf(-dh) * norm_cdf(-dk);
    } else {
        // High correlation case
        if r < 0.0 {
            let k = dk;
            let hk = dh * k;

            if r.abs() < 1.0 {
                let ass = (1.0 - r) * (1.0 + r);
                let a = ass.sqrt();
                let bs = (dh - k).powi(2);
                let c = (4.0 - hk) / 8.0;
                let d = (12.0 - hk) / 16.0;
                let asr = -(bs / ass + hk) / 2.0;

                if asr > -100.0 {
                    bvn = a
                        * asr.exp()
                        * (1.0 - c * (bs - ass) * (1.0 - d * bs / 5.0) / 3.0
                            + c * d * ass * ass / 5.0);
                }

                if -hk < 100.0 {
                    let b = bs.sqrt();
                    bvn -= (-hk / 2.0).exp()
                        * twopi.sqrt()
                        * norm_cdf(-b / a)
                        * b
                        * (1.0 - c * bs * (1.0 - d * bs / 5.0) / 3.0);
                }

                let a = a / 2.0;
                for i in 0..10 {
                    let xs = (a * (1.0 + X[i])).powi(2);
                    let rs = (1.0 - xs).sqrt();
                    let asr = -(bs / xs + hk) / 2.0;

                    if asr > -100.0 {
                        bvn += a
                            * W[i]
                            * asr.exp()
                            * (rs.exp() / rs - (1.0 + c * xs * (1.0 + d * xs)));
                    }
                }

                bvn = -bvn / twopi;
            }

            if r > -1.0 {
                bvn += norm_cdf(-dh.max(-k));
            }
        } else {
            // Positive high correlation
            let k = -dk;
            let hk = dh * k;

            if r.abs() < 1.0 {
                let ass = (1.0 - r) * (1.0 + r);
                let a = ass.sqrt();
                let bs = (dh + k).powi(2);
                let c = (4.0 - hk) / 8.0;
                let d = (12.0 - hk) / 16.0;
                let asr = -(bs / ass + hk) / 2.0;

                if asr > -100.0 {
                    bvn = a
                        * asr.exp()
                        * (1.0 - c * (bs - ass) * (1.0 - d * bs / 5.0) / 3.0
                            + c * d * ass * ass / 5.0);
                }

                if -hk < 100.0 {
                    let b = bs.sqrt();
                    bvn -= (-hk / 2.0).exp()
                        * twopi.sqrt()
                        * norm_cdf(-b / a)
                        * b
                        * (1.0 - c * bs * (1.0 - d * bs / 5.0) / 3.0);
                }

                let a = a / 2.0;
                for i in 0..10 {
                    let xs = (a * (1.0 + X[i])).powi(2);
                    let rs = (1.0 - xs).sqrt();
                    let asr = -(bs / xs + hk) / 2.0;

                    if asr > -100.0 {
                        bvn += a
                            * W[i]
                            * asr.exp()
                            * (rs.exp() / rs - (1.0 + c * xs * (1.0 + d * xs)));
                    }
                }

                bvn = -bvn / twopi;
            }

            if r < 1.0 {
                bvn += norm_cdf(-dh.max(k)) - norm_cdf(-dh) - norm_cdf(k);
            }
        }

        bvn += norm_cdf(-dh) + norm_cdf(-dk);
    }

    bvn
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    // ==========================================================================
    // Basic functionality tests
    // ==========================================================================

    #[test]
    fn test_bivariate_norm_cdf_independent() {
        // When ρ = 0, BVN(a, b, 0) = Φ(a) * Φ(b)
        let p = bivariate_norm_cdf(0.0_f64, 0.0_f64, 0.0_f64).unwrap();
        assert_relative_eq!(p, 0.25, epsilon = 1e-7);

        let p = bivariate_norm_cdf(1.0_f64, 1.0_f64, 0.0_f64).unwrap();
        let phi_1 = norm_cdf(1.0_f64);
        assert_relative_eq!(p, phi_1 * phi_1, epsilon = 1e-7);
    }

    #[test]
    fn test_bivariate_norm_cdf_perfect_positive_correlation() {
        // When ρ = 1, BVN(a, b, 1) = Φ(min(a, b))
        let p = bivariate_norm_cdf(1.0_f64, 2.0_f64, 1.0_f64).unwrap();
        let phi_1 = norm_cdf(1.0_f64);
        assert_relative_eq!(p, phi_1, epsilon = 1e-7);

        let p = bivariate_norm_cdf(2.0_f64, 1.0_f64, 1.0_f64).unwrap();
        assert_relative_eq!(p, phi_1, epsilon = 1e-7);
    }

    #[test]
    fn test_bivariate_norm_cdf_perfect_negative_correlation() {
        // When ρ = -1, BVN(a, b, -1) = max(0, Φ(a) + Φ(b) - 1)
        let p = bivariate_norm_cdf(0.0_f64, 0.0_f64, -1.0_f64).unwrap();
        // Φ(0) + Φ(0) - 1 = 0.5 + 0.5 - 1 = 0
        assert_relative_eq!(p, 0.0, epsilon = 1e-7);

        let p = bivariate_norm_cdf(1.0_f64, 1.0_f64, -1.0_f64).unwrap();
        // Φ(1) + Φ(1) - 1 = 0.8413 + 0.8413 - 1 = 0.6826
        let phi_1 = norm_cdf(1.0_f64);
        assert_relative_eq!(p, 2.0 * phi_1 - 1.0, epsilon = 1e-5);
    }

    #[test]
    fn test_bivariate_norm_cdf_symmetry() {
        // BVN(a, b, ρ) = BVN(b, a, ρ)
        let rho = 0.5_f64;
        let p1 = bivariate_norm_cdf(1.0_f64, 2.0_f64, rho).unwrap();
        let p2 = bivariate_norm_cdf(2.0_f64, 1.0_f64, rho).unwrap();
        assert_relative_eq!(p1, p2, epsilon = 1e-6);
    }

    #[test]
    fn test_bivariate_norm_cdf_bounds() {
        // Result should be in [0, 1]
        for &rho in &[-0.9, -0.5, 0.0, 0.5, 0.9] {
            for &a in &[-2.0, -1.0, 0.0, 1.0, 2.0] {
                for &b in &[-2.0, -1.0, 0.0, 1.0, 2.0] {
                    let p = bivariate_norm_cdf(a, b, rho).unwrap();
                    assert!(p >= 0.0 && p <= 1.0, "p = {p} for a={a}, b={b}, rho={rho}");
                }
            }
        }
    }

    // ==========================================================================
    // Reference value tests
    // ==========================================================================

    #[test]
    fn test_bivariate_norm_cdf_reference_values() {
        // Basic reference value: independent case
        let p = bivariate_norm_cdf(0.0_f64, 0.0_f64, 0.0_f64).unwrap();
        assert!((p - 0.25).abs() < 0.01);

        // Test that correlation increases/decreases probability appropriately
        // For a=0, b=0: positive correlation should increase probability
        let p_pos = bivariate_norm_cdf(0.0_f64, 0.0_f64, 0.5_f64).unwrap();
        let p_neg = bivariate_norm_cdf(0.0_f64, 0.0_f64, -0.5_f64).unwrap();
        assert!(
            p_pos > 0.25,
            "Positive correlation should increase probability"
        );
        assert!(
            p_neg < 0.25,
            "Negative correlation should decrease probability"
        );
    }

    // ==========================================================================
    // Error handling tests
    // ==========================================================================

    #[test]
    fn test_bivariate_norm_cdf_invalid_correlation_too_high() {
        let result = bivariate_norm_cdf(0.0_f64, 0.0_f64, 1.5_f64);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidCorrelation { rho }) if rho == 1.5
        ));
    }

    #[test]
    fn test_bivariate_norm_cdf_invalid_correlation_too_low() {
        let result = bivariate_norm_cdf(0.0_f64, 0.0_f64, -1.5_f64);
        assert!(matches!(
            result,
            Err(DistributionError::InvalidCorrelation { rho }) if rho == -1.5
        ));
    }

    // ==========================================================================
    // Edge case tests
    // ==========================================================================

    #[test]
    fn test_bivariate_norm_cdf_extreme_values() {
        // Very large positive values
        let p = bivariate_norm_cdf(10.0_f64, 10.0_f64, 0.5_f64).unwrap();
        assert!(
            p > 0.99,
            "Large positive values should give p > 0.99, got {p}"
        );

        // Very large negative values - should be close to 0
        let p = bivariate_norm_cdf(-10.0_f64, -10.0_f64, 0.5_f64).unwrap();
        assert!(
            p < 0.01,
            "Large negative values should give p < 0.01, got {p}"
        );
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn prop_bivariate_norm_cdf_bounds(
            a in -5.0_f64..5.0,
            b in -5.0_f64..5.0,
            rho in -0.99_f64..0.99
        ) {
            let p = bivariate_norm_cdf(a, b, rho).unwrap();
            prop_assert!(p >= 0.0 && p <= 1.0);
        }

        #[test]
        fn prop_bivariate_norm_cdf_symmetry(
            a in -3.0_f64..3.0,
            b in -3.0_f64..3.0,
            rho in -0.9_f64..0.9
        ) {
            let p1 = bivariate_norm_cdf(a, b, rho).unwrap();
            let p2 = bivariate_norm_cdf(b, a, rho).unwrap();
            // Use larger tolerance for symmetry
            prop_assert!((p1 - p2).abs() < 1e-6);
        }
    }
}
