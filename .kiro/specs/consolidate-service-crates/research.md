# Research & Design Decisions

---
**Purpose**: `consolidate-service-crates` の設計判断を支える調査記録
---

## Summary
- **Feature**: `consolidate-service-crates`
- **Discovery Scope**: Extension（既存システムの統合リファクタリング）
- **Key Findings**:
  - `clap` と `pyo3` は既にワークスペース依存として定義済み（追加不要）
  - `service_gateway` に `lib.rs` が存在しないため、Python バインディング統合には新規作成が必要
  - `service_cli` は `infra_store` に依存するが `service_gateway` にはない — 統合時に追加要否を判断

## Research Log

### Cargo の `[[bin]]` + `[lib]` 共存と feature-gated cdylib

- **Context**: Python バインディング（cdylib）と複数バイナリを同一 crate で共存させる必要がある
- **Findings**:
  - Cargo は `[lib]` と `[[bin]]` の共存をネイティブにサポートする
  - `crate-type = ["cdylib", "rlib"]` を指定すると、`rlib` はバイナリの依存解決に、`cdylib` は Python 拡張に使用される
  - `cdylib` ビルドは `--features python` 指定時のみ意味がある（feature 無効時は通常の `rlib` のみ）
  - PyO3 の `extension-module` feature はリンカに影響するため、`python` feature が無効のときにコンパイルされないよう `optional = true` が必須
- **Implications**: `lib.rs` を新規作成し、`#[cfg(feature = "python")]` で PyO3 モジュール登録をゲートする

### ワークスペース依存の現状確認

- **Context**: `clap` と `pyo3` を workspace.dependencies に追加する要件（3.7）の事前確認
- **Findings**:
  - `clap = { version = "4.4", features = ["derive"] }` — **既に定義済み**
  - `pyo3 = { version = "0.22", features = ["extension-module"] }` — **既に定義済み**
  - 両方とも workspace inheritance で使用可能
- **Implications**: 要件 3.7 は既に充足。service_gateway の Cargo.toml で `{ workspace = true, optional = true }` を指定するだけでよい

### service_cli の infra_store 依存

- **Context**: `service_cli` は `infra_store` に依存するが、`service_gateway` は依存していない
- **Findings**:
  - `service_cli` の commands で `infra_store` を直接利用しているコードは限定的
  - CLI コマンドは殆どスタブ実装で、`infra_store` への実質的な依存は config 読み込みのみ
  - `service_gateway` は `infra_config` 経由で設定を管理しており、`infra_store` は不要
- **Implications**: 統合時に `infra_store` 依存は追加しない。将来 CLI コマンドが永続化を必要とする場合に optional 依存として追加

### CLI バイナリターゲット戦略

- **Context**: `neutryx-server` と `neutryx` CLI を同一 crate から提供する方法
- **Findings**:
  - Cargo は `required-features` を `[[bin]]` セクションでサポートし、feature 無効時にバイナリをビルド対象から除外できる
  - `[[bin]] name = "neutryx" path = "src/cli_main.rs" required-features = ["cli"]` の形式で、`cli` feature が無効の場合にビルドをスキップ
  - `cargo build` でデフォルト feature（`rest`）のみの場合、`neutryx` バイナリはビルドされない
- **Implications**: `required-features` で要件 4.3 を実現。別の `cli_main.rs` エントリーポイントを用意

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Feature-gated モジュール統合 | CLI/Python を feature flag でゲートしたモジュールとして service_gateway に統合 | 単一 crate、ビジネスロジック共有、ワークスペース簡素化 | crate の責務が広がる、ビルド時間への影響 | **選択**: 既にコメントアウト済みで方向性が一致 |
| Interface crate 分離 | 共通インターフェースを別 crate に抽出し、各 service crate から参照 | 各 crate の独立性維持 | crate 数増加、共有ロジック重複 | 現状の実装量では過剰 |

## Design Decisions

### Decision: `lib.rs` の新規作成と crate-type 管理

- **Context**: Python バインディングには `cdylib` crate type が必要だが、現在 `service_gateway` は `lib.rs` を持たないバイナリ専用 crate
- **Alternatives Considered**:
  1. 常時 `crate-type = ["cdylib", "rlib"]` を設定 — ビルド時間増、不要な成果物
  2. `python` feature 有効時のみ cdylib として動作 — Cargo.toml の静的定義では feature 条件付き crate-type は不可
- **Selected Approach**: `crate-type = ["cdylib", "rlib"]` を常時定義し、`lib.rs` 内部で `#[cfg(feature = "python")]` による条件付きモジュール宣言を行う。Python feature 無効時は `lib.rs` が実質空になり、rlib のみが有意義に使用される
- **Rationale**: Cargo は crate-type の feature 条件付き切り替えをサポートしない。cdylib の追加ビルドコストは Python feature 無効時にも発生するが、空の cdylib は無視可能
- **Trade-offs**: ビルド時に余分な cdylib が生成されるが、実質的な影響は最小限
- **Follow-up**: CI/CD パイプラインで `--features python` 指定時のみ Python wheel をビルドするよう設定

### Decision: CLI エントリーポイントの分離

- **Context**: サーバーモード（`main.rs`）と CLI モード（`cli_main.rs`）を同一 crate で提供
- **Selected Approach**: 別の `[[bin]]` ターゲットとして `src/cli_main.rs` を追加し、`required-features = ["cli"]` で制御
- **Rationale**: `main.rs` の変更を最小限に抑え、既存のサーバー起動ロジックに影響を与えない

### Decision: エラー型の拡張戦略

- **Context**: `ServerError`（HTTP 向け）と `CliError`（テキスト向け）の統合
- **Selected Approach**: `ServerError` に CLI バリアントを追加し、`IntoResponse` 実装は REST コンテキストでのみ使用。CLI では `Display` trait で人間可読出力
- **Rationale**: 既存の `ServerError` は feature-gate なしで定義されており、バリアント追加は後方互換

## Risks & Mitigations

- `cdylib` の常時生成によるビルド時間微増 — Python feature 無効時の cdylib は空に近いため影響最小限
- CLI feature 有効時の依存グラフ拡大（`clap`） — `optional = true` により default ビルドに影響なし
- `infra_store` 依存の欠落 — CLI コマンドのスタブ実装では不要、将来必要時に optional 追加
