//! Shared service helpers.

use std::path::Path;

use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::error::ServerError;

/// Load a JSON file and deserialise to `T`.
pub fn load_json_file<T: DeserializeOwned>(path: &Path, label: &str) -> Result<T, ServerError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ServerError::Internal(format!("Failed to read {label}: {e}")))?;
    serde_json::from_str(&content)
        .map_err(|e| ServerError::Internal(format!("Failed to parse {label}: {e}")))
}

/// Load a JSON file as `serde_json::Value`.
pub fn load_json_value(path: &Path, label: &str) -> Result<serde_json::Value, ServerError> {
    load_json_file(path, label)
}

/// Load a JSON file, returning `None` if the file does not exist.
pub fn try_load_json_file<T: DeserializeOwned>(
    path: &Path,
    label: &str,
) -> Result<Option<T>, ServerError> {
    if !path.exists() {
        return Ok(None);
    }
    load_json_file(path, label).map(Some)
}

/// Parse a UUID string, returning `ServerError::InvalidRequest` on failure.
pub fn parse_uuid(id_str: &str, entity_name: &str) -> Result<Uuid, ServerError> {
    id_str
        .parse()
        .map_err(|_| ServerError::InvalidRequest(format!("Invalid {entity_name} ID format")))
}

/// Parse a UUID and look it up in a [`crate::state::TypedCache`], returning.
pub fn resolve_cached<T: Clone>(
    cache: &crate::state::TypedCache<T>,
    id_str: &str,
    entity_name: &str,
) -> Result<T, ServerError> {
    let id = parse_uuid(id_str, entity_name)?;
    cache
        .get(&id)
        .ok_or_else(|| ServerError::NotFound(format!("{entity_name} {id} not found")))
}

/// Execute a closure and return `(result, elapsed_ms)`.
pub fn timed<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let start = std::time::Instant::now();
    let result = f();
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    (result, ms)
}
