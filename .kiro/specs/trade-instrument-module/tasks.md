# Implementation Plan

## Overview

Trade & Instrument Module の実装タスク。`infra_master` クレート内に `trade/` と `convention/` サブモジュールを追加し、全ての金融取引を CF 展開済みの共通フォーマット（`Trade` → `Vec<Leg>` → `Vec<Cashflow>`）で表現する。

---

## Tasks

### Phase 1: Core Data Structures

- [x] 1. Trade モジュール基盤構築
- [x] 1.1 (P) エラー型を定義する
  - Trade 構築時のバリデーションエラーを表現する `TradeError` enum を作成
  - `InvalidSchedule`, `EmptyLeg`, `InvalidNotional`, `MismatchedCurrency`, `InvalidPayoff`, `IncompatibleConvention` バリアントを実装
  - `thiserror` を使用して `std::error::Error` と `std::fmt::Display` を実装
  - `DateError` からの `From` トレイト実装で `?` 演算子を使用可能に
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 8.6_

- [x] 1.2 (P) 市場指標型を定義する
  - 金利・FX・株式・インフレ・コモディティを含む `IndexType` enum を作成
  - 既存の `RateIndex` をラップする `Rate(RateIndex)` バリアント
  - `SwapRate`, `Fx`, `Equity`, `Inflation`, `Commodity` バリアントを追加
  - 観測条件を表現する `IndexObservation` struct（index_type, observation_lag, fixing_source）
  - `impl From<RateIndex> for IndexType` 変換を実装
  - `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash` derive
  - serde フィーチャーゲート対応
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 9.2, 9.4_

- [x] 1.3 (P) ペイオフ型を定義する
  - `OptionType` enum（Call, Put）を作成
  - キャッシュフロー計算式を表現する `Payoff` enum を実装
  - `Fixed` (固定金利), `Linear` (変動金利), `VanillaOption` (Cap/Floor), `Digital` (デジタルオプション) バリアント
  - `required_index()` メソッド（Fixed は None、他は Some を返す）
  - `is_fixed()` メソッド（Fixed のみ true）
  - serde フィーチャーゲート対応
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8_

- [x] 1.4 キャッシュフロー型を定義する
  - `CashflowType` enum（Coupon, Principal, Fee, Settlement）を作成
  - キャッシュフローの最小単位を表現する `Cashflow` struct
  - フィールド: cf_type, payment_date, accrual_start, accrual_end, year_fraction, notional, payoff, currency
  - `is_fixed(ref_date)` メソッド（Payoff が Fixed なら true）
  - `is_future(ref_date)` メソッド（payment_date > ref_date で true）
  - `Debug`, `Clone` derive と serde 対応
  - Task 1.2, 1.3 の型に依存
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 9.1, 9.3_

- [x] 1.5 Leg 型を定義する
  - `Direction` enum（Payer, Receiver）を作成
  - `sign()` メソッド（Payer = -1.0, Receiver = 1.0）で NPV 計算用の符号を返す
  - `LegType` enum（Fixed, Floating, CapFloor, Principal, Generic）
  - キャッシュフロー列を表現する `Leg` struct（cashflows, direction, leg_type, currency）
  - `new()` コンストラクタ、`future_cashflows(ref_date)` イテレータ、`notional()` メソッド
  - Task 1.4 の Cashflow に依存
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7_

- [x] 1.6 Trade 型を定義する
  - `TradeId` type alias（String）を作成
  - `ExerciseType` enum（European, Bermudan, American）
  - `SettlementType` enum（Cash, Physical）
  - `TradeType` enum（Swap, Swaption, Bond, CapFloor, FxForward, Generic）
  - Swaption バリアントに exercise_dates, exercise_type, settlement_type を含める
  - Bond バリアントに issuer_id, seniority を含める
  - `TradeMetadata` struct（trade_date, counterparty, portfolio, book）
  - 全取引を表現する `Trade` struct（id, legs, trade_type, metadata）
  - `all_cashflows()`, `future_cashflows(ref_date)`, `num_legs()`, `is_vanilla_swap()` メソッド
  - Task 1.5 の Leg に依存
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 5.10_

---

### Phase 2: Convention Module

- [x] 2. Convention サブモジュール構築
- [x] 2.1 (P) Swap 規約を定義する
  - `SwapLegConvention` struct を作成（day_count, payment_frequency, calendar, business_day_convention, payment_lag）
  - `SwapConvention` struct（fixed_leg, float_leg, float_index, spot_lag）
  - serde フィーチャーゲート対応
  - _Requirements: 11.2, 11.3_

- [x] 2.2 (P) FRA 規約を定義する
  - `FraConvention` struct を作成（day_count, calendar, business_day_convention, index）
  - serde フィーチャーゲート対応
  - _Requirements: 11.4_

- [x] 2.3 (P) Futures 規約を定義する
  - `FuturesConvention` struct を作成（contract_size, tick_size, day_count, calendar）
  - serde フィーチャーゲート対応
  - _Requirements: 11.5_

- [x] 2.4 (P) Cap/Floor 規約を定義する
  - `CapFloorConvention` struct を作成（day_count, payment_frequency, calendar, business_day_convention, index）
  - serde フィーチャーゲート対応
  - _Requirements: 11.6_

- [x] 2.5 (P) FX 規約を定義する
  - `FxConvention` struct を作成（spot_days, calendar, business_day_convention）
  - serde フィーチャーゲート対応
  - _Requirements: 11.7_

- [x] 2.6 (P) Bond 規約を定義する
  - `BondConvention` struct を作成（day_count, coupon_frequency, calendar, business_day_convention, settlement_days）
  - serde フィーチャーゲート対応
  - _Requirements: 11.8_

- [x] 2.7 (P) CDS 規約を定義する
  - `CdsConvention` struct を作成（day_count, payment_frequency, calendar, business_day_convention, recovery_rate）
  - serde フィーチャーゲート対応
  - _Requirements: 11.9_

- [x] 2.8 Convention プリセットを実装する
  - `SwapConvention::usd_sofr()` - USD SOFR スワップ規約
  - `SwapConvention::eur_euribor_6m()` - EUR EURIBOR 6M スワップ規約
  - `SwapConvention::jpy_tonar()` - JPY TONAR スワップ規約
  - `SwapConvention::gbp_sonia()` - GBP SONIA スワップ規約
  - `FxConvention::usd_jpy()` - USD/JPY FX 規約
  - `FxConvention::eur_usd()` - EUR/USD FX 規約
  - 適切な CalendarId（NewYork, Target, Tokyo, London）を使用
  - Task 2.1〜2.7 の規約型に依存
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5, 12.6, 12.7, 12.8, 11.10_

- [x] 2.9 Convention モジュールエクスポートを設定する
  - `convention/mod.rs` で全規約型を pub mod + re-export
  - _Requirements: 11.1_

---

### Phase 3: Instrument & Builder

- [x] 3. Instrument と Builder 実装
- [x] 3.1 Instrument 型を定義する
  - マーケット用規格化商品を表現する `Instrument` enum を作成
  - `Deposit`, `Fra`, `Futures`, `ParSwap`, `Ois`, `BasisSwap`, `CrossCurrencySwap` バリアント
  - 各バリアントに必要なフィールド（currency, start_date, tenor, rate/spread 等）
  - `quote()` メソッド（市場クォートを返す、Futures は 100 - price）
  - `currency()` メソッド（主要通貨を返す）
  - serde フィーチャーゲート対応
  - _Requirements: 6.1, 6.2, 6.3, 6.5, 6.7_

- [x] 3.2 ScheduleBuilder を実装する
  - スケジュール生成用の `ScheduleBuilder` struct を作成
  - `new(start_date, end_date, frequency)` コンストラクタ
  - `calendar()`, `business_day_convention()`, `end_of_month()` チェインメソッド
  - `build()` → `Result<Vec<Date>, TradeError>` でスケジュール日付生成
  - Frequency に基づく期間分割ロジック
  - BusinessDayConvention での調整、Calendar での祝日チェック
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 9.6_

- [x] 3.3 LegBuilder を実装する
  - Leg 構築用の `LegBuilder` struct を作成（schedule, notional, currency, direction, day_count）
  - `new()` でバリデーション（空スケジュール → InvalidSchedule、負 notional → InvalidNotional）
  - `direction()`, `day_count()` チェインメソッド
  - `build_fixed(rate)` → Fixed Payoff の Leg 構築
  - `build_floating(index, spread)` → Linear Payoff の Leg 構築
  - 内部 `build_cashflows()` でスケジュールから Cashflow を生成
  - Task 3.2 の ScheduleBuilder と Task 1.1〜1.5 の型に依存
  - _Requirements: 7.5, 7.6, 7.7, 8.3, 8.4, 9.5_

- [x] 3.4 TradeBuilder を実装する
  - Trade 構築用の `TradeBuilder` struct を作成（id, legs, trade_type, metadata）
  - `new(id)` コンストラクタ
  - `add_leg()`, `trade_type()`, `metadata()` チェインメソッド
  - `build()` → `Trade` を構築
  - Task 3.3 と Task 1.6 に依存
  - _Requirements: 7.8, 7.9, 7.10_

- [x] 3.5 Convention と Instrument の統合
  - `TradeBuilder::from_par_swap(id, instrument, convention)` を実装
  - ParSwap + SwapConvention → 固定 Leg + 変動 Leg の Trade を構築
  - Deposit + SwapLegConvention → 単一 Cashflow の Leg
  - FRA + FraConvention → FRA Cashflow
  - 不整合な組み合わせで `Err(TradeError::IncompatibleConvention)` を返す
  - `impl From<Instrument> for Trade` トレイト実装（内部で Convention を使用して変換）
  - `Instrument::maturity()` メソッドを実装（start_date + tenor または Futures は expiry）
  - Task 2.1〜2.8 の Convention と Task 3.1〜3.4 に依存
  - _Requirements: 6.4, 6.6, 6.8, 13.1, 13.2, 13.3, 13.4, 13.5, 13.6, 13.7_

---

### Phase 4: Module Integration & Testing

- [x] 4. モジュール統合と検証
- [x] 4.1 Trade モジュールエクスポートを設定する
  - `trade/mod.rs` で全型を pub mod + re-export
  - error, index, payoff, cashflow, leg, trade, instrument, builder の順
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6_

- [x] 4.2 lib.rs への統合
  - `infra_master/src/lib.rs` に `mod convention;` と `mod trade;` を追加
  - Convention 型の re-export（SwapConvention, SwapLegConvention, FxConvention 等）
  - Trade 型の re-export（Trade, TradeBuilder, Leg, Cashflow, Payoff, Direction 等）
  - prelude モジュールへの追加
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6_

- [x] 4.3 単体テストを実装する
  - `index.rs`: From<RateIndex> 変換テスト
  - `payoff.rs`: required_index(), is_fixed() テスト
  - `cashflow.rs`: is_fixed(), is_future() テスト
  - `leg.rs`: Direction::sign(), future_cashflows() テスト
  - `trade.rs`: all_cashflows(), is_vanilla_swap() テスト
  - `builder.rs`: バリデーションエラー、正常構築テスト
  - `convention/`: プリセット値の正確性テスト
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.5, 2.6, 2.7, 3.3, 3.4, 4.3, 4.4, 4.6, 4.7, 5.7, 5.8, 5.9, 8.3, 8.4_

- [x] 4.4 Serde ラウンドトリップテストを実装する
  - Trade → JSON → Trade のシリアライズ/デシリアライズ
  - Instrument, Leg, Cashflow の各型でラウンドトリップ検証
  - serde feature が有効な場合のみテスト実行
  - _Requirements: 10.1, 10.2, 10.3, 10.4_

- [x] 4.5 統合テストを実装する
  - Builder API で様々な Trade を構築するテスト
  - Convention + Instrument → Trade のワークフローテスト
  - Vanilla Swap、OIS、Deposit の構築を検証
  - _Requirements: 6.4, 7.10, 13.2, 13.3, 13.4_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1 (IndexType/IndexObservation) | 1.2, 4.3 |
| 2 (Payoff 定義) | 1.3, 4.3 |
| 3 (Cashflow 定義) | 1.4, 4.3 |
| 4 (Leg 定義) | 1.5, 4.3 |
| 5 (Trade 定義) | 1.6, 4.3 |
| 6 (Instrument 定義) | 3.1, 3.5, 4.5 |
| 7 (Builder パターン) | 3.2, 3.3, 3.4, 4.5 |
| 8 (エラーハンドリング) | 1.1, 3.3, 4.3 |
| 9 (既存型との統合) | 1.2, 1.4, 3.2, 3.3, 4.1, 4.2 |
| 10 (Serde シリアライゼーション) | 4.4 |
| 11 (Convention 定義) | 2.1〜2.9 |
| 12 (Convention プリセット) | 2.8 |
| 13 (Convention と Instrument の統合) | 3.5, 4.5 |
