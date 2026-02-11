//! Root-finding and optimisation solvers for financial applications.

mod config;
mod levenberg_marquardt;
mod multidim_newton;
mod newton_raphson;

// Re-export public types at module level
pub use config::SolverConfig;
pub use levenberg_marquardt::{LMConfig, LMResult, LevenbergMarquardtSolver};
pub use multidim_newton::{
    MultidimNewtonConfig, MultidimSolverResult, MultidimensionalNewtonSolver, SystemOfEquations,
};
pub use newton_raphson::NewtonRaphsonSolver;
