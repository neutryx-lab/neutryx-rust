# Implementation Plan

## Overview

`pricer_pricing`クレートに統一されたプライシングAPI（`GenericPricer`）を実装する。シンプルな具象構造体設計で、`get_pv()`は`reporting_currency`を必須引数として受け取り、f64固定の`PricingResult`を返す。`get_greeks()`のみEnzyme AD対応のジェネリック関数として実装する。

**Total Tasks**: 8 Major Tasks, 24 Sub-tasks
**Requirements Coverage**: 8 Requirements, 42 Acceptance Criteria（廃止2件を除く）

---

## Tasks

- [x] 1. エラー型の定義

- [x] 1.1 (P) PricingError 列挙型の実装
  - マーケットデータ欠落エラーを表現
  - 未対応商品タイプエラーを表現
  - TradeIdやCurrency等の診断情報を含む
  - thiserrorによる構造化エラーメッセージ
  - _Requirements: 1.5, 5.5_

- [x] 1.2 (P) ConfigError 列挙型の実装
  - 不正なモデルパラメータエラー（num_paths=0等）
  - 不正なプライサー設定エラー
  - パラメータ名と値を含む診断情報
  - _Requirements: 3.6_

- [x] 1.3 (P) MarketDataError の拡張
  - FxRateNotFound バリアント追加（base/quote通貨ペア情報）→ PricingError内に統合
  - SurfaceNotFound バリアント追加（必要に応じて）
  - 既存エラー型との整合性維持
  - _Requirements: 2.5, 6.8_

---

- [x] 2. 設定構造体の実装

- [x] 2.1 ModelConfig と Builder の実装
  - StochasticModelEnum選択フィールド（Option、Noneでデフォルト）
  - シミュレーションパス数、時間ステップ数、乱数シード
  - Builderパターンによる構築API
  - デフォルト値: num_paths=10,000、num_steps=100
  - _Requirements: 3.1, 3.5_

- [x] 2.2 PricerConfig と Builder の実装
  - GreeksConfig保持（計算モード、バンプ幅）
  - デフォルト出力通貨設定
  - スレッドローカルバッファ使用フラグ
  - Builderパターンによる構築API
  - _Requirements: 4.1, 4.4, 6.9_

- [x] 2.3 設定検証ロジックの実装
  - num_paths > 0、num_steps > 0 の検証
  - モデルパラメータの妥当性チェック（Feller条件等）
  - GreeksConfigの整合性検証
  - 検証失敗時はConfigErrorを返す
  - _Requirements: 3.6_

---

- [x] 3. PricingResult 階層構造の実装

- [x] 3.1 (P) CashflowPricingResult の実装
  - 報告通貨建てPV（f64）
  - 元通貨建てPV（f64）
  - 支払日、ディスカウントファクター
  - 元通貨情報
  - _Requirements: 6.3, 6.6_

- [x] 3.2 (P) LegPricingResult の実装
  - 報告通貨建てPV、元通貨建てPV
  - 使用したFXレート、元通貨
  - 支払/受取方向（Direction）
  - Cashflow結果のベクタ保持
  - _Requirements: 6.3, 6.5_

- [x] 3.3 PathDistribution の実装
  - mean、std_dev、percentiles（f64固定）
  - path_count
  - MC計算時のみ使用（Option型で保持）
  - _Requirements: 6.7_

- [x] 3.4 PricingResult の実装
  - total_pv（報告通貨建て、f64）
  - legs ベクタ、reporting_currency
  - path_distribution（Option）
  - by_leg()、by_cashflow()、by_path() メソッド
  - group_by_currency() メソッド（Leg単位から動的集計）
  - _Requirements: 1.4, 6.3, 6.4, 6.5, 6.6, 6.7_

---

- [x] 4. GenericPricer コア実装

- [x] 4.1 GenericPricer 構造体の定義
  - MarketProvider（Arc）保持
  - ModelConfig、PricerConfig保持
  - new() コンストラクタ
  - _Requirements: 2.1_

- [x] 4.2 マーケットデータ解決ロジック
  - Tradeの各通貨に対応するディスカウントカーブ取得
  - 必要なVolSurfaceの取得
  - 報告通貨へのFXレート取得（MarketProvider::get_fx_rate）
  - データ欠落時のエラーハンドリング
  - _Requirements: 2.2, 2.3, 2.4, 6.1, 6.2_

- [x] 4.3 デフォルトモデル選択ロジック
  - ModelConfig.modelがNone時の商品タイプ別デフォルト選択
  - 金利商品→Hull-White、株式→GBM等のマッピング
  - _Requirements: 3.2, 3.3_

- [x] 4.4 get_pv() メソッドの実装
  - Trade、評価日、報告通貨を受け取る
  - マーケットデータ解決 → カーネル実行 → FX換算
  - PricingResult（f64固定）を返す
  - エラー時はPricingErrorを返す
  - _Requirements: 1.1, 5.1_

---

- [x] 5. プライシングカーネル実装

- [x] 5.1 Trade/Leg/Cashflow パース処理
  - Trade構造からLeg配列を取得
  - 各LegからCashflow配列を展開
  - 静的ディスパッチによるInstrumentEnum処理
  - _Requirements: 5.2, 5.3, 5.4_

- [x] 5.2 Cashflow単位のプライシング
  - 支払日までの年率計算（DayCountConvention使用）
  - ディスカウントファクター取得
  - 元通貨建てPV計算
  - _Requirements: 7.3, 7.4_

- [x] 5.3 Leg単位のプライシング
  - 各CashflowのPVを集計
  - 方向（Pay/Receive）の適用
  - FXレートによる報告通貨換算
  - LegPricingResultの構築
  - _Requirements: 6.2, 6.5_

- [x] 5.4 日付処理ユーティリティ
  - Calendar、DayCountConvention、Frequencyとの統合
  - 営業日調整（Following、ModifiedFollowing等）
  - NaiveDateの一貫した使用
  - _Requirements: 7.1, 7.2, 7.5_

---

- [x] 6. Greeks計算実装

- [x] 6.1 get_greeks<T: Float>() メソッドの実装
  - ジェネリック関数としてEnzyme AD対応
  - GreeksConfigに基づくモード選択
  - GreeksResult<T>を返す
  - _Requirements: 1.3_

- [x] 6.2 AADモード実装
  - Enzyme ADを使用した自動微分
  - Delta、Gamma、Vega等の計算
  - pricer_pricingのEnzyme機能との統合
  - _Requirements: 4.2_

- [x] 6.3 BumpAndRevalueモード実装
  - 有限差分法によるGreeks計算
  - バンプ幅設定の適用
  - 中心差分または前方差分の選択
  - _Requirements: 4.3_

---

- [x] 7. バッチプライシング実装

- [x] 7.1 BatchPricer 構造体の実装
  - MarketProvider（Arc）、ModelConfig、PricerConfig保持
  - GenericPricerとの構成関係
  - _Requirements: 8.3_

- [x] 7.2 price_batch() メソッドの実装
  - 複数Tradeの並列プライシング
  - Rayon par_iter使用
  - 評価日と報告通貨の共通設定
  - _Requirements: 8.1, 8.2_

- [x] 7.3 BatchPricingResult の実装
  - 成功結果: Vec<(TradeId, PricingResult)>
  - 失敗結果: Vec<(TradeId, PricingError)>
  - 統計情報: total_count、success_count、failure_count、elapsed_ms
  - _Requirements: 8.4_

- [x] 7.4 部分エラー継続処理
  - 一部商品のエラー時も他商品の処理を継続
  - エラー情報の収集と結果への含有
  - _Requirements: 8.5_

- [ ] 7.5* スレッドローカルバッファプール（オプション）
  - バッチ処理時のメモリ割り当て最適化
  - Rayon work-stealing との整合性
  - _Requirements: 4.5_

---

- [x] 8. 統合とテスト

- [x] 8.1 モジュール構成と公開API
  - generic_pricer/ モジュール作成
  - lib.rsからの公開エクスポート
  - 既存コードとの整合性確認
  - _Requirements: 5.4_

- [x] 8.2 (P) 単体テスト
  - ModelConfig/PricerConfig検証テスト
  - PricingResult階層構造テスト
  - エラーハンドリングテスト
  - _Requirements: 1.5, 3.6, 5.5, 6.8_

- [x] 8.3 (P) 統合テスト
  - GenericPricer::get_pv() E2Eテスト
  - FX換算を含むマルチ通貨プライシング
  - BatchPricer並列処理テスト
  - _Requirements: 1.1, 6.2, 8.1_

- [ ] 8.4* パフォーマンステスト（オプション）
  - バッチプライシング並列効率測定
  - メモリ使用量確認
  - 8コアで80%以上のスケーリング目標
  - _Requirements: 8.2_

---

## Requirements Coverage Matrix

| Requirement | Tasks |
|-------------|-------|
| 1.1 | 4.4 |
| 1.2 | ※廃止（1.1に統合） |
| 1.3 | 6.1 |
| 1.4 | 3.4 |
| 1.5 | 1.1, 8.2 |
| 2.1 | 4.1 |
| 2.2 | 4.2 |
| 2.3 | 4.2 |
| 2.4 | 4.2 |
| 2.5 | 1.3 |
| 3.1 | 2.1 |
| 3.2 | 4.3 |
| 3.3 | 4.3 |
| 3.4 | (キャリブレーション統合は別スコープ) |
| 3.5 | 2.1 |
| 3.6 | 1.2, 2.3, 8.2 |
| 4.1 | 2.2 |
| 4.2 | 6.2 |
| 4.3 | 6.3 |
| 4.4 | 2.2 |
| 4.5 | 7.5 |
| 5.1 | 4.4 |
| 5.2 | 5.1 |
| 5.3 | 5.1 |
| 5.4 | 5.1, 8.1 |
| 5.5 | 1.1, 8.2 |
| 6.1 | 4.2 |
| 6.2 | 4.2, 5.3 |
| 6.3 | 3.1, 3.2, 3.4 |
| 6.4 | 3.4 |
| 6.5 | 3.2, 3.4 |
| 6.6 | 3.1, 3.4 |
| 6.7 | 3.3, 3.4 |
| 6.8 | 1.3, 8.2 |
| 6.9 | 2.2 |
| 7.1 | 5.4 |
| 7.2 | 5.4 |
| 7.3 | 5.2 |
| 7.4 | 5.2 |
| 7.5 | 5.4 |
| 8.1 | 7.2 |
| 8.2 | 7.2, 8.4 |
| 8.3 | 7.1 |
| 8.4 | 7.3 |
| 8.5 | 7.4 |

## Notes

- Requirement 1.2は廃止（reporting_currencyはget_pv()の必須引数に統合）
- Requirement 3.4（キャリブレーション連携）は既存キャリブレーション機能との統合として別スコープで対応可能
- Requirement 6.4はCurrencyBreakdown廃止に伴いgroup_by_currency()で対応
- Task 7.5（スレッドローカルバッファ）と8.4（パフォーマンステスト）はオプション
- 並列実行可能タスク (P) はエラー型定義、PricingResult構造、テストに適用
