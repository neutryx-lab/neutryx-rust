//! Type-safe ID types for financial entities.

#![allow(clippy::must_use_candidate)]

use derive_more::{AsRef, Display, From};

pub use crate::counterparty::{CcpId, CounterPartyId, LegalEntityId, NettingSetId};

/// Macro to define a type-safe ID with standard implementations.
macro_rules! define_id {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Display, From, AsRef)]
        #[as_ref(str)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $name(String);

        impl $name {
            /// Creates a new ID.
            #[inline]
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// Returns the ID as a string slice.
            #[inline]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_string())
            }
        }
    };
}

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
