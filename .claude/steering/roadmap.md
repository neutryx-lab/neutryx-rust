# Development Roadmap

実装状況と今後の開発項目を追跡するドキュメント。

_Updated: 2026-02-13_ — spec workflow lightweight化: 53 completed specs archived to `git history`; 4 active specs

---

## Current State Summary

### Archived Specifications

53 completed specs were removed (git history preserved). Key milestones:

- **2025-12**: Core foundations (traits, types, RNG, interpolators, market data, instruments)
- **2026-01**: Pricing engines (MC, stochastic models, Enzyme AD, tree pricing, PricingKernel IR), calibration (curve bootstrap, FX vol, IR VolCube, global solver), webapp (Ergodic Bank, dashboards), infrastructure (codebase cleanup, migrations)
- **2026-02**: Boilerplate reduction (bon, derive_more, enum_dispatch, thiserror), service consolidation (CLI/Python → service_gateway features)

### Layer Implementation Status

```text
Legend: ✅ Complete | 🔶 Basic/Partial | ❌ Not Started
```

#### Pricer Layer (P) - Core Engine
| Crate | Layer | Status | Notes |
|-------|-------|--------|-------|
| pricer_core | L1 | ✅ | math (smoothing, distributions, calculus, utilities, interpolators, solvers, integrators, optimisers, fitting, mesh, linalg), types, traits, ir (AlignedBuffer, PricingKernel, ScriptKernel) |
| pricer_models | L2 | ✅ | instruments, market (curves, surfaces, calibration, provider), models, schedules, analytical, compiler (IndexMapper, TradeCompiler), demo |
| pricer_pricing | L3 | ✅ | mc, rng, greeks, structured, checkpoint, context (l1l2-integration), pricer |
| pricer_risk | L4 | ✅ | portfolio, exposure, xva, scenarios (engine/shifts/aggregator/presets), soa, enzyme (AD, shadow, kernel, binder), demo |

> **Note**: `pricer_optimiser` (L2.5) was removed in 2026-01. All market data (curves, surfaces, bootstrapping, provider, calibration) consolidated into `pricer_models::market`.

#### Infra Layer (I) - Foundation
| Crate | Status | Notes |
|-------|--------|-------|
| infra_config | ✅ | Settings loading (TOML/YAML/Env) |
| infra_domain | ✅ | time/, market/, counterparty/, trade/ (CF-expanded), convention/, instrument_def/ |
| infra_store | 🔶 | Basic traits only, postgres optional |

#### Adapter Layer (A) - Input
| Crate | Status | Notes |
|-------|--------|-------|
| adapter_feeds | 🔶 | Basic quote types only |
| adapter_loader | 🔶 | CSV loader, CSA terms, FpML parser (`fpml` feature) |

#### Service Layer (S) - Output
| Crate | Status | Notes |
|-------|--------|-------|
| service_gateway | ✅ | Unified service crate: REST API + CLI (feature `cli`) + Python bindings (feature `python`) |

> **Note**: `service_cli` and `service_python` consolidated into `service_gateway` via feature-gated modules (2026-02). Old crate directories removed.

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

### Maintenance Tasks (Completed 2026-01-26)

Codebase redundancy cleanup performed:

| Task | Description | Status |
|------|-------------|--------|
| tokio-version-fix | Fixed tokio version inconsistency (workspace = true) | ✅ |
| dead-code-removal | Removed dead code in service_gateway | ✅ |
| unused-modules-cleanup | Removed unused modules in pricer_risk (bucket_dv01, irs_greeks_by_factor, portfolio_greeks) | ✅ |
| test-fixtures-common | Created shared test fixtures (pricer_pricing/tests/common, pricer_models/tests/common) | ✅ |
| app-js-refactor-plan | Documented app.js refactoring plan (15,000+ lines → modular) | ✅ |

### Maintenance Tasks (Completed 2026-02)

| Task | Description | Status |
|------|-------------|--------|
| ndarray-removal | Removed ndarray dependency, updated imports to infra_domain | ✅ |
| quoteid-migration | Refactored TickerMapping to use QuoteId instead of RateId | ✅ |
| ai-fixer-ci | Self-healing CI module (.github/ai_fixer/, Gemini API) | ✅ |
| service-gateway-reenable | Re-enabled service_gateway in workspace members | ✅ |
| consolidate-service-crates | Consolidated service_cli/service_python into service_gateway (feature-gated cli/python) | ✅ |

### Future Maintenance (Low Priority)

| Task | Description | Status |
|------|-------------|--------|
| app-js-modularisation | Split app.js into separate modules (chart-utils, portfolio-table, etc.) | ⏳ Planned |
| error-consolidation | Consolidate 31 error.rs files into shared patterns | ⏳ Planned |
| spec-consolidation | Consolidate Ergodic Bank-related specs (4 specs → 1) | ✅ Archived |
| d3-module-audit | Audit D3.js modules usage in demo/gui/static/vendor | ⏳ Planned |

---

### Active Specifications (In Progress)

| Spec | Phase | Description |
|------|-------|-------------|
| market-convention-instrument | tasks-generated | Market convention registry, EventInstrument, D3.js graph visualisation |
| fxvol-calibration-migration | tasks-generated | Migrate FX vol calibration module |
| models-module-refactor | tasks-generated | Refactor pricer_models module structure |
| pricer-computation-graph | tasks-generated | Computation graph extraction for pricer |

### Notes

- **demo/gui**: Temporarily disabled due to calibration module refactoring. Feature-gated handlers added for calibration-dependent endpoints. Will be re-enabled after `builder/paramsurface` stabilisation.
- **service_gateway**: Unified service crate (2026-02). REST API, CLI, and Python bindings via feature flags.

## Recommended Next Steps

1. **exotic-options** - Barriers, Asians, Lookbacks, Digitals for product coverage
2. **python-bindings-expansion** - Enables Jupyter research workflows
3. **volatility-surface** - Implied vol surface construction and interpolation

---

## Changelog

| Date | Change |
|------|--------|
| 2026-02-13 | Spec workflow lightweight化: 53 completed specs archived to `git history`, CLAUDE.md簡素化, roadmap整理. Active specs: 4 |
| 2026-02-10 | consolidate-service-crates completed. service_cli/service_python → service_gateway feature-gated modules |
| 2026-02-01 | boilerplate-reduction, enum-dispatch-migration completed (bon builder, derive_more, enum_dispatch) |
| 2026-01-28 | pricing-kernel-ir, curve-global-solver completed (PricingKernel IR, global calibration, 559 tests) |
| 2026-01-26 | 6 specs completed (shadow-object-aad, external-numerics, mc-memory-layout, pricer-pricing-architecture, market-index-keyed-access, fx/ir-vol calibration) |
| 2026-01-23 | 10 specs completed (instruments, domain, portfolio, webapp, curve-bootstrap, rate-index-pricing) |
| 2026-01-21 | pricer-core-math-library, financial-time, trade-instrument, counterparty-netting completed |
| 2026-01-19 | codebase-cleanup, portfolio-graph, infra-primitives, advanced-sensitivity completed |
| 2026-01-10 | Initial creation. Core foundations (traits, RNG, interpolators, MC, stochastic models, Enzyme AD) |
