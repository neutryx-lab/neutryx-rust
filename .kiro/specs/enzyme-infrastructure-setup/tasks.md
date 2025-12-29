# Implementation Plan

## Overview

Implementation tasks for Enzyme Infrastructure Setup (Phase 3.0). The pricer_kernel crate already exists with basic scaffolding; these tasks formalise and extend the infrastructure.

## Tasks

- [x] 1. Verify and Update Crate Configuration
- [x] 1.1 (P) Update Cargo.toml with description and complete metadata
  - Add description field explaining the crate's purpose as the Enzyme LLVM-level AD engine
  - Verify workspace inheritance for version, edition, license, repository, authors
  - Confirm llvm-sys dependency is set to version 180
  - Ensure num-traits workspace dependency is present
  - Verify approx is in dev-dependencies for testing
  - Confirm enzyme-ad feature flag exists and is correctly defined
  - Validate zero dependencies on pricer_core, pricer_models, or pricer_xva
  - _Requirements: 1.1, 1.3, 1.4, 3.1, 7.2_

- [x] 1.2 (P) Verify rust-toolchain.toml configuration
  - Confirm toolchain is pinned to nightly-2025-01-15
  - Verify rustfmt and clippy components are included
  - Add llvm-tools-preview component if missing
  - Ensure the file is located at crates/pricer_kernel/rust-toolchain.toml
  - _Requirements: 2.1, 2.2_

- [x] 2. Implement Build Script for LLVM Validation
- [x] 2.1 Create build.rs with LLVM version detection
  - Implement main function that orchestrates build validation
  - Add validate_llvm_version function to check for LLVM 18 availability
  - Use llvm-config command or environment variable to detect LLVM version
  - Emit cargo:rerun-if-env-changed directives for LLVM_CONFIG and ENZYME_LIB
  - Output cargo:warning with installation guidance when LLVM 18 is not found
  - Include URL to LLVM installation instructions in warning message
  - _Requirements: 3.2, 3.3, 3.4, 3.5, 7.3_

- [x] 2.2 Add Enzyme plugin configuration support
  - Implement configure_enzyme_plugin function for enzyme-ad feature
  - Check ENZYME_LIB environment variable for plugin path
  - Emit appropriate RUSTFLAGS configuration when enzyme-ad feature is enabled
  - Output clear warning with installation commands when Enzyme plugin is missing
  - Ensure graceful degradation when plugin is not available (Phase 3.0 uses placeholders)
  - _Requirements: 3.2, 3.5_

- [x] 3. Implement Enzyme Autodiff Module
- [x] 3.1 Create enzyme module with Activity enum and types
  - Create src/enzyme/mod.rs with module-level documentation
  - Implement Activity enum with Const, Dual, Active, and Duplicated variants
  - Add comprehensive doc comments explaining each activity annotation
  - Document Phase 3.0 placeholder status and Phase 4 integration plans
  - _Requirements: 4.1, 4.3, 7.1_

- [x] 3.2 Implement placeholder gradient function
  - Add generic gradient function that accepts a closure and input value
  - Implement placeholder logic using finite difference approximation
  - Include comprehensive doc comments explaining the function purpose
  - Document that actual Enzyme integration will replace this in Phase 4
  - Add inline attribute for performance
  - _Requirements: 4.2, 4.3, 7.1_

- [x] 3.3 Update lib.rs to export enzyme module
  - Uncomment or add enzyme module declaration in lib.rs
  - Re-export Activity enum and gradient function from crate root
  - Update crate-level documentation to describe enzyme module
  - Ensure public API is properly documented
  - _Requirements: 4.4, 7.1_

- [x] 4. Implement Gradient Verification Tests
- [x] 4.1 Create verify_enzyme.rs with dedicated verification tests
  - Create src/verify_enzyme.rs file for Enzyme-specific verification
  - Implement test for square function returning correct values
  - Implement test for square_gradient returning 2x at multiple points
  - Test gradient correctness at x = 1.0, 2.0, and 5.0
  - Use approx crate with epsilon tolerance of 1e-10
  - Add test for gradient at zero (edge case)
  - Add test for negative input values
  - _Requirements: 5.1, 5.2, 5.3, 5.4_

- [x] 4.2 Extend verify module with finite difference validation
  - Add test comparing analytical gradient against finite difference approximation
  - Use central difference formula: (f(x+h) - f(x-h)) / (2h)
  - Assert results match within appropriate tolerance
  - Document the verification approach in test comments
  - _Requirements: 5.2, 5.4_

- [x] 5. Validate Build Isolation and Integration
- [x] 5.1 Verify workspace exclusion build succeeds
  - Run cargo build --workspace --exclude pricer_kernel on stable toolchain
  - Confirm no Enzyme-related errors occur
  - Verify all other crates (pricer_core, pricer_models, pricer_xva) build successfully
  - Document the verification results
  - _Requirements: 1.5, 6.1, 6.3_

- [x] 5.2 Verify pricer_kernel builds with nightly toolchain
  - Run cargo +nightly build -p pricer_kernel
  - Confirm build succeeds with placeholder implementation
  - Verify all tests pass with cargo +nightly test -p pricer_kernel
  - Check that build warnings provide useful guidance when LLVM is missing
  - _Requirements: 2.3, 5.5, 6.4_

- [x] 5.3 Verify workspace member registration
  - Confirm pricer_kernel is listed in root Cargo.toml workspace members
  - Verify workspace dependency resolution works correctly
  - Test that cargo workspace commands include pricer_kernel appropriately
  - _Requirements: 1.2_

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1 | 1.1 |
| 1.2 | 5.3 |
| 1.3 | 1.1 |
| 1.4 | 1.1 |
| 1.5 | 5.1 |
| 2.1 | 1.2 |
| 2.2 | 1.2 |
| 2.3 | 5.2 |
| 2.4 | 3.3 |
| 3.1 | 1.1 |
| 3.2 | 2.1, 2.2 |
| 3.3 | 2.1 |
| 3.4 | 2.1 |
| 3.5 | 2.1, 2.2 |
| 4.1 | 3.1 |
| 4.2 | 3.2 |
| 4.3 | 3.1, 3.2 |
| 4.4 | 3.3 |
| 5.1 | 4.1 |
| 5.2 | 4.1, 4.2 |
| 5.3 | 4.1 |
| 5.4 | 4.1, 4.2 |
| 5.5 | 5.2 |
| 6.1 | 5.1 |
| 6.2 | 5.1 |
| 6.3 | 1.1, 5.1 |
| 6.4 | 5.2 |
| 7.1 | 3.1, 3.2, 3.3 |
| 7.2 | 1.1 |
| 7.3 | 2.1 |
