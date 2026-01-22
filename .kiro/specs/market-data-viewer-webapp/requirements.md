# Requirements Document

## Introduction

本仕様は、Frictional Bank Web App 向けのマーケットデータ閲覧機能を定義する。USD、EUR、JPY の主要通貨について、各アセットクラス（Rates、FX）のマーケットレートデータセットを整備し、Instrument と Convention の紐付け情報を含む包括的なデータビューア画面を Web App に実装する。

対象配置:
- データモデル: `demo/gui/src/web/` （型定義、ハンドラー）
- フロントエンド: `demo/gui/static/` （HTML/CSS/JS）

本機能は既存の `infra_master::market` モジュール（`MarketRate`, `RateId`, `RateType`, `RateIndex` 等）と `infra_master::trade::convention` モジュール（`SwapConvention`, `FxConvention` 等）を活用し、Web App 上でこれらの情報を統合表示する。

## Requirements

### Requirement 1: マーケットレートデータセット定義

**Objective:** As a Risk Manager, I want USD/EUR/JPY の主要マーケットレートを体系的に閲覧したい, so that 現在のマーケット状況を迅速に把握できる

#### Acceptance Criteria

1. The Market Data Viewer shall provide pre-configured market rate datasets for USD, EUR, and JPY currencies
2. When the viewer is initialised, the system shall load default rate sets containing:
   - **USD**: SOFR (ON, 1W, 1M, 3M, 6M, 1Y), Swap rates (2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y), FX forward points (USDJPY, EURUSD)
   - **EUR**: EURIBOR 3M/6M rates (1M, 3M, 6M, 1Y), EUR swap rates (2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y)
   - **JPY**: TONAR (ON, 1W, 1M, 3M, 6M, 1Y), JPY swap rates (2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y)
3. The Market Data Viewer shall categorise rates by `RateType` (Deposit, Fra, Futures, Swap, Ois, BasisSwap, FxSpot, FxForward)
4. While displaying rate data, the system shall show rate ID, tenor, rate type, value, quote type (Bid/Ask/Mid), and timestamp
5. If a rate is unavailable or stale, the system shall display a visual indicator (warning icon or greyed-out row)

### Requirement 2: Instrument 紐付け情報表示

**Objective:** As a Quant Developer, I want 各マーケットレートが紐付く Instrument を確認したい, so that カーブ構築に使用される商品の詳細を理解できる

#### Acceptance Criteria

1. The Market Data Viewer shall display the linked `Instrument` type for each market rate (Deposit, ParSwap, Ois, Futures, FxForward)
2. When a rate row is selected, the system shall show detailed instrument information in a side panel or modal:
   - Currency
   - Start date
   - End date (maturity)
   - Rate value
   - Instrument-specific parameters (e.g., notional for swaps, price for futures)
3. The Market Data Viewer shall use `StandardInstrumentMapper` to convert rates to instruments on-demand
4. If an instrument mapping fails, the system shall display "Mapping unavailable" with the error reason from `MappingError`
5. Where the `serde` feature is enabled, the system shall provide JSON export for instrument data

### Requirement 3: Convention 情報表示

**Objective:** As a Quant Developer, I want 各 Instrument に対応する Convention 設定を確認したい, so that 商品のコンベンション詳細（Day Count、Payment Frequency 等）を把握できる

#### Acceptance Criteria

1. The Market Data Viewer shall display convention information for each instrument type:
   - **Swap**: `SwapConvention` (fixed leg day count, float leg day count, payment frequency, calendar, spot lag, float index)
   - **FX**: `FxConvention` (base currency, quote currency, spot days, value date calendar)
2. When viewing a rate, the system shall show the applicable convention in a structured table format
3. The Market Data Viewer shall provide pre-configured conventions for:
   - USD SOFR swaps (`SwapConvention::usd_sofr()`)
   - EUR EURIBOR 6M swaps (`SwapConvention::eur_euribor_6m()`)
   - JPY TONAR swaps (`SwapConvention::jpy_tonar()`)
   - Major FX pairs (EUR/USD, USD/JPY)
4. The system shall display convention field labels in a user-friendly format (e.g., "Day Count: ACT/360" instead of raw enum values)
5. Where multiple conventions exist for the same currency, the system shall indicate the default convention

### Requirement 4: Web App データ閲覧画面

**Objective:** As a User, I want マーケットデータを見やすい画面で閲覧したい, so that 必要な情報を効率的に確認できる

#### Acceptance Criteria

1. The Web App shall provide a dedicated Market Data page accessible from the main navigation
2. The Market Data page shall include the following UI components:
   - Currency selector (tabs or dropdown: USD, EUR, JPY, All)
   - Rate type filter (multi-select: Deposit, Swap, FX, etc.)
   - Search box for filtering by rate ID or tenor
   - Sortable data table with columns: Rate ID, Currency, Tenor, Type, Value, Quote, Timestamp
3. When a currency is selected, the system shall filter and display only rates for that currency
4. When a rate row is clicked, the system shall expand or open a detail panel showing:
   - Full rate metadata
   - Linked instrument details
   - Convention information
5. The Market Data page shall support responsive layout for desktop and tablet screen sizes
6. While data is loading, the system shall display a loading indicator

### Requirement 5: REST API エンドポイント

**Objective:** As a Developer, I want REST API 経由でマーケットデータにアクセスしたい, so that 外部システムやスクリプトからデータを取得できる

#### Acceptance Criteria

1. The Web API shall provide a `/api/market-data/rates` endpoint that returns all market rates as JSON
2. The `/api/market-data/rates` endpoint shall support query parameters:
   - `currency` (optional): Filter by currency code (USD, EUR, JPY)
   - `type` (optional): Filter by rate type (deposit, swap, fx_spot, fx_forward)
   - `index` (optional): Filter by rate index (SOFR, EURIBOR, TONAR)
3. The Web API shall provide a `/api/market-data/rates/{rate_id}` endpoint that returns a single rate with full details
4. When a rate is requested, the response shall include:
   - Rate information (id, currency, tenor, type, value, quote_type, timestamp, source)
   - Linked instrument (if mappable)
   - Applicable convention
5. The Web API shall provide a `/api/market-data/conventions` endpoint that returns all available conventions
6. The Web API shall provide a `/api/market-data/conventions/{convention_id}` endpoint for specific convention lookup
7. If a requested resource is not found, the API shall return HTTP 404 with a structured error response

### Requirement 6: データ更新とリフレッシュ

**Objective:** As a User, I want マーケットデータを最新の状態で閲覧したい, so that 正確な情報に基づいて判断できる

#### Acceptance Criteria

1. The Market Data page shall display the last update timestamp for the rate set
2. The Market Data page shall provide a manual refresh button to reload rate data
3. When the refresh button is clicked, the system shall fetch updated rates from the backend and refresh the display
4. While rates are being refreshed, the system shall indicate the refresh state (loading spinner)
5. If the refresh fails, the system shall display an error message with retry option
6. The system shall highlight rates that have changed since the last refresh (e.g., colour indicator for up/down movement)

### Requirement 7: データエクスポート機能

**Objective:** As a Risk Manager, I want マーケットデータをエクスポートしたい, so that 外部システムやレポートで使用できる

#### Acceptance Criteria

1. The Market Data page shall provide an export button for downloading rate data
2. The export function shall support CSV format with columns: Rate ID, Currency, Tenor, Type, Value, Quote Type, Timestamp, Source
3. The export function shall support JSON format with full rate details including instrument and convention information
4. When exporting, the system shall apply the current filter settings (currency, type filters)
5. The exported file shall include a header row (CSV) or metadata section (JSON) with export timestamp and filter criteria

---

## Non-Functional Requirements

### NFR 1: パフォーマンス

1. The Market Data page shall load initial rate data within 2 seconds under normal conditions
2. The rate table shall render up to 500 rates without noticeable lag
3. The API endpoints shall respond within 500ms for typical requests

### NFR 2: ユーザビリティ

1. The Market Data page shall follow the existing Frictional Bank Web App design patterns
2. All interactive elements shall provide visual feedback on hover and click
3. Error messages shall be displayed in user-friendly language (not raw error codes)

### NFR 3: 互換性

1. The Web App shall support modern browsers (Chrome, Firefox, Safari, Edge - latest 2 versions)
2. The REST API shall return JSON responses with proper Content-Type headers
3. The implementation shall integrate with existing `demo/gui` infrastructure (Axum handlers, static files)

---

## Out of Scope

- リアルタイムマーケットデータフィードの接続（adapter_feeds の責務）
- マーケットレートの編集・更新機能（読み取り専用ビューア）
- ヒストリカルデータの表示（本フェーズはスナップショットデータのみ）
- カーブ構築・キャリブレーション機能（既存の Bootstrap 画面の責務）
- 認証・認可機能（デモアプリケーションのため）

## References

- [structure.md](../../steering/structure.md) - A-I-P-S アーキテクチャ定義
- [tech.md](../../steering/tech.md) - 技術スタック
- [market-rate-infrastructure](../market-rate-infrastructure/requirements.md) - マーケットレートインフラ仕様
- `infra_master::market` - マーケットデータ型定義
- `infra_master::trade::convention` - コンベンション定義
