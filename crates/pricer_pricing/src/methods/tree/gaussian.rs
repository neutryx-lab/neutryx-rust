//! Gaussian recombining trinomial tree for interest rate models.
//!
//! The state variable `x` follows an Ornstein-Uhlenbeck process:
//!
//!   `dx = -a * x * dt + sigma * dW`
//!
//! where `a` is the mean reversion speed and `sigma` is the volatility.
//! The tree uses a fixed equispaced grid centred at zero so that all slices
//! share the same spatial grid, which guarantees the recombining property.

use num_traits::Float;
use pricer_core::math::numeric::{from_f64, from_usize};

use crate::pricer::ConfigError;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for a Gaussian recombining trinomial tree.
#[derive(Debug, Clone)]
pub struct GaussianTreeConfig<T: Float> {
    /// Mean reversion speed (`a > 0`).
    pub mean_reversion: T,
    /// Volatility of the OU process (`sigma > 0`).
    pub volatility: T,
    /// Non-uniform time grid. `times[0]` must equal zero.
    pub times: Vec<T>,
    /// Number of standard deviations for grid truncation (default 5.0).
    pub num_std_devs: T,
    /// Number of spatial grid points (must be odd and >= 3).
    pub num_grid_points: usize,
}

// ---------------------------------------------------------------------------
// Tree slice (one point in time)
// ---------------------------------------------------------------------------

/// A single time-slice of the Gaussian tree.
#[derive(Debug, Clone)]
pub struct GaussianTreeSlice<T: Float> {
    /// State variable values on the grid.
    pub x_grid: Vec<T>,
    /// Grid spacing.
    pub dx: T,
    /// Conditional variance `Var(x)` at this time.
    pub conditional_variance: T,
    /// Time at this slice.
    pub time: T,
}

// ---------------------------------------------------------------------------
// Transition probabilities from one slice to the next
// ---------------------------------------------------------------------------

/// Transition probabilities for every source node in a slice.
///
/// Each entry is `(p_down, p_mid, p_up, j_center)` where `j_center` is the
/// index of the central target node in the next slice.
#[derive(Debug, Clone)]
pub struct GaussianTreeTransition<T: Float> {
    /// Per-node transition data: `(p_down, p_mid, p_up, j_center)`.
    pub transitions: Vec<(T, T, T, usize)>,
}

// ---------------------------------------------------------------------------
// The tree itself
// ---------------------------------------------------------------------------

/// A Gaussian recombining trinomial tree.
#[derive(Debug, Clone)]
pub struct GaussianTree<T: Float> {
    /// Tree configuration.
    pub config: GaussianTreeConfig<T>,
    /// Time slices (one per time grid point).
    pub slices: Vec<GaussianTreeSlice<T>>,
    /// Transition data between consecutive slices. Length = `slices.len() - 1`.
    pub transitions: Vec<GaussianTreeTransition<T>>,
}

impl<T: Float> GaussianTree<T> {
    // ---------------------------------------------------------------------
    // Construction
    // ---------------------------------------------------------------------

    /// Builds the tree from the given configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if any parameter is invalid.
    pub fn build(config: GaussianTreeConfig<T>) -> Result<Self, ConfigError> {
        let zero: T = from_f64(0.0);
        let one: T = from_f64(1.0);
        let two: T = from_f64(2.0);

        // ---- validation ----
        if config.mean_reversion <= zero {
            return Err(ConfigError::InvalidModelParameter {
                name: "mean_reversion",
                reason: "mean_reversion must be positive".to_string(),
            });
        }
        if config.volatility <= zero {
            return Err(ConfigError::InvalidModelParameter {
                name: "volatility",
                reason: "volatility must be positive".to_string(),
            });
        }
        #[allow(clippy::manual_is_multiple_of)]
        if config.num_grid_points < 3 || config.num_grid_points % 2 == 0 {
            return Err(ConfigError::InvalidModelParameter {
                name: "num_grid_points",
                reason: "num_grid_points must be odd and >= 3".to_string(),
            });
        }
        if config.times.len() < 2 {
            return Err(ConfigError::InvalidModelParameter {
                name: "times",
                reason: "times must have at least 2 entries".to_string(),
            });
        }
        if config.times[0] != zero {
            return Err(ConfigError::InvalidModelParameter {
                name: "times",
                reason: "times[0] must be zero".to_string(),
            });
        }

        let a = config.mean_reversion;
        let sigma = config.volatility;
        let n = config.num_grid_points;
        let n_t: T = from_usize(n);
        let center_idx = n / 2;

        // ---- Terminal (unconditional) variance for a fixed grid ----
        let t_last = config.times[config.times.len() - 1];
        let terminal_var = (sigma * sigma / (two * a)) * (one - (-two * a * t_last).exp());

        // dx based on terminal variance (guarantees grid covers num_std_devs
        // standard deviations at the last step).
        let dx = two * config.num_std_devs * terminal_var.sqrt() / (n_t - one);

        // Guard: if terminal_var is essentially zero the grid collapses.
        let eps: T = from_f64(1e-30);
        let dx = if dx < eps { from_f64(1e-10) } else { dx };

        // ---- Build slices ----
        let mut slices = Vec::with_capacity(config.times.len());
        for &t_k in &config.times {
            let cond_var = if t_k == zero {
                zero
            } else {
                (sigma * sigma / (two * a)) * (one - (-two * a * t_k).exp())
            };

            let x_grid: Vec<T> = (0..n)
                .map(|i| {
                    let offset: T = from_usize::<T>(i) - from_usize::<T>(center_idx);
                    offset * dx
                })
                .collect();

            slices.push(GaussianTreeSlice {
                x_grid,
                dx,
                conditional_variance: cond_var,
                time: t_k,
            });
        }

        // ---- Build transitions between consecutive slices ----
        let num_transitions = config.times.len() - 1;
        let mut transitions = Vec::with_capacity(num_transitions);

        for k in 0..num_transitions {
            let dt = config.times[k + 1] - config.times[k];
            let exp_neg_a_dt = (-a * dt).exp();
            let var_incr = (sigma * sigma / (two * a)) * (one - (-two * a * dt).exp());

            let mut node_transitions = Vec::with_capacity(n);

            for i in 0..n {
                let x_i = slices[k].x_grid[i];
                let mu = x_i * exp_neg_a_dt;

                // Nearest grid index in next slice.
                let shift = (mu / dx).round();
                let j_raw = Self::to_isize(shift) + center_idx as isize;

                // Clamp so that j-1 and j+1 exist.
                let j_center = j_raw.max(1).min((n as isize) - 2) as usize;

                let x_d = slices[k + 1].x_grid[j_center - 1];
                let x_m = slices[k + 1].x_grid[j_center];
                let x_u = slices[k + 1].x_grid[j_center + 1];

                // Solve 3x3 system for (p_d, p_m, p_u):
                //   p_d + p_m + p_u = 1
                //   p_d*x_d + p_m*x_m + p_u*x_u = mu
                //   p_d*x_d^2 + p_m*x_m^2 + p_u*x_u^2 = mu^2 + var_incr
                let (p_d, p_m, p_u) = Self::solve_probs(x_d, x_m, x_u, mu, var_incr);

                node_transitions.push((p_d, p_m, p_u, j_center));
            }

            transitions.push(GaussianTreeTransition {
                transitions: node_transitions,
            });
        }

        Ok(Self {
            config,
            slices,
            transitions,
        })
    }

    // ---------------------------------------------------------------------
    // Accessors
    // ---------------------------------------------------------------------

    /// Number of time steps (= number of slices minus one).
    #[inline]
    pub fn num_steps(&self) -> usize { self.slices.len().saturating_sub(1) }

    /// Number of spatial grid nodes (same for every slice).
    #[inline]
    pub fn num_nodes(&self) -> usize { self.config.num_grid_points }

    /// Returns the spatial grid at the given time step.
    #[inline]
    pub fn x_grid(&self, step: usize) -> &[T] { &self.slices[step].x_grid }

    /// Returns the time at the given step.
    #[inline]
    pub fn time(&self, step: usize) -> T { self.slices[step].time }

    /// Returns the transition data *from* the given step to the next.
    #[inline]
    pub fn transition(&self, step: usize) -> &GaussianTreeTransition<T> { &self.transitions[step] }

    // ---------------------------------------------------------------------
    // Rollback (backward induction for one step)
    // ---------------------------------------------------------------------

    /// Rolls back option values from `step + 1` to `step`.
    ///
    /// `values_next` has length `num_nodes()` and contains the values at
    /// time step `step + 1`. Returns a vector of length `num_nodes()`.
    pub fn rollback(&self, step: usize, values_next: &[T]) -> Vec<T> {
        let n = self.num_nodes();
        let trans = &self.transitions[step].transitions;
        let last_idx = n - 1;

        let mut values = Vec::with_capacity(n);
        for i in 0..n {
            let (p_d, p_m, p_u, j_center) = trans[i];
            let jd = if j_center == 0 { 0 } else { j_center - 1 };
            let ju = if j_center >= last_idx {
                last_idx
            } else {
                j_center + 1
            };
            let jm = j_center;
            let val = p_d * values_next[jd] + p_m * values_next[jm] + p_u * values_next[ju];
            values.push(val);
        }
        values
    }

    // ---------------------------------------------------------------------
    // Arrow-Debreu prices (forward induction)
    // ---------------------------------------------------------------------

    /// Computes Arrow-Debreu prices at every time slice via forward induction.
    ///
    /// `ad[k][i]` is the Arrow-Debreu price for node `i` at step `k`.
    /// `ad[0]` has a single 1 at the centre node and 0 elsewhere.
    pub fn arrow_debreu_prices(&self) -> Vec<Vec<T>> {
        let zero: T = from_f64(0.0);
        let one: T = from_f64(1.0);
        let n = self.num_nodes();
        let num_slices = self.slices.len();
        let center_idx = n / 2;

        let mut ad: Vec<Vec<T>> = Vec::with_capacity(num_slices);

        // Step 0: probability mass at the centre node.
        let mut ad0 = vec![zero; n];
        ad0[center_idx] = one;
        ad.push(ad0);

        // Forward induction.
        for k in 0..self.num_steps() {
            let mut ad_next = vec![zero; n];
            let trans = &self.transitions[k].transitions;

            for i in 0..n {
                let (p_d, p_m, p_u, j_center) = trans[i];
                let ad_i = ad[k][i];
                if ad_i == zero {
                    continue;
                }

                let jd = if j_center == 0 { 0 } else { j_center - 1 };
                let ju = if j_center >= n - 1 {
                    n - 1
                } else {
                    j_center + 1
                };

                ad_next[jd] = ad_next[jd] + ad_i * p_d;
                ad_next[j_center] = ad_next[j_center] + ad_i * p_m;
                ad_next[ju] = ad_next[ju] + ad_i * p_u;
            }

            ad.push(ad_next);
        }

        ad
    }

    // ---------------------------------------------------------------------
    // Private helpers
    // ---------------------------------------------------------------------

    /// Solves the 3-equation moment-matching system for trinomial
    /// probabilities and clamps / renormalises the result.
    fn solve_probs(x_d: T, x_m: T, x_u: T, mu: T, var: T) -> (T, T, T) {
        let zero: T = from_f64(0.0);
        let one: T = from_f64(1.0);

        // Denominators for the closed-form solution.
        let denom1 = (x_d - x_m) * (x_d - x_u);
        let denom2 = (x_u - x_d) * (x_u - x_m);

        let second_moment = mu * mu + var;

        let eps: T = from_f64(1e-30);
        let (p_d, p_u) = if denom1.abs() < eps || denom2.abs() < eps {
            // Degenerate grid spacing -- fall back to equal probs.
            let third: T = from_f64(1.0 / 3.0);
            (third, third)
        } else {
            let pd = (second_moment - mu * (x_m + x_u) + x_m * x_u) / denom1;
            let pu = (second_moment - mu * (x_d + x_m) + x_d * x_m) / denom2;
            (pd, pu)
        };

        let p_m = one - p_d - p_u;

        // Clamp to [0, 1] and renormalise.
        let p_d = p_d.max(zero).min(one);
        let p_m = p_m.max(zero).min(one);
        let p_u = p_u.max(zero).min(one);

        let total = p_d + p_m + p_u;
        if total > eps {
            (p_d / total, p_m / total, p_u / total)
        } else {
            let third: T = from_f64(1.0 / 3.0);
            (third, third, third)
        }
    }

    /// Converts a generic `Float` to `isize` via `f64` (used only for index
    /// arithmetic).
    fn to_isize(val: T) -> isize { val.to_f64().map(|v| v.round() as isize).unwrap_or(0) }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a default config with reasonable OU parameters.
    fn default_config() -> GaussianTreeConfig<f64> {
        let n_steps = 10;
        let t_max = 5.0;
        let dt = t_max / n_steps as f64;
        let times: Vec<f64> = (0..=n_steps).map(|i| i as f64 * dt).collect();

        GaussianTreeConfig {
            mean_reversion: 0.1,
            volatility: 0.01,
            times,
            num_std_devs: 5.0,
            num_grid_points: 41,
        }
    }

    #[test]
    fn test_build_basic() {
        let cfg = default_config();
        let num_times = cfg.times.len();
        let tree = GaussianTree::build(cfg).unwrap();

        assert_eq!(tree.slices.len(), num_times);
        assert_eq!(tree.transitions.len(), num_times - 1);
        assert_eq!(tree.num_steps(), num_times - 1);
        assert_eq!(tree.num_nodes(), 41);
    }

    #[test]
    fn test_grid_symmetry() {
        let cfg = default_config();
        let tree = GaussianTree::build(cfg).unwrap();

        for slice in &tree.slices {
            let n = slice.x_grid.len();
            for i in 0..n / 2 {
                let left = slice.x_grid[i];
                let right = slice.x_grid[n - 1 - i];
                assert!(
                    (left + right).abs() < 1e-12,
                    "Grid not symmetric: x[{}] = {}, x[{}] = {}",
                    i,
                    left,
                    n - 1 - i,
                    right
                );
            }
            // Centre node should be zero.
            assert!(
                slice.x_grid[n / 2].abs() < 1e-15,
                "Centre node should be zero"
            );
        }
    }

    #[test]
    fn test_transition_probabilities_sum_to_one() {
        let cfg = default_config();
        let tree = GaussianTree::build(cfg).unwrap();

        for (k, trans) in tree.transitions.iter().enumerate() {
            for (i, &(p_d, p_m, p_u, _j)) in trans.transitions.iter().enumerate() {
                let sum = p_d + p_m + p_u;
                assert!(
                    (sum - 1.0).abs() < 1e-10,
                    "Step {}, node {}: probabilities sum to {} (expected 1.0)",
                    k,
                    i,
                    sum
                );
                assert!(p_d >= 0.0, "p_d negative at step {}, node {}", k, i);
                assert!(p_m >= 0.0, "p_m negative at step {}, node {}", k, i);
                assert!(p_u >= 0.0, "p_u negative at step {}, node {}", k, i);
            }
        }
    }

    #[test]
    fn test_rollback_constant() {
        let cfg = default_config();
        let tree = GaussianTree::build(cfg).unwrap();

        let c = 42.0;
        let constant_vec = vec![c; tree.num_nodes()];

        for step in 0..tree.num_steps() {
            let rolled = tree.rollback(step, &constant_vec);
            for (i, &v) in rolled.iter().enumerate() {
                assert!(
                    (v - c).abs() < 1e-10,
                    "Rollback of constant failed at step {}, node {}: got {}",
                    step,
                    i,
                    v
                );
            }
        }
    }

    #[test]
    fn test_arrow_debreu_sum() {
        let cfg = default_config();
        let tree = GaussianTree::build(cfg).unwrap();
        let ad = tree.arrow_debreu_prices();

        for (k, ad_k) in ad.iter().enumerate() {
            let total: f64 = ad_k.iter().copied().sum();
            assert!(
                (total - 1.0).abs() < 1e-8,
                "Arrow-Debreu sum at step {} = {} (expected ~1.0)",
                k,
                total
            );
        }
    }

    #[test]
    fn test_invalid_config() {
        // Zero mean reversion
        let mut cfg = default_config();
        cfg.mean_reversion = 0.0;
        assert!(GaussianTree::build(cfg).is_err());

        // Negative volatility
        let mut cfg = default_config();
        cfg.volatility = -0.01;
        assert!(GaussianTree::build(cfg).is_err());

        // Even grid points
        let mut cfg = default_config();
        cfg.num_grid_points = 40;
        assert!(GaussianTree::build(cfg).is_err());

        // Grid points < 3
        let mut cfg = default_config();
        cfg.num_grid_points = 1;
        assert!(GaussianTree::build(cfg).is_err());

        // Only one time entry
        let mut cfg = default_config();
        cfg.times = vec![0.0];
        assert!(GaussianTree::build(cfg).is_err());

        // times[0] != 0
        let mut cfg = default_config();
        cfg.times[0] = 0.5;
        assert!(GaussianTree::build(cfg).is_err());
    }
}
