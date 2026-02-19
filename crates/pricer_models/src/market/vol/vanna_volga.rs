//! Vanna-Volga volatility surface for FX markets.
//!
//! Wraps [`pricer_core::math::formulas::vanna_volga`] with term-structure
//! interpolation. The FX market standard: constructs the smile from three
//! pillar quotes (25D put, ATM, 25D call).

use infra_domain::market::definition::TimeInterpolation;
use pricer_core::{
    math::formulas::vanna_volga::{vanna_volga_implied_vol, VannaVolgaParams},
    traits::Float,
};

use super::{
    interp::{find_bracket, linear_interp},
    VolSurface, VolSurfaceError,
};

/// Re-export slice params.
pub type VannaVolgaSliceParams<T> = VannaVolgaParams<T>;

/// A term-structure Vanna-Volga surface built from per-expiry pillar quotes.
#[derive(Clone, Debug)]
pub struct VannaVolgaSurface<T: Float> {
    expiries: Vec<T>,
    params: Vec<VannaVolgaParams<T>>,
    time_interpolation: TimeInterpolation,
    allow_extrapolation: bool,
}

impl<T: Float> VannaVolgaSurface<T> {
    /// Constructs the surface from per-expiry Vanna-Volga parameters.
    pub fn from_pillar_slices(
        expiries: Vec<T>,
        params: Vec<VannaVolgaParams<T>>,
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

    /// Returns the per-slice parameters.
    pub fn params(&self) -> &[VannaVolgaParams<T>] { &self.params }

    /// Returns the time interpolation method.
    pub fn time_interpolation(&self) -> TimeInterpolation { self.time_interpolation }

    /// Interpolates VV parameters to the target expiry.
    fn interpolate_params(&self, expiry: T) -> Result<VannaVolgaParams<T>, VolSurfaceError> {
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

        // Interpolate all volatilities and strikes linearly
        Ok(VannaVolgaParams {
            sigma_atm: linear_interp(t_lo, p_lo.sigma_atm, t_hi, p_hi.sigma_atm, expiry),
            sigma_25d_put: linear_interp(
                t_lo,
                p_lo.sigma_25d_put,
                t_hi,
                p_hi.sigma_25d_put,
                expiry,
            ),
            sigma_25d_call: linear_interp(
                t_lo,
                p_lo.sigma_25d_call,
                t_hi,
                p_hi.sigma_25d_call,
                expiry,
            ),
            strike_atm: linear_interp(t_lo, p_lo.strike_atm, t_hi, p_hi.strike_atm, expiry),
            strike_25d_put: linear_interp(
                t_lo,
                p_lo.strike_25d_put,
                t_hi,
                p_hi.strike_25d_put,
                expiry,
            ),
            strike_25d_call: linear_interp(
                t_lo,
                p_lo.strike_25d_call,
                t_hi,
                p_hi.strike_25d_call,
                expiry,
            ),
        })
    }
}

impl<T: Float> VolSurface<T> for VannaVolgaSurface<T> {
    fn implied_vol(&self, strike: T, expiry: T, _forward: T) -> Result<T, VolSurfaceError> {
        let slice = self.interpolate_params(expiry)?;
        vanna_volga_implied_vol(&slice, strike)
            .map_err(|e| VolSurfaceError::InvalidInput(format!("{e}")))
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    fn single_slice() -> VannaVolgaSurface<f64> {
        VannaVolgaSurface::from_pillar_slices(
            vec![1.0],
            vec![VannaVolgaParams {
                sigma_atm: 0.10,
                sigma_25d_put: 0.12,
                sigma_25d_call: 0.09,
                strike_atm: 1.10,
                strike_25d_put: 1.05,
                strike_25d_call: 1.15,
            }],
            TimeInterpolation::LinearVol,
        )
        .unwrap()
        .with_extrapolation(true)
    }

    #[test]
    fn test_reproduces_atm() {
        let s = single_slice();
        let vol = s.implied_vol(1.10, 1.0, 1.10).unwrap();
        assert_relative_eq!(vol, 0.10, epsilon = 1e-6);
    }

    #[test]
    fn test_reproduces_pillar() {
        let s = single_slice();
        let vol_put = s.implied_vol(1.05, 1.0, 1.10).unwrap();
        assert_relative_eq!(vol_put, 0.12, epsilon = 1e-6);
    }

    #[test]
    fn test_construction_empty() {
        let r = VannaVolgaSurface::<f64>::from_pillar_slices(
            vec![],
            vec![],
            TimeInterpolation::LinearVol,
        );
        assert!(r.is_err());
    }

    #[test]
    fn test_interpolation_between_pillars() {
        let s = single_slice();
        let vol = s.implied_vol(1.08, 1.0, 1.10).unwrap();
        assert!(vol > 0.0 && vol < 0.20);
    }
}
