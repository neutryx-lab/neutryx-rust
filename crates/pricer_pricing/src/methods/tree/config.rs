//! Tree pricing configuration.

use crate::generic_pricer::ConfigError;

/// Tree type selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TreeType {
    /// Cox-Ross-Rubinstein binomial tree.
    #[default]
    Binomial,
    /// Trinomial tree.
    Trinomial,
}

/// Configuration for tree-based pricing.
#[derive(Debug, Clone)]
pub struct TreeConfig {
    /// Number of time steps in the tree.
    pub num_steps: usize,
    /// Tree type (Binomial or Trinomial).
    pub tree_type: TreeType,
    /// Convergence tolerance for verification.
    pub convergence_tolerance: f64,
    /// Whether to compute Greeks.
    pub compute_greeks: bool,
}

impl Default for TreeConfig {
    fn default() -> Self {
        Self {
            num_steps: 100,
            tree_type: TreeType::Binomial,
            convergence_tolerance: 1e-6,
            compute_greeks: true,
        }
    }
}

impl TreeConfig {
    /// Creates a new TreeConfig with specified number of steps.
    pub fn new(num_steps: usize) -> Self {
        Self {
            num_steps,
            ..Default::default()
        }
    }

    /// Returns a builder for TreeConfig.
    pub fn builder() -> TreeConfigBuilder { TreeConfigBuilder::default() }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.num_steps == 0 {
            return Err(ConfigError::InvalidModelParameter {
                name: "num_steps",
                reason: "num_steps must be greater than 0".to_string(),
            });
        }
        if self.convergence_tolerance <= 0.0 {
            return Err(ConfigError::InvalidModelParameter {
                name: "convergence_tolerance",
                reason: "convergence_tolerance must be positive".to_string(),
            });
        }
        Ok(())
    }
}

/// Builder for TreeConfig.
#[derive(Debug, Clone, Default)]
pub struct TreeConfigBuilder {
    num_steps: Option<usize>,
    tree_type: Option<TreeType>,
    convergence_tolerance: Option<f64>,
    compute_greeks: Option<bool>,
}

impl TreeConfigBuilder {
    /// Sets the number of time steps.
    pub fn num_steps(mut self, steps: usize) -> Self {
        self.num_steps = Some(steps);
        self
    }

    /// Sets the tree type.
    pub fn tree_type(mut self, tree_type: TreeType) -> Self {
        self.tree_type = Some(tree_type);
        self
    }

    /// Sets the convergence tolerance.
    pub fn convergence_tolerance(mut self, tolerance: f64) -> Self {
        self.convergence_tolerance = Some(tolerance);
        self
    }

    /// Sets whether to compute Greeks.
    pub fn compute_greeks(mut self, compute: bool) -> Self {
        self.compute_greeks = Some(compute);
        self
    }

    /// Builds the TreeConfig.
    pub fn build(self) -> Result<TreeConfig, ConfigError> {
        let config = TreeConfig {
            num_steps: self.num_steps.unwrap_or(100),
            tree_type: self.tree_type.unwrap_or_default(),
            convergence_tolerance: self.convergence_tolerance.unwrap_or(1e-6),
            compute_greeks: self.compute_greeks.unwrap_or(true),
        };
        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_config_default() {
        let config = TreeConfig::default();
        assert_eq!(config.num_steps, 100);
        assert_eq!(config.tree_type, TreeType::Binomial);
        assert!((config.convergence_tolerance - 1e-6).abs() < 1e-15);
        assert!(config.compute_greeks);
    }

    #[test]
    fn test_tree_config_new() {
        let config = TreeConfig::new(200);
        assert_eq!(config.num_steps, 200);
    }

    #[test]
    fn test_tree_config_validate_success() {
        let config = TreeConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_tree_config_validate_zero_steps() {
        let config = TreeConfig {
            num_steps: 0,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("num_steps"));
    }

    #[test]
    fn test_tree_config_validate_negative_tolerance() {
        let config = TreeConfig {
            convergence_tolerance: -1e-6,
            ..Default::default()
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("convergence_tolerance"));
    }

    #[test]
    fn test_tree_config_builder() {
        let config = TreeConfig::builder()
            .num_steps(500)
            .tree_type(TreeType::Trinomial)
            .convergence_tolerance(1e-8)
            .compute_greeks(false)
            .build()
            .unwrap();

        assert_eq!(config.num_steps, 500);
        assert_eq!(config.tree_type, TreeType::Trinomial);
        assert!((config.convergence_tolerance - 1e-8).abs() < 1e-15);
        assert!(!config.compute_greeks);
    }

    #[test]
    fn test_tree_config_builder_defaults() {
        let config = TreeConfig::builder().build().unwrap();
        assert_eq!(config.num_steps, 100);
    }

    #[test]
    fn test_tree_config_builder_validation() {
        let result = TreeConfig::builder().num_steps(0).build();
        assert!(result.is_err());
    }

    #[test]
    fn test_tree_type_default() {
        assert_eq!(TreeType::default(), TreeType::Binomial);
    }
}
