# Requirements Document

## Introduction

本仕様書は、Neutryx XVAプライシングライブラリのPhase 6「本番環境への強化」に関する要件を定義する。Phase 0-5で構築されたコア機能（4層アーキテクチャ、Enzyme AD統合、モンテカルロカーネル、XVA計算）を本番運用可能な状態にするため、ドキュメント整備、ベンチマーク作成、CI/CDパイプライン構築を実施する。

## Project Description (Input)

Phase 6: Production hardening with documentation, benchmarks, and CI/CD

---

## Requirements

### Requirement 1: APIドキュメント整備

**Objective:** As a ライブラリ利用者, I want 各クレートの完全なAPIドキュメント, so that 関数・型・トレイトの使用方法を理解できる

#### Acceptance Criteria

1. The pricer_core crate shall ドキュメントカバレッジ100%を維持する（全パブリックアイテムにdocコメント）
2. The pricer_models crate shall ドキュメントカバレッジ100%を維持する（全パブリックアイテムにdocコメント）
3. The pricer_kernel crate shall ドキュメントカバレッジ100%を維持する（全パブリックアイテムにdocコメント）
4. The pricer_xva crate shall ドキュメントカバレッジ100%を維持する（全パブリックアイテムにdocコメント）
5. When `cargo doc --no-deps` を実行した場合, the build system shall 警告なしでドキュメントを生成する
6. The documentation shall 各パブリック関数に使用例（`# Examples`セクション）を含む

### Requirement 2: ユーザーガイド・クイックスタート

**Objective:** As a 新規ユーザー, I want 導入ガイドとサンプルコード, so that ライブラリを素早く導入・活用できる

#### Acceptance Criteria

1. The README.md shall プロジェクト概要、インストール手順、クイックスタート例を含む
2. The documentation shall 4層アーキテクチャの説明と各レイヤーの役割を明記する
3. The documentation shall Enzyme環境セットアップ手順（Docker推奨）を含む
4. When ユーザーがサンプルコードを実行した場合, the examples shall コンパイル・実行可能な状態を維持する
5. The documentation shall stable Rust（L1/L2/L4）とnightly Rust（L3）の使い分けを説明する

### Requirement 3: Criterionベンチマーク

**Objective:** As a 開発者, I want 性能ベンチマーク, so that パフォーマンス特性を把握し回帰を検出できる

#### Acceptance Criteria

1. The pricer_core crate shall 補間処理（linear, cubic_spline, bilinear）のベンチマークを含む
2. The pricer_models crate shall Black-Scholes価格計算のベンチマークを含む
3. The pricer_kernel crate shall モンテカルロパス生成のベンチマークを含む
4. The pricer_kernel crate shall Enzyme AD vs num-dual比較ベンチマークを含む
5. The pricer_xva crate shall ポートフォリオレベルXVA計算のベンチマークを含む
6. When `cargo bench` を実行した場合, the benchmark suite shall 全ベンチマークを実行し結果を出力する
7. The benchmarks shall 異なる問題サイズ（パス数、ポートフォリオ規模）でのスケーリング特性を測定する

### Requirement 4: CI/CDパイプライン（Stable Rust）

**Objective:** As a 開発チーム, I want 自動ビルド・テスト・品質チェック, so that コード品質を継続的に維持できる

#### Acceptance Criteria

1. When プルリクエストが作成された場合, the CI pipeline shall L1/L2/L4クレートのビルドを実行する
2. When プルリクエストが作成された場合, the CI pipeline shall L1/L2/L4クレートのテストを実行する
3. When プルリクエストが作成された場合, the CI pipeline shall `cargo fmt --check`でフォーマットを検証する
4. When プルリクエストが作成された場合, the CI pipeline shall `cargo clippy -- -D warnings`でリントを実行する
5. When mainブランチにマージされた場合, the CI pipeline shall ドキュメント生成・デプロイを実行する
6. The CI configuration shall Windows, Linux, macOSでのクロスプラットフォームテストをサポートする

### Requirement 5: CI/CDパイプライン（Nightly Rust + Enzyme）

**Objective:** As a 開発チーム, I want L3クレート（pricer_kernel）の自動テスト, so that Enzyme統合の健全性を継続的に検証できる

#### Acceptance Criteria

1. The CI pipeline shall nightly-2025-01-15ツールチェーンでL3をビルドする
2. When Enzyme環境が利用可能な場合, the CI pipeline shall pricer_kernelのテストを実行する
3. If Enzyme環境のセットアップに失敗した場合, the CI pipeline shall 明確なエラーメッセージを表示しジョブを失敗させる
4. The CI configuration shall DockerベースのEnzyme環境をサポートする
5. Where enzyme-ad featureが有効な場合, the CI pipeline shall Enzyme ADテストを実行する

### Requirement 6: コード品質ゲート

**Objective:** As a プロジェクトメンテナ, I want 自動品質チェック, so that コードベースの品質基準を強制できる

#### Acceptance Criteria

1. The CI pipeline shall テストカバレッジを測定しレポートを生成する
2. If テストが失敗した場合, the CI pipeline shall プルリクエストのマージをブロックする
3. If フォーマット違反がある場合, the CI pipeline shall プルリクエストのマージをブロックする
4. If Clippy警告がある場合, the CI pipeline shall プルリクエストのマージをブロックする
5. The CI configuration shall ドキュメント警告をエラーとして扱う（`#![deny(missing_docs)]`）

### Requirement 7: リリース自動化

**Objective:** As a リリースマネージャー, I want セマンティックバージョニングと自動リリース, so that 一貫性のあるリリースプロセスを実現できる

#### Acceptance Criteria

1. The repository shall セマンティックバージョニング（SemVer）に従う
2. When gitタグが作成された場合, the CI pipeline shall リリースビルドを生成する
3. The release process shall CHANGELOGの自動生成をサポートする
4. The CI configuration shall crates.ioへの公開ワークフローを含む（将来対応）
5. The workspace shall 全クレートのバージョンを一元管理する

### Requirement 8: 開発者体験（DX）向上

**Objective:** As a コントリビュータ, I want 明確な開発ガイドライン, so that 効率的にプロジェクトに貢献できる

#### Acceptance Criteria

1. The repository shall CONTRIBUTING.mdを含む（コーディング規約、PR手順）
2. The documentation shall ローカル開発環境のセットアップ手順を含む
3. The documentation shall テスト実行方法（stable/nightly分離）を説明する
4. When 新しいコントリビュータがセットアップ手順に従った場合, the environment shall 30分以内に開発準備完了となる
5. The repository shall issue/PRテンプレートを含む
