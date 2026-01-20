# Technical Design: pricer-core-math-library

## 概要

本ドキュメントは、`pricer_core::math`モジュールを拡張し、金融デリバティブ価格計算に必要な包括的な数学ライブラリを実装するための技術設計を定義する。

## 設計原則

1. **AD互換性**: すべての関数は`T: num_traits::Float`でジェネリックに実装し、Enzyme自動微分との互換性を確保
2. **一貫性**: 既存の`pricer_core`パターン（エラーハンドリング、ドキュメント、テスト）を踏襲
3. **段階的実装**: Phase 1（基盤）→ Phase 2（数値計算）→ Phase 3（高度機能）の順で実装
4. **層分離**: L1（pricer_core）は純粋数学、L2（pricer_models）は金融ロジックを維持

## モジュール構造

```
crates/pricer_core/src/math/
├── mod.rs                    # メインモジュール（既存 + 新規エクスポート）
├── numeric.rs                # 既存：型変換ユーティリティ
├── smoothing.rs              # 既存：スムース関数
├── interpolators/            # 既存 + 拡張
│   ├── mod.rs
│   ├── traits.rs            # 既存：Interpolator<T> trait
│   ├── linear.rs            # 既存
│   ├── cubic_spline.rs      # 既存
│   ├── monotonic.rs         # 既存
│   ├── bilinear.rs          # 既存
│   ├── smooth_interp.rs     # 既存
│   ├── flat.rs              # 新規：フラット補間
│   ├── log_linear.rs        # 新規：対数線形補間
│   ├── hermite.rs           # 新規：Hermiteスプライン
│   ├── svi.rs               # 新規：SVI補間
│   └── search.rs            # 新規：二分探索/線形探索
├── solvers/                  # 既存 + 拡張
│   ├── mod.rs
│   ├── config.rs            # 既存
│   ├── newton_raphson.rs    # 既存
│   ├── brent.rs             # 既存
│   ├── levenberg_marquardt.rs # 既存
│   ├── bisection.rs         # 新規：二分法
│   └── backtracking_newton.rs # 新規：直線探索付きNewton法
├── distributions/            # 新規モジュール
│   ├── mod.rs
│   ├── error.rs             # DistributionError
│   ├── normal.rs            # 正規分布（CDF, PDF, inverse CDF）
│   ├── bivariate_normal.rs  # 二変量正規分布
│   ├── chi_squared.rs       # 非心カイ二乗分布
│   └── copula.rs            # ガウシアンコピュラ
├── integrators/              # 新規モジュール
│   ├── mod.rs
│   ├── error.rs             # IntegrationError
│   ├── gauss_legendre.rs    # Gauss-Legendre求積法
│   ├── gauss_kronrod.rs     # Gauss-Kronrod求積法
│   ├── adaptive.rs          # 適応的積分
│   └── runge_kutta.rs       # RK4, RK45
├── calculus/                 # 新規モジュール
│   ├── mod.rs
│   ├── finite_difference.rs # 有限差分法
│   └── bump_selection.rs    # bump幅自動選択
├── optimisers/               # 新規モジュール
│   ├── mod.rs
│   ├── error.rs             # OptimisationError
│   ├── config.rs            # 最適化設定
│   ├── lbfgs.rs             # L-BFGS
│   ├── nelder_mead.rs       # Nelder-Mead
│   └── line_search.rs       # 直線探索アルゴリズム
├── linalg/                   # 新規モジュール
│   ├── mod.rs
│   ├── error.rs             # LinearAlgebraError
│   ├── matrix.rs            # Matrix<T>構造体
│   ├── cholesky.rs          # コレスキー分解
│   ├── lu.rs                # LU分解
│   └── operations.rs        # 行列演算
├── fitting/                  # 新規モジュール
│   ├── mod.rs
│   ├── least_squares.rs     # 線形最小二乗
│   └── gaussian.rs          # ガウシアンフィット
├── mesh/                     # 新規モジュール
│   ├── mod.rs
│   ├── grid_1d.rs           # 1次元メッシュ
│   └── grid_2d.rs           # 2次元メッシュ
└── utilities/                # 新規モジュール
    ├── mod.rs
    ├── basic.rs             # sign, clamp, lerp
    ├── combinatorics.rs     # factorial, binomial
    └── special.rs           # log_gamma, beta
```

## コンポーネント設計

### 1. distributions モジュール

#### 1.1 エラー型

```rust
// distributions/error.rs
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum DistributionError {
    #[error("Probability {p} out of range [0, 1]")]
    InvalidProbability { p: f64 },

    #[error("Correlation coefficient {rho} out of range [-1, 1]")]
    InvalidCorrelation { rho: f64 },

    #[error("Degrees of freedom must be positive: got {df}")]
    InvalidDegreesOfFreedom { df: f64 },

    #[error("Non-centrality parameter must be non-negative: got {ncp}")]
    InvalidNonCentrality { ncp: f64 },

    #[error("Correlation matrix is not positive definite")]
    NotPositiveDefinite,

    #[error("Numerical computation failed: {0}")]
    NumericalError(String),
}
```

#### 1.2 正規分布インターフェース

```rust
// distributions/normal.rs
use num_traits::Float;

/// 標準正規分布の累積分布関数
///
/// Hart近似を使用し、精度1e-15を達成。
///
/// # Arguments
/// * `x` - 入力値
///
/// # Returns
/// P(X <= x) where X ~ N(0, 1)
pub fn norm_cdf<T: Float>(x: T) -> T;

/// 標準正規分布の確率密度関数
///
/// φ(x) = (1/√2π) * exp(-x²/2)
pub fn norm_pdf<T: Float>(x: T) -> T;

/// 標準正規分布の逆累積分布関数（Quantile）
///
/// Acklam近似を使用し、精度1e-9を達成。
///
/// # Arguments
/// * `p` - 確率値 (0, 1)
///
/// # Returns
/// x such that P(X <= x) = p
///
/// # Errors
/// * `DistributionError::InvalidProbability` if p <= 0 or p >= 1
pub fn norm_inv_cdf<T: Float>(p: T) -> Result<T, DistributionError>;
```

#### 1.3 二変量正規分布インターフェース

```rust
// distributions/bivariate_normal.rs

/// 二変量正規分布の累積分布関数
///
/// Drezner-Wesolowsky近似を使用。
///
/// # Arguments
/// * `x` - 第1変数の上限
/// * `y` - 第2変数の上限
/// * `rho` - 相関係数 [-1, 1]
///
/// # Returns
/// P(X <= x, Y <= y) where (X, Y) ~ BVN(0, 0, 1, 1, rho)
pub fn bivariate_norm_cdf<T: Float>(x: T, y: T, rho: T) -> Result<T, DistributionError>;
```

### 2. integrators モジュール

#### 2.1 積分結果型

```rust
// integrators/mod.rs

/// 数値積分の結果
#[derive(Debug, Clone)]
pub struct IntegrationResult<T: Float> {
    /// 積分値
    pub value: T,
    /// 誤差推定（Gauss-Kronrodの場合）
    pub error_estimate: Option<T>,
    /// 関数評価回数
    pub evaluations: usize,
}
```

#### 2.2 Gauss-Legendreインターフェース

```rust
// integrators/gauss_legendre.rs

/// Gauss-Legendre求積法の次数
#[derive(Debug, Clone, Copy)]
pub enum GaussLegendreOrder {
    /// 7点（精度 O(h^14)）
    N7,
    /// 15点（精度 O(h^30)）
    N15,
    /// 21点（精度 O(h^42)）
    N21,
}

/// Gauss-Legendre求積法による積分
///
/// # Arguments
/// * `f` - 被積分関数
/// * `a` - 積分下限
/// * `b` - 積分上限
/// * `order` - 求積法の次数
pub fn integrate_gauss_legendre<T, F>(
    f: F,
    a: T,
    b: T,
    order: GaussLegendreOrder,
) -> IntegrationResult<T>
where
    T: Float,
    F: Fn(T) -> T;
```

#### 2.3 Gauss-Kronrodインターフェース

```rust
// integrators/gauss_kronrod.rs

/// Gauss-Kronrod規則
#[derive(Debug, Clone, Copy)]
pub enum GaussKronrodRule {
    /// G7-K15（7点Gauss、15点Kronrod）
    G7K15,
    /// G10-K21（10点Gauss、21点Kronrod）
    G10K21,
}

/// Gauss-Kronrod求積法による積分（誤差推定付き）
pub fn integrate_gauss_kronrod<T, F>(
    f: F,
    a: T,
    b: T,
    rule: GaussKronrodRule,
) -> IntegrationResult<T>
where
    T: Float,
    F: Fn(T) -> T;
```

#### 2.4 Runge-Kuttaインターフェース

```rust
// integrators/runge_kutta.rs

/// RK4による1ステップ積分
///
/// dy/dt = f(t, y) を時刻 t から t+h まで積分
pub fn rk4_step<T, F>(f: F, t: T, y: T, h: T) -> T
where
    T: Float,
    F: Fn(T, T) -> T;

/// RK45（Dormand-Prince）による適応的積分
pub fn rk45_integrate<T, F>(
    f: F,
    t0: T,
    y0: T,
    t_end: T,
    tol: T,
) -> Result<Vec<(T, T)>, IntegrationError>
where
    T: Float,
    F: Fn(T, T) -> T;
```

### 3. calculus モジュール

#### 3.1 有限差分インターフェース

```rust
// calculus/finite_difference.rs

/// 有限差分の種類
#[derive(Debug, Clone, Copy)]
pub enum DifferenceType {
    /// 前方差分: (f(x+h) - f(x)) / h
    Forward,
    /// 後方差分: (f(x) - f(x-h)) / h
    Backward,
    /// 中心差分: (f(x+h) - f(x-h)) / (2h)
    Central,
}

/// 1階導関数の有限差分近似
pub fn finite_diff<T, F>(
    f: F,
    x: T,
    h: T,
    diff_type: DifferenceType,
) -> T
where
    T: Float,
    F: Fn(T) -> T;

/// 2階導関数の有限差分近似
///
/// (f(x+h) - 2*f(x) + f(x-h)) / h²
pub fn finite_diff_second<T, F>(f: F, x: T, h: T) -> T
where
    T: Float,
    F: Fn(T) -> T;

/// 偏微分の有限差分近似
pub fn partial_diff<T, F>(
    f: F,
    x: &[T],
    index: usize,
    h: T,
    diff_type: DifferenceType,
) -> T
where
    T: Float,
    F: Fn(&[T]) -> T;
```

### 4. optimisers モジュール

#### 4.1 最適化設定

```rust
// optimisers/config.rs

/// 最適化の収束設定
#[derive(Debug, Clone)]
pub struct OptimisationConfig<T: Float> {
    /// 勾配ノルムの収束閾値
    pub gradient_tolerance: T,
    /// 関数値変化の収束閾値
    pub function_tolerance: T,
    /// パラメータ変化の収束閾値
    pub parameter_tolerance: T,
    /// 最大反復回数
    pub max_iterations: usize,
}

impl<T: Float> Default for OptimisationConfig<T> {
    fn default() -> Self {
        Self {
            gradient_tolerance: from_f64(1e-8),
            function_tolerance: from_f64(1e-10),
            parameter_tolerance: from_f64(1e-8),
            max_iterations: 1000,
        }
    }
}
```

#### 4.2 最適化結果

```rust
// optimisers/mod.rs

/// 最適化の結果
#[derive(Debug, Clone)]
pub struct OptimisationResult<T: Float> {
    /// 最適パラメータ
    pub params: Vec<T>,
    /// 最終関数値
    pub value: T,
    /// 収束したか
    pub converged: bool,
    /// 反復回数
    pub iterations: usize,
    /// 最終勾配ノルム
    pub gradient_norm: T,
    /// 収束理由
    pub termination_reason: TerminationReason,
}

#[derive(Debug, Clone, Copy)]
pub enum TerminationReason {
    GradientConverged,
    FunctionConverged,
    ParameterConverged,
    MaxIterationsReached,
    NumericalError,
}
```

#### 4.3 L-BFGSインターフェース

```rust
// optimisers/lbfgs.rs

/// L-BFGS最適化器
pub struct LBFGSOptimiser<T: Float> {
    config: OptimisationConfig<T>,
    memory_size: usize,  // 履歴数（デフォルト: 10）
}

impl<T: Float> LBFGSOptimiser<T> {
    pub fn new(config: OptimisationConfig<T>) -> Self;

    pub fn with_memory_size(self, m: usize) -> Self;

    /// 最小化を実行
    ///
    /// # Arguments
    /// * `f` - 目的関数
    /// * `grad` - 勾配関数
    /// * `x0` - 初期値
    pub fn minimize<F, G>(
        &self,
        f: F,
        grad: G,
        x0: Vec<T>,
    ) -> Result<OptimisationResult<T>, OptimisationError>
    where
        F: Fn(&[T]) -> T,
        G: Fn(&[T]) -> Vec<T>;
}
```

#### 4.4 Nelder-Meadインターフェース

```rust
// optimisers/nelder_mead.rs

/// Nelder-Mead最適化器（導関数不要）
pub struct NelderMeadOptimiser<T: Float> {
    config: OptimisationConfig<T>,
    alpha: T,  // 反射係数
    gamma: T,  // 膨張係数
    rho: T,    // 収縮係数
    sigma: T,  // 縮小係数
}

impl<T: Float> NelderMeadOptimiser<T> {
    pub fn new(config: OptimisationConfig<T>) -> Self;

    /// 最小化を実行（導関数不要）
    pub fn minimize<F>(
        &self,
        f: F,
        x0: Vec<T>,
    ) -> Result<OptimisationResult<T>, OptimisationError>
    where
        F: Fn(&[T]) -> T;
}
```

### 5. linalg モジュール

#### 5.1 行列型

```rust
// linalg/matrix.rs

/// 汎用行列（行優先格納）
#[derive(Debug, Clone)]
pub struct Matrix<T: Float> {
    data: Vec<T>,
    rows: usize,
    cols: usize,
}

impl<T: Float> Matrix<T> {
    pub fn new(rows: usize, cols: usize) -> Self;
    pub fn from_vec(rows: usize, cols: usize, data: Vec<T>) -> Result<Self, LinearAlgebraError>;
    pub fn identity(n: usize) -> Self;
    pub fn zeros(rows: usize, cols: usize) -> Self;

    pub fn get(&self, i: usize, j: usize) -> Option<T>;
    pub fn set(&mut self, i: usize, j: usize, value: T) -> Result<(), LinearAlgebraError>;

    pub fn rows(&self) -> usize;
    pub fn cols(&self) -> usize;

    pub fn transpose(&self) -> Self;
    pub fn add(&self, other: &Self) -> Result<Self, LinearAlgebraError>;
    pub fn sub(&self, other: &Self) -> Result<Self, LinearAlgebraError>;
    pub fn mul(&self, other: &Self) -> Result<Self, LinearAlgebraError>;
    pub fn scale(&self, scalar: T) -> Self;
}

/// 正方行列（コレスキー分解等に使用）
pub type SquareMatrix<T> = Matrix<T>;
```

#### 5.2 分解インターフェース

```rust
// linalg/cholesky.rs

/// コレスキー分解（LL^T）
///
/// # Arguments
/// * `a` - 正定値対称行列
///
/// # Returns
/// 下三角行列L such that A = L * L^T
pub fn cholesky<T: Float>(a: &Matrix<T>) -> Result<Matrix<T>, LinearAlgebraError>;

/// コレスキー分解を用いた線形方程式の解法
///
/// A * x = b を解く（A は正定値対称）
pub fn cholesky_solve<T: Float>(
    a: &Matrix<T>,
    b: &[T],
) -> Result<Vec<T>, LinearAlgebraError>;
```

```rust
// linalg/lu.rs

/// LU分解の結果
pub struct LUDecomposition<T: Float> {
    pub l: Matrix<T>,
    pub u: Matrix<T>,
    pub p: Vec<usize>,  // ピボット順列
}

/// LU分解（部分ピボット選択）
pub fn lu_decompose<T: Float>(a: &Matrix<T>) -> Result<LUDecomposition<T>, LinearAlgebraError>;

/// LU分解を用いた線形方程式の解法
pub fn lu_solve<T: Float>(
    lu: &LUDecomposition<T>,
    b: &[T],
) -> Result<Vec<T>, LinearAlgebraError>;

/// 行列式の計算
pub fn determinant<T: Float>(a: &Matrix<T>) -> Result<T, LinearAlgebraError>;
```

### 6. interpolators 拡張

#### 6.1 フラット補間

```rust
// interpolators/flat.rs

/// フラット補間（区分定数）
pub struct FlatInterpolator<T: Float> {
    xs: Vec<T>,
    ys: Vec<T>,
    mode: FlatMode,
}

#[derive(Debug, Clone, Copy)]
pub enum FlatMode {
    /// 左側の値を使用
    Left,
    /// 右側の値を使用
    Right,
}

impl<T: Float> Interpolator<T> for FlatInterpolator<T> {
    fn interpolate(&self, x: T) -> Result<T, InterpolationError>;
    fn domain(&self) -> (T, T);
}
```

#### 6.2 対数線形補間

```rust
// interpolators/log_linear.rs

/// 対数線形補間（ディスカウントファクター用）
pub struct LogLinearInterpolator<T: Float> {
    xs: Vec<T>,
    log_ys: Vec<T>,  // ln(y) を格納
}

impl<T: Float> LogLinearInterpolator<T> {
    /// ディスカウントファクター用コンストラクタ
    ///
    /// y値は正でなければならない
    pub fn new(xs: &[T], ys: &[T]) -> Result<Self, InterpolationError>;
}
```

#### 6.3 二分探索

```rust
// interpolators/search.rs

/// 二分探索によるインデックス検索
///
/// xs[i] <= x < xs[i+1] となるiを返す
pub fn binary_search<T: Float>(xs: &[T], x: T) -> usize;

/// 線形探索によるインデックス検索（小規模配列用）
pub fn linear_search<T: Float>(xs: &[T], x: T) -> usize;
```

### 7. solvers 拡張

#### 7.1 二分法

```rust
// solvers/bisection.rs

/// 二分法ソルバー
pub struct BisectionSolver<T: Float> {
    config: SolverConfig<T>,
}

impl<T: Float> BisectionSolver<T> {
    pub fn new(config: SolverConfig<T>) -> Self;

    /// 二分法による根の探索
    ///
    /// # Arguments
    /// * `f` - 目的関数
    /// * `a` - 左端点（f(a)とf(b)は異符号であること）
    /// * `b` - 右端点
    pub fn find_root<F>(&self, f: F, a: T, b: T) -> Result<T, SolverError>
    where
        F: Fn(T) -> T;
}
```

### 8. utilities モジュール

```rust
// utilities/basic.rs

/// 符号関数
pub fn sign<T: Float>(x: T) -> T;

/// クランプ関数
pub fn clamp<T: Float>(x: T, min: T, max: T) -> T;

/// 線形補間
pub fn lerp<T: Float>(a: T, b: T, t: T) -> T;
```

```rust
// utilities/combinatorics.rs

/// 階乗（n!）
pub fn factorial<T: Float>(n: usize) -> T;

/// 二項係数（nCk）
pub fn binomial<T: Float>(n: usize, k: usize) -> T;
```

```rust
// utilities/special.rs

/// 対数ガンマ関数
pub fn log_gamma<T: Float>(x: T) -> T;

/// ベータ関数
pub fn beta<T: Float>(a: T, b: T) -> T;
```

## 依存関係更新

### pricer_core/Cargo.toml 変更なし

既存の依存関係で実装可能:
- `num-traits`: Float trait
- `thiserror`: エラー型

### pricer_models の変更

`pricer_models`は`pricer_core::math::distributions`から`norm_cdf`、`norm_pdf`をインポート:

```rust
// pricer_models/src/analytical/distributions.rs を削除
// 代わりに pricer_core::math::distributions を使用

// pricer_models/src/analytical/black_scholes.rs
use pricer_core::math::distributions::{norm_cdf, norm_pdf};
```

## テスト戦略

### 単体テスト

各モジュールで以下をテスト:
1. **基本動作**: 期待される出力値の検証
2. **エッジケース**: 境界値、特殊入力
3. **エラーケース**: 不正入力に対するエラー返却
4. **精度検証**: 参照値との比較（`assert_relative_eq!`）

### プロパティベーステスト

`proptest`を使用:
1. **数学的性質**: 対称性、単調性、境界
2. **不変条件**: 入力範囲、出力範囲
3. **数値安定性**: NaN/Inf の回避

### 統合テスト

`pricer_models`との統合:
1. Black-Scholesが新しい`norm_cdf`、`norm_pdf`で正しく動作
2. 既存のテストが引き続きパス

## 要件トレーサビリティマトリクス

| 要件ID | 要件名 | 実装コンポーネント |
|--------|--------|-------------------|
| REQ-1 | 確率分布 | `distributions/` |
| REQ-2 | 数値積分 | `integrators/` |
| REQ-3 | 有限差分 | `calculus/` |
| REQ-4 | 最適化拡張 | `optimisers/` |
| REQ-5 | 1D補間拡張 | `interpolators/flat.rs`, `log_linear.rs`, `hermite.rs`, `svi.rs`, `search.rs` |
| REQ-6 | 2D/3D補間 | `interpolators/` (既存BilinearInterpolator拡張) |
| REQ-7 | 金融関数 | `pricer_models`に残す（設計決定） |
| REQ-8 | フィッティング | `fitting/` |
| REQ-9 | 線形代数 | `linalg/` |
| REQ-10 | 乱数生成 | `pricer_pricing`に残す（設計決定） |
| REQ-11 | ソルバー拡張 | `solvers/bisection.rs`, `backtracking_newton.rs` |
| REQ-12 | メッシュ | `mesh/` |
| REQ-13 | ユーティリティ | `utilities/` |
| REQ-14 | 非機能要件 | 全モジュールで対応 |

## 設計決定記録

### DD-1: 金融関数の配置

**決定**: Black-Scholes、Bachelier、SABR等の金融関数は`pricer_models`に残す。

**理由**:
1. L1（pricer_core）は純粋数学ライブラリとして維持
2. 金融関数はコール/プットの概念等、金融ドメイン知識を含む
3. 分布関数（norm_cdf等）は数学的基盤として`pricer_core`に移動

### DD-2: 乱数生成の配置

**決定**: 乱数生成は`pricer_pricing`に残す。

**理由**:
1. Enzyme AD互換性の観点で、RNGは`pricer_pricing`のMCシミュレーションと密結合
2. 他クレートでRNGが必要な場合は、トレイト定義のみ`pricer_core`に配置可能

### DD-3: 線形代数の実装方針

**決定**: 小規模行列（10x10以下）に特化した自前実装。

**理由**:
1. Enzyme AD互換性を完全に保証
2. 外部クレート（nalgebra等）の依存を回避
3. 金融計算で使用される行列サイズは通常小規模

### DD-4: 外挿モード

**決定**: 補間器に外挿モードを追加（Flat、Linear、Error）。

**理由**:
1. 実務では定義域外クエリが発生することがある
2. 既存の`OutOfBounds`エラーは厳格すぎる場合がある
3. 外挿モードを設定可能にすることで柔軟性を確保
