//! Polynomial total-variance volatility surface.
//!
//! Wraps [`pricer_core::math::formulas::polynomial_vol`] with term-structure
//! interpolation. Simple baseline for IR vol surfaces.

use infra_domain::market::definition::TimeInterpolation;
use pricer_core::{
    math::formulas::polynomial_vol::{polynomial_implied_vol, PolynomialVolParams},
    traits::Float,
};

use super::{
    interp::{find_bracket, linear_interp},
    VolSurface, VolSurfaceError,
};

/// A term-structure polynomial vol surface.
#[derive(Clone, Debug)]
pub struct PolynomialVolSurface<T: Float> {
    expiries: Vec<T>,
    params: Vec<PolynomialVolParams<T>>,
    time_interpolation: TimeInterpolation,
    allow_extrapolation: bool,
}

impl<T: Float> PolynomialVolSurface<T> {
    /// Constructs the surface from pre-calibrated slices.
    pub fn from_calibrated_slices(
        expiries: Vec<T>,
        params: Vec<PolynomialVolParams<T>>,
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

    /// Interpolates polynomial coefficients to the target expiry.
    ///
    /// Each coefficient is interpolated independently.
    fn interpolate_params(
        &self,
        expiry: T,
    ) -> Result<PolynomialVolParams<T>, VolSurfaceError> {
        let n = self.expiries.len();

        if n == 1 {
            return Ok(self.params[0].clone());
        }

        let (lo, hi) = find_bracket(&self.expiries, expiry);

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

        let max_len = p_lo.coefficients.len().max(p_hi.coefficients.len());
        let mut coeffs = Vec::with_capacity(max_len);

        for i in 0..max_len {
            let c_lo = p_lo.coefficients.get(i).copied().unwrap_or_else(T::zero);
            let c_hi = p_hi.coefficients.get(i).copied().unwrap_or_else(T::zero);

            let c = match self.time_interpolation {
                TimeInterpolation::LinearVariance if i == 0 => {
                    // Interpolate c₀ (level) in variance space
                    let var_lo = c_lo * t_lo;
                    let var_hi = c_hi * t_hi;
                    let var = linear_interp(t_lo, var_lo, t_hi, var_hi, expiry);
                    let eps = T::from(1e-14).unwrap_or_else(|| T::epsilon());
                    if expiry > eps { var / expiry } else { c_lo }
                }
                _ => linear_interp(t_lo, c_lo, t_hi, c_hi, expiry),
            };
            coeffs.push(c);
        }

        Ok(PolynomialVolParams { coefficients: coeffs })
    }
}

impl<T: Float> VolSurface<T> for PolynomialVolSurface<T> {
    fn implied_vol(&self, strike: T, expiry: T, forward: T) -> Result<T, VolSurfaceError> {
        let slice = self.interpolate_params(expiry)?;
        polynomial_implied_vol(&slice, strike, forward, expiry)
            .map_err(|e| VolSurfaceError::InvalidInput(format!("{e}")))
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_flat_surface() {
        let s = PolynomialVolSurface::from_calibrated_slices(
            vec![1.0],
            vec![PolynomialVolParams { coefficients: vec![0.04] }],
            TimeInterpolation::LinearVariance,
        )
        .unwrap();
        let vol = s.implied_vol(100.0, 1.0, 100.0).unwrap();
        assert_relative_eq!(vol, 0.2, epsilon = 1e-6);
    }

    #[test]
    fn test_empty_expiries() {
        let r = PolynomialVolSurface::<f64>::from_calibrated_slices(
            vec![],
            vec![],
            TimeInterpolation::LinearVariance,
        );
        assert!(r.is_err());
    }

    #[test]
    fn test_multi_slice() {
        let s = PolynomialVolSurface::from_calibrated_slices(
            vec![0.5, 2.0],
            vec![
                PolynomialVolParams { coefficients: vec![0.02] },
                PolynomialVolParams { coefficients: vec![0.06] },
            ],
            TimeInterpolation::LinearVol,
        )
        .unwrap()
        .with_extrapolation(true);
        let vol = s.implied_vol(100.0, 1.0, 100.0).unwrap();
        assert!(vol > 0.0);
    }
}
