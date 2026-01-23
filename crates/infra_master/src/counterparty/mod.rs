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
//! use infra_master::counterparty::prelude::*;
//!
//! // Create a counterparty with credit rating
//! let cp = CounterParty::builder("CP001", "Acme Bank")
//!     .sector(CounterPartySector::Banking)
//!     .rating(CreditRating::APlus)
//!     .build();
//!
//! assert_eq!(cp.name(), "Acme Bank");
//! ```

mod ccp;
mod counterparty_entity;
mod credit;
mod csa;
mod error;
mod ids;
mod margin;
mod netting_set;

// Re-export all public types
pub use ccp::*;
pub use counterparty_entity::*;
pub use credit::*;
pub use csa::*;
pub use error::*;
pub use ids::*;
pub use margin::*;
pub use netting_set::*;

/// Prelude for commonly used types.
///
/// Import this module to get all the essential counterparty types:
///
/// ```
/// use infra_master::counterparty::prelude::*;
/// ```
pub mod prelude {
    pub use super::{
        // CSA
        CallFrequency,
        // Entities
        Ccp,
        // IDs
        CcpId,
        CollateralHaircut,
        CounterParty,
        // Error
        CounterPartyError,
        CounterPartyId,
        // Config
        CounterPartySector,
        // Credit
        CreditParams,
        CreditRating,
        CsaTerms,
        EligibleCollateral,
        ExposureConfig,
        // Margin
        ImModel,
        ImTerms,
        // Agreement IDs
        IsdaAgreementId,
        LegalEntityId,
        MarginTerms,
        MarginType,
        NettingSet,
        NettingSetId,
        NettingType,
        RoundingDirection,
        RoundingRule,
        SegregationType,
        SimmVersion,
        VariationMarginAgreementId,
        VmTerms,
    };
}
