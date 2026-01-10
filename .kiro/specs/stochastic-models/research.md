# Research & Design Decisions: stochastic-models

## Summary

- **Feature**: `stochastic-models`
- **Discovery Scope**: Extension (既存システム拡張)
- **Key Findings**:
  - SABR の StochasticModel トレイト実装が最大のギャップ
  - キャリブレーション機能は pricer_optimiser に一元化すべき
  - 既存の Heston/Hull-White は高品質で、パターン踏襲可能

## Research Log

### SABR 離散化スキーム

- **Context**: SABR モデルの `evolve_step` 実装に適切な離散化スキームを決定
- **Sources Consulted**:
  - Andersen, L. (2008) "Efficient Simulation of the Heston Stochastic Volatility Model"
  - 既存 heston.rs の QE スキーム実装
- **Findings**:
  - SABR は CEV-type dynamics のため、標準 Euler-Maruyama で分散発散リスクあり
  - beta=0 (Normal) と beta=1 (Lognormal) は closed-form 解あり
  - 一般的な beta では Log-Euler または Milstein が推奨
- **Implications**:
  - Phase 1: Euler-Maruyama + absorption で実装
  - Phase 2: 精度向上が必要な場合、Log-Euler に移行

### StochasticModelEnum 拡張パターン

- **Context**: SABR を StochasticModelEnum に追加する際の既存パターン分析
- **Sources Consulted**:
  - [model_enum.rs](crates/pricer_models/src/models/model_enum.rs)
  - [heston.rs:1044](crates/pricer_models/src/models/heston.rs#L1044)
- **Findings**:
  - GBM, Heston, HullWhite, CIR がすでに enum バリアントとして存在
  - マクロベースの静的ディスパッチパターン確立済み
  - Feature flag による条件付きコンパイルパターンあり
- **Implications**:
  - 既存パターンに従い SABR バリアント追加
  - `rates` feature flag でゲート

### キャリブレーション統合

- **Context**: モデル別 calibrate 関数の配置決定
- **Sources Consulted**:
  - [pricer_optimiser/src/calibration/](crates/pricer_optimiser/src/calibration/)
  - A-I-P-S アーキテクチャガイドライン
- **Findings**:
  - `CalibrationEngine` は汎用 pricer 関数を受け取る設計
  - Levenberg-Marquardt ソルバーが利用可能
  - モデル固有ロジック (目的関数) はモデル側で定義すべき
- **Implications**:
  - ハイブリッドアプローチ採用
  - 目的関数は各モデルの associated function として定義
  - ソルバー呼び出しは pricer_optimiser 経由

### Hull-White θ(t) 時間依存性

- **Context**: イールドカーブフィッティング用の時間依存ドリフト
- **Sources Consulted**:
  - [hull_white.rs](crates/pricer_models/src/models/rates/hull_white.rs)
  - Hull-White (1990) 原論文
- **Findings**:
  - 現実装は θ を定数として扱う
  - 時間依存 θ(t) は forward curve から導出
  - `YieldCurve` トレイトからの θ(t) 計算が必要
- **Implications**:
  - `theta_function` パラメータを追加
  - Closure または lookup table ベースの実装

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Option A: 既存拡張 | 全てを既存ファイル内に追加 | 最小変更 | ファイル肥大化 | - |
| Option B: 新規モジュール | キャリブレーションを別モジュール化 | 関心分離 | 依存関係複雑化 | - |
| Option C: ハイブリッド | モデルは既存拡張、キャリブレーションは pricer_optimiser | バランス良好 | 2クレート調整 | **採用** |

## Design Decisions

### Decision: SABR StochasticModel 配置

- **Context**: SABR の StochasticModel 実装をどこに配置するか
- **Alternatives Considered**:
  1. 新規 `models/rates/sabr.rs` に移動
  2. 既存 `models/sabr.rs` に追加
- **Selected Approach**: 既存 `models/sabr.rs` に追加
- **Rationale**:
  - 既に SABRParams, SABRModel, Hagan 公式が同ファイルにある
  - 分離するとインポート複雑化
- **Trade-offs**: ファイルサイズ増加 (許容範囲)
- **Follow-up**: ファイルが 1500 行超えたら分割検討

### Decision: キャリブレーション API 設計

- **Context**: モデルにキャリブレーション機能を追加する方法
- **Alternatives Considered**:
  1. モデル構造体にメソッド追加 (`model.calibrate(data)`)
  2. Associated function (`HestonModel::calibrate(data)`)
  3. Trait ベース (`Calibratable` trait)
- **Selected Approach**: Associated function
- **Rationale**:
  - キャリブレーションはモデル構築前に実行
  - 既存パラメータ不要 (market data → params)
  - Trait は過剰 (モデル固有ロジック)
- **Trade-offs**: 共通インターフェースなし (許容)
- **Follow-up**: 必要に応じて Calibratable trait 追加

### Decision: TwoFactorState 再利用

- **Context**: SABR の状態ベクトル型
- **Alternatives Considered**:
  1. 新規 `SABRState<T>` 定義
  2. 既存 `TwoFactorState<T>` 再利用
- **Selected Approach**: `TwoFactorState<T>` 再利用
- **Rationale**:
  - SABR は [forward, volatility] の2因子
  - Heston の [spot, variance] と同構造
  - コード重複回避
- **Trade-offs**: フィールド名の意味が異なる
- **Follow-up**: ドキュメントで明確化

## Risks & Mitigations

- **SABR 離散化の数値安定性** — Absorption scheme + smooth_sqrt で負値回避
- **Enzyme AD 互換性未検証** — num-dual で先に検証、Enzyme は後続フェーズ
- **キャリブレーション収束失敗** — 適切な初期値、パラメータ境界制約
- **Feature flag 整合性** — Heston は `equity`、SABR は `rates` で異なる

## References

- [Hagan et al. (2002)](https://www.researchgate.net/publication/235622441) — SABR Implied Volatility Formula
- [Andersen (2008)](https://ssrn.com/abstract=946405) — Quadratic Exponential Scheme for Heston
- [Hull-White (1990)](https://www.jstor.org/stable/2328253) — One-Factor Interest Rate Model
- [pricer_models/stochastic.rs](crates/pricer_models/src/models/stochastic.rs) — StochasticModel Trait Definition
