# Implementation Plan

- [ ] 1. Cargo.toml と lib.rs の基盤構築
- [ ] 1.1 service_gateway の Cargo.toml に feature flag・依存・ターゲットを追加する
  - `cli` と `python` の2つの新規 feature flag を定義し、`default` に含めない
  - `clap` を `optional = true`（workspace inheritance）で追加し、`cli` feature にゲートする
  - `pyo3` を `optional = true`（workspace inheritance）で追加し、`python` feature にゲートする
  - `full` feature に `cli` を含める
  - `[lib]` セクションを追加し `name = "service_gateway"`, `crate-type = ["cdylib", "rlib"]` を設定する
  - `[[bin]]` セクションに `neutryx` CLI ターゲットを追加し `required-features = ["cli"]` を設定する
  - 既存の `neutryx-server` バイナリターゲットは変更しない
  - _Requirements: 1.2, 1.5, 2.4, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 4.1, 4.2, 4.3, 7.5, 7.6_

- [ ] 1.2 lib.rs を新規作成し、既存モジュールの公開宣言と feature-gated モジュール宣言を行う
  - 既存の `mod` 宣言（`config`, `error`, `rest`, `services`, `state`）を lib.rs に移動する
  - `#[cfg(feature = "cli")]` で `cli` モジュール宣言を追加する
  - `#[cfg(feature = "python")]` で `python` モジュール宣言と PyO3 `#[pymodule]` 登録を追加する
  - `ServerError` と `AppState` の `pub use` 再エクスポートを提供する
  - _Requirements: 2.2, 3.4_

- [ ] 1.3 main.rs を lib.rs 経由のモジュール参照に変更する
  - `main.rs` 内の `mod` 宣言を削除し、`use service_gateway::*` 経由での参照に切り替える
  - 既存の REST サーバー起動ロジックを維持する
  - デフォルト feature（`rest`）のみでのビルドが既存動作と同一であることを確認する
  - _Requirements: 4.1, 7.4_

- [ ] 2. (P) ServerError にCLI 固有のエラーバリアントを追加する
  - `Config(String)`, `Io(#[from] std::io::Error)`, `FileNotFound(String)`, `InvalidArgument(String)`, `Parse(String)` の5バリアントを追加する
  - 各バリアントに `#[error("...")]` アトリビュートを設定する（thiserror 準拠）
  - `IntoResponse` 実装に新規バリアントの HTTP ステータスマッピングを追加する（Config→500, Io→500, FileNotFound→404, InvalidArgument→400, Parse→400）
  - `error_code()` メソッドに新規バリアントのコード文字列を追加する
  - 既存バリアントの動作に影響を与えないことを確認する
  - タスク 1 完了後に実行可能。Python モジュール移植（タスク 3）と並行可能
  - _Requirements: 5.1, 5.2, 5.3, 5.4_

- [ ] 3. (P) Python バインディングの移植
- [ ] 3.1 (P) python モジュールの骨格を作成し、register_module 関数を定義する
  - `src/python/mod.rs` を作成し、`bindings` サブモジュールを宣言する
  - `register_module` 関数を定義し、PyO3 クラスと関数の登録を行う
  - lib.rs の `#[pymodule]` から `register_module` を呼び出す構成を確認する
  - タスク 1 完了後に実行可能。エラー型拡張（タスク 2）と並行可能
  - _Requirements: 2.1_

- [ ] 3.2 (P) service_python のバインディング実装を移植する
  - `PyVanillaOption`, `PyForward`, `PyHullWhite` の PyO3 クラスバインディングを移植する
  - `price_black_scholes`, `price_garman_kohlhagen` のプライシング関数を移植する
  - `version` ユーティリティ関数を移植する
  - `normal_cdf` ヘルパー関数を含めて移植する
  - 全バインディングが `#[cfg(feature = "python")]` 配下でコンパイルされることを確認する
  - _Requirements: 2.3, 2.4_

- [ ] 4. CLI モジュールの移植
- [ ] 4.1 cli モジュールの骨格を作成し、Clap CLI 構造体を定義する
  - `src/cli/mod.rs` を作成し、`Cli` 構造体と `Commands` enum を定義する
  - `Cli` に `verbose`, `config` フィールドと `command` サブコマンドを含める
  - `Commands` enum に `Calibrate`, `Price`, `Report`, `Check`, `Demo` バリアントを定義する
  - 各バリアントのフィールドを service_cli から移植する（デフォルト値含む）
  - `run()` ディスパッチ関数を定義し、`Commands` の各バリアントを対応するコマンドモジュールに委譲する
  - `commands` サブモジュールを宣言する
  - タスク 2（エラー型拡張）完了後に実行可能
  - _Requirements: 1.1, 1.2_

- [ ] 4.2 各 CLI コマンドの実装を移植する
  - `src/cli/commands/` 配下に `calibrate.rs`, `price.rs`, `report.rs`, `check.rs`, `demo.rs` を作成する
  - 各コマンドの既存スタブ実装を service_cli からそのまま移植する
  - services 層（`PricingService`, `CurveService` 等）を利用するインポートパスに置き換える
  - CLI コンテキストでのエラー出力を `ServerError` の `Display` trait で行う
  - 全コマンドモジュールが `#[cfg(feature = "cli")]` 配下でコンパイルされることを確認する
  - _Requirements: 1.3, 1.4, 1.5, 5.3_

- [ ] 4.3 CLI バイナリエントリーポイントを作成する
  - `src/cli_main.rs` を作成し、`Cli::parse()` と `run()` 呼び出しを実装する
  - tracing-subscriber を初期化し、`verbose` フラグに応じたログレベルを設定する
  - エラー発生時にプロセス終了コード 1 で終了する
  - `cli` feature 有効時のみビルドされることを `required-features` で保証する
  - _Requirements: 4.2, 4.3, 5.3_

- [ ] 5. ワークスペースのクリーンアップ
- [ ] 5.1 旧 crate ディレクトリを削除する
  - `crates/service_cli/` ディレクトリを完全に削除する
  - `crates/service_python/` ディレクトリを完全に削除する
  - _Requirements: 6.2_

- [ ] 5.2 ルート Cargo.toml からコメントアウトされたエントリを削除する
  - `workspace.members` からコメントアウトされた `service_cli` と `service_python` の行を削除する
  - _Requirements: 6.1_

- [ ] 6. ビルド検証と互換性テスト
  - `cargo build --workspace` でデフォルト feature のみのビルドが成功することを確認する
  - `cargo build -p service_gateway --features cli` で CLI feature のビルドが成功することを確認する
  - `cargo build -p service_gateway --features python` で Python feature のビルドが成功することを確認する
  - `cargo test --workspace` で既存テストがすべてパスすることを確認する
  - `cargo clippy --workspace -- -D warnings` で警告なくリントが完了することを確認する
  - デフォルトビルドで `clap` と `pyo3` がコンパイル対象外であることを依存ツリーで検証する
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_

- [ ] 7. (P) ステアリングドキュメントの更新
  - `structure.md` の Service 層セクションを更新し、service_cli/service_python の記述を削除、service_gateway の cli/python モジュール構成を追加する
  - `tech.md` の Service 層テクノロジースタックを更新し、統合後の構成を反映する
  - `roadmap.md` の Service Layer Status を更新し、本 spec を completed に追加する
  - タスク 5 完了後に実行可能。ビルド検証（タスク 6）と並行可能
  - _Requirements: 6.3_
