# Implementation Plan

## Overview

本実装計画は `market-convention-instrument` 仕様の 15 要件を 12 メジャータスクに分解する。
タスクは design.md の 4 フェーズ（Convention 移動 → 新規型 → Demo Data & API → GUI）に沿って構成。

---

## Tasks

### Phase 1: Convention Module Migration

- [x] 1. Convention モジュールを market/ に移動
- [x] 1.1 market/convention ディレクトリ作成とファイルコピー
  - trade/convention/ 配下の全ファイルを market/convention/ にコピー
  - market/mod.rs に `pub mod convention;` を追加
  - cargo build で正常にコンパイルされることを確認
  - _Requirements: 13_

- [x] 1.2 trade/convention の deprecation 付き re-export 設定
  - trade/convention/mod.rs を deprecation 警告付き re-export に変更
  - `#[deprecated(since = "0.x.0", note = "Use infra_master::market::convention instead")]` を追加
  - 既存の外部参照が警告付きでコンパイル成功することを確認
  - _Requirements: 13_

---

### Phase 2: New Types Implementation

- [x] 2. 新規 Convention 型の実装
- [x] 2.1 (P) DepositConvention の実装
  - 短期預金商品の Convention 型を定義
  - day_count, calendar, business_day_convention, spot_lag フィールドを含む
  - 各通貨のデフォルト値を提供するファクトリメソッドを実装
  - 単体テスト作成
  - _Requirements: 1_

- [x] 2.2 (P) XCcyBasisConvention の実装
  - クロスカレンシーベーシススワップの Convention 型を新規作成
  - 両通貨の leg convention と basis spread 慣行を定義
  - USD/JPY, EUR/USD 等の主要ペアのデフォルト値を実装
  - 単体テスト作成
  - _Requirements: 1_

- [x] 2.3 (P) FxSwapConvention の実装
  - FX スワップの Convention 型を新規作成
  - near leg と far leg の settlement 慣行を定義
  - 主要通貨ペアのデフォルト値を実装
  - 単体テスト作成
  - _Requirements: 1_

- [x] 3. MarketConvention enum の実装
- [x] 3.1 MarketConvention enum 定義
  - Deposit, Swap, Ois, Fra, Futures, XCcyBasis, FxForward, FxSwap の variant を持つ enum を定義
  - serde の tag 属性で snake_case シリアライズを設定
  - instrument_type_name() メソッドで商品種別名を返す
  - 単体テスト作成
  - _Requirements: 1_

- [x] 3.2 for_rate_id() ファクトリメソッド実装
  - RateId から適切な MarketConvention を導出するロジックを実装
  - 通貨と RateType の組み合わせに基づいて Convention を選択
  - 対応する Convention がない場合は None を返す
  - 全 (Currency, RateType) 組み合わせのテスト作成
  - _Requirements: 1_

- [x] 4. MarketInstrument 型の実装
- [x] 4.1 MarketInstrument struct 定義
  - rate_id, rate_value, convention, valuation_date, effective_date, maturity_date, notional フィールドを定義
  - new() コンストラクタで tenor から effective/maturity date を計算
  - 無効な rate value や convention の場合は MarketInstrumentError を返す
  - 単体テスト作成
  - _Requirements: 2_

- [x] 4.2 to_trade() メソッド実装
  - MarketInstrument を CF 展開された Trade に変換
  - Swap/OIS の場合は fixed leg と floating leg を生成
  - Deposit/FRA の場合は単一 leg を生成
  - 各 leg に適切な Cashflow を生成
  - Convention に基づく day count, frequency, calendar を適用
  - 変換失敗時は詳細なエラー情報を返す
  - 各商品種別の CF 展開テスト作成
  - _Requirements: 2_

- [x] 5. ConventionRegistry の実装
- [x] 5.1 ConventionRegistry struct と JSON パース
  - (Currency, RateType) → MarketConvention の HashMap を内部に持つ struct を定義
  - from_json() で conventions.json をパースして Registry を構築
  - JSON スキーマ検証と行/列情報付きエラーを返す
  - 単体テスト作成（正常/異常 JSON）
  - _Requirements: 6, 12_

- [x] 5.2 Registry ルックアップとキー列挙
  - get(currency, rate_type) で O(1) ルックアップを提供
  - keys() で登録済み全キーを列挙するイテレータを返す
  - len() で登録数を返す
  - ルックアップとキー列挙のテスト作成
  - _Requirements: 12_

- [x] 6. MarketRateSet 拡張
- [x] 6.1 to_instruments() メソッド実装
  - MarketRateSet に to_instruments(valuation_date) メソッドを追加
  - 各 MarketRate を ConventionRegistry 経由で MarketInstrument に変換
  - Convention が見つからない rate はスキップして warning ログ
  - 結果を maturity date 順にソート
  - 元の rate metadata (source, quote_type, timestamp) を保持
  - 一括変換とソートのテスト作成
  - _Requirements: 3_

- [x] 7. EventInstrument の実装
- [x] 7.1 (P) EventInstrument struct と impact_on_curve()
  - event_date, event_type, expected_spread, confidence, rate_index フィールドを定義
  - impact_on_curve() は expected_spread をそのまま返す（将来拡張用のプレースホルダー）
  - from_historical() コンストラクタを提供（CentralBankMeeting からの変換）
  - 単体テスト作成
  - _Requirements: 4_

---

### Phase 3: Demo Data & API

- [x] 8. Demo データファイル作成
- [x] 8.1 (P) conventions.json 作成
  - USD, EUR, GBP, JPY の全 (Currency, RateType) 組み合わせの Convention を定義
  - 各 Convention に spot_lag, day_count, payment_frequency, business_day_convention, fixing_calendar, roll_convention を含める
  - Futures の IMM 日付等、tenor 固有のオーバーライドを定義
  - JSON スキーマ検証に合格することを確認
  - _Requirements: 6_

- [x] 8.2 (P) 追加通貨の rates ファイル作成
  - CHF (SARON-based): deposit, swap の rates ファイル作成
  - AUD (RBA Cash Rate-based): deposit, swap の rates ファイル作成
  - CAD (CORRA-based): deposit, swap の rates ファイル作成
  - 全ファイルで一貫した valuation_date を使用
  - 各通貨に必要な tenor が含まれていることを確認
  - _Requirements: 5_

- [x] 9. REST API ハンドラー実装
- [x] 9.1 Rate → Instrument エンドポイント
  - GET /api/market/rates/{rate_id}/instrument を実装
  - rate_id から MarketRate を取得し、ConventionRegistry 経由で MarketInstrument を構築
  - valuation_date クエリパラメータをサポート（デフォルト: 今日）
  - rate が見つからない場合は 404、Convention が見つからない場合は 422 を返す
  - processing_time_ms をレスポンスに含める
  - API テスト作成
  - _Requirements: 10_

- [x] 9.2 Rate → Cashflows エンドポイント
  - GET /api/market/rates/{rate_id}/cashflows を実装
  - MarketInstrument.to_trade() を呼び出して CF 展開
  - 各 leg を legType, direction, cashflows 配列で返す
  - cashflow には paymentDate, accrualStart, accrualEnd, yearFraction, notional, rate, spread, payoffType を含める
  - エラーハンドリングと processing_time_ms を実装
  - API テスト作成
  - _Requirements: 10_

- [x] 9.3 (P) RateIndex エンドポイント群
  - GET /api/market/indices: 全 RateIndex を metadata と association counts 付きで返す
  - GET /api/market/indices/{code}: 単一 Index の詳細と associated rates/conventions を返す
  - GET /api/market/indices/{code}/rates: Index に関連する全 MarketRate を返す
  - GET /api/market/indices/{code}/conventions: Index を使用する全 Convention を返す
  - currency クエリパラメータでフィルタリング可能
  - 存在しない index code には 404 を返す
  - API テスト作成
  - _Requirements: 15_

---

### Phase 4: GUI Updates

- [ ] 10. MarketData 画面の拡張
- [ ] 10.1 Index パネルの追加
  - MarketData view に RateIndex 一覧パネルを追加
  - 各 Index に name, code, currency, tenor, dayCounter, compounding method を表示
  - associated rates count と conventions count を表示
  - Index 選択時に full metadata と関連 Rate/Convention リストを表示
  - 通貨フィルタリング機能を実装
  - Index 選択時に Rates テーブルで関連 Rate をハイライト
  - overnight RFR / term IBOR の区別を表示
  - 関連 Rate クリックで Rate detail view に遷移
  - _Requirements: 14_

- [ ] 10.2 Rate Detail パネルの拡張
  - Rate 選択時に Convention 詳細を表示（day_count, frequency, calendar, settlement）
  - Instrument 情報（effective_date, maturity_date, notional）を表示
  - Convention が見つからない場合は "Convention not available" を表示
  - 100ms 以内に更新されるようパフォーマンス最適化
  - ローディングインジケータを表示
  - _Requirements: 7_

- [ ] 10.3 CF Expansion パネルの追加
  - Rate 選択時に自動的に CF 展開を実行し下部に表示
  - Payment Date, Accrual Start, Accrual End, Year Fraction, Notional, Rate/Spread, Payoff Type 列を持つテーブル
  - 複数 leg がある場合は collapsible セクションで表示
  - Payer/Receiver の direction を視覚的にスタイリング
  - CF 展開失敗時はエラーメッセージを表示
  - 500ms 以内に展開完了するようパフォーマンス最適化
  - _Requirements: 8_

- [ ] 10.4 Convention 検索機能の追加
  - MarketData view に Convention ブラウザパネルを追加
  - 通貨フィルタと rate type フィルタを提供
  - 選択した Convention の詳細を detail パネルに表示
  - マッチする Convention 数を表示
  - _Requirements: 11_

- [ ] 11. TradeExpand 画面の廃止
- [ ] 11.1 Navigation 削除とリダイレクト設定
  - main.ts から trade-expansion-view の navigation item を削除
  - viewInitializers から trade-expansion-view を削除
  - TradeExpand への外部リンクを MarketData view にリダイレクト
  - trade-expansion.ts ファイルに deprecation コメントを追加または削除
  - Trade expansion API (/api/trades/expand) は維持
  - _Requirements: 9_

---

### Phase 5: Integration & Testing

- [ ] 12. 統合テストと検証
- [ ] 12.1 Backend 統合テスト
  - Demo データからの一括 MarketInstrument 変換テスト
  - ConventionRegistry → MarketInstrument → Trade の完全パイプラインテスト
  - 全 API エンドポイントの E2E テスト
  - エラーケース（missing rate, missing convention）のテスト
  - _Requirements: 3, 10, 15_

- [ ] 12.2 E2E/UI テスト
  - Rate 選択 → Detail パネル表示フローのテスト
  - Rate 選択 → CF 展開表示フローのテスト
  - Index 選択 → 関連 Rate ハイライト → Rate 選択 → Detail のテスト
  - TradeExpand view が非表示であることの確認
  - パフォーマンス検証（Detail 100ms、CF 展開 500ms）
  - _Requirements: 7, 8, 9, 14_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1 | 2.1, 2.2, 2.3, 3.1, 3.2 |
| 2 | 4.1, 4.2 |
| 3 | 6.1, 12.1 |
| 4 | 7.1 |
| 5 | 8.2 |
| 6 | 5.1, 8.1 |
| 7 | 10.2, 12.2 |
| 8 | 10.3, 12.2 |
| 9 | 11.1, 12.2 |
| 10 | 9.1, 9.2, 12.1 |
| 11 | 10.4 |
| 12 | 5.1, 5.2 |
| 13 | 1.1, 1.2 |
| 14 | 10.1, 12.2 |
| 15 | 9.3, 12.1 |

**全 15 要件がタスクにマッピング済み。**
