//! CCP (Central Counterparty Clearing House) entity.
//!
//! This module defines the CCP entity representing a central clearing house
//! for cleared derivative transactions.

#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use super::CcpId;

/// CCP (Central Counterparty Clearing House) entity.
///
/// Represents a central clearing house through which cleared derivatives
/// are settled. CCPs have special margin requirements and risk treatment.
///
/// # Examples
///
/// ```
/// use infra_master::counterparty::Ccp;
///
/// let ccp = Ccp::new("LCH", "LCH Ltd", true)
///     .with_country("GB");
///
/// assert!(ccp.is_qualifying());
/// assert_eq!(Ccp::CLEARED_MPOR_DAYS, 5);
/// ```
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::struct_field_names)]
pub struct Ccp {
    ccp_id: CcpId,
    name: String,
    country: Option<String>,
    qualifying: bool,
}

impl Ccp {
    /// Default cleared MPOR (Margin Period of Risk) in business days.
    ///
    /// Under SA-CCR, the MPOR for cleared transactions is typically 5 business
    /// days, compared to 10+ days for bilateral transactions.
    pub const CLEARED_MPOR_DAYS: u32 = 5;

    /// Creates a new CCP.
    ///
    /// # Arguments
    ///
    /// * `id` - CCP identifier
    /// * `name` - CCP name
    /// * `qualifying` - Whether this is a qualifying CCP for SA-CCR purposes
    pub fn new(id: impl Into<CcpId>, name: impl Into<String>, qualifying: bool) -> Self {
        Self {
            ccp_id: id.into(),
            name: name.into(),
            country: None,
            qualifying,
        }
    }

    /// Sets the country code.
    pub fn with_country(mut self, country: impl Into<String>) -> Self {
        self.country = Some(country.into());
        self
    }

    /// Returns the CCP ID.
    pub fn id(&self) -> &CcpId { &self.ccp_id }

    /// Returns the CCP name.
    pub fn name(&self) -> &str { &self.name }

    /// Returns the country code if set.
    pub fn country(&self) -> Option<&str> { self.country.as_deref() }

    /// Returns whether this is a qualifying CCP.
    ///
    /// Qualifying CCPs (QCCPs) receive preferential capital treatment under
    /// Basel III SA-CCR rules. A CCP must meet certain requirements to be
    /// considered qualifying.
    pub fn is_qualifying(&self) -> bool { self.qualifying }

    /// Returns the default MPOR for this CCP.
    ///
    /// Currently returns the standard 5 business days for all CCPs.
    pub fn default_mpor_days(&self) -> u32 { Self::CLEARED_MPOR_DAYS }
}

/// Well-known CCPs for convenience.
impl Ccp {
    /// LCH Ltd (London Clearing House) - SwapClear
    pub fn lch() -> Self { Self::new("LCH", "LCH Ltd", true).with_country("GB") }

    /// CME Clearing - Interest Rate Swaps
    pub fn cme() -> Self { Self::new("CME", "CME Clearing", true).with_country("US") }

    /// JSCC (Japan Securities Clearing Corporation)
    pub fn jscc() -> Self {
        Self::new("JSCC", "Japan Securities Clearing Corporation", true).with_country("JP")
    }

    /// Eurex Clearing
    pub fn eurex() -> Self { Self::new("EUREX", "Eurex Clearing AG", true).with_country("DE") }

    /// ICE Clear Credit
    pub fn ice_credit() -> Self {
        Self::new("ICE_CREDIT", "ICE Clear Credit", true).with_country("US")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ccp_new() {
        let ccp = Ccp::new("LCH", "LCH Ltd", true);
        assert_eq!(ccp.id().as_str(), "LCH");
        assert_eq!(ccp.name(), "LCH Ltd");
        assert!(ccp.is_qualifying());
        assert!(ccp.country().is_none());
    }

    #[test]
    fn test_ccp_with_country() {
        let ccp = Ccp::new("LCH", "LCH Ltd", true).with_country("GB");
        assert_eq!(ccp.country(), Some("GB"));
    }

    #[test]
    fn test_ccp_non_qualifying() {
        let ccp = Ccp::new("SMALL_CCP", "Small CCP Inc", false);
        assert!(!ccp.is_qualifying());
    }

    #[test]
    fn test_ccp_cleared_mpor_constant() {
        assert_eq!(Ccp::CLEARED_MPOR_DAYS, 5);
    }

    #[test]
    fn test_ccp_default_mpor() {
        let ccp = Ccp::new("TEST", "Test CCP", true);
        assert_eq!(ccp.default_mpor_days(), 5);
    }

    #[test]
    fn test_ccp_lch() {
        let ccp = Ccp::lch();
        assert_eq!(ccp.id().as_str(), "LCH");
        assert!(ccp.is_qualifying());
        assert_eq!(ccp.country(), Some("GB"));
    }

    #[test]
    fn test_ccp_cme() {
        let ccp = Ccp::cme();
        assert_eq!(ccp.id().as_str(), "CME");
        assert!(ccp.is_qualifying());
        assert_eq!(ccp.country(), Some("US"));
    }

    #[test]
    fn test_ccp_jscc() {
        let ccp = Ccp::jscc();
        assert_eq!(ccp.id().as_str(), "JSCC");
        assert!(ccp.is_qualifying());
        assert_eq!(ccp.country(), Some("JP"));
    }

    #[test]
    fn test_ccp_eurex() {
        let ccp = Ccp::eurex();
        assert_eq!(ccp.id().as_str(), "EUREX");
        assert!(ccp.is_qualifying());
        assert_eq!(ccp.country(), Some("DE"));
    }

    #[test]
    fn test_ccp_ice_credit() {
        let ccp = Ccp::ice_credit();
        assert_eq!(ccp.id().as_str(), "ICE_CREDIT");
        assert!(ccp.is_qualifying());
    }
}
