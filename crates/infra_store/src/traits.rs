//! Storage traits.

use crate::error::StoreError;

/// Trait for saving entities to storage.
pub trait Save<T> {
    /// Save an entity.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the save operation fails.
    fn save(&self, entity: &T) -> Result<(), StoreError>;
}

/// Trait for loading entities from storage.
pub trait Load<T, K> {
    /// Load an entity by key.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the load operation fails.
    fn load(&self, key: &K) -> Result<Option<T>, StoreError>;

    /// Load all entities.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] if the load operation fails.
    fn load_all(&self) -> Result<Vec<T>, StoreError>;
}

/// Async version of Save trait.
#[cfg(feature = "postgres")]
#[async_trait::async_trait]
pub trait SaveAsync<T: Send + Sync> {
    /// Save an entity asynchronously.
    async fn save(&self, entity: &T) -> Result<(), StoreError>;
}

/// Async version of Load trait.
#[cfg(feature = "postgres")]
#[async_trait::async_trait]
pub trait LoadAsync<T: Send + Sync, K: Send + Sync> {
    /// Load an entity by key asynchronously.
    async fn load(&self, key: &K) -> Result<Option<T>, StoreError>;

    /// Load all entities asynchronously.
    async fn load_all(&self) -> Result<Vec<T>, StoreError>;
}
