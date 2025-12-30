# Implementation Plan

## Tasks

- [x] 1. モジュール基盤とエラー型の実装
- [x] 1.1 instruments モジュール構造の作成
  - pricer_models に instruments モジュールを追加
  - サブモジュール構成を定義（error, payoff, params, options, forwards, swaps）
  - lib.rs に公開エクスポートを追加
  - _Requirements: 1.1_

- [x] 1.2 InstrumentError 型の実装
  - InvalidStrike、InvalidExpiry、InvalidNotional バリアントを定義
  - PayoffError、InvalidParameter バリアントを追加
  - PricingError への From 変換を実装
  - thiserror による Display 実装
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [x] 2. ペイオフと共通パラメータの実装
- [x] 2.1 (P) PayoffType enum の実装
  - Call、Put、DigitalCall、DigitalPut バリアントを定義
  - evaluate メソッドで smooth_max / smooth_indicator を使用
  - epsilon パラメータによる滑らかさ制御
  - Copy, Clone, Debug, PartialEq derive を追加
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 9.1, 9.2, 9.3, 9.4, 9.5_

- [x] 2.2 (P) InstrumentParams 構造体の実装
  - strike, expiry, notional フィールドを定義
  - new コンストラクタで正値バリデーション
  - 各フィールドのアクセサメソッドを実装
  - T: Float ジェネリクスで AD 互換性確保
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [x] 2.3 (P) ExerciseStyle enum の実装
  - European、American バリアントを定義
  - Bermudan バリアントで exercise_dates を格納
  - Asian バリアントで averaging_start, averaging_end, num_observations を格納
  - Clone, Debug, PartialEq derive を追加
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 3. 金融商品構造体の実装
- [x] 3.1 VanillaOption 構造体の実装
  - params, payoff_type, exercise_style, epsilon フィールドを組み合わせ
  - payoff メソッドで PayoffType::evaluate を呼び出し
  - アクセサメソッド（params, payoff_type, exercise_style）を実装
  - Clone, Debug derive を追加
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [x] 3.2 (P) Direction enum と Forward 構造体の実装
  - Direction enum（Long, Short）を定義
  - strike, expiry, notional, direction フィールドを定義
  - new コンストラクタで expiry 正値バリデーション
  - payoff メソッドで線形ペイオフ計算（Long: S-K, Short: K-S）
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 3.3 (P) PaymentFrequency enum と Swap 構造体の実装
  - PaymentFrequency enum（Annual, SemiAnnual, Quarterly, Monthly）を定義
  - notional, fixed_rate, payment_dates, frequency, currency フィールドを定義
  - new コンストラクタで payment_dates 順序検証
  - Clone, Debug derive を追加
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [x] 4. Instrument enum と統合
- [x] 4.1 Instrument enum の実装
  - Vanilla(VanillaOption<T>)、Forward(Forward<T>)、Swap(Swap<T>) バリアントを定義
  - payoff メソッドで各バリアントに静的ディスパッチ
  - expiry メソッドで各バリアントの満期を返却
  - Clone, Debug derive を追加
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

- [x] 4.2 モジュール公開エクスポートの最終確認
  - lib.rs からの公開 API を確認
  - ドキュメントコメントの追加（British English）
  - 型エイリアスやプレリュードの整備
  - _Requirements: 1.1_

- [x] 5. AD 互換性と統合テスト
- [x] 5.1 Dual64 による AD 互換性テスト
  - VanillaOption と Dual64 でデルタ伝播を検証
  - Forward と Dual64 で線形ペイオフの微分を検証
  - PayoffType::evaluate で smooth_max 微分連続性を検証
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5_

- [x] 5.2 プロパティベーステスト
  - Call/Put ペイオフの非負性を検証
  - smooth_max の epsilon 収束性を検証
  - Put-Call パリティ近似を検証（該当する場合）
  - _Requirements: 9.4, 9.5, 10.4_

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1-1.5 | 1.1, 4.1 |
| 2.1-2.5 | 2.1 |
| 3.1-3.5 | 2.2 |
| 4.1-4.5 | 3.1 |
| 5.1-5.5 | 2.3 |
| 6.1-6.5 | 3.2 |
| 7.1-7.5 | 3.3 |
| 8.1-8.5 | 1.2 |
| 9.1-9.5 | 2.1, 5.2 |
| 10.1-10.5 | 5.1, 5.2 |
