//! Mixture of Lognormals volatility surface for FX markets.
//!
//! Wraps [`pricer_core::math::formulas::mixture_lognormal`] with
//! term-structure interpolation. Captures bimodal FX distributions.

use infra_domain::market::definition::TimeInterpolation;
use pricer_core::{
    math::formulas::mixture_lognormal::{mixture_lognormal_implied_vol, MixtureLognormalParams},
    traits::Float,
};

use super::{interp::find_bracket, VolSurface, VolSurfaceError};

/// A term-structure mixture-lognormal surface.
#[derive(Clone, Debug)]
pub struct MixtureLognormalSurface<T: Float> {
    expiries: Vec<T>,
    params: Vec<MixtureLognormalParams<T>>,
    /// Overall forward at each expiry (for implied vol inversion).
    forwards: Vec<T>,
    time_interpolation: TimeInterpolation,
    allow_extrapolation: bool,
}

impl<T: Float> MixtureLognormalSurface<T> {
    /// Constructs the surface from per-expiry mixture parameters.
    pub fn from_calibrated_slices(
        expiries: Vec<T>,
        params: Vec<MixtureLognormalParams<T>>,
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

    /// Returns the forward rates.
    pub fn forwards(&self) -> &[T] { &self.forwards }

    /// Returns the time interpolation method.
    pub fn time_interpolation(&self) -> TimeInterpolation { self.time_interpolation }

    /// Returns the closest slice for the target expiry.
    fn find_slice(&self, expiry: T) -> Result<usize, VolSurfaceError> {
        let n = self.expiries.len();

        if n == 1 {
            return Ok(0);
        }

        let (lo, hi) = find_bracket(&self.expiries, expiry);

        if lo == hi {
            if !self.allow_extrapolation && lo == 0 && expiry < self.expiries[0] {
                return Err(VolSurfaceError::ExtrapolationNotAllowed);
            }
            if !self.allow_extrapolation && lo == n - 1 && expiry > self.expiries[n - 1] {
                return Err(VolSurfaceError::ExtrapolationNotAllowed);
            }
            return Ok(lo);
        }

        // Pick the closest slice (mixture params are hard to interpolate)
        let t_lo = self.expiries[lo];
        let t_hi = self.expiries[hi];
        let mid = (t_lo + t_hi) / (T::one() + T::one());
        if expiry <= mid {
            Ok(lo)
        } else {
            Ok(hi)
        }
    }
}

impl<T: Float> VolSurface<T> for MixtureLognormalSurface<T> {
    fn implied_vol(&self, strike: T, expiry: T, forward: T) -> Result<T, VolSurfaceError> {
        let idx = self.find_slice(expiry)?;
        mixture_lognormal_implied_vol(&self.params[idx], strike, forward, expiry)
            .map_err(|e| VolSurfaceError::InvalidInput(format!("{e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_component() {
        let params = MixtureLognormalParams::new(vec![1.0], vec![100.0], vec![0.20]).unwrap();

        let s = MixtureLognormalSurface::from_calibrated_slices(
            vec![1.0],
            vec![params],
            vec![100.0],
            TimeInterpolation::LinearVol,
        )
        .unwrap();

        let vol = s.implied_vol(100.0, 1.0, 100.0).unwrap();
        assert!((vol - 0.20).abs() < 0.01);
    }

    #[test]
    fn test_bimodal() {
        let params =
            MixtureLognormalParams::new(vec![0.6, 0.4], vec![100.0, 100.0], vec![0.15, 0.30])
                .unwrap();

        let s = MixtureLognormalSurface::from_calibrated_slices(
            vec![1.0],
            vec![params],
            vec![100.0],
            TimeInterpolation::LinearVol,
        )
        .unwrap();

        let vol = s.implied_vol(100.0, 1.0, 100.0).unwrap();
        assert!(vol > 0.10 && vol < 0.35);
    }

    #[test]
    fn test_empty() {
        let r = MixtureLognormalSurface::<f64>::from_calibrated_slices(
            vec![],
            vec![],
            vec![],
            TimeInterpolation::LinearVol,
        );
        assert!(r.is_err());
    }
}
