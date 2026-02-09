//! Credit instrument definitions.
//!
//! This module provides definitions for credit derivatives including
//! CDS, CDS indices, CDS options, and Nth-to-Default baskets.

use super::error::InstrumentError;
use crate::{market::Currency, time::Date};

/// ISDA standard credit events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CreditEvent {
    /// Bankruptcy of the reference entity.
    Bankruptcy,
    /// Failure to pay obligations.
    FailureToPay,
    /// Restructuring of debt.
    Restructuring,
    /// Acceleration of obligations.
    ObligationAcceleration,
    /// Default on obligations.
    ObligationDefault,
    /// Repudiation or moratorium.
    RepudiationMoratorium,
}

impl CreditEvent {
    /// Returns the ISDA standard name for this credit event.
    #[must_use]
    pub fn isda_name(&self) -> &'static str {
        match self {
            CreditEvent::Bankruptcy => "Bankruptcy",
            CreditEvent::FailureToPay => "Failure to Pay",
            CreditEvent::Restructuring => "Restructuring",
            CreditEvent::ObligationAcceleration => "Obligation Acceleration",
            CreditEvent::ObligationDefault => "Obligation Default",
            CreditEvent::RepudiationMoratorium => "Repudiation/Moratorium",
        }
    }
}

/// Single-name Credit Default Swap (CDS).
///
/// A bilateral contract where the protection buyer pays periodic premiums
/// in exchange for protection against default of a reference entity.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Cds {
    /// Reference entity identifier (e.g., company name or RED code).
    pub reference_entity: String,
    /// Notional amount.
    pub notional: f64,
    /// CDS spread (premium rate, as decimal).
    pub spread: f64,
    /// Start date (effective date).
    pub start_date: Date,
    /// Maturity date.
    pub maturity: Date,
    /// Recovery rate (assumed or market, as decimal).
    pub recovery_rate: Option<f64>,
    /// Currency.
    pub currency: Currency,
    /// Applicable credit events.
    pub credit_events: Vec<CreditEvent>,
}

impl Cds {
    /// Validates the CDS parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.reference_entity.is_empty() {
            return Err(InstrumentError::invalid_parameter(
                "Reference entity must be specified",
            ));
        }
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.spread < 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Spread must be non-negative",
            ));
        }
        if self.maturity <= self.start_date {
            return Err(InstrumentError::invalid_date(
                "Maturity must be after start date",
            ));
        }
        if let Some(recovery) = self.recovery_rate {
            if !(0.0..=1.0).contains(&recovery) {
                return Err(InstrumentError::invalid_parameter(
                    "Recovery rate must be between 0 and 1",
                ));
            }
        }
        Ok(())
    }

    /// Creates a CDS with standard North American credit events.
    #[must_use]
    pub fn with_na_events(mut self) -> Self {
        self.credit_events = vec![
            CreditEvent::Bankruptcy,
            CreditEvent::FailureToPay,
            CreditEvent::Restructuring,
        ];
        self
    }

    /// Creates a CDS with standard European credit events.
    #[must_use]
    pub fn with_eu_events(mut self) -> Self {
        self.credit_events = vec![
            CreditEvent::Bankruptcy,
            CreditEvent::FailureToPay,
            CreditEvent::Restructuring,
        ];
        self
    }
}

/// CDS Index (e.g., CDX, iTraxx).
///
/// A standardised portfolio of single-name CDS contracts.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CdsIndex {
    /// Index name (e.g., "CDX.NA.IG", "iTraxx Europe").
    pub index_name: String,
    /// Series number.
    pub series: u32,
    /// Version number.
    pub version: u32,
    /// Number of constituents in the index.
    pub constituent_count: u32,
    /// Notional amount.
    pub notional: f64,
    /// Index spread (as decimal).
    pub spread: f64,
    /// Start date.
    pub start_date: Date,
    /// Maturity date.
    pub maturity: Date,
    /// Currency.
    pub currency: Currency,
}

impl CdsIndex {
    /// Validates the CDS index parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.index_name.is_empty() {
            return Err(InstrumentError::invalid_parameter(
                "Index name must be specified",
            ));
        }
        if self.constituent_count == 0 {
            return Err(InstrumentError::invalid_parameter(
                "Constituent count must be positive",
            ));
        }
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.spread < 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Spread must be non-negative",
            ));
        }
        if self.maturity <= self.start_date {
            return Err(InstrumentError::invalid_date(
                "Maturity must be after start date",
            ));
        }
        Ok(())
    }

    /// Returns the full index identifier (e.g., "CDX.NA.IG.39v1").
    #[must_use]
    pub fn full_identifier(&self) -> String {
        format!("{}.{}v{}", self.index_name, self.series, self.version)
    }
}

/// CDS Option (Swaption on CDS).
///
/// An option to enter into a CDS at a specified spread.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CdsOption {
    /// Underlying CDS reference entity.
    pub reference_entity: String,
    /// Strike spread (as decimal).
    pub strike_spread: f64,
    /// Exercise date.
    pub exercise_date: Date,
    /// Underlying CDS maturity.
    pub underlying_maturity: Date,
    /// Notional amount.
    pub notional: f64,
    /// Currency.
    pub currency: Currency,
    /// Payer option (right to buy protection) or receiver option (right to sell
    /// protection).
    pub is_payer: bool,
}

impl CdsOption {
    /// Validates the CDS option parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.reference_entity.is_empty() {
            return Err(InstrumentError::invalid_parameter(
                "Reference entity must be specified",
            ));
        }
        if self.strike_spread < 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Strike spread must be non-negative",
            ));
        }
        if self.underlying_maturity <= self.exercise_date {
            return Err(InstrumentError::invalid_date(
                "Underlying maturity must be after exercise date",
            ));
        }
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        Ok(())
    }
}

/// Nth-to-Default Basket.
///
/// A credit derivative that pays out when the Nth default occurs
/// in a basket of reference entities.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NtdBasket {
    /// Basket constituents (reference entities).
    pub constituents: Vec<String>,
    /// N parameter (which default triggers payout).
    pub nth_to_default: u32,
    /// Notional amount.
    pub notional: f64,
    /// Premium spread (as decimal).
    pub spread: f64,
    /// Start date.
    pub start_date: Date,
    /// Maturity date.
    pub maturity: Date,
    /// Currency.
    pub currency: Currency,
    /// Correlation parameter (for pricing).
    pub correlation_parameter: Option<f64>,
}

impl NtdBasket {
    /// Validates the NTD basket parameters.
    pub fn validate(&self) -> Result<(), InstrumentError> {
        if self.constituents.is_empty() {
            return Err(InstrumentError::invalid_parameter(
                "Basket must have at least one constituent",
            ));
        }
        if self.nth_to_default == 0 {
            return Err(InstrumentError::invalid_parameter(
                "Nth-to-default must be at least 1",
            ));
        }
        if self.nth_to_default > self.constituents.len() as u32 {
            return Err(InstrumentError::invalid_parameter(
                "Nth-to-default cannot exceed number of constituents",
            ));
        }
        if self.notional <= 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Notional must be positive",
            ));
        }
        if self.spread < 0.0 {
            return Err(InstrumentError::invalid_parameter(
                "Spread must be non-negative",
            ));
        }
        if self.maturity <= self.start_date {
            return Err(InstrumentError::invalid_date(
                "Maturity must be after start date",
            ));
        }
        if let Some(corr) = self.correlation_parameter {
            if !(-1.0..=1.0).contains(&corr) {
                return Err(InstrumentError::invalid_parameter(
                    "Correlation parameter must be between -1 and 1",
                ));
            }
        }
        Ok(())
    }

    /// Returns true if this is a first-to-default basket.
    #[must_use]
    pub fn is_first_to_default(&self) -> bool { self.nth_to_default == 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_cds() -> Cds {
        Cds {
            reference_entity: "ACME Corp".to_string(),
            notional: 10_000_000.0,
            spread: 0.01, // 100bps
            start_date: Date::from_ymd(2025, 1, 20).unwrap(),
            maturity: Date::from_ymd(2030, 1, 20).unwrap(),
            recovery_rate: Some(0.4),
            currency: Currency::USD,
            credit_events: vec![CreditEvent::Bankruptcy, CreditEvent::FailureToPay],
        }
    }

    #[test]
    fn test_credit_event_isda_name() {
        assert_eq!(CreditEvent::Bankruptcy.isda_name(), "Bankruptcy");
        assert_eq!(CreditEvent::FailureToPay.isda_name(), "Failure to Pay");
        assert_eq!(CreditEvent::Restructuring.isda_name(), "Restructuring");
    }

    #[test]
    fn test_cds_validate_success() {
        let cds = make_test_cds();
        assert!(cds.validate().is_ok());
    }

    #[test]
    fn test_cds_validate_empty_reference() {
        let mut cds = make_test_cds();
        cds.reference_entity = "".to_string();
        assert!(cds.validate().is_err());
    }

    #[test]
    fn test_cds_validate_negative_notional() {
        let mut cds = make_test_cds();
        cds.notional = -1000.0;
        assert!(cds.validate().is_err());
    }

    #[test]
    fn test_cds_validate_invalid_recovery() {
        let mut cds = make_test_cds();
        cds.recovery_rate = Some(1.5); // Invalid: > 1
        assert!(cds.validate().is_err());
    }

    #[test]
    fn test_cds_validate_invalid_dates() {
        let mut cds = make_test_cds();
        cds.maturity = Date::from_ymd(2024, 1, 1).unwrap(); // Before start
        assert!(cds.validate().is_err());
    }

    #[test]
    fn test_cds_with_na_events() {
        let cds = make_test_cds().with_na_events();
        assert!(cds.credit_events.contains(&CreditEvent::Bankruptcy));
        assert!(cds.credit_events.contains(&CreditEvent::FailureToPay));
    }

    fn make_test_cds_index() -> CdsIndex {
        CdsIndex {
            index_name: "CDX.NA.IG".to_string(),
            series: 39,
            version: 1,
            constituent_count: 125,
            notional: 10_000_000.0,
            spread: 0.006, // 60bps
            start_date: Date::from_ymd(2025, 3, 20).unwrap(),
            maturity: Date::from_ymd(2030, 3, 20).unwrap(),
            currency: Currency::USD,
        }
    }

    #[test]
    fn test_cds_index_validate_success() {
        let index = make_test_cds_index();
        assert!(index.validate().is_ok());
    }

    #[test]
    fn test_cds_index_full_identifier() {
        let index = make_test_cds_index();
        assert_eq!(index.full_identifier(), "CDX.NA.IG.39v1");
    }

    #[test]
    fn test_cds_index_validate_empty_name() {
        let mut index = make_test_cds_index();
        index.index_name = "".to_string();
        assert!(index.validate().is_err());
    }

    #[test]
    fn test_cds_index_validate_zero_constituents() {
        let mut index = make_test_cds_index();
        index.constituent_count = 0;
        assert!(index.validate().is_err());
    }

    fn make_test_cds_option() -> CdsOption {
        CdsOption {
            reference_entity: "ACME Corp".to_string(),
            strike_spread: 0.01,
            exercise_date: Date::from_ymd(2025, 6, 20).unwrap(),
            underlying_maturity: Date::from_ymd(2030, 6, 20).unwrap(),
            notional: 10_000_000.0,
            currency: Currency::USD,
            is_payer: true,
        }
    }

    #[test]
    fn test_cds_option_validate_success() {
        let option = make_test_cds_option();
        assert!(option.validate().is_ok());
    }

    #[test]
    fn test_cds_option_validate_invalid_dates() {
        let mut option = make_test_cds_option();
        option.underlying_maturity = Date::from_ymd(2025, 1, 1).unwrap(); // Before exercise
        assert!(option.validate().is_err());
    }

    fn make_test_ntd_basket() -> NtdBasket {
        NtdBasket {
            constituents: vec![
                "Company A".to_string(),
                "Company B".to_string(),
                "Company C".to_string(),
                "Company D".to_string(),
                "Company E".to_string(),
            ],
            nth_to_default: 1, // First-to-default
            notional: 10_000_000.0,
            spread: 0.015, // 150bps
            start_date: Date::from_ymd(2025, 1, 20).unwrap(),
            maturity: Date::from_ymd(2030, 1, 20).unwrap(),
            currency: Currency::USD,
            correlation_parameter: Some(0.3),
        }
    }

    #[test]
    fn test_ntd_basket_validate_success() {
        let basket = make_test_ntd_basket();
        assert!(basket.validate().is_ok());
    }

    #[test]
    fn test_ntd_basket_is_first_to_default() {
        let basket = make_test_ntd_basket();
        assert!(basket.is_first_to_default());
    }

    #[test]
    fn test_ntd_basket_validate_empty_constituents() {
        let mut basket = make_test_ntd_basket();
        basket.constituents = vec![];
        assert!(basket.validate().is_err());
    }

    #[test]
    fn test_ntd_basket_validate_n_too_large() {
        let mut basket = make_test_ntd_basket();
        basket.nth_to_default = 10; // More than 5 constituents
        assert!(basket.validate().is_err());
    }

    #[test]
    fn test_ntd_basket_validate_invalid_correlation() {
        let mut basket = make_test_ntd_basket();
        basket.correlation_parameter = Some(1.5); // Invalid: > 1
        assert!(basket.validate().is_err());
    }

    #[test]
    fn test_credit_event_equality() {
        assert_eq!(CreditEvent::Bankruptcy, CreditEvent::Bankruptcy);
        assert_ne!(CreditEvent::Bankruptcy, CreditEvent::FailureToPay);
    }

    #[test]
    fn test_credit_event_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(CreditEvent::Bankruptcy);
        set.insert(CreditEvent::FailureToPay);
        set.insert(CreditEvent::Bankruptcy); // Duplicate
        assert_eq!(set.len(), 2);
    }
}
