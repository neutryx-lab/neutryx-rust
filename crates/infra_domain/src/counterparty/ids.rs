//! Type-safe ID types for counterparty module.

#![allow(clippy::must_use_candidate)]

use super::CounterPartyError;

define_id! {
    /// Type-safe CounterParty identifier.
    CounterPartyId
}

/// Legal Entity Identifier (LEI) per ISO 17442.
#[derive(Clone, Debug, PartialEq, Eq, Hash, derive_more::Display, derive_more::AsRef)]
#[as_ref(str)]
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct LegalEntityId(String);

impl LegalEntityId {
    /// Creates a new LEI with validation.
    pub fn new(lei: impl Into<String>) -> Result<Self, CounterPartyError> {
        let lei = lei.into();
        if lei.len() != 20 || !lei.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(CounterPartyError::InvalidLei(lei));
        }
        Ok(Self(lei))
    }

    /// Creates LEI without validation — use only for already-validated inputs.
    pub fn new_unchecked(lei: impl Into<String>) -> Self { Self(lei.into()) }

    /// Returns the LEI as a string slice.
    pub fn as_str(&self) -> &str { &self.0 }
}

define_id! {
    /// Type-safe NettingSet identifier.
    NettingSetId
}

define_id! {
    /// Type-safe CCP (Central Counterparty Clearing House) identifier.
    CcpId
}

define_id! {
    /// Type-safe ISDA Master Agreement identifier.
    IsdaAgreementId
}

define_id! {
    /// Type-safe Variation Margin Agreement identifier.
    VariationMarginAgreementId
}

define_id! {
    /// Type-safe Cross-Book Netting Agreement identifier.
    CrossBookNettingAgreementId
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counterparty_id() {
        let id = CounterPartyId::new("CP001");
        assert_eq!(id.as_str(), "CP001");
        assert_eq!(format!("{}", id), "CP001");
        let id2: CounterPartyId = "CP001".into();
        assert_eq!(id, id2);
    }

    #[test]
    fn test_legal_entity_id_validation() {
        let lei = LegalEntityId::new("529900T8BM49AURSDO55").unwrap();
        assert_eq!(lei.as_str(), "529900T8BM49AURSDO55");
        assert!(LegalEntityId::new("ABC").is_err());
        assert!(LegalEntityId::new("529900T8BM49AURSD-55").is_err());
        assert_eq!(LegalEntityId::new_unchecked("INVALID").as_str(), "INVALID");
    }

    #[test]
    fn test_netting_set_id() {
        let id = NettingSetId::new("NS001");
        assert_eq!(id.as_str(), "NS001");
        let id2: NettingSetId = "NS001".to_string().into();
        assert_eq!(id, id2);
    }

    #[test]
    fn test_ccp_id() {
        let id = CcpId::new("LCH");
        assert_eq!(id.as_str(), "LCH");
        let id2: CcpId = "LCH".into();
        assert_eq!(id, id2);
    }

    #[test]
    fn test_isda_agreement_id() {
        let id = IsdaAgreementId::new("ISDA001");
        assert_eq!(id.as_str(), "ISDA001");
    }

    #[test]
    fn test_variation_margin_agreement_id() {
        let id = VariationMarginAgreementId::new("VMA001");
        assert_eq!(id.as_str(), "VMA001");
    }

    #[test]
    fn test_cross_book_netting_agreement_id() {
        let id = CrossBookNettingAgreementId::new("CBNA001");
        assert_eq!(id.as_str(), "CBNA001");
    }
}
