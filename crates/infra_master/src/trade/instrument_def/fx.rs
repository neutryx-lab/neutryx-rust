//! Foreign exchange instrument definitions.
//!
//! This module provides definitions for FX derivatives including
//! spots, forwards, vanilla options, barrier options, and FX swaps.

use crate::{Currency, Date};

use super::common::{BarrierDirection, BarrierType, ExerciseStyle};
use super::error::InstrumentError;
use crate::trade::OptionType;

/// Currency pair representation.
///
/// Represents a pair of currencies for FX transactions.
/// Convention: Base/Quote, e.g., EUR/USD means EUR is base, USD is quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CurrencyPair {
    /// Base currency (first in the pair).
    pub base: Currency,
    /// Quote currency (second in the pair).
    pub quote: Currency,
}

impl CurrencyPair {
    /// Creates a new currency pair.
    #[must_use]
    pub fn new(base: Currency, quote: Currency) -> Self {
        Self { base, quote }
    }

    /// Returns the inverse currency pair.
    #[must_use]
    pub fn inverse(&self) -> Self {
        Self {
            base: self.quote,
            quote: self.base,
        }
    }

    /// Returns the pair as a string (e.g., "EUR/USD").
    #[must_use]
    pub fn to_string_pair(&self) -> String {
        format!("{}/{}", self.base.code(), self.quote.code())
    }
}

impl std::fmt::Display for CurrencyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.base.code(), self.quote.code())
    }
}

/// FX spot transaction.
///
/// An immediate exchange of currencies at the current spot rate,
/// typically settling T+2.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxSpot {
    /// Currency pair.
    pub currency_pair: CurrencyPair,
    /// Spot rate (quote currency per unit of base currency).
    pub spot_rate: f64,
    /// Settlement date.
    pub settlement_date: Date,
    /// Notional amount in the notional currency.
    pub notional: f64,
    /// Currency of the notional.
    pub notional_currency: Currency,
}

impl FxSpot {
    /// Validates the FX spot parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.spot_rate <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Spot rate must be positive",
            ));
        }
        Ok(())
    }
}

/// FX forward transaction.
///
/// An agreement to exchange currencies at a predetermined rate
/// on a future date.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxForward {
    /// Currency pair.
    pub currency_pair: CurrencyPair,
    /// Forward rate (quote currency per unit of base currency).
    pub forward_rate: f64,
    /// Settlement date.
    pub settlement_date: Date,
    /// Notional amount in the notional currency.
    pub notional: f64,
    /// Currency of the notional.
    pub notional_currency: Currency,
}

impl FxForward {
    /// Validates the FX forward parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.forward_rate <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Forward rate must be positive",
            ));
        }
        Ok(())
    }
}

/// FX vanilla option.
///
/// A standard European or American option to exchange currencies
/// at a predetermined strike rate.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxVanillaOption {
    /// Currency pair.
    pub currency_pair: CurrencyPair,
    /// Strike rate.
    pub strike: f64,
    /// Expiry date.
    pub expiry: Date,
    /// Delivery date (typically spot after expiry).
    pub delivery_date: Date,
    /// Option type (Call or Put).
    pub option_type: OptionType,
    /// Exercise style (European, American, Bermudan).
    pub exercise_style: ExerciseStyle,
    /// Notional amount.
    pub notional: f64,
    /// Currency of the notional.
    pub notional_currency: Currency,
}

impl FxVanillaOption {
    /// Validates the FX vanilla option parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.strike <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Strike must be positive",
            ));
        }
        if self.delivery_date < self.expiry {
            return Err(InstrumentError::invalid_date(
                "Delivery date must be on or after expiry",
            ));
        }
        Ok(())
    }
}

/// FX barrier option.
///
/// An option with a barrier that, if breached, either activates
/// (knock-in) or deactivates (knock-out) the option.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxBarrierOption {
    /// Underlying vanilla option.
    pub vanilla: FxVanillaOption,
    /// Barrier level.
    pub barrier_level: f64,
    /// Barrier type (KnockIn or KnockOut).
    pub barrier_type: BarrierType,
    /// Barrier direction (Up or Down).
    pub barrier_direction: BarrierDirection,
    /// Rebate amount (paid if option is knocked out).
    pub rebate: Option<f64>,
}

impl FxBarrierOption {
    /// Validates the FX barrier option parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        self.vanilla.validate()?;

        if self.barrier_level <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Barrier level must be positive",
            ));
        }

        if let Some(rebate) = self.rebate {
            if rebate < 0.0 {
                return Err(InstrumentError::invalid_parameter(
                    "Rebate must be non-negative",
                ));
            }
        }

        // Validate barrier vs strike consistency
        match (self.barrier_direction, self.vanilla.option_type) {
            (BarrierDirection::Up, OptionType::Call) => {
                if self.barrier_level <= self.vanilla.strike {
                    return Err(InstrumentError::invalid_parameter(
                        "Up-and-in/out call barrier must be above strike",
                    ));
                }
            }
            (BarrierDirection::Down, OptionType::Put) => {
                if self.barrier_level >= self.vanilla.strike {
                    return Err(InstrumentError::invalid_parameter(
                        "Down-and-in/out put barrier must be below strike",
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }
}

/// FX swap (short-term swap).
///
/// A combination of a spot and forward transaction,
/// exchanging currencies on the near leg and reversing on the far leg.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxSwap {
    /// Currency pair.
    pub currency_pair: CurrencyPair,
    /// Near leg date.
    pub near_leg_date: Date,
    /// Far leg date.
    pub far_leg_date: Date,
    /// Near leg rate.
    pub near_rate: f64,
    /// Far leg rate.
    pub far_rate: f64,
    /// Notional amount.
    pub notional: f64,
    /// Currency of the notional.
    pub notional_currency: Currency,
}

impl FxSwap {
    /// Validates the FX swap parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.near_rate <= 0.0 || self.far_rate <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Rates must be positive",
            ));
        }
        if self.far_leg_date <= self.near_leg_date {
            return Err(InstrumentError::invalid_date(
                "Far leg date must be after near leg date",
            ));
        }
        Ok(())
    }

    /// Returns the swap points (far rate - near rate).
    #[must_use]
    pub fn swap_points(&self) -> f64 {
        self.far_rate - self.near_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_currency_pair() -> CurrencyPair {
        CurrencyPair::new(Currency::EUR, Currency::USD)
    }

    #[test]
    fn test_currency_pair_new() {
        let pair = make_test_currency_pair();
        assert_eq!(pair.base, Currency::EUR);
        assert_eq!(pair.quote, Currency::USD);
    }

    #[test]
    fn test_currency_pair_inverse() {
        let pair = make_test_currency_pair();
        let inverse = pair.inverse();
        assert_eq!(inverse.base, Currency::USD);
        assert_eq!(inverse.quote, Currency::EUR);
    }

    #[test]
    fn test_currency_pair_display() {
        let pair = make_test_currency_pair();
        assert_eq!(pair.to_string(), "EUR/USD");
    }

    #[test]
    fn test_fx_spot_validate_success() {
        let spot = FxSpot {
            currency_pair: make_test_currency_pair(),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        assert!(spot.validate().is_ok());
    }

    #[test]
    fn test_fx_spot_validate_negative_notional() {
        let spot = FxSpot {
            currency_pair: make_test_currency_pair(),
            spot_rate: 1.1050,
            settlement_date: Date::from_ymd(2025, 1, 3).unwrap(),
            notional: -1_000.0,
            notional_currency: Currency::EUR,
        };
        assert!(spot.validate().is_err());
    }

    #[test]
    fn test_fx_forward_validate_success() {
        let fwd = FxForward {
            currency_pair: make_test_currency_pair(),
            forward_rate: 1.1100,
            settlement_date: Date::from_ymd(2025, 7, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        assert!(fwd.validate().is_ok());
    }

    #[test]
    fn test_fx_forward_validate_negative_rate() {
        let fwd = FxForward {
            currency_pair: make_test_currency_pair(),
            forward_rate: -1.1100,
            settlement_date: Date::from_ymd(2025, 7, 3).unwrap(),
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        assert!(fwd.validate().is_err());
    }

    fn make_test_vanilla_option() -> FxVanillaOption {
        FxVanillaOption {
            currency_pair: make_test_currency_pair(),
            strike: 1.1000,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            delivery_date: Date::from_ymd(2025, 6, 17).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        }
    }

    #[test]
    fn test_fx_vanilla_option_validate_success() {
        let option = make_test_vanilla_option();
        assert!(option.validate().is_ok());
    }

    #[test]
    fn test_fx_vanilla_option_validate_invalid_dates() {
        let mut option = make_test_vanilla_option();
        option.delivery_date = Date::from_ymd(2025, 6, 14).unwrap(); // before expiry
        assert!(option.validate().is_err());
    }

    #[test]
    fn test_fx_barrier_option_validate_success() {
        let barrier = FxBarrierOption {
            vanilla: make_test_vanilla_option(),
            barrier_level: 1.1500, // above strike for up-and-out call
            barrier_type: BarrierType::KnockOut,
            barrier_direction: BarrierDirection::Up,
            rebate: Some(0.001),
        };
        assert!(barrier.validate().is_ok());
    }

    #[test]
    fn test_fx_barrier_option_validate_invalid_barrier_level() {
        let barrier = FxBarrierOption {
            vanilla: make_test_vanilla_option(),
            barrier_level: 1.0500, // below strike (invalid for up-and-out call)
            barrier_type: BarrierType::KnockOut,
            barrier_direction: BarrierDirection::Up,
            rebate: None,
        };
        assert!(barrier.validate().is_err());
    }

    #[test]
    fn test_fx_swap_validate_success() {
        let swap = FxSwap {
            currency_pair: make_test_currency_pair(),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        assert!(swap.validate().is_ok());
    }

    #[test]
    fn test_fx_swap_validate_invalid_dates() {
        let swap = FxSwap {
            currency_pair: make_test_currency_pair(),
            near_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 1, 3).unwrap(), // before near leg
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        assert!(swap.validate().is_err());
    }

    #[test]
    fn test_fx_swap_points() {
        let swap = FxSwap {
            currency_pair: make_test_currency_pair(),
            near_leg_date: Date::from_ymd(2025, 1, 3).unwrap(),
            far_leg_date: Date::from_ymd(2025, 4, 3).unwrap(),
            near_rate: 1.1050,
            far_rate: 1.1070,
            notional: 1_000_000.0,
            notional_currency: Currency::EUR,
        };
        assert!((swap.swap_points() - 0.0020).abs() < 1e-10);
    }
}
