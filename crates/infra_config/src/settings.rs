//! Settings and configuration structures.

use config::{Config, Environment, File};
use serde::Deserialize;

use crate::error::ConfigError;

/// Main application settings.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Settings {
    /// Engine configuration
    #[serde(default)]
    pub engine: EngineConfig,
    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,
}

impl Settings {
    /// Load settings from configuration files and environment variables.
    ///
    /// Configuration is loaded in the following order (later sources override
    /// earlier):
    /// 1. `config/default.toml`
    /// 2. `config/{environment}.toml` (based on `NEUTRYX_ENV`)
    /// 3. Environment variables prefixed with `NEUTRYX_`
    pub fn load() -> Result<Self, ConfigError> {
        let env = std::env::var("NEUTRYX_ENV").unwrap_or_else(|_| "development".into());

        let config = Config::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name(&format!("config/{env}")).required(false))
            .add_source(Environment::with_prefix("NEUTRYX").separator("__"))
            .build()?;

        let settings: Settings = config.try_deserialize()?;
        Ok(settings)
    }
}

/// Engine configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct EngineConfig {
    /// Thread pool size for parallel computation
    #[serde(default = "default_thread_pool_size")]
    pub thread_pool_size: usize,
    /// Memory limit for AD engine (in MB)
    #[serde(default = "default_memory_limit_mb")]
    pub memory_limit_mb: usize,
    /// Monte Carlo simulation paths (default batch size)
    #[serde(default = "default_mc_paths")]
    pub mc_paths: usize,
    /// Checkpointing enabled
    #[serde(default)]
    pub checkpointing_enabled: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            thread_pool_size: default_thread_pool_size(),
            memory_limit_mb: default_memory_limit_mb(),
            mc_paths: default_mc_paths(),
            checkpointing_enabled: false,
        }
    }
}

fn default_thread_pool_size() -> usize { num_cpus::get() }

fn default_memory_limit_mb() -> usize {
    1024 // 1 GB
}

fn default_mc_paths() -> usize { 10_000 }

/// Database configuration.
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    /// Database connection URL
    #[serde(default)]
    pub url: String,
    /// Maximum connection pool size
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    /// Connection timeout (in seconds)
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_connections: default_max_connections(),
            connection_timeout_secs: default_connection_timeout(),
        }
    }
}

fn default_max_connections() -> u32 { 10 }

fn default_connection_timeout() -> u64 { 30 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert!(settings.engine.thread_pool_size > 0);
        assert_eq!(settings.engine.memory_limit_mb, 1024);
    }
}
