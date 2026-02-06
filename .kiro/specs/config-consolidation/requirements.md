# Requirements Document

## Introduction

`infra_config` クレートを設定のSingle Source of Truthとして確立し、Rust側の型定義・Enum定義から直接JSONをエクスポートする機能を追加する。これにより `demo/data/config/` の重複ファイル（enums.json、gui_defaults.json）への依存を排除し、フロントエンドとの同期リスクを解消する。

### 現状の課題

1. **設定の分散**: 同じEnumやデフォルト値が `infra_config`（Rust）と `demo/data/config/`（JSON）に重複して定義されている
2. **同期リスク**: Rustで新しい列挙値を追加しても、JSONファイルは手動更新が必要で乖離が発生しやすい
3. **エンドポイント未実装**: フロントエンドの `ConfigLoader` が期待する `/api/config` エンドポイントが存在しない

### 目標

- `infra_config` から自動的にEnum値とデフォルト値をJSON形式でエクスポート
- `service_gateway` の `/api/config` エンドポイントで `AppConfig` を提供
- JSONファイルの手動メンテナンスを不要にし、Single Source of Truthを確立

---

## Requirements

### Requirement 1: Enumエクスポート機能

**Objective:** As a フロントエンド開発者, I want `infra_config` のRust enumからJSON形式でEnum値一覧を取得したい, so that フロントエンドのドロップダウンやフォームで常に最新のEnum値を使用できる

#### Acceptance Criteria

1. The `infra_config` crate shall エクスポート対象のEnumに対して `EnumValues` traitを実装し、`to_json_array()` メソッドでJSON配列を返す
2. The `infra_config` crate shall 以下のEnumをエクスポート対象とする: `PricingMethod`, `GreeksMethod`, `GreekType`, `TreeType`, `ShiftType`, `SecondOrderMode`
3. When `EnumRegistry::to_json()` が呼び出された場合, the `infra_config` crate shall 全エクスポート対象Enumを `{ "enum_name": ["value1", "value2", ...] }` 形式のJSONオブジェクトとして返す
4. The `infra_config` crate shall Enum値をsnake_case文字列（例: `monte_carlo`, `bump`）として出力する（既存のserde設定と一致）

### Requirement 2: デフォルト値エクスポート機能

**Objective:** As a フロントエンド開発者, I want 各設定構造体のデフォルト値をJSON形式で取得したい, so that フォームの初期値を常にRust側の定義と同期できる

#### Acceptance Criteria

1. The `infra_config` crate shall `PricingConfig`, `RiskConfig`, `MonteCarloParams`, `TreeParams`, `BumpSizes` の各構造体に対して `Default` traitが実装されていることを前提とし、`DefaultsRegistry::to_json()` メソッドでJSONオブジェクトを返す
2. The `infra_config` crate shall デフォルト値を階層的なJSONオブジェクト（例: `{ "pricing": { ... }, "risk": { ... } }`）として出力する
3. When 新しい設定構造体が追加された場合, the `infra_config` crate shall `DefaultsRegistry` に追加するだけでエクスポート対象に含まれる設計とする

### Requirement 3: 通貨→金利インデックスマッピング

**Objective:** As a フロントエンド開発者, I want 通貨コードから対応する金利インデックス名を取得したい, so that 通貨選択時に適切な金利インデックスを自動提案できる

#### Acceptance Criteria

1. The `infra_config` crate shall `CurrencyRateIndexMap` 構造体を提供し、通貨コード（例: `USD`）から金利インデックス名（例: `SOFR`）へのマッピングを返す
2. The `infra_config` crate shall 主要通貨（USD→SOFR, EUR→ESTR, GBP→SONIA, JPY→TONA）のデフォルトマッピングを内蔵する
3. When 設定ファイルでカスタムマッピングが指定された場合, the `infra_config` crate shall デフォルトマッピングを上書きする
4. The `CurrencyRateIndexMap` shall `to_json()` メソッドで `{ "USD": "SOFR", "EUR": "ESTR", ... }` 形式のJSONを返す

### Requirement 4: AppConfig統合構造体

**Objective:** As a service_gateway開発者, I want 上記3機能を統合した単一の構造体を取得したい, so that `/api/config` エンドポイントで一括提供できる

#### Acceptance Criteria

1. The `infra_config` crate shall `AppConfig` 構造体を提供し、以下のフィールドを含む: `enums`, `defaults`, `rate_index_by_currency`
2. The `AppConfig` struct shall `serde::Serialize` を実装し、JSON形式でシリアライズ可能とする
3. When `AppConfig::build()` が呼び出された場合, the `infra_config` crate shall `EnumRegistry`, `DefaultsRegistry`, `CurrencyRateIndexMap` から自動的に構築する
4. The `AppConfig` struct shall フロントエンドの `ConfigLoader` が期待する形式（`enums: Record<string, string[]>`, `defaults: Record<string, unknown>`, `rateIndexByCurrency: Record<string, string>`）と互換性を持つ

### Requirement 5: /api/config エンドポイント

**Objective:** As a フロントエンド, I want `/api/config` エンドポイントから `AppConfig` を取得したい, so that アプリケーション起動時に設定を一括ロードできる

#### Acceptance Criteria

1. The `service_gateway` crate shall `GET /api/config` エンドポイントを提供する
2. When `/api/config` にGETリクエストが送信された場合, the `service_gateway` shall `infra_config::AppConfig::build()` を呼び出し、JSONレスポンスを返す
3. The `/api/config` endpoint shall `Content-Type: application/json` ヘッダを含むレスポンスを返す
4. If `AppConfig::build()` が失敗した場合, the `service_gateway` shall HTTP 500エラーとエラーメッセージを返す

### Requirement 6: 重複ファイルの非推奨化

**Objective:** As a プロジェクトメンテナ, I want 重複JSONファイルを非推奨化したい, so that Single Source of Truthを維持しメンテナンスコストを削減できる

#### Acceptance Criteria

1. The project shall `demo/data/config/enums.json` を非推奨とし、ファイル先頭にコメントで `infra_config::AppConfig` への移行を案内する
2. The project shall `demo/data/config/gui_defaults.json` を非推奨とし、ファイル先頭にコメントで `infra_config::AppConfig` への移行を案内する
3. The project shall `demo/data/config/rate_index_mapping.json` を非推奨とし、ファイル先頭にコメントで `infra_config::CurrencyRateIndexMap` への移行を案内する
4. While 移行期間中, the `demo/gui` ConfigLoader shall `/api/config` エンドポイントが利用可能な場合はそちらを優先し、フォールバックとして既存JSONファイルを使用する

---

## Out of Scope

- `demo/data/input/` ディレクトリの市場データファイル（curves、irvol、fxvol等）は本仕様の対象外
- `demo/data/config/scenarios/` のシナリオファイルは本仕様の対象外
- `settings.toml` 等のシステム設定ファイルは既存の `Settings::load()` 機能を継続使用

## Dependencies

- A-I-P-S依存ルール: `infra_config`（Iレイヤー）は `service_gateway`（Sレイヤー）に依存しない
- `serde`, `serde_json` クレートが `infra_config` で利用可能であること
