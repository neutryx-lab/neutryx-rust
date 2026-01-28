# Implementation Plan

## Tasks

- [x] 1. データ構造の拡張
  - VolQuote、SabrBounds、SliceCalibrationConfigの拡張を実装

- [x] 1.1 (P) VolQuoteにexpiryフィールドを追加
  - VolQuote構造体にexpiry: Tフィールドを追加
  - new()コンストラクタを4パラメータ版に変更（strike, volatility, forward, expiry）
  - 後方互換性のためnew_without_expiry()コンストラクタを追加（expiry = T::one()）
  - 既存テストを新しいシグネチャに更新
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [x] 1.2 (P) SabrBounds構造体を実装
  - alpha_min/max、rho_min/max、nu_min/maxの6フィールドを持つ構造体を作成
  - Default実装でデフォルト境界値を設定（alpha: 1e-6〜1.0, rho: -0.99〜0.99, nu: 1e-6〜2.0）
  - LMソルバー用のf64アクセサメソッド（alpha_min_f64()等）を実装
  - _Requirements: 2.2, 2.3_

- [x] 1.3 SliceCalibrationConfigを拡張
  - initial_rho、initial_nu、lm_lambda、lm_lambda_factor、boundsフィールドを追加
  - Default::default()で新規フィールドのデフォルト値を設定
  - rates()プリセットをβ=0.5に適した設定で拡張
  - fx()プリセットをβ=1.0に適した設定で拡張
  - to_lm_config()メソッドでLMConfigへの変換を実装
  - _Requirements: 2.1, 2.4, 2.5, 2.6_

- [x] 2. SabrSliceCalibratorの完全実装
  - TODOプレースホルダーを実際のLMソルバー統合に置き換え

- [x] 2.1 SABR残差クロージャを構築
  - quotes、beta、forward、expiryをキャプチャするクロージャを実装
  - 3パラメータ（alpha, rho, nu）を受け取りクォート数と同じ長さのVec<f64>を返す
  - 各クォートに対してσ_market - σ_sabrの残差を計算
  - pricer_core::math::formulas::sabr::sabr_implied_vol()を使用してモデルボラティリティを計算
  - クロージャ外で境界値をf64に事前変換
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.7_

- [x] 2.2 LMソルバーとの統合を実装
  - ATMクォートから初期alphaを推定（α ≈ σ_ATM × F^(1-β)）
  - config.boundsを使用してパラメータ境界をclampで適用
  - LevenbergMarquardtSolver::new()とsolve()を呼び出して最適化を実行
  - 収束結果からSabrParamsを構築
  - SabrParams::validate()を呼び出して境界制約を検証
  - _Requirements: 3.1, 3.6, 4.1, 4.5, 4.6, 4.7_

- [x] 2.3 エラーハンドリングを実装
  - quotesが空の場合にCalibrationError::InsufficientDataを返す
  - LMソルバーエラー時にCalibrationError::NumericalInstabilityにラップして返す
  - 最大イテレーション内で収束しない場合にCalibrationError::ConvergenceFailureを返す
  - 成功時に検証済みSabrParamsを返す
  - _Requirements: 4.2, 4.3, 4.4, 6.1, 6.2, 6.3, 6.4_

- [x] 3. VolCubeBuilder APIの更新
  - expiry対応のAPIに更新しカリブレーションを統合

- [x] 3.1 add_quote()とadd_slice()を更新
  - add_quote()でexpiry, tenor, strike, volatility, forwardの5パラメータを受け取る
  - 内部でVolQuote::new()を使用してクォートを構築
  - (expiry, tenor)キーでスライスを管理
  - add_slice()でexpiry, tenor, Vec<VolQuote>を受け取り各quoteのexpiryを統一
  - _Requirements: 5.1, 5.2_

- [x] 3.2 calibrate()メソッドを実装
  - 各(expiry, tenor)スライスに対してSabrSliceCalibratorを使用
  - 各スライスのカリブレーション結果を収集
  - 有効なSabrParamsを持つVolCubeResultを返す
  - カリブレーションエラーを適切に伝播
  - _Requirements: 5.3, 5.4_

- [x] 4. demo/guiのリファクタリング
  - スタンドアロン関数を削除しpricer_models APIに移行

- [x] 4.1 スタンドアロン関数を削除
  - calibrate_sabr_simple()を削除
  - optimize_sabr()を削除
  - sabr_implied_vol()を削除
  - black_call_price()は密度計算に必要なため保持
  - _Requirements: 7.1_

- [x] 4.2 VolCubeBuilderを使用するハンドラに移行
  - pricer_models::builder::vol::SabrSliceCalibratorを使用するようにvolcube_handlersを更新
  - pricer_core::math::formulas::sabr::sabr_implied_volを使用するように変更
  - CalibrationErrorを適切にフォールバックで処理
  - _Requirements: 7.2, 7.3, 7.4, 7.5_

- [x] 5. テストと検証
  - 単体テスト、統合テスト、性能テストを実装

- [x] 5.1 (P) 単体テストを実装
  - VolQuote::new()とnew_without_expiry()のシグネチャテスト
  - SabrBounds::default()のデフォルト値テスト
  - SabrBounds f64アクセサの変換精度テスト
  - SliceCalibrationConfig::to_lm_config()の正確性テスト
  - ATMボラティリティからのalpha推定テスト
  - パラメータ境界制約のclampテスト
  - _Requirements: 8.1, 8.4, 8.7_

- [x] 5.2 統合テストを実装
  - 単一スライスカリブレーションの収束とパラメータ検証
  - 複数スライスVolCubeカリブレーションの動作検証
  - 空quotesに対するInsufficientDataエラーテスト
  - カリブレーション後のモデルvolが市場volに対して50bp未満の誤差であることを検証
  - 移行検証テスト：既存calibrate_sabr_simple()との結果比較（許容誤差1e-6以内）
  - _Requirements: 8.2, 8.3, 8.4, 8.5, 8.6_

- [x] 5.3 性能テストを実装
  - デフォルトで100イテレーション以内に収束することを検証
  - tolerance=1e-8がデフォルト収束基準であることを確認
  - 典型的なスワプションスマイルデータで50イテレーション以内に収束することを検証
  - 同じ入力で同じ出力が得られること（再現可能性）を検証
  - _Requirements: 9.1, 9.2, 9.3, 9.4_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1-1.4 | 1.1 |
| 2.1-2.6 | 1.2, 1.3 |
| 3.1-3.7 | 2.1, 2.2 |
| 4.1-4.7 | 2.2, 2.3 |
| 5.1-5.4 | 3.1, 3.2 |
| 6.1-6.4 | 2.3 |
| 7.1-7.5 | 4.1, 4.2 |
| 8.1-8.7 | 5.1, 5.2 |
| 9.1-9.4 | 5.3 |

---

_Generated: 2026-01-28_
_Specification: sabr-volcube-crates-migration_
_Phase: Tasks_
