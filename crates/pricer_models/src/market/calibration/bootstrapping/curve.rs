//! Bootstrapped yield curve implementation.
//!
//! This module provides `BootstrappedCurve<T>`, a yield curve constructed
//! from bootstrapped pillar points with interpolation between them.

use num_traits::Float;
use pricer_core::math::{
    interpolators::{CubicSplineInterpolator, MonotonicInterpolator},
    numeric::from_f64,
};

use super::config::BootstrapInterpolation;
use crate::market::{curves::YieldCurve, MarketDataError};

/// A yield curve constructed from bootstrapped pillar points.
///
/// Stores discount factors at discrete pillar points and interpolates
/// between them using the specified interpolation method.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`) for AD compatibility
///
/// # Examples
///
/// ```
/// use pricer_models::market::calibration::bootstrapping::{BootstrappedCurve, BootstrapInterpolation};
/// use pricer_models::market::curves::YieldCurve;
///
/// // Create a curve with 3 pillars
/// let pillars = vec![1.0, 2.0, 3.0];
/// let dfs = vec![0.97, 0.94, 0.91];
///
/// let curve: BootstrappedCurve<f64> = BootstrappedCurve::new(
///     pillars,
///     dfs,
///     BootstrapInterpolation::LogLinear,
///     true, // allow extrapolation
/// ).unwrap();
///
/// // Query discount factor at 1.5 years
/// let df = curve.discount_factor(1.5).unwrap();
/// assert!(df > 0.94 && df < 0.97);
/// ```
#[derive(Debug, Clone)]
pub struct BootstrappedCurve<T: Float> {
    /// Pillar maturities in years (sorted ascending)
    pillars: Vec<T>,
    /// Discount factors at each pillar
    discount_factors: Vec<T>,
    /// Interpolation method
    interpolation: BootstrapInterpolation,
    /// Whether extrapolation beyond pillars is allowed
    allow_extrapolation: bool,
}

impl<T: Float> BootstrappedCurve<T> {
    /// Create a new bootstrapped curve.
    ///
    /// # Arguments
    ///
    /// * `pillars` - Maturity points in years
    /// * `discount_factors` - Discount factors at each pillar
    /// * `interpolation` - Interpolation method to use
    /// * `allow_extrapolation` - Whether to allow queries beyond pillar range
    ///
    /// # Returns
    ///
    /// * `Ok(curve)` - Successfully created curve
    /// * `Err(msg)` - If validation fails
    pub fn new(
        pillars: Vec<T>,
        discount_factors: Vec<T>,
        interpolation: BootstrapInterpolation,
        allow_extrapolation: bool,
    ) -> Result<Self, String> {
        // Validate inputs
        if pillars.len() != discount_factors.len() {
            return Err(format!(
                "Pillar count ({}) must match discount factor count ({})",
                pillars.len(),
                discount_factors.len()
            ));
        }

        if pillars.is_empty() {
            return Err("Cannot create curve with no pillars".to_string());
        }

        // Check pillars are sorted
        for i in 1..pillars.len() {
            if pillars[i] <= pillars[i - 1] {
                return Err(format!(
                    "Pillars must be strictly increasing (at index {})",
                    i
                ));
            }
        }

        // Check discount factors are positive
        for (i, df) in discount_factors.iter().enumerate() {
            if *df <= T::zero() {
                return Err(format!("Discount factor at index {} must be positive", i));
            }
        }

        Ok(Self {
            pillars,
            discount_factors,
            interpolation,
            allow_extrapolation,
        })
    }

    /// Create an empty curve builder.
    pub fn builder() -> BootstrappedCurveBuilder<T> { BootstrappedCurveBuilder::new() }

    /// Get the pillar maturities.
    pub fn pillars(&self) -> &[T] { &self.pillars }

    /// Get the discount factors at pillars.
    pub fn discount_factors_at_pillars(&self) -> &[T] { &self.discount_factors }

    /// Get the interpolation method.
    pub fn interpolation(&self) -> BootstrapInterpolation { self.interpolation }

    /// Get the minimum pillar maturity.
    pub fn min_maturity(&self) -> T { self.pillars.first().copied().unwrap_or_else(T::zero) }

    /// Get the maximum pillar maturity.
    pub fn max_maturity(&self) -> T { self.pillars.last().copied().unwrap_or_else(T::zero) }

    /// Get the number of pillars.
    pub fn pillar_count(&self) -> usize { self.pillars.len() }

    /// Interpolate discount factor at time t.
    fn interpolate(&self, t: T) -> Result<T, MarketDataError> {
        // Handle t = 0 case
        if t <= T::zero() {
            return Ok(T::one());
        }

        // Find bracketing pillars
        let n = self.pillars.len();

        // Check if t is before first pillar
        if t < self.pillars[0] {
            if self.allow_extrapolation {
                // Flat extrapolation using first pillar's rate
                return self.extrapolate_left(t);
            }
            return Err(MarketDataError::OutOfBounds {
                x: t.to_f64().unwrap_or(0.0),
                min: 0.0,
                max: self.pillars[0].to_f64().unwrap_or(0.0),
            });
        }

        // Check if t is after last pillar
        if t > self.pillars[n - 1] {
            if self.allow_extrapolation {
                return self.extrapolate_right(t);
            }
            return Err(MarketDataError::OutOfBounds {
                x: t.to_f64().unwrap_or(0.0),
                min: self.pillars[0].to_f64().unwrap_or(0.0),
                max: self.pillars[n - 1].to_f64().unwrap_or(0.0),
            });
        }

        // Find bracketing index
        let idx = self.find_bracket_index(t);

        // Exact match check
        if (t - self.pillars[idx]).abs() < from_f64(1e-12) {
            return Ok(self.discount_factors[idx]);
        }

        // Interpolate based on method
        match self.interpolation {
            BootstrapInterpolation::LogLinear => self.log_linear_interpolate(t, idx),
            BootstrapInterpolation::LinearZeroRate => self.linear_zero_rate_interpolate(t, idx),
            BootstrapInterpolation::CubicSpline => self.cubic_spline_interpolate(t, idx),
            BootstrapInterpolation::MonotonicCubic => self.monotonic_cubic_interpolate(t, idx),
            BootstrapInterpolation::FlatForward => self.flat_forward_interpolate(t, idx),
        }
    }

    /// Find the index of the left bracketing pillar.
    fn find_bracket_index(&self, t: T) -> usize {
        // Binary search for the bracket
        let mut lo = 0;
        let mut hi = self.pillars.len() - 1;

        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if self.pillars[mid] <= t {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }

        lo
    }

    /// Log-linear interpolation (default).
    #[allow(clippy::unnecessary_wraps)]
    fn log_linear_interpolate(&self, t: T, idx: usize) -> Result<T, MarketDataError> {
        let t1 = self.pillars[idx];
        let t2 = self.pillars[idx + 1];
        let df1 = self.discount_factors[idx];
        let df2 = self.discount_factors[idx + 1];

        // Linear interpolation on log(df)
        let log_df1 = df1.ln();
        let log_df2 = df2.ln();

        let w = (t - t1) / (t2 - t1);
        let log_df = log_df1 * (T::one() - w) + log_df2 * w;

        Ok(log_df.exp())
    }

    /// Linear interpolation on zero rates.
    #[allow(clippy::unnecessary_wraps)]
    fn linear_zero_rate_interpolate(&self, t: T, idx: usize) -> Result<T, MarketDataError> {
        let t1 = self.pillars[idx];
        let t2 = self.pillars[idx + 1];
        let df1 = self.discount_factors[idx];
        let df2 = self.discount_factors[idx + 1];

        // Compute zero rates
        let r1 = -df1.ln() / t1;
        let r2 = -df2.ln() / t2;

        // Linear interpolation on rates
        let w = (t - t1) / (t2 - t1);
        let r = r1 * (T::one() - w) + r2 * w;

        Ok((-r * t).exp())
    }

    /// Cubic spline interpolation on zero rates.
    ///
    /// Uses natural cubic spline with C² continuity on zero rates,
    /// then converts back to discount factors. This provides smooth
    /// forward rates at the cost of potential minor arbitrage.
    #[allow(clippy::similar_names)]
    fn cubic_spline_interpolate(&self, t: T, idx: usize) -> Result<T, MarketDataError> {
        // Build interpolator on zero rates
        let zero_rates: Vec<T> = self
            .pillars
            .iter()
            .zip(self.discount_factors.iter())
            .map(|(&ti, &dfi)| {
                if ti > T::zero() {
                    -dfi.ln() / ti
                } else {
                    T::zero()
                }
            })
            .collect();

        // Convert to f64 for the interpolator (if T is f64)
        // For other types, fall back to log-linear
        let pillars_f64: Vec<f64> = self
            .pillars
            .iter()
            .map(|x| x.to_f64().unwrap_or(0.0))
            .collect();
        let rates_f64: Vec<f64> = zero_rates
            .iter()
            .map(|x| x.to_f64().unwrap_or(0.0))
            .collect();

        match CubicSplineInterpolator::new(&pillars_f64, &rates_f64) {
            Ok(interp) => {
                use pricer_core::math::interpolators::Interpolator;
                let t_f64 = t.to_f64().unwrap_or(0.0);
                match interp.interpolate(t_f64) {
                    Ok(rate_f64) => {
                        let rate = T::from(rate_f64).unwrap_or_else(T::zero);
                        Ok((-rate * t).exp())
                    }
                    Err(_) => self.log_linear_interpolate(t, idx),
                }
            }
            Err(_) => self.log_linear_interpolate(t, idx),
        }
    }

    /// Monotonic cubic interpolation using Fritsch-Carlson method.
    ///
    /// Interpolates on log(DF) to preserve discount factor monotonicity
    /// (DFs must be decreasing over time). Uses Fritsch-Carlson slopes
    /// to guarantee monotonicity of the interpolated curve.
    #[allow(clippy::similar_names)]
    fn monotonic_cubic_interpolate(&self, t: T, idx: usize) -> Result<T, MarketDataError> {
        // Build interpolator on log(DF) - this is monotonically decreasing
        let log_dfs: Vec<T> = self.discount_factors.iter().map(|&dfi| dfi.ln()).collect();

        // Convert to f64 for the interpolator
        let pillars_f64: Vec<f64> = self
            .pillars
            .iter()
            .map(|x| x.to_f64().unwrap_or(0.0))
            .collect();
        let log_dfs_f64: Vec<f64> = log_dfs.iter().map(|x| x.to_f64().unwrap_or(0.0)).collect();

        match MonotonicInterpolator::new(&pillars_f64, &log_dfs_f64) {
            Ok(interp) => {
                use pricer_core::math::interpolators::Interpolator;
                let t_f64 = t.to_f64().unwrap_or(0.0);
                match interp.interpolate(t_f64) {
                    Ok(log_df_f64) => {
                        let log_df: T = from_f64(log_df_f64);
                        Ok(log_df.exp())
                    }
                    Err(_) => self.log_linear_interpolate(t, idx),
                }
            }
            Err(_) => self.log_linear_interpolate(t, idx),
        }
    }

    /// Flat forward interpolation.
    #[allow(clippy::unnecessary_wraps)]
    fn flat_forward_interpolate(&self, t: T, idx: usize) -> Result<T, MarketDataError> {
        let t1 = self.pillars[idx];
        let df1 = self.discount_factors[idx];

        // Compute forward rate from idx to idx+1
        let t2 = self.pillars[idx + 1];
        let df2 = self.discount_factors[idx + 1];

        let forward_rate = (df1 / df2).ln() / (t2 - t1);

        // Apply constant forward rate from t1 to t
        Ok(df1 * (-forward_rate * (t - t1)).exp())
    }

    /// Extrapolate left (before first pillar).
    #[allow(clippy::unnecessary_wraps)]
    fn extrapolate_left(&self, t: T) -> Result<T, MarketDataError> {
        // Use flat rate extrapolation from first pillar
        let t1 = self.pillars[0];
        let df1 = self.discount_factors[0];
        let r1 = -df1.ln() / t1;

        Ok((-r1 * t).exp())
    }

    /// Extrapolate right (after last pillar).
    #[allow(clippy::unnecessary_wraps)]
    fn extrapolate_right(&self, t: T) -> Result<T, MarketDataError> {
        // Use flat rate extrapolation from last pillar
        let n = self.pillars.len();
        let tn = self.pillars[n - 1];
        let dfn = self.discount_factors[n - 1];
        let rn = -dfn.ln() / tn;

        Ok((-rn * t).exp())
    }

    /// Add a new pillar point to the curve.
    ///
    /// This is used during bootstrap to incrementally build the curve.
    pub fn add_pillar(&mut self, maturity: T, discount_factor: T) -> Result<(), String> {
        // Validate
        if maturity <= T::zero() {
            return Err("Maturity must be positive".to_string());
        }
        if discount_factor <= T::zero() {
            return Err("Discount factor must be positive".to_string());
        }

        // Check ordering
        if let Some(&last_mat) = self.pillars.last() {
            if maturity <= last_mat {
                return Err(format!(
                    "New maturity {:?} must be greater than last maturity {:?}",
                    maturity.to_f64(),
                    last_mat.to_f64()
                ));
            }
        }

        self.pillars.push(maturity);
        self.discount_factors.push(discount_factor);
        Ok(())
    }
}

impl<T: Float> YieldCurve<T> for BootstrappedCurve<T> {
    fn discount_factor(&self, t: T) -> Result<T, MarketDataError> {
        if t < T::zero() {
            return Err(MarketDataError::InvalidMaturity {
                t: t.to_f64().unwrap_or(0.0),
            });
        }
        self.interpolate(t)
    }

    /// Compute the instantaneous forward rate at time t.
    ///
    /// For log-linear interpolation: f(t) = -d/dt log(D(t))
    /// Since log_df is linear between pillars:
    /// f(t) = -(log_df2 - log_df1) / (t2 - t1) = constant forward rate
    ///
    /// For other interpolation methods, we use numerical differentiation
    /// as a fallback.
    fn instantaneous_forward(&self, t: T) -> Result<T, MarketDataError> {
        if t < T::zero() {
            return Err(MarketDataError::InvalidMaturity {
                t: t.to_f64().unwrap_or(0.0),
            });
        }

        // For t = 0, use the short rate from the first pillar
        if t <= T::zero() {
            let t1 = self.pillars[0];
            let df1 = self.discount_factors[0];
            return Ok(-df1.ln() / t1);
        }

        let n = self.pillars.len();

        // Handle extrapolation cases
        if t < self.pillars[0] {
            if !self.allow_extrapolation {
                return Err(MarketDataError::OutOfBounds {
                    x: t.to_f64().unwrap_or(0.0),
                    min: 0.0,
                    max: self.pillars[0].to_f64().unwrap_or(0.0),
                });
            }
            // Flat rate extrapolation - forward rate is the zero rate
            let t1 = self.pillars[0];
            let df1 = self.discount_factors[0];
            return Ok(-df1.ln() / t1);
        }

        if t > self.pillars[n - 1] {
            if !self.allow_extrapolation {
                return Err(MarketDataError::OutOfBounds {
                    x: t.to_f64().unwrap_or(0.0),
                    min: self.pillars[0].to_f64().unwrap_or(0.0),
                    max: self.pillars[n - 1].to_f64().unwrap_or(0.0),
                });
            }
            // Flat rate extrapolation - forward rate is the zero rate
            let tn = self.pillars[n - 1];
            let dfn = self.discount_factors[n - 1];
            return Ok(-dfn.ln() / tn);
        }

        // Find bracketing index
        let idx = self.find_bracket_index(t);

        // Exact match at pillar - use forward rate from this segment
        // (forward rate is continuous at pillar for log-linear)
        if idx < n - 1 {
            match self.interpolation {
                BootstrapInterpolation::LogLinear | BootstrapInterpolation::FlatForward => {
                    // Analytic derivative for log-linear: constant forward rate
                    let t1 = self.pillars[idx];
                    let t2 = self.pillars[idx + 1];
                    let df1 = self.discount_factors[idx];
                    let df2 = self.discount_factors[idx + 1];

                    let log_df1 = df1.ln();
                    let log_df2 = df2.ln();

                    // f(t) = -(log_df2 - log_df1) / (t2 - t1)
                    Ok(-(log_df2 - log_df1) / (t2 - t1))
                }
                BootstrapInterpolation::LinearZeroRate => {
                    // For linear zero rate interpolation:
                    // r(t) = r1 * (1-w) + r2 * w
                    // log(D(t)) = -r(t) * t = -(r1 * (1-w) + r2 * w) * t
                    // f(t) = -d/dt log(D(t))
                    //
                    // Let w = (t - t1) / (t2 - t1), dw/dt = 1/(t2-t1)
                    // r(t) = r1 + (r2 - r1) * w
                    // dr/dt = (r2 - r1) / (t2 - t1)
                    // f(t) = r(t) + t * dr/dt
                    let t1 = self.pillars[idx];
                    let t2 = self.pillars[idx + 1];
                    let df1 = self.discount_factors[idx];
                    let df2 = self.discount_factors[idx + 1];

                    let r1 = -df1.ln() / t1;
                    let r2 = -df2.ln() / t2;

                    let w = (t - t1) / (t2 - t1);
                    let r_t = r1 * (T::one() - w) + r2 * w;
                    let dr_dt = (r2 - r1) / (t2 - t1);

                    Ok(r_t + t * dr_dt)
                }
                _ => {
                    // For cubic spline and monotonic cubic, use numerical derivative
                    self.numerical_instantaneous_forward(t)
                }
            }
        } else {
            // At or beyond last pillar
            let tn = self.pillars[n - 1];
            let dfn = self.discount_factors[n - 1];
            Ok(-dfn.ln() / tn)
        }
    }

    fn pillar_count(&self) -> Option<usize> {
        Some(self.pillars.len())
    }

    fn pillars(&self) -> Option<&[T]> {
        Some(&self.pillars)
    }

    fn pillar_values(&self) -> Option<&[T]> {
        // Return log(DF) values as the pillar parameters
        // Note: This returns the discount factors, not log(DF)
        // For proper pillar_values we'd need to store log_dfs
        Some(&self.discount_factors)
    }

    fn max_maturity(&self) -> Option<T> {
        self.pillars.last().copied()
    }
}

impl<T: Float> BootstrappedCurve<T> {
    /// Numerical approximation of instantaneous forward rate.
    ///
    /// Uses central difference: f(t) ≈ -[log(D(t+h)) - log(D(t-h))] / (2h)
    fn numerical_instantaneous_forward(&self, t: T) -> Result<T, MarketDataError> {
        let h: T = from_f64(1e-6);
        let t_plus = t + h;
        let t_minus = t - h;

        let df_plus = self.discount_factor(t_plus)?;
        let df_minus = self.discount_factor(t_minus)?;

        let log_df_plus = df_plus.ln();
        let log_df_minus = df_minus.ln();

        Ok(-(log_df_plus - log_df_minus) / (h + h))
    }
}

/// Builder for constructing `BootstrappedCurve`.
#[derive(Debug, Clone)]
pub struct BootstrappedCurveBuilder<T: Float> {
    pillars: Vec<T>,
    discount_factors: Vec<T>,
    interpolation: BootstrapInterpolation,
    allow_extrapolation: bool,
}

impl<T: Float> BootstrappedCurveBuilder<T> {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            pillars: Vec::new(),
            discount_factors: Vec::new(),
            interpolation: BootstrapInterpolation::LogLinear,
            allow_extrapolation: true,
        }
    }

    /// Add a pillar point.
    pub fn pillar(mut self, maturity: T, discount_factor: T) -> Self {
        self.pillars.push(maturity);
        self.discount_factors.push(discount_factor);
        self
    }

    /// Set the interpolation method.
    pub fn interpolation(mut self, interp: BootstrapInterpolation) -> Self {
        self.interpolation = interp;
        self
    }

    /// Set whether extrapolation is allowed.
    pub fn allow_extrapolation(mut self, allow: bool) -> Self {
        self.allow_extrapolation = allow;
        self
    }

    /// Build the curve.
    pub fn build(self) -> Result<BootstrappedCurve<T>, String> {
        BootstrappedCurve::new(
            self.pillars,
            self.discount_factors,
            self.interpolation,
            self.allow_extrapolation,
        )
    }
}

impl<T: Float> Default for BootstrappedCurveBuilder<T> {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Construction Tests
    // ========================================

    #[test]
    fn test_create_curve() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        assert_eq!(curve.pillar_count(), 3);
        assert!((curve.min_maturity() - 1.0).abs() < 1e-10);
        assert!((curve.max_maturity() - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_create_curve_mismatched_lengths() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94];

        let result: Result<BootstrappedCurve<f64>, _> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("count"));
    }

    #[test]
    fn test_create_curve_empty() {
        let result: Result<BootstrappedCurve<f64>, _> =
            BootstrappedCurve::new(vec![], vec![], BootstrapInterpolation::LogLinear, true);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no pillars"));
    }

    #[test]
    fn test_create_curve_unsorted_pillars() {
        let pillars = vec![1.0, 3.0, 2.0]; // Not sorted
        let dfs = vec![0.97, 0.94, 0.91];

        let result: Result<BootstrappedCurve<f64>, _> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("increasing"));
    }

    #[test]
    fn test_create_curve_negative_df() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, -0.94, 0.91];

        let result: Result<BootstrappedCurve<f64>, _> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("positive"));
    }

    // ========================================
    // Discount Factor Tests
    // ========================================

    #[test]
    fn test_df_at_zero() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        let df = curve.discount_factor(0.0).unwrap();
        assert!((df - 1.0).abs() < 1e-10, "DF at t=0 should be 1.0");
    }

    #[test]
    fn test_df_at_pillar() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        let df = curve.discount_factor(2.0).unwrap();
        assert!((df - 0.94).abs() < 1e-10, "DF at pillar should match");
    }

    #[test]
    fn test_df_interpolated() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        let df = curve.discount_factor(1.5).unwrap();
        assert!(
            df > 0.94 && df < 0.97,
            "Interpolated DF should be between pillar values"
        );
    }

    #[test]
    fn test_df_extrapolated_left() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        let df = curve.discount_factor(0.5).unwrap();
        assert!(
            df > 0.97 && df < 1.0,
            "Left extrapolated DF should be > 0.97"
        );
    }

    #[test]
    fn test_df_extrapolated_right() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        let df = curve.discount_factor(4.0).unwrap();
        assert!(
            df > 0.0 && df < 0.91,
            "Right extrapolated DF should be < 0.91"
        );
    }

    #[test]
    fn test_df_no_extrapolation_error() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, false).unwrap();

        let result = curve.discount_factor(4.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_df_negative_maturity() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        let result = curve.discount_factor(-1.0);
        assert!(result.is_err());
    }

    // ========================================
    // YieldCurve Trait Tests
    // ========================================

    #[test]
    fn test_zero_rate() {
        let pillars = vec![1.0, 2.0, 3.0];
        // Use proper exponential discount factors for 3% rate
        let rate = 0.03;
        let dfs = vec![
            (-rate * 1.0_f64).exp(),
            (-rate * 2.0_f64).exp(),
            (-rate * 3.0_f64).exp(),
        ];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        let r = curve.zero_rate(1.0).unwrap();
        assert!((r - rate).abs() < 1e-10, "Zero rate should be 3%");
    }

    #[test]
    fn test_forward_rate() {
        let pillars = vec![1.0, 2.0, 3.0];
        let rate = 0.03;
        let dfs = vec![
            (-rate * 1.0_f64).exp(),
            (-rate * 2.0_f64).exp(),
            (-rate * 3.0_f64).exp(),
        ];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        let f = curve.forward_rate(1.0, 2.0).unwrap();
        assert!((f - rate).abs() < 1e-10, "Forward rate should be 3%");
    }

    // ========================================
    // Builder Tests
    // ========================================

    #[test]
    fn test_builder() {
        let curve: BootstrappedCurve<f64> = BootstrappedCurve::builder()
            .pillar(1.0, 0.97)
            .pillar(2.0, 0.94)
            .pillar(3.0, 0.91)
            .interpolation(BootstrapInterpolation::LogLinear)
            .allow_extrapolation(true)
            .build()
            .unwrap();

        assert_eq!(curve.pillar_count(), 3);
    }

    #[test]
    fn test_builder_default() {
        let builder: BootstrappedCurveBuilder<f64> = BootstrappedCurveBuilder::default();
        assert!(builder.pillars.is_empty());
    }

    // ========================================
    // Add Pillar Tests
    // ========================================

    #[test]
    fn test_add_pillar() {
        let mut curve: BootstrappedCurve<f64> = BootstrappedCurve::builder()
            .pillar(1.0, 0.97)
            .build()
            .unwrap();

        curve.add_pillar(2.0, 0.94).unwrap();
        assert_eq!(curve.pillar_count(), 2);
    }

    #[test]
    fn test_add_pillar_out_of_order() {
        let mut curve: BootstrappedCurve<f64> = BootstrappedCurve::builder()
            .pillar(2.0, 0.94)
            .build()
            .unwrap();

        let result = curve.add_pillar(1.0, 0.97);
        assert!(result.is_err());
    }

    // ========================================
    // Interpolation Method Tests
    // ========================================

    #[test]
    fn test_linear_zero_rate_interpolation() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LinearZeroRate, true)
                .unwrap();

        let df = curve.discount_factor(1.5).unwrap();
        assert!(df > 0.94 && df < 0.97);
    }

    #[test]
    fn test_flat_forward_interpolation() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::FlatForward, true)
                .unwrap();

        let df = curve.discount_factor(1.5).unwrap();
        assert!(df > 0.94 && df < 0.97);
    }

    #[test]
    fn test_cubic_spline_interpolation() {
        // Use enough points for cubic spline (minimum 3)
        let pillars = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let rate = 0.03;
        let dfs = vec![
            (-rate * 1.0_f64).exp(),
            (-rate * 2.0_f64).exp(),
            (-rate * 3.0_f64).exp(),
            (-rate * 4.0_f64).exp(),
            (-rate * 5.0_f64).exp(),
        ];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::CubicSpline, true)
                .unwrap();

        // Test interpolation at midpoint
        let df = curve.discount_factor(2.5).unwrap();
        let expected_df = (-rate * 2.5_f64).exp();

        // Cubic spline on a constant rate curve should reproduce well
        assert!(
            (df - expected_df).abs() < 1e-6,
            "Cubic spline DF at 2.5y: expected {}, got {}",
            expected_df,
            df
        );
    }

    #[test]
    fn test_cubic_spline_smoothness() {
        // Test that cubic spline produces smooth interpolation
        let pillars = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let dfs = vec![0.97, 0.94, 0.90, 0.86, 0.82];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::CubicSpline, true)
                .unwrap();

        // Sample multiple points and verify smoothness
        let mut prev_df = curve.discount_factor(1.0).unwrap();
        for i in 1..40 {
            let t = 1.0 + (i as f64) * 0.1;
            let df = curve.discount_factor(t).unwrap();

            // DF should be decreasing
            assert!(
                df <= prev_df + 1e-10,
                "DF not decreasing at t={}: {} > {}",
                t,
                df,
                prev_df
            );
            prev_df = df;
        }
    }

    #[test]
    fn test_monotonic_cubic_interpolation() {
        let pillars = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let dfs = vec![0.97, 0.94, 0.90, 0.86, 0.82];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::MonotonicCubic, true)
                .unwrap();

        // Test interpolation
        let df = curve.discount_factor(2.5).unwrap();
        assert!(
            df > 0.86 && df < 0.94,
            "Monotonic cubic DF should be between adjacent pillars"
        );
    }

    #[test]
    fn test_monotonic_cubic_preserves_monotonicity() {
        // Critical test: verify that monotonic interpolation preserves DF monotonicity
        let pillars = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let dfs = vec![0.97, 0.94, 0.90, 0.86, 0.82];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::MonotonicCubic, true)
                .unwrap();

        // Sample many points and verify strict monotonicity
        let mut prev_df = curve.discount_factor(1.0).unwrap();
        for i in 1..100 {
            let t = 1.0 + (i as f64) * 0.04;
            let df = curve.discount_factor(t).unwrap();

            // DF MUST be strictly non-increasing (allowing for numerical tolerance)
            assert!(
                df <= prev_df + 1e-12,
                "Monotonicity violated at t={}: {} > {}",
                t,
                df,
                prev_df
            );
            prev_df = df;
        }
    }

    #[test]
    fn test_flat_forward_constant_forward_rate() {
        // Flat forward interpolation should produce constant forward rate between
        // pillars
        let pillars = vec![1.0, 2.0, 3.0];
        let rate = 0.03;
        let dfs = vec![
            (-rate * 1.0_f64).exp(),
            (-rate * 2.0_f64).exp(),
            (-rate * 3.0_f64).exp(),
        ];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::FlatForward, true)
                .unwrap();

        // Forward rate between 1.2 and 1.3 should equal forward rate between pillars
        let f1 = curve.forward_rate(1.2, 1.3).unwrap();
        let f2 = curve.forward_rate(1.5, 1.6).unwrap();

        // Both should be approximately the same (constant forward within segment)
        assert!(
            (f1 - f2).abs() < 1e-6,
            "Flat forward rates should be constant: {} vs {}",
            f1,
            f2
        );
    }

    #[test]
    fn test_log_linear_equivalence_to_flat_forward() {
        // Log-linear interpolation on DF is equivalent to flat forward
        let pillars = vec![1.0, 2.0];
        let dfs = vec![0.97, 0.94];

        let curve_ll: BootstrappedCurve<f64> = BootstrappedCurve::new(
            pillars.clone(),
            dfs.clone(),
            BootstrapInterpolation::LogLinear,
            true,
        )
        .unwrap();

        let curve_ff: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::FlatForward, true)
                .unwrap();

        // Log-linear and flat forward should produce the same results
        let df_ll = curve_ll.discount_factor(1.5).unwrap();
        let df_ff = curve_ff.discount_factor(1.5).unwrap();

        assert!(
            (df_ll - df_ff).abs() < 1e-10,
            "Log-linear and flat forward should be equivalent: {} vs {}",
            df_ll,
            df_ff
        );
    }

    #[test]
    fn test_all_interpolation_methods_at_pillars() {
        // All interpolation methods should return exact values at pillar points
        let pillars = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let dfs = vec![0.97, 0.94, 0.90, 0.86, 0.82];

        let methods = [
            BootstrapInterpolation::LogLinear,
            BootstrapInterpolation::LinearZeroRate,
            BootstrapInterpolation::CubicSpline,
            BootstrapInterpolation::MonotonicCubic,
            BootstrapInterpolation::FlatForward,
        ];

        for method in methods {
            let curve: BootstrappedCurve<f64> =
                BootstrappedCurve::new(pillars.clone(), dfs.clone(), method, true).unwrap();

            for (i, (&t, &expected_df)) in pillars.iter().zip(dfs.iter()).enumerate() {
                let df = curve.discount_factor(t).unwrap();
                assert!(
                    (df - expected_df).abs() < 1e-10,
                    "{:?}: DF at pillar {} should be exact ({} vs {})",
                    method,
                    i,
                    expected_df,
                    df
                );
            }
        }
    }

    // ========================================
    // Clone Tests
    // ========================================

    #[test]
    fn test_clone() {
        let curve1: BootstrappedCurve<f64> = BootstrappedCurve::builder()
            .pillar(1.0, 0.97)
            .pillar(2.0, 0.94)
            .build()
            .unwrap();

        let curve2 = curve1.clone();
        assert_eq!(curve1.pillar_count(), curve2.pillar_count());
    }

    // ========================================
    // YieldCurve Trait Extension Tests
    // ========================================

    #[test]
    fn test_trait_pillar_count() {
        let pillars = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let dfs = vec![0.97, 0.94, 0.90, 0.86, 0.82];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        // Use the trait method
        let count = YieldCurve::<f64>::pillar_count(&curve);
        assert_eq!(count, Some(5));
    }

    #[test]
    fn test_trait_pillars() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars.clone(), dfs, BootstrapInterpolation::LogLinear, true)
                .unwrap();

        // Use the trait method
        let trait_pillars = YieldCurve::<f64>::pillars(&curve).unwrap();
        assert_eq!(trait_pillars.len(), 3);
        assert!((trait_pillars[0] - 1.0).abs() < 1e-10);
        assert!((trait_pillars[2] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_trait_pillar_values() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs.clone(), BootstrapInterpolation::LogLinear, true)
                .unwrap();

        // Use the trait method
        let values = YieldCurve::<f64>::pillar_values(&curve).unwrap();
        assert_eq!(values.len(), 3);
        assert!((values[0] - 0.97).abs() < 1e-10);
    }

    #[test]
    fn test_trait_max_maturity() {
        let pillars = vec![1.0, 2.0, 5.0, 10.0, 30.0];
        let dfs = vec![0.97, 0.94, 0.85, 0.74, 0.45];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        // Use the trait method
        let max_mat = YieldCurve::<f64>::max_maturity(&curve);
        assert_eq!(max_mat, Some(30.0));
    }

    #[test]
    fn test_instantaneous_forward_log_linear() {
        // For log-linear interpolation, instantaneous forward = constant forward between pillars
        let pillars = vec![1.0, 2.0, 3.0];
        let rate = 0.03;
        let dfs = vec![
            (-rate * 1.0_f64).exp(),
            (-rate * 2.0_f64).exp(),
            (-rate * 3.0_f64).exp(),
        ];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        // Instantaneous forward at t=1.5 should equal the discrete forward rate
        let inst_fwd = curve.instantaneous_forward(1.5).unwrap();

        // For constant rate curve, instantaneous forward = the rate
        assert!(
            (inst_fwd - rate).abs() < 1e-10,
            "Instantaneous forward should be {}, got {}",
            rate,
            inst_fwd
        );
    }

    #[test]
    fn test_instantaneous_forward_constant_between_pillars() {
        // For log-linear, instantaneous forward should be constant within a segment
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.90];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        let f1 = curve.instantaneous_forward(1.2).unwrap();
        let f2 = curve.instantaneous_forward(1.5).unwrap();
        let f3 = curve.instantaneous_forward(1.8).unwrap();

        // All should be equal (within segment [1, 2])
        assert!(
            (f1 - f2).abs() < 1e-10,
            "Forward at 1.2 and 1.5 should be equal"
        );
        assert!(
            (f2 - f3).abs() < 1e-10,
            "Forward at 1.5 and 1.8 should be equal"
        );
    }

    #[test]
    fn test_instantaneous_forward_equals_discrete_forward() {
        // For log-linear, instantaneous forward = discrete forward between pillars
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.90];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        let inst_fwd = curve.instantaneous_forward(1.5).unwrap();
        let discrete_fwd = curve.forward_rate(1.0, 2.0).unwrap();

        assert!(
            (inst_fwd - discrete_fwd).abs() < 1e-10,
            "Instantaneous forward {} should equal discrete forward {}",
            inst_fwd,
            discrete_fwd
        );
    }

    #[test]
    fn test_instantaneous_forward_linear_zero_rate() {
        // For linear zero rate, forward rate is not constant
        let pillars = vec![1.0, 2.0, 3.0];
        let rate = 0.03;
        let dfs = vec![
            (-rate * 1.0_f64).exp(),
            (-rate * 2.0_f64).exp(),
            (-rate * 3.0_f64).exp(),
        ];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LinearZeroRate, true)
                .unwrap();

        // For constant rate curve, instantaneous forward should still be ~rate
        let inst_fwd = curve.instantaneous_forward(1.5).unwrap();
        assert!(
            (inst_fwd - rate).abs() < 1e-8,
            "Instantaneous forward should be ~{}, got {}",
            rate,
            inst_fwd
        );
    }

    #[test]
    fn test_instantaneous_forward_invalid_maturity() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        let result = curve.instantaneous_forward(-1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_instantaneous_forward_extrapolation_left() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        // Forward before first pillar uses flat rate extrapolation
        let fwd = curve.instantaneous_forward(0.5).unwrap();
        assert!(fwd > 0.0, "Forward rate should be positive");
    }

    #[test]
    fn test_instantaneous_forward_extrapolation_right() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        // Forward after last pillar uses flat rate extrapolation
        let fwd = curve.instantaneous_forward(4.0).unwrap();
        assert!(fwd > 0.0, "Forward rate should be positive");
    }

    #[test]
    fn test_instantaneous_forward_no_extrapolation() {
        let pillars = vec![1.0, 2.0, 3.0];
        let dfs = vec![0.97, 0.94, 0.91];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, false).unwrap();

        // Should error when extrapolation is disabled
        let result = curve.instantaneous_forward(4.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_instantaneous_forward_cubic_spline() {
        // Cubic spline uses numerical differentiation
        let pillars = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let rate = 0.03;
        let dfs = vec![
            (-rate * 1.0_f64).exp(),
            (-rate * 2.0_f64).exp(),
            (-rate * 3.0_f64).exp(),
            (-rate * 4.0_f64).exp(),
            (-rate * 5.0_f64).exp(),
        ];

        let curve: BootstrappedCurve<f64> =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::CubicSpline, true)
                .unwrap();

        let fwd = curve.instantaneous_forward(2.5).unwrap();

        // For constant rate curve, forward should be ~rate
        assert!(
            (fwd - rate).abs() < 1e-4, // Lower precision due to numerical derivative
            "Cubic spline forward should be ~{}, got {}",
            rate,
            fwd
        );
    }
}
