//! Equity derivative instruments.
//!
//! This module provides equity-linked derivative instruments including:
//! - Vanilla options (European, American, Bermudan)
//! - Forward contracts
//!
//! # Feature Flag
//!
//! This module is available when the `equity` feature is enabled (default).
//!
//! # Hierarchical Enum
//!
//! All equity instruments are wrapped in [`EquityInstrument`] enum for
//! static dispatch (Enzyme AD compatible).
//!
//! # Examples
//!
//! ```
//! use pricer_models::instruments::equity::{EquityInstrument, VanillaOption, Forward};
//! use pricer_models::instruments::{InstrumentParams, PayoffType, ExerciseStyle, Direction};
//!
//! // Create a vanilla call option
//! let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
//! let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
//! let equity_inst = EquityInstrument::Vanilla(call);
//!
//! // Create a forward contract
//! let forward = Forward::new(100.0_f64, 1.0, 1.0, Direction::Long).unwrap();
//! let forward_inst = EquityInstrument::Forward(forward);
//! ```

use num_traits::Float;
use pricer_core::types::Currency;

// Re-export equity instruments from parent module for organized access
pub use super::forward::{Direction, Forward};
pub use super::vanilla::VanillaOption;
use super::InstrumentTrait;

/// Equity derivative instruments.
///
/// Enum wrapping all equity-linked instruments for static dispatch.
/// This is the sub-enum for equity products in the hierarchical
/// instrument structure.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Variants
///
/// - `Vanilla`: Vanilla options (Call, Put, Digital)
/// - `Forward`: Forward contracts
///
/// # Examples
///
/// ```
/// use pricer_models::instruments::equity::{EquityInstrument, VanillaOption, Forward};
/// use pricer_models::instruments::{InstrumentParams, PayoffType, ExerciseStyle, Direction};
///
/// // Via enum constructor
/// let params = InstrumentParams::new(100.0_f64, 1.0, 1.0).unwrap();
/// let call = VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6);
/// let equity = EquityInstrument::Vanilla(call);
///
/// // Compute payoff
/// let payoff = equity.payoff(110.0);
/// ```
#[derive(Debug, Clone)]
pub enum EquityInstrument<T: Float> {
    /// Vanilla option (Call, Put, Digital).
    Vanilla(VanillaOption<T>),
    /// Forward contract.
    Forward(Forward<T>),
}

impl<T: Float> EquityInstrument<T> {
    /// Compute the payoff at given spot price.
    ///
    /// Delegates to the underlying instrument's payoff method.
    #[inline]
    pub fn payoff(&self, spot: T) -> T {
        match self {
            EquityInstrument::Vanilla(option) => option.payoff(spot),
            EquityInstrument::Forward(forward) => forward.payoff(spot),
        }
    }

    /// Return time to expiry in years.
    #[inline]
    pub fn expiry(&self) -> T {
        match self {
            EquityInstrument::Vanilla(option) => option.expiry(),
            EquityInstrument::Forward(forward) => forward.expiry(),
        }
    }

    /// Return the underlying instrument's currency.
    #[inline]
    pub fn currency(&self) -> Currency {
        // Both vanilla options and forwards default to USD
        // In a real implementation, this would be configurable
        Currency::USD
    }

    /// Return whether this is a vanilla option.
    #[inline]
    pub fn is_vanilla(&self) -> bool {
        matches!(self, EquityInstrument::Vanilla(_))
    }

    /// Return whether this is a forward contract.
    #[inline]
    pub fn is_forward(&self) -> bool {
        matches!(self, EquityInstrument::Forward(_))
    }

    /// Return a reference to the vanilla option if this is a Vanilla variant.
    pub fn as_vanilla(&self) -> Option<&VanillaOption<T>> {
        match self {
            EquityInstrument::Vanilla(option) => Some(option),
            _ => None,
        }
    }

    /// Return a reference to the forward if this is a Forward variant.
    pub fn as_forward(&self) -> Option<&Forward<T>> {
        match self {
            EquityInstrument::Forward(forward) => Some(forward),
            _ => None,
        }
    }
}

impl<T: Float> InstrumentTrait<T> for EquityInstrument<T> {
    #[inline]
    fn payoff(&self, spot: T) -> T {
        self.payoff(spot)
    }

    #[inline]
    fn expiry(&self) -> T {
        self.expiry()
    }

    #[inline]
    fn currency(&self) -> Currency {
        self.currency()
    }

    fn type_name(&self) -> &'static str {
        match self {
            EquityInstrument::Vanilla(_) => "EquityVanilla",
            EquityInstrument::Forward(_) => "EquityForward",
        }
    }
}

// Conversion from individual instruments to EquityInstrument
impl<T: Float> From<VanillaOption<T>> for EquityInstrument<T> {
    fn from(option: VanillaOption<T>) -> Self {
        EquityInstrument::Vanilla(option)
    }
}

impl<T: Float> From<Forward<T>> for EquityInstrument<T> {
    fn from(forward: Forward<T>) -> Self {
        EquityInstrument::Forward(forward)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::{ExerciseStyle, InstrumentParams, PayoffType};

    fn create_test_call() -> VanillaOption<f64> {
        let params = InstrumentParams::new(100.0, 1.0, 1.0).unwrap();
        VanillaOption::new(params, PayoffType::Call, ExerciseStyle::European, 1e-6)
    }

    fn create_test_forward() -> Forward<f64> {
        Forward::new(100.0, 1.0, 1.0, Direction::Long).unwrap()
    }

    #[test]
    fn test_equity_instrument_vanilla_payoff() {
        let call = create_test_call();
        let equity = EquityInstrument::Vanilla(call);

        let payoff = equity.payoff(110.0);
        assert!((payoff - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_equity_instrument_forward_payoff() {
        let forward = create_test_forward();
        let equity = EquityInstrument::Forward(forward);

        let payoff = equity.payoff(110.0);
        assert!((payoff - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_equity_instrument_expiry() {
        let call = create_test_call();
        let equity = EquityInstrument::Vanilla(call);
        assert!((equity.expiry() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_equity_instrument_currency() {
        let call = create_test_call();
        let equity = EquityInstrument::Vanilla(call);
        assert_eq!(equity.currency(), Currency::USD);
    }

    #[test]
    fn test_is_vanilla() {
        let call = create_test_call();
        let equity = EquityInstrument::Vanilla(call);

        assert!(equity.is_vanilla());
        assert!(!equity.is_forward());
    }

    #[test]
    fn test_is_forward() {
        let forward = create_test_forward();
        let equity = EquityInstrument::Forward(forward);

        assert!(!equity.is_vanilla());
        assert!(equity.is_forward());
    }

    #[test]
    fn test_as_vanilla() {
        let call = create_test_call();
        let equity = EquityInstrument::Vanilla(call);

        assert!(equity.as_vanilla().is_some());
        assert!(equity.as_forward().is_none());
    }

    #[test]
    fn test_as_forward() {
        let forward = create_test_forward();
        let equity = EquityInstrument::Forward(forward);

        assert!(equity.as_vanilla().is_none());
        assert!(equity.as_forward().is_some());
    }

    #[test]
    fn test_from_vanilla() {
        let call = create_test_call();
        let equity: EquityInstrument<f64> = call.into();
        assert!(equity.is_vanilla());
    }

    #[test]
    fn test_from_forward() {
        let forward = create_test_forward();
        let equity: EquityInstrument<f64> = forward.into();
        assert!(equity.is_forward());
    }

    #[test]
    fn test_instrument_trait_payoff() {
        let call = create_test_call();
        let equity = EquityInstrument::Vanilla(call);

        // Use trait method
        let payoff = <EquityInstrument<f64> as InstrumentTrait<f64>>::payoff(&equity, 110.0);
        assert!((payoff - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_instrument_trait_type_name() {
        let call = create_test_call();
        let forward = create_test_forward();

        let eq_vanilla = EquityInstrument::Vanilla(call);
        let eq_forward = EquityInstrument::Forward(forward);

        assert_eq!(eq_vanilla.type_name(), "EquityVanilla");
        assert_eq!(eq_forward.type_name(), "EquityForward");
    }

    #[test]
    fn test_clone() {
        let call = create_test_call();
        let eq1 = EquityInstrument::Vanilla(call);
        let eq2 = eq1.clone();

        assert!((eq1.expiry() - eq2.expiry()).abs() < 1e-10);
    }

    #[test]
    fn test_debug() {
        let call = create_test_call();
        let equity = EquityInstrument::Vanilla(call);
        let debug_str = format!("{:?}", equity);
        assert!(debug_str.contains("Vanilla"));
    }
}
