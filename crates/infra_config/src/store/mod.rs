//! Persistence and state management (SQLx, Redis, TimeScale).
//!
//! Provides `Save` and `Load` traits for persisting domain entities
//! using various backends (Postgres, Redis, etc.).

mod error;
mod traits;

pub use error::StoreError;
pub use traits::{Load, Save};

#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "postgres")]
pub use traits::{LoadAsync, SaveAsync};

#[cfg(feature = "postgres")]
pub use self::postgres::PostgresStore;
