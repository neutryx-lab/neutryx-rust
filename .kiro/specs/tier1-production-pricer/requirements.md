# Requirements Document

## Introduction

本要件書は、neutryx-rust ライブラリをXVA計算ライブラリから、Tier1銀行で実用に耐えうるプロダクショングレードのプライサーへと再定義するための包括的な要件を定義する。

現在のライブラリは以下の機能を有している:
- XVA計算 (CVA/DVA/FVA)
- エクスポージャー指標 (EE, EPE, PFE, EEPE, ENE)
- モンテカルロプライシング (パス依存オプション対応)
- Enzyme自動微分による高速Greeks計算
- 4層アーキテクチャ (pricer_core, pricer_models, pricer_pricing, pricer_risk)

Tier1銀行での本番運用には、アセットクラスの拡充、プライシングモデルの高度化、本番環境に求められる信頼性・パフォーマンス・規制対応が必要となる。

## Requirements

### Requirement 1: アセットクラスカバレッジ - 金利商品

**Objective:** As a クオンツトレーダー, I want 包括的な金利デリバティブのプライシング機能, so that Tier1銀行の金利トレーディングデスクで使用できる

#### Acceptance Criteria
1. The Pricer shall support vanilla interest rate swap (IRS) pricing with fixed/floating leg cashflow generation.
2. The Pricer shall support swaption pricing with Black, Bachelier, and SABR volatility models.
3. The Pricer shall support cap/floor pricing with individual caplet/floorlet decomposition.
4. The Pricer shall support cross-currency swap (XCCY) pricing with FX exposure calculation.
5. The Pricer shall support OIS (Overnight Indexed Swap) pricing with compounding conventions.
6. The Pricer shall support basis swap pricing with two floating legs on different indices.
7. When pricing an interest rate instrument, the Pricer shall accept multi-curve input (discounting curve separate from projection curve).
8. The Pricer shall support inflation swaps (zero-coupon and year-on-year) pricing.

### Requirement 2: アセットクラスカバレッジ - FX商品

**Objective:** As a FXトレーダー, I want 包括的なFXデリバティブのプライシング機能, so that Tier1銀行のFXデスクで使用できる

#### Acceptance Criteria
1. The Pricer shall support FX vanilla option pricing using Garman-Kohlhagen model.
2. The Pricer shall support FX forward and NDF (Non-Deliverable Forward) pricing.
3. The Pricer shall support FX barrier options (all 8 variants: up/down, in/out, call/put).
4. The Pricer shall support FX digital options with smooth payoff approximation for AD compatibility.
5. The Pricer shall support FX touch options (one-touch, no-touch, double-no-touch).
6. The Pricer shall support FX Asian options (arithmetic and geometric averaging).
7. When pricing FX options, the Pricer shall handle volatility smile via SABR or mixed log-normal models.
8. The Pricer shall support multi-currency settlement with proper FX rate triangulation.

### Requirement 3: アセットクラスカバレッジ - 株式商品

**Objective:** As a 株式デリバティブトレーダー, I want 包括的な株式デリバティブのプライシング機能, so that Tier1銀行のエクイティデスクで使用できる

#### Acceptance Criteria
1. The Pricer shall support European and American vanilla options pricing.
2. The Pricer shall support Asian options (arithmetic/geometric) with streaming Monte Carlo.
3. The Pricer shall support barrier options (all 8 variants) with smooth barrier detection.
4. The Pricer shall support lookback options (fixed/floating strike).
5. The Pricer shall support autocallable structured products with early redemption features.
6. The Pricer shall support worst-of/best-of basket options with correlation modeling.
7. When pricing equity options, the Pricer shall support local volatility and stochastic volatility models (Heston, SABR).
8. The Pricer shall support dividend modeling (discrete dividends and continuous yield).

### Requirement 4: アセットクラスカバレッジ - コモディティ商品

**Objective:** As a コモディティトレーダー, I want コモディティデリバティブのプライシング機能, so that Tier1銀行のコモディティデスクで使用できる

#### Acceptance Criteria
1. The Pricer shall support commodity forward and futures pricing with convenience yield.
2. The Pricer shall support commodity options (European, Asian, spread options).
3. The Pricer shall support mean-reverting models (Schwartz one-factor, two-factor).
4. When pricing commodity instruments, the Pricer shall handle seasonality in forward curves.
5. The Pricer shall support calendar spread options and crack spread options.
6. The Pricer shall support swing options with multiple exercise rights.

### Requirement 5: アセットクラスカバレッジ - クレジット商品

**Objective:** As a クレジットトレーダー, I want クレジットデリバティブのプライシング機能, so that Tier1銀行のクレジットデスクで使用できる

#### Acceptance Criteria
1. The Pricer shall support CDS (Credit Default Swap) pricing with ISDA standard model.
2. The Pricer shall support CDS index (CDX, iTraxx) pricing with index factor.
3. The Pricer shall support CDS swaption pricing.
4. The Pricer shall support synthetic CDO tranche pricing with base correlation.
5. When pricing credit instruments, the Pricer shall calibrate hazard rates from CDS spreads.
6. The Pricer shall support contingent CDS (CCDS) pricing with wrong-way risk.
7. The Pricer shall support credit-linked notes (CLN) pricing.

### Requirement 6: プライシングモデル - 解析解

**Objective:** As a クオンツアナリスト, I want 高速かつ正確な解析解プライシング, so that リアルタイムプライシングとモンテカルロ検証に使用できる

#### Acceptance Criteria
1. The Pricer shall provide Black-Scholes analytical solution for European options.
2. The Pricer shall provide Garman-Kohlhagen analytical solution for FX options.
3. The Pricer shall provide Kemna-Vorst analytical solution for geometric Asian options.
4. The Pricer shall provide Merton/Rubinstein-Reiner analytical solution for barrier options.
5. The Pricer shall provide Black and Bachelier analytical solutions for swaptions.
6. The Pricer shall provide analytical Greeks (delta, gamma, vega, theta, rho) for all analytical models.
7. When using analytical solutions, the Pricer shall return results in less than 1 millisecond per instrument.

### Requirement 7: プライシングモデル - モンテカルロ法

**Objective:** As a クオンツアナリスト, I want 高性能なモンテカルロプライシングエンジン, so that パス依存型エキゾチック商品をプライシングできる

#### Acceptance Criteria
1. The Pricer shall support configurable Monte Carlo parameters (paths, time steps, seed).
2. The Pricer shall support variance reduction techniques (antithetic variates, control variates).
3. The Pricer shall support quasi-Monte Carlo with Sobol sequences.
4. The Pricer shall provide streaming statistics (mean, variance, confidence interval) without full path storage.
5. When running Monte Carlo simulation, the Pricer shall utilize multi-core parallelism via Rayon.
6. The Pricer shall support checkpointing for long simulations with memory budget constraints.
7. The Pricer shall provide convergence diagnostics and error estimation.
8. While Monte Carlo is running, the Pricer shall support incremental result updates for early termination.

### Requirement 8: プライシングモデル - PDE法

**Objective:** As a クオンツアナリスト, I want PDEベースのプライシングエンジン, so that アメリカンオプションや複雑な境界条件を扱える

#### Acceptance Criteria
1. The Pricer shall support finite difference methods (explicit, implicit, Crank-Nicolson).
2. The Pricer shall support American option pricing via PSOR (Projected Successive Over-Relaxation).
3. The Pricer shall support adaptive grid refinement near strike and barriers.
4. When using PDE methods, the Pricer shall handle boundary conditions (Dirichlet, Neumann, absorbing).
5. The Pricer shall support ADI (Alternating Direction Implicit) for multi-factor models.
6. The Pricer shall provide grid convergence analysis and Richardson extrapolation.

### Requirement 9: 確率過程モデル

**Objective:** As a クオンツアナリスト, I want 包括的な確率過程モデル, so that 多様な市場ダイナミクスをモデリングできる

#### Acceptance Criteria
1. The Pricer shall support Geometric Brownian Motion (GBM) for equity and FX.
2. The Pricer shall support Heston stochastic volatility model with full parameter calibration.
3. The Pricer shall support SABR model for rates and FX volatility smile.
4. The Pricer shall support Hull-White one-factor and two-factor models for interest rates.
5. The Pricer shall support CIR (Cox-Ingersoll-Ross) model for interest rates.
6. The Pricer shall support local volatility model (Dupire) with surface calibration.
7. The Pricer shall support jump-diffusion models (Merton, Kou).
8. When simulating correlated processes, the Pricer shall use Cholesky decomposition with positive-definiteness enforcement.
9. The Pricer shall support hybrid models combining rates and equity/FX dynamics.

### Requirement 10: リスク感応度 (Greeks)

**Objective:** As a リスクマネージャー, I want 包括的かつ高速なGreeks計算, so that ポートフォリオリスクを正確に管理できる

#### Acceptance Criteria
1. The Pricer shall calculate first-order Greeks (delta, gamma, vega, theta, rho) for all instruments.
2. The Pricer shall calculate second-order Greeks (vanna, volga, charm, vomma).
3. The Pricer shall calculate cross-gammas for multi-factor models.
4. The Pricer shall support bump-and-revalue method with configurable bump sizes.
5. The Pricer shall support Enzyme AAD (Adjoint Algorithmic Differentiation) for efficient Greeks calculation.
6. When using AAD, the Pricer shall calculate all first-order Greeks in O(1) multiple of pricing time.
7. The Pricer shall support pathwise Greeks for Monte Carlo pricing.
8. The Pricer shall support likelihood ratio method for discontinuous payoffs.
9. While calculating Greeks, the Pricer shall validate sensitivities against finite difference approximation.

### Requirement 11: マーケットデータ管理

**Objective:** As a マーケットデータエンジニア, I want 堅牢なマーケットデータインフラ, so that プライシングに必要なデータを正確に供給できる

#### Acceptance Criteria
1. The Pricer shall support yield curve construction with multiple interpolation methods (linear, log-linear, cubic spline, monotonic).
2. The Pricer shall support volatility surface construction with SSVI, SVI, and grid interpolation.
3. The Pricer shall support forward curve construction for commodities with seasonality.
4. The Pricer shall support credit curve construction from CDS spreads.
5. When market data is updated, the Pricer shall invalidate dependent cached values.
6. The Pricer shall support AD-compatible market data structures for curve sensitivities.
7. The Pricer shall validate market data for arbitrage-free conditions.
8. If market data fails validation, then the Pricer shall return detailed error messages with correction suggestions.

### Requirement 12: キャリブレーション

**Objective:** As a クオンツアナリスト, I want 堅牢なモデルキャリブレーション機能, so that モデルを市場データに整合させられる

#### Acceptance Criteria
1. The Pricer shall support swaption volatility surface calibration with Levenberg-Marquardt.
2. The Pricer shall support Heston model calibration to vanilla option surface.
3. The Pricer shall support SABR calibration to smile data.
4. The Pricer shall support local volatility surface calibration (Dupire).
5. The Pricer shall support Hull-White calibration to cap/floor and swaption prices.
6. When calibration fails to converge, the Pricer shall return diagnostic information and parameter bounds.
7. The Pricer shall support global optimization (differential evolution, simulated annealing) for non-convex problems.
8. The Pricer shall provide calibration quality metrics (RMSE, max error, bid-ask coverage).

### Requirement 13: 本番運用信頼性

**Objective:** As a 本番システム管理者, I want 本番環境で安定稼働するシステム, so that Tier1銀行の厳格な運用要件を満たせる

#### Acceptance Criteria
1. The Pricer shall achieve 99.99% availability with graceful degradation under load.
2. The Pricer shall handle malformed input without panic, returning structured errors.
3. If numerical overflow or underflow occurs, then the Pricer shall return NaN or infinity with appropriate error flags.
4. The Pricer shall support deterministic replay with configurable random seeds.
5. While under heavy load, the Pricer shall maintain response time SLA through backpressure.
6. The Pricer shall provide comprehensive logging with configurable verbosity levels.
7. The Pricer shall support health checks and liveness probes for container orchestration.
8. The Pricer shall isolate experimental features (Enzyme) from stable production paths.

### Requirement 14: パフォーマンス要件

**Objective:** As a システムアーキテクト, I want 高性能なプライシングエンジン, so that リアルタイムトレーディングに使用できる

#### Acceptance Criteria
1. The Pricer shall price a vanilla option in less than 10 microseconds using analytical methods.
2. The Pricer shall price 100,000 Monte Carlo paths in less than 100 milliseconds per instrument.
3. The Pricer shall calculate full Greeks suite in less than 10x single pricing time using AAD.
4. The Pricer shall support batch pricing of 10,000 instruments per second.
5. When processing a large portfolio, the Pricer shall utilize all available CPU cores via Rayon.
6. The Pricer shall minimize memory allocations in hot paths using pre-allocated buffers.
7. The Pricer shall support SIMD vectorization for path generation.
8. While pricing portfolio XVA, the Pricer shall process 1 million paths x 100 time steps in less than 60 seconds.

### Requirement 15: XVA計算拡張

**Objective:** As a XVAクオンツ, I want 包括的なXVA計算機能, so that 規制要件と内部リスク管理を満たせる

#### Acceptance Criteria
1. The Pricer shall calculate CVA (Credit Valuation Adjustment) with WWR (Wrong-Way Risk) modeling.
2. The Pricer shall calculate DVA (Debit Valuation Adjustment) with own credit spread.
3. The Pricer shall calculate FVA (Funding Valuation Adjustment) with funding curve.
4. The Pricer shall calculate ColVA (Collateral Valuation Adjustment) with CSA modeling.
5. The Pricer shall calculate KVA (Capital Valuation Adjustment) with regulatory capital.
6. The Pricer shall calculate MVA (Margin Valuation Adjustment) with initial margin.
7. When calculating XVA, the Pricer shall support netting set aggregation with CSA terms.
8. The Pricer shall support incremental XVA calculation for trade-level attribution.
9. The Pricer shall calculate exposure profiles (EE, EPE, PFE, EEPE, ENE) at regulatory time points.

### Requirement 16: 規制対応

**Objective:** As a 規制対応担当者, I want 規制要件に準拠した計算機能, so that 監督当局の要求を満たせる

#### Acceptance Criteria
1. The Pricer shall support SA-CCR (Standardized Approach for Counterparty Credit Risk) calculation.
2. The Pricer shall support IMM (Internal Model Method) exposure calculation.
3. The Pricer shall support FRTB (Fundamental Review of Trading Book) sensitivities.
4. The Pricer shall support ISDA SIMM (Standard Initial Margin Model) calculation.
5. When calculating regulatory metrics, the Pricer shall use prescribed time grids and confidence levels.
6. The Pricer shall provide audit trail for all regulatory calculations.
7. The Pricer shall support Basel III/IV capital requirement calculations.
8. The Pricer shall generate regulatory reports in required formats.

### Requirement 17: API設計

**Objective:** As a システムインテグレーター, I want 使いやすく拡張可能なAPI, so that 既存システムと容易に統合できる

#### Acceptance Criteria
1. The Pricer shall expose a Rust native API with zero-cost abstractions.
2. The Pricer shall provide C FFI bindings for language interoperability.
3. The Pricer shall support Python bindings via PyO3 for quantitative research.
4. The Pricer shall support async/await for non-blocking batch operations.
5. When API contracts change, the Pricer shall maintain backward compatibility within major versions.
6. The Pricer shall provide builder patterns for complex configuration objects.
7. The Pricer shall use enum-based static dispatch for Enzyme AD compatibility.
8. The Pricer shall provide comprehensive error types with actionable messages.

### Requirement 18: テスト・検証

**Objective:** As a クオンツ開発者, I want 包括的なテスト・検証フレームワーク, so that 計算結果の正確性を保証できる

#### Acceptance Criteria
1. The Pricer shall include unit tests for all public functions.
2. The Pricer shall include property-based tests for mathematical invariants.
3. The Pricer shall include benchmark tests for performance regression detection.
4. The Pricer shall verify Monte Carlo results against analytical solutions where available.
5. When using Enzyme AD, the Pricer shall verify results against num-dual for correctness.
6. The Pricer shall support fuzzing for input validation robustness.
7. The Pricer shall provide integration tests with realistic market data scenarios.
8. The Pricer shall achieve minimum 80% code coverage for production code.

### Requirement 19: ドキュメンテーション

**Objective:** As a 開発者, I want 包括的なドキュメンテーション, so that ライブラリを効果的に使用・拡張できる

#### Acceptance Criteria
1. The Pricer shall provide API documentation for all public types and functions.
2. The Pricer shall provide mathematical documentation for pricing models.
3. The Pricer shall provide usage examples for common pricing scenarios.
4. The Pricer shall provide architecture documentation explaining the 4-layer design.
5. When new features are added, the Pricer shall update documentation before release.
6. The Pricer shall provide performance tuning guide with benchmarks.
7. The Pricer shall provide migration guide for version upgrades.

### Requirement 20: デプロイメント・運用

**Objective:** As a DevOpsエンジニア, I want 容易なデプロイメントと運用, so that 本番環境での管理が効率的にできる

#### Acceptance Criteria
1. The Pricer shall provide Docker images for stable and nightly builds.
2. The Pricer shall support feature flags for enabling/disabling asset classes.
3. The Pricer shall support configuration via environment variables and config files.
4. When deployed in Kubernetes, the Pricer shall support horizontal scaling.
5. The Pricer shall provide metrics export (Prometheus format) for monitoring.
6. The Pricer shall support distributed tracing (OpenTelemetry) for debugging.
7. The Pricer shall provide release artifacts with reproducible builds.
8. The Pricer shall support zero-downtime rolling updates.
