//! Traits for priceable and differentiable instruments.
//!
//! This module defines fundamental abstractions for:
//! - Price calculation (`Priceable` trait)
//! - Gradient computation (`Differentiable` trait)
//!
//! All traits are designed for static dispatch (enum-based) to ensure
//! compatibility with Enzyme AD optimisation at LLVM level.

use crate::{traits::Float, types::error::PricingError};

/// Trait for financial instruments that can be priced.
///
/// # Type Parameters
/// * `T` - Generic floating-point type (f64 or DualNumber)
///
/// # Design Philosophy
///
/// This trait is designed for **static dispatch only**. Do NOT use
/// `Box<dyn Priceable>` as it is incompatible with Enzyme's LLVM-level
/// optimisation. Instead, use `enum`-based dispatch:
///
/// ```
/// use pricer_core::traits::Float;
/// use pricer_core::traits::priceable::Priceable;
/// use pricer_core::types::error::PricingError;
///
/// enum Instrument<T: Float> {
///     VanillaOption { strike: T, maturity: T },
///     BarrierOption { strike: T, barrier: T, maturity: T },
/// }
///
/// impl<T: Float> Priceable<T> for Instrument<T> {
///     fn price(&self) -> Result<T, PricingError> {
///         match self {
///             Instrument::VanillaOption { strike, maturity } => {
///                 if *strike < T::zero() {
///                     return Err(PricingError::InvalidInput(
///                         "Strike must be non-negative".to_string()
///                     ));
///                 }
///                 // Black-Scholes or Monte Carlo pricing
///                 Ok(*strike)  // Placeholder
///             }
///             Instrument::BarrierOption { strike, barrier, maturity } => {
///                 if *barrier < T::zero() {
///                     return Err(PricingError::InvalidInput(
///                         "Barrier must be non-negative".to_string()
///                     ));
///                 }
///                 // Barrier option pricing
///                 Ok(*strike)  // Placeholder
///             }
///         }
///     }
/// }
/// ```
///
/// # Usage in Layer 2
///
/// Implement this trait for financial instrument enums in `pricer_models`.
/// The trait ensures uniform pricing interface across different asset classes.
pub trait Priceable<T: Float> {
    /// Calculate the fair value of the instrument.
    ///
    /// # Arguments
    /// Market data and model parameters are typically provided through the
    /// implementing type's fields (e.g., spot, strike, volatility, etc.).
    ///
    /// # Returns
    /// `Result<T, PricingError>` containing the price or an error.
    ///
    /// # Errors
    /// - `PricingError::InvalidInput`: Invalid market data or parameters
    /// - `PricingError::NumericalInstability`: Computation failed to converge
    /// - `PricingError::ModelFailure`: Model assumptions violated
    /// - `PricingError::UnsupportedInstrument`: Instrument type not supported
    ///
    /// # Invariants
    /// - The returned price should typically be non-negative (no arbitrage)
    /// - The method must be pure (no side effects, deterministic)
    ///
    /// # Examples
    /// ```
    /// use pricer_core::traits::Float;
    /// use pricer_core::traits::priceable::Priceable;
    /// use pricer_core::types::error::PricingError;
    ///
    /// struct SimpleOption<T: Float> {
    ///     spot: T,
    ///     strike: T,
    /// }
    ///
    /// impl<T: Float> Priceable<T> for SimpleOption<T> {
    ///     fn price(&self) -> Result<T, PricingError> {
    ///         if self.spot < T::zero() {
    ///             return Err(PricingError::InvalidInput(
    ///                 "Spot price cannot be negative".to_string()
    ///             ));
    ///         }
    ///         // Simplified intrinsic value
    ///         Ok((self.spot - self.strike).max(T::zero()))
    ///     }
    /// }
    ///
    /// let option = SimpleOption { spot: 105.0, strike: 100.0 };
    /// assert_eq!(option.price().unwrap(), 5.0);
    /// ```
    fn price(&self) -> Result<T, PricingError>;
}

/// Marker trait for types that guarantee smooth, differentiable computations.
///
/// This trait indicates that all operations on `Float` values use smooth
/// approximations instead of discontinuous functions, ensuring compatibility
/// with automatic differentiation (AD) backends like Enzyme and num-dual.
///
/// # Requirements
/// Types implementing this trait must ensure:
/// - All operations on `Float` values use smooth approximations
/// - No conditional branches (`if`) on Float values that create discontinuities
/// - Use `smooth_max`, `smooth_indicator` from `pricer_core::math::smoothing`
///
/// # Smoothing Epsilon
/// Types implementing this trait should provide a `smoothing_epsilon` parameter
/// for configurable approximation precision (typically 1e-6 to 1e-8).
///
/// # Examples
/// ```
/// use pricer_core::traits::priceable::Differentiable;
///
/// struct SmoothCallPayoff {
///     smoothing_epsilon: f64,
/// }
///
/// impl Differentiable for SmoothCallPayoff {}
///
/// // In the implementation, you would use:
/// // smooth_max(spot - strike, 0.0, self.smoothing_epsilon)
/// // instead of:
/// // (spot - strike).max(0.0)
/// ```
///
/// # Important
/// This is a marker trait with no methods. It serves as a compile-time
/// guarantee that the implementing type follows differentiability constraints.
pub trait Differentiable {}

#[cfg(test)]
mod tests {
    use super::*;

    // Task 4.3: Trait doctest verification

    #[test]
    fn test_priceable_with_f64() {
        // Verify static dispatch with enum pattern
        enum SimpleInstrument {
            FixedValue(f64),
        }

        impl Priceable<f64> for SimpleInstrument {
            fn price(&self) -> Result<f64, PricingError> {
                match self {
                    SimpleInstrument::FixedValue(val) => {
                        if *val < 0.0 {
                            return Err(PricingError::InvalidInput(
                                "Price cannot be negative".to_string(),
                            ));
                        }
                        Ok(*val)
                    }
                }
            }
        }

        let instrument = SimpleInstrument::FixedValue(100.0);
        assert_eq!(instrument.price().unwrap(), 100.0);

        let invalid = SimpleInstrument::FixedValue(-50.0);
        assert!(invalid.price().is_err());
    }

    #[test]
    fn test_priceable_with_f32() {
        // Verify generic type support
        enum SimpleInstrument {
            FixedValue(f32),
        }

        impl Priceable<f32> for SimpleInstrument {
            fn price(&self) -> Result<f32, PricingError> {
                match self {
                    SimpleInstrument::FixedValue(val) => Ok(*val),
                }
            }
        }

        let instrument = SimpleInstrument::FixedValue(100.0_f32);
        assert_eq!(instrument.price().unwrap(), 100.0_f32);
    }

    #[test]
    fn test_differentiable_marker_trait() {
        // Verify that Differentiable is a marker trait with no methods
        struct SmoothCallPayoff {
            #[allow(dead_code)]
            smoothing_epsilon: f64,
        }

        impl Differentiable for SmoothCallPayoff {}

        let payoff = SmoothCallPayoff {
            smoothing_epsilon: 1e-6,
        };

        // Marker trait - no methods to call, just compile-time guarantee
        let _: &dyn Differentiable = &payoff;
    }

    #[test]
    fn test_trait_method_has_no_side_effects() {
        // Verify that trait methods are pure (calling multiple times gives same result)
        enum SimpleInstrument {
            FixedValue(f64),
        }

        impl Priceable<f64> for SimpleInstrument {
            fn price(&self) -> Result<f64, PricingError> {
                match self {
                    SimpleInstrument::FixedValue(val) => Ok(*val),
                }
            }
        }

        let instrument = SimpleInstrument::FixedValue(100.0);
        let price1 = instrument.price().unwrap();
        let price2 = instrument.price().unwrap();
        assert_eq!(price1, price2); // Pure function - same result
    }

    #[test]
    fn test_priceable_error_handling() {
        // Verify error variant handling
        struct VolatileInstrument {
            volatility: f64,
        }

        impl Priceable<f64> for VolatileInstrument {
            fn price(&self) -> Result<f64, PricingError> {
                if self.volatility < 0.0 {
                    return Err(PricingError::InvalidInput(
                        "Volatility must be non-negative".to_string(),
                    ));
                }
                if self.volatility > 10.0 {
                    return Err(PricingError::NumericalInstability(
                        "Volatility too high for numerical stability".to_string(),
                    ));
                }
                Ok(100.0 * self.volatility)
            }
        }

        let valid = VolatileInstrument { volatility: 0.2 };
        assert_eq!(valid.price().unwrap(), 20.0);

        let invalid_negative = VolatileInstrument { volatility: -0.1 };
        match invalid_negative.price() {
            Err(PricingError::InvalidInput(_)) => (),
            _ => panic!("Expected InvalidInput error"),
        }

        let invalid_high = VolatileInstrument { volatility: 15.0 };
        match invalid_high.price() {
            Err(PricingError::NumericalInstability(_)) => (),
            _ => panic!("Expected NumericalInstability error"),
        }
    }
}
