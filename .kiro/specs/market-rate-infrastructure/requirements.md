# Requirements Document

## Introduction

本仕様は、外部マーケットデータプロバイダー（Reuters、Bloomberg等）からのレート入力を正規化し、適切な Instrument にマッピングして `MarketRateSet` として Pricer レイヤーに提供するインフラストラクチャを定義する。

対象配置: `crates/infra_master/src/market/`

A-I-P-S アーキテクチャにおいて、本モジュールは **I**nfra レイヤーに属し、**P**ricer レイヤーへの依存は許可されない。Adapter レイヤー（adapter_feeds）が本モジュールの型を使用して外部データを正規化する。

## Requirements

### Requirement 1: マーケットレート型定義

**Objective:** As a Quant Developer, I want 標準化されたマーケットレート型を使用したい, so that 外部データソースからのレートを一貫した形式で扱える

#### Acceptance Criteria

1. The `infra_master::market` module shall provide a `MarketRate` struct that encapsulates a single market quote with metadata（レート値、タイムスタンプ、ソース、品質情報）
2. When a `MarketRate` is created, the module shall validate that the rate value is within reasonable bounds（非 NaN、非 Infinite、非負でない場合は警告）
3. The `infra_master::market` module shall provide a `RateType` enum to classify market rates（Deposit, Fra, Futures, Swap, Ois, BasisSwap, FxSpot, FxForward, Vol）
4. The `infra_master::market` module shall provide a `QuoteType` enum for bid/ask/mid/last classification
5. Where the `serde` feature is enabled, the `MarketRate` struct shall support serialisation and deserialisation

### Requirement 2: レート識別子システム

**Objective:** As a Quant Developer, I want 一意のレート識別子を使用したい, so that Reuters/Bloomberg ティッカーと内部 Instrument を正確にマッピングできる

#### Acceptance Criteria

1. The `infra_master::market` module shall provide a `RateId` type that uniquely identifies a market rate（通貨、テナー、レートタイプの組み合わせ）
2. The `infra_master::market` module shall provide a `TickerMapping` struct that maps external tickers（Reuters RIC、Bloomberg ticker）to internal `RateId`
3. When a ticker is looked up, the `TickerMapping` shall return `Option<RateId>` for unknown tickers
4. The `infra_master::market` module shall provide standard ticker mappings for major currencies（USD、EUR、GBP、JPY、CHF）and common rate indices

### Requirement 3: マーケットレートセット管理

**Objective:** As a Quant Developer, I want 複数のマーケットレートをセットとして管理したい, so that カーブ構築やプライシングに必要なデータを一括で提供できる

#### Acceptance Criteria

1. The `infra_master::market` module shall provide a `MarketRateSet` struct that holds a collection of `MarketRate` entries keyed by `RateId`
2. When a rate is inserted into `MarketRateSet`, the module shall allow multiple quotes for the same `RateId`（bid/ask/mid を個別に保持）
3. The `MarketRateSet` shall provide a `get_rate(&RateId, QuoteType) -> Option<&MarketRate>` method for rate lookup
4. The `MarketRateSet` shall provide a `get_mid_rate(&RateId) -> Option<f64>` convenience method that computes mid from bid/ask if mid is absent
5. The `MarketRateSet` shall provide an iterator over all rates for a given `RateType`
6. While the `MarketRateSet` contains stale rates（タイムスタンプが閾値より古い）, the module shall provide a `stale_rates(&Duration) -> Vec<RateId>` method to identify them

### Requirement 4: Instrument マッピング

**Objective:** As a Quant Developer, I want マーケットレートから対応する Instrument を自動生成したい, so that カーブ構築に必要な商品定義を効率的に作成できる

#### Acceptance Criteria

1. The `infra_master::market` module shall provide an `InstrumentMapper` trait with method `map_to_instrument(&MarketRate, &Date) -> Result<Instrument, MappingError>`
2. The `infra_master::market` module shall provide a `StandardInstrumentMapper` implementation that maps common rate types to `infra_master::trade::Instrument`
3. When mapping a Deposit rate, the `StandardInstrumentMapper` shall create an `Instrument::Deposit` with correct currency, tenor, and rate
4. When mapping a ParSwap rate, the `StandardInstrumentMapper` shall create an `Instrument::ParSwap` with correct currency, tenor, and rate
5. When mapping an OIS rate, the `StandardInstrumentMapper` shall create an `Instrument::Ois` with correct currency, tenor, and rate
6. When mapping a Futures rate, the `StandardInstrumentMapper` shall create an `Instrument::Futures` with price converted from rate（price = 100 - rate * 100）
7. If a rate cannot be mapped, the `InstrumentMapper` shall return `MappingError::UnsupportedRateType` with details

### Requirement 5: バリデーションとエラー処理

**Objective:** As a Quant Developer, I want 入力データのバリデーションとエラー処理を行いたい, so that 不正なデータがプライシングに使用されることを防げる

#### Acceptance Criteria

1. The `infra_master::market` module shall provide a `MarketDataError` enum with variants for validation failures（InvalidRate, StaleData, MissingRate, MappingError）
2. When a rate value is NaN or Infinite, the validation shall return `MarketDataError::InvalidRate` with the problematic value
3. When a rate is suspiciously large（e.g., > 100% for interest rates）, the validation shall return `MarketDataError::InvalidRate` with a warning message
4. The `infra_master::market` module shall provide a `RateValidator` trait for custom validation logic
5. The `infra_master::market` module shall provide a `StandardRateValidator` implementation with reasonable default bounds per rate type

### Requirement 6: データソース抽象化

**Objective:** As a Quant Developer, I want データソースを抽象化したい, so that Reuters/Bloomberg/内部データ等を統一的に扱える

#### Acceptance Criteria

1. The `infra_master::market` module shall provide a `DataSource` enum to identify the origin of market data（Reuters, Bloomberg, Internal, Manual）
2. The `infra_master::market` module shall provide a `SourcePriority` configuration that defines preference order when multiple sources provide the same rate
3. When multiple sources provide the same rate, the `MarketRateSet` shall use the `SourcePriority` to select the preferred value
4. The `infra_master::market` module shall provide a `merge(&MarketRateSet, SourcePriority) -> MarketRateSet` function to combine rate sets from different sources

### Requirement 7: Pricer レイヤーへの受け渡し

**Objective:** As a Quant Developer, I want MarketRateSet を Pricer レイヤーに受け渡したい, so that カーブ構築やプライシングに使用できる

#### Acceptance Criteria

1. The `MarketRateSet` shall implement `Clone` and `Debug` traits for standard Rust interoperability
2. The `MarketRateSet` shall provide a `to_instruments(&Date) -> Result<Vec<Instrument>, MarketDataError>` method that converts all rates to instruments
3. The `MarketRateSet` shall provide a `filter_by_currency(Currency) -> MarketRateSet` method for currency-specific extraction
4. The `MarketRateSet` shall provide a `as_of(&Date) -> MarketRateSet` method that filters to rates valid at the given date
5. Where the `serde` feature is enabled, the `MarketRateSet` shall support JSON serialisation for debugging and logging

---

## Non-Functional Requirements

### NFR 1: パフォーマンス

1. The `MarketRateSet::get_rate` operation shall have O(1) average time complexity
2. The `MarketRateSet` shall support at least 10,000 rates without significant performance degradation

### NFR 2: 型安全性

1. The module shall use `thiserror` for structured error handling
2. The module shall avoid `unwrap()` and `expect()` in library code

### NFR 3: 互換性

1. The module shall not depend on `pricer_*` crates（A-I-P-S 依存ルール遵守）
2. The module shall be compatible with stable Rust toolchain

---

## Out of Scope

- リアルタイムデータフィードの接続実装（adapter_feeds の責務）
- カーブ構築ロジック（pricer_models::market::calibration の責務）
- ヒストリカルデータの永続化（infra_store の責務）

## References

- [structure.md](../../steering/structure.md) - A-I-P-S アーキテクチャ定義
- [tech.md](../../steering/tech.md) - 技術スタック
- `infra_master::trade::Instrument` - 既存の Instrument 定義
- `infra_master::market::RateIndex` - 既存の RateIndex 定義
