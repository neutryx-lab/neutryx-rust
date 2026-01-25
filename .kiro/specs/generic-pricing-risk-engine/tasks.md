# Implementation Plan

## Task Format

本実装計画はMigration Strategyに従い、4つの主要フェーズ + クリーンアップフェーズで構成される。各タスクは並列実行可能性を`(P)`マーカーで示す。

---

## Phase 1: 設定・ローダー追加

- [x] 1. infra_config に計算設定スキーマを追加
- [x] 1.1 (P) PricingConfig 構造体の実装
  - 価格計算パラメータ（valuation_date, reporting_currency, pricing_method）を定義
  - MonteCarloParams サブ構造体（num_paths, num_steps, seed）を定義
  - ファイルパス設定（market_data_path, trade_data_path, csa_data_path）を定義
  - serde Deserialize/Serialize derive を追加
  - validate() メソッドで必須フィールドと値範囲を検証
  - ConfigError を返すバリデーション失敗時の詳細メッセージ
  - _Requirements: 1.1, 1.3, 1.4, 1.5_

- [x] 1.2 (P) RiskConfig 構造体の実装
  - GreeksMethod enum（Aad, Bump）を定義
  - BumpSizes 構造体（rate, vol, spot デフォルト値付き）を定義
  - GreekType enum（Delta, Gamma, Vega, Theta, Rho, Vanna, Volga）を定義
  - SecondOrderMode enum（Parallel, Serial）を定義
  - ScenarioConfig オプション構造体を定義
  - target_greeks: Vec<GreekType> で計算対象を選択可能に
  - enzyme-ad feature 無効時の AAD 選択でエラー
  - _Requirements: 1.2, 7.1, 7.2, 7.4_

- [x] 1.3 Settings 統合と ConfigLoader の実装
  - 既存 Settings::load() に pricing セクションを追加
  - 既存 Settings::load() に risk セクションを追加
  - TOML と JSON 両形式からの読み込みをサポート
  - 全バリデーションエラーを収集して一括報告
  - _Requirements: 1.3, 1.5, 8.3_

- [x] 1.4 ConfigError 構造体の拡張
  - MissingField { field } バリアント追加
  - InvalidValue { field, reason } バリアント追加
  - FileNotFound { path } バリアント追加
  - ParseError { path, location, message } バリアント追加（行番号含む）
  - thiserror::Error derive 確認
  - _Requirements: 8.1, 8.3, 8.5_

- [x] 1.5 Config 単体テストの実装
  - PricingConfig の必須フィールド検証テスト
  - RiskConfig の bump_sizes 範囲検証テスト
  - TOML/JSON 両形式の読み込みテスト
  - バリデーションエラーメッセージの確認テスト
  - _Requirements: 1.1, 1.2, 1.3_

- [x] 2. adapter_loader に JSON ローダーを追加
- [x] 2.1 (P) JsonLoader 汎用読み込み機能の実装
  - load<T: DeserializeOwned>(path) 関数でファイルを読み込み型Tにデシリアライズ
  - load_glob<T>(pattern) で glob パターンによるバッチ読み込み
  - パースエラー時にファイルパスと行番号を含む LoaderError 返却
  - ファイル不在時の明確なエラーメッセージ
  - _Requirements: 2.4, 2.5_

- [x] 2.2 (P) TradeLoader 約定データ読み込みの実装
  - JSON スキーマから infra_master::trade::Trade への変換
  - load(path) で単一約定ファイル読み込み
  - load_portfolio(path) で複数約定（配列）読み込み
  - 必須フィールド検証（trade_id, instrument, currency）
  - 変換エラー時に trade_id を含むコンテキスト付きエラー
  - _Requirements: 2.1, 2.4_

- [x] 2.3 (P) MarketLoader マーケットデータ読み込みの実装
  - JSON から CurveData（カーブ種別、テナーポイント、レート）への変換
  - JSON から VolSurfaceData（満期、ストライク、ボラティリティ）への変換
  - FX スポットレートの HashMap への読み込み
  - MarketData 集約構造体（curves, vol_surfaces, fx_spots）の構築
  - _Requirements: 2.2_

- [x] 2.4 (P) CsaLoader CSA データ読み込みの実装
  - JSON から infra_master::counterparty::CsaTerms への変換
  - 担保通貨、閾値、MTA などのフィールドマッピング
  - _Requirements: 2.3_

- [x] 2.5 LoaderError 構造体の実装
  - FileNotFound { path } バリアント
  - ParseError { path, line, column, message } バリアント
  - ValidationError { path, field, reason } バリアント
  - thiserror::Error derive
  - _Requirements: 2.4, 8.5_

- [x] 2.6 Loader 統合テストの実装
  - サンプル trade.json からの読み込みテスト
  - サンプル market.json からの読み込みテスト
  - glob パターン読み込みテスト
  - 不正 JSON のエラーハンドリングテスト
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

---

## Phase 2: Pricer 拡張

- [x] 3. GenericPricer の設定駆動拡張
- [x] 3.1 GenericPricer::from_config() コンストラクタの実装
  - PricingConfig から ModelConfig への自動変換
  - PricingConfig から PricerConfig への自動変換
  - pricing_method に基づくプライサー選択（analytical/monte_carlo）
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 3.2 GenericPricer::price_with_config() メソッドの実装
  - Trade, MarketProvider, PricingConfig を入力として受け取る
  - MarketProvider 経由でディスカウントカーブ、フォワードカーブ、ボラティリティサーフェスを解決
  - 不足マーケットデータ時に PricingError::MissingMarketData 返却
  - PricingResult 返却（price, std_error, 計算メタデータ）
  - _Requirements: 3.1, 3.4, 3.5_

- [x] 3.3 単一取引プライシング統合テストの実装
  - 設定ファイル読み込み → GenericPricer 構築 → 価格計算の一連フロー
  - analytical メソッドと monte_carlo メソッドの両方をテスト
  - マーケットデータ不足時のエラーハンドリングテスト
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 4. PortfolioPricer ポートフォリオ並列プライシングの実装
- [x] 4.1 PortfolioPricer 構造体の実装
  - GenericPricer と PricingConfig をフィールドに保持
  - new(config) コンストラクタで設定からプライサーを構築
  - _Requirements: 4.1_

- [x] 4.2 price_portfolio() メソッドの実装
  - Vec<Trade> と MarketProvider を入力として受け取る
  - parallel_enabled 設定に基づき Rayon 並列処理を選択
  - 個別取引の失敗を記録しつつ全体処理を継続（partial success）
  - successes: Vec<(TradeId, PricingResult)> 構築
  - failures: Vec<(TradeId, PricingError)> 構築
  - _Requirements: 4.1, 4.3, 4.4_

- [x] 4.3 PortfolioAggregations 集約機能の実装
  - 通貨別 PV 集約（by_currency: HashMap<Currency, f64>）
  - Netting set 別 PV 集約（by_netting_set: HashMap<NettingSetId, f64>）
  - Book 別 PV 集約（by_book: HashMap<String, f64>）
  - _Requirements: 4.2_

- [x] 4.4 ExecutionStats 実行統計の実装
  - total_count: 処理対象取引数
  - success_count: 成功取引数
  - failure_count: 失敗取引数
  - elapsed_ms: 処理時間（ミリ秒）
  - _Requirements: 4.5_

- [x] 4.5 PortfolioPricer 統合テストの実装
  - 複数取引のポートフォリオ価格計算テスト
  - 並列処理有効/無効時の動作確認
  - 一部取引失敗時の継続処理テスト
  - 集約結果の検証テスト
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

---

## Phase 3: モジュール移行

- [x] 5. greeks モジュールを pricer_pricing から pricer_risk へ移行
- [x] 5.1 pricer_risk/src/greeks/ ディレクトリ構造の作成
  - mod.rs: モジュール公開宣言
  - config.rs: GreeksConfig, GreeksConfigBuilder, GreeksMode を pricer_pricing からコピー
  - error.rs: GreeksError を pricer_pricing からコピー
  - result.rs: GreeksResult<T> を pricer_pricing からコピー
  - tests.rs: 既存テストを pricer_pricing からコピー
  - _Requirements: 6.1, 6.4_

- [x] 5.2 pricer_risk/src/lib.rs への greeks モジュール追加
  - `pub mod greeks;` 宣言追加
  - `pub use greeks::{GreeksConfig, GreeksConfigBuilder, GreeksConfigError, GreeksError, GreeksMode, GreeksResult};` re-export 追加
  - _Requirements: 6.4_

- [x] 5.3 pricer_pricing における deprecated re-export の追加
  - `#[deprecated(since = "0.8.0", note = "Use pricer_risk::greeks instead")]` 属性追加
  - 既存 greeks re-export を pricer_risk からの再エクスポートに変更
  - コンパイル時警告メッセージに移行先パスを含める
  - _Requirements: 6.3, 6.5_

- [x] 5.4 greeks 移行テストの実装
  - pricer_risk::greeks からの import が正常動作することを確認
  - pricer_pricing::greeks からの import がコンパイル警告を出すことを確認
  - 既存機能が両パスから同等に動作することを確認
  - _Requirements: 6.1, 6.3, 6.4, 6.5_

- [x] 6. irs_greeks モジュールを pricer_pricing から pricer_risk へ移行
- [x] 6.1 pricer_risk/src/irs_greeks/ ディレクトリ構造の作成
  - mod.rs: モジュール公開宣言
  - config.rs: IrsGreeksConfig をコピー
  - error.rs: IrsGreeksError をコピー
  - result.rs: IrsGreeksResult, IrsDeltaResult をコピー
  - calculator.rs: IrsGreeksCalculator をコピー
  - lazy_evaluator.rs: IrsLazyEvaluator をコピー
  - benchmark.rs: BenchmarkRunner をコピー（l1l2-integration feature 付き）
  - xva_demo.rs: XvaDemoRunner をコピー（l1l2-integration feature 付き）
  - tests.rs: 既存テストをコピー
  - _Requirements: 6.2_

- [x] 6.2 pricer_risk/src/lib.rs への irs_greeks モジュール追加
  - `#[cfg(feature = "l1l2-integration")] pub mod irs_greeks;` 宣言追加
  - 28 個の型を re-export
  - _Requirements: 6.4_

- [x] 6.3 pricer_pricing における irs_greeks deprecated re-export の追加
  - `#[deprecated(since = "0.8.0", note = "Use pricer_risk::irs_greeks instead")]` 属性追加
  - 既存 irs_greeks re-export を pricer_risk からの再エクスポートに変更
  - _Requirements: 6.3, 6.5_

- [x] 6.4 irs_greeks 移行テストの実装
  - pricer_risk::irs_greeks からの import が正常動作することを確認
  - pricer_pricing::irs_greeks からの import がコンパイル警告を出すことを確認
  - l1l2-integration feature 有効/無効時の動作確認
  - _Requirements: 6.2, 6.3, 6.5_

- [x] 6.5 pricer_risk 内部参照の更新
  - scenarios/greeks_by_factor.rs: `use pricer_pricing::greeks::*` → `use crate::greeks::*`
  - scenarios/irs_greeks_by_factor.rs: `use pricer_pricing::irs_greeks::*` → `use crate::irs_greeks::*`
  - parallel/portfolio_greeks.rs: `use pricer_pricing::greeks::*` → `use crate::greeks::*`
  - _Requirements: 6.1, 6.2_

---

## Phase 4: RiskEngine 統合

- [x] 7. RiskEngine 統合ファサードの実装
- [x] 7.1 RiskEngine 構造体の実装
  - RiskConfig をフィールドに保持
  - new(config) コンストラクタ
  - _Requirements: 5.1_

- [x] 7.2 compute_greeks() 単一取引リスク計算メソッドの実装
  - Trade と MarketProvider を入力として受け取る
  - greeks_method に基づき AAD または Bump を選択
  - AAD モード: pricer_pricing::enzyme::gradient を呼び出し
  - Bump モード: finite_difference 計算を実行
  - 計算対象 Greeks を target_greeks 設定から決定
  - _Requirements: 5.1, 5.2, 5.3, 5.4_

- [x] 7.3 compute_portfolio_greeks() ポートフォリオリスク計算メソッドの実装
  - Vec<Trade> と MarketProvider を入力として受け取る
  - Rayon による並列処理
  - 個別失敗時の継続処理（partial success）
  - _Requirements: 5.1_

- [x] 7.4 RiskResult 構造体の実装
  - trade_id: 取引識別子
  - greeks: ComputedGreeks（delta, gamma, vega, theta, rho, vanna, volga）
  - method: 使用した計算手法（GreeksMethod）
  - metrics: PerformanceMetrics（computation_time_ms, memory_usage_bytes）
  - serde Serialize/Deserialize derive
  - _Requirements: 5.5, 9.3_

- [x] 7.5 PortfolioRiskResult 構造体の実装
  - results: Vec<RiskResult>
  - failures: Vec<FailedCalculation>
  - aggregations: AggregatedGreeks（by_risk_factor, by_currency, by_tenor_bucket）
  - stats: ExecutionStats
  - _Requirements: 5.5, 7.5_

- [x] 7.6 RiskError 構造体の実装
  - CalculationFailed { trade_id, reason, partial_results } バリアント
  - AadNotAvailable バリアント（enzyme-ad feature 無効時）
  - NumericalInstability { description, value, suggested_mitigation } バリアント
  - MarketData(PricingError) バリアント
  - Config(ConfigError) バリアント
  - _Requirements: 8.2, 8.4, 8.5_

- [x] 7.7 AggregatedGreeks 集約機能の実装
  - リスクファクター別感応度集約（by_risk_factor）
  - 通貨別 Greeks 集約（by_currency）
  - テナーバケット別感応度集約（by_tenor_bucket）
  - _Requirements: 7.5_

- [x] 7.8 bump_sizes 設定のリスクファクター種別対応
  - rate bump: デフォルト 1bp (0.0001)
  - vol bump: デフォルト 1% (0.01)
  - spot bump: デフォルト 1% (0.01)
  - 設定ファイルからのオーバーライド
  - _Requirements: 7.1_

- [x] 7.9 second_order_mode 二次 Greeks 計算モードの実装
  - Parallel モード: gamma, cross-gamma を並列計算
  - Serial モード: 逐次計算
  - _Requirements: 7.2_

- [x] 7.10 CSA 条件適用機能の実装
  - CsaTerms が提供された場合の担保調整
  - Netting 調整のエクスポージャー計算への適用
  - _Requirements: 7.3_

- [x] 7.11 シナリオベース Greeks 計算の実装
  - ScenarioConfig による事前定義シナリオの適用
  - カスタムマーケットシフトの適用
  - 既存 scenarios::ScenarioEngine との統合
  - _Requirements: 7.4_

- [x] 7.12 RiskEngine 単体・統合テストの実装
  - AAD モードと Bump モードの結果比較テスト（enzyme-ad feature 有効時）
  - bump_sizes カスタマイズテスト
  - ポートフォリオ集約テスト
  - CSA 条件適用テスト
  - エラーハンドリングテスト
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 8. Service 層統合準備
- [x] 8.1 (P) async ラッパーの実装
  - spawn_blocking による CPU バウンド処理のオフロード（risk_engine_handlers.rs にて実装）
  - compute_greeks(), compute_portfolio_greeks(), compute_scenario_greeks() ハンドラー
  - _Requirements: 9.1_

- [x] 8.2 (P) API リクエスト/レスポンス型の定義
  - GreeksRequest, PortfolioGreeksRequest, ScenarioGreeksRequest 構造体
  - GreeksResponse, PortfolioGreeksResponse, ScenarioGreeksResponse 構造体
  - RiskConfigOverride, MarketShifts, AggregatedGreeksDto など DTO 型
  - pricer_risk 型からの変換実装
  - _Requirements: 9.2, 9.3_

- [x] 8.3 (P) Demo GUI ハンドラーの実装
  - demo/gui/src/web/risk_engine_handlers.rs: POST /api/risk-engine/greeks ハンドラー
  - demo/gui/src/web/risk_engine_handlers.rs: POST /api/risk-engine/portfolio-greeks ハンドラー
  - demo/gui/src/web/risk_engine_handlers.rs: POST /api/risk-engine/scenario-greeks ハンドラー
  - demo/gui/src/web/risk_engine_types.rs: リクエスト/レスポンス型
  - demo/gui/src/web/mod.rs: /api/risk-engine/* ルート登録
  - _Requirements: 9.4, 9.5_

- [x] 8.4 ジョブベース実行パターンの実装
  - run_portfolio_greeks_async(): 100 件超のポートフォリオで非同期ジョブ実行
  - JobManager 統合（進捗報告、完了/失敗ステータス）
  - _Requirements: 9.4_

- [x] 8.5 Service 統合テストの実装
  - risk_engine_types: 5 テスト（シリアライズ/デシリアライズ）
  - risk_engine_handlers: 10 テスト（bump 計算、設定ビルダー、スポットシフト）
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_

---

## Phase 5: コードクリーンアップ ✅

> **ステータス (2026-01-25)**: Phase 5 完了。
>
> **実施内容**:
> - `BumpSizes` を `infra_config` から再エクスポート（A-I-P-S アーキテクチャ準拠）
> - `irs_greeks/` ディレクトリ完全削除（pricer_risk へ移行済み）
> - `greeks/` モジュールを `pub(crate)` に変更（内部用として維持）
> - 全 deprecated re-export を削除
> - ドキュメント例を `rust,ignore` に更新

- [x] 9. pricer_pricing からの Greeks 関連コード削除
- [x] 9.1 deprecated re-export の削除準備
  - ✅ downstream コードが pricer_risk への移行を完了していることを確認
  - ✅ deprecation 警告期間を経ずにクリーンアップ実施（ユーザー要求による）
  - _Requirements: 6.3_

- [x] 9.2 pricer_pricing/src/lib.rs の re-export 削除
  - ✅ greeks モジュールを `pub(crate)` に変更（内部用として維持）
  - ✅ irs_greeks モジュール宣言を削除
  - ✅ 28 個の deprecated re-export を削除
  - _Requirements: 6.3_

- [x] 9.3 pricer_pricing/src/greeks/ を内部モジュールに変更
  - ✅ `pub(crate) mod greeks;` に変更（generic_pricer が使用するため維持）
  - ✅ ドキュメント例を `rust,ignore` に更新
  - _Requirements: 6.3_

- [x] 9.4 pricer_pricing/src/irs_greeks/ ディレクトリの削除
  - ✅ 全 9 ファイル削除（mod.rs, config.rs, error.rs, result.rs, calculator.rs, lazy_evaluator.rs, benchmark.rs, xva_demo.rs, tests.rs）
  - ✅ ディレクトリ自体を削除
  - _Requirements: 6.3_

- [x] 9.5 generic_pricer/greeks_calculator.rs の BumpSizes 更新
  - ✅ 独自 BumpSizes を `infra_config::BumpSizes` の再エクスポートに変更
  - ✅ `infra_config` を常時依存に変更（l1l2-integration 不要）
  - ✅ `#[allow(deprecated)]` 属性を削除
  - _Requirements: 6.3_

- [x] 9.6 削除後の検証
  - ✅ `cargo build -p pricer_pricing` 成功
  - ✅ `cargo build -p pricer_pricing --features l1l2-integration` 成功
  - ✅ `cargo test -p pricer_pricing --lib` 全 797 テスト通過
  - ✅ `cargo test -p pricer_pricing --features l1l2-integration --lib` 全 816 テスト通過
  - ✅ `cargo build -p pricer_risk` 成功
  - ✅ pricer_pricing から Greeks 関連 pub API が消失していることを確認
  - _Requirements: 6.3_

---

## Requirements Coverage Summary

| Requirement | Tasks |
|-------------|-------|
| 1.1 (PricingConfig定義) | 1.1, 1.5 |
| 1.2 (RiskConfig定義) | 1.2, 1.5 |
| 1.3 (ConfigLoader検証) | 1.3, 1.5 |
| 1.4 (ネスト構造サポート) | 1.1 |
| 1.5 (JSON形式サポート) | 1.3, 1.5 |
| 2.1 (TradeLoader) | 2.2, 2.6 |
| 2.2 (MarketLoader) | 2.3, 2.6 |
| 2.3 (CsaLoader) | 2.4, 2.6 |
| 2.4 (エラー詳細) | 2.1, 2.2, 2.5, 2.6 |
| 2.5 (globバッチ読み込み) | 2.1, 2.6 |
| 3.1 (GenericPricer入力) | 3.1, 3.2, 3.3 |
| 3.2 (analyticalメソッド) | 3.1, 3.3 |
| 3.3 (monte_carloメソッド) | 3.1, 3.3 |
| 3.4 (MarketProvider解決) | 3.2, 3.3 |
| 3.5 (MissingMarketDataエラー) | 3.2, 3.3 |
| 4.1 (PortfolioPricer入力) | 4.1, 4.2, 4.5 |
| 4.2 (集約計算) | 4.3, 4.5 |
| 4.3 (Rayon並列処理) | 4.2, 4.5 |
| 4.4 (partial success) | 4.2, 4.5 |
| 4.5 (ExecutionStats) | 4.4, 4.5 |
| 5.1 (RiskEngine入力) | 7.1, 7.2, 7.3, 7.12 |
| 5.2 (AADモード) | 7.2, 7.12 |
| 5.3 (Bumpモード) | 7.2, 7.12 |
| 5.4 (target_greeks選択) | 7.2, 7.12 |
| 5.5 (RiskResult構造) | 7.4, 7.5, 7.12 |
| 6.1 (greeks移行) | 5.1, 5.4, 6.5 |
| 6.2 (irs_greeks移行) | 6.1, 6.4, 6.5 |
| 6.3 (L3 API削除) | 5.3, 6.3, 9.1, 9.2, 9.3, 9.4, 9.5, 9.6 |
| 6.4 (L4 re-export) | 5.1, 5.2, 5.4, 6.2 |
| 6.5 (deprecation警告) | 5.3, 5.4, 6.3, 6.4 |
| 7.1 (factor別bump_sizes) | 1.2, 7.8 |
| 7.2 (second_order_mode) | 1.2, 7.9 |
| 7.3 (CSA条件適用) | 7.10 |
| 7.4 (シナリオベースGreeks) | 1.2, 7.11 |
| 7.5 (ポートフォリオ集約) | 7.5, 7.7 |
| 8.1 (PricingErrorコンテキスト) | 1.4 |
| 8.2 (RiskError診断) | 7.6 |
| 8.3 (ConfigError一括報告) | 1.3, 1.4 |
| 8.4 (数値不安定性エラー) | 7.6 |
| 8.5 (thiserror実装) | 1.4, 2.5, 7.6 |
| 9.1 (async互換IF) | 8.1, 8.5 |
| 9.2 (serde対応設定) | 8.2, 8.5 |
| 9.3 (JSON応答型) | 7.4, 8.2, 8.5 |
| 9.4 (ジョブベース実行) | 8.3, 8.4, 8.5 |
| 9.5 (handlerパターン) | 8.3, 8.5 |
