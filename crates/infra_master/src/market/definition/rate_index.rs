//! Rate index definition for curve construction.
//!
//! This module provides [`RateIndexDefinition`] which defines benchmark
//! rate indices as master data. These definitions are used by
//! [`CurveDefinition`] to specify the underlying index for a curve.
//!
//! # Examples
//!
//! ```
//! use infra_master::market::definition::RateIndexDefinition;
//! use infra_master::market::{Currency, RateIndex};
//!
//! // SOFR definition
//! let sofr = RateIndexDefinition::new("USD-SOFR", Currency::USD, RateIndex::Sofr);
//!
//! // With display name
//! let estr = RateIndexDefinition::new("EUR-ESTR", Currency::EUR, RateIndex::Estr)
//!     .with_name("Euro Short-Term Rate");
//! ```

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::market::{CompoundingMethod, Currency, RateIndex};
use crate::time::{CalendarId, DayCounter, Tenor};

/// Rate index definition for curve construction.
///
/// Defines a benchmark rate index as master data, linking an ID to the
/// underlying [`RateIndex`] enum with optional convention overrides.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct RateIndexDefinition {
    /// Unique identifier (e.g., "USD-SOFR", "EUR-ESTR")
    pub id: String,

    /// Display name (optional)
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub name: Option<String>,

    /// Currency of the index
    pub currency: Currency,

    /// Index type from the enum
    pub index_type: RateIndex,

    /// Tenor of the index (e.g., O/N for SOFR, 6M for EURIBOR6M)
    pub tenor: Tenor,

    /// Convention overrides (optional, defaults derived from index_type)
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub conventions: Option<IndexConventions>,
}

/// Market conventions for a rate index.
///
/// If not specified, defaults are derived from the [`RateIndex`] metadata.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct IndexConventions {
    /// Day count convention
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub day_count: Option<DayCounter>,

    /// Compounding method
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub compounding: Option<CompoundingMethod>,

    /// Fixing lag in business days
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub fixing_lag: Option<u8>,

    /// Settlement lag in business days
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub settlement_lag: Option<u8>,

    /// Holiday calendar
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub calendar: Option<CalendarId>,
}

/// Error type for rate index definition validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateIndexDefError {
    /// Currency mismatch between definition and index type
    CurrencyMismatch {
        /// Definition ID
        id: String,
        /// Expected currency from index type
        expected: Currency,
        /// Provided currency
        got: Currency,
    },
    /// Missing required field
    MissingField(&'static str),
}

impl std::fmt::Display for RateIndexDefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CurrencyMismatch { id, expected, got } => {
                write!(
                    f,
                    "Currency mismatch for '{}': expected {:?}, got {:?}",
                    id, expected, got
                )
            }
            Self::MissingField(field) => write!(f, "Missing required field: {}", field),
        }
    }
}

impl std::error::Error for RateIndexDefError {}

impl RateIndexDefinition {
    /// Creates a new rate index definition.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier (e.g., "USD-SOFR")
    /// * `currency` - Currency of the index
    /// * `index_type` - The underlying rate index enum
    #[must_use]
    pub fn new(id: impl Into<String>, currency: Currency, index_type: RateIndex) -> Self {
        Self {
            id: id.into(),
            name: None,
            currency,
            index_type,
            tenor: index_type.tenor(),
            conventions: None,
        }
    }

    /// Sets the display name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the tenor explicitly.
    #[must_use]
    pub fn with_tenor(mut self, tenor: Tenor) -> Self {
        self.tenor = tenor;
        self
    }

    /// Sets the conventions override.
    #[must_use]
    pub fn with_conventions(mut self, conventions: IndexConventions) -> Self {
        self.conventions = Some(conventions);
        self
    }

    /// Returns true if this is an overnight index.
    #[must_use]
    pub fn is_overnight(&self) -> bool {
        self.tenor == Tenor::Overnight
    }

    /// Validates the rate index definition.
    ///
    /// Checks that the currency matches the index type's currency.
    ///
    /// # Errors
    ///
    /// Returns error if validation fails.
    pub fn validate(&self) -> Result<(), RateIndexDefError> {
        if self.id.is_empty() {
            return Err(RateIndexDefError::MissingField("id"));
        }

        // Currency must match the index type's currency
        let expected_currency = self.index_type.currency();
        if self.currency != expected_currency {
            return Err(RateIndexDefError::CurrencyMismatch {
                id: self.id.clone(),
                expected: expected_currency,
                got: self.currency,
            });
        }

        Ok(())
    }

    /// Returns the effective day count convention.
    ///
    /// Uses the convention override if set, otherwise falls back to index metadata.
    #[must_use]
    pub fn day_count(&self) -> DayCounter {
        self.conventions
            .as_ref()
            .and_then(|c| c.day_count)
            .unwrap_or_else(|| self.index_type.day_counter())
    }

    /// Returns the effective compounding method.
    ///
    /// Uses the convention override if set, otherwise falls back to index metadata.
    #[must_use]
    pub fn compounding(&self) -> CompoundingMethod {
        self.conventions
            .as_ref()
            .and_then(|c| c.compounding)
            .unwrap_or_else(|| self.index_type.metadata().compounding_method)
    }

    /// Returns the effective fixing lag.
    ///
    /// Uses the convention override if set, otherwise falls back to index metadata.
    #[must_use]
    pub fn fixing_lag(&self) -> u8 {
        self.conventions
            .as_ref()
            .and_then(|c| c.fixing_lag)
            .unwrap_or_else(|| self.index_type.metadata().fixing_lag)
    }

    /// Returns the effective settlement lag.
    ///
    /// Uses the convention override if set, otherwise falls back to index metadata.
    #[must_use]
    pub fn settlement_lag(&self) -> u8 {
        self.conventions
            .as_ref()
            .and_then(|c| c.settlement_lag)
            .unwrap_or_else(|| self.index_type.metadata().settlement_lag)
    }

    /// Returns the effective calendar.
    ///
    /// Uses the convention override if set, otherwise falls back to index metadata.
    #[must_use]
    pub fn calendar(&self) -> CalendarId {
        self.conventions
            .as_ref()
            .and_then(|c| c.calendar)
            .unwrap_or_else(|| self.index_type.metadata().calendar)
    }
}

impl IndexConventions {
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

    /// Sets the compounding method.
    #[must_use]
    pub fn with_compounding(mut self, compounding: CompoundingMethod) -> Self {
        self.compounding = Some(compounding);
        self
    }

    /// Sets the fixing lag.
    #[must_use]
    pub fn with_fixing_lag(mut self, fixing_lag: u8) -> Self {
        self.fixing_lag = Some(fixing_lag);
        self
    }

    /// Sets the settlement lag.
    #[must_use]
    pub fn with_settlement_lag(mut self, settlement_lag: u8) -> Self {
        self.settlement_lag = Some(settlement_lag);
        self
    }

    /// Sets the calendar.
    #[must_use]
    pub fn with_calendar(mut self, calendar: CalendarId) -> Self {
        self.calendar = Some(calendar);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_index_definition_new() {
        let def = RateIndexDefinition::new("USD-SOFR", Currency::USD, RateIndex::Sofr);

        assert_eq!(def.id, "USD-SOFR");
        assert_eq!(def.currency, Currency::USD);
        assert_eq!(def.index_type, RateIndex::Sofr);
        assert_eq!(def.tenor, Tenor::Overnight);
        assert!(def.is_overnight());
        assert!(def.name.is_none());
        assert!(def.conventions.is_none());
    }

    #[test]
    fn test_rate_index_definition_with_name() {
        let def = RateIndexDefinition::new("EUR-ESTR", Currency::EUR, RateIndex::Estr)
            .with_name("Euro Short-Term Rate");

        assert_eq!(def.name, Some("Euro Short-Term Rate".to_string()));
    }

    #[test]
    fn test_rate_index_definition_term_index() {
        let def = RateIndexDefinition::new("EUR-EURIBOR3M", Currency::EUR, RateIndex::Euribor3M);

        assert!(!def.is_overnight());
        assert_eq!(def.tenor, Tenor::ThreeMonths);
    }

    #[test]
    fn test_rate_index_definition_euribor6m() {
        let def = RateIndexDefinition::new("EUR-EURIBOR6M", Currency::EUR, RateIndex::Euribor6M);

        assert!(!def.is_overnight());
        assert_eq!(def.tenor, Tenor::SixMonths);
    }

    #[test]
    fn test_validate_success() {
        let def = RateIndexDefinition::new("USD-SOFR", Currency::USD, RateIndex::Sofr);
        assert!(def.validate().is_ok());
    }

    #[test]
    fn test_validate_currency_mismatch() {
        let def = RateIndexDefinition::new("USD-SOFR", Currency::EUR, RateIndex::Sofr);
        let result = def.validate();

        assert!(matches!(
            result,
            Err(RateIndexDefError::CurrencyMismatch { .. })
        ));
    }

    #[test]
    fn test_validate_empty_id() {
        let def = RateIndexDefinition::new("", Currency::USD, RateIndex::Sofr);
        assert!(matches!(
            def.validate(),
            Err(RateIndexDefError::MissingField("id"))
        ));
    }

    #[test]
    fn test_effective_conventions_defaults() {
        let def = RateIndexDefinition::new("USD-SOFR", Currency::USD, RateIndex::Sofr);

        assert_eq!(def.day_count(), DayCounter::Actual360);
        assert_eq!(def.compounding(), CompoundingMethod::Compounded);
        assert_eq!(def.fixing_lag(), 0);
        assert_eq!(def.settlement_lag(), 2);
        assert_eq!(def.calendar(), CalendarId::NewYork);
    }

    #[test]
    fn test_effective_conventions_override() {
        let conventions = IndexConventions::new()
            .with_day_count(DayCounter::Actual365Fixed)
            .with_settlement_lag(1);

        let def = RateIndexDefinition::new("USD-SOFR", Currency::USD, RateIndex::Sofr)
            .with_conventions(conventions);

        // Overridden values
        assert_eq!(def.day_count(), DayCounter::Actual365Fixed);
        assert_eq!(def.settlement_lag(), 1);

        // Defaults still used for non-overridden
        assert_eq!(def.fixing_lag(), 0);
        assert_eq!(def.compounding(), CompoundingMethod::Compounded);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let def = RateIndexDefinition::new("USD-SOFR", Currency::USD, RateIndex::Sofr)
            .with_name("Secured Overnight Financing Rate");

        let json = serde_json::to_string(&def).unwrap();
        let parsed: RateIndexDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(def, parsed);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_from_json_overnight() {
        let json = r#"{
            "id": "EUR-ESTR",
            "currency": "EUR",
            "indexType": "Estr",
            "tenor": "O/N"
        }"#;

        let def: RateIndexDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.id, "EUR-ESTR");
        assert_eq!(def.currency, Currency::EUR);
        assert_eq!(def.index_type, RateIndex::Estr);
        assert_eq!(def.tenor, Tenor::Overnight);
        assert!(def.is_overnight());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_from_json_term() {
        let json = r#"{
            "id": "EUR-EURIBOR6M",
            "currency": "EUR",
            "indexType": "Euribor6M",
            "tenor": "6M"
        }"#;

        let def: RateIndexDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.id, "EUR-EURIBOR6M");
        assert_eq!(def.currency, Currency::EUR);
        assert_eq!(def.index_type, RateIndex::Euribor6M);
        assert_eq!(def.tenor, Tenor::SixMonths);
        assert!(!def.is_overnight());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_with_conventions() {
        let json = r#"{
            "id": "USD-SOFR",
            "currency": "USD",
            "indexType": "Sofr",
            "tenor": "O/N",
            "conventions": {
                "dayCount": "Actual365Fixed",
                "compounding": "Compounded",
                "fixingLag": 1,
                "settlementLag": 2,
                "calendar": "NewYork"
            }
        }"#;

        let def: RateIndexDefinition = serde_json::from_str(json).unwrap();
        assert!(def.conventions.is_some());

        let conv = def.conventions.unwrap();
        assert_eq!(conv.day_count, Some(DayCounter::Actual365Fixed));
        assert_eq!(conv.compounding, Some(CompoundingMethod::Compounded));
        assert_eq!(conv.fixing_lag, Some(1));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_tenor_serializes_as_code() {
        let def = RateIndexDefinition::new("EUR-EURIBOR6M", Currency::EUR, RateIndex::Euribor6M);
        let json = serde_json::to_string(&def).unwrap();

        // Should serialize tenor as "6M", not "SixMonths"
        assert!(json.contains(r#""tenor":"6M""#));
    }
}
