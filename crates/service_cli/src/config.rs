//! CLI configuration loading
//!
//! Loads configuration from TOML files and environment variables.

use std::path::Path;

use serde::Deserialize;

use crate::{CliError, Result};

/// CLI configuration
#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
pub struct CliConfig {
    /// General settings
    #[serde(default)]
    pub general: GeneralConfig,

    /// Pricing settings
    #[serde(default)]
    pub pricing: PricingConfig,

    /// Database settings
    #[serde(default)]
    pub database: DatabaseConfig,
}

/// General CLI settings
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct GeneralConfig {
    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Output directory
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
}

/// Pricing configuration
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct PricingConfig {
    /// Default number of Monte Carlo paths
    #[serde(default = "default_num_paths")]
    pub default_num_paths: usize,

    /// Number of threads for parallel pricing
    #[serde(default = "default_num_threads")]
    pub num_threads: usize,

    /// Enable Enzyme AD (requires nightly)
    #[serde(default)]
    pub enzyme_enabled: bool,
}

/// Database configuration
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct DatabaseConfig {
    /// Database URL
    #[serde(default)]
    pub url: Option<String>,

    /// Connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
}

fn default_log_level() -> String { "info".to_string() }

fn default_output_dir() -> String { "./output".to_string() }

fn default_num_paths() -> usize { 10_000 }

fn default_num_threads() -> usize { num_cpus::get() }

fn default_pool_size() -> u32 { 5 }

impl CliConfig {
    /// Load configuration from a TOML file
    #[allow(dead_code)]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        if !path.exists() {
            // Return default config if file doesn't exist
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        let config: CliConfig = toml::from_str(&content)
            .map_err(|e| CliError::Parse(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }
}
