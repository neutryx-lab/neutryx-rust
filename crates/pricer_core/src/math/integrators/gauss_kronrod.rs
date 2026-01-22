//! Gauss-Kronrod quadrature for numerical integration with error estimation.
//!
//! Gauss-Kronrod rules extend Gauss-Legendre rules by adding additional nodes
//! while reusing the Gauss nodes. This allows for error estimation by comparing
//! the Gauss result with the Kronrod result.
//!
//! Common rules:
//! - G7-K15: 7-point Gauss embedded in 15-point Kronrod
//! - G10-K21: 10-point Gauss embedded in 21-point Kronrod

use num_traits::Float;

use super::IntegrationResult;

/// Gauss-Kronrod quadrature rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaussKronrodRule {
    /// G7-K15: 7-point Gauss embedded in 15-point Kronrod.
    G7K15,
    /// G10-K21: 10-point Gauss embedded in 21-point Kronrod.
    G10K21,
}

impl GaussKronrodRule {
    /// Returns the number of Kronrod points (total evaluations).
    #[must_use]
    pub const fn num_points(self) -> usize {
        match self {
            Self::G7K15 => 15,
            Self::G10K21 => 21,
        }
    }
}

// G7-K15 rule: 15 Kronrod nodes and weights, 7 Gauss weights (for embedded
// Gauss rule) Nodes are symmetric around 0

/// K15 nodes (on [-1, 1]).
const K15_NODES: [f64; 15] = [
    -0.991_455_371_120_812_6,
    -0.949_107_912_342_758_5,
    -0.864_864_423_359_769_1,
    -0.741_531_185_599_394_4,
    -0.586_087_235_467_691_1,
    -0.405_845_151_377_397_2,
    -0.207_784_955_007_898_47,
    0.0,
    0.207_784_955_007_898_47,
    0.405_845_151_377_397_2,
    0.586_087_235_467_691_1,
    0.741_531_185_599_394_4,
    0.864_864_423_359_769_1,
    0.949_107_912_342_758_5,
    0.991_455_371_120_812_6,
];

/// K15 weights for Kronrod rule.
const K15_WEIGHTS: [f64; 15] = [
    0.022_935_322_010_529_22,
    0.063_092_092_629_978_55,
    0.104_790_010_322_250_18,
    0.140_653_259_715_525_92,
    0.169_004_726_639_267_0,
    0.190_350_578_064_785_4,
    0.204_432_940_075_298_9,
    0.209_482_141_084_727_83,
    0.204_432_940_075_298_9,
    0.190_350_578_064_785_4,
    0.169_004_726_639_267_0,
    0.140_653_259_715_525_92,
    0.104_790_010_322_250_18,
    0.063_092_092_629_978_55,
    0.022_935_322_010_529_22,
];

/// G7 weights for embedded Gauss rule (at K15 nodes with indices
/// 1,3,5,7,9,11,13).
const G7_WEIGHTS_IN_K15: [f64; 7] = [
    0.129_484_966_168_869_69,
    0.279_705_391_489_276_67,
    0.381_830_050_505_118_94,
    0.417_959_183_673_469_4,
    0.381_830_050_505_118_94,
    0.279_705_391_489_276_67,
    0.129_484_966_168_869_69,
];

// G10-K21 rule: 21 Kronrod nodes and weights, 10 Gauss weights

/// K21 nodes (on [-1, 1]).
const K21_NODES: [f64; 21] = [
    -0.995_657_163_025_808_1,
    -0.973_906_528_517_171_7,
    -0.930_157_491_355_708_2,
    -0.865_063_366_688_984_5,
    -0.780_817_726_586_416_9,
    -0.679_409_568_299_024_4,
    -0.562_757_134_668_604_7,
    -0.433_395_394_129_247_2,
    -0.294_392_862_701_460_2,
    -0.148_874_338_981_631_21,
    0.0,
    0.148_874_338_981_631_21,
    0.294_392_862_701_460_2,
    0.433_395_394_129_247_2,
    0.562_757_134_668_604_7,
    0.679_409_568_299_024_4,
    0.780_817_726_586_416_9,
    0.865_063_366_688_984_5,
    0.930_157_491_355_708_2,
    0.973_906_528_517_171_7,
    0.995_657_163_025_808_1,
];

/// K21 weights for Kronrod rule.
const K21_WEIGHTS: [f64; 21] = [
    0.011_694_638_867_371_874,
    0.032_558_162_307_964_73,
    0.054_755_896_574_351_996,
    0.075_039_674_810_919_95,
    0.093_125_454_583_697_61,
    0.109_387_158_802_297_64,
    0.123_491_976_262_065_85,
    0.134_709_217_311_473_33,
    0.142_775_938_577_060_08,
    0.147_739_104_901_338_49,
    0.149_445_554_002_916_91,
    0.147_739_104_901_338_49,
    0.142_775_938_577_060_08,
    0.134_709_217_311_473_33,
    0.123_491_976_262_065_85,
    0.109_387_158_802_297_64,
    0.093_125_454_583_697_61,
    0.075_039_674_810_919_95,
    0.054_755_896_574_351_996,
    0.032_558_162_307_964_73,
    0.011_694_638_867_371_874,
];

/// G10 weights for embedded Gauss rule (at K21 nodes with indices
/// 1,3,5,7,9,11,13,15,17,19).
const G10_WEIGHTS_IN_K21: [f64; 10] = [
    0.066_671_344_308_688_14,
    0.149_451_349_150_580_59,
    0.219_086_362_515_982_04,
    0.269_266_719_309_996_36,
    0.295_524_224_714_752_87,
    0.295_524_224_714_752_87,
    0.269_266_719_309_996_36,
    0.219_086_362_515_982_04,
    0.149_451_349_150_580_59,
    0.066_671_344_308_688_14,
];

/// Computes the definite integral of f(x) from a to b using Gauss-Kronrod
/// quadrature.
///
/// This method provides both an integral estimate and an error estimate by
/// comparing the Gauss result (using a subset of nodes) with the Kronrod result
/// (using all nodes).
///
/// # Arguments
///
/// * `f` - The function to integrate
/// * `a` - Lower bound of integration
/// * `b` - Upper bound of integration
/// * `rule` - The quadrature rule (G7K15 or G10K21)
///
/// # Returns
///
/// An `IntegrationResult` containing the integral value, error estimate, and
/// number of evaluations.
///
/// # Example
///
/// ```
/// use pricer_core::math::integrators::{integrate_gauss_kronrod, GaussKronrodRule};
///
/// // Integrate exp(-x^2) from 0 to 1
/// let result = integrate_gauss_kronrod(|x: f64| (-x * x).exp(), 0.0, 1.0, GaussKronrodRule::G7K15);
/// assert!(result.error_estimate.unwrap() < 1e-10);
/// ```
pub fn integrate_gauss_kronrod<T, F>(
    f: F,
    a: T,
    b: T,
    rule: GaussKronrodRule,
) -> IntegrationResult<T>
where
    T: Float,
    F: Fn(T) -> T,
{
    let half = T::from(0.5).unwrap();

    // Transform from [-1, 1] to [a, b]
    let scale = (b - a) * half;
    let shift = (a + b) * half;

    match rule {
        GaussKronrodRule::G7K15 => {
            // Evaluate function at all K15 nodes
            let mut fvals = [T::zero(); 15];
            for (i, &node) in K15_NODES.iter().enumerate() {
                let t = T::from(node).unwrap();
                let x = scale * t + shift;
                fvals[i] = f(x);
            }

            // Compute Kronrod result (all 15 points)
            let mut kronrod_sum = T::zero();
            for (&fval, &weight) in fvals.iter().zip(K15_WEIGHTS.iter()) {
                let w = T::from(weight).unwrap();
                kronrod_sum = kronrod_sum + w * fval;
            }

            // Compute Gauss result (7 points at indices 1,3,5,7,9,11,13)
            let gauss_indices = [1, 3, 5, 7, 9, 11, 13];
            let mut gauss_sum = T::zero();
            for (&idx, &weight) in gauss_indices.iter().zip(G7_WEIGHTS_IN_K15.iter()) {
                let w = T::from(weight).unwrap();
                gauss_sum = gauss_sum + w * fvals[idx];
            }

            let kronrod_result = scale * kronrod_sum;
            let gauss_result = scale * gauss_sum;

            // Error estimate: |Kronrod - Gauss|
            let error = (kronrod_result - gauss_result).abs();

            IntegrationResult::with_error(kronrod_result, error, 15)
        }

        GaussKronrodRule::G10K21 => {
            // Evaluate function at all K21 nodes
            let mut fvals = [T::zero(); 21];
            for (i, &node) in K21_NODES.iter().enumerate() {
                let t = T::from(node).unwrap();
                let x = scale * t + shift;
                fvals[i] = f(x);
            }

            // Compute Kronrod result (all 21 points)
            let mut kronrod_sum = T::zero();
            for (&fval, &weight) in fvals.iter().zip(K21_WEIGHTS.iter()) {
                let w = T::from(weight).unwrap();
                kronrod_sum = kronrod_sum + w * fval;
            }

            // Compute Gauss result (10 points at indices 1,3,5,7,9,11,13,15,17,19)
            let gauss_indices = [1, 3, 5, 7, 9, 11, 13, 15, 17, 19];
            let mut gauss_sum = T::zero();
            for (&idx, &weight) in gauss_indices.iter().zip(G10_WEIGHTS_IN_K21.iter()) {
                let w = T::from(weight).unwrap();
                gauss_sum = gauss_sum + w * fvals[idx];
            }

            let kronrod_result = scale * kronrod_sum;
            let gauss_result = scale * gauss_sum;

            // Error estimate: |Kronrod - Gauss|
            let error = (kronrod_result - gauss_result).abs();

            IntegrationResult::with_error(kronrod_result, error, 21)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;

    #[test]
    fn test_rule_num_points() {
        assert_eq!(GaussKronrodRule::G7K15.num_points(), 15);
        assert_eq!(GaussKronrodRule::G10K21.num_points(), 21);
    }

    #[test]
    fn test_constant_function() {
        // Integral of 5 from 0 to 2 = 10
        let result = integrate_gauss_kronrod(|_x: f64| 5.0, 0.0, 2.0, GaussKronrodRule::G7K15);
        assert!((result.value - 10.0).abs() < 1e-14);
        assert!(result.error_estimate.unwrap() < 1e-14);
        assert_eq!(result.num_evaluations, 15);
    }

    #[test]
    fn test_linear_function() {
        // Integral of x from 0 to 1 = 0.5
        let result = integrate_gauss_kronrod(|x: f64| x, 0.0, 1.0, GaussKronrodRule::G7K15);
        assert!((result.value - 0.5).abs() < 1e-14);
        assert!(result.error_estimate.unwrap() < 1e-14);
    }

    #[test]
    fn test_quadratic_function() {
        // Integral of x^2 from 0 to 1 = 1/3
        let result = integrate_gauss_kronrod(|x: f64| x * x, 0.0, 1.0, GaussKronrodRule::G7K15);
        assert!((result.value - 1.0 / 3.0).abs() < 1e-14);
    }

    #[test]
    fn test_sine_function() {
        // Integral of sin(x) from 0 to pi = 2
        let result = integrate_gauss_kronrod(|x: f64| x.sin(), 0.0, PI, GaussKronrodRule::G10K21);
        assert!((result.value - 2.0).abs() < 1e-14);
        assert!(result.error_estimate.unwrap() < 1e-10);
    }

    #[test]
    fn test_exponential_function() {
        // Integral of exp(x) from 0 to 1 = e - 1
        let result = integrate_gauss_kronrod(|x: f64| x.exp(), 0.0, 1.0, GaussKronrodRule::G7K15);
        let expected = std::f64::consts::E - 1.0;
        assert!((result.value - expected).abs() < 1e-14);
    }

    #[test]
    fn test_gaussian_integral() {
        // Integral of exp(-x^2) from 0 to 1 ~ 0.7468241328124271
        let result =
            integrate_gauss_kronrod(|x: f64| (-x * x).exp(), 0.0, 1.0, GaussKronrodRule::G7K15);
        let expected = 0.746_824_132_812_427_1;
        assert!((result.value - expected).abs() < 1e-14);
        assert!(result.error_estimate.unwrap() < 1e-10);
    }

    #[test]
    fn test_error_estimate_meaningful() {
        // For a smooth function, error estimate should be small
        let result = integrate_gauss_kronrod(|x: f64| x.sin(), 0.0, PI, GaussKronrodRule::G7K15);
        let error = result.error_estimate.unwrap();

        // Error estimate should be positive
        assert!(error >= 0.0);

        // Error estimate should be much smaller than the result for smooth functions
        assert!(error < 1e-8);
    }

    #[test]
    fn test_g10k21_higher_precision() {
        // For oscillatory functions, G10K21 should give better error estimates
        let result7k15 =
            integrate_gauss_kronrod(|x: f64| (10.0 * x).sin(), 0.0, PI, GaussKronrodRule::G7K15);
        let result10k21 =
            integrate_gauss_kronrod(|x: f64| (10.0 * x).sin(), 0.0, PI, GaussKronrodRule::G10K21);

        // Expected: integral of sin(10x) from 0 to pi = (1 - cos(10*pi)) / 10 = 0
        // (since cos(10*pi) = 1)
        assert!((result10k21.value).abs() < 1e-13);

        // G10K21 should have better accuracy
        assert!(result10k21.error_estimate.unwrap() <= result7k15.error_estimate.unwrap() + 1e-10);
    }

    #[test]
    fn test_k15_weights_sum() {
        // K15 weights should sum to 2
        let sum: f64 = K15_WEIGHTS.iter().sum();
        assert!((sum - 2.0).abs() < 1e-14);
    }

    #[test]
    fn test_k21_weights_sum() {
        // K21 weights should sum to 2
        let sum: f64 = K21_WEIGHTS.iter().sum();
        assert!((sum - 2.0).abs() < 1e-14);
    }

    #[test]
    fn test_g7_weights_sum() {
        // G7 weights in K15 should sum to 2
        let sum: f64 = G7_WEIGHTS_IN_K15.iter().sum();
        assert!((sum - 2.0).abs() < 1e-14);
    }

    #[test]
    fn test_g10_weights_sum() {
        // G10 weights in K21 should sum to 2
        let sum: f64 = G10_WEIGHTS_IN_K21.iter().sum();
        assert!((sum - 2.0).abs() < 1e-14);
    }

    #[test]
    fn test_nodes_symmetry_k15() {
        // K15 nodes should be symmetric around 0
        for i in 0..7 {
            assert!((K15_NODES[i] + K15_NODES[14 - i]).abs() < 1e-15);
        }
    }

    #[test]
    fn test_nodes_symmetry_k21() {
        // K21 nodes should be symmetric around 0
        for i in 0..10 {
            assert!((K21_NODES[i] + K21_NODES[20 - i]).abs() < 1e-15);
        }
    }
}
