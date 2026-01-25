//! FX Volatility instrument definitions.
//!
//! This module provides definitions for FX volatility instruments including
//! ATM, Butterfly (BF), Risk Reversal (RR), and Delta-quoted options.
//! These instruments are used for calibrating FX volatility surfaces.

use super::{error::InstrumentError, CurrencyPair};
use crate::{
    time::{CalendarId, DayCounter},
    trade::OptionType,
    Currency, Date,
};

// ============================================================================
// Error Types
// ============================================================================

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

// ============================================================================
// Delta Type and Convention
// ============================================================================

/// Delta type convention for FX options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DeltaType {
    /// Spot delta (standard for most G10 pairs like EURUSD).
    SpotDelta,
    /// Premium-adjusted delta (standard for pairs like USDJPY).
    PremiumAdjustedDelta,
    /// Forward delta.
    ForwardDelta,
}

impl Default for DeltaType {
    fn default() -> Self { Self::SpotDelta }
}

/// Cut-off time for option expiry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CutOffTime {
    /// New York 10:00 AM (standard for most FX options).
    NewYork10am,
    /// Tokyo 3:00 PM.
    Tokyo3pm,
    /// London 10:00 AM.
    London10am,
}

impl Default for CutOffTime {
    fn default() -> Self { Self::NewYork10am }
}

/// FX Vol Convention specification.
///
/// Contains all the market conventions needed for FX volatility quoting
/// and delta-strike conversion.
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
            delta_type: DeltaType::PremiumAdjustedDelta,
            premium_currency: Currency::JPY,
            cut_off: CutOffTime::Tokyo3pm,
            calendar: CalendarId::Tokyo,
            day_count: DayCounter::Actual365Fixed,
        }
    }

    /// Creates default convention for a given currency pair.
    ///
    /// Returns premium-adjusted delta for JPY pairs, spot delta for others.
    #[must_use]
    pub fn for_currency_pair(pair: &CurrencyPair) -> Self {
        // USDJPY and other JPY pairs typically use premium-adjusted delta
        if pair.quote == Currency::JPY || pair.base == Currency::JPY {
            Self::usdjpy()
        } else {
            Self::eurusd()
        }
    }
}

// ============================================================================
// Delta Newtype
// ============================================================================

/// Delta value for FX options (0 < delta <= 50).
///
/// Delta is quoted as a percentage (e.g., 25 for 25-delta).
/// Valid range is (0, 50] where 50 represents ATM.
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
    ///
    /// # Arguments
    ///
    /// * `value` - Delta value (must be 0 < value <= 50)
    ///
    /// # Errors
    ///
    /// Returns `FxVolInstrumentError::InvalidDelta` if value is outside (0,
    /// 50].
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

// ============================================================================
// FX Vol Instrument
// ============================================================================

/// FX Volatility Instrument variants.
///
/// These instruments are used for calibrating FX volatility surfaces.
/// The standard market convention quotes ATM, Butterfly (BF), and
/// Risk Reversal (RR) instruments at various delta points.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FxVolInstrument {
    /// At-the-money volatility quote.
    ///
    /// ATM is typically quoted at 50-delta (straddle).
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
    ///
    /// BF = (σ_call + σ_put) / 2 - σ_ATM
    /// Measures the curvature of the volatility smile.
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
    ///
    /// RR = σ_call - σ_put
    /// Measures the skew of the volatility smile.
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
    ///
    /// Direct volatility quote for a specific delta and option type.
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
                // Butterfly spread can be positive or negative, but typically small
                if vol_spread.abs() > 1.0 {
                    return Err(InstrumentError::invalid_parameter(
                        "Butterfly spread seems unreasonably large",
                    ));
                }
            }
            Self::RiskReversal { vol_spread, .. } => {
                // Risk reversal can be positive or negative
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
                    OptionType::Call => "C",
                    OptionType::Put => "P",
                };
                write!(f, "{} {} {} {}", currency_pair, delta, opt_str, expiry)
            }
        }
    }
}

// ============================================================================
// Builder Pattern
// ============================================================================

/// Builder for constructing FxVolInstrument instances with fluent API.
///
/// # Example
///
/// ```
/// use infra_master::trade::instrument_def::{
///     FxVolInstrumentBuilder, CurrencyPair, Delta, FxVolConvention,
/// };
/// use infra_master::{Currency, Date};
///
/// let inst = FxVolInstrumentBuilder::new(
///         CurrencyPair::new(Currency::EUR, Currency::USD),
///         Date::from_ymd(2026, 6, 15).unwrap(),
///     )
///     .atm(0.10)
///     .build()
///     .unwrap();
/// ```
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
    ///
    /// # Errors
    ///
    /// Returns `InstrumentError` if:
    /// - No instrument type was specified
    /// - Validation fails (e.g., negative volatility)
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // === Delta Tests ===

    #[test]
    fn test_delta_new_valid() {
        let d25 = Delta::new(25.0).unwrap();
        assert!((d25.value() - 25.0).abs() < 1e-10);
        assert!((d25.as_decimal() - 0.25).abs() < 1e-10);
    }

    #[test]
    fn test_delta_new_at_boundary() {
        // 50 should be valid (ATM)
        let d50 = Delta::new(50.0).unwrap();
        assert!((d50.value() - 50.0).abs() < 1e-10);

        // Very small but positive should be valid
        let d_small = Delta::new(0.001).unwrap();
        assert!((d_small.value() - 0.001).abs() < 1e-10);
    }

    #[test]
    fn test_delta_new_invalid_zero() {
        let result = Delta::new(0.0);
        assert!(matches!(
            result,
            Err(FxVolInstrumentError::InvalidDelta(0.0))
        ));
    }

    #[test]
    fn test_delta_new_invalid_negative() {
        let result = Delta::new(-10.0);
        assert!(matches!(result, Err(FxVolInstrumentError::InvalidDelta(_))));
    }

    #[test]
    fn test_delta_new_invalid_above_50() {
        let result = Delta::new(51.0);
        assert!(matches!(
            result,
            Err(FxVolInstrumentError::InvalidDelta(51.0))
        ));
    }

    #[test]
    fn test_delta_constants() {
        assert!((Delta::D10.value() - 10.0).abs() < 1e-10);
        assert!((Delta::D25.value() - 25.0).abs() < 1e-10);
        assert!((Delta::ATM.value() - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_delta_display() {
        let d25 = Delta::D25;
        assert_eq!(d25.to_string(), "25D");
    }

    // === FxVolConvention Tests ===

    #[test]
    fn test_convention_default() {
        let conv = FxVolConvention::default();
        assert_eq!(conv.delta_type, DeltaType::SpotDelta);
        assert_eq!(conv.premium_currency, Currency::USD);
        assert_eq!(conv.cut_off, CutOffTime::NewYork10am);
    }

    #[test]
    fn test_convention_eurusd() {
        let conv = FxVolConvention::eurusd();
        assert_eq!(conv.delta_type, DeltaType::SpotDelta);
        assert_eq!(conv.premium_currency, Currency::USD);
    }

    #[test]
    fn test_convention_usdjpy() {
        let conv = FxVolConvention::usdjpy();
        assert_eq!(conv.delta_type, DeltaType::PremiumAdjustedDelta);
        assert_eq!(conv.premium_currency, Currency::JPY);
    }

    #[test]
    fn test_convention_for_currency_pair() {
        let eurusd = CurrencyPair::new(Currency::EUR, Currency::USD);
        let conv_eurusd = FxVolConvention::for_currency_pair(&eurusd);
        assert_eq!(conv_eurusd.delta_type, DeltaType::SpotDelta);

        let usdjpy = CurrencyPair::new(Currency::USD, Currency::JPY);
        let conv_usdjpy = FxVolConvention::for_currency_pair(&usdjpy);
        assert_eq!(conv_usdjpy.delta_type, DeltaType::PremiumAdjustedDelta);

        let eurjpy = CurrencyPair::new(Currency::EUR, Currency::JPY);
        let conv_eurjpy = FxVolConvention::for_currency_pair(&eurjpy);
        assert_eq!(conv_eurjpy.delta_type, DeltaType::PremiumAdjustedDelta);
    }

    // === FxVolInstrument Tests ===

    fn make_test_currency_pair() -> CurrencyPair { CurrencyPair::new(Currency::EUR, Currency::USD) }

    fn make_test_expiry() -> Date { Date::from_ymd(2026, 6, 15).unwrap() }

    #[test]
    fn test_atm_instrument() {
        let inst = FxVolInstrument::Atm {
            currency_pair: make_test_currency_pair(),
            expiry: make_test_expiry(),
            vol: 0.10,
            convention: FxVolConvention::eurusd(),
        };

        assert_eq!(inst.currency_pair(), make_test_currency_pair());
        assert_eq!(inst.expiry(), make_test_expiry());
        assert!(inst.validate().is_ok());
    }

    #[test]
    fn test_atm_instrument_invalid_vol() {
        let inst = FxVolInstrument::Atm {
            currency_pair: make_test_currency_pair(),
            expiry: make_test_expiry(),
            vol: -0.10,
            convention: FxVolConvention::eurusd(),
        };

        assert!(inst.validate().is_err());
    }

    #[test]
    fn test_butterfly_instrument() {
        let inst = FxVolInstrument::Butterfly {
            currency_pair: make_test_currency_pair(),
            expiry: make_test_expiry(),
            delta: Delta::D25,
            vol_spread: 0.005, // 0.5% butterfly
            convention: FxVolConvention::eurusd(),
        };

        assert!(inst.validate().is_ok());
        assert_eq!(inst.to_string(), "EUR/USD 25D BF 2026-06-15");
    }

    #[test]
    fn test_risk_reversal_instrument() {
        let inst = FxVolInstrument::RiskReversal {
            currency_pair: make_test_currency_pair(),
            expiry: make_test_expiry(),
            delta: Delta::D25,
            vol_spread: -0.01, // -1% risk reversal (puts richer)
            convention: FxVolConvention::eurusd(),
        };

        assert!(inst.validate().is_ok());
        assert_eq!(inst.to_string(), "EUR/USD 25D RR 2026-06-15");
    }

    #[test]
    fn test_delta_quoted_instrument() {
        let inst = FxVolInstrument::DeltaQuoted {
            currency_pair: make_test_currency_pair(),
            expiry: make_test_expiry(),
            delta: Delta::D25,
            vol: 0.11,
            option_type: OptionType::Call,
            convention: FxVolConvention::eurusd(),
        };

        assert!(inst.validate().is_ok());
        assert_eq!(inst.to_string(), "EUR/USD 25D C 2026-06-15");
    }

    #[test]
    fn test_delta_quoted_invalid_vol() {
        let inst = FxVolInstrument::DeltaQuoted {
            currency_pair: make_test_currency_pair(),
            expiry: make_test_expiry(),
            delta: Delta::D25,
            vol: 0.0,
            option_type: OptionType::Put,
            convention: FxVolConvention::eurusd(),
        };

        assert!(inst.validate().is_err());
    }

    #[test]
    fn test_instrument_display_atm() {
        let inst = FxVolInstrument::Atm {
            currency_pair: make_test_currency_pair(),
            expiry: make_test_expiry(),
            vol: 0.10,
            convention: FxVolConvention::eurusd(),
        };

        assert_eq!(inst.to_string(), "EUR/USD ATM 2026-06-15");
    }

    // === Clone and PartialEq Tests ===

    #[test]
    fn test_instrument_clone() {
        let inst = FxVolInstrument::Atm {
            currency_pair: make_test_currency_pair(),
            expiry: make_test_expiry(),
            vol: 0.10,
            convention: FxVolConvention::eurusd(),
        };

        let cloned = inst.clone();
        assert_eq!(inst, cloned);
    }

    #[test]
    fn test_delta_clone() {
        let d = Delta::D25;
        let cloned = d;
        assert_eq!(d, cloned);
    }

    // === Delta Type Tests ===

    #[test]
    fn test_delta_type_default() {
        assert_eq!(DeltaType::default(), DeltaType::SpotDelta);
    }

    #[test]
    fn test_cut_off_time_default() {
        assert_eq!(CutOffTime::default(), CutOffTime::NewYork10am);
    }

    // === Builder Tests ===

    #[test]
    fn test_builder_atm() {
        let inst = FxVolInstrumentBuilder::new(make_test_currency_pair(), make_test_expiry())
            .atm(0.10)
            .build()
            .unwrap();

        assert!(matches!(inst, FxVolInstrument::Atm { vol, .. } if (vol - 0.10).abs() < 1e-10));
    }

    #[test]
    fn test_builder_butterfly() {
        let inst = FxVolInstrumentBuilder::new(make_test_currency_pair(), make_test_expiry())
            .butterfly(Delta::D25, 0.005)
            .build()
            .unwrap();

        assert!(
            matches!(inst, FxVolInstrument::Butterfly { delta, vol_spread, .. }
            if (delta.value() - 25.0).abs() < 1e-10 && (vol_spread - 0.005).abs() < 1e-10)
        );
    }

    #[test]
    fn test_builder_risk_reversal() {
        let inst = FxVolInstrumentBuilder::new(make_test_currency_pair(), make_test_expiry())
            .risk_reversal(Delta::D25, -0.01)
            .build()
            .unwrap();

        assert!(
            matches!(inst, FxVolInstrument::RiskReversal { delta, vol_spread, .. }
            if (delta.value() - 25.0).abs() < 1e-10 && (vol_spread - (-0.01)).abs() < 1e-10)
        );
    }

    #[test]
    fn test_builder_delta_quoted() {
        let inst = FxVolInstrumentBuilder::new(make_test_currency_pair(), make_test_expiry())
            .delta_quoted(Delta::D25, 0.11, OptionType::Call)
            .build()
            .unwrap();

        assert!(
            matches!(inst, FxVolInstrument::DeltaQuoted { delta, vol, option_type: OptionType::Call, .. }
            if (delta.value() - 25.0).abs() < 1e-10 && (vol - 0.11).abs() < 1e-10)
        );
    }

    #[test]
    fn test_builder_with_custom_convention() {
        let custom_conv = FxVolConvention::usdjpy();
        let inst = FxVolInstrumentBuilder::new(make_test_currency_pair(), make_test_expiry())
            .with_convention(custom_conv)
            .atm(0.10)
            .build()
            .unwrap();

        assert_eq!(
            inst.convention().delta_type,
            DeltaType::PremiumAdjustedDelta
        );
    }

    #[test]
    fn test_builder_no_instrument_type_error() {
        let result =
            FxVolInstrumentBuilder::new(make_test_currency_pair(), make_test_expiry()).build();

        assert!(result.is_err());
    }

    #[test]
    fn test_builder_invalid_vol_error() {
        let result = FxVolInstrumentBuilder::new(make_test_currency_pair(), make_test_expiry())
            .atm(-0.10)
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_builder_fluent_chain() {
        // Test that builder methods can be chained in any order
        let inst = FxVolInstrumentBuilder::new(make_test_currency_pair(), make_test_expiry())
            .with_convention(FxVolConvention::eurusd())
            .butterfly(Delta::D10, 0.003)
            .build()
            .unwrap();

        assert!(matches!(inst, FxVolInstrument::Butterfly { .. }));
    }
}
