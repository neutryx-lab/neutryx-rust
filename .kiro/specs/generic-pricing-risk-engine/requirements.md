# Requirements Document

## Introduction

本仕様は、Neutryx派生商品プライシングライブラリにおける汎用プライシングエンジンおよびリスク計算エンジンの実装を定義する。計算設定ファイル（TOML/JSON）と入力データ（約定データ、マーケットデータ、CSAデータ）に基づいて、単一取引およびポートフォリオレベルの価格計算とリスク計算（Greeks）を汎用的に実行可能とする。また、`pricer_pricing`から`pricer_risk`へのリスク関連機能の適切な移行を含む。

## Requirements

### Requirement 1: 計算設定ファイル構造

**Objective:** As a クオンツ開発者, I want 計算設定ファイル（TOML/JSON）を定義して価格計算とリスク計算のパラメータを一元管理できること, so that 計算パイプラインの再現性と設定の可視化が向上する

#### Acceptance Criteria

1. The PricingConfig shall define calculation parameters including valuation date, currency, and pricing method selection
2. The RiskConfig shall define Greeks calculation parameters including method (AAD/Bump), bump sizes, and target sensitivities
3. When a configuration file is loaded, the ConfigLoader shall validate all required fields and return structured errors for missing or invalid values
4. The configuration schema shall support nested structures for market data paths, trade data paths, and CSA data paths
5. Where JSON format is used, the ConfigLoader shall parse and validate identically to TOML format

### Requirement 2: 入力データローダー

**Objective:** As a トレーダー/リスク管理者, I want 約定データ、マーケットデータ、CSAデータを統一的なインターフェースで読み込めること, so that 異なるデータソースからの入力を一貫して処理できる

#### Acceptance Criteria

1. The DataLoader shall load trade data from JSON files and convert to `Trade` structures defined in `infra_domain::trade`
2. The DataLoader shall load market data (curves, volatility surfaces) from JSON files and construct corresponding `CurveEnum`/`VolSurfaceEnum` objects
3. The DataLoader shall load CSA terms from JSON files and construct `CsaTerms` structures defined in `infra_domain::counterparty`
4. When input file is not found or malformed, the DataLoader shall return descriptive errors with file path and parse location
5. The DataLoader shall support batch loading of multiple files via glob patterns

### Requirement 3: 汎用プライシングエンジン（単一取引）

**Objective:** As a クオンツ開発者, I want 設定ファイルと入力データから単一取引の価格を計算できること, so that 個別取引の評価を自動化できる

#### Acceptance Criteria

1. The GenericPricer shall accept `PricingConfig`, `Trade`, and `MarketProvider` as inputs and return `PricingResult`
2. When pricing method is "analytical", the GenericPricer shall use closed-form solutions from `pricer_models::analytical`
3. When pricing method is "monte_carlo", the GenericPricer shall use `MonteCarloPricer` from `pricer_pricing::mc`
4. The GenericPricer shall resolve market data (discount curves, forward curves, volatility surfaces) via `MarketProvider`
5. If required market data is missing, the GenericPricer shall return `PricingError::MissingMarketData` with specific data identifier

### Requirement 4: 汎用プライシングエンジン（ポートフォリオ）

**Objective:** As a リスク管理者, I want 複数取引のポートフォリオを一括で価格計算できること, so that ポートフォリオレベルのP&L計算が効率化される

#### Acceptance Criteria

1. The PortfolioPricer shall accept `PricingConfig`, `Vec<Trade>`, and `MarketProvider` as inputs and return `PortfolioPricingResult`
2. The PortfolioPricer shall calculate individual trade prices and aggregate by currency, netting set, and book
3. While parallel execution is enabled in configuration, the PortfolioPricer shall use Rayon for concurrent trade pricing
4. The PortfolioPricer shall report pricing failures per trade without aborting entire portfolio calculation
5. The PortfolioPricingResult shall include execution statistics (total trades, success count, failure count, elapsed time)

### Requirement 5: 汎用リスク計算エンジン

**Objective:** As a クオンツ開発者, I want 設定ファイルでAADまたはBump-and-Revalueを選択してGreeks計算を実行できること, so that 計算手法の比較検証と最適な手法選択が可能になる

#### Acceptance Criteria

1. The RiskEngine shall accept `RiskConfig`, `Trade` or `Vec<Trade>`, and `MarketProvider` as inputs and return `RiskResult`
2. When Greeks method is "aad", the RiskEngine shall use Enzyme-based automatic differentiation from `pricer_pricing::enzyme`
3. When Greeks method is "bump", the RiskEngine shall use finite difference approximation with configurable bump sizes
4. The RiskConfig shall allow selection of target Greeks (delta, gamma, vega, theta, rho) to compute
5. The RiskResult shall include computed Greeks values, calculation method used, and performance metrics (computation time, memory usage)

### Requirement 6: pricer_pricingからpricer_riskへの機能移行

**Objective:** As a アーキテクト, I want リスク関連機能を`pricer_pricing`から`pricer_risk`に移行すること, so that A-I-P-Sアーキテクチャの依存関係ルールが遵守され、L4クレートがリスク計算の中心となる

#### Acceptance Criteria

1. The `greeks/` module containing `GreeksConfig`, `GreeksMode`, `GreeksResult` shall be relocated from `pricer_pricing` to `pricer_risk`
2. The `irs_greeks/` module containing IRS Greeks calculator and lazy evaluator shall be relocated from `pricer_pricing` to `pricer_risk`
3. After relocation, the `pricer_pricing` crate shall not expose any Greeks-related public API
4. The `pricer_risk` crate shall re-export relocated types via `pricer_risk::greeks` module
5. If downstream code references old module paths, the compiler shall produce clear deprecation warnings with migration guidance

### Requirement 7: リスク計算設定の柔軟性

**Objective:** As a リスク管理者, I want リスク計算パラメータを詳細に設定できること, so that 規制要件や社内リスク管理方針に応じたカスタマイズが可能になる

#### Acceptance Criteria

1. The RiskConfig shall support bump size configuration per risk factor type (rate: 1bp, vol: 1%, spot: 1%)
2. The RiskConfig shall support parallel/serial bump selection for second-order Greeks (gamma, cross-gamma)
3. Where CSA terms are provided, the RiskEngine shall apply collateral and netting adjustments to exposure calculations
4. The RiskConfig shall support scenario-based Greeks calculation with predefined or custom market shifts
5. While calculating portfolio Greeks, the RiskEngine shall aggregate sensitivities by risk factor, currency, and tenor bucket

### Requirement 8: エラーハンドリングと診断

**Objective:** As a 運用担当者, I want 計算失敗時に詳細な診断情報を取得できること, so that 問題の迅速な特定と修正が可能になる

#### Acceptance Criteria

1. The PricingError shall include variant-specific context (trade ID, market data identifier, calculation step)
2. The RiskError shall include diagnostic data (attempted Greeks, partial results, failure point)
3. When configuration validation fails, the ConfigError shall report all validation issues (not just first)
4. If numerical instability occurs, the error shall include relevant numeric values and suggested mitigation
5. The error types shall implement `thiserror::Error` with structured variants following project error-handling steering

### Requirement 9: Service層との統合準備

**Objective:** As a システムアーキテクト, I want service_cliおよびservice_gatewayとの統合ポイントを明確にすること, so that Service層の再有効化後にスムーズな統合が可能になる

#### Acceptance Criteria

1. The GenericPricer and RiskEngine shall expose async-compatible interfaces for REST API integration
2. The configuration schema shall be serialisable/deserialisable via serde for API request/response handling
3. The result types (PricingResult, RiskResult) shall be JSON-serialisable for service_gateway response formatting
4. When batch processing is requested, the engine shall support job-based execution compatible with `demo/gui/web/jobs.rs` pattern
5. The API surface shall follow existing demo web handler patterns (`*_handlers.rs` + `*_types.rs` structure)
