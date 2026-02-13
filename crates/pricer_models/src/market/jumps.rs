//! Utilities for converting JumpPillar definitions to curve-compatible format.

use infra_domain::{
    market::definition::JumpPillar,
    time::{Date, DayCounter},
};

/// A jump entry (time in years, cumulative log-DF offset) for bootstrapped
/// curves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JumpEntry {
    /// Time of the jump in years from valuation date.
    pub time: f64,
    /// Cumulative offset in log(df) space.
    ///
    /// Positive values decrease the discount factor (rate hike).
    pub cumulative_offset: f64,
}

impl JumpEntry {
    /// Creates a new jump entry.
    #[must_use]
    pub fn new(time: f64, cumulative_offset: f64) -> Self {
        Self {
            time,
            cumulative_offset,
        }
    }

    /// Returns the time of the jump.
    #[must_use]
    pub fn time(&self) -> f64 { self.time }

    /// Returns the cumulative offset.
    #[must_use]
    pub fn cumulative_offset(&self) -> f64 { self.cumulative_offset }

    /// Converts to a tuple (time, cumulative_offset).
    #[must_use]
    pub fn to_tuple(&self) -> (f64, f64) { (self.time, self.cumulative_offset) }
}

/// Converts JumpPillars to sorted JumpEntry with cumulative offsets.
#[must_use]
pub fn convert_jump_pillars(
    pillars: &[JumpPillar],
    valuation_date: Date,
    day_counter: DayCounter,
) -> Vec<JumpEntry> {
    if pillars.is_empty() {
        return Vec::new();
    }

    // Phase 1: Expand pillars into (time, individual_offset) entries.
    // For permanent jumps: one entry at jump_date.
    // For turn events (has end_date): two entries — spike up at jump_date,
    // spike down (revert) at end_date.
    let mut entries: Vec<(f64, f64)> = Vec::new();

    for p in pillars {
        let time = day_counter.year_fraction(valuation_date, p.jump_date());
        if time <= 0.0 {
            continue; // Skip past events
        }

        // Convert weighted bps to log-space offset: -bps / 10000
        // Negative because rate hike (positive bps) decreases discount factor
        let jump_offset = -p.weighted_jump_bps() / 10_000.0;
        entries.push((time, jump_offset));

        // Turn events: emit a reverting entry at end_date
        if let Some(end_date) = p.end_date() {
            let end_time = day_counter.year_fraction(valuation_date, end_date);
            if end_time > 0.0 {
                entries.push((end_time, -jump_offset));
            }
        }
    }

    // Phase 2: Sort by time
    entries.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Phase 3: Calculate cumulative offsets
    let mut cumulative = 0.0;
    entries
        .into_iter()
        .map(|(time, jump_offset)| {
            cumulative += jump_offset;
            JumpEntry::new(time, cumulative)
        })
        .collect()
}

/// Convenience wrapper returning `(time, cumulative_offset)` tuples.
#[must_use]
pub fn convert_jump_pillars_to_tuples(
    pillars: &[JumpPillar],
    valuation_date: Date,
    day_counter: DayCounter,
) -> Vec<(f64, f64)> {
    convert_jump_pillars(pillars, valuation_date, day_counter)
        .into_iter()
        .map(|e| e.to_tuple())
        .collect()
}

/// Finds the cumulative jump offset at time `t` (binary search).
#[must_use]
pub fn cumulative_offset_at(jumps: &[JumpEntry], t: f64) -> f64 {
    if jumps.is_empty() {
        return 0.0;
    }

    // Binary search for the last jump with time <= t
    match jumps
        .binary_search_by(|j| j.time.partial_cmp(&t).unwrap_or(std::cmp::Ordering::Equal))
    {
        Ok(idx) => jumps[idx].cumulative_offset,
        Err(idx) => {
            if idx == 0 {
                0.0
            } else {
                jumps[idx - 1].cumulative_offset
            }
        }
    }
}

/// Finds the cumulative jump offset just before time `t` (left limit).
#[must_use]
pub fn cumulative_offset_before(jumps: &[JumpEntry], t: f64) -> f64 {
    if jumps.is_empty() {
        return 0.0;
    }

    // Find the last jump with time < t
    let idx = jumps.partition_point(|j| j.time < t);
    if idx == 0 {
        0.0
    } else {
        jumps[idx - 1].cumulative_offset
    }
}

/// Checks if there is a jump within `tolerance` of time `t`.
#[must_use]
pub fn has_jump_at(jumps: &[JumpEntry], t: f64, tolerance: f64) -> bool {
    jumps.iter().any(|j| (j.time - t).abs() < tolerance)
}

/// Builds a daily forward-rate-shift grid from jump pillars.
///
/// Produces ramp offsets: `offset(t) = -Sum s_i * (t - t_i)` for `t_i <=
/// t`, yielding `df(t) = base_df(t) * exp(offset(t))`.
#[must_use]
pub fn build_forward_rate_shift_grid(
    pillars: &[JumpPillar],
    valuation_date: Date,
    day_counter: DayCounter,
    max_time: f64,
) -> Vec<(f64, f64)> {
    if pillars.is_empty() {
        return Vec::new();
    }

    // Phase 1: Convert pillars to (time, delta_rate) shifts.
    // Permanent jumps: one shift at jump_date.
    // Turn events: spike up at jump_date, spike down (revert) at end_date.
    let mut rate_shifts: Vec<(f64, f64)> = Vec::new();

    for p in pillars {
        let time = day_counter.year_fraction(valuation_date, p.jump_date());
        if time <= 0.0 {
            continue; // Skip past events
        }

        // Convert confidence-weighted bps to decimal rate
        let delta_rate = p.weighted_jump_bps() / 10_000.0;
        rate_shifts.push((time, delta_rate));

        // Turn events: forward rate reverts at end_date
        if let Some(end_date) = p.end_date() {
            let end_time = day_counter.year_fraction(valuation_date, end_date);
            if end_time > 0.0 {
                rate_shifts.push((end_time, -delta_rate));
            }
        }
    }

    rate_shifts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Phase 2: Build daily grid with ramp offsets.
    // offset(t) = -sum(s_i * (t - t_i)) for all t_i <= t
    //
    // Grid times MUST use year_fraction_from_days (d / N) to match the
    // query path in cumulative_offset_at. Using i * (1/N) would introduce
    // IEEE 754 rounding mismatches that shift forward rates by ±jump_bps
    // on affected days.
    let grid_count = (max_time / day_counter.year_fraction_from_days(1)).ceil() as usize + 2;

    (0..grid_count)
        .map(|i| {
            let t = day_counter.year_fraction_from_days(i as i64);
            let offset: f64 = rate_shifts
                .iter()
                .take_while(|(t_j, _)| *t_j <= t)
                .map(|(t_j, shift)| -shift * (t - t_j))
                .sum();
            (t, offset)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_valuation_date() -> Date { Date::from_ymd(2024, 1, 1).unwrap() }

    #[test]
    fn test_convert_empty_pillars() {
        let entries =
            convert_jump_pillars(&[], test_valuation_date(), DayCounter::Actual365Fixed);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_convert_single_pillar() {
        let valuation = test_valuation_date();
        let jump_date = Date::from_ymd(2024, 3, 20).unwrap();
        let pillars = vec![JumpPillar::new(jump_date, 25.0, 0.8)];

        let entries = convert_jump_pillars(&pillars, valuation, DayCounter::Actual365Fixed);

        assert_eq!(entries.len(), 1);
        let expected_time = DayCounter::Actual365Fixed.year_fraction(valuation, jump_date);
        assert!((entries[0].time - expected_time).abs() < 1e-10);
        // Weighted jump: 25 * 0.8 = 20 bps → -0.002 (negative for rate hike)
        assert!((entries[0].cumulative_offset - (-0.002)).abs() < 1e-10);
    }

    #[test]
    fn test_convert_multiple_pillars_sorted() {
        let valuation = test_valuation_date();
        let pillars = vec![
            JumpPillar::new(Date::from_ymd(2024, 6, 12).unwrap(), -25.0, 0.6),
            JumpPillar::new(Date::from_ymd(2024, 3, 20).unwrap(), 25.0, 0.8),
        ];

        let entries = convert_jump_pillars(&pillars, valuation, DayCounter::Actual365Fixed);

        assert_eq!(entries.len(), 2);
        assert!(entries[0].time < entries[1].time);
        assert!((entries[0].cumulative_offset - (-0.002)).abs() < 1e-10);
        assert!((entries[1].cumulative_offset - (-0.0005)).abs() < 1e-10);
    }

    #[test]
    fn test_convert_filters_past_jumps() {
        let valuation = Date::from_ymd(2024, 6, 1).unwrap();
        let pillars = vec![
            JumpPillar::new(Date::from_ymd(2024, 3, 20).unwrap(), 25.0, 0.8), // Past
            JumpPillar::new(Date::from_ymd(2024, 9, 18).unwrap(), 50.0, 0.7), // Future
        ];

        let entries = convert_jump_pillars(&pillars, valuation, DayCounter::Actual365Fixed);

        assert_eq!(entries.len(), 1);
        assert!((entries[0].cumulative_offset - (-0.0035)).abs() < 1e-10);
    }

    #[test]
    fn test_cumulative_offset_at() {
        let entries = vec![
            JumpEntry::new(0.25, 0.002),
            JumpEntry::new(0.50, 0.0035),
            JumpEntry::new(0.75, 0.006),
        ];

        assert_eq!(cumulative_offset_at(&entries, 0.1), 0.0);
        assert!((cumulative_offset_at(&entries, 0.25) - 0.002).abs() < 1e-10);
        assert!((cumulative_offset_at(&entries, 0.4) - 0.002).abs() < 1e-10);
        assert!((cumulative_offset_at(&entries, 0.50) - 0.0035).abs() < 1e-10);
        assert!((cumulative_offset_at(&entries, 1.0) - 0.006).abs() < 1e-10);
    }

    #[test]
    fn test_cumulative_offset_at_empty() {
        let entries: Vec<JumpEntry> = vec![];
        assert_eq!(cumulative_offset_at(&entries, 0.5), 0.0);
    }

    #[test]
    fn test_cumulative_offset_before() {
        let entries = vec![JumpEntry::new(0.25, 0.002), JumpEntry::new(0.50, 0.0035)];

        assert_eq!(cumulative_offset_before(&entries, 0.1), 0.0);
        assert_eq!(cumulative_offset_before(&entries, 0.25), 0.0);
        assert!((cumulative_offset_before(&entries, 0.4) - 0.002).abs() < 1e-10);
        assert!((cumulative_offset_before(&entries, 0.50) - 0.002).abs() < 1e-10);
    }

    #[test]
    fn test_has_jump_at() {
        let entries = vec![JumpEntry::new(0.25, 0.002), JumpEntry::new(0.50, 0.0035)];

        assert!(has_jump_at(&entries, 0.25, 1e-10));
        assert!(has_jump_at(&entries, 0.50, 1e-10));
        assert!(!has_jump_at(&entries, 0.30, 1e-10));
        assert!(!has_jump_at(&entries, 0.0, 1e-10));
    }

    #[test]
    fn test_jump_entry_methods() {
        let entry = JumpEntry::new(0.25, 0.002);

        assert_eq!(entry.time(), 0.25);
        assert_eq!(entry.cumulative_offset(), 0.002);
        assert_eq!(entry.to_tuple(), (0.25, 0.002));
    }

    #[test]
    fn test_convert_to_tuples() {
        let valuation = test_valuation_date();
        let pillars = vec![JumpPillar::new(
            Date::from_ymd(2024, 3, 20).unwrap(),
            25.0,
            0.8,
        )];

        let tuples =
            convert_jump_pillars_to_tuples(&pillars, valuation, DayCounter::Actual365Fixed);

        assert_eq!(tuples.len(), 1);
        assert!((tuples[0].1 - (-0.002)).abs() < 1e-10);
    }

    #[test]
    fn test_convert_single_turn_pillar() {
        let valuation = test_valuation_date();
        let turn_start = Date::from_ymd(2024, 12, 31).unwrap();
        let turn_end = Date::from_ymd(2025, 1, 2).unwrap();

        let pillars = vec![JumpPillar::new(turn_start, 12.5, 1.0).with_end_date(turn_end)];

        let entries = convert_jump_pillars(&pillars, valuation, DayCounter::Actual365Fixed);

        assert_eq!(entries.len(), 2);

        let t_start = DayCounter::Actual365Fixed.year_fraction(valuation, turn_start);
        let t_end = DayCounter::Actual365Fixed.year_fraction(valuation, turn_end);

        assert!((entries[0].time - t_start).abs() < 1e-10);
        assert!((entries[0].cumulative_offset - (-0.00125)).abs() < 1e-10);

        assert!((entries[1].time - t_end).abs() < 1e-10);
        assert!(entries[1].cumulative_offset.abs() < 1e-10);
    }

    #[test]
    fn test_convert_mixed_jump_and_turn() {
        let valuation = test_valuation_date();

        let jump = JumpPillar::new(Date::from_ymd(2024, 3, 20).unwrap(), 25.0, 1.0);

        let turn = JumpPillar::new(Date::from_ymd(2024, 12, 31).unwrap(), 12.5, 1.0)
            .with_end_date(Date::from_ymd(2025, 1, 2).unwrap());

        let pillars = vec![jump, turn];
        let entries = convert_jump_pillars(&pillars, valuation, DayCounter::Actual365Fixed);

        assert_eq!(entries.len(), 3);

        assert!((entries[0].cumulative_offset - (-0.0025)).abs() < 1e-10);
        assert!((entries[1].cumulative_offset - (-0.00375)).abs() < 1e-10);
        assert!((entries[2].cumulative_offset - (-0.0025)).abs() < 1e-10);
    }

    #[test]
    fn test_convert_turn_offset_reverts() {
        let valuation = test_valuation_date();
        let turn_start = Date::from_ymd(2024, 6, 30).unwrap();
        let turn_end = Date::from_ymd(2024, 7, 1).unwrap();

        let pillars = vec![JumpPillar::new(turn_start, 5.0, 1.0).with_end_date(turn_end)];

        let entries = convert_jump_pillars(&pillars, valuation, DayCounter::Actual365Fixed);
        assert_eq!(entries.len(), 2);

        let t_start = entries[0].time;
        assert_eq!(cumulative_offset_at(&entries, t_start - 0.001), 0.0);
        assert!((cumulative_offset_at(&entries, t_start) - (-0.0005)).abs() < 1e-10);

        let t_end = entries[1].time;
        assert!(cumulative_offset_at(&entries, t_end).abs() < 1e-10);
        assert!(cumulative_offset_at(&entries, t_end + 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_permanent_jump_no_revert_entry() {
        let valuation = test_valuation_date();
        let pillars = vec![JumpPillar::new(
            Date::from_ymd(2024, 3, 20).unwrap(),
            25.0,
            1.0,
        )];

        let entries = convert_jump_pillars(&pillars, valuation, DayCounter::Actual365Fixed);

        assert_eq!(entries.len(), 1);
        assert!((entries[0].cumulative_offset - (-0.0025)).abs() < 1e-10);
    }
}
