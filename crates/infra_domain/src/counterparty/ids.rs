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
/// use infra_domain::counterparty::CounterPartyId;
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
/// use infra_domain::counterparty::LegalEntityId;
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

    /// Creates LEI without validation — use only for already-validated inputs.
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
/// use infra_domain::counterparty::NettingSetId;
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
/// use infra_domain::counterparty::CcpId;
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
/// use infra_domain::counterparty::IsdaAgreementId;
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
/// use infra_domain::counterparty::VariationMarginAgreementId;
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
/// use infra_domain::counterparty::CrossBookNettingAgreementId;
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
