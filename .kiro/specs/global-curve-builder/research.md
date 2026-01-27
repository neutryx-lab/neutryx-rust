# Research & Design Decisions: Global Curve Builder

## Summary
- **Feature**: `global-curve-builder`
- **Discovery Scope**: Complex Integration（多次元ソルバー + AAD統合）
- **Key Findings**:
  1. nalgebra線形代数ラッパーが既存し、`RealField + Copy`バウンドでAD互換
  2. Levenberg-Marquardtソルバーパターンを多次元Newton-Raphsonに適用可能
  3. Shadow Objectパターンが既存し、Jacobian逆行列によるAAD統合に拡張可能

## Research Log

### 既存ソルバーパターン分析
- **Context**: 多次元Newton-Raphsonの設計パターンを確立するため
- **Sources Consulted**:
  - [levenberg_marquardt.rs](crates/pricer_core/src/math/solvers/levenberg_marquardt.rs)
  - [backtracking_newton.rs](crates/pricer_core/src/math/solvers/backtracking_newton.rs)
- **Findings**:
  - `LMConfig` / `LMResult` パターン：Config構造体で収束設定、Result構造体で結果を返す
  - 数値Jacobian計算: `compute_jacobian()` 関数で有限差分
  - 収束判定: 残差ノルムおよびパラメータ変化量の二重条件
  - f64固定（Float genericsではない）
- **Implications**:
  - 新しいソルバーはFloat generics (`T: RealField`) を採用
  - Config/Result パターンを踏襲
  - 数値Jacobianはデフォルト提供、解析的Jacobianはオプション

### nalgebra線形代数ラッパー分析
- **Context**: 行列演算のインターフェース設計
- **Sources Consulted**:
  - [wrappers.rs](crates/pricer_core/src/math/linalg/wrappers.rs)
  - [error.rs](crates/pricer_core/src/math/linalg/error.rs)
- **Findings**:
  - `DMatrix<T>` / `DVector<T>` 使用（動的サイズ）
  - `T: RealField + Copy` バウンド（AD互換）
  - `lu_solve()`, `inverse()`, `cholesky_solve()` 等が既存
  - `LinearAlgebraError` 列挙型: `SingularMatrix`, `NotPositiveDefinite`, `DimensionMismatch`
- **Implications**:
  - 新しいソルバーは既存の `lu_solve()` / `inverse()` を利用
  - 新しいエラー型は `LinearAlgebraError` を再利用または `SolverError` を拡張

### Enzyme AAD統合パターン
- **Context**: 陰関数定理によるAAD統合設計
- **Sources Consulted**:
  - [shadow.rs](crates/pricer_risk/src/enzyme/shadow.rs)
  - [binder.rs](crates/pricer_risk/src/enzyme/binder.rs)
  - [reverse.rs](crates/pricer_risk/src/enzyme/reverse.rs)
- **Findings**:
  - `Shadow` トレイト: `zero_out()`, `create_shadow()` メソッド
  - Shadow buffer によるadjoint蓄積
  - `MarketRiskCalculator`, `RiskResult<M>` パターン
  - Finite difference fallback 対応
- **Implications**:
  - Jacobian逆行列を `SolverResult` に含め、AADで利用
  - 陰関数定理: `∂L/∂m = J⁻ᵀ · ∂L/∂x*`
  - `ImplicitSolver` 層でEnzyme統合またはfinite differenceフォールバック

### BootstrapInstrument分析
- **Context**: CalibrationInstrumentトレイトの基盤設計
- **Sources Consulted**:
  - [instrument.rs](crates/pricer_models/src/market/calibration/bootstrapping/instrument.rs)
  - [engine.rs](crates/pricer_models/src/market/calibration/bootstrapping/engine.rs)
- **Findings**:
  - `BootstrapInstrument<T>` enum: OIS, IRS, FRA, Future variants
  - `residual(&self, curve) -> T` メソッド
  - `residual_derivative(&self, curve) -> T` メソッド（スカラー微分）
  - `SequentialBootstrapper<T>`: 1商品ずつ逐次的に解く
- **Implications**:
  - `CalibrationInstrument` トレイトでベクトル残差と行寄与に拡張
  - 既存のvariant構造を維持しつつトレイト実装

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Option A: 既存拡張 | solvers/に追加、BootstrapInstrumentをトレイト化 | 既存パターン踏襲、学習コスト低、ファイル数最小 | 既存コード変更必要 | **推奨** |
| Option B: 新規作成 | systems/, global/を新規作成 | クリーンな責務分離 | ファイル数増加、重複リスク | 後方互換性不要なら過剰 |

## Design Decisions

### Decision: Float型バウンド
- **Context**: AD互換性のためFloat genericsが必要
- **Alternatives Considered**:
  1. `num_traits::Float` — より広い互換性
  2. `nalgebra::RealField` — nalgebra統合が簡潔
- **Selected Approach**: `T: RealField + Copy`
- **Rationale**: 既存linalg wrappersと同じバウンド、nalgebra行列演算との自然な統合
- **Trade-offs**: num_traitsのみの型は使用不可、しかし実用上問題なし
- **Follow-up**: Enzymeのshadow typeとの互換性テストが必要

### Decision: Jacobian格納形式
- **Context**: AADで `J⁻ᵀ` が必要
- **Alternatives Considered**:
  1. LU分解のみ保持 — メモリ効率良
  2. 明示的逆行列 — AAD計算が簡潔
  3. 両方 — 柔軟だが複雑
- **Selected Approach**: `SolverResult` に `jacobian_inverse: Option<DMatrix<T>>` を含む
- **Rationale**: AADの `J⁻ᵀ · v` 計算が単純な行列乗算に帰着
- **Trade-offs**: O(n²) 追加メモリ使用、ただしn≤30程度なら許容範囲
- **Follow-up**: 大規模問題（n>100）向けにLU分解オプション追加を検討

### Decision: 数値Jacobianのデフォルト提供
- **Context**: 開発速度と精度のトレードオフ
- **Alternatives Considered**:
  1. 解析的Jacobian必須
  2. 数値Jacobianデフォルト、解析的はオプション
- **Selected Approach**: 数値Jacobianをデフォルト実装、解析的Jacobianはトレイトメソッドでオーバーライド可能
- **Rationale**: 開発速度優先、後で解析的Jacobian追加可能
- **Trade-offs**: 数値Jacobianは O(n) 追加評価コスト
- **Follow-up**: 性能クリティカルな場合に解析的Jacobian実装

### Decision: SequentialBootstrapperの置換
- **Context**: 後方互換性不要
- **Alternatives Considered**:
  1. 並行配置（feature flag切替）
  2. 直接置換
- **Selected Approach**: `GlobalBootstrapper` で直接置換、`SequentialBootstrapper` は削除
- **Rationale**: 後方互換性不要、コードベース簡潔化
- **Trade-offs**: 既存テスト移行が必要
- **Follow-up**: 既存テストを `GlobalBootstrapper` 用に更新

## Risks & Mitigations
- **Enzyme custom rule APIの不確実性** — Finite difference fallback を先行実装、Enzyme統合は段階的に追加
- **nalgebra RealFieldとEnzyme shadow typeの互換性** — 統合テストを早期実施、問題発生時はf64固定パスを用意
- **パフォーマンス目標未達** — ベンチマークを継続的に実行、ボトルネック特定後に最適化

## References
- [nalgebra Documentation](https://docs.rs/nalgebra/) — RealField, DMatrix API
- [Enzyme AD](https://enzyme.mit.edu/) — Custom gradient rules (研究中)
- Griewank, A. & Walther, A. (2008). Evaluating Derivatives: Principles and Techniques of Algorithmic Differentiation — 陰関数定理のAAD応用
