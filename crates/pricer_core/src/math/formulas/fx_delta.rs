//! FX Delta-Strike conversion functions.
//!
//! This module provides functions to convert between FX option delta values
//! and strike prices. These functions support various delta conventions
//! used in the FX options market.
//!
//! # Delta Conventions
//!
//! - **Spot Delta**: Standard for most G10 pairs (EURUSD, GBPUSD, etc.)
//! - **Forward Delta**: Delta measured against the forward price
//! - **Premium-Adjusted Delta**: Standard for EM pairs (USDJPY, etc.)
//!
//! # Mathematical Background
//!
//! For a call option with spot delta convention:
//! - Δ = e^(-rf×T) × N(d1)
//! - d1 = [ln(F/K) + 0.5×σ²×T] / (σ×√T)
//!
//! Given delta, we solve for strike K:
//! - d1 = Φ⁻¹(Δ / e^(-rf×T)) for spot delta
//! - d1 = Φ⁻¹(Δ) for forward delta
//! - K = F × exp(-d1×σ×√T + 0.5×σ²×T)
//!
//! # Example
//!
//! ```
//! use pricer_core::math::formulas::fx_delta::{delta_to_strike, strike_to_delta};
//! use infra_domain::trade::instrument_def::DeltaType;
//!
//! // Convert 25-delta call to strike
//! let strike = delta_to_strike(
//!     0.25,   // delta (positive = call)
//!     1.10,   // spot
//!     0.03,   // domestic rate
//!     0.01,   // foreign rate
//!     1.0,    // expiry (1 year)
//!     0.10,   // volatility
//!     DeltaType::SpotPercent,
//! ).unwrap();
//!
//! // Verify round-trip
//! let recovered_delta = strike_to_delta(
//!     strike,
//!     1.10,
//!     0.03,
//!     0.01,
//!     1.0,
//!     0.10,
//!     true, // is_call
//!     DeltaType::SpotPercent,
//! ).unwrap();
//!
//! assert!((recovered_delta - 0.25_f64).abs() < 1e-6);
//! ```

use infra_domain::trade::instrument_def::DeltaType;
use num_traits::Float;

use super::error::{
    require_positive_expiry, require_positive_spot, require_positive_strike, require_positive_vol,
    FormulaError,
};
use crate::math::{
    normal_dist::{norm_cdf, norm_inv_cdf, norm_pdf},
    numeric::from_f64,
    solvers::{NewtonRaphsonSolver, SolverConfig},
};

/// Converts a delta value to a strike price for the given delta convention.
pub fn delta_to_strike<T: Float>(
    delta: T,
    spot: T,
    domestic_rate: T,
    foreign_rate: T,
    expiry: T,
    volatility: T,
    delta_type: DeltaType,
) -> Result<T, FormulaError> {
    require_positive_spot(spot)?;
    require_positive_vol(volatility)?;
    require_positive_expiry(expiry)?;

    let abs_delta = delta.abs();
    let is_call = delta > T::zero();

    // Forward price: F = S × exp((rd - rf) × T)
    let drift = (domestic_rate - foreign_rate) * expiry;
    let forward = spot * drift.exp();

    let sqrt_t = expiry.sqrt();
    let vol_sqrt_t = volatility * sqrt_t;
    let half: T = from_f64(0.5);

    // Calculate d1 based on delta type
    let d1 = match delta_type {
        DeltaType::SpotPercent | DeltaType::SpotPips => {
            // For spot delta: Δ = e^(-rf×T) × N(d1)
            // N(d1) = Δ × e^(rf×T)
            let df_foreign = (-foreign_rate * expiry).exp();
            let adjusted_delta = abs_delta / df_foreign;

            // Clamp to valid CDF range
            let clamped = adjusted_delta
                .min(from_f64(0.9999999))
                .max(from_f64(0.0000001));
            norm_inv_cdf(clamped).map_err(|e| FormulaError::NumericalInstability {
                message: format!("norm_inv_cdf failed: {:?}", e),
            })?
        }
        DeltaType::ForwardPercent | DeltaType::ForwardPips => {
            // For forward delta: Δ = N(d1)
            let clamped = abs_delta.min(from_f64(0.9999999)).max(from_f64(0.0000001));
            norm_inv_cdf(clamped).map_err(|e| FormulaError::NumericalInstability {
                message: format!("norm_inv_cdf failed: {:?}", e),
            })?
        }
        DeltaType::PremiumAdjusted => {
            // Premium-adjusted delta requires Newton-Raphson iteration
            // For calls: Δ_pa = e^(-rf×T) × N(d1) × K / F
            // For puts: Δ_pa = e^(-rf×T) × (N(d1) - 1) × K / F
            return delta_to_strike_premium_adjusted(
                abs_delta,
                is_call,
                spot,
                forward,
                domestic_rate,
                foreign_rate,
                expiry,
                volatility,
            );
        }
    };

    // For puts, negate d1
    let d1_signed = if is_call { d1 } else { -d1 };

    // Strike from d1: K = F × exp(-d1 × σ × √T + 0.5 × σ² × T)
    let strike =
        forward * (-d1_signed * vol_sqrt_t + half * volatility * volatility * expiry).exp();

    Ok(strike)
}

/// Premium-adjusted delta to strike conversion via Newton-Raphson iteration.
fn delta_to_strike_premium_adjusted<T: Float>(
    target_delta: T,
    is_call: bool,
    _spot: T,
    forward: T,
    _domestic_rate: T,
    foreign_rate: T,
    expiry: T,
    volatility: T,
) -> Result<T, FormulaError> {
    let sqrt_t = expiry.sqrt();
    let vol_sqrt_t = volatility * sqrt_t;
    let half: T = from_f64(0.5);
    let df_foreign = (-foreign_rate * expiry).exp();

    // Initial guess: use spot delta approximation
    let initial_d1 = {
        let adjusted_delta = target_delta / df_foreign;
        let clamped = adjusted_delta
            .min(from_f64(0.9999999))
            .max(from_f64(0.0000001));
        norm_inv_cdf(clamped).map_err(|e| FormulaError::NumericalInstability {
            message: format!("norm_inv_cdf failed: {:?}", e),
        })?
    };
    let d1_signed = if is_call { initial_d1 } else { -initial_d1 };
    let initial_strike =
        forward * (-d1_signed * vol_sqrt_t + half * volatility * volatility * expiry).exp();

    // Define the objective function: f(K) = computed_pa_delta(K) - target_delta
    // For calls: Δ_pa = e^(-rf×T) × N(d1) × K / F
    // For puts: |Δ_pa| = e^(-rf×T) × N(-d1) × K / F
    let compute_pa_delta = |k: T| -> T {
        let log_fk = (forward / k).ln();
        let d1 = (log_fk + half * volatility * volatility * expiry) / vol_sqrt_t;

        if is_call {
            df_foreign * norm_cdf(d1) * k / forward
        } else {
            df_foreign * norm_cdf(-d1) * k / forward
        }
    };

    // Derivative: d(Δ_pa)/dK
    // For calls: d(Δ_pa)/dK = e^(-rf×T) × [N(d1)/F + n(d1) × K / (F × σ × √T × K)]
    //                       = e^(-rf×T) × [N(d1)/F - n(d1) / (σ × √T × F)]
    let compute_pa_delta_deriv = |k: T| -> T {
        let log_fk = (forward / k).ln();
        let d1 = (log_fk + half * volatility * volatility * expiry) / vol_sqrt_t;
        let dd1_dk = -T::one() / (k * vol_sqrt_t);

        if is_call {
            let nd1 = norm_cdf(d1);
            let npd1 = norm_pdf(d1);
            df_foreign * (nd1 / forward + npd1 * dd1_dk * k / forward)
        } else {
            let nmd1 = norm_cdf(-d1);
            let npd1 = norm_pdf(-d1);
            df_foreign * (nmd1 / forward - npd1 * dd1_dk * k / forward)
        }
    };

    // Newton-Raphson iteration
    let config = SolverConfig {
        tolerance: from_f64(1e-10),
        max_iterations: 50,
    };
    let solver = NewtonRaphsonSolver::new(config);

    let f = |k: T| compute_pa_delta(k) - target_delta;
    let f_prime = compute_pa_delta_deriv;

    solver
        .find_root(f, f_prime, initial_strike)
        .map_err(|e| FormulaError::NumericalInstability {
            message: format!("Newton-Raphson failed for premium-adjusted delta: {:?}", e),
        })
}

/// Converts a strike price to a delta value for the given delta convention.
pub fn strike_to_delta<T: Float>(
    strike: T,
    spot: T,
    domestic_rate: T,
    foreign_rate: T,
    expiry: T,
    volatility: T,
    is_call: bool,
    delta_type: DeltaType,
) -> Result<T, FormulaError> {
    require_positive_spot(spot)?;
    require_positive_strike(strike)?;
    require_positive_vol(volatility)?;
    require_positive_expiry(expiry)?;

    // Forward price: F = S × exp((rd - rf) × T)
    let drift = (domestic_rate - foreign_rate) * expiry;
    let forward = spot * drift.exp();

    let sqrt_t = expiry.sqrt();
    let vol_sqrt_t = volatility * sqrt_t;
    let half: T = from_f64(0.5);

    // Calculate d1: d1 = [ln(F/K) + 0.5×σ²×T] / (σ×√T)
    let log_fk = (forward / strike).ln();
    let d1 = (log_fk + half * volatility * volatility * expiry) / vol_sqrt_t;

    // Calculate delta based on delta type
    let delta = match delta_type {
        DeltaType::SpotPercent | DeltaType::SpotPips => {
            // Δ_call = e^(-rf×T) × N(d1)
            // Δ_put = -e^(-rf×T) × N(-d1) = e^(-rf×T) × (N(d1) - 1)
            let df_foreign = (-foreign_rate * expiry).exp();
            let nd1 = crate::math::normal_dist::norm_cdf(d1);
            if is_call {
                df_foreign * nd1
            } else {
                df_foreign * (nd1 - T::one())
            }
        }
        DeltaType::ForwardPercent | DeltaType::ForwardPips => {
            // Δ_call = N(d1)
            // Δ_put = N(d1) - 1
            let nd1 = crate::math::normal_dist::norm_cdf(d1);
            if is_call {
                nd1
            } else {
                nd1 - T::one()
            }
        }
        DeltaType::PremiumAdjusted => {
            // Premium-adjusted delta: Δ_pa = Δ_spot × K / F
            // For calls: Δ_pa = e^(-rf×T) × N(d1) × K / F
            // For puts: Δ_pa = e^(-rf×T) × (N(d1) - 1) × K / F
            let df_foreign = (-foreign_rate * expiry).exp();
            let nd1 = norm_cdf(d1);
            if is_call {
                df_foreign * nd1 * strike / forward
            } else {
                df_foreign * (nd1 - T::one()) * strike / forward
            }
        }
    };

    Ok(delta)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_delta_to_strike_spot_delta_call() {
        // 25-delta call, EURUSD style parameters
        let delta = 0.25_f64;
        let spot = 1.10;
        let rd = 0.03;
        let rf = 0.01;
        let t = 1.0;
        let vol = 0.10;

        let strike = delta_to_strike(delta, spot, rd, rf, t, vol, DeltaType::SpotPercent).unwrap();

        // Strike should be above forward for OTM call
        let forward = spot * ((rd - rf) * t).exp();
        assert!(strike > forward, "25D call strike should be above forward");

        // Verify round-trip
        let recovered =
            strike_to_delta(strike, spot, rd, rf, t, vol, true, DeltaType::SpotPercent).unwrap();
        assert_relative_eq!(recovered, delta, epsilon = 1e-6);
    }

    #[test]
    fn test_delta_to_strike_spot_delta_put() {
        // 25-delta put (negative delta)
        let delta = -0.25_f64;
        let spot = 1.10;
        let rd = 0.03;
        let rf = 0.01;
        let t = 1.0;
        let vol = 0.10;

        let strike = delta_to_strike(delta, spot, rd, rf, t, vol, DeltaType::SpotPercent).unwrap();

        // Strike should be below forward for OTM put
        let forward = spot * ((rd - rf) * t).exp();
        assert!(strike < forward, "25D put strike should be below forward");

        // Verify round-trip
        let recovered =
            strike_to_delta(strike, spot, rd, rf, t, vol, false, DeltaType::SpotPercent).unwrap();
        assert_relative_eq!(recovered, delta, epsilon = 1e-6);
    }

    #[test]
    fn test_delta_to_strike_atm() {
        // ATM delta ≈ 0.5 * df_foreign
        let spot = 1.10_f64;
        let rd = 0.03;
        let rf = 0.01;
        let t = 1.0;
        let vol = 0.10;

        let df_foreign = (-rf * t).exp();
        let atm_delta = 0.5 * df_foreign;

        let strike =
            delta_to_strike(atm_delta, spot, rd, rf, t, vol, DeltaType::SpotPercent).unwrap();

        // ATM strike should be close to forward
        let forward = spot * ((rd - rf) * t).exp();
        assert_relative_eq!(strike, forward, epsilon = 0.01);
    }

    #[test]
    fn test_delta_to_strike_forward_delta_call() {
        let delta = 0.25_f64;
        let spot = 1.10;
        let rd = 0.03;
        let rf = 0.01;
        let t = 1.0;
        let vol = 0.10;

        let strike = delta_to_strike(delta, spot, rd, rf, t, vol, DeltaType::ForwardPercent).unwrap();

        // Verify round-trip
        let recovered =
            strike_to_delta(strike, spot, rd, rf, t, vol, true, DeltaType::ForwardPercent).unwrap();
        assert_relative_eq!(recovered, delta, epsilon = 1e-6);
    }

    #[test]
    fn test_delta_to_strike_forward_delta_put() {
        let delta = -0.25_f64;
        let spot = 1.10;
        let rd = 0.03;
        let rf = 0.01;
        let t = 1.0;
        let vol = 0.10;

        let strike = delta_to_strike(delta, spot, rd, rf, t, vol, DeltaType::ForwardPercent).unwrap();

        // Verify round-trip
        let recovered =
            strike_to_delta(strike, spot, rd, rf, t, vol, false, DeltaType::ForwardPercent).unwrap();
        assert_relative_eq!(recovered, delta, epsilon = 1e-6);
    }

    #[test]
    fn test_invalid_spot() {
        let result = delta_to_strike(0.25, 0.0, 0.03, 0.01, 1.0, 0.10, DeltaType::SpotPercent);
        assert!(matches!(result, Err(FormulaError::InvalidSpot { .. })));

        let result = delta_to_strike(0.25, -1.0, 0.03, 0.01, 1.0, 0.10, DeltaType::SpotPercent);
        assert!(matches!(result, Err(FormulaError::InvalidSpot { .. })));
    }

    #[test]
    fn test_invalid_volatility() {
        let result = delta_to_strike(0.25, 1.10, 0.03, 0.01, 1.0, 0.0, DeltaType::SpotPercent);
        assert!(matches!(
            result,
            Err(FormulaError::InvalidVolatility { .. })
        ));

        let result = delta_to_strike(0.25, 1.10, 0.03, 0.01, 1.0, -0.10, DeltaType::SpotPercent);
        assert!(matches!(
            result,
            Err(FormulaError::InvalidVolatility { .. })
        ));
    }

    #[test]
    fn test_invalid_expiry() {
        let result = delta_to_strike(0.25, 1.10, 0.03, 0.01, 0.0, 0.10, DeltaType::SpotPercent);
        assert!(matches!(result, Err(FormulaError::InvalidExpiry { .. })));

        let result = delta_to_strike(0.25, 1.10, 0.03, 0.01, -1.0, 0.10, DeltaType::SpotPercent);
        assert!(matches!(result, Err(FormulaError::InvalidExpiry { .. })));
    }

    #[test]
    fn test_strike_to_delta_invalid_strike() {
        let result = strike_to_delta(0.0, 1.10, 0.03, 0.01, 1.0, 0.10, true, DeltaType::SpotPercent);
        assert!(matches!(result, Err(FormulaError::InvalidStrike { .. })));
    }

    #[test]
    fn test_extreme_delta_values() {
        let spot = 1.10_f64;
        let rd = 0.03;
        let rf = 0.01;
        let t = 1.0;
        let vol = 0.10;

        // Very deep ITM call (high delta)
        let high_delta = 0.90;
        let result = delta_to_strike(high_delta, spot, rd, rf, t, vol, DeltaType::SpotPercent);
        assert!(result.is_ok());

        // Very deep OTM call (low delta)
        let low_delta = 0.05;
        let result = delta_to_strike(low_delta, spot, rd, rf, t, vol, DeltaType::SpotPercent);
        assert!(result.is_ok());
    }

    #[test]
    fn test_short_expiry() {
        let spot = 1.10_f64;
        let rd = 0.03;
        let rf = 0.01;
        let t = 0.01; // ~3.6 days
        let vol = 0.10;

        let strike = delta_to_strike(0.25, spot, rd, rf, t, vol, DeltaType::SpotPercent).unwrap();
        assert!(strike.is_finite());

        let recovered =
            strike_to_delta(strike, spot, rd, rf, t, vol, true, DeltaType::SpotPercent).unwrap();
        assert_relative_eq!(recovered, 0.25, epsilon = 1e-6);
    }

    #[test]
    fn test_high_volatility() {
        let spot = 1.10_f64;
        let rd = 0.03;
        let rf = 0.01;
        let t = 1.0;
        let vol = 0.50; // 50% vol

        let strike = delta_to_strike(0.25, spot, rd, rf, t, vol, DeltaType::SpotPercent).unwrap();
        assert!(strike.is_finite());

        let recovered =
            strike_to_delta(strike, spot, rd, rf, t, vol, true, DeltaType::SpotPercent).unwrap();
        assert_relative_eq!(recovered, 0.25, epsilon = 1e-6);
    }

    #[test]
    fn test_spot_vs_forward_delta_difference() {
        let spot = 1.10_f64;
        let rd = 0.03;
        let rf = 0.01;
        let t = 1.0;
        let vol = 0.10;

        let strike_spot =
            delta_to_strike(0.25, spot, rd, rf, t, vol, DeltaType::SpotPercent).unwrap();
        let strike_fwd =
            delta_to_strike(0.25, spot, rd, rf, t, vol, DeltaType::ForwardPercent).unwrap();

        // They should be different (unless rf = 0)
        assert!((strike_spot - strike_fwd).abs() > 1e-6);
    }

    #[test]
    fn test_delta_to_strike_premium_adjusted_call() {
        // 25-delta call with premium-adjusted convention (USDJPY style)
        let delta = 0.25_f64;
        let spot = 150.0; // USDJPY
        let rd = 0.05; // USD rate
        let rf = 0.01; // JPY rate
        let t = 1.0;
        let vol = 0.10;

        let strike =
            delta_to_strike(delta, spot, rd, rf, t, vol, DeltaType::PremiumAdjusted).unwrap();

        // Verify round-trip
        let recovered = strike_to_delta(
            strike,
            spot,
            rd,
            rf,
            t,
            vol,
            true,
            DeltaType::PremiumAdjusted,
        )
        .unwrap();
        assert_relative_eq!(recovered, delta, epsilon = 1e-6);
    }

    #[test]
    fn test_delta_to_strike_premium_adjusted_put() {
        // 25-delta put with premium-adjusted convention
        let delta = -0.25_f64;
        let spot = 150.0;
        let rd = 0.05;
        let rf = 0.01;
        let t = 1.0;
        let vol = 0.10;

        let strike =
            delta_to_strike(delta, spot, rd, rf, t, vol, DeltaType::PremiumAdjusted).unwrap();

        // Verify round-trip
        let recovered = strike_to_delta(
            strike,
            spot,
            rd,
            rf,
            t,
            vol,
            false,
            DeltaType::PremiumAdjusted,
        )
        .unwrap();
        assert_relative_eq!(recovered, delta, epsilon = 1e-6);
    }

    #[test]
    fn test_premium_adjusted_vs_spot_delta_difference() {
        let spot = 150.0_f64;
        let rd = 0.05;
        let rf = 0.01;
        let t = 1.0;
        let vol = 0.10;

        let strike_spot =
            delta_to_strike(0.25, spot, rd, rf, t, vol, DeltaType::SpotPercent).unwrap();
        let strike_pa =
            delta_to_strike(0.25, spot, rd, rf, t, vol, DeltaType::PremiumAdjusted).unwrap();

        // Premium-adjusted delta should give different strike
        // (higher strike for calls, as premium adjustment reduces delta)
        assert!((strike_spot - strike_pa).abs() > 0.1);
    }

    #[test]
    fn test_premium_adjusted_atm() {
        // Near-ATM should work correctly
        let spot = 150.0_f64;
        let rd = 0.05;
        let rf = 0.01;
        let t = 1.0;
        let vol = 0.10;

        // ATM delta is around 0.5 for calls
        let atm_delta = 0.45;
        let strike =
            delta_to_strike(atm_delta, spot, rd, rf, t, vol, DeltaType::PremiumAdjusted).unwrap();
        assert!(strike.is_finite());

        let recovered = strike_to_delta(
            strike,
            spot,
            rd,
            rf,
            t,
            vol,
            true,
            DeltaType::PremiumAdjusted,
        )
        .unwrap();
        assert_relative_eq!(recovered, atm_delta, epsilon = 1e-5);
    }
}
