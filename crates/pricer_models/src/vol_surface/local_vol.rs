//! Local volatility surface via Dupire's framework.
//!
//! Stores pre-computed local volatilities on a discrete grid and
//! provides bilinear interpolation for arbitrary query points.

use pricer_core::traits::Float;

use super::interp::bilinear_interp;
use super::{VolSurface, VolSurfaceError};

/// A discrete local-volatility surface stored on a (strike x expiry) grid.
///
/// `local_vols` is row-major: rows correspond to expiries, columns to strikes.
/// That is, `local_vols[i * strikes.len() + j]` is the local vol at
/// `(expiries[i], strikes[j])`.
#[derive(Clone, Debug)]
pub struct LocalVolSurface<T: Float> {
    strikes: Vec<T>,
    expiries: Vec<T>,
    local_vols: Vec<T>,
    allow_extrapolation: bool,
}

impl<T: Float> LocalVolSurface<T> {
    /// Constructs a new local-vol surface from a discrete grid.
    ///
    /// `local_vols` must have length `expiries.len() * strikes.len()` in
    /// row-major order (expiries are rows, strikes are columns).
    pub fn new(
        strikes: Vec<T>,
        expiries: Vec<T>,
        local_vols: Vec<T>,
    ) -> Result<Self, VolSurfaceError> {
        let expected_len = expiries.len() * strikes.len();
        if strikes.is_empty() || expiries.is_empty() {
            return Err(VolSurfaceError::InvalidInput(
                "strikes and expiries must be non-empty".to_string(),
            ));
        }
        if local_vols.len() != expected_len {
            return Err(VolSurfaceError::InvalidInput(format!(
                "local_vols length {} does not match {} expiries x {} strikes = {}",
                local_vols.len(),
                expiries.len(),
                strikes.len(),
                expected_len,
            )));
        }
        Ok(Self {
            strikes,
            expiries,
            local_vols,
            allow_extrapolation: false,
        })
    }

    /// Enables or disables extrapolation outside the grid boundaries.
    pub fn with_extrapolation(mut self, allow: bool) -> Self {
        self.allow_extrapolation = allow;
        self
    }
}

impl<T: Float> VolSurface<T> for LocalVolSurface<T> {
    /// Implied volatility is not directly available from a local-vol surface.
    fn implied_vol(&self, _strike: T, _expiry: T, _forward: T) -> Result<T, VolSurfaceError> {
        Err(VolSurfaceError::UnsupportedOperation(
            "implied vol is not directly available from a local-vol surface".to_string(),
        ))
    }

    /// Returns the local volatility at the given point via bilinear interpolation.
    fn local_vol(&self, strike: T, expiry: T, _forward: T) -> Result<T, VolSurfaceError> {
        // Boundary check when extrapolation is disallowed
        if !self.allow_extrapolation {
            let s_min = self.strikes[0];
            let s_max = self.strikes[self.strikes.len() - 1];
            let t_min = self.expiries[0];
            let t_max = self.expiries[self.expiries.len() - 1];

            if strike < s_min || strike > s_max || expiry < t_min || expiry > t_max {
                return Err(VolSurfaceError::ExtrapolationNotAllowed);
            }
        }

        // bilinear_interp expects: xs = columns (strikes), ys = rows (expiries)
        bilinear_interp(&self.strikes, &self.expiries, &self.local_vols, strike, expiry)
            .ok_or_else(|| {
                VolSurfaceError::InterpolationFailed(
                    "bilinear interpolation failed".to_string(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn sample_surface() -> LocalVolSurface<f64> {
        // 2 expiries x 3 strikes
        let strikes = vec![90.0, 100.0, 110.0];
        let expiries = vec![0.25, 1.0];
        // Row-major: row 0 = expiry 0.25, row 1 = expiry 1.0
        let local_vols = vec![
            0.20, 0.18, 0.22, // t = 0.25
            0.22, 0.20, 0.24, // t = 1.0
        ];
        LocalVolSurface::new(strikes, expiries, local_vols).unwrap()
    }

    #[test]
    fn test_construction_valid() {
        let surface = sample_surface();
        assert_eq!(surface.strikes.len(), 3);
        assert_eq!(surface.expiries.len(), 2);
    }

    #[test]
    fn test_construction_dimension_mismatch() {
        let strikes = vec![90.0_f64, 100.0];
        let expiries = vec![0.25, 1.0];
        let local_vols = vec![0.2, 0.2, 0.2]; // should be 4
        let result = LocalVolSurface::new(strikes, expiries, local_vols);
        assert!(matches!(result, Err(VolSurfaceError::InvalidInput(_))));
    }

    #[test]
    fn test_construction_empty_strikes() {
        let result = LocalVolSurface::new(vec![], vec![1.0_f64], vec![]);
        assert!(matches!(result, Err(VolSurfaceError::InvalidInput(_))));
    }

    #[test]
    fn test_local_vol_at_grid_point() {
        let surface = sample_surface().with_extrapolation(true);
        // (strike=100, expiry=0.25) => row 0, col 1 => 0.18
        let vol = surface.local_vol(100.0, 0.25, 100.0).unwrap();
        assert_relative_eq!(vol, 0.18, epsilon = 1e-12);
    }

    #[test]
    fn test_local_vol_interpolated() {
        let surface = sample_surface().with_extrapolation(true);
        // Interpolate between grid nodes
        let vol = surface.local_vol(100.0, 0.625, 100.0).unwrap();
        // At strike=100 (col 1): t=0.25 -> 0.18, t=1.0 -> 0.20
        // Linear interp at t=0.625: 0.18 + (0.20 - 0.18) * (0.625 - 0.25) / (1.0 - 0.25) = 0.19
        assert_relative_eq!(vol, 0.19, epsilon = 1e-12);
    }

    #[test]
    fn test_implied_vol_unsupported() {
        let surface = sample_surface();
        let result = surface.implied_vol(100.0, 1.0, 100.0);
        assert!(matches!(result, Err(VolSurfaceError::UnsupportedOperation(_))));
    }

    #[test]
    fn test_extrapolation_not_allowed() {
        let surface = sample_surface(); // extrapolation off by default
        let result = surface.local_vol(80.0, 0.25, 100.0);
        assert!(matches!(result, Err(VolSurfaceError::ExtrapolationNotAllowed)));
    }

    #[test]
    fn test_extrapolation_allowed_clamps() {
        let surface = sample_surface().with_extrapolation(true);
        // Below strike grid - find_bracket clamps to boundary
        let vol = surface.local_vol(80.0, 0.25, 100.0).unwrap();
        // Clamped to (strike=90, t=0.25) => 0.20
        assert_relative_eq!(vol, 0.20, epsilon = 1e-12);
    }
}
