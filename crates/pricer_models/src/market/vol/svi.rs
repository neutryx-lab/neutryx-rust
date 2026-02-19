//! SVI volatility surface with time-interpolated parameter slices.
//!
//! Wraps the Gatheral (2004) SVI formula from
//! [`pricer_core::math::formulas::svi`] and provides term-structure
//! interpolation across calibrated expiry slices.

use infra_domain::market::definition::TimeInterpolation;
use pricer_core::{
    math::formulas::svi::{svi_implied_vol, SviParams},
    traits::Float,
};

use super::{
    interp::{find_bracket, linear_interp},
    VolSurface, VolSurfaceError,
};

/// Re-export slice params for convenience.
pub type SviSliceParams<T> = SviParams<T>;

/// A term-structure SVI surface built from calibrated slices.
///
/// Each expiry has its own [`SviParams`]; the surface interpolates
/// across expiries using the chosen [`TimeInterpolation`] method.
#[derive(Clone, Debug)]
pub struct SviSurface<T: Float> {
    expiries: Vec<T>,
    params: Vec<SviParams<T>>,
    time_interpolation: TimeInterpolation,
    allow_extrapolation: bool,
}

impl<T: Float> SviSurface<T> {
    /// Constructs the surface from pre-calibrated slices.
    pub fn from_calibrated_slices(
        expiries: Vec<T>,
        params: Vec<SviParams<T>>,
        time_interpolation: TimeInterpolation,
    ) -> Result<Self, VolSurfaceError> {
        if expiries.is_empty() {
            return Err(VolSurfaceError::InvalidInput(
                "expiries must be non-empty".to_string(),
            ));
        }
        if expiries.len() != params.len() {
            return Err(VolSurfaceError::InvalidInput(format!(
                "expiries length {} does not match params length {}",
                expiries.len(),
                params.len(),
            )));
        }
        Ok(Self {
            expiries,
            params,
            time_interpolation,
            allow_extrapolation: false,
        })
    }

    /// Enables or disables extrapolation.
    pub fn with_extrapolation(mut self, allow: bool) -> Self {
        self.allow_extrapolation = allow;
        self
    }

    /// Returns the time-interpolation method.
    pub fn time_interpolation(&self) -> TimeInterpolation { self.time_interpolation }

    /// Returns a reference to the expiry grid.
    pub fn expiries(&self) -> &[T] { &self.expiries }

    /// Returns a reference to the per-slice parameters.
    pub fn params(&self) -> &[SviParams<T>] { &self.params }

    /// Interpolates SVI parameters to the target expiry.
    fn interpolate_params(&self, expiry: T) -> Result<SviParams<T>, VolSurfaceError> {
        let n = self.expiries.len();

        if n == 1 {
            return Ok(self.params[0]);
        }

        let (lo, hi) = find_bracket(&self.expiries, expiry);

        if lo == hi {
            if !self.allow_extrapolation && lo == 0 && expiry < self.expiries[0] {
                return Err(VolSurfaceError::ExtrapolationNotAllowed);
            }
            if !self.allow_extrapolation && lo == n - 1 && expiry > self.expiries[n - 1] {
                return Err(VolSurfaceError::ExtrapolationNotAllowed);
            }
            return Ok(self.params[lo]);
        }

        let t_lo = self.expiries[lo];
        let t_hi = self.expiries[hi];
        let p_lo = &self.params[lo];
        let p_hi = &self.params[hi];

        // Interpolate each SVI parameter linearly.
        // For LinearVariance mode, interpolate `a` in total-variance space.
        match self.time_interpolation {
            TimeInterpolation::LinearVariance => {
                let a_var_lo = p_lo.a * t_lo;
                let a_var_hi = p_hi.a * t_hi;
                let a_var = linear_interp(t_lo, a_var_lo, t_hi, a_var_hi, expiry);

                let eps = T::from(1e-14).unwrap_or_else(|| T::epsilon());
                let a = if expiry > eps { a_var / expiry } else { p_lo.a };

                Ok(SviParams {
                    a,
                    b: linear_interp(t_lo, p_lo.b, t_hi, p_hi.b, expiry),
                    rho: linear_interp(t_lo, p_lo.rho, t_hi, p_hi.rho, expiry),
                    m: linear_interp(t_lo, p_lo.m, t_hi, p_hi.m, expiry),
                    sigma: linear_interp(t_lo, p_lo.sigma, t_hi, p_hi.sigma, expiry),
                })
            }
            TimeInterpolation::LinearVol | TimeInterpolation::FlatForward => Ok(SviParams {
                a: linear_interp(t_lo, p_lo.a, t_hi, p_hi.a, expiry),
                b: linear_interp(t_lo, p_lo.b, t_hi, p_hi.b, expiry),
                rho: linear_interp(t_lo, p_lo.rho, t_hi, p_hi.rho, expiry),
                m: linear_interp(t_lo, p_lo.m, t_hi, p_hi.m, expiry),
                sigma: linear_interp(t_lo, p_lo.sigma, t_hi, p_hi.sigma, expiry),
            }),
        }
    }
}

impl<T: Float> VolSurface<T> for SviSurface<T> {
    fn implied_vol(&self, strike: T, expiry: T, forward: T) -> Result<T, VolSurfaceError> {
        let slice = self.interpolate_params(expiry)?;
        svi_implied_vol(&slice, strike, forward, expiry)
            .map_err(|e| VolSurfaceError::InvalidInput(format!("{e}")))
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn single_slice() -> SviSurface<f64> {
        SviSurface::from_calibrated_slices(
            vec![1.0],
            vec![SviParams {
                a: 0.04,
                b: 0.4,
                rho: -0.4,
                m: 0.0,
                sigma: 0.1,
            }],
            TimeInterpolation::LinearVariance,
        )
        .unwrap()
        .with_extrapolation(true)
    }

    #[test]
    fn test_construction_valid() {
        assert!(single_slice().implied_vol(100.0, 1.0, 100.0).is_ok());
    }

    #[test]
    fn test_construction_empty() {
        let r = SviSurface::<f64>::from_calibrated_slices(
            vec![],
            vec![],
            TimeInterpolation::LinearVariance,
        );
        assert!(r.is_err());
    }

    #[test]
    fn test_atm_vol_positive() {
        let s = single_slice();
        let vol = s.implied_vol(100.0, 1.0, 100.0).unwrap();
        assert!(vol > 0.0 && vol < 1.0);
    }

    #[test]
    fn test_smile_shape() {
        let s = single_slice();
        let vol_low = s.implied_vol(90.0, 1.0, 100.0).unwrap();
        let vol_high = s.implied_vol(110.0, 1.0, 100.0).unwrap();
        assert!(vol_low > vol_high);
    }

    #[test]
    fn test_multi_slice_interpolation() {
        let s = SviSurface::from_calibrated_slices(
            vec![0.5, 2.0],
            vec![
                SviParams {
                    a: 0.02,
                    b: 0.3,
                    rho: -0.3,
                    m: 0.0,
                    sigma: 0.1,
                },
                SviParams {
                    a: 0.06,
                    b: 0.5,
                    rho: -0.5,
                    m: 0.0,
                    sigma: 0.1,
                },
            ],
            TimeInterpolation::LinearVol,
        )
        .unwrap()
        .with_extrapolation(true);

        let vol = s.implied_vol(100.0, 1.0, 100.0).unwrap();
        assert!(vol > 0.0);
    }

    #[test]
    fn test_extrapolation_not_allowed() {
        let s = SviSurface::from_calibrated_slices(
            vec![0.5, 1.0],
            vec![
                SviParams {
                    a: 0.04,
                    b: 0.4,
                    rho: -0.4,
                    m: 0.0,
                    sigma: 0.1,
                },
                SviParams {
                    a: 0.04,
                    b: 0.4,
                    rho: -0.4,
                    m: 0.0,
                    sigma: 0.1,
                },
            ],
            TimeInterpolation::LinearVariance,
        )
        .unwrap();
        assert!(s.implied_vol(100.0, 0.1, 100.0).is_err());
    }

    #[test]
    fn test_matches_direct_formula() {
        let s = single_slice();
        let surface_vol = s.implied_vol(105.0, 1.0, 100.0).unwrap();
        let direct_vol = svi_implied_vol(
            &SviParams {
                a: 0.04,
                b: 0.4,
                rho: -0.4,
                m: 0.0,
                sigma: 0.1,
            },
            105.0,
            100.0,
            1.0,
        )
        .unwrap();
        assert_relative_eq!(surface_vol, direct_vol, epsilon = 1e-12);
    }
}
