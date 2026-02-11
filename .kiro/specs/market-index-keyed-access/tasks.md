# Implementation Plan: Market Index-Keyed Access

## Phase 1: 基盤型定義

- [x] 1. IndexRequirementとエラー型の定義
- [x] 1.1 IndexRequirement enum を定義する
- [x] 1.2 MarketError に Index関連エラーバリアントを追加する
- [x] 1.3 MarketBuildError 型を定義する

## Phase 2: IndexedMarket ファサード実装

- [x] 2. IndexedMarket 構造体の実装
- [x] 2.1 IndexedMarket の基本構造を実装する
- [x] 2.2 Curve アクセスAPI を実装する
- [x] 2.3 VolCube アクセスAPI を実装する
- [x] 2.4 FX アクセスAPI を実装する
- [x] 2.5 Index存在確認メソッドを実装する

## Phase 3: IndexedMarketBuilder 実装

- [x] 3. IndexedMarketBuilder の実装
- [x] 3.1 Builder 基本構造を実装する
- [x] 3.2 Curve 登録メソッドを実装する
- [x] 3.3 VolCube 登録メソッドを実装する
- [x] 3.4 FX 登録メソッドを実装する
- [x] 3.5 既存コンポーネント統合メソッドを実装する
- [x] 3.6 build() メソッドを実装する

## Phase 4: Trade Index 抽出機能

- [x] 4. TradeIndexRequirements trait の実装
- [x] 4.1 TradeIndexRequirements trait を定義する
- [x] 4.2 Trade への blanket implementation を実装する
- [x] 4.3 各Cashflowタイプ別のIndex抽出を実装する

## Phase 5: Market 網羅性検証

- [x] 5. MarketValidator の実装
- [x] 5.1 MarketValidator 構造体を実装する
- [x] 5.2 単一Trade検証を実装する
- [x] 5.3 Portfolio検証を実装する
- [x] 5.4 IndexedMarket に validate_completeness を統合する

## Phase 6: 統合と後方互換性

- [x] 6. 統合テストと後方互換性
- [x] 6.1 CurveSet fallback 統合テストを実装する
- [x] 6.2 VolCubeCache 統合テストを実装する
- [x] 6.3 MarketProvider FxCurve 統合テストを実装する
- [x] 6.4 非推奨API属性を追加する
- [x] 6.5 性能ベンチマークを実装する
