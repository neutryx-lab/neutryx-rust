# Gap Analysis: curve-global-solver

## 概要

本ドキュメントは、`curve-global-solver` 機能の要件と既存コードベースの差分を分析し、設計フェーズへの入力情報を提供する。

---

## 1. 既存アセットの発見

### 1.1 Multi-dimensional Newton-Raphson Solver

**ファイル**: [multidim_newton.rs](../../../crates/pricer_core/src/math/solvers/multidim_newton.rs)

| コンポーネント | 状態 | 詳細 |
|---------------|------|------|
| `SystemOfEquations<T>` trait | ✅ 実装済み | `evaluate()`, `jacobian()`, `jacobian_numerical()` |
| `MultidimNewtonConfig<T>` | ✅ 実装済み | tolerance, param_tolerance, max_iterations |
| `MultidimensionalNewtonSolver<T>` | ✅ 実装済み | Newton反復、収束判定 |
| `MultidimSolverResult<T>` | ✅ 実装済み | `jacobian_inverse: Option<DMatrix<T>>` |
| LU分解・逆行列 | ✅ 実装済み | `pricer_core::math::linalg` wrappers |

### 1.2 Global Bootstrapper

**ファイル**: [globalsolver.rs](../../../crates/pricer_models/src/builder/globalsolver.rs)

| コンポーネント | 状態 | 詳細 |
|---------------|------|------|
| `GlobalBootstrapper<T>` | ✅ 実装済み | Newton反復ベースのキャリブレーション |
| `GlobalBootstrapConfig<T>` | ✅ 実装済み | Builder pattern, tolerance設定 |
| `GlobalBootstrapResult<T>` | ✅ 実装済み | `jacobian_inverse` 格納 |
| 数値Jacobian | ✅ 実装済み | `compute_jacobian()` via finite difference |
| log(DF) パラメトリゼーション | ✅ 実装済み | DF > 0 を保証 |

### 1.3 Calibration Instrument

**ファイル**: [instrument.rs](../../../crates/pricer_models/src/builder/instrument.rs)

| コンポーネント | 状態 | 詳細 |
|---------------|------|------|
| `CalibrationInstrument<T>` trait | ✅ 実装済み | `market_rate()`, `theoretical_rate()`, `pricing_error()` |
| `MarketInstrument<T>` impl | ✅ 実装済み | OIS, IRS, FRA, Future |
| OIS par rate計算 | ✅ 実装済み | `compute_ois_par_rate()` |
| IRS par rate計算 | ✅ 実装済み | `compute_irs_par_rate()` |
| FRA rate計算 | ✅ 実装済み | `compute_fra_rate()` |

### 1.4 Market Data Structures

**ファイル**: [market.rs](../../../crates/pricer_models/src/market.rs)

| コンポーネント | 状態 | 詳細 |
|---------------|------|------|
| `YieldCurve<T>` trait | ✅ 実装済み | `discount_factor()`, `zero_rate()`, `forward_rate()` |
| `BootstrappedCurve<T>` | ✅ 実装済み | Log-linear補間、外挿対応 |
| `MarketInstrument<T>` enum | ✅ 実装済み | OIS, IRS, FRA, Future variants |
| `Frequency` enum | ✅ 実装済み | Daily〜Annual |

### 1.5 Linear Algebra

**ファイル**: [linalg/](../../../crates/pricer_core/src/math/linalg/)

| コンポーネント | 状態 | 詳細 |
|---------------|------|------|
| `DMatrix<T>`, `DVector<T>` | ✅ 実装済み | nalgebra re-export |
| `lu_solve()` | ✅ 実装済み | LU分解による線形システム解法 |
| `inverse()` | ✅ 実装済み | 逆行列計算 |
| `cholesky_solve()` | ✅ 実装済み | Cholesky分解 |

---

## 2. 要件とアセットのマッピング

### Requirement 1: 多次元Newton-Raphsonソルバー

| 受け入れ基準 | カバレッジ | アセット/ギャップ |
|-------------|-----------|------------------|
| AC1.1: 収束解を返す | ✅ 完全 | `MultidimensionalNewtonSolver::solve()` |
| AC1.2: Jacobian逆行列を結果に含める | ✅ 完全 | `MultidimSolverResult::jacobian_inverse` |
| AC1.3: ノルム収束判定 | ✅ 完全 | `residual_norm < tolerance` |
| AC1.4: ConvergenceFailure エラー | ⚠️ 部分的 | `MaxIterationsExceeded` のみ、専用型なし |
| AC1.5: ndarray/BLAS最適化 | ⚠️ 部分的 | nalgebra使用、ndarrayではない |
| AC1.6: SystemOfEquations trait | ✅ 完全 | `pricer_core::math::solvers::SystemOfEquations` |

**ギャップ**:
- `CalibrationError::ConvergenceFailure` の専用エラー型が必要
- ndarray vs nalgebra の選択は設計判断

---

### Requirement 2: カーブキャリブレーション問題定義

| 受け入れ基準 | カバレッジ | アセット/ギャップ |
|-------------|-----------|------------------|
| AC2.1: パラメータ x をピラー値として定義 | ✅ 完全 | log(DF)として実装済み |
| AC2.2: ターゲット m を市場レートとして受け入れ | ✅ 完全 | `CalibrationInstrument::market_rate()` |
| AC2.3: F(x) を理論価格計算として実装 | ✅ 完全 | `compute_residuals()` |
| AC2.4: SystemOfEquations trait実装 | ❌ 未実装 | GlobalBootstrapper は直接実装、trait未使用 |
| AC2.5: NoInstruments エラー | ⚠️ 部分的 | `NumericalInstability` として返却 |
| AC2.6: 商品数とパラメータ数の検証 | ❌ 未実装 | 暗黙的に一致を仮定 |

**ギャップ**:
- `GlobalBootstrapper` を `SystemOfEquations<T>` 経由で抽象化
- `CalibrationError::NoInstruments` 専用エラー
- 商品数 vs ピラー数の明示的検証

---

### Requirement 3: Jacobian行列の構築

| 受け入れ基準 | カバレッジ | アセット/ギャップ |
|-------------|-----------|------------------|
| AC3.1: 感応度 ∂F_i/∂x_j 計算 | ✅ 完全 | `compute_jacobian()` |
| AC3.2: 解析的/数値微分の選択 | ⚠️ 部分的 | 数値微分のみ実装 |
| AC3.3: Enzyme AD による自動微分 | ❌ 未実装 | 将来のEnzyme統合待ち |
| AC3.4: 補間行列 W によるJacobian縮約 | ❌ 未実装 | 行列構造なし |
| AC3.5: キャッシュフロー行列 A のキャッシュ | ❌ 未実装 | 行列構造なし |

**ギャップ**:
- 解析的Jacobian計算の追加
- 行列 A, W の導入（設計判断）
- Enzyme AD フック（feature-gated）

---

### Requirement 4: AAD統合（陰関数定理）

| 受け入れ基準 | カバレッジ | アセット/ギャップ |
|-------------|-----------|------------------|
| AC4.1: 陰関数定理 ∂x*/∂m = J⁻¹ | ✅ 準備済み | `jacobian_inverse` 格納済み |
| AC4.2: jacobian_inv 再利用 | ✅ 完全 | `GlobalBootstrapResult::jacobian_inverse` |
| AC4.3: DV01, Key Rate Duration計算 | ❌ 未実装 | リスク計算モジュール未実装 |
| AC4.4: Enzyme AD カスタム微分ルール | ❌ 未実装 | Shadow function未定義 |

**ギャップ**:
- カーブ感応度（DV01, KRD）計算関数
- Enzyme AD との統合インターフェース

---

### Requirement 5: OIS/SOFR商品のテレスコープ法

| 受け入れ基準 | カバレッジ | アセット/ギャップ |
|-------------|-----------|------------------|
| AC5.1: 変動脚を DF(t_start)/DF(t_end) - 1 | ❌ 未実装 | 支払日ごとにループ |
| AC5.2: 日次ループ回避 | ❌ 未実装 | 現在は支払期間ごとに計算 |
| AC5.3: Payment Delay考慮 | ❌ 未実装 | 満期日=支払日と仮定 |
| AC5.4: スパースJacobian行 | ❌ 未実装 | Dense Jacobian |
| AC5.5: Single Curve完全簡約化 | ❌ 未実装 | テレスコープ未実装 |

**ギャップ**:
- `TelescopingOisEvaluator` の新規実装
- スパースJacobian構造の導入

---

### Requirement 6: Deposit/Futures商品サポート

| 受け入れ基準 | カバレッジ | アセット/ギャップ |
|-------------|-----------|------------------|
| AC6.1: Deposit インプライドレート | ⚠️ 部分的 | FRA計算で代用可能 |
| AC6.2: Jacobian行に1要素 | ❌ 未実装 | Dense構造 |
| AC6.3: Forward Rate計算 | ✅ 完全 | `compute_fra_rate()` |
| AC6.4: Jacobian行に2要素 | ❌ 未実装 | Dense構造 |
| AC6.5: Convexity Adjustment | ✅ 完全 | `MarketInstrument::Future::convexity_adjustment` |

**ギャップ**:
- `Deposit` variant を `MarketInstrument` に追加
- スパースJacobian構造

---

### Requirement 7: スワップ商品サポート

| 受け入れ基準 | カバレッジ | アセット/ギャップ |
|-------------|-----------|------------------|
| AC7.1: パーレート計算 | ✅ 完全 | `compute_irs_par_rate()` |
| AC7.2: キャッシュフロー行列構築 | ❌ 未実装 | 行列構造なし |
| AC7.3: GlobalTimeGrid マッピング | ❌ 未実装 | 時間グリッド未実装 |
| AC7.4: OISテレスコープ適用 | ❌ 未実装 | テレスコープ未実装 |

**ギャップ**:
- `GlobalTimeGrid` 構造
- キャッシュフロー行列 A

---

### Requirement 8: 時間グリッドと行列構築

| 受け入れ基準 | カバレッジ | アセット/ギャップ |
|-------------|-----------|------------------|
| AC8.1: キャッシュフロー日付収集 | ❌ 未実装 | 商品ごとに独立計算 |
| AC8.2: GlobalTimeGrid 生成 | ❌ 未実装 | 新規実装必要 |
| AC8.3: キャッシュフロー行列 A 構築 | ❌ 未実装 | 新規実装必要 |
| AC8.4: 補間行列 W 構築 | ❌ 未実装 | 新規実装必要 |
| AC8.5: A, W のキャッシュ | ❌ 未実装 | 新規実装必要 |

**ギャップ**:
- 完全な新規実装が必要
- 設計判断: 行列ベースアプローチの採用可否

---

### Requirement 9: エラーハンドリング

| 受け入れ基準 | カバレッジ | アセット/ギャップ |
|-------------|-----------|------------------|
| AC9.1: SingularJacobian エラー | ⚠️ 部分的 | `LinearAlgebraError` として伝播 |
| AC9.2: Divergence エラー | ❌ 未実装 | 発散検出なし |
| AC9.3: 反復ログ出力 | ❌ 未実装 | ログなし |
| AC9.4: 商品特定エラー | ❌ 未実装 | 汎用エラーのみ |
| AC9.5: 反復回数と最終残差を結果に含める | ✅ 完全 | `iterations`, `residual_norm` |

**ギャップ**:
- `CalibrationError` 専用エラー型の拡充
- 発散検出ロジック
- デバッグログ（tracing crate）

---

### Requirement 10: 設定とカスタマイズ

| 受け入れ基準 | カバレッジ | アセット/ギャップ |
|-------------|-----------|------------------|
| AC10.1: tolerance デフォルト 1e-10 | ✅ 完全 | `GlobalBootstrapConfig::default()` |
| AC10.2: max_iterations デフォルト 100 | ✅ 完全 | 実装済み |
| AC10.3: Jacobian計算方法の選択 | ❌ 未実装 | 数値微分のみ |
| AC10.4: 線形代数バックエンド選択 | ❌ 未実装 | nalgebraのみ |
| AC10.5: Builder パターン | ✅ 完全 | `with_interpolation()`, `with_jacobian_inverse()` |

**ギャップ**:
- `JacobianMethod` enum（Analytical, FiniteDifference, AAD）
- バックエンド選択（feature flags）

---

## 3. 実装アプローチオプション

### Option A: 既存コード拡張（推奨）

**概要**: 既存の `GlobalBootstrapper` と `CalibrationInstrument` を拡張し、新機能を追加。

**メリット**:
- 既存テストとの互換性維持
- 段階的な移行が可能
- リスクが低い

**デメリット**:
- 行列ベースアプローチへの移行が難しい
- テレスコープ法の効率が制限される

**工数**: M（中）

---

### Option B: 新規行列ベース実装

**概要**: `GlobalTimeGrid`, 行列 A/W を導入し、行列演算ベースの新アーキテクチャを構築。

**メリット**:
- 要件に完全準拠
- Enzyme AD との親和性が高い
- スパース構造の最適化が可能

**デメリット**:
- 実装工数が大きい
- 既存コードとの統合が複雑

**工数**: L〜XL（大〜特大）

---

### Option C: ハイブリッドアプローチ（推奨）

**概要**: 既存の `GlobalBootstrapper` をベースに、テレスコープ法とスパースJacobianを段階的に導入。行列 A/W は Phase 2 として後日実装。

**Phase 1 スコープ**:
1. `CalibrationError` 専用エラー型
2. `TelescopingOisEvaluator`
3. `Deposit` variant 追加
4. `JacobianMethod` enum
5. 発散検出とデバッグログ

**Phase 2 スコープ（将来）**:
1. `GlobalTimeGrid` と行列 A/W
2. Enzyme AD 統合
3. ndarray バックエンド

**メリット**:
- 段階的デリバリー
- 既存機能との共存
- リスク分散

**デメリット**:
- 完全な行列最適化は Phase 2 まで待機

**工数**: M（Phase 1）+ L（Phase 2）

---

## 4. ギャップサマリー

| 要件 | カバレッジ | 工数 | リスク | 優先度 |
|------|-----------|------|-------|-------|
| R1: Newton-Raphsonソルバー | 90% | S | Low | - |
| R2: キャリブレーション問題定義 | 70% | M | Low | High |
| R3: Jacobian構築 | 40% | M | Medium | High |
| R4: AAD統合 | 50% | M | Medium | Medium |
| R5: テレスコープ法 | 0% | L | Medium | High |
| R6: Deposit/Futures | 60% | S | Low | Medium |
| R7: スワップサポート | 60% | M | Low | Medium |
| R8: 時間グリッド/行列 | 0% | XL | High | Low (Phase 2) |
| R9: エラーハンドリング | 40% | S | Low | High |
| R10: 設定/カスタマイズ | 70% | S | Low | Medium |

**凡例**: S=Small, M=Medium, L=Large, XL=Extra Large

---

## 5. 設計フェーズへの入力

### 5.1 リサーチ項目

1. **Enzyme AD 統合戦略**: `enzyme` feature flag のスコープと Shadow function の定義方法
2. **スパース行列ライブラリ**: `sprs` vs `nalgebra::sparse` の比較
3. **テレスコープ法の精度**: Payment Delay による誤差の許容範囲

### 5.2 設計判断事項

1. **行列 A/W の導入タイミング**: Phase 1 で導入するか、Phase 2 に延期するか
2. **CalibrationProblem trait**: `SystemOfEquations` を拡張するか、別 trait を定義するか
3. **エラー型の統一**: `CalibrationError` を `builder` モジュール内に統一するか

### 5.3 推奨アプローチ

**Option C（ハイブリッド）** を推奨。Phase 1 でテレスコープ法とエラーハンドリングを実装し、Phase 2 で行列ベースアーキテクチャに移行する。

---

## 6. Demo WebApp統合要件

### 6.1 現状の構造

#### Backend: [demo/gui/src/web/handlers/curves.rs](../../../demo/gui/src/web/handlers/curves.rs)

| コンポーネント | 状態 | 詳細 |
|---------------|------|------|
| `BootstrapMethod` enum | ✅ 定義済み | `Sequential`, `Global` variants |
| `Global` メソッド | ⚠️ 無効化 | `is_enabled()` returns `false`, "Coming Soon" |
| `build_curve` handler | ⚠️ Stub | 単純な `df = 1/(1+r·t)` 計算、実際のブートストラップなし |
| `CurveBuildRequest` | ✅ 準備済み | `bootstrap_method` フィールドあり |
| `CurveBuildResponse` | ⚠️ 不完全 | `iterations`, `residual_norm` フィールドなし |

#### Frontend: [demo/gui/static/js/curve-builder.js](../../../demo/gui/static/js/curve-builder.js)

| コンポーネント | 状態 | 詳細 |
|---------------|------|------|
| Bootstrap method selector | ✅ 実装済み | Global は disabled として表示 |
| API request | ✅ 実装済み | `bootstrapMethod` をリクエストに含む |
| Build result display | ⚠️ 不完全 | iterations, residual 表示なし |

#### Feature Gates: [demo/gui/Cargo.toml](../../../demo/gui/Cargo.toml)

```toml
# 現状
calibration = []  # 空の feature、global-bootstrap を含まない

# 必要な変更
calibration = ["pricer_models/global-bootstrap"]
```

### 6.2 統合ギャップ

| 要件 | 状態 | 必要な作業 |
|------|------|-----------|
| Feature gate接続 | ❌ 未実装 | `demo_gui` の `calibration` feature に `global-bootstrap` 追加 |
| `build_curve` handler | ❌ 未実装 | `GlobalBootstrapper` を使用した実際のカーブ構築 |
| Jacobian inverse返却 | ❌ 未実装 | Response に `jacobian_inverse` オプション追加 |
| 収束情報表示 | ❌ 未実装 | Response に `iterations`, `residual_norm` 追加 |
| フロントエンド更新 | ❌ 未実装 | Global 有効化、収束情報表示 |

### 6.3 必要なAPI変更

#### CurveBuildResponse 拡張

```rust
pub struct CurveBuildResponse {
    // 既存フィールド
    pub curve_id: String,
    pub status: BuildStatus,
    pub index: String,
    pub interpolation_method: String,
    pub parameters: Vec<CurveParameter>,
    pub pillars: Vec<f64>,
    pub discount_factors: Vec<f64>,
    pub zero_rates: Vec<f64>,
    pub build_time_ms: f64,
    pub instrument_count: usize,

    // 新規フィールド（Global Solver用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterations: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub residual_norm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap_method_used: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub converged: Option<bool>,
}
```

#### build_curve handler 更新

```rust
// Sequential の場合: 既存ロジック維持
// Global の場合:
#[cfg(feature = "calibration")]
{
    use pricer_models::builder::{GlobalBootstrapper, GlobalBootstrapConfig};

    let instruments = convert_to_market_instruments(&request.instruments)?;
    let config = GlobalBootstrapConfig::new(request.tolerance, request.max_iterations);
    let bootstrapper = GlobalBootstrapper::new(config);
    let result = bootstrapper.calibrate(&instruments)?;

    // result.curve, result.iterations, result.residual_norm を使用
}
```

### 6.4 フロントエンド変更

#### curve-builder.js 更新

```javascript
// BootstrapMethod.Global を有効化
BootstrapMethodInfo {
    id: "global",
    name: "Global",
    description: "Solves all instruments simultaneously using global optimization",
    enabled: true,  // false → true
}

// Build summary に収束情報追加
if (result.iterations !== undefined) {
    summaryHtml += `
        <div class="summary-item">
            <span class="summary-label">Iterations</span>
            <span class="summary-value">${result.iterations}</span>
        </div>
        <div class="summary-item">
            <span class="summary-label">Residual</span>
            <span class="summary-value">${result.residualNorm?.toExponential(2) || '-'}</span>
        </div>
    `;
}
```

### 6.5 統合工数見積もり

| タスク | 工数 | 依存関係 |
|-------|------|---------|
| Feature gate修正 | S | なし |
| `build_curve` handler更新 | M | Global Solver 実装完了後 |
| Response型拡張 | S | なし |
| フロントエンド更新 | S | Backend API完了後 |

**合計**: M（中規模）、Global Solver Phase 1 完了後に実装可能

---

## 7. 結論

既存コードベースには Global Solver の基盤となる多くのコンポーネントが実装済みである。主なギャップは：

1. **テレスコープ法** - OIS/SOFR商品の効率化に必須
2. **行列構造（A, W）** - 将来のAAD最適化に重要
3. **エラーハンドリング** - プロダクション品質に必須
4. **Demo WebApp統合** - `build_curve` handler と frontend の更新

段階的なハイブリッドアプローチにより、既存機能を維持しながら要件を満たす実装が可能である。

### 実装順序（推奨）

```
Phase 1: Core Solver
├── CalibrationError 専用型
├── GlobalBootstrapper 拡張（テレスコープ法）
├── Deposit variant 追加
└── エラーハンドリング強化

Phase 1.5: Demo WebApp Integration
├── Feature gate 接続
├── build_curve handler 更新
├── Response 型拡張
└── Frontend 更新

Phase 2: Matrix-based Architecture (将来)
├── GlobalTimeGrid
├── 行列 A, W
└── Enzyme AD 統合
```
