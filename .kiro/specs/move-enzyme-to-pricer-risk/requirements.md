# Requirements Document

## Introduction

本仕様は、`crates/pricer_pricing/src/enzyme/`モジュールを`crates/pricer_risk/src/enzyme/`へ移動するリファクタリングを定義する。

**背景**: enzymeモジュールはAAD（自動微分）によるリスク計算（Greeks算出）のための機能を提供している。現在はL3 (pricer_pricing)に配置されているが、その主な用途はL4 (pricer_risk)でのリスク指標計算であるため、アーキテクチャ上の整合性を高めるために移動を行う。

**対象モジュール**:
- `enzyme/checkpoint_ad.rs` - チェックポイントAD
- `enzyme/forward.rs` - フォワードモードAD
- `enzyme/reverse.rs` - リバースモードAD
- `enzyme/greeks.rs` - Greeks計算
- `enzyme/loops.rs` - ループ最適化
- `enzyme/parallel.rs` - 並列処理
- `enzyme/smooth.rs` - スムース関数
- `enzyme/verification.rs` - 検証ユーティリティ
- `enzyme/wrappers.rs` - ラッパー
- `enzyme/fallback.rs` - フォールバック実装
- `enzyme/mod.rs` - モジュールエントリポイント

## Requirements

### Requirement 1: モジュール移動

**Objective:** As a 開発者, I want enzymeモジュールがpricer_riskに配置されていること, so that AADによるリスク計算機能がアーキテクチャ上適切な場所に存在する

#### Acceptance Criteria

1. When enzymeモジュールが移動された場合, the pricer_risk shall `crates/pricer_risk/src/enzyme/`ディレクトリに全11ファイルを含む
2. When 移動が完了した場合, the pricer_pricing shall `crates/pricer_pricing/src/enzyme/`ディレクトリを含まない
3. The pricer_risk shall `lib.rs`にてenzymeモジュールをpublic exportする

### Requirement 2: 依存関係の更新

**Objective:** As a 開発者, I want 移動後もコンパイルが成功すること, so that 既存機能が維持される

#### Acceptance Criteria

1. When enzymeモジュールが移動された場合, the pricer_risk の `Cargo.toml` shall Enzyme関連の依存関係（llvm-sys等）を含む
2. When enzymeモジュールが移動された場合, the pricer_pricing の `Cargo.toml` shall Enzyme専用依存関係を削除する（他モジュールで不要な場合）
3. The pricer_risk shall nightly Rustツールチェーンでのビルドをサポートする
4. If enzymeのfeature flagが定義されている場合, then the pricer_risk shall 同等のfeature flag（`enzyme-ad`等）を定義する

### Requirement 3: 既存参照の更新

**Objective:** As a 開発者, I want enzymeモジュールへの既存参照が正しく更新されること, so that コードベース全体の整合性が保たれる

#### Acceptance Criteria

1. When `pricer_pricing::enzyme`への参照が存在する場合, the codebase shall `pricer_risk::enzyme`への参照に更新される
2. When テストコードがenzymeを参照している場合, the test code shall 新しいパスを使用する
3. When デモコードがenzymeを参照している場合, the demo code shall 新しいパスを使用する
4. The pricer_pricing shall enzymeのre-exportを提供しない（完全移行）

### Requirement 4: ドキュメント更新

**Objective:** As a 開発者, I want steeringドキュメントが更新されること, so that アーキテクチャ変更が文書化される

#### Acceptance Criteria

1. When 移動が完了した場合, the `.kiro/steering/structure.md` shall pricer_riskにenzymeモジュールが含まれることを記載する
2. When 移動が完了した場合, the `.kiro/steering/structure.md` shall pricer_pricingからenzymeモジュールの記載を削除する
3. The `.kiro/steering/tech.md` shall pricer_riskがnightlyビルドを必要とすることを記載する

### Requirement 5: ビルド検証

**Objective:** As a 開発者, I want 移動後のビルドとテストが成功すること, so that リグレッションがないことを確認できる

#### Acceptance Criteria

1. When `cargo build -p pricer_risk`を実行した場合, the build shall エラーなく完了する
2. When `cargo test -p pricer_risk`を実行した場合, the tests shall 全て成功する
3. When `cargo build --workspace`を実行した場合, the workspace build shall エラーなく完了する
4. If enzyme feature flagが有効な場合, When `cargo build -p pricer_risk --features enzyme-ad`を実行した場合, the build shall エラーなく完了する
