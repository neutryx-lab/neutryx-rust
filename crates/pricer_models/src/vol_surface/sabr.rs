//! SABR volatility surface with time-interpolated parameter slices.
//!
//! Wraps the Hagan (2002) analytical formula from
//! [`pricer_core::math::formulas::sabr::sabr_implied_vol`] and provides
//! term-structure interpolation across calibrated expiry slices.

use pricer_core::math::formulas::sabr::{sabr_implied_vol, SabrImpliedVolParams};
use pricer_core::traits::Float;

use infra_domain::market::definition::TimeInterpolation;

use super::interp::{find_bracket, linear_interp};
use super::{VolSurface, VolSurfaceError};

/// Parameters for a single SABR smile slice at a given expiry.
#[derive(Clone, Debug)]
pub struct SabrSliceParams<T: Float> {
    /// Initial volatility level (alpha).
    pub alpha: T,
    /// CEV exponent controlling the backbone (0 = normal, 1 = log-normal).
    pub beta: T,
    /// Correlation between the forward and its volatility.
    pub rho: T,
    /// Volatility-of-volatility.
    pub nu: T,
}

/// Convenience alias matching the plan naming convention.
pub type SabrParams<T> = SabrSliceParams<T>;

/// A term-structure SABR surface built from calibrated slices.
///
/// Each expiry has its own [`SabrSliceParams`]; the surface interpolates
/// across expiries using the chosen [`TimeInterpolation`] method before
/// evaluating the Hagan formula.
#[derive(Clone, Debug)]
pub struct SabrSurface<T: Float> {
    expiries: Vec<T>,
    params: Vec<SabrSliceParams<T>>,
    time_interpolation: TimeInterpolation,
    allow_extrapolation: bool,
}

impl<T: Float> SabrSurface<T> {
    /// Constructs the surface from pre-calibrated slices.
    ///
    /// `expiries` and `params` must have the same length and at least one element.
    pub fn from_calibrated_slices(
        expiries: Vec<T>,
        params: Vec<SabrSliceParams<T>>,
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

    /// Enables or disables extrapolation beyond the calibrated region.
    pub fn with_extrapolation(mut self, allow: bool) -> Self {
        self.allow_extrapolation = allow;
        self
    }

    /// Returns the time-interpolation method.
    pub fn time_interpolation(&self) -> TimeInterpolation {
        self.time_interpolation
    }

    /// Returns a reference to the expiry grid.
    pub fn expiries(&self) -> &[T] {
        &self.expiries
    }

    /// Returns a reference to the per-slice parameters.
    pub fn params(&self) -> &[SabrSliceParams<T>] {
        &self.params
    }

    /// Interpolates SABR parameters to the target expiry.
    ///
    /// For a single-slice surface, returns the slice parameters directly.
    /// For multi-slice, finds the bracket and interpolates each parameter
    /// independently using the configured [`TimeInterpolation`] method.
    fn interpolate_params(&self, expiry: T) -> Result<SabrSliceParams<T>, VolSurfaceError> {
        let n = self.expiries.len();

        // Single-slice: no interpolation needed
        if n == 1 {
            return Ok(self.params[0].clone());
        }

        let (lo, hi) = find_bracket(&self.expiries, expiry);

        // Boundary: clamped to first or last slice
        if lo == hi {
            if !self.allow_extrapolation && lo == 0 && expiry < self.expiries[0] {
                return Err(VolSurfaceError::ExtrapolationNotAllowed);
            }
            if !self.allow_extrapolation && lo == n - 1 && expiry > self.expiries[n - 1] {
                return Err(VolSurfaceError::ExtrapolationNotAllowed);
            }
            return Ok(self.params[lo].clone());
        }

        let t_lo = self.expiries[lo];
        let t_hi = self.expiries[hi];
        let p_lo = &self.params[lo];
        let p_hi = &self.params[hi];

        match self.time_interpolation {
            TimeInterpolation::LinearVariance => {
                // Interpolate in total-variance space: alpha^2 * t
                let var_lo = p_lo.alpha * p_lo.alpha * t_lo;
                let var_hi = p_hi.alpha * p_hi.alpha * t_hi;
                let var_interp = linear_interp(t_lo, var_lo, t_hi, var_hi, expiry);

                let eps = T::from(1e-14).unwrap_or_else(|| T::epsilon());
                let alpha = if expiry > eps {
                    (var_interp / expiry).abs().sqrt()
                } else {
                    p_lo.alpha
                };

                let rho = linear_interp(t_lo, p_lo.rho, t_hi, p_hi.rho, expiry);
                let nu = linear_interp(t_lo, p_lo.nu, t_hi, p_hi.nu, expiry);
                let beta = linear_interp(t_lo, p_lo.beta, t_hi, p_hi.beta, expiry);

                Ok(SabrSliceParams { alpha, beta, rho, nu })
            }
            TimeInterpolation::LinearVol | TimeInterpolation::FlatForward => {
                let alpha = linear_interp(t_lo, p_lo.alpha, t_hi, p_hi.alpha, expiry);
                let beta = linear_interp(t_lo, p_lo.beta, t_hi, p_hi.beta, expiry);
                let rho = linear_interp(t_lo, p_lo.rho, t_hi, p_hi.rho, expiry);
                let nu = linear_interp(t_lo, p_lo.nu, t_hi, p_hi.nu, expiry);

                Ok(SabrSliceParams { alpha, beta, rho, nu })
            }
        }
    }
}

impl<T: Float> VolSurface<T> for SabrSurface<T> {
    fn implied_vol(&self, strike: T, expiry: T, forward: T) -> Result<T, VolSurfaceError> {
        let slice = self.interpolate_params(expiry)?;

        let sabr_params = SabrImpliedVolParams {
            forward,
            alpha: slice.alpha,
            beta: slice.beta,
            nu: slice.nu,
            rho: slice.rho,
            maturity: expiry,
        };

        sabr_implied_vol(&sabr_params, strike)
            .map_err(|e| VolSurfaceError::SabrError(format!("{e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn single_slice_surface() -> SabrSurface<f64> {
        SabrSurface::from_calibrated_slices(
            vec![1.0],
            vec![SabrSliceParams {
                alpha: 0.2,
                beta: 0.5,
                rho: -0.3,
                nu: 0.4,
            }],
            TimeInterpolation::LinearVariance,
        )
        .unwrap()
        .with_extrapolation(true)
    }

    fn multi_slice_surface() -> SabrSurface<f64> {
        SabrSurface::from_calibrated_slices(
            vec![0.5, 1.0, 2.0],
            vec![
                SabrSliceParams {
                    alpha: 0.18,
                    beta: 0.5,
                    rho: -0.25,
                    nu: 0.35,
                },
                SabrSliceParams {
                    alpha: 0.20,
                    beta: 0.5,
                    rho: -0.30,
                    nu: 0.40,
                },
                SabrSliceParams {
                    alpha: 0.22,
                    beta: 0.5,
                    rho: -0.35,
                    nu: 0.45,
                },
            ],
            TimeInterpolation::LinearVariance,
        )
        .unwrap()
        .with_extrapolation(true)
    }

    #[test]
    fn test_construction_valid() {
        let surface = SabrSurface::from_calibrated_slices(
            vec![1.0_f64],
            vec![SabrSliceParams {
                alpha: 0.04,
                beta: 0.5,
                rho: -0.3,
                nu: 0.4,
            }],
            TimeInterpolation::LinearVariance,
        );
        assert!(surface.is_ok());
    }

    #[test]
    fn test_construction_empty_expiries() {
        let result = SabrSurface::<f64>::from_calibrated_slices(
            vec![],
            vec![],
            TimeInterpolation::LinearVariance,
        );
        assert!(matches!(result, Err(VolSurfaceError::InvalidInput(_))));
    }

    #[test]
    fn test_construction_length_mismatch() {
        let result = SabrSurface::from_calibrated_slices(
            vec![1.0_f64, 2.0],
            vec![SabrSliceParams {
                alpha: 0.04,
                beta: 0.5,
                rho: -0.3,
                nu: 0.4,
            }],
            TimeInterpolation::LinearVariance,
        );
        assert!(matches!(result, Err(VolSurfaceError::InvalidInput(_))));
    }

    #[test]
    fn test_single_slice_matches_sabr_formula() {
        let surface = single_slice_surface();
        let vol = surface.implied_vol(100.0, 1.0, 100.0).unwrap();

        let direct = sabr_implied_vol(
            &SabrImpliedVolParams {
                forward: 100.0,
                alpha: 0.2,
                beta: 0.5,
                nu: 0.4,
                rho: -0.3,
                maturity: 1.0,
            },
            100.0,
        )
        .unwrap();

        assert_relative_eq!(vol, direct, epsilon = 1e-12);
    }

    #[test]
    fn test_single_slice_atm_positive() {
        let surface = single_slice_surface();
        let vol = surface.implied_vol(100.0, 1.0, 100.0).unwrap();
        assert!(vol > 0.0);
        assert!(vol < 1.0);
    }

    #[test]
    fn test_smile_shape() {
        let surface = single_slice_surface();
        let forward = 100.0;

        let vol_90 = surface.implied_vol(90.0, 1.0, forward).unwrap();
        let vol_100 = surface.implied_vol(100.0, 1.0, forward).unwrap();
        let vol_110 = surface.implied_vol(110.0, 1.0, forward).unwrap();

        assert!(
            vol_90 > vol_110,
            "Expected negative skew: {} > {}",
            vol_90,
            vol_110
        );
        assert!(vol_100 > 0.0);
    }

    #[test]
    fn test_multi_slice_at_grid_point() {
        let surface = multi_slice_surface();

        let vol = surface.implied_vol(100.0, 1.0, 100.0).unwrap();

        let direct = sabr_implied_vol(
            &SabrImpliedVolParams {
                forward: 100.0,
                alpha: 0.20,
                beta: 0.5,
                nu: 0.40,
                rho: -0.30,
                maturity: 1.0,
            },
            100.0,
        )
        .unwrap();

        assert_relative_eq!(vol, direct, epsilon = 1e-6);
    }

    #[test]
    fn test_multi_slice_interpolated() {
        let surface = multi_slice_surface();

        let vol = surface.implied_vol(100.0, 0.75, 100.0).unwrap();
        assert!(vol > 0.0);

        let vol_05 = surface.implied_vol(100.0, 0.5, 100.0).unwrap();
        let vol_10 = surface.implied_vol(100.0, 1.0, 100.0).unwrap();

        let lower = vol_05.min(vol_10);
        let upper = vol_05.max(vol_10);
        assert!(
            vol >= lower - 1e-6 && vol <= upper + 1e-6,
            "Interpolated vol {vol} should be between {lower} and {upper}"
        );
    }

    #[test]
    fn test_extrapolation_allowed() {
        let surface = multi_slice_surface();
        let vol = surface.implied_vol(100.0, 3.0, 100.0).unwrap();
        assert!(vol > 0.0);
    }

    #[test]
    fn test_extrapolation_not_allowed() {
        let surface = SabrSurface::from_calibrated_slices(
            vec![0.5, 1.0],
            vec![
                SabrSliceParams {
                    alpha: 0.20,
                    beta: 0.5,
                    rho: -0.3,
                    nu: 0.4,
                },
                SabrSliceParams {
                    alpha: 0.22,
                    beta: 0.5,
                    rho: -0.3,
                    nu: 0.4,
                },
            ],
            TimeInterpolation::LinearVariance,
        )
        .unwrap();

        let result = surface.implied_vol(100.0, 0.1, 100.0);
        assert!(matches!(
            result,
            Err(VolSurfaceError::ExtrapolationNotAllowed)
        ));
    }

    #[test]
    fn test_linear_vol_interpolation() {
        let surface = SabrSurface::from_calibrated_slices(
            vec![0.5, 1.0],
            vec![
                SabrSliceParams {
                    alpha: 0.18,
                    beta: 0.5,
                    rho: -0.25,
                    nu: 0.35,
                },
                SabrSliceParams {
                    alpha: 0.22,
                    beta: 0.5,
                    rho: -0.35,
                    nu: 0.45,
                },
            ],
            TimeInterpolation::LinearVol,
        )
        .unwrap()
        .with_extrapolation(true);

        let vol = surface.implied_vol(100.0, 0.75, 100.0).unwrap();
        assert!(vol > 0.0);
    }

    #[test]
    fn test_atm_vol_delegates() {
        let surface = single_slice_surface();
        let atm = surface.atm_vol(1.0, 100.0).unwrap();
        let direct = surface.implied_vol(100.0, 1.0, 100.0).unwrap();
        assert_relative_eq!(atm, direct, epsilon = 1e-12);
    }

    #[test]
    fn test_accessors() {
        let surface = multi_slice_surface();
        assert_eq!(surface.expiries().len(), 3);
        assert_eq!(surface.params().len(), 3);
        assert_eq!(
            surface.time_interpolation(),
            TimeInterpolation::LinearVariance
        );
    }
}
