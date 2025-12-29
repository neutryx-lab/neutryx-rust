//! Traits for priceable and differentiable instruments.
//!
//! This module defines fundamental abstractions for:
//! - Price calculation (`Priceable` trait)
//! - Gradient computation (`Differentiable` trait)
//!
//! All traits are designed for static dispatch (enum-based) to ensure
//! compatibility with Enzyme AD optimization at LLVM level.

use num_traits::Float;

/// Trait for entities that can be priced.
///
/// # Type Parameters
/// * `T` - Floating-point type (f32 or f64)
///
/// # Design Philosophy
///
/// This trait is designed for **static dispatch only**. Do NOT use
/// `Box<dyn Priceable>` as it is incompatible with Enzyme's LLVM-level
/// optimization. Instead, use `enum`-based dispatch:
///
/// ```
/// use pricer_core::traits::priceable::Priceable;
/// use num_traits::Float;
///
/// enum Instrument<T: Float> {
///     VanillaOption { strike: T, maturity: T },
///     BarrierOption { strike: T, barrier: T, maturity: T },
/// }
///
/// impl<T: Float> Priceable<T> for Instrument<T> {
///     fn price(&self) -> T {
///         match self {
///             Instrument::VanillaOption { strike, maturity } => {
///                 // Black-Scholes or Monte Carlo pricing
///                 *strike  // Placeholder
///             }
///             Instrument::BarrierOption { strike, barrier, maturity } => {
///                 // Barrier option pricing
///                 *strike  // Placeholder
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
    /// Calculate the price of the instrument.
    ///
    /// # Returns
    /// The calculated price (present value) as a floating-point number.
    ///
    /// # Invariants
    /// - The returned price must be non-negative (no arbitrage)
    /// - The method must be pure (no side effects, deterministic)
    fn price(&self) -> T;
}

/// Trait for entities that can compute gradients (first derivatives).
///
/// # Type Parameters
/// * `T` - Floating-point type (f32 or f64)
///
/// # Design Philosophy
///
/// This trait is designed for **static dispatch only**. It will be implemented
/// by Layer 3's AD engine using either:
/// - Enzyme (LLVM-level automatic differentiation)
/// - num-dual (Dual number automatic differentiation for verification)
///
/// # Usage in Layer 3
///
/// ```ignore
/// use pricer_core::traits::priceable::Differentiable;
/// use pricer_core::types::dual::DualNumber;
///
/// struct OptionGreeks<T: Float> {
///     spot: T,
///     strike: T,
/// }
///
/// impl Differentiable<DualNumber> for OptionGreeks<DualNumber> {
///     fn gradient(&self) -> DualNumber {
///         // Compute delta (∂price/∂spot) using dual numbers
///         self.spot  // Simplified example
///     }
/// }
/// ```
///
/// # Important
/// Do NOT use `Box<dyn Differentiable>` - use static dispatch only.
pub trait Differentiable<T: Float> {
    /// Compute the gradient (first-order derivative).
    ///
    /// # Returns
    /// The gradient vector as a floating-point number (or vector in multi-dimensional case).
    ///
    /// # Invariants
    /// - The returned gradient must be finite (no NaN or Infinity)
    /// - The method must be pure (no side effects, deterministic)
    fn gradient(&self) -> T;
}

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
            fn price(&self) -> f64 {
                match self {
                    SimpleInstrument::FixedValue(val) => *val,
                }
            }
        }

        let instrument = SimpleInstrument::FixedValue(100.0);
        assert_eq!(instrument.price(), 100.0);
    }

    #[test]
    fn test_priceable_with_f32() {
        // Verify generic type support
        enum SimpleInstrument {
            FixedValue(f32),
        }

        impl Priceable<f32> for SimpleInstrument {
            fn price(&self) -> f32 {
                match self {
                    SimpleInstrument::FixedValue(val) => *val,
                }
            }
        }

        let instrument = SimpleInstrument::FixedValue(100.0_f32);
        assert_eq!(instrument.price(), 100.0_f32);
    }

    #[test]
    fn test_differentiable_with_f64() {
        // Simple differentiable entity
        struct LinearFunction {
            slope: f64,
        }

        impl Differentiable<f64> for LinearFunction {
            fn gradient(&self) -> f64 {
                self.slope
            }
        }

        let func = LinearFunction { slope: 2.5 };
        assert_eq!(func.gradient(), 2.5);
    }

    #[test]
    fn test_trait_method_has_no_side_effects() {
        // Verify that trait methods are pure (calling multiple times gives same result)
        enum SimpleInstrument {
            FixedValue(f64),
        }

        impl Priceable<f64> for SimpleInstrument {
            fn price(&self) -> f64 {
                match self {
                    SimpleInstrument::FixedValue(val) => *val,
                }
            }
        }

        let instrument = SimpleInstrument::FixedValue(100.0);
        let price1 = instrument.price();
        let price2 = instrument.price();
        assert_eq!(price1, price2); // Pure function - same result
    }
}
