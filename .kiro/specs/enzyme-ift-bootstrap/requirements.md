# Requirements Document

## Project Description (Input)
数値的安定性と AAD への最適化pricer_models/src/builder/curve/global.rs では、全ピラーを同時に解く Global Bootstrapping が実装されており、Jacobian 逆行列の保存による感度計算が企図されています。課題：JacobianMethod::Analytical の未成熟problem.rs に JacobianMethod::Analytical の定義がありますが、イテレーションごとの解析的ヤコビアンの計算ロジックが、内挿手法（Interpolation Scheme）の微分と密結合していません。洗練化案：Enzyme AD への完全移行と Implicit Function Theorem の実装Kentaro氏が重視する Enzyme AD を CalibrationProblem に適用し、有限差分法を廃止すべきです。具体的には、ソルバーの収束解 $x^*$ において、Implicit Function Theorem (IFT) を用いた感度抽出を行いますが、この際 $J^{-1}$ を GlobalBootstrapper にキャッシュする現在の設計 は非常に合理的です。これを、pricer_core の線形代数バックエンドと密結合させ、LUStrategy 以外の疎行列向けアルゴリズムへの切り替えを容易にする必要があります。

## Introduction

本仕様書は、Neutryx の Global Curve Bootstrapping における Enzyme AD 統合と Implicit Function Theorem (IFT) 実装のための要件を定義します。現在の有限差分法による Jacobian 計算を Enzyme AD に置き換え、収束解における感度抽出を IFT により効率化します。また、pricer_core の線形代数バックエンドとの統合により、疎行列向けアルゴリズムへの拡張性を確保します。

## Requirements

### Requirement 1: Enzyme AD による Jacobian 計算

**Objective:** As a quantitative developer, I want the CalibrationProblem to compute Jacobians using Enzyme AD instead of finite differences, so that I can achieve machine-precision sensitivities with improved computational efficiency.

#### Acceptance Criteria

1. When JacobianMethod::AutomaticDifferentiation is selected, the CalibrationProblem shall compute the Jacobian matrix using Enzyme AD reverse mode differentiation.
2. The Enzyme AD-computed Jacobian shall produce results within 1e-12 relative tolerance compared to analytical derivatives for polynomial interpolation schemes.
3. While the `enzyme-ad` feature flag is enabled, the CalibrationProblem shall automatically select Enzyme AD as the default Jacobian method.
4. If Enzyme AD computation fails due to unsupported operations, the CalibrationProblem shall fall back to finite difference method and log a warning.
5. The Enzyme AD Jacobian computation shall support all interpolation methods defined in BootstrapInterpolation enum (Flat, Linear, LogLinear, MonotonicCubic, NaturalCubicSpline).

### Requirement 2: 内挿スキームの微分可能実装

**Objective:** As a quantitative developer, I want interpolation schemes to provide analytical derivatives tightly coupled with the Jacobian computation, so that Enzyme AD can efficiently differentiate through the entire curve building process.

#### Acceptance Criteria

1. The BootstrappedCurve shall implement a `discount_factor_with_gradient` method that returns both the discount factor and its gradient with respect to pillar values.
2. When using LogLinear interpolation, the BootstrappedCurve shall compute exact analytical derivatives: ∂DF(t)/∂DF_i for all pillar indices i.
3. The InterpolatorEnum shall implement the Enzyme `#[autodiff]` compatible interface for all supported interpolation methods.
4. While computing residuals, the CalibrationProblem shall propagate AD dual numbers through the interpolation layer without numerical truncation.
5. If an interpolation method does not support analytical differentiation, the system shall raise a compile-time error when used with JacobianMethod::AutomaticDifferentiation.

### Requirement 3: Implicit Function Theorem (IFT) 感度抽出

**Objective:** As a risk manager, I want to extract market sensitivities from the calibrated curve using the Implicit Function Theorem, so that I can compute Greeks with O(1) additional cost after calibration.

#### Acceptance Criteria

1. The GlobalBootstrapResult shall provide an `ift_sensitivity` method that computes ∂x*/∂m = -J⁻¹ · ∂F/∂m for any market parameter m.
2. When `store_jacobian_inverse` is enabled, the GlobalBootstrapper shall cache J⁻¹ at the solution point for subsequent IFT computations.
3. The IFT sensitivity computation shall support batch market parameter perturbations, computing sensitivities to multiple parameters in a single matrix-vector operation.
4. If the Jacobian inverse is not cached, the `ift_sensitivity` method shall return an error indicating that recalibration with `store_jacobian_inverse=true` is required.
5. The IFT-based sensitivity shall agree with bump-and-recalibrate sensitivities within 1e-8 relative tolerance for standard OIS instruments.

### Requirement 4: LinearSolveStrategy 拡張

**Objective:** As a library developer, I want the LinearSolveStrategy trait to support sparse matrix algorithms beyond LU decomposition, so that large-scale curve calibration problems can be solved efficiently.

#### Acceptance Criteria

1. The LinearSolveStrategy trait shall be extended with a `sparse_solve` method for Compressed Sparse Row (CSR) matrices.
2. The pricer_core::math::linalg module shall provide a SparseCholeskyStrategy implementing LinearSolveStrategy for symmetric positive definite sparse Jacobians.
3. When the Jacobian matrix has sparsity exceeding 70%, the GlobalBootstrapper shall automatically select a sparse solver strategy.
4. The SparseStrategy shall store the sparse LU factorisation for efficient repeated solves during Newton iterations.
5. While using sparse strategies, the Jacobian inverse computation shall use iterative methods (e.g., GMRES) with configurable tolerance rather than explicit inversion.

### Requirement 5: 数値安定性保証

**Objective:** As a quantitative analyst, I want the calibration process to maintain numerical stability across all Jacobian computation methods, so that curve construction remains robust for ill-conditioned market data.

#### Acceptance Criteria

1. The GlobalBootstrapper shall monitor the condition number of the Jacobian matrix and warn if it exceeds the configured `max_condition_number` threshold.
2. When condition number exceeds 1e10, the GlobalBootstrapper shall automatically apply Tikhonov regularisation with configurable damping parameter.
3. The CalibrationProblem shall implement a `validate_jacobian_quality` method that checks for NaN, Inf, and near-zero diagonal elements.
4. If Enzyme AD produces unstable gradients (variance > 1e6 compared to finite differences), the system shall automatically switch to central differences.
5. The GlobalBootstrapResult shall include a `numerical_diagnostics` field reporting condition number, residual norm history, and any regularisation applied.

### Requirement 6: 設定とフィーチャーフラグ統合

**Objective:** As an integration engineer, I want Enzyme AD calibration to be feature-gated and configurable, so that production deployments can choose between stable finite differences and experimental AD methods.

#### Acceptance Criteria

1. The `enzyme-ad` feature flag shall control availability of JacobianMethod::AutomaticDifferentiation at compile time.
2. When `enzyme-ad` is disabled, the GlobalBootstrapConfig shall not expose Enzyme-related configuration options.
3. The GlobalBootstrapConfig shall provide a `with_jacobian_method(JacobianMethod)` builder method that validates compatibility with enabled features.
4. If JacobianMethod::AutomaticDifferentiation is requested without the `enzyme-ad` feature, the configuration shall return a compile-time error.
5. The CalibrationProblemConfig shall include an `ad_checkpoint_interval` parameter for memory-efficient reverse-mode AD on large problems.

### Requirement 7: pricer_risk AAD Binder 統合

**Objective:** As a risk system developer, I want the IFT-based sensitivities to integrate with the pricer_risk AAD binder layer, so that market risk calculations can leverage cached Jacobian inverses.

#### Acceptance Criteria

1. The pricer_risk::enzyme::binder module shall accept GlobalBootstrapResult as input for curve sensitivity propagation.
2. When computing portfolio Greeks, the AAD binder shall use the cached J⁻¹ from GlobalBootstrapResult instead of recalibrating.
3. The ShadowObject trait shall be implemented for GlobalBootstrapResult to enable reverse-mode gradient accumulation.
4. While processing a portfolio of trades, the AAD binder shall batch curve sensitivities across all trades sharing the same calibrated curve.
5. If the GlobalBootstrapResult lacks a cached Jacobian inverse, the AAD binder shall transparently perform on-demand calibration with `store_jacobian_inverse=true`.
