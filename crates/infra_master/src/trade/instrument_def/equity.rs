//! Equity instrument definitions.
//!
//! This module provides definitions for equity derivatives including
//! forwards, vanilla options, barrier options, Asian options,
//! lookback options, equity swaps, and basket options.

use crate::{Currency, Date, Frequency};

use super::common::{BarrierDirection, BarrierType, ExerciseStyle};
use super::error::InstrumentError;
use crate::trade::OptionType;

/// Underlying asset for equity instruments.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EquityUnderlying {
    /// Single stock.
    SingleStock {
        /// Stock ticker/identifier.
        ticker: String,
        /// Exchange code (e.g., "NYSE", "LSE").
        exchange: Option<String>,
    },
    /// Equity index.
    Index {
        /// Index name (e.g., "S&P 500", "FTSE 100").
        name: String,
    },
}

impl EquityUnderlying {
    /// Creates a single stock underlying.
    #[must_use]
    pub fn stock(ticker: impl Into<String>) -> Self {
        EquityUnderlying::SingleStock {
            ticker: ticker.into(),
            exchange: None,
        }
    }

    /// Creates a single stock underlying with exchange.
    #[must_use]
    pub fn stock_with_exchange(ticker: impl Into<String>, exchange: impl Into<String>) -> Self {
        EquityUnderlying::SingleStock {
            ticker: ticker.into(),
            exchange: Some(exchange.into()),
        }
    }

    /// Creates an index underlying.
    #[must_use]
    pub fn index(name: impl Into<String>) -> Self {
        EquityUnderlying::Index { name: name.into() }
    }
}

/// Equity forward contract.
///
/// An agreement to buy/sell an equity at a predetermined price
/// on a future date.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EquityForward {
    /// Underlying asset.
    pub underlying: EquityUnderlying,
    /// Forward price.
    pub forward_price: f64,
    /// Settlement date.
    pub settlement_date: Date,
    /// Notional amount (number of shares or index units).
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
}

impl EquityForward {
    /// Validates the equity forward parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.forward_price <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Forward price must be positive",
            ));
        }
        Ok(())
    }
}

/// Equity vanilla option.
///
/// A standard option on an equity underlying.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EquityVanillaOption {
    /// Underlying asset.
    pub underlying: EquityUnderlying,
    /// Strike price.
    pub strike: f64,
    /// Expiry date.
    pub expiry: Date,
    /// Option type (Call or Put).
    pub option_type: OptionType,
    /// Exercise style.
    pub exercise_style: ExerciseStyle,
    /// Notional (number of shares or multiplier).
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
}

impl EquityVanillaOption {
    /// Validates the equity vanilla option parameters.
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
        Ok(())
    }
}

/// Monitoring frequency for path-dependent options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MonitoringFrequency {
    /// Continuous monitoring.
    Continuous,
    /// Discrete monitoring at specific intervals.
    Discrete(Frequency),
}

/// Equity barrier option.
///
/// An option with a barrier that activates or deactivates the option.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EquityBarrierOption {
    /// Underlying vanilla option.
    pub vanilla: EquityVanillaOption,
    /// Barrier level.
    pub barrier_level: f64,
    /// Barrier type (KnockIn or KnockOut).
    pub barrier_type: BarrierType,
    /// Barrier direction (Up or Down).
    pub barrier_direction: BarrierDirection,
    /// Monitoring frequency.
    pub monitoring_frequency: MonitoringFrequency,
    /// Rebate amount.
    pub rebate: Option<f64>,
}

impl EquityBarrierOption {
    /// Validates the equity barrier option parameters.
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

        Ok(())
    }
}

/// Averaging type for Asian options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AveragingType {
    /// Arithmetic average.
    Arithmetic,
    /// Geometric average.
    Geometric,
}

/// Asian option (average price option).
///
/// An option whose payoff depends on the average price of the underlying
/// over a specified period.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AsianOption {
    /// Underlying asset.
    pub underlying: EquityUnderlying,
    /// Strike price.
    pub strike: f64,
    /// Expiry date.
    pub expiry: Date,
    /// Option type (Call or Put).
    pub option_type: OptionType,
    /// Averaging type.
    pub averaging_type: AveragingType,
    /// Observation frequency.
    pub observation_frequency: Frequency,
    /// Already observed values (for in-progress options).
    pub observed_values: Vec<f64>,
    /// Notional.
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
}

impl AsianOption {
    /// Validates the Asian option parameters.
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
        for val in &self.observed_values {
            if *val < 0.0 {
                return Err(InstrumentError::invalid_parameter(
                    "Observed values must be non-negative",
                ));
            }
        }
        Ok(())
    }
}

/// Lookback option type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LookbackType {
    /// Fixed strike lookback (payoff uses maximum/minimum price).
    FixedStrike,
    /// Floating strike lookback (strike set at maximum/minimum price).
    FloatingStrike,
}

/// Lookback option.
///
/// An option whose payoff depends on the maximum or minimum price
/// of the underlying over the option's life.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LookbackOption {
    /// Underlying asset.
    pub underlying: EquityUnderlying,
    /// Strike price (for fixed strike lookback).
    pub strike: Option<f64>,
    /// Expiry date.
    pub expiry: Date,
    /// Option type (Call or Put).
    pub option_type: OptionType,
    /// Lookback type.
    pub lookback_type: LookbackType,
    /// Observation start date.
    pub observation_start: Date,
    /// Notional.
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
}

impl LookbackOption {
    /// Validates the lookback option parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }

        // Fixed strike lookback must have a strike
        if self.lookback_type == LookbackType::FixedStrike && self.strike.is_none() {
            return Err(InstrumentError::invalid_parameter(
                "Fixed strike lookback must have a strike",
            ));
        }

        if let Some(strike) = self.strike {
            if strike <= 0.0 {
                return Err(InstrumentError::invalid_parameter(
                    "Strike must be positive",
                ));
            }
        }

        if self.observation_start > self.expiry {
            return Err(InstrumentError::invalid_date(
                "Observation start must be on or before expiry",
            ));
        }

        Ok(())
    }
}

/// Return type for equity leg of equity swaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EquityReturnType {
    /// Price return only.
    Price,
    /// Total return (including dividends).
    TotalReturn,
}

/// Equity swap.
///
/// A swap exchanging equity returns for interest payments.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EquitySwap {
    /// Underlying asset for equity leg.
    pub underlying: EquityUnderlying,
    /// Return type for equity leg.
    pub return_type: EquityReturnType,
    /// Funding rate index (e.g., SOFR, EURIBOR).
    pub funding_index: String,
    /// Funding spread (over the index).
    pub funding_spread: f64,
    /// Start date.
    pub start_date: Date,
    /// Maturity date.
    pub maturity: Date,
    /// Notional amount.
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
}

impl EquitySwap {
    /// Validates the equity swap parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.maturity <= self.start_date {
            return Err(InstrumentError::invalid_date(
                "Maturity must be after start date",
            ));
        }
        if self.funding_index.is_empty() {
            return Err(InstrumentError::invalid_parameter(
                "Funding index must be specified",
            ));
        }
        Ok(())
    }
}

/// Component of a basket option.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BasketComponent {
    /// Underlying asset.
    pub underlying: EquityUnderlying,
    /// Weight in the basket (sum of weights typically equals 1).
    pub weight: f64,
}

/// Basket option.
///
/// An option on a weighted basket of underlying assets.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BasketOption {
    /// Components of the basket.
    pub components: Vec<BasketComponent>,
    /// Strike price (for the basket value).
    pub strike: f64,
    /// Expiry date.
    pub expiry: Date,
    /// Option type (Call or Put).
    pub option_type: OptionType,
    /// Exercise style.
    pub exercise_style: ExerciseStyle,
    /// Notional.
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
    /// Reference to correlation matrix (external ID).
    pub correlation_matrix_ref: Option<String>,
}

impl BasketOption {
    /// Validates the basket option parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.components.is_empty() {
            return Err(InstrumentError::invalid_parameter(
                "Basket must have at least one component",
            ));
        }
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

        // Check weights sum to approximately 1
        let weight_sum: f64 = self.components.iter().map(|c| c.weight).sum();
        if (weight_sum - 1.0).abs() > 0.0001 {
            return Err(InstrumentError::invalid_parameter(
                "Component weights must sum to 1",
            ));
        }

        for component in &self.components {
            if component.weight <= 0.0 {
                return Err(InstrumentError::invalid_parameter(
                    "Component weights must be positive",
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_underlying() -> EquityUnderlying {
        EquityUnderlying::stock("AAPL")
    }

    #[test]
    fn test_equity_underlying_stock() {
        let underlying = EquityUnderlying::stock("AAPL");
        assert!(matches!(underlying, EquityUnderlying::SingleStock { .. }));
    }

    #[test]
    fn test_equity_underlying_stock_with_exchange() {
        let underlying = EquityUnderlying::stock_with_exchange("VOD", "LSE");
        if let EquityUnderlying::SingleStock { ticker, exchange } = underlying {
            assert_eq!(ticker, "VOD");
            assert_eq!(exchange, Some("LSE".to_string()));
        } else {
            panic!("Expected SingleStock");
        }
    }

    #[test]
    fn test_equity_underlying_index() {
        let underlying = EquityUnderlying::index("S&P 500");
        assert!(matches!(underlying, EquityUnderlying::Index { .. }));
    }

    #[test]
    fn test_equity_forward_validate_success() {
        let fwd = EquityForward {
            underlying: make_test_underlying(),
            forward_price: 150.0,
            settlement_date: Date::from_ymd(2025, 6, 15).unwrap(),
            notional: 100.0,
            currency: Currency::USD,
        };
        assert!(fwd.validate().is_ok());
    }

    #[test]
    fn test_equity_forward_validate_negative_notional() {
        let fwd = EquityForward {
            underlying: make_test_underlying(),
            forward_price: 150.0,
            settlement_date: Date::from_ymd(2025, 6, 15).unwrap(),
            notional: -100.0,
            currency: Currency::USD,
        };
        assert!(fwd.validate().is_err());
    }

    fn make_test_vanilla_option() -> EquityVanillaOption {
        EquityVanillaOption {
            underlying: make_test_underlying(),
            strike: 150.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 100.0,
            currency: Currency::USD,
        }
    }

    #[test]
    fn test_equity_vanilla_option_validate_success() {
        let option = make_test_vanilla_option();
        assert!(option.validate().is_ok());
    }

    #[test]
    fn test_equity_barrier_option_validate_success() {
        let barrier = EquityBarrierOption {
            vanilla: make_test_vanilla_option(),
            barrier_level: 170.0,
            barrier_type: BarrierType::KnockOut,
            barrier_direction: BarrierDirection::Up,
            monitoring_frequency: MonitoringFrequency::Continuous,
            rebate: None,
        };
        assert!(barrier.validate().is_ok());
    }

    #[test]
    fn test_asian_option_validate_success() {
        let asian = AsianOption {
            underlying: make_test_underlying(),
            strike: 150.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            averaging_type: AveragingType::Arithmetic,
            observation_frequency: Frequency::Daily,
            observed_values: vec![145.0, 148.0],
            notional: 100.0,
            currency: Currency::USD,
        };
        assert!(asian.validate().is_ok());
    }

    #[test]
    fn test_asian_option_validate_negative_observed() {
        let asian = AsianOption {
            underlying: make_test_underlying(),
            strike: 150.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            averaging_type: AveragingType::Geometric,
            observation_frequency: Frequency::Weekly,
            observed_values: vec![145.0, -10.0],
            notional: 100.0,
            currency: Currency::USD,
        };
        assert!(asian.validate().is_err());
    }

    #[test]
    fn test_lookback_option_validate_success() {
        let lookback = LookbackOption {
            underlying: make_test_underlying(),
            strike: Some(150.0),
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            lookback_type: LookbackType::FixedStrike,
            observation_start: Date::from_ymd(2025, 1, 1).unwrap(),
            notional: 100.0,
            currency: Currency::USD,
        };
        assert!(lookback.validate().is_ok());
    }

    #[test]
    fn test_lookback_option_validate_missing_strike() {
        let lookback = LookbackOption {
            underlying: make_test_underlying(),
            strike: None, // Missing for fixed strike
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            lookback_type: LookbackType::FixedStrike,
            observation_start: Date::from_ymd(2025, 1, 1).unwrap(),
            notional: 100.0,
            currency: Currency::USD,
        };
        assert!(lookback.validate().is_err());
    }

    #[test]
    fn test_equity_swap_validate_success() {
        let swap = EquitySwap {
            underlying: make_test_underlying(),
            return_type: EquityReturnType::TotalReturn,
            funding_index: "SOFR".to_string(),
            funding_spread: 0.001,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2026, 1, 1).unwrap(),
            notional: 1_000_000.0,
            currency: Currency::USD,
        };
        assert!(swap.validate().is_ok());
    }

    #[test]
    fn test_equity_swap_validate_invalid_dates() {
        let swap = EquitySwap {
            underlying: make_test_underlying(),
            return_type: EquityReturnType::Price,
            funding_index: "SOFR".to_string(),
            funding_spread: 0.001,
            start_date: Date::from_ymd(2026, 1, 1).unwrap(),
            maturity: Date::from_ymd(2025, 1, 1).unwrap(),
            notional: 1_000_000.0,
            currency: Currency::USD,
        };
        assert!(swap.validate().is_err());
    }

    #[test]
    fn test_basket_option_validate_success() {
        let basket = BasketOption {
            components: vec![
                BasketComponent {
                    underlying: EquityUnderlying::stock("AAPL"),
                    weight: 0.5,
                },
                BasketComponent {
                    underlying: EquityUnderlying::stock("MSFT"),
                    weight: 0.5,
                },
            ],
            strike: 100.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 1000.0,
            currency: Currency::USD,
            correlation_matrix_ref: Some("CORR001".to_string()),
        };
        assert!(basket.validate().is_ok());
    }

    #[test]
    fn test_basket_option_validate_empty_basket() {
        let basket = BasketOption {
            components: vec![],
            strike: 100.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 1000.0,
            currency: Currency::USD,
            correlation_matrix_ref: None,
        };
        assert!(basket.validate().is_err());
    }

    #[test]
    fn test_basket_option_validate_weights_not_sum_to_one() {
        let basket = BasketOption {
            components: vec![
                BasketComponent {
                    underlying: EquityUnderlying::stock("AAPL"),
                    weight: 0.5,
                },
                BasketComponent {
                    underlying: EquityUnderlying::stock("MSFT"),
                    weight: 0.3, // Sum = 0.8, not 1.0
                },
            ],
            strike: 100.0,
            expiry: Date::from_ymd(2025, 6, 15).unwrap(),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 1000.0,
            currency: Currency::USD,
            correlation_matrix_ref: None,
        };
        assert!(basket.validate().is_err());
    }

    #[test]
    fn test_averaging_type_equality() {
        assert_eq!(AveragingType::Arithmetic, AveragingType::Arithmetic);
        assert_ne!(AveragingType::Arithmetic, AveragingType::Geometric);
    }

    #[test]
    fn test_lookback_type_equality() {
        assert_eq!(LookbackType::FixedStrike, LookbackType::FixedStrike);
        assert_ne!(LookbackType::FixedStrike, LookbackType::FloatingStrike);
    }

    #[test]
    fn test_equity_return_type_equality() {
        assert_eq!(EquityReturnType::Price, EquityReturnType::Price);
        assert_ne!(EquityReturnType::Price, EquityReturnType::TotalReturn);
    }
}
