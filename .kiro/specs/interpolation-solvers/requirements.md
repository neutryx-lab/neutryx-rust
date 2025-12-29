# Requirements Document

## Introduction

This specification defines requirements for Phase 1.3: Numerical Methods - Interpolation and Solvers. The implementation targets `crates/pricer_core/src/math/interpolators/` and `solvers/`, providing foundational numerical infrastructure for the XVA pricing library. All components must support automatic differentiation (AD) through compatibility with `Dual64` (num-dual) and future Enzyme integration.

## Requirements

### Requirement 1: Interpolator Trait Infrastructure

**Objective:** As a quantitative developer, I want a generic `Interpolator<T>` trait, so that I can perform interpolation operations with any floating-point type including AD-compatible dual numbers.

#### Acceptance Criteria

1. The `Interpolator` trait shall define a generic type parameter `T: num_traits::Float` for floating-point operations.
2. When `interpolate(x)` is called with a value within the data range, the Interpolator shall return the interpolated value at point `x`.
3. If `interpolate(x)` is called with a value outside the data range, the Interpolator shall return an `InterpolationError::OutOfBounds` error.
4. The Interpolator shall provide a `domain()` method that returns the valid interpolation range as `(T, T)`.
5. When constructing an Interpolator with fewer than 2 data points, the Interpolator shall return an `InterpolationError::InsufficientData` error.
6. The Interpolator trait shall be generic over `T: Float` to support both `f64` and `Dual64` types.

### Requirement 2: Linear Interpolation

**Objective:** As a quantitative developer, I want a linear interpolator implementation, so that I can perform simple piecewise-linear interpolation on market data.

#### Acceptance Criteria

1. The `LinearInterpolator` shall implement the `Interpolator<T>` trait for any `T: Float`.
2. When `interpolate(x)` is called, the LinearInterpolator shall compute the value using the formula: `y = y0 + (y1 - y0) * (x - x0) / (x1 - x0)`.
3. The LinearInterpolator shall store data points in sorted order by x-coordinate.
4. When constructed with unsorted data, the LinearInterpolator shall automatically sort the data points.
5. When `interpolate(x)` is called with a `Dual64` input, the LinearInterpolator shall correctly propagate gradient information through the computation.

### Requirement 3: Cubic Spline Interpolation

**Objective:** As a quantitative developer, I want a cubic spline interpolator, so that I can achieve smooth C² continuous interpolation for yield curves and volatility surfaces.

#### Acceptance Criteria

1. The `CubicSplineInterpolator` shall implement the `Interpolator<T>` trait for any `T: Float`.
2. The CubicSplineInterpolator shall compute natural cubic spline coefficients (second derivative = 0 at boundaries).
3. When `interpolate(x)` is called, the CubicSplineInterpolator shall evaluate the cubic polynomial for the appropriate segment.
4. The CubicSplineInterpolator shall guarantee C² continuity (continuous second derivative) across all interior knots.
5. When constructing a CubicSplineInterpolator with fewer than 3 data points, the constructor shall return an `InterpolationError::InsufficientData` error.
6. When `interpolate(x)` is called with `Dual64` inputs, the CubicSplineInterpolator shall correctly propagate gradients through the spline evaluation.

### Requirement 4: Monotonic Interpolation

**Objective:** As a quantitative developer, I want a monotonic interpolator, so that I can interpolate discount factors and survival probabilities without introducing arbitrage opportunities.

#### Acceptance Criteria

1. The `MonotonicInterpolator` shall implement the `Interpolator<T>` trait for any `T: Float`.
2. While the input data is monotonically increasing, the MonotonicInterpolator shall guarantee monotonically increasing output.
3. While the input data is monotonically decreasing, the MonotonicInterpolator shall guarantee monotonically decreasing output.
4. The MonotonicInterpolator shall use the Fritsch-Carlson method to preserve monotonicity.
5. When `interpolate(x)` is called with `Dual64` inputs, the MonotonicInterpolator shall correctly propagate gradient information.
6. If the input data contains non-monotonic segments, the MonotonicInterpolator shall return an `InterpolationError::NonMonotonicData` error.

### Requirement 5: Smooth Interpolation Function

**Objective:** As a quantitative developer, I want a `smooth_interp` function that uses smoothing primitives, so that I can perform differentiable interpolation compatible with Enzyme AD.

#### Acceptance Criteria

1. The `smooth_interp` function shall accept x/y data arrays, query point, and smoothing epsilon parameter.
2. The `smooth_interp` function shall use `smooth_indicator` from `crate::math::smoothing` to blend between segments.
3. When epsilon approaches zero, `smooth_interp` shall converge to standard linear interpolation behaviour.
4. The `smooth_interp` function shall be generic over `T: Float` to support both `f64` and `Dual64`.
5. When `smooth_interp` is called with `Dual64` inputs, the function shall correctly propagate gradient information through all smoothing operations.
6. The `smooth_interp` function shall not use conditional branches (`if`) on Float values for segment selection.

### Requirement 6: Bilinear Interpolation (2D)

**Objective:** As a quantitative developer, I want a 2D bilinear interpolator, so that I can interpolate volatility surfaces and correlation matrices.

#### Acceptance Criteria

1. The `BilinearInterpolator` shall implement interpolation on a 2D grid of data points.
2. When `interpolate(x, y)` is called with coordinates within the grid, the BilinearInterpolator shall return the bilinearly interpolated value.
3. If `interpolate(x, y)` is called with coordinates outside the grid bounds, the BilinearInterpolator shall return an `InterpolationError::OutOfBounds` error.
4. The BilinearInterpolator shall be generic over `T: Float` to support both `f64` and `Dual64`.
5. When `interpolate(x, y)` is called with `Dual64` inputs, the BilinearInterpolator shall correctly propagate partial derivatives ∂z/∂x and ∂z/∂y.
6. The BilinearInterpolator shall provide `domain_x()` and `domain_y()` methods returning the valid interpolation ranges.

### Requirement 7: Root Finder Trait Infrastructure

**Objective:** As a quantitative developer, I want a generic `RootFinder<T>` trait, so that I can solve equations f(x) = 0 for implied volatility and other calibration tasks.

#### Acceptance Criteria

1. The `RootFinder` trait shall define a generic type parameter `T: Float` for floating-point operations.
2. When `find_root(f, initial_guess)` is called, the RootFinder shall return the root `x` where `|f(x)| < tolerance`.
3. If the root-finding algorithm fails to converge within the maximum iterations, the RootFinder shall return a `SolverError::MaxIterationsExceeded` error.
4. The RootFinder trait shall provide configurable `tolerance` and `max_iterations` parameters.
5. The RootFinder shall support functions of type `Fn(T) -> T` for AD compatibility.

### Requirement 8: Newton-Raphson Solver

**Objective:** As a quantitative developer, I want a Newton-Raphson root finder, so that I can efficiently solve smooth equations using derivative information.

#### Acceptance Criteria

1. The `NewtonRaphsonSolver` shall implement the `RootFinder<T>` trait for any `T: Float`.
2. When `find_root(f, f_prime, x0)` is called, the NewtonRaphsonSolver shall iterate using `x_{n+1} = x_n - f(x_n) / f'(x_n)`.
3. If `f'(x)` is near zero (|f'(x)| < epsilon), the NewtonRaphsonSolver shall return a `SolverError::DerivativeNearZero` error.
4. The NewtonRaphsonSolver shall provide a `find_root_ad(f, x0)` method that automatically computes derivatives using `Dual64`.
5. When using AD mode with `Dual64`, the NewtonRaphsonSolver shall extract both function value and derivative from a single function evaluation.

### Requirement 9: Brent's Method Solver

**Objective:** As a quantitative developer, I want a Brent's method root finder, so that I can robustly solve equations without requiring derivative information.

#### Acceptance Criteria

1. The `BrentSolver` shall implement the `RootFinder<T>` trait for any `T: Float`.
2. When `find_root(f, a, b)` is called with a bracketing interval, the BrentSolver shall find a root where `f(a) * f(b) < 0`.
3. If `f(a)` and `f(b)` have the same sign, the BrentSolver shall return a `SolverError::NoBracket` error.
4. The BrentSolver shall combine bisection, secant, and inverse quadratic interpolation for optimal convergence.
5. The BrentSolver shall guarantee convergence for continuous functions with a valid bracket.

### Requirement 10: Error Types

**Objective:** As a quantitative developer, I want well-defined error types, so that I can handle interpolation and solver failures appropriately.

#### Acceptance Criteria

1. The `InterpolationError` enum shall include variants: `OutOfBounds`, `InsufficientData`, `NonMonotonicData`, and `InvalidInput`.
2. The `SolverError` enum shall include variants: `MaxIterationsExceeded`, `DerivativeNearZero`, `NoBracket`, and `NumericalInstability`.
3. Both error types shall implement `std::error::Error` and `std::fmt::Display` traits.
4. Both error types shall derive `Debug`, `Clone`, and `PartialEq` for testing purposes.
5. Where the `serde` feature is enabled, both error types shall implement `Serialize` and `Deserialize`.

### Requirement 11: Automatic Differentiation Compatibility

**Objective:** As a quantitative developer, I want all interpolators and solvers to work with `Dual64`, so that I can compute sensitivities (Greeks) through the numerical methods.

#### Acceptance Criteria

1. All interpolator implementations shall pass unit tests with both `f64` and `Dual64` inputs.
2. When interpolating with `Dual64`, the computed gradients shall match finite difference approximations within 1e-6 relative tolerance.
3. All solver implementations shall correctly handle `Dual64` function inputs and outputs.
4. The gradient propagation tests shall verify correct derivative computation for all interpolation methods.
5. Property-based tests using `proptest` shall verify gradient correctness across random inputs.

### Requirement 12: Module Organisation

**Objective:** As a maintainer, I want clear module organisation, so that the codebase follows the project's structural conventions.

#### Acceptance Criteria

1. The interpolators module shall be located at `crates/pricer_core/src/math/interpolators/mod.rs`.
2. The solvers module shall be located at `crates/pricer_core/src/math/solvers/mod.rs`.
3. The `math` module shall re-export interpolator and solver types through `pub mod interpolators` and `pub mod solvers`.
4. Each interpolator implementation shall be in a separate submodule (e.g., `linear.rs`, `cubic_spline.rs`).
5. Each solver implementation shall be in a separate submodule (e.g., `newton_raphson.rs`, `brent.rs`).
6. All public types shall include rustdoc documentation with examples.

