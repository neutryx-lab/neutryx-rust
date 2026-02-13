//! Market data structures for yield curves, volatility surfaces, and
//! calibration.
//!
//! This module provides the core market data types used throughout the pricing
//! library.

use num_traits::Float;
use pricer_core::math::numeric::from_f64;
use thiserror::Error;

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

/// Yield curve traits and implementations.
pub mod curves {
    use enum_dispatch::enum_dispatch;

    use super::*;

    /// Trait for yield curves providing discount factors and rates.
    #[enum_dispatch]
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
        pub fn new(rate: T) -> Self { Self { rate } }

        /// Returns the constant rate.
        pub fn rate(&self) -> T { self.rate }
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

        fn zero_rate(&self, _t: T) -> Result<T, MarketDataError> { Ok(self.rate) }

        fn forward_rate(&self, t1: T, t2: T) -> Result<T, MarketDataError> {
            if t2 <= t1 {
                return Err(MarketDataError::InvalidInput {
                    message: "t2 must be greater than t1".to_string(),
                });
            }
            Ok(self.rate)
        }
    }

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
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum BootstrapInterpolation {
        /// Linear interpolation on discount factors.
        #[serde(rename = "linear_df", alias = "linear")]
        Linear,
        /// Log-linear interpolation (linear on log of discount factors).
        #[default]
        #[serde(rename = "log_linear_df", alias = "log_linear")]
        LogLinear,
        /// Flat forward interpolation (constant simple forward rate between
        /// pillars).
        ///
        /// DF interpolation is identical to LogLinear: `DF(t) = DF_i^(1-w) *
        /// DF_{i+1}^w`. The distinction is in forward rate computation:
        /// FlatForward guarantees that the simple forward rate `F(t,
        /// t+δ)` is constant within each pillar interval when δ matches
        /// the index's accrual period (e.g. 1/360 for ACT/360 indices).
        FlatForward,
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
        /// Central bank meeting or scheduled event (rate jump)
        Event {
            /// Time to event date in years
            maturity: T,
            /// Expected rate jump (absolute rate, not bps)
            expected_jump: T,
        },
        /// Fixed-coupon bond (government or corporate)
        Bond {
            /// Maturity in years
            maturity: T,
            /// Market-quoted yield-to-maturity
            rate: T,
            /// Fixed coupon rate (annual equivalent)
            coupon_rate: T,
            /// Coupon payment frequency
            payment_frequency: Frequency,
        },
        /// Credit Default Swap for credit curve calibration.
        ///
        /// The "rate" is the CDS par spread; the curve being bootstrapped is
        /// a survival probability curve (discount_factor = survival_probability).
        Cds {
            /// Maturity in years
            maturity: T,
            /// CDS par spread (market-quoted, as decimal e.g. 0.01 = 100bp)
            spread: T,
            /// Recovery rate (e.g. 0.40)
            recovery_rate: T,
            /// Pre-sampled risk-free discount factors at quarterly intervals.
            /// Stored as (time, df) pairs to avoid runtime curve dependency.
            risk_free_dfs: Vec<(T, T)>,
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
        pub fn fra(start: T, end: T, rate: T) -> Self { Self::Fra { start, end, rate } }

        /// Creates a Future instrument.
        pub fn future(maturity: T, rate: T) -> Self {
            Self::Future {
                maturity,
                rate,
                convexity_adjustment: T::zero(),
            }
        }

        /// Creates an Event instrument (rate jump in basis points).
        pub fn event(maturity: T, expected_jump_bps: T) -> Self {
            // Convert basis points to absolute rate
            let expected_jump = expected_jump_bps * from_f64::<T>(0.0001);
            Self::Event {
                maturity,
                expected_jump,
            }
        }

        /// Creates an Event instrument with absolute rate jump.
        pub fn event_with_rate(maturity: T, expected_jump: T) -> Self {
            Self::Event {
                maturity,
                expected_jump,
            }
        }

        /// Creates a Bond instrument with semi-annual coupons (default for
        /// government bonds).
        pub fn bond(maturity: T, ytm: T, coupon_rate: T) -> Self {
            Self::Bond {
                maturity,
                rate: ytm,
                coupon_rate,
                payment_frequency: Frequency::SemiAnnual,
            }
        }

        /// Creates a Bond instrument with explicit coupon frequency.
        pub fn bond_with_frequency(
            maturity: T,
            ytm: T,
            coupon_rate: T,
            payment_frequency: Frequency,
        ) -> Self {
            Self::Bond {
                maturity,
                rate: ytm,
                coupon_rate,
                payment_frequency,
            }
        }

        /// Creates a CDS instrument with pre-sampled risk-free discount factors.
        pub fn cds(maturity: T, spread: T, recovery_rate: T, risk_free_dfs: Vec<(T, T)>) -> Self {
            Self::Cds {
                maturity,
                spread,
                recovery_rate,
                risk_free_dfs,
            }
        }

        /// Returns the market-quoted rate (or expected jump for Event).
        pub fn rate(&self) -> T {
            match self {
                Self::Ois { rate, .. } => *rate,
                Self::Irs { rate, .. } => *rate,
                Self::Fra { rate, .. } => *rate,
                Self::Future { rate, .. } => *rate,
                Self::Event { expected_jump, .. } => *expected_jump,
                Self::Bond { rate, .. } => *rate,
                Self::Cds { spread, .. } => *spread,
            }
        }

        /// Returns the instrument's maturity.
        pub fn maturity(&self) -> T {
            match self {
                Self::Ois { maturity, .. } => *maturity,
                Self::Irs { maturity, .. } => *maturity,
                Self::Fra { end, .. } => *end,
                Self::Future { maturity, .. } => *maturity,
                Self::Event { maturity, .. } => *maturity,
                Self::Bond { maturity, .. } => *maturity,
                Self::Cds { maturity, .. } => *maturity,
            }
        }

        /// Returns a descriptive name for the instrument type.
        pub fn instrument_type(&self) -> &'static str {
            match self {
                Self::Ois { .. } => "OIS",
                Self::Irs { .. } => "IRS",
                Self::Fra { .. } => "FRA",
                Self::Future { .. } => "Future",
                Self::Event { .. } => "Event",
                Self::Bond { .. } => "Bond",
                Self::Cds { .. } => "CDS",
            }
        }

        /// Returns true if this is an Event instrument.
        pub fn is_event(&self) -> bool { matches!(self, Self::Event { .. }) }

        /// Returns the expected jump for Event instruments, None otherwise.
        pub fn expected_jump(&self) -> Option<T> {
            match self {
                Self::Event { expected_jump, .. } => Some(*expected_jump),
                _ => None,
            }
        }

        /// Creates a copy of this instrument with the market rate bumped by
        /// `delta`.
        ///
        /// Used for finite-difference Jacobian computation in the sequential
        /// bootstrapper.
        pub fn with_bumped_rate(&self, delta: T) -> Self {
            match self {
                Self::Ois {
                    maturity,
                    rate,
                    payment_frequency,
                } => Self::Ois {
                    maturity: *maturity,
                    rate: *rate + delta,
                    payment_frequency: *payment_frequency,
                },
                Self::Irs {
                    maturity,
                    rate,
                    fixed_frequency,
                } => Self::Irs {
                    maturity: *maturity,
                    rate: *rate + delta,
                    fixed_frequency: *fixed_frequency,
                },
                Self::Fra { start, end, rate } => Self::Fra {
                    start: *start,
                    end: *end,
                    rate: *rate + delta,
                },
                Self::Future {
                    maturity,
                    rate,
                    convexity_adjustment,
                } => Self::Future {
                    maturity: *maturity,
                    rate: *rate + delta,
                    convexity_adjustment: *convexity_adjustment,
                },
                Self::Event {
                    maturity,
                    expected_jump,
                } => Self::Event {
                    maturity: *maturity,
                    expected_jump: *expected_jump + delta,
                },
                Self::Bond {
                    maturity,
                    rate,
                    coupon_rate,
                    payment_frequency,
                } => Self::Bond {
                    maturity: *maturity,
                    rate: *rate + delta,
                    coupon_rate: *coupon_rate,
                    payment_frequency: *payment_frequency,
                },
                Self::Cds {
                    maturity,
                    spread,
                    recovery_rate,
                    ref risk_free_dfs,
                } => Self::Cds {
                    maturity: *maturity,
                    spread: *spread + delta,
                    recovery_rate: *recovery_rate,
                    risk_free_dfs: risk_free_dfs.clone(),
                },
            }
        }
    }

    /// A bootstrapped yield curve with pillar discount factors.
    ///
    /// Supports jump-aware interpolation for modelling rate discontinuities
    /// at central bank meeting dates.
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
        /// Jump events: (time, cumulative_offset) pairs.
        /// The offset is in log-space: adjusted_df = df *
        /// exp(cumulative_offset)
        jumps: Vec<(T, T)>,
    }

    /// Decomposition of a forward rate into continuous and jump components.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct ForwardRateDecomposition<T> {
        /// Continuous component of the forward rate.
        pub continuous: T,
        /// Jump component of the forward rate.
        pub jump: T,
        /// Total forward rate (continuous + jump).
        pub total: T,
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
                jumps: Vec::new(),
            })
        }

        /// Adds jump data `(time, cumulative_log_offset)` to the curve.
        pub fn with_jumps(mut self, jumps: Vec<(T, T)>) -> Self {
            self.jumps = jumps;
            self
        }

        /// Returns the pillar maturities.
        pub fn pillars(&self) -> &[T] { &self.pillars }

        /// Returns the discount factors.
        pub fn discount_factors(&self) -> &[T] { &self.discount_factors }

        /// Returns true if the curve has jump data.
        pub fn has_jumps(&self) -> bool { !self.jumps.is_empty() }

        /// Returns the jump data.
        pub fn jumps(&self) -> &[(T, T)] { &self.jumps }

        /// Returns the cumulative jump offset at time `t`.
        ///
        /// Uses binary search for O(log n) lookup.
        fn cumulative_offset_at(&self, t: T) -> T {
            if self.jumps.is_empty() {
                return T::zero();
            }

            let idx = self.jumps.partition_point(|(jump_t, _)| *jump_t <= t);
            if idx == 0 {
                T::zero()
            } else {
                self.jumps[idx - 1].1
            }
        }

        /// Returns the cumulative jump offset just before time `t` (left
        /// limit).
        fn cumulative_offset_before(&self, t: T) -> T {
            if self.jumps.is_empty() {
                return T::zero();
            }

            let idx = self.jumps.partition_point(|(jump_t, _)| *jump_t < t);
            if idx == 0 {
                T::zero()
            } else {
                self.jumps[idx - 1].1
            }
        }

        /// Returns the discount factor with limit specification (Left, Right,
        /// or Continuous).
        pub fn discount_factor_with_limit(
            &self,
            t: T,
            limit: pricer_core::types::Limit,
        ) -> Result<T, MarketDataError> {
            use pricer_core::types::Limit;

            // Get base discount factor (without jump adjustment)
            let base_df = self.discount_factor_base(t)?;

            if self.jumps.is_empty() {
                return Ok(base_df);
            }

            // Get the appropriate offset based on limit
            let offset = match limit {
                Limit::Left => self.cumulative_offset_before(t),
                Limit::Right | Limit::Continuous => self.cumulative_offset_at(t),
            };

            // Apply offset: adjusted_df = base_df * exp(offset)
            Ok(base_df * offset.exp())
        }

        /// Base discount factor calculation (without jump adjustment).
        fn discount_factor_base(&self, t: T) -> Result<T, MarketDataError> {
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

            // Handle single-pillar curve
            if n == 1 {
                let t1 = self.pillars[0];
                let df1 = self.discount_factors[0];
                if t1 > T::zero() && df1 > T::zero() {
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
                BootstrapInterpolation::LogLinear | BootstrapInterpolation::FlatForward => {
                    let log_df = df1.ln() * (T::one() - w) + df2.ln() * w;
                    Ok(log_df.exp())
                }
            }
        }

        /// Returns the forward rate with limit specification.
        pub fn forward_rate_with_limit(
            &self,
            t1: T,
            t2: T,
            limit: pricer_core::types::Limit,
        ) -> Result<T, MarketDataError> {
            let df1 = if t1 <= T::zero() {
                T::one()
            } else {
                self.discount_factor_with_limit(t1, limit)?
            };
            let df2 = self.discount_factor_with_limit(t2, limit)?;
            let tau = t2 - t1;

            if tau <= T::zero() {
                return Err(MarketDataError::InvalidInput {
                    message: "t2 must be greater than t1".to_string(),
                });
            }

            Ok((df1 / df2 - T::one()) / tau)
        }

        /// Decomposes the forward rate into continuous and jump components.
        pub fn decompose_forward_rate(
            &self,
            t1: T,
            t2: T,
        ) -> Result<ForwardRateDecomposition<T>, MarketDataError> {
            use pricer_core::types::Limit;

            let tau = t2 - t1;
            if tau <= T::zero() {
                return Err(MarketDataError::InvalidInput {
                    message: "t2 must be greater than t1".to_string(),
                });
            }

            // Total rate (with jumps)
            let total = self.forward_rate_with_limit(t1, t2, Limit::Continuous)?;

            // Continuous rate (without jumps applied in forward calculation)
            // We compute this by getting the base forward rate
            let df1_base = if t1 <= T::zero() {
                T::one()
            } else {
                self.discount_factor_base(t1)?
            };
            let df2_base = self.discount_factor_base(t2)?;
            let continuous = (df1_base / df2_base - T::one()) / tau;

            // Jump component
            let jump = total - continuous;

            Ok(ForwardRateDecomposition {
                continuous,
                jump,
                total,
            })
        }

        /// Returns the discount factor and its gradient w.r.t. pillar DFs.
        pub fn discount_factor_with_gradient(&self, t: T) -> Result<(T, Vec<T>), MarketDataError> {
            self.discount_factor_gradient_impl(t, false)
        }

        /// Returns the discount factor and its gradient w.r.t. log(DF) values.
        pub fn discount_factor_with_log_gradient(
            &self,
            t: T,
        ) -> Result<(T, Vec<T>), MarketDataError> {
            self.discount_factor_gradient_impl(t, true)
        }

        /// Shared gradient implementation. When `log_mode` is true, computes
        /// dDF(t)/d_log(DF_i); otherwise dDF(t)/dDF_i.
        fn discount_factor_gradient_impl(
            &self,
            t: T,
            log_mode: bool,
        ) -> Result<(T, Vec<T>), MarketDataError> {
            let n = self.pillars.len();
            let mut gradient = vec![T::zero(); n];

            if t <= T::zero() {
                return Ok((T::one(), gradient));
            }

            let max_t = self.pillars[n - 1];
            if t > max_t && !self.allow_extrapolation {
                return Err(MarketDataError::MaturityOutOfRange {
                    maturity: t.to_f64().unwrap_or(0.0),
                    max_maturity: max_t.to_f64().unwrap_or(0.0),
                });
            }

            if n == 1 {
                let t1 = self.pillars[0];
                let df1 = self.discount_factors[0];
                if t1 > T::zero() && df1 > T::zero() {
                    let r = -df1.ln() / t1;
                    let df = (-r * t).exp();
                    gradient[0] = if log_mode {
                        df * (t / t1)
                    } else {
                        df * (t / (t1 * df1))
                    };
                    return Ok((df, gradient));
                }
                gradient[0] = if log_mode { df1 } else { T::one() };
                return Ok((df1, gradient));
            }

            let mut i = 0;
            while i < n - 1 && self.pillars[i + 1] < t {
                i += 1;
            }
            if i >= n - 1 {
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

            let df = match self.interpolation {
                BootstrapInterpolation::Linear => {
                    let df = df1 * (T::one() - w) + df2 * w;
                    if log_mode {
                        gradient[i] = df1 * (T::one() - w);
                        gradient[i + 1] = df2 * w;
                    } else {
                        gradient[i] = T::one() - w;
                        gradient[i + 1] = w;
                    }
                    df
                }
                BootstrapInterpolation::LogLinear | BootstrapInterpolation::FlatForward => {
                    let log_df = df1.ln() * (T::one() - w) + df2.ln() * w;
                    let df = log_df.exp();
                    if log_mode {
                        gradient[i] = df * (T::one() - w);
                        gradient[i + 1] = df * w;
                    } else {
                        gradient[i] = df * (T::one() - w) / df1;
                        gradient[i + 1] = df * w / df2;
                    }
                    df
                }
            };

            Ok((df, gradient))
        }
    }

    impl<T: Float> YieldCurve<T> for BootstrappedCurve<T> {
        fn discount_factor(&self, t: T) -> Result<T, MarketDataError> {
            // Default behavior uses Continuous limit (includes jumps, right-limit)
            self.discount_factor_with_limit(t, pricer_core::types::Limit::Continuous)
        }
    }

    /// Enum wrapper for different curve types (static dispatch via
    /// `enum_dispatch`).
    #[derive(Debug, Clone)]
    #[enum_dispatch(YieldCurve<T>)]
    pub enum CurveEnum<T: Float> {
        /// Flat curve with constant rate.
        Flat(FlatCurve<T>),
        /// Bootstrapped curve from market instruments.
        Bootstrapped(BootstrappedCurve<T>),
    }

    impl<T: Float> CurveEnum<T> {
        /// Creates a flat curve with the given rate.
        pub fn flat(rate: T) -> Self { Self::Flat(FlatCurve::new(rate)) }

        /// Creates a bootstrapped curve.
        pub fn bootstrapped(curve: BootstrappedCurve<T>) -> Self { Self::Bootstrapped(curve) }
    }

    // Re-export from parent module for compatibility with curves:: path
    pub use super::{CurveName, CurveSet};
}

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

// Re-export CurveEnum from curves module for backwards compatibility
pub use curves::CurveEnum;

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
    pub fn get(&self, name: &CurveName) -> Option<&CurveEnum<T>> { self.curves.get(name) }

    /// Returns the number of curves.
    pub fn len(&self) -> usize { self.curves.len() }

    /// Returns true if the set is empty.
    pub fn is_empty(&self) -> bool { self.curves.is_empty() }

    /// Computes the forward rate for a rate index between two times.
    ///
    /// Maps the rate index to a curve name and looks up the forward rate.
    pub fn forward_rate_for_index(
        &self,
        rate_index: infra_domain::market::RateIndex,
        t1: T,
        t2: T,
    ) -> Result<T, MarketDataError> {
        use curves::YieldCurve;
        use infra_domain::market::RateIndex;

        let curve_name = match rate_index {
            RateIndex::Sofr => CurveName::Sofr,
            RateIndex::Euribor3M | RateIndex::Euribor6M => CurveName::Euribor,
            RateIndex::Estr => CurveName::Estr,
            RateIndex::Tonar => CurveName::Tonar,
            RateIndex::Sonia => CurveName::Sonia,
            RateIndex::Saron => CurveName::Custom("SARON"),
            _ => CurveName::Sofr, // Default fallback for unknown indices
        };

        let curve = self
            .curves
            .get(&curve_name)
            .ok_or(MarketDataError::CurveNotFound {
                name: format!("{:?}", rate_index),
            })?;

        curve.forward_rate(t1, t2)
    }

    /// Returns an iterator over (name, curve) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&CurveName, &CurveEnum<T>)> { self.curves.iter() }

    /// Gets the discount curve if set.
    pub fn discount_curve(&self) -> Option<&CurveEnum<T>> { self.curves.get(&CurveName::Discount) }

    /// Sets the discount curve.
    pub fn set_discount_curve(&mut self, _name: CurveName) {
        // Placeholder - actual implementation would handle curve assignment
    }
}

/// Market data provider for pricing operations (placeholder).
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
    pub fn get_curve(&self, _currency: infra_domain::market::Currency) -> Option<&CurveEnum<f64>> {
        // Placeholder: return first available curve or None
        self.curve_set.curves.values().next()
    }

    /// Returns the curve set.
    pub fn curve_set(&self) -> &CurveSet<f64> { &self.curve_set }
}

/// FX forward curve traits and implementations.
pub mod fx_curves {
    use enum_dispatch::enum_dispatch;
    use infra_domain::trade::instrument_def::CurrencyPair;

    use super::{curves::YieldCurve, *};

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

    // Import CurveEnum from curves module for use in FxCurveEnum
    use super::curves::{CurveEnum, FlatCurve};

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
}

// Re-export FxCurveEnum from fx_curves module for backwards compatibility
// Re-export commonly used types at module level
pub use curves::{
    BootstrapInterpolation, BootstrappedCurve, FlatCurve, Frequency, MarketInstrument, YieldCurve,
};
pub use fx_curves::{FlatFxCurve, FxCurve, FxCurveEnum, IrpFxCurve};

/// Utilities for converting JumpPillar definitions to curve-compatible format.
pub mod jumps {
    use infra_domain::{
        market::definition::JumpPillar,
        time::{Date, DayCounter},
    };

    /// A jump entry (time in years, cumulative log-DF offset) for bootstrapped
    /// curves.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct JumpEntry {
        /// Time of the jump in years from valuation date.
        pub time: f64,
        /// Cumulative offset in log(df) space.
        ///
        /// Positive values decrease the discount factor (rate hike).
        pub cumulative_offset: f64,
    }

    impl JumpEntry {
        /// Creates a new jump entry.
        #[must_use]
        pub fn new(time: f64, cumulative_offset: f64) -> Self {
            Self {
                time,
                cumulative_offset,
            }
        }

        /// Returns the time of the jump.
        #[must_use]
        pub fn time(&self) -> f64 { self.time }

        /// Returns the cumulative offset.
        #[must_use]
        pub fn cumulative_offset(&self) -> f64 { self.cumulative_offset }

        /// Converts to a tuple (time, cumulative_offset).
        #[must_use]
        pub fn to_tuple(&self) -> (f64, f64) { (self.time, self.cumulative_offset) }
    }

    /// Converts JumpPillars to sorted JumpEntry with cumulative offsets.
    #[must_use]
    pub fn convert_jump_pillars(
        pillars: &[JumpPillar],
        valuation_date: Date,
        day_counter: DayCounter,
    ) -> Vec<JumpEntry> {
        if pillars.is_empty() {
            return Vec::new();
        }

        // Phase 1: Expand pillars into (time, individual_offset) entries.
        // For permanent jumps: one entry at jump_date.
        // For turn events (has end_date): two entries — spike up at jump_date,
        // spike down (revert) at end_date.
        let mut entries: Vec<(f64, f64)> = Vec::new();

        for p in pillars {
            let time = day_counter.year_fraction(valuation_date, p.jump_date());
            if time <= 0.0 {
                continue; // Skip past events
            }

            // Convert weighted bps to log-space offset: -bps / 10000
            // Negative because rate hike (positive bps) decreases discount factor
            let jump_offset = -p.weighted_jump_bps() / 10_000.0;
            entries.push((time, jump_offset));

            // Turn events: emit a reverting entry at end_date
            if let Some(end_date) = p.end_date() {
                let end_time = day_counter.year_fraction(valuation_date, end_date);
                if end_time > 0.0 {
                    entries.push((end_time, -jump_offset));
                }
            }
        }

        // Phase 2: Sort by time
        entries.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Phase 3: Calculate cumulative offsets
        let mut cumulative = 0.0;
        entries
            .into_iter()
            .map(|(time, jump_offset)| {
                cumulative += jump_offset;
                JumpEntry::new(time, cumulative)
            })
            .collect()
    }

    /// Convenience wrapper returning `(time, cumulative_offset)` tuples.
    #[must_use]
    pub fn convert_jump_pillars_to_tuples(
        pillars: &[JumpPillar],
        valuation_date: Date,
        day_counter: DayCounter,
    ) -> Vec<(f64, f64)> {
        convert_jump_pillars(pillars, valuation_date, day_counter)
            .into_iter()
            .map(|e| e.to_tuple())
            .collect()
    }

    /// Finds the cumulative jump offset at time `t` (binary search).
    #[must_use]
    pub fn cumulative_offset_at(jumps: &[JumpEntry], t: f64) -> f64 {
        if jumps.is_empty() {
            return 0.0;
        }

        // Binary search for the last jump with time <= t
        match jumps
            .binary_search_by(|j| j.time.partial_cmp(&t).unwrap_or(std::cmp::Ordering::Equal))
        {
            Ok(idx) => jumps[idx].cumulative_offset,
            Err(idx) => {
                if idx == 0 {
                    0.0
                } else {
                    jumps[idx - 1].cumulative_offset
                }
            }
        }
    }

    /// Finds the cumulative jump offset just before time `t` (left limit).
    #[must_use]
    pub fn cumulative_offset_before(jumps: &[JumpEntry], t: f64) -> f64 {
        if jumps.is_empty() {
            return 0.0;
        }

        // Find the last jump with time < t
        let idx = jumps.partition_point(|j| j.time < t);
        if idx == 0 {
            0.0
        } else {
            jumps[idx - 1].cumulative_offset
        }
    }

    /// Checks if there is a jump within `tolerance` of time `t`.
    #[must_use]
    pub fn has_jump_at(jumps: &[JumpEntry], t: f64, tolerance: f64) -> bool {
        jumps.iter().any(|j| (j.time - t).abs() < tolerance)
    }

    /// Builds a daily forward-rate-shift grid from jump pillars.
    ///
    /// Produces ramp offsets: `offset(t) = -Sum s_i * (t - t_i)` for `t_i <=
    /// t`, yielding `df(t) = base_df(t) * exp(offset(t))`.
    #[must_use]
    pub fn build_forward_rate_shift_grid(
        pillars: &[JumpPillar],
        valuation_date: Date,
        day_counter: DayCounter,
        max_time: f64,
    ) -> Vec<(f64, f64)> {
        if pillars.is_empty() {
            return Vec::new();
        }

        // Phase 1: Convert pillars to (time, delta_rate) shifts.
        // Permanent jumps: one shift at jump_date.
        // Turn events: spike up at jump_date, spike down (revert) at end_date.
        let mut rate_shifts: Vec<(f64, f64)> = Vec::new();

        for p in pillars {
            let time = day_counter.year_fraction(valuation_date, p.jump_date());
            if time <= 0.0 {
                continue; // Skip past events
            }

            // Convert confidence-weighted bps to decimal rate
            let delta_rate = p.weighted_jump_bps() / 10_000.0;
            rate_shifts.push((time, delta_rate));

            // Turn events: forward rate reverts at end_date
            if let Some(end_date) = p.end_date() {
                let end_time = day_counter.year_fraction(valuation_date, end_date);
                if end_time > 0.0 {
                    rate_shifts.push((end_time, -delta_rate));
                }
            }
        }

        rate_shifts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Phase 2: Build daily grid with ramp offsets.
        // offset(t) = -sum(s_i * (t - t_i)) for all t_i <= t
        //
        // Grid times MUST use year_fraction_from_days (d / N) to match the
        // query path in cumulative_offset_at. Using i * (1/N) would introduce
        // IEEE 754 rounding mismatches that shift forward rates by ±jump_bps
        // on affected days.
        let grid_count = (max_time / day_counter.year_fraction_from_days(1)).ceil() as usize + 2;

        (0..grid_count)
            .map(|i| {
                let t = day_counter.year_fraction_from_days(i as i64);
                let offset: f64 = rate_shifts
                    .iter()
                    .take_while(|(t_j, _)| *t_j <= t)
                    .map(|(t_j, shift)| -shift * (t - t_j))
                    .sum();
                (t, offset)
            })
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn test_valuation_date() -> Date { Date::from_ymd(2024, 1, 1).unwrap() }

        #[test]
        fn test_convert_empty_pillars() {
            let entries =
                convert_jump_pillars(&[], test_valuation_date(), DayCounter::Actual365Fixed);
            assert!(entries.is_empty());
        }

        #[test]
        fn test_convert_single_pillar() {
            let valuation = test_valuation_date();
            let jump_date = Date::from_ymd(2024, 3, 20).unwrap();
            let pillars = vec![JumpPillar::new(jump_date, 25.0, 0.8)];

            let entries = convert_jump_pillars(&pillars, valuation, DayCounter::Actual365Fixed);

            assert_eq!(entries.len(), 1);
            // Time should be approximately 79 days / 365 ≈ 0.2164
            let expected_time = DayCounter::Actual365Fixed.year_fraction(valuation, jump_date);
            assert!((entries[0].time - expected_time).abs() < 1e-10);
            // Weighted jump: 25 * 0.8 = 20 bps → -0.002 (negative for rate hike)
            assert!((entries[0].cumulative_offset - (-0.002)).abs() < 1e-10);
        }

        #[test]
        fn test_convert_multiple_pillars_sorted() {
            let valuation = test_valuation_date();
            let pillars = vec![
                JumpPillar::new(Date::from_ymd(2024, 6, 12).unwrap(), -25.0, 0.6),
                JumpPillar::new(Date::from_ymd(2024, 3, 20).unwrap(), 25.0, 0.8),
            ];

            let entries = convert_jump_pillars(&pillars, valuation, DayCounter::Actual365Fixed);

            assert_eq!(entries.len(), 2);
            // Should be sorted by time
            assert!(entries[0].time < entries[1].time);
            // First jump (March): 25 * 0.8 = 20 bps → -0.002 (negative for rate hike)
            assert!((entries[0].cumulative_offset - (-0.002)).abs() < 1e-10);
            // Second jump (June): cumulative = -0.002 + -(-25) * 0.6 / 10000 = -0.002 +
            // 0.0015 = -0.0005
            assert!((entries[1].cumulative_offset - (-0.0005)).abs() < 1e-10);
        }

        #[test]
        fn test_convert_filters_past_jumps() {
            let valuation = Date::from_ymd(2024, 6, 1).unwrap();
            let pillars = vec![
                JumpPillar::new(Date::from_ymd(2024, 3, 20).unwrap(), 25.0, 0.8), // Past
                JumpPillar::new(Date::from_ymd(2024, 9, 18).unwrap(), 50.0, 0.7), // Future
            ];

            let entries = convert_jump_pillars(&pillars, valuation, DayCounter::Actual365Fixed);

            assert_eq!(entries.len(), 1);
            // Only future jump should be included: 50 * 0.7 = 35 bps → -0.0035
            assert!((entries[0].cumulative_offset - (-0.0035)).abs() < 1e-10);
        }

        #[test]
        fn test_cumulative_offset_at() {
            let entries = vec![
                JumpEntry::new(0.25, 0.002),
                JumpEntry::new(0.50, 0.0035),
                JumpEntry::new(0.75, 0.006),
            ];

            // Before all jumps
            assert_eq!(cumulative_offset_at(&entries, 0.1), 0.0);

            // At first jump
            assert!((cumulative_offset_at(&entries, 0.25) - 0.002).abs() < 1e-10);

            // Between first and second jump
            assert!((cumulative_offset_at(&entries, 0.4) - 0.002).abs() < 1e-10);

            // At second jump
            assert!((cumulative_offset_at(&entries, 0.50) - 0.0035).abs() < 1e-10);

            // After all jumps
            assert!((cumulative_offset_at(&entries, 1.0) - 0.006).abs() < 1e-10);
        }

        #[test]
        fn test_cumulative_offset_at_empty() {
            let entries: Vec<JumpEntry> = vec![];
            assert_eq!(cumulative_offset_at(&entries, 0.5), 0.0);
        }

        #[test]
        fn test_cumulative_offset_before() {
            let entries = vec![JumpEntry::new(0.25, 0.002), JumpEntry::new(0.50, 0.0035)];

            // Before all jumps
            assert_eq!(cumulative_offset_before(&entries, 0.1), 0.0);

            // At first jump (should return 0, not 0.002)
            assert_eq!(cumulative_offset_before(&entries, 0.25), 0.0);

            // Between jumps
            assert!((cumulative_offset_before(&entries, 0.4) - 0.002).abs() < 1e-10);

            // At second jump (should return first jump's offset)
            assert!((cumulative_offset_before(&entries, 0.50) - 0.002).abs() < 1e-10);
        }

        #[test]
        fn test_has_jump_at() {
            let entries = vec![JumpEntry::new(0.25, 0.002), JumpEntry::new(0.50, 0.0035)];

            assert!(has_jump_at(&entries, 0.25, 1e-10));
            assert!(has_jump_at(&entries, 0.50, 1e-10));
            assert!(!has_jump_at(&entries, 0.30, 1e-10));
            assert!(!has_jump_at(&entries, 0.0, 1e-10));
        }

        #[test]
        fn test_jump_entry_methods() {
            let entry = JumpEntry::new(0.25, 0.002);

            assert_eq!(entry.time(), 0.25);
            assert_eq!(entry.cumulative_offset(), 0.002);
            assert_eq!(entry.to_tuple(), (0.25, 0.002));
        }

        #[test]
        fn test_convert_to_tuples() {
            let valuation = test_valuation_date();
            let pillars = vec![JumpPillar::new(
                Date::from_ymd(2024, 3, 20).unwrap(),
                25.0,
                0.8,
            )];

            let tuples =
                convert_jump_pillars_to_tuples(&pillars, valuation, DayCounter::Actual365Fixed);

            assert_eq!(tuples.len(), 1);
            // 25 * 0.8 = 20 bps → -0.002 (negative for rate hike)
            assert!((tuples[0].1 - (-0.002)).abs() < 1e-10);
        }

        #[test]
        fn test_convert_single_turn_pillar() {
            let valuation = test_valuation_date();
            let turn_start = Date::from_ymd(2024, 12, 31).unwrap();
            let turn_end = Date::from_ymd(2025, 1, 2).unwrap();

            let pillars = vec![JumpPillar::new(turn_start, 12.5, 1.0).with_end_date(turn_end)];

            let entries = convert_jump_pillars(&pillars, valuation, DayCounter::Actual365Fixed);

            // Should produce 2 entries: spike up + spike down
            assert_eq!(entries.len(), 2);

            let t_start = DayCounter::Actual365Fixed.year_fraction(valuation, turn_start);
            let t_end = DayCounter::Actual365Fixed.year_fraction(valuation, turn_end);

            // First entry: spike at turn_start
            assert!((entries[0].time - t_start).abs() < 1e-10);
            // 12.5bp * 1.0 = 12.5bp → -0.00125
            assert!((entries[0].cumulative_offset - (-0.00125)).abs() < 1e-10);

            // Second entry: revert at turn_end — cumulative returns to 0
            assert!((entries[1].time - t_end).abs() < 1e-10);
            assert!(entries[1].cumulative_offset.abs() < 1e-10);
        }

        #[test]
        fn test_convert_mixed_jump_and_turn() {
            let valuation = test_valuation_date();

            // Permanent jump: FOMC rate hike on March 20
            let jump = JumpPillar::new(Date::from_ymd(2024, 3, 20).unwrap(), 25.0, 1.0);

            // Turn: year-end spike from Dec 31 to Jan 2
            let turn = JumpPillar::new(Date::from_ymd(2024, 12, 31).unwrap(), 12.5, 1.0)
                .with_end_date(Date::from_ymd(2025, 1, 2).unwrap());

            let pillars = vec![jump, turn];
            let entries = convert_jump_pillars(&pillars, valuation, DayCounter::Actual365Fixed);

            // 3 entries: permanent jump + turn spike up + turn spike down
            assert_eq!(entries.len(), 3);

            // Entry 0: permanent jump at March 20
            // 25bp → -0.0025
            assert!((entries[0].cumulative_offset - (-0.0025)).abs() < 1e-10);

            // Entry 1: turn spike at Dec 31 (cumulative = -0.0025 + -0.00125 = -0.00375)
            assert!((entries[1].cumulative_offset - (-0.00375)).abs() < 1e-10);

            // Entry 2: turn revert at Jan 2 (cumulative = -0.00375 + 0.00125 = -0.0025)
            // Reverts to the permanent jump level
            assert!((entries[2].cumulative_offset - (-0.0025)).abs() < 1e-10);
        }

        #[test]
        fn test_convert_turn_offset_reverts() {
            let valuation = test_valuation_date();
            let turn_start = Date::from_ymd(2024, 6, 30).unwrap();
            let turn_end = Date::from_ymd(2024, 7, 1).unwrap();

            let pillars = vec![JumpPillar::new(turn_start, 5.0, 1.0).with_end_date(turn_end)];

            let entries = convert_jump_pillars(&pillars, valuation, DayCounter::Actual365Fixed);
            assert_eq!(entries.len(), 2);

            // Before turn: offset should be 0 (no entries before t_start)
            let t_start = entries[0].time;
            assert_eq!(cumulative_offset_at(&entries, t_start - 0.001), 0.0);

            // During turn: offset should be -0.0005
            assert!((cumulative_offset_at(&entries, t_start) - (-0.0005)).abs() < 1e-10);

            // After turn: offset should revert to 0
            let t_end = entries[1].time;
            assert!(cumulative_offset_at(&entries, t_end).abs() < 1e-10);
            assert!(cumulative_offset_at(&entries, t_end + 0.1).abs() < 1e-10);
        }

        #[test]
        fn test_permanent_jump_no_revert_entry() {
            let valuation = test_valuation_date();
            let pillars = vec![JumpPillar::new(
                Date::from_ymd(2024, 3, 20).unwrap(),
                25.0,
                1.0,
            )];

            let entries = convert_jump_pillars(&pillars, valuation, DayCounter::Actual365Fixed);

            // Permanent jump: exactly 1 entry, no revert
            assert_eq!(entries.len(), 1);
            assert!((entries[0].cumulative_offset - (-0.0025)).abs() < 1e-10);
        }
    }
}

// Re-export jump utilities
pub use jumps::{
    build_forward_rate_shift_grid, convert_jump_pillars, convert_jump_pillars_to_tuples, JumpEntry,
};

#[cfg(test)]
mod tests {
    use pricer_core::types::Limit;

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

    #[test]
    fn test_bootstrapped_curve_no_jumps_same_as_base() {
        // Without jumps, discount_factor should be the same as base
        let pillars = vec![0.25_f64, 0.5, 1.0, 2.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();
        let curve = BootstrappedCurve::new(
            pillars.clone(),
            dfs.clone(),
            BootstrapInterpolation::LogLinear,
            true,
        )
        .unwrap();

        // No jumps - all limits should give same result
        let df_cont = curve
            .discount_factor_with_limit(0.75, Limit::Continuous)
            .unwrap();
        let df_left = curve.discount_factor_with_limit(0.75, Limit::Left).unwrap();
        let df_right = curve
            .discount_factor_with_limit(0.75, Limit::Right)
            .unwrap();

        assert!((df_cont - df_left).abs() < 1e-12);
        assert!((df_cont - df_right).abs() < 1e-12);
        assert!(!curve.has_jumps());
    }

    #[test]
    fn test_bootstrapped_curve_single_jump_left_right_limit() {
        // Create curve with a single jump at t=0.5
        let pillars = vec![0.25_f64, 0.5, 1.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

        // Jump: 25 bps rate hike at t=0.5
        // In log-space, offset = -0.0025 (negative because rate hike decreases DF)
        let jump_offset = -0.0025_f64;
        let jumps = vec![(0.5_f64, jump_offset)];

        let curve = BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true)
            .unwrap()
            .with_jumps(jumps);

        assert!(curve.has_jumps());
        assert_eq!(curve.jumps().len(), 1);

        // At jump point t=0.5
        let df_left = curve.discount_factor_with_limit(0.5, Limit::Left).unwrap();
        let df_right = curve.discount_factor_with_limit(0.5, Limit::Right).unwrap();

        // Left limit: before jump (offset = 0)
        // Right limit: after jump (offset = -0.0025)
        // df_right = df_base * exp(-0.0025) < df_left
        assert!(df_right < df_left);

        // The difference should be exp(-0.0025)
        let ratio = df_right / df_left;
        assert!((ratio - jump_offset.exp()).abs() < 1e-10);
    }

    #[test]
    fn test_bootstrapped_curve_multiple_jumps_cumulative() {
        let pillars = vec![0.25_f64, 0.5, 0.75, 1.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

        // Two jumps: 25 bps at t=0.3, 25 bps at t=0.6
        // Cumulative offsets: first = -0.0025, second = -0.005
        let jumps = vec![(0.3_f64, -0.0025), (0.6_f64, -0.005)];

        let curve = BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true)
            .unwrap()
            .with_jumps(jumps);

        // Before any jump (t=0.2)
        let df_before = curve
            .discount_factor_with_limit(0.2, Limit::Continuous)
            .unwrap();
        let base_df_at_02 = (-0.03 * 0.2_f64).exp();
        assert!((df_before - base_df_at_02).abs() < 1e-6);

        // Between jumps (t=0.4) - should have first jump's offset applied
        let df_between_cont = curve
            .discount_factor_with_limit(0.4, Limit::Continuous)
            .unwrap();
        let df_between_left = curve.discount_factor_with_limit(0.4, Limit::Left).unwrap();
        // Both should have the first jump's offset (no jump at t=0.4)
        assert!((df_between_cont - df_between_left).abs() < 1e-12);

        // After all jumps (t=0.8) - should have cumulative offset of -0.005
        let df_after_left = curve.discount_factor_with_limit(0.8, Limit::Left).unwrap();
        let df_after_right = curve.discount_factor_with_limit(0.8, Limit::Right).unwrap();

        // Both should have the same offset at t=0.8 (no jump at that point)
        assert!((df_after_left - df_after_right).abs() < 1e-12);
    }

    #[test]
    fn test_bootstrapped_curve_forward_rate_with_limit() {
        let pillars = vec![0.25_f64, 0.5, 1.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

        // Jump at t=0.5
        let jumps = vec![(0.5_f64, -0.0025)];

        let curve = BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true)
            .unwrap()
            .with_jumps(jumps);

        // Forward rate spanning the jump
        let fwd_rate = curve
            .forward_rate_with_limit(0.25, 0.75, Limit::Continuous)
            .unwrap();

        // Forward rate should be computable
        assert!(fwd_rate.is_finite());

        // Verify consistency: fwd_rate = (df1/df2 - 1) / tau
        let df1 = curve
            .discount_factor_with_limit(0.25, Limit::Continuous)
            .unwrap();
        let df2 = curve
            .discount_factor_with_limit(0.75, Limit::Continuous)
            .unwrap();
        let expected_fwd = (df1 / df2 - 1.0) / 0.5;

        assert!((fwd_rate - expected_fwd).abs() < 1e-10);
    }

    #[test]
    fn test_bootstrapped_curve_decompose_forward_rate() {
        let pillars = vec![0.25_f64, 0.5, 0.75, 1.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

        // Jump at t=0.5
        let jump_offset = -0.0025_f64;
        let jumps = vec![(0.5_f64, jump_offset)];

        let curve = BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true)
            .unwrap()
            .with_jumps(jumps);

        // Decompose forward rate spanning the jump
        let decomp = curve.decompose_forward_rate(0.25, 0.75).unwrap();

        // total = continuous + jump
        assert!((decomp.total - (decomp.continuous + decomp.jump)).abs() < 1e-10);

        // Jump component should be non-zero when spanning a jump
        assert!(decomp.jump.abs() > 1e-10);
    }

    #[test]
    fn test_bootstrapped_curve_decompose_forward_rate_no_jump_in_range() {
        let pillars = vec![0.25_f64, 0.5, 0.75, 1.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

        // Jump at t=0.5
        let jumps = vec![(0.5_f64, -0.0025)];

        let curve = BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true)
            .unwrap()
            .with_jumps(jumps);

        // Forward rate in range without jump (0.6 to 0.8)
        let decomp = curve.decompose_forward_rate(0.6, 0.8).unwrap();

        // Jump component should be ~0 since we're past the jump and both ends
        // have the same cumulative offset
        assert!(decomp.jump.abs() < 1e-10);
        assert!((decomp.total - decomp.continuous).abs() < 1e-10);
    }

    #[test]
    fn test_bootstrapped_curve_with_jumps_builder() {
        let pillars = vec![0.25_f64, 0.5, 1.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

        let curve = BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true)
            .unwrap()
            .with_jumps(vec![(0.5, -0.0025)]);

        assert!(curve.has_jumps());
        assert_eq!(curve.jumps().len(), 1);
        assert_eq!(curve.jumps()[0], (0.5, -0.0025));
    }

    #[test]
    fn test_bootstrapped_curve_default_discount_factor_uses_continuous() {
        let pillars = vec![0.25_f64, 0.5, 1.0];
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-0.03 * t).exp()).collect();

        let jumps = vec![(0.5_f64, -0.0025)];

        let curve = BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true)
            .unwrap()
            .with_jumps(jumps);

        // Default discount_factor should use Continuous limit
        let df_default = curve.discount_factor(0.75).unwrap();
        let df_continuous = curve
            .discount_factor_with_limit(0.75, Limit::Continuous)
            .unwrap();

        assert!((df_default - df_continuous).abs() < 1e-12);
    }

    #[test]
    fn test_bootstrapped_curve_forward_rate_consistency_with_zero_rate() {
        // Test that forward_rate computed from DFs matches the expected formula
        let pillars = vec![0.25_f64, 0.5, 1.0, 2.0];
        let rate = 0.03_f64;
        let dfs: Vec<f64> = pillars.iter().map(|&t| (-rate * t).exp()).collect();

        let curve =
            BootstrappedCurve::new(pillars, dfs, BootstrapInterpolation::LogLinear, true).unwrap();

        // For a flat rate curve, forward rate should equal the zero rate
        let fwd = curve
            .forward_rate_with_limit(0.5, 1.0, Limit::Continuous)
            .unwrap();

        // Forward rate formula: (df1/df2 - 1) / tau
        // For continuous compounding: should be close to rate
        // Actual: (exp(-r*t1) / exp(-r*t2) - 1) / (t2-t1) = (exp(r*(t2-t1)) - 1) /
        // (t2-t1)
        let expected = ((rate * 0.5).exp() - 1.0) / 0.5;
        assert!((fwd - expected).abs() < 1e-10);
    }

    #[test]
    fn test_flat_fx_curve() {
        use infra_domain::{market::Currency, trade::instrument_def::CurrencyPair};

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
        use infra_domain::{market::Currency, trade::instrument_def::CurrencyPair};

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
        use infra_domain::{market::Currency, trade::instrument_def::CurrencyPair};

        let pair = CurrencyPair::new(Currency::USD, Currency::JPY);
        let fx_curve = FxCurveEnum::irp_flat(150.0, 0.05, 0.01, pair);

        // Forward at 1 year: F = 150 × exp(0.04) ≈ 156.12
        let fwd_1y = fx_curve.forward_rate(1.0).unwrap();
        let expected = 150.0 * (0.04_f64).exp();
        assert!((fwd_1y - expected).abs() < 1e-8);
    }

    #[test]
    fn test_fx_curve_currency_pair() {
        use infra_domain::{market::Currency, trade::instrument_def::CurrencyPair};

        let pair = CurrencyPair::new(Currency::GBP, Currency::USD);
        let fx_curve: FxCurveEnum<f64> = FxCurveEnum::flat(1.25, 0.01, pair);

        assert_eq!(fx_curve.currency_pair(), pair);
    }

    #[test]
    fn test_irp_fx_curve_with_negative_spread() {
        use infra_domain::{market::Currency, trade::instrument_def::CurrencyPair};

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
