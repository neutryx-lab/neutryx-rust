# Requirements Document

## Project Description (Input)
pricer_core における「車輪の再発明」の無駄
pricer_core/src/math/optimisers（Nelder-Mead, L-BFGS）や solvers（Brent, Newton-Raphson）において、数値計算アルゴリズムを自前で実装しています。

無駄の所在: 高度な最適化アルゴリズムの保守は非常にコストがかかり、バグが入り込みやすい領域です。自前実装は、コミュニティ主導で高度に最適化されたクレート（例: argmin や roots、faer）に比べて、収束速度や数値安定性で劣る傾向があります。

改善による効果:

シンプル化: pricer_core から数千行の複雑なアルゴリズムコードを削除し、外部クレートへのラッパーに置き換えることで、コードベースを大幅に軽量化できます。

強力化: faer（最新のRust線形代数ライブラリ）などを導入することで、SIMD命令をフル活用した現代的なパフォーマンスが得られます。

## Introduction

本仕様は、pricer_core の数値計算モジュール（`optimisers/`、`solvers/`、`linalg/`）を外部クレートベースの実装に移行するための要件を定義します。

現在の pricer_core では、Nelder-Mead、L-BFGS などの最適化アルゴリズムや、Brent、Newton-Raphson などの求根アルゴリズムが自前で実装されています。これらを `argmin`、`roots`、`faer` といったコミュニティ主導の高品質クレートに置き換えることで、以下を達成します：

1. **保守コスト削減**: 複雑なアルゴリズム実装の保守から解放
2. **数値的堅牢性**: プロダクション実績のあるライブラリによる安定性向上
3. **パフォーマンス向上**: SIMD 最適化、コミュニティによる継続的改善の恩恵
4. **コードベース軽量化**: 数千行のアルゴリズムコードを薄いラッパーに置換

## Requirements

### Requirement 1: 最適化アルゴリズムの外部クレート移行

**Objective:** As a quant developer, I want optimisation algorithms backed by the `argmin` crate, so that we benefit from well-tested, performant implementations without maintaining complex code ourselves.

#### Acceptance Criteria
1. When a user calls `minimize_nelder_mead`, the pricer_core shall delegate to `argmin::solver::neldermead::NelderMead`.
2. When a user calls `minimize_lbfgs`, the pricer_core shall delegate to `argmin::solver::linesearch::MoreThuente` combined with L-BFGS solver.
3. When a user calls `minimize_lbfgs_numerical`, the pricer_core shall use argmin's finite-difference gradient approximation.
4. The pricer_core shall maintain the existing public API signatures (`minimize_nelder_mead`, `minimize_lbfgs`, `minimize_lbfgs_numerical`).
5. The pricer_core shall maintain the existing configuration types (`NelderMeadConfig`, `LbfgsConfig`, `OptimisationConfig`) with compatible fields.
6. The pricer_core shall maintain the existing result type (`OptimisationResult`) with compatible fields (optimal parameters, final value, iteration count, convergence status).
7. If argmin returns an error, the pricer_core shall convert it to `OptimisationError` with appropriate error variant.
8. The pricer_core shall remove the self-implemented algorithm logic from `nelder_mead.rs` and `lbfgs.rs`, retaining only wrapper code.

### Requirement 2: 求根アルゴリズムの外部クレート移行

**Objective:** As a quant developer, I want root-finding algorithms backed by the `roots` crate, so that we achieve reliable convergence for implied volatility calculation and curve calibration.

#### Acceptance Criteria
1. When a user calls `BrentSolver::find_root`, the pricer_core shall delegate to `roots::find_root_brent`.
2. When a user calls `BisectionSolver::find_root`, the pricer_core shall delegate to `roots::find_root_bisection`.
3. When a user calls `NewtonRaphsonSolver::find_root`, the pricer_core shall delegate to `roots::find_root_newton_raphson`.
4. The pricer_core shall maintain the existing solver struct APIs (`BrentSolver`, `BisectionSolver`, `NewtonRaphsonSolver`, `BacktrackingNewtonSolver`).
5. The pricer_core shall maintain `SolverConfig` with compatible fields (tolerance, max_iterations).
6. If the `roots` crate does not support backtracking line search, the pricer_core shall retain the self-implemented `BacktrackingNewtonSolver` as a thin wrapper around Newton-Raphson with custom line search.
7. The pricer_core shall remove the self-implemented algorithm logic from `brent.rs`, `bisection.rs`, `newton_raphson.rs`, retaining only wrapper code.
8. If roots returns an error or fails to converge, the pricer_core shall return a descriptive error indicating the failure reason.

### Requirement 3: Levenberg-Marquardt ソルバーの外部クレート移行

**Objective:** As a quant developer, I want Levenberg-Marquardt nonlinear least-squares backed by an external crate, so that model calibration benefits from robust, well-maintained implementations.

#### Acceptance Criteria
1. When a user calls `LevenbergMarquardtSolver::solve`, the pricer_core shall delegate to `argmin::solver::leastsquares::LevenbergMarquardt` or equivalent.
2. The pricer_core shall maintain the existing `LMConfig` and `LMResult` types with compatible fields.
3. The pricer_core shall support Jacobian computation via numerical differentiation when not provided by the user.
4. The pricer_core shall maintain convergence criteria compatibility (residual tolerance, parameter tolerance, max iterations).
5. The pricer_core shall remove the self-implemented LM algorithm from `levenberg_marquardt.rs`, retaining only wrapper code.

### Requirement 4: 線形代数ライブラリの評価と最適化

**Objective:** As a quant developer, I want to evaluate `faer` as a potential replacement or complement to `nalgebra`, so that we can leverage SIMD-optimised linear algebra operations.

#### Acceptance Criteria
1. The pricer_core shall benchmark `nalgebra` vs `faer` for key operations: matrix multiplication, Cholesky decomposition, LU decomposition, SVD.
2. If faer provides ≥20% performance improvement for benchmarked operations, the pricer_core shall provide optional `faer` backend via feature flag.
3. Where faer is adopted, the pricer_core shall maintain the existing `linalg` module API (`cholesky_solve`, `lu_solve`, `qr_solve`, etc.).
4. The pricer_core shall ensure AD compatibility (support for `Dual64` and similar types) is preserved regardless of backend choice.
5. If faer does not support required AD types, the pricer_core shall retain `nalgebra` as the default backend.
6. The pricer_core shall document benchmark results and rationale for backend selection.

### Requirement 5: 自動微分互換性の維持

**Objective:** As a quant developer, I want external crate integrations to preserve automatic differentiation compatibility, so that Enzyme AD and num-dual workflows remain functional.

#### Acceptance Criteria
1. The pricer_core shall ensure all public optimiser and solver functions remain generic over `T: Float` or equivalent AD-compatible bounds.
2. When using argmin, the pricer_core shall implement `ArgminOp` trait for AD-compatible objective functions.
3. When using roots, the pricer_core shall verify that root-finding functions work with `Dual64` types for AD-powered derivative computation.
4. The pricer_core shall maintain the existing `find_root_ad` method on `NewtonRaphsonSolver` that automatically computes derivatives using `Dual64`.
5. If an external crate does not support AD types natively, the pricer_core shall provide a conversion layer or retain self-implemented fallback for AD use cases.

### Requirement 6: 既存テストの互換性と回帰テスト

**Objective:** As a quant developer, I want migration to preserve all existing test cases, so that numerical correctness is verified against known-good results.

#### Acceptance Criteria
1. The pricer_core shall pass all existing unit tests in `optimisers/` and `solvers/` modules after migration.
2. The pricer_core shall add regression tests comparing external crate results against baseline values from self-implemented algorithms.
3. The pricer_core shall verify convergence characteristics (iteration count within 2x of original, final tolerance within 10x) for representative test cases.
4. If any test fails due to numerical differences, the pricer_core shall document the difference and confirm it is an improvement or acceptable variation.
5. The pricer_core shall run calibration integration tests (SABR, Heston, Hull-White calibrators) to verify end-to-end correctness.

### Requirement 7: 依存関係管理とフィーチャーフラグ設計

**Objective:** As a maintainer, I want external numeric crates managed via workspace dependencies with appropriate feature flags, so that build times and binary sizes are controlled.

#### Acceptance Criteria
1. The pricer_core shall add `argmin` and `argmin-math` to workspace dependencies with minimal required features.
2. The pricer_core shall add `roots` to workspace dependencies.
3. Where faer is adopted, the pricer_core shall add `faer` and `faer-ext` to workspace dependencies behind an optional feature flag (`faer-backend`).
4. The pricer_core shall ensure no duplicate compilation of transitive dependencies (verify with `cargo tree --duplicates`).
5. The pricer_core shall gate heavy external dependencies behind feature flags to keep default build minimal.
6. The workspace `Cargo.toml` shall define all external numeric crate versions in `[workspace.dependencies]`.

### Requirement 8: ドキュメントとマイグレーションガイド

**Objective:** As a user of pricer_core, I want clear documentation on the migration, so that I understand any behavioural differences and can update my code if needed.

#### Acceptance Criteria
1. The pricer_core shall update module-level documentation (`//!` comments) in `optimisers/mod.rs` and `solvers/mod.rs` to reflect external crate usage.
2. The pricer_core shall document any behavioural differences (e.g., default tolerance changes, iteration limit changes) in code comments.
3. If API changes are required (deprecated functions, renamed types), the pricer_core shall use `#[deprecated]` attributes with migration guidance.
4. The pricer_core shall update `CHANGELOG.md` with migration notes for the release containing these changes.
