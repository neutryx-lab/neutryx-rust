//! Jump pillar utilities for curve construction.
//!
//! This module provides utilities for converting [`JumpPillar`] definitions
//! from `infra_domain` into time-based representations suitable for curve
//! interpolation and bootstrapping.
//!
//! # Design Notes
//!
//! Jump offsets are applied in log-space to preserve the multiplicative nature
//! of discount factors. A rate jump of `r` basis points over time period `dt`
//! results in a cumulative log-discount factor offset of:
//!
//! ```text
//! offset = -r * dt / 10000
//! ```
//!
//! For instantaneous jumps (typical central bank meetings), `dt` is effectively
//! zero, so we apply the jump as a discrete shift to the log discount factor.
//!
//! # Examples
//!
//! ```
//! use pricer_models::jump::{convert_jump_pillars_to_times, JumpTime};
//! use infra_domain::market::definition::JumpPillar;
//! use infra_domain::time::{Date, DayCounter};
//!
//! let valuation_date = Date::from_ymd(2024, 1, 1).unwrap();
//! let day_counter = DayCounter::Actual365Fixed;
//!
//! let pillars = vec![
//!     JumpPillar::new(Date::from_ymd(2024, 3, 20).unwrap(), 25.0, 0.85),
//!     JumpPillar::new(Date::from_ymd(2024, 6, 12).unwrap(), 25.0, 0.70),
//! ];
//!
//! let jump_times = convert_jump_pillars_to_times(&pillars, valuation_date, &day_counter);
//!
//! assert_eq!(jump_times.len(), 2);
//! assert!(jump_times[0].time < jump_times[1].time);
//! ```

use infra_domain::{
    market::definition::JumpPillar,
    time::{Date, DayCounter},
};
use num_traits::Float;

/// A jump event converted to time coordinates.
///
/// Contains the time (in year fractions) and the cumulative log-space offset
/// to apply to discount factors after this point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JumpTime<T> {
    /// Time in year fractions from valuation date.
    pub time: T,
    /// Cumulative jump offset in log-space.
    ///
    /// This is the sum of all jump effects up to and including this point.
    /// For a discount factor DF, the adjusted DF at time t is:
    /// `DF_adjusted = DF * exp(cumulative_offset)`
    pub cumulative_offset: T,
}

impl<T: Float> JumpTime<T> {
    /// Creates a new JumpTime.
    pub fn new(time: T, cumulative_offset: T) -> Self {
        Self {
            time,
            cumulative_offset,
        }
    }
}

/// Converts a slice of JumpPillars to time-based jump representations.
///
/// This function transforms date-based JumpPillar definitions into time
/// coordinates (year fractions) suitable for curve interpolation. The
/// cumulative offsets are calculated in log-space to be applied to discount
/// factors.
///
/// # Arguments
///
/// * `pillars` - Slice of JumpPillar definitions
/// * `valuation_date` - The curve's valuation date
/// * `day_counter` - Day count convention for year fraction calculation
///
/// # Returns
///
/// A vector of `JumpTime<f64>` sorted by time, with cumulative offsets.
///
/// # Details
///
/// - Pillars with jump dates before or on the valuation date are excluded
/// - Results are sorted by time (ascending)
/// - Cumulative offsets use weighted jump (expected_jump_bps * confidence)
/// - The offset is converted from basis points to log-space: `offset =
///   -weighted_jump_bps / 10000`
///
/// # Examples
///
/// ```
/// use pricer_models::jump::convert_jump_pillars_to_times;
/// use infra_domain::market::definition::JumpPillar;
/// use infra_domain::time::{Date, DayCounter};
///
/// let valuation_date = Date::from_ymd(2024, 1, 1).unwrap();
/// let day_counter = DayCounter::Actual365Fixed;
///
/// let pillars = vec![
///     JumpPillar::new(Date::from_ymd(2024, 3, 20).unwrap(), 25.0, 1.0),
/// ];
///
/// let jumps = convert_jump_pillars_to_times(&pillars, valuation_date, &day_counter);
///
/// assert_eq!(jumps.len(), 1);
/// // 25bp = 0.0025, offset = -0.0025
/// assert!((jumps[0].cumulative_offset - (-0.0025)).abs() < 1e-10);
/// ```
pub fn convert_jump_pillars_to_times(
    pillars: &[JumpPillar],
    valuation_date: Date,
    day_counter: &DayCounter,
) -> Vec<JumpTime<f64>> {
    convert_jump_pillars_to_times_generic::<f64>(pillars, valuation_date, day_counter)
}

/// Generic version of `convert_jump_pillars_to_times` for any Float type.
///
/// # Type Parameters
///
/// * `T` - Floating point type (e.g., f64, f32, or AD-compatible types)
pub fn convert_jump_pillars_to_times_generic<T: Float>(
    pillars: &[JumpPillar],
    valuation_date: Date,
    day_counter: &DayCounter,
) -> Vec<JumpTime<T>> {
    if pillars.is_empty() {
        return Vec::new();
    }

    // Phase 1: Expand pillars into (time, individual_offset) entries.
    // For permanent jumps: one entry at jump_date.
    // For turn events (has end_date): two entries — spike up at jump_date,
    // spike down (revert) at end_date.
    let mut time_offsets: Vec<(T, T)> = Vec::new();

    for p in pillars.iter().filter(|p| p.jump_date() > valuation_date) {
        let time = T::from(day_counter.year_fraction(valuation_date, p.jump_date())).unwrap();
        // Convert weighted jump from bps to decimal rate offset
        // The offset is negative because a rate increase reduces discount factors
        let single_offset = T::from(-p.weighted_jump_rate()).unwrap();
        time_offsets.push((time, single_offset));

        // Turn events: emit a reverting entry at end_date
        if let Some(end_date) = p.end_date() {
            if end_date > valuation_date {
                let end_time =
                    T::from(day_counter.year_fraction(valuation_date, end_date)).unwrap();
                time_offsets.push((end_time, T::zero() - single_offset));
            }
        }
    }

    // Phase 2: Sort by time
    time_offsets.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Phase 3: Calculate cumulative offsets
    let mut cumulative = T::zero();
    time_offsets
        .into_iter()
        .map(|(time, offset)| {
            cumulative = cumulative + offset;
            JumpTime::new(time, cumulative)
        })
        .collect()
}

/// Finds the cumulative jump offset at a given time.
///
/// Uses binary search for O(log n) lookup.
///
/// # Arguments
///
/// * `jumps` - Sorted slice of JumpTime entries
/// * `t` - Time to query
///
/// # Returns
///
/// The cumulative offset at time `t`. Returns zero if there are no jumps
/// before or at time `t`.
///
/// # Examples
///
/// ```
/// use pricer_models::jump::{JumpTime, effective_jump_offset_at};
///
/// let jumps: Vec<JumpTime<f64>> = vec![
///     JumpTime::new(0.25, -0.0025),
///     JumpTime::new(0.50, -0.005),
/// ];
///
/// // Before first jump
/// assert!((effective_jump_offset_at(&jumps, 0.1_f64) - 0.0_f64).abs() < 1e-10);
///
/// // After first jump, before second
/// assert!((effective_jump_offset_at(&jumps, 0.3_f64) - (-0.0025_f64)).abs() < 1e-10);
///
/// // After second jump
/// assert!((effective_jump_offset_at(&jumps, 0.6_f64) - (-0.005_f64)).abs() < 1e-10);
/// ```
pub fn effective_jump_offset_at<T: Float>(jumps: &[JumpTime<T>], t: T) -> T {
    if jumps.is_empty() {
        return T::zero();
    }

    // Binary search for the last jump at or before time t
    let idx = jumps.partition_point(|j| j.time <= t);

    if idx == 0 {
        T::zero()
    } else {
        jumps[idx - 1].cumulative_offset
    }
}

/// Returns the jump offset specifically at time `t` (right limit minus left
/// limit).
///
/// This is useful for determining the discontinuity magnitude at a jump date.
///
/// # Arguments
///
/// * `jumps` - Sorted slice of JumpTime entries
/// * `t` - Time to query
/// * `tolerance` - Time tolerance for matching jump dates
///
/// # Returns
///
/// The instantaneous jump offset at time `t`, or zero if no jump exists there.
pub fn jump_offset_at<T: Float>(jumps: &[JumpTime<T>], t: T, tolerance: T) -> T {
    for (i, jump) in jumps.iter().enumerate() {
        if (jump.time - t).abs() <= tolerance {
            // Found a jump at this time
            let prev_offset = if i == 0 {
                T::zero()
            } else {
                jumps[i - 1].cumulative_offset
            };
            return jump.cumulative_offset - prev_offset;
        }
    }
    T::zero()
}

/// Checks if there is a jump at the given time.
///
/// # Arguments
///
/// * `jumps` - Sorted slice of JumpTime entries
/// * `t` - Time to check
/// * `tolerance` - Time tolerance for matching (typically 1e-10)
pub fn has_jump_at<T: Float>(jumps: &[JumpTime<T>], t: T, tolerance: T) -> bool {
    jumps.iter().any(|j| (j.time - t).abs() <= tolerance)
}

/// Converts a vector of (time, cumulative_offset) tuples to JumpTime vector.
///
/// This is a convenience function for creating jump data from raw tuples.
pub fn from_tuples<T: Float>(data: Vec<(T, T)>) -> Vec<JumpTime<T>> {
    data.into_iter()
        .map(|(time, offset)| JumpTime::new(time, offset))
        .collect()
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn make_date(year: i32, month: u32, day: u32) -> Date {
        Date::from_ymd(year, month, day).unwrap()
    }

    #[test]
    fn test_convert_empty_pillars() {
        let pillars: Vec<JumpPillar> = vec![];
        let valuation = make_date(2024, 1, 1);
        let dc = DayCounter::Actual365Fixed;

        let result = convert_jump_pillars_to_times(&pillars, valuation, &dc);
        assert!(result.is_empty());
    }

    #[test]
    fn test_convert_single_pillar() {
        let valuation = make_date(2024, 1, 1);
        let jump_date = make_date(2024, 3, 20); // 79 days from valuation
        let dc = DayCounter::Actual365Fixed;

        let pillars = vec![JumpPillar::new(jump_date, 25.0, 1.0)];

        let result = convert_jump_pillars_to_times(&pillars, valuation, &dc);

        assert_eq!(result.len(), 1);
        // 79 days / 365 ≈ 0.2164
        assert_relative_eq!(result[0].time, 79.0 / 365.0, epsilon = 1e-6);
        // 25bp with 100% confidence = -0.0025 offset
        assert_relative_eq!(result[0].cumulative_offset, -0.0025, epsilon = 1e-10);
    }

    #[test]
    fn test_convert_multiple_pillars_sorted() {
        let valuation = make_date(2024, 1, 1);
        let dc = DayCounter::Actual365Fixed;

        // Create pillars out of order
        let pillars = vec![
            JumpPillar::new(make_date(2024, 6, 12), 25.0, 0.80), // Later
            JumpPillar::new(make_date(2024, 3, 20), 25.0, 1.00), // Earlier
        ];

        let result = convert_jump_pillars_to_times(&pillars, valuation, &dc);

        assert_eq!(result.len(), 2);
        // Should be sorted by time
        assert!(result[0].time < result[1].time);

        // First jump: 25bp * 100% = -0.0025
        assert_relative_eq!(result[0].cumulative_offset, -0.0025, epsilon = 1e-10);

        // Second jump: cumulative = -0.0025 + (-25bp * 80%) = -0.0025 + (-0.002) =
        // -0.0045
        assert_relative_eq!(result[1].cumulative_offset, -0.0045, epsilon = 1e-10);
    }

    #[test]
    fn test_convert_filters_past_dates() {
        let valuation = make_date(2024, 6, 1);
        let dc = DayCounter::Actual365Fixed;

        let pillars = vec![
            JumpPillar::new(make_date(2024, 3, 20), 25.0, 1.00), // Before valuation
            JumpPillar::new(make_date(2024, 6, 1), 25.0, 0.90),  // On valuation
            JumpPillar::new(make_date(2024, 9, 18), 25.0, 0.80), // After valuation
        ];

        let result = convert_jump_pillars_to_times(&pillars, valuation, &dc);

        // Only the future jump should be included
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_convert_weighted_jump() {
        let valuation = make_date(2024, 1, 1);
        let dc = DayCounter::Actual365Fixed;

        // 50bp with 60% confidence = 30bp weighted
        let pillars = vec![JumpPillar::new(make_date(2024, 3, 20), 50.0, 0.60)];

        let result = convert_jump_pillars_to_times(&pillars, valuation, &dc);

        // 30bp = 0.003, offset = -0.003
        assert_relative_eq!(result[0].cumulative_offset, -0.003, epsilon = 1e-10);
    }

    #[test]
    fn test_effective_jump_offset_empty() {
        let jumps: Vec<JumpTime<f64>> = vec![];
        assert_relative_eq!(effective_jump_offset_at(&jumps, 0.5), 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_effective_jump_offset_before_first() {
        let jumps = vec![JumpTime::new(0.25, -0.0025), JumpTime::new(0.50, -0.005)];

        let offset = effective_jump_offset_at(&jumps, 0.1);
        assert_relative_eq!(offset, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_effective_jump_offset_after_first() {
        let jumps = vec![JumpTime::new(0.25, -0.0025), JumpTime::new(0.50, -0.005)];

        let offset = effective_jump_offset_at(&jumps, 0.3);
        assert_relative_eq!(offset, -0.0025, epsilon = 1e-10);
    }

    #[test]
    fn test_effective_jump_offset_after_all() {
        let jumps = vec![JumpTime::new(0.25, -0.0025), JumpTime::new(0.50, -0.005)];

        let offset = effective_jump_offset_at(&jumps, 1.0);
        assert_relative_eq!(offset, -0.005, epsilon = 1e-10);
    }

    #[test]
    fn test_effective_jump_offset_at_boundary() {
        let jumps = vec![JumpTime::new(0.25, -0.0025), JumpTime::new(0.50, -0.005)];

        // Exactly at jump time should include that jump
        let offset = effective_jump_offset_at(&jumps, 0.25);
        assert_relative_eq!(offset, -0.0025, epsilon = 1e-10);

        let offset2 = effective_jump_offset_at(&jumps, 0.50);
        assert_relative_eq!(offset2, -0.005, epsilon = 1e-10);
    }

    #[test]
    fn test_jump_offset_at() {
        let jumps = vec![JumpTime::new(0.25, -0.0025), JumpTime::new(0.50, -0.005)];

        // At first jump: offset = -0.0025 - 0 = -0.0025
        let offset = jump_offset_at(&jumps, 0.25, 1e-10);
        assert_relative_eq!(offset, -0.0025, epsilon = 1e-10);

        // At second jump: offset = -0.005 - (-0.0025) = -0.0025
        let offset2 = jump_offset_at(&jumps, 0.50, 1e-10);
        assert_relative_eq!(offset2, -0.0025, epsilon = 1e-10);

        // Not at a jump
        let offset3 = jump_offset_at(&jumps, 0.30, 1e-10);
        assert_relative_eq!(offset3, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_has_jump_at() {
        let jumps = vec![JumpTime::new(0.25, -0.0025), JumpTime::new(0.50, -0.005)];

        assert!(has_jump_at(&jumps, 0.25, 1e-10));
        assert!(has_jump_at(&jumps, 0.50, 1e-10));
        assert!(!has_jump_at(&jumps, 0.30, 1e-10));
    }

    #[test]
    fn test_from_tuples() {
        let data = vec![(0.25, -0.0025), (0.50, -0.005)];
        let jumps = from_tuples(data);

        assert_eq!(jumps.len(), 2);
        assert_relative_eq!(jumps[0].time, 0.25, epsilon = 1e-10);
        assert_relative_eq!(jumps[0].cumulative_offset, -0.0025, epsilon = 1e-10);
    }

    #[test]
    fn test_jump_time_clone_and_eq() {
        let j1 = JumpTime::new(0.25_f64, -0.0025);
        let j2 = j1;
        assert_eq!(j1, j2);
    }

    #[test]
    fn test_negative_jump_rate_cut() {
        let valuation = make_date(2024, 1, 1);
        let dc = DayCounter::Actual365Fixed;

        // Rate cut: -25bp
        let pillars = vec![JumpPillar::new(make_date(2024, 3, 20), -25.0, 1.0)];

        let result = convert_jump_pillars_to_times(&pillars, valuation, &dc);

        // Rate cut increases discount factors, so positive offset
        // -(-25bp) = +0.0025
        assert_relative_eq!(result[0].cumulative_offset, 0.0025, epsilon = 1e-10);
    }

    // =========================================================================
    // Turn event tests
    // =========================================================================

    #[test]
    fn test_convert_turn_pillar_generates_paired_entries() {
        let valuation = make_date(2024, 1, 1);
        let dc = DayCounter::Actual365Fixed;

        let turn_start = make_date(2024, 12, 31);
        let turn_end = make_date(2025, 1, 2);

        let pillars = vec![JumpPillar::new(turn_start, 12.5, 1.0).with_end_date(turn_end)];

        let result = convert_jump_pillars_to_times(&pillars, valuation, &dc);

        // Should produce 2 entries: spike up + spike down
        assert_eq!(result.len(), 2);

        // First: spike at turn_start
        // 12.5bp * 1.0 = 0.00125 rate → offset = -0.00125
        assert_relative_eq!(result[0].cumulative_offset, -0.00125, epsilon = 1e-10);

        // Second: revert at turn_end — cumulative returns to 0
        assert!(result[1].cumulative_offset.abs() < 1e-10);
    }

    #[test]
    fn test_convert_mixed_jump_and_turn_generic() {
        let valuation = make_date(2024, 1, 1);
        let dc = DayCounter::Actual365Fixed;

        // Permanent jump: 25bp on March 20
        let jump = JumpPillar::new(make_date(2024, 3, 20), 25.0, 1.0);

        // Turn: 10bp spike from Dec 31 to Jan 2
        let turn = JumpPillar::new(make_date(2024, 12, 31), 10.0, 1.0)
            .with_end_date(make_date(2025, 1, 2));

        let pillars = vec![jump, turn];
        let result = convert_jump_pillars_to_times(&pillars, valuation, &dc);

        // 3 entries: permanent + turn up + turn down
        assert_eq!(result.len(), 3);

        // Entry 0: permanent jump -0.0025
        assert_relative_eq!(result[0].cumulative_offset, -0.0025, epsilon = 1e-10);

        // Entry 1: turn spike (cumulative = -0.0025 + -0.001 = -0.0035)
        assert_relative_eq!(result[1].cumulative_offset, -0.0035, epsilon = 1e-10);

        // Entry 2: turn revert (cumulative = -0.0035 + 0.001 = -0.0025)
        assert_relative_eq!(result[2].cumulative_offset, -0.0025, epsilon = 1e-10);
    }

    #[test]
    fn test_turn_offset_reverts_via_effective_jump_offset() {
        let valuation = make_date(2024, 1, 1);
        let dc = DayCounter::Actual365Fixed;

        let turn_start = make_date(2024, 6, 30);
        let turn_end = make_date(2024, 7, 1);

        let pillars = vec![JumpPillar::new(turn_start, 5.0, 1.0).with_end_date(turn_end)];

        let result = convert_jump_pillars_to_times(&pillars, valuation, &dc);
        assert_eq!(result.len(), 2);

        // Before turn: offset 0
        assert_relative_eq!(
            effective_jump_offset_at(&result, result[0].time - 0.001),
            0.0,
            epsilon = 1e-10
        );

        // During turn: offset -0.0005
        assert_relative_eq!(
            effective_jump_offset_at(&result, result[0].time),
            -0.0005,
            epsilon = 1e-10
        );

        // After turn: offset reverts to 0
        assert!(effective_jump_offset_at(&result, result[1].time).abs() < 1e-10);
        assert!(effective_jump_offset_at(&result, result[1].time + 0.1).abs() < 1e-10);
    }
}
