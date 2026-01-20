//! Server configuration
//!
//! Re-exports configuration from `infra_config` for gateway use.
//! This module provides a unified configuration interface, eliminating
//! duplicate config definitions across service crates.

use anyhow::Result;
pub use infra_config::Settings;

/// Server configuration wrapper.
///
/// Provides convenient access to gateway-specific settings from the unified
/// `Settings` struct.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ServerConfig {
    /// Enable REST API
    pub rest_enabled: bool,
    /// REST API address
    pub rest_addr: String,
    /// Enable gRPC API
    pub grpc_enabled: bool,
    /// gRPC address (reserved for future gRPC implementation)
    pub grpc_addr: String,
    /// Number of worker threads (reserved for future thread pool configuration)
    pub workers: usize,
}

impl ServerConfig {
    /// Load configuration from the unified settings system.
    ///
    /// This loads from config files and environment variables via
    /// `infra_config`.
    pub fn load() -> Result<Self> {
        let settings = Settings::load()?;
        Ok(Self::from_settings(&settings))
    }

    /// Create from existing settings.
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            rest_enabled: settings.service.rest.enabled,
            rest_addr: settings.service.rest.addr.clone(),
            grpc_enabled: settings.service.grpc.enabled,
            grpc_addr: settings.service.grpc.addr.clone(),
            workers: settings.engine.thread_pool_size,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self { Self::from_settings(&Settings::default()) }
}
