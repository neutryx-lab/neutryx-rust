//! Antithetic variance reduction for Monte Carlo simulations.
//!
//! Provides [`AntitheticGenerator`] which generates antithetic (negated)
//! normal variates and combines original and antithetic simulation results
//! to reduce variance.

/// Generator for antithetic Monte Carlo variance reduction.
///
/// Given a set of standard normal draws `Z`, the antithetic variates are
/// `-Z`. Averaging the payoffs from both sets reduces variance for
/// monotonic payoff functions.
#[derive(Clone, Debug, Default)]
pub struct AntitheticGenerator;

impl AntitheticGenerator {
    /// Creates a new antithetic generator.
    #[inline]
    pub fn new() -> Self { Self }

    /// Generates antithetic pairs from the given normal variates.
    ///
    /// Returns a tuple `(original, antithetic)` where the antithetic
    /// values are the negation of the originals.
    pub fn generate_pairs(normals: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let original = normals.to_vec();
        let antithetic = normals.iter().map(|&z| -z).collect();
        (original, antithetic)
    }

    /// Combines results from original and antithetic simulations by averaging.
    ///
    /// # Panics
    ///
    /// Panics if `original` and `antithetic` have different lengths.
    pub fn combine_results(original: &[f64], antithetic: &[f64]) -> Vec<f64> {
        assert_eq!(
            original.len(),
            antithetic.len(),
            "original and antithetic slices must have equal length"
        );
        original
            .iter()
            .zip(antithetic.iter())
            .map(|(&o, &a)| 0.5 * (o + a))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_generate_pairs_basic() {
        let normals = vec![1.0, -0.5, 2.3, 0.0];
        let (orig, anti) = AntitheticGenerator::generate_pairs(&normals);

        assert_eq!(orig.len(), 4);
        assert_eq!(anti.len(), 4);

        for (i, &z) in normals.iter().enumerate() {
            assert_relative_eq!(orig[i], z);
            assert_relative_eq!(anti[i], -z);
        }
    }

    #[test]
    fn test_generate_pairs_empty() {
        let (orig, anti) = AntitheticGenerator::generate_pairs(&[]);
        assert!(orig.is_empty());
        assert!(anti.is_empty());
    }

    #[test]
    fn test_combine_results() {
        let original = vec![10.0, 20.0, 30.0];
        let antithetic = vec![12.0, 18.0, 32.0];

        let combined = AntitheticGenerator::combine_results(&original, &antithetic);
        assert_eq!(combined.len(), 3);
        assert_relative_eq!(combined[0], 11.0);
        assert_relative_eq!(combined[1], 19.0);
        assert_relative_eq!(combined[2], 31.0);
    }

    #[test]
    fn test_combine_results_empty() {
        let combined = AntitheticGenerator::combine_results(&[], &[]);
        assert!(combined.is_empty());
    }

    #[test]
    #[should_panic(expected = "equal length")]
    fn test_combine_results_mismatched_lengths_panics() {
        AntitheticGenerator::combine_results(&[1.0, 2.0], &[3.0]);
    }

    #[test]
    fn test_antithetic_reduces_variance_for_linear_payoff() {
        // For a linear payoff f(Z) = Z, combining Z and -Z gives exactly 0.
        let normals = vec![1.5, -0.3, 0.7, -1.2];
        let (orig, anti) = AntitheticGenerator::generate_pairs(&normals);
        let combined = AntitheticGenerator::combine_results(&orig, &anti);

        for &val in &combined {
            assert_relative_eq!(val, 0.0, epsilon = 1e-15);
        }
    }

    #[test]
    fn test_antithetic_pairs_sum_to_zero() {
        let normals = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let (orig, anti) = AntitheticGenerator::generate_pairs(&normals);

        let sum_orig: f64 = orig.iter().sum();
        let sum_anti: f64 = anti.iter().sum();
        assert_relative_eq!(sum_orig + sum_anti, 0.0, epsilon = 1e-15);
    }

    #[test]
    fn test_new_creates_default() {
        let _gen = AntitheticGenerator::new();
        // Ensure it compiles and is usable.
        let (orig, anti) = AntitheticGenerator::generate_pairs(&[1.0]);
        assert_relative_eq!(orig[0], 1.0);
        assert_relative_eq!(anti[0], -1.0);
    }
}
