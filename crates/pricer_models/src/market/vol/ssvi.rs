//! SSVI volatility surface (Gatheral & Jacquier 2014).
//!
//! A single global parameter set `(ρ, η, γ)` combined with a term-structure
//! of ATM total variances provides a calendar-spread-arbitrage-free surface.

use pricer_core::{
    math::formulas::ssvi::{ssvi_implied_vol, SsviParams},
    traits::Float,
};

use super::{
    interp::{find_bracket, linear_interp},
    VolSurface, VolSurfaceError,
};

/// Re-export global params.
pub type SsviGlobalParams<T> = SsviParams<T>;

/// A full SSVI surface: global `(ρ, η, γ)` plus ATM total-variance
/// term-structure.
#[derive(Clone, Debug)]
pub struct SsviSurface<T: Float> {
    /// (expiry, θ_T = σ²_ATM · T) pairs, sorted by expiry.
    atm_total_variances: Vec<(T, T)>,
    /// Global SSVI parameters.
    params: SsviParams<T>,
    allow_extrapolation: bool,
}

impl<T: Float> SsviSurface<T> {
    /// Constructs the surface.
    ///
    /// `atm_total_variances` must be sorted by expiry and non-decreasing in θ
    /// (calendar-spread-arbitrage-free condition).
    pub fn new(
        atm_total_variances: Vec<(T, T)>,
        params: SsviParams<T>,
    ) -> Result<Self, VolSurfaceError> {
        if atm_total_variances.is_empty() {
            return Err(VolSurfaceError::InvalidInput(
                "atm_total_variances must be non-empty".to_string(),
            ));
        }
        // Verify non-decreasing θ
        for i in 1..atm_total_variances.len() {
            if atm_total_variances[i].1 < atm_total_variances[i - 1].1 {
                return Err(VolSurfaceError::InvalidInput(
                    "ATM total variances must be non-decreasing for arbitrage-free SSVI"
                        .to_string(),
                ));
            }
        }
        Ok(Self {
            atm_total_variances,
            params,
            allow_extrapolation: false,
        })
    }

    /// Enables or disables extrapolation.
    pub fn with_extrapolation(mut self, allow: bool) -> Self {
        self.allow_extrapolation = allow;
        self
    }

    /// Returns the global parameters.
    pub fn params(&self) -> &SsviParams<T> { &self.params }

    /// Returns the ATM total-variance term-structure.
    pub fn atm_total_variances(&self) -> &[(T, T)] { &self.atm_total_variances }

    /// Interpolates the ATM total variance at the target expiry.
    fn interpolate_atm_var(&self, expiry: T) -> Result<(T, T), VolSurfaceError> {
        let n = self.atm_total_variances.len();
        let expiries: Vec<T> = self.atm_total_variances.iter().map(|&(t, _)| t).collect();

        if n == 1 {
            let (t0, theta0) = self.atm_total_variances[0];
            // Scale linearly: θ(T) = θ₀ · (T / T₀)
            let eps = T::from(1e-14).unwrap_or_else(|| T::epsilon());
            let theta = if t0 > eps {
                theta0 * expiry / t0
            } else {
                theta0
            };
            return Ok((expiry, theta));
        }

        let (lo, hi) = find_bracket(&expiries, expiry);

        if lo == hi {
            if !self.allow_extrapolation && lo == 0 && expiry < expiries[0] {
                return Err(VolSurfaceError::ExtrapolationNotAllowed);
            }
            if !self.allow_extrapolation && lo == n - 1 && expiry > expiries[n - 1] {
                return Err(VolSurfaceError::ExtrapolationNotAllowed);
            }
            return Ok((expiry, self.atm_total_variances[lo].1));
        }

        let t_lo = expiries[lo];
        let t_hi = expiries[hi];
        let theta_lo = self.atm_total_variances[lo].1;
        let theta_hi = self.atm_total_variances[hi].1;

        // Linear interpolation in total variance (ensures non-decreasing)
        let theta = linear_interp(t_lo, theta_lo, t_hi, theta_hi, expiry);

        Ok((expiry, theta))
    }
}

impl<T: Float> VolSurface<T> for SsviSurface<T> {
    fn implied_vol(&self, strike: T, expiry: T, forward: T) -> Result<T, VolSurfaceError> {
        let (_, theta) = self.interpolate_atm_var(expiry)?;

        // Recover ATM vol: σ_ATM = √(θ / T)
        let eps = T::from(1e-14).unwrap_or_else(|| T::epsilon());
        let atm_vol = if expiry > eps {
            (theta / expiry).sqrt()
        } else {
            return Err(VolSurfaceError::InvalidInput(
                "expiry too small".to_string(),
            ));
        };

        ssvi_implied_vol(&self.params, strike, forward, expiry, atm_vol)
            .map_err(|e| VolSurfaceError::InvalidInput(format!("{e}")))
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn test_surface() -> SsviSurface<f64> {
        SsviSurface::new(
            vec![(0.5, 0.02), (1.0, 0.04), (2.0, 0.08)],
            SsviParams {
                rho: -0.3,
                eta: 1.0,
                gamma: 0.5,
            },
        )
        .unwrap()
        .with_extrapolation(true)
    }

    #[test]
    fn test_construction_valid() {
        assert!(test_surface().implied_vol(100.0, 1.0, 100.0).is_ok());
    }

    #[test]
    fn test_construction_empty() {
        let r = SsviSurface::<f64>::new(
            vec![],
            SsviParams {
                rho: -0.3,
                eta: 1.0,
                gamma: 0.5,
            },
        );
        assert!(r.is_err());
    }

    #[test]
    fn test_non_decreasing_violation() {
        let r = SsviSurface::<f64>::new(
            vec![(1.0, 0.04), (2.0, 0.02)], // decreasing θ
            SsviParams {
                rho: -0.3,
                eta: 1.0,
                gamma: 0.5,
            },
        );
        assert!(r.is_err());
    }

    #[test]
    fn test_atm_vol_recovery() {
        let s = test_surface();
        let vol = s.implied_vol(100.0, 1.0, 100.0).unwrap();
        // θ = 0.04 at T=1 → σ_ATM = √0.04 = 0.2
        assert_relative_eq!(vol, 0.2, epsilon = 1e-4);
    }

    #[test]
    fn test_smile_shape() {
        let s = test_surface();
        let vol_low = s.implied_vol(90.0, 1.0, 100.0).unwrap();
        let vol_high = s.implied_vol(110.0, 1.0, 100.0).unwrap();
        assert!(vol_low > vol_high, "Expected negative skew");
    }

    #[test]
    fn test_interpolated_expiry() {
        let s = test_surface();
        let vol = s.implied_vol(100.0, 0.75, 100.0).unwrap();
        assert!(vol > 0.0 && vol < 1.0);
    }
}
