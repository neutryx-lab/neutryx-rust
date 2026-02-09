//! Numerical diagnostics and Jacobian validation utilities for calibration.
//!
//! # Requirement: 5.3, 5.5
//!
//! ## Numerical Stability (Requirement 5)
//!
//! - [`JacobianQuality`]: Classification of Jacobian matrix quality
//!   (Good/Warning/Poor)
//! - [`NumericalDiagnostics`]: Comprehensive diagnostic information for
//!   calibration
//! - [`RegularisationType`]: Type of regularisation applied (None/Tikhonov/LM)

use num_traits::Float;
use pricer_core::math::numeric::from_f64;

// =============================================================================
// Numerical Diagnostics (Requirement 5.5)
// =============================================================================

/// Quality classification for Jacobian matrix.
///
/// Used to assess the numerical stability of the calibration problem.
///
/// # Requirement: 5.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JacobianQuality {
    /// Jacobian is well-conditioned with no issues detected.
    Good,

    /// Jacobian has potential issues but calibration may proceed.
    ///
    /// Contains a reason describing the warning condition.
    Warning {
        /// Reason for the warning (e.g., "near-zero diagonal element")
        reason: &'static str,
    },

    /// Jacobian is poorly conditioned and may cause numerical issues.
    ///
    /// Contains a reason describing the poor quality.
    Poor {
        /// Reason for poor quality (e.g., "NaN detected", "Inf detected")
        reason: &'static str,
    },
}

impl JacobianQuality {
    /// Create a Good quality result.
    pub fn good() -> Self { JacobianQuality::Good }

    /// Create a Warning quality result.
    pub fn warning(reason: &'static str) -> Self { JacobianQuality::Warning { reason } }

    /// Create a Poor quality result.
    pub fn poor(reason: &'static str) -> Self { JacobianQuality::Poor { reason } }

    /// Check if the quality is acceptable (Good or Warning).
    pub fn is_acceptable(&self) -> bool { !matches!(self, JacobianQuality::Poor { .. }) }

    /// Check if the quality is Good.
    pub fn is_good(&self) -> bool { matches!(self, JacobianQuality::Good) }

    /// Get the reason string if this is a Warning or Poor quality.
    pub fn reason(&self) -> Option<&'static str> {
        match self {
            JacobianQuality::Good => None,
            JacobianQuality::Warning { reason } | JacobianQuality::Poor { reason } => Some(reason),
        }
    }
}

impl std::fmt::Display for JacobianQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JacobianQuality::Good => write!(f, "Good"),
            JacobianQuality::Warning { reason } => write!(f, "Warning: {reason}"),
            JacobianQuality::Poor { reason } => write!(f, "Poor: {reason}"),
        }
    }
}

/// Type of regularisation applied during calibration.
///
/// # Requirement: 5.2, 5.5
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RegularisationType<T: Float> {
    /// No regularisation applied.
    #[default]
    None,

    /// Tikhonov regularisation (ridge regression).
    ///
    /// Adds λI to the Hessian/Jacobian to improve conditioning.
    Tikhonov {
        /// Damping parameter λ
        damping: T,
    },

    /// Levenberg-Marquardt regularisation.
    ///
    /// Adds λ·diag(J^T·J) to blend between Gauss-Newton and gradient descent.
    LevenbergMarquardt {
        /// Damping parameter λ
        lambda: T,
    },
}

impl<T: Float> RegularisationType<T> {
    /// Create Tikhonov regularisation with the given damping factor.
    pub fn tikhonov(damping: T) -> Self { RegularisationType::Tikhonov { damping } }

    /// Create Levenberg-Marquardt regularisation with the given lambda.
    pub fn levenberg_marquardt(lambda: T) -> Self {
        RegularisationType::LevenbergMarquardt { lambda }
    }

    /// Check if any regularisation is applied.
    pub fn is_regularised(&self) -> bool { !matches!(self, RegularisationType::None) }

    /// Get the damping parameter if regularisation is applied.
    pub fn damping(&self) -> Option<T> {
        match self {
            RegularisationType::None => None,
            RegularisationType::Tikhonov { damping } => Some(*damping),
            RegularisationType::LevenbergMarquardt { lambda } => Some(*lambda),
        }
    }
}

impl<T: Float + std::fmt::Display> std::fmt::Display for RegularisationType<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegularisationType::None => write!(f, "None"),
            RegularisationType::Tikhonov { damping } => write!(f, "Tikhonov(λ={damping})"),
            RegularisationType::LevenbergMarquardt { lambda } => write!(f, "LM(λ={lambda})"),
        }
    }
}

/// Comprehensive numerical diagnostics for calibration.
///
/// This structure captures diagnostic information about the numerical
/// stability of a calibration run, including condition numbers, residual
/// history, and any regularisation that was applied.
///
/// # Requirement: 5.5
#[derive(Debug, Clone)]
pub struct NumericalDiagnostics<T: Float> {
    /// Estimated condition number of the Jacobian matrix.
    ///
    /// Higher values indicate ill-conditioning. Values > 1e10 typically
    /// require regularisation.
    pub condition_number: Option<T>,

    /// Residual norm history at each iteration.
    ///
    /// Useful for diagnosing convergence behaviour.
    pub residual_history: Vec<T>,

    /// Type of regularisation applied during calibration.
    pub regularisation_applied: RegularisationType<T>,

    /// Quality assessment of the final Jacobian matrix.
    pub jacobian_quality: JacobianQuality,

    /// Number of NaN values detected (and handled) during calibration.
    pub nan_count: usize,

    /// Number of Inf values detected (and handled) during calibration.
    pub inf_count: usize,

    /// Number of near-zero diagonal elements detected.
    pub near_zero_diagonal_count: usize,

    /// Whether AD fallback to finite differences was triggered.
    pub ad_fallback_used: bool,

    /// Variance of AD gradients vs finite differences (if compared).
    ///
    /// High variance (> 1e6) indicates AD instability.
    pub ad_variance: Option<T>,
}

impl<T: Float> Default for NumericalDiagnostics<T> {
    fn default() -> Self {
        Self {
            condition_number: None,
            residual_history: Vec::new(),
            regularisation_applied: RegularisationType::None,
            jacobian_quality: JacobianQuality::Good,
            nan_count: 0,
            inf_count: 0,
            near_zero_diagonal_count: 0,
            ad_fallback_used: false,
            ad_variance: None,
        }
    }
}

impl<T: Float> NumericalDiagnostics<T> {
    /// Create new empty diagnostics.
    pub fn new() -> Self { Self::default() }

    /// Create diagnostics with a condition number.
    pub fn with_condition_number(mut self, cond: T) -> Self {
        self.condition_number = Some(cond);
        self
    }

    /// Add a residual value to the history.
    pub fn push_residual(&mut self, residual: T) { self.residual_history.push(residual); }

    /// Set the regularisation type.
    pub fn with_regularisation(mut self, reg: RegularisationType<T>) -> Self {
        self.regularisation_applied = reg;
        self
    }

    /// Set the Jacobian quality.
    pub fn with_jacobian_quality(mut self, quality: JacobianQuality) -> Self {
        self.jacobian_quality = quality;
        self
    }

    /// Record that AD fallback was used.
    pub fn mark_ad_fallback(&mut self) { self.ad_fallback_used = true; }

    /// Set the AD variance.
    pub fn with_ad_variance(mut self, variance: T) -> Self {
        self.ad_variance = Some(variance);
        self
    }

    /// Increment NaN counter.
    pub fn record_nan(&mut self) { self.nan_count += 1; }

    /// Increment Inf counter.
    pub fn record_inf(&mut self) { self.inf_count += 1; }

    /// Increment near-zero diagonal counter.
    pub fn record_near_zero_diagonal(&mut self) { self.near_zero_diagonal_count += 1; }

    /// Check if there were any numerical issues.
    pub fn has_issues(&self) -> bool {
        self.nan_count > 0
            || self.inf_count > 0
            || self.near_zero_diagonal_count > 0
            || !self.jacobian_quality.is_good()
    }

    /// Check if condition number exceeds threshold.
    pub fn is_ill_conditioned(&self, threshold: T) -> bool {
        self.condition_number
            .map(|c| c > threshold)
            .unwrap_or(false)
    }

    /// Check if AD was unstable (high variance compared to finite differences).
    pub fn is_ad_unstable(&self, variance_threshold: T) -> bool {
        self.ad_variance
            .map(|v| v > variance_threshold)
            .unwrap_or(false)
    }

    /// Get the final residual norm (last entry in history).
    pub fn final_residual(&self) -> Option<T> { self.residual_history.last().copied() }

    /// Get the number of iterations (length of residual history).
    pub fn iteration_count(&self) -> usize { self.residual_history.len() }

    /// Check if regularisation was applied.
    pub fn was_regularised(&self) -> bool { self.regularisation_applied.is_regularised() }

    /// Generate a summary string for logging.
    pub fn summary(&self) -> String
    where
        T: std::fmt::Display + std::fmt::LowerExp,
    {
        let cond_str = self
            .condition_number
            .map(|c| format!("{c:.2e}"))
            .unwrap_or_else(|| "N/A".to_string());

        let issues = if self.has_issues() {
            format!(
                " [NaN:{}, Inf:{}, ZeroDiag:{}]",
                self.nan_count, self.inf_count, self.near_zero_diagonal_count
            )
        } else {
            String::new()
        };

        format!(
            "Iterations: {}, Final residual: {}, Condition: {}, Quality: {}, Regularisation: {}{}",
            self.iteration_count(),
            self.final_residual()
                .map(|r| format!("{r:.6e}"))
                .unwrap_or_else(|| "N/A".to_string()),
            cond_str,
            self.jacobian_quality,
            self.regularisation_applied,
            issues
        )
    }
}

// =============================================================================
// Jacobian Validation Utilities (Requirement 5.3)
// =============================================================================

/// Validate a Jacobian matrix for numerical quality.
///
/// Checks for:
/// - NaN values
/// - Inf values
/// - Near-zero diagonal elements (< 1e-14)
///
/// # Requirement: 5.3
///
/// # Arguments
///
/// * `jacobian` - The Jacobian matrix to validate (as row-major flattened
///   vector)
/// * `nrows` - Number of rows
/// * `ncols` - Number of columns
/// * `zero_threshold` - Threshold below which diagonal elements are considered
///   near-zero
///
/// # Returns
///
/// A tuple of (JacobianQuality, NumericalDiagnostics) with validation results.
pub fn validate_jacobian_matrix<T: Float>(
    jacobian: &[T],
    nrows: usize,
    ncols: usize,
    zero_threshold: T,
) -> (JacobianQuality, NumericalDiagnostics<T>) {
    let mut diagnostics = NumericalDiagnostics::new();

    // Check for NaN and Inf
    for &val in jacobian {
        if val.is_nan() {
            diagnostics.record_nan();
        }
        if val.is_infinite() {
            diagnostics.record_inf();
        }
    }

    // Check diagonal elements (for square matrices)
    if nrows == ncols {
        for i in 0..nrows {
            let idx = i * ncols + i;
            if idx < jacobian.len() {
                let diag_val = jacobian[idx];
                if diag_val.abs() < zero_threshold {
                    diagnostics.record_near_zero_diagonal();
                }
            }
        }
    }

    // Determine quality
    let quality = if diagnostics.nan_count > 0 {
        JacobianQuality::poor("NaN detected in Jacobian")
    } else if diagnostics.inf_count > 0 {
        JacobianQuality::poor("Inf detected in Jacobian")
    } else if diagnostics.near_zero_diagonal_count > 0 {
        JacobianQuality::warning("Near-zero diagonal element detected")
    } else {
        JacobianQuality::good()
    };

    diagnostics.jacobian_quality = quality;

    (quality, diagnostics)
}

/// Validate a Jacobian DMatrix for numerical quality.
///
/// # Requirement: 5.3
#[cfg(feature = "global-bootstrap")]
pub fn validate_jacobian_dmatrix<T>(
    jacobian: &pricer_core::math::linalg::DMatrix<T>,
    zero_threshold: T,
) -> (JacobianQuality, NumericalDiagnostics<T>)
where
    T: Float + pricer_core::math::linalg::RealField,
{
    let nrows = jacobian.nrows();
    let ncols = jacobian.ncols();

    let mut diagnostics = NumericalDiagnostics::new();

    // Check for NaN and Inf
    for &val in jacobian.iter() {
        if val.is_nan() {
            diagnostics.record_nan();
        }
        if val.is_infinite() {
            diagnostics.record_inf();
        }
    }

    // Check diagonal elements (for square matrices)
    if nrows == ncols {
        for i in 0..nrows {
            let diag_val = jacobian[(i, i)];
            if Float::abs(diag_val) < zero_threshold {
                diagnostics.record_near_zero_diagonal();
            }
        }
    }

    // Determine quality
    let quality = if diagnostics.nan_count > 0 {
        JacobianQuality::poor("NaN detected in Jacobian")
    } else if diagnostics.inf_count > 0 {
        JacobianQuality::poor("Inf detected in Jacobian")
    } else if diagnostics.near_zero_diagonal_count > 0 {
        JacobianQuality::warning("Near-zero diagonal element detected")
    } else {
        JacobianQuality::good()
    };

    diagnostics.jacobian_quality = quality;

    (quality, diagnostics)
}

/// Estimate condition number using row-sum norm heuristic.
///
/// This is a cheap O(n²) estimate, not the true condition number
/// (which would require SVD).
///
/// # Requirement: 5.1
#[cfg(feature = "global-bootstrap")]
pub fn estimate_condition_number<T>(jacobian: &pricer_core::math::linalg::DMatrix<T>) -> Option<T>
where
    T: Float + pricer_core::math::linalg::RealField,
{
    let nrows = jacobian.nrows();
    if nrows == 0 {
        return None;
    }

    let mut max_row_sum = T::zero();
    let mut min_row_sum = T::infinity();

    for i in 0..nrows {
        let row_sum = (0..jacobian.ncols())
            .map(|j| Float::abs(jacobian[(i, j)]))
            .fold(T::zero(), |acc, x| acc + x);

        if row_sum > max_row_sum {
            max_row_sum = row_sum;
        }
        if row_sum < min_row_sum && row_sum > T::zero() {
            min_row_sum = row_sum;
        }
    }

    if min_row_sum > T::zero() {
        Some(max_row_sum / min_row_sum)
    } else {
        None
    }
}

/// Apply Tikhonov regularisation to a matrix.
///
/// Adds λI to the matrix to improve conditioning.
///
/// # Requirement: 5.2
#[cfg(feature = "global-bootstrap")]
pub fn apply_tikhonov_regularisation<T>(
    matrix: &mut pricer_core::math::linalg::DMatrix<T>,
    damping: T,
) where
    T: Float + pricer_core::math::linalg::RealField,
{
    let n = matrix.nrows().min(matrix.ncols());
    for i in 0..n {
        matrix[(i, i)] = matrix[(i, i)] + damping;
    }
}

/// Check if Tikhonov regularisation should be applied based on condition
/// number.
///
/// # Requirement: 5.2
///
/// Returns the recommended damping factor if regularisation is needed.
#[allow(dead_code)]
pub fn should_apply_regularisation<T: Float>(
    condition_number: T,
    max_condition_number: T,
) -> Option<T> {
    if condition_number > max_condition_number {
        // Use sqrt of excess as damping factor
        let excess = condition_number / max_condition_number;
        let damping = excess.sqrt() * from_f64::<T>(1e-6);
        Some(damping)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // JacobianQuality Tests (Requirement 5.3)
    // =========================================================================

    #[test]
    fn test_jacobian_quality_good() {
        let quality = JacobianQuality::good();
        assert!(quality.is_good());
        assert!(quality.is_acceptable());
        assert!(quality.reason().is_none());
        assert_eq!(format!("{quality}"), "Good");
    }

    #[test]
    fn test_jacobian_quality_warning() {
        let quality = JacobianQuality::warning("near-zero diagonal");
        assert!(!quality.is_good());
        assert!(quality.is_acceptable());
        assert_eq!(quality.reason(), Some("near-zero diagonal"));
        assert!(format!("{quality}").contains("Warning"));
    }

    #[test]
    fn test_jacobian_quality_poor() {
        let quality = JacobianQuality::poor("NaN detected");
        assert!(!quality.is_good());
        assert!(!quality.is_acceptable());
        assert_eq!(quality.reason(), Some("NaN detected"));
        assert!(format!("{quality}").contains("Poor"));
    }

    #[test]
    fn test_jacobian_quality_equality() {
        assert_eq!(JacobianQuality::good(), JacobianQuality::good());
        assert_eq!(
            JacobianQuality::warning("test"),
            JacobianQuality::warning("test")
        );
        assert_ne!(JacobianQuality::good(), JacobianQuality::poor("NaN"));
    }

    // =========================================================================
    // RegularisationType Tests (Requirement 5.2, 5.5)
    // =========================================================================

    #[test]
    fn test_regularisation_none() {
        let reg: RegularisationType<f64> = RegularisationType::None;
        assert!(!reg.is_regularised());
        assert!(reg.damping().is_none());
        assert_eq!(format!("{reg}"), "None");
    }

    #[test]
    fn test_regularisation_tikhonov() {
        let reg: RegularisationType<f64> = RegularisationType::tikhonov(1e-6);
        assert!(reg.is_regularised());
        assert!((reg.damping().unwrap() - 1e-6).abs() < 1e-15);
        assert!(format!("{reg}").contains("Tikhonov"));
    }

    #[test]
    fn test_regularisation_levenberg_marquardt() {
        let reg: RegularisationType<f64> = RegularisationType::levenberg_marquardt(0.01);
        assert!(reg.is_regularised());
        assert!((reg.damping().unwrap() - 0.01).abs() < 1e-15);
        assert!(format!("{reg}").contains("LM"));
    }

    #[test]
    fn test_regularisation_default() {
        let reg: RegularisationType<f64> = RegularisationType::default();
        assert!(!reg.is_regularised());
        assert!(matches!(reg, RegularisationType::None));
    }

    // =========================================================================
    // NumericalDiagnostics Tests (Requirement 5.5)
    // =========================================================================

    #[test]
    fn test_numerical_diagnostics_default() {
        let diag: NumericalDiagnostics<f64> = NumericalDiagnostics::new();
        assert!(diag.condition_number.is_none());
        assert!(diag.residual_history.is_empty());
        assert!(!diag.has_issues());
        assert!(!diag.was_regularised());
        assert!(!diag.ad_fallback_used);
    }

    #[test]
    fn test_numerical_diagnostics_with_condition_number() {
        let diag: NumericalDiagnostics<f64> =
            NumericalDiagnostics::new().with_condition_number(1e8);
        assert!(diag.condition_number.is_some());
        assert!((diag.condition_number.unwrap() - 1e8).abs() < 1.0);
    }

    #[test]
    fn test_numerical_diagnostics_residual_history() {
        let mut diag: NumericalDiagnostics<f64> = NumericalDiagnostics::new();
        diag.push_residual(1.0);
        diag.push_residual(0.1);
        diag.push_residual(0.01);

        assert_eq!(diag.iteration_count(), 3);
        assert!((diag.final_residual().unwrap() - 0.01).abs() < 1e-15);
    }

    #[test]
    fn test_numerical_diagnostics_issues() {
        let mut diag: NumericalDiagnostics<f64> = NumericalDiagnostics::new();
        assert!(!diag.has_issues());

        diag.record_nan();
        assert!(diag.has_issues());
        assert_eq!(diag.nan_count, 1);

        diag.record_inf();
        assert_eq!(diag.inf_count, 1);

        diag.record_near_zero_diagonal();
        assert_eq!(diag.near_zero_diagonal_count, 1);
    }

    #[test]
    fn test_numerical_diagnostics_ill_conditioned() {
        let diag: NumericalDiagnostics<f64> =
            NumericalDiagnostics::new().with_condition_number(1e12);
        assert!(diag.is_ill_conditioned(1e10));
        assert!(!diag.is_ill_conditioned(1e14));
    }

    #[test]
    fn test_numerical_diagnostics_ad_fallback() {
        let mut diag: NumericalDiagnostics<f64> = NumericalDiagnostics::new();
        assert!(!diag.ad_fallback_used);
        diag.mark_ad_fallback();
        assert!(diag.ad_fallback_used);
    }

    #[test]
    fn test_numerical_diagnostics_ad_unstable() {
        let diag: NumericalDiagnostics<f64> = NumericalDiagnostics::new().with_ad_variance(1e7);
        assert!(diag.is_ad_unstable(1e6));
        assert!(!diag.is_ad_unstable(1e8));
    }

    #[test]
    fn test_numerical_diagnostics_summary() {
        let mut diag: NumericalDiagnostics<f64> =
            NumericalDiagnostics::new().with_condition_number(1e8);
        diag.push_residual(0.001);
        diag.record_nan();

        let summary = diag.summary();
        assert!(summary.contains("Iterations: 1"));
        assert!(summary.contains("NaN:1"));
    }

    #[test]
    fn test_numerical_diagnostics_with_regularisation() {
        let diag: NumericalDiagnostics<f64> =
            NumericalDiagnostics::new().with_regularisation(RegularisationType::tikhonov(1e-6));
        assert!(diag.was_regularised());
        let summary = diag.summary();
        assert!(summary.contains("Tikhonov"));
    }

    // =========================================================================
    // Jacobian Validation Tests (Requirement 5.3)
    // =========================================================================

    #[test]
    fn test_validate_jacobian_good() {
        let jacobian = vec![1.0, 0.1, 0.1, 1.0];
        let (quality, diag) = validate_jacobian_matrix(&jacobian, 2, 2, 1e-14);

        assert!(quality.is_good());
        assert!(!diag.has_issues());
    }

    #[test]
    fn test_validate_jacobian_with_nan() {
        let jacobian = vec![1.0, f64::NAN, 0.1, 1.0];
        let (quality, diag) = validate_jacobian_matrix(&jacobian, 2, 2, 1e-14);

        assert!(matches!(quality, JacobianQuality::Poor { .. }));
        assert_eq!(diag.nan_count, 1);
    }

    #[test]
    fn test_validate_jacobian_with_inf() {
        let jacobian = vec![1.0, 0.1, f64::INFINITY, 1.0];
        let (quality, diag) = validate_jacobian_matrix(&jacobian, 2, 2, 1e-14);

        assert!(matches!(quality, JacobianQuality::Poor { .. }));
        assert_eq!(diag.inf_count, 1);
    }

    #[test]
    fn test_validate_jacobian_near_zero_diagonal() {
        let jacobian = vec![1e-16, 0.1, 0.1, 1.0];
        let (quality, diag) = validate_jacobian_matrix(&jacobian, 2, 2, 1e-14);

        assert!(matches!(quality, JacobianQuality::Warning { .. }));
        assert_eq!(diag.near_zero_diagonal_count, 1);
    }

    #[test]
    fn test_should_apply_regularisation() {
        // Below threshold - no regularisation
        let damping = should_apply_regularisation(1e8, 1e10);
        assert!(damping.is_none());

        // Above threshold - regularisation needed
        let damping = should_apply_regularisation(1e12, 1e10);
        assert!(damping.is_some());
        assert!(damping.unwrap() > 0.0);
    }
}
