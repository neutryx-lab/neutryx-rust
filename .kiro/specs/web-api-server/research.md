# Research & Design Decisions

---
**Purpose**: Capture discovery findings, architectural investigations, and rationale that inform the technical design.

**Usage**:
- Log research activities and outcomes during the discovery phase.
- Document design decision trade-offs that are too detailed for `design.md`.
- Provide references and evidence for future audits or reuse.
---

## Summary
- **Feature**: `web-api-server`
- **Discovery Scope**: New Feature (Layer 5追加)
- **Key Findings**:
  - Axum 0.8.x は stable Rust で動作し、型安全なルーティングとTower middleware統合を提供
  - utoipa crate でコンパイル時OpenAPI 3.0/3.1仕様生成が可能
  - tower_governor/tokio-rate-limit がAPI key別レート制限を提供
  - pricer_kernel (L3) はnightly必須のため、L5はfeature flag経由でオプショナル依存にする必要あり

## Research Log

### Axum Webフレームワークとエコシステム
- **Context**: Layer 5 (pricer_server) の基盤フレームワーク選定
- **Sources Consulted**:
  - [Axum公式ドキュメント](https://docs.rs/axum/latest/axum/)
  - [Axum GitHub](https://github.com/tokio-rs/axum)
  - [Build High-Performance REST APIs with Rust and Axum](https://www.twilio.com/en-us/blog/developers/community/build-high-performance-rest-apis-rust-axum)
  - [The Ultimate Guide to Axum (2025)](https://www.shuttle.dev/blog/2023/12/06/using-axum-rust)
- **Findings**:
  - Axum 0.8.x が最新安定版 (0.9はまだmain branch)
  - stable Rust Edition 2021で動作
  - Tokio async runtime、Tower middleware、Hyper HTTPの上に構築
  - マクロフリーAPI設計により予測可能なエラーハンドリング
  - Router::merge() でルーティング分割とモジュール化が容易
  - hyperへのオーバーヘッドが minimal でパフォーマンス優秀
- **Implications**:
  - L5は stable Rustでビルド可能
  - 既存プロジェクトのstable/nightly分離戦略と整合
  - モジュール化されたルーター設計により各エンドポイント群を独立ファイルに分割可能

### OpenAPI仕様生成 (utoipa)
- **Context**: Requirement 7でOpenAPI 3.0仕様とSwagger UI提供が必須
- **Sources Consulted**:
  - [utoipa GitHub](https://github.com/juhaku/utoipa)
  - [utoipa公式ドキュメント](https://docs.rs/utoipa/latest/utoipa/)
  - [Working with OpenAPI using Rust](https://www.shuttle.dev/blog/2024/04/04/using-openapi-rust)
  - [Auto-Generating & Validating OpenAPI Docs in Rust](https://identeco.de/en/blog/generating_and_validating_openapi_docs_in_rust/)
- **Findings**:
  - OpenAPI 3.0.3 および 3.1.0 対応
  - Code-firstアプローチ: Rustコードから自動生成
  - utoipa-axum crate でAxumルーター自動検出とOpenAPI生成可能
  - utoipa-swagger-ui でSwagger UI埋め込み対応
  - コンパイル時にスキーマ生成 (runtime overhead なし)
- **Implications**:
  - Request/Response型にutoipaマクロ適用でスキーマ自動生成
  - エンドポイントハンドラーにOpenAPIアノテーション追加
  - `/api/v1/openapi.json` および `/docs` エンドポイント提供

### Graceful Shutdown処理
- **Context**: Requirement 1.5でSIGTERM/SIGINT時のgraceful shutdown必須
- **Sources Consulted**:
  - [Tokio Graceful Shutdown](https://tokio.rs/tokio/topics/shutdown)
  - [Cancellation and Graceful Shutdown in Rust Async](https://www.slingacademy.com/article/cancellation-and-graceful-shutdown-in-rust-async-applications/)
  - [tokio-graceful-shutdown crate](https://lib.rs/crates/tokio-graceful-shutdown)
  - [Implementing graceful shutdown in Axum](https://app.studyraid.com/en/read/15308/530952/implementing-graceful-shutdown-in-axum)
- **Findings**:
  - tokio::signal でSIGINT/SIGTERM非同期キャプチャ
  - Axum の with_graceful_shutdown() で進行中リクエスト完了待機
  - tokio-graceful-shutdown crateが抽象化レイヤー提供
  - コンテナ環境 (Docker/K8s) での標準終了シグナル対応
- **Implications**:
  - サーバー起動時にsignal listener設定
  - Axum Router に with_graceful_shutdown() 適用
  - ログに shutdown イベント記録

### Rate Limiting と API Key認証
- **Context**: Requirement 9でレート制限とAPI key認証が必須
- **Sources Consulted**:
  - [Implementing API Rate Limiting in Rust](https://www.shuttle.dev/blog/2024/02/22/api-rate-limiting-rust)
  - [tower_governor crate](https://crates.io/crates/tower_governor)
  - [tokio-rate-limit crate](https://lib.rs/crates/tokio-rate-limit)
  - [API Development in Rust: Tower Middleware](https://dev.to/amaendeepm/api-development-in-rust-cors-tower-middleware-and-the-power-of-axum-397k)
  - [Creating a Rate Limiter Middleware using Tower for Axum](https://medium.com/@khalludi123/creating-a-rate-limiter-middleware-using-tower-for-axum-rust-be1d65fbeca)
- **Findings**:
  - **tower_governor**: governor crateベース、複数キー抽出戦略対応 (IP、API key)
  - **tokio-rate-limit**: lock-free token accounting、per-client制限、カスタムキー抽出
  - **tower-resilience-ratelimiter**: Resilience4jパターン、プロダクション用
  - API keyをカスタムヘッダー (`X-API-Key`) で抽出し、レート制限と認証に使用可能
- **Implications**:
  - tower_governor を選択 (Towerエコシステム標準、API key別制限対応)
  - カスタムAPI key extractorでAuthorizationヘッダーまたはX-API-Keyヘッダー対応
  - レート制限超過時にHTTP 429 + Retry-Afterヘッダー返却

### JSON Serialization (serde)
- **Context**: Requirement 2でJSON入出力とcamelCase、ISO 8601/4217対応
- **Sources Consulted**:
  - [Serialize and Deserialize Data in Rust Using serde](https://blog.ediri.io/serialize-and-deserialize-data-in-rust-using-serde-and-serdejson)
  - [time::serde::iso8601](https://time-rs.github.io/internal-api/time/serde/iso8601/index.html)
  - [Custom date format with Serde](https://serde.rs/custom-date-format.html)
  - [Fine-Grained JSON Serialization Control with Serde](https://leapcell.io/blog/fine-grained-json-serialization-control-in-rust-with-serde)
- **Findings**:
  - `#[serde(rename_all = "camelCase")]` でフィールド名自動変換
  - chrono crate の ISO 8601サポート (既存依存関係)
  - pricer_core::types::Currency が既にISO 4217コード提供
  - カスタムシリアライザーで日時フォーマット制御可能
- **Implications**:
  - 全API Request/Response構造体に `#[serde(rename_all = "camelCase")]` 適用
  - chrono::DateTime<Utc> でISO 8601タイムスタンプ処理
  - pricer_core::types::Currency を直接使用

### Prometheus Metrics
- **Context**: Requirement 8でPrometheus互換メトリクス必須
- **Sources Consulted**:
  - [axum-prometheus crate](https://docs.rs/axum-prometheus/latest/axum_prometheus/)
  - [axum-prometheus GitHub](https://github.com/Ptrskay3/axum-prometheus)
  - [prometheus-axum-middleware](https://docs.rs/prometheus-axum-middleware/latest/prometheus_axum_middleware/)
  - [Axum Prometheus例](https://github.com/tokio-rs/axum/blob/main/examples/prometheus-metrics/src/main.rs)
- **Findings**:
  - axum-prometheus: metrics.rs + metrics_exporter_prometheus使用
  - リクエスト/レスポンスサイズ、レイテンシー、エラー率を自動収集
  - `/metrics` エンドポイントでPrometheusテキストフォーマット提供
  - MSRV 1.75 (stable Rust対応)
- **Implications**:
  - axum-prometheusをTower middlewareとして適用
  - エンドポイント別メトリクス収集 (pricing, greeks, xva)
  - GET /metrics でPrometheusスクレイプエンドポイント公開

### Structured Logging (tracing)
- **Context**: Requirement 1.6で構造化ログ必須
- **Sources Consulted**:
  - [Axum App tracing with Rust's logging](https://carlosmv.hashnode.dev/adding-logging-and-tracing-to-an-axum-app-rust)
  - [A Gentle Introduction to Axum, Tracing, and Logging](https://ianbull.com/posts/axum-rust-tracing/)
  - [Building Modular Web Services with Axum Layers](https://leapcell.io/blog/building-modular-web-services-with-axum-layers-for-observability-and-security)
  - [How to Use Axum Middleware for Logging](https://www.ruststepbystep.com/how-to-use-axum-middleware-for-logging-in-rust-web-apps/)
- **Findings**:
  - tower-http TraceLayer でリクエストコンテキスト自動付与
  - tracing-subscriber でJSON構造化ログ出力
  - リクエストメタデータ (method, URI, request_id) が自動span含まれる
  - フィルター設定で実行時ログレベル調整可能
- **Implications**:
  - TraceLayer::new_for_http() をmiddleware stack適用
  - tracing-subscriber でJSON formatと環境変数ベースフィルター設定
  - エラー時にrequest_id、endpoint、input_paramsをログコンテキストに含む

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Layered API (Router分割) | エンドポイントグループ別にRouterモジュール分割 (pricing, greeks, xva, health) | モジュール性高、並行開発容易、テスト分離 | 軽微な複雑性増加 | 既存L1-L4レイヤーアーキテクチャと整合 |
| Monolithic Router | 全エンドポイントを単一ファイルで定義 | シンプル | 保守性低下、テスト結合度高 | 小規模プロジェクトのみ適用 |
| CQRS分離 | コマンド (POST) とクエリ (GET) をサービスレイヤー分離 | 読み書き最適化可能 | 過剰設計、現状は全てコマンド (POST) | 将来拡張時に検討 |

**Selected**: Layered API (Router分割) - 既存アーキテクチャ原則と一貫性、モジュール性確保

## Design Decisions

### Decision: Layer 5 としての stable Rust実装
- **Context**: pricer_kernel (L3) はnightly必須だが、L5はHTTP APIレイヤーでnightly不要
- **Alternatives Considered**:
  1. L5全体をnightly化 → デプロイメント複雑化、Docker imageサイズ増
  2. L3依存を必須化 → stable環境でビルド不可
- **Selected Approach**: L3をoptional feature (`kernel-integration`) として提供
- **Rationale**:
  - 既存の4層分離戦略維持
  - Dockerfileで stable版 (L1/L2/L4/L5) と nightly版 (全層) を分離可能
  - プロダクションデプロイでL3機能不要な場合、stable版のみ使用可能
- **Trade-offs**:
  - Benefits: デプロイ柔軟性、stable環境対応
  - Compromises: feature flag管理必要、L3機能はoptional
- **Follow-up**: CI/CDで両パターンテスト (stable-only, stable+nightly)

### Decision: Axum + utoipa + tower-httpスタック
- **Context**: Web API実装の技術スタック選定
- **Alternatives Considered**:
  1. Actix-web → パフォーマンス高いがTowerエコシステム外
  2. Rocket → 簡潔だがマクロ多用、型安全性低
  3. warp → Filterベース、学習曲線急
- **Selected Approach**: Axum + utoipa + tower-http
- **Rationale**:
  - Axumはマクロフリー、型安全、Towerネイティブ
  - utoipaでコンパイル時OpenAPI生成
  - tower-httpで標準middleware (tracing, CORS, compression)
- **Trade-offs**:
  - Benefits: 型安全性、エコシステム統合、保守性
  - Compromises: Actix-webよりわずかに低速 (が、hyperと同等レベル)
- **Follow-up**: ベンチマーク実施 (criterion) でレイテンシー計測

### Decision: API Request/Response型の完全型安全化
- **Context**: Requirement 2でJSON入出力、serde使用
- **Alternatives Considered**:
  1. serde_json::Value → 型安全性なし、実行時エラー
  2. 部分的型定義 → バリデーション漏れリスク
- **Selected Approach**: 全Request/Response型を明示的struct定義 + utoipa derive
- **Rationale**:
  - コンパイル時型チェックで実行時エラー防止
  - OpenAPIスキーマ自動生成
  - バリデーションロジック一元化
- **Trade-offs**:
  - Benefits: 型安全性、ドキュメント自動生成
  - Compromises: ボイラープレートコード増加
- **Follow-up**: リクエストバリデーションテスト (proptestでfuzzing)

### Decision: Optional Enzyme Greeks (feature flag)
- **Context**: Requirement 4でGreeks計算、L3 Enzyme使用可能だがnightly必須
- **Alternatives Considered**:
  1. Enzymeのみ → stable環境で動作不可
  2. num-dual fallback → パフォーマンス低下
- **Selected Approach**: kernel-integration feature flagでL3オプショナル化、feature無効時はエラー返却
- **Rationale**:
  - stable版デプロイでもビルド成功
  - nightly版でEnzyme Greeks有効化
  - プロダクション要件に応じたデプロイ戦略選択可能
- **Trade-offs**:
  - Benefits: デプロイ柔軟性
  - Compromises: ランタイムfeatureチェック必要、一部APIがconditional
- **Follow-up**: Feature無効時のHTTP 501 Not Implemented返却

### Decision: tower_governor によるレート制限
- **Context**: Requirement 9でレート制限とAPI key認証
- **Alternatives Considered**:
  1. tokio-rate-limit → 高性能だが新しいcrate、コミュニティ小
  2. カスタム実装 → メンテナンスコスト高
- **Selected Approach**: tower_governor + カスタムAPI key extractor
- **Rationale**:
  - Towerネイティブ、Axum統合容易
  - 複数キー抽出戦略 (IP、API key) 対応
  - コミュニティ実績あり
- **Trade-offs**:
  - Benefits: エコシステム統合、保守性
  - Compromises: tokio-rate-limitより若干低速 (が、十分なパフォーマンス)
- **Follow-up**: レート制限テスト (負荷試験でHTTP 429確認)

## Risks & Mitigations
- **Risk 1: pricer_kernel (L3) の nightly依存がL5ビルド制約に** → Mitigation: feature flagでオプショナル化、stable-only版提供
- **Risk 2: 大規模ポートフォリオXVA計算でタイムアウト (30秒制約)** → Mitigation: タイムアウト設定可能化、非同期ジョブキューへの移行パス検討
- **Risk 3: OpenAPIスキーマとRequest型の不整合** → Mitigation: 統合テストでOpenAPI生成検証、reqwest clientでスキーマ準拠確認
- **Risk 4: API keyストレージ未定義** → Mitigation: Phase 1では環境変数/設定ファイルでキー管理、将来的にはデータベース/Vault統合検討
- **Risk 5: CORS設定がフロントエンド要件と不整合** → Mitigation: 設定可能なCORS originリスト、開発環境では緩和設定

## References
- [Axum公式ドキュメント](https://docs.rs/axum/latest/axum/) — Webフレームワーク
- [utoipa GitHub](https://github.com/juhaku/utoipa) — OpenAPI生成
- [Tokio Graceful Shutdown](https://tokio.rs/tokio/topics/shutdown) — シグナルハンドリング
- [tower_governor crate](https://crates.io/crates/tower_governor) — レート制限
- [axum-prometheus crate](https://docs.rs/axum-prometheus/latest/axum_prometheus/) — メトリクス
- [Serde公式ドキュメント](https://serde.rs/) — JSON serialization
- [tower-http TraceLayer](https://docs.rs/tower-http/latest/tower_http/trace/index.html) — 構造化ログ
