//! Server configuration

use anyhow::Result;
use serde::Deserialize;

/// Server configuration
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ServerConfig {
    /// Enable REST API
    #[serde(default = "default_true")]
    pub rest_enabled: bool,

    /// REST API address
    #[serde(default = "default_rest_addr")]
    pub rest_addr: String,

    /// Enable gRPC API
    #[serde(default)]
    pub grpc_enabled: bool,

    /// gRPC address
    #[serde(default = "default_grpc_addr")]
    pub grpc_addr: String,

    /// Number of worker threads
    #[serde(default = "default_workers")]
    pub workers: usize,
}

fn default_true() -> bool { true }

fn default_rest_addr() -> String { "0.0.0.0:8080".to_string() }

fn default_grpc_addr() -> String { "0.0.0.0:50051".to_string() }

fn default_workers() -> usize { num_cpus::get() }

impl ServerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let rest_enabled = std::env::var("NEUTRYX_REST_ENABLED")
            .map(|v| v.parse().unwrap_or(true))
            .unwrap_or(true);

        let rest_addr = std::env::var("NEUTRYX_REST_ADDR").unwrap_or_else(|_| default_rest_addr());

        let grpc_enabled = std::env::var("NEUTRYX_GRPC_ENABLED")
            .map(|v| v.parse().unwrap_or(false))
            .unwrap_or(false);

        let grpc_addr = std::env::var("NEUTRYX_GRPC_ADDR").unwrap_or_else(|_| default_grpc_addr());

        let workers = std::env::var("NEUTRYX_WORKERS")
            .map(|v| v.parse().unwrap_or_else(|_| default_workers()))
            .unwrap_or_else(|_| default_workers());

        Ok(Self {
            rest_enabled,
            rest_addr,
            grpc_enabled,
            grpc_addr,
            workers,
        })
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            rest_enabled: true,
            rest_addr: default_rest_addr(),
            grpc_enabled: false,
            grpc_addr: default_grpc_addr(),
            workers: default_workers(),
        }
    }
}
