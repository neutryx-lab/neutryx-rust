# Requirements Document

## Introduction

`service_cli` と `service_python` を `service_gateway` に feature-gate で吸収統合するリファクタリング仕様。3つの独立した Service 層 crate を単一の `service_gateway` crate に統合し、CLI コマンド群と PyO3 バインディングを feature flag で選択的にビルド可能にする。既存の gateway services 層を CLI コマンドから直接利用することでビジネスロジックの重複を排除し、ワークスペースの簡素化と保守性向上を実現する。

## Requirements

### Requirement 1: CLI モジュールの移植

**Objective:** 開発者として、`service_cli` の CLI コマンド群を `service_gateway` 内の feature-gated モジュールとして利用したい。これにより、gateway の services 層を CLI から直接再利用でき、ビジネスロジックの重複を排除できる。

#### Acceptance Criteria

1. Where `cli` feature が有効, the `service_gateway` shall `src/cli/` 配下に CLI コマンドモジュールを提供する
2. When `cargo build -p service_gateway --features cli` を実行した場合, the `service_gateway` shall `clap` ベースの CLI エントリーポイントをビルドする
3. The `service_gateway` shall 既存の `service_cli` で定義されたすべてのコマンド（`calibrate`, `price`, `report`, `check`, `demo`）を `src/cli/commands/` に移植する
4. When CLI コマンドがビジネスロジックを必要とする場合, the `service_gateway` shall 既存の services 層（`CurveService`, `PricingService` 等）を直接利用する
5. The `service_gateway` shall `cli` feature が無効の場合、CLI 関連コードをコンパイルから除外する

### Requirement 2: Python バインディングの移植

**Objective:** 研究者として、`service_python` の PyO3 バインディングを `service_gateway` 内の feature-gated モジュールとして利用したい。これにより、Jupyter ワークフローで引き続き Rust ライブラリにアクセスできる。

#### Acceptance Criteria

1. Where `python` feature が有効, the `service_gateway` shall `src/python/` 配下に PyO3 バインディングモジュールを提供する
2. When `cargo build -p service_gateway --features python` を実行した場合, the `service_gateway` shall `neutryx_py` Python 拡張モジュールをビルドする
3. The `service_gateway` shall 既存の `service_python` で定義されたすべてのバインディング（`PyVanillaOption`, `PyForward`, `PyHullWhite`, `price_black_scholes`, `price_garman_kohlhagen`, `version`）を移植する
4. The `service_gateway` shall `python` feature が無効の場合、PyO3 関連コードと `pyo3` 依存をコンパイルから除外する

### Requirement 3: Feature Flag アーキテクチャ

**Objective:** ビルドエンジニアとして、CLI・Python・REST API を feature flag で独立に制御したい。これにより、デプロイメント要件に応じた最小限のバイナリをビルドできる。

#### Acceptance Criteria

1. The `service_gateway` shall `cli`, `python` の2つの新規 feature flag を `Cargo.toml` に定義する
2. The `service_gateway` shall デフォルト feature に `cli` と `python` を含めない（既存の `default = ["rest"]` を維持）
3. The `service_gateway` shall `full` feature に `cli` を含める
4. Where `python` feature が有効, the `service_gateway` shall `Cargo.toml` の `[lib]` セクションで `crate-type = ["cdylib", "rlib"]` を設定する
5. The `service_gateway` shall `clap` 依存を `optional = true` として `cli` feature でゲートする
6. The `service_gateway` shall `pyo3` 依存を `optional = true` として `python` feature でゲートする
7. The `service_gateway` shall ワークスペースの `[workspace.dependencies]` に `clap` と `pyo3` を追加する

### Requirement 4: バイナリターゲットの管理

**Objective:** 開発者として、サーバーモードと CLI モードの両方を単一 crate から実行したい。これにより、デプロイメントアーティファクトを一元管理できる。

#### Acceptance Criteria

1. The `service_gateway` shall 既存の `neutryx-server` バイナリターゲット（`src/main.rs`）を維持する
2. Where `cli` feature が有効, the `service_gateway` shall `neutryx` CLI バイナリターゲット（追加の `[[bin]]` セクション）を提供する
3. If `cli` feature が無効の状態で CLI バイナリをビルドしようとした場合, the `service_gateway` shall コンパイルエラーでビルド不可であることを明示する

### Requirement 5: エラー型の統合

**Objective:** 開発者として、Service 層のエラー型を統一的に扱いたい。これにより、CLI・REST・Python の各インターフェースで一貫したエラーハンドリングが実現できる。

#### Acceptance Criteria

1. The `service_gateway` shall 既存の `ServerError` を拡張し、CLI 固有のエラーバリアントを追加する
2. The `service_gateway` shall `thiserror` を使用したエラー型定義を維持する（steering: error-handling.md 準拠）
3. When CLI コマンドでエラーが発生した場合, the `service_gateway` shall CLI に適したフォーマット（人間可読テキスト）でエラーを表示する
4. When REST API でエラーが発生した場合, the `service_gateway` shall 既存の HTTP エラーレスポンス形式を維持する

### Requirement 6: ワークスペースのクリーンアップ

**Objective:** メンテナとして、統合後の不要な crate をワークスペースから除去したい。これにより、ワークスペースの簡素化と保守負荷の軽減を実現する。

#### Acceptance Criteria

1. When 統合が完了した場合, the workspace shall ルート `Cargo.toml` から `service_cli` と `service_python` のコメントアウトされたエントリを削除する
2. The workspace shall `crates/service_cli/` および `crates/service_python/` ディレクトリを削除する
3. The workspace shall ステアリングドキュメント（`structure.md`, `tech.md`, `roadmap.md`）を更新し、統合後の Service 層構成を反映する

### Requirement 7: ビルド互換性の維持

**Objective:** CI/CD エンジニアとして、統合後も既存のビルドパイプラインが正常に動作することを確認したい。

#### Acceptance Criteria

1. When `cargo build --workspace` を実行した場合, the workspace shall エラーなくビルドが完了する
2. When `cargo test --workspace` を実行した場合, the workspace shall 既存テストがすべてパスする
3. When `cargo clippy --workspace -- -D warnings` を実行した場合, the workspace shall 警告なくリントが完了する
4. The `service_gateway` shall `default` feature のみでのビルドにおいて、既存の REST API 機能に影響を与えない
5. While `python` feature が無効の場合, the `service_gateway` shall `pyo3` クレートをダウンロード・コンパイルしない
6. While `cli` feature が無効の場合, the `service_gateway` shall `clap` クレートをダウンロード・コンパイルしない
