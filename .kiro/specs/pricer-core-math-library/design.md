# Technical Design: pricer-core-math-library

## 概要

本ドキュメントは、`pricer_core::math`モジュールを拡張し、金融デリバティブ価格計算に必要な包括的な数学ライブラリを実装するための技術設計を定義する。

## 設計原則

1. **AD互換性**: すべての関数は`T: num_traits::Float`でジェネリックに実装し、Enzyme自動微分との互換性を確保
2. **一貫性**: 既存の`pricer_core`パターン（エラーハンドリング、ドキュメント、テスト）を踏襲
3. **段階的実装**: Phase 1（基盤）→ Phase 2（数値計算）→ Phase 3（高度機能）の順で実装
4. **層分離**: L1（pricer_core）は純粋数学、L2（pricer_models）は金融ロジックを維持
5. **外部クレート活用**: AD互換性を損なわない範囲で成熟した外部クレートを活用し、メンテナンス負担を軽減

## 外部クレート活用方針

### 採用判断基準

| 基準 | 重要度 | 説明 |
|------|--------|------|
| AD互換性 | 必須 | `T: Float`ジェネリック対応、またはラッパーで対応可能 |
| 成熟度 | 高 | 十分なテスト、ドキュメント、メンテナンス |
| 依存関係 | 中 | 依存ツリーが軽量 |
| ライセンス | 必須 | MIT/Apache 2.0互換 |

### 採用決定

| モジュール | 方針 | クレート | 理由 |
|-----------|------|---------|------|
| `distributions` | **自前実装** | - | AD互換性必須、statrsは`f64`固定 |
| `integrators` | **自前実装** | - | AD互換性必須、クロージャのジェネリック対応 |
| `calculus` | **自前実装** | - | 単純、外部依存不要 |
| `optimisers` | **argmin採用** | `argmin` | 成熟、豊富なアルゴリズム、AD統合事例あり |
| `linalg` | **nalgebra採用** | `nalgebra` | 成熟、ジェネリック対応、メンテナンス負担軽減 |
| `interpolators` | **自前実装** | - | 金融特化補間（SVI等）は外部になし |
| `solvers` | **自前実装** | - | 既存コードベースに統合済み |
| `fitting` | **nalgebra活用** | `nalgebra` | 最小二乗はnalgebraのSVDを活用 |
| `mesh` | **自前実装** | - | 単純、外部依存不要 |
| `utilities` | **自前実装** | - | 単純、外部依存不要 |

### 参考：調査した外部クレート

| クレート | 用途 | AD互換性 | 採用 |
|----------|------|---------|------|
| [statrs](https://docs.rs/statrs/) | 統計分布 | ❌ `f64`固定 | 不採用 |
| [gauss-quad](https://docs.rs/gauss-quad) | 数値積分 | ❌ `f64`固定 | 不採用 |
| [Peroxide](https://github.com/Axect/Peroxide) | 数値計算全般 | ❌ `f64`固定 | 不採用 |
| [argmin](https://github.com/argmin-rs/argmin) | 最適化 | ⚠️ 部分的 | **採用** |
| [nalgebra](https://github.com/dimforge/nalgebra) | 線形代数 | ✅ ジェネリック | **採用** |

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
├── optimisers/               # 新規モジュール（argminラッパー）
│   ├── mod.rs               # argmin re-export + 統一インターフェース
│   ├── error.rs             # OptimisationError（argminエラーからの変換）
│   ├── config.rs            # 最適化設定
│   └── wrappers.rs          # argminアルゴリズムの薄いラッパー
├── linalg/                   # 新規モジュール（nalgebraラッパー）
│   ├── mod.rs               # nalgebra re-export + 統一インターフェース
│   ├── error.rs             # LinearAlgebraError
│   └── wrappers.rs          # nalgebra機能の薄いラッパー
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

### 4. optimisers モジュール（argminラッパー）

`argmin`クレートを活用し、薄いラッパーで統一インターフェースを提供する。

#### 4.1 設計方針

- `argmin`の豊富なアルゴリズム（L-BFGS、Nelder-Mead、直線探索等）をそのまま活用
- `pricer_core`固有のエラー型への変換レイヤーを提供
- 将来的なAD統合のためのトレイト定義

```rust
// optimisers/mod.rs

//! 最適化アルゴリズム（argminラッパー）
//!
//! このモジュールは`argmin`クレートの薄いラッパーを提供し、
//! `pricer_core`の規約に沿った統一インターフェースを実現する。
//!
//! ## 提供アルゴリズム
//!
//! - **L-BFGS**: `argmin::solver::linesearch::LBFGS`
//! - **Nelder-Mead**: `argmin::solver::neldermead::NelderMead`
//! - **直線探索**: `argmin::solver::linesearch::{BacktrackingLineSearch, MoreThuenteLineSearch}`
//!
//! ## 使用例
//!
//! ```ignore
//! use pricer_core::math::optimisers::{minimize_lbfgs, OptimisationConfig};
//!
//! let result = minimize_lbfgs(
//!     |x| x[0].powi(2) + x[1].powi(2),  // 目的関数
//!     |x| vec![2.0 * x[0], 2.0 * x[1]], // 勾配
//!     vec![1.0, 1.0],                    // 初期値
//!     OptimisationConfig::default(),
//! )?;
//! ```

// argminの主要型をre-export
pub use argmin::core::{CostFunction, Gradient, Executor, State};
pub use argmin::solver::linesearch::condition::ArmijoCondition;
pub use argmin::solver::linesearch::{BacktrackingLineSearch, MoreThuenteLineSearch};
pub use argmin::solver::neldermead::NelderMead;
pub use argmin::solver::quasinewton::LBFGS;

mod config;
mod error;
mod wrappers;

pub use config::OptimisationConfig;
pub use error::OptimisationError;
pub use wrappers::{minimize_lbfgs, minimize_nelder_mead};
```

#### 4.2 エラー型

```rust
// optimisers/error.rs

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum OptimisationError {
    #[error("Optimisation did not converge after {iterations} iterations")]
    NotConverged { iterations: usize },

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Numerical error: {0}")]
    NumericalError(String),

    #[error("Argmin error: {0}")]
    ArgminError(String),
}

impl From<argmin::core::Error> for OptimisationError {
    fn from(err: argmin::core::Error) -> Self {
        OptimisationError::ArgminError(err.to_string())
    }
}
```

#### 4.3 ラッパー関数

```rust
// optimisers/wrappers.rs

use argmin::core::{CostFunction, Gradient, Executor};
use argmin::solver::quasinewton::LBFGS;
use argmin::solver::neldermead::NelderMead;

/// L-BFGSによる最小化（簡易インターフェース）
///
/// # Arguments
/// * `cost` - 目的関数
/// * `gradient` - 勾配関数
/// * `init` - 初期パラメータ
/// * `config` - 最適化設定
pub fn minimize_lbfgs<F, G>(
    cost: F,
    gradient: G,
    init: Vec<f64>,
    config: OptimisationConfig,
) -> Result<OptimisationResult, OptimisationError>
where
    F: Fn(&[f64]) -> f64,
    G: Fn(&[f64]) -> Vec<f64>;

/// Nelder-Meadによる最小化（導関数不要）
///
/// # Arguments
/// * `cost` - 目的関数
/// * `init` - 初期シンプレックス頂点
/// * `config` - 最適化設定
pub fn minimize_nelder_mead<F>(
    cost: F,
    init: Vec<Vec<f64>>,
    config: OptimisationConfig,
) -> Result<OptimisationResult, OptimisationError>
where
    F: Fn(&[f64]) -> f64;
```

#### 4.4 AD統合の将来計画

`argmin`は`argmin-math`を通じて様々な数学バックエンドをサポート。
将来的に`num-dual`との統合により、AD対応の最適化が可能：

```rust
// 将来的なAD対応（計画）
// argmin-math-numdualフィーチャーを使用
pub fn minimize_lbfgs_ad<F>(
    cost: F,
    init: Vec<Dual64>,
    config: OptimisationConfig,
) -> Result<OptimisationResult<Dual64>, OptimisationError>
where
    F: Fn(&[Dual64]) -> Dual64;
```

### 5. linalg モジュール（nalgebraラッパー）

`nalgebra`クレートを活用し、薄いラッパーで統一インターフェースを提供する。

#### 5.1 設計方針

- `nalgebra`の成熟した行列演算・分解機能をそのまま活用
- `nalgebra`は`T: RealField`でジェネリック対応しており、AD互換性あり
- 型エイリアスとユーティリティ関数で使いやすさを向上

```rust
// linalg/mod.rs

//! 線形代数演算（nalgebraラッパー）
//!
//! このモジュールは`nalgebra`クレートの薄いラッパーを提供し、
//! `pricer_core`の規約に沿った統一インターフェースを実現する。
//!
//! ## 提供機能
//!
//! - **行列型**: `DMatrix<T>`, `DVector<T>`（動的サイズ）
//! - **分解**: コレスキー、LU、QR、SVD
//! - **演算**: 行列積、転置、逆行列、行列式
//!
//! ## 使用例
//!
//! ```ignore
//! use pricer_core::math::linalg::{Matrix, cholesky_solve};
//!
//! let a = Matrix::from_row_slice(2, 2, &[4.0, 2.0, 2.0, 3.0]);
//! let b = vec![1.0, 2.0];
//! let x = cholesky_solve(&a, &b)?;
//! ```

// nalgebraの主要型をre-export
pub use nalgebra::{DMatrix, DVector, Matrix as NMatrix, Vector as NVector};
pub use nalgebra::{Cholesky, LU, QR, SVD};
pub use nalgebra::RealField;

mod error;
mod wrappers;

pub use error::LinearAlgebraError;
pub use wrappers::*;

/// 動的サイズ行列の型エイリアス
pub type Matrix<T> = DMatrix<T>;

/// 動的サイズベクトルの型エイリアス
pub type Vector<T> = DVector<T>;
```

#### 5.2 エラー型

```rust
// linalg/error.rs

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum LinearAlgebraError {
    #[error("Matrix is not positive definite")]
    NotPositiveDefinite,

    #[error("Matrix is singular")]
    SingularMatrix,

    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: String, got: String },

    #[error("Matrix is not square: {rows}x{cols}")]
    NotSquare { rows: usize, cols: usize },

    #[error("Decomposition failed: {0}")]
    DecompositionFailed(String),
}
```

#### 5.3 ラッパー関数

```rust
// linalg/wrappers.rs

use nalgebra::{DMatrix, DVector, Cholesky, LU, RealField};

/// コレスキー分解を用いた線形方程式の解法
///
/// A * x = b を解く（A は正定値対称）
///
/// # Arguments
/// * `a` - 正定値対称行列
/// * `b` - 右辺ベクトル
///
/// # Returns
/// 解ベクトル x
pub fn cholesky_solve<T: RealField + Copy>(
    a: &DMatrix<T>,
    b: &[T],
) -> Result<Vec<T>, LinearAlgebraError> {
    let chol = Cholesky::new(a.clone())
        .ok_or(LinearAlgebraError::NotPositiveDefinite)?;
    let b_vec = DVector::from_column_slice(b);
    let x = chol.solve(&b_vec);
    Ok(x.iter().copied().collect())
}

/// LU分解を用いた線形方程式の解法
///
/// A * x = b を解く
pub fn lu_solve<T: RealField + Copy>(
    a: &DMatrix<T>,
    b: &[T],
) -> Result<Vec<T>, LinearAlgebraError> {
    let lu = LU::new(a.clone());
    let b_vec = DVector::from_column_slice(b);
    let x = lu.solve(&b_vec)
        .ok_or(LinearAlgebraError::SingularMatrix)?;
    Ok(x.iter().copied().collect())
}

/// 行列式の計算
pub fn determinant<T: RealField + Copy>(a: &DMatrix<T>) -> Result<T, LinearAlgebraError> {
    if a.nrows() != a.ncols() {
        return Err(LinearAlgebraError::NotSquare {
            rows: a.nrows(),
            cols: a.ncols(),
        });
    }
    Ok(a.determinant())
}

/// 逆行列の計算
pub fn inverse<T: RealField + Copy>(a: &DMatrix<T>) -> Result<DMatrix<T>, LinearAlgebraError> {
    a.clone()
        .try_inverse()
        .ok_or(LinearAlgebraError::SingularMatrix)
}

/// コレスキー分解（LL^T）
///
/// # Returns
/// 下三角行列L such that A = L * L^T
pub fn cholesky<T: RealField + Copy>(
    a: &DMatrix<T>,
) -> Result<DMatrix<T>, LinearAlgebraError> {
    let chol = Cholesky::new(a.clone())
        .ok_or(LinearAlgebraError::NotPositiveDefinite)?;
    Ok(chol.l())
}
```

#### 5.4 AD互換性

`nalgebra`は`T: RealField`トレイト境界を使用しており、`num-dual`の`Dual64`型は
`RealField`を実装している。これによりAD互換性が保証される：

```rust
// AD対応の使用例
use num_dual::Dual64;
use pricer_core::math::linalg::{Matrix, cholesky_solve};

let a: Matrix<Dual64> = Matrix::from_row_slice(2, 2, &[
    Dual64::from(4.0), Dual64::from(2.0),
    Dual64::from(2.0), Dual64::from(3.0),
]);
let b = vec![Dual64::from(1.0), Dual64::from(2.0)];
let x = cholesky_solve(&a, &b)?;  // Dual64でも動作
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

### pricer_core/Cargo.toml 追加

```toml
[dependencies]
# 既存
num-traits = "0.2"
thiserror = "1.0"

# 新規追加
nalgebra = "0.33"          # 線形代数
argmin = "0.10"            # 最適化
argmin-math = { version = "0.4", features = ["nalgebra_latest"] }
```

### 依存関係の理由

| クレート | バージョン | 用途 |
|----------|-----------|------|
| `nalgebra` | 0.33 | 行列演算、コレスキー/LU分解 |
| `argmin` | 0.10 | L-BFGS、Nelder-Mead等の最適化 |
| `argmin-math` | 0.4 | argminとnalgebraの統合 |

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

### DD-3: 線形代数の実装方針（更新）

**決定**: `nalgebra`クレートを採用し、薄いラッパーで統一インターフェースを提供。

**理由**:
1. `nalgebra`は`T: RealField`でジェネリック対応しており、`num-dual::Dual64`と互換
2. 成熟したライブラリでテスト・ドキュメントが充実
3. メンテナンス負担を大幅に軽減
4. コレスキー、LU、QR、SVD等の分解が全て利用可能

**以前の決定（却下）**: 自前実装
- 却下理由: 工数が大きく、`nalgebra`で十分なAD互換性が得られるため

### DD-4: 外挿モード

**決定**: 補間器に外挿モードを追加（Flat、Linear、Error）。

**理由**:
1. 実務では定義域外クエリが発生することがある
2. 既存の`OutOfBounds`エラーは厳格すぎる場合がある
3. 外挿モードを設定可能にすることで柔軟性を確保

### DD-5: 最適化の実装方針

**決定**: `argmin`クレートを採用し、薄いラッパーで統一インターフェースを提供。

**理由**:
1. L-BFGS、Nelder-Mead、直線探索等の豊富なアルゴリズムが実装済み
2. `argmin-math`を通じて`nalgebra`と統合可能
3. `num-dual`との統合事例があり、AD対応の道筋がある
4. 活発にメンテナンスされている

**注意事項**:
- 現時点では`f64`向けのラッパーを提供
- 将来的にAD対応が必要な場合は`argmin-math`のnum-dual統合を検討

### DD-6: 外部クレート不採用の判断

**不採用クレート**:

| クレート | 不採用理由 |
|----------|-----------|
| `statrs` | `f64`固定でジェネリック非対応、AD互換性なし |
| `gauss-quad` | `f64`固定でジェネリック非対応 |
| `Peroxide` | `f64`固定、依存関係が大きい |

**教訓**: 数値計算ライブラリは`f64`固定のものが多く、AD互換性を重視する本プロジェクトでは
分布関数・数値積分は自前実装が必要
