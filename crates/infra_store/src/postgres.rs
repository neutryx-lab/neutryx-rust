//! PostgreSQL database backend for infra_store.
//!
//! This module provides PostgreSQL-based persistence using sqlx.

use async_trait::async_trait;
use sqlx::postgres::PgPool;

use crate::{
    error::StoreError,
    traits::{Load, Save},
};

/// PostgreSQL-backed store implementation.
pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    /// Connect to a PostgreSQL database.
    ///
    /// # Arguments
    ///
    /// * `url` - PostgreSQL connection URL
    ///
    /// # Returns
    ///
    /// A new PostgresStore instance connected to the database.
    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        let pool = PgPool::connect(url)
            .await
            .map_err(|e| StoreError::ConnectionFailed(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Get a reference to the underlying connection pool.
    pub fn pool(&self) -> &PgPool { &self.pool }
}
