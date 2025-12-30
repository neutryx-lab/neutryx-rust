//! Financial instrument definitions.
//!
//! This module provides instrument definitions for pricing with
//! enum dispatch architecture for Enzyme AD compatibility.
//!
//! # Architecture
//!
//! Uses enum dispatch (NOT trait objects) for static dispatch optimisation:
//! - `Instrument<T>` enum wraps all instrument types
//! - All types generic over `T: Float` for AD compatibility
//! - Smooth payoff approximations via `pricer_core::math::smoothing`
//!
//! # Instrument Types
//!
//! - [`VanillaOption`]: European/American/Asian options with Call/Put payoffs
//! - [`Forward`]: Forward contracts with linear payoffs
//! - [`Swap`]: Interest rate swaps with payment schedules
//!
//! # Examples
//!
//! ```
//! use pricer_models::instruments::{
//!     Instrument, VanillaOption, Forward, Swap,
//!     InstrumentParams, PayoffType, ExerciseStyle, Direction, PaymentFrequency,
//! };
//! use pricer_core::types::Currency;
//!
//! // Create a vanilla call option
//! let params = InstrumentParams::new(100.0_f64, 1.0, 1_000_000.0).unwrap();
//! let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
//! let instrument = Instrument::Vanilla(call);
//!
//! // Compute payoff
//! let payoff = instrument.payoff(110.0);
//! assert!(payoff > 0.0);
//! ```

mod error;
mod exercise;
mod forward;
mod params;
mod payoff;
mod swap;
mod vanilla;

// Re-export all public types
pub use error::InstrumentError;
pub use exercise::ExerciseStyle;
pub use forward::{Direction, Forward};
pub use params::InstrumentParams;
pub use payoff::PayoffType;
pub use swap::{PaymentFrequency, Swap};
pub use vanilla::VanillaOption;

use num_traits::Float;

/// Unified instrument enum for static dispatch.
///
/// Wraps all instrument types for Enzyme-compatible static dispatch
/// without trait objects or dynamic allocation.
///
/// # Type Parameters
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Variants
/// - `Vanilla`: Vanilla options (European, American, Bermudan, Asian)
/// - `Forward`: Forward contracts
/// - `Swap`: Interest rate swaps
///
/// # Examples
/// ```
/// use pricer_models::instruments::{
///     Instrument, VanillaOption, Forward, InstrumentParams, PayoffType,
///     ExerciseStyle, Direction,
/// };
///
/// // Create instruments
/// let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
/// let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
/// let forward = Forward::new(100.0, 1.0, 1.0, Direction::Long).unwrap();
///
/// let vanilla_inst = Instrument::Vanilla(call);
/// let forward_inst = Instrument::Forward(forward);
///
/// // Compute payoffs via enum dispatch
/// let payoff1 = vanilla_inst.payoff(110.0);
/// let payoff2 = forward_inst.payoff(110.0);
/// ```
#[derive(Debug, Clone)]
pub enum Instrument<T: Float> {
    /// Vanilla option (Call, Put, Digital)
    Vanilla(VanillaOption<T>),
    /// Forward contract
    Forward(Forward<T>),
    /// Interest rate swap
    Swap(Swap<T>),
}

impl<T: Float> Instrument<T> {
    /// Compute the payoff for the instrument at given spot price.
    ///
    /// Static dispatch to the appropriate payoff method based on variant.
    ///
    /// # Arguments
    /// * `spot` - Current spot price
    ///
    /// # Returns
    /// Payoff value for the instrument.
    ///
    /// # Examples
    /// ```
    /// use pricer_models::instruments::{
    ///     Instrument, VanillaOption, InstrumentParams, PayoffType, ExerciseStyle,
    /// };
    ///
    /// let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
    /// let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
    /// let instrument = Instrument::Vanilla(call);
    ///
    /// let payoff = instrument.payoff(110.0);
    /// assert!((payoff - 10.0).abs() < 0.01);
    /// ```
    #[inline]
    pub fn payoff(&self, spot: T) -> T {
        match self {
            Instrument::Vanilla(option) => option.payoff(spot),
            Instrument::Forward(forward) => forward.payoff(spot),
            Instrument::Swap(_swap) => {
                // Swaps don't have a simple payoff function
                // Return zero as placeholder (actual valuation requires curve)
                T::zero()
            }
        }
    }

    /// Returns the expiry time of the instrument.
    ///
    /// # Returns
    /// Time to expiry in years.
    ///
    /// # Examples
    /// ```
    /// use pricer_models::instruments::{
    ///     Instrument, Forward, Direction,
    /// };
    ///
    /// let forward = Forward::new(100.0_f64, 0.5, 1.0, Direction::Long).unwrap();
    /// let instrument = Instrument::Forward(forward);
    ///
    /// assert_eq!(instrument.expiry(), 0.5);
    /// ```
    #[inline]
    pub fn expiry(&self) -> T {
        match self {
            Instrument::Vanilla(option) => option.expiry(),
            Instrument::Forward(forward) => forward.expiry(),
            Instrument::Swap(swap) => swap.maturity(),
        }
    }

    /// Returns whether this is a vanilla option.
    #[inline]
    pub fn is_vanilla(&self) -> bool {
        matches!(self, Instrument::Vanilla(_))
    }

    /// Returns whether this is a forward contract.
    #[inline]
    pub fn is_forward(&self) -> bool {
        matches!(self, Instrument::Forward(_))
    }

    /// Returns whether this is a swap.
    #[inline]
    pub fn is_swap(&self) -> bool {
        matches!(self, Instrument::Swap(_))
    }

    /// Returns a reference to the vanilla option if this is a Vanilla variant.
    pub fn as_vanilla(&self) -> Option<&VanillaOption<T>> {
        match self {
            Instrument::Vanilla(option) => Some(option),
            _ => None,
        }
    }

    /// Returns a reference to the forward if this is a Forward variant.
    pub fn as_forward(&self) -> Option<&Forward<T>> {
        match self {
            Instrument::Forward(forward) => Some(forward),
            _ => None,
        }
    }

    /// Returns a reference to the swap if this is a Swap variant.
    pub fn as_swap(&self) -> Option<&Swap<T>> {
        match self {
            Instrument::Swap(swap) => Some(swap),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use pricer_core::types::Currency;

    fn create_test_call() -> VanillaOption<f64> {
        let params = InstrumentParams::new(100.0, 1.0, 1.0).unwrap();
        VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6)
    }

    fn create_test_forward() -> Forward<f64> {
        Forward::new(100.0, 1.0, 1.0, Direction::Long).unwrap()
    }

    fn create_test_swap() -> Swap<f64> {
        let dates = vec![0.5, 1.0, 1.5, 2.0];
        Swap::new(1_000_000.0, 0.03, dates, PaymentFrequency::SemiAnnual, Currency::USD).unwrap()
    }

    #[test]
    fn test_instrument_vanilla_payoff() {
        let call = create_test_call();
        let instrument = Instrument::Vanilla(call);

        let payoff = instrument.payoff(110.0);
        assert_relative_eq!(payoff, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_instrument_forward_payoff() {
        let forward = create_test_forward();
        let instrument = Instrument::Forward(forward);

        let payoff = instrument.payoff(110.0);
        assert_relative_eq!(payoff, 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_instrument_swap_payoff() {
        let swap = create_test_swap();
        let instrument = Instrument::Swap(swap);

        // Swap payoff returns zero (placeholder)
        let payoff = instrument.payoff(110.0);
        assert_eq!(payoff, 0.0);
    }

    #[test]
    fn test_instrument_expiry() {
        let call = create_test_call();
        let forward = create_test_forward();
        let swap = create_test_swap();

        let vanilla_inst = Instrument::Vanilla(call);
        let forward_inst = Instrument::Forward(forward);
        let swap_inst = Instrument::Swap(swap);

        assert_eq!(vanilla_inst.expiry(), 1.0);
        assert_eq!(forward_inst.expiry(), 1.0);
        assert_eq!(swap_inst.expiry(), 2.0); // maturity
    }

    #[test]
    fn test_is_vanilla() {
        let call = create_test_call();
        let instrument = Instrument::Vanilla(call);

        assert!(instrument.is_vanilla());
        assert!(!instrument.is_forward());
        assert!(!instrument.is_swap());
    }

    #[test]
    fn test_is_forward() {
        let forward = create_test_forward();
        let instrument = Instrument::Forward(forward);

        assert!(!instrument.is_vanilla());
        assert!(instrument.is_forward());
        assert!(!instrument.is_swap());
    }

    #[test]
    fn test_is_swap() {
        let swap = create_test_swap();
        let instrument = Instrument::Swap(swap);

        assert!(!instrument.is_vanilla());
        assert!(!instrument.is_forward());
        assert!(instrument.is_swap());
    }

    #[test]
    fn test_as_vanilla() {
        let call = create_test_call();
        let instrument = Instrument::Vanilla(call);

        assert!(instrument.as_vanilla().is_some());
        assert!(instrument.as_forward().is_none());
        assert!(instrument.as_swap().is_none());
    }

    #[test]
    fn test_as_forward() {
        let forward = create_test_forward();
        let instrument = Instrument::Forward(forward);

        assert!(instrument.as_vanilla().is_none());
        assert!(instrument.as_forward().is_some());
        assert!(instrument.as_swap().is_none());
    }

    #[test]
    fn test_as_swap() {
        let swap = create_test_swap();
        let instrument = Instrument::Swap(swap);

        assert!(instrument.as_vanilla().is_none());
        assert!(instrument.as_forward().is_none());
        assert!(instrument.as_swap().is_some());
    }

    #[test]
    fn test_clone() {
        let call = create_test_call();
        let inst1 = Instrument::Vanilla(call);
        let inst2 = inst1.clone();

        assert_eq!(inst1.expiry(), inst2.expiry());
    }

    #[test]
    fn test_debug() {
        let call = create_test_call();
        let instrument = Instrument::Vanilla(call);
        let debug_str = format!("{:?}", instrument);
        assert!(debug_str.contains("Vanilla"));
    }

    // AD compatibility test with Dual64
    #[test]
    fn test_dual64_compatibility() {
        use num_dual::Dual64;

        let params = InstrumentParams::new(
            Dual64::new(100.0, 0.0),
            Dual64::new(1.0, 0.0),
            Dual64::new(1.0, 0.0),
        )
        .unwrap();

        let call = VanillaOption::new(
            params,
            PayoffType::Call,
            ExerciseStyle::European,
            Dual64::new(1e-6, 0.0),
        );

        let instrument = Instrument::Vanilla(call);

        // Compute payoff with derivative tracking
        let spot = Dual64::new(110.0, 1.0);
        let payoff = instrument.payoff(spot);

        // Payoff should be approximately 10
        assert!((payoff.re - 10.0).abs() < 0.01);
        // Delta should be approximately 1 for deep ITM
        assert!(payoff.eps > 0.9);
    }

    #[test]
    fn test_dual64_forward() {
        use num_dual::Dual64;

        let forward = Forward::new(
            Dual64::new(100.0, 0.0),
            Dual64::new(1.0, 0.0),
            Dual64::new(1.0, 0.0),
            Direction::Long,
        )
        .unwrap();

        let instrument = Instrument::Forward(forward);

        // Compute payoff with derivative tracking
        let spot = Dual64::new(110.0, 1.0);
        let payoff = instrument.payoff(spot);

        // Payoff = spot - strike = 10
        assert!((payoff.re - 10.0).abs() < 1e-10);
        // Delta = 1 for forward
        assert!((payoff.eps - 1.0).abs() < 1e-10);
    }
}
