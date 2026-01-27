# Requirements Document

## Introduction

本仕様書は、カーブ構築を行列計算（連立方程式）として解き、AAD（Adjoint Algorithmic Differentiation）のJacobian逆行列アプローチと整合させるグローバル・カーブビルダーの要件を定義する。

従来の逐次的なブートストラップ（1商品ずつ解く方式）に代わり、グローバル・ソルバー（全期間同時推定）として実装することで、すべての観測商品がすべてのカーブパラメータに依存する構造をJacobianとして明示的に扱う。これにより、Implicit Function Theorem（陰関数定理）を用いたAADの高速化が可能となる。

### 実装レイヤー構成

A-I-P-Sアーキテクチャに従い、以下の3レイヤーで構成する：

1. **Math Layer (`pricer_core`)**: 金融ロジックに依存しない汎用的な多次元Newton-Raphsonソルバー
2. **Model Logic (`pricer_models`)**: カーブ構築問題を残差関数 F(x) = 0 として定義するアダプター
3. **AAD Integration (`pricer_risk`)**: Implicit Function Theoremを用いたカスタム勾配ルール

## Requirements

### Requirement 1: 多次元Newton-Raphsonソルバー

**Objective:** As a クォンツ開発者, I want 汎用的な多次元Newton-Raphsonソルバー, so that カーブ構築やモデル較正など様々な連立方程式問題を統一的に解くことができる

#### Acceptance Criteria

1. The `MultidimensionalSolver` shall accept an initial guess vector x₀, a residual function F(x), and a Jacobian function J(x) as inputs.
2. When the solver converges within the specified tolerance, the `MultidimensionalSolver` shall return the solution vector x* and the Jacobian inverse matrix J⁻¹ at the convergence point.
3. The `MultidimensionalSolver` shall implement the Newton-Raphson iteration: x_{k+1} = x_k - J(x_k)⁻¹ F(x_k).
4. If the solver fails to converge within the maximum iteration count, the `MultidimensionalSolver` shall return a `SolverError::MaxIterationsExceeded` error with diagnostic information.
5. If the Jacobian matrix is singular or near-singular, the `MultidimensionalSolver` shall return a `SolverError::SingularJacobian` error.
6. The `MultidimensionalSolver` shall support configurable convergence criteria including absolute tolerance, relative tolerance, and maximum iterations.
7. The `MultidimensionalSolver` shall be generic over the Float type `T` to support both `f64` and AD-enabled numeric types.

### Requirement 2: SystemOfEquations トレイト

**Objective:** As a クォンツ開発者, I want 連立方程式問題を抽象化するトレイト, so that 異なる較正問題を統一的なインターフェースで扱える

#### Acceptance Criteria

1. The `SystemOfEquations` trait shall define an `evaluate(&self, x: &Array1<T>) -> Result<Array1<T>, SolverError>` method for computing the residual vector.
2. The `SystemOfEquations` trait shall define a `jacobian(&self, x: &Array1<T>) -> Result<Array2<T>, SolverError>` method for computing the Jacobian matrix.
3. The `SystemOfEquations` trait shall define a `dimension(&self) -> usize` method returning the problem dimension.
4. Where a problem supports analytical Jacobian, the implementing struct shall provide an exact Jacobian computation.
5. Where a problem does not support analytical Jacobian, the `SystemOfEquations` trait shall provide a default numerical Jacobian via finite differences.

### Requirement 3: SolverResult構造体

**Objective:** As a クォンツ開発者, I want ソルバー結果をAAD用に拡張された形式で取得, so that 陰関数定理による効率的な勾配計算が可能になる

#### Acceptance Criteria

1. The `SolverResult` struct shall contain the solution vector `x*` as `Array1<T>`.
2. The `SolverResult` struct shall contain the Jacobian inverse matrix `J⁻¹` at the convergence point as `Array2<T>`.
3. The `SolverResult` struct shall contain the number of iterations taken to converge.
4. The `SolverResult` struct shall contain the final residual norm for diagnostic purposes.
5. Where memory efficiency is required, the `SolverResult` struct shall optionally store LU factorization instead of the explicit inverse matrix.

### Requirement 4: カーブ較正問題定義

**Objective:** As a クォンツ開発者, I want カーブ構築問題をSystemOfEquationsとして定義, so that グローバル・ソルバーでイールドカーブを較正できる

#### Acceptance Criteria

1. The `CurveCalibrationProblem` struct shall implement the `SystemOfEquations` trait.
2. The `CurveCalibrationProblem` shall accept a list of calibration instruments (Deposit, FRA, Futures, Swap, etc.) and their market quotes.
3. When `evaluate` is called, the `CurveCalibrationProblem` shall compute the difference between model prices and market quotes for all instruments.
4. When `jacobian` is called, the `CurveCalibrationProblem` shall compute the sensitivity of each instrument price to each curve parameter.
5. The `CurveCalibrationProblem` shall support configurable curve parameterisation (Zero Rate, Discount Factor log, Instantaneous Forward).
6. The `CurveCalibrationProblem` shall support multiple interpolation methods for the constructed curve (Linear, LogLinear, MonotonicCubic).

### Requirement 5: 商品価格計算インターフェース

**Objective:** As a クォンツ開発者, I want 較正商品の統一的な価格計算インターフェース, so that 新規商品を追加する際に容易に拡張できる

#### Acceptance Criteria

1. The `CalibrationInstrument` trait shall define a `price<T: Float>(&self, curve: &Curve<T>) -> Result<T, PricingError>` method.
2. The `CalibrationInstrument` trait shall define a `par_rate<T: Float>(&self, curve: &Curve<T>) -> Result<T, PricingError>` method for instruments quoted as rates.
3. The `CalibrationInstrument` trait shall define a `maturity(&self) -> Date` method returning the instrument's maturity date.
4. The implementation shall support Deposit, FRA, Futures, and IRS instruments at minimum.
5. Where the instrument requires multiple curves (e.g., dual-curve framework), the `CalibrationInstrument` shall accept a `CurveSet` containing discount and projection curves.

### Requirement 6: AAD陰関数定理統合

**Objective:** As a クォンツ開発者, I want ソルバーの勾配をImplicit Function Theoremで計算, so that AADによる高速な市場リスク計算が可能になる

#### Acceptance Criteria

1. When reverse-mode AAD is applied to the solver, the `ImplicitSolver` shall use the Implicit Function Theorem instead of differentiating through iterations.
2. The `ImplicitSolver` shall compute the adjoint as: ∂L/∂m = J⁻ᵀ · ∂L/∂x*, where m is market parameters and x* is the solution.
3. The `ImplicitSolver` shall reuse the Jacobian inverse stored in `SolverResult` for adjoint computation.
4. The `ImplicitSolver` shall support Enzyme AD custom gradient rules via the `#[enzyme_rules]` attribute or equivalent mechanism.
5. If Enzyme is not available, the `ImplicitSolver` shall fall back to numerical differentiation with appropriate warning.

### Requirement 7: 線形代数演算

**Objective:** As a クォンツ開発者, I want 数値安定な線形代数演算, so that ソルバーが悪条件問題でも安定して動作する

#### Acceptance Criteria

1. The linear algebra module shall provide LU decomposition with partial pivoting for solving linear systems.
2. The linear algebra module shall provide matrix inversion via LU factorisation.
3. If the matrix is detected as singular (pivot below threshold), the linear algebra module shall return `LinalgError::SingularMatrix`.
4. The linear algebra module shall support both dense matrices (`Array2<T>`) and sparse matrices for large-scale problems.
5. The linear algebra module shall be compatible with `ndarray` and optionally `ndarray-linalg` (LAPACK backend).

### Requirement 8: GlobalBootstrapper統合

**Objective:** As a クォンツ開発者, I want 既存のブートストラップフレームワークとの互換性, so that 段階的に移行できる

#### Acceptance Criteria

1. The `GlobalBootstrapper` shall provide the same public API as the existing sequential bootstrapper.
2. When `build_curve` is called, the `GlobalBootstrapper` shall use the global solver internally while returning a compatible `Curve` object.
3. The `GlobalBootstrapper` shall support the existing `BootstrapConfig` for curve construction settings.
4. Where backward compatibility is required, the `GlobalBootstrapper` shall be selectable via feature flag or configuration option.
5. The `GlobalBootstrapper` shall expose the Jacobian inverse for downstream AAD calculations via an optional method.

### Requirement 9: エラーハンドリング

**Objective:** As a クォンツ開発者, I want 構造化されたエラー型, so that 問題診断と適切なリカバリーが可能になる

#### Acceptance Criteria

1. The `SolverError` enum shall include variants for: `MaxIterationsExceeded`, `SingularJacobian`, `NumericalInstability`, `DimensionMismatch`.
2. The `SolverError::MaxIterationsExceeded` variant shall contain the iteration count and final residual norm.
3. The `SolverError::SingularJacobian` variant shall contain the smallest pivot value encountered.
4. The `SolverError` shall implement `thiserror::Error` for proper error propagation.
5. The `SolverError` shall implement `Clone`, `Debug`, and `PartialEq` for testability.

### Requirement 10: パフォーマンス要件

**Objective:** As a トレーディングデスク担当者, I want 高速なカーブ較正, so that リアルタイムプライシングワークフローに組み込める

#### Acceptance Criteria

1. The `GlobalSolver` shall converge within 10 iterations for well-conditioned standard yield curve problems (30 instruments or fewer).
2. The solver computation time shall not exceed 10ms for a standard 30-instrument USD SOFR curve on a single core.
3. The Jacobian computation shall support parallelisation via Rayon for large instrument sets.
4. While computing Greeks via AAD, the `ImplicitSolver` shall achieve at least 5x speedup compared to bump-and-revalue for portfolios with 100+ curve dependencies.
5. The memory footprint for storing Jacobian inverse shall not exceed 8 * n² bytes where n is the number of curve parameters.

### Requirement 11: テスト要件

**Objective:** As a クォンツ開発者, I want 包括的なテストスイート, so that 実装の正確性と安定性を検証できる

#### Acceptance Criteria

1. The solver shall include unit tests verifying convergence for standard test functions (Rosenbrock, quadratic systems).
2. The solver shall include integration tests comparing global solver results against sequential bootstrap for identical market data.
3. The AAD integration shall include verification tests comparing implicit differentiation against finite differences.
4. The solver shall include property-based tests using `proptest` for numerical stability across random inputs.
5. The solver shall include benchmark tests using `criterion` for performance regression tracking.

## Project Description (Input)

「カーブ構築を行列計算（連立方程式）として解き、かつAAD（Adjoint Algorithmic Differentiation）のJacobian逆行列アプローチと整合させる」

このアプローチは、逐次的なブートストラップ（1期間ずつ解く）ではなく、**グローバル・ソルバー（全期間同時推定）**として実装することを意味します。これにより、すべての観測商品がすべてのカーブパラメータに依存する構造をJacobianとして明示的に扱うことができ、Implicit Function Theorem（陰関数定理）を用いたAADの高速化が可能になります。

### 1. 実装の全体像

実装は主に以下の3つのレイヤーに分割するべきです。

1. **Math Layer (`pricer_core`)**: 金融ロジックに依存しない、汎用的な「多次元ニュートン・ラフソン法」ソルバー。ここでJacobianの逆行列（またはLU分解）を扱います。
2. **Model Logic (`pricer_models`)**: カーブ構築問題を  という形式の残差関数として定義するアダプター。
3. **AAD Integration (`pricer_risk` / Enzyme)**: ソルバーの逆伝播を定義するカスタム微分ルール（Custom Gradient）。
