/! Root-finding and optimisation solvers for financial applications.

mod config;
mod levenberg_marquardt;
mod newton_raphson;

// Multi-dimensional Newton-Raphson (requires linalg feature)
#[cfg(feature = "linalg")]
mod multidim_newton;

// Re-export public types at module level
pub use config::SolverConfig;
pub use levenberg_marquardt::{LMConfig, LMResult, LevenbergMarquardtSolver};
// Multi-dimensional solver exports (requires linalg feature)
#[cfg(feature = "linalg")]
pub use multidim_newton::{
    MultidimNewtonConfig, MultidimSolverResult, MultidimensionalNewtonSolver, SystemOfEquations,
};
pub use newton_raphson::NewtonRaphsonSolver;
