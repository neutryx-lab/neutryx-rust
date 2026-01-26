# Implementation Plan: move-enzyme-to-pricer-risk

## Tasks

- [x] 1. enzymeモジュールをpricer_riskに移動
- [x] 1.1 enzymeディレクトリをpricer_pricingからpricer_riskにコピー
  - 全11ファイル（mod.rs, forward.rs, reverse.rs, greeks.rs, loops.rs, parallel.rs, smooth.rs, checkpoint_ad.rs, fallback.rs, verification.rs, wrappers.rs）をコピー
  - ディレクトリ構造を維持
  - _Requirements: 1.1_

- [x] 1.2 pricer_pricingからenzymeディレクトリを削除
  - enzyme/ディレクトリ全体を削除
  - verify_enzyme.rsを削除（後続タスクで移動先に再配置）
  - _Requirements: 1.2_

- [x] 2. Cargo.toml依存関係を更新
- [x] 2.1 (P) pricer_risk/Cargo.tomlにenzyme依存を追加
  - llvm-sys依存をoptionalで追加（version 180, features = ["prefer-dynamic"]）
  - enzyme-ad featureを定義（dep:llvm-sysを有効化）
  - _Requirements: 2.1, 2.4_

- [x] 2.2 (P) pricer_pricing/Cargo.tomlからenzyme依存を削除
  - llvm-sys依存を削除
  - enzyme-ad feature定義を削除
  - _Requirements: 2.2_

- [x] 3. lib.rsモジュールエクスポートを更新
- [x] 3.1 pricer_risk/lib.rsにenzymeモジュールを追加
  - `#![cfg_attr(feature = "enzyme-ad", feature(autodiff))]`をファイル先頭に追加
  - `pub mod enzyme;`を宣言
  - `pub use enzyme::{gradient, gradient_with_step, ADMode, Activity};`でre-export
  - _Requirements: 1.3, 2.3_

- [x] 3.2 pricer_pricing/lib.rsからenzyme関連を削除
  - `pub mod enzyme;`を削除
  - enzymeのre-export（`pub use enzyme::*`）を削除
  - nightly feature属性（enzyme-ad関連）を削除
  - _Requirements: 1.2, 3.4_

- [x] 4. enzyme内部のcrate参照を更新
- [x] 4.1 greeks.rsの内部参照をpricer_pricingパスに変更
  - `crate::mc::{GbmParams, MonteCarloPricer, PayoffParams, PricingResult}`を`pricer_pricing::mc::`に変更
  - テストモジュール内の`crate::mc::MonteCarloConfig`も同様に更新
  - _Requirements: 3.1_

- [x] 4.2 verification.rsの内部参照をpricer_pricingパスに変更
  - `crate::mc::{GbmParams, MonteCarloConfig, MonteCarloPricer, PayoffParams}`を`pricer_pricing::mc::`に変更
  - _Requirements: 3.1_

- [x] 4.3 checkpoint_ad.rsの内部参照をpricer_pricingパスに変更
  - `crate::checkpoint::{CheckpointManager, CheckpointStrategy}`を`pricer_pricing::checkpoint::`に変更
  - _Requirements: 3.1_

- [x] 5. verify_enzymeテストをpricer_riskに移動
- [x] 5.1 verify_enzyme.rsをpricer_risk/tests/に配置
  - インポートパスを更新：`pricer_risk::enzyme::gradient`
  - pricer_pricingからの参照を維持：`pricer_pricing::verify::{square, square_gradient}`、`pricer_pricing::path_dependent::PathPayoffType`
  - テストが独立して実行できることを確認
  - _Requirements: 3.2_

- [x] 6. docstring内のパス参照を一括更新
- [x] 6.1 (P) enzyme/*.rs内の`pricer_pricing::enzyme`参照を`pricer_risk::enzyme`に置換
  - mod.rs, forward.rs, reverse.rs, greeks.rs, loops.rs内のdocstring更新
  - parallel.rs, smooth.rs, checkpoint_ad.rs, fallback.rs, verification.rs, wrappers.rs内のdocstring更新
  - 約40箇所の参照を一括置換
  - _Requirements: 3.1_

- [x] 6.2 (P) .kiro/specsドキュメント内の参照を更新
  - generic-pricing-risk-engine, irs-aad-demo, aad-yield-curve-bootstrapper, enzyme-autodiff-integration内の参照を確認・更新
  - _Requirements: 3.1_

- [x] 7. steeringドキュメントを更新
- [x] 7.1 (P) structure.mdのpricer_risk/pricer_pricingセクションを更新
  - pricer_riskにenzyme/モジュールを追加
  - pricer_pricingからenzyme/を削除
  - L4説明に「Nightly when enzyme-ad」を追記
  - _Requirements: 4.1, 4.2_

- [x] 7.2 (P) tech.mdにpricer_riskのnightly要件を記載
  - pricer_riskがenzyme-ad feature有効時にnightly Rustを必要とすることを記載
  - _Requirements: 4.3_

- [x] 8. ビルドとテストの検証
- [x] 8.1 stableビルドを検証
  - `cargo build -p pricer_risk`が成功することを確認
  - `cargo build -p pricer_pricing`が成功することを確認
  - `cargo build --workspace`が成功することを確認
  - _Requirements: 5.1, 5.3_

- [x] 8.2 nightlyビルドを検証
  - `cargo +nightly build -p pricer_risk --features enzyme-ad`が成功することを確認
  - enzyme機能が正しく有効化されることを確認
  - _Requirements: 5.4_

- [x] 8.3 テストを実行
  - `cargo test -p pricer_risk`が全て成功することを確認
  - verify_enzyme統合テストが成功することを確認
  - 既存のenzyme関連テストがパスすることを確認
  - _Requirements: 5.2_

## Requirements Coverage

| Requirement ID | Task(s) |
|----------------|---------|
| 1.1 | 1.1 |
| 1.2 | 1.2, 3.2 |
| 1.3 | 3.1 |
| 2.1 | 2.1 |
| 2.2 | 2.2 |
| 2.3 | 3.1 |
| 2.4 | 2.1 |
| 3.1 | 4.1, 4.2, 4.3, 6.1, 6.2 |
| 3.2 | 5.1 |
| 3.3 | 6.2 |
| 3.4 | 3.2 |
| 4.1 | 7.1 |
| 4.2 | 7.1 |
| 4.3 | 7.2 |
| 5.1 | 8.1 |
| 5.2 | 8.3 |
| 5.3 | 8.1 |
| 5.4 | 8.2 |
