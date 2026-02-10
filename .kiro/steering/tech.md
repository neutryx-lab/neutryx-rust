# Technology Stack

## Architecture

**A-I-P-S Unidirectional Data Flow**: The workspace enforces a strict unidirectional data flow mirroring alphabetical order (**A**dapter → **I**nfra → **P**ricer → **S**ervice). This logical progression guides developers from data ingestion to computation and finally to delivery.

```text
A: Adapter   → adapter_feeds, adapter_loader (incl. fpml feature)
I: Infra     → infra_config, infra_domain, infra_store
P: Pricer    → pricer_core (L1), pricer_models (L2), pricer_pricing (L3), pricer_risk (L4)
S: Service   → service_gateway (active), service_cli (paused), service_python (paused)
```

**Neutryx Facade Crate**: The workspace root (`neutryx` crate) provides a unified entry point for external consumers, re-exporting all underlying crates with intuitive aliases (`master`, `config`, `core`, `models`, `pricing`, `risk`). Feature flags (`minimal`, `analytics`, `full`) control which layers are included.

> **Note**: `pricer_optimiser` (L2.5) was removed in 2026-01. Its functionality consolidated: market data (curves, surfaces, bootstrapping, provider) → `pricer_models::market`, calibration engine → `pricer_models::market::calibration`.

**Dependency Rules**:
1. **S**ervices may depend on any **P**, **I**, or **A** crate.
2. **P**ricer crates must never depend on **S** or **A** crates.
3. **I**nfra crates must never depend on **P** or **S** crates.
4. **A**dapter crates depend only on **I** (for definitions) or **P** (for target types), never on **S**.

## Core Technologies

- **Language**: Rust Edition 2021
- **Default Toolchain**: Stable Rust (workspace default)
- **Nightly Toolchain**: `nightly-2025-01-15` (required only for `pricer_risk` with `enzyme-ad` feature)
- **Stable Compatibility**: All crates build on stable; Enzyme AD requires nightly
- **AD Backend**: Enzyme LLVM plugin (LLVM 18 required, via `pricer_risk::enzyme`)
- **Build System**: Cargo workspace with resolver = "2"

## Key Libraries

### Core
- **Numeric**: `num-traits`
- **Linear Algebra**: `nalgebra` (optional `linalg` feature, matrix operations, decompositions)
- **Optimisation**: `argmin`, `argmin-math` (optional, L-BFGS, Nelder-Mead via feature-gated wrappers)
- **Parallelisation**: `rayon` (portfolio-level parallelism)
- **Random**: `rand`, `rand_distr` (Monte Carlo, Ziggurat algorithm for normals)
- **Time**: `chrono` (date arithmetic, day count conventions)
- **LLVM Bindings**: `llvm-sys = "180"` (optional, `enzyme-ad` feature in pricer_risk)
- **Serialisation**: `serde` (optional, ISO 4217 currency support)
- **Error Handling**: `thiserror` (structured error types)
- **Testing**: `approx`, `proptest`, `criterion`
- **Benchmarking**: `criterion` (time-based), `iai-callgrind` (instruction-count for CI reproducibility)

### Adapter Layer
- **XML Parsing**: `quick-xml` (FpML parsing in adapter_loader, `fpml` feature)
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
- **WebSocket**: `axum` WebSocket, `futures-util` (service_gateway real-time updates)

### Demo Layer

- **TUI**: `ratatui`, `crossterm` (FrictionalBank TUI)
- **Web**: `axum`, `tower-http` (REST API and HTTP server)
- **WebSocket**: `tokio-tungstenite` (real-time dashboard)
- **Visualisation**: D3.js-compatible JSON graph export (computation DAG), Chart.js for charts
- **Frontend**: Vue 3 + Pinia + Tailwind CSS + Vite (demo dashboard with SFC architecture)
  - `vue` + `vue-router`: Component-based SPA with routing
  - `pinia`: State management
  - `tailwindcss`: Utility-first CSS
  - `chart.js`: Interactive charts and visualisations

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
- Verification tests: Enzyme vs bump-and-revalue for correctness
- Benchmarks: `criterion` for performance regression tracking

### Differentiability Requirements

- **Smooth Approximations**: Use `smooth_max`, `smooth_indicator` instead of `if` conditions
- **Enzyme-Friendly Loops**: Fixed-size `for` loops, avoid `while` and dynamic iteration
- **No Discontinuities**: All payoff functions smoothed (e.g., digital options → sigmoid)

## Development Environment

### Required Tools

- Rust stable (workspace default)
- Rust nightly-2025-01-15 (required only for pricer_risk with enzyme-ad feature)
- LLVM 18 (for Enzyme AD in pricer_risk)
- Docker (recommended for reproducible Enzyme builds)
- Google Cloud SDK (`gcloud`) for Cloud Run deployments (optional)

### Common Commands

```bash
# Dev (all crates - stable)
cargo build --workspace
cargo test --workspace

# Dev (with Enzyme AD - pricer_risk enzyme-ad feature)
export RUSTFLAGS="-C llvm-args=-load=/usr/local/lib/LLVMEnzyme-18.so"
cargo +nightly build -p pricer_risk --features enzyme-ad
cargo +nightly test -p pricer_risk --features enzyme-ad

# Docker (full Enzyme environment)
docker build -f docker/Dockerfile.nightly -t neutryx-enzyme .
docker run -it neutryx-enzyme
```

## Key Technical Decisions

| Decision | Rationale |
|----------|-----------|
| **A-I-P-S Architecture** | Unidirectional data flow from Adapters through Infrastructure and Pricing to Services |
| **Pricer Layer Hierarchy** | L1→L2→L3→L4 with Enzyme AD in L4 (pricer_risk) for risk integration |
| **Static Dispatch (enum)** | Enzyme performs better with concrete types than trait objects |
| **enum_dispatch Pattern** | Use `#[enum_dispatch]` macro for zero-cost trait dispatch via enums; apply to traits like `YieldCurve<T>`, `PathDependentPayoff` where multiple implementations exist |
| **StochasticModel Trait** | Unified interface for stochastic processes with enum-based dispatch |
| **Dual-Mode Verification** | Enzyme (performance) + bump-and-revalue (correctness) for validation |
| **Smooth Approximations** | Replace all discontinuities (if/max) with differentiable functions |
| **3-Stage Rocket Pattern** | Definition (L2) → Linking (PricingContext) → Execution (pure kernel); zero HashMap lookups in hot path |
| **IndexedMarket Pattern** | Market data keyed by `RateIndex`/`CurrencyPair` not strings; `TradeIndexRequirements` trait declares dependencies; `MarketValidator` checks completeness |
| **IR Compilation Pattern** | Trade (hierarchical) → TradeCompiler → PricingKernel (SoA, 64-byte aligned); `IndexMapper` converts indices to numeric IDs for SIMD-friendly access |
| **Kernel Engine Hierarchy** | LinearEngine (PricingKernel), ScriptEngine (ScriptKernel), CallableEngine (CallableKernel) with static dispatch via `CurveProvider<T>` trait |
| **LSMC Regression** | Longstaff-Schwartz Monte Carlo for Bermudan exercise; Cholesky-based regression, forward/backward pass, continuation value estimation |
| **Calibration Patterns** | Sequential (`curve::bootstrap`), Global (`curve::global`, feature-gated), Slice-wise (`vol::surface`, `vol::cube`) in `pricer_models::builder` |
| **Linear Solve Strategy** | Pluggable matrix solve strategies (`LUStrategy`, `LowerTriangularStrategy`) enable O(n²) vs O(n³) complexity; both store J⁻¹ for AAD via implicit function theorem |
| **Shadow Object Pattern** | Reverse mode AAD uses shadow buffers for gradient accumulation; `binder.rs` orchestrates market data → portfolio Greeks flow |
| **Feature Flag Coordination** | Features propagate through dependency chain (demo→frictional_bank→pricer_pricing) enabling modular compilation for different deployment scenarios |
| **Feature Flags** | `enzyme-mode`, `serde` for serialisation; Asset classes: `equity` (default), `rates`, `credit`, `fx`, `commodity`, `exotic`; Convenience: `all`; Integration: `l1l2-integration` |
| **Convention Registry Pattern** | `ConventionRegistry` with `ConventionKey` lookup; `ConventionSet` bundles per-currency conventions; `EventInstrument` models expected rate jumps at CB meetings |
| **ConventionTemplate Pattern** | `ConventionTemplate` generates multiple conventions across currencies from compact JSON with `{currency}`, `{index}` placeholders; reduces configuration duplication |
| **CurveDefinition Pattern** | `CurveDefinition` specifies curve recipes (rate_index, instruments[], calibration_method, interpolation); `CurveRegistry` resolves references at runtime |
| **MarketInstrument Pattern** | `MarketInstrument` combines rate data + convention → CF-expandable Trade; bridges market quotes to pricing |

## Performance Optimisation

- **LTO**: Link-time optimisation enabled in release profile
- **Single Codegen Unit**: `codegen-units = 1` for maximum optimisation
- **Structure of Arrays (SoA)**: Memory layout for vectorisation (pricer_risk)
- **64-byte SIMD Alignment**: `AlignedBuffer<T>` ensures cache-line and AVX-512 alignment for PricingKernel IR
- **Rayon Parallelism**: Portfolio-level parallel processing (>80% efficiency on 8+ cores)
- **Parallel Portfolio Greeks**: Batch processing for 1000+ trades with memory monitoring
- **Thread-local Buffers**: RAII buffer pools for zero-allocation hot paths

## Deployment Infrastructure

### Containerisation

- **Multi-stage Docker builds**: Separate Dockerfiles for stable, nightly, and web dashboard
  - `docker/Dockerfile.gui` for web dashboard (Cloud Run deployments)
  - `docker/Dockerfile.stable` for stable crates (all crates without enzyme-ad)
  - `docker/Dockerfile.nightly` for pricer_risk with Enzyme AD (enzyme-ad feature)
- **Cloud Run support**: Environment-based port binding (`PORT` env var), health endpoints (`/health`)
- **CI/CD**: Google Cloud Build pipeline (`docker/cloudbuild.yaml`) for automated build→push→deploy
  - Uses `-f docker/Dockerfile.gui` for web dashboard builds
- **Container registry**: GCR (Google Container Registry)
- **Target region**: Configurable via `docker/cloudbuild.yaml` substitutions (default: `asia-northeast1`)

### Observability

- **Performance Metrics**: Built-in metrics collection (API response times, WebSocket latency, server uptime)
- **Request Tracing**: `tower-http::TraceLayer` for HTTP request logging
- **Health Checks**: Kubernetes/Cloud Run-compatible readiness probes
- **OpenAPI/Swagger**: API documentation with `utoipa` (feature-gated `openapi`)
- **Prometheus Export**: Prometheus-style metrics endpoint (`/api/metrics`)

### Self-Healing CI

- **AI Fixer**: Automated CI failure remediation (`.github/ai_fixer/`, `.github/workflows/ai-fixer.yml`)
- **Pattern**: Parses CI error logs → gathers code context → generates fix patches via Gemini API → creates draft PR
- **Safety**: Confidence scoring, draft-only PRs, human review required

---
_Created: 2025-12-29_
_Updated: 2026-02-09_ — service_gateway re-enabled, AI Fixer CI, ndarray removed, QuoteId migration
_Document standards and patterns, not every dependency_
