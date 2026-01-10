# Implementation Plan

## Overview

Heston、SABR、Hull-White 確率過程モデルの完成。SABR の StochasticModel 実装、全モデルのキャリブレーション機能、Hull-White の時間依存 θ(t) サポートを追加する。

**Total Tasks**: 6 Major Tasks, 17 Sub-tasks
**Requirements Coverage**: 7 Requirements, 39 Acceptance Criteria

---

## Tasks

- [x] 1. SABR StochasticModel トレイト実装

- [x] 1.1 SABR の evolve_step 実装
  - forward rate と volatility の2因子状態を1タイムステップ進化させる
  - Euler-Maruyama 離散化スキームを適用
  - volatility の非負制約を absorption scheme で維持
  - smooth_sqrt を使用して AD 互換性を確保
  - _Requirements: 3.2, 3.3, 5.1, 5.2_

- [x] 1.2 SABR の initial_state と brownian_dim 実装
  - SABRParams から TwoFactorState への初期状態生成
  - brownian_dim() = 2 を返す実装
  - model_name() = "SABR", num_factors() = 2 を実装
  - _Requirements: 3.1, 3.4_

- [x] 1.3 SABR の beta 特殊ケース処理
  - beta=0 で Normal SABR dynamics に切り替え
  - beta=1 で Lognormal SABR dynamics に切り替え
  - smooth_indicator による滑らかな切り替え
  - _Requirements: 3.5, 3.6, 5.1_

- [x] 1.4 StochasticModelEnum への SABR 追加
  - enum に SABR(SABRModel<T>) バリアント追加
  - `rates` feature flag でゲート
  - 既存の match パターンを SABR 対応に拡張
  - _Requirements: 1.2, 1.4, 1.5, 3.8_

---

- [x] 2. Hull-White 時間依存 θ(t) サポート

- [x] 2.1 ThetaFunction 型の実装
  - Constant(T) と FromCurve バリアントを持つ enum 定義
  - at(t) メソッドで時点 t の θ 値を取得
  - 定数 θ との後方互換性を維持
  - _Requirements: 4.5_

- [x] 2.2 イールドカーブからの θ(t) 導出
  - YieldCurve トレイトから θ(t) を計算するロジック
  - from_curve メソッドで離散時点の θ 値を事前計算
  - 補間によるスムーズな θ(t) 取得
  - _Requirements: 4.5, 5.1_

- [x] 2.3 HullWhiteParams への ThetaFunction 統合
  - 既存の定数 θ を ThetaFunction::Constant に移行
  - evolve_step で ThetaFunction.at(t) を使用
  - 既存テストの後方互換性確認
  - _Requirements: 4.3, 4.5_

---

- [x] 3. キャリブレーション基盤

- [x] 3.1 (P) CalibrationError 型の定義
  - NonConvergence, InvalidMarketData, ParameterBounds バリアント
  - thiserror による構造化エラーメッセージ
  - 診断情報（iterations, residual）を含む
  - _Requirements: 6.4_

- [x] 3.2 (P) CalibrationResult 型の定義
  - params, residual, iterations, converged フィールド
  - ジェネリック P でパラメータ型を抽象化
  - Debug, Clone 派生
  - _Requirements: 6.5_

---

- [x] 4. モデル別キャリブレーション実装

- [x] 4.1 (P) Heston キャリブレーション
  - HestonCalibrationData 構造体（options, spot, rate）
  - calibrate_heston 関数で Levenberg-Marquardt 呼び出し
  - Feller 条件をパラメータ境界として適用
  - 目的関数: model price vs market price の残差二乗和
  - _Requirements: 6.1, 6.5_

- [x] 4.2 (P) SABR キャリブレーション
  - SABRCalibrationData 構造体（forward, expiry, atm_vol, smile）
  - calibrate_sabr 関数で Hagan 公式のインバース
  - fixed_beta オプションによる beta 固定
  - ATM vol と smile へのフィッティング
  - _Requirements: 6.2, 6.5_

- [x] 4.3 (P) Hull-White キャリブレーション
  - HullWhiteCalibrationData 構造体（swaptions, yield_curve）
  - calibrate_hull_white 関数で swaption vol フィッティング
  - mean reversion と volatility の同時推定
  - _Requirements: 6.3, 6.5_

---

- [x] 5. AD 互換性と検証

- [x] 5.1 smooth approximation 適用確認
  - SABR evolve_step で smooth_sqrt, smooth_max 使用確認
  - Hull-White θ(t) 補間で smooth_interp 使用
  - 動的メモリ割当がないことの確認
  - _Requirements: 5.1, 5.2, 5.5_

- [x] 5.2 num-dual 勾配テスト
  - 各モデルの evolve_step を num-dual で実行
  - 有限差分との勾配比較（相対誤差 1e-6 以内）
  - 全パラメータに対する勾配検証
  - _Requirements: 5.3, 5.4, 7.4_

---

- [x] 6. 単体テストと検証

- [x] 6.1 (P) SABR モデルテスト
  - evolve_step のモーメントマッチング（期待値、分散）
  - Hagan 公式との implied vol 比較
  - beta=0, beta=1 の特殊ケーステスト
  - 境界値パラメータでのパニック回避確認
  - _Requirements: 7.2, 7.5_

- [x] 6.2 (P) Hull-White モデルテスト
  - 債券価格の解析解との比較
  - ThetaFunction からの θ(t) 導出テスト
  - 境界値パラメータでのパニック回避確認
  - _Requirements: 7.3, 7.5_

- [x] 6.3 (P) Heston モデルテスト拡張
  - モーメントマッチングテストの追加・拡張
  - Feller 条件違反時の挙動テスト
  - キャリブレーション収束テスト
  - _Requirements: 7.1, 7.5_

---

## Requirements Coverage Matrix

| Requirement | Tasks |
|-------------|-------|
| 1.2, 1.4, 1.5 | 1.4 |
| 2.1-2.6 | (既存実装でカバー) |
| 2.7 | (既存: equity feature) |
| 3.1, 3.2, 3.3, 3.4 | 1.1, 1.2 |
| 3.5, 3.6 | 1.3 |
| 3.7 | 1.1 (Float ジェネリック) |
| 3.8 | 1.4 |
| 4.1-4.4, 4.6, 4.7 | (既存実装でカバー) |
| 4.5 | 2.1, 2.2, 2.3 |
| 5.1, 5.2 | 1.1, 5.1 |
| 5.3, 5.4 | 5.2 |
| 5.5 | 5.1 |
| 6.1 | 4.1 |
| 6.2 | 4.2 |
| 6.3 | 4.3 |
| 6.4 | 3.1 |
| 6.5 | 3.2, 4.1, 4.2, 4.3 |
| 7.1 | 6.3 |
| 7.2 | 6.1 |
| 7.3 | 6.2 |
| 7.4 | 5.2 |
| 7.5 | 6.1, 6.2, 6.3 |

## Notes

- Requirement 1.1 (Heston StochasticModel) と 1.3 (Hull-White StochasticModel) は既存実装でカバー済み
- Requirement 2.1-2.6 (Heston パラメータ、状態、evolve_step) は既存実装でカバー済み
- Requirement 4.1-4.4, 4.6, 4.7 (Hull-White 基本実装) は既存実装でカバー済み
- 並列実行可能タスク (P) はキャリブレーション実装とテストに適用
