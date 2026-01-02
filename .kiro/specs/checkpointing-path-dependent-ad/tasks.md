# Implementation Plan

## Task Overview

本実装計画は、Phase 4（チェックポイントとパス依存オプション）を4つのサブフェーズに分割して段階的に実装する。

- **Phase 4.1**: L1/L2クレート統合（基盤整備）
- **Phase 4.2**: パス依存オプション基盤
- **Phase 4.3**: チェックポイント機構
- **Phase 4.4**: 検証・ベンチマーク

---

## Phase 4.1: L1/L2クレート統合

- [x] 1. L1/L2依存関係の追加と既存コードの統合

- [x] 1.1 pricer_kernel Cargo.tomlへの依存関係追加
  - pricer_coreとpricer_modelsをオプション依存として追加
  - featureフラグでの切り替え可能な設計（enzyme-ad feature時も動作）
  - nightly Rustでの互換性確認
  - _Requirements: 4.1, 4.2, 4.5_

- [x] 1.2 (P) smoothing関数のpricer_core統合
  - ローカルのsoft_plus関数をpricer_core::math::smoothing::smooth_maxで置換
  - 既存のpayoff.rs内のsmooth payoff関数を更新
  - Float trait制約の追加とジェネリック化
  - 単体テストで既存動作との等価性を確認
  - _Requirements: 4.1_

- [x] 1.3 (P) YieldCurve traitの統合
  - 割引計算でpricer_core::market_data::curves::YieldCurveを使用
  - FlatCurve/InterpolatedCurveのimport追加
  - MCエンジンでの割引適用ポイントの特定と修正
  - price_european_with_curve, price_with_greeks_and_curveメソッド追加
  - _Requirements: 4.4_

- [x] 1.4 StochasticModel/Instrument enumの統合検証
  - pricer_models::models::stochastic::StochasticModelのimport確認
  - Instrument enumからペイオフ関数へのディスパッチ検討
  - 静的ディスパッチパターンの維持確認（instrument_tests module追加）
  - 全クレートのコンパイル成功を検証
  - _Requirements: 4.2, 4.3_

---

## Phase 4.2: パス依存オプション基盤

- [x] 2. 経路観測と統計蓄積の実装

- [x] 2.1 PathObserverの実装
  - ストリーミング方式での経路統計蓄積（算術平均、幾何平均、最大値、最小値）
  - 各ステップでの価格観測とリセット機能
  - チェックポイント用の状態スナップショット機能（snapshot/restore）
  - Float型ジェネリックによるAD互換性
  - 単体テストで統計計算の精度検証（15テストパス）
  - _Requirements: 2.2_

- [x] 2.2 PathDependentPayoff traitの定義
  - パス全体からのペイオフ計算インターフェース
  - 必要な観測タイプを返すメソッド（ObservationType struct）
  - スムージングイプシロンのアクセサ
  - Send + Sync制約によるスレッド安全性
  - _Requirements: 2.1, 2.6_

- [x] 3. アジアンオプションの拡張

- [x] 3.1 (P) 幾何平均アジアンオプションの実装
  - 対数和からの幾何平均計算（PathObserverで実装済み）
  - smooth_maxによるスムーズペイオフ（soft_plus関数）
  - Call/Put両方のペイオフ関数（AsianGeometricPayoff）
  - 算術平均アジアン（AsianArithmeticPayoff）との統一インターフェース
  - _Requirements: 2.3, 2.6_

- [x] 4. バリアオプションの実装

- [x] 4.1 (P) Up-and-In/Up-and-Outバリアオプション
  - 経路最大値とバリアレベルの比較
  - smooth_indicatorによるバリア判定のスムージング
  - Call/Put両方のペイオフ計算
  - バリア条件とペイオフの積による最終価値
  - _Requirements: 2.4, 2.6_

- [x] 4.2 (P) Down-and-In/Down-and-Outバリアオプション
  - 経路最小値とバリアレベルの比較
  - スムーズバリア判定の実装
  - 4種類のバリアタイプの完全実装
  - _Requirements: 2.4, 2.6_

- [x] 5. ルックバックオプションの実装

- [x] 5.1 (P) 固定ストライクルックバックオプション
  - 経路最大値/最小値を使用したペイオフ計算
  - Fixed Strike Call: max(S_max - K, 0)
  - Fixed Strike Put: max(K - S_min, 0)
  - smooth_maxによるスムーズ化
  - _Requirements: 2.5, 2.6_

- [x] 5.2 (P) フローティングストライクルックバックオプション
  - 終端価格と経路極値の比較
  - Floating Strike Call: max(S_T - S_min, 0)
  - Floating Strike Put: max(S_max - S_T, 0)
  - _Requirements: 2.5, 2.6_

- [x] 6. パス依存ペイオフの統合

- [x] 6.1 PathPayoffType enumの実装
  - 全パス依存オプションタイプの列挙
  - 静的ディスパッチによるcompute関数
  - Enzyme AD互換の維持
  - _Requirements: 2.1, 2.6_

- [x] 6.2 MCエンジンへのパス依存ペイオフ統合
  - PathObserverをシミュレーションループに組み込み
  - 各ステップでのobserve呼び出し
  - ペイオフ計算でのPathPayoffType使用
  - 既存のEuropean payoffとの互換性維持
  - _Requirements: 2.1, 2.2_

---

## Phase 4.3: チェックポイント機構

- [x] 7. チェックポイント戦略と状態管理

- [x] 7.1 CheckpointStrategy enumの実装
  - 均等間隔戦略（デフォルト）
  - 対数間隔戦略（Griewank理論ベース）
  - 適応的戦略（メモリ圧力応答）
  - チェックポイントなし戦略
  - should_checkpoint判定ロジック
  - _Requirements: 1.5_

- [x] 7.2 SimulationStateとCheckpointStorageの実装
  - シミュレーション状態の定義（ステップ、RNG状態、観測統計、価格値）
  - 状態のClone/Copy効率化
  - ストレージへの保存/復元機能
  - 最大チェックポイント数の制限
  - _Requirements: 1.1, 1.2_

- [x] 8. チェックポイントマネージャーの実装

- [x] 8.1 CheckpointManagerコア機能
  - 戦略に基づくチェックポイント判定
  - 状態の保存と復元
  - 指定ステップに最も近いチェックポイントの検索
  - 全チェックポイントのクリア機能
  - _Requirements: 1.1, 1.2, 1.3_

- [x] 8.2 MemoryBudgetの実装
  - メモリ使用量の予算設定
  - 推奨チェックポイント間隔の計算
  - 予算内かどうかの判定
  - 警告閾値の設定
  - _Requirements: 3.4_

- [x] 8.3 CheckpointManagerへのMemoryBudget統合
  - メモリ予算に基づく自動チェックポイント間隔調整
  - 予算超過時の警告/エラー
  - _Requirements: 1.3, 3.4_

- [x] 9. ワークスペース拡張

- [x] 9.1 PathWorkspaceのチェックポイント対応拡張
  - チェックポイント保存用のメモリ領域参照
  - パス依存計算用の追加バッファ確保
  - 再利用時の安全なクリア機能
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 9.2 スレッドローカルバッファの設計
  - Rayon並列処理用のスレッドローカルワークスペース
  - スレッド間でのメモリ競合回避
  - thread_local!マクロまたはRayon統合パターンの選択
  - _Requirements: 3.5, 6.5_

- [x] 10. チェックポイントのAD統合

- [x] 10.1 MCエンジンへのチェックポイント統合
  - forward pass中のチェックポイント保存
  - reverse pass時のチェックポイントからの再計算
  - Enzyme ADとの連携確認
  - チェックポイント有無での勾配等価性検証
  - _Requirements: 1.4, 5.1_

---

## Phase 4.4: 検証・ベンチマーク

- [x] 11. 解析解の実装

- [x] 11.1 (P) 幾何平均アジアンオプション解析解
  - Black-Scholesベースの閉形式解実装
  - 調整ボラティリティ（σ_G = σ/√3）の適用
  - YieldCurve traitを使用した割引計算
  - _Requirements: 5.2_

- [x] 11.2 (P) バリアオプション解析解
  - Rubinstein-Reiner公式の実装
  - 8種類のバリアタイプ（Up/Down × In/Out × Call/Put）
  - 連続監視バリアの解析価格
  - _Requirements: 5.3_

- [x] 12. 検証テストの実装

- [x] 12.1 チェックポイント等価性テスト
  - チェックポイント有無での価格結果比較
  - チェックポイント有無での勾配結果比較
  - 異なるチェックポイント間隔での結果一致確認
  - _Requirements: 5.1_

- [x] 12.2 解析解比較テスト
  - 幾何平均アジアンのMC価格 vs 解析解
  - バリアオプションのMC価格 vs 解析解（連続監視）
  - 収束テスト（パス数増加による誤差減少確認）
  - _Requirements: 5.2, 5.3_

- [x] 12.3 Enzyme vs num-dual検証テスト
  - パス依存オプションでの両ADモード比較
  - チェックポイント使用時の勾配比較
  - 許容誤差範囲内での一致確認
  - _Requirements: 5.4_

- [x] 13. パフォーマンスベンチマーク

- [x] 13.1 チェックポイントオーバーヘッド測定
  - チェックポイント間隔によるメモリ使用量測定
  - チェックポイント間隔による計算時間測定
  - メモリ/計算時間トレードオフの可視化
  - 2倍以内のオーバーヘッド目標確認
  - _Requirements: 5.5, 6.1, 6.2_

- [x] 13.2 スケーリングベンチマーク
  - n_paths, n_stepsの増加に対する性能特性
  - パス依存オプション種類別の計算コスト比較
  - criterion benchmarkの追加
  - _Requirements: 6.3_

- [x] 13.3 (P) Rayon並列化の統合と検証
  - PathWorkspaceのスレッドローカル化完了確認
  - 並列処理時のメモリ競合テスト
  - 並列スケーラビリティの測定
  - _Requirements: 6.5_

- [x] 14. 最終統合と最適化設定

- [x] 14.1 LTO/codegen-units設定の検証
  - release profileでのLTO有効化確認
  - codegen-units=1設定の確認
  - 最適化ビルドでの全テスト実行
  - _Requirements: 6.4_

- [x] 14.2 E2Eパス依存オプション価格計算テスト
  - 全オプションタイプでのMC価格計算
  - チェックポイント使用時のギリシャ計算
  - L1/L2統合状態での完全動作確認
  - _Requirements: 1.4, 2.1, 4.3_

---

## Requirements Coverage Matrix

| Requirement | Tasks |
|-------------|-------|
| 1.1 | 7.2, 8.1 |
| 1.2 | 7.2, 8.1 |
| 1.3 | 8.1, 8.3 |
| 1.4 | 10.1, 14.2 |
| 1.5 | 7.1 |
| 2.1 | 2.2, 6.1, 6.2, 14.2 |
| 2.2 | 2.1, 6.2 |
| 2.3 | 3.1 |
| 2.4 | 4.1, 4.2 |
| 2.5 | 5.1, 5.2 |
| 2.6 | 2.2, 3.1, 4.1, 4.2, 5.1, 5.2, 6.1 |
| 3.1 | 9.1 |
| 3.2 | 9.1 |
| 3.3 | 9.1 |
| 3.4 | 8.2, 8.3 |
| 3.5 | 9.2 |
| 4.1 | 1.1, 1.2 |
| 4.2 | 1.1, 1.4 |
| 4.3 | 1.4, 14.2 |
| 4.4 | 1.3 |
| 4.5 | 1.1 |
| 5.1 | 10.1, 12.1 |
| 5.2 | 11.1, 12.2 |
| 5.3 | 11.2, 12.2 |
| 5.4 | 12.3 |
| 5.5 | 13.1 |
| 6.1 | 13.1 |
| 6.2 | 13.1 |
| 6.3 | 13.2 |
| 6.4 | 14.1 |
| 6.5 | 9.2, 13.3 |

---

_生成日: 2026-01-01_
