//! Common trapezoidal integration for XVA calculations.

/// Trapezoidal integration of `exposure[i]` weighted by `weight_fn(i, t1, t2)`.
///
/// Returns `max(result, 0.0)`.
///
/// # Arguments
/// * `exposure` - Exposure profile values at each time point
/// * `time_grid` - Time points (must have same length as `exposure`)
/// * `weight_fn` - Callback `(index, t_start, t_end) -> weight` applied to each interval
#[inline]
pub fn trapezoidal_xva(
    exposure: &[f64],
    time_grid: &[f64],
    weight_fn: impl Fn(usize, f64, f64) -> f64,
) -> f64 {
    if time_grid.len() < 2 || exposure.len() != time_grid.len() {
        return 0.0;
    }

    let mut result = 0.0;
    for i in 0..time_grid.len() - 1 {
        let avg = 0.5 * (exposure[i] + exposure[i + 1]);
        result += avg * weight_fn(i, time_grid[i], time_grid[i + 1]);
    }

    result.max(0.0)
}

/// Same as [`trapezoidal_xva`] but with an additional survival-probability
/// weighting array.
#[inline]
pub fn trapezoidal_xva_with_survival(
    exposure: &[f64],
    time_grid: &[f64],
    survival: &[f64],
    weight_fn: impl Fn(usize, f64, f64) -> f64,
) -> f64 {
    if time_grid.len() < 2
        || exposure.len() != time_grid.len()
        || survival.len() != time_grid.len()
    {
        return 0.0;
    }

    let mut result = 0.0;
    for i in 0..time_grid.len() - 1 {
        let avg = 0.5 * (exposure[i] + exposure[i + 1]);
        let avg_surv = 0.5 * (survival[i] + survival[i + 1]);
        result += avg * avg_surv * weight_fn(i, time_grid[i], time_grid[i + 1]);
    }

    result.max(0.0)
}
