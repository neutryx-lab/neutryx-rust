//! Credit curve abstractions for credit risk calculations.
//!
//! This module provides:
//! - [`CreditCurve`]: Generic trait for hazard rate and survival probability calculations
//! - [`HazardRateCurve`]: Interpolated hazard rate curve implementation

use crate::market_data::error::MarketDataError;
use crate::math::interpolators::{Interpolator, LinearInterpolator};
use num_traits::Float;

/// Generic credit curve trait for hazard rate and survival probability calculations.
///
/// All implementations must be generic over `T: Float` for AD compatibility.
/// This ensures the curve can be used with both standard floating-point types
/// (f64, f32) and automatic differentiation types (Dual64).
///
/// # Contract
///
/// - `hazard_rate(t)` returns the instantaneous hazard rate λ(t) at time t
/// - `survival_probability(t)` returns P(τ > t) = exp(-∫₀ᵗ λ(s)ds)
/// - `default_probability(t)` returns P(τ ≤ t) = 1 - P(τ > t)
///
/// # Invariants
///
/// - λ(t) ≥ 0 for all t ≥ 0 (hazard rates are non-negative)
/// - P(τ > 0) = 1 (survival probability at time 0 is 1)
/// - P(τ > t) ≤ P(τ > s) for t ≥ s (survival probability is non-increasing)
///
/// # Example
///
/// ```
/// use pricer_core::market_data::curves::{CreditCurve, HazardRateCurve};
///
/// // Create a hazard rate curve with 100bp hazard rate
/// let tenors = [1.0_f64, 2.0, 5.0];
/// let hazard_rates = [0.01, 0.012, 0.015];
/// let curve = HazardRateCurve::new(&tenors, &hazard_rates, true).unwrap();
///
/// // Get survival probability for 1 year
/// let surv = curve.survival_probability(1.0).unwrap();
/// assert!(surv > 0.98 && surv < 1.0);
/// ```
pub trait CreditCurve<T: Float> {
    /// Return the instantaneous hazard rate at time `t`.
    ///
    /// The hazard rate λ(t) represents the instantaneous rate of default
    /// conditional on survival up to time t.
    ///
    /// # Arguments
    ///
    /// * `t` - Time in years (must be >= 0)
    ///
    /// # Returns
    ///
    /// * `Ok(λ(t))` - Hazard rate at time t
    /// * `Err(MarketDataError::InvalidMaturity)` - If t < 0
    ///
    /// # Mathematical Definition
    ///
    /// ```text
    /// λ(t) = lim(Δt→0) P(t < τ ≤ t+Δt | τ > t) / Δt
    /// ```
    fn hazard_rate(&self, t: T) -> Result<T, MarketDataError>;

    /// Return the survival probability P(τ > t).
    ///
    /// The survival probability is the probability that default has not
    /// occurred by time t.
    ///
    /// # Arguments
    ///
    /// * `t` - Time in years (must be >= 0)
    ///
    /// # Returns
    ///
    /// * `Ok(P(τ > t))` - Survival probability at time t
    /// * `Err(MarketDataError::InvalidMaturity)` - If t < 0
    ///
    /// # Mathematical Definition
    ///
    /// For a piecewise constant hazard rate:
    /// ```text
    /// P(τ > t) = exp(-∫₀ᵗ λ(s)ds)
    /// ```
    fn survival_probability(&self, t: T) -> Result<T, MarketDataError>;

    /// Return the default probability P(τ ≤ t).
    ///
    /// The default probability is the probability that default has
    /// occurred by time t.
    ///
    /// # Arguments
    ///
    /// * `t` - Time in years (must be >= 0)
    ///
    /// # Returns
    ///
    /// * `Ok(P(τ ≤ t))` - Default probability at time t
    /// * `Err(MarketDataError::InvalidMaturity)` - If t < 0
    ///
    /// # Default Implementation
    ///
    /// ```text
    /// P(τ ≤ t) = 1 - P(τ > t)
    /// ```
    fn default_probability(&self, t: T) -> Result<T, MarketDataError> {
        Ok(T::one() - self.survival_probability(t)?)
    }

    /// Return the forward survival probability P(τ > t2 | τ > t1).
    ///
    /// The probability of surviving from t1 to t2, given survival to t1.
    ///
    /// # Arguments
    ///
    /// * `t1` - Start time in years (must be >= 0)
    /// * `t2` - End time in years (must be > t1)
    ///
    /// # Returns
    ///
    /// * `Ok(P(τ > t2 | τ > t1))` - Forward survival probability
    /// * `Err(MarketDataError::InvalidMaturity)` - If t2 <= t1
    ///
    /// # Default Implementation
    ///
    /// ```text
    /// P(τ > t2 | τ > t1) = P(τ > t2) / P(τ > t1)
    /// ```
    fn forward_survival_probability(&self, t1: T, t2: T) -> Result<T, MarketDataError> {
        if t2 <= t1 {
            return Err(MarketDataError::InvalidMaturity {
                t: (t2 - t1).to_f64().unwrap_or(0.0),
            });
        }
        let s1 = self.survival_probability(t1)?;
        let s2 = self.survival_probability(t2)?;
        Ok(s2 / s1)
    }
}

/// Interpolated hazard rate curve.
///
/// Stores a set of (tenor, hazard_rate) pairs and interpolates between them
/// to compute survival probabilities for arbitrary maturities. Uses linear
/// interpolation on hazard rates with piecewise constant assumption for
/// the integral calculation.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Mathematical Model
///
/// Given pillar points (tᵢ, λᵢ), the hazard rate is interpolated linearly
/// between pillars. The survival probability is computed as:
///
/// ```text
/// P(τ > t) = exp(-∫₀ᵗ λ(s)ds)
/// ```
///
/// For piecewise linear hazard rates, the integral is computed segment by segment.
///
/// # Example
///
/// ```
/// use pricer_core::market_data::curves::{CreditCurve, HazardRateCurve};
///
/// let tenors = [1.0_f64, 3.0, 5.0, 10.0];
/// let hazard_rates = [0.01, 0.015, 0.02, 0.025];
///
/// let curve = HazardRateCurve::new(&tenors, &hazard_rates, true).unwrap();
///
/// // Survival probability at 2 years
/// let surv_2y = curve.survival_probability(2.0).unwrap();
///
/// // Default probability at 5 years
/// let def_5y = curve.default_probability(5.0).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct HazardRateCurve<T: Float> {
    /// Sorted tenor points (years)
    tenors: Vec<T>,
    /// Corresponding hazard rates
    hazard_rates: Vec<T>,
    /// Whether to allow flat extrapolation beyond pillars
    allow_extrapolation: bool,
}

impl<T: Float> HazardRateCurve<T> {
    /// Construct a hazard rate curve from pillar points.
    ///
    /// # Arguments
    ///
    /// * `tenors` - Tenor points in years (must be sorted, at least 2 points)
    /// * `hazard_rates` - Corresponding hazard rates (must be non-negative)
    /// * `allow_extrapolation` - Whether to allow flat extrapolation beyond pillars
    ///
    /// # Returns
    ///
    /// * `Ok(HazardRateCurve)` - Successfully constructed curve
    /// * `Err(MarketDataError::InsufficientData)` - Fewer than 2 pillar points
    /// * `Err(MarketDataError::InvalidMaturity)` - Invalid tenor values
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::HazardRateCurve;
    ///
    /// let curve = HazardRateCurve::new(
    ///     &[1.0_f64, 2.0, 5.0],
    ///     &[0.01, 0.012, 0.015],
    ///     true,
    /// ).unwrap();
    /// ```
    pub fn new(
        tenors: &[T],
        hazard_rates: &[T],
        allow_extrapolation: bool,
    ) -> Result<Self, MarketDataError> {
        if tenors.len() < 2 {
            return Err(MarketDataError::InsufficientData {
                got: tenors.len(),
                need: 2,
            });
        }

        if tenors.len() != hazard_rates.len() {
            return Err(MarketDataError::InsufficientData {
                got: hazard_rates.len(),
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

        // Validate hazard rates are non-negative
        for &h in hazard_rates {
            if h < T::zero() {
                return Err(MarketDataError::InterpolationFailed {
                    reason: format!(
                        "Hazard rate must be non-negative, got {}",
                        h.to_f64().unwrap_or(0.0)
                    ),
                });
            }
        }

        Ok(Self {
            tenors: tenors.to_vec(),
            hazard_rates: hazard_rates.to_vec(),
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

    /// Return the number of pillar points.
    #[inline]
    pub fn len(&self) -> usize {
        self.tenors.len()
    }

    /// Check if the curve has no pillar points.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.tenors.is_empty()
    }

    /// Return whether extrapolation is allowed.
    #[inline]
    pub fn allow_extrapolation(&self) -> bool {
        self.allow_extrapolation
    }

    /// Interpolate hazard rate at time t.
    fn interpolate_hazard_rate(&self, t: T) -> Result<T, MarketDataError> {
        let (t_min, t_max) = self.domain();

        // Handle extrapolation
        if t < t_min {
            if self.allow_extrapolation {
                return Ok(self.hazard_rates[0]);
            } else {
                return Err(MarketDataError::OutOfBounds {
                    x: t.to_f64().unwrap_or(0.0),
                    min: t_min.to_f64().unwrap_or(0.0),
                    max: t_max.to_f64().unwrap_or(0.0),
                });
            }
        }

        if t > t_max {
            if self.allow_extrapolation {
                return Ok(self.hazard_rates[self.hazard_rates.len() - 1]);
            } else {
                return Err(MarketDataError::OutOfBounds {
                    x: t.to_f64().unwrap_or(0.0),
                    min: t_min.to_f64().unwrap_or(0.0),
                    max: t_max.to_f64().unwrap_or(0.0),
                });
            }
        }

        // Linear interpolation
        let interp = LinearInterpolator::new(&self.tenors, &self.hazard_rates)?;
        interp.interpolate(t).map_err(MarketDataError::from)
    }

    /// Compute the integrated hazard rate ∫₀ᵗ λ(s)ds.
    ///
    /// For linear interpolation between pillars, the integral of a linear
    /// function over each segment is the trapezoidal area.
    fn integrated_hazard(&self, t: T) -> Result<T, MarketDataError> {
        if t <= T::zero() {
            return Ok(T::zero());
        }

        let (t_min, t_max) = self.domain();

        // For t <= t_min, assume constant hazard rate from 0 to t_min
        if t <= t_min {
            let h0 = if self.allow_extrapolation {
                self.hazard_rates[0]
            } else {
                return Err(MarketDataError::OutOfBounds {
                    x: t.to_f64().unwrap_or(0.0),
                    min: t_min.to_f64().unwrap_or(0.0),
                    max: t_max.to_f64().unwrap_or(0.0),
                });
            };
            return Ok(h0 * t);
        }

        let mut integral = T::zero();

        // Integrate from 0 to t_min with constant hazard rate (extrapolation)
        if self.allow_extrapolation {
            integral = integral + self.hazard_rates[0] * t_min;
        }
        let mut prev_t = t_min;

        // Integrate over each segment up to t
        for i in 0..self.tenors.len() {
            let curr_t = self.tenors[i];

            if prev_t >= t {
                break;
            }

            // Skip segments before the first tenor
            if curr_t <= prev_t {
                continue;
            }

            let end_t = if t < curr_t { t } else { curr_t };

            // Get hazard rates at segment endpoints
            let h_start = self.interpolate_hazard_rate(prev_t)?;
            let h_end = self.interpolate_hazard_rate(end_t)?;

            // Trapezoidal integration for linear hazard rate
            let segment_integral = (h_start + h_end) / (T::one() + T::one()) * (end_t - prev_t);
            integral = integral + segment_integral;

            prev_t = curr_t;
        }

        // Handle extrapolation beyond t_max
        if t > t_max {
            if self.allow_extrapolation {
                let h_last = self.hazard_rates[self.hazard_rates.len() - 1];
                integral = integral + h_last * (t - t_max);
            } else {
                return Err(MarketDataError::OutOfBounds {
                    x: t.to_f64().unwrap_or(0.0),
                    min: t_min.to_f64().unwrap_or(0.0),
                    max: t_max.to_f64().unwrap_or(0.0),
                });
            }
        }

        Ok(integral)
    }
}

impl<T: Float> CreditCurve<T> for HazardRateCurve<T> {
    fn hazard_rate(&self, t: T) -> Result<T, MarketDataError> {
        if t < T::zero() {
            return Err(MarketDataError::InvalidMaturity {
                t: t.to_f64().unwrap_or(0.0),
            });
        }
        self.interpolate_hazard_rate(t)
    }

    fn survival_probability(&self, t: T) -> Result<T, MarketDataError> {
        if t < T::zero() {
            return Err(MarketDataError::InvalidMaturity {
                t: t.to_f64().unwrap_or(0.0),
            });
        }

        if t == T::zero() {
            return Ok(T::one());
        }

        let integrated = self.integrated_hazard(t)?;
        Ok((-integrated).exp())
    }
}

/// A flat (constant) hazard rate curve.
///
/// Simple credit curve where the same hazard rate applies to all maturities.
/// Useful for prototyping, testing, and when only a single CDS spread is known.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Mathematical Model
///
/// For constant hazard rate λ:
/// ```text
/// P(τ > t) = exp(-λ * t)
/// P(τ ≤ t) = 1 - exp(-λ * t)
/// ```
///
/// # Example
///
/// ```
/// use pricer_core::market_data::curves::{CreditCurve, FlatHazardRateCurve};
///
/// let curve = FlatHazardRateCurve::new(0.01_f64); // 100bp hazard rate
///
/// // Survival probability at 5 years
/// let surv = curve.survival_probability(5.0).unwrap();
/// let expected = (-0.01_f64 * 5.0).exp();
/// assert!((surv - expected).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatHazardRateCurve<T: Float> {
    /// The constant hazard rate
    hazard_rate: T,
}

impl<T: Float> FlatHazardRateCurve<T> {
    /// Construct a flat hazard rate curve.
    ///
    /// # Arguments
    ///
    /// * `hazard_rate` - The constant hazard rate (must be >= 0)
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::curves::FlatHazardRateCurve;
    ///
    /// let curve = FlatHazardRateCurve::new(0.01_f64);
    /// assert_eq!(curve.rate(), 0.01);
    /// ```
    #[inline]
    pub fn new(hazard_rate: T) -> Self {
        Self { hazard_rate }
    }

    /// Return the constant hazard rate.
    #[inline]
    pub fn rate(&self) -> T {
        self.hazard_rate
    }
}

impl<T: Float> CreditCurve<T> for FlatHazardRateCurve<T> {
    fn hazard_rate(&self, t: T) -> Result<T, MarketDataError> {
        if t < T::zero() {
            return Err(MarketDataError::InvalidMaturity {
                t: t.to_f64().unwrap_or(0.0),
            });
        }
        Ok(self.hazard_rate)
    }

    fn survival_probability(&self, t: T) -> Result<T, MarketDataError> {
        if t < T::zero() {
            return Err(MarketDataError::InvalidMaturity {
                t: t.to_f64().unwrap_or(0.0),
            });
        }
        Ok((-self.hazard_rate * t).exp())
    }

    fn default_probability(&self, t: T) -> Result<T, MarketDataError> {
        if t < T::zero() {
            return Err(MarketDataError::InvalidMaturity {
                t: t.to_f64().unwrap_or(0.0),
            });
        }
        Ok(T::one() - (-self.hazard_rate * t).exp())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // FlatHazardRateCurve Tests
    // ========================================

    #[test]
    fn test_flat_curve_new() {
        let curve = FlatHazardRateCurve::new(0.01_f64);
        assert_eq!(curve.rate(), 0.01);
    }

    #[test]
    fn test_flat_curve_hazard_rate() {
        let curve = FlatHazardRateCurve::new(0.02_f64);
        let h = curve.hazard_rate(1.0).unwrap();
        assert!((h - 0.02).abs() < 1e-10);

        let h2 = curve.hazard_rate(5.0).unwrap();
        assert!((h2 - 0.02).abs() < 1e-10);
    }

    #[test]
    fn test_flat_curve_hazard_rate_invalid() {
        let curve = FlatHazardRateCurve::new(0.02_f64);
        let result = curve.hazard_rate(-1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_flat_curve_survival_at_zero() {
        let curve = FlatHazardRateCurve::new(0.02_f64);
        let surv = curve.survival_probability(0.0).unwrap();
        assert!((surv - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_flat_curve_survival_probability() {
        let curve = FlatHazardRateCurve::new(0.01_f64);
        let surv = curve.survival_probability(5.0).unwrap();
        let expected = (-0.01_f64 * 5.0).exp();
        assert!((surv - expected).abs() < 1e-10);
    }

    #[test]
    fn test_flat_curve_default_probability() {
        let curve = FlatHazardRateCurve::new(0.01_f64);
        let def = curve.default_probability(5.0).unwrap();
        let expected = 1.0 - (-0.01_f64 * 5.0).exp();
        assert!((def - expected).abs() < 1e-10);
    }

    #[test]
    fn test_flat_curve_survival_plus_default() {
        let curve = FlatHazardRateCurve::new(0.015_f64);
        let surv = curve.survival_probability(3.0).unwrap();
        let def = curve.default_probability(3.0).unwrap();
        assert!((surv + def - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_flat_curve_forward_survival() {
        let curve = FlatHazardRateCurve::new(0.02_f64);
        let fwd = curve.forward_survival_probability(1.0, 3.0).unwrap();
        // P(τ > 3 | τ > 1) = P(τ > 3) / P(τ > 1) = exp(-λ*3) / exp(-λ*1) = exp(-λ*2)
        let expected = (-0.02_f64 * 2.0).exp();
        assert!((fwd - expected).abs() < 1e-10);
    }

    #[test]
    fn test_flat_curve_clone_copy() {
        let curve = FlatHazardRateCurve::new(0.01_f64);
        let cloned = curve;
        assert_eq!(curve.rate(), cloned.rate());
    }

    // ========================================
    // HazardRateCurve Construction Tests
    // ========================================

    #[test]
    fn test_hazard_curve_new_valid() {
        let tenors = [1.0_f64, 2.0, 5.0];
        let hazard_rates = [0.01, 0.012, 0.015];
        let curve = HazardRateCurve::new(&tenors, &hazard_rates, false).unwrap();

        assert_eq!(curve.domain(), (1.0, 5.0));
        assert_eq!(curve.len(), 3);
        assert!(!curve.is_empty());
    }

    #[test]
    fn test_hazard_curve_new_insufficient_data() {
        let tenors = [1.0_f64];
        let hazard_rates = [0.01];
        let result = HazardRateCurve::new(&tenors, &hazard_rates, false);

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
    fn test_hazard_curve_new_mismatched_lengths() {
        let tenors = [1.0_f64, 2.0, 5.0];
        let hazard_rates = [0.01, 0.012];
        let result = HazardRateCurve::new(&tenors, &hazard_rates, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_hazard_curve_new_negative_tenor() {
        let tenors = [-1.0_f64, 2.0];
        let hazard_rates = [0.01, 0.012];
        let result = HazardRateCurve::new(&tenors, &hazard_rates, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_hazard_curve_new_unsorted_tenors() {
        let tenors = [2.0_f64, 1.0, 5.0];
        let hazard_rates = [0.01, 0.012, 0.015];
        let result = HazardRateCurve::new(&tenors, &hazard_rates, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_hazard_curve_new_negative_hazard_rate() {
        let tenors = [1.0_f64, 2.0];
        let hazard_rates = [0.01, -0.005];
        let result = HazardRateCurve::new(&tenors, &hazard_rates, false);
        assert!(result.is_err());
    }

    // ========================================
    // HazardRateCurve Hazard Rate Tests
    // ========================================

    #[test]
    fn test_hazard_curve_rate_at_pillars() {
        let tenors = [1.0_f64, 2.0, 5.0];
        let hazard_rates = [0.01, 0.015, 0.02];
        let curve = HazardRateCurve::new(&tenors, &hazard_rates, false).unwrap();

        assert!((curve.hazard_rate(1.0).unwrap() - 0.01).abs() < 1e-10);
        assert!((curve.hazard_rate(2.0).unwrap() - 0.015).abs() < 1e-10);
        assert!((curve.hazard_rate(5.0).unwrap() - 0.02).abs() < 1e-10);
    }

    #[test]
    fn test_hazard_curve_rate_interpolated() {
        let tenors = [1.0_f64, 3.0];
        let hazard_rates = [0.01, 0.02];
        let curve = HazardRateCurve::new(&tenors, &hazard_rates, false).unwrap();

        // At t=2.0, linear interpolation: 0.01 + (0.02-0.01)*(2-1)/(3-1) = 0.015
        let h = curve.hazard_rate(2.0).unwrap();
        assert!((h - 0.015).abs() < 1e-10);
    }

    #[test]
    fn test_hazard_curve_rate_invalid_maturity() {
        let tenors = [1.0_f64, 2.0, 5.0];
        let hazard_rates = [0.01, 0.015, 0.02];
        let curve = HazardRateCurve::new(&tenors, &hazard_rates, false).unwrap();

        let result = curve.hazard_rate(-1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_hazard_curve_rate_out_of_bounds() {
        let tenors = [1.0_f64, 2.0, 5.0];
        let hazard_rates = [0.01, 0.015, 0.02];
        let curve = HazardRateCurve::new(&tenors, &hazard_rates, false).unwrap();

        let result = curve.hazard_rate(0.5);
        assert!(result.is_err());

        let result = curve.hazard_rate(10.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_hazard_curve_rate_with_extrapolation() {
        let tenors = [1.0_f64, 2.0, 5.0];
        let hazard_rates = [0.01, 0.015, 0.02];
        let curve = HazardRateCurve::new(&tenors, &hazard_rates, true).unwrap();

        // Extrapolate below: should use first rate
        let h_low = curve.hazard_rate(0.5).unwrap();
        assert!((h_low - 0.01).abs() < 1e-10);

        // Extrapolate above: should use last rate
        let h_high = curve.hazard_rate(10.0).unwrap();
        assert!((h_high - 0.02).abs() < 1e-10);
    }

    // ========================================
    // HazardRateCurve Survival Probability Tests
    // ========================================

    #[test]
    fn test_hazard_curve_survival_at_zero() {
        let tenors = [1.0_f64, 2.0, 5.0];
        let hazard_rates = [0.01, 0.015, 0.02];
        let curve = HazardRateCurve::new(&tenors, &hazard_rates, true).unwrap();

        let surv = curve.survival_probability(0.0).unwrap();
        assert!((surv - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_hazard_curve_survival_monotonic_decreasing() {
        let tenors = [1.0_f64, 3.0, 5.0, 10.0];
        let hazard_rates = [0.01, 0.015, 0.02, 0.025];
        let curve = HazardRateCurve::new(&tenors, &hazard_rates, true).unwrap();

        let surv_1 = curve.survival_probability(1.0).unwrap();
        let surv_3 = curve.survival_probability(3.0).unwrap();
        let surv_5 = curve.survival_probability(5.0).unwrap();
        let surv_10 = curve.survival_probability(10.0).unwrap();

        assert!(surv_1 > surv_3);
        assert!(surv_3 > surv_5);
        assert!(surv_5 > surv_10);
    }

    #[test]
    fn test_hazard_curve_survival_bounded() {
        let tenors = [1.0_f64, 2.0, 5.0];
        let hazard_rates = [0.01, 0.015, 0.02];
        let curve = HazardRateCurve::new(&tenors, &hazard_rates, true).unwrap();

        for t in [0.5_f64, 1.0, 2.0, 3.0, 5.0, 10.0] {
            let surv = curve.survival_probability(t).unwrap();
            assert!(
                surv > 0.0,
                "Survival probability should be positive at t={}",
                t
            );
            assert!(
                surv <= 1.0,
                "Survival probability should be <= 1 at t={}",
                t
            );
        }
    }

    #[test]
    fn test_hazard_curve_survival_plus_default() {
        let tenors = [1.0_f64, 3.0, 5.0];
        let hazard_rates = [0.01, 0.015, 0.02];
        let curve = HazardRateCurve::new(&tenors, &hazard_rates, true).unwrap();

        for t in [1.0_f64, 2.0, 3.0, 4.0, 5.0] {
            let surv = curve.survival_probability(t).unwrap();
            let def = curve.default_probability(t).unwrap();
            assert!(
                (surv + def - 1.0).abs() < 1e-10,
                "Survival + Default should equal 1 at t={}",
                t
            );
        }
    }

    // ========================================
    // HazardRateCurve Forward Survival Tests
    // ========================================

    #[test]
    fn test_hazard_curve_forward_survival() {
        let tenors = [1.0_f64, 3.0, 5.0];
        let hazard_rates = [0.01, 0.015, 0.02];
        let curve = HazardRateCurve::new(&tenors, &hazard_rates, true).unwrap();

        let fwd = curve.forward_survival_probability(1.0, 3.0).unwrap();
        let s1 = curve.survival_probability(1.0).unwrap();
        let s3 = curve.survival_probability(3.0).unwrap();

        assert!((fwd - s3 / s1).abs() < 1e-10);
    }

    #[test]
    fn test_hazard_curve_forward_survival_invalid() {
        let tenors = [1.0_f64, 3.0, 5.0];
        let hazard_rates = [0.01, 0.015, 0.02];
        let curve = HazardRateCurve::new(&tenors, &hazard_rates, true).unwrap();

        let result = curve.forward_survival_probability(3.0, 1.0);
        assert!(result.is_err());

        let result = curve.forward_survival_probability(2.0, 2.0);
        assert!(result.is_err());
    }

    // ========================================
    // Generic Type Tests
    // ========================================

    #[test]
    fn test_flat_curve_with_f32() {
        let curve = FlatHazardRateCurve::new(0.01_f32);
        let surv = curve.survival_probability(5.0_f32).unwrap();
        let expected = (-0.01_f32 * 5.0_f32).exp();
        assert!((surv - expected).abs() < 1e-6);
    }

    #[test]
    fn test_hazard_curve_with_f32() {
        let tenors = [1.0_f32, 2.0, 5.0];
        let hazard_rates = [0.01_f32, 0.012, 0.015];
        let curve = HazardRateCurve::new(&tenors, &hazard_rates, true).unwrap();

        let surv = curve.survival_probability(2.0_f32).unwrap();
        assert!(surv > 0.0 && surv < 1.0);
    }

    // ========================================
    // Clone Tests
    // ========================================

    #[test]
    fn test_hazard_curve_clone() {
        let tenors = [1.0_f64, 2.0, 5.0];
        let hazard_rates = [0.01, 0.012, 0.015];
        let curve = HazardRateCurve::new(&tenors, &hazard_rates, true).unwrap();
        let cloned = curve.clone();

        assert_eq!(curve.domain(), cloned.domain());
        assert_eq!(curve.len(), cloned.len());

        let h1 = curve.hazard_rate(2.0).unwrap();
        let h2 = cloned.hazard_rate(2.0).unwrap();
        assert!((h1 - h2).abs() < 1e-10);
    }
}
