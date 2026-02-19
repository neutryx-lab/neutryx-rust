//! Volatility surface abstractions and implementations.
//!
//! Provides a unified trait ([`VolSurface`]) and concrete implementations
//! for querying implied and local volatilities across strike-expiry space.
//!
//! ## Implementations
//!
//! | Surface | Description |
//! |---------|-------------|
//! | [`BlackScholesVol`] | Flat constant volatility |
//! | [`SabrSurface`] | SABR term-structure with Hagan formula |
//! | [`LocalVolSurface`] | Dupire local-vol grid |
//!
//! ## Example
//!
//! ```
//! use pricer_models::market::vol::{VolSurfaceEnum, BlackScholesVol};
//!
//! let surface = VolSurfaceEnum::<f64>::flat(0.2).unwrap();
//! assert!(surface.is_parametric());
//! ```

pub mod black_scholes;
pub mod interp;
pub mod local_vol;
pub mod mixture_lognormal;
pub mod polynomial;
pub mod sabr;
pub mod ssvi;
pub mod svi;
pub mod vanna_volga;
pub mod variance_gamma;
pub mod zabr;

// Re-export public types for convenient access.
pub use black_scholes::BlackScholesVol;
use enum_dispatch::enum_dispatch;
pub use local_vol::LocalVolSurface;
pub use mixture_lognormal::MixtureLognormalSurface;
pub use polynomial::PolynomialVolSurface;
use pricer_core::traits::Float;
pub use sabr::{SabrParams, SabrSliceParams, SabrSurface};
pub use ssvi::SsviSurface;
pub use svi::SviSurface;
pub use vanna_volga::VannaVolgaSurface;
pub use variance_gamma::VarianceGammaSurface;
pub use zabr::ZabrSurface;

// ─── Error type ───────────────────────────────────────────────────────

/// Errors arising from volatility surface operations.
#[derive(Debug, Clone, PartialEq)]
pub enum VolSurfaceError {
    /// The input parameters are invalid (e.g. negative sigma, empty grid).
    InvalidInput(String),
    /// Interpolation could not produce a result for the query point.
    InterpolationFailed(String),
    /// The query point lies outside the calibrated region and
    /// extrapolation is not permitted.
    ExtrapolationNotAllowed,
    /// The requested operation is not supported by this surface type.
    UnsupportedOperation(String),
    /// SABR formula returned an error.
    SabrError(String),
}

impl std::fmt::Display for VolSurfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(msg) => write!(f, "invalid input: {}", msg),
            Self::InterpolationFailed(msg) => write!(f, "interpolation failed: {}", msg),
            Self::ExtrapolationNotAllowed => write!(f, "extrapolation not allowed"),
            Self::UnsupportedOperation(msg) => write!(f, "unsupported operation: {}", msg),
            Self::SabrError(msg) => write!(f, "SABR error: {}", msg),
        }
    }
}

impl std::error::Error for VolSurfaceError {}

// ─── Trait ────────────────────────────────────────────────────────────

/// Core trait for querying a volatility surface.
#[enum_dispatch]
pub trait VolSurface<T: Float> {
    /// Returns the Black-Scholes implied volatility for the given point.
    fn implied_vol(&self, strike: T, expiry: T, forward: T) -> Result<T, VolSurfaceError>;

    /// Returns the Dupire local volatility for the given point.
    ///
    /// The default implementation returns an error; override in surface
    /// types that natively provide local vol (e.g. `LocalVolSurface`).
    fn local_vol(&self, _strike: T, _expiry: T, _forward: T) -> Result<T, VolSurfaceError> {
        Err(VolSurfaceError::UnsupportedOperation(
            "local vol not available for this surface type".to_string(),
        ))
    }

    /// Returns the at-the-money implied volatility for the given expiry.
    ///
    /// Defaults to `implied_vol(forward, expiry, forward)`.
    fn atm_vol(&self, expiry: T, forward: T) -> Result<T, VolSurfaceError> {
        self.implied_vol(forward, expiry, forward)
    }
}

// ─── Dispatch enum ────────────────────────────────────────────────────

/// Static-dispatch enum wrapping all supported vol surface types.
///
/// Uses `enum_dispatch` for zero-cost dynamic polymorphism, keeping
/// the code Enzyme-friendly (no trait objects).
#[derive(Debug, Clone)]
#[enum_dispatch(VolSurface<T>)]
pub enum VolSurfaceEnum<T: Float> {
    /// Flat Black-Scholes volatility.
    BlackScholes(BlackScholesVol<T>),
    /// SABR term-structure surface.
    Sabr(SabrSurface<T>),
    /// Dupire local-volatility grid.
    LocalVol(LocalVolSurface<T>),
    /// SVI (Stochastic Volatility Inspired) surface.
    Svi(SviSurface<T>),
    /// SSVI (Surface SVI) arbitrage-free surface.
    Ssvi(SsviSurface<T>),
    /// Vanna-Volga FX surface.
    VannaVolga(VannaVolgaSurface<T>),
    /// ZABR generalised SABR surface.
    Zabr(ZabrSurface<T>),
    /// Mixture of Lognormals surface.
    MixtureLn(MixtureLognormalSurface<T>),
    /// Polynomial total-variance surface.
    Polynomial(PolynomialVolSurface<T>),
    /// Variance Gamma surface.
    VarianceGamma(VarianceGammaSurface<T>),
}

impl<T: Float> VolSurfaceEnum<T> {
    /// Creates a flat Black-Scholes surface with the given constant sigma.
    pub fn flat(sigma: T) -> Result<Self, VolSurfaceError> {
        BlackScholesVol::new(sigma).map(Self::BlackScholes)
    }

    /// Wraps an existing SABR surface.
    pub fn sabr(surface: SabrSurface<T>) -> Self { Self::Sabr(surface) }

    /// Wraps an existing local-vol surface.
    pub fn local_vol(surface: LocalVolSurface<T>) -> Self { Self::LocalVol(surface) }

    /// Wraps an existing SVI surface.
    pub fn svi(surface: SviSurface<T>) -> Self { Self::Svi(surface) }

    /// Wraps an existing SSVI surface.
    pub fn ssvi(surface: SsviSurface<T>) -> Self { Self::Ssvi(surface) }

    /// Wraps an existing Vanna-Volga surface.
    pub fn vanna_volga(surface: VannaVolgaSurface<T>) -> Self { Self::VannaVolga(surface) }

    /// Wraps an existing ZABR surface.
    pub fn zabr(surface: ZabrSurface<T>) -> Self { Self::Zabr(surface) }

    /// Wraps an existing Mixture Lognormal surface.
    pub fn mixture_ln(surface: MixtureLognormalSurface<T>) -> Self { Self::MixtureLn(surface) }

    /// Wraps an existing Polynomial surface.
    pub fn polynomial(surface: PolynomialVolSurface<T>) -> Self { Self::Polynomial(surface) }

    /// Wraps an existing Variance Gamma surface.
    pub fn variance_gamma(surface: VarianceGammaSurface<T>) -> Self { Self::VarianceGamma(surface) }

    /// Returns `true` for parametric models that produce implied
    /// volatilities directly; `false` for grid-based models (local vol)
    /// that require numerical inversion.
    pub fn is_parametric(&self) -> bool { !matches!(self, Self::LocalVol(_)) }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_flat_factory() {
        let surface = VolSurfaceEnum::<f64>::flat(0.2).unwrap();
        assert!(surface.is_parametric());
    }

    #[test]
    fn test_flat_factory_invalid() {
        let result = VolSurfaceEnum::<f64>::flat(-0.1);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_parametric_local_vol() {
        let lv = LocalVolSurface::new(vec![100.0_f64], vec![1.0], vec![0.2]).unwrap();
        let surface = VolSurfaceEnum::local_vol(lv);
        assert!(!surface.is_parametric());
    }

    #[test]
    fn test_enum_dispatch_implied_vol() {
        let surface = VolSurfaceEnum::<f64>::flat(0.25).unwrap();
        let vol = surface.implied_vol(100.0, 1.0, 100.0).unwrap();
        assert_relative_eq!(vol, 0.25);
    }

    #[test]
    fn test_enum_dispatch_atm_vol() {
        let surface = VolSurfaceEnum::<f64>::flat(0.30).unwrap();
        let vol = surface.atm_vol(1.0, 100.0).unwrap();
        assert_relative_eq!(vol, 0.30);
    }

    #[test]
    fn test_error_display() {
        let e = VolSurfaceError::InvalidInput("bad value".to_string());
        assert_eq!(format!("{}", e), "invalid input: bad value");

        let e = VolSurfaceError::ExtrapolationNotAllowed;
        assert_eq!(format!("{}", e), "extrapolation not allowed");
    }
}
