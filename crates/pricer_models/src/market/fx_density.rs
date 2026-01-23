//! FX probability density calculator for risk-neutral density analysis.
//!
//! This module provides:
//! - [`FxDensityCalculator`]: Delta-Strike conversion and probability density
//!   calculation for FX options
//! - [`DeltaType`]: Delta convention types (Spot, Forward, Premium-adjusted)
//!
//! # Algorithms
//!
//! ## Delta-Strike Conversion
//!
//! Uses the Garman-Kohlhagen model inverse calculation:
//! - Spot Delta: Δ = exp(-r_f * T) * N(d1)
//! - Forward Delta: Δ = N(d1)
//! - Premium-adjusted: Δ = exp(-r_f * T) * N(d1) * K / F
//!
//! where d1 = \[ln(S/K) + (r_d - r_f + σ²/2) * T\] / (σ * √T)
//!
//! ## Probability Density (Breeden-Litzenberger)
//!
//! The risk-neutral probability density is computed using:
//! pdf(K) = exp(r_d * T) * d²C/dK²
//!
//! where d²C/dK² is approximated using central difference:
//! d²C/dK² ≈ \[C(K+h) - 2*C(K) + C(K-h)\] / h² with h = 0.001 * K
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
//! // Delta to strike conversion
//! let strike = calculator.delta_to_strike(0.25, 0.5, 0.10, DeltaType::SpotDelta).unwrap();
//!
//! // Probability density at a given strike
//! let density = calculator.probability_density(1.10, 0.5).unwrap();
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

/// Statistics of the probability density function.
///
/// Contains moments and distribution characteristics computed from
/// the risk-neutral density using numerical integration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DensityStatistics<T: Float> {
    /// Expected value (first moment): E\[K\]
    pub mean: T,
    /// Variance (second central moment): E\[(K - μ)²\]
    pub variance: T,
    /// Skewness (third standardised moment): E\[(K - μ)³\] / σ³
    pub skewness: T,
    /// Kurtosis (fourth standardised moment): E\[(K - μ)⁴\] / σ⁴
    /// Note: Excess kurtosis (normal = 0) not raw kurtosis (normal = 3)
    pub kurtosis: T,
}

impl<T: Float> Default for DensityStatistics<T> {
    fn default() -> Self {
        Self {
            mean: T::zero(),
            variance: T::zero(),
            skewness: T::zero(),
            kurtosis: T::zero(),
        }
    }
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

    /// Compute the risk-neutral probability density at a given strike.
    ///
    /// Uses the Breeden-Litzenberger method with central difference
    /// numerical differentiation.
    ///
    /// # Arguments
    ///
    /// * `strike` - Absolute strike price (must be positive)
    /// * `expiry` - Time to expiry in years (must be positive)
    ///
    /// # Returns
    ///
    /// * `Ok(density)` - The probability density (always >= 0)
    /// * `Err(MarketDataError)` - If parameters are invalid
    ///
    /// # Algorithm
    ///
    /// The Breeden-Litzenberger formula states:
    /// pdf(K) = exp(r_d * T) * d²C/dK²
    ///
    /// We approximate d²C/dK² using central difference:
    /// d²C/dK² ≈ [C(K+h) - 2*C(K) + C(K-h)] / h²
    ///
    /// where h = 0.001 * K.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let density = calculator.probability_density(1.10, 0.5)?;
    /// println!("PDF at K=1.10: {}", density);
    /// ```
    pub fn probability_density(&self, strike: T, expiry: T) -> Result<T, MarketDataError> {
        // Validate inputs
        if strike <= T::zero() {
            return Err(MarketDataError::InvalidStrike {
                strike: strike.to_f64().unwrap_or(0.0),
            });
        }
        if expiry <= T::zero() {
            return Err(MarketDataError::InvalidExpiry {
                expiry: expiry.to_f64().unwrap_or(0.0),
            });
        }

        // Get volatility at this strike
        // Use the sticky-delta approach: convert strike to approximate delta
        let volatility = self.volatility_at_strike(strike, expiry)?;

        // Central difference step: h = 0.001 * K
        let h = strike * from_f64::<T>(0.001);

        // Compute call prices at K-h, K, K+h
        let k_low = strike - h;
        let k_mid = strike;
        let k_high = strike + h;

        // Get volatility at each strike point (sticky-delta)
        let vol_low = self.volatility_at_strike(k_low, expiry)?;
        let vol_mid = volatility;
        let vol_high = self.volatility_at_strike(k_high, expiry)?;

        let c_low = self.call_price(k_low, expiry, vol_low);
        let c_mid = self.call_price(k_mid, expiry, vol_mid);
        let c_high = self.call_price(k_high, expiry, vol_high);

        // Central difference: d²C/dK² ≈ [C(K+h) - 2*C(K) + C(K-h)] / h²
        let d2c_dk2 = (c_high - from_f64::<T>(2.0) * c_mid + c_low) / (h * h);

        // Breeden-Litzenberger: pdf(K) = exp(r_d * T) * d²C/dK²
        let discount = (self.domestic_rate * expiry).exp();
        let density = discount * d2c_dk2;

        // Density must be non-negative
        Ok(density.max(T::zero()))
    }

    /// Compute statistics of the probability density function.
    ///
    /// Uses numerical integration (trapezoidal rule) to compute moments
    /// of the risk-neutral distribution.
    ///
    /// # Arguments
    ///
    /// * `expiry` - Time to expiry in years (must be positive)
    /// * `strike_range` - Range of strikes (min, max) for integration
    /// * `num_points` - Number of integration points (at least 10)
    ///
    /// # Returns
    ///
    /// * `Ok(DensityStatistics)` - Statistics including mean, variance,
    ///   skewness, kurtosis
    /// * `Err(MarketDataError)` - If parameters are invalid
    ///
    /// # Example
    ///
    /// ```ignore
    /// let forward = 1.085 * ((0.045 - 0.035) * 0.5).exp();
    /// let stats = calculator.statistics(0.5, (forward * 0.7, forward * 1.3), 100)?;
    /// println!("Mean: {}, Variance: {}", stats.mean, stats.variance);
    /// ```
    pub fn statistics(
        &self,
        expiry: T,
        strike_range: (T, T),
        num_points: usize,
    ) -> Result<DensityStatistics<T>, MarketDataError> {
        let (k_min, k_max) = strike_range;

        // Validate inputs
        if expiry <= T::zero() {
            return Err(MarketDataError::InvalidExpiry {
                expiry: expiry.to_f64().unwrap_or(0.0),
            });
        }
        if k_min <= T::zero() || k_max <= T::zero() {
            return Err(MarketDataError::InvalidStrike {
                strike: k_min.min(k_max).to_f64().unwrap_or(0.0),
            });
        }
        if k_min >= k_max {
            return Err(MarketDataError::InterpolationFailed {
                reason: format!(
                    "Strike range invalid: min {} >= max {}",
                    k_min.to_f64().unwrap_or(0.0),
                    k_max.to_f64().unwrap_or(0.0)
                ),
            });
        }
        if num_points < 10 {
            return Err(MarketDataError::InsufficientData {
                got: num_points,
                need: 10,
            });
        }

        // Grid setup
        let n = num_points;
        let dk = (k_max - k_min) / from_f64::<T>(n as f64);

        // Compute densities at each point
        let mut strikes = Vec::with_capacity(n + 1);
        let mut densities = Vec::with_capacity(n + 1);

        for i in 0..=n {
            let k = k_min + from_f64::<T>(i as f64) * dk;
            strikes.push(k);
            let density = self.probability_density(k, expiry).unwrap_or(T::zero());
            densities.push(density);
        }

        // Normalise densities (trapezoidal rule)
        let mut total_weight = T::zero();
        for i in 0..=n {
            let weight = if i == 0 || i == n {
                from_f64(0.5)
            } else {
                T::one()
            };
            total_weight = total_weight + weight * densities[i] * dk;
        }

        // Compute first moment (mean)
        let mut mean = T::zero();
        for i in 0..=n {
            let weight = if i == 0 || i == n {
                from_f64(0.5)
            } else {
                T::one()
            };
            mean = mean + weight * strikes[i] * densities[i] * dk;
        }
        if total_weight > T::zero() {
            mean = mean / total_weight;
        }

        // Compute central moments
        let mut m2 = T::zero(); // Second central moment (variance)
        let mut m3 = T::zero(); // Third central moment
        let mut m4 = T::zero(); // Fourth central moment

        for i in 0..=n {
            let weight = if i == 0 || i == n {
                from_f64(0.5)
            } else {
                T::one()
            };
            let deviation = strikes[i] - mean;
            let d2 = deviation * deviation;
            let d3 = d2 * deviation;
            let d4 = d3 * deviation;

            m2 = m2 + weight * d2 * densities[i] * dk;
            m3 = m3 + weight * d3 * densities[i] * dk;
            m4 = m4 + weight * d4 * densities[i] * dk;
        }

        if total_weight > T::zero() {
            m2 = m2 / total_weight;
            m3 = m3 / total_weight;
            m4 = m4 / total_weight;
        }

        // Compute variance (m2)
        let variance = m2;

        // Compute standardised moments (skewness and kurtosis)
        let std_dev = variance.sqrt();
        let std_dev3 = std_dev * std_dev * std_dev;
        let std_dev4 = std_dev3 * std_dev;

        let skewness = if std_dev3 > T::zero() {
            m3 / std_dev3
        } else {
            T::zero()
        };

        // Excess kurtosis (normal distribution = 0)
        let kurtosis = if std_dev4 > T::zero() {
            m4 / std_dev4 - from_f64(3.0)
        } else {
            T::zero()
        };

        Ok(DensityStatistics {
            mean,
            variance,
            skewness,
            kurtosis,
        })
    }

    /// Get volatility at a given absolute strike.
    ///
    /// Uses sticky-delta approach: converts strike to delta, then looks up
    /// the volatility from the surface.
    fn volatility_at_strike(&self, strike: T, expiry: T) -> Result<T, MarketDataError> {
        // Get ATM volatility as initial guess
        let atm_vol = self.surface.atm_volatility(expiry)?;

        // Convert strike to approximate delta using ATM vol
        let delta = self.strike_to_delta(strike, expiry, atm_vol);

        // Clamp delta to valid range (0.01, 0.99)
        let delta_clamped = delta.max(from_f64(0.01)).min(from_f64(0.99));

        // Look up volatility at this delta
        self.surface.volatility_by_delta(delta_clamped, expiry)
    }

    /// Convert strike to delta using Garman-Kohlhagen.
    fn strike_to_delta(&self, strike: T, expiry: T, volatility: T) -> T {
        let d1 = self.compute_d1(strike, expiry, volatility);
        let discount_foreign = (-self.foreign_rate * expiry).exp();

        // Forward delta formula (simpler for this purpose)
        // For call: delta ≈ N(d1) which is in (0, 1)
        // We return the "absolute" delta (0.5 at ATM)
        discount_foreign * norm_cdf(d1)
    }

    /// Compute call price using Garman-Kohlhagen.
    fn call_price(&self, strike: T, expiry: T, volatility: T) -> T {
        let sqrt_t = expiry.sqrt();
        let d1 = self.compute_d1(strike, expiry, volatility);
        let d2 = d1 - volatility * sqrt_t;

        let discount_foreign = (-self.foreign_rate * expiry).exp();
        let discount_domestic = (-self.domestic_rate * expiry).exp();

        // C = S * exp(-r_f * T) * N(d1) - K * exp(-r_d * T) * N(d2)
        self.spot * discount_foreign * norm_cdf(d1) - strike * discount_domestic * norm_cdf(d2)
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

    // ========================================
    // Probability Density Tests (Task 1.2)
    // ========================================

    #[test]
    fn test_probability_density_positive() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let expiry = 0.5;
        // Strike near ATM forward
        let forward = 1.085 * ((0.045 - 0.035) * expiry).exp();

        let density = calc.probability_density(forward, expiry).unwrap();

        assert!(
            density > 0.0,
            "Probability density must be positive, got {}",
            density
        );
    }

    #[test]
    fn test_probability_density_at_forward() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let expiry = 0.5;
        let forward = 1.085 * ((0.045 - 0.035) * expiry).exp();

        // Density at forward should be the mode (highest point) for symmetric smile
        let density_atm = calc.probability_density(forward, expiry).unwrap();
        let density_otm_low = calc.probability_density(forward * 0.95, expiry).unwrap();
        let density_otm_high = calc.probability_density(forward * 1.05, expiry).unwrap();

        assert!(
            density_atm >= density_otm_low,
            "ATM density {} should be >= OTM low density {}",
            density_atm,
            density_otm_low
        );
        assert!(
            density_atm >= density_otm_high,
            "ATM density {} should be >= OTM high density {}",
            density_atm,
            density_otm_high
        );
    }

    #[test]
    fn test_probability_density_integration_approximation() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let expiry = 0.5;
        let forward = 1.085 * ((0.045 - 0.035) * expiry).exp();

        // Integrate density using trapezoidal rule
        // Strike range: forward * 0.7 to forward * 1.3 (approximately 3 sigma)
        let k_min = forward * 0.7;
        let k_max = forward * 1.3;
        let n_points = 100;
        let dk = (k_max - k_min) / n_points as f64;

        let mut integral = 0.0;
        for i in 0..=n_points {
            let k = k_min + i as f64 * dk;
            if let Ok(density) = calc.probability_density(k, expiry) {
                let weight = if i == 0 || i == n_points { 0.5 } else { 1.0 };
                integral += density * weight * dk;
            }
        }

        // The integral should be close to 1 (within the strike range)
        // Allow some tolerance since we're not integrating the full domain
        assert!(
            integral > 0.5 && integral < 1.5,
            "Density integral {} should be approximately 1.0 (within range {}..{})",
            integral,
            k_min,
            k_max
        );
    }

    #[test]
    fn test_probability_density_longer_expiry_wider() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        // For longer expiry, distribution should be wider (lower peak)
        let forward_3m = 1.085 * ((0.045 - 0.035) * 0.25).exp();
        let forward_1y = 1.085 * ((0.045 - 0.035) * 1.0).exp();

        let density_3m = calc.probability_density(forward_3m, 0.25).unwrap();
        let density_1y = calc.probability_density(forward_1y, 1.0).unwrap();

        // ATM density should be lower for longer expiry (wider distribution)
        assert!(
            density_3m > density_1y,
            "3M ATM density {} should be > 1Y ATM density {} (wider distribution)",
            density_3m,
            density_1y
        );
    }

    #[test]
    fn test_probability_density_invalid_strike() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let result = calc.probability_density(0.0, 0.5);
        assert!(result.is_err(), "Should error for strike = 0");

        let result = calc.probability_density(-1.0, 0.5);
        assert!(result.is_err(), "Should error for negative strike");
    }

    #[test]
    fn test_probability_density_invalid_expiry() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let result = calc.probability_density(1.085, 0.0);
        assert!(result.is_err(), "Should error for expiry = 0");

        let result = calc.probability_density(1.085, -0.5);
        assert!(result.is_err(), "Should error for negative expiry");
    }

    #[test]
    fn test_probability_density_extreme_strikes() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let expiry = 0.5;
        let forward = 1.085 * ((0.045 - 0.035) * expiry).exp();

        // Very OTM strikes should have very small density
        let density_far_low = calc.probability_density(forward * 0.5, expiry).unwrap();
        let density_far_high = calc.probability_density(forward * 2.0, expiry).unwrap();

        assert!(
            density_far_low < 0.1,
            "Far OTM low density {} should be small",
            density_far_low
        );
        assert!(
            density_far_high < 0.1,
            "Far OTM high density {} should be small",
            density_far_high
        );
    }

    // ========================================
    // Density Statistics Tests (Task 1.3)
    // ========================================

    #[test]
    fn test_density_statistics_basic() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let expiry = 0.5;
        let forward = 1.085 * ((0.045 - 0.035) * expiry).exp();
        let strike_range = (forward * 0.7, forward * 1.3);

        let stats = calc.statistics(expiry, strike_range, 100).unwrap();

        // Mean should be close to forward for risk-neutral measure
        assert!(
            (stats.mean - forward).abs() < 0.1,
            "Mean {} should be close to forward {}",
            stats.mean,
            forward
        );

        // Variance should be positive
        assert!(
            stats.variance > 0.0,
            "Variance {} should be positive",
            stats.variance
        );
    }

    #[test]
    fn test_density_statistics_variance_increases_with_expiry() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        // 3M vs 1Y: longer expiry should have higher variance
        let forward_3m = 1.085 * ((0.045 - 0.035) * 0.25).exp();
        let forward_1y = 1.085 * ((0.045 - 0.035) * 1.0).exp();

        let stats_3m = calc
            .statistics(0.25, (forward_3m * 0.6, forward_3m * 1.4), 100)
            .unwrap();
        let stats_1y = calc
            .statistics(1.0, (forward_1y * 0.5, forward_1y * 1.5), 100)
            .unwrap();

        assert!(
            stats_1y.variance > stats_3m.variance,
            "1Y variance {} should be > 3M variance {}",
            stats_1y.variance,
            stats_3m.variance
        );
    }

    #[test]
    fn test_density_statistics_skewness() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let expiry = 0.5;
        let forward = 1.085 * ((0.045 - 0.035) * expiry).exp();

        let stats = calc
            .statistics(expiry, (forward * 0.7, forward * 1.3), 100)
            .unwrap();

        // Skewness should be a finite number
        assert!(
            stats.skewness.is_finite(),
            "Skewness {} should be finite",
            stats.skewness
        );
    }

    #[test]
    fn test_density_statistics_kurtosis() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let expiry = 0.5;
        let forward = 1.085 * ((0.045 - 0.035) * expiry).exp();

        let stats = calc
            .statistics(expiry, (forward * 0.7, forward * 1.3), 100)
            .unwrap();

        // Kurtosis should be a finite number (normal distribution has kurtosis ~3)
        assert!(
            stats.kurtosis.is_finite(),
            "Kurtosis {} should be finite",
            stats.kurtosis
        );
    }

    #[test]
    fn test_density_statistics_invalid_range() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        // Invalid range: min > max
        let result = calc.statistics(0.5, (1.2, 1.0), 100);
        assert!(result.is_err(), "Should error when min > max");

        // Invalid range: negative strikes
        let result = calc.statistics(0.5, (-1.0, 1.0), 100);
        assert!(result.is_err(), "Should error for negative strike range");
    }

    #[test]
    fn test_density_statistics_invalid_expiry() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        let result = calc.statistics(0.0, (0.8, 1.2), 100);
        assert!(result.is_err(), "Should error for zero expiry");

        let result = calc.statistics(-0.5, (0.8, 1.2), 100);
        assert!(result.is_err(), "Should error for negative expiry");
    }

    #[test]
    fn test_density_statistics_insufficient_points() {
        let surface = create_test_surface();
        let calc = FxDensityCalculator::new(&surface, 1.085, 0.045, 0.035);

        // Too few points
        let result = calc.statistics(0.5, (0.8, 1.2), 1);
        assert!(result.is_err(), "Should error for too few points");
    }
}
