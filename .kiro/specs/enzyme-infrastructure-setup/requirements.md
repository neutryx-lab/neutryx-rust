# Requirements Document

## Project Description (Input)
Enzyme Infrastructure Setup for pricer_kernel - Set up automatic differentiation infrastructure using Enzyme/LLVM for the pricer_kernel crate. This crate will provide GPU-accelerated automatic differentiation capabilities for financial derivative pricing. The setup includes: (1) Creating the pricer_kernel crate with Rust nightly toolchain, (2) Configuring Enzyme LLVM bindings and llvm-sys dependencies, (3) Implementing verification tests demonstrating gradient computation (f(x)=x² → gradient=2x), (4) Ensuring complete isolation from pricer_core. Target language: British English.

## Introduction

This specification defines the requirements for establishing the foundational Enzyme automatic differentiation infrastructure within the `pricer_kernel` crate (Layer 3). The crate serves as the isolated AD engine layer, requiring Rust nightly toolchain and LLVM Enzyme plugin whilst maintaining strict independence from other workspace crates during initial setup.

## Requirements

### Requirement 1: Crate Creation and Workspace Integration

**Objective:** As a developer, I want to have a properly configured `pricer_kernel` crate within the Cargo workspace, so that Layer 3 AD functionality is structurally isolated and workspace-integrated.

#### Acceptance Criteria

1. The pricer_kernel crate shall exist at `crates/pricer_kernel/` with a valid `Cargo.toml` manifest.
2. The pricer_kernel crate shall be registered as a workspace member in the root `Cargo.toml`.
3. The pricer_kernel crate shall specify `edition = "2021"` in its manifest.
4. The pricer_kernel crate shall have zero dependencies on `pricer_core`, `pricer_models`, or `pricer_xva` crates.
5. When the workspace is built excluding pricer_kernel, the build shall succeed without Enzyme-related toolchain requirements.

### Requirement 2: Rust Nightly Toolchain Configuration

**Objective:** As a developer, I want the pricer_kernel crate to enforce Rust nightly toolchain usage, so that Enzyme LLVM plugin integration is properly supported.

#### Acceptance Criteria

1. The pricer_kernel crate shall include a `rust-toolchain.toml` file specifying nightly channel.
2. The rust-toolchain.toml shall pin the nightly version to `nightly-2025-01-15` for reproducibility.
3. When building pricer_kernel without nightly toolchain, the build shall fail with a clear error message indicating nightly requirement.
4. The pricer_kernel crate shall enable required nightly features via `#![feature(...)]` declarations where necessary.

### Requirement 3: LLVM and Enzyme Dependency Configuration

**Objective:** As a developer, I want proper LLVM and Enzyme bindings configured, so that automatic differentiation capabilities are available at the LLVM IR level.

#### Acceptance Criteria

1. The pricer_kernel Cargo.toml shall include `llvm-sys` dependency with appropriate LLVM version constraints.
2. The pricer_kernel crate shall configure Enzyme LLVM plugin path via environment variable `RUSTFLAGS` or build script.
3. If LLVM 18 is not available in the build environment, the build shall fail with a descriptive error message.
4. The pricer_kernel crate shall provide a `build.rs` script that validates Enzyme plugin availability.
5. Where Enzyme plugin is not installed, the build script shall output installation guidance.

### Requirement 4: Enzyme Autodiff Module Structure

**Objective:** As a developer, I want a well-organised module structure for Enzyme bindings, so that autodiff functionality is cleanly encapsulated and extensible.

#### Acceptance Criteria

1. The pricer_kernel crate shall contain an `enzyme/` module directory for autodiff bindings.
2. The enzyme module shall export a public `autodiff` macro or function for gradient computation.
3. The enzyme module shall provide type-safe wrappers around raw LLVM Enzyme calls.
4. The pricer_kernel crate shall re-export essential Enzyme functionality from `lib.rs`.

### Requirement 5: Gradient Verification Test

**Objective:** As a developer, I want a verification test demonstrating correct gradient computation, so that Enzyme integration is validated with a simple mathematical proof.

#### Acceptance Criteria

1. The pricer_kernel crate shall include `src/verify_enzyme.rs` containing verification tests.
2. When computing the gradient of `f(x) = x * x`, the Enzyme autodiff shall return `2 * x`.
3. The verification test shall assert gradient correctness for at least three distinct input values (e.g., x = 1.0, 2.0, 5.0).
4. The verification test shall use `approx` crate or equivalent for floating-point comparison with appropriate epsilon tolerance.

### Requirement 6: Build Isolation and CI Compatibility

**Objective:** As a developer, I want pricer_kernel builds isolated from stable crates, so that CI pipelines can build stable layers without Enzyme infrastructure.

#### Acceptance Criteria

1. When running `cargo build --workspace --exclude pricer_kernel`, the build shall succeed on stable Rust toolchain.
2. When running `cargo test --workspace --exclude pricer_kernel`, all tests shall pass without Enzyme dependencies.
3. The pricer_kernel crate shall not introduce any dependencies that pollute the stable workspace build.
4. If pricer_kernel is included in workspace build without proper Enzyme setup, the build shall fail gracefully with actionable error messages.

### Requirement 7: Documentation and Developer Guidance

**Objective:** As a developer, I want clear documentation on Enzyme setup and usage, so that contributors can understand and extend the AD infrastructure.

#### Acceptance Criteria

1. The pricer_kernel crate shall include inline documentation (`///` comments) for all public APIs.
2. The pricer_kernel `Cargo.toml` shall include a `description` field explaining the crate's purpose.
3. Where build prerequisites are not met, error messages shall include URLs or commands for resolution.
