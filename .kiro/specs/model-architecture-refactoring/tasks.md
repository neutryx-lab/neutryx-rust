# Implementation Plan

## Task Overview

| Phase | Description | Tasks |
|-------|-------------|-------|
| 1 | trades モジュール移動 | 1.1-1.4 |
| 2 | bootstrapping 移動 | 2.1-2.3 |
| 3 | モデル構造整理 | 3.1-3.2 |
| 4 | キャリブレーション整理 | 4.1-4.2 |
| 5 | pricer_optimiser 削除と依存関係整理 | 5.1-5.3 |
| 6 | ドキュメントとテスト更新 | 6.1-6.2 |

---

## Phase 1: trades モジュール移動

- [x] 1. trades モジュールを pricer_core に新設し、instruments と schedules を移動する

- [x] 1.1 pricer_core に trades モジュールを作成し、instruments を移動する
  - pricer_core 内に trades ディレクトリを新設する ✓ (既存)
  - instruments ディレクトリを pricer_models から trades/ にコピーする ✓ (既存)
  - 各ファイル内の `pricer_core::` 参照を `crate::` に変更する ✓
  - feature flags（equity, rates, credit, fx, commodity, exotic）を pricer_core の Cargo.toml に追加する ✓ (既存)
  - trades/mod.rs を作成し、instruments モジュールを公開する ✓
  - pricer_core/lib.rs から trades モジュールを公開する ✓
  - _Requirements: 7.1, 7.2, 7.5, 7.6_

- [x] 1.2 (P) schedules を pricer_core/trades に移動する
  - schedules ディレクトリを pricer_models から trades/ にコピーする ✓
  - 各ファイル内の `pricer_core::` 参照を `crate::` に変更する ✓
  - trades/mod.rs に schedules モジュールを追加する ✓
  - _Requirements: 7.1, 7.3, 7.5_

- [x] 1.3 pricer_models から trades を re-export して後方互換性を維持する
  - pricer_models の Cargo.toml に pricer_core への依存を確認する ✓
  - pricer_models/lib.rs で `pub use pricer_core::trades::instruments;` を追加する (schedules のみ)
  - pricer_models/lib.rs で `pub use pricer_core::trades::schedules;` を追加する ✓
  - 既存の instruments, schedules ディレクトリを削除する (schedules のみ削除)
  - _Requirements: 7.4_

- [x] 1.4 trades 移動後のビルドとテストを検証する
  - `cargo build -p pricer_core --all-features` を実行する ✓
  - `cargo build -p pricer_models --all-features` を実行する ✓
  - `cargo test -p pricer_core --all-features` を実行する ✓ (130 passed)
  - 既存の `use pricer_models::instruments::*` が動作することを確認する ✓
  - _Requirements: 7.4, 7.5_

---

## Phase 2: bootstrapping 移動

- [ ] 2. bootstrapping と provider を pricer_core/market_data に移動する

- [ ] 2.1 bootstrapping モジュールを pricer_core/market_data に移動する
  - pricer_optimiser/src/bootstrapping/ を pricer_core/src/market_data/bootstrapping/ にコピーする
  - 各ファイル内の `pricer_optimiser::` 参照を削除または `crate::` に変更する
  - pricer_models への依存がある場合は削除する（pricer_core 内で完結）
  - market_data/mod.rs に bootstrapping モジュールを追加する
  - _Requirements: 1.1, 4.1, 4.2, 4.3_

- [ ] 2.2 (P) provider を pricer_core/market_data に移動する
  - pricer_optimiser/src/provider.rs を pricer_core/src/market_data/provider.rs にコピーする
  - 内部の import を `crate::` に変更する
  - MarketProvider が CurveEnum, VolSurfaceEnum を使用するよう調整する
  - market_data/mod.rs に provider モジュールを追加する
  - _Requirements: 1.4_

- [ ] 2.3 bootstrapping 移動後のビルドとテストを検証する
  - `cargo build -p pricer_core --all-features` を実行する
  - bootstrapping の単体テストを実行する
  - BootstrappedCurve が YieldCurve trait を実装していることを確認する
  - _Requirements: 4.3_

---

## Phase 3: モデル構造整理

- [ ] 3. モデル定義を論理的なカテゴリ（equity, rates, hybrid）に整理する

- [ ] 3.1 株式系モデル（GBM, Heston, SABR）を equity/ に移動する
  - models/ ルートにある gbm.rs, heston.rs, sabr.rs を models/equity/ に移動する
  - models/equity/mod.rs を更新し、移動したモデルを公開する
  - models/mod.rs の re-export を更新し、既存の `use pricer_models::models::HestonModel` が動作し続けるようにする
  - feature flag `equity` の条件コンパイルを維持する
  - _Requirements: 2.1, 2.2, 2.3, 2.4_

- [ ] 3.2 モデル構造整理後のビルドとテストを検証する
  - `cargo build -p pricer_models --all-features` を実行する
  - `cargo test -p pricer_models --all-features` を実行する
  - StochasticModelEnum が全モデルを含むことを確認する
  - _Requirements: 2.3_

---

## Phase 4: キャリブレーション整理

- [ ] 4. キャリブレーション機能を整理し、ソルバー依存を統一する

- [ ] 4.1 キャリブレーションのソルバー依存を pricer_core に統一する
  - calibration 内の `pricer_optimiser::solvers` 参照を `pricer_core::math::solvers` に変更する
  - LevenbergMarquardtSolver, LMConfig, LMResult を使用するよう更新する
  - CalibrationScope（Global/TermByTerm/Piecewise）を traits.rs に追加する（存在しない場合）
  - ModelCalibrator を CalibrationEngine にリネームする
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

- [ ] 4.2 キャリブレーション整理後のビルドとテストを検証する
  - `cargo build -p pricer_models --all-features` を実行する
  - キャリブレーションの単体テストを実行する
  - HestonCalibrator, SABRCalibrator, HullWhiteCalibrator が動作することを確認する
  - _Requirements: 3.3_

---

## Phase 5: pricer_optimiser 削除と依存関係整理

- [ ] 5. pricer_optimiser を廃止し、依存関係を整理する

- [ ] 5.1 依存クレートの Cargo.toml を更新する
  - pricer_risk の Cargo.toml から pricer_optimiser 依存を削除する
  - pricer_risk に pricer_core（bootstrapping 用）への依存を追加/更新する
  - service_cli, service_gateway, service_python の依存を更新する
  - pricer_pricing から pricer_optimiser 依存を削除する（存在する場合）
  - _Requirements: 1.6, 5.2, 5.3_

- [ ] 5.2 pricer_optimiser を workspace から削除する
  - ルートの Cargo.toml から pricer_optimiser を workspace members から削除する
  - crates/pricer_optimiser/ ディレクトリを削除する
  - _Requirements: 1.2, 1.3, 1.5_

- [ ] 5.3 依存関係整理後の検証を行う
  - `cargo build --workspace` を実行し、警告なしで成功することを確認する
  - `cargo tree` で循環依存がないことを確認する
  - 依存グラフが L1 ← L2 ← L3 ← L4 を維持していることを確認する
  - _Requirements: 5.1, 5.4, 5.5_

---

## Phase 6: ドキュメントとテスト更新

- [ ] 6. ドキュメントを更新し、全体テストを実行する

- [ ] 6.1 (P) steering ドキュメントを更新する
  - .kiro/steering/structure.md から pricer_optimiser セクションを削除する
  - pricer_core セクションに trades, bootstrapping, provider を追加する
  - pricer_models セクションを更新する（instruments, schedules の削除を反映）
  - .kiro/steering/tech.md のレイヤー図から L2.5 を削除する
  - _Requirements: 6.1, 6.2_

- [ ] 6.2 全体テストと最終検証を実行する
  - `cargo test --workspace --all-features` を実行する
  - 全 feature flag 組み合わせでのビルドを確認する
  - 各モジュールの doc comments が新しい配置を反映していることを確認する
  - _Requirements: 6.3, 6.4_
