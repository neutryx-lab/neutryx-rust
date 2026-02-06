//! Type-safe ID types for counterparty module.
//!
//! This module provides newtype wrappers for various identifiers used
//! in the counterparty domain, ensuring type safety at compile time.

#![allow(clippy::must_use_candidate)]

use std::fmt;

use derive_more::{AsRef, Display, From};

use super::CounterPartyError;

// ============================================================================
// CounterPartyId
// ============================================================================

/// Type-safe CounterParty identifier.
///
/// Wraps a string identifier for counterparties, providing type safety
/// to prevent accidental mixing with other ID types.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::CounterPartyId;
///
/// let id = CounterPartyId::new("CP001");
/// assert_eq!(id.as_str(), "CP001");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Display, From, AsRef)]
#[as_ref(str)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct CounterPartyId(String);

impl CounterPartyId {
    /// Creates a new CounterParty ID.
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }

    /// Returns the ID as a string slice.
    pub fn as_str(&self) -> &str { &self.0 }
}

impl From<&str> for CounterPartyId {
    fn from(s: &str) -> Self { Self(s.to_string()) }
}

// ============================================================================
// LegalEntityId
// ============================================================================

/// Legal Entity Identifier (LEI) per ISO 17442.
///
/// The LEI is a 20-character alphanumeric code that uniquely identifies
/// legally distinct entities participating in financial transactions.
///
/// # Validation
///
/// The [`new`](LegalEntityId::new) constructor validates that the LEI:
/// - Is exactly 20 characters long
/// - Contains only ASCII alphanumeric characters
///
/// Use [`new_unchecked`](LegalEntityId::new_unchecked) for trusted sources
/// where validation has already been performed.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::LegalEntityId;
///
/// // Valid LEI (20 alphanumeric characters)
/// let lei = LegalEntityId::new("529900T8BM49AURSDO55").unwrap();
/// assert_eq!(lei.as_str(), "529900T8BM49AURSDO55");
///
/// // Invalid LEI (too short)
/// assert!(LegalEntityId::new("ABC").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct LegalEntityId(String);

impl LegalEntityId {
    /// Creates a new LEI with validation.
    ///
    /// # Errors
    ///
    /// Returns [`CounterPartyError::InvalidLei`] if the LEI is not exactly
    /// 20 alphanumeric characters.
    pub fn new(lei: impl Into<String>) -> Result<Self, CounterPartyError> {
        let lei = lei.into();
        if lei.len() != 20 || !lei.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(CounterPartyError::InvalidLei(lei));
        }
        Ok(Self(lei))
    }

    /// Creates LEI without validation (for trusted sources).
    ///
    /// # Safety
    ///
    /// This method does not validate the LEI format. Use only when the
    /// LEI has already been validated or comes from a trusted source.
    pub fn new_unchecked(lei: impl Into<String>) -> Self { Self(lei.into()) }

    /// Returns the LEI as a string slice.
    pub fn as_str(&self) -> &str { &self.0 }
}

impl fmt::Display for LegalEntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
}

impl AsRef<str> for LegalEntityId {
    fn as_ref(&self) -> &str { &self.0 }
}

// ============================================================================
// NettingSetId
// ============================================================================

/// Type-safe NettingSet identifier.
///
/// Wraps a string identifier for netting sets, providing type safety
/// to prevent accidental mixing with other ID types.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::NettingSetId;
///
/// let id = NettingSetId::new("NS001");
/// assert_eq!(id.as_str(), "NS001");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Display, From, AsRef)]
#[as_ref(str)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct NettingSetId(String);

impl NettingSetId {
    /// Creates a new NettingSet ID.
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }

    /// Returns the ID as a string slice.
    pub fn as_str(&self) -> &str { &self.0 }
}

impl From<&str> for NettingSetId {
    fn from(s: &str) -> Self { Self(s.to_string()) }
}

// ============================================================================
// CcpId
// ============================================================================

/// Type-safe CCP (Central Counterparty Clearing House) identifier.
///
/// Wraps a string identifier for CCPs, providing type safety
/// to prevent accidental mixing with other ID types.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::CcpId;
///
/// let id = CcpId::new("LCH");
/// assert_eq!(id.as_str(), "LCH");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Display, From, AsRef)]
#[as_ref(str)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct CcpId(String);

impl CcpId {
    /// Creates a new CCP ID.
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }

    /// Returns the ID as a string slice.
    pub fn as_str(&self) -> &str { &self.0 }
}

impl From<&str> for CcpId {
    fn from(s: &str) -> Self { Self(s.to_string()) }
}

// ============================================================================
// IsdaAgreementId
// ============================================================================

/// Type-safe ISDA Master Agreement identifier.
///
/// Wraps a string identifier for ISDA agreements, providing type safety
/// to prevent accidental mixing with other ID types.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::IsdaAgreementId;
///
/// let id = IsdaAgreementId::new("ISDA001");
/// assert_eq!(id.as_str(), "ISDA001");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Display, From, AsRef)]
#[as_ref(str)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct IsdaAgreementId(String);

impl IsdaAgreementId {
    /// Creates a new ISDA Agreement ID.
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }

    /// Returns the ID as a string slice.
    pub fn as_str(&self) -> &str { &self.0 }
}

impl From<&str> for IsdaAgreementId {
    fn from(s: &str) -> Self { Self(s.to_string()) }
}

// ============================================================================
// VariationMarginAgreementId
// ============================================================================

/// Type-safe Variation Margin Agreement identifier.
///
/// Wraps a string identifier for VM agreements (CSA contracts), providing
/// type safety to prevent accidental mixing with other ID types.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::VariationMarginAgreementId;
///
/// let id = VariationMarginAgreementId::new("VMA001");
/// assert_eq!(id.as_str(), "VMA001");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Display, From, AsRef)]
#[as_ref(str)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct VariationMarginAgreementId(String);

impl VariationMarginAgreementId {
    /// Creates a new Variation Margin Agreement ID.
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }

    /// Returns the ID as a string slice.
    pub fn as_str(&self) -> &str { &self.0 }
}

impl From<&str> for VariationMarginAgreementId {
    fn from(s: &str) -> Self { Self(s.to_string()) }
}

// ============================================================================
// CrossBookNettingAgreementId
// ============================================================================

/// Type-safe Cross-Book Netting Agreement identifier.
///
/// Wraps a string identifier for cross-book netting agreements, providing
/// type safety to prevent accidental mixing with other ID types.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::CrossBookNettingAgreementId;
///
/// let id = CrossBookNettingAgreementId::new("CBNA001");
/// assert_eq!(id.as_str(), "CBNA001");
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Display, From, AsRef)]
#[as_ref(str)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct CrossBookNettingAgreementId(String);

impl CrossBookNettingAgreementId {
    /// Creates a new Cross-Book Netting Agreement ID.
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }

    /// Returns the ID as a string slice.
    pub fn as_str(&self) -> &str { &self.0 }
}

impl From<&str> for CrossBookNettingAgreementId {
    fn from(s: &str) -> Self { Self(s.to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // CounterPartyId tests
    // ========================================================================

    #[test]
    fn test_counterparty_id_new() {
        let id = CounterPartyId::new("CP001");
        assert_eq!(id.as_str(), "CP001");
    }

    #[test]
    fn test_counterparty_id_from_string() {
        let id: CounterPartyId = "CP002".to_string().into();
        assert_eq!(id.as_str(), "CP002");
    }

    #[test]
    fn test_counterparty_id_from_str() {
        let id: CounterPartyId = "CP003".into();
        assert_eq!(id.as_str(), "CP003");
    }

    #[test]
    fn test_counterparty_id_display() {
        let id = CounterPartyId::new("CP001");
        assert_eq!(format!("{}", id), "CP001");
    }

    #[test]
    fn test_counterparty_id_as_ref() {
        let id = CounterPartyId::new("CP001");
        let s: &str = id.as_ref();
        assert_eq!(s, "CP001");
    }

    #[test]
    fn test_counterparty_id_equality() {
        let id1 = CounterPartyId::new("CP001");
        let id2 = CounterPartyId::new("CP001");
        let id3 = CounterPartyId::new("CP002");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_counterparty_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(CounterPartyId::new("CP001"));
        assert!(set.contains(&CounterPartyId::new("CP001")));
        assert!(!set.contains(&CounterPartyId::new("CP002")));
    }

    // ========================================================================
    // LegalEntityId tests
    // ========================================================================

    #[test]
    fn test_legal_entity_id_valid() {
        // Valid 20-character alphanumeric LEI
        let lei = LegalEntityId::new("529900T8BM49AURSDO55").unwrap();
        assert_eq!(lei.as_str(), "529900T8BM49AURSDO55");
    }

    #[test]
    fn test_legal_entity_id_invalid_length_short() {
        let result = LegalEntityId::new("ABC");
        assert!(result.is_err());
        match result {
            Err(CounterPartyError::InvalidLei(s)) => assert_eq!(s, "ABC"),
            _ => panic!("Expected InvalidLei error"),
        }
    }

    #[test]
    fn test_legal_entity_id_invalid_length_long() {
        let result = LegalEntityId::new("529900T8BM49AURSDO55X");
        assert!(result.is_err());
    }

    #[test]
    fn test_legal_entity_id_invalid_characters() {
        // Contains special character
        let result = LegalEntityId::new("529900T8BM49AURSD-55");
        assert!(result.is_err());
    }

    #[test]
    fn test_legal_entity_id_new_unchecked() {
        // This should not validate
        let lei = LegalEntityId::new_unchecked("INVALID");
        assert_eq!(lei.as_str(), "INVALID");
    }

    #[test]
    fn test_legal_entity_id_display() {
        let lei = LegalEntityId::new_unchecked("529900T8BM49AURSDO55");
        assert_eq!(format!("{}", lei), "529900T8BM49AURSDO55");
    }

    // ========================================================================
    // NettingSetId tests
    // ========================================================================

    #[test]
    fn test_netting_set_id_new() {
        let id = NettingSetId::new("NS001");
        assert_eq!(id.as_str(), "NS001");
    }

    #[test]
    fn test_netting_set_id_from_string() {
        let id: NettingSetId = "NS002".to_string().into();
        assert_eq!(id.as_str(), "NS002");
    }

    #[test]
    fn test_netting_set_id_display() {
        let id = NettingSetId::new("NS001");
        assert_eq!(format!("{}", id), "NS001");
    }

    #[test]
    fn test_netting_set_id_equality() {
        let id1 = NettingSetId::new("NS001");
        let id2 = NettingSetId::new("NS001");
        let id3 = NettingSetId::new("NS002");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    // ========================================================================
    // CcpId tests
    // ========================================================================

    #[test]
    fn test_ccp_id_new() {
        let id = CcpId::new("LCH");
        assert_eq!(id.as_str(), "LCH");
    }

    #[test]
    fn test_ccp_id_from_string() {
        let id: CcpId = "CME".to_string().into();
        assert_eq!(id.as_str(), "CME");
    }

    #[test]
    fn test_ccp_id_from_str() {
        let id: CcpId = "JSCC".into();
        assert_eq!(id.as_str(), "JSCC");
    }

    #[test]
    fn test_ccp_id_display() {
        let id = CcpId::new("LCH");
        assert_eq!(format!("{}", id), "LCH");
    }

    #[test]
    fn test_ccp_id_equality() {
        let id1 = CcpId::new("LCH");
        let id2 = CcpId::new("LCH");
        let id3 = CcpId::new("CME");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    // ========================================================================
    // IsdaAgreementId tests
    // ========================================================================

    #[test]
    fn test_isda_agreement_id_new() {
        let id = IsdaAgreementId::new("ISDA001");
        assert_eq!(id.as_str(), "ISDA001");
    }

    #[test]
    fn test_isda_agreement_id_from_string() {
        let id: IsdaAgreementId = "ISDA002".to_string().into();
        assert_eq!(id.as_str(), "ISDA002");
    }

    #[test]
    fn test_isda_agreement_id_from_str() {
        let id: IsdaAgreementId = "ISDA003".into();
        assert_eq!(id.as_str(), "ISDA003");
    }

    #[test]
    fn test_isda_agreement_id_display() {
        let id = IsdaAgreementId::new("ISDA001");
        assert_eq!(format!("{}", id), "ISDA001");
    }

    #[test]
    fn test_isda_agreement_id_equality() {
        let id1 = IsdaAgreementId::new("ISDA001");
        let id2 = IsdaAgreementId::new("ISDA001");
        let id3 = IsdaAgreementId::new("ISDA002");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_isda_agreement_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(IsdaAgreementId::new("ISDA001"));
        assert!(set.contains(&IsdaAgreementId::new("ISDA001")));
        assert!(!set.contains(&IsdaAgreementId::new("ISDA002")));
    }

    #[test]
    fn test_isda_agreement_id_default() {
        let id = IsdaAgreementId::default();
        assert_eq!(id.as_str(), "");
    }

    // ========================================================================
    // VariationMarginAgreementId tests
    // ========================================================================

    #[test]
    fn test_variation_margin_agreement_id_new() {
        let id = VariationMarginAgreementId::new("VMA001");
        assert_eq!(id.as_str(), "VMA001");
    }

    #[test]
    fn test_variation_margin_agreement_id_from_string() {
        let id: VariationMarginAgreementId = "VMA002".to_string().into();
        assert_eq!(id.as_str(), "VMA002");
    }

    #[test]
    fn test_variation_margin_agreement_id_from_str() {
        let id: VariationMarginAgreementId = "VMA003".into();
        assert_eq!(id.as_str(), "VMA003");
    }

    #[test]
    fn test_variation_margin_agreement_id_display() {
        let id = VariationMarginAgreementId::new("VMA001");
        assert_eq!(format!("{}", id), "VMA001");
    }

    #[test]
    fn test_variation_margin_agreement_id_equality() {
        let id1 = VariationMarginAgreementId::new("VMA001");
        let id2 = VariationMarginAgreementId::new("VMA001");
        let id3 = VariationMarginAgreementId::new("VMA002");
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_variation_margin_agreement_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(VariationMarginAgreementId::new("VMA001"));
        assert!(set.contains(&VariationMarginAgreementId::new("VMA001")));
        assert!(!set.contains(&VariationMarginAgreementId::new("VMA002")));
    }

    #[test]
    fn test_variation_margin_agreement_id_default() {
        let id = VariationMarginAgreementId::default();
        assert_eq!(id.as_str(), "");
    }

    // ========================================================================
    // CrossBookNettingAgreementId tests
    // ========================================================================

    #[test]
    fn test_cross_book_netting_agreement_id_new() {
        let id = CrossBookNettingAgreementId::new("CBNA001");
        assert_eq!(id.as_str(), "CBNA001");
    }

    #[test]
    fn test_cross_book_netting_agreement_id_from_string() {
        let id: CrossBookNettingAgreementId = "CBNA002".to_string().into();
        assert_eq!(id.as_str(), "CBNA002");
    }

    #[test]
    fn test_cross_book_netting_agreement_id_from_str() {
        let id: CrossBookNettingAgreementId = "CBNA003".into();
        assert_eq!(id.as_str(), "CBNA003");
    }

    #[test]
    fn test_cross_book_netting_agreement_id_display() {
        let id = CrossBookNettingAgreementId::new("CBNA001");
        assert_eq!(format!("{}", id), "CBNA001");
    }
}
