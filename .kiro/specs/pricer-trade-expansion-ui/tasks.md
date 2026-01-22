# Implementation Plan

## Overview

本タスクリストは、Frictional Bank Web App の Pricer 画面を拡張し、すべての対応 Instrument タイプを選択可能にし、CF 展開された Trade を生成・表示する機能を実装する。

## Tasks

### Backend Foundation

- [x] 1. Trade 展開 API 型定義
- [x] 1.1 (P) Instrument タイプ enum の実装
  - 全カテゴリ（Rates、FX、Equity）の Instrument タイプを定義
  - serde による snake_case シリアライズ設定
  - 将来拡張用の Credit/Commodity プレースホルダーを追加
  - _Requirements: 1.1_

- [x] 1.2 (P) Instrument パラメータ型の実装
  - Rates 系パラメータ（通貨、開始日、テナー、レート、想定元本）を定義
  - Swap 系パラメータ（固定金利、スプレッド、支払頻度、日数計算方式）を定義
  - FX 系パラメータ（通貨ペア、スポット/フォワードレート、ストライク、満期）を定義
  - Equity 系パラメータ（原資産、価格、行使価格、ボラティリティ）を定義
  - タグ付き union による型安全なパラメータ受け渡しを実装
  - _Requirements: 2.1, 2.2, 2.3, 2.4_

- [x] 1.3 Trade 展開リクエスト/レスポンス型の実装
  - TradeExpandRequest（Instrument タイプとパラメータ union）を定義
  - TradeExpandResponse（Trade ID、タイプ、Leg 配列、メタデータ）を定義
  - LegDto/CashflowDto による infra_master 型からの変換用 DTO を定義
  - camelCase による JSON シリアライズ設定
  - _Requirements: 3.3, 5.2, 5.3_

### Schedule Generation

- [x] 2. スケジュール生成機能
- [x] 2.1 支払いスケジュール生成ロジックの実装
  - 開始日、テナー、支払頻度からスケジュールを計算
  - 月末ルール（EndOfMonthRule）の適用
  - 各種頻度（Monthly、Quarterly、SemiAnnual、Annual）への対応
  - infra_master::time の Tenor/Frequency 型を活用
  - _Requirements: 3.2_

- [x] 2.2 スケジュール生成の単体テスト
  - 各テナー/頻度の組み合わせをテスト
  - 月末日開始の場合の特殊ケースを検証
  - 閏年を跨ぐケースを検証
  - _Requirements: 3.2_

### Trade Expansion Handler

- [x] 3. Trade 展開 API ハンドラ
- [x] 3.1 Rates 系 Instrument の Trade 展開ハンドラ
  - Deposit（単一 CF）の展開ロジックを実装
  - FRA（Forward Rate Agreement）の展開ロジックを実装
  - Futures の展開ロジックを実装
  - ParSwap/OIS の展開ロジックを実装
  - BasisSwap（2 Leg）の展開ロジックを実装
  - IRS（Fixed/Floating 2 Leg）の展開ロジックを実装
  - _Requirements: 2.1, 2.2, 3.1, 3.2_

- [x] 3.2 (P) FX 系 Instrument の Trade 展開ハンドラ
  - FxForward（通貨交換）の展開ロジックを実装
  - FxOption（Option Payoff）の展開ロジックを実装
  - CrossCurrencySwap（複数通貨 Leg）の展開ロジックを実装
  - _Requirements: 2.3, 3.1, 3.2_

- [x] 3.3 (P) Equity 系 Instrument の Trade 展開ハンドラ
  - VanillaOption（Call/Put）の展開ロジックを実装
  - EquityForward（Long/Short）の展開ロジックを実装
  - _Requirements: 2.4, 3.1, 3.2_

- [x] 3.4 infra_master 型から DTO への変換ロジック
  - Trade → TradeExpandResponse 変換を実装
  - Leg → LegDto 変換を実装
  - Cashflow → CashflowDto 変換を実装
  - 処理時間メトリクスの計測とメタデータ追加
  - _Requirements: 3.3, 5.3_

- [x] 3.5 入力バリデーションとエラーハンドリング
  - 必須フィールドの存在チェック
  - 数値範囲の検証（notional > 0、rate 範囲等）
  - 日付フォーマットの検証（ISO 8601）
  - 構造化エラーレスポンスの生成（フィールド名、エラーメッセージ）
  - _Requirements: 2.5, 3.4, 5.4_

### REST API Endpoints

- [x] 4. REST API エンドポイント実装
- [x] 4.1 POST /api/trade/expand エンドポイント
  - Axum ルーターへのルート追加
  - リクエスト JSON のデシリアライズ処理
  - Instrument タイプに応じた展開ハンドラへのディスパッチ
  - レスポンス JSON のシリアライズと返却
  - _Requirements: 5.1, 5.2, 5.3, 5.4_

- [x] 4.2 (P) GET /api/instruments エンドポイント
  - 利用可能な Instrument タイプ一覧の返却
  - アセットクラス別グループ化情報の追加
  - 各 Instrument タイプの必須/オプションパラメータ情報の返却
  - デフォルト値情報の追加
  - _Requirements: 6.1, 6.2, 6.3_

- [x] 4.3 demo/gui Cargo.toml への依存追加
  - infra_master の serde feature を有効化
  - 必要な依存関係の追加
  - _Requirements: 5.1_

### Frontend - Instrument Selector

- [x] 5. Instrument セレクタ UI の拡張
- [x] 5.1 Instrument ドロップダウンの拡張
  - GET /api/instruments からメタデータを取得
  - アセットクラス別 optgroup でグループ化表示
  - Rates、FX、Equity、Credit、Commodity カテゴリの表示
  - 選択時のイベントハンドリング
  - _Requirements: 1.1, 1.2, 1.3_

### Frontend - Dynamic Forms

- [x] 6. Instrument 別動的フォーム生成
- [x] 6.1 (P) Rates 系フォームの実装
  - Currency、Start Date、Tenor、Rate、Notional 入力フィールド
  - Swap 系追加フィールド（Fixed Rate、Spread、Payment Frequency、Day Count）
  - Instrument タイプに応じたフィールド表示/非表示の切り替え
  - _Requirements: 2.1, 2.2_

- [x] 6.2 (P) FX 系フォームの実装
  - Base Currency、Quote Currency、Spot Rate、Forward Rate/Strike 入力フィールド
  - Expiry、Notional、Option Type（FxOption の場合）入力フィールド
  - _Requirements: 2.3_

- [x] 6.3 (P) Equity 系フォームの実装
  - Underlying、Spot Price、Strike、Expiry 入力フィールド
  - Volatility、Risk-free Rate 入力フィールド
  - Option Type（VanillaOption）、Direction（Forward）入力フィールド
  - _Requirements: 2.4_

- [x] 6.4 フォームバリデーションの実装
  - 必須フィールドの入力チェック
  - 数値フィールドの範囲検証
  - 日付フォーマットの検証
  - エラーメッセージの視覚的表示
  - _Requirements: 2.5_

### Frontend - Trade Display

- [x] 7. Trade/Cashflow 表示 UI
- [x] 7.1 Trade サマリーカードの実装
  - Trade ID、Trade Type の表示
  - 合計 Leg 数、合計 Cashflow 数の表示
  - 展開成功/失敗のステータス表示
  - _Requirements: 4.1_

- [x] 7.2 Leg カードの実装
  - Leg 番号、Direction、Currency、Leg Type の表示
  - Cashflow 件数の表示
  - クリックによる展開/折りたたみ機能
  - _Requirements: 4.2, 4.3_

- [x] 7.3 Cashflow テーブルの実装
  - Payment Date、Accrual Start/End、Year Fraction 列の表示
  - Notional、Payoff Type、Rate/Spread 列の表示
  - 列ヘッダークリックによるソート機能
  - _Requirements: 4.3, 4.4_

- [x] 7.4 Cashflow ページネーションの実装
  - 20 件/ページでのページネーション
  - ページ番号表示と前後ナビゲーション
  - 多数 Cashflow 時の表示パフォーマンス維持
  - _Requirements: 4.5_

### Integration & Testing

- [x] 8. 統合とテスト
- [x] 8.1 フロントエンド-バックエンド結合テスト
  - 展開ボタンクリックから API 呼び出しまでのフロー検証
  - レスポンス受信から Trade 表示までのフロー検証
  - エラーレスポンス時の UI 表示検証
  - _Requirements: 3.1, 4.1_

- [x] 8.2 各 Instrument タイプの E2E テスト
  - Rates 系（Deposit、FRA、IRS、OIS、BasisSwap）の展開テスト
  - FX 系（FxForward、FxOption、CrossCurrencySwap）の展開テスト
  - Equity 系（VanillaOption、EquityForward）の展開テスト
  - _Requirements: 1.1, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3_

- [x] 8.3 エラーケースのテスト
  - 無効なパラメータ入力時のエラー表示
  - 未対応 Instrument タイプ選択時のエラー表示
  - ネットワークエラー時の graceful degradation
  - _Requirements: 2.5, 3.4, 5.4_

## Requirements Coverage Matrix

| Requirement | Tasks |
|-------------|-------|
| 1.1 Instrument タイプ一覧表示 | 1.1, 5.1, 8.2 |
| 1.2 動的フォーム表示 | 5.1 |
| 1.3 アセットクラス別グループ化 | 5.1 |
| 2.1 Rates 系入力フォーム | 1.2, 3.1, 6.1, 8.2 |
| 2.2 Swap 系入力フォーム | 1.2, 3.1, 6.1, 8.2 |
| 2.3 FX 系入力フォーム | 1.2, 3.2, 6.2, 8.2 |
| 2.4 Equity 系入力フォーム | 1.2, 3.3, 6.3, 8.2 |
| 2.5 バリデーション | 3.5, 6.4, 8.3 |
| 3.1 展開ボタン処理 | 3.1, 3.2, 3.3, 8.1, 8.2 |
| 3.2 CF 展開ロジック | 2.1, 2.2, 3.1, 3.2, 3.3, 8.2 |
| 3.3 展開レスポンス | 1.3, 3.4, 8.2 |
| 3.4 エラーレスポンス | 3.5, 8.3 |
| 4.1 Trade サマリー表示 | 7.1, 8.1 |
| 4.2 Leg カード表示 | 7.2 |
| 4.3 Cashflow テーブル表示 | 7.2, 7.3 |
| 4.4 ソート機能 | 7.3 |
| 4.5 ページネーション | 7.4 |
| 5.1 POST /api/trade/expand | 4.1, 4.3 |
| 5.2 リクエストスキーマ | 1.3, 4.1 |
| 5.3 レスポンススキーマ | 1.3, 3.4, 4.1 |
| 5.4 エラーレスポンス | 3.5, 4.1, 8.3 |
| 6.1 GET /api/instruments | 4.2 |
| 6.2 Instrument メタデータ | 4.2 |
| 6.3 パラメータ情報 | 4.2 |
