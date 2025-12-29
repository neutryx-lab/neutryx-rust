# Implementation Plan

## Overview

Implementation tasks for Phase 1.3: Numerical Methods - Interpolation and Solvers. All 12 requirements mapped to executable tasks with proper progression from foundation to integration.

---

## Tasks

- [x] 1. Establish error types and module structure
- [x] 1.1 (P) Define interpolation and solver error types
  - Create `InterpolationError` enum with variants for out-of-bounds queries, insufficient data, non-monotonic data, and invalid input
  - Create `SolverError` enum with variants for max iterations exceeded, derivative near zero, no bracket found, and numerical instability
  - Implement `std::error::Error` and `Display` traits using thiserror
  - Derive `Debug`, `Clone`, and `PartialEq` for testing
  - Add serde serialisation support behind `serde` feature flag
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5_

- [x] 1.2 (P) Create interpolators module scaffold
  - Establish interpolators submodule under the math module
  - Create module entry point with public exports for traits and implementations
  - Add module-level rustdoc documentation describing purpose and usage
  - _Requirements: 12.1, 12.3, 12.4_

- [x] 1.3 (P) Create solvers module scaffold
  - Establish solvers submodule under the math module
  - Create module entry point with public exports for solver types
  - Add module-level rustdoc documentation describing purpose and usage
  - _Requirements: 12.2, 12.3, 12.5_

- [x] 2. Implement interpolator trait foundation
- [x] 2.1 Define the generic interpolator trait
  - Create `Interpolator<T>` trait with generic type parameter bounded by `num_traits::Float`
  - Define `interpolate(x: T) -> Result<T, InterpolationError>` method signature
  - Define `domain() -> (T, T)` method returning the valid interpolation range
  - Add comprehensive rustdoc with usage examples
  - Depends on: 1.1, 1.2 (error types and module structure)
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.6_

- [x] 3. Implement linear interpolation
- [x] 3.1 (P) Build linear interpolator structure and construction
  - Create struct storing sorted x and y coordinate vectors
  - Implement constructor that validates minimum data point count (at least 2)
  - Add automatic sorting of input data when not already sorted
  - Return `InsufficientData` error for fewer than 2 points
  - Depends on: 2.1 (interpolator trait)
  - _Requirements: 2.1, 2.3, 2.4, 1.5_

- [x] 3.2 Implement linear interpolation algorithm
  - Implement the `Interpolator<T>` trait for `LinearInterpolator<T>`
  - Use `partition_point` for O(log n) segment lookup via binary search
  - Apply linear interpolation formula: `y = y0 + (y1 - y0) * (x - x0) / (x1 - x0)`
  - Return `OutOfBounds` error for queries outside domain
  - Depends on: 3.1 (structure complete)
  - _Requirements: 2.1, 2.2, 1.2, 1.3_

- [x] 3.3 Add linear interpolator unit tests
  - Test exact interpolation at knot points returns original y values
  - Test mid-segment interpolation accuracy against expected values
  - Test boundary behaviour at domain edges
  - Test error handling for insufficient data and out-of-bounds queries
  - _Requirements: 2.5, 11.1_

- [ ] 4. Implement cubic spline interpolation
- [ ] 4.1 (P) Build cubic spline structure and coefficient computation
  - Create struct storing x coordinates and polynomial coefficients (a, b, c, d) per segment
  - Implement Thomas algorithm for O(n) tridiagonal system solution
  - Compute natural cubic spline coefficients with zero second derivative at boundaries
  - Validate minimum data point count (at least 3)
  - Return `InsufficientData` error for fewer than 3 points
  - Depends on: 2.1 (interpolator trait)
  - _Requirements: 3.1, 3.2, 3.5, 1.5_

- [ ] 4.2 Implement cubic spline evaluation
  - Implement the `Interpolator<T>` trait for `CubicSplineInterpolator<T>`
  - Use binary search to locate the appropriate segment
  - Evaluate cubic polynomial: `y = a + b*(x-xi) + c*(x-xi)² + d*(x-xi)³`
  - Return `OutOfBounds` error for queries outside domain
  - Depends on: 4.1 (coefficients computed)
  - _Requirements: 3.1, 3.3, 3.4_

- [ ] 4.3 Add cubic spline unit tests
  - Test exact interpolation at knot points returns original y values
  - Test C² continuity by verifying second derivative continuity at interior knots
  - Test smooth curve behaviour between points
  - Test error handling for insufficient data
  - _Requirements: 3.6, 11.1_

- [ ] 5. Implement monotonic interpolation
- [ ] 5.1 (P) Build monotonic interpolator with Fritsch-Carlson method
  - Create struct storing x coordinates, y values, and computed Hermite slopes
  - Implement monotonicity validation checking consecutive differences have consistent sign
  - Compute initial slopes from secants between adjacent points
  - Apply Fritsch-Carlson correction to modify slopes preserving monotonicity
  - Return `NonMonotonicData` error for input data that violates monotonicity
  - Depends on: 2.1 (interpolator trait)
  - _Requirements: 4.1, 4.4, 4.6, 1.5_

- [ ] 5.2 Implement monotonic Hermite evaluation
  - Implement the `Interpolator<T>` trait for `MonotonicInterpolator<T>`
  - Use binary search for segment location
  - Evaluate using Hermite basis functions with corrected slopes
  - Guarantee monotonically increasing output for increasing input, decreasing for decreasing
  - Depends on: 5.1 (slopes computed)
  - _Requirements: 4.1, 4.2, 4.3_

- [ ] 5.3 Add monotonic interpolator unit tests
  - Test that strictly increasing input produces strictly increasing output at any query point
  - Test that strictly decreasing input produces strictly decreasing output
  - Test `NonMonotonicData` error returned for non-monotonic data
  - Test exact interpolation at knot points
  - _Requirements: 4.5, 11.1_

- [ ] 6. Implement smooth interpolation function
- [ ] 6.1 Build branch-free smooth interpolation using smoothing primitives
  - Create `smooth_interp` function accepting x/y arrays, query point, and smoothing epsilon
  - Use `smooth_indicator` from `crate::math::smoothing` to compute soft segment weights
  - Compute weight for segment i as: `w_i = smooth_indicator(x - x_i, ε) - smooth_indicator(x - x_{i+1}, ε)`
  - Sum weighted linear interpolations across all segments with normalised weights
  - Avoid all conditional branches (`if`) on Float values for segment selection
  - Depends on: 1.2 (interpolators module exists)
  - _Requirements: 5.1, 5.2, 5.6_

- [ ] 6.2 Add smooth interpolation tests
  - Test convergence to standard linear interpolation as epsilon approaches zero
  - Test gradient propagation through smoothing operations with Dual64
  - Test behaviour across various epsilon values (1e-3 to 1e-6)
  - _Requirements: 5.3, 5.4, 5.5_

- [ ] 7. Implement bilinear 2D interpolation
- [ ] 7.1 (P) Build bilinear interpolator structure and construction
  - Create struct storing x axis vector, y axis vector, and 2D grid values `zs[i][j] = z(xs[i], ys[j])`
  - Implement constructor validating grid dimensions match axis lengths
  - Add `domain_x()` and `domain_y()` methods returning valid interpolation ranges
  - Validate minimum grid requirements (at least 2 points on each axis)
  - Depends on: 1.2 (interpolators module exists)
  - _Requirements: 6.1, 6.6, 1.5_

- [ ] 7.2 Implement bilinear interpolation algorithm
  - Implement `interpolate(x, y)` method accepting two coordinates
  - Use binary search on both axes to locate containing grid cell
  - Apply bilinear formula: `(1-u)(1-v)z00 + u(1-v)z10 + (1-u)v*z01 + uv*z11`
  - Return `OutOfBounds` error for queries outside grid boundaries
  - Depends on: 7.1 (structure complete)
  - _Requirements: 6.2, 6.3, 6.4_

- [ ] 7.3 Add bilinear interpolator unit tests
  - Test exact interpolation at grid corner points returns original z values
  - Test mid-cell interpolation accuracy against manually computed values
  - Test boundary and corner behaviour at domain edges
  - Test error handling for out-of-bounds queries in both x and y
  - _Requirements: 6.5, 11.1_

- [ ] 8. Implement solver trait foundation
- [ ] 8.1 Define solver configuration and trait infrastructure
  - Create `SolverConfig<T>` struct with tolerance and max_iterations fields
  - Implement sensible defaults (tolerance: 1e-10, max_iterations: 100)
  - Add validation ensuring tolerance and max_iterations are positive
  - Create rustdoc documentation with usage examples
  - Depends on: 1.1, 1.3 (error types and solvers module)
  - _Requirements: 7.1, 7.4_

- [ ] 9. Implement Newton-Raphson solver
- [ ] 9.1 (P) Build Newton-Raphson solver with explicit derivative
  - Create `NewtonRaphsonSolver<T>` struct holding `SolverConfig<T>`
  - Implement `find_root(f, f_prime, x0)` method accepting function, derivative function, and initial guess
  - Apply Newton iteration: `x_{n+1} = x_n - f(x_n) / f'(x_n)` until convergence
  - Return `MaxIterationsExceeded` error when iteration limit reached
  - Return `DerivativeNearZero` error when |f'(x)| < small epsilon before division
  - Depends on: 8.1 (solver config)
  - _Requirements: 8.1, 8.2, 8.3, 7.2, 7.3, 7.5_

- [ ] 9.2 Add automatic differentiation mode for Newton-Raphson
  - Implement `find_root_ad(f, x0)` method behind `num-dual-mode` feature flag
  - Create `Dual64` with derivative seed: `Dual64::from(x).derivative()`
  - Extract function value from real part and derivative from dual part in single evaluation
  - Use extracted derivative for Newton iteration step
  - Depends on: 9.1 (base solver complete)
  - _Requirements: 8.4, 8.5_

- [ ] 9.3 Add Newton-Raphson solver tests
  - Test finding square root of 2 via solving x² - 2 = 0
  - Test AD mode produces identical results to explicit derivative mode
  - Test `DerivativeNearZero` error when derivative approaches zero
  - Test `MaxIterationsExceeded` error for non-convergent cases
  - _Requirements: 11.3_

- [ ] 10. Implement Brent's method solver
- [ ] 10.1 (P) Build Brent solver with bracketing validation
  - Create `BrentSolver<T>` struct holding `SolverConfig<T>`
  - Implement `find_root(f, a, b)` method accepting function and bracket endpoints
  - Validate bracket by checking `f(a) * f(b) < 0` (opposite signs)
  - Return `NoBracket` error when function values at endpoints have same sign
  - Depends on: 8.1 (solver config)
  - _Requirements: 9.1, 9.2, 9.3_

- [ ] 10.2 Implement full Brent algorithm
  - Combine bisection, secant method, and inverse quadratic interpolation
  - Apply step selection logic with mflag for fallback to bisection when interpolation steps too large
  - Swap endpoints to maintain |f(b)| ≤ |f(a)| invariant
  - Guarantee convergence for continuous functions with valid bracket
  - Depends on: 10.1 (bracket validation)
  - _Requirements: 9.4, 9.5_

- [ ] 10.3 Add Brent solver tests
  - Test finding roots of standard polynomial functions (e.g., x³ - x - 2 = 0)
  - Test guaranteed convergence on continuous functions achieves machine precision
  - Test `NoBracket` error for invalid brackets where endpoints have same sign
  - Test tolerance achievement matches configuration
  - _Requirements: 11.3_

- [ ] 11. Implement AD compatibility verification
- [ ] 11.1 (P) Add dual number tests for all interpolators
  - Test `LinearInterpolator` with `Dual64` inputs returns finite gradient
  - Test `CubicSplineInterpolator` with `Dual64` inputs returns finite gradient
  - Test `MonotonicInterpolator` with `Dual64` inputs returns finite gradient
  - Test `BilinearInterpolator` with `Dual64` inputs returns finite partial derivatives
  - Test `smooth_interp` function with `Dual64` inputs propagates gradients
  - Feature-gate all tests behind `num-dual-mode`
  - _Requirements: 11.1, 2.5, 3.6, 4.5, 5.5, 6.5_

- [ ] 11.2 (P) Add gradient verification tests
  - Compare AD-computed gradients against finite difference approximations for all interpolators
  - Use central difference formula: `(f(x+h) - f(x-h)) / (2h)` with small h
  - Verify relative tolerance within 1e-6 for linear, cubic spline, monotonic, and bilinear
  - Test gradient correctness at interior points, near boundaries, and at knots
  - _Requirements: 11.2, 11.4_

- [ ] 11.3* Add property-based gradient tests
  - Use proptest to generate random input data arrays and query points
  - Verify gradient bounds are reasonable (not NaN, not infinite) across random inputs
  - Test monotonicity of gradient sign where mathematically expected
  - Test solver convergence properties across random starting points and functions
  - _Requirements: 11.5_

- [ ] 12. Final integration and documentation
- [ ] 12.1 Wire up module exports and re-exports
  - Export `interpolators` and `solvers` submodules from `math` module
  - Re-export primary types (`LinearInterpolator`, `CubicSplineInterpolator`, etc.) at module level
  - Verify all public types accessible via documented import paths
  - Depends on: All implementation tasks complete
  - _Requirements: 12.3, 12.6_

- [ ] 12.2 Add comprehensive rustdoc with examples
  - Add module-level documentation for interpolators with overview and usage patterns
  - Add module-level documentation for solvers with overview and usage patterns
  - Include runnable examples in all public type documentation
  - Verify documentation examples compile and pass via `cargo test --doc`
  - _Requirements: 12.6_

- [ ] 12.3 Run full test suite and verify all requirements
  - Execute complete test suite including property tests with `cargo test --all-features`
  - Verify clippy passes without warnings: `cargo clippy --all-targets -- -D warnings`
  - Verify formatting consistent: `cargo fmt --all -- --check`
  - Confirm all 12 requirements satisfied with passing tests
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1 (Interpolator trait) | 2.1, 3.2, 4.2, 5.2, 7.2 |
| 2 (Linear interpolation) | 3.1, 3.2, 3.3 |
| 3 (Cubic spline) | 4.1, 4.2, 4.3 |
| 4 (Monotonic interpolation) | 5.1, 5.2, 5.3 |
| 5 (Smooth interpolation) | 6.1, 6.2 |
| 6 (Bilinear 2D) | 7.1, 7.2, 7.3 |
| 7 (RootFinder trait) | 8.1, 9.1, 10.1 |
| 8 (Newton-Raphson) | 9.1, 9.2, 9.3 |
| 9 (Brent's method) | 10.1, 10.2, 10.3 |
| 10 (Error types) | 1.1 |
| 11 (AD compatibility) | 3.3, 4.3, 5.3, 6.2, 7.3, 9.3, 10.3, 11.1, 11.2, 11.3 |
| 12 (Module organisation) | 1.2, 1.3, 12.1, 12.2 |
