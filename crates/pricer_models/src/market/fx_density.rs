//! FX probability density calculator for risk-neutral density analysis.
//!
//! This module provides:
//! - [`FxDensityCalculator`]: Delta-Strike conversion and probability density
//!   calculation for FX options
//! - [`DeltaType`]: Delta convention types (Spot, Forward, Premium-adjusted)
//!
//! # Algorithm
//!
//! Delta-Strike conversion uses the Garman-Kohlhagen model inverse calculation:
//! - Spot Delta: Δ = exp(-r_f * T) * N(d1)
//! - Forward Delta: Δ = N(d1)
//! - Premium-adjusted: Δ = exp(-r_f * T) * N(d1) * K / F
//!
//! where d1 = [ln(S/K) + (r_d - r_f + σ²/2) * T] / (σ * √T)
//!
//! # Example
//!
//! ```ignore
//! use pricer_models::market::fx_density::{FxDensityCalculator, DeltaType};
//! use pricer_models::market::surfaces::FxVolatilitySurface;
//!
//! let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();
//! let calculator = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);
//!
//! let strike = calculator.delta_to_strike(0.25, 0.5, 0.10, DeltaType::SpotDelta).unwrap();
//! ```

use num_traits::Float;
use pricer_core::math::{distributions::norm_cdf, numeric::from_f64};

use super::{error::MarketDataError, surfaces::FxVolatilitySurface};

/// Delta convention type for FX options.
///
/// Different market conventions use different delta definitions:
/// - Spot Delta: Premium excluded (most common in G10)
/// - Forward Delta: Premium excluded, measured vs forward
/// - Premium-adjusted: Premium included (common in EM)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeltaType {
    /// Spot delta (premium excluded).
    ///
    /// Δ = exp(-r_f * T) * N(d1) for calls
    /// Δ = -exp(-r_f * T) * N(-d1) for puts
    #[default]
    SpotDelta,

    /// Forward delta.
    ///
    /// Δ = N(d1) for calls
    /// Δ = N(d1) - 1 for puts
    ForwardDelta,

    /// Premium-adjusted delta.
    ///
    /// Δ = exp(-r_f * T) * N(d1) * K / F for calls
    PremiumAdjusted,
}

/// FX probability density calculator.
///
/// Provides Delta-Strike conversion and risk-neutral probability density
/// calculation for FX volatility surfaces using the Garman-Kohlhagen model.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
#[derive(Debug, Clone)]
pub struct FxDensityCalculator<'a, T: Float> {
    /// Reference to the underlying volatility surface
    surface: &'a FxVolatilitySurface<T>,
    /// Spot FX rate
    spot: T,
    /// Domestic interest rate (continuously compounded)
    domestic_rate: T,
    /// Foreign interest rate (continuously compounded)
    foreign_rate: T,
}

impl<'a, T: Float> FxDensityCalculator<'a, T> {
    /// Create a new FX density calculator.
    ///
    /// # Arguments
    ///
    /// * `surface` - Reference to the FX volatility surface
    /// * `spot` - Spot FX rate (must be positive)
    /// * `domestic_rate` - Domestic interest rate (continuously compounded)
    /// * `foreign_rate` - Foreign interest rate (continuously compounded)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let calculator = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);
    /// ```
    pub fn new(
        surface: &'a FxVolatilitySurface<T>,
        spot: T,
        domestic_rate: T,
        foreign_rate: T,
    ) -> Self {
        Self {
            surface,
            spot,
            domestic_rate,
            foreign_rate,
        }
    }

    /// Convert delta to strike using Garman-Kohlhagen inverse calculation.
    ///
    /// Uses bisection method to solve for the strike K such that
    /// the option with given parameters has the specified delta.
    ///
    /// # Arguments
    ///
    /// * `delta` - Target delta value (0 < |delta| < 1)
    ///   - Positive delta for calls (e.g., 0.25 for 25D call)
    ///   - Negative delta for puts (e.g., -0.25 for 25D put)
    /// * `expiry` - Time to expiry in years (must be positive)
    /// * `volatility` - Implied volatility (must be positive)
    /// * `delta_type` - Delta convention to use
    ///
    /// # Returns
    ///
    /// * `Ok(K)` - The strike price corresponding to the given delta
    /// * `Err(MarketDataError)` - If parameters are invalid or solver fails
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Find strike for 25D call with 10% vol at 6M expiry
    /// let strike = calculator.delta_to_strike(0.25, 0.5, 0.10, DeltaType::SpotDelta)?;
    ///
    /// // Find strike for 25D put (negative delta)
    /// let put_strike = calculator.delta_to_strike(-0.25, 0.5, 0.10, DeltaType::SpotDelta)?;
    /// ```
    pub fn delta_to_strike(
        &self,
        delta: T,
        expiry: T,
        volatility: T,
        delta_type: DeltaType,
    ) -> Result<T, MarketDataError> {
        // Validate inputs
        let delta_abs = delta.abs();
        if delta_abs <= T::zero() || delta_abs >= T::one() {
            return Err(MarketDataError::InvalidStrike {
                strike: delta.to_f64().unwrap_or(0.0),
            });
        }
        if expiry <= T::zero() {
            return Err(MarketDataError::InvalidExpiry {
                expiry: expiry.to_f64().unwrap_or(0.0),
            });
        }
        if volatility <= T::zero() {
            return Err(MarketDataError::InterpolationFailed {
                reason: format!(
                    "Volatility must be positive, got {}",
                    volatility.to_f64().unwrap_or(0.0)
                ),
            });
        }
        if self.spot <= T::zero() {
            return Err(MarketDataError::InvalidStrike {
                strike: self.spot.to_f64().unwrap_or(0.0),
            });
        }

        let is_call = delta > T::zero();

        // Calculate forward price: F = S * exp((r_d - r_f) * T)
        let forward = self.spot * ((self.domestic_rate - self.foreign_rate) * expiry).exp();

        // Use bisection method for robust root finding
        // This is slower but guaranteed to converge and stay within bounds
        let result = self
            .bisection_delta_to_strike(forward, expiry, volatility, delta, delta_type, is_call)?;

        Ok(result)
    }

    /// Compute delta for a given strike using Garman-Kohlhagen.
    fn compute_delta(
        &self,
        strike: T,
        expiry: T,
        volatility: T,
        delta_type: DeltaType,
        is_call: bool,
    ) -> T {
        let d1 = self.compute_d1(strike, expiry, volatility);
        let discount_foreign = (-self.foreign_rate * expiry).exp();

        // Calculate forward
        let forward = self.spot * ((self.domestic_rate - self.foreign_rate) * expiry).exp();

        match delta_type {
            DeltaType::SpotDelta => {
                if is_call {
                    // Call: Δ = exp(-r_f * T) * N(d1)
                    discount_foreign * norm_cdf(d1)
                } else {
                    // Put: Δ = -exp(-r_f * T) * N(-d1)
                    -discount_foreign * norm_cdf(-d1)
                }
            }
            DeltaType::ForwardDelta => {
                if is_call {
                    // Call: Δ = N(d1)
                    norm_cdf(d1)
                } else {
                    // Put: Δ = N(d1) - 1
                    norm_cdf(d1) - T::one()
                }
            }
            DeltaType::PremiumAdjusted => {
                if is_call {
                    // Call: Δ = exp(-r_f * T) * N(d1) * K / F
                    discount_foreign * norm_cdf(d1) * strike / forward
                } else {
                    // Put: Δ = -exp(-r_f * T) * N(-d1) * K / F
                    -discount_foreign * norm_cdf(-d1) * strike / forward
                }
            }
        }
    }

    /// Bisection method for delta-to-strike conversion.
    ///
    /// More robust than Brent for this specific problem as it guarantees
    /// the solution stays within bounds.
    fn bisection_delta_to_strike(
        &self,
        forward: T,
        expiry: T,
        volatility: T,
        target_delta: T,
        delta_type: DeltaType,
        is_call: bool,
    ) -> Result<T, MarketDataError> {
        // Define bracket based on volatility and expiry
        // Strike range: roughly exp(-3*sigma*sqrt(T)) * F to exp(3*sigma*sqrt(T)) * F
        let vol_factor = (volatility * expiry.sqrt() * from_f64::<T>(3.0)).exp();
        let mut k_low = forward / vol_factor;
        let mut k_high = forward * vol_factor;

        // Evaluate objective at bracket endpoints
        let f_low =
            self.compute_delta(k_low, expiry, volatility, delta_type, is_call) - target_delta;
        let f_high =
            self.compute_delta(k_high, expiry, volatility, delta_type, is_call) - target_delta;

        // Check for valid bracket
        if f_low * f_high > T::zero() {
            return Err(MarketDataError::InterpolationFailed {
                reason: format!(
                    "No valid bracket for delta {}: f({}) = {}, f({}) = {}",
                    target_delta.to_f64().unwrap_or(0.0),
                    k_low.to_f64().unwrap_or(0.0),
                    f_low.to_f64().unwrap_or(0.0),
                    k_high.to_f64().unwrap_or(0.0),
                    f_high.to_f64().unwrap_or(0.0)
                ),
            });
        }

        // Bisection iterations
        let tolerance: T = from_f64(1e-10);
        let max_iterations = 100;

        // Track sign at lower bound to avoid recomputation
        let mut sign_low = if f_low < T::zero() { -1i8 } else { 1i8 };

        for _ in 0..max_iterations {
            let k_mid = (k_low + k_high) / from_f64::<T>(2.0);
            let f_mid =
                self.compute_delta(k_mid, expiry, volatility, delta_type, is_call) - target_delta;

            if f_mid.abs() < tolerance || (k_high - k_low) / from_f64::<T>(2.0) < tolerance {
                return Ok(k_mid);
            }

            // Update bracket based on sign
            let sign_mid = if f_mid < T::zero() { -1i8 } else { 1i8 };
            if sign_low != sign_mid {
                k_high = k_mid;
            } else {
                k_low = k_mid;
                sign_low = sign_mid;
            }
        }

        // Return best estimate
        Ok((k_low + k_high) / from_f64::<T>(2.0))
    }

    /// Compute d1 for Black-Scholes/Garman-Kohlhagen.
    fn compute_d1(&self, strike: T, expiry: T, volatility: T) -> T {
        // Guard against invalid strikes
        if strike <= T::zero() {
            // Return extreme value to make delta close to boundary
            return from_f64(100.0);
        }

        let sqrt_t = expiry.sqrt();
        let half: T = from_f64(0.5);

        let numerator = (self.spot / strike).ln()
            + (self.domestic_rate - self.foreign_rate + half * volatility * volatility) * expiry;

        numerator / (volatility * sqrt_t)
    }

    /// Get the spot rate.
    #[inline]
    pub fn spot(&self) -> T { self.spot }

    /// Get the domestic rate.
    #[inline]
    pub fn domestic_rate(&self) -> T { self.domestic_rate }

    /// Get the foreign rate.
    #[inline]
    pub fn foreign_rate(&self) -> T { self.foreign_rate }

    /// Get reference to the underlying surface.
    #[inline]
    pub fn surface(&self) -> &FxVolatilitySurface<T> { self.surface }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a test surface
    fn create_test_surface() -> FxVolatilitySurface<f64> {
        let deltas = [0.1_f64, 0.25, 0.5, 0.75, 0.9];
        let expiries = [0.25, 0.5, 1.0];
        let vols = [
            [0.12, 0.11, 0.10, 0.11, 0.12], // 3M
            [0.13, 0.12, 0.11, 0.12, 0.13], // 6M
            [0.14, 0.13, 0.12, 0.13, 0.14], // 1Y
        ];
        FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap()
    }

    // ========================================
    // DeltaType Tests
    // ========================================

    #[test]
    fn test_delta_type_default() {
        let dt: DeltaType = DeltaType::default();
        assert_eq!(dt, DeltaType::SpotDelta);
    }

    #[test]
    fn test_delta_type_clone() {
        let dt = DeltaType::ForwardDelta;
        let cloned = dt.clone();
        assert_eq!(dt, cloned);
    }

    // ========================================
    // Constructor Tests
    // ========================================

    #[test]
    fn test_constructor() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        assert!((calc.spot() - 1.085).abs() < 1e-10);
        assert!((calc.domestic_rate() - 0.045).abs() < 1e-10);
        assert!((calc.foreign_rate() - 0.035).abs() < 1e-10);
    }

    // ========================================
    // Delta-Strike Conversion Tests (Spot Delta)
    // ========================================

    #[test]
    fn test_delta_to_strike_atm_call() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        // ATM call (50 delta)
        let volatility = 0.11;
        let expiry = 0.5;
        let delta = 0.5;

        let strike = calc
            .delta_to_strike(delta, expiry, volatility, DeltaType::SpotDelta)
            .unwrap();

        // For ATM, strike should be close to forward
        let forward = 1.085 * ((0.045 - 0.035) * 0.5_f64).exp();
        assert!(
            (strike - forward).abs() < 0.05,
            "ATM strike {} should be close to forward {}",
            strike,
            forward
        );
    }

    #[test]
    fn test_delta_to_strike_25d_call() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        // 25D call should have higher strike than ATM
        let volatility = 0.11;
        let expiry = 0.5;

        let strike_25d = calc
            .delta_to_strike(0.25, expiry, volatility, DeltaType::SpotDelta)
            .unwrap();
        let strike_50d = calc
            .delta_to_strike(0.5, expiry, volatility, DeltaType::SpotDelta)
            .unwrap();

        assert!(
            strike_25d > strike_50d,
            "25D call strike {} should be > ATM strike {}",
            strike_25d,
            strike_50d
        );
    }

    #[test]
    fn test_delta_to_strike_25d_put() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        // 25D put (negative delta) should have lower strike than ATM
        let volatility = 0.11;
        let expiry = 0.5;

        let strike_25d_put = calc
            .delta_to_strike(-0.25, expiry, volatility, DeltaType::SpotDelta)
            .unwrap();
        let strike_50d = calc
            .delta_to_strike(0.5, expiry, volatility, DeltaType::SpotDelta)
            .unwrap();

        assert!(
            strike_25d_put < strike_50d,
            "25D put strike {} should be < ATM strike {}",
            strike_25d_put,
            strike_50d
        );
    }

    #[test]
    fn test_delta_to_strike_roundtrip() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let volatility = 0.11;
        let expiry = 0.5;
        let target_delta = 0.25;

        // Convert delta to strike
        let strike = calc
            .delta_to_strike(target_delta, expiry, volatility, DeltaType::SpotDelta)
            .unwrap();

        // Verify by computing delta at that strike
        let computed_delta =
            calc.compute_delta(strike, expiry, volatility, DeltaType::SpotDelta, true);

        assert!(
            (computed_delta - target_delta).abs() < 1e-6,
            "Roundtrip failed: target {} vs computed {}",
            target_delta,
            computed_delta
        );
    }

    // ========================================
    // Forward Delta Tests
    // ========================================

    #[test]
    fn test_delta_to_strike_forward_delta() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let volatility = 0.11;
        let expiry = 0.5;

        // Forward delta ATM should be at forward price
        let strike_fwd_atm = calc
            .delta_to_strike(0.5, expiry, volatility, DeltaType::ForwardDelta)
            .unwrap();

        let forward = 1.085 * ((0.045 - 0.035) * 0.5_f64).exp();

        assert!(
            (strike_fwd_atm - forward).abs() < 0.02,
            "Forward ATM strike {} should be close to forward {}",
            strike_fwd_atm,
            forward
        );
    }

    #[test]
    fn test_delta_to_strike_forward_delta_put() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let volatility = 0.11;
        let expiry = 0.5;
        let target_delta = -0.25;

        let strike = calc
            .delta_to_strike(target_delta, expiry, volatility, DeltaType::ForwardDelta)
            .unwrap();

        // Verify roundtrip
        let computed_delta =
            calc.compute_delta(strike, expiry, volatility, DeltaType::ForwardDelta, false);

        assert!(
            (computed_delta - target_delta).abs() < 1e-6,
            "Forward delta roundtrip failed: target {} vs computed {}",
            target_delta,
            computed_delta
        );
    }

    // ========================================
    // Premium-Adjusted Delta Tests
    // ========================================

    #[test]
    fn test_delta_to_strike_premium_adjusted() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let volatility = 0.11;
        let expiry = 0.5;
        let target_delta = 0.25;

        let strike = calc
            .delta_to_strike(target_delta, expiry, volatility, DeltaType::PremiumAdjusted)
            .unwrap();

        // Verify roundtrip
        let computed_delta =
            calc.compute_delta(strike, expiry, volatility, DeltaType::PremiumAdjusted, true);

        assert!(
            (computed_delta - target_delta).abs() < 1e-6,
            "Premium-adjusted roundtrip failed: target {} vs computed {}",
            target_delta,
            computed_delta
        );
    }

    // ========================================
    // Validation Tests
    // ========================================

    #[test]
    fn test_delta_to_strike_invalid_delta_zero() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let result = calc.delta_to_strike(0.0, 0.5, 0.10, DeltaType::SpotDelta);
        assert!(result.is_err());
    }

    #[test]
    fn test_delta_to_strike_invalid_delta_one() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let result = calc.delta_to_strike(1.0, 0.5, 0.10, DeltaType::SpotDelta);
        assert!(result.is_err());
    }

    #[test]
    fn test_delta_to_strike_invalid_expiry() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let result = calc.delta_to_strike(0.25, 0.0, 0.10, DeltaType::SpotDelta);
        assert!(result.is_err());

        let result = calc.delta_to_strike(0.25, -0.5, 0.10, DeltaType::SpotDelta);
        assert!(result.is_err());
    }

    #[test]
    fn test_delta_to_strike_invalid_volatility() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let result = calc.delta_to_strike(0.25, 0.5, 0.0, DeltaType::SpotDelta);
        assert!(result.is_err());

        let result = calc.delta_to_strike(0.25, 0.5, -0.10, DeltaType::SpotDelta);
        assert!(result.is_err());
    }

    #[test]
    fn test_delta_to_strike_invalid_spot() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 0.0, 0.045, 0.035);

        let result = calc.delta_to_strike(0.25, 0.5, 0.10, DeltaType::SpotDelta);
        assert!(result.is_err());
    }

    // ========================================
    // Edge Cases
    // ========================================

    #[test]
    fn test_delta_to_strike_extreme_delta_10d() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        // 10D call and put
        let volatility = 0.11;
        let expiry = 0.5;

        let strike_10d_call = calc
            .delta_to_strike(0.10, expiry, volatility, DeltaType::SpotDelta)
            .unwrap();
        let strike_10d_put = calc
            .delta_to_strike(-0.10, expiry, volatility, DeltaType::SpotDelta)
            .unwrap();
        let strike_atm = calc
            .delta_to_strike(0.5, expiry, volatility, DeltaType::SpotDelta)
            .unwrap();

        assert!(
            strike_10d_call > strike_atm,
            "10D call strike should be > ATM"
        );
        assert!(
            strike_10d_put < strike_atm,
            "10D put strike should be < ATM"
        );
    }

    #[test]
    fn test_delta_to_strike_varying_expiry() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let volatility = 0.11;
        let delta = 0.25;

        // Strike spread should increase with expiry
        let strike_3m = calc
            .delta_to_strike(delta, 0.25, volatility, DeltaType::SpotDelta)
            .unwrap();
        let strike_1y = calc
            .delta_to_strike(delta, 1.0, volatility, DeltaType::SpotDelta)
            .unwrap();

        let atm_3m = calc
            .delta_to_strike(0.5, 0.25, volatility, DeltaType::SpotDelta)
            .unwrap();
        let atm_1y = calc
            .delta_to_strike(0.5, 1.0, volatility, DeltaType::SpotDelta)
            .unwrap();

        let spread_3m = strike_3m - atm_3m;
        let spread_1y = strike_1y - atm_1y;

        assert!(
            spread_1y > spread_3m,
            "Longer expiry should have wider spread: 1Y={} vs 3M={}",
            spread_1y,
            spread_3m
        );
    }

    #[test]
    fn test_delta_to_strike_varying_volatility() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let expiry = 0.5;
        let delta = 0.25;

        // Higher vol should give wider strike from ATM
        let strike_low_vol = calc
            .delta_to_strike(delta, expiry, 0.05, DeltaType::SpotDelta)
            .unwrap();
        let strike_high_vol = calc
            .delta_to_strike(delta, expiry, 0.20, DeltaType::SpotDelta)
            .unwrap();

        let atm_low_vol = calc
            .delta_to_strike(0.5, expiry, 0.05, DeltaType::SpotDelta)
            .unwrap();
        let atm_high_vol = calc
            .delta_to_strike(0.5, expiry, 0.20, DeltaType::SpotDelta)
            .unwrap();

        let spread_low = strike_low_vol - atm_low_vol;
        let spread_high = strike_high_vol - atm_high_vol;

        assert!(
            spread_high > spread_low,
            "Higher vol should have wider spread: high={} vs low={}",
            spread_high,
            spread_low
        );
    }
}
