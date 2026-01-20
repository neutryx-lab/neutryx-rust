//! Counterparty and CSA (Credit Support Annex) master data.
//!
//! This module defines types for managing counterparty relationships and
//! collateral agreements. These are static master data types that define
//! the contractual terms governing derivative trades.

#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use crate::Currency;

/// Credit Support Annex terms.
///
/// Defines the collateral agreement between counterparties. CSA terms govern
/// how margin is exchanged to mitigate counterparty credit risk.
///
/// # Fields
///
/// * `csa_id` - Unique identifier for this CSA agreement
/// * `threshold` - Exposure below which no collateral is required (in base
///   currency)
/// * `minimum_transfer_amount` - Minimum amount for margin calls
/// * `independent_amount` - Initial margin amount (also known as initial
///   amount)
/// * `collateral_currency` - Currency for collateral (type-safe ISO 4217)
/// * `margin_period_of_risk` - Risk period in days (typically 10 for cleared,
///   14+ for bilateral)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CsaTerms {
    /// CSA identifier
    pub csa_id: String,
    /// Threshold amount (exposure below which no collateral is required)
    pub threshold: f64,
    /// Minimum transfer amount
    pub minimum_transfer_amount: f64,
    /// Independent amount (initial margin)
    pub independent_amount: f64,
    /// Collateral currency (type-safe)
    pub collateral_currency: Currency,
    /// Margin period of risk (in days)
    pub margin_period_of_risk: u32,
}

impl Default for CsaTerms {
    fn default() -> Self {
        Self {
            csa_id: String::new(),
            threshold: 0.0,
            minimum_transfer_amount: 0.0,
            independent_amount: 0.0,
            collateral_currency: Currency::USD,
            margin_period_of_risk: 10,
        }
    }
}

impl CsaTerms {
    /// Create a new CSA terms instance.
    pub fn new(csa_id: impl Into<String>) -> Self {
        Self {
            csa_id: csa_id.into(),
            ..Default::default()
        }
    }

    /// Set threshold amount.
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    /// Set minimum transfer amount.
    pub fn with_mta(mut self, mta: f64) -> Self {
        self.minimum_transfer_amount = mta;
        self
    }

    /// Set independent amount.
    pub fn with_independent_amount(mut self, ia: f64) -> Self {
        self.independent_amount = ia;
        self
    }

    /// Set collateral currency.
    pub fn with_currency(mut self, currency: Currency) -> Self {
        self.collateral_currency = currency;
        self
    }

    /// Set margin period of risk.
    pub fn with_mpor(mut self, days: u32) -> Self {
        self.margin_period_of_risk = days;
        self
    }
}

/// Netting set configuration.
///
/// Defines how trades are grouped for netting purposes. A netting set is a
/// collection of trades with a single counterparty that can be legally netted
/// in the event of default.
///
/// # Fields
///
/// * `netting_set_id` - Unique identifier for this netting set
/// * `counterparty_id` - Identifier of the counterparty
/// * `csa_terms` - Optional CSA terms governing collateral exchange
/// * `closeout_netting` - Whether close-out netting applies (typically true for
///   ISDA agreements)
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NettingSetConfig {
    /// Netting set identifier
    pub netting_set_id: String,
    /// Counterparty identifier
    pub counterparty_id: String,
    /// Associated CSA terms (if any)
    pub csa_terms: Option<CsaTerms>,
    /// Whether close-out netting applies
    pub closeout_netting: bool,
}

impl NettingSetConfig {
    /// Create a new netting set configuration.
    pub fn new(netting_set_id: impl Into<String>, counterparty_id: impl Into<String>) -> Self {
        Self {
            netting_set_id: netting_set_id.into(),
            counterparty_id: counterparty_id.into(),
            csa_terms: None,
            closeout_netting: true,
        }
    }

    /// Set CSA terms for this netting set.
    pub fn with_csa(mut self, csa: CsaTerms) -> Self {
        self.csa_terms = Some(csa);
        self
    }

    /// Set close-out netting flag.
    pub fn with_closeout_netting(mut self, enabled: bool) -> Self {
        self.closeout_netting = enabled;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csa_terms_default() {
        let csa = CsaTerms::default();
        assert_eq!(csa.csa_id, "");
        assert_eq!(csa.threshold, 0.0);
        assert_eq!(csa.collateral_currency, Currency::USD);
        assert_eq!(csa.margin_period_of_risk, 10);
    }

    #[test]
    fn test_csa_terms_builder() {
        let csa = CsaTerms::new("CSA001")
            .with_threshold(1_000_000.0)
            .with_mta(50_000.0)
            .with_independent_amount(100_000.0)
            .with_currency(Currency::EUR)
            .with_mpor(14);

        assert_eq!(csa.csa_id, "CSA001");
        assert_eq!(csa.threshold, 1_000_000.0);
        assert_eq!(csa.minimum_transfer_amount, 50_000.0);
        assert_eq!(csa.independent_amount, 100_000.0);
        assert_eq!(csa.collateral_currency, Currency::EUR);
        assert_eq!(csa.margin_period_of_risk, 14);
    }

    #[test]
    fn test_netting_set_config() {
        let config = NettingSetConfig::new("NS001", "CP001");
        assert_eq!(config.netting_set_id, "NS001");
        assert_eq!(config.counterparty_id, "CP001");
        assert!(config.closeout_netting);
        assert!(config.csa_terms.is_none());
    }

    #[test]
    fn test_netting_set_with_csa() {
        let csa = CsaTerms::new("CSA001").with_threshold(500_000.0);
        let config = NettingSetConfig::new("NS001", "CP001").with_csa(csa);

        assert!(config.csa_terms.is_some());
        assert_eq!(config.csa_terms.unwrap().threshold, 500_000.0);
    }
}
