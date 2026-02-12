use num_traits::Float;
use pricer_core::math::numeric::from_f64;

use crate::{
    builder::{jump::JumpConfig, problem::JacobianMethod, CalibrationProblemConfig},
    market::curves::BootstrapInterpolation,
};

/// Configuration for global bootstrapping.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalBootstrapConfig<T: Float> {
    /// Convergence tolerance for residual norm (||F(x)||).
    pub tolerance: T,

    /// Convergence tolerance for parameter change (||Δx||).
    pub param_tolerance: T,

    /// Maximum number of Newton iterations.
    pub max_iterations: usize,

    /// Step size for numerical Jacobian approximation.
    pub jacobian_epsilon: T,

    /// Whether to store the Jacobian inverse for AAD.
    pub store_jacobian_inverse: bool,

    /// Interpolation method for the output curve.
    pub interpolation: BootstrapInterpolation,

    /// Whether to allow extrapolation in the output curve.
    pub allow_extrapolation: bool,

    /// Jacobian calculation method.
    pub jacobian_method: JacobianMethod,

    /// Enable telescoping for OIS/SOFR instruments.
    pub enable_telescoping: bool,

    /// Damping factor for Levenberg-Marquardt style regularisation.
    pub damping_factor: Option<T>,

    /// Enable debug logging of iteration progress.
    pub debug_logging: bool,

    /// Maximum allowed condition number for Jacobian matrix.
    pub max_condition_number: T,

    /// Jump configuration for CB meeting dates.
    ///
    /// When set, the bootstrapper will calibrate additional jump parameters
    /// at the specified central bank meeting dates.
    pub jump_config: Option<JumpConfig<T>>,

    /// Variance threshold for AD instability detection.
    ///
    /// When comparing AD Jacobian with finite difference approximation,
    /// if the variance exceeds this threshold, the system falls back to
    /// central difference method. Default: 1e6.
    #[cfg(feature = "enzyme-ad")]
    pub ad_variance_threshold: T,

    /// Checkpointing interval for AD gradient computation.
    ///
    /// Specifies how often to checkpoint during reverse-mode AD.
    /// Lower values use more memory but reduce re-computation.
    /// Default: None (no checkpointing).
    #[cfg(feature = "enzyme-ad")]
    pub ad_checkpoint_interval: Option<usize>,
}

impl<T: Float> Default for GlobalBootstrapConfig<T> {
    fn default() -> Self {
        Self {
            tolerance: from_f64(1e-10),
            param_tolerance: from_f64(1e-10),
            max_iterations: 100,
            jacobian_epsilon: from_f64(1e-8),
            store_jacobian_inverse: true,
            interpolation: BootstrapInterpolation::LogLinear,
            allow_extrapolation: true,
            jacobian_method: JacobianMethod::default(),
            enable_telescoping: true,
            damping_factor: None,
            debug_logging: false,
            max_condition_number: from_f64(1e12),
            jump_config: None,
            #[cfg(feature = "enzyme-ad")]
            ad_variance_threshold: from_f64(1e6),
            #[cfg(feature = "enzyme-ad")]
            ad_checkpoint_interval: None,
        }
    }
}

impl<T: Float> GlobalBootstrapConfig<T> {
    /// Create a new configuration with specified tolerances.
    pub fn new(tolerance: T, max_iterations: usize) -> Self {
        Self {
            tolerance,
            param_tolerance: tolerance,
            max_iterations,
            ..Self::default()
        }
    }

    /// Create a high-precision configuration.
    pub fn high_precision() -> Self {
        Self {
            tolerance: from_f64(1e-14),
            param_tolerance: from_f64(1e-14),
            max_iterations: 500,
            jacobian_epsilon: from_f64(1e-10),
            jacobian_method: JacobianMethod::CentralDifference,
            max_condition_number: from_f64(1e14),
            #[cfg(feature = "enzyme-ad")]
            ad_variance_threshold: from_f64(1e8),
            ..Default::default()
        }
    }

    /// Create a fast configuration with relaxed tolerances.
    pub fn fast() -> Self {
        Self {
            tolerance: from_f64(1e-6),
            param_tolerance: from_f64(1e-6),
            max_iterations: 50,
            jacobian_epsilon: from_f64(1e-6),
            store_jacobian_inverse: false,
            jacobian_method: JacobianMethod::FiniteDifference,
            max_condition_number: from_f64(1e10),
            ..Default::default()
        }
    }

    /// Check if jump calibration is configured and active.
    pub fn has_jumps(&self) -> bool { self.jump_config.as_ref().is_some_and(|jc| jc.is_active()) }

    /// Get the number of configured jump pillars.
    pub fn num_jumps(&self) -> usize { self.jump_config.as_ref().map_or(0, |jc| jc.num_jumps()) }
}

// Conversion to CalibrationProblemConfig
impl<T: Float> From<&GlobalBootstrapConfig<T>> for CalibrationProblemConfig<T> {
    fn from(config: &GlobalBootstrapConfig<T>) -> Self {
        Self {
            jacobian_method: config.jacobian_method,
            jacobian_epsilon: config.jacobian_epsilon,
            interpolation: config.interpolation,
            allow_extrapolation: config.allow_extrapolation,
        }
    }
}
