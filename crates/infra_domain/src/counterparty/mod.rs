//! CounterParty and NettingSet management module.
//!
//! This module provides types for managing counterparty relationships,
//! netting sets, CSA (Credit Support Annex) terms, and credit parameters.
//!
//! # Module Structure
//!
//! - `error`: Module-specific error types
//! - `csa`: CSA terms and collateral settings
//! - `credit`: Credit ratings and parameters
//! - `margin`: VM/IM margin terms
//! - `netting_set`: Netting set and exposure configuration
//! - `counterparty_entity`: CounterParty entity
//! - `ccp`: CCP entity

mod aggregation;
mod ccp;
mod counterparty_entity;
mod counterparty_portfolio;
mod credit;
mod csa;
mod error;
mod ids;
mod margin;
mod netting_agreement;
mod netting_set;
mod xva_config;

// Re-export all public types
pub use aggregation::*;
pub use ccp::*;
pub use counterparty_entity::*;
pub use counterparty_portfolio::*;
pub use credit::*;
pub use csa::*;
pub use error::*;
pub use ids::*;
pub use margin::*;
pub use netting_agreement::*;
pub use netting_set::*;
pub use xva_config::*;

/// Prelude for commonly used types.
pub mod prelude {
    pub use super::{
        // Aggregation (stubs)
        AggregationHierarchy,
        AggregationMethod,
        // CSA
        CallFrequency,
        // Entities
        Ccp,
        CcpId,
        CloseoutCalculationMethod,
        CollateralCallFrequency,
        CollateralHaircut,
        CounterParty,
        CounterPartyError,
        CounterPartyId,
        CounterPartySector,
        // Credit
        CreditParams,
        CreditRating,
        // Cross-Book Netting
        CrossBookNettingAgreement,
        CrossBookNettingAgreementBuilder,
        CrossBookNettingAgreementId,
        CsaTerms,
        EligibleCollateral,
        // Margin
        ImModel,
        ImTerms,
        IsdaAgreementId,
        IsdaPaymentMethod,
        LegalEntityId,
        MarginTerms,
        MarginType,
        // Netting Agreement (stubs)
        NettingAgreementType,
        NettingEligibility,
        NettingSet,
        NettingSetId,
        NettingType,
        PaymentNettingFrequency,
        // XVA Config (stubs)
        RegulatoryCapitalMethod,
        RoundingDirection,
        RoundingRule,
        SegregationType,
        SimmVersion,
        VariationMarginAgreementId,
        VmTerms,
        WwrModelType,
        XvaCalculationLevel,
    };
}
