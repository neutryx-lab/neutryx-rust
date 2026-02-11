//! Server configuration.

use anyhow::Result;
pub use infra_config::Settings;

/// Server configuration wrapper.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Enable REST API.
    pub rest_enabled: bool,
    /// REST API address.
    pub rest_addr: String,
    /// Enable gRPC API.
    pub grpc_enabled: bool,
    /// gRPC address (reserved for future gRPC implementation).
    #[allow(dead_code)]
    pub grpc_addr: String,
    /// Number of worker threads (reserved for future thread pool configuration).
    #[allow(dead_code)]
    pub workers: usize,
}

impl ServerConfig {
    /// Load configuration from the unified settings system.
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
