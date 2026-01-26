# Design Document: external-numerics-migration

## Overview

**Purpose**: 本機能は、pricer_core の数値計算モジュール（optimisers, solvers）を外部クレートベースの実装に移行し、約 1,500 行の自前アルゴリズムコードを薄いラッパーに置き換える。

**Users**: クオンツ開発者、リスク計算担当者、ライブラリメンテナーが、保守コスト削減と数値的堅牢性向上の恩恵を受ける。

**Impact**: 既存の公開 API を維持しつつ、内部実装を argmin、levenberg-marquardt クレートに委譲。コードベースの大幅な軽量化と、コミュニティ主導の継続的改善へのアクセスを実現。

### Goals

- 自前実装の Nelder-Mead、L-BFGS、Brent、LM を外部クレートに移行
- 既存の公開 API 完全互換を維持
- AD 互換性（num-dual / Enzyme）を保持
- 約 1,500 行のアルゴリズムコードを ~300 行のラッパーコードに削減

### Non-Goals

- faer による線形代数バックエンド置換（Phase 2 で評価）
- 新規最適化アルゴリズムの追加
- 公開 API の破壊的変更

## Architecture

### Existing Architecture Analysis

**現行構造**:
```
pricer_core/src/math/
├── optimisers/
│   ├── mod.rs          # 公開 API
│   ├── nelder_mead.rs  # 自前実装 (~200行)
│   ├── lbfgs.rs        # 自前実装 (~310行)
│   ├── config.rs       # 設定型
│   ├── result.rs       # 結果型
│   └── error.rs        # エラー型
└── solvers/
    ├── mod.rs          # 公開 API
    ├── brent.rs        # 自前実装 (~230行)
    ├── newton_raphson.rs    # 自前実装 (~150行) + AD
    ├── bisection.rs         # 自前実装 (~200行)
    ├── backtracking_newton.rs  # 自前実装 (~250行)
    ├── levenberg_marquardt.rs  # 自前実装 (~350行)
    └── config.rs       # 設定型
```

**移行後構造**:
```
pricer_core/src/math/
├── optimisers/
│   ├── mod.rs          # 公開 API（変更なし）
│   ├── argmin_wrapper.rs   # argmin ラッパー (新規)
│   ├── config.rs       # 設定型（変更なし）
│   ├── result.rs       # 結果型（変更なし）
│   └── error.rs        # エラー型（拡張）
└── solvers/
    ├── mod.rs          # 公開 API（変更なし）
    ├── argmin_wrapper.rs   # argmin BrentRoot ラッパー (新規)
    ├── lm_wrapper.rs       # levenberg-marquardt ラッパー (新規)
    ├── newton_raphson.rs   # AD 対応のため保持
    ├── bisection.rs        # 簡易実装のため保持
    ├── backtracking_newton.rs  # roots 非対応のため保持
    └── config.rs       # 設定型（変更なし）
```

### Architecture Pattern & Boundary Map

```mermaid
graph TB
    subgraph PublicAPI[Public API Layer]
        NM[minimize_nelder_mead]
        LBFGS[minimize_lbfgs]
        Brent[BrentSolver]
        NR[NewtonRaphsonSolver]
        Bisect[BisectionSolver]
        LM[LevenbergMarquardtSolver]
    end

    subgraph Wrapper[Wrapper Layer]
        ArgminOpt[ArgminOptimiserWrapper]
        ArgminRoot[ArgminRootWrapper]
        LMWrap[LevenbergMarquardtWrapper]
    end

    subgraph External[External Crates]
        ArgminNM[argmin NelderMead]
        ArgminLBFGS[argmin LBFGS]
        ArgminBrent[argmin BrentRoot]
        LMCrate[levenberg-marquardt]
    end

    subgraph Retained[Retained Self-Impl]
        NRImpl[newton_raphson.rs AD]
        BisectImpl[bisection.rs]
        BacktrackImpl[backtracking_newton.rs]
    end

    NM --> ArgminOpt
    LBFGS --> ArgminOpt
    Brent --> ArgminRoot
    LM --> LMWrap

    ArgminOpt --> ArgminNM
    ArgminOpt --> ArgminLBFGS
    ArgminRoot --> ArgminBrent
    LMWrap --> LMCrate

    NR --> NRImpl
    Bisect --> BisectImpl
```

**Architecture Integration**:
- **Selected pattern**: Wrapper Approach（公開 API 維持、内部委譲）
- **Domain boundaries**: Wrapper 層が外部クレートの API 差異を吸収
- **Existing patterns preserved**: `OptimisationResult`, `SolverConfig`, エラー型
- **New components rationale**: 外部クレートとの型変換・エラー変換を担当
- **Steering compliance**: A-I-P-S レイヤー維持、Pricer 内部完結

### Technology Stack

| Layer | Choice / Version | Role in Feature | Notes |
|-------|------------------|-----------------|-------|
| Optimisation | argmin 0.10, argmin-math 0.4 | Nelder-Mead, L-BFGS, Brent | workspace 既存 |
| Least Squares | levenberg-marquardt 0.14 | LM solver | 新規追加 |
| Linear Algebra | nalgebra (既存) | LM の行列操作 | 変更なし |
| AD | num-dual (既存) | Newton-Raphson AD | 変更なし |

## Requirements Traceability

| Requirement | Summary | Components | Interfaces | Flows |
|-------------|---------|------------|------------|-------|
| 1.1-1.8 | argmin 最適化移行 | ArgminOptimiserWrapper | CostFunction, Gradient | ConfigToArgmin |
| 2.1-2.8 | roots 求根移行 | ArgminRootWrapper | BrentRoot | ConfigToArgmin |
| 3.1-3.5 | LM 移行 | LMWrapper | LeastSquaresProblem | ResidualToLM |
| 4.1-4.6 | faer 評価 | (Phase 2) | - | - |
| 5.1-5.5 | AD 互換性 | NewtonRaphsonSolver | find_root_ad | ADPreserved |
| 6.1-6.5 | テスト互換 | All | - | RegressionTests |
| 7.1-7.6 | 依存管理 | Cargo.toml | - | FeatureFlags |
| 8.1-8.4 | ドキュメント | mod.rs docs | - | - |

## Components and Interfaces

| Component | Domain/Layer | Intent | Req Coverage | Key Dependencies | Contracts |
|-----------|--------------|--------|--------------|------------------|-----------|
| ArgminOptimiserWrapper | math/optimisers | argmin への委譲 | 1.1-1.8 | argmin (P0) | Service |
| ArgminRootWrapper | math/solvers | argmin BrentRoot への委譲 | 2.1-2.7 | argmin (P0) | Service |
| LMWrapper | math/solvers | levenberg-marquardt への委譲 | 3.1-3.5 | levenberg-marquardt (P0), nalgebra (P0) | Service |
| ConfigConverter | math/optimisers | Config → argmin パラメータ変換 | 1.4-1.6 | - | - |
| ResultConverter | math/optimisers | argmin 結果 → OptimisationResult | 1.6 | - | - |

### math/optimisers

#### ArgminOptimiserWrapper

| Field | Detail |
|-------|--------|
| Intent | argmin クレートの NelderMead, LBFGS を公開 API に適合させる |
| Requirements | 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8 |

**Responsibilities & Constraints**
- 公開関数 `minimize_nelder_mead`, `minimize_lbfgs`, `minimize_lbfgs_numerical` の内部実装を argmin に委譲
- `NelderMeadConfig`, `LbfgsConfig` から argmin builder パラメータへの変換
- argmin エラーから `OptimisationError` への変換

**Dependencies**
- Outbound: argmin::solver::neldermead::NelderMead — 最適化実行 (P0)
- Outbound: argmin::solver::quasinewton::LBFGS — 最適化実行 (P0)
- Outbound: argmin::solver::linesearch::MoreThuente — L-BFGS line search (P0)
- External: argmin, argmin-math — 最適化フレームワーク (P0)

**Contracts**: Service [x]

##### Service Interface

```rust
/// CostFunction アダプター（目的関数ラッパー）
struct ObjectiveFn<F> {
    f: F,
}

impl<F> CostFunction for ObjectiveFn<F>
where
    F: Fn(&[f64]) -> f64,
{
    type Param = Vec<f64>;
    type Output = f64;

    fn cost(&self, p: &Self::Param) -> Result<Self::Output, argmin::core::Error>;
}

/// Gradient アダプター（勾配付き目的関数ラッパー）
struct GradientFn<F> {
    f: F,
}

impl<F> CostFunction for GradientFn<F>
where
    F: Fn(&[f64]) -> (f64, Vec<f64>),
{
    type Param = Vec<f64>;
    type Output = f64;

    fn cost(&self, p: &Self::Param) -> Result<Self::Output, argmin::core::Error>;
}

impl<F> Gradient for GradientFn<F>
where
    F: Fn(&[f64]) -> (f64, Vec<f64>),
{
    type Param = Vec<f64>;
    type Gradient = Vec<f64>;

    fn gradient(&self, p: &Self::Param) -> Result<Self::Gradient, argmin::core::Error>;
}

/// Config 変換
fn nelder_mead_config_to_argmin(config: &NelderMeadConfig, x0: &[f64])
    -> Result<NelderMead<Vec<f64>, f64>, OptimisationError>;

fn lbfgs_config_to_argmin(config: &LbfgsConfig)
    -> LBFGS<MoreThuente<Vec<f64>, Vec<f64>, f64>, Vec<f64>, Vec<f64>, f64>;

/// 結果変換
fn argmin_result_to_optimisation_result(
    result: &ArgminResult<IterState<Vec<f64>, (), (), (), (), f64>>,
    iterations: u64,
    func_evals: u64,
) -> OptimisationResult;
```

**Implementation Notes**
- Integration: `#[cfg(feature = "argmin")]` でゲート、デフォルト有効
- Validation: 空の初期点は `OptimisationError::InvalidInput` を返す
- Risks: argmin API 変更時の互換性維持

---

#### ConfigConverter

| Field | Detail |
|-------|--------|
| Intent | pricer_core Config 型を argmin パラメータに変換 |
| Requirements | 1.4, 1.5, 2.5 |

**Responsibilities & Constraints**
- `NelderMeadConfig` → argmin NelderMead builder 呼び出し
- `LbfgsConfig` → argmin LBFGS + MoreThuente 設定
- `SolverConfig` → argmin Executor max_iters 設定

**Contracts**: (Internal utility, no public contract)

##### Conversion Mappings

| pricer_core | argmin | Notes |
|-------------|--------|-------|
| `NelderMeadConfig.alpha` | `.with_alpha()` | 反射係数 |
| `NelderMeadConfig.gamma` | `.with_gamma()` | 拡大係数 |
| `NelderMeadConfig.rho` | `.with_rho()` | 収縮係数 |
| `NelderMeadConfig.sigma` | `.with_sigma()` | 縮小係数 |
| `NelderMeadConfig.base.abs_tol` | `.with_sd_tolerance()` | 収束許容誤差 |
| `NelderMeadConfig.base.max_iterations` | Executor `.max_iters()` | 最大反復回数 |
| `LbfgsConfig.m` | `LBFGS::new(_, m)` | メモリサイズ |
| `LbfgsConfig.c1`, `c2` | MoreThuente パラメータ | Wolfe 条件 |

---

### math/solvers

#### ArgminRootWrapper

| Field | Detail |
|-------|--------|
| Intent | argmin BrentRoot を BrentSolver 互換で提供 |
| Requirements | 2.1, 2.4, 2.5, 2.7, 2.8 |

**Responsibilities & Constraints**
- `BrentSolver::find_root` の内部実装を argmin BrentRoot に委譲
- ブラケット検証とエラー変換
- 既存の `SolverConfig` との互換維持

**Dependencies**
- Outbound: argmin::solver::brent::BrentRoot — 求根実行 (P0)
- External: argmin — 最適化フレームワーク (P0)

**Contracts**: Service [x]

##### Service Interface

```rust
/// BrentSolver の argmin 委譲実装
impl<T: Float> BrentSolver<T> {
    /// argmin BrentRoot を使用した求根（f64 専用）
    pub fn find_root_argmin<F>(&self, f: F, a: f64, b: f64) -> Result<f64, SolverError>
    where
        F: Fn(f64) -> f64;
}

/// CostFunction アダプター（求根用）
struct RootFn<F> {
    f: F,
}

impl<F> CostFunction for RootFn<F>
where
    F: Fn(f64) -> f64,
{
    type Param = f64;
    type Output = f64;

    fn cost(&self, x: &Self::Param) -> Result<Self::Output, argmin::core::Error>;
}

/// エラー変換
fn argmin_error_to_solver_error(err: argmin::core::Error) -> SolverError;
```

**Implementation Notes**
- Integration: 既存の `find_root` メソッドを内部で置換
- Validation: `f(a) * f(b) > 0` の場合 `SolverError::NoBracket`
- Risks: ジェネリック `T: Float` は f64 専用に制限される可能性

---

#### LMWrapper

| Field | Detail |
|-------|--------|
| Intent | levenberg-marquardt クレートを LevenbergMarquardtSolver 互換で提供 |
| Requirements | 3.1, 3.2, 3.3, 3.4, 3.5 |

**Responsibilities & Constraints**
- `LevenbergMarquardtSolver::solve` の内部実装を levenberg-marquardt に委譲
- クロージャベース残差関数を `LeastSquaresProblem` トレイトに適合
- `LMConfig` から levenberg-marquardt ハイパーパラメータへの変換
- `MinimizationReport` から `LMResult` への変換

**Dependencies**
- Outbound: levenberg_marquardt::LevenbergMarquardt — LM 実行 (P0)
- External: levenberg-marquardt, nalgebra — 非線形最小二乗 (P0)

**Contracts**: Service [x]

##### Service Interface

```rust
/// LeastSquaresProblem アダプター
struct ResidualProblem<F> {
    residuals_fn: F,
    params: DVector<f64>,
    n_residuals: usize,
}

impl<F> LeastSquaresProblem<f64> for ResidualProblem<F>
where
    F: Fn(&[f64]) -> Vec<f64>,
{
    type ParameterStorage = Owned<f64, Dyn, U1>;
    type ResidualStorage = Owned<f64, Dyn, U1>;
    type JacobianStorage = Owned<f64, Dyn, Dyn>;

    fn set_params(&mut self, params: &DVector<f64>);
    fn params(&self) -> DVector<f64>;
    fn residuals(&self) -> Option<DVector<f64>>;
    fn jacobian(&self) -> Option<DMatrix<f64>> { None } // 数値微分
}

/// LMConfig から levenberg-marquardt 設定への変換
fn lm_config_to_lm_crate(config: &LMConfig) -> LevenbergMarquardt<f64>;

/// MinimizationReport から LMResult への変換
fn lm_report_to_result(
    problem: &ResidualProblem<impl Fn(&[f64]) -> Vec<f64>>,
    report: &MinimizationReport<f64>,
) -> LMResult;
```

**Implementation Notes**
- Integration: 既存 `solve` メソッドを内部で置換
- Validation: パラメータ数と残差数の整合性チェック
- Risks: Jacobian 自動計算のパフォーマンス（数値微分）

---

## Error Handling

### Error Strategy

外部クレートのエラーを pricer_core のエラー型に変換し、ユーザーに一貫したエラーインターフェースを提供。

### Error Categories and Responses

**Optimisation Errors (OptimisationError)**:
- `InvalidInput` → argmin パラメータ検証失敗
- `ConvergenceFailed` → argmin TerminationReason 非成功
- `External(String)` → argmin::core::Error のラップ（新規追加）

**Solver Errors (SolverError)**:
- `NoBracket` → ブラケット検証失敗（変更なし）
- `MaxIterationsExceeded` → argmin/lm 反復上限到達
- `External(String)` → 外部クレートエラーのラップ（新規追加）

### Monitoring

- 外部クレートの verbose/callback は `OptimisationConfig.verbose` で制御
- エラー時は元のエラーメッセージを保持してラップ

## Testing Strategy

### Unit Tests

1. **Config 変換テスト**: `NelderMeadConfig` → argmin パラメータの正確な変換
2. **Result 変換テスト**: argmin 結果 → `OptimisationResult` フィールドマッピング
3. **エラー変換テスト**: argmin エラー → `OptimisationError` 変換
4. **LM Problem 構築テスト**: クロージャ → `LeastSquaresProblem` 適合

### Integration Tests

1. **Rosenbrock 関数**: Nelder-Mead と L-BFGS の収束確認
2. **√2 求根**: Brent solver の精度確認
3. **SABR キャリブレーション**: LM solver のエンドツーエンド確認
4. **既存テスト回帰**: 全既存テストのパス確認

### Performance Tests

1. **bench_nelder_mead_argmin_vs_self**: 自前実装と argmin の比較
2. **bench_lbfgs_argmin_vs_self**: 自前実装と argmin の比較
3. **bench_brent_argmin_vs_self**: 自前実装と argmin の比較
4. **bench_lm_crate_vs_self**: 自前実装と levenberg-marquardt の比較

## Migration Strategy

### Phase 1: Dependency Setup

1. `pricer_core/Cargo.toml` に `argmin`, `argmin-math`, `levenberg-marquardt` 追加
2. Feature flag `external-numerics` 追加（デフォルト有効）
3. `cargo tree --duplicates` で重複確認

### Phase 2: Wrapper Implementation

1. `argmin_wrapper.rs` 作成（NelderMead, LBFGS）
2. `lm_wrapper.rs` 作成
3. 既存実装ファイルを `_legacy.rs` にリネーム
4. 公開関数の内部実装をラッパーに切り替え

### Phase 3: Validation

1. 全既存テスト実行
2. 回帰ベンチマーク実行
3. 数値結果の差異検証

### Phase 4: Cleanup

1. `_legacy.rs` ファイル削除
2. ドキュメント更新
3. CHANGELOG 更新

### Rollback Triggers

- 既存テストの 5% 以上が失敗
- ベンチマークでスループット 20% 以上低下
- AD 機能の互換性問題発生

---

## Supporting References

### argmin Executor Pattern

```rust
use argmin::core::{Executor, State};
use argmin::solver::neldermead::NelderMead;

// 完全な実行パターン
let result = Executor::new(problem, solver)
    .configure(|state| {
        state
            .param(initial_params)
            .max_iters(config.base.max_iterations as u64)
    })
    .run()?;

// 結果の取得
let best_param = result.state().get_best_param().unwrap();
let best_cost = result.state().get_best_cost();
let iterations = result.state().get_iter();
let func_evals = result.state().get_func_counts();
let terminated = result.state().is_terminated();
```

### levenberg-marquardt Minimization Pattern

```rust
use levenberg_marquardt::{LevenbergMarquardt, MinimizationReport};
use nalgebra::{DVector, DMatrix};

let problem = ResidualProblem::new(residuals_fn, initial_params);
let (final_problem, report) = LevenbergMarquardt::new()
    .with_patience(config.max_iterations)
    .with_scale_diag(true)
    .minimize(problem);

if report.termination.was_successful() {
    let params = final_problem.params();
    let residual_ss = report.objective_function;
    // ...
}
```
