//! ZABR generalised SABR volatility surface.
//!
//! Wraps [`pricer_core::math::formulas::zabr`] with term-structure
//! interpolation. ZABR extends SABR with a flexible backbone function
//! and a mixing parameter.

use infra_domain::market::definition::TimeInterpolation;
use pricer_core::{
    math::formulas::zabr::{zabr_implied_vol, ZabrBackbone, ZabrParams},
    traits::Float,
};

use super::{
    interp::{find_bracket, linear_interp},
    VolSurface, VolSurfaceError,
};

/// Re-export types.
pub type ZabrSliceParams<T> = ZabrParams<T>;

/// A term-structure ZABR surface built from calibrated slices.
#[derive(Clone, Debug)]
pub struct ZabrSurface<T: Float> {
    expiries: Vec<T>,
    params: Vec<ZabrParams<T>>,
    time_interpolation: TimeInterpolation,
    allow_extrapolation: bool,
}

impl<T: Float> ZabrSurface<T> {
    /// Constructs the surface from pre-calibrated slices.
    pub fn from_calibrated_slices(
        expiries: Vec<T>,
        params: Vec<ZabrParams<T>>,
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

    /// Returns the expiry grid.
    pub fn expiries(&self) -> &[T] { &self.expiries }

    /// Returns per-slice parameters.
    pub fn params(&self) -> &[ZabrParams<T>] { &self.params }

    /// Interpolates ZABR parameters to the target expiry.
    fn interpolate_params(&self, expiry: T) -> Result<ZabrParams<T>, VolSurfaceError> {
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

        // Interpolate scalar parameters linearly
        let alpha = match self.time_interpolation {
            TimeInterpolation::LinearVariance => {
                let var_lo = p_lo.alpha * p_lo.alpha * t_lo;
                let var_hi = p_hi.alpha * p_hi.alpha * t_hi;
                let var = linear_interp(t_lo, var_lo, t_hi, var_hi, expiry);
                let eps = T::from(1e-14).unwrap_or_else(|| T::epsilon());
                if expiry > eps { (var / expiry).abs().sqrt() } else { p_lo.alpha }
            }
            _ => linear_interp(t_lo, p_lo.alpha, t_hi, p_hi.alpha, expiry),
        };

        let nu = linear_interp(t_lo, p_lo.nu, t_hi, p_hi.nu, expiry);
        let rho = linear_interp(t_lo, p_lo.rho, t_hi, p_hi.rho, expiry);
        let gamma_mix = linear_interp(t_lo, p_lo.gamma_mix, t_hi, p_hi.gamma_mix, expiry);

        // Interpolate backbone beta
        let beta_lo = p_lo.backbone.beta();
        let beta_hi = p_hi.backbone.beta();
        let beta = linear_interp(t_lo, beta_lo, t_hi, beta_hi, expiry);

        let backbone = match (&p_lo.backbone, &p_hi.backbone) {
            (
                ZabrBackbone::Displaced { displacement: d_lo, .. },
                ZabrBackbone::Displaced { displacement: d_hi, .. },
            ) => ZabrBackbone::Displaced {
                beta,
                displacement: linear_interp(t_lo, *d_lo, t_hi, *d_hi, expiry),
            },
            _ => ZabrBackbone::Power { beta },
        };

        Ok(ZabrParams {
            alpha,
            backbone,
            nu,
            rho,
            gamma_mix,
        })
    }
}

impl<T: Float> VolSurface<T> for ZabrSurface<T> {
    fn implied_vol(&self, strike: T, expiry: T, forward: T) -> Result<T, VolSurfaceError> {
        let slice = self.interpolate_params(expiry)?;
        zabr_implied_vol(&slice, forward, strike, expiry)
            .map_err(|e| VolSurfaceError::InvalidInput(format!("{e}")))
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use pricer_core::math::formulas::sabr::{sabr_implied_vol, SabrImpliedVolParams};

    use super::*;

    fn single_slice() -> ZabrSurface<f64> {
        ZabrSurface::from_calibrated_slices(
            vec![1.0],
            vec![ZabrParams {
                alpha: 0.2,
                backbone: ZabrBackbone::Power { beta: 0.5 },
                nu: 0.4,
                rho: -0.3,
                gamma_mix: 0.0, // Pure SABR
            }],
            TimeInterpolation::LinearVariance,
        )
        .unwrap()
        .with_extrapolation(true)
    }

    #[test]
    fn test_gamma_zero_matches_sabr() {
        let s = single_slice();
        let vol = s.implied_vol(100.0, 1.0, 100.0).unwrap();

        let sabr = sabr_implied_vol(
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

        assert_relative_eq!(vol, sabr, epsilon = 1e-10);
    }

    #[test]
    fn test_construction_empty() {
        let r = ZabrSurface::<f64>::from_calibrated_slices(
            vec![],
            vec![],
            TimeInterpolation::LinearVariance,
        );
        assert!(r.is_err());
    }

    #[test]
    fn test_smile_shape() {
        let s = ZabrSurface::from_calibrated_slices(
            vec![1.0],
            vec![ZabrParams {
                alpha: 0.2,
                backbone: ZabrBackbone::Power { beta: 0.5 },
                nu: 0.4,
                rho: -0.3,
                gamma_mix: 0.5,
            }],
            TimeInterpolation::LinearVariance,
        )
        .unwrap()
        .with_extrapolation(true);

        let vol_low = s.implied_vol(90.0, 1.0, 100.0).unwrap();
        let vol_high = s.implied_vol(110.0, 1.0, 100.0).unwrap();
        assert!(vol_low > vol_high, "Expected negative skew");
    }
}
