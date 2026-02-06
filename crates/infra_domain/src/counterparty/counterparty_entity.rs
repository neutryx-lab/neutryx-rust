//! CounterParty entity and related types.
//!
//! This module defines the CounterParty entity representing a trading
//! counterparty with credit information.
//!
//! Uses `bon::Builder` for fluent construction with compile-time safety.

#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use bon::Builder;

use super::{CounterPartyId, CreditParams, CreditRating, LegalEntityId};

// ============================================================================
// CounterPartySector
// ============================================================================

/// CounterParty sector classification.
///
/// Classifies counterparties by their business sector for risk
/// aggregation and reporting purposes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CounterPartySector {
    /// Commercial or investment bank
    Banking,
    /// Investment firm
    Investment,
    /// Securities broker/dealer
    Securities,
    /// Insurance company
    Insurance,
    /// Proprietary trading firm
    Trading,
    /// Asset management company
    AssetManagement,
    /// Hedge fund
    HedgeFund,
    /// Non-financial corporate
    Corporate,
    /// Sovereign or government entity
    Sovereign,
    /// Other/unclassified
    #[default]
    Other,
}

impl std::fmt::Display for CounterPartySector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CounterPartySector::Banking => "Banking",
            CounterPartySector::Investment => "Investment",
            CounterPartySector::Securities => "Securities",
            CounterPartySector::Insurance => "Insurance",
            CounterPartySector::Trading => "Trading",
            CounterPartySector::AssetManagement => "Asset Management",
            CounterPartySector::HedgeFund => "Hedge Fund",
            CounterPartySector::Corporate => "Corporate",
            CounterPartySector::Sovereign => "Sovereign",
            CounterPartySector::Other => "Other",
        };
        write!(f, "{}", s)
    }
}

// ============================================================================
// CounterParty
// ============================================================================

/// CounterParty entity with credit parameters.
///
/// Represents a trading counterparty with identification information,
/// sector classification, and optional credit parameters.
///
/// Uses `bon::Builder` for fluent construction with compile-time safety.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::{CounterParty, CounterPartySector, CreditRating};
///
/// let cp = CounterParty::builder()
///     .counterparty_id("CP001")
///     .name("Acme Bank")
///     .sector(CounterPartySector::Banking)
///     .country("US")
///     .rating(CreditRating::APlus)
///     .build();
///
/// assert_eq!(cp.name(), "Acme Bank");
/// assert_eq!(cp.sector(), CounterPartySector::Banking);
/// ```
#[derive(Clone, Debug, Builder)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CounterParty {
    /// Unique identifier for this counterparty.
    #[builder(into)]
    counterparty_id: CounterPartyId,
    /// Human-readable name.
    #[builder(into)]
    name: String,
    /// Legal Entity Identifier (LEI).
    lei: Option<LegalEntityId>,
    /// Sector classification.
    #[builder(default)]
    sector: CounterPartySector,
    /// ISO 3166-1 alpha-2 country code.
    #[builder(into)]
    country: Option<String>,
    /// Credit rating.
    rating: Option<CreditRating>,
    /// Credit parameters (hazard rate, recovery rate).
    credit_params: Option<CreditParams>,
}

impl CounterParty {
    /// Returns the counterparty ID.
    pub fn id(&self) -> &CounterPartyId { &self.counterparty_id }

    /// Returns the counterparty name.
    pub fn name(&self) -> &str { &self.name }

    /// Returns the LEI if set.
    pub fn lei(&self) -> Option<&LegalEntityId> { self.lei.as_ref() }

    /// Returns the sector.
    pub fn sector(&self) -> CounterPartySector { self.sector }

    /// Returns the country code if set.
    pub fn country(&self) -> Option<&str> { self.country.as_deref() }

    /// Returns the credit rating if set.
    pub fn rating(&self) -> Option<CreditRating> { self.rating }

    /// Returns the credit parameters if set.
    pub fn credit_params(&self) -> Option<&CreditParams> { self.credit_params.as_ref() }

    /// Returns whether this counterparty is investment grade.
    ///
    /// Returns `None` if no rating is set.
    pub fn is_investment_grade(&self) -> Option<bool> {
        self.rating.map(|r| r.is_investment_grade())
    }

    /// Returns the hazard rate from credit params, or from rating's indicative
    /// rate.
    pub fn hazard_rate(&self) -> Option<f64> {
        self.credit_params
            .as_ref()
            .map(|p| p.hazard_rate())
            .or_else(|| self.rating.map(|r| r.indicative_hazard_rate()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counterparty_sector_default() {
        assert_eq!(CounterPartySector::default(), CounterPartySector::Other);
    }

    #[test]
    fn test_counterparty_sector_display() {
        assert_eq!(format!("{}", CounterPartySector::Banking), "Banking");
        assert_eq!(
            format!("{}", CounterPartySector::AssetManagement),
            "Asset Management"
        );
        assert_eq!(format!("{}", CounterPartySector::HedgeFund), "Hedge Fund");
    }

    #[test]
    fn test_counterparty_builder_minimal() {
        let cp = CounterParty::builder()
            .counterparty_id("CP001")
            .name("Test Bank")
            .build();

        assert_eq!(cp.id().as_str(), "CP001");
        assert_eq!(cp.name(), "Test Bank");
        assert!(cp.lei().is_none());
        assert_eq!(cp.sector(), CounterPartySector::Other);
        assert!(cp.country().is_none());
        assert!(cp.rating().is_none());
        assert!(cp.credit_params().is_none());
    }

    #[test]
    fn test_counterparty_builder_full() {
        let lei = LegalEntityId::new_unchecked("529900T8BM49AURSDO55");
        let params = CreditParams::from_rating(CreditRating::APlus, 0.4).unwrap();

        let cp = CounterParty::builder()
            .counterparty_id("CP001")
            .name("Acme Bank")
            .lei(lei)
            .sector(CounterPartySector::Banking)
            .country("US")
            .rating(CreditRating::APlus)
            .credit_params(params)
            .build();

        assert_eq!(cp.id().as_str(), "CP001");
        assert_eq!(cp.name(), "Acme Bank");
        assert!(cp.lei().is_some());
        assert_eq!(cp.lei().unwrap().as_str(), "529900T8BM49AURSDO55");
        assert_eq!(cp.sector(), CounterPartySector::Banking);
        assert_eq!(cp.country(), Some("US"));
        assert_eq!(cp.rating(), Some(CreditRating::APlus));
        assert!(cp.credit_params().is_some());
    }

    #[test]
    fn test_counterparty_is_investment_grade() {
        let cp_ig = CounterParty::builder()
            .counterparty_id("CP001")
            .name("IG Bank")
            .rating(CreditRating::APlus)
            .build();
        assert_eq!(cp_ig.is_investment_grade(), Some(true));

        let cp_hy = CounterParty::builder()
            .counterparty_id("CP002")
            .name("HY Corp")
            .rating(CreditRating::Bb)
            .build();
        assert_eq!(cp_hy.is_investment_grade(), Some(false));

        let cp_none = CounterParty::builder()
            .counterparty_id("CP003")
            .name("Unrated Corp")
            .build();
        assert_eq!(cp_none.is_investment_grade(), None);
    }

    #[test]
    fn test_counterparty_hazard_rate_from_params() {
        let params = CreditParams::new(0.02, 0.4).unwrap();
        let cp = CounterParty::builder()
            .counterparty_id("CP001")
            .name("Test")
            .credit_params(params)
            .build();

        assert!((cp.hazard_rate().unwrap() - 0.02).abs() < f64::EPSILON);
    }

    #[test]
    fn test_counterparty_hazard_rate_from_rating() {
        let cp = CounterParty::builder()
            .counterparty_id("CP001")
            .name("Test")
            .rating(CreditRating::Bbb)
            .build();

        assert!(
            (cp.hazard_rate().unwrap() - CreditRating::Bbb.indicative_hazard_rate()).abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn test_counterparty_hazard_rate_params_takes_precedence() {
        // When both params and rating are set, params should take precedence
        let params = CreditParams::new(0.05, 0.4).unwrap();
        let cp = CounterParty::builder()
            .counterparty_id("CP001")
            .name("Test")
            .rating(CreditRating::Aaa) // Would give ~0.0001
            .credit_params(params) // But we override with 0.05
            .build();

        assert!((cp.hazard_rate().unwrap() - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn test_counterparty_hazard_rate_none() {
        let cp = CounterParty::builder()
            .counterparty_id("CP001")
            .name("Test")
            .build();
        assert!(cp.hazard_rate().is_none());
    }
}
