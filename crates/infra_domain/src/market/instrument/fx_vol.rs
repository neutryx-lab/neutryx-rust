//! FX Volatility instrument definitions.

use super::error::InstrumentError;
use crate::{
    market::{Currency, CurrencyPair},
    time::{CalendarId, Date, DayCounter},
    trade::OptionType,
};

/// Errors specific to FX Vol instrument operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FxVolInstrumentError {
    /// Invalid delta value (must be 0 < delta <= 50).
    #[error("Invalid delta: {0} (must be 0 < delta <= 50)")]
    InvalidDelta(f64),

    /// Invalid expiry date.
    #[error("Invalid expiry: {0} (must be future date)")]
    InvalidExpiry(Date),

    /// Invalid volatility value.
    #[error("Invalid volatility: {0} (must be positive)")]
    InvalidVolatility(f64),
}

/// Delta type convention for FX options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum DeltaType {
    /// Spot delta (standard for most G10 pairs like EURUSD).
    #[default]
    SpotDelta,
    /// Premium-adjusted delta (standard for pairs like USDJPY).
    PremiumAdjusted,
    /// Forward delta.
    ForwardDelta,
}

impl DeltaType {
    /// Returns the display name for this delta type.
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::SpotDelta => "Spot Delta",
            Self::ForwardDelta => "Forward Delta",
            Self::PremiumAdjusted => "Premium-Adjusted Delta",
        }
    }

    /// Returns a description of this delta type.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::SpotDelta => "Premium excluded, standard G10 convention",
            Self::ForwardDelta => "Premium excluded, measured vs forward",
            Self::PremiumAdjusted => "Premium included, common in EM markets",
        }
    }
}

/// Cut-off time for option expiry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CutOffTime {
    /// New York 10:00 AM (standard for most FX options).
    #[default]
    NewYork10am,
    /// Tokyo 3:00 PM.
    Tokyo3pm,
    /// London 10:00 AM.
    London10am,
}

/// FX Vol Convention specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FxVolConvention {
    /// Delta type (spot delta, premium-adjusted, forward).
    pub delta_type: DeltaType,
    /// Currency in which premium is quoted/paid.
    pub premium_currency: Currency,
    /// Cut-off time for option expiry.
    pub cut_off: CutOffTime,
    /// Holiday calendar for business day adjustments.
    pub calendar: CalendarId,
    /// Day count convention for time calculations.
    pub day_count: DayCounter,
}

impl Default for FxVolConvention {
    fn default() -> Self {
        Self {
            delta_type: DeltaType::SpotDelta,
            premium_currency: Currency::USD,
            cut_off: CutOffTime::NewYork10am,
            calendar: CalendarId::NewYork,
            day_count: DayCounter::Actual365Fixed,
        }
    }
}

impl FxVolConvention {
    /// Creates a convention for EURUSD (spot delta, USD premium).
    #[must_use]
    pub fn eurusd() -> Self {
        Self {
            delta_type: DeltaType::SpotDelta,
            premium_currency: Currency::USD,
            cut_off: CutOffTime::NewYork10am,
            calendar: CalendarId::NewYork,
            day_count: DayCounter::Actual365Fixed,
        }
    }

    /// Creates a convention for USDJPY (premium-adjusted delta, JPY premium).
    #[must_use]
    pub fn usdjpy() -> Self {
        Self {
            delta_type: DeltaType::PremiumAdjusted,
            premium_currency: Currency::JPY,
            cut_off: CutOffTime::Tokyo3pm,
            calendar: CalendarId::Tokyo,
            day_count: DayCounter::Actual365Fixed,
        }
    }

    /// Creates default convention for a given currency pair.
    #[must_use]
    pub fn for_currency_pair(pair: &CurrencyPair) -> Self {
        if pair.quote == Currency::JPY || pair.base == Currency::JPY {
            Self::usdjpy()
        } else {
            Self::eurusd()
        }
    }
}

/// Delta value for FX options (0 < delta <= 50).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Delta(f64);

impl Delta {
    /// Standard 10-delta.
    pub const D10: Delta = Delta(10.0);
    /// Standard 25-delta.
    pub const D25: Delta = Delta(25.0);
    /// Standard ATM (50-delta).
    pub const ATM: Delta = Delta(50.0);

    /// Creates a new Delta with validation.
    pub fn new(value: f64) -> Result<Self, FxVolInstrumentError> {
        if value <= 0.0 || value > 50.0 {
            return Err(FxVolInstrumentError::InvalidDelta(value));
        }
        Ok(Self(value))
    }

    /// Returns the delta value.
    #[inline]
    #[must_use]
    pub fn value(&self) -> f64 { self.0 }

    /// Returns the delta as a decimal (e.g., 0.25 for 25-delta).
    #[inline]
    #[must_use]
    pub fn as_decimal(&self) -> f64 { self.0 / 100.0 }
}

impl std::fmt::Display for Delta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}D", self.0) }
}

/// FX Volatility Instrument variants.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FxVolInstrument {
    /// At-the-money volatility quote.
    Atm {
        /// Currency pair for this instrument.
        currency_pair: CurrencyPair,
        /// Expiry date.
        expiry: Date,
        /// ATM volatility (annualised).
        vol: f64,
        /// Market convention.
        convention: FxVolConvention,
    },

    /// Butterfly spread volatility quote.
    Butterfly {
        /// Currency pair for this instrument.
        currency_pair: CurrencyPair,
        /// Expiry date.
        expiry: Date,
        /// Delta point (e.g., 25 for 25-delta butterfly).
        delta: Delta,
        /// Butterfly spread (vol_spread = BF value).
        vol_spread: f64,
        /// Market convention.
        convention: FxVolConvention,
    },

    /// Risk reversal volatility quote.
    RiskReversal {
        /// Currency pair for this instrument.
        currency_pair: CurrencyPair,
        /// Expiry date.
        expiry: Date,
        /// Delta point (e.g., 25 for 25-delta risk reversal).
        delta: Delta,
        /// Risk reversal spread (call vol minus put vol).
        vol_spread: f64,
        /// Market convention.
        convention: FxVolConvention,
    },

    /// Delta-quoted option volatility.
    DeltaQuoted {
        /// Currency pair for this instrument.
        currency_pair: CurrencyPair,
        /// Expiry date.
        expiry: Date,
        /// Delta point.
        delta: Delta,
        /// Implied volatility (annualised).
        vol: f64,
        /// Option type (Call or Put).
        option_type: OptionType,
        /// Market convention.
        convention: FxVolConvention,
    },
}

impl FxVolInstrument {
    /// Returns the currency pair for this instrument.
    #[must_use]
    pub fn currency_pair(&self) -> CurrencyPair {
        match self {
            Self::Atm { currency_pair, .. }
            | Self::Butterfly { currency_pair, .. }
            | Self::RiskReversal { currency_pair, .. }
            | Self::DeltaQuoted { currency_pair, .. } => *currency_pair,
        }
    }

    /// Returns the expiry date for this instrument.
    #[must_use]
    pub fn expiry(&self) -> Date {
        match self {
            Self::Atm { expiry, .. }
            | Self::Butterfly { expiry, .. }
            | Self::RiskReversal { expiry, .. }
            | Self::DeltaQuoted { expiry, .. } => *expiry,
        }
    }

    /// Returns the convention for this instrument.
    #[must_use]
    pub fn convention(&self) -> &FxVolConvention {
        match self {
            Self::Atm { convention, .. }
            | Self::Butterfly { convention, .. }
            | Self::RiskReversal { convention, .. }
            | Self::DeltaQuoted { convention, .. } => convention,
        }
    }

    /// Validates the instrument.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        match self {
            Self::Atm { vol, .. } => {
                if *vol <= 0.0 {
                    return Err(InstrumentError::invalid_parameter(
                        "ATM volatility must be positive",
                    ));
                }
            }
            Self::Butterfly { vol_spread, .. } => {
                if vol_spread.abs() > 1.0 {
                    return Err(InstrumentError::invalid_parameter(
                        "Butterfly spread seems unreasonably large",
                    ));
                }
            }
            Self::RiskReversal { vol_spread, .. } => {
                if vol_spread.abs() > 1.0 {
                    return Err(InstrumentError::invalid_parameter(
                        "Risk reversal spread seems unreasonably large",
                    ));
                }
            }
            Self::DeltaQuoted { vol, .. } => {
                if *vol <= 0.0 {
                    return Err(InstrumentError::invalid_parameter(
                        "Volatility must be positive",
                    ));
                }
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for FxVolInstrument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Atm {
                currency_pair,
                expiry,
                ..
            } => write!(f, "{} ATM {}", currency_pair, expiry),
            Self::Butterfly {
                currency_pair,
                expiry,
                delta,
                ..
            } => write!(f, "{} {} BF {}", currency_pair, delta, expiry),
            Self::RiskReversal {
                currency_pair,
                expiry,
                delta,
                ..
            } => write!(f, "{} {} RR {}", currency_pair, delta, expiry),
            Self::DeltaQuoted {
                currency_pair,
                expiry,
                delta,
                option_type,
                ..
            } => {
                let opt_str = match option_type {
                    OptionType::Call | OptionType::DigitalCall => "C",
                    OptionType::Put | OptionType::DigitalPut => "P",
                };
                write!(f, "{} {} {} {}", currency_pair, delta, opt_str, expiry)
            }
        }
    }
}

/// Builder for constructing FxVolInstrument instances with fluent API.
#[derive(Debug, Clone)]
pub struct FxVolInstrumentBuilder {
    currency_pair: CurrencyPair,
    expiry: Date,
    convention: FxVolConvention,
    instrument_type: Option<BuilderInstrumentType>,
}

#[derive(Debug, Clone)]
enum BuilderInstrumentType {
    Atm {
        vol: f64,
    },
    Butterfly {
        delta: Delta,
        vol_spread: f64,
    },
    RiskReversal {
        delta: Delta,
        vol_spread: f64,
    },
    DeltaQuoted {
        delta: Delta,
        vol: f64,
        option_type: OptionType,
    },
}

impl FxVolInstrumentBuilder {
    /// Creates a new builder with required currency pair and expiry.
    #[must_use]
    pub fn new(currency_pair: CurrencyPair, expiry: Date) -> Self {
        Self {
            currency_pair,
            expiry,
            convention: FxVolConvention::for_currency_pair(&currency_pair),
            instrument_type: None,
        }
    }

    /// Sets a custom convention (overrides the default for the currency pair).
    #[must_use]
    pub fn with_convention(mut self, convention: FxVolConvention) -> Self {
        self.convention = convention;
        self
    }

    /// Configures this builder for an ATM instrument.
    #[must_use]
    pub fn atm(mut self, vol: f64) -> Self {
        self.instrument_type = Some(BuilderInstrumentType::Atm { vol });
        self
    }

    /// Configures this builder for a Butterfly instrument.
    #[must_use]
    pub fn butterfly(mut self, delta: Delta, vol_spread: f64) -> Self {
        self.instrument_type = Some(BuilderInstrumentType::Butterfly { delta, vol_spread });
        self
    }

    /// Configures this builder for a Risk Reversal instrument.
    #[must_use]
    pub fn risk_reversal(mut self, delta: Delta, vol_spread: f64) -> Self {
        self.instrument_type = Some(BuilderInstrumentType::RiskReversal { delta, vol_spread });
        self
    }

    /// Configures this builder for a Delta-quoted instrument.
    #[must_use]
    pub fn delta_quoted(mut self, delta: Delta, vol: f64, option_type: OptionType) -> Self {
        self.instrument_type = Some(BuilderInstrumentType::DeltaQuoted {
            delta,
            vol,
            option_type,
        });
        self
    }

    /// Builds the FxVolInstrument, validating all parameters.
    pub fn build(self) -> Result<FxVolInstrument, InstrumentError> {
        let instrument = match self.instrument_type {
            Some(BuilderInstrumentType::Atm { vol }) => FxVolInstrument::Atm {
                currency_pair: self.currency_pair,
                expiry: self.expiry,
                vol,
                convention: self.convention,
            },
            Some(BuilderInstrumentType::Butterfly { delta, vol_spread }) => {
                FxVolInstrument::Butterfly {
                    currency_pair: self.currency_pair,
                    expiry: self.expiry,
                    delta,
                    vol_spread,
                    convention: self.convention,
                }
            }
            Some(BuilderInstrumentType::RiskReversal { delta, vol_spread }) => {
                FxVolInstrument::RiskReversal {
                    currency_pair: self.currency_pair,
                    expiry: self.expiry,
                    delta,
                    vol_spread,
                    convention: self.convention,
                }
            }
            Some(BuilderInstrumentType::DeltaQuoted {
                delta,
                vol,
                option_type,
            }) => FxVolInstrument::DeltaQuoted {
                currency_pair: self.currency_pair,
                expiry: self.expiry,
                delta,
                vol,
                option_type,
                convention: self.convention,
            },
            None => {
                return Err(InstrumentError::invalid_parameter(
                    "No instrument type specified. Call atm(), butterfly(), risk_reversal(), or delta_quoted() before build()",
                ));
            }
        };

        instrument.validate()?;
        Ok(instrument)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pair() -> CurrencyPair { CurrencyPair::new(Currency::EUR, Currency::USD) }
    fn make_expiry() -> Date { Date::from_ymd(2026, 6, 15).unwrap() }

    #[test]
    fn test_delta_validation_and_features() {
        let d25 = Delta::new(25.0).unwrap();
        assert!((d25.value() - 25.0).abs() < 1e-10);
        assert!((d25.as_decimal() - 0.25).abs() < 1e-10);

        let d50 = Delta::new(50.0).unwrap();
        assert!((d50.value() - 50.0).abs() < 1e-10);
        assert!((Delta::new(0.001).unwrap().value() - 0.001).abs() < 1e-10);

        assert!(matches!(
            Delta::new(0.0),
            Err(FxVolInstrumentError::InvalidDelta(0.0))
        ));
        assert!(matches!(
            Delta::new(-10.0),
            Err(FxVolInstrumentError::InvalidDelta(_))
        ));
        assert!(matches!(
            Delta::new(51.0),
            Err(FxVolInstrumentError::InvalidDelta(51.0))
        ));

        assert!((Delta::D10.value() - 10.0).abs() < 1e-10);
        assert!((Delta::D25.value() - 25.0).abs() < 1e-10);
        assert!((Delta::ATM.value() - 50.0).abs() < 1e-10);
        assert_eq!(Delta::D25.to_string(), "25D");
        let d = Delta::D25;
        let copied = d;
        assert_eq!(d, copied);

        assert_eq!(DeltaType::default(), DeltaType::SpotDelta);
        assert_eq!(CutOffTime::default(), CutOffTime::NewYork10am);
    }

    #[test]
    fn test_fx_vol_convention() {
        let def = FxVolConvention::default();
        assert_eq!(def.delta_type, DeltaType::SpotDelta);
        assert_eq!(def.premium_currency, Currency::USD);
        assert_eq!(def.cut_off, CutOffTime::NewYork10am);

        let eurusd = FxVolConvention::eurusd();
        assert_eq!(eurusd.delta_type, DeltaType::SpotDelta);
        assert_eq!(eurusd.premium_currency, Currency::USD);

        let usdjpy = FxVolConvention::usdjpy();
        assert_eq!(usdjpy.delta_type, DeltaType::PremiumAdjusted);
        assert_eq!(usdjpy.premium_currency, Currency::JPY);

        let pair_eu = CurrencyPair::new(Currency::EUR, Currency::USD);
        assert_eq!(
            FxVolConvention::for_currency_pair(&pair_eu).delta_type,
            DeltaType::SpotDelta
        );
        let pair_uj = CurrencyPair::new(Currency::USD, Currency::JPY);
        assert_eq!(
            FxVolConvention::for_currency_pair(&pair_uj).delta_type,
            DeltaType::PremiumAdjusted
        );
        let pair_ej = CurrencyPair::new(Currency::EUR, Currency::JPY);
        assert_eq!(
            FxVolConvention::for_currency_pair(&pair_ej).delta_type,
            DeltaType::PremiumAdjusted
        );
    }

    #[test]
    fn test_fx_vol_instrument_types() {
        let atm = FxVolInstrument::Atm {
            currency_pair: make_pair(),
            expiry: make_expiry(),
            vol: 0.10,
            convention: FxVolConvention::eurusd(),
        };
        assert_eq!(atm.currency_pair(), make_pair());
        assert_eq!(atm.expiry(), make_expiry());
        assert!(atm.validate().is_ok());
        assert_eq!(atm.to_string(), "EUR/USD ATM 2026-06-15");
        assert_eq!(atm.clone(), atm);

        let bad_atm = FxVolInstrument::Atm {
            currency_pair: make_pair(),
            expiry: make_expiry(),
            vol: -0.10,
            convention: FxVolConvention::eurusd(),
        };
        assert!(bad_atm.validate().is_err());

        let bf = FxVolInstrument::Butterfly {
            currency_pair: make_pair(),
            expiry: make_expiry(),
            delta: Delta::D25,
            vol_spread: 0.005,
            convention: FxVolConvention::eurusd(),
        };
        assert!(bf.validate().is_ok());
        assert_eq!(bf.to_string(), "EUR/USD 25D BF 2026-06-15");

        let rr = FxVolInstrument::RiskReversal {
            currency_pair: make_pair(),
            expiry: make_expiry(),
            delta: Delta::D25,
            vol_spread: -0.01,
            convention: FxVolConvention::eurusd(),
        };
        assert!(rr.validate().is_ok());
        assert_eq!(rr.to_string(), "EUR/USD 25D RR 2026-06-15");

        let dq = FxVolInstrument::DeltaQuoted {
            currency_pair: make_pair(),
            expiry: make_expiry(),
            delta: Delta::D25,
            vol: 0.11,
            option_type: OptionType::Call,
            convention: FxVolConvention::eurusd(),
        };
        assert!(dq.validate().is_ok());
        assert_eq!(dq.to_string(), "EUR/USD 25D C 2026-06-15");

        let bad_dq = FxVolInstrument::DeltaQuoted {
            currency_pair: make_pair(),
            expiry: make_expiry(),
            delta: Delta::D25,
            vol: 0.0,
            option_type: OptionType::Put,
            convention: FxVolConvention::eurusd(),
        };
        assert!(bad_dq.validate().is_err());
    }

    #[test]
    fn test_fx_vol_builder() {
        let pair = make_pair();
        let exp = make_expiry();

        let atm = FxVolInstrumentBuilder::new(pair, exp)
            .atm(0.10)
            .build()
            .unwrap();
        assert!(matches!(atm, FxVolInstrument::Atm { vol, .. } if (vol - 0.10).abs() < 1e-10));

        let bf = FxVolInstrumentBuilder::new(pair, exp)
            .butterfly(Delta::D25, 0.005)
            .build()
            .unwrap();
        assert!(
            matches!(bf, FxVolInstrument::Butterfly { delta, vol_spread, .. }
            if (delta.value() - 25.0).abs() < 1e-10 && (vol_spread - 0.005).abs() < 1e-10)
        );

        let rr = FxVolInstrumentBuilder::new(pair, exp)
            .risk_reversal(Delta::D25, -0.01)
            .build()
            .unwrap();
        assert!(
            matches!(rr, FxVolInstrument::RiskReversal { delta, vol_spread, .. }
            if (delta.value() - 25.0).abs() < 1e-10 && (vol_spread - (-0.01)).abs() < 1e-10)
        );

        let dq = FxVolInstrumentBuilder::new(pair, exp)
            .delta_quoted(Delta::D25, 0.11, OptionType::Call)
            .build()
            .unwrap();
        assert!(
            matches!(dq, FxVolInstrument::DeltaQuoted { delta, vol, option_type: OptionType::Call, .. }
            if (delta.value() - 25.0).abs() < 1e-10 && (vol - 0.11).abs() < 1e-10)
        );

        let custom = FxVolInstrumentBuilder::new(pair, exp)
            .with_convention(FxVolConvention::usdjpy())
            .atm(0.10)
            .build()
            .unwrap();
        assert_eq!(custom.convention().delta_type, DeltaType::PremiumAdjusted);

        let fluent = FxVolInstrumentBuilder::new(pair, exp)
            .with_convention(FxVolConvention::eurusd())
            .butterfly(Delta::D10, 0.003)
            .build()
            .unwrap();
        assert!(matches!(fluent, FxVolInstrument::Butterfly { .. }));

        assert!(FxVolInstrumentBuilder::new(pair, exp).build().is_err());
        assert!(FxVolInstrumentBuilder::new(pair, exp)
            .atm(-0.10)
            .build()
            .is_err());
    }
}
