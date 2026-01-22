//! Gauss-Legendre quadrature for numerical integration.
//!
//! Gauss-Legendre quadrature approximates the integral of a function f(x)
//! over the interval [a, b] using weighted sums of function values at
//! carefully chosen points (nodes).
//!
//! The N-point Gauss-Legendre rule is exact for polynomials up to degree 2N-1.

use super::IntegrationResult;
use num_traits::Float;

/// Order of the Gauss-Legendre quadrature rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaussLegendreOrder {
    /// 7-point rule (exact for polynomials up to degree 13).
    N7,
    /// 15-point rule (exact for polynomials up to degree 29).
    N15,
    /// 21-point rule (exact for polynomials up to degree 41).
    N21,
}

impl GaussLegendreOrder {
    /// Returns the number of quadrature points.
    #[must_use]
    pub const fn num_points(self) -> usize {
        match self {
            Self::N7 => 7,
            Self::N15 => 15,
            Self::N21 => 21,
        }
    }
}

/// Gauss-Legendre 7-point nodes (on [-1, 1]).
const GL7_NODES: [f64; 7] = [
    -0.949_107_912_342_758_5,
    -0.741_531_185_599_394_4,
    -0.405_845_151_377_397_2,
    0.0,
    0.405_845_151_377_397_2,
    0.741_531_185_599_394_4,
    0.949_107_912_342_758_5,
];

/// Gauss-Legendre 7-point weights.
const GL7_WEIGHTS: [f64; 7] = [
    0.129_484_966_168_869_69,
    0.279_705_391_489_276_67,
    0.381_830_050_505_118_94,
    0.417_959_183_673_469_4,
    0.381_830_050_505_118_94,
    0.279_705_391_489_276_67,
    0.129_484_966_168_869_69,
];

/// Gauss-Legendre 15-point nodes (on [-1, 1]).
const GL15_NODES: [f64; 15] = [
    -0.987_992_518_020_485_4,
    -0.937_273_392_400_705_9,
    -0.848_206_583_410_427_2,
    -0.724_417_731_360_170_0,
    -0.570_972_172_608_538_8,
    -0.394_151_347_077_563_4,
    -0.201_194_093_997_434_5,
    0.0,
    0.201_194_093_997_434_5,
    0.394_151_347_077_563_4,
    0.570_972_172_608_538_8,
    0.724_417_731_360_170_0,
    0.848_206_583_410_427_2,
    0.937_273_392_400_705_9,
    0.987_992_518_020_485_4,
];

/// Gauss-Legendre 15-point weights.
const GL15_WEIGHTS: [f64; 15] = [
    0.030_753_241_996_117_27,
    0.070_366_047_488_108_12,
    0.107_159_220_467_171_94,
    0.139_570_677_926_154_1,
    0.166_269_205_816_993_93,
    0.186_161_000_015_562_21,
    0.198_431_485_327_111_58,
    0.202_578_241_925_561_27,
    0.198_431_485_327_111_58,
    0.186_161_000_015_562_21,
    0.166_269_205_816_993_93,
    0.139_570_677_926_154_1,
    0.107_159_220_467_171_94,
    0.070_366_047_488_108_12,
    0.030_753_241_996_117_27,
];

/// Gauss-Legendre 21-point nodes (on [-1, 1]).
const GL21_NODES: [f64; 21] = [
    -0.993_752_170_620_389_0,
    -0.967_226_838_566_306_3,
    -0.920_099_334_150_400_8,
    -0.853_363_364_583_317_3,
    -0.768_439_963_475_677_9,
    -0.667_138_804_197_412_3,
    -0.551_618_835_887_219_8,
    -0.424_342_120_207_438_9,
    -0.288_021_316_802_401_1,
    -0.145_561_854_160_895_1,
    0.0,
    0.145_561_854_160_895_1,
    0.288_021_316_802_401_1,
    0.424_342_120_207_438_9,
    0.551_618_835_887_219_8,
    0.667_138_804_197_412_3,
    0.768_439_963_475_677_9,
    0.853_363_364_583_317_3,
    0.920_099_334_150_400_8,
    0.967_226_838_566_306_3,
    0.993_752_170_620_389_0,
];

/// Gauss-Legendre 21-point weights.
const GL21_WEIGHTS: [f64; 21] = [
    0.016_017_228_257_774_33,
    0.036_953_789_770_852_49,
    0.057_134_425_426_857_21,
    0.076_100_113_628_379_3,
    0.093_444_423_456_033_86,
    0.108_797_299_167_148_4,
    0.121_831_416_053_728_6,
    0.132_268_938_633_337_5,
    0.139_887_394_791_072_36,
    0.144_524_403_989_970_06,
    0.146_081_133_649_690_43,
    0.144_524_403_989_970_06,
    0.139_887_394_791_072_36,
    0.132_268_938_633_337_5,
    0.121_831_416_053_728_6,
    0.108_797_299_167_148_4,
    0.093_444_423_456_033_86,
    0.076_100_113_628_379_3,
    0.057_134_425_426_857_21,
    0.036_953_789_770_852_49,
    0.016_017_228_257_774_33,
];

/// Computes the definite integral of f(x) from a to b using Gauss-Legendre quadrature.
///
/// # Arguments
///
/// * `f` - The function to integrate
/// * `a` - Lower bound of integration
/// * `b` - Upper bound of integration
/// * `order` - The quadrature order (N7, N15, or N21)
///
/// # Returns
///
/// An `IntegrationResult` containing the integral value and number of evaluations.
///
/// # Example
///
/// ```
/// use pricer_core::math::integrators::{integrate_gauss_legendre, GaussLegendreOrder};
///
/// // Integrate sin(x) from 0 to pi (exact answer: 2)
/// let result = integrate_gauss_legendre(|x: f64| x.sin(), 0.0, std::f64::consts::PI, GaussLegendreOrder::N15);
/// assert!((result.value - 2.0).abs() < 1e-14);
/// ```
pub fn integrate_gauss_legendre<T, F>(f: F, a: T, b: T, order: GaussLegendreOrder) -> IntegrationResult<T>
where
    T: Float,
    F: Fn(T) -> T,
{
    let half = T::from(0.5).unwrap();

    // Transform from [-1, 1] to [a, b]
    // x = (b - a) / 2 * t + (a + b) / 2
    // dx = (b - a) / 2 * dt
    let scale = (b - a) * half;
    let shift = (a + b) * half;

    let (nodes, weights): (&[f64], &[f64]) = match order {
        GaussLegendreOrder::N7 => (&GL7_NODES, &GL7_WEIGHTS),
        GaussLegendreOrder::N15 => (&GL15_NODES, &GL15_WEIGHTS),
        GaussLegendreOrder::N21 => (&GL21_NODES, &GL21_WEIGHTS),
    };

    let mut sum = T::zero();
    for (&node, &weight) in nodes.iter().zip(weights.iter()) {
        let t = T::from(node).unwrap();
        let w = T::from(weight).unwrap();
        let x = scale * t + shift;
        sum = sum + w * f(x);
    }

    IntegrationResult::new(scale * sum, order.num_points())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_order_num_points() {
        assert_eq!(GaussLegendreOrder::N7.num_points(), 7);
        assert_eq!(GaussLegendreOrder::N15.num_points(), 15);
        assert_eq!(GaussLegendreOrder::N21.num_points(), 21);
    }

    #[test]
    fn test_constant_function() {
        // Integral of 5 from 0 to 2 = 10
        let result = integrate_gauss_legendre(|_x: f64| 5.0, 0.0, 2.0, GaussLegendreOrder::N7);
        assert!((result.value - 10.0).abs() < 1e-14);
        assert_eq!(result.num_evaluations, 7);
    }

    #[test]
    fn test_linear_function() {
        // Integral of x from 0 to 1 = 0.5
        let result = integrate_gauss_legendre(|x: f64| x, 0.0, 1.0, GaussLegendreOrder::N7);
        assert!((result.value - 0.5).abs() < 1e-14);
    }

    #[test]
    fn test_quadratic_function() {
        // Integral of x^2 from 0 to 1 = 1/3
        let result = integrate_gauss_legendre(|x: f64| x * x, 0.0, 1.0, GaussLegendreOrder::N7);
        assert!((result.value - 1.0 / 3.0).abs() < 1e-14);
    }

    #[test]
    fn test_cubic_function() {
        // Integral of x^3 from 0 to 1 = 0.25
        let result = integrate_gauss_legendre(|x: f64| x * x * x, 0.0, 1.0, GaussLegendreOrder::N7);
        assert!((result.value - 0.25).abs() < 1e-14);
    }

    #[test]
    fn test_sine_function() {
        // Integral of sin(x) from 0 to pi = 2
        let result = integrate_gauss_legendre(|x: f64| x.sin(), 0.0, PI, GaussLegendreOrder::N15);
        assert!((result.value - 2.0).abs() < 1e-14);
    }

    #[test]
    fn test_exponential_function() {
        // Integral of exp(x) from 0 to 1 = e - 1
        let result = integrate_gauss_legendre(|x: f64| x.exp(), 0.0, 1.0, GaussLegendreOrder::N15);
        let expected = std::f64::consts::E - 1.0;
        assert!((result.value - expected).abs() < 1e-14);
    }

    #[test]
    fn test_higher_order_polynomial() {
        // Integral of x^10 from 0 to 1 = 1/11
        // N7 is exact for degree <= 13
        let result = integrate_gauss_legendre(|x: f64| x.powi(10), 0.0, 1.0, GaussLegendreOrder::N7);
        assert!((result.value - 1.0 / 11.0).abs() < 1e-13);
    }

    #[test]
    fn test_n21_higher_precision() {
        // Integral of exp(-x^2) from 0 to 1
        // This is a non-polynomial function that benefits from higher order
        let result7 = integrate_gauss_legendre(|x: f64| (-x * x).exp(), 0.0, 1.0, GaussLegendreOrder::N7);
        let result21 = integrate_gauss_legendre(|x: f64| (-x * x).exp(), 0.0, 1.0, GaussLegendreOrder::N21);

        // Known value: approximately 0.7468241328124271
        let expected = 0.746_824_132_812_427_1;

        // N21 should be more accurate
        let error7 = (result7.value - expected).abs();
        let error21 = (result21.value - expected).abs();
        assert!(error21 <= error7);
        assert!(error21 < 1e-14);
    }

    #[test]
    fn test_negative_bounds() {
        // Integral of x from -1 to 1 = 0
        let result = integrate_gauss_legendre(|x: f64| x, -1.0, 1.0, GaussLegendreOrder::N7);
        assert!(result.value.abs() < 1e-14);
    }

    #[test]
    fn test_arbitrary_bounds() {
        // Integral of x^2 from 2 to 5 = (5^3 - 2^3) / 3 = 39
        let result = integrate_gauss_legendre(|x: f64| x * x, 2.0, 5.0, GaussLegendreOrder::N15);
        assert!((result.value - 39.0).abs() < 1e-12);
    }

    #[test]
    fn test_weights_sum_to_two() {
        // Gauss-Legendre weights should sum to 2 (the length of [-1, 1])
        let sum7: f64 = GL7_WEIGHTS.iter().sum();
        let sum15: f64 = GL15_WEIGHTS.iter().sum();
        let sum21: f64 = GL21_WEIGHTS.iter().sum();

        assert!((sum7 - 2.0).abs() < 1e-14);
        assert!((sum15 - 2.0).abs() < 1e-14);
        assert!((sum21 - 2.0).abs() < 1e-14);
    }

    #[test]
    fn test_nodes_symmetry() {
        // Nodes should be symmetric around 0
        for i in 0..3 {
            assert!((GL7_NODES[i] + GL7_NODES[6 - i]).abs() < 1e-15);
        }
        for i in 0..7 {
            assert!((GL15_NODES[i] + GL15_NODES[14 - i]).abs() < 1e-15);
        }
        for i in 0..10 {
            assert!((GL21_NODES[i] + GL21_NODES[20 - i]).abs() < 1e-15);
        }
    }
}
