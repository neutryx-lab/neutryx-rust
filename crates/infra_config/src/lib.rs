// Clippy configuration for infra_config
#![allow(clippy::doc_markdown)]
#![allow(clippy::missing_errors_doc)]

//! # infra_config
//!
//! System configuration and environment management for Neutryx.
//!
//! This crate loads runtime settings (TOML/YAML/Env Vars) and defines
//! memory limits for the AD engine, thread pool sizes, and database
//! connection strings.
//!
//! ## Architecture Position
//!
//! Part of the **I**nfra layer in the A-I-P-S architecture.
//! Must not depend on **P**ricer or **S**ervice crates.
//!
//! ## Example
//!
//! ```rust,ignore
//! use infra_config::Settings;
//!
//! let settings = Settings::load()?;
//! println!("Thread pool size: {}", settings.engine.thread_pool_size);
//! ```

mod error;
mod settings;

pub use error::ConfigError;
pub use settings::{
    DatabaseConfig, EngineConfig, GrpcConfig, LoggingConfig, RestConfig, ServiceConfig, Settings,
};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        ConfigError, DatabaseConfig, EngineConfig, GrpcConfig, LoggingConfig, RestConfig,
        ServiceConfig, Settings,
    };
}
