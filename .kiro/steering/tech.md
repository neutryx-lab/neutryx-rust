# Technology Stack

## Architecture

**A-I-P-S Unidirectional Data Flow**: The workspace enforces a strict unidirectional data flow mirroring alphabetical order (**A**dapter → **I**nfra → **P**ricer → **S**ervice). This logical progression guides developers from data ingestion to computation and finally to delivery.

```text
A: Adapter   → adapter_feeds, adapter_fpml, adapter_loader
I: Infra     → infra_config, infra_master, infra_store
P: Pricer    → pricer_core (L1), pricer_models (L2), pricer_optimiser (L2.5), pricer_pricing (L3), pricer_risk (L4)
S: Service   → service_cli, service_gateway, service_python
```

**Dependency Rules**:
1. **S**ervices may depend on any **P**, **I**, or **A** crate.
2. **P**ricer crates must never depend on **S** or **A** crates.
3. **I**nfra crates must never depend on **P** or **S** crates.
4. **A**dapter crates depend only on **I** (for definitions) or **P** (for target types), never on **S**.

## Core Technologies

- **Language**: Rust Edition 2021
- **Nightly Toolchain**: `nightly-2025-01-15` (workspace default, pricer_pricing-first development)
- **Stable Compatibility**: All crates except pricer_pricing can build on stable
- **AD Backend**: Enzyme LLVM plugin (LLVM 18 required)
- **Build System**: Cargo workspace with resolver = "2"

## Key Libraries

### Core
- **Numeric**: `num-traits`, `num-dual` (verification mode)
- **Parallelisation**: `rayon` (portfolio-level parallelism)
- **Random**: `rand`, `rand_distr` (Monte Carlo, Ziggurat algorithm for normals)
- **Time**: `chrono` (date arithmetic, day count conventions)
- **LLVM Bindings**: `llvm-sys = "180"` (optional, `enzyme-ad` feature in pricer_pricing)
- **Serialisation**: `serde` (optional, ISO 4217 currency support)
- **Error Handling**: `thiserror` (structured error types)
- **Testing**: `approx`, `proptest`, `criterion`
- **Benchmarking**: `criterion` (time-based), `iai-callgrind` (instruction-count for CI reproducibility)

### Adapter Layer
- **XML Parsing**: `quick-xml` (FpML parsing in adapter_fpml)
- **File Formats**: `csv`, `parquet` (data loading in adapter_loader)
- **Market Data**: WebSocket/REST clients for adapter_feeds

### Infra Layer
- **Configuration**: `config` crate (TOML/YAML/Env vars in infra_config)
- **Database**: `sqlx` (async PostgreSQL in infra_store)
- **Caching**: `redis` (optional, state management)

### Service Layer
- **CLI**: `clap` (argument parsing in service_cli)
- **Python Bindings**: `pyo3` (service_python)
- **gRPC**: `tonic` (service_gateway)
- **REST**: `axum` (service_gateway)

### Demo Layer

- **TUI**: `ratatui`, `crossterm` (FrictionalBank TUI)
- **WebSocket**: `tokio-tungstenite` (real-time dashboard)

## Development Standards

### Type Safety

- Strict type checking, no `unsafe` except in Enzyme bindings
- Static dispatch via `enum` (not `Box<dyn Trait>`) for Enzyme optimisation
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
- LLVM 18 (for Enzyme in pricer_pricing)
- Docker (recommended for reproducible Enzyme builds)

### Common Commands

```bash
# Dev (stable crates only)
cargo build --workspace --exclude pricer_pricing
cargo test --workspace --exclude pricer_pricing

# Dev (with Enzyme - pricer_pricing)
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
| **A-I-P-S Architecture** | Unidirectional data flow from Adapters through Infrastructure and Pricing to Services |
| **Pricer Layer Hierarchy** | L1→L2→L2.5→L3→L4 isolates experimental Enzyme code |
| **Static Dispatch (enum)** | Enzyme performs better with concrete types than trait objects |
| **StochasticModel Trait** | Unified interface for stochastic processes with enum-based dispatch |
| **Dual-Mode Verification** | Enzyme (performance) + num-dual (correctness) for validation |
| **Smooth Approximations** | Replace all discontinuities (if/max) with differentiable functions |
| **3-Stage Rocket Pattern** | Definition (L2) → Linking (PricingContext) → Execution (pure kernel); zero HashMap lookups in hot path |
| **Feature Flags** | `num-dual-mode` (default), `enzyme-mode`, `serde` for serialisation; Asset classes: `equity` (default), `rates`, `credit`, `fx`, `commodity`, `exotic`; Convenience: `all` |

## Performance Optimisation

- **LTO**: Link-time optimisation enabled in release profile
- **Single Codegen Unit**: `codegen-units = 1` for maximum optimisation
- **Structure of Arrays (SoA)**: Memory layout for vectorisation (pricer_risk)
- **Rayon Parallelism**: Portfolio-level parallel processing

---
_Created: 2025-12-29_
_Updated: 2026-01-11_ — Added Demo Layer technologies (ratatui, tokio-tungstenite)
_Document standards and patterns, not every dependency_
