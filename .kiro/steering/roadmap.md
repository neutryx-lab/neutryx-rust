# Development Roadmap

実装状況と今後の開発項目を追跡するドキュメント。

_Updated: 2026-01-19_

---

## Current State Summary

### Completed Specifications (17)

| Spec | Description | Completed |
|------|-------------|-----------|
| core-traits-types-2 | Core traits and type definitions | 2025-12 |
| rng-infrastructure | Random number generation (PRNG/QMC) | 2025-12 |
| enzyme-infrastructure-setup | Enzyme AD infrastructure | 2025-12 |
| interpolation-solvers | Interpolation and numerical solvers | 2025-12 |
| market-data-structures | Yield curves and volatility surfaces | 2025-12 |
| instrument-definitions | Financial instrument definitions | 2025-12 |
| monte-carlo-kernel-enzyme | Monte Carlo pricing kernel | 2026-01 |
| service-layer-rename | Crate renaming (kernel→pricing, xva→risk) | 2026-01 |
| stochastic-models | Heston, SABR, Hull-White stochastic models | 2026-01 |
| enzyme-autodiff-integration | Enzyme #[autodiff] macro integration | 2026-01 |
| frictional-bank | FrictionalBank demo system (TUI, Web, Workflows) | 2026-01 |
| frictionalbank-irs-bootstrap-risk | IRS bootstrapping and risk workflows | 2026-01-14 |
| frictional-bank-webapp-polish | Web dashboard UX improvements (78 tasks) | 2026-01-15 |
| frictionalbank-webapp-pricer | Web dashboard pricer integration (19 tasks) | 2026-01-16 |
| advanced-sensitivity-webapp | Advanced sensitivity analysis for web dashboard | 2026-01-19 |
| codebase-cleanup-optimisation | Codebase cleanup and optimisation (16 tasks) | 2026-01-19 |
| portfolio-graph-optimisation | Portfolio Graph REST API and WebSocket handlers | 2026-01-19 |

### Layer Implementation Status

```text
Legend: ✅ Complete | 🔶 Basic/Partial | ❌ Not Started
```

#### Pricer Layer (P) - Core Engine
| Crate | Layer | Status | Notes |
|-------|-------|--------|-------|
| pricer_core | L1 | ✅ | math, market_data, types, traits |
| pricer_models | L2 | ✅ | instruments, analytical, calibration, schedules, demo |
| pricer_optimiser | L2.5 | ✅ | bootstrapping, calibration, solvers, provider |
| pricer_pricing | L3 | ✅ | mc, rng, enzyme, greeks, path_dependent, checkpoint, context (l1l2-integration) |
| pricer_risk | L4 | ✅ | portfolio, exposure, xva, scenarios (engine/shifts/aggregator/presets), soa, demo |

#### Infra Layer (I) - Foundation
| Crate | Status | Notes |
|-------|--------|-------|
| infra_config | ✅ | Settings loading (TOML/YAML/Env) |
| infra_master | ✅ | Calendars, day count conventions |
| infra_store | 🔶 | Basic traits only, postgres optional |

#### Adapter Layer (A) - Input
| Crate | Status | Notes |
|-------|--------|-------|
| adapter_feeds | 🔶 | Basic quote types only |
| adapter_fpml | 🔶 | Basic FpML parser skeleton |
| adapter_loader | 🔶 | CSV loader, CSA terms |

#### Service Layer (S) - Output
| Crate | Status | Notes |
|-------|--------|-------|
| service_cli | 🔶 | Basic commands (calibrate/price/report/demo) |
| service_gateway | ✅ | REST API (price, portfolio, graph), WebSocket, gRPC skeleton |
| service_python | 🔶 | Basic bindings (VanillaOption, Forward, HullWhite) |

---

## Future Development Items

### Phase 1: Enzyme AD Full Integration (Priority: High) ✅

| Item | Description | Status |
|------|-------------|--------|
| enzyme-full-integration | Full `#[autodiff]` macro integration | ✅ |
| greeks-enzyme-ad | Delta/Gamma/Vega computation via Enzyme AD | ✅ |
| irs-greeks-workflow | IRS Greeks with AAD vs Bump-and-Revalue | ✅ |
| computation-graph | DAG visualisation for debugging | ✅ |
| enzyme-benchmarks | Criterion benchmarks (requires LLVM 18 runtime) | ⏳ Deferred |

**Note**: Core implementation complete including IRS Greeks workflow (lazy evaluation, benchmarks, XVA demo). Benchmarks deferred pending LLVM 18 environment.

### Phase 2: Additional Products & Models (Priority: Medium)

| Item | Description | Status |
|------|-------------|--------|
| exotic-options | Barriers, Asians, Lookbacks, Digitals | ❌ |
| rates-instruments | Swaption, Cap/Floor pricing completion | ❌ |
| stochastic-models | Heston, SABR, Hull-White completion | ✅ |
| volatility-surface | Implied vol surface construction & interpolation | ❌ |

### Phase 3: Service Layer Enhancement (Priority: Medium)

| Item | Description | Status |
|------|-------------|--------|
| grpc-implementation | gRPC service with streaming support | ❌ |
| python-bindings-expansion | Expand PyO3 bindings (MC Pricer, Greeks, etc.) | ❌ |
| cli-commands-completion | Complete CLI commands implementation | ❌ |
| cloud-deployment | Cloud Run deployment infrastructure | ✅ |
| dual-mode-ui | TUI + Web dashboard | ✅ |
| rest-orchestration | REST API for workflow orchestration | ✅ |
| openapi-documentation | OpenAPI/Swagger documentation | ✅ |
| scenario-analysis-api | Scenario analysis REST endpoints | ✅ |
| async-job-management | Background job processing infrastructure | ✅ |
| prometheus-metrics | Prometheus-style metrics export | ✅ |

### Phase 4: Adapter Layer Enhancement (Priority: Low)

| Item | Description | Status |
|------|-------------|--------|
| fpml-parser-complete | Complete FpML/XML parser (IRS, Swaption, etc.) | ❌ |
| market-data-feeds | Real-time market data connectivity | ❌ |
| parquet-loader | Parquet file loader implementation | ❌ |

### Phase 5: Production Hardening (Priority: Low-Medium)

| Item | Description | Status |
|------|-------------|--------|
| performance-optimization | Benchmarks, SIMD, parallelisation improvements | ❌ |
| error-handling | Unified error handling, logging enhancement | ❌ |
| documentation | API docs, user guides, examples | ❌ |
| testing-coverage | Test coverage, property-based testing | ❌ |

---

### Active Specifications (Ready for Implementation)

| Spec | Phase | Description |
|------|-------|-------------|
| infra-primitives-migration | tasks-generated | Financial primitives migration to infra_master |

## Recommended Next Steps

1. **exotic-options** - Barriers, Asians, Lookbacks, Digitals for product coverage
2. **python-bindings-expansion** - Enables Jupyter research workflows
3. **volatility-surface** - Implied vol surface construction and interpolation

---

## Changelog

| Date | Change |
|------|--------|
| 2026-01-19 | Steering sync: portfolio-graph-optimisation completed, service_gateway upgraded to ✅, infra-primitives-migration active |
| 2026-01-19 | codebase-cleanup-optimisation: Complete (16 tasks, all phases verified). Total completed specs: 16 |
| 2026-01-19 | Steering sync: Added 3 completed specs (webapp-polish, webapp-pricer, irs-bootstrap-risk), added advanced-sensitivity-webapp to active specs |
| 2026-01-16 | Steering sync: OpenAPI/Swagger documentation, scenario analysis handlers, async job manager, Prometheus metrics, parallel portfolio Greeks |
| 2026-01-15 | Steering sync: Cloud Run deployment, dual-mode UI, REST orchestration, computation graph, IRS Greeks workflow |
| 2026-01-14 | frictional-bank: Added IRS AAD workflow and benchmark visualisation module |
| 2026-01-11 | frictional-bank: Complete (all tasks including optional Chart and Web dashboard) |
| 2026-01-10 | enzyme-autodiff-integration: Complete (18/20 tasks, benchmarks deferred) |
| 2026-01-10 | stochastic-models: Implementation complete (17/17 tasks) |
| 2026-01-10 | stochastic-models: Calibration complete (Heston, SABR, Hull-White calibrators) |
| 2026-01-10 | Steering sync: Added scenarios, demo, context, provider modules to layer status |
| 2026-01-10 | Initial creation; cleaned 20 incomplete specs, documented current state |
