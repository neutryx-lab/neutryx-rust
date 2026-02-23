//! Zero-rate shift/bump utilities for inflation curve risk analysis.
//!
//! Instead of directly bumping forward rates (which introduces compounding
//! distortion across maturities), we convert to zero rates first, apply the
//! shift, then restore:
//!
//! ```text
//! Z   = ln(rate / base_rate) / t
//! Z'  = Z + shift_val            (Absolute)
//!     or Z * (1 + shift_val)     (Relative)
//! rate' = base_rate * exp(Z' * t)
//! ```

use num_traits::Float;

// ─── ShiftRange ─────────────────────────────────────────────────────

/// Selects a subset of grid points for a shift operation.
///
/// Grid points are identified by zero-based index into the curve's
/// `grid_months` / `grid_rates` arrays.
///
/// # Examples
///
/// ```text
/// ShiftRange::EQ(3)  — bump only grid point 3 (key-rate delta)
/// ShiftRange::GE(0)  — bump all grid points  (parallel shift)
/// ShiftRange::LT(5)  — bump the short end only
/// ShiftRange::GE(5)  — bump the long end only
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShiftRange {
    /// All grid points with index strictly less than `idx`.
    LT(usize),
    /// All grid points with index less than or equal to `idx`.
    LE(usize),
    /// Exactly the grid point at `idx`.
    EQ(usize),
    /// All grid points with index greater than or equal to `idx`.
    GE(usize),
    /// All grid points with index strictly greater than `idx`.
    GT(usize),
}

impl ShiftRange {
    /// Returns `true` if grid point `i` is selected by this range.
    #[inline]
    pub fn contains(&self, i: usize) -> bool {
        match *self {
            Self::LT(idx) => i < idx,
            Self::LE(idx) => i <= idx,
            Self::EQ(idx) => i == idx,
            Self::GE(idx) => i >= idx,
            Self::GT(idx) => i > idx,
        }
    }

    /// Parallel shift — selects all grid points.
    #[inline]
    #[must_use]
    pub fn all() -> Self { Self::GE(0) }
}

// ─── ZeroRateShiftMode ──────────────────────────────────────────────

/// How the shift is applied to the zero rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZeroRateShiftMode {
    /// `Z_shifted = Z + shift_val` (additive, in absolute rate units).
    Absolute,
    /// `Z_shifted = Z * (1 + shift_val)` (multiplicative, as fraction).
    Relative,
}

// ─── Pure-function shifter ──────────────────────────────────────────

/// Creates shifted forward rates from a zero-rate shift specification.
///
/// This is a pure function (no mutation) that returns a new `Vec<T>`.
///
/// # Arguments
///
/// * `grid_months` — absolute month grid points
/// * `rates` — original forward rates at each grid point
/// * `base_rate` — base index level (for zero-rate conversion)
/// * `base_month` — absolute month of the base date
/// * `range` — which grid points to shift
/// * `shift_val` — magnitude of the shift
/// * `mode` — absolute or relative zero-rate shift
pub fn make_zero_rate_shifter<T: Float>(
    grid_months: &[i32],
    rates: &[T],
    base_rate: T,
    base_month: i32,
    range: ShiftRange,
    shift_val: T,
    mode: ZeroRateShiftMode,
) -> Vec<T> {
    assert_eq!(grid_months.len(), rates.len());
    let mut result = rates.to_vec();

    for i in 0..grid_months.len() {
        if range.contains(i) {
            let t_months = grid_months[i] - base_month;
            if t_months <= 0 {
                continue;
            }
            let y = rates[i];
            if y <= T::zero() || base_rate <= T::zero() {
                continue;
            }
            let t = T::from(t_months).unwrap_or_else(|| T::one());
            let z = (y / base_rate).ln() / t;
            let z_shifted = match mode {
                ZeroRateShiftMode::Absolute => z + shift_val,
                ZeroRateShiftMode::Relative => z * (T::one() + shift_val),
            };
            result[i] = base_rate * (z_shifted * t).exp();
        }
    }
    result
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shift_range_eq() {
        let r = ShiftRange::EQ(3);
        assert!(!r.contains(2));
        assert!(r.contains(3));
        assert!(!r.contains(4));
    }

    #[test]
    fn test_shift_range_ge() {
        let r = ShiftRange::GE(2);
        assert!(!r.contains(1));
        assert!(r.contains(2));
        assert!(r.contains(3));
        assert!(r.contains(100));
    }

    #[test]
    fn test_shift_range_lt() {
        let r = ShiftRange::LT(3);
        assert!(r.contains(0));
        assert!(r.contains(2));
        assert!(!r.contains(3));
        assert!(!r.contains(4));
    }

    #[test]
    fn test_shift_range_le() {
        let r = ShiftRange::LE(3);
        assert!(r.contains(0));
        assert!(r.contains(3));
        assert!(!r.contains(4));
    }

    #[test]
    fn test_shift_range_gt() {
        let r = ShiftRange::GT(3);
        assert!(!r.contains(3));
        assert!(r.contains(4));
    }

    #[test]
    fn test_shift_range_all() {
        let r = ShiftRange::all();
        assert!(r.contains(0));
        assert!(r.contains(1000));
    }

    #[test]
    fn test_make_zero_rate_shifter_parallel_absolute() {
        let base_rate = 100.0_f64;
        let base_month = 0;
        let rate = 0.02;
        let shift = 0.0001; // 1bp

        // Grid: months 12, 60, 120 from base
        let grid_months = vec![12, 60, 120];
        let rates: Vec<f64> = grid_months
            .iter()
            .map(|&m| base_rate * (rate * (m as f64)).exp())
            .collect();

        let shifted = make_zero_rate_shifter(
            &grid_months,
            &rates,
            base_rate,
            base_month,
            ShiftRange::all(),
            shift,
            ZeroRateShiftMode::Absolute,
        );

        for (i, &m) in grid_months.iter().enumerate() {
            let t = m as f64;
            let expected = base_rate * ((rate + shift) * t).exp();
            assert!(
                (shifted[i] - expected).abs() < 1e-10,
                "Mismatch at month {}: got {}, expected {}",
                m,
                shifted[i],
                expected,
            );
        }
    }

    #[test]
    fn test_make_zero_rate_shifter_key_rate() {
        let base_rate = 100.0_f64;
        let base_month = 0;
        let grid_months = vec![12, 24, 60, 120];
        let rates: Vec<f64> = grid_months
            .iter()
            .map(|&m| base_rate * (0.02 * (m as f64)).exp())
            .collect();
        let original = rates.clone();

        let shifted = make_zero_rate_shifter(
            &grid_months,
            &rates,
            base_rate,
            base_month,
            ShiftRange::EQ(2), // bump only 5Y
            0.001,
            ZeroRateShiftMode::Absolute,
        );

        // Points 0, 1, 3 unchanged
        assert!((shifted[0] - original[0]).abs() < 1e-14);
        assert!((shifted[1] - original[1]).abs() < 1e-14);
        assert!((shifted[3] - original[3]).abs() < 1e-14);
        // Point 2 changed
        assert!((shifted[2] - original[2]).abs() > 1e-6);
    }

    #[test]
    fn test_make_zero_rate_shifter_relative() {
        let base_rate = 100.0_f64;
        let base_month = 0;
        let rate = 0.02;
        let grid_months = vec![12, 60, 120];
        let rates: Vec<f64> = grid_months
            .iter()
            .map(|&m| base_rate * (rate * (m as f64)).exp())
            .collect();

        let shifted = make_zero_rate_shifter(
            &grid_months,
            &rates,
            base_rate,
            base_month,
            ShiftRange::all(),
            0.10, // +10%
            ZeroRateShiftMode::Relative,
        );

        for (i, &m) in grid_months.iter().enumerate() {
            let t = m as f64;
            let expected = base_rate * ((rate * 1.10) * t).exp();
            assert!(
                (shifted[i] - expected).abs() < 1e-10,
                "Mismatch at month {}: got {}, expected {}",
                m,
                shifted[i],
                expected,
            );
        }
    }

    #[test]
    fn test_zero_shift_is_identity() {
        let base_rate = 100.0_f64;
        let base_month = 0;
        let grid_months = vec![12, 60, 120];
        let rates: Vec<f64> = grid_months
            .iter()
            .map(|&m| base_rate * (0.025 * (m as f64)).exp())
            .collect();

        let shifted = make_zero_rate_shifter(
            &grid_months,
            &rates,
            base_rate,
            base_month,
            ShiftRange::all(),
            0.0,
            ZeroRateShiftMode::Absolute,
        );

        for i in 0..rates.len() {
            assert!((shifted[i] - rates[i]).abs() < 1e-12);
        }
    }
}
