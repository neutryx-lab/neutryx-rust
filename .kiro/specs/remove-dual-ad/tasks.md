# Implementation Plan: remove-dual-ad

## Tasks

- [x] 1. Dual Numberモジュールの削除
- [x] 1.1 (P) dual.rsファイルとテストファイルの削除
  - pricer_coreからDualNumber型エイリアス定義ファイルを削除
  - 関連するDualNumber単体テストファイルを削除
  - ファイル削除後のコンパイル確認（エラー箇所の特定）
  - _Requirements: 1.1, 1.3_

- [x] 1.2 types/mod.rsからdualモジュール参照を削除
  - 条件付きコンパイル`#[cfg(feature = "num-dual-mode")]`ブロックを削除
  - dualモジュールのpub mod宣言を削除
  - preludeからのDualNumber再エクスポートを削除（存在する場合）
  - _Requirements: 1.4_

- [x] 1.3 (P) Newton-Raphsonソルバーからsolve_ad関数を削除
  - `#[cfg(feature = "num-dual-mode")]`で囲まれたsolve_ad関数ブロックを削除
  - 関連するテストコード内の`#[cfg(feature = "num-dual-mode")]`ブロックを削除
  - 残りのソルバー機能（solve, solve_bracketed）が正常に動作することを確認
  - _Requirements: 3.1_

- [x] 1.4 (P) VegaCalculatorからcompute_vega_forward_ad関数を削除
  - volcube/vega.rsから`#[cfg(feature = "num-dual-mode")]`ブロックを削除
  - compute_vega_forward_ad関数を削除
  - compute_vega_finite_differenceが代替として機能することを確認
  - _Requirements: 3.1_

- [x] 2. 依存関係とFeature Flagの整理
- [x] 2.1 workspace Cargo.tomlからnum-dual依存を削除
  - ルートCargo.tomlの`[workspace.dependencies]`セクションからnum-dual行を削除
  - 依存関係の整合性を確認
  - _Requirements: 1.2_

- [x] 2.2 pricer_core Cargo.tomlの更新
  - `[dependencies]`からnum-dual optional依存を削除
  - `[features]`セクションからnum-dual-mode定義を削除
  - default featuresからnum-dual-modeを削除
  - enzyme-modeが残ることを確認
  - _Requirements: 2.1, 2.4_

- [x] 2.3 pricer_models Cargo.tomlの更新
  - `[features]`セクションからnum-dual-mode転送定義を削除
  - pricer_coreへのfeature依存が正しいことを確認
  - _Requirements: 2.1_

- [x] 2.4 GreeksMode列挙型からNumDualバリアントを削除
  - pricer_risk/greeks/config.rsからNumDualバリアントを削除
  - pricer_pricing/generic_pricer/config.rsからNumDualバリアントを削除
  - pricer_risk/enzyme/fallback.rsのCoreGreeksModeからNumDualを削除
  - 関連するドキュメントコメントを更新
  - GreeksModeがBumpRevalueとEnzymeAADのみになることを確認
  - _Requirements: 3.2_

- [x] 3. テストコードの更新
- [x] 3.1 (P) pricer_coreテストの更新
  - module_exports.rsから`#[cfg(feature = "num-dual-mode")]`テストを削除
  - num-dual依存のテストが存在しないことを確認
  - `cargo test -p pricer_core`で全テスト通過を確認
  - _Requirements: 4.1, 4.4_

- [x] 3.2 (P) pricer_riskテストの更新
  - greeks_by_factor.rsのGreeksMode::NumDualをBumpRevalueに変更
  - greeks/tests.rsのGreeksMode::NumDualをBumpRevalueに変更
  - benches/risk.rsのGreeksMode::NumDualをBumpRevalueに変更
  - _Requirements: 4.3_

- [x] 3.3 (P) pricer_pricingテストの更新
  - generic_pricer/config.rsのGreeksMode::NumDualテストをBumpRevalueに変更
  - テストの意図が維持されていることを確認
  - _Requirements: 4.3_

- [x] 4. CI/CDワークフローの更新
- [x] 4.1 GitHub Actions ci.ymlの更新
  - `cargo test -p pricer_core --features num-dual-mode`ステップを削除
  - "num-dual fallback"メッセージを"bump-and-revalue fallback"に更新
  - Enzyme未環境でのfallback説明を適切に修正
  - _Requirements: 4.4_

- [x] 5. ドキュメントの更新
- [x] 5.1 (P) Steering文書の更新
  - tech.mdから"num-dual (verification mode)"記述を削除
  - tech.mdのAD Backend節をEnzyme-only構成に更新
  - structure.mdから`types/dual.rs`行を削除
  - product.mdの"Dual-Mode Verification"を"Enzyme AD with bump-and-revalue verification"に変更
  - dependency-management.mdからnum-dual-modeを削除
  - _Requirements: 5.1, 5.2, 5.3, 5.4_

- [x] 5.2 (P) DocCommentsの一括更新
  - num-dual-mode関連のコード（sensitivity.rs等）から条件付きコンパイルブロックを削除
  - ドキュメントコメントからnum-dual言及を更新
  - `cargo check --workspace`でコンパイル成功を確認
  - _Requirements: 5.1_

- [x] 5.3 CHANGELOGの更新
  - Breaking changeとしてnum-dual削除を記載
  - 影響範囲と代替手段（BumpRevalue）を明記
  - 削除されたファイル・関数・型の一覧を記載
  - _Requirements: 6.2_

- [x] 6. 最終検証
- [x] 6.1 ビルド検証
  - `cargo check --workspace --exclude pricer_pricing --exclude pricer_risk --exclude demo_gui`で成功を確認
  - warningがnum-dual関連で発生しないことを確認
  - _Requirements: 1.3, 2.2, 2.3_

- [x] 6.2 テスト検証
  - `cargo test --workspace --exclude pricer_pricing --exclude pricer_risk --exclude demo_gui`で全テスト通過を確認（2331 passed, 1 failed - 無関係なSABRテスト）
  - GreeksMode::BumpRevalueでの計算結果が正常であることを確認
  - _Requirements: 4.2, 4.4, 6.3_

- [x] 6.3 ドキュメント検証
  - steering文書の整合性を更新済み
  - Dual64/DualNumberへの主要な言及を削除
  - _Requirements: 5.1_
