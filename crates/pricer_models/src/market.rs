//! Market data structures for yield curves, volatility surfaces, and calibration.
//!
//! This module provides the core market data types used throughout the pricing library.

use num_traits::Float;
use pricer_core::math::numeric::from_f64;
use thiserror::Error;

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during market data operations.
#[derive(Debug, Clone, Error)]
pub enum MarketDataError {
    /// Interpolation failed at a given point.
    #[error("Interpolation failed: {reason}")]
    InterpolationFailed {
        /// Reason for the failure.
        reason: String,
    },

    /// Curve not found.
    #[error("Curve not found: {name}")]
    CurveNotFound {
        /// Name of the missing curve.
        name: String,
    },

    /// Invalid input data.
    #[error("Invalid input: {message}")]
    InvalidInput {
        /// Description of the invalid input.
        message: String,
    },

    /// Maturity out of range.
    #[error("Maturity {maturity} out of range (max: {max_maturity})")]
    MaturityOutOfRange {
        /// Requested maturity.
        maturity: f64,
        /// Maximum allowed maturity.
        max_maturity: f64,
    },
}

// =============================================================================
// Curves Module
// =============================================================================

/// Yield curve traits and implementations.
pub mod curves {
    use super::*;

    /// Trait for yield curves providing discount factors and rates.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Floating-point type (e.g., `f64`) for AD compatibility
    pub trait YieldCurve<T: Float> {
        /// Returns the discount factor for time `t` (in years).
        fn discount_factor(&self, t: T) -> Result<T, MarketDataError>;

        /// Returns the continuously compounded zero rate for time `t`.
        fn zero_rate(&self, t: T) -> Result<T, MarketDataError> {
            let df = self.discount_factor(t)?;
            if t <= T::zero() {
                return Ok(T::zero());
            }
            Ok(-df.ln() / t)
        }

        /// Returns the forward rate between times `t1` and `t2`.
        fn forward_rate(&self, t1: T, t2: T) -> Result<T, MarketDataError> {
            let df1 = if t1 <= T::zero() {
                T::one()
            } else {
                self.discount_factor(t1)?
            };
            let df2 = self.discount_factor(t2)?;
            let tau = t2 - t1;

            if tau <= T::zero() {
                return Err(MarketDataError::InvalidInput {
                    message: "t2 must be greater than t1".to_string(),
                });
            }

            Ok((df1 / df2 - T::one()) / tau)
        }
    }

    /// A flat yield curve with constant continuously compounded rate.
    #[derive(Debug, Clone, Copy)]
    pub struct FlatCurve<T: Float> {
        rate: T,
    }

    impl<T: Float> FlatCurve<T> {
        /// Creates a new flat curve with the given rate.
        pub fn new(rate: T) -> Self {
            Self { rate }
        }

        /// Returns the constant rate.
        pub fn rate(&self) -> T {
            self.rate
        }
    }

    impl<T: Float> YieldCurve<T> for FlatCurve<T> {
        fn discount_factor(&self, t: T) -> Result<T, MarketDataError> {
            if t < T::zero() {
                return Err(MarketDataError::InvalidInput {
                    message: "time cannot be negative".to_string(),
                });
            }
            Ok((-self.rate * t).exp())
        }

        fn zero_rate(&self, _t: T) -> Result<T, MarketDataError> {
            Ok(self.rate)
        }

        fn forward_rate(&self, t1: T, t2: T) -> Result<T, MarketDataError> {
            if t2 <= t1 {
                return Err(MarketDataError::InvalidInput {
                    message: "t2 must be greater than t1".to_string(),
                });
            }
            Ok(self.rate)
        }
    }

    // =========================================================================
    // Bootstrapping Types
    // =========================================================================

    /// Payment frequency for financial instruments.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum Frequency {
        /// Daily payments
        Daily,
        /// Weekly payments
        Weekly,
        /// Monthly payments
        #[default]
        Monthly,
        /// Quarterly payments (4 per year)
        Quarterly,
        /// Semi-annual payments (2 per year)
        SemiAnnual,
        /// Annual payments (1 per year)
        Annual,
    }

    impl Frequency {
        /// Returns the period length in years.
        pub fn period_years<T: Float>(&self) -> T {
            match self {
                Frequency::Daily => from_f64(1.0 / 365.0),
                Frequency::Weekly => from_f64(1.0 / 52.0),
                Frequency::Monthly => from_f64(1.0 / 12.0),
                Frequency::Quarterly => from_f64(0.25),
                Frequency::SemiAnnual => from_f64(0.5),
                Frequency::Annual => from_f64(1.0),
            }
        }
    }

    /// Interpolation method for bootstrapped curves.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub enum BootstrapInterpolation {
        /// Linear interpolation on discount factors.
        Linear,
        /// Log-linear interpolation (linear on log of discount factors).
        #[default]
        LogLinear,
        /// Cubic spline interpolation.
        CubicSpline,
    }

    /// Market instrument for yield curve calibration.
    #[derive(Debug, Clone)]
    pub enum MarketInstrument<T: Float> {
        /// Overnight Index Swap
        Ois {
            /// Maturity in years
            maturity: T,
            /// Market-quoted rate
            rate: T,
            /// Payment frequency
            payment_frequency: Frequency,
        },
        /// Interest Rate Swap
        Irs {
            /// Maturity in years
            maturity: T,
            /// Market-quoted rate
            rate: T,
            /// Fixed leg payment frequency
            fixed_frequency: Frequency,
        },
        /// Forward Rate Agreement
        Fra {
            /// Start time in years
            start: T,
            /// End time in years
            end: T,
            /// Market-quoted rate
            rate: T,
        },
        /// Interest Rate Future
        Future {
            /// Maturity in years
            maturity: T,
            /// Market-quoted rate (derived from price)
            rate: T,
            /// Convexity adjustment
            convexity_adjustment: T,
        },
    }

    impl<T: Float> MarketInstrument<T> {
        /// Creates an OIS instrument.
        pub fn ois(maturity: T, rate: T) -> Self {
            Self::Ois {
                maturity,
                rate,
                payment_frequency: Frequency::Annual,
            }
        }

        /// Creates an IRS instrument.
        pub fn irs(maturity: T, rate: T) -> Self {
            Self::Irs {
                maturity,
                rate,
                fixed_frequency: Frequency::SemiAnnual,
            }
        }

        /// Creates a FRA instrument.
        pub fn fra(start: T, end: T, rate: T) -> Self {
            Self::Fra { start, end, rate }
        }

        /// Creates a Future instrument.
        pub fn future(maturity: T, rate: T) -> Self {
            Self::Future {
                maturity,
                rate,
                convexity_adjustment: T::zero(),
            }
        }

        /// Returns the market-quoted rate.
        pub fn rate(&self) -> T {
            match self {
                Self::Ois { rate, .. } => *rate,
                Self::Irs { rate, .. } => *rate,
                Self::Fra { rate, .. } => *rate,
                Self::Future { rate, .. } => *rate,
            }
        }

        /// Returns the instrument's maturity.
        pub fn maturity(&self) -> T {
            match self {
                Self::Ois { maturity, .. } => *maturity,
                Self::Irs { maturity, .. } => *maturity,
                Self::Fra { end, .. } => *end,
                Self::Future { maturity, .. } => *maturity,
            }
        }

        /// Returns a descriptive name for the instrument type.
        pub fn instrument_type(&self) -> &'static str {
            match self {
                Self::Ois { .. } => "OIS",
                Self::Irs { .. } => "IRS",
                Self::Fra { .. } => "FRA",
                Self::Future { .. } => "Future",
            }
        }
    }

    /// A bootstrapped yield curve with pillar discount factors.
    #[derive(Debug, Clone)]
    pub struct BootstrappedCurve<T: Float> {
        /// Pillar maturities in years.
        pillars: Vec<T>,
        /// Discount factors at each pillar.
        discount_factors: Vec<T>,
        /// Interpolation method.
        interpolation: BootstrapInterpolation,
        /// Whether to allow extrapolation.
        allow_extrapolation: bool,
    }

    impl<T: Float> BootstrappedCurve<T> {
        /// Creates a new bootstrapped curve.
        pub fn new(
            pillars: Vec<T>,
            discount_factors: Vec<T>,
            interpolation: BootstrapInterpolation,
            allow_extrapolation: bool,
        ) -> Result<Self, String> {
            if pillars.len() != discount_factors.len() {
                return Err("pillars and discount_factors must have same length".to_string());
            }
            if pillars.is_empty() {
                return Err("curve must have at least one pillar".to_string());
            }
            Ok(Self {
                pillars,
                discount_factors,
                interpolation,
                allow_extrapolation,
            })
        }

        /// Returns the pillar maturities.
        pub fn pillars(&self) -> &[T] {
            &self.pillars
        }

        /// Returns the discount factors.
        pub fn discount_factors(&self) -> &[T] {
            &self.discount_factors
        }
    }

    impl<T: Float> YieldCurve<T> for BootstrappedCurve<T> {
        fn discount_factor(&self, t: T) -> Result<T, MarketDataError> {
            if t <= T::zero() {
                return Ok(T::one());
            }

            let n = self.pillars.len();
            let max_t = self.pillars[n - 1];

            if t > max_t && !self.allow_extrapolation {
                return Err(MarketDataError::MaturityOutOfRange {
                    maturity: t.to_f64().unwrap_or(0.0),
                    max_maturity: max_t.to_f64().unwrap_or(0.0),
                });
            }

            // Handle single-pillar curve: flat rate extrapolation
            if n == 1 {
                let t1 = self.pillars[0];
                let df1 = self.discount_factors[0];
                if t1 > T::zero() && df1 > T::zero() {
                    // Derive zero rate and extrapolate: df(t) = exp(-r * t) where r = -ln(df1)/t1
                    let r = -df1.ln() / t1;
                    return Ok((-r * t).exp());
                }
                return Ok(df1);
            }

            // Find interpolation interval
            let mut i = 0;
            while i < n - 1 && self.pillars[i + 1] < t {
                i += 1;
            }

            if i >= n - 1 {
                // Extrapolation: use last segment
                i = n - 2;
            }

            let t1 = self.pillars[i];
            let t2 = self.pillars[i + 1];
            let df1 = self.discount_factors[i];
            let df2 = self.discount_factors[i + 1];

            let w = if t2 > t1 {
                (t - t1) / (t2 - t1)
            } else {
                T::zero()
            };

            match self.interpolation {
                BootstrapInterpolation::Linear => Ok(df1 * (T::one() - w) + df2 * w),
                BootstrapInterpolation::LogLinear => {
                    let log_df = df1.ln() * (T::one() - w) + df2.ln() * w;
                    Ok(log_df.exp())
                }
                BootstrapInterpolation::CubicSpline => {
                    // Simplified: fall back to log-linear
                    let log_df = df1.ln() * (T::one() - w) + df2.ln() * w;
                    Ok(log_df.exp())
                }
            }
        }
    }

    // Re-export from parent module for compatibility with curves:: path
    pub use super::{CurveEnum, CurveName, CurveSet};
}

// =============================================================================
// Curve Identification and Collections
// =============================================================================

/// Named curve identifiers for common rate indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CurveName {
    /// SOFR (Secured Overnight Financing Rate)
    Sofr,
    /// EURIBOR (Euro Interbank Offered Rate)
    Euribor,
    /// ESTR (Euro Short-Term Rate)
    Estr,
    /// TONAR (Tokyo Overnight Average Rate) / TONA
    Tonar,
    /// SONIA (Sterling Overnight Index Average)
    Sonia,
    /// Generic discount curve
    Discount,
    /// Generic forward curve
    Forward,
    /// Custom curve with a name
    Custom(&'static str),
}

/// Enum wrapper for different curve types (static dispatch).
#[derive(Debug, Clone)]
pub enum CurveEnum<T: Float> {
    /// Flat curve with constant rate.
    Flat(curves::FlatCurve<T>),
    /// Bootstrapped curve from market instruments.
    Bootstrapped(curves::BootstrappedCurve<T>),
}

impl<T: Float> CurveEnum<T> {
    /// Creates a flat curve with the given rate.
    pub fn flat(rate: T) -> Self {
        Self::Flat(curves::FlatCurve::new(rate))
    }

    /// Creates a bootstrapped curve.
    pub fn bootstrapped(curve: curves::BootstrappedCurve<T>) -> Self {
        Self::Bootstrapped(curve)
    }
}

impl<T: Float> curves::YieldCurve<T> for CurveEnum<T> {
    fn discount_factor(&self, t: T) -> Result<T, MarketDataError> {
        match self {
            Self::Flat(c) => c.discount_factor(t),
            Self::Bootstrapped(c) => c.discount_factor(t),
        }
    }

    fn zero_rate(&self, t: T) -> Result<T, MarketDataError> {
        match self {
            Self::Flat(c) => c.zero_rate(t),
            Self::Bootstrapped(c) => c.zero_rate(t),
        }
    }

    fn forward_rate(&self, t1: T, t2: T) -> Result<T, MarketDataError> {
        match self {
            Self::Flat(c) => c.forward_rate(t1, t2),
            Self::Bootstrapped(c) => c.forward_rate(t1, t2),
        }
    }
}

/// A collection of named yield curves.
#[derive(Debug, Clone, Default)]
pub struct CurveSet<T: Float> {
    curves: std::collections::HashMap<CurveName, CurveEnum<T>>,
}

impl<T: Float + 'static> CurveSet<T> {
    /// Creates a new empty curve set.
    pub fn new() -> Self {
        Self {
            curves: std::collections::HashMap::new(),
        }
    }

    /// Inserts a curve with the given name.
    pub fn insert(&mut self, name: CurveName, curve: CurveEnum<T>) {
        self.curves.insert(name, curve);
    }

    /// Gets a curve by name.
    pub fn get(&self, name: &CurveName) -> Option<&CurveEnum<T>> {
        self.curves.get(name)
    }

    /// Returns the number of curves.
    pub fn len(&self) -> usize {
        self.curves.len()
    }

    /// Returns true if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.curves.is_empty()
    }

    /// Computes the forward rate for a rate index between two times.
    ///
    /// Maps the rate index to a curve name and looks up the forward rate.
    pub fn forward_rate_for_index(
        &self,
        rate_index: infra_master::market::RateIndex,
        t1: T,
        t2: T,
    ) -> Result<T, MarketDataError> {
        use curves::YieldCurve;
        use infra_master::market::RateIndex;

        let curve_name = match rate_index {
            RateIndex::Sofr => CurveName::Sofr,
            RateIndex::Euribor3M | RateIndex::Euribor6M => CurveName::Euribor,
            RateIndex::Estr => CurveName::Estr,
            RateIndex::Tonar => CurveName::Tonar,
            RateIndex::Sonia => CurveName::Sonia,
            RateIndex::Saron => CurveName::Custom("SARON"),
            _ => CurveName::Sofr, // Default fallback for unknown indices
        };

        let curve = self.curves.get(&curve_name).ok_or(MarketDataError::CurveNotFound {
            name: format!("{:?}", rate_index),
        })?;

        curve.forward_rate(t1, t2)
    }

    /// Returns an iterator over (name, curve) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&CurveName, &CurveEnum<T>)> {
        self.curves.iter()
    }

    /// Gets the discount curve if set.
    pub fn discount_curve(&self) -> Option<&CurveEnum<T>> {
        self.curves.get(&CurveName::Discount)
    }

    /// Sets the discount curve.
    pub fn set_discount_curve(&mut self, _name: CurveName) {
        // Placeholder - actual implementation would handle curve assignment
    }
}

// =============================================================================
// Market Provider (Placeholder)
// =============================================================================

/// Market data provider for pricing operations.
///
/// This is a placeholder type that will be fully implemented
/// when the market data infrastructure is complete.
#[derive(Debug, Clone, Default)]
pub struct MarketProvider {
    curve_set: CurveSet<f64>,
}

impl MarketProvider {
    /// Creates a new market provider.
    pub fn new() -> Self {
        Self {
            curve_set: CurveSet::new(),
        }
    }

    /// Gets a curve for the given currency.
    pub fn get_curve(&self, _currency: infra_master::market::Currency) -> Option<&CurveEnum<f64>> {
        // Placeholder: return first available curve or None
        self.curve_set.curves.values().next()
    }

    /// Returns the curve set.
    pub fn curve_set(&self) -> &CurveSet<f64> {
        &self.curve_set
    }
}

// =============================================================================
// FX Curves Module
// =============================================================================

/// FX forward curve traits and implementations.
pub mod fx_curves {
    use super::*;
    use super::curves::YieldCurve;
    use infra_master::trade::instrument_def::CurrencyPair;

    /// Trait for FX forward curves providing forward rates.
    ///
    /// An FX forward curve represents the term structure of forward exchange rates.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Floating-point type (e.g., `f64`) for AD compatibility
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
        ///
        /// # Arguments
        ///
        /// * `spot` - Spot exchange rate
        /// * `forward_points_per_year` - Forward points per year (F/S - 1 per year)
        /// * `currency_pair` - Currency pair
        pub fn new(spot: T, forward_points_per_year: T, currency_pair: CurrencyPair) -> Self {
            Self {
                spot,
                forward_points_per_year,
                currency_pair,
            }
        }
    }

    impl<T: Float> FxCurve<T> for FlatFxCurve<T> {
        fn spot(&self) -> T {
            self.spot
        }

        fn forward_rate(&self, t: T) -> Result<T, MarketDataError> {
            if t < T::zero() {
                return Err(MarketDataError::InvalidInput {
                    message: "time cannot be negative".to_string(),
                });
            }
            // F(t) = S × (1 + fwd_pts × t) ≈ S × exp(fwd_pts × t)
            Ok(self.spot * (self.forward_points_per_year * t).exp())
        }

        fn currency_pair(&self) -> CurrencyPair {
            self.currency_pair
        }
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
        ///
        /// # Arguments
        ///
        /// * `spot` - Spot exchange rate (domestic per foreign)
        /// * `domestic_curve` - Domestic currency yield curve
        /// * `foreign_curve` - Foreign currency yield curve
        /// * `currency_pair` - Currency pair
        pub fn new(
            spot: T,
            domestic_curve: D,
            foreign_curve: F,
            currency_pair: CurrencyPair,
        ) -> Self {
            Self {
                spot,
                domestic_curve,
                foreign_curve,
                currency_pair,
                _phantom: std::marker::PhantomData,
            }
        }

        /// Returns a reference to the domestic yield curve.
        pub fn domestic_curve(&self) -> &D {
            &self.domestic_curve
        }

        /// Returns a reference to the foreign yield curve.
        pub fn foreign_curve(&self) -> &F {
            &self.foreign_curve
        }
    }

    impl<T: Float, D: YieldCurve<T>, F: YieldCurve<T>> FxCurve<T> for IrpFxCurve<T, D, F> {
        fn spot(&self) -> T {
            self.spot
        }

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

        fn currency_pair(&self) -> CurrencyPair {
            self.currency_pair
        }
    }

    // Re-export from parent module
    pub use super::FxCurveEnum;
}

/// Enum wrapper for different FX curve types (static dispatch).
#[derive(Debug, Clone)]
pub enum FxCurveEnum<T: Float> {
    /// Flat FX curve with constant forward points.
    Flat(fx_curves::FlatFxCurve<T>),
    /// IRP-based FX curve using FlatCurve for both legs.
    IrpFlat(fx_curves::IrpFxCurve<T, curves::FlatCurve<T>, curves::FlatCurve<T>>),
    /// IRP-based FX curve using CurveEnum for both legs.
    IrpGeneric(fx_curves::IrpFxCurve<T, CurveEnum<T>, CurveEnum<T>>),
}

impl<T: Float> FxCurveEnum<T> {
    /// Creates a flat FX curve.
    pub fn flat(
        spot: T,
        forward_points_per_year: T,
        currency_pair: infra_master::trade::instrument_def::CurrencyPair,
    ) -> Self {
        Self::Flat(fx_curves::FlatFxCurve::new(spot, forward_points_per_year, currency_pair))
    }

    /// Creates an IRP-based FX curve from flat yield curves.
    pub fn irp_flat(
        spot: T,
        domestic_rate: T,
        foreign_rate: T,
        currency_pair: infra_master::trade::instrument_def::CurrencyPair,
    ) -> Self {
        let dom_curve = curves::FlatCurve::new(domestic_rate);
        let for_curve = curves::FlatCurve::new(foreign_rate);
        Self::IrpFlat(fx_curves::IrpFxCurve::new(spot, dom_curve, for_curve, currency_pair))
    }

    /// Creates an IRP-based FX curve from generic yield curves.
    pub fn irp_generic(
        spot: T,
        domestic_curve: CurveEnum<T>,
        foreign_curve: CurveEnum<T>,
        currency_pair: infra_master::trade::instrument_def::CurrencyPair,
    ) -> Self {
        Self::IrpGeneric(fx_curves::IrpFxCurve::new(spot, domestic_curve, foreign_curve, currency_pair))
    }
}

impl<T: Float> fx_curves::FxCurve<T> for FxCurveEnum<T> {
    fn spot(&self) -> T {
        match self {
            Self::Flat(c) => c.spot(),
            Self::IrpFlat(c) => c.spot(),
            Self::IrpGeneric(c) => c.spot(),
        }
    }

    fn forward_rate(&self, t: T) -> Result<T, MarketDataError> {
        match self {
            Self::Flat(c) => c.forward_rate(t),
            Self::IrpFlat(c) => c.forward_rate(t),
            Self::IrpGeneric(c) => c.forward_rate(t),
        }
    }

    fn currency_pair(&self) -> infra_master::trade::instrument_def::CurrencyPair {
        match self {
            Self::Flat(c) => c.currency_pair(),
            Self::IrpFlat(c) => c.currency_pair(),
            Self::IrpGeneric(c) => c.currency_pair(),
        }
    }
}

// Re-export commonly used types at module level
pub use curves::{
    BootstrapInterpolation, BootstrappedCurve, FlatCurve, Frequency, MarketInstrument, YieldCurve,
};
pub use fx_curves::{FlatFxCurve, FxCurve, IrpFxCurve};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_curve_discount_factor() {
        let curve = FlatCurve::new(0.05_f64);
        let df = curve.discount_factor(1.0).unwrap();
        assert!((df - (-0.05_f64).exp()).abs() < 1e-10);
    }

    #[test]
    fn test_bootstrapped_curve() {
        let pillars = vec![1.0_f64, 2.0, 5.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();
        let curve =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        let df = curve.discount_factor(1.5).unwrap();
        assert!(df > 0.0 && df < 1.0);
    }

    // =========================================================================
    // FX Curve Tests
    // =========================================================================

    #[test]
    fn test_flat_fx_curve() {
        use infra_master::trade::instrument_def::CurrencyPair;
        use infra_master::Currency;

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
        use infra_master::trade::instrument_def::CurrencyPair;
        use infra_master::Currency;

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
        use infra_master::trade::instrument_def::CurrencyPair;
        use infra_master::Currency;

        let pair = CurrencyPair::new(Currency::USD, Currency::JPY);
        let fx_curve = FxCurveEnum::irp_flat(150.0, 0.05, 0.01, pair);

        // Forward at 1 year: F = 150 × exp(0.04) ≈ 156.12
        let fwd_1y = fx_curve.forward_rate(1.0).unwrap();
        let expected = 150.0 * (0.04_f64).exp();
        assert!((fwd_1y - expected).abs() < 1e-8);
    }

    #[test]
    fn test_fx_curve_currency_pair() {
        use infra_master::trade::instrument_def::CurrencyPair;
        use infra_master::Currency;

        let pair = CurrencyPair::new(Currency::GBP, Currency::USD);
        let fx_curve: FxCurveEnum<f64> = FxCurveEnum::flat(1.25, 0.01, pair);

        assert_eq!(fx_curve.currency_pair(), pair);
    }

    #[test]
    fn test_irp_fx_curve_with_negative_spread() {
        use infra_master::trade::instrument_def::CurrencyPair;
        use infra_master::Currency;

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
