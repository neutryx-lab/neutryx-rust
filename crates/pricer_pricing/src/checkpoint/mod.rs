//! Checkpointing for memory-efficient reverse-mode automatic differentiation.
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
