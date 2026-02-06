# Implementation Plan

## Overview

マーケットレートインフラストラクチャの実装タスク。外部データプロバイダーからのレート入力を正規化し、Instrument へマッピングして Pricer レイヤーに提供する。

**対象モジュール**: `crates/infra_domain/src/market/`

---

## Tasks

- [x] 1. 基盤型の実装
- [x] 1.1 (P) QuoteType 列挙型を実装する
  - マーケットクォートの種別を分類する列挙型を作成する（Bid, Ask, Mid, Last）
  - Copy, Clone, PartialEq, Eq, Hash トレイトを導出する
  - serde feature gate 付きでシリアライゼーションをサポートする
  - _Requirements: 1.4, 1.5_

- [x] 1.2 (P) RateType 列挙型を実装する
  - マーケットレートの商品種別を分類する列挙型を作成する
  - Deposit, Fra, Futures, Swap, Ois, BasisSwap, FxSpot, FxForward, Vol の 9 種類をサポートする
  - non_exhaustive 属性で将来の拡張に対応する
  - _Requirements: 1.3, 1.5_

- [x] 1.3 (P) DataSource 列挙型と SourcePriority を実装する
  - マーケットデータの出所を識別する列挙型を作成する（Reuters, Bloomberg, Internal, Manual）
  - データソースの優先順位を定義する SourcePriority 構造体を作成する
  - デフォルト優先順位（Bloomberg > Reuters > Internal > Manual）を提供する
  - 2 つのソースを比較して優先度を返すメソッドを実装する
  - _Requirements: 6.1, 6.2_

- [x] 2. エラー型の実装
- [x] 2.1 MarketRateError を実装する
  - thiserror を使用して構造化エラー型を定義する
  - InvalidRate: 不正なレート値（NaN, Infinite, 閾値超過）
  - StaleData: 古いタイムスタンプのレート
  - MissingRate: 存在しないレート
  - MappingFailed: Instrument へのマッピング失敗
  - ValidationFailed: カスタムバリデーション失敗
  - _Requirements: 4.7, 5.1, 5.2, 5.3_

- [x] 3. レート識別子の実装
- [x] 3.1 (P) RateId 構造体を実装する
  - マーケットレートを一意に識別する構造体を作成する
  - 通貨、テナー、レートタイプ、オプショナルなレートインデックスを保持する
  - HashMap キーとして使用できるよう Hash トレイトを実装する
  - ビルダーパターンでレートインデックスを追加するメソッドを提供する
  - _Requirements: 2.1_

- [x] 3.2 (P) TickerMapping 構造体を実装する
  - 外部ティッカー（Reuters RIC、Bloomberg ticker）を内部 RateId にマッピングする構造体を作成する
  - ティッカーの登録、検索、存在確認メソッドを実装する
  - 主要通貨（USD, EUR, GBP, JPY, CHF）のデフォルトマッピングを提供する
  - 存在しないティッカーの検索時は None を返す
  - _Requirements: 2.2, 2.3, 2.4_

- [x] 4. MarketRate 構造体の実装
- [x] 4.1 MarketRate 構造体を実装する
  - 単一のマーケットレートをメタデータとともに表現する構造体を作成する
  - RateId, QuoteType, レート値, タイムスタンプ, DataSource を保持する
  - コンストラクタで NaN/Infinite チェックを行い、不正な場合は MarketRateError を返す
  - タイムスタンプとソースを変更するビルダーメソッドを提供する
  - Task 1（基盤型）の完了後に実装可能
  - _Requirements: 1.1, 1.2, 1.5_

- [x] 5. バリデーション機能の実装
- [x] 5.1 (P) RateValidator トレイトを実装する
  - マーケットレートのバリデーションインターフェースを定義する
  - MarketRate を受け取り、Result<(), MarketRateError> を返すメソッドを定義する
  - _Requirements: 5.4_

- [x] 5.2 StandardRateValidator を実装する
  - レート種別ごとの閾値チェックを実装する
  - 金利: -10% ～ 100% の範囲
  - FX レート: 0.0001 ～ 100000.0 の範囲
  - ボラティリティ: 0% ～ 500% の範囲
  - 閾値を超えた場合は InvalidRate エラーを返す
  - Task 5.1（RateValidator トレイト）の完了後に実装
  - _Requirements: 5.3, 5.5_

- [x] 6. MarketRateSet の実装
- [x] 6.1 MarketRateSet 基本機能を実装する
  - 複数のマーケットレートをコレクションとして管理する構造体を作成する
  - HashMap を使用して (RateId, QuoteType) 複合キーで O(1) ルックアップを実現する
  - レートの挿入、取得、削除メソッドを実装する
  - Clone, Debug, Default トレイトを導出する
  - Task 4（MarketRate）の完了後に実装可能
  - _Requirements: 3.1, 3.2, 3.3, 7.1_

- [x] 6.2 MarketRateSet クエリ機能を実装する
  - 指定した RateId の mid レートを取得するメソッドを実装する（mid がなければ bid/ask から計算）
  - 指定した RateType のレートをイテレートするメソッドを実装する
  - 古いタイムスタンプのレートを検出するメソッドを実装する
  - Task 6.1（基本機能）の完了後に実装
  - _Requirements: 3.4, 3.5, 3.6_

- [x] 6.3 MarketRateSet フィルタ機能を実装する
  - 指定した通貨のレートのみを抽出するメソッドを実装する
  - 指定した日付時点で有効なレートを抽出するメソッドを実装する（タイムスタンプベース）
  - Task 6.1（基本機能）の完了後に実装
  - _Requirements: 7.3, 7.4_

- [x] 6.4 MarketRateSet マージ機能を実装する
  - 2 つの MarketRateSet をソース優先順位に基づいてマージするメソッドを実装する
  - 同一レートが複数ソースに存在する場合、優先度の高いソースを採用する
  - Task 6.1（基本機能）の完了後に実装
  - _Requirements: 6.3, 6.4_

- [x] 7. Instrument マッピング機能の実装
- [x] 7.1 (P) InstrumentMapper トレイトを実装する
  - MarketRate から Instrument への変換インターフェースを定義する
  - 評価日を受け取り、Result<Instrument, MarketRateError> を返すメソッドを定義する
  - _Requirements: 4.1_

- [x] 7.2 StandardInstrumentMapper を実装する
  - Deposit レートを Instrument::Deposit に変換する
  - Swap レートを Instrument::ParSwap に変換する
  - OIS レートを Instrument::Ois に変換する
  - Futures レートを Instrument::Futures に変換する（rate から price への変換: 100 - rate * 100）
  - サポート外の RateType は MappingFailed エラーを返す
  - Task 7.1（InstrumentMapper トレイト）の完了後に実装
  - _Requirements: 4.2, 4.3, 4.4, 4.5, 4.6, 4.7_

- [x] 7.3 MarketRateSet の to_instruments メソッドを実装する
  - MarketRateSet 内の全レートを Instrument に変換するメソッドを実装する
  - StandardInstrumentMapper を使用して変換を行う
  - 1 つでも変換に失敗した場合はエラーを返す
  - Task 6.1 および 7.2 の完了後に実装
  - _Requirements: 7.2_

- [x] 8. モジュール統合とエクスポート
- [x] 8.1 mod.rs を更新してモジュールをエクスポートする
  - 新規作成した全モジュールを market/mod.rs に追加する
  - 公開 API を整理し、必要な型を re-export する
  - _Requirements: 1.5, 7.5_

- [x] 9. テストの実装
- [x] 9.1 (P) 基盤型のユニットテストを実装する
  - QuoteType, RateType, DataSource の基本動作をテストする
  - SourcePriority の比較ロジックをテストする
  - serde シリアライゼーションの往復テストを行う
  - _Requirements: 1.3, 1.4, 6.1, 6.2_

- [x] 9.2 (P) MarketRate と RateId のユニットテストを実装する
  - RateId の生成と Hash 動作をテストする
  - MarketRate のコンストラクタバリデーションをテストする（NaN, Infinite, 正常値）
  - TickerMapping のルックアップとデフォルトマッピングをテストする
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 2.3, 2.4_

- [x] 9.3 (P) バリデーション機能のユニットテストを実装する
  - StandardRateValidator の閾値チェックをテストする
  - 各レート種別の境界値テストを行う
  - MarketRateError のエラーメッセージをテストする
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 9.4 MarketRateSet のユニットテストを実装する
  - CRUD 操作（insert, get_rate, remove）をテストする
  - get_mid_rate の計算ロジックをテストする（mid 存在/bid-ask 計算）
  - rates_by_type と stale_rates のクエリをテストする
  - filter_by_currency と as_of のフィルタをテストする
  - merge のソース優先順位による選択をテストする
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 6.3, 6.4, 7.3, 7.4_

- [x] 9.5 InstrumentMapper のユニットテストを実装する
  - StandardInstrumentMapper の各 RateType 変換をテストする
  - Futures の rate → price 変換の正確性を検証する
  - サポート外 RateType のエラーをテストする
  - to_instruments の全体フローをテストする
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 7.2_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1 | 4.1, 9.2 |
| 1.2 | 4.1, 9.2 |
| 1.3 | 1.2, 9.1 |
| 1.4 | 1.1, 9.1 |
| 1.5 | 1.1, 1.2, 4.1, 8.1 |
| 2.1 | 3.1, 9.2 |
| 2.2 | 3.2, 9.2 |
| 2.3 | 3.2, 9.2 |
| 2.4 | 3.2, 9.2 |
| 3.1 | 6.1, 9.4 |
| 3.2 | 6.1, 9.4 |
| 3.3 | 6.1, 9.4 |
| 3.4 | 6.2, 9.4 |
| 3.5 | 6.2, 9.4 |
| 3.6 | 6.2, 9.4 |
| 4.1 | 7.1, 9.5 |
| 4.2 | 7.2, 9.5 |
| 4.3 | 7.2, 9.5 |
| 4.4 | 7.2, 9.5 |
| 4.5 | 7.2, 9.5 |
| 4.6 | 7.2, 9.5 |
| 4.7 | 2.1, 7.2, 9.5 |
| 5.1 | 2.1, 9.3 |
| 5.2 | 2.1, 9.3 |
| 5.3 | 2.1, 5.2, 9.3 |
| 5.4 | 5.1, 9.3 |
| 5.5 | 5.2, 9.3 |
| 6.1 | 1.3, 9.1 |
| 6.2 | 1.3, 9.1 |
| 6.3 | 6.4, 9.4 |
| 6.4 | 6.4, 9.4 |
| 7.1 | 6.1, 9.4 |
| 7.2 | 7.3, 9.5 |
| 7.3 | 6.3, 9.4 |
| 7.4 | 6.3, 9.4 |
| 7.5 | 8.1 |
