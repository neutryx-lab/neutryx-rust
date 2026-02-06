# Research & Design Decisions

---
**Purpose**: グローバルソルバー機能に関するディスカバリー結果、アーキテクチャ調査、設計根拠を記録する。

**Usage**:
- ディスカバリーフェーズでの調査結果をログ
- design.md には詳細すぎる設計決定のトレードオフを文書化
- 将来の監査や再利用のための参照とエビデンスを提供
---

## Summary

- **Feature**: `curve-global-solver`
- **Discovery Scope**: Extension（既存システムの拡張）
- **Key Findings**:
  1. `MultidimensionalNewtonSolver` と `SystemOfEquations` トレイトが `pricer_core::math::solvers` に既存
  2. `ImplicitSolver` が `pricer_risk::greeks::ad` に既存、AAD統合の基盤として利用可能
  3. `GlobalBootstrapper` の基本実装が `pricer_models::builder::globalsolver` に既存（feature-gated）
  4. nalgebra の `DMatrix`/`DVector` による行列演算が標準パターン

## Research Log

### 既存ソルバーインフラストラクチャ

- **Context**: グローバルキャリブレーションに使用する多次元Newton-Raphson実装の調査
- **Sources Consulted**:
  - `crates/pricer_core/src/math/solvers/mod.rs`
  - `crates/pricer_core/src/math/solvers/multidim_newton.rs`
- **Findings**:
  - `MultidimensionalNewtonSolver<T>` は `T: RealField + Copy + Float` でジェネリック
  - `SystemOfEquations<T>` トレイトが `evaluate()` と `jacobian()` を定義
  - `MultidimSolverResult<T>` に `jacobian_inverse: Option<DMatrix<T>>` が含まれる
  - 数値Jacobianのフォールバック実装（`jacobian_numerical`）が提供済み
  - `MultidimNewtonConfig` に `store_jacobian_inverse: bool` オプションあり
- **Implications**:
  - 新規ソルバー実装は不要、既存の `MultidimensionalNewtonSolver` を再利用
  - `SystemOfEquations` トレイトを実装した `CurveCalibrationProblem` を作成
  - Jacobian逆行列のAAD統合は既存インフラで対応可能

### AAD統合インフラストラクチャ

- **Context**: 陰関数定理によるAAD統合パターンの調査
- **Sources Consulted**:
  - `crates/pricer_risk/src/greeks/ad/implicit_solver.rs`
  - `crates/pricer_risk/src/greeks/ad/shadow.rs`
- **Findings**:
  - `ImplicitSolver::compute_curve_sensitivities()` が J⁻ᵀ · ∂L/∂x* を計算
  - `CurveSensitivities<T>` が市場データ感応度を保持
  - Finite Differenceフォールバック（`compute_curve_sensitivities_fd`）が提供
  - `Shadow` トレイトが勾配蓄積用のシャドウオブジェクト生成を定義
  - `SimpleYieldCurve`/`SimpleVolSurface` が Shadow 実装例として存在
- **Implications**:
  - `GlobalBootstrapResult` の `jacobian_inverse` を `ImplicitSolver` に渡すだけでAAD統合完了
  - カスタム Shadow 実装は不要、既存パターンを踏襲

### 既存GlobalBootstrapper実装

- **Context**: 既存のグローバルブートストラッパー実装の状態確認
- **Sources Consulted**:
  - `crates/pricer_models/src/builder/globalsolver.rs`
  - `crates/pricer_models/src/builder/mod.rs`
- **Findings**:
  - `GlobalBootstrapper<T>` が基本的な多次元Newton-Raphson実装を持つ
  - x = log(DF) パラメータ化で DF > 0 を保証
  - 数値Jacobian（有限差分）のみ対応、解析Jacobianは未実装
  - `GlobalBootstrapConfig<T>` で許容誤差、最大反復数、補間法を設定可能
  - `GlobalBootstrapResult<T>` に `jacobian_inverse: Option<DMatrix<T>>` あり
  - feature gate: `global-bootstrap`
- **Implications**:
  - 既存実装の拡張として設計（破壊的変更を回避）
  - テレスコープ法、解析Jacobian、時間グリッドビルダーを追加

### CalibrationInstrumentトレイト

- **Context**: キャリブレーション商品インターフェースの調査
- **Sources Consulted**:
  - `crates/pricer_models/src/builder/instrument.rs`
  - `crates/pricer_models/src/market.rs`
- **Findings**:
  - `CalibrationInstrument<T>` トレイトが `market_rate()`, `theoretical_rate()`, `maturity()`, `pricing_error()` を定義
  - `MarketInstrument<T>` enum が OIS, IRS, FRA, Future をサポート
  - 理論レート計算ヘルパー: `compute_ois_par_rate`, `compute_irs_par_rate`, `compute_fra_rate`
  - テレスコープ法は未実装（日次ループではなく簡略化された計算）
- **Implications**:
  - テレスコープ評価器を別モジュールとして追加
  - Jacobian行要素計算のためのインターフェース拡張が必要

### 線形代数バックエンド

- **Context**: 行列演算と分解のバックエンド調査
- **Sources Consulted**:
  - `crates/pricer_core/src/math/linalg/mod.rs`
  - `Cargo.toml` (pricer_core)
- **Findings**:
  - nalgebra ベースの `DMatrix<T>`/`DVector<T>` が標準
  - `lu_solve`, `inverse` が `pricer_core::math::linalg` で提供
  - `LinearAlgebraError` が特異行列エラーを表現
  - feature flag: `linalg` (nalgebraの有効化)
- **Implications**:
  - ndarray ではなく nalgebra を使用（既存パターンに従う）
  - BLAS最適化は将来の拡張として検討

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 既存GlobalBootstrapper拡張 | 現在の globalsolver.rs にテレスコープ法と解析Jacobianを追加 | 既存コードとの互換性維持、学習コスト低 | 単一ファイルが肥大化の可能性 | **選択**: モジュール分割で対応 |
| SystemOfEquations再利用 | pricer_core の MultidimensionalNewtonSolver を使用 | 汎用ソルバーの再利用、テスト済み実装 | カスタム収束制御が難しい | **選択**: 適切な抽象化レイヤーで対応 |
| 独自ソルバー実装 | globalsolver 内にカスタムNewton-Raphson実装 | 完全な制御、最適化の自由度 | コード重複、保守負担 | **却下**: 既存インフラで十分 |
| Enzyme AAD直接統合 | Jacobian計算にEnzyme自動微分を使用 | 高精度、高速勾配計算 | feature gate複雑化、nightly依存 | **延期**: Phase 2で検討 |

## Design Decisions

### Decision: パラメータ化方式 x = log(DF)

- **Context**: グローバルソルバーのパラメータ空間設計
- **Alternatives Considered**:
  1. x = DF（Discount Factor直接） — シンプルだが DF > 0 制約に追加処理が必要
  2. x = log(DF)（対数変換） — 無制約最適化、DF は常に正
  3. x = Zero Rate — 金利表現として自然だが、DF との変換コスト
- **Selected Approach**: x = log(DF)
- **Rationale**:
  - 無制約最適化が可能（x は任意の実数）
  - exp(x) により DF > 0 が自動的に保証
  - Jacobian計算時に連鎖律が自然に適用
  - 既存の GlobalBootstrapper が同じ方式を採用
- **Trade-offs**:
  - log/exp 変換のオーバーヘッド（微小）
  - Zero Rate 表現との変換が必要な場合に追加計算
- **Follow-up**: パフォーマンス検証（30ピラー以上のカーブ）

### Decision: テレスコープ法の商品別モジュール化

- **Context**: OIS/SOFR商品のテレスコープ計算とJacobian行要素の設計
- **Alternatives Considered**:
  1. CalibrationInstrument トレイトに `jacobian_row()` メソッド追加
  2. 別トレイト `TelescopingEvaluator` を定義
  3. 関数ベースの評価器（トレイトなし）
- **Selected Approach**: `TelescopingEvaluator` トレイトを定義し、商品タイプごとに実装
- **Rationale**:
  - 既存の CalibrationInstrument を変更せず後方互換性を維持
  - テレスコープ計算は特定の商品タイプにのみ適用
  - 将来の商品追加が容易
- **Trade-offs**:
  - 新規トレイト追加による API 複雑化
  - CalibrationInstrument との二重実装の可能性
- **Follow-up**: Single Curve / Dual Curve フレームワークの切り替え検討

### Decision: Jacobian計算方式の選択可能化

- **Context**: 要件3「Jacobian計算方法の選択」への対応
- **Alternatives Considered**:
  1. 数値微分のみ（現状）— シンプルだが n+1 回の関数評価
  2. 解析微分のみ — 高速だが実装複雑
  3. 選択可能（Analytical, FiniteDifference, AAD）— 柔軟だが設定複雑
- **Selected Approach**: 選択可能方式（enum JacobianMethod）
- **Rationale**:
  - 異なるユースケースに対応（検証時は数値、本番は解析）
  - AAD統合への将来拡張が容易
  - 要件10.3を満たす
- **Trade-offs**:
  - 設定オプション増加
  - 解析Jacobianの正確性検証が必要
- **Follow-up**: 数値vs解析の一致検証テスト

### Decision: 時間グリッドとキャッシュフロー行列の分離

- **Context**: 要件8「時間グリッドと行列構築」への対応
- **Alternatives Considered**:
  1. 商品ごとに個別計算 — シンプルだが非効率
  2. 共有時間グリッド + スパース行列 — 効率的だが実装複雑
  3. 共有時間グリッド + 商品別インデックスマップ — バランス型
- **Selected Approach**: `GlobalTimeGrid` + 商品別インデックスマップ
- **Rationale**:
  - 全商品で統一された時間軸を使用
  - 各商品は自身のキャッシュフロー日付をグリッドインデックスに変換
  - キャッシュフロー行列は反復ごとの再計算を回避
- **Trade-offs**:
  - グリッド構築のオーバーヘッド
  - 大規模ポートフォリオでのメモリ使用量
- **Follow-up**: スパース行列形式の検討（ndarray-sparse）

## Risks & Mitigations

- **Risk 1: 特異Jacobian** — 商品が線形従属の場合、Jacobianが特異になる可能性
  - Mitigation: 条件数チェック、`CalibrationError::SingularJacobian` エラー返却
- **Risk 2: 収束失敗** — 悪い初期値や ill-conditioned 問題での発散
  - Mitigation: ダンピング係数（Levenberg-Marquardt的アプローチ）、初期値推定ヒューリスティック
- **Risk 3: パフォーマンス** — 大規模カーブ（30+ピラー）での計算時間
  - Mitigation: 解析Jacobianの優先使用、スパース行列、BLAS最適化
- **Risk 4: テレスコープ法の適用範囲** — Dual Curve フレームワークでの制限
  - Mitigation: Single/Dual Curve フラグによる切り替え、完全テレスコープと部分テレスコープのオプション

## References

- [Newton-Raphson for Curve Calibration](https://quantlib.org/slides/qlws10/hagan.pdf) — QuantLib カーブ構築の数学的基盤
- [Implicit Function Theorem in AAD](https://arxiv.org/abs/1705.03663) — 最適化問題の微分とAAD
- [nalgebra Documentation](https://docs.rs/nalgebra/latest/nalgebra/) — Rust線形代数ライブラリ
- Existing codebase patterns:
  - `pricer_core::math::solvers::multidim_newton` — SystemOfEquations トレイト定義
  - `pricer_risk::greeks::ad::implicit_solver` — ImplicitSolver 実装
  - `pricer_models::builder::globalsolver` — GlobalBootstrapper 基本実装
