# Research & Design Decisions: Enzyme Infrastructure Setup

**Purpose**: Document discovery findings for Enzyme AD infrastructure in pricer_kernel.

## Summary

- **Feature**: `enzyme-infrastructure-setup`
- **Discovery Scope**: Complex Integration (nightly Rust + LLVM plugin)
- **Key Findings**:
  - Rust nightly provides `#[autodiff_forward]` and `#[autodiff_reverse]` macros via Enzyme
  - LLVM 18 is required; llvm-sys 180 provides Rust bindings
  - Existing pricer_kernel crate has placeholder implementation ready for upgrade

## Research Log

### Enzyme AD Integration Status in Rust

**Context**: Determining current state of Enzyme support in Rust nightly toolchain

**Findings**:
- `std::intrinsics::autodiff` available in nightly Rust
- Attribute macros `#[autodiff_forward]` and `#[autodiff_reverse]` expand to intrinsic calls
- Function signature: `autodiff<F, G, T, R>(f: F, df: G, args: T) -> R`
- Requires `#![feature(autodiff)]` or `#![feature(core_intrinsics)]`

**Implications**: Phase 3.0 can use placeholder; Phase 4 will integrate actual Enzyme macros

### LLVM Version Requirements

**Context**: Determining required LLVM version for Enzyme compatibility

**Findings**:
- Enzyme requires LLVM 18 for current nightly compatibility
- llvm-sys version 180 corresponds to LLVM 18.x
- RUSTFLAGS must include `-C llvm-args=-load=/path/to/LLVMEnzyme-18.so`

**Implications**: Build script should validate LLVM 18 presence; Docker recommended for reproducibility

### Existing Crate Structure Analysis

**Context**: Analysing current pricer_kernel implementation

**Findings**:
- rust-toolchain.toml already specifies `nightly-2025-01-15`
- Cargo.toml has `llvm-sys = "180"` dependency
- verify module contains placeholder `square` and `square_gradient` functions
- Comprehensive test suite with 7 tests including finite difference validation
- enzyme/mc/checkpoint modules exist but are commented out

**Implications**: Infrastructure largely in place; design focuses on formalising contracts and enabling actual Enzyme integration

## Design Decisions

### Decision: Placeholder Implementation for Phase 3.0

**Selected Approach**: Placeholder using analytical derivatives (2x for x²)

**Rationale**: Allows test suite validation, CI/CD setup, and documentation without Enzyme dependency

**Trade-offs**: Tests pass but don't verify actual AD; clear TODO markers for Phase 4

### Decision: Feature Flag Architecture

**Selected Approach**: Feature flag `enzyme-ad` in Cargo.toml

**Rationale**: Allows `--workspace --exclude pricer_kernel` builds; feature enables actual AD

**Trade-offs**: Conditional compilation complexity

### Decision: Nightly Toolchain Pinning

**Selected Approach**: Pin to `nightly-2025-01-15`

**Rationale**: Reproducible builds; known-working version with Enzyme

**Trade-offs**: May miss newer features/fixes; requires periodic updates

## Risks & Mitigations

- **Risk 1**: Enzyme plugin not available in CI environment → Mitigation: Docker image with pre-installed Enzyme; `--exclude pricer_kernel` for stable jobs
- **Risk 2**: LLVM version mismatch → Mitigation: build.rs validates LLVM 18; clear error messages with installation guidance
- **Risk 3**: Nightly API instability → Mitigation: Pin specific nightly version; comprehensive test suite catches regressions

## References

- [Enzyme AD Project](https://enzyme.mit.edu/)
- [Rust autodiff intrinsic](https://doc.rust-lang.org/nightly/std/intrinsics/fn.autodiff.html)
- [Enzyme GitHub](https://github.com/EnzymeAD/Enzyme)
