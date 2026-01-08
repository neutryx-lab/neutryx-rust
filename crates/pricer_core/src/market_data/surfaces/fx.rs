//! FX volatility surface abstractions for FX option pricing.
//!
//! This module provides:
//! - [`FxVolatilitySurface`]: Delta-expiry based volatility surface for FX options
//! - [`FxDeltaPoint`]: Standard delta points used in FX markets

use super::VolatilitySurface;
use crate::market_data::error::MarketDataError;
use crate::math::interpolators::BilinearInterpolator;
use num_traits::Float;

/// Standard delta points used in FX volatility quoting.
///
/// In FX markets, volatility is typically quoted for specific delta points
/// rather than for absolute strikes. Common convention includes:
/// - 10D Put (10 delta put)
/// - 25D Put (25 delta put)
/// - ATM (at-the-money, typically 50 delta straddle)
/// - 25D Call (25 delta call)
/// - 10D Call (10 delta call)
///
/// # Example
///
/// ```
/// use pricer_core::market_data::surfaces::FxDeltaPoint;
///
/// let atm = FxDeltaPoint::Atm;
/// assert!((atm.as_delta() - 0.5).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FxDeltaPoint {
    /// 10 delta put
    Put10D,
    /// 25 delta put
    Put25D,
    /// At-the-money (50 delta)
    Atm,
    /// 25 delta call
    Call25D,
    /// 10 delta call
    Call10D,
}

impl FxDeltaPoint {
    /// Return the delta value for this point.
    ///
    /// # Returns
    ///
    /// The delta value in the range [-1, 1], where:
    /// - Puts have negative delta
    /// - ATM is approximately 0.5
    /// - Calls have positive delta
    ///
    /// Note: Returns absolute delta values for simplicity (0 to 1 scale).
    #[inline]
    pub fn as_delta(&self) -> f64 {
        match self {
            FxDeltaPoint::Put10D => 0.1,
            FxDeltaPoint::Put25D => 0.25,
            FxDeltaPoint::Atm => 0.5,
            FxDeltaPoint::Call25D => 0.75,
            FxDeltaPoint::Call10D => 0.9,
        }
    }

    /// Return all standard delta points in order.
    #[inline]
    pub fn all() -> [FxDeltaPoint; 5] {
        [
            FxDeltaPoint::Put10D,
            FxDeltaPoint::Put25D,
            FxDeltaPoint::Atm,
            FxDeltaPoint::Call25D,
            FxDeltaPoint::Call10D,
        ]
    }
}

/// FX volatility surface using delta × expiry grid.
///
/// This surface stores implied volatilities for FX options organized by
/// delta (moneyness) and expiry. This is the standard market convention
/// for quoting FX volatilities.
///
/// # Type Parameters
///
/// * `T` - Floating-point type (e.g., `f64`, `Dual64`)
///
/// # Grid Structure
///
/// - X-axis: Delta points (0.1, 0.25, 0.5, 0.75, 0.9)
/// - Y-axis: Expiry tenors in years
/// - Values: Implied volatilities
///
/// # Example
///
/// ```
/// use pricer_core::market_data::surfaces::FxVolatilitySurface;
///
/// // Create a 5x3 surface (5 deltas × 3 expiries)
/// let deltas = [0.1_f64, 0.25, 0.5, 0.75, 0.9];
/// let expiries = [0.25, 1.0, 2.0];
/// let vols = [
///     // 10D Put, 25D Put, ATM, 25D Call, 10D Call for each expiry
///     [0.12, 0.11, 0.10, 0.11, 0.12],  // 3M
///     [0.13, 0.12, 0.11, 0.12, 0.13],  // 1Y
///     [0.14, 0.13, 0.12, 0.13, 0.14],  // 2Y
/// ];
///
/// let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();
///
/// // Get ATM vol at 1 year
/// let atm_vol = surface.atm_volatility(1.0).unwrap();
/// assert!((atm_vol - 0.11).abs() < 1e-10);
/// ```
#[derive(Debug, Clone)]
pub struct FxVolatilitySurface<T: Float> {
    /// Delta points (X-axis)
    deltas: Vec<T>,
    /// Expiry tenors in years (Y-axis)
    expiries: Vec<T>,
    /// Volatility grid (expiry × delta)
    volatilities: Vec<Vec<T>>,
    /// Whether to allow extrapolation
    allow_extrapolation: bool,
}

impl<T: Float> FxVolatilitySurface<T> {
    /// Construct an FX volatility surface from a delta × expiry grid.
    ///
    /// # Arguments
    ///
    /// * `deltas` - Delta points (must be sorted, at least 2 points)
    /// * `expiries` - Expiry tenors in years (must be sorted, at least 2 points)
    /// * `volatilities` - Grid of volatilities [expiry][delta]
    /// * `allow_extrapolation` - Whether to allow flat extrapolation
    ///
    /// # Returns
    ///
    /// * `Ok(FxVolatilitySurface)` - Successfully constructed surface
    /// * `Err(MarketDataError)` - If input validation fails
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::surfaces::FxVolatilitySurface;
    ///
    /// let deltas = [0.25_f64, 0.5, 0.75];
    /// let expiries = [0.5, 1.0];
    /// let vols = [
    ///     [0.11, 0.10, 0.11],
    ///     [0.12, 0.11, 0.12],
    /// ];
    ///
    /// let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();
    /// ```
    pub fn new(
        deltas: &[T],
        expiries: &[T],
        volatilities: &[impl AsRef<[T]>],
        allow_extrapolation: bool,
    ) -> Result<Self, MarketDataError> {
        // Validate delta points
        if deltas.len() < 2 {
            return Err(MarketDataError::InsufficientData {
                got: deltas.len(),
                need: 2,
            });
        }

        // Validate expiries
        if expiries.len() < 2 {
            return Err(MarketDataError::InsufficientData {
                got: expiries.len(),
                need: 2,
            });
        }

        // Validate deltas are sorted and in valid range
        for i in 0..deltas.len() {
            if deltas[i] <= T::zero() || deltas[i] >= T::one() {
                return Err(MarketDataError::InvalidStrike {
                    strike: deltas[i].to_f64().unwrap_or(0.0),
                });
            }
            if i > 0 && deltas[i] <= deltas[i - 1] {
                return Err(MarketDataError::InvalidStrike {
                    strike: deltas[i].to_f64().unwrap_or(0.0),
                });
            }
        }

        // Validate expiries are sorted and positive
        for i in 0..expiries.len() {
            if expiries[i] <= T::zero() {
                return Err(MarketDataError::InvalidExpiry {
                    expiry: expiries[i].to_f64().unwrap_or(0.0),
                });
            }
            if i > 0 && expiries[i] <= expiries[i - 1] {
                return Err(MarketDataError::InvalidExpiry {
                    expiry: expiries[i].to_f64().unwrap_or(0.0),
                });
            }
        }

        // Validate grid dimensions
        if volatilities.len() != expiries.len() {
            return Err(MarketDataError::InsufficientData {
                got: volatilities.len(),
                need: expiries.len(),
            });
        }

        let mut vol_grid = Vec::with_capacity(expiries.len());
        for row in volatilities {
            let row_ref = row.as_ref();
            if row_ref.len() != deltas.len() {
                return Err(MarketDataError::InsufficientData {
                    got: row_ref.len(),
                    need: deltas.len(),
                });
            }

            // Validate volatilities are positive
            for &vol in row_ref {
                if vol <= T::zero() {
                    return Err(MarketDataError::InterpolationFailed {
                        reason: format!(
                            "Volatility must be positive, got {}",
                            vol.to_f64().unwrap_or(0.0)
                        ),
                    });
                }
            }

            vol_grid.push(row_ref.to_vec());
        }

        Ok(Self {
            deltas: deltas.to_vec(),
            expiries: expiries.to_vec(),
            volatilities: vol_grid,
            allow_extrapolation,
        })
    }

    /// Return the delta domain.
    #[inline]
    pub fn delta_domain(&self) -> (T, T) {
        (self.deltas[0], self.deltas[self.deltas.len() - 1])
    }

    /// Return whether extrapolation is allowed.
    #[inline]
    pub fn allow_extrapolation(&self) -> bool {
        self.allow_extrapolation
    }

    /// Get the ATM (at-the-money) volatility for a given expiry.
    ///
    /// This method returns the volatility at delta = 0.5 (ATM point).
    ///
    /// # Arguments
    ///
    /// * `expiry` - Time to expiry in years
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - ATM volatility
    /// * `Err(MarketDataError)` - If expiry is invalid or out of bounds
    ///
    /// # Example
    ///
    /// ```
    /// use pricer_core::market_data::surfaces::FxVolatilitySurface;
    ///
    /// let deltas = [0.25_f64, 0.5, 0.75];
    /// let expiries = [0.5, 1.0, 2.0];
    /// let vols = [
    ///     [0.11, 0.10, 0.11],
    ///     [0.12, 0.11, 0.12],
    ///     [0.13, 0.12, 0.13],
    /// ];
    ///
    /// let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();
    /// let atm_1y = surface.atm_volatility(1.0).unwrap();
    /// assert!((atm_1y - 0.11).abs() < 1e-10);
    /// ```
    pub fn atm_volatility(&self, expiry: T) -> Result<T, MarketDataError> {
        let atm_delta = T::from(0.5).unwrap();
        self.volatility_by_delta(atm_delta, expiry)
    }

    /// Get volatility by delta and expiry.
    ///
    /// # Arguments
    ///
    /// * `delta` - Delta value (0 < delta < 1)
    /// * `expiry` - Time to expiry in years
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - Interpolated volatility
    /// * `Err(MarketDataError)` - If parameters are invalid or out of bounds
    pub fn volatility_by_delta(&self, delta: T, expiry: T) -> Result<T, MarketDataError> {
        if delta <= T::zero() || delta >= T::one() {
            return Err(MarketDataError::InvalidStrike {
                strike: delta.to_f64().unwrap_or(0.0),
            });
        }
        if expiry <= T::zero() {
            return Err(MarketDataError::InvalidExpiry {
                expiry: expiry.to_f64().unwrap_or(0.0),
            });
        }

        let (d_min, d_max) = self.delta_domain();
        let (t_min, t_max) = self.expiry_domain();

        // Handle extrapolation
        if !self.allow_extrapolation {
            if delta < d_min || delta > d_max {
                return Err(MarketDataError::OutOfBounds {
                    x: delta.to_f64().unwrap_or(0.0),
                    min: d_min.to_f64().unwrap_or(0.0),
                    max: d_max.to_f64().unwrap_or(0.0),
                });
            }
            if expiry < t_min || expiry > t_max {
                return Err(MarketDataError::OutOfBounds {
                    x: expiry.to_f64().unwrap_or(0.0),
                    min: t_min.to_f64().unwrap_or(0.0),
                    max: t_max.to_f64().unwrap_or(0.0),
                });
            }
        }

        // Use bilinear interpolation
        // Convert Vec<Vec<T>> to &[&[T]] for BilinearInterpolator
        let vol_slices: Vec<&[T]> = self.volatilities.iter().map(|v| v.as_slice()).collect();
        // Grid is stored as volatilities[expiry_idx][delta_idx]
        // BilinearInterpolator expects zs[x_idx][y_idx] = z(xs[x_idx], ys[y_idx])
        // So we pass expiries as x-axis and deltas as y-axis
        let interp =
            BilinearInterpolator::new(&self.expiries, &self.deltas, vol_slices.as_slice())?;

        // Clamp for extrapolation
        let clamped_delta = if delta < d_min {
            d_min
        } else if delta > d_max {
            d_max
        } else {
            delta
        };
        let clamped_expiry = if expiry < t_min {
            t_min
        } else if expiry > t_max {
            t_max
        } else {
            expiry
        };

        // Note: BilinearInterpolator was constructed with (expiries, deltas, ...)
        // so we call interpolate(expiry, delta)
        interp
            .interpolate(clamped_expiry, clamped_delta)
            .map_err(MarketDataError::from)
    }

    /// Get the 25-delta risk reversal for a given expiry.
    ///
    /// Risk reversal = σ(25D Call) - σ(25D Put)
    ///
    /// This measures the skew of the volatility smile.
    ///
    /// # Arguments
    ///
    /// * `expiry` - Time to expiry in years
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - 25-delta risk reversal
    /// * `Err(MarketDataError)` - If calculation fails
    pub fn risk_reversal_25d(&self, expiry: T) -> Result<T, MarketDataError> {
        let call_25d = T::from(0.75).unwrap();
        let put_25d = T::from(0.25).unwrap();

        let vol_call = self.volatility_by_delta(call_25d, expiry)?;
        let vol_put = self.volatility_by_delta(put_25d, expiry)?;

        Ok(vol_call - vol_put)
    }

    /// Get the 25-delta butterfly for a given expiry.
    ///
    /// Butterfly = (σ(25D Call) + σ(25D Put)) / 2 - σ(ATM)
    ///
    /// This measures the curvature of the volatility smile.
    ///
    /// # Arguments
    ///
    /// * `expiry` - Time to expiry in years
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - 25-delta butterfly
    /// * `Err(MarketDataError)` - If calculation fails
    pub fn butterfly_25d(&self, expiry: T) -> Result<T, MarketDataError> {
        let call_25d = T::from(0.75).unwrap();
        let put_25d = T::from(0.25).unwrap();
        let atm = T::from(0.5).unwrap();

        let vol_call = self.volatility_by_delta(call_25d, expiry)?;
        let vol_put = self.volatility_by_delta(put_25d, expiry)?;
        let vol_atm = self.volatility_by_delta(atm, expiry)?;

        let two = T::one() + T::one();
        Ok((vol_call + vol_put) / two - vol_atm)
    }
}

impl<T: Float> VolatilitySurface<T> for FxVolatilitySurface<T> {
    /// Return the implied volatility for given strike (interpreted as delta) and expiry.
    ///
    /// Note: For FxVolatilitySurface, the `strike` parameter is interpreted as delta.
    /// For strike-based lookups, use a separate conversion method.
    fn volatility(&self, strike: T, expiry: T) -> Result<T, MarketDataError> {
        // Interpret strike as delta for consistency with FX convention
        self.volatility_by_delta(strike, expiry)
    }

    fn strike_domain(&self) -> (T, T) {
        self.delta_domain()
    }

    fn expiry_domain(&self) -> (T, T) {
        (self.expiries[0], self.expiries[self.expiries.len() - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // FxDeltaPoint Tests
    // ========================================

    #[test]
    fn test_delta_point_as_delta() {
        assert!((FxDeltaPoint::Put10D.as_delta() - 0.1).abs() < 1e-10);
        assert!((FxDeltaPoint::Put25D.as_delta() - 0.25).abs() < 1e-10);
        assert!((FxDeltaPoint::Atm.as_delta() - 0.5).abs() < 1e-10);
        assert!((FxDeltaPoint::Call25D.as_delta() - 0.75).abs() < 1e-10);
        assert!((FxDeltaPoint::Call10D.as_delta() - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_delta_point_all() {
        let all = FxDeltaPoint::all();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0], FxDeltaPoint::Put10D);
        assert_eq!(all[4], FxDeltaPoint::Call10D);
    }

    // ========================================
    // FxVolatilitySurface Construction Tests
    // ========================================

    #[test]
    fn test_surface_new_valid() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0, 2.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12], [0.13, 0.12, 0.13]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        assert_eq!(surface.delta_domain(), (0.25, 0.75));
        assert_eq!(surface.expiry_domain(), (0.5, 2.0));
    }

    #[test]
    fn test_surface_new_insufficient_deltas() {
        let deltas = [0.5_f64];
        let expiries = [0.5, 1.0];
        let vols = [[0.10], [0.11]];

        let result = FxVolatilitySurface::new(&deltas, &expiries, &vols, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_surface_new_insufficient_expiries() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [1.0];
        let vols = [[0.11, 0.10, 0.11]];

        let result = FxVolatilitySurface::new(&deltas, &expiries, &vols, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_surface_new_unsorted_deltas() {
        let deltas = [0.5_f64, 0.25, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.10, 0.11, 0.11], [0.11, 0.12, 0.12]];

        let result = FxVolatilitySurface::new(&deltas, &expiries, &vols, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_surface_new_invalid_delta() {
        let deltas = [0.0_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.10, 0.11, 0.11], [0.11, 0.12, 0.12]];

        let result = FxVolatilitySurface::new(&deltas, &expiries, &vols, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_surface_new_negative_expiry() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [-0.5, 1.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12]];

        let result = FxVolatilitySurface::new(&deltas, &expiries, &vols, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_surface_new_negative_volatility() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, -0.10, 0.11], [0.12, 0.11, 0.12]];

        let result = FxVolatilitySurface::new(&deltas, &expiries, &vols, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_surface_new_mismatched_grid() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10], [0.12, 0.11]]; // Wrong number of delta columns

        let result = FxVolatilitySurface::new(&deltas, &expiries, &vols, true);
        assert!(result.is_err());
    }

    // ========================================
    // ATM Volatility Tests
    // ========================================

    #[test]
    fn test_atm_volatility_at_pillar() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0, 2.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12], [0.13, 0.12, 0.13]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        let atm_1y = surface.atm_volatility(1.0).unwrap();
        assert!((atm_1y - 0.11).abs() < 1e-10);
    }

    #[test]
    fn test_atm_volatility_interpolated() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.10, 0.10, 0.10], [0.12, 0.12, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        // At 0.75 years, ATM vol should be between 0.10 and 0.12
        let atm = surface.atm_volatility(0.75).unwrap();
        assert!(atm > 0.10 && atm < 0.12);
    }

    // ========================================
    // Volatility by Delta Tests
    // ========================================

    #[test]
    fn test_volatility_by_delta_at_pillar() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.09], [0.12, 0.11, 0.10]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        // 25D Put at 1Y
        let vol_25d = surface.volatility_by_delta(0.25, 1.0).unwrap();
        assert!((vol_25d - 0.12).abs() < 1e-10);

        // 75D (25D Call) at 1Y
        let vol_75d = surface.volatility_by_delta(0.75, 1.0).unwrap();
        assert!((vol_75d - 0.10).abs() < 1e-10);
    }

    #[test]
    fn test_volatility_by_delta_invalid_delta() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        assert!(surface.volatility_by_delta(0.0, 1.0).is_err());
        assert!(surface.volatility_by_delta(1.0, 1.0).is_err());
        assert!(surface.volatility_by_delta(-0.5, 1.0).is_err());
    }

    #[test]
    fn test_volatility_by_delta_invalid_expiry() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        assert!(surface.volatility_by_delta(0.5, 0.0).is_err());
        assert!(surface.volatility_by_delta(0.5, -1.0).is_err());
    }

    #[test]
    fn test_volatility_by_delta_out_of_bounds_no_extrapolation() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, false).unwrap();

        // Delta out of bounds
        assert!(surface.volatility_by_delta(0.1, 0.75).is_err());
        assert!(surface.volatility_by_delta(0.9, 0.75).is_err());

        // Expiry out of bounds
        assert!(surface.volatility_by_delta(0.5, 0.25).is_err());
        assert!(surface.volatility_by_delta(0.5, 2.0).is_err());
    }

    #[test]
    fn test_volatility_by_delta_with_extrapolation() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.09], [0.12, 0.11, 0.10]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        // Should succeed with extrapolation
        let _ = surface.volatility_by_delta(0.1, 0.75).unwrap();
        let _ = surface.volatility_by_delta(0.9, 0.75).unwrap();
        let _ = surface.volatility_by_delta(0.5, 0.25).unwrap();
        let _ = surface.volatility_by_delta(0.5, 2.0).unwrap();
    }

    // ========================================
    // Risk Reversal Tests
    // ========================================

    #[test]
    fn test_risk_reversal_25d() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [
            [0.12, 0.10, 0.11], // 25D Put = 0.12, 25D Call = 0.11
            [0.13, 0.11, 0.12],
        ];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        // RR = σ(25D Call) - σ(25D Put) = 0.11 - 0.12 = -0.01
        let rr = surface.risk_reversal_25d(0.5).unwrap();
        assert!((rr - (-0.01)).abs() < 1e-10);
    }

    // ========================================
    // Butterfly Tests
    // ========================================

    #[test]
    fn test_butterfly_25d() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [
            [0.12, 0.10, 0.12], // Symmetric smile
            [0.13, 0.11, 0.13],
        ];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        // BF = (σ(25D Call) + σ(25D Put)) / 2 - σ(ATM) = (0.12 + 0.12) / 2 - 0.10 = 0.02
        let bf = surface.butterfly_25d(0.5).unwrap();
        assert!((bf - 0.02).abs() < 1e-10);
    }

    // ========================================
    // VolatilitySurface Trait Tests
    // ========================================

    #[test]
    fn test_volatility_surface_trait() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        // VolatilitySurface trait method (strike = delta for FX)
        let vol = surface.volatility(0.5, 1.0).unwrap();
        assert!((vol - 0.11).abs() < 1e-10);
    }

    #[test]
    fn test_strike_domain() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        let (k_min, k_max) = surface.strike_domain();
        assert!((k_min - 0.25).abs() < 1e-10);
        assert!((k_max - 0.75).abs() < 1e-10);
    }

    // ========================================
    // Clone Tests
    // ========================================

    #[test]
    fn test_surface_clone() {
        let deltas = [0.25_f64, 0.5, 0.75];
        let expiries = [0.5, 1.0];
        let vols = [[0.11, 0.10, 0.11], [0.12, 0.11, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();
        let cloned = surface.clone();

        let vol1 = surface.atm_volatility(1.0).unwrap();
        let vol2 = cloned.atm_volatility(1.0).unwrap();
        assert!((vol1 - vol2).abs() < 1e-10);
    }

    // ========================================
    // Generic Type Tests
    // ========================================

    #[test]
    fn test_surface_with_f32() {
        let deltas = [0.25_f32, 0.5, 0.75];
        let expiries = [0.5_f32, 1.0];
        let vols = [[0.11_f32, 0.10, 0.11], [0.12_f32, 0.11, 0.12]];

        let surface = FxVolatilitySurface::new(&deltas, &expiries, &vols, true).unwrap();

        let vol = surface.atm_volatility(1.0_f32).unwrap();
        assert!((vol - 0.11_f32).abs() < 1e-6);
    }
}
