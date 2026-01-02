//! REST API server for Neutryx XVA pricing library
//!
//! This crate provides an HTTP REST API for the Neutryx pricing library,
//! exposing option pricing, Greeks calculation, and XVA computation endpoints.
//!
//! # Architecture
//!
//! The server is organized into the following modules:
//! - [`config`]: Server configuration management (environment variables, TOML files, CLI)
//! - [`routes`]: HTTP route handlers organized by endpoint group
//! - [`server`]: Server startup and lifecycle management
//!
//! # Example
//!
//! ```no_run
//! use pricer_server::{config::ServerConfig, server::Server};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = ServerConfig::default();
//!     let server = Server::new(config);
//!     server.run().await.expect("Server failed");
//! }
//! ```

pub mod config;
pub mod routes;
pub mod server;

// Re-export pricer dependencies for integration
pub use pricer_core;
pub use pricer_models;
pub use pricer_xva;

#[cfg(feature = "kernel-integration")]
pub use pricer_kernel;

/// Server version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
