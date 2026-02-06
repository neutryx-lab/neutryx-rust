# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Removed

#### num-dual Dependency Removal

- **Removed `num-dual` crate dependency** from workspace
  - Removed `num-dual-mode` feature flag from `pricer_core` and `pricer_models`
  - Greeks calculation now uses bump-and-revalue as the default method
  - Enzyme AD remains available via the `enzyme-ad` feature flag

- **Removed Files and Types**:
  - Deleted `crates/pricer_core/src/types/dual.rs` and associated tests
  - Removed `GreeksMode::NumDual` variant from all Greeks enums
  - Removed `NewtonRaphsonSolver::find_root_ad()` method
  - Removed `VegaCalculator::compute_vega_ad()` num-dual version (kept bump-and-revalue version)
  - Removed `SensitivityBootstrapper::bootstrap_with_sensitivities()` num-dual version
  - Removed `SensitivityVerification` struct

- **Updated CI/CD**:
  - Removed `num-dual-mode` test step from GitHub Actions workflow
  - Updated fallback messages to reference bump-and-revalue instead of num-dual

### Added

#### External Numerics Migration (`external-numerics` feature)

- **New Feature Flag**: `external-numerics` (enabled by default in `pricer_core`)
  - Provides access to battle-tested external implementations of optimisation algorithms
  - Falls back gracefully when disabled, using internal implementations

- **New External Implementations**:
  - `minimize_nelder_mead_external` - Nelder-Mead simplex optimisation via `argmin` crate
  - `minimize_lbfgs_external` - L-BFGS quasi-Newton optimisation via `argmin` crate
  - `minimize_lbfgs_numerical_external` - L-BFGS with numerical gradient
  - `solve_lm_external` - Levenberg-Marquardt nonlinear least-squares via `levenberg-marquardt` crate

- **New Dependencies** (optional, behind `external-numerics` feature):
  - `argmin` 0.10 - Optimisation framework
  - `argmin-math` 0.4 - Mathematical backend (with `nalgebra_latest` feature)
  - `levenberg-marquardt` 0.14 - LM solver implementation

- **New Error Variants**:
  - `OptimisationError::External(String)` - Wraps errors from external optimisation crates
  - `SolverError::External(String)` - Wraps errors from external solver crates

- **Regression Tests**: 14 new tests comparing internal vs external implementations
  - Validates numerical precision within 10x tolerance
  - Validates iteration counts within reasonable bounds

### Changed

- Internal implementations (`minimize_nelder_mead`, `minimize_lbfgs`, `LevenbergMarquardtSolver`)
  remain available as fallback and for AD compatibility
- Module documentation updated with feature flag information and behavioural differences

### Notes

#### Migration Guide

No breaking changes to existing API. External implementations are additive:

```rust
// Internal implementation (unchanged)
use pricer_core::math::optimisers::minimize_nelder_mead;

// External implementation (new, requires external-numerics feature)
#[cfg(feature = "external-numerics")]
use pricer_core::math::optimisers::minimize_nelder_mead_external;
```

#### AD Compatibility

External implementations only support `f64`. For automatic differentiation with `Dual64`,
continue using internal implementations:

```rust
// AD-compatible (internal implementation)
use pricer_core::math::solvers::NewtonRaphsonSolver;
let solver = NewtonRaphsonSolver::new(config);
let root = solver.find_root_ad(f, x0)?;  // Works with Dual64

// Not AD-compatible (external implementation)
#[cfg(feature = "external-numerics")]
use pricer_core::math::solvers::solve_lm_external;
// solve_lm_external only accepts f64
```

#### Dependency Notes

- `nalgebra` 0.32/0.33 duplicate is expected due to `argmin-math` constraints
- Run `cargo tree --duplicates` to verify no unexpected duplicates
