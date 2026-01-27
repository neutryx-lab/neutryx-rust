// Clippy configuration for pricer_pricing
// Mathematical constants from academic papers are left as-is for traceability
#![allow(unexpected_cfgs)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::similar_names)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::redundant_else)]
#![allow(clippy::unused_self)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::if_not_else)]
#![allow(clippy::panic)]
#![allow(clippy::suboptimal_flops)]
#![allow(clippy::copy_iterator)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::expect_used)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::manual_is_power_of_two)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::single_match_else)]
#![allow(clippy::format_push_string)]

//! # Pricer Engine (Layer 3: AD Engine)
//!
//! ## Layer 3 Role
//!
//! pricer_pricing serves as the AD (Automatic Differentiation) engine in the
//! 4-layer architecture:
//! - Enzyme LLVM-level automatic differentiation
//! - Monte Carlo pricing kernels with AD integration (Phase 3.2)
//! - Gradient verification utilities
//!
//! ## Nightly Rust Requirement
//!
//! This is the **only crate** that requires nightly Rust toolchain
//! (`nightly-2025-01-15`). Enzyme operates at LLVM level and requires nightly
//! features for optimal performance.
//!
//! ## Layer Integration (Phase 4)
//!
//! Phase 4 adds optional L1/L2 integration via the `l1l2-integration` feature:
//! - Layer 1 (pricer_core): smoothing functions, Float trait, YieldCurve
//! - Layer 2 (pricer_models): StochasticModel trait, Instrument enum
//!
//! Without the feature flag, pricer_pricing remains fully isolated.
//!
//!
//! ## Installation
//!
//! ### Docker (Recommended)
//!
//! ```bash
//! docker build -f docker/Dockerfile.nightly -t neutryx-enzyme .
//! docker run -it neutryx-enzyme
//! ```
//!
//! ### Manual Installation
//!
//! 1. Install LLVM 18: ```bash # Ubuntu/Debian wget https://apt.llvm.org/llvm.sh
//!    chmod +x llvm.sh sudo ./llvm.sh 18 ```
//!
//! 2. Install nightly Rust: ```bash rustup toolchain install nightly-2025-01-15
//!    ```
//!
//! 3. Build pricer_pricing: ```bash cargo +nightly build -p pricer_pricing
//!    cargo +nightly test -p pricer_pricing ```
//!
//! ## Known Constraints
//!
//! - **Nightly Rust Required**: This crate uses `rust-toolchain.toml` to
//!   enforce nightly-2025-01-15
//! - **LLVM 18 Dependency**: llvm-sys requires LLVM 18 to be installed on the
//!   system (enzyme-ad feature)
//! - **Optional L1/L2**: Use `--features l1l2-integration` to enable
//!   pricer_core/pricer_models
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
// Allow unknown lints for clippy compatibility across versions
#![allow(unknown_lints)]

// Phase 4: L1/L2 integration tests (conditional compilation)
#[cfg(all(test, feature = "l1l2-integration"))]
mod integration_tests;

// Pricing Kernel IR runtime engine (Phase 4: L1/L2 integration)
#[cfg(feature = "l1l2-integration")]
pub mod kernel;

// Numeric conversion utilities (standalone, no l1l2-integration dependency)
pub mod numeric;

// Pricing methods (Monte Carlo, Tree, Path-dependent)
pub mod methods;

// Phase 4: Checkpointing for memory-efficient AD
pub mod checkpoint;

// Computation graph visualisation data structures
pub mod graph;

// Generic Pricer Engine - unified pricing API
pub mod generic_pricer;

// Unified pricing result types
pub mod result;

// Re-export pricing method submodules for convenient access
pub use methods::mc;
pub use methods::path_dependent;

// Re-export commonly used items for convenience
pub use graph::{
    ComputationGraph, GraphBuilder, GraphEdge, GraphError, GraphExtractable, GraphMetadata,
    GraphNode, GraphNodeUpdate, NodeGroup, NodeType, SimpleGraphExtractor,
};
pub use methods::mc::{
    GbmParams, Greek, MonteCarloConfig, MonteCarloPricer, PayoffParams, PricingResult,
};
pub use methods::tree::{BinomialTree, CrrParams, TreeConfig, TreeMethod, TreeType};
pub use result::{PricingMetadata, TreeTypeMetadata, UnifiedGreeks, UnifiedPricingResult};
