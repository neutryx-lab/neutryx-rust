# Implementation Plan

## Task 1: Delta-Strike変換関数の実装

- [ ] 1.1 SpotDeltaとForwardDeltaの変換関数を実装する
  - デルタ値からストライク価格を計算する閉形式ソリューションを実装
  - 逆正規分布関数を使用したd1計算（SpotDelta: d1 = Φ⁻¹(Δ / e^(-rf×T))、ForwardDelta: d1 = Φ⁻¹(Δ)）
  - フォワード価格とボラティリティからストライクへの変換式を実装
  - Float ジェネリクスによるAD互換性を確保
  - _Requirements: 1.1, 1.2, 1.3, 1.6_

- [ ] 1.2 PremiumAdjustedデルタの変換を実装する
  - プレミアム調整デルタはストライクに依存するため反復解法を使用
  - Newton-Raphson法による収束ソルバーを実装
  - 収束判定条件と最大イテレーション数を設定
  - 収束失敗時にFormulaErrorを返す
  - _Requirements: 1.4_

- [ ] 1.3 strike_to_delta関数を実装する
  - ストライク価格からデルタ値への逆変換を実装
  - GarmanKohlhagenのデルタ計算を活用
  - 全DeltaType（SpotDelta, ForwardDelta, PremiumAdjusted）をサポート
  - _Requirements: 1.5_

- [ ] 1.4 Delta-Strike変換のユニットテストを作成する
  - delta_to_strike と strike_to_delta の往復変換テスト
  - 各DeltaType（SpotDelta, ForwardDelta, PremiumAdjusted）の精度検証
  - エッジケース（極端なデルタ値、短い満期、高ボラティリティ）のテスト
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

## Task 2: FXフォワードカーブの実装

- [ ] 2.1 (P) FxCurveトレイトとIrpFxCurveを実装する
  - FxCurveトレイトにspot()、forward(expiry)、currency_pair()メソッドを定義
  - Interest Rate Parityに基づくフォワード計算（F = S × df_foreign / df_domestic）
  - 国内・外国金利カーブをジェネリクスで受け取る構造体を設計
  - forward(0) = spot の不変条件を満たす
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [ ] 2.2 (P) FxCurveEnumを実装する
  - IrpFlat、IrpBootstrapped、Irpの3バリアントを定義
  - 各バリアントでspot()、forward()、currency_pair()をデリゲート
  - 静的ディスパッチによるEnzyme AAD互換性を確保
  - CurveEnumパターンとの一貫性を維持
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [ ] 2.3 FXフォワードカーブのユニットテストを作成する
  - フォワード価格計算の精度検証（既知の金利差に対する期待値）
  - forward(0) = spot の不変条件テスト
  - 複数満期でのフォワード計算テスト
  - _Requirements: 2.2, 2.3_

## Task 3: SABRキャリブレーションの実装

- [ ] 3.1 (P) LeastSquaresProblemトレイトを実装する
  - levenberg-marquardtクレートのLeastSquaresProblemを実装
  - SABR残差関数（σ_SABR(K) - σ_market）をベクトルとして定義
  - パラメータ数3（α, ρ, ν）、残差数は入力クォート数
  - ヤコビアン計算を実装（数値微分または解析的）
  - _Requirements: 4.2_

- [ ] 3.2 (P) SabrSliceCalibrator::calibrate_sliceを実装する
  - Levenberg-Marquardt最適化アルゴリズムを使用
  - 初期推定値を設定（α = σ_ATM × F^(1-β)、ρ = -0.2、ν = 0.3）
  - パラメータ境界制約をクリッピングで適用（α > 0、-1 < ρ < 1、ν > 0）
  - pricer_core::sabr_implied_volを使用してモデルボラティリティを計算
  - 収束失敗時にCalibrationError::NonConvergenceを返す
  - _Requirements: 4.1, 4.3, 4.4, 4.5, 4.6_

- [ ] 3.3 SABRキャリブレーションのユニットテストを作成する
  - 既知のSABRパラメータからクォートを生成し、キャリブレーション後にパラメータが一致することを確認
  - 収束失敗ケースのエラーハンドリング確認
  - 複数のスライスでのキャリブレーション精度検証
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6_

## Task 4: RR/BF変換ロジックの実装

- [ ] 4.1 DeltaVolSlice構造体を実装する
  - expiry、forward、atm_vol、vol_25d_call、vol_25d_put、vol_10d_call（Option）、vol_10d_put（Option）フィールドを定義
  - from_rr_bfコンストラクタを実装（ATM vol、25D RR、25D BFから各デルタ点のボラティリティを計算）
  - RR/BF変換式を適用（vol_25d_call = atm + bf_25d + rr_25d / 2）
  - _Requirements: 3.1, 3.2, 3.3, 3.4_

- [ ] 4.2 to_strike_vol_quotesメソッドを実装する
  - delta_to_strike関数を使用して各デルタ点のストライクを計算
  - ATM、25D Call、25D Put（およびオプションで10D）のVolQuoteを生成
  - 3〜5個のVolQuoteを含むベクターを返す
  - DeltaTypeパラメータに応じた変換を実行
  - _Requirements: 3.5, 3.6_

- [ ] 4.3 RR/BF変換のユニットテストを作成する
  - from_rr_bfによるボラティリティ計算の検証
  - to_strike_vol_quotesによるストライク変換の精度確認
  - 10Dクォートの有無による出力数の確認
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

## Task 5: FxVolBuilder拡張

- [ ] 5.1 FxVolBuilderにFxCurveとConventionを設定するメソッドを追加する
  - with_fx_curveメソッドでFxCurveEnumを受け取り内部フィールドに保持
  - with_conventionメソッドでFxVolConventionを設定
  - ビルダーパターンでメソッドチェーンをサポート
  - _Requirements: 5.1, 5.2_

- [ ] 5.2 FxVolInstrumentを受け取るメソッドを追加する
  - add_instrumentメソッドで単一のFxVolInstrumentを受け取る
  - 同じexpiryのインストゥルメントをBTreeMapでグループ化
  - add_instrumentsメソッドで複数インストゥルメントを一括追加
  - infra_domainの型定義を直接使用
  - _Requirements: 5.3, 5.4, 5.5, 5.6_

- [ ] 5.3 calibrateメソッドを実装する
  - ATM/RR/BFが揃ったスライスからDeltaVolSliceを構築
  - to_strike_vol_quotesでストライクベースに変換
  - SabrSliceCalibratorを呼び出して各スライスをキャリブレーション
  - FxVolResult（params + diagnostics）を返す
  - _Requirements: 5.4_

- [ ] 5.4 FxVolBuilder統合テストを作成する
  - FxVolInstrumentからのサーフェス構築フロー全体をテスト
  - 複数満期のキャリブレーション結果を検証
  - エラーケース（不足データ、無効な入力）のハンドリング確認
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

## Task 6: キャリブレーション診断情報の実装

- [ ] 6. SliceCalibrationDiagnosticsを追加しFxVolResultに含める
  - expiry、residual（最終残差SSE）、iterations、convergedフィールドを定義
  - calibrate_sliceの戻り値にDiagnosticsを含める
  - FxVolResultのdiagnosticsフィールドに各スライスの診断情報を格納
  - 収束成功時はconverged = true、失敗時はfalseと最終残差を記録
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

## Task 7: demo_gui簡略化

- [ ] 7.1 既存のキャリブレーションロジックを削除する
  - to_delta_volsメソッド、DeltaVols構造体を削除
  - delta_to_strike関数（ローカル実装）を削除
  - フォワード計算のインラインコードを削除
  - 削除対象のコードを特定しクリーンアップ
  - _Requirements: 7.1_

- [ ] 7.2 pricer_modelsへの委譲コードを実装する
  - FxVolInstrumentBuilderを使用してリクエストからFxVolInstrumentを構築
  - IrpFxCurveを構築してFxVolBuilderに渡す
  - FxVolBuilder.calibrate()を呼び出してキャリブレーションを実行
  - CalibrationErrorを適切なHTTPステータスコードに変換
  - _Requirements: 7.2, 7.3, 7.4, 7.5, 7.6, 7.7_

- [ ] 7.3 demo_guiのE2Eテストを作成する
  - /api/fxvol/calibrateエンドポイントの正常系テスト
  - 入力エラー（無効なDeltaType、負のボラティリティ）の400レスポンス確認
  - キャリブレーション失敗時の422レスポンス確認
  - 診断情報がレスポンスに含まれることを検証
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1, 1.2, 1.3, 1.4, 1.5, 1.6 | 1.1, 1.2, 1.3, 1.4 |
| 2.1, 2.2, 2.3, 2.4, 2.5 | 2.1, 2.2, 2.3 |
| 3.1, 3.2, 3.3, 3.4, 3.5, 3.6 | 4.1, 4.2, 4.3 |
| 4.1, 4.2, 4.3, 4.4, 4.5, 4.6 | 3.1, 3.2, 3.3 |
| 5.1, 5.2, 5.3, 5.4, 5.5, 5.6 | 5.1, 5.2, 5.3, 5.4 |
| 6.1, 6.2, 6.3, 6.4, 6.5 | 6 |
| 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7 | 7.1, 7.2, 7.3 |
| NFR-1, NFR-2, NFR-3, NFR-4 | All tasks |

## Parallel Execution Notes

- **Task 2** (FxCurve) と **Task 3** (SABR Calibration) は並行実行可能
- Task 4 は Task 1 に依存（delta_to_strike関数を使用）
- Task 5 は Task 1, 2, 4 に依存
- Task 6 は Task 3 に依存（SabrSliceCalibratorの拡張）
- Task 7 は Task 1-6 全てに依存

## Implementation Order

推奨実装順序: 1 → (2 || 3) → 4 → 5 → 6 → 7
