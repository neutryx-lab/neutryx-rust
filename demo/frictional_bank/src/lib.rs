// Clippy configuration for frictional_bank
// Demo code uses simpler patterns acceptable for non-production code
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::similar_names)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::ref_option)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::unnecessary_literal_bound)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::unused_self)]

//! # FrictionalBank Demo Orchestrator
//!
//! FrictionalBank is a comprehensive demo system that demonstrates the full
//! capabilities of the Neutryx derivatives pricing library using the A-I-P-S
//! (Adapter → Infra → Pricer → Service) architecture.
//!
//! ## Features
//!
//! - **EOD Batch Processing**: End-of-day batch workflow for pricing and risk
//!   calculation
//! - **Intraday Processing**: Real-time portfolio re-evaluation on market data
//!   updates
//! - **Stress Testing**: Scenario-based stress testing with preset shocks
//! - **IRS AAD Demo**: IRS pricing with AAD vs Bump-and-Revalue performance
//!   comparison
//!
//! ## Architecture Compliance
//!
//! This crate sits in the Demo layer, which is external to A-I-P-S:
//! - Inputs flow through Adapter layer (adapter_feeds, adapter_fpml,
//!   adapter_loader)
//! - Outputs flow through Service layer (service_cli, service_gateway)
//! - Pricer layer is accessed including pricer_pricing for IRS AAD Demo (with
//!   l1l2-integration)

pub mod config;
pub mod error;
pub mod workflow;

/// Prelude module for convenient imports
pub mod prelude {
    #[cfg(feature = "l1l2-integration")]
    pub use crate::workflow::{
        IrsAadConfig, IrsAadWorkflow, IrsComputeResult, IrsParams, XvaDemoResult,
    };
    pub use crate::{
        config::{DemoConfig, DemoMode},
        error::DemoError,
        workflow::{
            DemoWorkflow, EodBatchWorkflow, IntradayWorkflow, IrsAadWorkflow, IrsParams,
            ProgressCallback, StressTestWorkflow, WorkflowResult, WorkflowStep,
        },
    };
}
