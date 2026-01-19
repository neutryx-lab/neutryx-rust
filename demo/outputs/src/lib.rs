// Clippy configuration for demo_outputs
// Demo code uses simpler patterns acceptable for non-production code
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::similar_names)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::unused_self)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::match_same_arms)]

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
    pub use crate::{
        regulatory::{AuditStore, RegulatorApi},
        report_sink::{EmailSender, FileWriter, ReportSink},
        risk_dashboard::{MetricsStore, WebSocketSink},
        settlement::{NettingEngine, SwiftReceiver},
    };
}
