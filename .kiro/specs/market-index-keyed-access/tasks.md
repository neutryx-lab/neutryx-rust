# Implementation Plan: Market Index-Keyed Access

## Overview

本実装計画は、PricerにおけるMarketデータアクセスをIndex単位で統一化するためのファサード層実装を定義する。

**コンポーネント**: IndexedMarket, IndexedMarketBuilder, IndexRequirement, TradeIndexRequirements, MarketValidator
**レイヤー**: infra_master::trade (型定義), pricer_models::market (ファサード)

---

## Phase 1: 基盤型定義

- [x] 1. IndexRequirementとエラー型の定義
- [x] 1.1 (P) IndexRequirement enum を定義する
  - Trade/Cashflowが必要とするIndexの型表現を定義
  - RateCurve, SwaptionVol, FxCurve, FxVol の4バリアントを実装
  - Hash, Eq, Clone, Debug, PartialEq derive を追加
  - 各バリアントは対応するIndex型（RateIndex, CurrencyPair）を保持
  - _Requirements: 1.1, 1.2, 1.3, 1.5_

- [x] 1.2 (P) MarketError に Index関連エラーバリアントを追加する
  - IndexNotFound エラー（存在しないIndex参照時）を追加
  - CurveNotBuilt エラー（Curve未構築時）を追加
  - VolCubeNotCalibrated エラー（VolCube未キャリブレーション時）を追加
  - 既存MarketError enum への拡張として実装
  - _Requirements: 1.4, 2.6, 3.5_

- [x] 1.3 (P) MarketBuildError 型を定義する
  - DuplicateIndexMapping エラー（重複Index登録時）を定義
  - IndexNotSpecified エラー（Builder でIndex未指定時）を定義
  - InvalidValuationDate エラー（無効な評価日）を定義
  - thiserror による構造化エラー型として実装
  - _Requirements: 4.5, 6.5_

## Phase 2: IndexedMarket ファサード実装

- [x] 2. IndexedMarket 構造体の実装
- [x] 2.1 IndexedMarket の基本構造を実装する
  - valuation_date と内部HashMap（curves, volcubes, fx_curves, fx_vol_surfaces）を定義
  - CurveSet と IndexCurveMapper への fallback 参照を保持
  - T: Float + Send + Sync のジェネリクス制約を適用
  - Arc による共有所有権パターンを実装
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 2.2 (P) Curve アクセスAPI を実装する
  - get_df(index, term) で Discount Factor を取得する機能を実装
  - get_forward_rate(index, start, end) で forward rate を取得する機能を実装
  - get_zero_rate(index, term) で zero rate を取得する機能を実装
  - キー不在時は IndexNotFound エラーを返却
  - CurveSet への fallback 動作を実装
  - _Requirements: 2.1, 2.2, 2.3, 2.4_

- [x] 2.3 (P) VolCube アクセスAPI を実装する
  - get_swaption_vol(index, expiry, tenor, strike) で swaption vol を取得する機能を実装
  - get_bs_vol(index, expiry, strike) で Black-Scholes vol を取得する機能を実装
  - RateIndex から VolCubeProviderKey への変換ロジックを実装
  - SABR補間が有効な場合の任意Strike補間を実装
  - VolCubeCache との統合を実装
  - _Requirements: 3.1, 3.2, 3.4, 3.6_

- [x] 2.4 (P) FX アクセスAPI を実装する
  - get_fx_forward(pair, term) で FX forward を取得する機能を実装
  - get_fx_vol(pair, expiry, strike) で FX vol を取得する機能を実装
  - CurrencyPair でのキー化とMarketProviderとの統合
  - _Requirements: 1.2, 3.3_

- [x] 2.5 Index存在確認メソッドを実装する
  - has_curve(index), has_volcube(index), has_fx_curve(pair), has_fx_vol(pair) を実装
  - available_rate_indices(), available_currency_pairs() を実装
  - valuation_date() アクセサを実装
  - Thread-safe な並列アクセスを保証
  - _Requirements: 5.5, 5.6_

## Phase 3: IndexedMarketBuilder 実装

- [x] 3. IndexedMarketBuilder の実装
- [x] 3.1 Builder 基本構造を実装する
  - new(valuation_date) コンストラクタを定義
  - 内部 HashMap 初期化処理を実装
  - index_mapper オプション保持を実装
  - _Requirements: 6.6_

- [x] 3.2 (P) Curve 登録メソッドを実装する
  - with_curve(index, curve) で RateIndex→YieldCurve を登録する機能を実装
  - 重複Index検出と DuplicateIndexMapping エラー返却を実装
  - Arc<dyn YieldCurve> の型制約を適用
  - _Requirements: 2.5, 6.1_

- [x] 3.3 (P) VolCube 登録メソッドを実装する
  - with_volcube(index, volcube) で RateIndex→VolCube を登録する機能を実装
  - 重複Index検出と DuplicateIndexMapping エラー返却を実装
  - _Requirements: 3.4, 6.2_

- [x] 3.4 (P) FX 登録メソッドを実装する
  - with_fx_curve(pair, curve) で CurrencyPair→FxCurve を登録する機能を実装
  - with_fx_vol_surface(pair, surface) で CurrencyPair→VolSurface を登録する機能を実装
  - 重複ペア検出とエラー返却を実装
  - _Requirements: 6.3, 6.4_

- [x] 3.5 既存コンポーネント統合メソッドを実装する
  - with_curve_set(curve_set) で既存 CurveSet を統合する機能を実装
  - with_index_mapper(mapper) で既存 IndexCurveMapper を設定する機能を実装
  - fallback 動作の有効化を実装
  - _Requirements: 4.6_

- [x] 3.6 build() メソッドを実装する
  - IndexedMarket<T> の構築と返却を実装
  - valuation_date 整合性検証を実装
  - Immutable な Market 構造体の生成を保証
  - _Requirements: 6.6_

## Phase 4: Trade Index 抽出機能

- [x] 4. TradeIndexRequirements trait の実装
- [x] 4.1 TradeIndexRequirements trait を定義する
  - required_indices() -> Vec<IndexRequirement> メソッド定義
  - Trade, Leg, Cashflow の階層を辿る抽出ロジック設計
  - _Requirements: 7.3_

- [x] 4.2 Trade への blanket implementation を実装する
  - Trade → Vec<Leg> → Vec<Cashflow> → IndexObservation からIndex抽出
  - IndexType::Rate は RateCurve、IndexType::Fx は FxCurve に変換
  - 重複Index除去と sort 処理を実装
  - _Requirements: 7.3_

- [x] 4.3 各Cashflowタイプ別のIndex抽出を実装する
  - Fixed Cashflow は Index不要として処理
  - Floating Cashflow は RateIndex 抽出
  - FX Cashflow は CurrencyPair 抽出
  - Option Cashflow は Vol Index 追加
  - _Requirements: 7.3_

## Phase 5: Market 網羅性検証

- [x] 5. MarketValidator の実装
- [x] 5.1 MarketValidator 構造体を実装する
  - IndexedMarket への参照を保持
  - lifetime 'a でMarket借用を管理
  - MissingIndex 型を定義（requirement と context を含む）
  - _Requirements: 7.1_

- [x] 5.2 単一Trade検証を実装する
  - validate_trade(trade) で単一Tradeの必要Indexを検証
  - 不足Index は Vec<MissingIndex> で返却
  - MissingIndex に requirement と context を含める
  - _Requirements: 7.2, 7.5_

- [x] 5.3 Portfolio検証を実装する
  - validate_portfolio(trades) で複数Tradeを一括検証
  - 全Tradeの required_indices を集約
  - 重複除去後に Market との照合を実行
  - _Requirements: 7.4_

- [x] 5.4 IndexedMarket に validate_completeness を統合する
  - validate_completeness(required) -> Result<(), Vec<MissingIndex>> を追加
  - MarketValidator との連携を実装
  - _Requirements: 7.1_

## Phase 6: 統合と後方互換性

- [ ] 6. 統合テストと後方互換性
- [ ] 6.1 (P) CurveSet fallback 統合テストを実装する
  - IndexedMarket 経由と CurveSet 直接アクセスの結果一致を検証
  - forward_rate_for_index との互換性検証
  - _Requirements: 4.1, 4.2_

- [ ] 6.2 (P) VolCubeCache 統合テストを実装する
  - lazy evaluation との連携動作を検証
  - キャッシュヒット率の確認
  - _Requirements: 4.2_

- [ ] 6.3 (P) MarketProvider FxCurve 統合テストを実装する
  - CurrencyPair でのアクセスと Currency でのアクセスの一貫性を検証
  - _Requirements: 4.3, 4.4_

- [ ] 6.4 非推奨API属性を追加する
  - CurveSet 直接アクセスに #[deprecated] 属性を追加
  - deprecation warning メッセージに移行先APIを記載
  - _Requirements: 8.1, 8.3_

- [ ] 6.5 性能ベンチマークを実装する
  - HashMap lookup overhead の測定（1000 Index）
  - 大規模Portfolio検証（10000 trades）の性能測定
  - get_df() latency < 100ns の確認
  - _Requirements: 5.6_

---

## Requirements Mapping

| Requirement | Tasks |
|-------------|-------|
| 1.1, 1.2, 1.3, 1.5 | 1.1 |
| 1.4 | 1.2 |
| 2.1, 2.2, 2.3, 2.4 | 2.2 |
| 2.5 | 3.2 |
| 2.6 | 1.2 |
| 3.1, 3.2, 3.4, 3.6 | 2.3 |
| 3.3 | 2.4 |
| 3.5 | 1.2 |
| 4.1, 4.2 | 6.1 |
| 4.2 | 6.2 |
| 4.3, 4.4 | 6.3 |
| 4.5 | 1.3 |
| 4.6 | 3.5 |
| 5.1, 5.2, 5.3, 5.4, 5.5 | 2.1 |
| 5.5, 5.6 | 2.5 |
| 6.1 | 3.2 |
| 6.2 | 3.3 |
| 6.3, 6.4 | 3.4 |
| 6.5 | 1.3 |
| 6.6 | 3.1, 3.6 |
| 7.1 | 5.1, 5.4 |
| 7.2, 7.5 | 5.2 |
| 7.3 | 4.1, 4.2, 4.3 |
| 7.4 | 5.3 |
| 8.1, 8.3 | 6.4 |
| 8.2, 8.4, 8.5 | Documentation (out of scope) |

---

_Generated: 2026-01-26_
