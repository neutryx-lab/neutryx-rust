//! Jump pillar and configuration types for CB meeting jump calibration.
//!
//! This module provides types for handling forward rate jumps at central bank
//! meeting dates during yield curve calibration.
//!
//! # Key Types
//!
//! - [`JumpPillar`]: Represents a jump at a specific date with
//!   expected/realised values
//! - [`JumpConfig`]: Configuration for jump-aware calibration
//!
//! # Requirements Coverage
//!
//! - Requirement 2.1: JumpPillar for CB meeting dates
//! - Requirement 7.5: JumpConfig with default disabled

use num_traits::Float;
use pricer_core::math::numeric::from_f64;

use super::CalibrationError;

// =============================================================================
// JumpPillar
// =============================================================================

/// Jump pillar for CB meeting date.
///
/// Represents a forward rate jump at a central bank meeting date.
/// The jump is modelled as a discrete shift in the forward rate curve.
///
/// # Type Parameters
///
/// * `T` - Floating-point type for numerical values
///
/// # Example
///
/// ```ignore
/// use pricer_models::builder::JumpPillar;
///
/// // Create a jump pillar for a 25bps rate hike expectation
/// let jump = JumpPillar::new(0.5, 25.0);
/// assert_eq!(jump.expected_jump_rate(), 0.0025);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JumpPillar<T: Float> {
    /// Time to jump date in years from reference date.
    pub time: T,
    /// Expected jump size in absolute rate (converted from bps).
    pub expected_jump: T,
    /// Realised jump size after calibration.
    pub realised_jump: Option<T>,
    /// Index in the extended parameter vector (set during calibration setup).
    pub param_index: Option<usize>,
}

impl<T: Float> JumpPillar<T> {
    /// Create a new jump pillar.
    ///
    /// # Arguments
    ///
    /// * `time` - Time to jump date in years
    /// * `expected_jump_bps` - Expected jump size in basis points
    ///
    /// # Returns
    ///
    /// A new `JumpPillar` with expected jump converted to absolute rate.
    pub fn new(time: T, expected_jump_bps: T) -> Self {
        Self {
            time,
            expected_jump: Self::bps_to_rate(expected_jump_bps),
            realised_jump: None,
            param_index: None,
        }
    }

    /// Create a jump pillar with an already-converted rate.
    ///
    /// # Arguments
    ///
    /// * `time` - Time to jump date in years
    /// * `expected_jump_rate` - Expected jump size in absolute rate
    pub fn with_rate(time: T, expected_jump_rate: T) -> Self {
        Self {
            time,
            expected_jump: expected_jump_rate,
            realised_jump: None,
            param_index: None,
        }
    }

    /// Convert basis points to absolute rate.
    ///
    /// # Arguments
    ///
    /// * `bps` - Value in basis points
    ///
    /// # Returns
    ///
    /// Value converted to absolute rate (bps * 0.0001).
    pub fn bps_to_rate(bps: T) -> T { bps * from_f64::<T>(0.0001) }

    /// Convert absolute rate to basis points.
    ///
    /// # Arguments
    ///
    /// * `rate` - Value in absolute rate
    ///
    /// # Returns
    ///
    /// Value converted to basis points (rate * 10000).
    pub fn rate_to_bps(rate: T) -> T { rate * from_f64::<T>(10000.0) }

    /// Get the expected jump in absolute rate.
    pub fn expected_jump_rate(&self) -> T { self.expected_jump }

    /// Get the expected jump in basis points.
    pub fn expected_jump_bps(&self) -> T { Self::rate_to_bps(self.expected_jump) }

    /// Get the realised jump in absolute rate, if available.
    pub fn realised_jump_rate(&self) -> Option<T> { self.realised_jump }

    /// Get the realised jump in basis points, if available.
    pub fn realised_jump_bps(&self) -> Option<T> { self.realised_jump.map(Self::rate_to_bps) }

    /// Set the realised jump value.
    pub fn set_realised_jump(&mut self, rate: T) { self.realised_jump = Some(rate); }

    /// Set the parameter index in the calibration vector.
    pub fn set_param_index(&mut self, index: usize) { self.param_index = Some(index); }

    /// Check if this jump pillar has a realised value.
    pub fn is_calibrated(&self) -> bool { self.realised_jump.is_some() }

    /// Create from date strings and expected jump in bps.
    ///
    /// # Arguments
    ///
    /// * `reference_date` - Reference date string (YYYY-MM-DD)
    /// * `jump_date` - Jump date string (YYYY-MM-DD)
    /// * `expected_bps` - Expected jump in basis points
    ///
    /// # Returns
    ///
    /// A `JumpPillar` with time calculated from date difference.
    pub fn from_date_bps(
        reference_date: &str,
        jump_date: &str,
        expected_bps: T,
    ) -> Result<Self, CalibrationError> {
        let time = parse_date_diff_years(reference_date, jump_date)?;
        Ok(Self::new(from_f64(time), expected_bps))
    }
}

/// Parse the difference between two dates in years.
fn parse_date_diff_years(reference: &str, target: &str) -> Result<f64, CalibrationError> {
    use chrono::NaiveDate;

    let ref_date = NaiveDate::parse_from_str(reference, "%Y-%m-%d").map_err(|e| {
        CalibrationError::InvalidMarketData {
            message: format!("Invalid reference date '{}': {}", reference, e),
        }
    })?;

    let target_date = NaiveDate::parse_from_str(target, "%Y-%m-%d").map_err(|e| {
        CalibrationError::InvalidMarketData {
            message: format!("Invalid target date '{}': {}", target, e),
        }
    })?;

    let days = (target_date - ref_date).num_days();
    Ok(days as f64 / 365.0)
}

// =============================================================================
// JumpConfig
// =============================================================================

/// Configuration for jump-aware calibration.
///
/// Controls how the GlobalBootstrapper handles forward rate jumps
/// at central bank meeting dates.
///
/// # Type Parameters
///
/// * `T` - Floating-point type for numerical values
///
/// # Example
///
/// ```ignore
/// use pricer_models::builder::{JumpConfig, JumpPillar};
///
/// let config = JumpConfig::default()
///     .with_jump_pillars(vec![
///         JumpPillar::new(0.5, 25.0),  // 25bps expected at 6M
///         JumpPillar::new(1.0, 25.0),  // 25bps expected at 1Y
///     ])
///     .with_fallback(true);
///
/// assert!(config.enabled);
/// assert_eq!(config.jump_pillars.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct JumpConfig<T: Float> {
    /// Enable jump calibration.
    pub enabled: bool,
    /// Jump pillars for CB meeting dates.
    pub jump_pillars: Vec<JumpPillar<T>>,
    /// Fallback to non-jump calibration on convergence failure.
    pub fallback_on_failure: bool,
    /// Damping factor for jump parameters (0.0 to 1.0).
    /// Lower values reduce step size for jump parameters.
    pub jump_damping: Option<T>,
}

impl<T: Float> Default for JumpConfig<T> {
    /// Create a default JumpConfig with jumps disabled.
    ///
    /// # Requirement 7.5
    ///
    /// The default configuration has jumps disabled to ensure
    /// backward compatibility with existing workflows.
    fn default() -> Self {
        Self {
            enabled: false,
            jump_pillars: Vec::new(),
            fallback_on_failure: true,
            jump_damping: None,
        }
    }
}

impl<T: Float> JumpConfig<T> {
    /// Create a new JumpConfig with default settings.
    pub fn new() -> Self { Self::default() }

    /// Create an enabled JumpConfig with the given pillars.
    pub fn with_pillars(pillars: Vec<JumpPillar<T>>) -> Self {
        Self {
            enabled: !pillars.is_empty(),
            jump_pillars: pillars,
            fallback_on_failure: true,
            jump_damping: None,
        }
    }

    /// Set the jump pillars and enable jump calibration.
    pub fn with_jump_pillars(mut self, pillars: Vec<JumpPillar<T>>) -> Self {
        self.jump_pillars = pillars;
        self.enabled = !self.jump_pillars.is_empty();
        self
    }

    /// Enable or disable fallback to non-jump calibration on failure.
    pub fn with_fallback(mut self, enabled: bool) -> Self {
        self.fallback_on_failure = enabled;
        self
    }

    /// Set the damping factor for jump parameters.
    pub fn with_damping(mut self, factor: T) -> Self {
        self.jump_damping = Some(factor);
        self
    }

    /// Enable jump calibration.
    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Disable jump calibration.
    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Check if jump calibration is enabled and there are jump pillars.
    pub fn is_active(&self) -> bool { self.enabled && !self.jump_pillars.is_empty() }

    /// Get the number of jump pillars.
    pub fn num_jumps(&self) -> usize { self.jump_pillars.len() }

    /// Get jump pillars sorted by time.
    pub fn sorted_pillars(&self) -> Vec<JumpPillar<T>> {
        let mut pillars = self.jump_pillars.clone();
        pillars.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        pillars
    }

    /// Validate the jump configuration.
    ///
    /// Checks that all jump pillars have valid parameters.
    pub fn validate(&self) -> Result<(), CalibrationError> {
        for (i, pillar) in self.jump_pillars.iter().enumerate() {
            // Check time is positive
            if pillar.time <= T::zero() {
                return Err(CalibrationError::InvalidMarketData {
                    message: format!(
                        "Jump pillar {} has non-positive time: {}",
                        i,
                        pillar.time.to_f64().unwrap_or(0.0)
                    ),
                });
            }

            // Check expected jump is within reasonable range (±100bps)
            let max_bps = from_f64::<T>(100.0);
            let min_bps = from_f64::<T>(-100.0);
            let bps = pillar.expected_jump_bps();
            if bps < min_bps || bps > max_bps {
                return Err(CalibrationError::BoundsViolation {
                    param_name: format!("jump_pillar_{}", i),
                    value: bps.to_f64().unwrap_or(0.0),
                    lower: -100.0,
                    upper: 100.0,
                });
            }
        }
        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_jump_pillar_new() {
        let pillar: JumpPillar<f64> = JumpPillar::new(0.5, 25.0);

        assert_relative_eq!(pillar.time, 0.5, epsilon = 1e-10);
        assert_relative_eq!(pillar.expected_jump_rate(), 0.0025, epsilon = 1e-10);
        assert_relative_eq!(pillar.expected_jump_bps(), 25.0, epsilon = 1e-10);
        assert!(pillar.realised_jump.is_none());
        assert!(pillar.param_index.is_none());
    }

    #[test]
    fn test_jump_pillar_with_rate() {
        let pillar: JumpPillar<f64> = JumpPillar::with_rate(1.0, 0.0050);

        assert_relative_eq!(pillar.time, 1.0, epsilon = 1e-10);
        assert_relative_eq!(pillar.expected_jump_rate(), 0.0050, epsilon = 1e-10);
        assert_relative_eq!(pillar.expected_jump_bps(), 50.0, epsilon = 1e-10);
    }

    #[test]
    fn test_jump_pillar_bps_conversion() {
        assert_relative_eq!(JumpPillar::<f64>::bps_to_rate(100.0), 0.01, epsilon = 1e-10);
        assert_relative_eq!(
            JumpPillar::<f64>::bps_to_rate(-50.0),
            -0.005,
            epsilon = 1e-10
        );
        assert_relative_eq!(JumpPillar::<f64>::rate_to_bps(0.01), 100.0, epsilon = 1e-10);
        assert_relative_eq!(
            JumpPillar::<f64>::rate_to_bps(-0.005),
            -50.0,
            epsilon = 1e-10
        );
    }

    #[test]
    fn test_jump_pillar_realised() {
        let mut pillar: JumpPillar<f64> = JumpPillar::new(0.5, 25.0);
        assert!(!pillar.is_calibrated());

        pillar.set_realised_jump(0.0030);
        assert!(pillar.is_calibrated());
        assert_relative_eq!(
            pillar.realised_jump_rate().unwrap(),
            0.0030,
            epsilon = 1e-10
        );
        assert_relative_eq!(pillar.realised_jump_bps().unwrap(), 30.0, epsilon = 1e-10);
    }

    #[test]
    fn test_jump_pillar_param_index() {
        let mut pillar: JumpPillar<f64> = JumpPillar::new(0.5, 25.0);
        assert!(pillar.param_index.is_none());

        pillar.set_param_index(5);
        assert_eq!(pillar.param_index, Some(5));
    }

    #[test]
    fn test_jump_pillar_from_date_bps() {
        let pillar: JumpPillar<f64> =
            JumpPillar::from_date_bps("2024-01-01", "2024-07-01", 25.0).unwrap();

        // Approximately 0.5 years (182 days / 365)
        assert!(pillar.time > 0.4 && pillar.time < 0.6);
        assert_relative_eq!(pillar.expected_jump_bps(), 25.0, epsilon = 1e-10);
    }

    #[test]
    fn test_jump_pillar_from_date_bps_invalid() {
        let result: Result<JumpPillar<f64>, _> =
            JumpPillar::from_date_bps("invalid", "2024-07-01", 25.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_jump_config_default() {
        let config: JumpConfig<f64> = JumpConfig::default();

        assert!(!config.enabled);
        assert!(config.jump_pillars.is_empty());
        assert!(config.fallback_on_failure);
        assert!(config.jump_damping.is_none());
        assert!(!config.is_active());
    }

    #[test]
    fn test_jump_config_with_pillars() {
        let config: JumpConfig<f64> =
            JumpConfig::with_pillars(vec![JumpPillar::new(0.5, 25.0), JumpPillar::new(1.0, 25.0)]);

        assert!(config.enabled);
        assert_eq!(config.num_jumps(), 2);
        assert!(config.is_active());
    }

    #[test]
    fn test_jump_config_builder() {
        let config: JumpConfig<f64> = JumpConfig::new()
            .with_jump_pillars(vec![JumpPillar::new(0.5, 25.0)])
            .with_fallback(false)
            .with_damping(0.5);

        assert!(config.enabled);
        assert_eq!(config.num_jumps(), 1);
        assert!(!config.fallback_on_failure);
        assert_relative_eq!(config.jump_damping.unwrap(), 0.5, epsilon = 1e-10);
    }

    #[test]
    fn test_jump_config_enable_disable() {
        let config: JumpConfig<f64> = JumpConfig::with_pillars(vec![JumpPillar::new(0.5, 25.0)]);
        assert!(config.enabled);

        let config = config.disable();
        assert!(!config.enabled);
        assert!(!config.is_active());

        let config = config.enable();
        assert!(config.enabled);
        assert!(config.is_active());
    }

    #[test]
    fn test_jump_config_sorted_pillars() {
        let config: JumpConfig<f64> = JumpConfig::with_pillars(vec![
            JumpPillar::new(1.0, 25.0),
            JumpPillar::new(0.5, 25.0),
            JumpPillar::new(2.0, 25.0),
        ]);

        let sorted = config.sorted_pillars();
        assert_relative_eq!(sorted[0].time, 0.5, epsilon = 1e-10);
        assert_relative_eq!(sorted[1].time, 1.0, epsilon = 1e-10);
        assert_relative_eq!(sorted[2].time, 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_jump_config_validate_success() {
        let config: JumpConfig<f64> = JumpConfig::with_pillars(vec![
            JumpPillar::new(0.5, 25.0),
            JumpPillar::new(1.0, -25.0),
        ]);

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_jump_config_validate_out_of_range() {
        let config: JumpConfig<f64> = JumpConfig::with_pillars(vec![JumpPillar::new(0.5, 150.0)]);

        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CalibrationError::BoundsViolation { .. }
        ));
    }

    #[test]
    fn test_jump_config_validate_negative_time() {
        let config: JumpConfig<f64> = JumpConfig::with_pillars(vec![JumpPillar::new(-0.5, 25.0)]);

        let result = config.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CalibrationError::InvalidMarketData { .. }
        ));
    }
}
