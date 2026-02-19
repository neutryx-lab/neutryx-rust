//! Monte Carlo pricing kernels with Enzyme AD integration.
pub mod aligned_buffer;
pub mod capacity;
pub mod config;
pub mod error;
pub mod layout_config;
pub mod paths;
pub mod payoff;
pub mod pricer;
pub mod pricer_checkpoint;
pub mod streaming;
pub mod thread_local;
pub mod workspace;
pub mod workspace_checkpoint;
pub mod workspace_enum;
pub mod workspace_timestep_first;
pub mod workspace_trait;

#[cfg(test)]
mod e2e_path_dependent;
#[cfg(test)]
mod yield_curve_tests;

pub use aligned_buffer::AlignedPathBuffer;
pub use config::{AdMode, MonteCarloConfig, MonteCarloConfigBuilder};
pub use error::{LayoutConfigError, MonteCarloConfigError};
pub use layout_config::{PathLayout, PathLayoutConfig, StreamingConfig};
pub use paths::{
    generate_gbm_paths, generate_gbm_paths_generic, terminal_prices_generic, GbmParams,
};
pub use payoff::{
    asian_arithmetic_call_smooth, asian_arithmetic_put_smooth, compute_payoff, compute_payoffs,
    european_call_smooth, european_put_smooth, soft_plus, soft_plus_derivative, PayoffParams,
    PayoffType,
};
pub use pricer::{Greek, MonteCarloPricer, PricingResult};
pub use streaming::{
    ArithmeticAverageObserver, BarrierObserver, EuropeanObserver, LookbackObserver,
    StreamingEngine, StreamingObserver, StreamingResult,
};
pub use thread_local::{
    current_thread_index, DefaultWorkspaceFactory, ParallelWorkspaces, ThreadLocalWorkspacePool,
    WorkspaceFactory,
};
pub use workspace::PathWorkspace;
pub use workspace_checkpoint::CheckpointWorkspace;
pub use workspace_enum::WorkspaceEnum;
pub use workspace_timestep_first::TimeStepFirstWorkspace;
pub use workspace_trait::PathWorkspaceTrait;
