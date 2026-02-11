//! Pricing kernel module for linear and exotic products.

mod callable_engine;

mod context;

mod engine;

mod lsmc;

mod provider;

mod script_engine;

pub use callable_engine::{
    BackwardPassResult, CallableEngine, ExerciseDecision, ExerciseState, SimulatedPaths,
};
pub use context::KernelContext;
pub use engine::{days_to_years, years_to_days, LinearEngine};
pub use lsmc::{BasisFunction, LSMCRegressor, RegressionResult};
pub use provider::{
    CurveProvider, FlatCurveProvider, IndexedMarketAdapter, IndexedMarketAdapterBuilder,
};
pub use script_engine::{ExecutionTrace, FlatSpotProvider, ScriptEngine, SpotProvider, TraceStep};

#[cfg(test)]
mod integration;
