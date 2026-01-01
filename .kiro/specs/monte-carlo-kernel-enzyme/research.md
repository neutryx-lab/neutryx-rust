# Research & Design Decisions: Monte Carlo Kernel - Enzyme Integration

---
**Purpose**: Capture discovery findings and architectural decisions for the Monte Carlo kernel design.

**Discovery Scope**: Extension (enhancing existing Phase 3.2 implementation)
---

## Summary

- **Feature**: monte-carlo-kernel-enzyme
- **Discovery Scope**: Extension (existing implementation in `pricer_kernel/src/mc/`)
- **Key Findings**:
  1. Phase 3.2 implementation is substantially complete with bump-and-revalue Greeks
  2. Manual tangent propagation prototype exists (`price_with_delta_ad`)
  3. Enzyme integration requires replacing finite difference with `#[autodiff]` macros

## Research Log

### Existing Implementation Analysis

- **Context**: Analyse current `pricer_kernel/src/mc/` module to determine design scope
- **Sources Consulted**: Direct code analysis of existing implementation
- **Findings**:
  - `MonteCarloConfig` with builder pattern fully implemented
  - `PathWorkspace` provides allocation-free simulation via pre-allocated buffers
  - `generate_gbm_paths` uses log-space formulation for numerical stability
  - `MonteCarloPricer` orchestrates path generation, payoff, and Greeks
  - Greeks computed via central finite difference (bump-and-revalue)
  - Forward-mode AD prototype via `generate_gbm_paths_tangent_spot`
  - `Activity` enum defined in `enzyme/mod.rs`
- **Implications**: Design focuses on documenting existing architecture and specifying Enzyme integration path

### Enzyme Rust Integration Status

- **Context**: Verify current Enzyme Rust ecosystem status for `#[autodiff]` macros
- **Sources Consulted**: Enzyme project documentation, Rust nightly features
- **Findings**:
  - `#![feature(autodiff)]` available on nightly-2025-01-15
  - Enzyme operates at LLVM IR level, requiring LLVM 18
  - Forward mode via `#[autodiff_forward]`, reverse mode via `#[autodiff_reverse]`
  - Activity annotations: `Const`, `Dual`, `Active`, `Duplicated`, `DuplicatedOnly`
  - Current implementation uses finite difference as placeholder
- **Implications**: Phase 4 will add `#[autodiff]` macros; Phase 3.2 design validates architecture

### Buffer Hoisting for Enzyme Optimisation

- **Context**: Ensure memory layout supports Enzyme LLVM-level optimisation
- **Sources Consulted**: Enzyme documentation, existing workspace implementation
- **Findings**:
  - Vec allocations must occur outside simulation loop
  - `PathWorkspace` correctly hoists all allocations
  - Row-major contiguous layout enables SIMD vectorisation
  - `ensure_capacity` uses doubling strategy for amortised O(1) growth
- **Implications**: Existing buffer design is Enzyme-compatible; no changes needed

### Smooth Payoff Functions

- **Context**: Verify AD compatibility of payoff functions
- **Sources Consulted**: Existing `payoff.rs` implementation
- **Findings**:
  - `soft_plus(x, epsilon)` replaces `max(x, 0)` for differentiability
  - `soft_plus_derivative` provides gradient for tangent propagation
  - European call/put use smooth approximations
  - Asian payoff uses running sum (AD-compatible)
- **Implications**: Payoff design is AD-ready; no architectural changes needed

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Current (Finite Difference) | Bump-and-revalue Greeks | Simple, no Enzyme dependency | 3x cost per Greek, numerical error | Phase 3.2 baseline |
| Forward-Mode AD | Enzyme `#[autodiff_forward]` | Efficient for single input (Delta) | One Greek per pass | Prototype exists |
| Reverse-Mode AD | Enzyme `#[autodiff_reverse]` | Efficient for multiple inputs | Requires checkpointing | Phase 4 target |
| Hybrid Mode | Forward for Delta, Reverse for others | Optimal efficiency | Complexity | Recommended approach |

**Selected Pattern**: Hybrid Mode with current architecture preserved

## Design Decisions

### Decision: Preserve Existing Module Structure

- **Context**: Determine whether to refactor or extend existing implementation
- **Alternatives Considered**:
  1. Complete refactor with new abstractions
  2. Extend existing modules with Enzyme hooks
- **Selected Approach**: Extend existing modules
- **Rationale**: Phase 3.2 implementation is well-designed and tested; minimal changes needed
- **Trade-offs**: Must maintain backward compatibility with finite difference mode
- **Follow-up**: Add `#[cfg(feature = "enzyme-ad")]` guards for Enzyme-specific code

### Decision: Finite Difference as Fallback

- **Context**: Decide how to handle environments without Enzyme
- **Alternatives Considered**:
  1. Enzyme-only mode
  2. Dual-mode with runtime selection
  3. Compile-time feature flag
- **Selected Approach**: Compile-time feature flag (`enzyme-ad`)
- **Rationale**: Zero runtime overhead when Enzyme not used; aligns with L3 isolation
- **Trade-offs**: Two code paths to maintain
- **Follow-up**: Verification tests compare both modes for correctness

### Decision: Activity Analysis Documentation

- **Context**: Document parameter activities for Enzyme integration
- **Alternatives Considered**:
  1. Runtime activity configuration
  2. Type-level activity encoding
  3. Documentation-only (compile-time attributes)
- **Selected Approach**: Documentation + compile-time `#[autodiff]` attributes
- **Rationale**: Enzyme uses attribute-based activity; runtime config not needed
- **Trade-offs**: Activities fixed at compile time
- **Follow-up**: Doc comments already specify activities; ready for Phase 4

## Risks & Mitigations

- **Risk**: Enzyme nightly API changes before Phase 4
  - **Mitigation**: Pin to `nightly-2025-01-15`; monitor upstream changes

- **Risk**: Performance regression with AD enabled
  - **Mitigation**: Benchmark suite comparing AD vs finite difference

- **Risk**: Numerical accuracy differences between AD and finite difference
  - **Mitigation**: Verification tests in `verify/` module; tolerance thresholds defined

## References

- [Enzyme Project](https://enzyme.mit.edu/) — LLVM-level automatic differentiation
- [Rust Autodiff RFC](https://github.com/rust-lang/rust/pull/124509) — Enzyme Rust integration
- Existing implementation: `crates/pricer_kernel/src/mc/`
- RNG infrastructure: `crates/pricer_kernel/src/rng/`
- Enzyme module: `crates/pricer_kernel/src/enzyme/mod.rs`
