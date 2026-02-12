//! Equity instrument definitions.

use super::{
    common::{BarrierDirection, BarrierType, ExerciseStyle},
    error::InstrumentError,
};
use crate::{
    market::Currency,
    time::{Date, Frequency},
    trade::OptionType,
};

/// Underlying asset for equity instruments.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub fn index(name: impl Into<String>) -> Self { EquityUnderlying::Index { name: name.into() } }
}

/// Equity forward contract.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
        InstrumentError::check_positive(self.notional, "Notional")?;
        InstrumentError::check_positive(self.forward_price, "Forward price")?;
        Ok(())
    }
}

/// Equity vanilla option.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
        InstrumentError::check_positive(self.notional, "Notional")?;
        InstrumentError::check_positive(self.strike, "Strike")?;
        Ok(())
    }
}

/// Monitoring frequency for path-dependent options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum MonitoringFrequency {
    /// Continuous monitoring.
    Continuous,
    /// Discrete monitoring at specific intervals.
    Discrete(Frequency),
}

/// Equity barrier option.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

        InstrumentError::check_positive(self.barrier_level, "Barrier level")?;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum AveragingType {
    /// Arithmetic average.
    Arithmetic,
    /// Geometric average.
    Geometric,
}

/// Asian option (average price option).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
        InstrumentError::check_positive(self.notional, "Notional")?;
        InstrumentError::check_positive(self.strike, "Strike")?;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LookbackType {
    /// Fixed strike lookback (payoff uses maximum/minimum price).
    FixedStrike,
    /// Floating strike lookback (strike set at maximum/minimum price).
    FloatingStrike,
}

/// Lookback option.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
        InstrumentError::check_positive(self.notional, "Notional")?;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EquityReturnType {
    /// Price return only.
    Price,
    /// Total return (including dividends).
    TotalReturn,
}

/// Equity swap.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
        InstrumentError::check_positive(self.notional, "Notional")?;
        InstrumentError::check_date_order(self.start_date, self.maturity, "Maturity must be after start date")?;
        InstrumentError::check_not_empty(&self.funding_index, "Funding Index")?;
        Ok(())
    }
}

/// Component of a basket option.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BasketComponent {
    /// Underlying asset.
    pub underlying: EquityUnderlying,
    /// Weight in the basket (sum of weights typically equals 1).
    pub weight: f64,
}

/// Basket option.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
                "Components must not be empty",
            ));
        }
        InstrumentError::check_positive(self.notional, "Notional")?;
        InstrumentError::check_positive(self.strike, "Strike")?;

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

    fn aapl() -> EquityUnderlying { EquityUnderlying::stock("AAPL") }

    #[test]
    fn test_equity_underlying_and_types() {
        assert!(matches!(
            EquityUnderlying::stock("AAPL"),
            EquityUnderlying::SingleStock { .. }
        ));
        assert!(matches!(
            EquityUnderlying::index("S&P 500"),
            EquityUnderlying::Index { .. }
        ));
        if let EquityUnderlying::SingleStock { ticker, exchange } =
            EquityUnderlying::stock_with_exchange("VOD", "LSE")
        {
            assert_eq!(ticker, "VOD");
            assert_eq!(exchange, Some("LSE".to_string()));
        } else {
            panic!("Expected SingleStock");
        }

        assert_eq!(AveragingType::Arithmetic, AveragingType::Arithmetic);
        assert_ne!(AveragingType::Arithmetic, AveragingType::Geometric);
        assert_eq!(LookbackType::FixedStrike, LookbackType::FixedStrike);
        assert_ne!(LookbackType::FixedStrike, LookbackType::FloatingStrike);
        assert_eq!(EquityReturnType::Price, EquityReturnType::Price);
        assert_ne!(EquityReturnType::Price, EquityReturnType::TotalReturn);
    }

    #[test]
    fn test_equity_instruments_validation() {
        let exp = Date::from_ymd(2025, 6, 15).unwrap();

        let fwd = EquityForward {
            underlying: aapl(),
            forward_price: 150.0,
            settlement_date: exp,
            notional: 100.0,
            currency: Currency::USD,
        };
        assert!(fwd.validate().is_ok());
        let mut bad = fwd.clone();
        bad.notional = -100.0;
        assert!(bad.validate().is_err());

        let opt = EquityVanillaOption {
            underlying: aapl(),
            strike: 150.0,
            expiry: exp,
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 100.0,
            currency: Currency::USD,
        };
        assert!(opt.validate().is_ok());

        let barrier = EquityBarrierOption {
            vanilla: opt.clone(),
            barrier_level: 170.0,
            barrier_type: BarrierType::KnockOut,
            barrier_direction: BarrierDirection::Up,
            monitoring_frequency: MonitoringFrequency::Continuous,
            rebate: None,
        };
        assert!(barrier.validate().is_ok());

        let asian = AsianOption {
            underlying: aapl(),
            strike: 150.0,
            expiry: exp,
            option_type: OptionType::Call,
            averaging_type: AveragingType::Arithmetic,
            observation_frequency: Frequency::Daily,
            observed_values: vec![145.0, 148.0],
            notional: 100.0,
            currency: Currency::USD,
        };
        assert!(asian.validate().is_ok());
        let mut bad = asian.clone();
        bad.observed_values = vec![145.0, -10.0];
        bad.averaging_type = AveragingType::Geometric;
        assert!(bad.validate().is_err());

        let lookback = LookbackOption {
            underlying: aapl(),
            strike: Some(150.0),
            expiry: exp,
            option_type: OptionType::Call,
            lookback_type: LookbackType::FixedStrike,
            observation_start: Date::from_ymd(2025, 1, 1).unwrap(),
            notional: 100.0,
            currency: Currency::USD,
        };
        assert!(lookback.validate().is_ok());
        let mut bad = lookback.clone();
        bad.strike = None;
        assert!(bad.validate().is_err());

        let swap = EquitySwap {
            underlying: aapl(),
            return_type: EquityReturnType::TotalReturn,
            funding_index: "SOFR".to_string(),
            funding_spread: 0.001,
            start_date: Date::from_ymd(2025, 1, 1).unwrap(),
            maturity: Date::from_ymd(2026, 1, 1).unwrap(),
            notional: 1_000_000.0,
            currency: Currency::USD,
        };
        assert!(swap.validate().is_ok());
        let mut bad = swap.clone();
        bad.start_date = Date::from_ymd(2026, 1, 1).unwrap();
        bad.maturity = Date::from_ymd(2025, 1, 1).unwrap();
        assert!(bad.validate().is_err());

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
            expiry: exp,
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            notional: 1000.0,
            currency: Currency::USD,
            correlation_matrix_ref: Some("CORR001".to_string()),
        };
        assert!(basket.validate().is_ok());
        let mut bad = basket.clone();
        bad.components = vec![];
        assert!(bad.validate().is_err());
        let mut bad = basket.clone();
        bad.components[1].weight = 0.3;
        assert!(bad.validate().is_err());
    }
}
