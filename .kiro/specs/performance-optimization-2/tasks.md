# Implementation Plan: performance-optimization-2

## Phase 1: ベンチマーク・計測インフラ & ビルド設定

- [x] 1. ベンチマークインフラ強化
- [x] 1.1 (P) Iaiベンチマーク追加
  - 命令数ベースのベンチマークをpricer_kernelに追加
  - GBMパス生成、Greeks計算、ワークスペース操作をカバー
  - Criterionとの並行運用を可能にする構成
  - CI環境での再現性を重視した設計
  - _Requirements: 6.1, 6.2, 6.3_

- [x] 1.2 (P) CIリグレッション検出機構
  - GitHub ActionsワークフローにIaiベンチマーク統合
  - ベースラインとの比較スクリプト作成
  - 閾値超過時のビルド失敗とアラート設定
  - 履歴データ保存とグラフ生成
  - _Requirements: 6.2, 6.4, 6.5_

- [ ] 2. ビルド最適化設定
- [x] 2.1 (P) target-cpu=native設定
  - .cargo/config.tomlにx86_64向けRUSTFLAGS設定
  - 既存のLTO/codegen-units設定との整合性確認
  - クロスコンパイル時の考慮事項をドキュメント化
  - _Requirements: 5.1, 5.2, 5.3_

- [x] 2.2 (P) PGOビルドインフラ
  - cargo-pgoを使用したビルドスクリプト作成
  - プロファイル収集用ベンチマークワークロード定義
  - PGO最適化ビルド手順のドキュメント化
  - _Requirements: 5.5_

## Phase 2: メモリ効率 & 並列処理

- [ ] 3. スレッドローカルバッファプール
- [x] 3.1 pool moduleの基本実装
  - ThreadLocalPoolとPooledBuffer構造体の実装
  - thread_local!マクロを使用したスレッドごとのストレージ
  - RAII Drop実装による自動バッファ返却
  - プール統計情報の収集機能
  - _Requirements: 1.2, 3.3, 3.5_

- [x] 3.2 PathWorkspaceへのpool統合
  - 既存のPathWorkspaceをpool moduleと統合
  - バッファ取得をプールから行うよう変更
  - ゼロアロケーション動作の検証テスト
  - 初期化コスト最小化のためのreset_fast実装
  - _Requirements: 1.2, 1.5_

- [ ] 4. チェックポイント最小化
- [x] 4.1 MinimalState実装
  - 勾配計算に必要な最小限の状態定義
  - RNG状態とPathObserverスナップショットの保存
  - メモリ使用量O(√n)を目指すBinomial戦略
  - save/restore操作の整合性テスト
  - _Requirements: 3.4_

- [ ] 5. 並列処理最適化
- [ ] 5.1 (P) ThreadRngProvider実装
  - スレッドインデックスベースの決定的シード生成
  - PricerRngのスレッドセーフな提供
  - 並列シミュレーションでの再現性保証
  - _Requirements: 4.4_

- [ ] 5.2 (P) ParallelConfig拡張
  - 動的スレッド数調整機能の追加
  - バッチサイズと並列閾値の最適化
  - Rayon設定との統合
  - _Requirements: 4.5_

- [ ] 5.3 XvaCalculator並列化強化
  - カウンターパーティ単位の並列XVA計算
  - ThreadRngProviderを使用したRNG管理
  - 80%並列効率達成の検証ベンチマーク
  - _Requirements: 4.1, 4.2, 4.3_

## Phase 3: Monte Carlo高速化 & Enzyme AD

- [ ] 6. SIMDモジュール実装
- [ ] 6.1 simd module基本構造
  - SimdOps traitとSimdLevel/SimdError型定義
  - ランタイムSIMD能力検出
  - AVX2/AVX-512の条件付きコンパイル設定
  - スカラーフォールバック実装
  - _Requirements: 1.4_

- [ ] 6.2 SIMD乱数生成
  - fill_normal_simd関数の実装
  - f64x4（AVX2）バッチ処理
  - Ziggurat法との統合
  - スカラー実装との数値一致テスト
  - _Requirements: 1.3, 1.4_

- [ ] 6.3 SIMDパス生成
  - generate_gbm_paths_simd関数の実装
  - ベクトル化されたGBMステップ計算
  - PathWorkspace/poolとの統合
  - 2倍速度向上の検証ベンチマーク
  - _Requirements: 1.1, 1.4_

- [ ] 7. Enzyme AD統合強化
- [ ] 7.1 #[autodiff]マクロ基盤
  - #![feature(autodiff)]の有効化
  - autodiff_forward/autodiff_reverseマクロの使用パターン確立
  - 基本的な価格関数への適用
  - -Zautodiff=Enableフラグ設定
  - _Requirements: 2.4_

- [ ] 7.2 EnzymeResult型とフォールバック
  - AdError enum（EnzymeNotAvailable, DifferentiationFailed等）
  - AdMode enum（Enzyme, NumDual, Auto）
  - enzyme_available()ランタイム検出
  - num-dualへの自動フォールバック実装
  - _Requirements: 2.5_

- [ ] 7.3 GreeksResult統合
  - price_with_greeks関数の実装
  - Delta, Gamma, Vega, Theta, Rhoの計算
  - Enzyme vs num-dual結果比較テスト
  - 有限差分比5倍高速化の検証
  - _Requirements: 2.1, 2.2_

- [ ] 7.4 バッチGreeks計算
  - batch_greeks関数の実装
  - 複数パラメータセットの効率的処理
  - Rayon並列化との統合
  - 個別計算比50%効率化の検証
  - _Requirements: 2.3_

## Phase 4: 統合 & 検証

- [ ] 8. 統合テスト & パフォーマンス検証
- [ ] 8.1 エンドツーエンド統合テスト
  - SIMD → pool → MonteCarloPricer フロー検証
  - Enzyme → フォールバック → Greeks計算フロー検証
  - 並列XVA計算の結果整合性テスト
  - 全コンポーネント連携の動作確認
  - _Requirements: 1.1, 2.1, 4.1_

- [ ] 8.2 パフォーマンス目標達成確認
  - Monte Carlo 2倍速度向上の最終測定
  - Enzyme 5倍高速化の最終測定
  - 80%並列効率の最終測定
  - メモリ90%削減の最終測定
  - _Requirements: 1.1, 2.1, 3.1, 4.1_

- [ ] 8.3 CIパイプライン最終検証
  - Iaiベンチマークのリグレッション検出動作確認
  - 閾値アラートのトリガー検証
  - 履歴比較グラフ生成確認
  - 全プラットフォーム（Linux/macOS）でのEnzyme動作確認
  - _Requirements: 6.2, 6.4, 6.5_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1 | 6.3, 8.1, 8.2 |
| 1.2 | 3.1, 3.2 |
| 1.3 | 6.2 |
| 1.4 | 6.1, 6.2, 6.3 |
| 1.5 | 3.2 |
| 2.1 | 7.3, 8.1, 8.2 |
| 2.2 | 7.3 |
| 2.3 | 7.4 |
| 2.4 | 7.1 |
| 2.5 | 7.2 |
| 3.1 | 8.2 (既存PathObserver) |
| 3.2 | (既存TradeSoA) |
| 3.3 | 3.1 |
| 3.4 | 4.1 |
| 3.5 | 3.1 |
| 4.1 | 5.3, 8.1, 8.2 |
| 4.2 | 5.3 (Rayon内蔵) |
| 4.3 | 5.3 |
| 4.4 | 5.1 |
| 4.5 | 5.2 |
| 5.1 | 2.1 (既存) |
| 5.2 | 2.1 (既存) |
| 5.3 | 2.1 |
| 5.4 | (既存enum設計) |
| 5.5 | 2.2 |
| 6.1 | 1.1 (既存Criterion) |
| 6.2 | 1.1, 1.2, 8.3 |
| 6.3 | 1.1 |
| 6.4 | 1.2, 8.3 |
| 6.5 | 1.2, 8.3 |

**Notes**:
- 3.1（ストリーミング統計）: 既存PathObserverで対応済み
- 3.2（SoAレイアウト）: 既存TradeSoAで対応済み
- 5.4（静的ディスパッチ）: 既存enum設計で対応済み
