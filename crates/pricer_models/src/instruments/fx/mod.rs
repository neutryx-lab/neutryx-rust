//! Foreign exchange derivative instruments.
//!
//! This module provides FX derivative instruments including:
//! - FX Options (vanilla call/put)
//! - FX Forwards
//!
//! # Feature Flag
//!
//! This module is available when the `fx` feature is enabled.
//!
//! # Examples
//!
//! ```
//! use pricer_models::instruments::fx::{FxOption, FxForward, FxOptionType, FxForwardDirection};
//! use pricer_core::types::{Currency, CurrencyPair};
//!
//! // Create an FX option
//! let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
//! let option = FxOption::new(
//!     pair,
//!     1.12,              // strike
//!     1.0,               // expiry (1 year)
//!     1_000_000.0,       // notional (1M EUR)
//!     FxOptionType::Call,
//!     1e-6,              // smoothing epsilon
//! ).unwrap();
//!
//! // Create an FX forward
//! let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
//! let forward = FxForward::new(
//!     pair,
//!     1.12,              // forward rate
//!     1.0,               // maturity (1 year)
//!     1_000_000.0,       // notional (1M EUR)
//!     FxForwardDirection::Buy,
//! ).unwrap();
//! ```

mod forward;
mod option;

pub use forward::{FxForward, FxForwardDirection, FxForwardError};
pub use option::{FxOption, FxOptionError, FxOptionType};

use num_traits::Float;
use pricer_core::types::Currency;

use crate::instruments::traits::InstrumentTrait;

/// FX instrument sub-enum for static dispatch.
///
/// Wraps all FX derivative instruments for enum-based dispatch
/// compatible with Enzyme AD.
///
/// # Type Parameters
///
/// * `T` - Floating-point type implementing `Float` (e.g., `f64`, `Dual64`)
///
/// # Variants
///
/// - `Option`: FX vanilla options (call/put)
/// - `Forward`: FX forward contracts
///
/// # Examples
///
/// ```
/// use pricer_models::instruments::fx::{FxInstrument, FxOption, FxOptionType};
/// use pricer_core::types::{Currency, CurrencyPair};
///
/// let pair = CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap();
/// let option = FxOption::new(pair, 1.12, 1.0, 1_000_000.0, FxOptionType::Call, 1e-6).unwrap();
/// let instrument = FxInstrument::Option(option);
///
/// let payoff = instrument.payoff(1.15);
/// assert!(payoff > 0.0);
/// ```
#[derive(Debug, Clone)]
pub enum FxInstrument<T: Float> {
    /// FX vanilla option (call or put).
    Option(FxOption<T>),
    /// FX forward contract.
    Forward(FxForward<T>),
}

impl<T: Float> FxInstrument<T> {
    /// Compute the payoff at given spot rate.
    #[inline]
    pub fn payoff(&self, spot: T) -> T {
        match self {
            FxInstrument::Option(opt) => opt.payoff(spot),
            FxInstrument::Forward(fwd) => fwd.payoff(spot),
        }
    }

    /// Return time to expiry/maturity in years.
    #[inline]
    pub fn expiry(&self) -> T {
        match self {
            FxInstrument::Option(opt) => opt.expiry_time(),
            FxInstrument::Forward(fwd) => fwd.maturity(),
        }
    }

    /// Return the settlement currency (quote currency).
    #[inline]
    pub fn currency(&self) -> Currency {
        match self {
            FxInstrument::Option(opt) => opt.quote_currency(),
            FxInstrument::Forward(fwd) => fwd.quote_currency(),
        }
    }

    /// Return the notional amount.
    #[inline]
    pub fn notional(&self) -> T {
        match self {
            FxInstrument::Option(opt) => opt.notional_amount(),
            FxInstrument::Forward(fwd) => fwd.notional_amount(),
        }
    }

    /// Return whether this is an option.
    #[inline]
    pub fn is_option(&self) -> bool {
        matches!(self, FxInstrument::Option(_))
    }

    /// Return whether this is a forward.
    #[inline]
    pub fn is_forward(&self) -> bool {
        matches!(self, FxInstrument::Forward(_))
    }

    /// Return a reference to the option if this is an Option variant.
    pub fn as_option(&self) -> Option<&FxOption<T>> {
        match self {
            FxInstrument::Option(opt) => Some(opt),
            _ => None,
        }
    }

    /// Return a reference to the forward if this is a Forward variant.
    pub fn as_forward(&self) -> Option<&FxForward<T>> {
        match self {
            FxInstrument::Forward(fwd) => Some(fwd),
            _ => None,
        }
    }
}

impl<T: Float> InstrumentTrait<T> for FxInstrument<T> {
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

    #[inline]
    fn notional(&self) -> T {
        self.notional()
    }

    fn type_name(&self) -> &'static str {
        match self {
            FxInstrument::Option(opt) => opt.type_name(),
            FxInstrument::Forward(fwd) => fwd.type_name(),
        }
    }
}

// Conversion from individual types to FxInstrument
impl<T: Float> From<FxOption<T>> for FxInstrument<T> {
    fn from(opt: FxOption<T>) -> Self {
        FxInstrument::Option(opt)
    }
}

impl<T: Float> From<FxForward<T>> for FxInstrument<T> {
    fn from(fwd: FxForward<T>) -> Self {
        FxInstrument::Forward(fwd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pair() -> pricer_core::types::CurrencyPair<f64> {
        pricer_core::types::CurrencyPair::new(Currency::EUR, Currency::USD, 1.10).unwrap()
    }

    #[test]
    fn test_fx_instrument_option() {
        let pair = create_test_pair();
        let option = FxOption::new(
            pair,
            1.10,
            1.0,
            1_000_000.0,
            FxOptionType::Call,
            1e-6,
        ).unwrap();
        let instrument = FxInstrument::Option(option);

        assert!(instrument.is_option());
        assert!(!instrument.is_forward());
        assert!(instrument.as_option().is_some());
        assert!(instrument.as_forward().is_none());
    }

    #[test]
    fn test_fx_instrument_forward() {
        let pair = create_test_pair();
        let forward = FxForward::new(
            pair,
            1.12,
            1.0,
            1_000_000.0,
            FxForwardDirection::Buy,
        ).unwrap();
        let instrument = FxInstrument::Forward(forward);

        assert!(!instrument.is_option());
        assert!(instrument.is_forward());
        assert!(instrument.as_option().is_none());
        assert!(instrument.as_forward().is_some());
    }

    #[test]
    fn test_fx_instrument_payoff() {
        let pair = create_test_pair();
        let option = FxOption::new(
            pair,
            1.10,
            1.0,
            1_000_000.0,
            FxOptionType::Call,
            1e-6,
        ).unwrap();
        let instrument = FxInstrument::Option(option);

        // At spot 1.15, payoff = 1M * (1.15 - 1.10) = 50,000 USD
        let payoff = instrument.payoff(1.15);
        assert!((payoff - 50_000.0).abs() < 100.0);
    }

    #[test]
    fn test_fx_instrument_expiry() {
        let pair = create_test_pair();
        let option = FxOption::new(
            pair,
            1.10,
            0.5,
            1_000_000.0,
            FxOptionType::Call,
            1e-6,
        ).unwrap();
        let instrument = FxInstrument::Option(option);

        assert!((instrument.expiry() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_fx_instrument_currency() {
        let pair = create_test_pair();
        let option = FxOption::new(
            pair,
            1.10,
            1.0,
            1_000_000.0,
            FxOptionType::Call,
            1e-6,
        ).unwrap();
        let instrument = FxInstrument::Option(option);

        assert_eq!(instrument.currency(), Currency::USD);
    }

    #[test]
    fn test_fx_instrument_notional() {
        let pair = create_test_pair();
        let forward = FxForward::new(
            pair,
            1.12,
            1.0,
            500_000.0,
            FxForwardDirection::Sell,
        ).unwrap();
        let instrument = FxInstrument::Forward(forward);

        assert!((instrument.notional() - 500_000.0).abs() < 1e-10);
    }

    #[test]
    fn test_fx_instrument_type_name() {
        let pair = create_test_pair();

        let call = FxOption::new(pair, 1.10, 1.0, 1_000_000.0, FxOptionType::Call, 1e-6).unwrap();
        let call_inst = FxInstrument::Option(call);
        assert_eq!(call_inst.type_name(), "FxCall");

        let pair = create_test_pair();
        let put = FxOption::new(pair, 1.10, 1.0, 1_000_000.0, FxOptionType::Put, 1e-6).unwrap();
        let put_inst = FxInstrument::Option(put);
        assert_eq!(put_inst.type_name(), "FxPut");

        let pair = create_test_pair();
        let fwd = FxForward::new(pair, 1.12, 1.0, 1_000_000.0, FxForwardDirection::Buy).unwrap();
        let fwd_inst = FxInstrument::Forward(fwd);
        assert_eq!(fwd_inst.type_name(), "FxForwardBuy");
    }

    #[test]
    fn test_fx_instrument_from_option() {
        let pair = create_test_pair();
        let option = FxOption::new(
            pair,
            1.10,
            1.0,
            1_000_000.0,
            FxOptionType::Call,
            1e-6,
        ).unwrap();
        let instrument: FxInstrument<f64> = option.into();

        assert!(instrument.is_option());
    }

    #[test]
    fn test_fx_instrument_from_forward() {
        let pair = create_test_pair();
        let forward = FxForward::new(
            pair,
            1.12,
            1.0,
            1_000_000.0,
            FxForwardDirection::Buy,
        ).unwrap();
        let instrument: FxInstrument<f64> = forward.into();

        assert!(instrument.is_forward());
    }

    #[test]
    fn test_fx_instrument_clone() {
        let pair = create_test_pair();
        let option = FxOption::new(
            pair,
            1.10,
            1.0,
            1_000_000.0,
            FxOptionType::Call,
            1e-6,
        ).unwrap();
        let inst1 = FxInstrument::Option(option);
        let inst2 = inst1.clone();

        assert!((inst1.expiry() - inst2.expiry()).abs() < 1e-10);
    }

    #[test]
    fn test_fx_instrument_debug() {
        let pair = create_test_pair();
        let option = FxOption::new(
            pair,
            1.10,
            1.0,
            1_000_000.0,
            FxOptionType::Call,
            1e-6,
        ).unwrap();
        let instrument = FxInstrument::Option(option);

        let debug_str = format!("{:?}", instrument);
        assert!(debug_str.contains("Option"));
    }

    #[test]
    fn test_fx_instrument_trait() {
        let pair = create_test_pair();
        let option = FxOption::new(
            pair,
            1.10,
            0.5,
            1_000_000.0,
            FxOptionType::Call,
            1e-6,
        ).unwrap();
        let instrument = FxInstrument::Option(option);

        // Test InstrumentTrait methods via trait
        let payoff = <FxInstrument<f64> as InstrumentTrait<f64>>::payoff(&instrument, 1.15);
        assert!(payoff > 0.0);
    }
}
