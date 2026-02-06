# Implementation Plan: enzyme-ift-bootstrap

## Phase 1: 疎行列基盤構築

- [ ] 1. 疎行列モジュールのセットアップと基本型定義

- [ ] 1.1 (P) 疎行列ライブラリ依存関係と feature flag の追加
  - pricer_core に nalgebra-sparse 依存関係を追加
  - `sparse` feature flag を定義し、オプション依存として設定
  - 既存の linalg モジュールとの feature 連携を確認
  - _Requirements: 4.1, 6.1_

- [ ] 1.2 疎行列型エイリアスと変換ユーティリティの実装
  - CSR/CSC 疎行列型のエイリアス定義
  - 密行列から疎行列への変換関数（threshold パラメータ付き）
  - 疎行列から密行列への逆変換関数
  - スパース率計算ユーティリティの実装
  - _Requirements: 4.1_

- [ ] 1.3 SparseLUStrategy の LinearSolveStrategy trait 実装
  - 既存 LUStrategy と同一インターフェースでの疎行列 LU 分解
  - CSR フォーマット行列の分解とキャッシュ機構
  - スパース率閾値による自動選択ロジック（70% 以上で効果的）
  - decompose, solve, inverse メソッドの実装
  - _Requirements: 4.1, 4.3, 4.4_

- [ ] 1.4 疎行列演算の単体テスト
  - CSR 変換の正確性検証
  - SparseLUStrategy と LUStrategy の数値的等価性テスト
  - スパース率計算の境界値テスト
  - _Requirements: 4.1, 4.4_

## Phase 2: Enzyme AD Jacobian 統合

- [ ] 2. Enzyme AD による Jacobian 計算機能

- [ ] 2.1 残差関数の微分可能カーネル実装
  - autodiff マクロによる残差計算カーネルの定義
  - reverse-mode 微分のためのカーネルシグネチャ設計
  - enzyme-ad feature gate による条件付きコンパイル
  - _Requirements: 1.1, 1.3_

- [ ] 2.2 Enzyme Jacobian 計算とフォールバック機構
  - 残差カーネルを用いた完全 Jacobian 行列の計算
  - 計算失敗時の有限差分法への自動フォールバック
  - フォールバック発生時の警告ログ出力
  - 計算結果メタデータ（使用メソッド、計算時間、フォールバック有無）の記録
  - _Requirements: 1.1, 1.4_

- [ ] 2.3 CalibrationProblem への Enzyme Jacobian 統合
  - JacobianMethod::AutomaticDifferentiation の完全実装
  - enzyme-ad 有効時のデフォルト Jacobian メソッド設定
  - 既存の有限差分・中央差分メソッドとの統合
  - _Requirements: 1.1, 1.3_

- [ ] 2.4 (P) 補間レイヤーの微分可能実装
  - Flat, Linear, LogLinear 補間の autodiff 互換インターフェース
  - discount_factor_with_gradient メソッドの追加
  - LogLinear 補間の解析的微分（∂DF(t)/∂DF_i）実装
  - AD dual numbers の補間レイヤー伝播確認
  - _Requirements: 2.1, 2.2, 2.3, 2.4_

- [ ] 2.5 Enzyme Jacobian の単体・統合テスト
  - Enzyme 計算と解析微分の 1e-12 相対誤差以内の検証
  - フォールバック機構の動作確認テスト
  - 各補間メソッドでの Jacobian 計算正確性テスト
  - _Requirements: 1.2, 1.4, 1.5_

## Phase 3: IFT 感度抽出と AAD Binder 統合

- [ ] 3. IFT による市場感度抽出機能

- [ ] 3.1 GlobalBootstrapResult への IFT 感度メソッド追加
  - ift_sensitivity メソッドの実装（∂x*/∂m = -J⁻¹ · ∂F/∂m）
  - J⁻¹ 未キャッシュ時の明確なエラー返却
  - can_compute_ift ヘルパーメソッドの追加
  - _Requirements: 3.1, 3.2, 3.4_

- [ ] 3.2 バッチ市場パラメータ感度計算
  - ift_sensitivity_batch メソッドの実装
  - 複数パラメータを単一行列-行列演算で処理
  - 次元不整合エラーのハンドリング
  - _Requirements: 3.3_

- [ ] 3.3 IftError エラー型の定義
  - NoJacobianInverse エラー（再キャリブレーション指示付き）
  - DimensionMismatch エラー（期待値と実際値の報告）
  - thiserror による構造化エラー実装
  - _Requirements: 3.4_

- [ ] 3.4 (P) GlobalBootstrapResult の Shadow trait 実装
  - AAD 勾配蓄積のための zero_out メソッド実装
  - active input（discount_factors）とconst（jacobian_inverse）の区別
  - 既存 Shadow 実装パターンとの整合性確保
  - _Requirements: 7.3_

- [ ] 3.5 MarketRiskCalculator への GlobalBootstrapResult 統合
  - GlobalBootstrapResult を入力として受け付ける機能追加
  - キャッシュ済み J⁻¹ を用いた感度計算
  - 同一カーブを共有するトレード間のバッチ処理
  - J⁻¹ 未キャッシュ時のオンデマンド再キャリブレーション
  - _Requirements: 7.1, 7.2, 7.4, 7.5_

- [ ] 3.6 IFT 感度の単体・統合テスト
  - IFT 感度と bump-and-recalibrate の 1e-8 相対誤差以内の検証
  - バッチ処理の正確性テスト
  - Shadow zero_out の動作確認
  - _Requirements: 3.5_

## Phase 4: 数値安定性機能

- [ ] 4. Jacobian 品質検証と数値安定性保証

- [ ] 4.1 Jacobian 品質検証メソッドの実装
  - validate_jacobian_quality メソッドの追加
  - NaN、Inf、近ゼロ対角要素の検出
  - JacobianQuality 列挙型（Good/Warning/Poor）による結果表現
  - _Requirements: 5.3_

- [ ] 4.2 NumericalDiagnostics 構造体の追加
  - 条件数（推定値）フィールド
  - 残差ノルム履歴フィールド
  - 適用された正則化タイプ（None/Tikhonov/LM）フィールド
  - Jacobian 品質検証結果フィールド
  - _Requirements: 5.5_

- [ ] 4.3 条件数モニタリングと Tikhonov 正則化
  - Jacobian 条件数の推定計算
  - max_condition_number 閾値超過時の警告出力
  - 条件数 1e10 超過時の Tikhonov 正則化自動適用
  - 正則化パラメータの設定可能化
  - _Requirements: 5.1, 5.2_

- [ ] 4.4 AD 不安定時の自動フォールバック
  - Enzyme AD 勾配と有限差分の分散比較
  - 分散 1e6 超過時の中央差分への自動切替
  - フォールバック発生の NumericalDiagnostics への記録
  - _Requirements: 5.4_

- [ ] 4.5 数値安定性の単体テスト
  - 条件数推定の正確性テスト
  - 正則化適用の動作確認テスト
  - 品質検証の境界値テスト（NaN、Inf 検出）
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

## Phase 5: Feature Flag 統合

- [ ] 5. 設定とコンパイル時検証

- [ ] 5.1 Feature Flag の Cargo.toml 設定
  - enzyme-ad feature の定義と依存関係設定
  - sparse feature の定義と nalgebra-sparse 連携
  - feature 間の依存関係（enzyme-ad → nightly toolchain）の文書化
  - _Requirements: 6.1_

- [ ] 5.2 GlobalBootstrapConfig の条件付き設定公開
  - enzyme-ad 無効時の Enzyme 関連設定非公開
  - with_jacobian_method ビルダーメソッドの feature 互換性検証
  - ad_checkpoint_interval パラメータの追加
  - _Requirements: 6.2, 6.3, 6.5_

- [ ] 5.3 コンパイル時互換性エラー
  - enzyme-ad 無効時の AutomaticDifferentiation 選択でコンパイルエラー
  - 非対応補間メソッドと AD 組み合わせのコンパイル時検証
  - _Requirements: 6.4, 2.5_

## Phase 6: 統合検証

- [ ] 6. エンドツーエンド統合とパフォーマンス検証

- [ ] 6.1 エンドツーエンド統合テスト
  - 疎行列 Strategy と Enzyme AD の組み合わせテスト
  - IFT 感度から AAD Binder までのフルパイプラインテスト
  - 数値安定性機能の統合動作確認
  - _Requirements: 1.1, 3.1, 4.1, 7.1_

- [ ] 6.2 パフォーマンスベンチマーク
  - 疎行列 Strategy vs 密行列のスピード比較（大規模問題）
  - Enzyme AD vs 有限差分のスピードアップ測定
  - IFT バッチ処理のスケーラビリティ検証
  - _Requirements: 1.2, 4.3_

---

## Requirements Coverage Summary

| Requirement | Tasks |
|-------------|-------|
| 1.1-1.5 | 2.1, 2.2, 2.3, 2.4, 2.5 |
| 2.1-2.5 | 2.4, 5.3 |
| 3.1-3.5 | 3.1, 3.2, 3.3, 3.6 |
| 4.1-4.5 | 1.1, 1.2, 1.3, 1.4 |
| 5.1-5.5 | 4.1, 4.2, 4.3, 4.4, 4.5 |
| 6.1-6.5 | 1.1, 5.1, 5.2, 5.3 |
| 7.1-7.5 | 3.4, 3.5 |

**Note**: Requirement 4.5 (GMRES 反復解法) is explicitly deferred to Phase 2 per design.md Non-Goals.
