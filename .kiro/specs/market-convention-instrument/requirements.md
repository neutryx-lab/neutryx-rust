# Requirements Document

## Project Description (Input)
crates\infra_domain内のcrates\infra_domain\src\marketとcrates\infra_domain\src\trade\conventionの再定義を行いたい。マーケットレートには対応するMarketConventionがあり、それと合わせることでTradeExpandできるInstrumentとして定義できる(Eventsも同様に日付とSpreadを持つようなInstrumentとできる)、としたい。現状のdemo_guiを出発点に、あらゆる通貨のdemo\data\inputを正し以下たちに修正して用意し、MarketData画面ではRateを選択すると対応するConvention情報と合わせてInstrumentとしての情報をRateDetailとして表示し、TradeExpand画面を廃止して、MarketData画面下部にそのInsturumentをCFにまで展開して表示するようにする。

## Introduction

本仕様は `infra_domain` クレートにおけるマーケットデータとコンベンションの統合アーキテクチャを定義する。MarketRate と MarketConvention を組み合わせて Instrument として統一的に扱い、demo GUI の MarketData 画面で Rate 選択時に Instrument 詳細とキャッシュフロー展開を表示する機能を実現する。

### 設計原則: RateIndex と MarketConvention の分離

本仕様では、`RateIndex` と `MarketConvention` を明確に異なる概念として扱う：

| 概念 | 責務 | 例 |
|------|------|-----|
| **RateIndex** | 浮動金利の参照インデックス（RFR/IBOR）を表す | SOFR, ESTR, EURIBOR 3M, SONIA, TONAR |
| **MarketConvention** | 商品種別 × 通貨の取引慣行を表す | USD OIS Convention, EUR Swap Convention |

**関係性:**
- `RateIndex` は浮動金利 leg の参照インデックスとして `MarketConvention` 内で使用される
- `MarketConvention` は `RateIndex.metadata()` を拡張するものではなく、独立した商品定義メタデータ
- Swap Convention は `RateIndex` を参照して floating leg の fixing を定義する

```
MarketRate (USD 5Y OIS, rate=3.42%)
    │
    ├── RateId
    │     └── rate_index: Option<RateIndex::Sofr>  ← 参照インデックス
    │
    └── MarketConvention::Ois(OisConvention)       ← 商品取引慣行
          ├── fixed_leg: FixedLegConvention
          │     ├── day_count: Actual360
          │     ├── frequency: Annual
          │     └── payment_lag: 2
          └── floating_leg: FloatingLegConvention
                ├── rate_index: RateIndex::Sofr    ← RateIndex を参照
                ├── compounding: Compounded
                └── payment_lag: 2
```

この分離により：
1. `RateIndex` は参照インデックスの metadata（fixing lag, compounding method）に集中
2. `MarketConvention` は商品固有の取引慣行（payment frequency, roll convention）を定義
3. 両者の組み合わせで完全な CF 展開が可能

### データ関連図: Index を起点とした紐付け

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           RateIndex                                      │
│                    (SOFR, ESTR, EURIBOR 3M...)                          │
│                                                                          │
│  ┌─ IndexMetadata ──────────────────────────────────────────────────┐   │
│  │  compounding_method, fixing_lag, settlement_lag, calendar        │   │
│  └──────────────────────────────────────────────────────────────────┘   │
└───────────────────────────────┬─────────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
        ▼                       ▼                       ▼
┌───────────────────┐  ┌───────────────────┐  ┌───────────────────────────┐
│   MarketRate      │  │ MarketConvention  │  │ (Indirect via Rate+Conv)  │
│                   │  │                   │  │                           │
│ RateId:           │  │ SwapConvention:   │  │     MarketInstrument      │
│  └─ rate_index    │──│  └─ floating_leg  │──│                           │
│     = SOFR        │  │      .rate_index  │  │  rate + convention        │
│                   │  │      = SOFR       │  │  → CF 展開可能            │
│ Examples:         │  │                   │  │                           │
│  - USD 1Y OIS     │  │ OisConvention:    │  └───────────────────────────┘
│  - USD 5Y OIS     │  │  └─ floating_leg  │
│  - USD 10Y OIS    │  │      .rate_index  │
└───────────────────┘  │      = SOFR       │
                       │                   │
                       │ FraConvention:    │
                       │  └─ rate_index    │
                       │      = SOFR       │
                       └───────────────────┘
```

**GUI での表示イメージ:**
- Index パネルで SOFR を選択 → 関連する全 Rate (1Y OIS, 5Y OIS...) と Convention (OIS Conv, FRA Conv) を表示
- Rate を選択 → Convention と合わせて MarketInstrument として詳細 + CF 展開を表示

### 用語定義

- **RateIndex**: 浮動金利の参照インデックス（SOFR, ESTR, EURIBOR 等）。`IndexMetadata` で fixing lag, compounding method を持つ
- **MarketRate**: マーケットから取得されるレート値（Deposit, Swap, FRA, FX Forward 等）
- **MarketConvention**: 商品種別 × 通貨に対応する市場慣行（DayCount, Frequency, Calendar, SettlementDays 等）
- **MarketInstrument**: MarketRate + MarketConvention を統合した、CF展開可能な商品定義
- **CF展開**: Instrument を個別のキャッシュフロー（支払日、利息計算期間、Notional、Rate）に分解すること

---

## Requirements

### Requirement 1: MarketConvention 型の定義

**Objective:** 開発者として、MarketRate に対応する市場慣行を型安全に表現したい。これにより、レートから Instrument への変換に必要なメタデータを一元管理できる。

#### Acceptance Criteria

1. The Infra Master shall provide a `MarketConvention` enum with variants for each rate type (Deposit, Swap, OIS, FRA, Futures, XCcyBasis, FxForward).
2. When a `MarketConvention` is created for a specific currency and rate type, the Infra Master shall provide default values for:
   - Day count convention (DayCountConvention)
   - Payment frequency (Frequency)
   - Business day convention (BusinessDayConvention)
   - Settlement days (SpotLag)
   - Fixing calendar (CalendarId)
3. Where the rate type is Swap or OIS, the `MarketConvention` shall include both fixed leg and floating leg conventions.
4. The Infra Master shall provide factory methods `MarketConvention::for_rate_id(rate_id: &RateId)` to automatically derive the appropriate convention.
5. If a rate type has no matching convention, the factory method shall return `None`.

---

### Requirement 2: MarketInstrument 型の定義

**Objective:** 開発者として、MarketRate と MarketConvention を統合した CF 展開可能な Instrument 型を定義したい。これにより、マーケットデータから直接トレード展開が可能になる。

#### Acceptance Criteria

1. The Infra Master shall provide a `MarketInstrument` struct containing:
   - Source rate reference (`RateId`)
   - Rate value (`f64`)
   - Associated convention (`MarketConvention`)
   - Valuation date (`Date`)
   - Maturity date (`Date`)
2. When a `MarketInstrument` is constructed from `MarketRate` and `MarketConvention`, the Infra Master shall calculate the maturity date from the rate tenor and valuation date.
3. The `MarketInstrument` shall implement a `to_trade()` method that returns a CF-expanded `Trade` struct.
4. While constructing a `MarketInstrument` for a swap rate, the Infra Master shall generate both fixed and floating legs with appropriate conventions.
5. If the rate value or convention is invalid, the construction shall return `MarketInstrumentError` with diagnostic information.

---

### Requirement 3: MarketRateSet から MarketInstrument への変換

**Objective:** 開発者として、既存の `MarketRateSet` から一括で `MarketInstrument` コレクションを生成したい。これにより、カーブ構築入力からシームレスに CF 展開が可能になる。

#### Acceptance Criteria

1. The `MarketRateSet` shall provide a method `to_instruments(&self, valuation_date: Date) -> Vec<MarketInstrument>` that converts all contained rates.
2. When converting rates, the MarketRateSet shall automatically lookup the appropriate `MarketConvention` for each rate type.
3. If a rate has no matching convention, the conversion shall skip that rate and log a warning.
4. The resulting `MarketInstrument` collection shall be sorted by maturity date in ascending order.
5. The conversion shall preserve the original rate metadata (source, quote type, timestamp).

---

### Requirement 4: イベントの Instrument 表現

**Objective:** 開発者として、中央銀行イベント（会合日程、政策金利決定）を Spread を持つ Instrument として表現したい。これにより、イベントリスクを CF ベースで分析できる。

#### Acceptance Criteria

1. The Infra Master shall provide an `EventInstrument` struct containing:
   - Event date (`Date`)
   - Event type (`EventType`)
   - Expected spread/change (`Option<f64>`)
   - Confidence level (`Option<f64>`)
2. When an `EventInstrument` is created for a central bank meeting, the Infra Master shall associate it with the relevant `RateIndex`.
3. The `EventInstrument` shall provide a method `impact_on_curve()` that returns the expected basis point impact on the yield curve.
4. Where historical data exists, the Infra Master shall provide `EventInstrument::from_historical(meeting: &CentralBankMeeting)` constructor.

---

### Requirement 5: デモデータの多通貨対応

**Objective:** デモユーザーとして、主要通貨（USD, EUR, GBP, JPY, CHF, AUD, CAD）のマーケットデータを利用したい。これにより、クロスカレンシー分析のデモが可能になる。

#### Acceptance Criteria

1. The Demo Data shall provide rate curve inputs for each supported currency:
   - USD: SOFR-based (deposit, swap, futures)
   - EUR: ESTR-based (deposit, swap)
   - GBP: SONIA-based (deposit, swap)
   - JPY: TONA-based (deposit, swap)
   - CHF: SARON-based (deposit, swap)
   - AUD: RBA Cash Rate-based (deposit, swap)
   - CAD: CORRA-based (deposit, swap)
2. When loading demo data, the Demo Service shall validate that all required tenors are present for each currency.
3. The Demo Data shall provide FX spot rates for all currency pairs involving USD as base or quote currency.
4. The Demo Data shall provide consistent valuation date across all rate files.
5. If a currency's data file is missing or invalid, the Demo Service shall return a descriptive error message including the expected file path.

---

### Requirement 6: デモデータのコンベンション定義

**Objective:** デモユーザーとして、各通貨・商品種別に対応するコンベンション定義を利用したい。これにより、正確な CF 展開が可能になる。

#### Acceptance Criteria

1. The Demo Data shall provide a `conventions.json` file containing conventions for all supported currency/rate type combinations.
2. When loading conventions, the Demo Service shall validate the JSON schema and return errors for invalid entries.
3. The conventions file shall include at minimum:
   - Spot lag (settlement days)
   - Day count convention
   - Payment frequency
   - Business day convention
   - Fixing calendar
   - Roll convention
4. Where a currency uses different conventions for different tenors (e.g., IMM dates for futures), the conventions file shall specify tenor-specific overrides.

---

### Requirement 7: MarketData 画面の Rate Detail 表示

**Objective:** デモユーザーとして、MarketData 画面で Rate を選択した際に、対応する Convention 情報と Instrument 詳細を確認したい。これにより、レートの意味と展開結果を理解できる。

#### Acceptance Criteria

1. When a rate is selected in the MarketData table, the Demo GUI shall display the Rate Detail panel containing:
   - Rate value and metadata (currency, tenor, type, source)
   - Associated convention details (day count, frequency, calendar, settlement)
   - Derived instrument information (effective date, maturity date, notional)
2. The Rate Detail panel shall show the instrument type derived from the rate type (e.g., "Par Swap 5Y" for a 5Y swap rate).
3. If the selected rate has no matching convention, the Demo GUI shall display "Convention not available" with the rate type shown.
4. The Rate Detail panel shall update within 100ms of rate selection.
5. While the rate detail is loading, the Demo GUI shall display a loading indicator.

---

### Requirement 8: MarketData 画面の CF 展開表示

**Objective:** デモユーザーとして、選択した Rate に対応する Instrument の CF 展開結果を MarketData 画面下部で確認したい。これにより、TradeExpand 画面に移動せずに CF 詳細を確認できる。

#### Acceptance Criteria

1. When a rate is selected, the Demo GUI shall automatically expand the corresponding instrument and display cashflows below the Rate Detail panel.
2. The CF display shall show a table with columns: Payment Date, Accrual Start, Accrual End, Year Fraction, Notional, Rate/Spread, Payoff Type.
3. Where the instrument has multiple legs (e.g., swap), the Demo GUI shall display each leg in a collapsible section.
4. The CF display shall indicate the leg direction (Payer/Receiver) with appropriate visual styling.
5. If the CF expansion fails, the Demo GUI shall display an error message with the failure reason.
6. The CF expansion shall complete within 500ms for standard instruments.

---

### Requirement 9: TradeExpand 画面の廃止

**Objective:** 開発者として、TradeExpand 画面を廃止し、その機能を MarketData 画面に統合したい。これにより、UI の簡素化とコード重複の削減が実現できる。

#### Acceptance Criteria

1. The Demo GUI shall remove the TradeExpand navigation item from the main menu.
2. When the Demo GUI is loaded, the TradeExpand component shall not be initialised.
3. The trade expansion API (`/api/trades/expand`) shall remain available for programmatic access.
4. The Demo GUI shall remove the `trade-expansion.ts` component file or mark it as deprecated.
5. If external links reference the TradeExpand view, the Demo GUI shall redirect to the MarketData view.

---

### Requirement 10: API エンドポイントの追加

**Objective:** 開発者として、Rate から Instrument および CF 展開を行う API エンドポイントを提供したい。これにより、GUI とバックエンドの連携が実現できる。

#### Acceptance Criteria

1. The Demo Web API shall provide `GET /api/market/rates/{rate_id}/instrument` returning the `MarketInstrument` for the specified rate.
2. The Demo Web API shall provide `GET /api/market/rates/{rate_id}/cashflows` returning the CF expansion of the rate's instrument.
3. When the rate ID is not found, the API shall return HTTP 404 with error details.
4. The API response shall include processing time metadata for performance monitoring.
5. The API shall support an optional `valuation_date` query parameter (default: today).
6. If the convention lookup fails, the API shall return HTTP 422 with the missing convention details.

---

### Requirement 11: Convention 検索機能

**Objective:** デモユーザーとして、通貨やレートタイプで Convention を検索・フィルタリングしたい。これにより、目的の Convention を素早く見つけられる。

#### Acceptance Criteria

1. The Demo GUI shall provide a Convention browser in the MarketData view's convention panel.
2. When a currency filter is applied, the Convention browser shall display only conventions for that currency.
3. When a convention is selected, the Demo GUI shall display the full convention details in a detail panel.
4. The Convention browser shall support filtering by rate type (Deposit, Swap, OIS, FRA, etc.).
5. The Convention browser shall display the number of matching conventions.

---

### Requirement 12: 型安全な Rate-Convention マッピング

**Objective:** 開発者として、RateType と Currency の組み合わせから Convention を型安全に取得したい。これにより、ランタイムエラーを防止できる。

#### Acceptance Criteria

1. The Infra Master shall provide a `ConventionRegistry` struct that maps (Currency, RateType) tuples to `MarketConvention`.
2. When the registry is queried with a supported combination, it shall return `Some(MarketConvention)`.
3. When the registry is queried with an unsupported combination, it shall return `None`.
4. The `ConventionRegistry` shall be initialised from the conventions JSON file at startup.
5. The registry shall provide `keys()` method to enumerate all supported (Currency, RateType) combinations.
6. If the JSON file is malformed, the registry initialisation shall return `ConventionRegistryError` with line/column information.

---

### Requirement 13: Convention モジュールの market への移動

**Objective:** 開発者として、`trade/convention/` モジュールを `market/convention/` に移動し、MarketRate と MarketConvention の概念的な統合を反映したい。これにより、モジュール構造が機能的な関連性を正確に表現できる。

#### Acceptance Criteria

1. The Infra Master shall move all files from `trade/convention/` to `market/convention/`.
2. When the migration is complete, the module shall be accessible via `infra_domain::market::convention::*`.
3. The Infra Master shall maintain backward compatibility by re-exporting from `trade::convention`:
   ```rust
   // In trade/convention/mod.rs (deprecated)
   #[deprecated(since = "0.x.0", note = "Use infra_domain::market::convention instead")]
   pub use crate::market::convention::*;
   ```
4. The `market/convention/` module shall include all existing convention types:
   - `SwapConvention`, `SwapLegConvention`
   - `FxConvention`, `FxOptionConvention`
   - `BondConvention`, `FraConvention`, `FuturesConvention`
   - `CapFloorConvention`, `SwaptionConvention`
   - `CdsConvention`, `EquityConvention`, `CommodityConvention`
   - `InflationSwapConvention`
   - `ConventionSet`
5. The `market/` module's public API shall export convention types via `infra_domain::market::convention`.
6. If any crate depends on `trade::convention`, the build shall succeed with deprecation warnings.

---

### Requirement 14: MarketData 画面への RateIndex 一覧表示（関連データ紐付き）

**Objective:** デモユーザーとして、MarketData 画面で利用可能な RateIndex（SOFR, ESTR, EURIBOR 等）の一覧を確認し、各インデックスに関連する MarketRate, Convention, Instrument を確認したい。これにより、インデックスを起点としたマーケットデータの全体像を把握できる。

#### Acceptance Criteria

1. The Demo GUI shall display a RateIndex panel in the MarketData view showing all available rate indices.
2. The RateIndex panel shall display for each index:
   - Index name and code (e.g., "SOFR", "EURIBOR 3M")
   - Currency
   - Tenor (Overnight, 3M, 6M)
   - Day count convention
   - Compounding method (Simple, Compounded)
   - **Count of associated MarketRates** (e.g., "12 rates")
   - **Count of associated Conventions** (e.g., "3 conventions")
3. When a RateIndex is selected, the Demo GUI shall display:
   - Full `IndexMetadata` (fixing lag, settlement lag, fixing calendar)
   - **List of associated MarketRates** grouped by rate type (Deposit, OIS, Swap, FRA)
   - **List of associated Conventions** that use this index for floating leg
   - **Quick-expand links** to view any associated rate as MarketInstrument with CF
4. The Demo GUI shall provide filtering by currency to show only indices for the selected currency.
5. When a RateIndex is selected, the Demo GUI shall highlight all MarketRates that reference that index in the Rates table.
6. The RateIndex panel shall indicate whether each index is an overnight RFR or term IBOR.
7. When clicking an associated MarketRate from the Index detail, the Demo GUI shall navigate to that rate's detail view with CF expansion.

---

### Requirement 15: RateIndex API エンドポイント（関連データ含む）

**Objective:** 開発者として、RateIndex 情報と関連する Rate/Convention/Instrument を取得する API エンドポイントを提供したい。これにより、フロントエンドがインデックス中心のナビゲーションを実現できる。

#### Acceptance Criteria

1. The Demo Web API shall provide `GET /api/market/indices` returning all available `RateIndex` with metadata and association counts.
2. The API response shall include for each index:
   - `code`: API code (e.g., "SOFR")
   - `name`: Display name (e.g., "SOFR")
   - `currency`: Currency code
   - `tenor`: Tenor code (e.g., "O/N", "3M")
   - `dayCounter`: Day count convention
   - `metadata`: Full `IndexMetadata` object
   - `associatedRatesCount`: Number of MarketRates referencing this index
   - `associatedConventionsCount`: Number of Conventions using this index
3. The Demo Web API shall provide `GET /api/market/indices/{code}` returning a single `RateIndex` with full metadata and associated data:
   - `associatedRates`: Array of `RateId` that reference this index
   - `associatedConventions`: Array of convention IDs that use this index for floating leg
4. The Demo Web API shall provide `GET /api/market/indices/{code}/rates` returning all MarketRates for this index with their instruments and CF expansion capability.
5. The Demo Web API shall provide `GET /api/market/indices/{code}/conventions` returning all Conventions that use this index.
6. When the index code is not found, the API shall return HTTP 404 with error details.
7. The API shall support filtering by currency via query parameter `?currency=USD`.

---

## Non-Functional Requirements

### Performance

- CF 展開は標準商品で 500ms 以内に完了すること
- Rate Detail パネルの更新は 100ms 以内に完了すること
- Convention レジストリのルックアップは O(1) であること

### Compatibility

- 既存の `MarketRateSet` API との後方互換性を維持すること
- 既存の Trade 展開 API (`/api/trades/expand`) は維持すること
- demo/data/input の既存ファイル形式との互換性を維持すること

### Maintainability

- 新しい通貨・商品種別の追加は JSON 設定のみで可能であること
- Convention 定義は一元化され、コード内にハードコードしないこと
