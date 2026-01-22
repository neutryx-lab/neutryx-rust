//! Configuration types for optimisation algorithms.

/// Configuration for optimisation algorithms.
///
/// Provides common settings shared across optimisation methods.
///
/// # Example
///
/// ```ignore
/// use pricer_core::math::optimisers::OptimisationConfig;
///
/// let config = OptimisationConfig::default();
/// let config = OptimisationConfig::new(1e-8, 1e-8, 500);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptimisationConfig {
    /// Absolute tolerance for convergence.
    pub abs_tol: f64,
    /// Relative tolerance for convergence.
    pub rel_tol: f64,
    /// Maximum number of iterations.
    pub max_iterations: usize,
    /// Whether to print progress information.
    pub verbose: bool,
}

impl Default for OptimisationConfig {
    fn default() -> Self {
        Self {
            abs_tol: 1e-10,
            rel_tol: 1e-10,
            max_iterations: 1000,
            verbose: false,
        }
    }
}

impl OptimisationConfig {
    /// Create a new configuration with specified tolerances.
    ///
    /// # Arguments
    ///
    /// * `abs_tol` - Absolute tolerance for convergence
    /// * `rel_tol` - Relative tolerance for convergence
    /// * `max_iterations` - Maximum number of iterations
    #[must_use]
    pub fn new(abs_tol: f64, rel_tol: f64, max_iterations: usize) -> Self {
        Self {
            abs_tol,
            rel_tol,
            max_iterations,
            verbose: false,
        }
    }

    /// Create a configuration with high precision settings.
    #[must_use]
    pub fn high_precision() -> Self {
        Self {
            abs_tol: 1e-14,
            rel_tol: 1e-14,
            max_iterations: 5000,
            verbose: false,
        }
    }

    /// Create a configuration optimised for fast convergence.
    #[must_use]
    pub fn fast() -> Self {
        Self {
            abs_tol: 1e-6,
            rel_tol: 1e-6,
            max_iterations: 200,
            verbose: false,
        }
    }

    /// Set verbose mode.
    #[must_use]
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set maximum iterations.
    #[must_use]
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }
}

/// Configuration specific to L-BFGS algorithm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LbfgsConfig {
    /// Base optimisation configuration.
    pub base: OptimisationConfig,
    /// Number of corrections to store (history size).
    pub m: usize,
    /// Line search parameters - backtracking parameter.
    pub c1: f64,
    /// Line search parameters - curvature condition.
    pub c2: f64,
}

impl Default for LbfgsConfig {
    fn default() -> Self {
        Self {
            base: OptimisationConfig::default(),
            m: 10, // Standard choice
            c1: 1e-4,
            c2: 0.9,
        }
    }
}

impl LbfgsConfig {
    /// Create a new L-BFGS configuration.
    ///
    /// # Arguments
    ///
    /// * `base` - Base optimisation configuration
    /// * `m` - Number of corrections to store (history size)
    #[must_use]
    pub fn new(base: OptimisationConfig, m: usize) -> Self {
        Self {
            base,
            m,
            ..Default::default()
        }
    }
}

/// Configuration specific to Nelder-Mead algorithm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NelderMeadConfig {
    /// Base optimisation configuration.
    pub base: OptimisationConfig,
    /// Reflection coefficient (typically 1.0).
    pub alpha: f64,
    /// Expansion coefficient (typically 2.0).
    pub gamma: f64,
    /// Contraction coefficient (typically 0.5).
    pub rho: f64,
    /// Shrink coefficient (typically 0.5).
    pub sigma: f64,
    /// Initial simplex scale.
    pub initial_scale: f64,
}

impl Default for NelderMeadConfig {
    fn default() -> Self {
        Self {
            base: OptimisationConfig::default(),
            alpha: 1.0,
            gamma: 2.0,
            rho: 0.5,
            sigma: 0.5,
            initial_scale: 1.0,
        }
    }
}

impl NelderMeadConfig {
    /// Create a new Nelder-Mead configuration.
    #[must_use]
    pub fn new(base: OptimisationConfig) -> Self {
        Self {
            base,
            ..Default::default()
        }
    }

    /// Set the initial simplex scale.
    #[must_use]
    pub fn with_initial_scale(mut self, scale: f64) -> Self {
        self.initial_scale = scale;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OptimisationConfig::default();
        assert!((config.abs_tol - 1e-10).abs() < 1e-15);
        assert!((config.rel_tol - 1e-10).abs() < 1e-15);
        assert_eq!(config.max_iterations, 1000);
        assert!(!config.verbose);
    }

    #[test]
    fn test_new_config() {
        let config = OptimisationConfig::new(1e-8, 1e-6, 500);
        assert!((config.abs_tol - 1e-8).abs() < 1e-15);
        assert!((config.rel_tol - 1e-6).abs() < 1e-15);
        assert_eq!(config.max_iterations, 500);
    }

    #[test]
    fn test_high_precision_config() {
        let config = OptimisationConfig::high_precision();
        assert!(config.abs_tol < 1e-12);
        assert!(config.max_iterations >= 5000);
    }

    #[test]
    fn test_fast_config() {
        let config = OptimisationConfig::fast();
        assert!(config.abs_tol > 1e-8);
        assert!(config.max_iterations <= 200);
    }

    #[test]
    fn test_with_verbose() {
        let config = OptimisationConfig::default().with_verbose(true);
        assert!(config.verbose);
    }

    #[test]
    fn test_with_max_iterations() {
        let config = OptimisationConfig::default().with_max_iterations(2000);
        assert_eq!(config.max_iterations, 2000);
    }

    #[test]
    fn test_lbfgs_config_default() {
        let config = LbfgsConfig::default();
        assert_eq!(config.m, 10);
        assert!((config.c1 - 1e-4).abs() < 1e-15);
        assert!((config.c2 - 0.9).abs() < 1e-15);
    }

    #[test]
    fn test_nelder_mead_config_default() {
        let config = NelderMeadConfig::default();
        assert!((config.alpha - 1.0).abs() < 1e-15);
        assert!((config.gamma - 2.0).abs() < 1e-15);
        assert!((config.rho - 0.5).abs() < 1e-15);
        assert!((config.sigma - 0.5).abs() < 1e-15);
    }

    #[test]
    fn test_nelder_mead_with_initial_scale() {
        let config = NelderMeadConfig::default().with_initial_scale(0.1);
        assert!((config.initial_scale - 0.1).abs() < 1e-15);
    }
}
