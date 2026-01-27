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
}

// Re-export commonly used types at module level
pub use curves::{
    BootstrapInterpolation, BootstrappedCurve, FlatCurve, Frequency, MarketInstrument, YieldCurve,
};

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
}
