use num_traits::Float;
use pricer_core::math::numeric::from_f64;

use crate::{
    builder::{
        jump::{JumpConfig, JumpPillar},
        problem::JacobianMethod,
        CalibrationProblemConfig,
    },
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
            store_jacobian_inverse: true,
            interpolation: BootstrapInterpolation::LogLinear,
            allow_extrapolation: true,
            jacobian_method: JacobianMethod::CentralDifference,
            enable_telescoping: true,
            damping_factor: None,
            debug_logging: false,
            max_condition_number: from_f64(1e14),
            jump_config: None,
            #[cfg(feature = "enzyme-ad")]
            ad_variance_threshold: from_f64(1e8), // Higher threshold for high-precision
            #[cfg(feature = "enzyme-ad")]
            ad_checkpoint_interval: None,
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
            interpolation: BootstrapInterpolation::LogLinear,
            allow_extrapolation: true,
            jacobian_method: JacobianMethod::FiniteDifference,
            enable_telescoping: true,
            damping_factor: None,
            debug_logging: false,
            max_condition_number: from_f64(1e10),
            jump_config: None,
            #[cfg(feature = "enzyme-ad")]
            ad_variance_threshold: from_f64(1e6),
            #[cfg(feature = "enzyme-ad")]
            ad_checkpoint_interval: None,
        }
    }

    /// Set the interpolation method.
    pub fn with_interpolation(mut self, method: BootstrapInterpolation) -> Self {
        self.interpolation = method;
        self
    }

    /// Enable or disable Jacobian inverse storage.
    pub fn with_jacobian_inverse(mut self, store: bool) -> Self {
        self.store_jacobian_inverse = store;
        self
    }

    /// Set the Jacobian calculation method.
    pub fn with_jacobian_method(mut self, method: JacobianMethod) -> Self {
        self.jacobian_method = method;
        self
    }

    /// Enable or disable telescoping for OIS/SOFR instruments.
    pub fn with_telescoping(mut self, enable: bool) -> Self {
        self.enable_telescoping = enable;
        self
    }

    /// Set the damping factor for Levenberg-Marquardt regularisation.
    pub fn with_damping(mut self, factor: T) -> Self {
        self.damping_factor = Some(factor);
        self
    }

    /// Enable or disable debug logging.
    pub fn with_debug_logging(mut self, enable: bool) -> Self {
        self.debug_logging = enable;
        self
    }

    /// Set the maximum allowed condition number.
    pub fn with_max_condition_number(mut self, max_cond: T) -> Self {
        self.max_condition_number = max_cond;
        self
    }

    /// Set the tolerance.
    pub fn with_tolerance(mut self, tol: T) -> Self {
        self.tolerance = tol;
        self
    }

    /// Set the maximum iterations.
    pub fn with_max_iterations(mut self, max_iter: usize) -> Self {
        self.max_iterations = max_iter;
        self
    }

    /// Set the jump configuration for CB meeting dates.
    pub fn with_jump_config(mut self, config: JumpConfig<T>) -> Self {
        self.jump_config = Some(config);
        self
    }

    /// Set jump pillars directly (convenience method).
    pub fn with_jumps(mut self, pillars: Vec<JumpPillar<T>>) -> Self {
        self.jump_config = Some(JumpConfig::with_pillars(pillars));
        self
    }

    /// Check if jump calibration is configured and active.
    pub fn has_jumps(&self) -> bool { self.jump_config.as_ref().is_some_and(|jc| jc.is_active()) }

    /// Get the number of configured jump pillars.
    pub fn num_jumps(&self) -> usize { self.jump_config.as_ref().map_or(0, |jc| jc.num_jumps()) }

    /// Enable Automatic Differentiation for Jacobian computation.
    ///
    /// Only available when the `enzyme-ad` feature is enabled.
    #[cfg(feature = "enzyme-ad")]
    pub fn with_automatic_differentiation(mut self) -> Self {
        self.jacobian_method = JacobianMethod::AutomaticDifferentiation;
        self
    }

    /// Set the AD variance threshold for instability detection.
    #[cfg(feature = "enzyme-ad")]
    pub fn with_ad_variance_threshold(mut self, threshold: T) -> Self {
        self.ad_variance_threshold = threshold;
        self
    }

    /// Set the AD checkpointing interval.
    #[cfg(feature = "enzyme-ad")]
    pub fn with_ad_checkpoint_interval(mut self, interval: usize) -> Self {
        self.ad_checkpoint_interval = Some(interval);
        self
    }
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
