//! Mesh and grid generation utilities.
//!
//! This module provides functions for creating 1D and 2D computational grids
//! commonly used in numerical methods such as finite differences and PDE solvers.
//!
//! ## 1D Grids
//!
//! - [`uniform_grid`]: Equally spaced points
//! - [`log_grid`]: Logarithmically spaced points
//! - [`chebyshev_grid`]: Cosine (Chebyshev) spacing for interpolation
//! - [`concentrated_grid`]: Points concentrated around a center value
//! - [`refine_grid`]: Mesh refinement by adding midpoints
//!
//! ## 2D Grids
//!
//! - [`Grid2D`]: 2D grid structure
//! - [`tensor_product_grid`]: Create 2D grid from two 1D grids
//! - [`uniform_grid_2d`]: Uniform 2D grid
//!
//! ## Example
//!
//! ```ignore
//! use pricer_core::math::mesh::{uniform_grid, log_grid, refine_grid, Grid2D};
//!
//! // 1D uniform grid
//! let grid = uniform_grid(0.0, 1.0, 11);  // 11 points from 0 to 1
//!
//! // 1D logarithmic grid (useful for asset prices)
//! let price_grid = log_grid(10.0, 1000.0, 50);
//!
//! // Mesh refinement
//! let fine = refine_grid(&grid);  // Doubles the number of points
//!
//! // 2D grid for PDE solving
//! let grid_2d = Grid2D::new(
//!     uniform_grid(0.0, 100.0, 50),  // S: stock price
//!     uniform_grid(0.0, 1.0, 20),    // t: time
//! );
//! ```

pub mod grid_1d;
pub mod grid_2d;

// Re-export main types and functions
pub use grid_1d::{
    chebyshev_grid, concentrated_grid, log_grid, multi_refine_grid, refine_grid,
    two_sided_geometric_grid, uniform_grid,
};
pub use grid_2d::{flatten_index, tensor_product_grid, unflatten_index, uniform_grid_2d, Grid2D};
