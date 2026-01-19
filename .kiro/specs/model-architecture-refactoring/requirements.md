# Requirements Document

## Project Description (Input)
機能的な重複、不自然さを解消したい。気になるのは、Model(Heston、HullWhiteなど)の扱い。pricer_optimiserはCalibrationを想定していたが、pricer_models内にCalibrationフォルダがある、あとそこらにHestonが散らばっている、modelsがきちんと整理されていないように感じる。分析して抜本的解決を含めて検討し実施したい。

## Introduction

本仕様は、Pricerレイヤーのアーキテクチャを簡素化し、`pricer_optimiser`クレートを廃止してより明確な責務分離を実現するリファクタリングを定義する。

### 現状分析

**問題点1: pricer_optimiserの存在意義が不明確**
- `pricer_optimiser/src/solvers/`: LM, BFGS → `pricer_core/math/solvers/`に既に同等実装あり
- `pricer_optimiser/src/calibration/`: 簡易版 → `pricer_models/calibration/`に充実した実装あり
- `pricer_optimiser/src/bootstrapping/`: Yield Curve構築 → `pricer_core/market_data/`に移動可能

**問題点2: モデル定義の分散**
- `heston.rs`, `sabr.rs`, `gbm.rs`がルートレベルに散在
- `equity/`ディレクトリがほぼ空で未使用
- 株式系モデルと金利系モデルの分離が不完全

**問題点3: LMソルバーの重複**
- `pricer_core/math/solvers/levenberg_marquardt.rs`
- `pricer_optimiser/src/solvers/levenberg_marquardt.rs`
- 2つの独立した実装が存在

**問題点4: 取引定義の配置**
- `pricer_models/src/instruments/`: 取引構造（Swap、Option等）がモデル層に存在
- `pricer_models/src/schedules/`: 支払日計算がモデル層に存在
- これらはモデル（確率過程）とは本質的に異なり、L1に属すべき

### 設計方針

**pricer_optimiserを廃止**し、レイヤー構造を簡素化：

```
L1: pricer_core      → 数学基盤 + マーケットデータ抽象 + Yield Curve構築 + 取引定義
L2: pricer_models    → モデル定義 + キャリブレーション
L3: pricer_pricing   → Monte Carlo + Enzyme AD
L4: pricer_risk      → ポートフォリオ + XVA
```

- **pricer_core (L1)**: ソルバー、補間、Yield Curve bootstrapping、trades（instruments + schedules）
- **pricer_models (L2)**: モデル定義＋キャリブレーション（モデル固有知識が必要なため同居）

---

## Requirements

### Requirement 1: pricer_optimiserの廃止

**Objective:** As a 開発者, I want `pricer_optimiser`クレートを廃止すること, so that レイヤー構造が簡素化され重複が解消される

#### Acceptance Criteria

1. The system shall `pricer_optimiser/src/bootstrapping/`を`pricer_core/src/market_data/bootstrapping/`に移動する

2. The system shall `pricer_optimiser/src/solvers/`を削除し、`pricer_core/math/solvers/`の既存実装を使用する

3. The system shall `pricer_optimiser/src/calibration/`の簡易実装を削除する（pricer_modelsの実装を正とする）

4. The system shall `pricer_optimiser/src/provider.rs`を`pricer_core/src/market_data/provider.rs`に移動する

5. When 移動完了後, the system shall `Cargo.toml`からpricer_optimiserをworkspace membersから削除する

6. The system shall pricer_optimiserに依存していた他クレートの依存関係を更新する

### Requirement 2: モデル構造の整理

**Objective:** As a 開発者, I want モデル定義を論理的なカテゴリで整理すること, so that コードの発見性と理解が向上する

#### Acceptance Criteria

1. The system shall `pricer_models/src/models/`を以下の構造に再編成する:
   ```
   models/
   ├── mod.rs             # re-exports
   ├── traits.rs          # StochasticModel, ModelState traits
   ├── model_enum.rs      # StochasticModelEnum (static dispatch)
   ├── equity/
   │   ├── mod.rs
   │   ├── gbm.rs         # Geometric Brownian Motion
   │   ├── heston.rs      # Heston stochastic volatility
   │   └── sabr.rs        # SABR volatility model
   ├── rates/
   │   ├── mod.rs
   │   ├── hull_white.rs  # Hull-White short rate
   │   └── cir.rs         # Cox-Ingersoll-Ross
   └── hybrid/
       ├── mod.rs
       └── correlated.rs  # Multi-factor models
   ```

2. The system shall 現在ルートレベルにある`heston.rs`, `sabr.rs`, `gbm.rs`を`equity/`に移動する

3. The system shall `mod.rs`から適切なre-exportを維持し、既存の`use pricer_models::models::HestonModel`が動作し続けるようにする

4. When モデルを移動する場合, the system shall 既存のfeature flag構造（`equity`, `rates`等）を維持する

### Requirement 3: キャリブレーション構造の整理

**Objective:** As a 開発者, I want キャリブレーション機能がモデルと同じクレート内で整理されていること, so that モデル固有知識とキャリブレーションが一貫性を持つ

#### Acceptance Criteria

1. The system shall `pricer_models/src/calibration/`を以下の構造に整理する:
   ```
   calibration/
   ├── mod.rs             # re-exports, Calibrator trait
   ├── traits.rs          # Calibrator trait, CalibrationScope, CalibratedParams
   ├── market_data.rs     # OptionSmileData, SwaptionData等
   ├── heston.rs          # HestonCalibrator
   ├── sabr.rs            # SABRCalibrator
   └── hull_white.rs      # HullWhiteCalibrator
   ```

2. The system shall `CalibrationScope`（Global/TermByTerm/Piecewise）を`traits.rs`に定義する

3. When キャリブレーションを実行する場合, the system shall `pricer_core::math::solvers::LevenbergMarquardtSolver`を使用する

4. The system shall 既存の`ModelCalibrator`を`CalibrationEngine`としてリネームし、汎用キャリブレーションエンジンとして機能させる

### Requirement 4: Yield Curve Bootstrappingの移動

**Objective:** As a 開発者, I want Yield Curve構築機能が`pricer_core`に配置されること, so that マーケットデータ構築がL1レイヤーで完結する

#### Acceptance Criteria

1. The system shall `pricer_optimiser/src/bootstrapping/`を`pricer_core/src/market_data/bootstrapping/`に移動する

2. The system shall 移動後のモジュールが以下の構造を持つ:
   ```
   pricer_core/src/market_data/
   ├── mod.rs
   ├── curves/            # (既存) YieldCurve trait, InterpolatedCurve
   ├── surfaces/          # (既存) VolatilitySurface trait
   ├── bootstrapping/     # (新規) Yield Curve構築
   │   ├── mod.rs
   │   ├── engine.rs
   │   ├── curve_builder.rs
   │   └── ...
   └── provider.rs        # (新規) MarketProvider
   ```

3. The system shall bootstrappingモジュールの依存関係を更新し、`pricer_core`内で完結させる

### Requirement 5: 依存関係の整理

**Objective:** As a 開発者, I want クレート間の依存関係が簡素化されたA-I-P-Sアーキテクチャに準拠すること, so that 循環依存やレイヤー違反が発生しない

#### Acceptance Criteria

1. The system shall 以下の依存関係グラフを維持する:
   ```
   pricer_core (L1) ← pricer_models (L2) ← pricer_pricing (L3) ← pricer_risk (L4)
   ```

2. The system shall `pricer_risk`から`pricer_optimiser`への依存を`pricer_core`への依存に変更する

3. The system shall `pricer_pricing`から`pricer_optimiser`への依存（存在する場合）を削除する

4. When ビルドする場合, the system shall `cargo build --workspace`が警告なしで成功することを保証する

5. The system shall `cargo tree`で循環依存がないことを確認する

### Requirement 6: ドキュメントとテストの更新

**Objective:** As a 開発者, I want リファクタリング後のアーキテクチャが文書化されていること, so that 将来の開発者が構造を理解できる

#### Acceptance Criteria

1. The system shall `.kiro/steering/structure.md`を更新し、pricer_optimiserセクションを削除、pricer_coreとpricer_modelsセクションを更新する

2. The system shall `.kiro/steering/tech.md`のレイヤー図を更新し、L2.5を削除する

3. When 既存のテストがある場合, the system shall importパスを更新してテストが継続して動作することを保証する

4. The system shall 各モジュールのdoc commentsを更新し、新しい配置と責務を説明する

5. The system shall CHANGELOG.mdに破壊的変更と移行ガイドを記載する

### Requirement 7: tradesモジュールの新設

**Objective:** As a 開発者, I want 取引定義（instruments, schedules）が`pricer_core`に配置されること, so that キャッシュフロー定義がL1レイヤーで完結し、モデル層との責務が明確に分離される

#### Acceptance Criteria

1. The system shall `pricer_core/src/trades/`を新設し、以下の構造を持つ:
   ```
   pricer_core/src/trades/
   ├── mod.rs             # re-exports
   ├── instruments/       # pricer_modelsから移動
   │   ├── mod.rs
   │   ├── vanilla.rs
   │   ├── forward.rs
   │   ├── swap.rs
   │   ├── equity/
   │   ├── rates/
   │   ├── credit/
   │   ├── fx/
   │   └── ...
   └── schedules/         # pricer_modelsから移動
       ├── mod.rs
       ├── schedule.rs
       ├── period.rs
       └── frequency.rs
   ```

2. The system shall `pricer_models/src/instruments/`を`pricer_core/src/trades/instruments/`に移動する

3. The system shall `pricer_models/src/schedules/`を`pricer_core/src/trades/schedules/`に移動する

4. The system shall `pricer_models`から`pricer_core::trades`をre-exportし、既存の`use pricer_models::instruments::*`が動作し続けるようにする

5. When 移動完了後, the system shall instruments内の`pricer_core`依存（Currency, Date等）がクレート内参照に変更されることを確認する

6. The system shall 既存のfeature flag構造（`equity`, `rates`, `credit`, `fx`, `commodity`, `exotic`）を`pricer_core`に移動する
