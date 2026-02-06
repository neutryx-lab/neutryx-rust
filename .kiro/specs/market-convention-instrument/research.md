# Discovery Research: market-convention-instrument

## 調査概要

本ドキュメントは `market-convention-instrument` 仕様のための技術調査結果をまとめる。

---

## 1. 既存アーキテクチャ分析

### 1.1 現在の market モジュール構造

**Location:** `crates/infra_domain/src/market/`

```
market/
├── mod.rs              # モジュールエクスポート
├── currency.rs         # ISO 4217 通貨コード
├── rate_index.rs       # RateIndex enum と IndexMetadata
├── rate_id.rs          # RateId 識別子
├── rate_type.rs        # RateType 分類
├── rate.rs             # MarketRate 構造体
├── rate_set.rs         # MarketRateSet コレクション
├── quote_type.rs       # QuoteType (Bid/Ask/Mid/Last)
├── data_source.rs      # DataSource 列挙
├── mapper.rs           # InstrumentMapper トレイト
├── validation.rs       # RateValidator
├── ticker.rs           # TickerMapping
├── compounding.rs      # CompoundingMethod
├── error.rs            # MarketRateError
├── events/             # マーケットイベント
└── volatility/         # ボラティリティサーフェス
```

### 1.2 現在の convention モジュール構造

**Location:** `crates/infra_domain/src/trade/convention/`

```
convention/
├── mod.rs              # モジュールエクスポート
├── swap.rs             # SwapConvention, SwapLegConvention
├── bond.rs             # BondConvention
├── fra.rs              # FraConvention
├── futures.rs          # FuturesConvention
├── fx.rs               # FxConvention
├── fx_option.rs        # FxOptionConvention
├── swaption.rs         # SwaptionConvention
├── capfloor.rs         # CapFloorConvention
├── cds.rs              # CdsConvention
├── equity.rs           # EquityConvention
├── commodity.rs        # CommodityConvention
├── inflation.rs        # InflationSwapConvention
└── convention_set.rs   # ConventionSet
```

### 1.3 既存型の関係

```
RateId {
    currency: Currency,
    tenor: Tenor,
    rate_type: RateType,
    rate_index: Option<RateIndex>,  ← RateIndex への参照
}

MarketRate {
    id: RateId,
    quote_type: QuoteType,
    value: f64,
    timestamp: i64,
    source: DataSource,
}

SwapConvention {
    fixed_leg: SwapLegConvention,
    float_leg: SwapLegConvention,
    float_index: RateIndex,         ← RateIndex への参照
    spot_lag: u32,
}
```

---

## 2. 通貨別 Convention パターン

### 2.1 USD (SOFR)

| 商品 | Day Count (Fixed) | Day Count (Float) | Frequency | Calendar | Spot Lag |
|------|-------------------|-------------------|-----------|----------|----------|
| OIS | ACT/360 | ACT/360 | Annual | New York | 2 |
| Deposit | ACT/360 | - | - | New York | 2 |
| FRA | - | ACT/360 | Quarterly | New York | 2 |
| Futures | - | ACT/360 | Quarterly | New York | 0 (IMM) |

### 2.2 EUR (ESTR)

| 商品 | Day Count (Fixed) | Day Count (Float) | Frequency | Calendar | Spot Lag |
|------|-------------------|-------------------|-----------|----------|----------|
| OIS | ACT/360 | ACT/360 | Annual | TARGET | 2 |
| Swap (EURIBOR) | 30/360 | ACT/360 | Annual/SA | TARGET | 2 |

### 2.3 GBP (SONIA)

| 商品 | Day Count (Fixed) | Day Count (Float) | Frequency | Calendar | Spot Lag |
|------|-------------------|-------------------|-----------|----------|----------|
| OIS | ACT/365 | ACT/365 | Annual | London | 0 |

### 2.4 JPY (TONAR)

| 商品 | Day Count (Fixed) | Day Count (Float) | Frequency | Calendar | Spot Lag |
|------|-------------------|-------------------|-----------|----------|----------|
| OIS | ACT/365 | ACT/365 | Annual | Tokyo | 2 |

---

## 3. Demo GUI 分析

### 3.1 現在の画面構成

```typescript
type ViewId =
  | 'dashboard-view'
  | 'portfolio-view'
  | 'risk-view'
  | 'exposure-view'
  | 'scenarios-view'
  | 'market-data-view'
  | 'trade-expansion-view'      // ← 廃止対象
  | 'curve-builder-view'
  | 'volcube-calibration-view'
  | 'pricer-view'
  | 'graph-view';
```

### 3.2 MarketData コンポーネント状態

```typescript
const state: MarketDataState = {
  rates: [],
  filteredRates: [],
  selectedRateId: null,
  sortColumn: 'tenor',
  sortDirection: 'asc',
  lastUpdated: null,
  previousValues: new Map(),
  isInitialised: false,
  assetClass: 'Rates',            // 'Rates' | 'FX' | 'IRVol' | 'FXVol' | 'Events'
  allConventions: [],
  filteredConventions: [],
  selectedConventionId: null,
  // ... IR/FX Vol, Events 関連
};
```

### 3.3 既存 API エンドポイント

| Endpoint | 用途 |
|----------|------|
| `GET /api/market/rates` | 全レート取得 |
| `GET /api/market/rates/{id}` | レート詳細 |
| `GET /api/market/conventions` | Convention 一覧 |
| `GET /api/trades/expand` | トレード展開 (維持) |

---

## 4. Demo Data 構造分析

### 4.1 rates ファイル形式

**File:** `demo/data/input/rates/usd-sofr.json`

```json
{
  "index": "usd-sofr",
  "currency": "USD",
  "reference_date": "2026-01-29",
  "instruments": [
    {
      "type": "deposit",
      "tenor": "O/N",
      "tenor_years": 0.00274,
      "rate": 0.0433,
      "frequency": "annual",
      "description": "SOFR overnight fixing"
    },
    // ...
  ]
}
```

### 4.2 indices.json 構造

**File:** `demo/data/input/indices.json`

```json
{
  "metadata": { ... },
  "indices": {
    "rates": {
      "items": [
        { "id": "usd-sofr", "currency": "USD", "index": "SOFR", "displayName": "USD SOFR" },
        // ...
      ]
    },
    // fx, irvol, fxvol, events
  },
  "currencies": [
    { "code": "USD", "name": "US Dollar", "index": "SOFR", "calendar": "New York" },
    // ...
  ]
}
```

---

## 5. 依存関係マッピング

### 5.1 クレート依存

```
service_gateway → pricer_models → infra_domain
              ↓
         demo/gui (API client)
```

### 5.2 モジュール移動の影響

**Before:**
```rust
use infra_domain::trade::convention::SwapConvention;
```

**After:**
```rust
use infra_domain::market::convention::SwapConvention;
```

**Backward Compatibility:**
```rust
// trade/convention/mod.rs
#[deprecated(note = "Use infra_domain::market::convention instead")]
pub use crate::market::convention::*;
```

---

## 6. 新規型設計案

### 6.1 MarketConvention enum

```rust
pub enum MarketConvention {
    Deposit(DepositConvention),
    Swap(SwapConvention),
    Ois(SwapConvention),
    Fra(FraConvention),
    Futures(FuturesConvention),
    XCcyBasis(XCcyBasisConvention),
    FxForward(FxConvention),
    FxSwap(FxSwapConvention),
}

impl MarketConvention {
    pub fn for_rate_id(rate_id: &RateId) -> Option<Self> {
        // (Currency, RateType) → Convention マッピング
    }
}
```

### 6.2 MarketInstrument struct

```rust
pub struct MarketInstrument {
    pub rate_id: RateId,
    pub rate_value: f64,
    pub convention: MarketConvention,
    pub valuation_date: Date,
    pub effective_date: Date,
    pub maturity_date: Date,
    pub notional: f64,
}

impl MarketInstrument {
    pub fn to_trade(&self) -> Result<Trade, MarketInstrumentError> {
        // CF 展開ロジック
    }
}
```

### 6.3 ConventionRegistry

```rust
pub struct ConventionRegistry {
    conventions: HashMap<(Currency, RateType), MarketConvention>,
}

impl ConventionRegistry {
    pub fn from_json(path: &Path) -> Result<Self, ConventionRegistryError>;
    pub fn get(&self, currency: Currency, rate_type: RateType) -> Option<&MarketConvention>;
    pub fn keys(&self) -> impl Iterator<Item = &(Currency, RateType)>;
}
```

---

## 7. API 設計案

### 7.1 新規エンドポイント

| Method | Endpoint | Request | Response |
|--------|----------|---------|----------|
| GET | `/api/market/rates/{id}/instrument` | - | `MarketInstrumentResponse` |
| GET | `/api/market/rates/{id}/cashflows` | `?valuation_date=` | `CashflowsResponse` |
| GET | `/api/market/indices` | `?currency=` | `IndicesResponse` |
| GET | `/api/market/indices/{code}` | - | `IndexDetailResponse` |
| GET | `/api/market/indices/{code}/rates` | - | `IndexRatesResponse` |
| GET | `/api/market/indices/{code}/conventions` | - | `IndexConventionsResponse` |

### 7.2 レスポンス例

```json
// GET /api/market/rates/usd-5y-ois/instrument
{
  "rateId": "usd-5y-ois",
  "rateValue": 0.0342,
  "instrumentType": "OIS",
  "convention": {
    "type": "ois",
    "fixedLeg": { "dayCount": "ACT/360", "frequency": "Annual", ... },
    "floatingLeg": { "dayCount": "ACT/360", "rateIndex": "SOFR", ... },
    "spotLag": 2
  },
  "effectiveDate": "2026-01-31",
  "maturityDate": "2031-01-31",
  "notional": 1000000
}
```

---

## 8. テスト戦略

### 8.1 単体テスト

- `MarketConvention::for_rate_id()` の全 (Currency, RateType) 組み合わせ
- `MarketInstrument::to_trade()` の CF 展開正確性
- `ConventionRegistry` JSON パース

### 8.2 統合テスト

- Demo データからの一括変換
- API エンドポイント E2E
- GUI Rate 選択 → CF 表示フロー

---

## 9. 調査結論

1. **RateIndex と MarketConvention の分離** は既存設計と整合的
2. **convention モジュール移動** は deprecation 警告付きで後方互換可能
3. **MarketInstrument** は MarketRate + MarketConvention の組み合わせで実現
4. **Demo GUI** は MarketData コンポーネントの拡張で対応可能
5. **ConventionRegistry** は JSON 駆動で柔軟な通貨・商品追加が可能
