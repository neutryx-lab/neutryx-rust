#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::similar_names)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::unused_self)]
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

//! # Checkpointing for Memory-Efficient AD
//!
//! Provides checkpoint strategies, memory budgets, simulation state capture,
//! and a checkpoint manager for memory-efficient reverse-mode automatic
//! differentiation in Monte Carlo simulations.

mod budget;
mod manager;
mod observer_state;
mod state;
mod strategy;

pub use budget::MemoryBudget;
pub use manager::{CheckpointError, CheckpointManager, CheckpointResult};
pub use observer_state::PathObserverState;
pub use state::{CheckpointStorage, MinimalState, SimulationState};
pub use strategy::CheckpointStrategy;
