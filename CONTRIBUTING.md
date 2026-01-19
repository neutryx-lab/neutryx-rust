# Contributing to Neutryx

Thank you for contributing to Neutryx. This project serves as a bridge between academic research in quantitative finance and production-grade implementation.

To maintain the highest standards of numerical correctness and system robustness, we adhere to the following strict development guidelines.

---

## Core Philosophy: The Source of Truth

* **Code is the "How":** The code itself is the only source of truth for the procedural logic.
* **Comments are the "Why":** Comments must explain the *intent*, *mathematical derivation*, and *safety rationale*. They must not duplicate the code's logic.
* **Types are the Documentation:** Use the type system (e.g., Newtypes) to enforce semantic constraints.

---

## Documentation Standards

### Language
* All code, comments, documentation, and commit messages must be written in **British English** (e.g., *optimisation*, *behaviour*, *modelling*, *centred*).

### Public Interfaces (`///`)
All public structs and functions must have documentation comments including:
1.  **Summary:** A concise description of the functionality.
2.  **Mathematics:** References to papers or LaTeX equations explaining the model (e.g., *Heston (1993), Eq. 2*).
3.  **Contracts:** Explicit preconditions (e.g., "Volatility must be strictly positive") and panic conditions.

### Implementation Comments (`//`)
* **Allowed:** Explanations of numerical stability tricks (e.g., LogSumExp), hard-to-read hardware optimisations, and safety proofs for `unsafe` blocks.
* **Prohibited:** Direct translation of syntax, "TODO" comments (use issue tracker), and commented-out code.

---

## Coding Standards (Rust)

### Safety & Error Handling
* **No Panics:** Library code must never panic. Usage of `.unwrap()`, `.expect()`, or `panic!()` is strictly forbidden in the core library. Use `Result<T, Error>` for all fallible operations.
* **Arithmetic:** Use checked arithmetic or rigorous bounds checking where overflow is possible.

### Type System
* **Newtype Pattern:** Do not use raw `f64` for physically distinct quantities. Use semantic wrappers (e.g., `pub struct Strike(pub f64);`) to prevent unit errors.
* **Immutability:** Prefer immutable data structures. Interior mutability (`RefCell`, `Mutex`) should be minimised.

### Numerical Correctness
* **Floating Point:** Never compare floating-point numbers using `==`. Use `approx_eq!` or explicit epsilon checks.
* **NaN Propagation:** Ensure that `NaN`s are caught at the boundary.

---

## Testing Standards

* **Property-Based Testing:** We use `proptest` to verify mathematical invariants (e.g., Put-Call Parity, Monotonicity) over a wide range of inputs.
* **Regression Testing:** Numerical results must match reference implementations (C++ or Python prototypes) within a defined tolerance.

---

## Workflow for C++ Developers

* **Tooling:** We rely exclusively on `cargo` for build and dependency management. Do not check in `Makefile` or shell scripts unless absolutely necessary.
* **Linting:** Ensure `cargo clippy` passes without warnings before submitting a PR.

---

## Quick Start (< 30 minutes)

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- Git

### Setup

```bash
# Clone the repository
git clone https://github.com/torohash/neutryx-rust.git
cd neutryx-rust

# Install stable toolchain (for L1/L2/L4 layers)
rustup default stable

# Optional: Install nightly toolchain (for L3/pricer_pricing with Enzyme AD)
rustup toolchain install nightly

# Build the project
cargo build

# Run tests
cargo test

# Run Clippy
cargo clippy -- -D warnings
```

### Toolchain Guide

| Layer | Crate | Toolchain | Purpose |
|-------|-------|-----------|---------|
| L1 | pricer_core | stable | Core utilities, interpolation, types |
| L2 | pricer_models | stable | Financial models (Black-Scholes, Hull-White, CIR) |
| L3 | pricer_pricing | nightly | Enzyme AD, Monte Carlo |
| L4 | pricer_risk | stable | Risk analytics, XVA, portfolio |

**Note:** L3 (pricer_pricing) requires nightly for Enzyme AD features. For most contributions, stable toolchain is sufficient.

## Running Tests

```bash
# Run all tests (stable layers)
cargo test -p pricer_core -p pricer_models -p pricer_risk

# Run specific crate tests
cargo test -p pricer_core

# Run tests with output
cargo test -- --nocapture

# Run benchmarks (requires criterion)
cargo bench -p pricer_core
cargo bench -p pricer_models
cargo bench -p pricer_risk
```

## Code Quality Checks

Before submitting a PR, ensure your code passes these checks:

```bash
# Format code
cargo fmt

# Check formatting (CI runs this)
cargo fmt --check

# Run Clippy lints
cargo clippy -- -D warnings

# Build documentation
cargo doc --no-deps
```

## Pull Request Process

1. **Fork & Clone**: Fork the repository and clone your fork.

2. **Create a Branch**: Create a feature branch from `main`.
   ```bash
   git checkout -b feat/your-feature-name
   ```

3. **Make Changes**: Implement your changes following the coding standards.

4. **Test**: Ensure all tests pass and add new tests for your changes.

5. **Commit**: Use [Conventional Commits](https://www.conventionalcommits.org/) format.
   ```
   feat(pricer_core): add new interpolation method
   fix(pricer_risk): correct CVA calculation edge case
   docs(readme): update installation instructions
   ```

6. **Push & PR**: Push your branch and create a Pull Request.
   - Fill out the [PR template](.github/PULL_REQUEST_TEMPLATE.md)
   - Link related issues
   - Wait for CI checks to pass

7. **Review**: Address review feedback and update your PR.

8. **Merge**: Once approved and CI passes, your PR will be merged.

## Reporting Issues

- **Bug Reports**: Use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.yml)
- **Feature Requests**: Use the [feature request template](.github/ISSUE_TEMPLATE/feature_request.yml)

---

# Neutryx Development Protocol: Spec-Driven Development

## 1. Objective

To ensure mathematical consistency, reproducibility, and rigorous documentation across the Neutryx codebase by strictly adhering to a Specification-Driven Development (SDD) workflow.

## 2. Standards & Conventions

* **Documentation Language:** Japanese or British English.
* **Code & Comments:** British English.
* **Comment Style:** Minimal, essential, and strictly formal.
* **Verification:** All numerical implementations must include verifiable test cases (e.g., checking positive definiteness, convergence rates).

## 3. Workflow

### Phase I: Context Synchronisation

Execute upon session initialisation or significant architectural changes to align the AI agent's context.

```bash
> /kiro:steering
```

* **Action:** Verify that `tech.md` accurately reflects the latest JAX/Rust crate versions.

### Phase II: Specification & Design

**Do not commence coding without an approved design.**

1. **Initialisation:** Define the scope of the new module or feature.
```bash
> /kiro:spec-init "Brief description of the feature"
```


2. **Requirements Definition:** Define acceptance criteria and mathematical constraints.
```bash
> /kiro:spec-requirements <feature-name>
```


3. **Technical Design:** Generate the architecture and interface definitions.
```bash
> /kiro:spec-design <feature-name>
```


* **Mandate:** Review `design.md` to ensure variable naming conventions and interface signatures are logically consistent.


4. **Task Breakdown:** Decompose the design into atomic implementation steps.
```bash
> /kiro:spec-tasks <feature-name>
```



### Phase III: Implementation (TDD)

Execute implementation tasks sequentially.

```bash
> /kiro:spec-impl <feature-name> <task-id>
```

* **Constraint:** If the AI suggests deviations from `design.md`, reject the change and enforce the original specification.

### Phase IV: Status & Verification

Monitor progress and sign off on artefacts.

```bash
> /kiro:spec-status <feature-name>
```
