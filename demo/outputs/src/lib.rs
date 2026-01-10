//! # Downstream Systems
//!
//! Mock downstream systems that receive output from the Neutryx Service layer.
//!
//! This crate simulates external systems that consume pricing results,
//! risk metrics, and regulatory reports from the A-I-P-S architecture.
//!
//! ## Modules
//!
//! - [`regulatory`]: Simulates regulatory reporting systems
//! - [`settlement`]: Simulates settlement and payment systems
//! - [`risk_dashboard`]: Simulates risk monitoring dashboards
//! - [`report_sink`]: Simulates report output destinations

pub mod regulatory;
pub mod report_sink;
pub mod risk_dashboard;
pub mod settlement;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::regulatory::{AuditStore, RegulatorApi};
    pub use crate::report_sink::{EmailSender, FileWriter, ReportSink};
    pub use crate::risk_dashboard::{MetricsStore, WebSocketSink};
    pub use crate::settlement::{NettingEngine, SwiftReceiver};
}
