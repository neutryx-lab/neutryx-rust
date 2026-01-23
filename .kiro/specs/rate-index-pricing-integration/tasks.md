# Implementation Plan

## Task Overview

本実装計画は、RateIndex を Neutryx プライシングパイプライン全体に統合するための作業を定義する。4つのフェーズ（infra_master → pricer_models → pricer_pricing → demo/gui）に沿って、段階的に実装を進める。

## Tasks

### Phase 1: infra_master 拡張

- [ ] 1. コンパウンディング方式の列挙型を追加
- [ ] 1.1 コンパウンディング方式を表す列挙型を作成する
  - Simple（単利）、Compounded（複利）、Averaged（平均化）の3つのバリアントを定義
  - Default トレイトを実装し、Simple をデフォルト値に設定
  - serde 対応（feature = "serde" での条件付きコンパイル）
  - Debug, Clone, Copy, PartialEq, Eq, Hash を derive
  - _Requirements: 1.4, 2.2_

- [ ] 1.2 (P) RateIndex にフィクシングメタデータを提供する機能を追加する
  - フィクシングカレンダー、公表ラグ、フィクシングオフセット、コンパウンディング方式を保持する構造体を作成
  - 各 RateIndex バリアント（SOFR, TONAR, EURIBOR3M/6M, SONIA, SARON）に対応するメタデータ値を定義
  - SOFR: NewYork カレンダー、公表ラグ 1 日、Compounded
  - SONIA: London カレンダー、公表ラグ 0 日、Compounded
  - EURIBOR: Target カレンダー、公表ラグ 2 日、Simple
  - metadata() メソッドを RateIndex に追加（const fn）
  - 既存の currency(), tenor(), day_counter() メソッドとの互換性を維持
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_

- [ ] 1.3 (P) IndexObservation を拡張してコンパウンディング設定をサポートする
  - リセット頻度フィールドを追加（OIS: Daily、IBOR: 期間に応じる）
  - コンパウンディング方式フィールドを追加
  - ルックバック期間とロックアウト期間のオプションフィールドを追加
  - RateIndex から適切なデフォルト設定で IndexObservation を作成するファクトリメソッドを追加
  - OIS インデックスの場合は Compounded、IBOR の場合は Simple をデフォルトに設定
  - 既存の new(), with_lag(), with_source() メソッドとの後方互換性を維持
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

- [ ] 1.4 infra_master の単体テストを追加する
  - 全 RateIndex バリアントのメタデータ値を検証
  - OIS インデックスと IBOR インデックスのデフォルトコンパウンディング方式を検証
  - IndexObservation のファクトリメソッドの自動設定を検証
  - 既存テストが通過することを確認
  - _Requirements: 1.5, 2.5, 2.6, 10.1, 10.3_

### Phase 2: pricer_models 拡張

- [ ] 2. RateIndex からカーブへのマッピング機能を追加
- [ ] 2.1 RateIndex を CurveName に変換するマッパーを作成する
  - RateIndex を CurveName に変換するトレイトを定義
  - 全 RateIndex バリアントに対応するマッチングを実装
  - SOFR → Sofr、EURIBOR3M/6M → Euribor、SONIA → Sonia、TONAR → Tonar、SARON → Saron
  - サポートされていないインデックスの場合は適切なエラーを返す
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

- [ ] 2.2 CurveSet に RateIndex でカーブを取得する機能を追加する
  - RateIndex を引数に取りカーブを返すメソッドを追加
  - 内部でマッパーを使用して CurveName に変換後、既存の get() を呼び出す
  - カーブが存在しない場合は CurveNotFound エラーを返す
  - Float トレイト境界を使用して AD 互換性を維持
  - _Requirements: 3.5, 3.6, 4.5, 10.4_

- [ ] 2.3 インデックス対応のフォワードレート計算メソッドを追加する
  - RateIndex、開始時点、終了時点を引数に取りフォワードレートを返すメソッドを追加
  - インデックスの day_counter に基づいた年率換算をサポート
  - Float トレイト境界を使用して f64 と Dual64 の両方で動作
  - _Requirements: 4.1, 4.2, 4.5, 10.2_

- [ ] 2.4 pricer_models の単体テストを追加する
  - 全 RateIndex のマッピングを検証
  - 存在するカーブと存在しないカーブの取得をテスト
  - フォワードレート計算の精度を検証
  - AD 互換性（Dual64 での計算）を検証
  - _Requirements: 3.6, 4.5, 10.2_

### Phase 3: pricer_pricing 統合

- [ ] 3. Payoff 評価機能を実装
- [ ] 3.1 Payoff バリアントを評価する機能を作成する
  - CurveSet への参照を保持する評価器を作成
  - Fixed Payoff: notional × rate × year_fraction を計算
  - Linear Payoff: インデックスからフォワードレートを取得し、スプレッドとマルチプライヤを適用
  - カーブが存在しない場合は MissingMarketData エラーを返す
  - Float トレイト境界を使用して AD 互換性を維持
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.6_

- [ ] 3.2 (P) OIS キャッシュフローの日次複利計算機能を作成する
  - 日次アクルーアルのリストから複利レートを計算（∏(1 + r_i × δ_i) - 1）
  - 複利レートを年率換算するヘルパーメソッドを提供
  - 空のアクルーアルリストの場合はゼロを返す
  - Float トレイト境界を使用して AD 互換性を維持
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [ ] 3.3 Cap/Floor オプションの評価機能を追加する
  - VanillaOption Payoff: Black/Bachelier モデルでオプション価値を計算
  - ボラティリティサーフェスをオプショナルに保持
  - Call オプション: max(forward_rate - strike, 0) のペイオフ
  - Put オプション: max(strike - forward_rate, 0) のペイオフ
  - ボラティリティサーフェスが存在しない場合は MissingVolatility エラーを返す
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [ ] 3.4 GenericPricer に Payoff 評価を統合する
  - price_leg メソッド内で Payoff 評価器を使用するように修正
  - cf.notional を使用し、ハードコードされた値を除去
  - daily_accruals が存在する場合は日次複利計算を使用
  - daily_accruals が空の場合は期間全体のフォワードレートを使用
  - 既存の get_notional_for_cashflow メソッドを維持
  - _Requirements: 5.2, 5.5, 6.1, 6.3, 10.5_

- [ ] 3.5 pricer_pricing の統合テストを追加する
  - Fixed, Linear, VanillaOption の各 Payoff タイプをテスト
  - OIS 複利計算の精度を既知の値と比較
  - GenericPricer を通じた全 Payoff バリアントの統合テスト
  - AD 互換性（Dual64 での計算）を検証
  - _Requirements: 5.6, 6.5, 10.2_

### Phase 4: demo/gui 拡張

- [ ] 4. Demo WebApp の DTO を拡張
- [ ] 4.1 (P) 入力 DTO にインデックス指定機能を追加する
  - SwapParams に rate_index オプションフィールドを追加
  - RatesParams に rate_index オプションフィールドを追加
  - "SOFR", "EURIBOR3M", "EURIBOR6M", "SONIA", "TONAR", "SARON" を受け付ける
  - rate_index が指定されない場合は通貨に基づくデフォルトインデックスを使用
  - 無効な rate_index 値の場合は InvalidInput エラーを返す
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [ ] 4.2 (P) 出力 DTO にインデックス情報を追加する
  - LegDto に rate_index オプションフィールドを追加
  - CashflowDto に rate_index オプションフィールドを追加
  - 変動レッグの場合は対応する rate_index を含める
  - Payoff が Linear または VanillaOption の場合は rate_index を含める
  - skip_serializing_if を使用して None の場合は出力しない
  - _Requirements: 9.1, 9.2, 9.3, 9.4_

- [ ] 4.3 トレード変換ロジックを更新する
  - convert_trade_to_dto で Payoff::required_index() を使用してインデックス情報を抽出
  - 入力の rate_index を RateIndex に変換する処理を追加
  - トレード構築時にインデックスを Payoff に設定
  - _Requirements: 8.3, 9.5_

- [ ] 4.4 demo/gui の API テストを追加する
  - rate_index を指定したスワップ作成をテスト
  - デフォルトインデックスの適用をテスト
  - 無効な rate_index のエラーハンドリングをテスト
  - レスポンスに rate_index が含まれることを検証
  - _Requirements: 8.4, 8.5, 9.3, 9.4_

### Phase 5: 最終統合と検証

- [ ] 5. エンドツーエンド統合と後方互換性検証
- [ ] 5.1 既存テストの回帰テストを実行する
  - infra_master、pricer_models、pricer_pricing の全既存テストを実行
  - テスト失敗がないことを確認
  - 実行時間が 10% 以上悪化していないことを確認
  - _Requirements: 10.1_

- [ ] 5.2 AD 互換性の検証テストを追加する
  - f64 と Dual64 の両方で全数値計算が動作することを検証
  - PayoffEvaluator、OisCalculator の AD 互換性をテスト
  - GenericPricer を通じた感度計算が正しく動作することを確認
  - _Requirements: 10.2, 5.6, 6.5_

- [ ] 5.3 l1l2-integration feature フラグの動作を検証する
  - feature 有効時: 全機能が正常動作
  - feature 無効時: pricer_pricing がスタンドアロンモードで動作
  - 条件付きコンパイルが正しく機能することを確認
  - _Requirements: 10.5_

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1 | 1.1, 1.2, 1.4 |
| 2 | 1.1, 1.3, 1.4 |
| 3 | 2.1, 2.2, 2.4 |
| 4 | 2.2, 2.3, 2.4 |
| 5 | 3.1, 3.4, 3.5 |
| 6 | 3.2, 3.4, 3.5 |
| 7 | 3.3 |
| 8 | 4.1, 4.3, 4.4 |
| 9 | 4.2, 4.3, 4.4 |
| 10 | 1.4, 2.4, 3.5, 5.1, 5.2, 5.3 |

## Parallel Execution Notes

以下のタスクは並列実行可能（`(P)` マーク）:
- **1.2 と 1.3**: CompoundingMethod（1.1）完了後、IndexMetadata と IndexObservation は独立して実装可能
- **3.2**: OisCalculator は PayoffEvaluator と並列実装可能（異なる責務）
- **4.1 と 4.2**: 入力 DTO と出力 DTO は独立して実装可能

依存関係により並列不可:
- **2.2 は 2.1 に依存**: CurveSet 拡張は IndexCurveMapper が必要
- **3.4 は 3.1, 3.2, 3.3 に依存**: GenericPricer 統合は全評価機能が必要
- **4.3 は 4.1, 4.2 に依存**: トレード変換は DTO 拡張が必要
