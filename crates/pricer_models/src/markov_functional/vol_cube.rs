//! Swaption volatility cube interface for MFM calibration.
//!
//! Provides a trait for querying swaption implied volatilities and
//! concrete implementations including a flat vol cube for testing
//! and a SABR-based vol cube wrapping the existing SABR formula.

use enum_dispatch::enum_dispatch;
use pricer_core::math::formulas::sabr::{sabr_implied_vol, SabrImpliedVolParams};
use pricer_core::math::numeric::from_f64;
use pricer_core::traits::Float;

use super::MfmError;

// ─── Trait ──────────────────────────────────────────────────────────

/// Core trait for querying a swaption volatility cube.
///
/// Implementations provide swaption implied volatilities across the
/// (expiry, tenor, strike) cube. Both normal (Bachelier) and lognormal
/// (Black) quoting conventions are supported.
#[enum_dispatch]
pub trait SwaptionVolCube<T: Float> {
    /// Returns the normal (Bachelier) implied vol for a swaption.
    ///
    /// # Arguments
    ///
    /// * `expiry` - Option expiry in year fractions
    /// * `tenor` - Underlying swap tenor in year fractions
    /// * `strike` - Swaption strike rate
    /// * `forward` - Forward swap rate
    fn normal_vol(&self, expiry: T, tenor: T, strike: T, forward: T) -> Result<T, MfmError>;

    /// Returns the lognormal (Black) implied vol for a swaption.
    ///
    /// # Arguments
    ///
    /// * `expiry` - Option expiry in year fractions
    /// * `tenor` - Underlying swap tenor in year fractions
    /// * `strike` - Swaption strike rate
    /// * `forward` - Forward swap rate
    fn lognormal_vol(&self, expiry: T, tenor: T, strike: T, forward: T) -> Result<T, MfmError>;

    /// Returns the ATM normal vol (convenience method).
    ///
    /// Defaults to `normal_vol(expiry, tenor, forward, forward)`.
    fn atm_normal_vol(&self, expiry: T, tenor: T, forward: T) -> Result<T, MfmError> {
        self.normal_vol(expiry, tenor, forward, forward)
    }

    /// Returns the ATM lognormal vol (convenience method).
    ///
    /// Defaults to `lognormal_vol(expiry, tenor, forward, forward)`.
    fn atm_lognormal_vol(&self, expiry: T, tenor: T, forward: T) -> Result<T, MfmError> {
        self.lognormal_vol(expiry, tenor, forward, forward)
    }
}

// ─── Flat vol cube ──────────────────────────────────────────────────

/// Flat (constant) swaption volatility cube.
///
/// Returns the same normal and lognormal volatilities for all queries,
/// regardless of expiry, tenor, or strike. Primarily useful for testing
/// and as a baseline.
#[derive(Debug, Clone)]
pub struct FlatSwaptionVolCube<T: Float> {
    /// Constant normal (Bachelier) volatility.
    pub normal_vol_value: T,
    /// Constant lognormal (Black) volatility.
    pub lognormal_vol_value: T,
}

impl<T: Float> FlatSwaptionVolCube<T> {
    /// Creates a new flat swaption vol cube with explicit normal and
    /// lognormal volatilities.
    ///
    /// # Errors
    ///
    /// Returns [`MfmError::InvalidParameter`] if either vol is not
    /// strictly positive.
    pub fn new(normal_vol: T, lognormal_vol: T) -> Result<Self, MfmError> {
        if normal_vol <= T::zero() {
            return Err(MfmError::InvalidParameter {
                name: "normal_vol",
                reason: "must be strictly positive".to_string(),
            });
        }
        if lognormal_vol <= T::zero() {
            return Err(MfmError::InvalidParameter {
                name: "lognormal_vol",
                reason: "must be strictly positive".to_string(),
            });
        }
        Ok(Self {
            normal_vol_value: normal_vol,
            lognormal_vol_value: lognormal_vol,
        })
    }

    /// Creates a flat vol cube from a normal vol, using a default
    /// lognormal vol of 0.20 (20%).
    ///
    /// # Errors
    ///
    /// Returns [`MfmError::InvalidParameter`] if the vol is not
    /// strictly positive.
    pub fn from_normal_vol(vol: T) -> Result<Self, MfmError> {
        if vol <= T::zero() {
            return Err(MfmError::InvalidParameter {
                name: "normal_vol",
                reason: "must be strictly positive".to_string(),
            });
        }
        Ok(Self {
            normal_vol_value: vol,
            lognormal_vol_value: from_f64(0.20),
        })
    }

    /// Creates a flat vol cube from a lognormal vol, using a default
    /// normal vol of 0.005 (50 bps).
    ///
    /// # Errors
    ///
    /// Returns [`MfmError::InvalidParameter`] if the vol is not
    /// strictly positive.
    pub fn from_lognormal_vol(vol: T) -> Result<Self, MfmError> {
        if vol <= T::zero() {
            return Err(MfmError::InvalidParameter {
                name: "lognormal_vol",
                reason: "must be strictly positive".to_string(),
            });
        }
        Ok(Self {
            normal_vol_value: from_f64(0.005),
            lognormal_vol_value: vol,
        })
    }
}

impl<T: Float> SwaptionVolCube<T> for FlatSwaptionVolCube<T> {
    fn normal_vol(&self, _expiry: T, _tenor: T, _strike: T, _forward: T) -> Result<T, MfmError> {
        Ok(self.normal_vol_value)
    }

    fn lognormal_vol(
        &self,
        _expiry: T,
        _tenor: T,
        _strike: T,
        _forward: T,
    ) -> Result<T, MfmError> {
        Ok(self.lognormal_vol_value)
    }
}

// ─── SABR vol cube ─────────────────────────────────────────────────

/// SABR-based swaption volatility cube.
///
/// Stores per-(expiry, tenor) SABR parameters in row-major order and
/// evaluates implied volatilities via the Hagan et al. (2002) formula.
/// Nearest-neighbour lookup is used when the query point does not
/// coincide exactly with a grid node.
#[derive(Debug, Clone)]
pub struct SabrSwaptionVolCube<T: Float> {
    /// Expiry grid in year fractions (sorted ascending).
    pub expiries: Vec<T>,
    /// Tenor grid in year fractions (sorted ascending).
    pub tenors: Vec<T>,
    /// SABR alpha per (expiry, tenor) pair, stored row-major
    /// `[expiry_idx * num_tenors + tenor_idx]`.
    pub alphas: Vec<T>,
    /// SABR beta per (expiry, tenor) pair.
    pub betas: Vec<T>,
    /// SABR rho per (expiry, tenor) pair.
    pub rhos: Vec<T>,
    /// SABR nu (vol-of-vol) per (expiry, tenor) pair.
    pub nus: Vec<T>,
}

impl<T: Float> SabrSwaptionVolCube<T> {
    /// Creates a new SABR swaption vol cube.
    ///
    /// # Arguments
    ///
    /// * `expiries` - Sorted expiry year fractions
    /// * `tenors` - Sorted tenor year fractions
    /// * `alphas` - SABR alpha values, row-major over (expiry, tenor)
    /// * `betas` - SABR beta values
    /// * `rhos` - SABR rho values
    /// * `nus` - SABR nu values
    ///
    /// # Errors
    ///
    /// Returns [`MfmError::InvalidParameter`] if any dimension is
    /// empty or the parameter vectors are inconsistent.
    pub fn new(
        expiries: Vec<T>,
        tenors: Vec<T>,
        alphas: Vec<T>,
        betas: Vec<T>,
        rhos: Vec<T>,
        nus: Vec<T>,
    ) -> Result<Self, MfmError> {
        if expiries.is_empty() {
            return Err(MfmError::InvalidParameter {
                name: "expiries",
                reason: "must not be empty".to_string(),
            });
        }
        if tenors.is_empty() {
            return Err(MfmError::InvalidParameter {
                name: "tenors",
                reason: "must not be empty".to_string(),
            });
        }
        let expected_len = expiries.len() * tenors.len();
        if alphas.len() != expected_len {
            return Err(MfmError::InvalidParameter {
                name: "alphas",
                reason: format!(
                    "length {} does not match expiries({}) * tenors({}) = {}",
                    alphas.len(),
                    expiries.len(),
                    tenors.len(),
                    expected_len
                ),
            });
        }
        if betas.len() != expected_len {
            return Err(MfmError::InvalidParameter {
                name: "betas",
                reason: format!(
                    "length {} does not match expected {}",
                    betas.len(),
                    expected_len
                ),
            });
        }
        if rhos.len() != expected_len {
            return Err(MfmError::InvalidParameter {
                name: "rhos",
                reason: format!(
                    "length {} does not match expected {}",
                    rhos.len(),
                    expected_len
                ),
            });
        }
        if nus.len() != expected_len {
            return Err(MfmError::InvalidParameter {
                name: "nus",
                reason: format!(
                    "length {} does not match expected {}",
                    nus.len(),
                    expected_len
                ),
            });
        }
        Ok(Self {
            expiries,
            tenors,
            alphas,
            betas,
            rhos,
            nus,
        })
    }

    /// Finds the index of the nearest value in `grid` to `target`.
    fn nearest_index(grid: &[T], target: T) -> usize {
        let mut best_idx = 0;
        let mut best_dist = (grid[0] - target).abs();
        for (i, &val) in grid.iter().enumerate().skip(1) {
            let dist = (val - target).abs();
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }
        best_idx
    }

    /// Returns the flat index into the row-major parameter arrays for
    /// the given (expiry_idx, tenor_idx) pair.
    #[inline]
    fn flat_index(&self, expiry_idx: usize, tenor_idx: usize) -> usize {
        expiry_idx * self.tenors.len() + tenor_idx
    }

    /// Evaluates the SABR Hagan lognormal implied vol at the nearest
    /// grid node for the given query point.
    fn sabr_lognormal_vol(
        &self,
        expiry: T,
        tenor: T,
        strike: T,
        forward: T,
    ) -> Result<T, MfmError> {
        let ei = Self::nearest_index(&self.expiries, expiry);
        let ti = Self::nearest_index(&self.tenors, tenor);
        let idx = self.flat_index(ei, ti);

        let params = SabrImpliedVolParams {
            forward,
            alpha: self.alphas[idx],
            beta: self.betas[idx],
            nu: self.nus[idx],
            rho: self.rhos[idx],
            maturity: expiry,
        };

        sabr_implied_vol(&params, strike).map_err(|e| {
            MfmError::VolSurface(format!(
                "SABR implied vol failed at expiry={}, tenor={}: {}",
                expiry.to_f64().unwrap_or(0.0),
                tenor.to_f64().unwrap_or(0.0),
                e
            ))
        })
    }
}

impl<T: Float> SwaptionVolCube<T> for SabrSwaptionVolCube<T> {
    fn normal_vol(&self, expiry: T, tenor: T, strike: T, forward: T) -> Result<T, MfmError> {
        // Compute lognormal vol and convert to normal vol using the
        // first-order approximation: sigma_n ~ sigma_ln * forward.
        let ln_vol = self.sabr_lognormal_vol(expiry, tenor, strike, forward)?;
        Ok(ln_vol * forward)
    }

    fn lognormal_vol(&self, expiry: T, tenor: T, strike: T, forward: T) -> Result<T, MfmError> {
        self.sabr_lognormal_vol(expiry, tenor, strike, forward)
    }
}

// ─── Dispatch enum ──────────────────────────────────────────────────

/// Static-dispatch enum wrapping all swaption vol cube variants.
///
/// Uses `enum_dispatch` for zero-cost dynamic polymorphism, keeping
/// the code Enzyme-friendly (no trait objects).
#[derive(Debug, Clone)]
#[enum_dispatch(SwaptionVolCube<T>)]
pub enum SwaptionVolCubeEnum<T: Float> {
    /// Flat (constant) volatility cube.
    Flat(FlatSwaptionVolCube<T>),
    /// SABR-based volatility cube.
    Sabr(SabrSwaptionVolCube<T>),
}

// ─── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_flat_vol_cube_normal() {
        let cube = FlatSwaptionVolCube::new(0.005_f64, 0.20).unwrap();
        let vol = cube.normal_vol(1.0, 5.0, 0.03, 0.03).unwrap();
        assert_relative_eq!(vol, 0.005, epsilon = 1e-12);
    }

    #[test]
    fn test_flat_vol_cube_lognormal() {
        let cube = FlatSwaptionVolCube::new(0.005_f64, 0.20).unwrap();
        let vol = cube.lognormal_vol(1.0, 5.0, 0.03, 0.03).unwrap();
        assert_relative_eq!(vol, 0.20, epsilon = 1e-12);
    }

    #[test]
    fn test_flat_vol_cube_atm() {
        let cube = FlatSwaptionVolCube::new(0.005_f64, 0.20).unwrap();
        let normal = cube.atm_normal_vol(2.0, 10.0, 0.04).unwrap();
        let lognormal = cube.atm_lognormal_vol(2.0, 10.0, 0.04).unwrap();
        assert_relative_eq!(normal, 0.005, epsilon = 1e-12);
        assert_relative_eq!(lognormal, 0.20, epsilon = 1e-12);
    }

    #[test]
    fn test_flat_vol_cube_validation() {
        // Zero normal vol rejected.
        let result = FlatSwaptionVolCube::new(0.0_f64, 0.20);
        assert!(result.is_err());

        // Negative lognormal vol rejected.
        let result = FlatSwaptionVolCube::new(0.005_f64, -0.10);
        assert!(result.is_err());

        // Negative normal vol in from_normal_vol rejected.
        let result = FlatSwaptionVolCube::<f64>::from_normal_vol(-0.001);
        assert!(result.is_err());

        // Zero lognormal vol in from_lognormal_vol rejected.
        let result = FlatSwaptionVolCube::<f64>::from_lognormal_vol(0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_flat_from_normal_vol() {
        let cube = FlatSwaptionVolCube::<f64>::from_normal_vol(0.008).unwrap();
        assert_relative_eq!(cube.normal_vol_value, 0.008, epsilon = 1e-12);
        assert_relative_eq!(cube.lognormal_vol_value, 0.20, epsilon = 1e-12);
    }

    #[test]
    fn test_flat_from_lognormal_vol() {
        let cube = FlatSwaptionVolCube::<f64>::from_lognormal_vol(0.25).unwrap();
        assert_relative_eq!(cube.normal_vol_value, 0.005, epsilon = 1e-12);
        assert_relative_eq!(cube.lognormal_vol_value, 0.25, epsilon = 1e-12);
    }

    #[test]
    fn test_enum_dispatch() {
        let flat = FlatSwaptionVolCube::new(0.005_f64, 0.20).unwrap();
        let cube_enum = SwaptionVolCubeEnum::Flat(flat);

        let normal = cube_enum.normal_vol(1.0, 5.0, 0.03, 0.03).unwrap();
        assert_relative_eq!(normal, 0.005, epsilon = 1e-12);

        let lognormal = cube_enum.lognormal_vol(1.0, 5.0, 0.03, 0.03).unwrap();
        assert_relative_eq!(lognormal, 0.20, epsilon = 1e-12);

        let atm_n = cube_enum.atm_normal_vol(1.0, 5.0, 0.03).unwrap();
        assert_relative_eq!(atm_n, 0.005, epsilon = 1e-12);
    }

    #[test]
    fn test_sabr_vol_cube_basic() {
        // Single (1x1) expiry-tenor grid.
        let cube = SabrSwaptionVolCube::new(
            vec![1.0_f64],       // expiries
            vec![5.0],           // tenors
            vec![0.03],          // alphas
            vec![0.5],           // betas
            vec![-0.3],          // rhos
            vec![0.4],           // nus
        )
        .unwrap();

        let forward = 0.03_f64;
        let strike = 0.03_f64;

        let ln_vol = cube.lognormal_vol(1.0, 5.0, strike, forward).unwrap();
        assert!(ln_vol > 0.0, "lognormal vol should be positive");
        assert!(ln_vol < 5.0, "lognormal vol should be reasonable");

        let n_vol = cube.normal_vol(1.0, 5.0, strike, forward).unwrap();
        assert!(n_vol > 0.0, "normal vol should be positive");
        // Normal vol ~ lognormal vol * forward
        assert_relative_eq!(n_vol, ln_vol * forward, epsilon = 1e-12);
    }

    #[test]
    fn test_sabr_vol_cube_nearest_lookup() {
        // 2x2 grid with different alphas.
        let cube = SabrSwaptionVolCube::new(
            vec![1.0_f64, 5.0],  // expiries
            vec![5.0, 10.0],     // tenors
            vec![0.03, 0.04, 0.05, 0.06], // alphas
            vec![0.5, 0.5, 0.5, 0.5],     // betas
            vec![-0.3, -0.3, -0.3, -0.3], // rhos
            vec![0.4, 0.4, 0.4, 0.4],     // nus
        )
        .unwrap();

        let forward = 0.03;

        // Query near (1.0, 5.0) -> should use alpha=0.03
        let vol_1_5 = cube.lognormal_vol(0.9, 4.5, forward, forward).unwrap();
        // Query near (5.0, 10.0) -> should use alpha=0.06
        let vol_5_10 = cube.lognormal_vol(4.8, 9.5, forward, forward).unwrap();

        // Different alphas produce different vols.
        assert!(
            (vol_1_5 - vol_5_10).abs() > 1e-6,
            "different grid nodes should produce different vols"
        );
    }

    #[test]
    fn test_sabr_vol_cube_dimension_validation() {
        // Mismatched alpha length.
        let result = SabrSwaptionVolCube::new(
            vec![1.0_f64, 5.0],
            vec![5.0, 10.0],
            vec![0.03],          // should be 4
            vec![0.5, 0.5, 0.5, 0.5],
            vec![-0.3, -0.3, -0.3, -0.3],
            vec![0.4, 0.4, 0.4, 0.4],
        );
        assert!(result.is_err());

        // Empty expiries.
        let result = SabrSwaptionVolCube::<f64>::new(
            vec![],
            vec![5.0],
            vec![],
            vec![],
            vec![],
            vec![],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_sabr_enum_dispatch() {
        let sabr = SabrSwaptionVolCube::new(
            vec![1.0_f64],
            vec![5.0],
            vec![0.03],
            vec![0.5],
            vec![-0.3],
            vec![0.4],
        )
        .unwrap();
        let cube_enum = SwaptionVolCubeEnum::Sabr(sabr);

        let forward = 0.03;
        let vol = cube_enum.lognormal_vol(1.0, 5.0, forward, forward).unwrap();
        assert!(vol > 0.0);
    }
}
