//! # infra_store
//!
//! Persistence and state management (SQLx, Redis, TimeScale) for Neutryx.
//!
//! This crate implements the Database Access Layer (DAL), providing `Save` and
//! `Load` traits for Trades and Risk Reports using `sqlx` (Postgres) or other
//! backends. It isolates I/O dependencies from the kernel.
//!
//! ## Architecture Position
//!
//! Part of the **I**nfra layer in the A-I-P-S architecture.
//! Must not depend on **P**ricer or **S**ervice crates.
//!
//! ## Example
//!
//! ```rust,ignore
//! use infra_store::{Store, PostgresStore};
//!
//! let store = PostgresStore::connect("postgres://...").await?;
//! store.save(&trade).await?;
//! ```

mod error;
mod traits;

pub use error::StoreError;
pub use traits::{Load, Save};

#[cfg(feature = "postgres")]
mod postgres;

#[cfg(feature = "postgres")]
pub use postgres::PostgresStore;

/// Prelude module for convenient imports
pub mod prelude {
    #[cfg(feature = "postgres")]
    pub use crate::PostgresStore;
    pub use crate::{Load, Save, StoreError};
}
