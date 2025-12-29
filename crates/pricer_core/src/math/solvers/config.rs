//! Solver configuration types.

use num_traits::Float;

/// Configuration for root-finding algorithms.
///
/// Provides common settings shared across all solver implementations,
/// including convergence tolerance and iteration limits.
///
/// # Type Parameters
///
/// * `T` - Floating-point type for tolerance (e.g., `f64`)
///
/// # Example
///
/// ```
/// use pricer_core::math::solvers::SolverConfig;
///
/// // Use default configuration
/// let config: SolverConfig<f64> = SolverConfig::default();
/// assert!(config.tolerance < 1e-8);
/// assert!(config.max_iterations >= 50);
///
/// // Custom configuration
/// let custom = SolverConfig {
///     tolerance: 1e-12,
///     max_iterations: 200,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolverConfig<T: Float> {
    /// Convergence tolerance for root finding.
    ///
    /// The solver stops when `|f(x)| < tolerance`.
    /// Smaller values provide more precision but may require more iterations.
    pub tolerance: T,

    /// Maximum number of iterations before giving up.
    ///
    /// If the solver doesn't converge within this limit,
    /// it returns `SolverError::MaxIterationsExceeded`.
    pub max_iterations: usize,
}

impl<T: Float> Default for SolverConfig<T> {
    /// Create a default configuration with sensible values.
    ///
    /// Default values:
    /// - `tolerance`: 1e-10
    /// - `max_iterations`: 100
    fn default() -> Self {
        Self {
            tolerance: T::from(1e-10).unwrap(),
            max_iterations: 100,
        }
    }
}

impl<T: Float> SolverConfig<T> {
    /// Create a new configuration with specified values.
    ///
    /// # Arguments
    ///
    /// * `tolerance` - Convergence tolerance (must be positive)
    /// * `max_iterations` - Maximum iteration count (must be > 0)
    ///
    /// # Panics
    ///
    /// Panics if `tolerance <= 0` or `max_iterations == 0`.
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::math::solvers::SolverConfig;
    ///
    /// let config = SolverConfig::new(1e-12, 200);
    /// assert_eq!(config.max_iterations, 200);
    /// ```
    pub fn new(tolerance: T, max_iterations: usize) -> Self {
        assert!(tolerance > T::zero(), "tolerance must be positive");
        assert!(max_iterations > 0, "max_iterations must be > 0");
        Self {
            tolerance,
            max_iterations,
        }
    }

    /// Create a configuration with high precision settings.
    ///
    /// Uses tighter tolerance (1e-14) and more iterations (500)
    /// for cases requiring extreme precision.
    pub fn high_precision() -> Self {
        Self {
            tolerance: T::from(1e-14).unwrap(),
            max_iterations: 500,
        }
    }

    /// Create a configuration optimised for fast convergence.
    ///
    /// Uses relaxed tolerance (1e-6) and fewer iterations (50)
    /// for cases where speed is more important than precision.
    pub fn fast() -> Self {
        Self {
            tolerance: T::from(1e-6).unwrap(),
            max_iterations: 50,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config: SolverConfig<f64> = SolverConfig::default();
        assert!((config.tolerance - 1e-10).abs() < 1e-15);
        assert_eq!(config.max_iterations, 100);
    }

    #[test]
    fn test_new_config() {
        let config: SolverConfig<f64> = SolverConfig::new(1e-12, 200);
        assert!((config.tolerance - 1e-12).abs() < 1e-17);
        assert_eq!(config.max_iterations, 200);
    }

    #[test]
    #[should_panic(expected = "tolerance must be positive")]
    fn test_new_config_zero_tolerance_panics() {
        let _: SolverConfig<f64> = SolverConfig::new(0.0, 100);
    }

    #[test]
    #[should_panic(expected = "tolerance must be positive")]
    fn test_new_config_negative_tolerance_panics() {
        let _: SolverConfig<f64> = SolverConfig::new(-1e-10, 100);
    }

    #[test]
    #[should_panic(expected = "max_iterations must be > 0")]
    fn test_new_config_zero_iterations_panics() {
        let _: SolverConfig<f64> = SolverConfig::new(1e-10, 0);
    }

    #[test]
    fn test_high_precision_config() {
        let config: SolverConfig<f64> = SolverConfig::high_precision();
        assert!(config.tolerance < 1e-12);
        assert!(config.max_iterations >= 500);
    }

    #[test]
    fn test_fast_config() {
        let config: SolverConfig<f64> = SolverConfig::fast();
        assert!(config.tolerance > 1e-8);
        assert!(config.max_iterations <= 50);
    }

    #[test]
    fn test_config_clone() {
        let config1: SolverConfig<f64> = SolverConfig::new(1e-8, 150);
        let config2 = config1.clone();
        assert_eq!(config1, config2);
    }

    #[test]
    fn test_config_copy() {
        let config1: SolverConfig<f64> = SolverConfig::default();
        let config2 = config1; // Copy semantics
        assert_eq!(config1, config2);
    }

    #[test]
    fn test_config_debug() {
        let config: SolverConfig<f64> = SolverConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("SolverConfig"));
        assert!(debug_str.contains("tolerance"));
        assert!(debug_str.contains("max_iterations"));
    }

    #[test]
    fn test_config_with_f32() {
        let config: SolverConfig<f32> = SolverConfig::default();
        assert!(config.tolerance > 0.0);
        assert_eq!(config.max_iterations, 100);
    }
}
