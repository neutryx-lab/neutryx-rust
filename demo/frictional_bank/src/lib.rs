//! # FrictionalBank Demo Orchestrator
//!
//! FrictionalBank is a comprehensive demo system that demonstrates the full
//! capabilities of the Neutryx derivatives pricing library using the A-I-P-S
//! (Adapter → Infra → Pricer → Service) architecture.
//!
//! ## Features
//!
//! - **EOD Batch Processing**: End-of-day batch workflow for pricing and risk calculation
//! - **Intraday Processing**: Real-time portfolio re-evaluation on market data updates
//! - **Stress Testing**: Scenario-based stress testing with preset shocks
//!
//! ## Architecture Compliance
//!
//! This crate sits in the Demo layer, which is external to A-I-P-S:
//! - Inputs flow through Adapter layer (adapter_feeds, adapter_fpml, adapter_loader)
//! - Outputs flow through Service layer (service_cli, service_gateway)
//! - Pricer layer is accessed but pricer_pricing is excluded (Enzyme isolation)

pub mod config;
pub mod error;
pub mod workflow;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::config::{DemoConfig, DemoMode};
    pub use crate::error::DemoError;
    pub use crate::workflow::{
        DemoWorkflow, EodBatchWorkflow, IntradayWorkflow, ProgressCallback, StressTestWorkflow,
        WorkflowResult, WorkflowStep,
    };
}
