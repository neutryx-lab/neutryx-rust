# Requirements Document

## Project Description (Input)
Frictional Bank Web AppのPricer画面をアップデートする。すべてのInstrumentsを選択可能にし、必要項目を入力して、展開ボタンを押すと、CF展開されたTradeを作成してその内容を表示する。ここまでを完成させたい。

## Introduction

本仕様は、Frictional Bank Web Appの Pricer 画面機能を拡張し、すべての対応 Instrument タイプを選択可能にし、ユーザーが入力したパラメータに基づいてキャッシュフロー（CF）展開されたTradeを生成・表示する機能を実現する。現在のPricer画面はEquity Option、FX Option、IRSの3種類に限定されているが、infra_domain trade モジュールで定義されているすべての金融商品（Deposit、FRA、Futures、ParSwap、OIS、BasisSwap、CrossCurrencySwap、VanillaOption、Forward等）に対応を拡張する。

---

## Requirements

### Requirement 1: Instrument セレクタの拡張

**Objective:** トレーダーとして、すべての対応Instrumentタイプを選択できるようにしたい。これにより、多様な金融商品のプライシングとCF展開が可能になる。

#### Acceptance Criteria

1. When ユーザーがPricer画面を開いた時, the Pricer UI shall Instrumentタイプ選択ドロップダウンに以下のすべてのカテゴリを表示する:
   - Rates: Deposit, FRA, Futures, ParSwap, OIS, BasisSwap, IRS
   - FX: FxForward, FxOption, CrossCurrencySwap
   - Equity: VanillaOption, Forward
   - Credit: CDS（将来拡張用プレースホルダー）
   - Commodity: CommodityForward（将来拡張用プレースホルダー）

2. When ユーザーがInstrumentタイプを選択した時, the Pricer UI shall 選択されたInstrumentに対応する入力フォームを動的に表示する。

3. The Pricer UI shall Instrumentタイプをアセットクラス別にグループ化して表示する（Rates、FX、Equity、Credit、Commodity）。

### Requirement 2: Instrument別入力フォーム

**Objective:** トレーダーとして、各Instrumentタイプに必要なパラメータを入力できるようにしたい。これにより、正確なTrade生成が可能になる。

#### Acceptance Criteria

1. When ユーザーがRates系Instrument（Deposit、FRA、Futures、ParSwap、OIS）を選択した時, the Pricer UI shall 以下の入力フィールドを表示する:
   - Currency（通貨選択）
   - Start Date（開始日）
   - Tenor（期間：例 3M、6M、1Y）
   - Rate/Price（レートまたは価格）
   - Notional（想定元本）

2. When ユーザーがSwap系Instrument（IRS、BasisSwap）を選択した時, the Pricer UI shall 追加で以下の入力フィールドを表示する:
   - Fixed Rate（固定金利、IRSの場合）
   - Spread（スプレッド、BasisSwapの場合）
   - Payment Frequency（支払い頻度）
   - Day Count Convention（日数計算方式）

3. When ユーザーがFX系Instrument（FxForward、FxOption、CrossCurrencySwap）を選択した時, the Pricer UI shall 以下の入力フィールドを表示する:
   - Base Currency（基軸通貨）
   - Quote Currency（クォート通貨）
   - Spot Rate（スポットレート）
   - Forward Rate/Strike（フォワードレートまたはストライク）
   - Expiry/Maturity（満期日）
   - Notional（想定元本）
   - Option Type（オプションの場合：Call/Put）

4. When ユーザーがEquity系Instrument（VanillaOption、Forward）を選択した時, the Pricer UI shall 以下の入力フィールドを表示する:
   - Underlying（原資産ティッカー）
   - Spot Price（現在価格）
   - Strike（行使価格）
   - Expiry（満期日）
   - Volatility（ボラティリティ）
   - Risk-free Rate（無リスク金利）
   - Option Type（VanillaOptionの場合：Call/Put）
   - Direction（Forwardの場合：Long/Short）

5. The Pricer UI shall すべての必須フィールドにバリデーションを適用し、無効な入力に対してエラーメッセージを表示する。

### Requirement 3: Trade展開（CF Expansion）機能

**Objective:** トレーダーとして、入力パラメータに基づいてCF展開されたTradeを生成したい。これにより、詳細なキャッシュフロー分析が可能になる。

#### Acceptance Criteria

1. When ユーザーが「展開」ボタンをクリックした時, the Pricer Backend shall 入力パラメータを検証し、有効な場合はTrade構造を生成する。

2. When Trade生成リクエストを受信した時, the Pricer Backend shall infra_domain::trade モジュールのTradeBuilder/LegBuilderを使用してCF展開されたTradeを構築する。

3. When CF展開が完了した時, the Pricer Backend shall 以下の情報を含むレスポンスを返す:
   - Trade ID
   - Trade Type
   - 各Legの情報（Direction、Currency、LegType）
   - 各Cashflowの詳細（Payment Date、Accrual Period、Year Fraction、Notional、Payoff Type）

4. If 入力パラメータが無効な場合, then the Pricer Backend shall 具体的なエラーメッセージ（どのフィールドが無効か）を返す。

### Requirement 4: Trade/Cashflow表示

**Objective:** トレーダーとして、生成されたTradeとCashflowの詳細を視覚的に確認したい。これにより、取引内容の検証が容易になる。

#### Acceptance Criteria

1. When Trade展開が成功した時, the Pricer UI shall Tradeサマリーセクションに以下を表示する:
   - Trade ID
   - Trade Type（Swap、Forward、Option等）
   - 合計Leg数
   - 合計Cashflow数

2. When Trade展開が成功した時, the Pricer UI shall 各Legの情報をカード形式で表示する:
   - Leg番号
   - Direction（Payer/Receiver）
   - Currency
   - Leg Type（Fixed/Floating/CapFloor等）
   - Cashflow件数

3. When ユーザーがLegカードをクリックまたは展開した時, the Pricer UI shall そのLegに含まれるすべてのCashflowをテーブル形式で表示する:
   - Payment Date
   - Accrual Start/End
   - Year Fraction
   - Notional
   - Payoff Type（Fixed/Linear/VanillaOption/Digital）
   - Rate/Spread（該当する場合）

4. The Pricer UI shall Cashflowテーブルをソート可能にする（Payment Date、Notional等でソート）。

5. While 多数のCashflowが存在する場合, the Pricer UI shall ページネーションまたは仮想スクロールを使用して表示パフォーマンスを維持する。

### Requirement 5: REST API エンドポイント

**Objective:** 開発者として、Trade展開機能にアクセスするためのREST APIを提供したい。これにより、フロントエンドとの統合が可能になる。

#### Acceptance Criteria

1. The Pricer Backend shall 以下のREST APIエンドポイントを公開する:
   - `POST /api/trade/expand` - パラメータからTrade展開を実行

2. When `POST /api/trade/expand` リクエストを受信した時, the Pricer Backend shall 以下のJSONスキーマに従ったリクエストを受け付ける:
   ```json
   {
     "instrument_type": "string",
     "params": { /* instrument-specific parameters */ }
   }
   ```

3. When Trade展開が成功した時, the API shall 以下のJSONスキーマに従ったレスポンスを返す:
   ```json
   {
     "trade_id": "string",
     "trade_type": "string",
     "legs": [
       {
         "leg_number": "number",
         "direction": "string",
         "currency": "string",
         "leg_type": "string",
         "cashflows": [
           {
             "payment_date": "string (ISO 8601)",
             "accrual_start": "string (ISO 8601)",
             "accrual_end": "string (ISO 8601)",
             "year_fraction": "number",
             "notional": "number",
             "payoff_type": "string",
             "rate": "number | null",
             "spread": "number | null"
           }
         ]
       }
     ],
     "metadata": {
       "total_legs": "number",
       "total_cashflows": "number",
       "processing_time_ms": "number"
     }
   }
   ```

4. If エラーが発生した場合, the API shall 適切なHTTPステータスコード（400 Bad Request、500 Internal Server Error等）と構造化されたエラーレスポンスを返す。

### Requirement 6: Instrumentタイプ一覧API

**Objective:** 開発者として、利用可能なInstrumentタイプとその必須パラメータを取得するAPIを提供したい。これにより、UIの動的フォーム生成が容易になる。

#### Acceptance Criteria

1. The Pricer Backend shall 以下のREST APIエンドポイントを公開する:
   - `GET /api/instruments` - 利用可能なInstrumentタイプ一覧を取得

2. When `GET /api/instruments` リクエストを受信した時, the API shall 以下の情報を含むレスポンスを返す:
   - Instrumentタイプ名
   - アセットクラス（Rates、FX、Equity、Credit、Commodity）
   - 必須パラメータのリスト（名前、型、バリデーションルール）
   - オプションパラメータのリスト

3. The API shall 各パラメータのデフォルト値（存在する場合）を含める。

---

## Out of Scope

- プライシング（PV計算）機能：本仕様ではTrade展開（CF生成）のみを対象とする
- Greeks計算：既存機能は維持するが、本仕様では拡張しない
- 永続化（データベース保存）：生成されたTradeはセッション内のみで保持
- Credit/Commodity Instrumentの完全実装：プレースホルダーのみ提供

## Technical Notes

- フロントエンド: 既存のindex.html、app.jsを拡張
- バックエンド: demo/gui/web/handlers.rsに新規エンドポイントを追加
- Trade構造: infra_domain::trade モジュールのTradeBuilder、LegBuilder、Cashflow等を使用
- 型定義: demo/gui/src/web/pricer_types.rsに新規型を追加
