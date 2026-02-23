//! CounterParty and NettingSet management module.

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
mod vm_csa;
mod xva_config;

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
pub use vm_csa::*;
pub use xva_config::*;

/// Prelude for commonly used types.
pub mod prelude {
    pub use super::{
        AggregationHierarchy, AggregationMethod, CallFrequency, Ccp, CcpId,
        CloseoutCalculationMethod, CollateralCallFrequency, CollateralHaircut, CounterParty,
        CounterPartyError, CounterPartyId, CounterPartySector, CreditParams, CreditRating,
        CrossBookNettingAgreement, CrossBookNettingAgreementBuilder, CrossBookNettingAgreementId,
        CsaTerms, EligibleCollateral, ImModel, ImTerms, IsdaAgreementId, IsdaPaymentMethod,
        LegalEntityId, MarginTerms, MarginType, NettingAgreementType, NettingEligibility,
        NettingSet, NettingSetId, NettingType, PaymentNettingFrequency, RegulatoryCapitalMethod,
        RoundingDirection, RoundingRule, SegregationType, SimmVersion, VariationMarginAgreementId,
        VmCsa, VmTerms, WwrModelType, XvaCalculationLevel,
    };
}
