# Implementation Plan

## Task Summary

| Metric | Value |
|--------|-------|
| Major Tasks | 6 |
| Sub-tasks | 14 |
| Requirements Covered | 1-8 (全 40 受入条件) |
| Estimated Duration | 各サブタスク 1-3 時間 |

## Implementation Tasks

- [x] 1. エラー型と基盤構造の定義
- [x] 1.1 (P) コンパイルエラー型の実装
  - thiserror を使用した構造化エラー enum を作成
  - InvalidMaturity、InvalidYearFraction、ConventionMismatch、InvalidConvention、UnsupportedInstrument の各バリアントを定義
  - 各バリアントに商品インデックスとレートID を含めるフィールドを追加
  - エラーメッセージは問題の特定に十分な情報を提供
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_

- [x] 1.2 (P) 商品タイプ列挙型の定義
  - InstrumentType enum を作成（Deposit, Swap, Ois, Fra, Futures）
  - as_str() メソッドで文字列表現を提供
  - Clone, Copy, Debug トレイトを派生
  - _Requirements: 1.4_

- [x] 2. CompiledInstrument コア構造の実装
- [x] 2.1 事前計算済み商品構造体の定義
  - CompiledInstrument<T> 構造体を pricer_models::builder モジュールに作成
  - instrument_type、market_rate、maturity、cashflow_times、year_fractions、notionals、df_indices、fixed_rate フィールドを定義
  - cashflow_times()、year_fractions()、notionals() アクセサメソッドを実装
  - Clone、Debug トレイトを派生
  - 不変条件（配列長一致、昇順、正値）をコンストラクタで検証
  - _Requirements: 1.2, 2.5, 3.5_
  - _Contracts: CompiledInstrument State_

- [x] 2.2 CalibrationInstrument トレイト実装
  - CompiledInstrument<T> に CalibrationInstrument<T> トレイトを実装
  - market_rate() は保持値を直接返却
  - theoretical_rate() は DF 取得とベクトル積のみで計算（カレンダー演算なし）
  - pricing_error() は theoretical_rate と market_rate の差を返却
  - maturity() と instrument_type() を実装
  - O(n) 時間計算量を維持（n = キャッシュフロー数）
  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _Contracts: CalibrationInstrument<T> Service_

- [x] 3. InstrumentCompiler の実装
- [x] 3.1 コンパイラスキャフォールドと検証ロジック
  - InstrumentCompiler<T> 構造体を pricer_models::builder に配置
  - new(valuation_date) コンストラクタを実装
  - compile() メソッドのシグネチャを定義（infra_master::market::MarketInstrument を入力）
  - compile_batch() メソッドでバッチ処理をサポート
  - 満期日が評価日より前の場合に InvalidMaturity を返却
  - 負の年率係数に対して InvalidYearFraction を返却
  - XCcyBasis、FxForward、FxSwap に対して UnsupportedInstrument を返却
  - _Requirements: 1.1, 1.3, 1.5, 5.1, 5.2, 5.4, 8.1, 8.2, 8.3_
  - _Contracts: InstrumentCompiler Service_

- [x] 3.2 (P) Deposit 商品のコンパイル
  - Deposit コンベンションからキャッシュフロー情報を抽出
  - to_trade() を呼び出してキャッシュフロー日付を取得
  - 年率係数と想定元本を事前計算
  - CompiledInstrument を生成して返却
  - 単体テストで正常系とエラーケースを検証
  - _Requirements: 1.1, 1.4, 5.3_

- [x] 3.3 (P) Swap 商品のコンパイル
  - IRS コンベンションから固定・変動両レグのキャッシュフローを抽出
  - 固定レートを fixed_rate フィールドに格納
  - 両レグのキャッシュフロー日付、年率係数、想定元本を統合
  - 単体テストで複数キャッシュフローの正確性を検証
  - _Requirements: 1.1, 1.4, 5.3_

- [x] 3.4 (P) OIS 商品のコンパイル
  - OIS コンベンションから複利計算用キャッシュフローを抽出
  - 日次複利の年率係数を事前計算
  - OIS 固有の theoretical_rate 計算ロジックを CompiledInstrument に対応
  - 単体テストで OIS 特有の計算を検証
  - _Requirements: 1.1, 1.4, 5.3_

- [x] 3.5 FRA と Futures のコンパイル
  - FRA コンベンションから決済日と満期日のキャッシュフローを抽出
  - Futures コンベンションから IMM 日付とコンベクシティ調整を考慮
  - 両商品タイプの単体テストを作成
  - コンベンション不整合時の ConventionMismatch エラーを検証
  - 依存: 3.1 完了後に実行
  - _Requirements: 1.1, 1.4, 5.3, 8.3_

- [x] 4. CalibrationProblem 統合
- [x] 4.1 コンパイル済み商品からの構築メソッド
  - CalibrationProblem に from_compiled() メソッドを追加
  - CompiledInstrument のベクトルと InterpolationMatrix を受け取る
  - コンパイル済み商品の所有権を CalibrationProblem が管理
  - イテレーション中に再コンパイルしない設計を確認
  - 依存: 2.1, 2.2 完了後に実行
  - _Requirements: 2.1, 2.2, 2.4_
  - _Contracts: CalibrationProblem Service_

- [x] 4.2 CurveDefinition からの構築メソッド
  - from_market_instruments() メソッドを実装
  - 内部で InstrumentCompiler を使用して全商品をコンパイル
  - コンパイル完了時に商品数、総キャッシュフロー数、コンパイル時間をログ出力
  - コンパイルエラー時は部分状態を残さずエラーを伝播
  - 依存: 3.1-3.5, 4.1 完了後に実行
  - _Requirements: 2.1, 2.3, 2.4_

- [x] 4.3 後方互換性の検証
  - 既存 CalibrationProblem::new() のシグネチャが維持されていることを確認
  - CalibrationInstrument<T> トレイトが変更なく動作することを検証
  - 既存の MarketInstrument<T>（pricer_models）が従来通り使用可能であることをテスト
  - 既存のキャリブレーションテストが全てパスすることを確認（17件のテスト）
  - 依存: 4.1, 4.2 完了後に実行
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 5. InterpolationMatrix の拡張
- [x] 5.1 (P) ベクトル積による一括 DF 計算メソッド
  - apply() メソッドを InterpolationMatrix に追加
  - ピラー DF ベクトルから全キャッシュフロー日付の DF をベクトル積で計算
  - nalgebra の DVector を入出力に使用
  - SIMD 最適化が可能な連続メモリレイアウトを維持
  - 既存 interpolate() との結果一致を検証するテストを作成
  - _Requirements: 4.1, 4.3, 4.4_
  - _Contracts: InterpolationMatrix Service_

- [x] 5.2 log-linear 補間メソッド
  - apply_log_linear() メソッドを追加
  - log(DF) 空間で補間係数を計算
  - exp() を適用して最終 DF を返却
  - 補間係数の正規化を検証
  - 依存: 5.1 完了後に実行
  - _Requirements: 4.5_

- [x] 6. パフォーマンス検証
- [x] 6.1 ベンチマーク環境のセットアップ
  - criterion クレートをベンチマーク依存に追加（既存）
  - コンパイル前後の pricing_error 計算時間を比較するベンチマークを作成
  - 10 商品カーブキャリブレーションのベンチマークシナリオを定義
  - コンパイル時間を個別に測定するベンチマークを追加
  - ベンチマークファイル: `benches/instrument_compile.rs`
  - 依存: 4.2 完了後に実行
  - _Requirements: 7.1, 7.3_

- [x] 6.2 パフォーマンス目標の検証
  - イテレーションあたり **70%以上** の速度向上を達成（目標30%を大幅超過）
  - コンパイルコストが ~1µs と極めて軽量（キャリブレーション全体の5%未満を達成）
  - ベンチマーク結果:
    - evaluate() 10商品: 1.245µs → 350ns（71.9%改善）
    - evaluate() 12商品: 2.72µs → 591ns（78.3%改善）
    - jacobian() 10商品: 18.99µs → 4.63µs（75.6%改善）
  - 依存: 6.1 完了後に実行
  - _Requirements: 7.2, 7.4, 7.5_

## Requirements Coverage Matrix

| Requirement | Tasks | Coverage |
|-------------|-------|----------|
| 1 (Instrument Compiler) | 1.2, 2.1, 3.1-3.5 | 完全 |
| 2 (CalibrationProblem Integration) | 2.1, 4.1, 4.2 | 完全 |
| 3 (Efficient Pricing Error) | 2.1, 2.2 | 完全 |
| 4 (Interpolation Matrix) | 5.1, 5.2 | 完全 |
| 5 (Domain Separation) | 3.1-3.5 | 完全 |
| 6 (Backward Compatibility) | 4.3 | 完全 |
| 7 (Performance Verification) | 6.1, 6.2 | 完全 |
| 8 (Error Handling) | 1.1, 3.1, 3.5 | 完全 |

## Parallel Execution Notes

以下のタスクは並列実行可能（`(P)` マーカー付き）:
- 1.1, 1.2: 基盤型は相互依存なし
- 3.2, 3.3, 3.4: 各商品タイプのコンパイルは独立（3.1 完了後）
- 5.1: InterpolationMatrix 拡張は CompiledInstrument と並列可能

依存関係により順次実行が必要:
- 2.x → 3.x → 4.x → 6.x の主要フロー
- 5.2 は 5.1 に依存
- 6.2 は 6.1 に依存

---

_生成日: 2026-02-06_
