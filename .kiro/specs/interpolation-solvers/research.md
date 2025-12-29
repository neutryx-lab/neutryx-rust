# Research & Design Decisions

---
**Purpose**: Capture discovery findings, architectural investigations, and rationale for interpolation and solver infrastructure.
---

## Summary
- **Feature**: `interpolation-solvers`
- **Discovery Scope**: Extension (adding modules to existing math infrastructure)
- **Key Findings**:
  - Existing `T: num_traits::Float` pattern enables seamless AD compatibility
  - Smoothing functions provide foundation for `smooth_interp` via `smooth_indicator`
  - Error type patterns established in `types/error.rs` guide new error definitions
  - Cubic spline requires tridiagonal solver; Fritsch-Carlson for monotonicity

## Research Log

### Cubic Spline Algorithm Selection
- **Context**: Requirement 3 specifies natural cubic spline with C² continuity
- **Sources Consulted**: Numerical Recipes, scipy.interpolate documentation
- **Findings**:
  - Natural cubic splines require solving tridiagonal system for second derivatives
  - Thomas algorithm (O(n)) optimal for tridiagonal systems
  - Boundary conditions: natural (M₀ = Mₙ = 0) simplest to implement
- **Implications**: Implement internal tridiagonal solver; coefficients (a,b,c,d) stored per segment

### Monotonic Interpolation Method
- **Context**: Requirement 4 requires monotonicity preservation for discount factors
- **Sources Consulted**: Fritsch-Carlson (1980), Steffen (1990)
- **Findings**:
  - Fritsch-Carlson method modifies Hermite slopes to preserve monotonicity
  - Algorithm: compute secants, initialise slopes, apply monotonicity correction
  - Steffen alternative more conservative but may overshoot less
- **Implications**: Fritsch-Carlson selected for balance of smoothness and monotonicity

### Smooth Interpolation Design
- **Context**: Requirement 5 demands branch-free interpolation using existing smoothing primitives
- **Sources Consulted**: Existing `smooth_indicator` implementation in crate
- **Findings**:
  - `smooth_indicator(x - x_i, ε)` produces soft selection weights
  - Sum of weighted linear segments: `Σ w_i * lerp(x_i, y_i, x_{i+1}, y_{i+1})`
  - Weights must be normalised to sum to 1
- **Implications**: Use `smooth_indicator` for segment blending; epsilon controls transition sharpness

### Newton-Raphson AD Integration
- **Context**: Requirement 8.4 specifies AD-powered root finding
- **Sources Consulted**: num-dual documentation, autodiff patterns
- **Findings**:
  - `Dual64::derivative()` sets ∂x/∂x = 1 for differentiation
  - Single function evaluation f(x_dual) yields both value and derivative
  - Extract `re()` for function value, `eps` for derivative
- **Implications**: `find_root_ad` evaluates `f(Dual64::from(x).derivative())` per iteration

### Brent's Method Implementation
- **Context**: Requirement 9 needs robust bracket-based root finding
- **Sources Consulted**: Numerical Recipes, Boost Math documentation
- **Findings**:
  - Combines bisection, secant, inverse quadratic interpolation
  - Guaranteed convergence with bracketing interval
  - Machine epsilon tolerance achievable
- **Implications**: Implement full Brent algorithm with all three interpolation modes

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Trait-based polymorphism | Generic `Interpolator<T>` trait | Uniform interface, extensible | Enum dispatch preferred per steering | Selected |
| Enum dispatch | InterpolatorKind enum with match | Enzyme-optimised, static dispatch | Less extensible | Defer to L2/L3 |
| Builder pattern | Fluent construction | Good UX | Complexity overhead | Use for solver config |

## Design Decisions

### Decision: Trait-Based Generic Design
- **Context**: Need unified interface for multiple interpolator types with AD support
- **Alternatives Considered**:
  1. Concrete structs with separate functions — less composable
  2. Enum-based dispatch — steering recommends for L2/L3, not foundational traits
- **Selected Approach**: Generic trait `Interpolator<T: Float>` with struct implementations
- **Rationale**: L1 foundation establishes contracts; enum dispatch added in L2 if needed
- **Trade-offs**: Trait objects not Enzyme-friendly, but concrete generics are
- **Follow-up**: Verify Enzyme compatibility in L3 verification tests

### Decision: Separate Error Types per Domain
- **Context**: Interpolation and solver errors have distinct failure modes
- **Alternatives Considered**:
  1. Single unified `MathError` enum — less specific
  2. Extend `PricingError` — semantic mismatch
- **Selected Approach**: `InterpolationError` and `SolverError` as separate enums
- **Rationale**: Domain-specific errors enable precise error handling
- **Trade-offs**: More types to maintain, but clearer semantics

### Decision: Coefficient Storage for Splines
- **Context**: Cubic spline evaluation requires polynomial coefficients
- **Alternatives Considered**:
  1. Recompute on each call — expensive
  2. Store coefficients at construction — memory vs compute trade-off
- **Selected Approach**: Store precomputed coefficients (a, b, c, d) per segment
- **Rationale**: Interpolation called frequently; amortise construction cost
- **Trade-offs**: Higher memory for large datasets, but standard practice

## Risks & Mitigations
- **Risk**: Tridiagonal solver numerical stability for large datasets
  - **Mitigation**: Use pivoting-free Thomas algorithm; add condition number check
- **Risk**: Smooth interpolation convergence for very small epsilon
  - **Mitigation**: Document recommended epsilon range; add warning for extreme values
- **Risk**: Newton-Raphson divergence for non-smooth functions
  - **Mitigation**: Recommend Brent for unknown functions; add iteration limit

## References
- [Fritsch-Carlson Monotonic Interpolation](https://doi.org/10.1137/0717021) — SIAM 1980
- [Numerical Recipes in C, Chapter 3](https://numerical.recipes/) — Spline algorithms
- [num-dual crate documentation](https://docs.rs/num-dual) — Dual number types
- [scipy.interpolate.CubicSpline](https://docs.scipy.org/doc/scipy/reference/generated/scipy.interpolate.CubicSpline.html) — Reference implementation
