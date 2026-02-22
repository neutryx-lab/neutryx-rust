//! TARN (Target Redemption Note) pricing engine via state space expansion.
//!
//! Extends the Gaussian tree with a second dimension tracking cumulative
//! coupon amounts. When the cumulative coupon reaches the target amount,
//! the product auto-redeems. This path-dependent feature requires a 2D
//! grid: dimension 1 is the Gaussian state variable, dimension 2 is the
//! discretised cumulative coupon.

use num_traits::Float;
use pricer_core::math::numeric::from_f64;

use super::gaussian::GaussianTree;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for TARN pricing on a Gaussian tree.
#[derive(Debug, Clone)]
pub struct TarnConfig<T: Float> {
    /// Target cumulative coupon amount for auto-redemption.
    pub tarn_amount: T,
    /// Number of grid points for cumulative coupon dimension (default 10).
    pub num_coupon_grid_points: usize,
    /// Whether excess coupon above target is paid on auto-redemption.
    pub excess_coupon_flag: bool,
    /// Whether the product also has Bermudan exercise features.
    pub has_bermudan_exercise: bool,
    /// Callable (issuer) or puttable (holder) direction.
    pub is_callable: bool,
}

impl<T: Float> Default for TarnConfig<T> {
    fn default() -> Self {
        Self {
            tarn_amount: T::zero(),
            num_coupon_grid_points: 10,
            excess_coupon_flag: false,
            has_bermudan_exercise: false,
            is_callable: false,
        }
    }
}

// ---------------------------------------------------------------------------
// 2D Grid
// ---------------------------------------------------------------------------

/// Two-dimensional grid for TARN pricing.
///
/// Dimension 1 is the Gaussian state variable (index `j`, size `num_j`).
/// Dimension 2 is the discretised cumulative coupon (index `k`, size `num_k`).
/// Data is stored in row-major order: `data[j * num_k + k]`.
#[derive(Debug, Clone)]
pub struct TarnGrid<T: Float> {
    /// Number of Gaussian state nodes (dimension 1).
    num_j: usize,
    /// Number of cumulative coupon grid points (dimension 2).
    num_k: usize,
    /// Grid spacing for cumulative coupon dimension.
    d_pres: T,
    /// Flat row-major data: `data[j * num_k + k]`.
    data: Vec<T>,
}

impl<T: Float> TarnGrid<T> {
    /// Creates a new grid initialised to zero.
    ///
    /// `tarn_amount` is the target cumulative coupon; the grid spacing is
    /// `tarn_amount / num_k`.
    pub fn new(num_j: usize, num_k: usize, tarn_amount: T) -> Self {
        let d_pres = if num_k > 0 {
            tarn_amount / from_f64(num_k as f64)
        } else {
            T::zero()
        };
        Self {
            num_j,
            num_k,
            d_pres,
            data: vec![T::zero(); num_j * num_k],
        }
    }

    /// Returns the value at Gaussian node `j` and cumulative coupon index `k`.
    #[inline]
    pub fn get(&self, j: usize, k: usize) -> T {
        self.data[j * self.num_k + k]
    }

    /// Sets the value at Gaussian node `j` and cumulative coupon index `k`.
    #[inline]
    pub fn set(&mut self, j: usize, k: usize, value: T) {
        self.data[j * self.num_k + k] = value;
    }

    /// Returns the cumulative coupon amount at grid index `k`.
    #[inline]
    pub fn cumulative_at(&self, k: usize) -> T {
        from_f64::<T>(k as f64) * self.d_pres
    }

    /// Returns the grid spacing for the cumulative coupon dimension.
    #[inline]
    pub fn d_pres(&self) -> T {
        self.d_pres
    }

    /// Returns the number of Gaussian state nodes.
    #[inline]
    pub fn num_j(&self) -> usize {
        self.num_j
    }

    /// Returns the number of cumulative coupon grid points.
    #[inline]
    pub fn num_k(&self) -> usize {
        self.num_k
    }

    /// Resets all grid values to zero.
    pub fn fill_zero(&mut self) {
        for v in &mut self.data {
            *v = T::zero();
        }
    }

    /// Returns the total memory usage of the grid data in bytes.
    pub fn memory_bytes(&self) -> usize {
        self.data.len() * std::mem::size_of::<T>()
    }
}

// ---------------------------------------------------------------------------
// Coupon and Exercise info
// ---------------------------------------------------------------------------

/// Coupon information at one time step for TARN pricing.
#[derive(Debug, Clone)]
pub struct TarnCouponInfo<T: Float> {
    /// Coupon value at each Gaussian node (before cumulation).
    pub node_coupons: Vec<T>,
}

/// Bermudan exercise values at one time step for TARN pricing.
#[derive(Debug, Clone)]
pub struct TarnExerciseInfo<T: Float> {
    /// Exercise value at each Gaussian node.
    pub node_values: Vec<T>,
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

/// Result of TARN tree pricing.
#[derive(Debug, Clone)]
pub struct TarnTreeResult<T: Float> {
    /// Present value at root (state=0, cumulative=0).
    pub pv: T,
    /// Probability of auto-redemption (reaching target).
    pub auto_redemption_probability: T,
    /// Expected time to auto-redemption.
    pub expected_redemption_time: T,
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// TARN pricing engine using state space expansion on a Gaussian tree.
pub struct TarnTreeEngine;

impl TarnTreeEngine {
    /// Prices a TARN product on a Gaussian trinomial tree.
    ///
    /// # Arguments
    ///
    /// * `tree` - Pre-built Gaussian tree with time slices and transitions.
    /// * `config` - TARN-specific configuration (target amount, grid size, etc.).
    /// * `coupons` - Sparse list of `(step_index, coupon_info)` pairs indicating
    ///   which time steps have coupon payments.
    /// * `exercises` - Sparse list of `(step_index, exercise_info)` pairs for
    ///   optional Bermudan exercise dates.
    /// * `redemption_value` - Par value paid on auto-redemption or at maturity.
    ///
    /// # Returns
    ///
    /// A [`TarnTreeResult`] containing the present value and (placeholder)
    /// auto-redemption statistics.
    #[allow(clippy::too_many_lines)]
    pub fn price<T: Float>(
        tree: &GaussianTree<T>,
        config: &TarnConfig<T>,
        coupons: &[(usize, TarnCouponInfo<T>)],
        exercises: &[(usize, TarnExerciseInfo<T>)],
        redemption_value: T,
    ) -> TarnTreeResult<T> {
        let num_j = tree.num_nodes();
        let num_k = config.num_coupon_grid_points;
        let num_steps = tree.num_steps();
        let d_pres = config.tarn_amount / from_f64::<T>(num_k as f64);

        // Build lookup tables: step -> Option<index into coupons/exercises>
        let mut coupon_at_step: Vec<Option<usize>> = vec![None; num_steps + 1];
        for (idx, &(step, _)) in coupons.iter().enumerate() {
            if step <= num_steps {
                coupon_at_step[step] = Some(idx);
            }
        }

        let mut exercise_at_step: Vec<Option<usize>> = vec![None; num_steps + 1];
        for (idx, &(step, _)) in exercises.iter().enumerate() {
            if step <= num_steps {
                exercise_at_step[step] = Some(idx);
            }
        }

        // Double buffers for backward induction
        let mut grid_next = TarnGrid::new(num_j, num_k, config.tarn_amount);
        let mut grid_current = TarnGrid::new(num_j, num_k, config.tarn_amount);

        // Terminal payoff: at maturity, the product expires with no further value
        // grid_next is already initialised to zero

        // Backward induction
        for step in (0..num_steps).rev() {
            let transition = tree.transition(step);

            for j in 0..num_j {
                // Get coupon at this node if this is a coupon date
                let coupon_j = if let Some(idx) = coupon_at_step[step] {
                    coupons[idx].1.node_coupons[j]
                } else {
                    T::zero()
                };

                for k in 0..num_k {
                    let old_cumulative = from_f64::<T>(k as f64) * d_pres;
                    let new_cumulative = old_cumulative + coupon_j;

                    let value = if new_cumulative >= config.tarn_amount {
                        // AUTO-REDEMPTION: cumulative coupon reaches target
                        let final_coupon = if config.excess_coupon_flag {
                            // Full coupon paid even if it exceeds target
                            coupon_j
                        } else {
                            // Cap coupon at remaining amount to target
                            let remaining = config.tarn_amount - old_cumulative;
                            if remaining > T::zero() {
                                remaining
                            } else {
                                T::zero()
                            }
                        };
                        redemption_value + final_coupon
                    } else {
                        // CONTINUE: interpolate in cumulative dimension
                        // Find bracketing k indices in the NEXT grid
                        let k_float = new_cumulative / d_pres;
                        let k_low = k_float
                            .floor()
                            .to_usize()
                            .unwrap_or(0)
                            .min(num_k - 1);
                        let k_high = (k_low + 1).min(num_k - 1);
                        let w = k_float - from_f64::<T>(k_low as f64);
                        let w = if w < T::zero() {
                            T::zero()
                        } else if w > T::one() {
                            T::one()
                        } else {
                            w
                        };

                        // Compute expected continuation value at next step
                        let (p_down, p_mid, p_up, j_center) = transition.transitions[j];
                        let j_down = if j_center > 0 { j_center - 1 } else { 0 };
                        let j_up = (j_center + 1).min(num_j - 1);

                        // Continuation at k_low
                        let cont_low = p_down * grid_next.get(j_down, k_low)
                            + p_mid * grid_next.get(j_center, k_low)
                            + p_up * grid_next.get(j_up, k_low);

                        // Continuation at k_high
                        let cont_high = p_down * grid_next.get(j_down, k_high)
                            + p_mid * grid_next.get(j_center, k_high)
                            + p_up * grid_next.get(j_up, k_high);

                        // Linear interpolation in cumulative coupon dimension
                        (T::one() - w) * cont_low + w * cont_high + coupon_j
                    };

                    // Bermudan exercise overlay
                    let final_value = if config.has_bermudan_exercise {
                        if let Some(ex_idx) = exercise_at_step[step] {
                            let exercise_val = exercises[ex_idx].1.node_values[j];
                            if config.is_callable {
                                // Issuer exercises to minimise value
                                if exercise_val < value {
                                    exercise_val
                                } else {
                                    value
                                }
                            } else {
                                // Holder exercises to maximise value
                                if exercise_val > value {
                                    exercise_val
                                } else {
                                    value
                                }
                            }
                        } else {
                            value
                        }
                    } else {
                        value
                    };

                    grid_current.set(j, k, final_value);
                }
            }

            // Swap grids: current becomes next for the preceding step
            std::mem::swap(&mut grid_current, &mut grid_next);
        }

        // After the loop, grid_next holds the values at step 0
        // (because of the final swap).
        let center_j = num_j / 2;
        let pv = grid_next.get(center_j, 0);

        // Auto-redemption probability and expected time require a forward
        // pass; set to zero for the initial implementation.
        TarnTreeResult {
            pv,
            auto_redemption_probability: T::zero(),
            expected_redemption_time: T::zero(),
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::gaussian::{GaussianTree, GaussianTreeConfig};

    /// Helper: build a small Gaussian tree for testing.
    fn build_test_tree(n_steps: usize) -> GaussianTree<f64> {
        let t_max = 1.0;
        let dt = t_max / n_steps as f64;
        let times: Vec<f64> = (0..=n_steps).map(|i| i as f64 * dt).collect();

        let config = GaussianTreeConfig {
            mean_reversion: 0.1,
            volatility: 0.01,
            times,
            num_std_devs: 5.0,
            num_grid_points: 11,
        };
        GaussianTree::build(config).unwrap()
    }

    #[test]
    fn test_tarn_grid_basic() {
        let grid = TarnGrid::<f64>::new(5, 10, 1.0);
        assert_eq!(grid.num_j(), 5);
        assert_eq!(grid.num_k(), 10);

        // d_pres should be tarn_amount / num_k = 1.0 / 10 = 0.1
        assert!((grid.d_pres() - 0.1).abs() < 1e-12);

        // All values should be initialised to zero
        for j in 0..5 {
            for k in 0..10 {
                assert!((grid.get(j, k) - 0.0).abs() < 1e-15);
            }
        }

        // Test set/get
        let mut grid = grid;
        grid.set(2, 3, 42.0);
        assert!((grid.get(2, 3) - 42.0).abs() < 1e-15);
        assert!((grid.get(0, 0) - 0.0).abs() < 1e-15);

        // Test cumulative_at
        assert!((grid.cumulative_at(0) - 0.0).abs() < 1e-12);
        assert!((grid.cumulative_at(5) - 0.5).abs() < 1e-12);
        assert!((grid.cumulative_at(10) - 1.0).abs() < 1e-10);

        // Test fill_zero
        grid.fill_zero();
        assert!((grid.get(2, 3) - 0.0).abs() < 1e-15);
    }

    #[test]
    fn test_tarn_grid_memory() {
        let grid = TarnGrid::<f64>::new(11, 20, 1.0);
        let expected = 11 * 20 * std::mem::size_of::<f64>();
        assert_eq!(grid.memory_bytes(), expected);
    }

    #[test]
    fn test_tarn_immediate_redemption() {
        // When tarn_amount is very small, any coupon triggers immediate
        // auto-redemption, so PV should equal redemption_value + first coupon.
        let tree = build_test_tree(4);
        let num_j = tree.num_nodes();
        let coupon_val = 0.05;
        let redemption = 100.0;

        let config = TarnConfig {
            tarn_amount: 1e-12,
            num_coupon_grid_points: 10,
            excess_coupon_flag: true,
            has_bermudan_exercise: false,
            is_callable: false,
        };

        // Place a coupon at step 0 with uniform coupon across all nodes
        let coupon_info = TarnCouponInfo {
            node_coupons: vec![coupon_val; num_j],
        };
        let coupons = vec![(0_usize, coupon_info)];
        let exercises: Vec<(usize, TarnExerciseInfo<f64>)> = vec![];

        let result = TarnTreeEngine::price(&tree, &config, &coupons, &exercises, redemption);

        // With excess_coupon_flag=true and tarn_amount ~ 0, the full coupon
        // is paid plus redemption value.
        assert!(
            (result.pv - (redemption + coupon_val)).abs() < 1e-6,
            "Expected PV ~ {}, got {}",
            redemption + coupon_val,
            result.pv
        );
    }

    #[test]
    fn test_tarn_no_redemption() {
        // Very large tarn_amount means the product never auto-redeems.
        // With no coupons, the continuation value propagates back as zero
        // (terminal payoff is zero and no coupons are added).
        let tree = build_test_tree(4);
        let num_j = tree.num_nodes();

        let config = TarnConfig {
            tarn_amount: 1e10,
            num_coupon_grid_points: 10,
            excess_coupon_flag: false,
            has_bermudan_exercise: false,
            is_callable: false,
        };

        let coupons: Vec<(usize, TarnCouponInfo<f64>)> = vec![];
        let exercises: Vec<(usize, TarnExerciseInfo<f64>)> = vec![];
        let redemption = 100.0;

        let result = TarnTreeEngine::price(&tree, &config, &coupons, &exercises, redemption);

        // With no coupons and a huge target, cumulative never reaches the
        // target, and terminal payoff is zero, so PV should be ~ 0.
        assert!(
            result.pv.abs() < 1e-10,
            "Expected PV ~ 0, got {}",
            result.pv
        );

        // Now add small coupons at every step -- PV should be the sum of
        // coupons rolled back through the tree.
        let coupon_val = 0.01;
        let mut coupons_with_values: Vec<(usize, TarnCouponInfo<f64>)> = Vec::new();
        for step in 0..tree.num_steps() {
            coupons_with_values.push((
                step,
                TarnCouponInfo {
                    node_coupons: vec![coupon_val; num_j],
                },
            ));
        }

        let result2 = TarnTreeEngine::price(
            &tree,
            &config,
            &coupons_with_values,
            &exercises,
            redemption,
        );

        // PV should be positive (sum of rolled-back coupons)
        assert!(
            result2.pv > 0.0,
            "Expected positive PV with coupons, got {}",
            result2.pv
        );
        // PV should be less than sum of all coupons (no discounting beyond
        // tree rollback) plus redemption. Since we never redeem, PV is just
        // the accumulated coupons rolled back.
        let max_coupon_sum = coupon_val * tree.num_steps() as f64;
        assert!(
            result2.pv <= max_coupon_sum + 1e-10,
            "PV {} exceeds total coupons {}",
            result2.pv,
            max_coupon_sum
        );
    }

    #[test]
    fn test_tarn_with_coupons() {
        // Moderate tarn_amount with known coupons; verify PV is reasonable.
        let tree = build_test_tree(4);
        let num_j = tree.num_nodes();
        let coupon_val = 0.03;
        let redemption = 100.0;

        let config = TarnConfig {
            tarn_amount: 0.10,
            num_coupon_grid_points: 20,
            excess_coupon_flag: false,
            has_bermudan_exercise: false,
            is_callable: false,
        };

        // Place coupons at every step
        let mut coupons: Vec<(usize, TarnCouponInfo<f64>)> = Vec::new();
        for step in 0..tree.num_steps() {
            coupons.push((
                step,
                TarnCouponInfo {
                    node_coupons: vec![coupon_val; num_j],
                },
            ));
        }
        let exercises: Vec<(usize, TarnExerciseInfo<f64>)> = vec![];

        let result = TarnTreeEngine::price(&tree, &config, &coupons, &exercises, redemption);

        // PV should be between 0 and sum of all coupons + redemption
        let total_coupons = coupon_val * tree.num_steps() as f64;
        assert!(
            result.pv > 0.0,
            "PV should be positive, got {}",
            result.pv
        );
        assert!(
            result.pv <= total_coupons + redemption + 1e-6,
            "PV {} exceeds upper bound {}",
            result.pv,
            total_coupons + redemption
        );
    }

    #[test]
    fn test_tarn_excess_coupon_flag() {
        // With excess_coupon_flag=true, the full coupon is paid even if it
        // exceeds the remaining target. With flag=false, the coupon is capped.
        // The PV with flag=true should be >= PV with flag=false.
        let tree = build_test_tree(4);
        let num_j = tree.num_nodes();
        let coupon_val = 0.06;
        let redemption = 100.0;

        // Place coupons at every step
        let mut coupons: Vec<(usize, TarnCouponInfo<f64>)> = Vec::new();
        for step in 0..tree.num_steps() {
            coupons.push((
                step,
                TarnCouponInfo {
                    node_coupons: vec![coupon_val; num_j],
                },
            ));
        }
        let exercises: Vec<(usize, TarnExerciseInfo<f64>)> = vec![];

        let config_excess = TarnConfig {
            tarn_amount: 0.10,
            num_coupon_grid_points: 20,
            excess_coupon_flag: true,
            has_bermudan_exercise: false,
            is_callable: false,
        };

        let config_no_excess = TarnConfig {
            tarn_amount: 0.10,
            num_coupon_grid_points: 20,
            excess_coupon_flag: false,
            has_bermudan_exercise: false,
            is_callable: false,
        };

        let result_excess =
            TarnTreeEngine::price(&tree, &config_excess, &coupons, &exercises, redemption);
        let result_no_excess =
            TarnTreeEngine::price(&tree, &config_no_excess, &coupons, &exercises, redemption);

        assert!(
            result_excess.pv >= result_no_excess.pv - 1e-10,
            "Excess coupon PV ({}) should be >= no-excess PV ({})",
            result_excess.pv,
            result_no_excess.pv
        );

        // They should differ when the coupon overshoots the target
        assert!(
            (result_excess.pv - result_no_excess.pv).abs() > 1e-8,
            "Expected different PVs: excess={}, no_excess={}",
            result_excess.pv,
            result_no_excess.pv
        );
    }

    #[test]
    fn test_tarn_config_default() {
        let config = TarnConfig::<f64>::default();
        assert!((config.tarn_amount - 0.0).abs() < 1e-15);
        assert_eq!(config.num_coupon_grid_points, 10);
        assert!(!config.excess_coupon_flag);
        assert!(!config.has_bermudan_exercise);
        assert!(!config.is_callable);
    }
}
