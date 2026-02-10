/! Checkpointing for memory-efficient reverse-mode automatic differentiation.

mod budget;
mod manager;
mod state;
mod strategy;

pub use budget::MemoryBudget;
pub use manager::{CheckpointError, CheckpointManager, CheckpointResult};
pub use state::{CheckpointStorage, MinimalState, SimulationState};
pub use strategy::CheckpointStrategy;
