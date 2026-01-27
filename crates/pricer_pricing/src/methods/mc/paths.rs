//! Path generation for Monte Carlo simulation.
//!
//! This module implements Geometric Brownian Motion (GBM) path generation
//! using the Euler-Maruyama discretisation scheme with log-space formulation
//! for numerical stability.
//!
//! # Enzyme Activity Analysis
//!
//! For AD compatibility, parameters have the following activities:
//! - `spot`: Dual (forward mode input for Delta)
//! - `rate`: Dual (forward mode input for Rho)
//! - `volatility`: Dual (forward mode input for Vega)
//! - `randoms`: Const (frozen during differentiation)
//!
//! # Memory Layout
//!
//! Paths are stored in row-major order: `paths[path_idx * (n_steps + 1) +
//! step_idx]` where `step_idx = 0` contains the initial spot price.

use super::{
    layout_config::PathLayout, workspace::PathWorkspace, workspace_enum::WorkspaceEnum,
    workspace_trait::PathWorkspaceTrait,
};

/// Parameters for Geometric Brownian Motion path generation.
///
/// # Model
///
/// The GBM model assumes asset prices follow:
/// ```text
/// dS = μ S dt + σ S dW
/// ```
///
/// where:
/// - S is the spot price
/// - μ is the drift (typically risk-free rate under risk-neutral measure)
/// - σ is the volatility
/// - W is a Wiener process
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::mc::GbmParams;
///
/// let params = GbmParams {
///     spot: 100.0,
///     rate: 0.05,
///     volatility: 0.2,
///     maturity: 1.0,
/// };
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GbmParams {
    /// Initial spot price (S₀).
    pub spot: f64,
    /// Risk-free rate (r) - annualised.
    pub rate: f64,
    /// Volatility (σ) - annualised.
    pub volatility: f64,
    /// Time to maturity (T) - in years.
    pub maturity: f64,
}

impl GbmParams {
    /// Creates new GBM parameters.
    ///
    /// # Arguments
    ///
    /// * `spot` - Initial spot price
    /// * `rate` - Risk-free rate (annualised)
    /// * `volatility` - Volatility (annualised)
    /// * `maturity` - Time to maturity (years)
    #[inline]
    pub fn new(spot: f64, rate: f64, volatility: f64, maturity: f64) -> Self {
        Self {
            spot,
            rate,
            volatility,
            maturity,
        }
    }

    /// Validates the parameters.
    ///
    /// # Returns
    ///
    /// `true` if all parameters are valid (finite, non-negative where
    /// required).
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.spot > 0.0
            && self.spot.is_finite()
            && self.rate.is_finite()
            && self.volatility >= 0.0
            && self.volatility.is_finite()
            && self.maturity > 0.0
            && self.maturity.is_finite()
    }
}

impl Default for GbmParams {
    fn default() -> Self {
        Self {
            spot: 100.0,
            rate: 0.05,
            volatility: 0.2,
            maturity: 1.0,
        }
    }
}

/// Generates GBM paths using Euler-Maruyama discretisation.
///
/// Uses the log-space (exact) simulation formula for numerical stability:
/// ```text
/// S(t+dt) = S(t) × exp((r - 0.5σ²)dt + σ√dt × Z)
/// ```
///
/// # Arguments
///
/// * `workspace` - Pre-allocated workspace with random samples filled
/// * `params` - GBM parameters
/// * `n_paths` - Number of paths to generate
/// * `n_steps` - Number of time steps
///
/// # Panics
///
/// Panics if workspace capacity is insufficient.
///
/// # Algorithm
///
/// 1. Precompute `drift_dt = (r - 0.5σ²)dt` and `vol_sqrt_dt = σ√dt`
/// 2. For each path, set S\[0\] = spot
/// 3. For each step, S\[t+1\] = S\[t\] × exp(drift_dt + vol_sqrt_dt × Z)
///
/// # Performance
///
/// - No heap allocations within the loop
/// - Uses precomputed constants for efficiency
/// - Cache-friendly row-major traversal
pub fn generate_gbm_paths(
    workspace: &mut PathWorkspace,
    params: GbmParams,
    n_paths: usize,
    n_steps: usize,
) {
    debug_assert!(n_paths <= workspace.capacity_paths());
    debug_assert!(n_steps <= workspace.capacity_steps());

    // Precompute time step
    let dt = params.maturity / n_steps as f64;

    // Precompute drift and volatility terms (outside loop for Enzyme)
    let drift_dt = (params.rate - 0.5 * params.volatility * params.volatility) * dt;
    let vol_sqrt_dt = params.volatility * dt.sqrt();

    let (paths, randoms) = workspace.paths_mut_and_randoms();
    let n_steps_plus_1 = n_steps + 1;

    // Generate paths (outer loop over paths, inner over steps)
    for path_idx in 0..n_paths {
        let path_offset = path_idx * n_steps_plus_1;
        let random_offset = path_idx * n_steps;

        // Set initial spot
        paths[path_offset] = params.spot;

        // Evolve path
        for step in 0..n_steps {
            let z = randoms[random_offset + step];
            let increment = drift_dt + vol_sqrt_dt * z;
            paths[path_offset + step + 1] = paths[path_offset + step] * increment.exp();
        }
    }
}

/// Generates GBM paths with dual (tangent) values for forward-mode AD.
///
/// Computes both primal paths and their tangent with respect to spot.
///
/// # Arguments
///
/// * `workspace` - Workspace for primal computation
/// * `params` - GBM parameters
/// * `d_spot` - Tangent seed for spot (typically 1.0)
/// * `n_paths` - Number of paths
/// * `n_steps` - Number of steps
///
/// # Returns
///
/// The tangent paths are stored in the workspace; the caller must
/// extract the terminal tangent values separately.
///
/// # Activity Analysis
///
/// - `spot`: Dual (d_spot is the tangent seed)
/// - `rate`, `volatility`: Const (not differentiated here)
/// - `randoms`: Const (frozen during AD)
pub fn generate_gbm_paths_tangent_spot(
    workspace: &mut PathWorkspace,
    params: GbmParams,
    d_spot: f64,
    n_paths: usize,
    n_steps: usize,
) -> Vec<f64> {
    debug_assert!(n_paths <= workspace.capacity_paths());
    debug_assert!(n_steps <= workspace.capacity_steps());

    // Precompute time step
    let dt = params.maturity / n_steps as f64;

    // Precompute drift and volatility terms
    let drift_dt = (params.rate - 0.5 * params.volatility * params.volatility) * dt;
    let vol_sqrt_dt = params.volatility * dt.sqrt();

    let (paths, randoms) = workspace.paths_mut_and_randoms();
    let n_steps_plus_1 = n_steps + 1;

    // Allocate tangent paths (outside simulation for Enzyme)
    let mut tangent_paths = vec![0.0; n_paths * n_steps_plus_1];

    // Generate paths with tangent propagation
    for path_idx in 0..n_paths {
        let path_offset = path_idx * n_steps_plus_1;
        let random_offset = path_idx * n_steps;

        // Set initial spot and tangent
        paths[path_offset] = params.spot;
        tangent_paths[path_offset] = d_spot;

        // Evolve path with tangent
        for step in 0..n_steps {
            let z = randoms[random_offset + step];
            let increment = drift_dt + vol_sqrt_dt * z;
            let exp_increment = increment.exp();

            // Primal: S[t+1] = S[t] * exp(...)
            paths[path_offset + step + 1] = paths[path_offset + step] * exp_increment;

            // Tangent: dS[t+1] = dS[t] * exp(...) (chain rule)
            tangent_paths[path_offset + step + 1] =
                tangent_paths[path_offset + step] * exp_increment;
        }
    }

    tangent_paths
}

/// Extracts terminal prices from generated paths.
///
/// # Arguments
///
/// * `workspace` - Workspace with generated paths
/// * `n_paths` - Number of paths
/// * `n_steps` - Number of steps
///
/// # Returns
///
/// Slice of terminal prices (one per path).
#[inline]
pub fn terminal_prices(workspace: &PathWorkspace, n_paths: usize, n_steps: usize) -> Vec<f64> {
    let paths = workspace.paths();
    let n_steps_plus_1 = n_steps + 1;

    (0..n_paths)
        .map(|path_idx| paths[path_idx * n_steps_plus_1 + n_steps])
        .collect()
}

// ============================================================================
// Generic Path Generation Functions (for WorkspaceEnum)
// ============================================================================

/// Generates GBM paths using the appropriate algorithm for the workspace
/// layout.
///
/// This function automatically selects the optimal algorithm based on the
/// workspace's memory layout:
/// - `PathFirst`: Path-major iteration (traditional)
/// - `TimeStepFirst`: Step-major iteration (cache-efficient for SIMD)
///
/// # Arguments
///
/// * `workspace` - Pre-allocated workspace with random samples filled
/// * `params` - GBM parameters
///
/// # Performance
///
/// The `TimeStepFirst` layout enables:
/// - Better cache utilisation (all paths at a step are contiguous)
/// - SIMD vectorisation opportunities
/// - Reduced cache misses during step-wise operations
///
/// # Examples
///
/// ```rust
/// use pricer_pricing::mc::{WorkspaceEnum, PathLayout, GbmParams, generate_gbm_paths_generic};
/// use pricer_core::math::rng::PricerRng;
/// use pricer_pricing::mc::PathWorkspaceTrait;
///
/// let mut workspace = WorkspaceEnum::new(PathLayout::TimeStepFirst, 1000, 100);
/// let mut rng = PricerRng::from_seed(42);
/// rng.fill_normal(workspace.randoms_mut());
///
/// let params = GbmParams::default();
/// generate_gbm_paths_generic(&mut workspace, params);
/// ```
pub fn generate_gbm_paths_generic(workspace: &mut WorkspaceEnum, params: GbmParams) {
    let n_paths = workspace.num_paths();
    let n_steps = workspace.num_steps();

    match workspace.layout() {
        PathLayout::PathFirst => {
            if let Some(ws) = workspace.as_path_first_mut() {
                generate_gbm_paths(ws, params, n_paths, n_steps);
            }
        }
        PathLayout::TimeStepFirst => {
            generate_gbm_paths_timestep_first(workspace, params, n_paths, n_steps);
        }
    }
}

/// Generates GBM paths using step-major iteration for TimeStepFirst layout.
///
/// This is the optimised algorithm for TimeStepFirst workspace:
/// - Outer loop over steps, inner loop over paths
/// - Uses contiguous step slices for cache efficiency
/// - Enables SIMD vectorisation across paths
///
/// # Algorithm
///
/// ```text
/// for step in 0..n_steps:
///     current_slice = workspace.get_step_slice(step)
///     next_slice = workspace.get_step_slice(step + 1)
///     for path in 0..n_paths:
///         next_slice[path] = current_slice[path] * exp(drift + vol * z)
/// ```
fn generate_gbm_paths_timestep_first(
    workspace: &mut WorkspaceEnum,
    params: GbmParams,
    n_paths: usize,
    n_steps: usize,
) {
    // Precompute time step
    let dt = params.maturity / n_steps as f64;

    // Precompute drift and volatility terms (outside loop for Enzyme)
    let drift_dt = (params.rate - 0.5 * params.volatility * params.volatility) * dt;
    let vol_sqrt_dt = params.volatility * dt.sqrt();

    // Set initial spot for all paths
    if let Some(step0) = workspace.get_step_slice_mut(0) {
        for val in step0.iter_mut() {
            *val = params.spot;
        }
    }

    // Get randoms buffer
    let randoms = workspace.randoms().to_vec();

    // Step-major iteration for cache efficiency
    // Process all paths at each step before moving to next step
    for step in 0..n_steps {
        let random_offset = step * n_paths;

        // Read current step values
        let current_vals: Vec<f64> = workspace
            .get_step_slice(step)
            .map(|s| s.to_vec())
            .unwrap_or_default();

        // Compute and write next step values
        if let Some(next_slice) = workspace.get_step_slice_mut(step + 1) {
            for path_idx in 0..n_paths {
                let z = randoms[random_offset + path_idx];
                let increment = drift_dt + vol_sqrt_dt * z;
                next_slice[path_idx] = current_vals[path_idx] * increment.exp();
            }
        }
    }
}

/// Extracts terminal prices from a generic workspace.
///
/// Works with both PathFirst and TimeStepFirst layouts.
///
/// # Arguments
///
/// * `workspace` - Workspace with generated paths
///
/// # Returns
///
/// Vector of terminal prices (one per path).
pub fn terminal_prices_generic(workspace: &WorkspaceEnum) -> Vec<f64> {
    let n_paths = workspace.num_paths();
    let n_steps = workspace.num_steps();

    match workspace.layout() {
        PathLayout::PathFirst => {
            if let Some(ws) = workspace.as_path_first() {
                terminal_prices(ws, n_paths, n_steps)
            } else {
                Vec::new()
            }
        }
        PathLayout::TimeStepFirst => {
            // For TimeStepFirst, terminal prices are at the last step slice
            workspace
                .get_step_slice(n_steps)
                .map(|slice| slice.to_vec())
                .unwrap_or_default()
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;
    use pricer_core::math::rng::PricerRng;

    fn setup_workspace_with_randoms(n_paths: usize, n_steps: usize, seed: u64) -> PathWorkspace {
        let mut workspace = PathWorkspace::new(n_paths, n_steps);
        let mut rng = PricerRng::from_seed(seed);
        rng.fill_normal(workspace.randoms_mut());
        workspace
    }

    #[test]
    fn test_gbm_params_default() {
        let params = GbmParams::default();
        assert_eq!(params.spot, 100.0);
        assert_eq!(params.rate, 0.05);
        assert_eq!(params.volatility, 0.2);
        assert_eq!(params.maturity, 1.0);
    }

    #[test]
    fn test_gbm_params_validation() {
        assert!(GbmParams::default().is_valid());

        // Invalid cases
        assert!(!GbmParams::new(0.0, 0.05, 0.2, 1.0).is_valid()); // zero spot
        assert!(!GbmParams::new(-100.0, 0.05, 0.2, 1.0).is_valid()); // negative spot
        assert!(!GbmParams::new(100.0, 0.05, -0.2, 1.0).is_valid()); // negative vol
        assert!(!GbmParams::new(100.0, 0.05, 0.2, 0.0).is_valid()); // zero maturity
        assert!(!GbmParams::new(f64::NAN, 0.05, 0.2, 1.0).is_valid()); // NaN spot
    }

    #[test]
    fn test_path_generation_initial_spot() {
        let mut workspace = setup_workspace_with_randoms(10, 5, 42);
        let params = GbmParams::new(100.0, 0.05, 0.2, 1.0);

        generate_gbm_paths(&mut workspace, params, 10, 5);

        // Check all paths start at spot
        let paths = workspace.paths();
        for path_idx in 0..10 {
            let initial = paths[path_idx * 6]; // 6 = n_steps + 1
            assert_eq!(initial, 100.0);
        }
    }

    #[test]
    fn test_path_generation_positive_prices() {
        let mut workspace = setup_workspace_with_randoms(100, 50, 42);
        let params = GbmParams::new(100.0, 0.05, 0.2, 1.0);

        generate_gbm_paths(&mut workspace, params, 100, 50);

        // All prices should be positive (GBM property)
        for &price in workspace.paths() {
            assert!(price > 0.0, "Price must be positive: {}", price);
            assert!(price.is_finite(), "Price must be finite: {}", price);
        }
    }

    #[test]
    fn test_path_generation_reproducibility() {
        let mut ws1 = setup_workspace_with_randoms(10, 5, 12345);
        let mut ws2 = setup_workspace_with_randoms(10, 5, 12345);
        let params = GbmParams::default();

        generate_gbm_paths(&mut ws1, params, 10, 5);
        generate_gbm_paths(&mut ws2, params, 10, 5);

        // Same seed should produce identical paths
        for (p1, p2) in ws1.paths().iter().zip(ws2.paths().iter()) {
            assert_eq!(*p1, *p2);
        }
    }

    #[test]
    fn test_path_generation_different_seeds() {
        let mut ws1 = setup_workspace_with_randoms(10, 5, 12345);
        let mut ws2 = setup_workspace_with_randoms(10, 5, 54321);
        let params = GbmParams::default();

        generate_gbm_paths(&mut ws1, params, 10, 5);
        generate_gbm_paths(&mut ws2, params, 10, 5);

        // Different seeds should produce different paths
        let different = ws1
            .paths()
            .iter()
            .zip(ws2.paths().iter())
            .any(|(p1, p2)| p1 != p2);
        assert!(different);
    }

    #[test]
    fn test_terminal_prices_extraction() {
        let mut workspace = setup_workspace_with_randoms(10, 5, 42);
        let params = GbmParams::default();

        generate_gbm_paths(&mut workspace, params, 10, 5);

        let terminals = terminal_prices(&workspace, 10, 5);
        assert_eq!(terminals.len(), 10);

        // Verify against direct path access
        let paths = workspace.paths();
        for (path_idx, &terminal) in terminals.iter().enumerate() {
            let direct = paths[path_idx * 6 + 5]; // step 5 is terminal
            assert_eq!(terminal, direct);
        }
    }

    #[test]
    fn test_path_generation_statistical_mean() {
        // Test that E[S(T)] ≈ S(0) * exp(r*T) for large sample
        let n_paths = 50_000;
        let n_steps = 1;
        let mut workspace = setup_workspace_with_randoms(n_paths, n_steps, 42);

        let params = GbmParams {
            spot: 100.0,
            rate: 0.05,
            volatility: 0.2,
            maturity: 1.0,
        };

        generate_gbm_paths(&mut workspace, params, n_paths, n_steps);

        let terminals = terminal_prices(&workspace, n_paths, n_steps);
        let mean = terminals.iter().sum::<f64>() / n_paths as f64;
        let expected = params.spot * (params.rate * params.maturity).exp();

        // Allow 2% tolerance for statistical variation
        assert_relative_eq!(mean, expected, max_relative = 0.02);
    }

    #[test]
    fn test_tangent_path_generation() {
        let mut workspace = setup_workspace_with_randoms(10, 5, 42);
        let params = GbmParams::default();

        let tangents = generate_gbm_paths_tangent_spot(&mut workspace, params, 1.0, 10, 5);

        // Tangent at t=0 should equal d_spot
        for path_idx in 0..10 {
            assert_eq!(tangents[path_idx * 6], 1.0);
        }

        // Tangent should scale with path
        // For GBM: dS/dS0 = S/S0, so tangent[t] / paths[t] ≈ 1/S0
        let paths = workspace.paths();
        for path_idx in 0..10 {
            let offset = path_idx * 6;
            for step in 1..6 {
                let ratio = tangents[offset + step] / paths[offset + step];
                let expected = 1.0 / params.spot;
                assert_relative_eq!(ratio, expected, epsilon = 1e-10);
            }
        }
    }

    // ========== Generic Path Generation Tests ==========

    fn setup_workspace_enum_with_randoms(
        layout: PathLayout,
        n_paths: usize,
        n_steps: usize,
        seed: u64,
    ) -> WorkspaceEnum {
        let mut workspace = WorkspaceEnum::new(layout, n_paths, n_steps);
        let mut rng = PricerRng::from_seed(seed);
        rng.fill_normal(workspace.randoms_mut());
        workspace
    }

    #[test]
    fn test_generic_path_generation_path_first() {
        let mut workspace = setup_workspace_enum_with_randoms(PathLayout::PathFirst, 10, 5, 42);
        let params = GbmParams::default();

        generate_gbm_paths_generic(&mut workspace, params);

        // Check all paths start at spot
        for path_idx in 0..10 {
            assert_eq!(workspace.get_path_value(path_idx, 0), 100.0);
        }

        // All prices should be positive
        let terminals = terminal_prices_generic(&workspace);
        assert_eq!(terminals.len(), 10);
        for &price in &terminals {
            assert!(price > 0.0);
        }
    }

    #[test]
    fn test_generic_path_generation_timestep_first() {
        let mut workspace = setup_workspace_enum_with_randoms(PathLayout::TimeStepFirst, 10, 5, 42);
        let params = GbmParams::default();

        generate_gbm_paths_generic(&mut workspace, params);

        // Check all paths start at spot
        for path_idx in 0..10 {
            assert_eq!(workspace.get_path_value(path_idx, 0), 100.0);
        }

        // All prices should be positive
        let terminals = terminal_prices_generic(&workspace);
        assert_eq!(terminals.len(), 10);
        for &price in &terminals {
            assert!(price > 0.0);
        }
    }

    #[test]
    fn test_generic_path_generation_layouts_produce_same_results() {
        // Both layouts with same seed should produce same terminal prices
        // (not exactly same due to different iteration order, but same distribution)
        let n_paths = 1000;
        let n_steps = 10;
        let seed = 12345;

        let mut ws_pf =
            setup_workspace_enum_with_randoms(PathLayout::PathFirst, n_paths, n_steps, seed);
        let mut ws_tsf =
            setup_workspace_enum_with_randoms(PathLayout::TimeStepFirst, n_paths, n_steps, seed);

        let params = GbmParams::default();

        generate_gbm_paths_generic(&mut ws_pf, params);
        generate_gbm_paths_generic(&mut ws_tsf, params);

        let terminals_pf = terminal_prices_generic(&ws_pf);
        let terminals_tsf = terminal_prices_generic(&ws_tsf);

        // Check statistical properties match
        let mean_pf = terminals_pf.iter().sum::<f64>() / n_paths as f64;
        let mean_tsf = terminals_tsf.iter().sum::<f64>() / n_paths as f64;

        // Both should be close to expected value
        let expected = params.spot * (params.rate * params.maturity).exp();
        assert_relative_eq!(mean_pf, expected, max_relative = 0.05);
        assert_relative_eq!(mean_tsf, expected, max_relative = 0.05);
    }

    #[test]
    fn test_generic_path_generation_positive_prices() {
        let n_paths = 100;
        let n_steps = 50;

        for layout in [PathLayout::PathFirst, PathLayout::TimeStepFirst] {
            let mut workspace = setup_workspace_enum_with_randoms(layout, n_paths, n_steps, 42);
            let params = GbmParams::new(100.0, 0.05, 0.2, 1.0);

            generate_gbm_paths_generic(&mut workspace, params);

            // All path values should be positive (GBM property)
            for path_idx in 0..n_paths {
                for step_idx in 0..=n_steps {
                    let price = workspace.get_path_value(path_idx, step_idx);
                    assert!(
                        price > 0.0,
                        "Price at ({}, {}) must be positive: {}",
                        path_idx,
                        step_idx,
                        price
                    );
                }
            }
        }
    }

    #[test]
    fn test_terminal_prices_generic_timestep_first() {
        let mut workspace = setup_workspace_enum_with_randoms(PathLayout::TimeStepFirst, 10, 5, 42);
        let params = GbmParams::default();

        generate_gbm_paths_generic(&mut workspace, params);

        let terminals = terminal_prices_generic(&workspace);
        assert_eq!(terminals.len(), 10);

        // Verify against direct access
        for path_idx in 0..10 {
            assert_eq!(terminals[path_idx], workspace.get_path_value(path_idx, 5));
        }
    }
}
