# Requirements Document

## Introduction

本仕様書は、デリバティブ価格計算ライブラリ Neutryx における確率過程モデル（Heston、SABR、Hull-White）の完成を定義する。これらのモデルは `pricer_models` クレートの `models/` モジュールに配置され、既存の `StochasticModel` トレイトおよび `StochasticModelEnum` 静的ディスパッチアーキテクチャと統合される。

**対象モデル**:
- **Heston**: 確率的ボラティリティモデル（Equity/FX向け）
- **SABR**: Stochastic Alpha Beta Rho モデル（金利/ボラティリティスマイル向け）
- **Hull-White**: 平均回帰金利モデル（金利デリバティブ向け）

## Project Description (Input)
stochastic-models	Heston, SABR, Hull-White完成

---

## Requirements

### Requirement 1: StochasticModel トレイト統合

**Objective:** As a クオンツ開発者, I want 全確率過程モデルが統一された `StochasticModel` トレイトを実装すること, so that モンテカルロエンジンで一貫したインターフェースでパス生成できる。

#### Acceptance Criteria

1. The Heston Model shall implement the `StochasticModel<T>` trait with `evolve_step`, `initial_state`, and `brownian_dim` methods.
2. The SABR Model shall implement the `StochasticModel<T>` trait with `evolve_step`, `initial_state`, and `brownian_dim` methods.
3. The Hull-White Model shall implement the `StochasticModel<T>` trait with `evolve_step`, `initial_state`, and `brownian_dim` methods.
4. The `StochasticModelEnum` shall include variants for Heston, SABR, and Hull-White models for static dispatch.
5. When a `StochasticModelEnum` is used, the Pricer shall delegate method calls to the underlying concrete model without dynamic dispatch overhead.

---

### Requirement 2: Heston モデル実装

**Objective:** As a クオンツ開発者, I want Heston 確率的ボラティリティモデルを使用すること, so that ボラティリティスマイルを内生的に生成するエクイティ・FX デリバティブを価格計算できる。

#### Acceptance Criteria

1. The Heston Model shall accept parameters: initial variance (v0), long-term variance (theta), mean reversion speed (kappa), volatility of volatility (xi), and correlation (rho).
2. The Heston Model shall maintain a two-dimensional state vector: [spot price, variance].
3. When `evolve_step` is called, the Heston Model shall evolve both spot price and variance using the Euler-Maruyama scheme.
4. The Heston Model shall return `brownian_dim() = 2` to indicate two correlated Brownian motions.
5. While variance becomes negative during simulation, the Heston Model shall apply a reflection or absorption scheme to maintain non-negativity.
6. The Heston Model shall be generic over `T: Float` for AD compatibility with Enzyme and num-dual.
7. The Heston Model shall be placed in `pricer_models/src/models/equity/` with feature flag `equity`.

---

### Requirement 3: SABR モデル実装

**Objective:** As a クオンツ開発者, I want SABR モデルを使用すること, so that 金利・FX 市場のボラティリティスマイルをモデル化できる。

#### Acceptance Criteria

1. The SABR Model shall accept parameters: alpha (initial volatility), beta (CEV exponent), rho (correlation), and nu (volatility of volatility).
2. The SABR Model shall maintain a two-dimensional state vector: [forward rate, volatility].
3. When `evolve_step` is called, the SABR Model shall evolve both forward rate and volatility using the appropriate discretisation scheme.
4. The SABR Model shall return `brownian_dim() = 2` to indicate two correlated Brownian motions.
5. Where beta equals 0, the SABR Model shall reduce to normal SABR dynamics.
6. Where beta equals 1, the SABR Model shall reduce to lognormal SABR dynamics.
7. The SABR Model shall be generic over `T: Float` for AD compatibility with Enzyme and num-dual.
8. The SABR Model shall be placed in `pricer_models/src/models/rates/` with feature flag `rates`.

---

### Requirement 4: Hull-White モデル実装

**Objective:** As a クオンツ開発者, I want Hull-White 平均回帰金利モデルを使用すること, so that 金利デリバティブの価格計算とヘッジができる。

#### Acceptance Criteria

1. The Hull-White Model shall accept parameters: mean reversion speed (a), volatility (sigma), and initial short rate (r0).
2. The Hull-White Model shall maintain a one-dimensional state: short rate (r).
3. When `evolve_step` is called, the Hull-White Model shall evolve the short rate using the Euler-Maruyama scheme with mean reversion.
4. The Hull-White Model shall return `brownian_dim() = 1` to indicate one Brownian motion.
5. The Hull-White Model shall support time-dependent theta function θ(t) for yield curve fitting.
6. The Hull-White Model shall be generic over `T: Float` for AD compatibility with Enzyme and num-dual.
7. The Hull-White Model shall be placed in `pricer_models/src/models/rates/` with feature flag `rates`.

---

### Requirement 5: AD 互換性要件

**Objective:** As a リスクエンジニア, I want 全モデルが自動微分と互換であること, so that Greeks（感応度）を効率的に計算できる。

#### Acceptance Criteria

1. The stochastic models shall use smooth approximations (from `pricer_core::math::smoothing`) instead of branching conditions in hot paths.
2. The stochastic models shall avoid dynamic allocation in `evolve_step` for Enzyme compatibility.
3. When used with Enzyme AD, the stochastic models shall produce correct gradients for all model parameters.
4. When used with num-dual, the stochastic models shall produce gradients that match Enzyme results within numerical tolerance.
5. The stochastic models shall use fixed-size arrays or tuples for state vectors instead of `Vec<T>`.

---

### Requirement 6: キャリブレーションインターフェース

**Objective:** As a クオンツ開発者, I want モデルパラメータをマーケットデータにキャリブレーションすること, so that 市場インプライドボラティリティに整合するモデルを構築できる。

#### Acceptance Criteria

1. The Heston Model shall provide a `calibrate` associated function that accepts vanilla option prices and returns calibrated parameters.
2. The SABR Model shall provide a `calibrate` associated function that accepts ATM volatility and smile data and returns calibrated parameters.
3. The Hull-White Model shall provide a `calibrate` associated function that accepts swaption volatilities and returns calibrated parameters.
4. When calibration fails due to non-convergence, the models shall return a descriptive `CalibrationError`.
5. The calibration interfaces shall integrate with `pricer_optimiser` solvers (Levenberg-Marquardt).

---

### Requirement 7: 単体テストおよび検証

**Objective:** As a 開発者, I want 各モデルが十分にテストされていること, so that 実装の正確性を保証できる。

#### Acceptance Criteria

1. The Heston Model shall include unit tests verifying moment matching (mean and variance of simulated paths).
2. The SABR Model shall include unit tests verifying the Hagan et al. analytical approximation for implied volatility.
3. The Hull-White Model shall include unit tests verifying bond price recovery from the analytical formula.
4. The stochastic models shall include verification tests comparing Enzyme and num-dual gradients.
5. When model parameters are at boundary values, the models shall handle edge cases without panicking.

---

## Non-Functional Requirements

### NFR 1: パフォーマンス
- The `evolve_step` method shall execute in O(1) time complexity.
- The models shall support vectorised operations for batch path generation.

### NFR 2: コード品質
- The models shall follow British English spelling conventions (e.g., `optimiser`, `serialisation`).
- The models shall pass `cargo clippy --all-targets -- -D warnings`.
- The models shall include documentation comments for all public APIs.

### NFR 3: Feature Flag 構成
- The Heston model shall be gated by the `equity` feature flag.
- The SABR and Hull-White models shall be gated by the `rates` feature flag.
- All models shall be included when the `all` convenience feature is enabled.
