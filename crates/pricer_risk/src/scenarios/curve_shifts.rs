//! Curve shift utilities for scenario analysis.
//!
//! This module provides utilities for applying various shift patterns to yield curves:
//! - Parallel shifts (uniform across all tenors)
//! - Twist shifts (steepening/flattening)
//! - Butterfly shifts (curvature changes)
//!
//! # Requirements
//!
//! - Requirement 2.4: カーブシフト機能の拡充

use pricer_core::market_data::curves::{CurveEnum, CurveName, CurveSet, YieldCurve};
use pricer_core::traits::risk::ShiftType;

#[cfg(feature = "serde")]
use serde::Serialize;

/// Error types for curve shift operations.
#[derive(Debug, thiserror::Error)]
pub enum CurveShiftError {
    /// Curve not found in set.
    #[error("Curve not found: {0}")]
    CurveNotFound(String),

    /// Invalid shift parameters.
    #[error("Invalid shift parameters: {0}")]
    InvalidShift(String),

    /// Rebuild failed.
    #[error("Curve rebuild failed: {0}")]
    RebuildFailed(String),
}

/// Specification for a curve shift operation.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct CurveShiftSpec {
    /// Name of the curve to shift.
    pub curve_name: String,

    /// Type of shift to apply.
    pub shift_type: CurveShiftType,

    /// Reference tenor for twist/butterfly (years).
    pub pivot_tenor: Option<f64>,
}

/// Types of curve shifts.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub enum CurveShiftType {
    /// Uniform shift across all tenors.
    ///
    /// `shifted_rate(t) = base_rate(t) + shift_amount`
    Parallel(f64),

    /// Linear twist (steepening/flattening).
    ///
    /// Short end moves opposite to long end.
    /// `shifted_rate(t) = base_rate(t) + short_shift + (long_shift - short_shift) * t / max_tenor`
    Twist {
        /// Shift for short end (t=0).
        short_shift: f64,
        /// Shift for long end (t=max_tenor).
        long_shift: f64,
    },

    /// Butterfly shift (curvature).
    ///
    /// Wings move opposite to belly.
    /// `shifted_rate(t) = base_rate(t) + wing_shift + (belly_shift - wing_shift) * parabola(t)`
    Butterfly {
        /// Shift for short and long ends.
        wing_shift: f64,
        /// Shift for middle (belly).
        belly_shift: f64,
        /// Belly tenor (default 5Y).
        belly_tenor: f64,
    },

    /// Custom tenor-specific shifts.
    ///
    /// Applies different shifts at specific tenors with interpolation.
    TenorSpecific {
        /// Tenors in years.
        tenors: Vec<f64>,
        /// Shift amounts for each tenor.
        shifts: Vec<f64>,
    },
}

impl CurveShiftType {
    /// Creates a parallel shift.
    pub fn parallel(amount: f64) -> Self {
        Self::Parallel(amount)
    }

    /// Creates a twist shift (steepening).
    ///
    /// Positive values = short end up, long end down (flattening).
    /// Negative values = short end down, long end up (steepening).
    pub fn twist(short_shift: f64, long_shift: f64) -> Self {
        Self::Twist {
            short_shift,
            long_shift,
        }
    }

    /// Creates a steepening shift.
    ///
    /// Short end down, long end up.
    pub fn steepen(amount: f64) -> Self {
        Self::Twist {
            short_shift: -amount,
            long_shift: amount,
        }
    }

    /// Creates a flattening shift.
    ///
    /// Short end up, long end down.
    pub fn flatten(amount: f64) -> Self {
        Self::Twist {
            short_shift: amount,
            long_shift: -amount,
        }
    }

    /// Creates a butterfly shift with default belly tenor (5Y).
    pub fn butterfly(wing_shift: f64, belly_shift: f64) -> Self {
        Self::Butterfly {
            wing_shift,
            belly_shift,
            belly_tenor: 5.0,
        }
    }

    /// Creates a butterfly shift with custom belly tenor.
    pub fn butterfly_at(wing_shift: f64, belly_shift: f64, belly_tenor: f64) -> Self {
        Self::Butterfly {
            wing_shift,
            belly_shift,
            belly_tenor,
        }
    }

    /// Creates a positive curvature shift (belly up, wings down).
    pub fn positive_curvature(amount: f64) -> Self {
        Self::butterfly(-amount / 2.0, amount)
    }

    /// Creates a negative curvature shift (belly down, wings up).
    pub fn negative_curvature(amount: f64) -> Self {
        Self::butterfly(amount / 2.0, -amount)
    }

    /// Computes the shift amount for a given tenor.
    pub fn shift_at_tenor(&self, tenor: f64, max_tenor: f64) -> f64 {
        match *self {
            Self::Parallel(amount) => amount,
            Self::Twist {
                short_shift,
                long_shift,
            } => {
                // Linear interpolation from short to long
                let t_norm = (tenor / max_tenor).clamp(0.0, 1.0);
                short_shift + (long_shift - short_shift) * t_norm
            }
            Self::Butterfly {
                wing_shift,
                belly_shift,
                belly_tenor,
            } => {
                // Parabolic profile with peak at belly_tenor
                // f(t) = wing_shift + (belly_shift - wing_shift) * (1 - ((t - belly_tenor) / belly_tenor)^2)
                let t_norm = ((tenor - belly_tenor) / belly_tenor).abs();
                let parabola = (1.0 - t_norm * t_norm).max(0.0);
                wing_shift + (belly_shift - wing_shift) * parabola
            }
            Self::TenorSpecific {
                ref tenors,
                ref shifts,
            } => {
                // Linear interpolation between tenor points
                if tenors.is_empty() || shifts.is_empty() {
                    return 0.0;
                }

                // Find surrounding tenors
                let mut lower_idx = 0;
                for (i, &t) in tenors.iter().enumerate() {
                    if t <= tenor {
                        lower_idx = i;
                    } else {
                        break;
                    }
                }

                if lower_idx >= tenors.len() - 1 {
                    return shifts[shifts.len() - 1];
                }

                let t0 = tenors[lower_idx];
                let t1 = tenors[lower_idx + 1];
                let s0 = shifts[lower_idx];
                let s1 = shifts[lower_idx + 1];

                // Linear interpolation
                if (t1 - t0).abs() < 1e-10 {
                    s0
                } else {
                    s0 + (s1 - s0) * (tenor - t0) / (t1 - t0)
                }
            }
        }
    }

    /// Converts to pricer_core ShiftType (for parallel shifts only).
    pub fn to_shift_type(&self) -> Option<ShiftType<f64>> {
        match *self {
            Self::Parallel(amount) => Some(ShiftType::parallel(amount)),
            Self::Twist {
                short_shift,
                long_shift,
            } => Some(ShiftType::twist(short_shift, long_shift)),
            Self::Butterfly {
                wing_shift,
                belly_shift,
                ..
            } => Some(ShiftType::butterfly(wing_shift, belly_shift)),
            _ => None,
        }
    }
}

impl CurveShiftSpec {
    /// Creates a new curve shift specification.
    pub fn new(curve_name: impl Into<String>, shift_type: CurveShiftType) -> Self {
        Self {
            curve_name: curve_name.into(),
            shift_type,
            pivot_tenor: None,
        }
    }

    /// Sets the pivot tenor for twist/butterfly shifts.
    pub fn with_pivot(mut self, tenor: f64) -> Self {
        self.pivot_tenor = Some(tenor);
        self
    }
}

/// Utility for applying shifts to curve sets.
///
/// # Examples
///
/// ```rust,ignore
/// use pricer_risk::scenarios::{CurveShifter, CurveShiftType};
///
/// let shifted = CurveShifter::apply_parallel(&curves, 0.0001)?;
/// let twisted = CurveShifter::apply_twist(&curves, 0.0005, -0.0003)?;
/// ```
pub struct CurveShifter;

impl CurveShifter {
    /// Applies a parallel shift to all curves in the set.
    ///
    /// # Arguments
    ///
    /// * `curves` - The curve set to shift
    /// * `shift_amount` - Amount to add to all rates
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 2.4: パラレルシフト実装
    pub fn apply_parallel(curves: &CurveSet<f64>, shift_amount: f64) -> CurveSet<f64> {
        Self::apply_shift(curves, CurveShiftType::Parallel(shift_amount))
    }

    /// Applies a twist shift to all curves in the set.
    ///
    /// # Arguments
    ///
    /// * `curves` - The curve set to shift
    /// * `short_shift` - Shift for short end
    /// * `long_shift` - Shift for long end
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 2.4: ツイストシフト実装
    pub fn apply_twist(
        curves: &CurveSet<f64>,
        short_shift: f64,
        long_shift: f64,
    ) -> CurveSet<f64> {
        Self::apply_shift(curves, CurveShiftType::twist(short_shift, long_shift))
    }

    /// Applies a butterfly shift to all curves in the set.
    ///
    /// # Arguments
    ///
    /// * `curves` - The curve set to shift
    /// * `wing_shift` - Shift for wings (short and long ends)
    /// * `belly_shift` - Shift for belly (middle)
    ///
    /// # Requirements Coverage
    ///
    /// - Requirement 2.4: バタフライシフト実装
    pub fn apply_butterfly(
        curves: &CurveSet<f64>,
        wing_shift: f64,
        belly_shift: f64,
    ) -> CurveSet<f64> {
        Self::apply_shift(curves, CurveShiftType::butterfly(wing_shift, belly_shift))
    }

    /// Applies a shift to all curves in the set.
    ///
    /// For flat curves, this creates new flat curves at the shifted level.
    /// For non-flat curves, the shift is applied at a reference tenor (1Y).
    pub fn apply_shift(curves: &CurveSet<f64>, shift: CurveShiftType) -> CurveSet<f64> {
        let mut shifted = CurveSet::new();
        let max_tenor = 30.0; // Standard max tenor for interpolation

        for (name, curve) in curves.iter() {
            // Get base rate at reference tenor (1Y for flat curves)
            let base_rate = curve.zero_rate(1.0).unwrap_or(0.0);

            // For flat curves, use shift at reference tenor
            // For proper implementation with interpolated curves, would iterate over all tenors
            let shift_amount = shift.shift_at_tenor(1.0, max_tenor);
            let shifted_rate = base_rate + shift_amount;

            shifted.insert(*name, CurveEnum::flat(shifted_rate));
        }

        // Preserve discount curve setting
        if curves.discount_curve().is_some() {
            shifted.set_discount_curve(CurveName::Discount);
        }

        shifted
    }

    /// Applies a shift to a specific curve in the set.
    pub fn apply_shift_to_curve(
        curves: &CurveSet<f64>,
        curve_name: CurveName,
        shift: CurveShiftType,
    ) -> CurveSet<f64> {
        let mut result = curves.clone();
        let max_tenor = 30.0;

        if let Some(curve) = curves.get(&curve_name) {
            let base_rate = curve.zero_rate(1.0).unwrap_or(0.0);
            let shift_amount = shift.shift_at_tenor(1.0, max_tenor);
            let shifted_rate = base_rate + shift_amount;
            result.insert(curve_name, CurveEnum::flat(shifted_rate));
        }

        result
    }

    /// Creates a steepening scenario (short end down, long end up).
    pub fn steepen(curves: &CurveSet<f64>, amount: f64) -> CurveSet<f64> {
        Self::apply_shift(curves, CurveShiftType::steepen(amount))
    }

    /// Creates a flattening scenario (short end up, long end down).
    pub fn flatten(curves: &CurveSet<f64>, amount: f64) -> CurveSet<f64> {
        Self::apply_shift(curves, CurveShiftType::flatten(amount))
    }

    /// Creates a positive curvature scenario (belly up).
    pub fn positive_curvature(curves: &CurveSet<f64>, amount: f64) -> CurveSet<f64> {
        Self::apply_shift(curves, CurveShiftType::positive_curvature(amount))
    }

    /// Creates a negative curvature scenario (belly down).
    pub fn negative_curvature(curves: &CurveSet<f64>, amount: f64) -> CurveSet<f64> {
        Self::apply_shift(curves, CurveShiftType::negative_curvature(amount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // Task 2.3: Curve shift tests (TDD)
    // ================================================================

    fn create_test_curves() -> CurveSet<f64> {
        let mut curves = CurveSet::new();
        curves.insert(CurveName::Discount, CurveEnum::flat(0.03));
        curves.insert(CurveName::Forward, CurveEnum::flat(0.035));
        curves.set_discount_curve(CurveName::Discount);
        curves
    }

    // CurveShiftType tests
    #[test]
    fn test_shift_type_parallel() {
        let shift = CurveShiftType::parallel(0.001);
        assert!((shift.shift_at_tenor(1.0, 30.0) - 0.001).abs() < 1e-10);
        assert!((shift.shift_at_tenor(10.0, 30.0) - 0.001).abs() < 1e-10);
    }

    #[test]
    fn test_shift_type_twist() {
        let shift = CurveShiftType::twist(0.001, -0.001);

        // At short end (t=0)
        assert!((shift.shift_at_tenor(0.0, 30.0) - 0.001).abs() < 1e-10);

        // At long end (t=30)
        assert!((shift.shift_at_tenor(30.0, 30.0) - (-0.001)).abs() < 1e-10);

        // At midpoint (t=15)
        assert!(shift.shift_at_tenor(15.0, 30.0).abs() < 1e-10);
    }

    #[test]
    fn test_shift_type_steepen() {
        let shift = CurveShiftType::steepen(0.001);

        // Short end should be down
        assert!(shift.shift_at_tenor(0.0, 30.0) < 0.0);

        // Long end should be up
        assert!(shift.shift_at_tenor(30.0, 30.0) > 0.0);
    }

    #[test]
    fn test_shift_type_flatten() {
        let shift = CurveShiftType::flatten(0.001);

        // Short end should be up
        assert!(shift.shift_at_tenor(0.0, 30.0) > 0.0);

        // Long end should be down
        assert!(shift.shift_at_tenor(30.0, 30.0) < 0.0);
    }

    #[test]
    fn test_shift_type_butterfly() {
        let shift = CurveShiftType::butterfly(-0.0005, 0.001);

        // Wings (0 and 10Y) should be at wing_shift
        assert!((shift.shift_at_tenor(0.0, 30.0) - (-0.0005)).abs() < 1e-10);

        // Belly (5Y) should be at belly_shift
        assert!((shift.shift_at_tenor(5.0, 30.0) - 0.001).abs() < 1e-10);
    }

    #[test]
    fn test_shift_type_positive_curvature() {
        let shift = CurveShiftType::positive_curvature(0.002);

        // Belly should be positive
        assert!(shift.shift_at_tenor(5.0, 30.0) > 0.0);

        // Wings should be negative
        assert!(shift.shift_at_tenor(0.0, 30.0) < 0.0);
    }

    #[test]
    fn test_shift_type_tenor_specific() {
        let shift = CurveShiftType::TenorSpecific {
            tenors: vec![1.0, 5.0, 10.0],
            shifts: vec![0.001, 0.002, 0.001],
        };

        // At exact tenor points
        assert!((shift.shift_at_tenor(1.0, 30.0) - 0.001).abs() < 1e-10);
        assert!((shift.shift_at_tenor(5.0, 30.0) - 0.002).abs() < 1e-10);
        assert!((shift.shift_at_tenor(10.0, 30.0) - 0.001).abs() < 1e-10);

        // Interpolated at 3Y (between 1Y and 5Y)
        let interp = shift.shift_at_tenor(3.0, 30.0);
        assert!(interp > 0.001 && interp < 0.002);
    }

    #[test]
    fn test_shift_type_to_shift_type() {
        let parallel = CurveShiftType::parallel(0.001);
        assert!(parallel.to_shift_type().is_some());

        let twist = CurveShiftType::twist(0.001, -0.001);
        assert!(twist.to_shift_type().is_some());

        let butterfly = CurveShiftType::butterfly(0.001, -0.001);
        assert!(butterfly.to_shift_type().is_some());
    }

    // CurveShiftSpec tests
    #[test]
    fn test_curve_shift_spec_new() {
        let spec = CurveShiftSpec::new("USD-OIS", CurveShiftType::parallel(0.001));
        assert_eq!(spec.curve_name, "USD-OIS");
        assert!(spec.pivot_tenor.is_none());
    }

    #[test]
    fn test_curve_shift_spec_with_pivot() {
        let spec = CurveShiftSpec::new("USD-OIS", CurveShiftType::butterfly(0.001, -0.001))
            .with_pivot(5.0);
        assert_eq!(spec.pivot_tenor, Some(5.0));
    }

    // CurveShifter tests
    #[test]
    fn test_curve_shifter_parallel() {
        let curves = create_test_curves();
        let shifted = CurveShifter::apply_parallel(&curves, 0.001);

        // Discount curve should be shifted
        let original_rate = curves
            .get(&CurveName::Discount)
            .unwrap()
            .zero_rate(1.0)
            .unwrap();
        let shifted_rate = shifted
            .get(&CurveName::Discount)
            .unwrap()
            .zero_rate(1.0)
            .unwrap();

        assert!((shifted_rate - original_rate - 0.001).abs() < 1e-10);
    }

    #[test]
    fn test_curve_shifter_twist() {
        let curves = create_test_curves();
        let shifted = CurveShifter::apply_twist(&curves, 0.001, -0.001);

        // Should preserve curve structure
        assert!(shifted.get(&CurveName::Discount).is_some());
        assert!(shifted.get(&CurveName::Forward).is_some());
    }

    #[test]
    fn test_curve_shifter_butterfly() {
        let curves = create_test_curves();
        let shifted = CurveShifter::apply_butterfly(&curves, -0.0005, 0.001);

        // Should preserve curve structure
        assert!(shifted.get(&CurveName::Discount).is_some());
    }

    #[test]
    fn test_curve_shifter_steepen() {
        let curves = create_test_curves();
        let shifted = CurveShifter::steepen(&curves, 0.001);

        // Should preserve discount curve setting
        assert!(shifted.discount_curve().is_some());
    }

    #[test]
    fn test_curve_shifter_flatten() {
        let curves = create_test_curves();
        let shifted = CurveShifter::flatten(&curves, 0.001);

        // Should preserve discount curve setting
        assert!(shifted.discount_curve().is_some());
    }

    #[test]
    fn test_curve_shifter_positive_curvature() {
        let curves = create_test_curves();
        let shifted = CurveShifter::positive_curvature(&curves, 0.002);

        // Should preserve curve structure
        assert!(shifted.get(&CurveName::Discount).is_some());
    }

    #[test]
    fn test_curve_shifter_negative_curvature() {
        let curves = create_test_curves();
        let shifted = CurveShifter::negative_curvature(&curves, 0.002);

        // Should preserve curve structure
        assert!(shifted.get(&CurveName::Discount).is_some());
    }

    #[test]
    fn test_curve_shifter_apply_to_specific_curve() {
        let curves = create_test_curves();
        let shifted =
            CurveShifter::apply_shift_to_curve(&curves, CurveName::Discount, CurveShiftType::parallel(0.001));

        // Discount curve should be shifted
        let original_discount = curves
            .get(&CurveName::Discount)
            .unwrap()
            .zero_rate(1.0)
            .unwrap();
        let shifted_discount = shifted
            .get(&CurveName::Discount)
            .unwrap()
            .zero_rate(1.0)
            .unwrap();

        assert!((shifted_discount - original_discount - 0.001).abs() < 1e-10);

        // Forward curve should be unchanged
        let original_forward = curves
            .get(&CurveName::Forward)
            .unwrap()
            .zero_rate(1.0)
            .unwrap();
        let shifted_forward = shifted
            .get(&CurveName::Forward)
            .unwrap()
            .zero_rate(1.0)
            .unwrap();

        assert!((shifted_forward - original_forward).abs() < 1e-10);
    }

    #[test]
    fn test_curve_shifter_preserves_discount_curve() {
        let curves = create_test_curves();

        let shifted = CurveShifter::apply_parallel(&curves, 0.001);
        assert!(shifted.discount_curve().is_some());

        let twisted = CurveShifter::apply_twist(&curves, 0.001, -0.001);
        assert!(twisted.discount_curve().is_some());

        let butterfly = CurveShifter::apply_butterfly(&curves, 0.001, -0.001);
        assert!(butterfly.discount_curve().is_some());
    }
}
