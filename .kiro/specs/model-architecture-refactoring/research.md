# Research & Design Decisions

## Summary
- **Feature**: model-architecture-refactoring
- **Discovery Scope**: Complex Integration（pricer_optimiser廃止 + 複数モジュール再編成）
- **Key Findings**:
  - pricer_optimiserの機能は pricer_core と pricer_models に分散可能
  - LMソルバーは pricer_core に統一すべき
  - trades（instruments + schedules）は pricer_core に属すべき

## Research Log

### pricer_optimiser の機能分析

- **Context**: pricer_optimiser (L2.5) の存在意義を検証
- **Sources Consulted**:
  - `crates/pricer_optimiser/src/lib.rs`
  - `crates/pricer_optimiser/src/bootstrapping/mod.rs`
  - `crates/pricer_optimiser/src/solvers/mod.rs`
- **Findings**:
  - **bootstrapping/** (12サブモジュール): Yield Curve構築、AAD対応、Multi-curve framework
  - **solvers/** (LM, BFGS): pricer_core にも LM 実装あり（重複）
  - **calibration/** (211行): pricer_models に 4,081行の充実した実装あり
  - **provider.rs**: MarketProvider（キャッシュ層）
- **Implications**:
  - bootstrapping → pricer_core/market_data/
  - solvers → 削除（pricer_core を使用）
  - calibration → 削除（pricer_models を正とする）
  - provider → pricer_core/market_data/

### LMソルバーの重複調査

- **Context**: 2つの LM 実装の比較
- **Sources Consulted**:
  - `crates/pricer_core/src/math/solvers/levenberg_marquardt.rs`
  - `crates/pricer_optimiser/src/solvers/levenberg_marquardt.rs`
- **Findings**:
  - **pricer_core版**: `LevenbergMarquardtSolver`, `LMConfig`, `LMResult`
  - **pricer_optimiser版**: `LevenbergMarquardt`, `LevenbergMarquardtConfig`, `OptimisationResult`
  - 両者とも非線形最小二乗法を実装、APIが若干異なる
- **Implications**: pricer_core版を正とし、pricer_optimiser版は削除

### モデル構造の現状

- **Context**: models/ ディレクトリの整理状況
- **Sources Consulted**: `crates/pricer_models/src/models/`
- **Findings**:
  - **ルートレベル**: `gbm.rs`, `heston.rs`, `sabr.rs`, `stochastic.rs`, `model_enum.rs`
  - **rates/**: `hull_white.rs`, `cir.rs`（feature-gated）
  - **equity/**: 空に近い（re-exportのみ）
  - **hybrid/**: `correlated.rs`（feature-gated）
- **Implications**: 株式系モデル（GBM, Heston, SABR）を equity/ に移動

### キャリブレーション構造の現状

- **Context**: calibration/ の責務と依存関係
- **Sources Consulted**: `crates/pricer_models/src/calibration/mod.rs`
- **Findings**:
  - **モデル固有**: `HestonCalibrator`, `SABRCalibrator`, `HullWhiteCalibrator`
  - **汎用**: `ModelCalibrator`, `SwaptionCalibrator`
  - **サポート**: `CalibrationTarget`, `CalibrationResult`, `CalibrationDiagnostics`
  - ソルバー依存: `pricer_core::math::solvers` または `pricer_optimiser::solvers`
- **Implications**: pricer_core のソルバーのみを使用するよう統一

### trades（instruments + schedules）の分析

- **Context**: instruments と schedules の配置検討
- **Sources Consulted**:
  - `crates/pricer_models/src/instruments/mod.rs`
  - `crates/pricer_models/src/schedules/mod.rs`
- **Findings**:
  - **instruments** (24ファイル):
    - 依存: `pricer_core::types::Currency`, `num_traits::Float`
    - 責務: キャッシュフロー構造の定義（Payoff, Exercise, Forward, Swap等）
  - **schedules** (5ファイル):
    - 依存: `pricer_core::types::time::Date`
    - 責務: 支払日計算ロジック
  - 両者ともモデル（確率過程）とは本質的に無関係
- **Implications**:
  - trades はキャッシュフローの「何を・いつ」を定義
  - models は確率過程の「どう動くか」を定義
  - 責務が明確に異なるため、trades は L1 (pricer_core) に属すべき

### 依存関係の分析

- **Context**: クレート間の依存グラフ
- **Sources Consulted**: 各 Cargo.toml
- **Findings**:
  - **pricer_risk → pricer_optimiser**: bootstrapping, calibration 使用
  - **pricer_optimiser → pricer_core, pricer_models**: 型定義、トレイト
  - **pricer_models → pricer_core**: Currency, Date, Float
- **Implications**:
  - pricer_optimiser 廃止後、pricer_risk は pricer_core（bootstrapping）と pricer_models（calibration）に依存
  - 循環依存なし

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 現状維持 (L2.5) | pricer_optimiser を維持 | 変更なし | 重複継続、責務不明確 | 却下 |
| 完全統合 | 全機能を pricer_core に | シンプル | pricer_core が肥大化 | 却下 |
| **責務分離** | bootstrapping→core, calibration→models, trades→core | 明確な責務、重複解消 | 移行コスト | **採用** |

## Design Decisions

### Decision: pricer_optimiser の廃止

- **Context**: L2.5 レイヤーの存在意義が不明確
- **Alternatives Considered**:
  1. 現状維持 — 重複と不明確な責務が継続
  2. 完全統合 — pricer_core が肥大化
  3. 責務分離 — 機能ごとに適切なレイヤーに配置
- **Selected Approach**: 責務分離（Option 3）
- **Rationale**:
  - bootstrapping はマーケットデータ構築であり L1 に属する
  - calibration はモデル固有知識を要するため L2 に属する
  - solvers は純粋な数値計算であり L1 に属する
- **Trade-offs**:
  - 利点: 明確な責務、重複解消、依存関係の簡素化
  - 欠点: 移行作業、既存コードの import 変更
- **Follow-up**: re-export による後方互換性の維持

### Decision: trades モジュールの新設

- **Context**: instruments と schedules がモデル層に存在
- **Alternatives Considered**:
  1. 現状維持 — 責務の混在が継続
  2. pricer_core/trades/ に移動 — キャッシュフロー定義を L1 に集約
- **Selected Approach**: pricer_core/trades/ に移動
- **Rationale**:
  - trades = 精緻なキャッシュフロー定義（いつ、いくら、どの通貨で）
  - models = 確率過程の定義（どう動くか）
  - 責務が本質的に異なる
- **Trade-offs**:
  - 利点: 責務の明確化、pricer_models の純粋化
  - 欠点: feature flags の移動、import パスの変更
- **Follow-up**: pricer_models から re-export して後方互換性を維持

### Decision: LMソルバーの統一

- **Context**: 2つの独立した LM 実装が存在
- **Alternatives Considered**:
  1. pricer_core版を使用 — 既存の API を維持
  2. pricer_optimiser版を使用 — 削除予定クレートに依存
  3. 新規実装 — 不要な作業
- **Selected Approach**: pricer_core版を使用
- **Rationale**: pricer_optimiser は廃止予定、pricer_core版で十分
- **Trade-offs**: calibration の import パス変更が必要
- **Follow-up**: pricer_models/calibration が pricer_core::math::solvers を使用するよう更新

## Risks & Mitigations

- **Risk 1**: 既存の import パスが破壊される
  - **Mitigation**: pricer_models から re-export して後方互換性を維持
- **Risk 2**: feature flags の移動で機能が欠落
  - **Mitigation**: テストで全 feature 組み合わせを検証
- **Risk 3**: bootstrapping の pricer_models 依存が循環を生む
  - **Mitigation**: bootstrapping は pricer_core 内で完結するよう依存を整理

## References

- [Rust API Guidelines - Re-exports](https://rust-lang.github.io/api-guidelines/interoperability.html)
- A-I-P-S アーキテクチャ: `.kiro/steering/tech.md`
- クレート構造: `.kiro/steering/structure.md`
