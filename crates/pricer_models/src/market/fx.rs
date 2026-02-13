//! FX forward curve traits and implementations.

use enum_dispatch::enum_dispatch;
use infra_domain::trade::instrument_def::CurrencyPair;
use num_traits::Float;

use super::{
    curves::{CurveEnum, FlatCurve, YieldCurve},
    MarketDataError,
};

/// Trait for FX forward curves providing forward rates.
#[enum_dispatch]
pub trait FxCurve<T: Float> {
    /// Returns the spot exchange rate.
    fn spot(&self) -> T;

    /// Returns the forward exchange rate for time `t` (in years).
    ///
    /// The forward rate is typically computed using Interest Rate Parity:
    /// F(t) = S × exp((rd - rf) × t)
    fn forward_rate(&self, t: T) -> Result<T, MarketDataError>;

    /// Returns the currency pair for this curve.
    fn currency_pair(&self) -> CurrencyPair;
}

/// A flat FX forward curve with constant forward points.
#[derive(Debug, Clone, Copy)]
pub struct FlatFxCurve<T: Float> {
    spot: T,
    forward_points_per_year: T,
    currency_pair: CurrencyPair,
}

impl<T: Float> FlatFxCurve<T> {
    /// Creates a new flat FX curve.
    pub fn new(spot: T, forward_points_per_year: T, currency_pair: CurrencyPair) -> Self {
        Self {
            spot,
            forward_points_per_year,
            currency_pair,
        }
    }
}

impl<T: Float> FxCurve<T> for FlatFxCurve<T> {
    fn spot(&self) -> T { self.spot }

    fn forward_rate(&self, t: T) -> Result<T, MarketDataError> {
        if t < T::zero() {
            return Err(MarketDataError::InvalidInput {
                message: "time cannot be negative".to_string(),
            });
        }
        // F(t) = S × (1 + fwd_pts × t) ≈ S × exp(fwd_pts × t)
        Ok(self.spot * (self.forward_points_per_year * t).exp())
    }

    fn currency_pair(&self) -> CurrencyPair { self.currency_pair }
}

/// An FX forward curve based on Interest Rate Parity (IRP).
///
/// Uses domestic and foreign yield curves to compute forward rates:
/// F(t) = S × exp((rd - rf) × t) = S × df_foreign(t) / df_domestic(t)
#[derive(Debug, Clone)]
pub struct IrpFxCurve<T: Float, D: YieldCurve<T>, F: YieldCurve<T>> {
    spot: T,
    domestic_curve: D,
    foreign_curve: F,
    currency_pair: CurrencyPair,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Float, D: YieldCurve<T>, F: YieldCurve<T>> IrpFxCurve<T, D, F> {
    /// Creates a new IRP-based FX curve.
    pub fn new(spot: T, domestic_curve: D, foreign_curve: F, currency_pair: CurrencyPair) -> Self {
        Self {
            spot,
            domestic_curve,
            foreign_curve,
            currency_pair,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Returns a reference to the domestic yield curve.
    pub fn domestic_curve(&self) -> &D { &self.domestic_curve }

    /// Returns a reference to the foreign yield curve.
    pub fn foreign_curve(&self) -> &F { &self.foreign_curve }
}

impl<T: Float, D: YieldCurve<T>, F: YieldCurve<T>> FxCurve<T> for IrpFxCurve<T, D, F> {
    fn spot(&self) -> T { self.spot }

    fn forward_rate(&self, t: T) -> Result<T, MarketDataError> {
        if t < T::zero() {
            return Err(MarketDataError::InvalidInput {
                message: "time cannot be negative".to_string(),
            });
        }
        if t <= T::zero() {
            return Ok(self.spot);
        }

        // F(t) = S × df_foreign(t) / df_domestic(t)
        let df_dom = self.domestic_curve.discount_factor(t)?;
        let df_for = self.foreign_curve.discount_factor(t)?;

        if df_dom <= T::zero() {
            return Err(MarketDataError::InvalidInput {
                message: "domestic discount factor must be positive".to_string(),
            });
        }

        Ok(self.spot * df_for / df_dom)
    }

    fn currency_pair(&self) -> CurrencyPair { self.currency_pair }
}

/// Enum wrapper for different FX curve types (static dispatch via
/// `enum_dispatch`).
#[derive(Debug, Clone)]
#[enum_dispatch(FxCurve<T>)]
pub enum FxCurveEnum<T: Float> {
    /// Flat FX curve with constant forward points.
    Flat(FlatFxCurve<T>),
    /// IRP-based FX curve using FlatCurve for both legs.
    IrpFlat(IrpFxCurve<T, FlatCurve<T>, FlatCurve<T>>),
    /// IRP-based FX curve using CurveEnum for both legs.
    IrpGeneric(IrpFxCurve<T, CurveEnum<T>, CurveEnum<T>>),
}

impl<T: Float> FxCurveEnum<T> {
    /// Creates a flat FX curve.
    pub fn flat(spot: T, forward_points_per_year: T, currency_pair: CurrencyPair) -> Self {
        Self::Flat(FlatFxCurve::new(
            spot,
            forward_points_per_year,
            currency_pair,
        ))
    }

    /// Creates an IRP-based FX curve from flat yield curves.
    pub fn irp_flat(
        spot: T,
        domestic_rate: T,
        foreign_rate: T,
        currency_pair: CurrencyPair,
    ) -> Self {
        let dom_curve = FlatCurve::new(domestic_rate);
        let for_curve = FlatCurve::new(foreign_rate);
        Self::IrpFlat(IrpFxCurve::new(spot, dom_curve, for_curve, currency_pair))
    }

    /// Creates an IRP-based FX curve from generic yield curves.
    pub fn irp_generic(
        spot: T,
        domestic_curve: CurveEnum<T>,
        foreign_curve: CurveEnum<T>,
        currency_pair: CurrencyPair,
    ) -> Self {
        Self::IrpGeneric(IrpFxCurve::new(
            spot,
            domestic_curve,
            foreign_curve,
            currency_pair,
        ))
    }
}

#[cfg(test)]
mod tests {
    use infra_domain::{market::Currency, trade::instrument_def::CurrencyPair};

    use super::*;

    #[test]
    fn test_flat_fx_curve() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let fwd_pts = 0.02_f64; // 2% forward points per year
        let curve = FlatFxCurve::new(1.10, fwd_pts, pair);

        // At spot
        let spot = curve.spot();
        assert!((spot - 1.10).abs() < 1e-10);

        // At 1 year
        let fwd_1y = curve.forward_rate(1.0).unwrap();
        let expected = 1.10 * (0.02_f64).exp();
        assert!((fwd_1y - expected).abs() < 1e-10);
    }

    #[test]
    fn test_irp_fx_curve() {
        let pair = CurrencyPair::new(Currency::EUR, Currency::USD);
        let dom_curve = FlatCurve::new(0.03_f64); // USD rate
        let for_curve = FlatCurve::new(0.01_f64); // EUR rate

        let fx_curve = IrpFxCurve::new(1.10, dom_curve, for_curve, pair);

        // Spot
        assert!((fx_curve.spot() - 1.10).abs() < 1e-10);

        // Forward at 1 year: F = S × exp((rd - rf) × T) = 1.10 × exp(0.02)
        let fwd_1y = fx_curve.forward_rate(1.0).unwrap();
        let expected = 1.10 * (0.02_f64).exp();
        assert!((fwd_1y - expected).abs() < 1e-10);

        // Forward at 0 should be spot
        let fwd_0 = fx_curve.forward_rate(0.0).unwrap();
        assert!((fwd_0 - 1.10).abs() < 1e-10);
    }

    #[test]
    fn test_fx_curve_enum_irp_flat() {
        let pair = CurrencyPair::new(Currency::USD, Currency::JPY);
        let fx_curve = FxCurveEnum::irp_flat(150.0, 0.05, 0.01, pair);

        // Forward at 1 year: F = 150 × exp(0.04) ≈ 156.12
        let fwd_1y = fx_curve.forward_rate(1.0).unwrap();
        let expected = 150.0 * (0.04_f64).exp();
        assert!((fwd_1y - expected).abs() < 1e-8);
    }

    #[test]
    fn test_fx_curve_currency_pair() {
        let pair = CurrencyPair::new(Currency::GBP, Currency::USD);
        let fx_curve: FxCurveEnum<f64> = FxCurveEnum::flat(1.25, 0.01, pair);

        assert_eq!(fx_curve.currency_pair(), pair);
    }

    #[test]
    fn test_irp_fx_curve_with_negative_spread() {
        // Case where foreign rate > domestic rate (forward < spot)
        let pair = CurrencyPair::new(Currency::USD, Currency::JPY);
        let dom_curve = FlatCurve::new(0.01_f64); // JPY rate (low)
        let for_curve = FlatCurve::new(0.05_f64); // USD rate (high)

        let fx_curve = IrpFxCurve::new(150.0, dom_curve, for_curve, pair);

        // Forward at 1 year: F = 150 × exp(-0.04) < 150
        let fwd_1y = fx_curve.forward_rate(1.0).unwrap();
        assert!(fwd_1y < 150.0);
        let expected = 150.0 * (-0.04_f64).exp();
        assert!((fwd_1y - expected).abs() < 1e-8);
    }
}
