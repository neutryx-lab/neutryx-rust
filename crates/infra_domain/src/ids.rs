//! Type-safe ID types for financial entities.

#![allow(clippy::must_use_candidate)]

pub use crate::counterparty::{CcpId, CounterPartyId, LegalEntityId, NettingSetId};

define_id! {
    /// Type-safe trade identifier.
    TradeId
}

define_id! {
    /// Type-safe portfolio identifier.
    PortfolioId
}

define_id! {
    /// Type-safe trading book identifier.
    BookId
}

define_id! {
    /// Type-safe issuer identifier.
    IssuerId
}

/// Alias for `CounterPartyId` with lowercase 'p' (American English style).
pub type CounterpartyId = CounterPartyId;
