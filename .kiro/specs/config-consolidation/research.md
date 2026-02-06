# Research & Design Decisions: config-consolidation

## Summary
- **Feature**: config-consolidation
- **Discovery Scope**: Extension（既存システムへの機能追加）
- **Key Findings**:
  - `strum` の `VariantNames` マクロで Enum バリアント名を `&'static [&'static str]` として自動取得可能
  - `strum(serialize_all = "snake_case")` で既存の serde 設定と完全互換
  - `serde_json::to_value()` で `Default` trait 実装済み構造体を動的に JSON 変換可能

## Research Log

### strum マクロによる Enum 列挙の自動化
- **Context**: 要件1「Enumエクスポート機能」で全バリアント名をJSON配列として出力する必要がある
- **Sources Consulted**:
  - [strum - GitHub](https://github.com/Peternator7/strum)
  - [strum - docs.rs](https://docs.rs/strum)
  - [EnumIter in strum](https://docs.rs/strum/latest/strum/derive.EnumIter.html)
- **Findings**:
  - `VariantNames` derive で `VARIANTS: &'static [&'static str]` 定数を自動生成
  - `#[strum(serialize_all = "snake_case")]` で snake_case 形式の文字列を取得
  - unit variant（データを持たないバリアント）のみサポート → 対象Enumはすべて unit variant なので問題なし
  - strum 0.26 が最新安定版（2024年リリース）
- **Implications**:
  - 手動でのEnum値配列定義が不要（コード量削減）
  - 新バリアント追加時に自動で反映（メンテナンス性向上）

### serde_json による Default 値のエクスポート
- **Context**: 要件2「デフォルト値エクスポート機能」で構造体のデフォルト値をJSONとして出力
- **Sources Consulted**: serde_json 公式ドキュメント
- **Findings**:
  - `serde_json::to_value(&T::default())` で `serde::Value` に変換可能
  - `serde::Value::Object` を merge して階層的な JSON を構築可能
  - 既存の `Default` 実装をそのまま利用可能
- **Implications**:
  - 追加のデフォルト値定義が不要
  - Rust側の `Default` 実装が Single Source of Truth

### service_gateway ルーター構造
- **Context**: 要件5「/api/config エンドポイント」の追加位置
- **Findings**:
  - 既存: `/api` と `/api/v1` の2つのルートグループ
  - `/api/config` は `/api` ルートグループに追加（バージョンなし、設定は共通）
  - 既存パターン: `handlers::health` のようにシンプルなGETハンドラー
- **Implications**:
  - `handlers/config.rs` を新規追加し、`api_routes()` にルートを追加

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| strum 自動化 | `VariantNames` マクロで全バリアント自動列挙 | コード量最小、自動メンテナンス | 外部依存追加 | プロジェクト内で既に広く使用（116ファイル） |
| 手動実装 | 各Enumに `EnumValues` trait を手動実装 | 外部依存なし | コード量多、手動メンテナンス必要 | - |

**選択**: strum 自動化（ユーザー要望「コード量削減」に合致）

## Design Decisions

### Decision: strum マクロの採用
- **Context**: Enum バリアント名をJSON配列として出力する必要がある
- **Alternatives Considered**:
  1. 手動配列定義 — 各Enumに `const VALUES: &[&str]` を手動定義
  2. strum マクロ — `VariantNames` derive で自動生成
- **Selected Approach**: strum の `VariantNames` + `serialize_all` 属性
- **Rationale**:
  - コード量削減（ユーザー要望）
  - 自動メンテナンス（新バリアント追加時に更新不要）
  - プロジェクト内で既に広く使用されているため導入障壁が低い
- **Trade-offs**:
  - ✅ コード量大幅削減（各Enumで約10行削減）
  - ✅ タイプセーフ（コンパイル時に検証）
  - ❌ 外部依存追加（strum, strum_macros）
- **Follow-up**: serde と strum の `serialize_all` 設定が一致していることを確認

### Decision: 単一の `app_config.rs` ファイル
- **Context**: AppConfig, EnumRegistry, DefaultsRegistry, CurrencyRateIndexMap の配置場所
- **Alternatives Considered**:
  1. `lib.rs` に直接追加
  2. 新規 `app_config.rs` ファイル作成
- **Selected Approach**: `app_config.rs` 新規ファイル
- **Rationale**:
  - 単一責任の原則（設定エクスポート機能を分離）
  - `lib.rs` の肥大化を防止
- **Trade-offs**:
  - ✅ 責務分離が明確
  - ❌ ファイル数増加（1ファイル）

### Decision: `/api/config` の配置
- **Context**: エンドポイントのURLパス設計
- **Alternatives Considered**:
  1. `/api/config` — バージョンなし
  2. `/api/v1/config` — バージョン付き
- **Selected Approach**: `/api/config`（バージョンなし）
- **Rationale**:
  - 設定情報はAPIバージョンに依存しない共通リソース
  - フロントエンドの既存実装が `/api/config` を呼び出している
- **Trade-offs**:
  - ✅ フロントエンド互換性維持
  - ✅ シンプルなURLパス

## Risks & Mitigations
- **Risk**: strum と serde の serialize 設定が不一致 → **Mitigation**: 両方で `serialize_all = "snake_case"` を明示的に指定
- **Risk**: 新Enum追加時に `EnumRegistry` への登録漏れ → **Mitigation**: マクロ or ドキュメントでチェックリスト提供
- **Risk**: フロントエンドとの型不一致 → **Mitigation**: TypeScript の `AppConfig` 型定義と照合テスト

## References
- [strum - GitHub](https://github.com/Peternator7/strum) — Rust enum derive macros
- [strum - docs.rs](https://docs.rs/strum) — API documentation
- [EnumIter in strum](https://docs.rs/strum/latest/strum/derive.EnumIter.html) — Iterator derive
- [VariantNames - TiKV docs](https://tikv.github.io/doc/strum_macros/derive.EnumVariantNames.html) — VARIANTS constant
