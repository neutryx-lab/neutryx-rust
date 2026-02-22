//! Integral adjuster (moment matching) for Gaussian tree discretisation.
//!
//! Prevents arbitrage arising from mapping continuous-distribution quantities
//! onto a discrete tree grid. At each time step, applies additive corrections
//! to rates and multiplicative corrections to discount factors so that
//! tree-implied expectations match the analytical values from the yield curve.

use pricer_core::math::numeric::from_f64;
use pricer_core::traits::Float;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Correction type produced by moment matching.
#[derive(Debug, Clone, Copy)]
pub enum MomentMatchCorrection<T: Float> {
    /// Additive correction applied to rates.
    Additive { adder: T },
    /// Multiplicative correction applied to discount factors.
    Multiplicative { multiplier: T },
}

/// Result of a single moment-matching computation.
#[derive(Debug, Clone)]
pub struct MomentMatchResult<T: Float> {
    /// Expected value implied by the tree.
    pub tree_expected: T,
    /// Analytical (yield-curve) expected value.
    pub analytical_expected: T,
    /// Correction that reconciles the two.
    pub correction: MomentMatchCorrection<T>,
}

/// Per-time-step additive and multiplicative corrections for a Gaussian tree.
///
/// At each time step the adjuster stores:
/// - an **adder** (applied to rates so that the tree-implied forward
///   matches the curve forward), and
/// - a **multiplier** (applied to discount factors so that the tree-implied
///   discount factor matches the curve discount factor).
#[derive(Debug, Clone)]
pub struct IntegralAdjusterNormal<T: Float> {
    /// Per time-step additive corrections (for rates).
    pub adders: Vec<T>,
    /// Per time-step multiplicative corrections (for discount factors).
    pub multipliers: Vec<T>,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl<T: Float> IntegralAdjusterNormal<T> {
    /// Create a new adjuster with `num_steps` entries initialised to
    /// zero (adders) and one (multipliers).
    pub fn new(num_steps: usize) -> Self {
        Self {
            adders: vec![T::zero(); num_steps],
            multipliers: vec![T::one(); num_steps],
        }
    }

    // -- static computation helpers ----------------------------------------

    /// Compute the additive correction for a set of node values.
    ///
    /// `tree_expected = sum(node_values[i] * probabilities[i])`
    /// `adder = analytical_expected - tree_expected`
    pub fn compute_additive_correction(
        node_values: &[T],
        probabilities: &[T],
        analytical_expected: T,
    ) -> MomentMatchResult<T> {
        let tree_expected = node_values
            .iter()
            .zip(probabilities.iter())
            .fold(T::zero(), |acc, (&v, &p)| acc + v * p);

        let adder = analytical_expected - tree_expected;

        MomentMatchResult {
            tree_expected,
            analytical_expected,
            correction: MomentMatchCorrection::Additive { adder },
        }
    }

    /// Compute the multiplicative correction for a set of node values.
    ///
    /// `tree_expected = sum(node_values[i] * probabilities[i])`
    /// `multiplier = analytical_df / tree_expected`
    ///
    /// If `|tree_expected| < 1e-15` the multiplier defaults to `1.0`
    /// to avoid division by zero.
    pub fn compute_multiplicative_correction(
        node_values: &[T],
        probabilities: &[T],
        analytical_df: T,
    ) -> MomentMatchResult<T> {
        let tree_expected = node_values
            .iter()
            .zip(probabilities.iter())
            .fold(T::zero(), |acc, (&v, &p)| acc + v * p);

        let eps: T = from_f64(1e-15);
        let multiplier = if tree_expected.abs() < eps {
            T::one()
        } else {
            analytical_df / tree_expected
        };

        MomentMatchResult {
            tree_expected,
            analytical_expected: analytical_df,
            correction: MomentMatchCorrection::Multiplicative { multiplier },
        }
    }

    // -- in-place application helpers --------------------------------------

    /// Shift every element of `values` by `adder`.
    pub fn apply_additive(values: &mut [T], adder: T) {
        for v in values.iter_mut() {
            *v = *v + adder;
        }
    }

    /// Scale every element of `values` by `multiplier`.
    pub fn apply_multiplicative(values: &mut [T], multiplier: T) {
        for v in values.iter_mut() {
            *v = *v * multiplier;
        }
    }

    /// Apply a [`MomentMatchCorrection`] to a slice of values.
    pub fn apply_correction(correction: &MomentMatchCorrection<T>, values: &mut [T]) {
        match *correction {
            MomentMatchCorrection::Additive { adder } => Self::apply_additive(values, adder),
            MomentMatchCorrection::Multiplicative { multiplier } => {
                Self::apply_multiplicative(values, multiplier);
            }
        }
    }

    // -- per-step accessors ------------------------------------------------

    /// Store an additive correction for a given time step.
    pub fn set_adder(&mut self, step: usize, adder: T) {
        self.adders[step] = adder;
    }

    /// Store a multiplicative correction for a given time step.
    pub fn set_multiplier(&mut self, step: usize, multiplier: T) {
        self.multipliers[step] = multiplier;
    }

    /// Retrieve the additive correction for a given time step.
    pub fn adder(&self, step: usize) -> T {
        self.adders[step]
    }

    /// Retrieve the multiplicative correction for a given time step.
    pub fn multiplier(&self, step: usize) -> T {
        self.multipliers[step]
    }

    // -- convenience: apply stored corrections at a step -------------------

    /// Apply the stored additive correction at `step` to `rates`.
    pub fn adjust_rates_at_step(&self, step: usize, rates: &mut [T]) {
        Self::apply_additive(rates, self.adders[step]);
    }

    /// Apply the stored multiplicative correction at `step` to `dfs`.
    pub fn adjust_dfs_at_step(&self, step: usize, dfs: &mut [T]) {
        Self::apply_multiplicative(dfs, self.multipliers[step]);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let adj = IntegralAdjusterNormal::<f64>::new(5);
        assert_eq!(adj.adders.len(), 5);
        assert_eq!(adj.multipliers.len(), 5);
        for i in 0..5 {
            assert!((adj.adders[i] - 0.0).abs() < f64::EPSILON);
            assert!((adj.multipliers[i] - 1.0).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn test_additive_correction() {
        // Two nodes: values [0.04, 0.06], equal probability [0.5, 0.5]
        // tree_expected = 0.04*0.5 + 0.06*0.5 = 0.05
        // analytical = 0.052  =>  adder = 0.052 - 0.05 = 0.002
        let node_values = [0.04, 0.06];
        let probs = [0.5, 0.5];
        let analytical = 0.052;

        let result =
            IntegralAdjusterNormal::compute_additive_correction(&node_values, &probs, analytical);

        assert!((result.tree_expected - 0.05).abs() < 1e-12);
        assert!((result.analytical_expected - 0.052).abs() < 1e-12);
        match result.correction {
            MomentMatchCorrection::Additive { adder } => {
                assert!((adder - 0.002).abs() < 1e-12);
            }
            _ => panic!("expected Additive correction"),
        }
    }

    #[test]
    fn test_multiplicative_correction() {
        // Three nodes: values [0.95, 0.97, 0.99], probs [0.25, 0.50, 0.25]
        // tree_expected = 0.95*0.25 + 0.97*0.50 + 0.99*0.25 = 0.97
        // analytical_df = 0.975  =>  multiplier = 0.975 / 0.97
        let node_values = [0.95, 0.97, 0.99];
        let probs = [0.25, 0.50, 0.25];
        let analytical_df = 0.975;

        let result = IntegralAdjusterNormal::compute_multiplicative_correction(
            &node_values,
            &probs,
            analytical_df,
        );

        assert!((result.tree_expected - 0.97).abs() < 1e-12);
        assert!((result.analytical_expected - 0.975).abs() < 1e-12);
        let expected_mult = 0.975 / 0.97;
        match result.correction {
            MomentMatchCorrection::Multiplicative { multiplier } => {
                assert!((multiplier - expected_mult).abs() < 1e-12);
            }
            _ => panic!("expected Multiplicative correction"),
        }
    }

    #[test]
    fn test_apply_additive() {
        let mut vals = vec![1.0, 2.0, 3.0];
        IntegralAdjusterNormal::<f64>::apply_additive(&mut vals, 0.5);
        assert!((vals[0] - 1.5).abs() < 1e-12);
        assert!((vals[1] - 2.5).abs() < 1e-12);
        assert!((vals[2] - 3.5).abs() < 1e-12);
    }

    #[test]
    fn test_apply_multiplicative() {
        let mut vals = vec![1.0, 2.0, 3.0];
        IntegralAdjusterNormal::<f64>::apply_multiplicative(&mut vals, 2.0);
        assert!((vals[0] - 2.0).abs() < 1e-12);
        assert!((vals[1] - 4.0).abs() < 1e-12);
        assert!((vals[2] - 6.0).abs() < 1e-12);
    }

    #[test]
    fn test_apply_correction_additive() {
        let correction = MomentMatchCorrection::Additive { adder: 0.1 };
        let mut vals = vec![1.0, 2.0];
        IntegralAdjusterNormal::<f64>::apply_correction(&correction, &mut vals);
        assert!((vals[0] - 1.1).abs() < 1e-12);
        assert!((vals[1] - 2.1).abs() < 1e-12);
    }

    #[test]
    fn test_apply_correction_multiplicative() {
        let correction = MomentMatchCorrection::Multiplicative { multiplier: 3.0 };
        let mut vals = vec![1.0, 2.0];
        IntegralAdjusterNormal::<f64>::apply_correction(&correction, &mut vals);
        assert!((vals[0] - 3.0).abs() < 1e-12);
        assert!((vals[1] - 6.0).abs() < 1e-12);
    }

    #[test]
    fn test_multiplicative_near_zero_safety() {
        // If tree_expected is essentially zero the multiplier should fall back to 1.0.
        let node_values = [1e-18, -1e-18];
        let probs = [0.5, 0.5];
        let analytical_df = 0.98;

        let result = IntegralAdjusterNormal::compute_multiplicative_correction(
            &node_values,
            &probs,
            analytical_df,
        );

        match result.correction {
            MomentMatchCorrection::Multiplicative { multiplier } => {
                assert!(
                    (multiplier - 1.0).abs() < 1e-12,
                    "expected multiplier = 1.0 for near-zero tree_expected, got {multiplier}"
                );
            }
            _ => panic!("expected Multiplicative correction"),
        }
    }

    #[test]
    fn test_flat_curve_corrections_near_zero() {
        // When all node values are equal and equal the analytical value,
        // corrections should be (effectively) zero.
        let val = 0.05;
        let node_values = [val; 5];
        let probs = [0.2; 5];

        // Additive: adder should be ~0
        let add_result =
            IntegralAdjusterNormal::compute_additive_correction(&node_values, &probs, val);
        match add_result.correction {
            MomentMatchCorrection::Additive { adder } => {
                assert!(
                    adder.abs() < 1e-12,
                    "expected adder near zero, got {adder}"
                );
            }
            _ => panic!("expected Additive correction"),
        }

        // Multiplicative: multiplier should be ~1
        let mul_result =
            IntegralAdjusterNormal::compute_multiplicative_correction(&node_values, &probs, val);
        match mul_result.correction {
            MomentMatchCorrection::Multiplicative { multiplier } => {
                assert!(
                    (multiplier - 1.0).abs() < 1e-12,
                    "expected multiplier near 1.0, got {multiplier}"
                );
            }
            _ => panic!("expected Multiplicative correction"),
        }
    }
}
