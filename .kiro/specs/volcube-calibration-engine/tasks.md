# Implementation Plan

## Task Overview

**Feature**: volcube-calibration-engine
**Total Requirements**: 10 (47 acceptance criteria)
**Estimated Tasks**: 9 major tasks, 23 sub-tasks

---

## Tasks

- [ ] 1. 基盤型とカリブレーション設定の定義
- [ ] 1.1 (P) カリブレーション設定用列挙型の実装
  - Smile補間方式（SABR, SVI, Linear, CubicSpline, FlatVol）を表す列挙型を定義
  - Extrapolation方式（Flat, Linear, Error）を表す列挙型を定義
  - Strike軸表現（Absolute, Moneyness, LogMoneyness, Delta）を表す列挙型を定義
  - 最適化アルゴリズム（LevenbergMarquardt, NelderMead）を表す列挙型を定義
  - 各列挙型にDebug, Clone, Copy, Defaultトレイトを実装
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 10.1, 10.2_

- [ ] 1.2 (P) VolCubeConfig構造体の実装
  - 補間・外挿・Strike軸・最適化アルゴリズムを保持する設定構造体を定義
  - Arbitrage-free検証フラグ、SABRベータ値、Shiftパラメータを含める
  - Defaultトレイトを実装し、標準的な設定値を提供
  - Builderパターンによる設定構築APIを提供
  - _Requirements: 6.5_

- [ ] 1.3 (P) volcubeモジュール構造の設定
  - `pricer_models::market`配下に`volcube/`サブモジュールを作成
  - mod.rsでpublic APIをエクスポート
  - A-I-P-Sアーキテクチャに準拠したモジュール配置を確認
  - _Requirements: 8.1, 8.2_

- [ ] 2. エラーハンドリング基盤
- [ ] 2.1 CalibrationError型の拡張
  - 収束失敗エラー（反復回数、残差、パラメータ値を含む）を定義
  - 入力データ不足エラー（必要数と実際の数を含む）を定義
  - 入力不正エラー（詳細メッセージを含む）を定義
  - Arbitrage-free条件違反エラー（条件、expiry、strikeを含む）を定義
  - thiserrorマクロによる構造化エラーメッセージを実装
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [ ] 2.2 カリブレーション診断情報の実装
  - カリブレーション結果に診断情報（iterations, residuals, parameter_values）を付与
  - エラー発生時に診断情報を含める仕組みを構築
  - _Requirements: 7.5_

- [ ] 3. キャッシュインフラストラクチャ
- [ ] 3.1 VolCubeKeyの実装
  - Instrumentリストのハッシュ値を計算するロジックを実装
  - 設定のハッシュ値を計算するロジックを実装
  - タイムスタンプを含むキャッシュキー構造を定義
  - キーの等価性判定とHashトレイトを実装
  - _Requirements: 5.1_

- [ ] 3.2 VolCubeCacheの実装
  - LRUキャッシュ戦略を採用したキャッシュ構造を実装
  - parking_lot::RwLockによるスレッドセーフなアクセスを提供
  - lookup、insert、invalidate、clearメソッドを実装
  - キャッシュ統計情報（ヒット率、エントリ数、メモリ使用量）を提供
  - 最大容量設定によるメモリ使用量制限を実装
  - _Requirements: 5.2, 5.3, 5.4, 5.5_

- [ ] 4. VolCubeコア実装
- [ ] 4.1 SabrParameterSurface構造体の実装
  - expiry-tenor格子上のSABRパラメータ（alpha, rho, nu）を保持
  - Bilinear補間によるパラメータ取得メソッドを実装
  - 既存のSABR calibrationパターンとの整合性を確保
  - _Requirements: 1.2, 1.5_

- [ ] 4.2 VolCube構造体とVolatilityCubeトレイトの実装
  - 3次元ボラティリティ構造（expiry, tenor, strike）を保持する構造体を定義
  - ジェネリック型`T: Float`によるAD互換性を確保
  - Send + Syncトレイト境界によるスレッドセーフ性を保証
  - ソースInstrument IDリストを保持
  - 設定と格子点（expiries, tenors）を保持
  - _Requirements: 2.3, 2.4, 4.1_

- [ ] 4.3 volatilityメソッドの実装
  - 任意の(expiry, tenor, strike)に対するボラティリティ補間を実装
  - パラメータ補間方式：expiry-tenor平面でSABRパラメータをBilinear補間
  - Strike軸smile計算：補間されたSABRパラメータでHagan公式を適用
  - 外挿設定に応じたドメイン外処理（Flat, Linear, Error）を実装
  - _Requirements: 2.1, 2.2_

- [ ] 4.4 ドメイン範囲メソッドの実装
  - expiry_domain()でexpiry軸の有効範囲を返す
  - tenor_domain()でtenor軸の有効範囲を返す
  - strike_domain()でstrike軸の有効範囲を返す
  - source_instruments()でソースInstrument IDリストを返す
  - _Requirements: 2.5, 4.1_

- [ ] 5. VolCubeBuilder実装
- [ ] 5.1 Builder APIの実装
  - new()で空のBuilderを作成
  - with_instruments()でVolInstrumentリストを設定
  - with_config()でVolCubeConfigを設定
  - with_cache()でVolCubeCacheへの参照を設定
  - Fluent API（メソッドチェイン）パターンを採用
  - _Requirements: 1.3_

- [ ] 5.2 build()メソッドとカリブレーションロジックの実装
  - Instrumentリストが空の場合はInsufficientDataエラーを返す
  - キャッシュが設定されている場合はlookupを試行
  - キャッシュミスの場合、expiry-tenor毎にSABRカリブレーションを実行
  - 既存のSABRCalibratorを活用してパラメータを最適化
  - カリブレーション結果からVolCubeを構築
  - キャッシュに結果を格納
  - 収束失敗時はNotConvergedエラーと診断情報を返す
  - _Requirements: 1.1, 1.4, 5.2, 7.1, 7.5_

- [ ] 6. 確率密度関数の実装
- [ ] 6.1 BreedenLitzenberger計算モジュールの実装
  - Breeden-Litzenberger公式（f(K) = e^(rT) × d²C/dK²）を実装
  - Black-Scholesコール価格計算を活用
  - 中心差分による二次微分を実装
  - delta_kパラメータによる微分幅の調整を可能にする
  - _Requirements: 3.1_

- [ ] 6.2 probability_densityとcumulative_probabilityの実装
  - VolCubeのvolatilityメソッドを使用してstrike±ΔKのvolを取得
  - BreedenLitzenbergerモジュールを呼び出して密度を計算
  - 累積確率はPut価格から導出（CDF = 1 - P/F × e^(rT)）
  - expiry範囲外の場合はMarketDataError::OutOfBoundsを返す
  - 数値安定性のためのsmoothing処理を適用
  - _Requirements: 3.2, 3.3, 3.4_

- [ ] 7. 計算グラフ統合
- [ ] 7.1 GraphExtractable実装
  - VolCube構造体にGraphExtractableトレイトを実装
  - ソースInstrumentをノードとして出力
  - VolCubeとソースInstrument間のエッジを定義
  - D3.js互換のJSON形式で出力可能にする
  - AADモードでの感度計算（Vega, Volga, Vanna）への対応を考慮した設計
  - _Requirements: 4.2, 4.3, 4.4_

- [ ] 8. 統合と拡張性
- [ ] 8.1 (P) VolSurfaceEnumへのCube variant追加
  - 既存のVolSurfaceEnum（static dispatch）にCube(VolCube)を追加
  - 既存の2D VolatilitySurfaceトレイトとの互換性を維持
  - pricer_coreの数学ユーティリティ（interpolators, solvers）との連携を確認
  - _Requirements: 8.3, 8.4, 8.5_

- [ ] 8.2 (P) 拡張性基盤の整備
  - 新しい補間方法を追加可能なenum構造であることを確認
  - トレイトベースの抽象化で新しいカリブレータを受け入れ可能にする
  - feature flagで追加モデル（LocalVol, StochasticLocalVol）を有効化できる構造を準備
  - _Requirements: 10.2, 10.3, 10.4_

- [ ] 9. テストと検証
- [ ] 9.1 (P) 単体テストの実装
  - VolCube::volatilityの既知SABRパラメータでの再現テスト
  - BreedenLitzenberger::probability_densityの解析解との比較テスト
  - VolCubeCacheのLRU evictionとスレッドセーフ性テスト
  - VolCubeConfig::Defaultのデフォルト値検証テスト
  - VolCubeBuilder::buildのend-to-end構築フローテスト
  - _Requirements: 9.3_

- [ ] 9.2 (P) プロパティベーステストの実装
  - proptestによるArbitrage-free条件検証（Butterfly spread ≥ 0）
  - Calendar spread条件の検証
  - Vol monotonicity in strike domainの検証
  - PDF積分 ≈ 1の検証
  - 数学的不変条件（positive vol, non-negative density）の検証
  - _Requirements: 9.1, 9.2_

- [ ] 9.3 (P) ベンチマークとAAD検証の実装
  - criterionによる1000 instrumentカリブレーションスループット計測
  - 10000 volクエリレイテンシ計測
  - キャッシュlookupパフォーマンス計測
  - num-dual検証モードでのAAD正確性検証
  - Vega, Volga, Vanna計算精度の検証
  - _Requirements: 9.4, 9.5_

---

## Requirements Coverage Matrix

| Requirement | Tasks |
|-------------|-------|
| 1.1 | 5.2 |
| 1.2 | 4.1 |
| 1.3 | 5.1 |
| 1.4 | 5.2 |
| 1.5 | 4.1, 4.2 |
| 2.1 | 4.3 |
| 2.2 | 4.3 |
| 2.3 | 4.2 |
| 2.4 | 4.2 |
| 2.5 | 4.4 |
| 3.1 | 6.1 |
| 3.2 | 6.2 |
| 3.3 | 6.2 |
| 3.4 | 6.2 |
| 4.1 | 4.2, 4.4 |
| 4.2 | 7.1 |
| 4.3 | 7.1 |
| 4.4 | 7.1 |
| 5.1 | 3.1 |
| 5.2 | 3.2, 5.2 |
| 5.3 | 3.2 |
| 5.4 | 3.2 |
| 5.5 | 3.2 |
| 6.1 | 1.1 |
| 6.2 | 1.1 |
| 6.3 | 1.1 |
| 6.4 | 1.1 |
| 6.5 | 1.2 |
| 7.1 | 2.1, 5.2 |
| 7.2 | 2.1 |
| 7.3 | 2.1 |
| 7.4 | 2.1 |
| 7.5 | 2.2, 5.2 |
| 8.1 | 1.3 |
| 8.2 | 1.3 |
| 8.3 | 8.1 |
| 8.4 | 8.1 |
| 8.5 | 8.1 |
| 9.1 | 9.2 |
| 9.2 | 9.2 |
| 9.3 | 9.1 |
| 9.4 | 9.3 |
| 9.5 | 9.3 |
| 10.1 | 1.1 |
| 10.2 | 1.1, 8.2 |
| 10.3 | 8.2 |
| 10.4 | 8.2 |

---

## Dependency Graph

```
Task 1 (Config/Types) ─────┬──────────────────────────────┐
                           │                              │
Task 2 (Errors) ───────────┼──────────────────────────────┤
                           │                              │
                           ▼                              │
                    Task 3 (Cache) ───────────────────────┤
                           │                              │
                           ▼                              │
                    Task 4 (VolCube Core) ────────────────┤
                           │                              │
              ┌────────────┼────────────┬─────────────────┤
              │            │            │                 │
              ▼            ▼            ▼                 │
       Task 5 (Builder)  Task 6 (PDF)  Task 7 (Graph)    │
              │            │            │                 │
              └────────────┼────────────┘                 │
                           │                              │
                           ▼                              │
                    Task 8 (Integration) ─────────────────┘
                           │
                           ▼
                    Task 9 (Tests)
```

**Parallel Execution Groups**:
- Group A: Task 1.1, 1.2, 1.3 (P) - 独立した型定義
- Group B: Task 8.1, 8.2 (P) - 独立した統合作業
- Group C: Task 9.1, 9.2, 9.3 (P) - 独立したテスト種別

---

_Generated: 2026-01-23_
