# Requirements Document

## Introduction

本仕様は、コードベースから**Dual Number方式の自動微分（Operator Overloading / num-dual）**を完全に削除し、**Enzyme（LLVM-IR変換）方式**に統一するリファクタリングを定義する。

現在、Neutryx は2つの自動微分方式を実装している：
1. **オペレーター・オーバーローディング法（Dual Number）**: `pricer_core/src/types/dual.rs` - `num-dual`クレートを使用
2. **LLVM-IR変換法（Enzyme）**: `pricer_risk/src/enzyme/*` - Enzyme LLVMプラグインを使用

Dual Number方式は「検証モード」として使用されてきたが、Enzyme方式が成熟した現在、コードベースの複雑さを削減し、単一のADバックエンドに統一する。

## Requirements

### Requirement 1: Dual Number モジュールの削除

**Objective:** As a 開発者, I want Dual Number関連のコードを完全に削除する, so that コードベースの複雑さを削減し保守性を向上させる

#### Acceptance Criteria
1. When リファクタリングが完了した場合, the pricer_core shall not contain `types/dual.rs` module
2. When リファクタリングが完了した場合, the Cargo.toml shall not contain `num-dual` dependency
3. When ビルドを実行した場合, the workspace shall compile without any Dual<T> type references
4. The pricer_core shall remove all Dual<f64> type aliases and re-exports from prelude

### Requirement 2: Feature Flag の整理

**Objective:** As a 開発者, I want 不要になった feature flag を削除する, so that ビルド設定がシンプルになる

#### Acceptance Criteria
1. When リファクタリングが完了した場合, the Cargo.toml shall not contain `num-dual-mode` feature flag
2. When リファクタリングが完了した場合, the pricer_core shall remove all `#[cfg(feature = "num-dual-mode")]` conditional compilation
3. The workspace shall build with only `enzyme-ad` feature for AD functionality
4. If `num-dual-mode` feature was enabled by default, the workspace shall update default features accordingly

### Requirement 3: 依存コードの更新

**Objective:** As a 開発者, I want Dual<T>を使用していた全てのコードを更新する, so that Enzyme方式のみでGreeks計算が動作する

#### Acceptance Criteria
1. When Dual<f64>型を使用していた関数がある場合, the pricer_models shall update those functions to use f64 or Enzyme-compatible types
2. When `GreeksMode::NumDual` が指定された場合, the pricer_pricing shall return error or fallback to Enzyme mode
3. The pricer_risk/enzyme module shall remain the sole source for AD functionality
4. While pricing calculations are executed, the pricer_pricing shall not require Dual number types

### Requirement 4: テストコードの更新

**Objective:** As a 開発者, I want num-dualを使用していたテストを更新する, so that テストがEnzyme方式のみで動作する

#### Acceptance Criteria
1. When verification tests referenced num-dual, the tests shall be updated to use Enzyme AD or removed if redundant
2. The workspace shall maintain test coverage without num-dual verification mode
3. If Enzyme vs num-dual comparison tests existed, the tests shall be converted to Enzyme-only tests or analytical verification
4. When `cargo test --workspace` is executed, the tests shall pass without num-dual dependency

### Requirement 5: ドキュメントの更新

**Objective:** As a 開発者, I want ドキュメントをEnzyme単一方式に合わせて更新する, so that 開発者が正確な情報を得られる

#### Acceptance Criteria
1. When リファクタリングが完了した場合, the steering documents shall remove references to "Dual-Mode Verification"
2. The tech.md shall update AD Backend section to reflect Enzyme-only architecture
3. The structure.md shall remove `types/dual.rs` from pricer_core structure documentation
4. The product.md shall update "Dual-Mode Verification" value proposition to "Enzyme AD with analytical verification"

### Requirement 6: 移行の安全性

**Objective:** As a 開発者, I want 移行が既存機能を破壊しないことを確認する, so that 本番環境への影響を最小化する

#### Acceptance Criteria
1. While migration is in progress, the workspace shall maintain backward compatibility for public API signatures
2. When breaking changes are unavoidable, the migration shall document them in CHANGELOG
3. The pricer_risk shall maintain all existing Enzyme AD functionality unchanged
4. If any Greeks calculation relied exclusively on num-dual, the calculation shall be reimplemented using Enzyme or analytical methods
