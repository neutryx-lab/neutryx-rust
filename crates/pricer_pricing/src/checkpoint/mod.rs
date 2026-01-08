//! Checkpointing for memory-efficient automatic differentiation.
//!
//! This module provides infrastructure for saving and restoring simulation
//! state during Monte Carlo forward pass, enabling memory-efficient
//! reverse-mode automatic differentiation (AD).
//!
//! # Overview
//!
//! When computing gradients via reverse-mode AD, intermediate values from
//! the forward pass are needed. Storing all intermediate values requires
//! O(n) memory where n is the number of steps. Checkpointing reduces this
//! to O(√n) by saving state at strategic points and recomputing forward
//! values as needed during the reverse pass.
//!
//! # Key Components
//!
//! - [`CheckpointStrategy`]: Determines when to save checkpoints
//! - [`MinimalState`]: Lightweight state for O(√n) memory checkpointing
//! - [`SimulationState`]: Full captured state at a checkpoint
//! - [`CheckpointStorage`]: Storage for checkpoint states
//! - [`CheckpointManager`]: Orchestrates checkpoint operations
//!
//! # Example
//!
//! ```rust,ignore
//! use pricer_pricing::checkpoint::{CheckpointManager, CheckpointStrategy};
//!
//! // Create manager with uniform checkpointing every 50 steps
//! let strategy = CheckpointStrategy::Uniform { interval: 50 };
//! let mut manager = CheckpointManager::new(strategy);
//!
//! // During forward pass
//! for step in 0..1000 {
//!     if manager.should_checkpoint(step) {
//!         manager.save_state(step, &current_state);
//!     }
//!     // ... simulation step ...
//! }
//!
//! // During reverse pass, restore and recompute as needed
//! let nearest = manager.nearest_checkpoint(750);
//! let state = manager.restore_state(nearest.unwrap());
//! ```

mod budget;
mod manager;
mod state;
mod strategy;

pub use budget::MemoryBudget;
pub use manager::{CheckpointError, CheckpointManager, CheckpointResult};
pub use state::{CheckpointStorage, MinimalState, SimulationState};
pub use strategy::CheckpointStrategy;
