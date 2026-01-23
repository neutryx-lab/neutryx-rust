# Implementation Plan

## Overview

**Feature**: volcube-calibration-ui
**Total Tasks**: 7 major tasks, 21 sub-tasks
**Estimated Effort**: 各サブタスク1-3時間

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1 (1.1-1.10) | 2.1, 3.1, 4.2, 4.3, 5.1, 6.1 |
| 2 (2.1-2.5) | 2.2, 5.1 |
| 3 (3.1-3.6) | 2.1, 2.2, 5.1 |
| 4 (4.1-4.6) | 2.1, 2.2, 5.2 |
| 5 (5.1-5.6) | 2.2, 5.2 |
| 6 (6.1-6.6) | 2.2, 5.2 |
| 7 (7.1-7.6) | 2.2, 5.3 |
| 8 (8.1-8.8) | 2.1, 2.2, 2.3 |
| 9 (9.1-9.8) | 4.1, 4.2, 4.3 |
| 10 (10.1-10.8) | 1.1, 1.2, 1.3, 3.2, 6.2 |
| 11 (11.1-11.8) | 3.1, 3.2, 3.3 |

**Deferred**: Req 9.6 (Equity sample data) - Phase 2対応予定

---

## Tasks

- [x] 1. FxDensityCalculator バックエンド実装
- [x] 1.1 (P) Delta-Strike変換機能の実装
  - FX市場慣行に沿ったDelta（Spot Delta、Forward Delta、Premium-adjusted）からAbsolute Strikeへの変換ロジックを実装する
  - Garman-Kohlhagenモデルの逆算にBisectionソルバーを使用する（数値安定性のためBrentから変更）
  - Spot、Domestic Rate、Foreign Rate、Expiry、Volatilityを入力としてStrike値を出力する
  - Put Delta（負値）とCall Delta（正値）の両方に対応する
  - _Requirements: 10.5, 10.6_
  - _Contracts: FxDensityCalculator Service_

- [x] 1.2 確率密度計算機能の実装
  - Strike軸上でのボラティリティ補間を行い、Breeden-Litzenberger法による数値微分でPDF（確率密度関数）を計算する
  - Central difference法（h = 0.001 * strike）で2階微分を近似する
  - 計算結果の正規化（∫ density dK = 1）を検証する
  - 外挿はFxVolatilitySurfaceのallow_extrapolation設定に従う
  - 1.1のDelta-Strike変換機能に依存
  - _Requirements: 10.7_
  - _Contracts: FxDensityCalculator Service_

- [x] 1.3 確率密度統計量計算の実装
  - 期待値、分散、歪度、尖度の統計量を数値積分（台形則）で計算する
  - DensityStatistics構造体にまとめて返却する
  - 超過尖度（正規分布 = 0）を使用
  - _Requirements: 10.8_
  - _Contracts: FxDensityCalculator Service_

- [ ] 2. VolCube API実装
- [x] 2.1 (P) VolCube API型定義の実装
  - キャリブレーションモデル選択（SABR、SVI、LocalVolatility）の列挙型を定義する
  - Strike軸タイプ（Absolute、Moneyness、LogMoneyness、Delta）の列挙型を定義する
  - SwaptionInstrument構造体（expiry、tenor、strike、implied_vol、forward、weight）を定義する
  - キャリブレーションリクエスト・レスポンス型を定義する
  - SABRパラメータ出力型（alpha、beta、rho、nu）を定義する
  - スマイル・密度データレスポンス型を定義する
  - Serde属性でcamelCase JSON変換を設定する
  - _Requirements: 1.5, 3.1, 3.2, 3.3, 4.1, 4.2, 8.1, 8.2, 8.3, 8.4_
  - _Contracts: volcube_types API_

- [ ] 2.2 VolCube APIハンドラー基本エンドポイントの実装
  - `/api/volcube/indices` GETで利用可能なIndex一覧を返す
  - `/api/volcube/instruments/{index}` GETで指定IndexのJSON読み込みとデータ返却を行う
  - `/api/volcube/instruments/{index}` PUTでインストゥルメントデータの更新・保存を行う
  - JSONファイルは`demo/data/input/volsurface/`から読み込む
  - AppStateにvolcube_cacheを追加してLRUキャッシュを実装する
  - RFC 7807 ProblemDetailsでエラーハンドリングする
  - 2.1の型定義に依存
  - _Requirements: 1.4, 1.7, 1.8, 1.9, 2.1, 2.3, 8.1, 8.2, 8.3, 8.8_
  - _Contracts: volcube_handlers API_

- [ ] 2.3 VolCube APIキャリブレーション・分析エンドポイントの実装
  - `/api/volcube/calibrate` POSTでVolCubeBuilderを呼び出しキャリブレーションを実行する
  - `/api/volcube/smile` GETで指定Expiry/Tenorのスマイルデータを返す
  - `/api/volcube/density` GETでBreeden-Litzenberger法による確率密度データを返す
  - `/api/volcube/surface` GETで3Dサーフェス用グリッドデータを返す
  - キャリブレーション結果をキャッシュに格納する
  - 適合度メトリクス（RMSE、最大誤差、R²、処理時間）を計算・返却する
  - 2.2のハンドラー基盤に依存
  - _Requirements: 3.5, 3.6, 4.3, 5.2, 6.2, 6.3, 7.2, 8.4, 8.5, 8.6, 8.7_
  - _Contracts: volcube_handlers API_

- [ ] 3. FxVol API実装
- [x] 3.1 (P) FxVol API型定義の実装
  - FxQuoteEntry構造体（expiry、atm_vol、rr_25d、bf_25d、rr_10d、bf_10d）を定義する
  - FxVolFile構造体（currency_pair、spot、domestic_rate、foreign_rate、quotes）を定義する
  - DeltaType列挙型（SpotDelta、ForwardDelta、PremiumAdjusted）を定義する
  - Delta-Strike変換リクエスト・レスポンス型を定義する
  - FX密度レスポンス型（warnings配列含む）を定義する
  - Serde属性でcamelCase JSON変換を設定する
  - _Requirements: 1.6, 10.2, 10.5, 11.1, 11.2, 11.3, 11.8_
  - _Contracts: fxvol_types API_

- [ ] 3.2 FxVol APIハンドラー基本エンドポイントの実装
  - `/api/fxvol/pairs` GETで利用可能な通貨ペア一覧を返す
  - `/api/fxvol/quotes/{pair}` GETで指定通貨ペアのボラティリティQuotesを返す
  - `/api/fxvol/quotes/{pair}` PUTでQuotesデータの更新・保存を行う
  - `/api/fxvol/build` POSTでFxVolatilitySurfaceを構築する
  - AppStateにfxvol_cacheを追加してLRUキャッシュを実装する
  - 3.1の型定義に依存
  - _Requirements: 1.3, 1.4, 10.2, 11.1, 11.2, 11.3, 11.4_
  - _Contracts: fxvol_handlers API_

- [ ] 3.3 FxVol API分析エンドポイントの実装
  - `/api/fxvol/smile` GETで指定ExpiryのDelta-Volスマイルデータを返す
  - `/api/fxvol/rr-bf` GETでRisk Reversal/Butterflyの時系列データを返す
  - `/api/fxvol/density` GETでFxDensityCalculatorを使用して確率密度データを返す
  - `/api/fxvol/delta-strike` POSTでDelta-Strike変換結果を返す
  - 数値的問題発生時はwarnings配列に警告を含めて部分結果を返す
  - Task 1のFxDensityCalculatorおよび3.2のハンドラー基盤に依存
  - _Requirements: 10.1, 10.3, 10.4, 10.6, 10.7, 10.8, 11.5, 11.6, 11.7, 11.8_
  - _Contracts: fxvol_handlers API_

- [ ] 4. サンプルデータ準備
- [ ] 4.1 (P) ボラティリティデータディレクトリ構造の作成
  - `demo/data/input/volsurface/`ディレクトリを作成する
  - データフォーマット仕様をREADME.mdで文書化する
  - _Requirements: 9.1, 9.8_

- [ ] 4.2 (P) Swaptionサンプルデータの作成
  - USD-SOFR-Swaption用JSONファイルを作成する（複数Expiry × Tenor × Strikeグリッド）
  - EUR-ESTR-Swaption用JSONファイルを作成する
  - 各ファイルにrealisticな市場レート（implied_vol、forward）を含める
  - VolCube JSONスキーマに準拠（index、reference_date、instruments配列）
  - _Requirements: 9.2, 9.3, 9.7_

- [ ] 4.3 (P) FX Optionsサンプルデータの作成
  - EURUSD FX Options用JSONファイルを作成する（ATM、25D/10D RR/BF × 複数Expiry）
  - USDJPY FX Options用JSONファイルを作成する
  - 各ファイルにspot、domestic_rate、foreign_rateを含める
  - FxVol JSONスキーマに準拠（currency_pair、quotes配列）
  - _Requirements: 9.4, 9.5, 9.7_

- [ ] 5. VolCube フロントエンド実装
- [ ] 5.1 volcube-builder.js コア機能の実装
  - 資産クラス選択（Swaption、FX Options）とIndex選択ドロップダウンを実装する
  - インストゥルメントデータをテーブル形式で表示・編集可能にする
  - 依存カーブ選択セクションを実装し、Curve Builder APIと連携する
  - キャリブレーションモデル選択（SABR、SVI、Local Vol）と詳細設定UIを実装する
  - 「Calibrate」ボタンでAPI呼び出しと処理中インジケータを表示する
  - データのCSV/JSONエクスポート・インポート機能を実装する
  - Task 2のVolCube APIに依存
  - _Requirements: 1.1, 1.2, 1.8, 1.9, 1.10, 2.1, 2.2, 2.3, 2.4, 2.5, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [ ] 5.2 スマイル・確率密度チャートの実装
  - Expiry/Tenor選択スライダーを実装する
  - Chart.jsでスマイルカーブ（Strike vs Implied Vol）をプロットする
  - 市場観測点（マーカー）とモデル曲線（線）を重ねて表示する
  - 確率密度関数（PDF）と累積分布関数（CDF）の表示切替を実装する
  - 統計情報（期待値、分散、歪度、尖度）を表示する
  - Strike軸タイプ切替（Absolute、Moneyness、Delta）を実装する
  - チャートのPNG/SVGエクスポート機能を実装する
  - キャリブレーション結果パラメータテーブル（SABR alpha/beta/rho/nu）を表示する
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_

- [ ] 5.3 3Dボラティリティサーフェスの実装
  - Plotly.jsをCDN経由で読み込む
  - Expiry × Strike × Implied Volの3Dサーフェスを描画する
  - Tenor選択による3Dサーフェス切替を実装する
  - マウスドラッグ回転・ズーム操作を有効化する
  - カラーマップ選択（Viridis、Plasma、等高線）を実装する
  - 市場観測点を3Dマーカーとして表示する
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_

- [ ] 6. FxVol フロントエンド実装
- [ ] 6.1 fxvol-builder.js コア機能の実装
  - 通貨ペア選択ドロップダウンを実装する
  - ATM vol、25D/10D Risk Reversal、25D/10D Butterflyの入力フィールドを各Expiryで提供する
  - Spot価格、Domestic金利、Foreign金利の入力フィールドを実装する
  - RR/BFから5点Delta volへの自動変換機能を実装する
  - データのCSV/JSONエクスポート・インポート機能を実装する
  - Task 3のFxVol APIに依存
  - _Requirements: 1.3, 1.8, 1.9, 1.10, 10.2, 10.3, 10.5_

- [ ] 6.2 FXスマイル・密度分析UIの実装
  - Delta軸（10D Put、25D Put、ATM、25D Call、10D Call）でスマイルを表示する
  - Risk Reversal（スキュー）とButterfly（曲率）の時系列チャートを表示する
  - Delta-Strike変換結果をテーブル表示する
  - FX確率密度関数をChart.jsでプロットする
  - 統計情報（期待値、分散、歪度、尖度）を表示する
  - 数値的警告がある場合はUIに表示する
  - _Requirements: 10.1, 10.3, 10.4, 10.6, 10.7, 10.8_

- [ ] 7. 統合とテスト
- [ ] 7.1 APIルート登録とAppState拡張
  - VolCube API（/api/volcube/*）のルートを登録する
  - FxVol API（/api/fxvol/*）のルートを登録する
  - AppStateにvolcube_cache、fxvol_cacheを追加する
  - 既存のCurve Builder APIとの共存を確認する
  - _Requirements: 8.1, 11.1_

- [ ] 7.2 フロントエンドHTML統合
  - Model Calib画面にvolcube-builder.js、fxvol-builder.jsを読み込む
  - Plotly.js CDNリンクをHTMLに追加する
  - 資産クラス切替タブを実装する
  - 既存のCurve Builder UIとの整合性を確認する
  - _Requirements: 1.1, 1.2, 1.3_

- [ ] 7.3 E2Eテストと動作確認
  - VolCube: Index選択 → データ編集 → キャリブレーション → 結果表示の一連フローを検証する
  - FxVol: 通貨ペア選択 → RR/BF入力 → Delta-Strike変換 → 確率密度表示の一連フローを検証する
  - エラーケース（存在しないIndex、無効なパラメータ）のハンドリングを確認する
  - パフォーマンス目標（キャリブレーション < 500ms、スマイル取得 < 50ms）を検証する
  - _Requirements: 3.6, 6.6, 8.8_

---

## Parallel Execution Guide

以下のタスクは独立して並列実行可能:

**Group A (Backend Types)**: 2.1, 3.1 - API型定義は相互依存なし
**Group B (Sample Data)**: 4.1, 4.2, 4.3 - データファイル作成は独立
**Group C (Backend Core)**: 1.1 - FxDensityCalculatorのDelta-Strike変換は独立

**依存関係チェーン**:
- 1.1 → 1.2 → 1.3 (FxDensityCalculator内部依存)
- 2.1 → 2.2 → 2.3 (VolCube API依存)
- 3.1 → 3.2 → 3.3 (FxVol API依存)
- 1.3 + 3.2 → 3.3 (FxDensityCalculator完成後にFxVol density endpoint)
- 2.3 → 5.1 → 5.2 → 5.3 (VolCube Frontend依存)
- 3.3 → 6.1 → 6.2 (FxVol Frontend依存)
- 5.3 + 6.2 → 7.1 → 7.2 → 7.3 (統合フェーズ)
