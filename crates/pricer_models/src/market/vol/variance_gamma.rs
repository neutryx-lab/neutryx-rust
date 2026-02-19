//! Variance Gamma volatility surface (Madan, Carr & Chang 1998).
//!
//! Wraps [`pricer_core::math::formulas::variance_gamma`] with term-structure
//! interpolation. Captures fat tails and skew for FX/Equity.

use infra_domain::market::definition::TimeInterpolation;
use pricer_core::{
    math::formulas::variance_gamma::{vg_implied_vol, VarianceGammaParams},
    traits::Float,
};

use super::{
    interp::{find_bracket, linear_interp},
    VolSurface, VolSurfaceError,
};

/// Re-export slice params.
pub type VgSliceParams<T> = VarianceGammaParams<T>;

/// A term-structure Variance Gamma surface.
#[derive(Clone, Debug)]
pub struct VarianceGammaSurface<T: Float> {
    expiries: Vec<T>,
    params: Vec<VarianceGammaParams<T>>,
    /// Forward at each expiry.
    forwards: Vec<T>,
    time_interpolation: TimeInterpolation,
    allow_extrapolation: bool,
}

impl<T: Float> VarianceGammaSurface<T> {
    /// Constructs from per-expiry VG parameters.
    pub fn from_calibrated_slices(
        expiries: Vec<T>,
        params: Vec<VarianceGammaParams<T>>,
        forwards: Vec<T>,
        time_interpolation: TimeInterpolation,
    ) -> Result<Self, VolSurfaceError> {
        if expiries.is_empty() {
            return Err(VolSurfaceError::InvalidInput(
                "expiries must be non-empty".to_string(),
            ));
        }
        if expiries.len() != params.len() || expiries.len() != forwards.len() {
            return Err(VolSurfaceError::InvalidInput(
                "expiries, params, and forwards must have equal length".to_string(),
            ));
        }
        Ok(Self {
            expiries,
            params,
            forwards,
            time_interpolation,
            allow_extrapolation: false,
        })
    }

    /// Enables or disables extrapolation.
    pub fn with_extrapolation(mut self, allow: bool) -> Self {
        self.allow_extrapolation = allow;
        self
    }

    /// Returns the expiry grid.
    pub fn expiries(&self) -> &[T] { &self.expiries }

    /// Returns the time interpolation method.
    pub fn time_interpolation(&self) -> TimeInterpolation { self.time_interpolation }

    /// Interpolates VG parameters to the target expiry.
    fn interpolate_params(
        &self,
        expiry: T,
    ) -> Result<(VarianceGammaParams<T>, T), VolSurfaceError> {
        let n = self.expiries.len();

        if n == 1 {
            return Ok((self.params[0], self.forwards[0]));
        }

        let (lo, hi) = find_bracket(&self.expiries, expiry);

        if lo == hi {
            if !self.allow_extrapolation && lo == 0 && expiry < self.expiries[0] {
                return Err(VolSurfaceError::ExtrapolationNotAllowed);
            }
            if !self.allow_extrapolation && lo == n - 1 && expiry > self.expiries[n - 1] {
                return Err(VolSurfaceError::ExtrapolationNotAllowed);
            }
            return Ok((self.params[lo], self.forwards[lo]));
        }

        let t_lo = self.expiries[lo];
        let t_hi = self.expiries[hi];
        let p_lo = &self.params[lo];
        let p_hi = &self.params[hi];

        let sigma = linear_interp(t_lo, p_lo.sigma, t_hi, p_hi.sigma, expiry);
        let nu = linear_interp(t_lo, p_lo.nu, t_hi, p_hi.nu, expiry);
        let theta = linear_interp(t_lo, p_lo.theta, t_hi, p_hi.theta, expiry);
        let fwd = linear_interp(t_lo, self.forwards[lo], t_hi, self.forwards[hi], expiry);

        Ok((VarianceGammaParams { sigma, nu, theta }, fwd))
    }
}

impl<T: Float> VolSurface<T> for VarianceGammaSurface<T> {
    fn implied_vol(&self, strike: T, expiry: T, forward: T) -> Result<T, VolSurfaceError> {
        let (params, _fwd) = self.interpolate_params(expiry)?;
        vg_implied_vol(&params, forward, strike, expiry)
            .map_err(|e| VolSurfaceError::InvalidInput(format!("{e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construction_valid() {
        let s = VarianceGammaSurface::from_calibrated_slices(
            vec![1.0],
            vec![VarianceGammaParams {
                sigma: 0.20,
                nu: 0.25,
                theta: -0.10,
            }],
            vec![100.0],
            TimeInterpolation::LinearVol,
        );
        assert!(s.is_ok());
    }

    #[test]
    fn test_construction_empty() {
        let r = VarianceGammaSurface::<f64>::from_calibrated_slices(
            vec![],
            vec![],
            vec![],
            TimeInterpolation::LinearVol,
        );
        assert!(r.is_err());
    }

    #[test]
    fn test_implied_vol_positive() {
        let s = VarianceGammaSurface::from_calibrated_slices(
            vec![1.0],
            vec![VarianceGammaParams {
                sigma: 0.20,
                nu: 0.25,
                theta: -0.10,
            }],
            vec![100.0],
            TimeInterpolation::LinearVol,
        )
        .unwrap()
        .with_extrapolation(true);

        let vol = s.implied_vol(100.0, 1.0, 100.0).unwrap();
        assert!(vol > 0.0 && vol < 1.0, "Vol {} out of range", vol);
    }
}
