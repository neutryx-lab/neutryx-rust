//! Instrument definition for curve calibration.
//!
//! This module provides [`InstrumentDefinition`] which defines calibration
//! instruments as master data. These definitions are used by
//! [`CurveDefinition`] to specify which instruments make up a curve.
//!
//! # Examples
//!
//! ```
//! use infra_master::market::{InstrumentDefinition, Currency, RateType};
//!
//! // Deposit instrument
//! let depo = InstrumentDefinition::new(
//!     "USD-Depo-ON",
//!     Currency::USD,
//!     RateType::Deposit,
//!     "O/N",
//! );
//!
//! // OIS instrument with rate index
//! let ois = InstrumentDefinition::new(
//!     "USD-OIS-5Y",
//!     Currency::USD,
//!     RateType::Ois,
//!     "5Y",
//! ).with_rate_index("USD-SOFR");
//! ```

use std::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::market::{Currency, RateId, RateType};
use crate::time::{parse_fra_tenor, parse_tenor_to_years, CalendarId, DayCounter, Frequency, Tenor};

/// Instrument definition for curve calibration.
///
/// Defines a calibration instrument as master data, specifying its type,
/// currency, tenor, and optional market conventions.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct InstrumentDefinition {
    /// Unique identifier (e.g., "USD-Depo-ON", "USD-OIS-5Y")
    pub id: String,

    /// Currency of the instrument
    pub currency: Currency,

    /// Instrument type (Deposit, OIS, Swap, FRA, Futures)
    pub rate_type: RateType,

    /// Tenor specification (e.g., "O/N", "3M", "5Y", or FRA format "3x6")
    pub tenor: String,

    /// Related rate index ID (e.g., "USD-SOFR") - optional
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub rate_index: Option<String>,

    /// Market conventions override - optional
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub conventions: Option<InstrumentConventions>,
}

/// Market conventions for an instrument.
///
/// If not specified, defaults are derived from the instrument type and currency.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct InstrumentConventions {
    /// Day count convention
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub day_count: Option<DayCounter>,

    /// Spot lag in business days
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub spot_lag: Option<u8>,

    /// Holiday calendar
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub calendar: Option<CalendarId>,

    /// Payment frequency
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub payment_frequency: Option<Frequency>,
}

/// Error type for instrument definition parsing and validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstrumentDefError {
    /// Invalid tenor format
    InvalidTenor(String),
    /// Missing required field
    MissingField(&'static str),
    /// Invalid FRA tenor
    InvalidFraTenor(String),
}

impl std::fmt::Display for InstrumentDefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTenor(s) => write!(f, "Invalid tenor: {}", s),
            Self::MissingField(field) => write!(f, "Missing required field: {}", field),
            Self::InvalidFraTenor(s) => write!(f, "Invalid FRA tenor: {}", s),
        }
    }
}

impl std::error::Error for InstrumentDefError {}

impl InstrumentDefinition {
    /// Creates a new instrument definition.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier
    /// * `currency` - Currency of the instrument
    /// * `rate_type` - Type of the instrument
    /// * `tenor` - Tenor string (e.g., "O/N", "3M", "5Y", "3x6")
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        currency: Currency,
        rate_type: RateType,
        tenor: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            currency,
            rate_type,
            tenor: tenor.into(),
            rate_index: None,
            conventions: None,
        }
    }

    /// Sets the rate index for this instrument.
    #[must_use]
    pub fn with_rate_index(mut self, rate_index: impl Into<String>) -> Self {
        self.rate_index = Some(rate_index.into());
        self
    }

    /// Sets the conventions for this instrument.
    #[must_use]
    pub fn with_conventions(mut self, conventions: InstrumentConventions) -> Self {
        self.conventions = Some(conventions);
        self
    }

    /// Converts the tenor to years.
    ///
    /// For FRA instruments, returns the end tenor in years.
    ///
    /// # Errors
    ///
    /// Returns error if the tenor cannot be parsed.
    pub fn tenor_years(&self) -> Result<f64, InstrumentDefError> {
        // Check for FRA tenor first
        if let Some((_, end)) = parse_fra_tenor(&self.tenor) {
            return Ok(end);
        }

        parse_tenor_to_years(&self.tenor).map_err(|_| InstrumentDefError::InvalidTenor(self.tenor.clone()))
    }

    /// Returns the FRA start and end tenors in years, if this is a FRA instrument.
    ///
    /// Returns `None` if this is not a FRA or the tenor is not in FRA format.
    #[must_use]
    pub fn fra_tenors(&self) -> Option<(f64, f64)> {
        parse_fra_tenor(&self.tenor)
    }

    /// Checks if this instrument has a FRA tenor format.
    #[must_use]
    pub fn is_fra_tenor(&self) -> bool {
        parse_fra_tenor(&self.tenor).is_some()
    }

    /// Converts to a [`RateId`] for lookup in [`MarketRateSet`].
    ///
    /// # Errors
    ///
    /// Returns error if the tenor cannot be parsed.
    pub fn to_rate_id(&self) -> Result<RateId, InstrumentDefError> {
        let tenor = self.parse_tenor()?;
        Ok(RateId::new(self.currency, tenor, self.rate_type))
    }

    /// Parses the tenor string to a [`Tenor`] enum.
    ///
    /// For FRA tenors, returns the end tenor.
    fn parse_tenor(&self) -> Result<Tenor, InstrumentDefError> {
        // For FRA, extract the end tenor
        if let Some((_, end_years)) = parse_fra_tenor(&self.tenor) {
            return Self::years_to_tenor(end_years)
                .ok_or_else(|| InstrumentDefError::InvalidFraTenor(self.tenor.clone()));
        }

        // Try parsing as standard Tenor
        Tenor::from_str(&self.tenor).map_err(|_| InstrumentDefError::InvalidTenor(self.tenor.clone()))
    }

    /// Converts a year fraction to the closest standard Tenor.
    fn years_to_tenor(years: f64) -> Option<Tenor> {
        let months = (years * 12.0).round() as u32;

        match months {
            0 => Some(Tenor::Overnight),
            1 => Some(Tenor::OneMonth),
            2 => Some(Tenor::TwoMonths),
            3 => Some(Tenor::ThreeMonths),
            6 => Some(Tenor::SixMonths),
            9 => Some(Tenor::NineMonths),
            12 => Some(Tenor::OneYear),
            24 => Some(Tenor::TwoYears),
            36 => Some(Tenor::ThreeYears),
            60 => Some(Tenor::FiveYears),
            84 => Some(Tenor::SevenYears),
            120 => Some(Tenor::TenYears),
            180 => Some(Tenor::FifteenYears),
            240 => Some(Tenor::TwentyYears),
            360 => Some(Tenor::ThirtyYears),
            _ => None,
        }
    }

    /// Validates the instrument definition.
    ///
    /// # Errors
    ///
    /// Returns error if validation fails.
    pub fn validate(&self) -> Result<(), InstrumentDefError> {
        if self.id.is_empty() {
            return Err(InstrumentDefError::MissingField("id"));
        }

        // Verify tenor can be parsed
        let _ = self.tenor_years()?;

        // FRA-specific validation
        if self.rate_type == RateType::Fra && !self.is_fra_tenor() {
            // FRA should have FRA tenor format, but we allow standard tenor too
        }

        Ok(())
    }
}

impl InstrumentConventions {
    /// Creates a new conventions builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the day count convention.
    #[must_use]
    pub fn with_day_count(mut self, day_count: DayCounter) -> Self {
        self.day_count = Some(day_count);
        self
    }

    /// Sets the spot lag.
    #[must_use]
    pub fn with_spot_lag(mut self, spot_lag: u8) -> Self {
        self.spot_lag = Some(spot_lag);
        self
    }

    /// Sets the calendar.
    #[must_use]
    pub fn with_calendar(mut self, calendar: CalendarId) -> Self {
        self.calendar = Some(calendar);
        self
    }

    /// Sets the payment frequency.
    #[must_use]
    pub fn with_payment_frequency(mut self, frequency: Frequency) -> Self {
        self.payment_frequency = Some(frequency);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instrument_definition_new() {
        let def = InstrumentDefinition::new("USD-Depo-ON", Currency::USD, RateType::Deposit, "O/N");

        assert_eq!(def.id, "USD-Depo-ON");
        assert_eq!(def.currency, Currency::USD);
        assert_eq!(def.rate_type, RateType::Deposit);
        assert_eq!(def.tenor, "O/N");
        assert!(def.rate_index.is_none());
        assert!(def.conventions.is_none());
    }

    #[test]
    fn test_instrument_definition_with_rate_index() {
        let def = InstrumentDefinition::new("USD-OIS-5Y", Currency::USD, RateType::Ois, "5Y")
            .with_rate_index("USD-SOFR");

        assert_eq!(def.rate_index, Some("USD-SOFR".to_string()));
    }

    #[test]
    fn test_instrument_definition_with_conventions() {
        let conventions = InstrumentConventions::new()
            .with_day_count(DayCounter::Actual360)
            .with_spot_lag(2);

        let def = InstrumentDefinition::new("USD-Depo-3M", Currency::USD, RateType::Deposit, "3M")
            .with_conventions(conventions.clone());

        assert_eq!(def.conventions, Some(conventions));
    }

    #[test]
    fn test_tenor_years_standard() {
        let def = InstrumentDefinition::new("USD-OIS-5Y", Currency::USD, RateType::Ois, "5Y");
        assert!((def.tenor_years().unwrap() - 5.0).abs() < 1e-10);

        let def = InstrumentDefinition::new("USD-Depo-3M", Currency::USD, RateType::Deposit, "3M");
        assert!((def.tenor_years().unwrap() - 0.25).abs() < 1e-10);

        let def = InstrumentDefinition::new("USD-Depo-ON", Currency::USD, RateType::Deposit, "O/N");
        assert!((def.tenor_years().unwrap() - 1.0 / 365.0).abs() < 1e-10);
    }

    #[test]
    fn test_tenor_years_fra() {
        let def = InstrumentDefinition::new("USD-FRA-3x6", Currency::USD, RateType::Fra, "3x6");
        // FRA 3x6 ends at 6 months = 0.5 years
        assert!((def.tenor_years().unwrap() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_fra_tenors() {
        let def = InstrumentDefinition::new("USD-FRA-3x6", Currency::USD, RateType::Fra, "3x6");
        let (start, end) = def.fra_tenors().unwrap();
        assert!((start - 0.25).abs() < 1e-10);
        assert!((end - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_fra_tenors_none_for_standard() {
        let def = InstrumentDefinition::new("USD-OIS-5Y", Currency::USD, RateType::Ois, "5Y");
        assert!(def.fra_tenors().is_none());
    }

    #[test]
    fn test_to_rate_id() {
        let def = InstrumentDefinition::new("USD-OIS-5Y", Currency::USD, RateType::Ois, "5Y");
        let rate_id = def.to_rate_id().unwrap();

        assert_eq!(rate_id.currency, Currency::USD);
        assert_eq!(rate_id.rate_type, RateType::Ois);
        assert_eq!(rate_id.tenor, Tenor::FiveYears);
    }

    #[test]
    fn test_validate_success() {
        let def = InstrumentDefinition::new("USD-OIS-5Y", Currency::USD, RateType::Ois, "5Y");
        assert!(def.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_id() {
        let def = InstrumentDefinition::new("", Currency::USD, RateType::Ois, "5Y");
        assert!(matches!(
            def.validate(),
            Err(InstrumentDefError::MissingField("id"))
        ));
    }

    #[test]
    fn test_validate_invalid_tenor() {
        let def = InstrumentDefinition::new("USD-OIS-INVALID", Currency::USD, RateType::Ois, "INVALID");
        assert!(matches!(
            def.validate(),
            Err(InstrumentDefError::InvalidTenor(_))
        ));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let def = InstrumentDefinition::new("USD-OIS-5Y", Currency::USD, RateType::Ois, "5Y")
            .with_rate_index("USD-SOFR");

        let json = serde_json::to_string(&def).unwrap();
        let parsed: InstrumentDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(def, parsed);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_from_json() {
        let json = r#"{
            "id": "USD-Depo-ON",
            "currency": "USD",
            "rateType": "Deposit",
            "tenor": "O/N"
        }"#;

        let def: InstrumentDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.id, "USD-Depo-ON");
        assert_eq!(def.currency, Currency::USD);
        assert_eq!(def.rate_type, RateType::Deposit);
        assert_eq!(def.tenor, "O/N");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_with_conventions() {
        let json = r#"{
            "id": "USD-OIS-5Y",
            "currency": "USD",
            "rateType": "OIS",
            "tenor": "5Y",
            "rateIndex": "USD-SOFR",
            "conventions": {
                "dayCount": "Actual360",
                "spotLag": 2
            }
        }"#;

        let def: InstrumentDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.rate_index, Some("USD-SOFR".to_string()));
        assert!(def.conventions.is_some());
        let conv = def.conventions.unwrap();
        assert_eq!(conv.day_count, Some(DayCounter::Actual360));
        assert_eq!(conv.spot_lag, Some(2));
    }
}
