# Technology Stack

## Architecture

**4-Layer Isolation Design**: Experimental AD technology confined to Layer 3, enabling 75% of codebase to use stable Rust while isolating Enzyme (nightly Rust + LLVM plugin) dependencies.

```text
L1: pricer_core     → Foundation (Stable)
L2: pricer_models   → Business Logic (Stable)
L3: pricer_pricing  → AD Engine (Nightly + Enzyme, currently isolated)
L4: pricer_risk     → Application (Stable)
```

**Phase 3.0 Note**: L3 currently has zero pricer_* dependencies (complete isolation). L1/L2 integration planned for Phase 4.

## Core Technologies

- **Language**: Rust Edition 2021
- **Nightly Toolchain**: `nightly-2025-01-15` (workspace default, L3-first development)
- **Stable Compatibility**: L1/L2/L4 (pricer_core, pricer_models, pricer_risk) can build on stable when excluding L3
- **AD Backend**: Enzyme LLVM plugin (LLVM 18 required)
- **Build System**: Cargo workspace with resolver = "2"

## Key Libraries

- **Numeric**: `num-traits`, `num-dual` (verification mode)
- **Parallelization**: `rayon` (portfolio-level parallelism)
- **Random**: `rand`, `rand_distr` (Monte Carlo, Ziggurat algorithm for normals)
- **Time**: `chrono` (date arithmetic, day count conventions)
- **LLVM Bindings**: `llvm-sys = "180"` (optional, `enzyme-ad` feature in L3)
- **Serialization**: `serde` (optional, ISO 4217 currency support)
- **Error Handling**: `thiserror` (structured error types)
- **Testing**: `approx`, `proptest`, `criterion`
- **Benchmarking**: `criterion` (time-based), `iai-callgrind` (instruction-count for CI reproducibility)

## Development Standards

### Type Safety

- Strict type checking, no `unsafe` except in Enzyme bindings
- Static dispatch via `enum` (not `Box<dyn Trait>`) for Enzyme optimization
- Per-instrument configurable `smoothing_epsilon` for differentiability

### Code Quality

- `cargo fmt --all -- --check` (formatting)
- `cargo clippy --all-targets -- -D warnings` (linting)
- Property-based testing with `proptest` for mathematical invariants

### Testing

- Unit tests per module (traits, smoothing, instruments)
- Verification tests: Enzyme vs num-dual for correctness
- Benchmarks: `criterion` for performance regression tracking

### Differentiability Requirements

- **Smooth Approximations**: Use `smooth_max`, `smooth_indicator` instead of `if` conditions
- **Enzyme-Friendly Loops**: Fixed-size `for` loops, avoid `while` and dynamic iteration
- **No Discontinuities**: All payoff functions smoothed (e.g., digital options → sigmoid)

## Development Environment

### Required Tools

- Rust nightly-2025-01-15 (workspace default)
- LLVM 18 (for Enzyme in L3)
- Docker (recommended for reproducible Enzyme builds)

### Common Commands

```bash
# Dev (stable crates only)
cargo build --workspace --exclude pricer_pricing
cargo test --workspace --exclude pricer_pricing

# Dev (with Enzyme - L3)
export RUSTFLAGS="-C llvm-args=-load=/usr/local/lib/LLVMEnzyme-18.so"
cargo +nightly build -p pricer_pricing
cargo +nightly test -p pricer_pricing

# Docker (full Enzyme environment)
docker build -f docker/Dockerfile.nightly -t neutryx-enzyme .
docker run -it neutryx-enzyme
```

## Key Technical Decisions

| Decision | Rationale |
|----------|-----------|
| **4-Layer Architecture** | Isolate experimental Enzyme code (L3) from stable production code (L1/L2/L4) |
| **Static Dispatch (enum)** | Enzyme performs better with concrete types than trait objects |
| **StochasticModel Trait** | Unified interface for stochastic processes with enum-based dispatch |
| **Dual-Mode Verification** | Enzyme (performance) + num-dual (correctness) for validation |
| **Smooth Approximations** | Replace all discontinuities (if/max) with differentiable functions |
| **Feature Flags** | `num-dual-mode` (default), `enzyme-mode`, `serde` for serialization; Asset classes: `equity` (default), `rates`, `credit`, `fx`, `commodity`, `exotic`; Convenience: `all` |

## Performance Optimization

- **LTO**: Link-time optimization enabled in release profile
- **Single Codegen Unit**: `codegen-units = 1` for maximum optimization
- **Structure of Arrays (SoA)**: Memory layout for vectorization (L4)
- **Rayon Parallelism**: Portfolio-level parallel processing

---
_Created: 2025-12-29_
_Updated: 2026-01-09_ — Updated feature flags (added commodity, exotic, all)
_Document standards and patterns, not every dependency_
