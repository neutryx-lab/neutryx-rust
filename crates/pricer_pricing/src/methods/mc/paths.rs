//! Path generation for Monte Carlo simulation.

use super::{
    layout_config::PathLayout, workspace::PathWorkspace, workspace_enum::WorkspaceEnum,
    workspace_trait::PathWorkspaceTrait,
};

/// Parameters for Geometric Brownian Motion path generation.
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
pub fn generate_gbm_paths(
    workspace: &mut PathWorkspace,
    params: GbmParams,
    n_paths: usize,
    n_steps: usize,
) {
    debug_assert!(n_paths <= workspace.capacity_paths());
    debug_assert!(n_steps <= workspace.capacity_steps());

    let dt = params.maturity / n_steps as f64;

    let drift_dt = (params.rate - 0.5 * params.volatility * params.volatility) * dt;
    let vol_sqrt_dt = params.volatility * dt.sqrt();

    let (paths, randoms) = workspace.paths_mut_and_randoms();
    let n_steps_plus_1 = n_steps + 1;

    for path_idx in 0..n_paths {
        let path_offset = path_idx * n_steps_plus_1;
        let random_offset = path_idx * n_steps;

        paths[path_offset] = params.spot;

        for step in 0..n_steps {
            let z = randoms[random_offset + step];
            let increment = drift_dt + vol_sqrt_dt * z;
            paths[path_offset + step + 1] = paths[path_offset + step] * increment.exp();
        }
    }
}

/// Generates GBM paths with dual (tangent) values for forward-mode AD.
pub fn generate_gbm_paths_tangent_spot(
    workspace: &mut PathWorkspace,
    params: GbmParams,
    d_spot: f64,
    n_paths: usize,
    n_steps: usize,
) -> Vec<f64> {
    debug_assert!(n_paths <= workspace.capacity_paths());
    debug_assert!(n_steps <= workspace.capacity_steps());

    let dt = params.maturity / n_steps as f64;

    let drift_dt = (params.rate - 0.5 * params.volatility * params.volatility) * dt;
    let vol_sqrt_dt = params.volatility * dt.sqrt();

    let (paths, randoms) = workspace.paths_mut_and_randoms();
    let n_steps_plus_1 = n_steps + 1;

    let mut tangent_paths = vec![0.0; n_paths * n_steps_plus_1];

    for path_idx in 0..n_paths {
        let path_offset = path_idx * n_steps_plus_1;
        let random_offset = path_idx * n_steps;

        paths[path_offset] = params.spot;
        tangent_paths[path_offset] = d_spot;

        for step in 0..n_steps {
            let z = randoms[random_offset + step];
            let increment = drift_dt + vol_sqrt_dt * z;
            let exp_increment = increment.exp();

            paths[path_offset + step + 1] = paths[path_offset + step] * exp_increment;

            tangent_paths[path_offset + step + 1] =
                tangent_paths[path_offset + step] * exp_increment;
        }
    }

    tangent_paths
}

/// Extracts terminal prices from generated paths.
#[inline]
pub fn terminal_prices(workspace: &PathWorkspace, n_paths: usize, n_steps: usize) -> Vec<f64> {
    let paths = workspace.paths();
    let n_steps_plus_1 = n_steps + 1;

    (0..n_paths)
        .map(|path_idx| paths[path_idx * n_steps_plus_1 + n_steps])
        .collect()
}

/// Generates GBM paths using the appropriate algorithm for the workspace
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
fn generate_gbm_paths_timestep_first(
    workspace: &mut WorkspaceEnum,
    params: GbmParams,
    n_paths: usize,
    n_steps: usize,
) {
    let dt = params.maturity / n_steps as f64;

    let drift_dt = (params.rate - 0.5 * params.volatility * params.volatility) * dt;
    let vol_sqrt_dt = params.volatility * dt.sqrt();

    if let Some(step0) = workspace.get_step_slice_mut(0) {
        for val in step0.iter_mut() {
            *val = params.spot;
        }
    }

    let randoms = workspace.randoms().to_vec();

    for step in 0..n_steps {
        let random_offset = step * n_paths;

        let current_vals: Vec<f64> = workspace
            .get_step_slice(step)
            .map(|s| s.to_vec())
            .unwrap_or_default();

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
        PathLayout::TimeStepFirst => workspace
            .get_step_slice(n_steps)
            .map(|slice| slice.to_vec())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use pricer_core::math::rng::PricerRng;

    use super::*;

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

        assert!(!GbmParams::new(0.0, 0.05, 0.2, 1.0).is_valid());
        assert!(!GbmParams::new(-100.0, 0.05, 0.2, 1.0).is_valid());
        assert!(!GbmParams::new(100.0, 0.05, -0.2, 1.0).is_valid());
        assert!(!GbmParams::new(100.0, 0.05, 0.2, 0.0).is_valid());
        assert!(!GbmParams::new(f64::NAN, 0.05, 0.2, 1.0).is_valid());
    }

    #[test]
    fn test_path_generation_initial_spot() {
        let mut workspace = setup_workspace_with_randoms(10, 5, 42);
        let params = GbmParams::new(100.0, 0.05, 0.2, 1.0);

        generate_gbm_paths(&mut workspace, params, 10, 5);

        let paths = workspace.paths();
        for path_idx in 0..10 {
            let initial = paths[path_idx * 6];
            assert_eq!(initial, 100.0);
        }
    }

    #[test]
    fn test_path_generation_positive_prices() {
        let mut workspace = setup_workspace_with_randoms(100, 50, 42);
        let params = GbmParams::new(100.0, 0.05, 0.2, 1.0);

        generate_gbm_paths(&mut workspace, params, 100, 50);

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

        let paths = workspace.paths();
        for (path_idx, &terminal) in terminals.iter().enumerate() {
            let direct = paths[path_idx * 6 + 5];
            assert_eq!(terminal, direct);
        }
    }

    #[test]
    fn test_path_generation_statistical_mean() {
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

        assert_relative_eq!(mean, expected, max_relative = 0.02);
    }

    #[test]
    fn test_tangent_path_generation() {
        let mut workspace = setup_workspace_with_randoms(10, 5, 42);
        let params = GbmParams::default();

        let tangents = generate_gbm_paths_tangent_spot(&mut workspace, params, 1.0, 10, 5);

        for path_idx in 0..10 {
            assert_eq!(tangents[path_idx * 6], 1.0);
        }

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

        for path_idx in 0..10 {
            assert_eq!(workspace.get_path_value(path_idx, 0), 100.0);
        }

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

        for path_idx in 0..10 {
            assert_eq!(workspace.get_path_value(path_idx, 0), 100.0);
        }

        let terminals = terminal_prices_generic(&workspace);
        assert_eq!(terminals.len(), 10);
        for &price in &terminals {
            assert!(price > 0.0);
        }
    }

    #[test]
    fn test_generic_path_generation_layouts_produce_same_results() {
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

        let mean_pf = terminals_pf.iter().sum::<f64>() / n_paths as f64;
        let mean_tsf = terminals_tsf.iter().sum::<f64>() / n_paths as f64;

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

        for path_idx in 0..10 {
            assert_eq!(terminals[path_idx], workspace.get_path_value(path_idx, 5));
        }
    }
}
