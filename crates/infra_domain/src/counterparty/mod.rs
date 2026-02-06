//! CounterParty and NettingSet management module.
//!
//! This module provides comprehensive types for managing counterparty
//! relationships, netting sets, CSA (Credit Support Annex) terms, and credit
//! parameters.
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
//!
//! # Example
//!
//! ```
//! use infra_domain::counterparty::prelude::*;
//!
//! // Create a counterparty with credit rating
//! let cp = CounterParty::builder("CP001", "Acme Bank")
//!     .sector(CounterPartySector::Banking)
//!     .rating(CreditRating::APlus)
//!     .build();
//!
//! assert_eq!(cp.name(), "Acme Bank");
//! ```

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
///
/// Import this module to get all the essential counterparty types:
///
/// ```
/// use infra_domain::counterparty::prelude::*;
/// ```
pub mod prelude {
    pub use super::{
        AggregationConfig,
        AggregationError,
        // Aggregation
        AggregationHierarchy,
        AggregationMethod,
        // CSA
        CallFrequency,
        CapitalConfig,
        // Entities
        Ccp,
        // IDs
        CcpId,
        CloseoutCalculationMethod,
        CloseoutNetting,
        CollateralCallFrequency,
        CollateralHaircut,
        CollateralizedExposureConfig,
        CounterParty,
        // Error
        CounterPartyError,
        CounterPartyId,
        // Config
        CounterPartySector,
        // CounterpartyPortfolio hierarchy
        CounterpartyPortfolio,
        CounterpartyPortfolioBuilder,
        // Credit
        CreditParams,
        CreditRating,
        // Cross-Book Netting
        CrossBookNettingAgreement,
        CrossBookNettingAgreementBuilder,
        CrossBookNettingAgreementId,
        CrossProductNettingEligibility,
        CsaTerms,
        DrillDownPath,
        DrillDownSegment,
        EligibleCollateral,
        ExposureAggregation,
        ExposureConfig,
        ExposurePathBuilder,
        FundingConfig,
        GroupingKey,
        // Margin
        ImModel,
        ImTerms,
        // ISDA and VM Agreements
        IndependentAmountConfig,
        IsdaAgreementId,
        IsdaInitialMargin,
        IsdaMasterAgreement,
        IsdaMasterAgreementBuilder,
        IsdaPaymentMethod,
        LegalEntityId,
        MarginTerms,
        MarginType,
        MporConfig,
        NettingAgreement,
        NettingAgreementBuilder,
        // Netting Agreement
        NettingAgreementType,
        NettingEligibility,
        NettingJurisdiction,
        NettingSet,
        NettingSetId,
        NettingType,
        NonNettableTrades,
        PaymentNetting,
        PaymentNettingFrequency,
        PfeConfidenceLevel,
        PreCalculatedExposurePath,
        RegulatoryCapitalMethod,
        RoundingDirection,
        RoundingRule,
        SegregationType,
        SimmVersion,
        VariationMarginAgreement,
        VariationMarginAgreementBuilder,
        VariationMarginAgreementId,
        VmTerms,
        WrongWayRiskConfig,
        WwrModelType,
        // XVA Configuration
        XvaCalculationLevel,
        XvaConfig,
        XvaScope,
    };
}
