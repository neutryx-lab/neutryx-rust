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
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![allow(unknown_lints)]

#[cfg(test)]
mod integration_tests;

/// Pricing Kernel IR runtime engine.
pub mod kernel;

pub mod numeric;

pub mod methods;

/// Checkpointing for memory-efficient AD.
pub mod checkpoint;

pub mod graph;

pub mod generic_pricer;

pub mod result;

pub use graph::{
    ComputationGraph, GraphBuilder, GraphEdge, GraphError, GraphExtractable, GraphMetadata,
    GraphNode, GraphNodeUpdate, NodeGroup, NodeType, SimpleGraphExtractor,
};
pub use methods::{
    mc,
    mc::{GbmParams, Greek, MonteCarloConfig, MonteCarloPricer, PayoffParams, PricingResult},
    path_dependent, tree,
    tree::{BinomialTree, CrrParams, TreeConfig, TreeMethod, TreeType},
};
pub use result::{PricingMetadata, TreeTypeMetadata, UnifiedGreeks, UnifiedPricingResult};
