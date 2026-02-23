//! Bermudan option pricing engine on a Gaussian recombining tree.
//!
//! Implements backward induction for callable/puttable interest rate
//! products with CIF-decomposed coupon integration. At each exercise
//! date, compares continuation value (holding the product) against
//! exercise value (terminating the swap) to determine optimal strategy.

use num_traits::Float;
use pricer_core::math::numeric::from_f64;

use super::gaussian::GaussianTree;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the Bermudan exercise schedule.
#[derive(Debug, Clone)]
pub struct BermudanTreeConfig<T: Float> {
    /// Whether the issuer can call (terminate) -- callable bond.
    /// If `false`, the holder can put (terminate) -- puttable bond.
    pub is_callable: bool,
    /// Exercise dates as year fractions (subset of tree time steps).
    pub exercise_times: Vec<T>,
    /// Exercise cost at each exercise date (e.g., 0 for no cost).
    /// Must have the same length as `exercise_times`.
    pub exercise_costs: Vec<T>,
    /// Coupon dates as year fractions (subset of tree time steps).
    pub coupon_times: Vec<T>,
}

// ---------------------------------------------------------------------------
// Coupon / Exercise payloads
// ---------------------------------------------------------------------------

/// Coupon information at a single time step for all tree nodes.
#[derive(Debug, Clone)]
pub struct CouponInfo<T: Float> {
    /// Discounted coupon value at each tree node.
    pub values: Vec<T>,
}

/// Exercise value at a single time step for all tree nodes.
#[derive(Debug, Clone)]
pub struct ExerciseInfo<T: Float> {
    /// Value of exercising (terminating the swap) at each node.
    pub values: Vec<T>,
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

/// Result of the Bermudan tree pricing.
#[derive(Debug, Clone)]
pub struct BermudanTreeResult<T: Float> {
    /// Present value at the root (centre) node.
    pub pv: T,
    /// Continuation value at the root node (value if never exercised).
    pub continuation_value: T,
    /// Exercise boundary: for each exercise date, the critical x-value
    /// where the exercise decision switches.
    /// `NAN` if the holder always holds or always exercises at that date.
    pub exercise_boundary: Vec<T>,
    /// Option value embedded in the Bermudan feature.
    ///
    /// For callable: `continuation_value - pv` (the call right reduces
    /// holder value).
    /// For puttable: `pv - continuation_value` (the put right increases
    /// holder value).
    pub option_value: T,
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Bermudan option pricing engine (unit struct).
///
/// Provides static methods for pricing callable/puttable products on a
/// [`GaussianTree`] via backward induction with optional CIF coupon
/// integration.
pub struct BermudanTreeEngine;

impl BermudanTreeEngine {
    /// Prices a Bermudan callable/puttable product on the given tree.
    ///
    /// # Arguments
    ///
    /// * `tree`      - A pre-built [`GaussianTree`].
    /// * `config`    - Bermudan exercise schedule and type.
    /// * `coupons`   - Slice of `(step_index, CouponInfo)` pairs. Each entry
    ///   contributes the coupon cash-flow at the indicated tree step.
    /// * `exercises` - Slice of `(step_index, ExerciseInfo)` pairs. Each entry
    ///   provides the exercise (termination) value at the indicated tree step.
    ///
    /// # Returns
    ///
    /// A [`BermudanTreeResult`] containing PV, continuation value,
    /// exercise boundaries, and option value.
    pub fn price<T: Float>(
        tree: &GaussianTree<T>,
        config: &BermudanTreeConfig<T>,
        coupons: &[(usize, CouponInfo<T>)],
        exercises: &[(usize, ExerciseInfo<T>)],
    ) -> BermudanTreeResult<T> {
        let num_steps = tree.num_steps();
        let num_nodes = tree.num_nodes();
        let zero: T = from_f64(0.0);
        let center = num_nodes / 2;

        // -- Build step-indexed lookup tables --------------------------------
        let coupon_at_step = Self::build_step_lookup(num_steps, coupons);
        let exercise_at_step = Self::build_step_lookup(num_steps, exercises);

        // -- Backward induction WITH exercise --------------------------------
        let mut values_next = vec![zero; num_nodes];

        // We collect boundaries in reverse order (last exercise date first)
        // and reverse at the end.
        let mut boundaries_rev: Vec<T> = Vec::new();

        for step in (0..num_steps).rev() {
            // (a) Rollback: discounted expected value from step+1
            let mut fv_hold = tree.rollback(step, &values_next);

            // (b) Add coupon if this step is a coupon date
            if let Some(idx) = coupon_at_step[step] {
                let coupon_vals = &coupons[idx].1.values;
                for j in 0..num_nodes {
                    fv_hold[j] = fv_hold[j] + coupon_vals[j];
                }
            }

            // (c) Exercise decision if this step is an exercise date
            if let Some(idx) = exercise_at_step[step] {
                let exercise_vals = &exercises[idx].1.values;
                let mut values_current = vec![zero; num_nodes];

                for j in 0..num_nodes {
                    if config.is_callable {
                        // Callable: issuer minimizes value to holder.
                        // Exercise if exercise_value < hold_value.
                        values_current[j] = if exercise_vals[j] < fv_hold[j] {
                            exercise_vals[j]
                        } else {
                            fv_hold[j]
                        };
                    } else {
                        // Puttable: holder maximizes value.
                        // Exercise if exercise_value > hold_value.
                        values_current[j] = if exercise_vals[j] > fv_hold[j] {
                            exercise_vals[j]
                        } else {
                            fv_hold[j]
                        };
                    }
                }

                // Record exercise boundary
                let boundary = Self::find_exercise_boundary(
                    tree,
                    step,
                    &fv_hold,
                    exercise_vals,
                    config.is_callable,
                );
                boundaries_rev.push(boundary);

                values_next = values_current;
            } else {
                values_next = fv_hold;
            }
        }

        let pv = values_next[center];

        // Reverse the boundary list so it corresponds to exercise dates in
        // chronological order.
        boundaries_rev.reverse();
        let exercise_boundary = boundaries_rev;

        // -- Continuation value (no-exercise pass) ---------------------------
        let continuation_value = Self::price_no_exercise(tree, coupons);

        // -- Option value ----------------------------------------------------
        let option_value = if config.is_callable {
            // Callable reduces holder value; option value is positive.
            continuation_value - pv
        } else {
            // Puttable increases holder value; option value is positive.
            pv - continuation_value
        };

        BermudanTreeResult {
            pv,
            continuation_value,
            exercise_boundary,
            option_value,
        }
    }

    /// Prices the product without any exercise -- pure backward induction
    /// with coupon accumulation.
    ///
    /// This gives the "European" or "hold to maturity" value, useful for
    /// computing the embedded option value.
    pub fn price_no_exercise<T: Float>(
        tree: &GaussianTree<T>,
        coupons: &[(usize, CouponInfo<T>)],
    ) -> T {
        let num_steps = tree.num_steps();
        let num_nodes = tree.num_nodes();
        let zero: T = from_f64(0.0);
        let center = num_nodes / 2;

        let coupon_at_step = Self::build_step_lookup(num_steps, coupons);

        let mut values_next = vec![zero; num_nodes];

        for step in (0..num_steps).rev() {
            let mut fv = tree.rollback(step, &values_next);

            if let Some(idx) = coupon_at_step[step] {
                let coupon_vals = &coupons[idx].1.values;
                for j in 0..num_nodes {
                    fv[j] = fv[j] + coupon_vals[j];
                }
            }

            values_next = fv;
        }

        values_next[center]
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Builds a `Vec<Option<usize>>` indexed by tree step. The value, when
    /// present, is the index into the corresponding `items` slice.
    ///
    /// This is an O(1)-lookup alternative to `HashMap` that works well
    /// because step indices are dense and bounded.
    fn build_step_lookup<V>(num_steps: usize, items: &[(usize, V)]) -> Vec<Option<usize>> {
        let mut lookup = vec![None; num_steps];
        for (pos, (step, _)) in items.iter().enumerate() {
            if *step < num_steps {
                lookup[*step] = Some(pos);
            }
        }
        lookup
    }

    /// Detects the exercise boundary at a given tree step.
    ///
    /// Scans nodes from low to high x-value and finds the transition point
    /// where the optimal decision switches between exercise and hold.
    /// Returns the x-value at the boundary, or `NAN` if no switch occurs
    /// (i.e. the decision is uniform across all nodes).
    fn find_exercise_boundary<T: Float>(
        tree: &GaussianTree<T>,
        step: usize,
        hold_values: &[T],
        exercise_values: &[T],
        is_callable: bool,
    ) -> T {
        let nan: T = from_f64(f64::NAN);
        let num_nodes = tree.num_nodes();
        let x_grid = tree.x_grid(step);

        if num_nodes < 2 {
            return nan;
        }

        // Determine exercise decision at each node.
        // `true` means exercise is chosen.
        let exercise_chosen: Vec<bool> = (0..num_nodes)
            .map(|j| {
                if is_callable {
                    exercise_values[j] < hold_values[j]
                } else {
                    exercise_values[j] > hold_values[j]
                }
            })
            .collect();

        // Scan for the first transition.
        for j in 1..num_nodes {
            if exercise_chosen[j] != exercise_chosen[j - 1] {
                // Linear interpolation of the boundary x-value.
                let diff_prev = if is_callable {
                    hold_values[j - 1] - exercise_values[j - 1]
                } else {
                    exercise_values[j - 1] - hold_values[j - 1]
                };
                let diff_curr = if is_callable {
                    hold_values[j] - exercise_values[j]
                } else {
                    exercise_values[j] - hold_values[j]
                };

                let denom = diff_prev - diff_curr;
                let eps: T = from_f64(1e-30);
                if denom.abs() < eps {
                    // Midpoint fallback
                    let two: T = from_f64(2.0);
                    return (x_grid[j - 1] + x_grid[j]) / two;
                }

                // Linearly interpolate: boundary where diff crosses zero.
                let alpha = diff_prev / denom;
                return x_grid[j - 1] + alpha * (x_grid[j] - x_grid[j - 1]);
            }
        }

        // No transition found: uniform decision across the grid.
        nan
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::methods::tree::gaussian::{GaussianTree, GaussianTreeConfig};

    /// Helper: build a small Gaussian tree for testing.
    fn build_test_tree(n_steps: usize, num_grid: usize) -> GaussianTree<f64> {
        let t_max = 5.0;
        let dt = t_max / n_steps as f64;
        let times: Vec<f64> = (0..=n_steps).map(|i| i as f64 * dt).collect();

        let config = GaussianTreeConfig {
            mean_reversion: 0.1,
            volatility: 0.01,
            times,
            num_std_devs: 5.0,
            num_grid_points: num_grid,
        };
        GaussianTree::build(config).unwrap()
    }

    // -----------------------------------------------------------------------
    // test_no_exercise_flat_coupons
    // -----------------------------------------------------------------------

    #[test]
    fn test_no_exercise_flat_coupons() {
        // With a constant coupon at each step and no exercise, PV should
        // equal the sum of (undiscounted, since the tree rollback only
        // applies transition probabilities which sum to 1 for a constant)
        // coupon contributions.
        let n_steps = 10;
        let num_grid = 21;
        let tree = build_test_tree(n_steps, num_grid);

        let coupon_val = 1.0;
        let coupons: Vec<(usize, CouponInfo<f64>)> = (0..n_steps)
            .map(|s| {
                (
                    s,
                    CouponInfo {
                        values: vec![coupon_val; num_grid],
                    },
                )
            })
            .collect();

        let pv = BermudanTreeEngine::price_no_exercise(&tree, &coupons);

        // Since rollback of a constant preserves the constant (probabilities
        // sum to 1), each coupon of 1.0 accumulates exactly. With n_steps
        // coupons added during backward induction, the PV at root should
        // be n_steps * coupon_val.
        let expected = n_steps as f64 * coupon_val;
        assert!(
            (pv - expected).abs() < 1e-8,
            "PV {} should equal {} for flat constant coupons",
            pv,
            expected
        );
    }

    // -----------------------------------------------------------------------
    // test_exercise_always_better
    // -----------------------------------------------------------------------

    #[test]
    fn test_exercise_always_better() {
        // If exercise value is very high (puttable) at every node at the
        // first exercise date, the engine should exercise immediately and
        // PV should equal that exercise value.
        let n_steps = 10;
        let num_grid = 21;
        let tree = build_test_tree(n_steps, num_grid);

        let exercise_value = 1000.0;
        // Place exercise opportunity at step 1 (earliest non-zero step).
        let exercises: Vec<(usize, ExerciseInfo<f64>)> = vec![(
            1,
            ExerciseInfo {
                values: vec![exercise_value; num_grid],
            },
        )];

        let config = BermudanTreeConfig {
            is_callable: false, // puttable: holder maximizes
            exercise_times: vec![tree.time(1)],
            exercise_costs: vec![0.0],
            coupon_times: vec![],
        };

        let result = BermudanTreeEngine::price(&tree, &config, &[], &exercises);

        // Puttable with exercise_value >> 0 and no coupons: should exercise.
        // PV at root = rollback of constant exercise_value from step 1 to 0.
        // Rollback of constant = constant (since probs sum to 1).
        assert!(
            (result.pv - exercise_value).abs() < 1e-8,
            "PV {} should equal exercise value {} when exercise is always optimal",
            result.pv,
            exercise_value
        );
    }

    // -----------------------------------------------------------------------
    // test_exercise_never_better
    // -----------------------------------------------------------------------

    #[test]
    fn test_exercise_never_better() {
        // If exercise value is very low (puttable), exercise is never
        // optimal. PV should equal continuation value.
        let n_steps = 10;
        let num_grid = 21;
        let tree = build_test_tree(n_steps, num_grid);

        let coupon_val = 2.0;
        let coupons: Vec<(usize, CouponInfo<f64>)> = (0..n_steps)
            .map(|s| {
                (
                    s,
                    CouponInfo {
                        values: vec![coupon_val; num_grid],
                    },
                )
            })
            .collect();

        // Exercise value = 0 at all nodes, much worse than accumulated coupons.
        let exercises: Vec<(usize, ExerciseInfo<f64>)> = vec![(
            5,
            ExerciseInfo {
                values: vec![0.0; num_grid],
            },
        )];

        let config = BermudanTreeConfig {
            is_callable: false, // puttable: holder maximizes
            exercise_times: vec![tree.time(5)],
            exercise_costs: vec![0.0],
            coupon_times: (0..n_steps).map(|s| tree.time(s)).collect(),
        };

        let result = BermudanTreeEngine::price(&tree, &config, &coupons, &exercises);

        // Exercise never chosen => PV = continuation value, option value ~ 0
        assert!(
            (result.pv - result.continuation_value).abs() < 1e-8,
            "PV {} should equal continuation {} when exercise is never optimal",
            result.pv,
            result.continuation_value
        );
        assert!(
            result.option_value.abs() < 1e-8,
            "Option value {} should be ~0 when exercise is never optimal",
            result.option_value
        );
    }

    // -----------------------------------------------------------------------
    // test_callable_reduces_value
    // -----------------------------------------------------------------------

    #[test]
    fn test_callable_reduces_value() {
        // For callable, the embedded option reduces holder value:
        // PV <= continuation_value.
        let n_steps = 10;
        let num_grid = 21;
        let tree = build_test_tree(n_steps, num_grid);

        let coupon_val = 5.0;
        let coupons: Vec<(usize, CouponInfo<f64>)> = (0..n_steps)
            .map(|s| {
                (
                    s,
                    CouponInfo {
                        values: vec![coupon_val; num_grid],
                    },
                )
            })
            .collect();

        // Exercise value less than total accumulated coupons but positive,
        // so the issuer may call at some nodes.
        let exercise_val = 20.0;
        let exercises: Vec<(usize, ExerciseInfo<f64>)> = vec![
            (
                3,
                ExerciseInfo {
                    values: vec![exercise_val; num_grid],
                },
            ),
            (
                6,
                ExerciseInfo {
                    values: vec![exercise_val; num_grid],
                },
            ),
        ];

        let config = BermudanTreeConfig {
            is_callable: true,
            exercise_times: vec![tree.time(3), tree.time(6)],
            exercise_costs: vec![0.0, 0.0],
            coupon_times: (0..n_steps).map(|s| tree.time(s)).collect(),
        };

        let result = BermudanTreeEngine::price(&tree, &config, &coupons, &exercises);

        assert!(
            result.pv <= result.continuation_value + 1e-10,
            "Callable PV {} should be <= continuation {} (option_value = {})",
            result.pv,
            result.continuation_value,
            result.option_value
        );
        assert!(
            result.option_value >= -1e-10,
            "Callable option value {} should be >= 0",
            result.option_value
        );
    }

    // -----------------------------------------------------------------------
    // test_puttable_increases_value
    // -----------------------------------------------------------------------

    #[test]
    fn test_puttable_increases_value() {
        // For puttable, the embedded option increases holder value:
        // PV >= continuation_value.
        let n_steps = 10;
        let num_grid = 21;
        let tree = build_test_tree(n_steps, num_grid);

        // Small coupons, large exercise value -> put is valuable.
        let coupon_val = 0.5;
        let coupons: Vec<(usize, CouponInfo<f64>)> = (0..n_steps)
            .map(|s| {
                (
                    s,
                    CouponInfo {
                        values: vec![coupon_val; num_grid],
                    },
                )
            })
            .collect();

        let exercise_val = 100.0;
        let exercises: Vec<(usize, ExerciseInfo<f64>)> = vec![
            (
                3,
                ExerciseInfo {
                    values: vec![exercise_val; num_grid],
                },
            ),
            (
                7,
                ExerciseInfo {
                    values: vec![exercise_val; num_grid],
                },
            ),
        ];

        let config = BermudanTreeConfig {
            is_callable: false, // puttable
            exercise_times: vec![tree.time(3), tree.time(7)],
            exercise_costs: vec![0.0, 0.0],
            coupon_times: (0..n_steps).map(|s| tree.time(s)).collect(),
        };

        let result = BermudanTreeEngine::price(&tree, &config, &coupons, &exercises);

        assert!(
            result.pv >= result.continuation_value - 1e-10,
            "Puttable PV {} should be >= continuation {} (option_value = {})",
            result.pv,
            result.continuation_value,
            result.option_value
        );
        assert!(
            result.option_value >= -1e-10,
            "Puttable option value {} should be >= 0",
            result.option_value
        );
    }

    // -----------------------------------------------------------------------
    // test_exercise_boundary_exists
    // -----------------------------------------------------------------------

    #[test]
    fn test_exercise_boundary_exists() {
        // Construct a scenario where exercise is optimal at some nodes but
        // not others, producing a non-NAN boundary.
        let n_steps = 10;
        let num_grid = 21;
        let tree = build_test_tree(n_steps, num_grid);

        // Exercise values that depend on x: exercise is cheap for negative x
        // (low rates) and expensive for positive x. This creates a boundary.
        let x_grid = tree.x_grid(5);
        let exercise_values: Vec<f64> = x_grid
            .iter()
            .map(|&x| {
                // Exercise value = 50 - 500*x  (decreasing in x)
                50.0 - 500.0 * x
            })
            .collect();

        // Hold values will be the accumulated coupons, which are constant
        // across nodes. We set coupon = 3.0 at each step, so by step 5
        // the hold value ~ 5 * 3 = 15 (from steps 5..9 remaining).
        let coupon_val = 3.0;
        let coupons: Vec<(usize, CouponInfo<f64>)> = (0..n_steps)
            .map(|s| {
                (
                    s,
                    CouponInfo {
                        values: vec![coupon_val; num_grid],
                    },
                )
            })
            .collect();

        let exercises: Vec<(usize, ExerciseInfo<f64>)> = vec![(
            5,
            ExerciseInfo {
                values: exercise_values,
            },
        )];

        let config = BermudanTreeConfig {
            is_callable: true, // issuer calls when exercise < hold
            exercise_times: vec![tree.time(5)],
            exercise_costs: vec![0.0],
            coupon_times: (0..n_steps).map(|s| tree.time(s)).collect(),
        };

        let result = BermudanTreeEngine::price(&tree, &config, &coupons, &exercises);

        // There should be exactly one exercise date -> one boundary entry.
        assert_eq!(
            result.exercise_boundary.len(),
            1,
            "Should have one boundary entry"
        );

        let boundary = result.exercise_boundary[0];
        assert!(
            !boundary.is_nan(),
            "Boundary should be non-NAN when exercise is optimal at some nodes"
        );
    }

    // -----------------------------------------------------------------------
    // test_no_coupons_no_exercise
    // -----------------------------------------------------------------------

    #[test]
    fn test_no_coupons_no_exercise() {
        // With no coupons and no exercise, PV and continuation should both
        // be zero.
        let n_steps = 10;
        let num_grid = 21;
        let tree = build_test_tree(n_steps, num_grid);

        let config = BermudanTreeConfig {
            is_callable: true,
            exercise_times: vec![],
            exercise_costs: vec![],
            coupon_times: vec![],
        };

        let result = BermudanTreeEngine::price(&tree, &config, &[], &[]);

        assert!(
            result.pv.abs() < 1e-15,
            "PV {} should be zero with no coupons and no exercise",
            result.pv
        );
        assert!(
            result.continuation_value.abs() < 1e-15,
            "Continuation {} should be zero",
            result.continuation_value
        );
    }

    // -----------------------------------------------------------------------
    // test_price_no_exercise_matches_full_price_without_exercise_dates
    // -----------------------------------------------------------------------

    #[test]
    fn test_price_no_exercise_matches_full_price_without_exercise_dates() {
        // When no exercise dates are provided, `price` should give the
        // same PV as `price_no_exercise`.
        let n_steps = 10;
        let num_grid = 21;
        let tree = build_test_tree(n_steps, num_grid);

        let coupon_val = 2.5;
        let coupons: Vec<(usize, CouponInfo<f64>)> = (0..n_steps)
            .map(|s| {
                (
                    s,
                    CouponInfo {
                        values: vec![coupon_val; num_grid],
                    },
                )
            })
            .collect();

        let config = BermudanTreeConfig {
            is_callable: true,
            exercise_times: vec![],
            exercise_costs: vec![],
            coupon_times: (0..n_steps).map(|s| tree.time(s)).collect(),
        };

        let result = BermudanTreeEngine::price(&tree, &config, &coupons, &[]);
        let pv_no_ex = BermudanTreeEngine::price_no_exercise(&tree, &coupons);

        assert!(
            (result.pv - pv_no_ex).abs() < 1e-12,
            "price PV {} should match price_no_exercise PV {} when no exercise dates",
            result.pv,
            pv_no_ex
        );
        assert!(
            result.option_value.abs() < 1e-12,
            "Option value {} should be ~0 when no exercise dates",
            result.option_value
        );
    }

    // -----------------------------------------------------------------------
    // test_multiple_exercise_dates
    // -----------------------------------------------------------------------

    #[test]
    fn test_multiple_exercise_dates() {
        // With multiple exercise dates, the number of boundary entries
        // should match the number of exercise dates.
        let n_steps = 10;
        let num_grid = 21;
        let tree = build_test_tree(n_steps, num_grid);

        let exercise_val = 10.0;
        let exercises: Vec<(usize, ExerciseInfo<f64>)> = vec![
            (
                2,
                ExerciseInfo {
                    values: vec![exercise_val; num_grid],
                },
            ),
            (
                4,
                ExerciseInfo {
                    values: vec![exercise_val; num_grid],
                },
            ),
            (
                6,
                ExerciseInfo {
                    values: vec![exercise_val; num_grid],
                },
            ),
            (
                8,
                ExerciseInfo {
                    values: vec![exercise_val; num_grid],
                },
            ),
        ];

        let config = BermudanTreeConfig {
            is_callable: true,
            exercise_times: vec![tree.time(2), tree.time(4), tree.time(6), tree.time(8)],
            exercise_costs: vec![0.0, 0.0, 0.0, 0.0],
            coupon_times: vec![],
        };

        let result = BermudanTreeEngine::price(&tree, &config, &[], &exercises);

        assert_eq!(
            result.exercise_boundary.len(),
            4,
            "Should have 4 boundary entries for 4 exercise dates"
        );
    }
}
