//! Shared macros for infra_domain.

/// Define a type-safe newtype ID with standard implementations.
///
/// Generates: `Clone`, `Debug`, `Default`, `PartialEq`, `Eq`, `Hash`,
/// `Display`, `From<String>`, `From<&str>`, `AsRef<str>`, plus
/// optional serde `Serialize`/`Deserialize` (transparent).
macro_rules! define_id {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Default, PartialEq, Eq, Hash,
                 derive_more::Display, derive_more::From, derive_more::AsRef)]
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
