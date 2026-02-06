# Technical Design Document

## Overview

**Feature**: グローバルカーブソルバー（Global Curve Solver）

**Purpose**: カーブ構築を連立方程式として解くグローバルソルバーの実装。全期間同時推定により、すべての観測商品がすべてのカーブパラメータに依存する構造をJacobianとして明示的に扱い、陰関数定理を用いたAADの高速化を実現する。

**Scope**:
- 多次元Newton-Raphsonソルバーの活用（既存 `MultidimensionalNewtonSolver` の再利用）
- `SystemOfEquations` トレイトを実装したカーブキャリブレーション問題の定義
- テレスコープ法によるOIS/SOFR商品の効率的評価
- 時間グリッドとキャッシュフロー行列の構築
- AAD統合（`ImplicitSolver` との連携）

---

## Architecture Pattern & Boundary Map

### Architectural Overview

本設計は「Extension（既存システムの拡張）」パターンに基づき、既存の `pricer_core` ソルバーインフラストラクチャと `pricer_risk` AADインフラストラクチャを再利用する。

```
┌──────────────────────────────────────────────────────────────────────┐
│                      pricer_models::builder                          │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │              curve-global-solver モジュール群                    │  │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────┐  │  │
│  │  │ CurveCalibration │  │ Telescoping     │  │ GlobalTimeGrid │  │  │
│  │  │ Problem          │  │ Evaluator       │  │ Builder        │  │  │
│  │  └────────┬─────────┘  └────────┬────────┘  └───────┬────────┘  │  │
│  │           │                     │                   │           │  │
│  │           └──────────┬──────────┴───────────────────┘           │  │
│  │                      ▼                                          │  │
│  │          ┌───────────────────────┐                              │  │
│  │          │ GlobalBootstrapper    │◄───────────────────────────┐ │  │
│  │          │ (拡張)                 │                            │ │  │
│  │          └───────────┬───────────┘                            │ │  │
│  └──────────────────────┼────────────────────────────────────────┘ │
│                         │                                          │
└─────────────────────────┼──────────────────────────────────────────┘
                          │
    ┌─────────────────────┼─────────────────────────────────────┐
    │ pricer_core         │                                      │
    │  ┌──────────────────▼──────────────┐                       │
    │  │ MultidimensionalNewtonSolver    │                       │
    │  │ SystemOfEquations trait         │                       │
    │  │ MultidimSolverResult            │                       │
    │  └─────────────────────────────────┘                       │
    │  ┌─────────────────────────────────┐                       │
    │  │ nalgebra::DMatrix / DVector     │                       │
    │  │ linalg::{lu_solve, inverse}     │                       │
    │  └─────────────────────────────────┘                       │
    └────────────────────────────────────────────────────────────┘
                          │
    ┌─────────────────────┼─────────────────────────────────────┐
    │ pricer_risk         │                                      │
    │  ┌──────────────────▼──────────────┐                       │
    │  │ ImplicitSolver                  │                       │
    │  │ CurveSensitivities              │                       │
    │  └─────────────────────────────────┘                       │
    └────────────────────────────────────────────────────────────┘
```

### Boundary Definition

| Boundary | Responsibility | Interface |
|----------|---------------|-----------|
| `CurveCalibrationProblem` | `SystemOfEquations` 実装、残差関数 F(x) と Jacobian J(x) の提供 | `evaluate()`, `jacobian()` |
| `TelescopingEvaluator` | OIS/SOFR商品の効率的PV/感応度計算 | `evaluate_pv()`, `jacobian_row()` |
| `GlobalTimeGrid` | 全商品の日付を統合した時間軸管理 | `add_dates()`, `get_index()` |
| `CashflowMatrix` | キャッシュフロー係数行列 A の構築・キャッシュ | `build()`, `get()` |
| `GlobalBootstrapper` | ソルバー実行、結果・Jacobian逆行列の返却 | `calibrate()` → `GlobalBootstrapResult` |
| `ImplicitSolver` (既存) | AAD感応度計算 | `compute_curve_sensitivities()` |

---

## Technology Stack & Alignment

### 使用技術

| Category | Technology | Rationale | Alignment |
|----------|-----------|-----------|-----------|
| 線形代数 | nalgebra (`DMatrix`, `DVector`) | 既存パターンに従う、feature flag `linalg` | 要件10.4 |
| ソルバー | `MultidimensionalNewtonSolver` (既存) | 汎用Newton-Raphson、テスト済み | 要件1.1-1.6 |
| AAD | `ImplicitSolver` (既存) | 陰関数定理、J⁻ᵀ計算 | 要件4.1-4.4 |
| エラー | `CalibrationError`, `SolverError` (既存) | 統一エラーハンドリング | 要件9.1-9.5 |
| 設定 | Builder パターン | メソッドチェーン設定 | 要件10.5 |

### Feature Flags

```toml
[features]
global-bootstrap = ["linalg"]  # 既存フラグを維持
telescoping = ["global-bootstrap"]  # テレスコープ法サポート
```

### Steering Alignment

- **A-I-P-S Stream**: 本モジュールは `P` (Pricer) レイヤーに属し、`I` (Infra) や `A` (Adapter) への依存は持たない
- **Enzyme AAD**: `pricer_risk` の Shadow パターンと `ImplicitSolver` を通じて統合（直接 Enzyme 呼び出しは行わない）
- **British English**: `optimiser`, `calibration`, `serialisation` を使用

---

## Components & Interface Contracts

### Component 1: CurveCalibrationProblem

**Purpose**: カーブキャリブレーション問題を `SystemOfEquations` トレイトとして表現

**Requirements Traceability**: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6

```rust
/// カーブキャリブレーション問題の定義
///
/// パラメータ x = log(DF) として定義し、
/// 残差関数 F(x) = theoretical_rate(x) - market_rate を計算する。
pub struct CurveCalibrationProblem<T: Float, I: CalibrationInstrument<T>> {
    /// キャリブレーション商品リスト
    instruments: Vec<I>,
    /// 時間グリッド
    time_grid: GlobalTimeGrid<T>,
    /// キャッシュフロー行列 (キャッシュ)
    cashflow_matrix: Option<CashflowMatrix<T>>,
    /// Jacobian計算方法
    jacobian_method: JacobianMethod,
    /// テレスコープ評価器 (OIS/SOFR用)
    telescoping_evaluators: Vec<Option<TelescopingEvaluator<T>>>,
}

impl<T: RealField + Float + Copy, I: CalibrationInstrument<T>> SystemOfEquations<T>
    for CurveCalibrationProblem<T, I>
{
    fn dimension(&self) -> usize {
        self.instruments.len()
    }

    fn evaluate(&self, x: &DVector<T>) -> Result<DVector<T>, SolverError> {
        // x = log(DF) → DF = exp(x)
        // カーブ構築 → 各商品の pricing_error 計算
    }

    fn jacobian(&self, x: &DVector<T>) -> Result<DMatrix<T>, SolverError> {
        // JacobianMethod に応じて解析/数値/AADを選択
    }
}
```

**Interface Contract**:
- `new()`: 商品リストから問題を構築。商品数 = ピラー数を検証（要件2.6）
- `evaluate()`: 残差ベクトル F(x) を返す（要件2.3）
- `jacobian()`: Jacobian行列を返す（要件3.1）
- Errors: `CalibrationError::NoInstruments` if instruments.is_empty()（要件2.5）

### Component 2: JacobianBuilder

**Purpose**: Jacobian行列の効率的構築

**Requirements Traceability**: 3.1, 3.2, 3.3, 3.4, 3.5

```rust
/// Jacobian計算方法
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JacobianMethod {
    /// 解析的微分（行列積 A·diag(-D)·W）
    Analytical,
    /// 有限差分法（デフォルト）
    #[default]
    FiniteDifference,
    /// AAD (Enzyme) - 将来拡張
    #[cfg(feature = "enzyme-ad")]
    AutomaticDifferentiation,
}

/// Jacobian構築器
pub struct JacobianBuilder<T: Float> {
    /// Jacobian計算方法
    method: JacobianMethod,
    /// 有限差分のステップサイズ
    epsilon: T,
    /// 補間行列 W のキャッシュ
    interpolation_matrix: Option<DMatrix<T>>,
}

impl<T: RealField + Float + Copy> JacobianBuilder<T> {
    /// 解析的Jacobianを構築
    ///
    /// J = A · diag(-DF) · W
    /// where:
    ///   A = キャッシュフロー係数行列 (instruments × dates)
    ///   DF = Discount Factor ベクトル
    ///   W = 補間行列 (dates × pillars)
    pub fn build_analytical(
        &self,
        cashflow_matrix: &CashflowMatrix<T>,
        discount_factors: &[T],
        interpolation_matrix: &DMatrix<T>,
    ) -> DMatrix<T>;

    /// 数値的Jacobianを構築（有限差分）
    pub fn build_numerical<S: SystemOfEquations<T>>(
        &self,
        system: &S,
        x: &DVector<T>,
    ) -> Result<DMatrix<T>, SolverError>;
}
```

**Interface Contract**:
- `build_analytical()`: A·diag(-D)·W による行列積（要件3.2, 3.4）
- `build_numerical()`: 有限差分 (F(x+ε) - F(x)) / ε（デフォルト）
- `JacobianMethod::AutomaticDifferentiation`: 将来拡張（要件3.3）
- キャッシュフロー行列 A は定数としてキャッシュ（要件3.5）

### Component 3: TelescopingEvaluator

**Purpose**: OIS/SOFR商品のテレスコープ法による効率的評価

**Requirements Traceability**: 5.1, 5.2, 5.3, 5.4, 5.5

```rust
/// テレスコープ法評価器
///
/// OIS変動脚を DF(t_start)/DF(t_end) - 1 として計算し、
/// 日次ループを回避する。
pub struct TelescopingEvaluator<T: Float> {
    /// 開始日のグリッドインデックス
    start_index: usize,
    /// 終了日のグリッドインデックス
    end_index: usize,
    /// 支払日のグリッドインデックス (Payment Delay)
    payment_index: usize,
    /// 年率換算係数 (Day Count Fraction)
    year_fraction: T,
    /// Single Curve フラグ
    single_curve: bool,
}

impl<T: RealField + Float + Copy> TelescopingEvaluator<T> {
    /// テレスコープPVを計算
    ///
    /// Float PV = DF(t_payment) * (DF(t_start)/DF(t_end) - 1)
    pub fn evaluate_pv(&self, discount_factors: &[T]) -> T;

    /// Jacobian行要素を計算
    ///
    /// スパース構造: 開始日、終了日、支払日の3要素のみ非ゼロ
    pub fn jacobian_row(&self, discount_factors: &[T]) -> SparseJacobianRow<T>;
}

/// スパースJacobian行
pub struct SparseJacobianRow<T: Float> {
    /// (インデックス, 値) のペア
    entries: Vec<(usize, T)>,
}
```

**Interface Contract**:
- `evaluate_pv()`: DF(t_start)/DF(t_end) - 1 の計算（要件5.1, 5.2）
- `jacobian_row()`: 2〜3個の非ゼロ要素（要件5.4）
- Payment Delay 考慮（要件5.3）
- Single Curve 時の完全テレスコープ（要件5.5）

### Component 4: InstrumentEvaluators

**Purpose**: Deposit, Futures, Swap の評価器

**Requirements Traceability**: 6.1-6.5, 7.1-7.4

```rust
/// Deposit 評価器
pub struct DepositEvaluator<T: Float> {
    /// 満期日のグリッドインデックス
    maturity_index: usize,
    /// Day Count Fraction
    day_count_fraction: T,
}

impl<T: RealField + Float + Copy> DepositEvaluator<T> {
    /// インプライドレート = (1/DF - 1) / δ
    pub fn implied_rate(&self, discount_factors: &[T]) -> T;

    /// Jacobian行: 満期日のみに依存（1要素）
    pub fn jacobian_row(&self, discount_factors: &[T]) -> SparseJacobianRow<T>;
}

/// Futures/FRA 評価器
pub struct FuturesEvaluator<T: Float> {
    /// 開始日のグリッドインデックス
    start_index: usize,
    /// 終了日のグリッドインデックス
    end_index: usize,
    /// Day Count Fraction
    day_count_fraction: T,
    /// Convexity Adjustment
    convexity_adjustment: T,
}

impl<T: RealField + Float + Copy> FuturesEvaluator<T> {
    /// Forward Rate = (DF(start)/DF(end) - 1) / δ + convexity
    pub fn forward_rate(&self, discount_factors: &[T]) -> T;

    /// Jacobian行: 開始日と終了日（2要素）
    pub fn jacobian_row(&self, discount_factors: &[T]) -> SparseJacobianRow<T>;
}

/// Swap 評価器
pub struct SwapEvaluator<T: Float> {
    /// 固定脚キャッシュフロー日付インデックス
    fixed_leg_indices: Vec<usize>,
    /// 固定脚 Day Count Fractions
    fixed_leg_dcfs: Vec<T>,
    /// 変動脚評価器 (OIS の場合は TelescopingEvaluator)
    float_leg_evaluator: FloatLegEvaluator<T>,
}

/// 変動脚評価器
pub enum FloatLegEvaluator<T: Float> {
    /// テレスコープ法 (OIS/SOFR)
    Telescoping(TelescopingEvaluator<T>),
    /// 標準 (LIBOR) - 将来拡張
    Standard(StandardFloatLegEvaluator<T>),
}
```

**Interface Contract**:
- `DepositEvaluator`: 1要素のJacobian（要件6.2）
- `FuturesEvaluator`: 2要素のJacobian、Convexity対応（要件6.3-6.5）
- `SwapEvaluator`: 複数CF日付対応、OIS時はテレスコープ（要件7.1-7.4）

### Component 5: GlobalTimeGrid

**Purpose**: 全商品のキャッシュフロー日付を統合した時間グリッド

**Requirements Traceability**: 8.1, 8.2

```rust
/// グローバル時間グリッド
///
/// 全キャリブレーション商品のキャッシュフロー日付を収集・ソート・重複除去する。
pub struct GlobalTimeGrid<T: Float> {
    /// ソート済み・重複除去済みの年数 (year fractions)
    times: Vec<T>,
    /// 日付 → インデックスのマップ (高速検索用)
    index_map: HashMap<OrderedFloat<T>, usize>,
}

impl<T: RealField + Float + Copy + Hash> GlobalTimeGrid<T> {
    /// 商品リストから時間グリッドを構築
    pub fn from_instruments<I: CalibrationInstrument<T>>(instruments: &[I]) -> Self;

    /// 日付リストを追加
    pub fn add_dates(&mut self, dates: impl IntoIterator<Item = T>);

    /// 日付のインデックスを取得
    pub fn get_index(&self, time: T) -> Option<usize>;

    /// 最近傍インデックスを取得（補間用）
    pub fn get_nearest_indices(&self, time: T) -> (usize, usize, T);

    /// グリッドサイズ
    pub fn len(&self) -> usize;
}
```

**Interface Contract**:
- `from_instruments()`: 商品からCF日付を収集（要件8.1）
- `add_dates()`: 追加日付のマージ
- `get_index()`: O(1) 検索
- 重複除去・ソート保証（要件8.2）

### Component 6: CashflowMatrix

**Purpose**: キャッシュフロー係数行列の構築・キャッシュ

**Requirements Traceability**: 8.3, 8.4, 8.5

```rust
/// キャッシュフロー行列
///
/// 行: 商品, 列: GlobalTimeGrid の日付
/// 各要素はその商品・日付のキャッシュフロー係数
pub struct CashflowMatrix<T: Float> {
    /// 行列データ (instruments × dates)
    data: DMatrix<T>,
    /// 構築時のグリッド参照
    time_grid_size: usize,
}

impl<T: RealField + Float + Copy> CashflowMatrix<T> {
    /// 商品リストと時間グリッドから行列を構築
    pub fn build<I: CalibrationInstrument<T>>(
        instruments: &[I],
        time_grid: &GlobalTimeGrid<T>,
    ) -> Self;

    /// 行列への参照を取得
    pub fn as_matrix(&self) -> &DMatrix<T>;

    /// 行（商品）のスライスを取得
    pub fn row(&self, instrument_idx: usize) -> DVector<T>;
}

/// 補間行列ビルダー
///
/// W[i,j] = 日付 i のピラー j に対する補間重み
pub struct InterpolationMatrixBuilder<T: Float> {
    /// 補間方法
    interpolation: BootstrapInterpolation,
}

impl<T: RealField + Float + Copy> InterpolationMatrixBuilder<T> {
    /// 補間行列を構築 (dates × pillars)
    pub fn build(
        &self,
        time_grid: &GlobalTimeGrid<T>,
        pillars: &[T],
    ) -> DMatrix<T>;
}
```

**Interface Contract**:
- `CashflowMatrix::build()`: 商品→行、日付→列のマッピング（要件8.3）
- `InterpolationMatrixBuilder::build()`: W行列の構築（要件8.4）
- 定数行列としてキャッシュ、反復ごとの再計算回避（要件8.5）

### Component 7: GlobalBootstrapper (拡張)

**Purpose**: 既存 `GlobalBootstrapper` の拡張

**Requirements Traceability**: 1.1-1.6, 9.1-9.5, 10.1-10.5

```rust
/// グローバルブートストラップ設定（拡張）
#[derive(Debug, Clone)]
pub struct GlobalBootstrapConfig<T: Float> {
    // --- 既存フィールド ---
    pub tolerance: T,
    pub param_tolerance: T,
    pub max_iterations: usize,
    pub jacobian_epsilon: T,
    pub store_jacobian_inverse: bool,
    pub interpolation: BootstrapInterpolation,
    pub allow_extrapolation: bool,

    // --- 新規フィールド ---
    /// Jacobian計算方法
    pub jacobian_method: JacobianMethod,
    /// テレスコープ法の有効化
    pub enable_telescoping: bool,
    /// ダンピング係数 (Levenberg-Marquardt 的)
    pub damping_factor: Option<T>,
    /// デバッグログの有効化
    pub debug_logging: bool,
}

/// グローバルブートストラップ結果（拡張）
#[derive(Debug, Clone)]
pub struct GlobalBootstrapResult<T: Float> {
    // --- 既存フィールド ---
    pub curve: BootstrappedCurve<T>,
    pub pillars: Vec<T>,
    pub discount_factors: Vec<T>,
    pub residual_norm: T,
    pub iterations: usize,
    pub converged: bool,
    pub jacobian_inverse: Option<DMatrix<T>>,

    // --- 新規フィールド ---
    /// 反復ごとの残差履歴 (デバッグ用)
    pub residual_history: Option<Vec<T>>,
    /// 最終Jacobian行列の条件数
    pub condition_number: Option<T>,
}

impl<T: RealField + Float + Copy> GlobalBootstrapper<T> {
    /// カーブをキャリブレート（SystemOfEquations 使用版）
    pub fn calibrate_with_problem<I: CalibrationInstrument<T>>(
        &self,
        problem: CurveCalibrationProblem<T, I>,
    ) -> Result<GlobalBootstrapResult<T>, CalibrationError>;
}
```

**Interface Contract**:
- `calibrate_with_problem()`: `CurveCalibrationProblem` を受け取り、`MultidimensionalNewtonSolver` で解く
- `GlobalBootstrapConfig`: Builder パターンで設定（要件10.5）
- `GlobalBootstrapResult`: 反復数、残差、条件数を含む（要件9.3, 9.5）
- Errors: `CalibrationError::SingularJacobian`, `CalibrationError::ConvergenceFailure`（要件9.1, 9.2）

### Component 8: Error Types (拡張)

**Purpose**: キャリブレーションエラーの拡張

**Requirements Traceability**: 9.1, 9.2, 9.4

```rust
/// 拡張キャリブレーションエラー
#[derive(Error, Debug, Clone)]
pub enum CalibrationError {
    // --- 既存バリアント ---
    #[error("キャリブレーションが収束しませんでした (iterations: {iterations}, residual: {residual:.6e})")]
    ConvergenceFailure { iterations: usize, residual: f64 },

    // ... 既存バリアント省略 ...

    // --- 新規バリアント ---
    /// 商品リストが空
    #[error("キャリブレーション商品が指定されていません")]
    NoInstruments,

    /// Jacobian行列が特異
    #[error("Jacobian行列が特異です (condition number: {condition_number:.2e})")]
    SingularJacobian { condition_number: f64 },

    /// ソルバーが発散
    #[error("ソルバーが発散しました (iteration: {iteration}, residual: {residual:.6e})")]
    Divergence { iteration: usize, residual: f64 },

    /// 商品評価エラー
    #[error("商品 {instrument_index} の評価に失敗しました: {message}")]
    InstrumentEvaluationFailed {
        instrument_index: usize,
        message: String,
    },
}
```

---

## Data Models

### Input Models

```rust
/// キャリブレーション入力
pub struct CalibrationInput<T: Float, I: CalibrationInstrument<T>> {
    /// キャリブレーション商品リスト
    pub instruments: Vec<I>,
    /// ピラー日付 (省略時は商品満期から推定)
    pub pillars: Option<Vec<T>>,
    /// ソルバー設定
    pub config: GlobalBootstrapConfig<T>,
}
```

### Output Models

```rust
/// AAD用カーブ感応度
pub struct CurveRiskResult<T: Float> {
    /// 市場レートへの感応度 ∂V/∂m
    pub market_sensitivities: DVector<T>,
    /// ピラーへの感応度 ∂V/∂x
    pub pillar_sensitivities: DVector<T>,
    /// Key Rate Duration (10Y等)
    pub key_rate_durations: HashMap<String, T>,
}
```

---

## Error Handling Strategy

### Error Hierarchy

```
CalibrationError (top-level)
├── NoInstruments           → 入力検証で早期リターン
├── SingularJacobian        → LU分解失敗時
├── ConvergenceFailure      → 最大反復到達時
├── Divergence              → 残差増大検出時
├── InstrumentEvaluationFailed → 商品PV計算失敗時
└── NumericalInstability    → NaN/Inf検出時
```

### Recovery Strategies

| Error | Recovery |
|-------|----------|
| `SingularJacobian` | ダンピング係数追加、正則化項追加 |
| `ConvergenceFailure` | 初期値変更、許容誤差緩和 |
| `Divergence` | ダンピング係数増加、ラインサーチ追加 |

---

## Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|------------|
| `CurveCalibrationProblem` | `evaluate()` と `jacobian()` の整合性、次元検証 |
| `TelescopingEvaluator` | PV計算の正確性、Jacobian行の非ゼロ要素数 |
| `GlobalTimeGrid` | ソート・重複除去、インデックス検索の正確性 |
| `CashflowMatrix` | 行列サイズ、要素配置の正確性 |
| `JacobianBuilder` | 解析 vs 数値Jacobianの一致 (相対誤差 < 1e-6) |

### Integration Tests

| Scenario | Validation |
|----------|------------|
| OISカーブ構築 (5商品) | 全商品の pricing error < 1e-10 |
| SOFR+Futuresカーブ | 混合商品タイプでの収束 |
| 30ピラーカーブ | パフォーマンス (< 100ms) |
| 逆イールドカーブ | 負の傾きでの安定性 |
| AAD統合 | ImplicitSolver との連携、DV01計算 |

### Property-Based Tests

- Jacobian行列の対称性（OIS only のケース）
- 残差ノルムの単調減少（正常収束時）
- Jacobian逆行列の J · J⁻¹ = I 検証

---

## Requirements Traceability Matrix

| Requirement ID | Component | Interface/Method |
|----------------|-----------|------------------|
| 1.1, 1.3, 1.4 | `MultidimensionalNewtonSolver` (既存) | `solve()` |
| 1.2 | `GlobalBootstrapResult` | `jacobian_inverse` |
| 1.5 | `pricer_core::math::linalg` | nalgebra |
| 1.6 | `CurveCalibrationProblem` | `SystemOfEquations` impl |
| 2.1, 2.2, 2.3, 2.4 | `CurveCalibrationProblem` | `evaluate()`, `jacobian()` |
| 2.5, 2.6 | `CurveCalibrationProblem::new()` | 入力検証 |
| 3.1, 3.2, 3.4, 3.5 | `JacobianBuilder` | `build_analytical()`, `build_numerical()` |
| 3.3 | `JacobianMethod::AutomaticDifferentiation` | (将来拡張) |
| 4.1, 4.2, 4.3 | `ImplicitSolver` (既存) | `compute_curve_sensitivities()` |
| 4.4 | `pricer_risk::greeks::ad::shadow` (既存) | Shadow トレイト |
| 5.1, 5.2, 5.3, 5.4, 5.5 | `TelescopingEvaluator` | `evaluate_pv()`, `jacobian_row()` |
| 6.1, 6.2 | `DepositEvaluator` | `implied_rate()`, `jacobian_row()` |
| 6.3, 6.4, 6.5 | `FuturesEvaluator` | `forward_rate()`, `jacobian_row()` |
| 7.1, 7.2, 7.3, 7.4 | `SwapEvaluator` | 固定脚/変動脚評価 |
| 8.1, 8.2 | `GlobalTimeGrid` | `from_instruments()`, `add_dates()` |
| 8.3, 8.4, 8.5 | `CashflowMatrix`, `InterpolationMatrixBuilder` | `build()` |
| 9.1 | `CalibrationError::SingularJacobian` | エラーハンドリング |
| 9.2 | `CalibrationError::Divergence` | エラーハンドリング |
| 9.3 | `GlobalBootstrapConfig::debug_logging` | ログ出力 |
| 9.4 | `CalibrationError::InstrumentEvaluationFailed` | エラーハンドリング |
| 9.5 | `GlobalBootstrapResult` | `iterations`, `residual_norm` |
| 10.1 | `GlobalBootstrapConfig::tolerance` | デフォルト 1e-10 |
| 10.2 | `GlobalBootstrapConfig::max_iterations` | デフォルト 100 |
| 10.3 | `GlobalBootstrapConfig::jacobian_method` | 選択可能 |
| 10.4 | nalgebra | 線形代数バックエンド |
| 10.5 | `GlobalBootstrapConfig` | Builder パターン |

---

## File Structure

```
crates/pricer_models/src/builder/
├── mod.rs                    # モジュール宣言
├── globalsolver.rs           # GlobalBootstrapper (既存、拡張)
├── calibration_problem.rs    # CurveCalibrationProblem [NEW]
├── jacobian_builder.rs       # JacobianBuilder, JacobianMethod [NEW]
├── telescoping.rs            # TelescopingEvaluator [NEW]
├── evaluators/               # 商品評価器 [NEW]
│   ├── mod.rs
│   ├── deposit.rs
│   ├── futures.rs
│   └── swap.rs
├── time_grid.rs              # GlobalTimeGrid [NEW]
├── cashflow_matrix.rs        # CashflowMatrix, InterpolationMatrixBuilder [NEW]
├── instrument.rs             # CalibrationInstrument (既存)
├── error.rs                  # CalibrationError (既存、拡張)
└── bootstrap.rs              # 逐次ブートストラップ (既存)
```

---

## Implementation Notes

### パフォーマンス考慮事項

1. **キャッシュフロー行列のキャッシュ**: `CashflowMatrix` と `InterpolationMatrixBuilder` は初回のみ計算し、反復中は再利用
2. **スパースJacobian**: テレスコープ法により、行あたり2〜3要素の非ゼロ（将来的にスパース行列形式を検討）
3. **解析Jacobianの優先**: 数値Jacobian (n+1 評価) より解析Jacobian (1 評価 + 行列積) を優先

### 既存コードとの互換性

1. **後方互換**: 既存の `GlobalBootstrapper::calibrate()` は維持、新規メソッド `calibrate_with_problem()` を追加
2. **feature gate**: `global-bootstrap` フラグ下で有効化（既存パターンを維持）
3. **CalibrationInstrument**: 既存トレイトを変更せず、新規評価器で拡張

### 将来拡張

1. **Enzyme AAD**: `JacobianMethod::AutomaticDifferentiation` は feature flag で有効化
2. **Dual Curve**: テレスコープ法の部分適用、プロジェクションカーブ対応
3. **スパース行列**: ndarray-sparse または sprs によるメモリ最適化
