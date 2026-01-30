# Implementation Plan

## Overview
service_gateway の services 層拡充：RiskService, PortfolioService, ModelService, VolatilityService, DemoService の実装と demo_gui 統合。

---

## Phase 1: Foundation Infrastructure

- [x] 1. Feature Flags と基盤設定
- [x] 1.1 Feature flags を Cargo.toml に設定
  - `risk`, `models`, `volatility`, `demo`, `full` features を定義
  - pricer_* crates との依存関係を設定
  - `tower-http` を demo feature の依存に追加
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [x] 1.2 (P) ServerError にドメイン別 variant を追加
  - Risk, Portfolio, Model, Volatility のエラー variant を定義
  - pricer_risk, pricer_models からの自動変換（From 実装）を追加
  - HTTP ステータスコードへのマッピングを実装
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_

- [x] 1.3 (P) 新規キャッシュ構造体を実装
  - PortfolioCache: Portfolio インスタンスの LRU キャッシュ
  - ModelCache: StochasticModel 設定の LRU キャッシュ
  - VolSurfaceCache: Vol Surface/Cube の LRU キャッシュ
  - 各キャッシュに add/get/update/remove 操作を実装
  - parking_lot::RwLock による並行アクセス保護
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5_

- [x] 1.4 AppState を新規キャッシュで拡張
  - PortfolioCache, ModelCache, VolSurfaceCache フィールドを追加
  - 各キャッシュを対応する feature flag で条件付きコンパイル
  - コンストラクタでキャッシュサイズ設定を受け付ける
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5_

---

## Phase 2: Risk Domain Services

- [x] 2. RiskService 実装
- [x] 2.1 Greeks 計算機能を実装
  - pricer_risk::RiskEngine を使用した Greeks 計算ロジック
  - BumpAndRevalue モードのサポート
  - EnzymeAAD モードのサポート（enzyme-ad feature 条件付き）
  - 計算時間の計測とレスポンスへの含有
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

- [x] 2.2 シナリオ分析機能を実装
  - pricer_risk::ScenarioEngine を使用したシナリオ P&L 計算
  - PresetScenario（定義済みシナリオ）のサポート
  - BumpScenario（カスタムシナリオ）のサポート
  - 各シナリオ結果（base_value, scenario_value, pnl）の配列形式返却
  - 不正シナリオ定義時のエラーハンドリング
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [x] 2.3 (P) Risk DTO を定義
  - GreeksRequest/RiskGreeksResponse の定義
  - GreeksModeDto, GreekTypeDto の enum 定義
  - ScenarioRequest/ScenarioResponse の定義
  - ScenarioDefinition（Preset/Custom）の tagged enum
  - _Requirements: 1.1, 2.1, 10.1_

- [x] 2.4 Risk ハンドラーを実装
  - POST /api/v1/risk/greeks ハンドラー
  - POST /api/v1/risk/scenarios ハンドラー
  - リクエストバリデーションとエラーレスポンス
  - _Requirements: 1.1, 2.1, 10.2, 10.3_

---

## Phase 3: Portfolio Domain Services

- [x] 3. PortfolioService 実装
- [x] 3.1 Portfolio CRUD 操作を実装
  - create_portfolio: 新規ポートフォリオ作成と ID 返却
  - get_portfolio: キャッシュからのポートフォリオ取得
  - add_trades: 既存ポートフォリオへのトレード追加
  - delete_portfolio: ポートフォリオのキャッシュからの削除
  - 存在しない portfolio_id 時の NotFound エラー
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [x] 3.2 Portfolio 集計機能を実装
  - price_portfolio: 全トレードの現在価値合計
  - compute_portfolio_greeks: 集約 Greeks 計算
  - Counterparty/NettingSet 別集計
  - 成功/失敗トレード数のレスポンス含有
  - 一部トレード失敗時のエラー配列返却
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [x] 3.3 (P) Portfolio DTO を定義
  - CreatePortfolioRequest/Response の定義
  - GetPortfolioResponse の定義
  - AddTradesRequest/Response の定義
  - PortfolioPriceResponse, PortfolioGreeksRequest/Response の定義
  - TradeDto, CounterpartyDto, NettingSetDto の定義
  - _Requirements: 3.1, 4.1, 10.1_

- [x] 3.4 Portfolio ハンドラーを実装
  - POST /api/v1/portfolios ハンドラー
  - GET /api/v1/portfolios/{id} ハンドラー
  - PUT /api/v1/portfolios/{id}/trades ハンドラー
  - DELETE /api/v1/portfolios/{id} ハンドラー
  - POST /api/v1/portfolios/{id}/price ハンドラー
  - POST /api/v1/portfolios/{id}/greeks ハンドラー
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 10.2, 10.3_

---

## Phase 4: Model Domain Services

- [x] 4. ModelService 実装
- [x] 4.1 確率モデル設定機能を実装
  - create_model: StochasticModelEnum インスタンス生成とキャッシュ保存
  - get_model: モデル設定詳細の取得
  - GBM, Heston, HullWhite, CIR, SABR モデルのサポート
  - パラメータバリデーションとエラーハンドリング
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 4.2 モデルベース価格計算機能を実装
  - price_with_model: 指定モデルでの価格計算
  - Monte Carlo pricing (MonteCarloPricer) のサポート
  - Tree pricing (TreeMethod) のサポート
  - 計算手法のレスポンス含有
  - 存在しない model_id 時の NotFound エラー
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 4.3 (P) Model DTO を定義
  - CreateModelRequest（tagged enum: GBM/Heston/HullWhite/CIR/SABR）の定義
  - CreateModelResponse, GetModelResponse の定義
  - ModelPricingRequest/Response の定義
  - PricingMethodDto（Analytical/MonteCarlo/Tree）の定義
  - GbmParamsDto, HestonParamsDto 等のパラメータ DTO
  - _Requirements: 5.1, 6.1, 10.1_

- [x] 4.4 Model ハンドラーを実装
  - POST /api/v1/models ハンドラー
  - GET /api/v1/models/{id} ハンドラー
  - POST /api/v1/models/{id}/price ハンドラー
  - _Requirements: 5.1, 5.4, 6.1, 10.2, 10.3_

---

## Phase 5: Volatility Domain Services

- [x] 5. VolatilityService 実装
- [x] 5.1 Vol Surface/Cube 構築機能を実装
  - build_fx_vol_surface: FxVolBuilder による FX Vol Surface 構築
  - build_vol_cube: VolCubeBuilder による Vol Cube 構築
  - SABR calibration 結果（alpha, beta, rho, nu）のレスポンス含有
  - キャリブレーション非収束時のエラーハンドリング
  - _Requirements: 7.1, 7.2, 7.4, 7.5_

- [x] 5.2 Implied Vol 照会機能を実装
  - get_implied_vol: 指定 expiry/strike の補間ボラティリティ取得
  - キャッシュからの Surface/Cube 取得
  - 存在しない surface_id 時の NotFound エラー
  - _Requirements: 7.3_

- [x] 5.3 (P) Volatility DTO を定義
  - BuildFxVolSurfaceRequest/Response の定義
  - BuildVolCubeRequest/Response の定義
  - GetImpliedVolRequest/Response の定義
  - VolQuoteDto, SabrCalibrationDto の定義
  - _Requirements: 7.1, 7.2, 7.3, 10.1_

- [x] 5.4 Volatility ハンドラーを実装
  - POST /api/v1/volatility/fx-surface ハンドラー
  - POST /api/v1/volatility/cube ハンドラー
  - POST /api/v1/volatility/{id}/implied-vol ハンドラー
  - _Requirements: 7.1, 7.2, 7.3, 10.2, 10.3_

---

## Phase 6: Demo GUI Integration

- [x] 6. DemoService と demo_gui 統合
- [x] 6.1 既存 demo/gui/web/handlers を削除
  - demo/gui は TypeScript フロントエンドのみで、Rust ハンドラーは元々存在しない
  - 全 API エンドポイントを service_gateway で実装（設計通り）
  - _Requirements: 15.1, 15.2_

- [x] 6.2 DemoService を実装
  - get_config: アプリケーション設定取得
  - get_instruments: 利用可能な金融商品一覧
  - get_fx_vol_pairs, get_fx_vol_quotes: FX Vol データ
  - get_ir_vol_currencies, get_ir_vol_quotes: IR Vol データ
  - get_market_rates, get_market_config: マーケットデータ
  - get_conventions: 市場慣行
  - get_events: マーケットイベント
  - refresh_market_rates: マーケットレート更新
  - export_market_data: マーケットデータエクスポート（CSV/JSON）
  - expand_trade: トレード展開
  - price_trade, calculate_greeks: 価格計算・Greeks計算
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 13.6_

- [x] 6.3 (P) Demo DTO を定義
  - AppConfigResponse, InstrumentsResponse の定義
  - FxVolPairsResponse, FxVolQuotesResponse の定義
  - IrVolCurrenciesResponse, IrVolQuotesResponse の定義
  - MarketRatesResponse, ConventionsResponse の定義
  - EventsResponse, EventTypesResponse の定義
  - TradeExpandRequest, ExpandedTrade の定義
  - DemoPricingRequest/Result, DemoGreeksRequest/Result の定義
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 10.1_

- [x] 6.4 Demo ハンドラーを実装
  - /api/config, /api/instruments エンドポイント
  - /api/trade/expand, /api/pricer/* エンドポイント
  - /api/market/*, /api/curves/* エンドポイント
  - /api/irvol/*, /api/fxvol/* エンドポイント
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 10.2, 10.3_

- [x] 6.5 静的ファイル配信を設定
  - tower-http::ServeDir による demo/gui/static/ からの静的ファイル配信
  - SPA フォールバック（存在しないパスは index.html へ）
  - /static/* パスでアセット配信
  - _Requirements: 14.1, 14.2, 14.3, 14.4, 14.5_

---

## Phase 7: Router Integration & API Versioning

- [x] 7. Router 統合と API バージョニング
- [x] 7.1 Feature-gated ルーター構成を実装
  - risk feature 有効時: /api/v1/risk/*, /api/v1/portfolios/* ルート追加
  - models feature 有効時: /api/v1/models/* ルート追加
  - volatility feature 有効時: /api/v1/volatility/* ルート追加
  - demo feature 有効時: /api/* demo ルートと静的ファイル配信追加
  - _Requirements: 8.2, 8.3, 8.4, 8.5_

- [x] 7.2 API バージョニングを実装
  - 全エンドポイントを /api/v1/ プレフィックス配下に配置
  - X-API-Version レスポンスヘッダーの追加
  - 未サポート API バージョン時の 400 Bad Request
  - _Requirements: 12.1, 12.2, 12.4_

- [x] 7.3 create_demo_router 関数を実装
  - demo_api_routes() による全 demo エンドポイントの定義
  - api_v1_routes() との統合
  - ServeDir による静的ファイル配信と SPA フォールバック
  - AppState の注入
  - _Requirements: 13.5, 13.6, 14.1_

---

## Phase 8: Testing & Validation

- [x] 8. テストとバリデーション
- [x] 8.1 サービス層の単体テストを実装
  - RiskService: Greeks 計算、シナリオ実行のテスト
  - PortfolioService: CRUD 操作、集計計算のテスト
  - ModelService: モデル作成、パラメータバリデーションのテスト
  - VolatilityService: Surface 構築、補間のテスト
  - _Requirements: 10.5_

- [x] 8.2 キャッシュ層の単体テストを実装
  - PortfolioCache: add/get/update/remove 操作のテスト
  - ModelCache: add/get/remove 操作のテスト
  - VolSurfaceCache: add/get/remove 操作のテスト
  - 並行アクセス時の動作テスト
  - _Requirements: 11.1, 11.2, 11.3, 11.5_

- [x] 8.3 エンドポイント統合テストを実装
  - DemoService 単体テスト: get_instruments, get_fx_vol_pairs, get_ir_vol_currencies,
    get_conventions, get_market_config, get_events, get_event_types
  - 既存の統合テスト: portfolio_tests, simulated_http_tests
  - 全 40 テスト合格
  - _Requirements: 1.1, 2.1, 3.1, 4.1, 5.1, 6.1, 7.1, 13.1_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1 (Greeks計算) | 2.1, 2.3, 2.4, 8.1, 8.3 |
| 2 (シナリオ分析) | 2.2, 2.3, 2.4, 8.1, 8.3 |
| 3 (Portfolio CRUD) | 3.1, 3.3, 3.4, 8.1, 8.3 |
| 4 (Portfolio集計) | 3.2, 3.3, 3.4, 8.1, 8.3 |
| 5 (モデル設定) | 4.1, 4.3, 4.4, 8.1, 8.3 |
| 6 (モデル価格計算) | 4.2, 4.3, 4.4, 8.1, 8.3 |
| 7 (Vol Surface) | 5.1, 5.2, 5.3, 5.4, 8.1, 8.3 |
| 8 (Feature Flags) | 1.1, 7.1 |
| 9 (Error Domain分離) | 1.2 |
| 10 (一貫パターン) | 2.3, 2.4, 3.3, 3.4, 4.3, 4.4, 5.3, 5.4, 6.3, 6.4, 8.1 |
| 11 (AppState拡張) | 1.3, 1.4, 8.2 |
| 12 (APIバージョニング) | 7.2 |
| 13 (Demo GUI統合) | 6.2, 6.3, 6.4, 7.3, 8.3 |
| 14 (静的ファイル配信) | 6.5, 7.3 |
| 15 (Demo GUI依存制約) | 6.1 |
