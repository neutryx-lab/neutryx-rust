# Implementation Plan: config-consolidation

## Tasks

- [x] 1. strum依存追加とEnum derive属性の追加
- [x] 1.1 infra_config に strum 依存を追加
  - Cargo.toml に `strum` と `strum_macros` を workspace 依存として追加
  - バージョン 0.26 を使用（ワークスペースで一元管理）
  - _Requirements: 1.1_

- [x] 1.2 pricing_config の Enum に strum derive を追加
  - `PricingMethod` に `strum::VariantNames` と `strum(serialize_all = "snake_case")` を追加
  - `TreeType` に同様の derive を追加
  - 既存の serde 設定との整合性を確認
  - _Requirements: 1.2, 1.4_

- [x] 1.3 risk_config の Enum に strum derive を追加
  - `GreeksMethod`, `GreekType`, `SecondOrderMode`, `ShiftType` に `strum::VariantNames` を追加
  - `strum(serialize_all = "snake_case")` を追加
  - 既存の serde 設定との整合性を確認
  - _Requirements: 1.2, 1.4_

- [x] 2. AppConfig モジュールの実装
- [x] 2.1 EnumRegistry を実装
  - `EnumRegistry::to_json()` で全対象 Enum のバリアント名を JSON オブジェクトとして返す
  - strum の `VariantNames::VARIANTS` を使用して自動列挙
  - キー名を snake_case に変換（`PricingMethod` → `pricing_method`）
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [x] 2.2 DefaultsRegistry を実装
  - `DefaultsRegistry::to_json()` で Default 実装済み構造体のデフォルト値を返す
  - `MonteCarloParams`, `TreeParams`, `BumpSizes` を対象
  - `serde_json::to_value(&T::default())` で JSON に変換
  - 階層的な JSON オブジェクトとして構造化
  - _Requirements: 2.1, 2.2, 2.3_

- [x] 2.3 CurrencyRateIndexMap を実装
  - デフォルトマッピング（USD→SOFR, EUR→ESTR, GBP→SONIA, JPY→TONA, CHF→SARON）を内蔵
  - `to_json()` で HashMap を JSON オブジェクトとして返す
  - _Requirements: 3.1, 3.2, 3.4_

- [x] 2.4 AppConfig 構造体を実装
  - `enums`, `defaults`, `rate_index_by_currency` フィールドを持つ構造体
  - `#[serde(rename_all = "camelCase")]` でフロントエンド互換のフィールド名
  - `AppConfig::build()` で 3 つのレジストリから構築
  - _Requirements: 4.1, 4.2, 4.3, 4.4_

- [x] 2.5 lib.rs に app_config モジュールを追加
  - `mod app_config` を追加
  - `AppConfig` を prelude にエクスポート
  - _Requirements: 4.1_

- [x] 3. /api/config エンドポイントの実装
- [x] 3.1 config handler を実装
  - `get_config` ハンドラーで `AppConfig::build()` を呼び出し
  - JSON レスポンスを返す
  - エラー時は HTTP 500 を返す
  - _Requirements: 5.2, 5.3, 5.4_

- [x] 3.2 ルーターにエンドポイントを追加
  - `api_routes()` に `.route("/config", get(handlers::get_config))` を追加
  - handlers/mod.rs で `pub mod config` を追加し re-export
  - _Requirements: 5.1_

- [x] 4. 重複ファイルの非推奨化
  - `demo/data/config/enums.json` 先頭に非推奨コメントを追加
  - `demo/data/config/gui_defaults.json` 先頭に非推奨コメントを追加
  - `demo/data/config/rate_index_mapping.json` 先頭に非推奨コメントを追加
  - コメントで `infra_config::AppConfig` への移行を案内
  - _Requirements: 6.1, 6.2, 6.3_

- [x] 5. テストと検証
- [x] 5.1 ユニットテストを追加
  - `EnumRegistry::to_json()` が全対象 Enum を含むことを検証
  - `DefaultsRegistry::to_json()` が期待するデフォルト値を返すことを検証
  - `CurrencyRateIndexMap::to_json()` がデフォルトマッピングを含むことを検証
  - `AppConfig::build()` がフロントエンド互換の構造を返すことを検証
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 3.1, 3.2, 4.1, 4.2, 4.3, 4.4_

- [x] 5.2 統合テストを追加
  - `GET /api/config` が 200 OK と JSON を返すことを検証
  - Content-Type が `application/json` であることを検証
  - _Requirements: 5.1, 5.2, 5.3_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1-1.4 | 1.1, 1.2, 1.3, 2.1, 5.1 |
| 2.1-2.3 | 2.2, 5.1 |
| 3.1-3.4 | 2.3, 5.1 |
| 4.1-4.4 | 2.4, 2.5, 5.1 |
| 5.1-5.4 | 3.1, 3.2, 5.2 |
| 6.1-6.4 | 4 |

## Parallel Execution Summary

以下のタスクは並列実行可能:
- **1.2 と 1.3**: 異なるファイルの Enum 修正（pricing_config.rs vs risk_config.rs）
- **2.2 と 2.3**: 独立したレジストリ実装（DefaultsRegistry vs CurrencyRateIndexMap）
- **4**: 他の実装タスクと並行して非推奨コメント追加可能
