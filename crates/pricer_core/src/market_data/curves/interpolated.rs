//! Interpolated yield curve implementation.

use super::YieldCurve;
use crate::market_data::error::MarketDataError;
use crate::math::interpolators::{Interpolator, LinearInterpolator};
use num_traits::Float;

/// Interpolation method for yield curves.
///
/// Determines how rates or discount factors are interpolated between
/// pillar points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveInterpolation {
    /// Linear interpolation on zero rates.
    ///
    /// Interpolates the zero rate linearly between pillar points,
    /// then computes the discount factor as exp(-r*t).
    Linear,

    /// Log-linear interpolation on discount factors.
    ///
    /// Interpolates ln(D(t)) linearly, which is equivalent to
    /// assuming a constant forward rate between pillars.
    LogLinear,
}

/// Interpolated yield curve using pillar points.
///
/// Stores a set of (tenor, rate) pairs and interpolates between them
/// to compute discount factors for arbitrary maturities.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Interpolation Methods
///
/// - `Linear`: Interpolates zero rates linearly
/// - `LogLinear`: Interpolates log discount factors linearly (constant forward)
///
/// # Example
///
/// ```
/// use pricer_core::market_data::curves::{YieldCurve, InterpolatedCurve, CurveInterpolation};
///
/// let tenors = [0.25, 0.5, 1.0, 2.0, 5.0];
/// let rates = [0.02, 0.025, 0.03, 0.035, 0.04];
///
/// let curve = InterpolatedCurve::new(
///     &tenors,
///     &rates,
///     CurveInterpolation::Linear,
///     false,
/// ).unwrap();
///
/// // Interpolate at 0.75 years (between 0.5 and 1.0)
/// let df = curve.discount_factor(0.75).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct InterpolatedCurve<T: Float> {
    /// Sorted tenor points (years)
    tenors: Vec<T>,
    /// Corresponding zero rates
    rates: Vec<T>,
    /// Interpolation method
    method: CurveInterpolation,
    /// Whether to allow flat extrapolation
    allow_extrapolation: bool,
}

impl<T: Float> InterpolatedCurve<T> {
    /// Construct an interpolated curve from pillar points.
    ///
    /// # Arguments
    ///
    /// * `tenors` - Tenor points in years (must be sorted, at least 2 points)
    /// * `rates` - Corresponding zero rates
    /// * `method` - Interpolation method to use
    /// * `allow_extrapolation` - Whether to allow flat extrapolation beyond pillars
    ///
    /// # Returns
    ///
    /// * `Ok(InterpolatedCurve)` - Successfully constructed curve
    /// * `Err(MarketDataError::InsufficientData)` - Fewer than 2 pillar points
    /// * `Err(MarketDataError::Interpolation)` - Invalid input data
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::{InterpolatedCurve, CurveInterpolation};
    ///
    /// let curve = InterpolatedCurve::new(
    ///     &[0.5, 1.0, 2.0],
    ///     &[0.02, 0.025, 0.03],
    ///     CurveInterpolation::Linear,
    ///     true,
    /// ).unwrap();
    /// ```
    pub fn new(
        tenors: &[T],
        rates: &[T],
        method: CurveInterpolation,
        allow_extrapolation: bool,
    ) -> Result<Self, MarketDataError> {
        if tenors.len() < 2 {
            return Err(MarketDataError::InsufficientData {
                got: tenors.len(),
                need: 2,
            });
        }

        if tenors.len() != rates.len() {
            return Err(MarketDataError::InsufficientData {
                got: rates.len(),
                need: tenors.len(),
            });
        }

        // Validate tenors are positive and sorted
        for i in 0..tenors.len() {
            if tenors[i] <= T::zero() {
                return Err(MarketDataError::InvalidMaturity {
                    t: tenors[i].to_f64().unwrap_or(0.0),
                });
            }
            if i > 0 && tenors[i] <= tenors[i - 1] {
                return Err(MarketDataError::InvalidMaturity {
                    t: tenors[i].to_f64().unwrap_or(0.0),
                });
            }
        }

        Ok(Self {
            tenors: tenors.to_vec(),
            rates: rates.to_vec(),
            method,
            allow_extrapolation,
        })
    }

    /// Return the tenor domain.
    ///
    /// # Returns
    ///
    /// A tuple (t_min, t_max) representing the range of pillar tenors.
    #[inline]
    pub fn domain(&self) -> (T, T) {
        (self.tenors[0], self.tenors[self.tenors.len() - 1])
    }

    /// Return the interpolation method.
    #[inline]
    pub fn method(&self) -> CurveInterpolation {
        self.method
    }

    /// Return whether extrapolation is allowed.
    #[inline]
    pub fn allow_extrapolation(&self) -> bool {
        self.allow_extrapolation
    }

    /// Interpolate zero rate at time t using Linear method.
    fn interpolate_linear(&self, t: T) -> Result<T, MarketDataError> {
        let (t_min, t_max) = self.domain();

        if t < t_min || t > t_max {
            if self.allow_extrapolation {
                // Flat extrapolation
                if t < t_min {
                    return Ok(self.rates[0]);
                } else {
                    return Ok(self.rates[self.rates.len() - 1]);
                }
            } else {
                return Err(MarketDataError::OutOfBounds {
                    x: t.to_f64().unwrap_or(0.0),
                    min: t_min.to_f64().unwrap_or(0.0),
                    max: t_max.to_f64().unwrap_or(0.0),
                });
            }
        }

        // Use LinearInterpolator for rate interpolation
        let interp = LinearInterpolator::new(&self.tenors, &self.rates)?;
        let rate = interp.interpolate(t)?;
        Ok(rate)
    }

    /// Interpolate discount factor at time t using LogLinear method.
    fn interpolate_log_linear(&self, t: T) -> Result<T, MarketDataError> {
        let (t_min, t_max) = self.domain();

        if t < t_min || t > t_max {
            if self.allow_extrapolation {
                // Flat extrapolation using boundary rate
                if t < t_min {
                    let rate = self.rates[0];
                    return Ok((-rate * t).exp());
                } else {
                    let rate = self.rates[self.rates.len() - 1];
                    return Ok((-rate * t).exp());
                }
            } else {
                return Err(MarketDataError::OutOfBounds {
                    x: t.to_f64().unwrap_or(0.0),
                    min: t_min.to_f64().unwrap_or(0.0),
                    max: t_max.to_f64().unwrap_or(0.0),
                });
            }
        }

        // Compute log discount factors: ln(D(t)) = -r(t) * t
        let log_dfs: Vec<T> = self
            .tenors
            .iter()
            .zip(self.rates.iter())
            .map(|(&tenor, &rate)| -rate * tenor)
            .collect();

        // Interpolate ln(D(t))
        let interp = LinearInterpolator::new(&self.tenors, &log_dfs)?;
        let log_df = interp.interpolate(t)?;

        // Return D(t) = exp(ln(D(t)))
        Ok(log_df.exp())
    }
}

impl<T: Float> YieldCurve<T> for InterpolatedCurve<T> {
    /// Return the discount factor for maturity `t`.
    ///
    /// Uses the configured interpolation method to compute the discount factor.
    ///
    /// # Arguments
    ///
    /// * `t` - Time to maturity in years (must be >= 0)
    ///
    /// # Returns
    ///
    /// * `Ok(D(t))` - Discount factor at time t
    /// * `Err(MarketDataError::InvalidMaturity)` - If t < 0
    /// * `Err(MarketDataError::OutOfBounds)` - If outside domain and extrapolation disabled
    fn discount_factor(&self, t: T) -> Result<T, MarketDataError> {
        if t < T::zero() {
            return Err(MarketDataError::InvalidMaturity {
                t: t.to_f64().unwrap_or(0.0),
            });
        }

        // Handle t=0 special case
        if t == T::zero() {
            return Ok(T::one());
        }

        match self.method {
            CurveInterpolation::Linear => {
                let rate = self.interpolate_linear(t)?;
                Ok((-rate * t).exp())
            }
            CurveInterpolation::LogLinear => self.interpolate_log_linear(t),
        }
    }

    /// Return the zero rate for maturity `t`.
    ///
    /// For Linear interpolation, returns the interpolated rate directly.
    /// For LogLinear interpolation, derives the rate from the discount factor.
    fn zero_rate(&self, t: T) -> Result<T, MarketDataError> {
        if t <= T::zero() {
            return Err(MarketDataError::InvalidMaturity {
                t: t.to_f64().unwrap_or(0.0),
            });
        }

        match self.method {
            CurveInterpolation::Linear => self.interpolate_linear(t),
            CurveInterpolation::LogLinear => {
                let df = self.interpolate_log_linear(t)?;
                Ok(-df.ln() / t)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Construction Tests
    // ========================================

    #[test]
    fn test_new_valid() {
        let tenors = [0.5_f64, 1.0, 2.0];
        let rates = [0.02, 0.025, 0.03];
        let curve =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false).unwrap();

        assert_eq!(curve.domain(), (0.5, 2.0));
        assert_eq!(curve.method(), CurveInterpolation::Linear);
        assert!(!curve.allow_extrapolation());
    }

    #[test]
    fn test_new_insufficient_data() {
        let tenors = [1.0_f64];
        let rates = [0.02];
        let result = InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false);

        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::InsufficientData { got, need } => {
                assert_eq!(got, 1);
                assert_eq!(need, 2);
            }
            _ => panic!("Expected InsufficientData error"),
        }
    }

    #[test]
    fn test_new_mismatched_lengths() {
        let tenors = [0.5_f64, 1.0, 2.0];
        let rates = [0.02, 0.025];
        let result = InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false);

        assert!(result.is_err());
    }

    #[test]
    fn test_new_negative_tenor() {
        let tenors = [-0.5_f64, 1.0];
        let rates = [0.02, 0.025];
        let result = InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false);

        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::InvalidMaturity { .. } => {}
            _ => panic!("Expected InvalidMaturity error"),
        }
    }

    #[test]
    fn test_new_unsorted_tenors() {
        let tenors = [1.0_f64, 0.5, 2.0];
        let rates = [0.02, 0.025, 0.03];
        let result = InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false);

        assert!(result.is_err());
    }

    // ========================================
    // Linear Interpolation Tests
    // ========================================

    #[test]
    fn test_linear_at_pillar_points() {
        let tenors = [0.5_f64, 1.0, 2.0];
        let rates = [0.02, 0.03, 0.04];
        let curve =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false).unwrap();

        // At pillar points, zero rate should match input
        assert!((curve.zero_rate(0.5).unwrap() - 0.02).abs() < 1e-10);
        assert!((curve.zero_rate(1.0).unwrap() - 0.03).abs() < 1e-10);
        assert!((curve.zero_rate(2.0).unwrap() - 0.04).abs() < 1e-10);
    }

    #[test]
    fn test_linear_interpolation_midpoint() {
        let tenors = [0.5_f64, 1.0, 2.0];
        let rates = [0.02, 0.04, 0.04]; // rate jumps at 1.0
        let curve =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false).unwrap();

        // At 0.75 (midpoint of 0.5 and 1.0), rate should be 0.03
        let rate = curve.zero_rate(0.75).unwrap();
        assert!((rate - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_linear_discount_factor() {
        let tenors = [0.5_f64, 1.0, 2.0];
        let rates = [0.03, 0.03, 0.03]; // flat rates for easy verification
        let curve =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false).unwrap();

        let df = curve.discount_factor(1.0).unwrap();
        let expected = (-0.03_f64).exp();
        assert!((df - expected).abs() < 1e-10);
    }

    #[test]
    fn test_linear_out_of_bounds() {
        let tenors = [0.5_f64, 1.0, 2.0];
        let rates = [0.02, 0.03, 0.04];
        let curve =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false).unwrap();

        let result = curve.discount_factor(0.25);
        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::OutOfBounds { .. } => {}
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_linear_with_extrapolation() {
        let tenors = [0.5_f64, 1.0, 2.0];
        let rates = [0.02, 0.03, 0.04];
        let curve =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, true).unwrap();

        // Extrapolate below: should use first rate
        let rate_low = curve.zero_rate(0.25).unwrap();
        assert!((rate_low - 0.02).abs() < 1e-10);

        // Extrapolate above: should use last rate
        let rate_high = curve.zero_rate(3.0).unwrap();
        assert!((rate_high - 0.04).abs() < 1e-10);
    }

    // ========================================
    // LogLinear Interpolation Tests
    // ========================================

    #[test]
    fn test_log_linear_at_pillar_points() {
        let tenors = [0.5_f64, 1.0, 2.0];
        let rates = [0.02, 0.03, 0.04];
        let curve =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::LogLinear, false).unwrap();

        // At pillar points, discount factors should match
        for (&t, &r) in tenors.iter().zip(rates.iter()) {
            let df = curve.discount_factor(t).unwrap();
            let expected = (-r * t).exp();
            assert!(
                (df - expected).abs() < 1e-10,
                "Failed at t={}: got {}, expected {}",
                t,
                df,
                expected
            );
        }
    }

    #[test]
    fn test_log_linear_implies_constant_forward() {
        let tenors = [1.0_f64, 2.0];
        let rates = [0.03, 0.04];
        let curve =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::LogLinear, false).unwrap();

        // LogLinear implies constant forward rate between pillars
        let df1 = curve.discount_factor(1.0).unwrap();
        let df2 = curve.discount_factor(2.0).unwrap();

        // Forward rate from 1 to 2
        let fwd = -(df2 / df1).ln();
        let fwd_at_1_5 = curve.forward_rate(1.0, 1.5).unwrap();
        let fwd_at_1_5_to_2 = curve.forward_rate(1.5, 2.0).unwrap();

        // Forward rates should be approximately equal (constant forward)
        assert!((fwd_at_1_5 - fwd_at_1_5_to_2).abs() < 1e-8);
    }

    // ========================================
    // Edge Cases
    // ========================================

    #[test]
    fn test_discount_factor_at_zero() {
        let tenors = [0.5_f64, 1.0, 2.0];
        let rates = [0.02, 0.03, 0.04];
        let curve =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false).unwrap();

        let df = curve.discount_factor(0.0).unwrap();
        assert!((df - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_discount_factor_negative_maturity() {
        let tenors = [0.5_f64, 1.0, 2.0];
        let rates = [0.02, 0.03, 0.04];
        let curve =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false).unwrap();

        let result = curve.discount_factor(-1.0);
        assert!(result.is_err());
        match result.unwrap_err() {
            MarketDataError::InvalidMaturity { t } => assert_eq!(t, -1.0),
            _ => panic!("Expected InvalidMaturity error"),
        }
    }

    #[test]
    fn test_zero_rate_invalid_maturity() {
        let tenors = [0.5_f64, 1.0, 2.0];
        let rates = [0.02, 0.03, 0.04];
        let curve =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false).unwrap();

        let result = curve.zero_rate(0.0);
        assert!(result.is_err());
    }

    // ========================================
    // Generic Type Tests
    // ========================================

    #[test]
    fn test_with_f32() {
        let tenors = [0.5_f32, 1.0, 2.0];
        let rates = [0.02_f32, 0.03, 0.04];
        let curve =
            InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false).unwrap();

        let df = curve.discount_factor(1.0_f32).unwrap();
        assert!(df > 0.0 && df < 1.0);
    }

    // ========================================
    // AD Compatibility Tests
    // ========================================

    #[cfg(feature = "num-dual-mode")]
    mod ad_tests {
        use super::*;
        use num_dual::*;

        #[test]
        fn test_interpolated_curve_with_dual64() {
            let tenors = [Dual64::from(0.5), Dual64::from(1.0), Dual64::from(2.0)];
            let rates = [
                Dual64::from(0.02),
                Dual64::from(0.03).derivative(), // Differentiate w.r.t. 1Y rate
                Dual64::from(0.04),
            ];

            let curve =
                InterpolatedCurve::new(&tenors, &rates, CurveInterpolation::Linear, false).unwrap();

            let t = Dual64::from(1.0);
            let df = curve.discount_factor(t).unwrap();

            // At t=1, rate = 0.03, so df ≈ exp(-0.03)
            assert!((df.re() - (-0.03_f64).exp()).abs() < 1e-5);
        }
    }
}
