# Implementation Plan

## Overview

Enzyme LLVM-level AD の `#[autodiff]` マクロを pricer_pricing クレートに本格統合し、高性能 Greeks 計算を実現する。

**Total Tasks**: 7 Major Tasks, 20 Sub-tasks
**Requirements Coverage**: 8 Requirements (1-8), 39 Acceptance Criteria

---

## Tasks

- [x] 1. Nightly Feature 有効化とビルド環境整備

- [x] 1.1 (P) Enzyme feature flag とコンパイラ機能の有効化
  - pricer_pricing の lib.rs に autodiff 機能を条件付きで有効化する仕組みを追加
  - enzyme-ad feature が有効な場合のみ nightly 機能を使用、無効時は stable Rust でコンパイル可能を維持
  - rust-toolchain.toml で nightly-2025-01-15 ツールチェーンを指定
  - _Requirements: 1.1, 1.2, 1.3_

- [x] 1.2 (P) ビルドスクリプトでの LLVM 検出とエラーメッセージ
  - build.rs で LLVM 18 の存在を検出するロジックを追加
  - LLVM 未インストール時に明確なエラーメッセージを表示
  - Enzyme プラグインのパス検出と環境変数サポート
  - _Requirements: 1.4_

---

- [x] 2. Forward Mode AD 基盤実装

- [x] 2.1 Forward mode AD のラッパー関数実装
  - `#[autodiff_forward]` マクロを使用した primal 関数を作成
  - Activity annotation (Dual, Const) による微分対象の指定
  - 基本的な European option 価格計算関数への適用
  - enzyme-ad feature 無効時の有限差分フォールバック実装
  - _Requirements: 2.1, 2.2_

- [x] 2.2 ForwardAD 型と単一パラメータ微分 API
  - tangent 値を管理する ForwardAD<T> 構造体を実装
  - constant (微分対象外) と variable (微分対象) のコンストラクタ
  - spot 価格に対する tangent seed 設定時に Delta 値を出力
  - 単一変数微分の O(1) 追加コスト実装
  - _Requirements: 2.3, 2.4, 2.5_

---

- [x] 3. Reverse Mode AD 基盤実装

- [x] 3.1 Reverse mode AD のラッパー関数実装
  - `#[autodiff_reverse]` マクロを使用した adjoint 関数を作成
  - Activity annotation (Active, Duplicated, Const) による微分対象の指定
  - 出力の adjoint seed を 1.0 に設定して全勾配を取得
  - enzyme-ad feature 無効時の有限差分フォールバック実装
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 3.2 ReverseAD 型と全パラメータ勾配 API
  - adjoint 値を管理する ReverseAD<T> 構造体を実装
  - spot, rate, vol, time に対する勾配フィールド
  - 単一の reverse pass で全パラメータの勾配を計算
  - GreeksResult<T> への変換メソッド
  - _Requirements: 3.4, 3.5_

- [x] 3.3 GammaAD 型と二次微分（Nested AD）実装
  - Gamma (∂²V/∂S²) を計算する `GammaAD<T>` 構造体を実装
  - Forward-over-Reverse パターンで nested AD を試行
  - nested AD 未対応時は Delta への有限差分をフォールバック
  - compute_gamma() メソッドの実装
  - _Requirements: 4.2_

---

- [x] 4. Enzyme 対応関数設計パターン

- [x] 4.1 (P) Smooth payoff 関数群の実装
  - Enzyme 互換の smooth_payoff 関数を作成 (call, put, digital)
  - 不連続性を含まない smooth approximation パターンの適用
  - 条件分岐では smooth_indicator, smooth_max を使用
  - _Requirements: 6.1, 6.2, 6.3_

- [x] 4.2 (P) Enzyme 互換ループパターンの適用
  - 固定サイズ for ループのみを使用するパス生成
  - while ループの排除と fixed iteration への変換
  - Enzyme 非互換パターンのコンパイル時検出 (lint 警告)
  - _Requirements: 6.4, 6.5_

---

- [x] 5. Monte Carlo Pricer への Enzyme 統合

- [x] 5.1 GreeksEnzyme トレイトと基本統合
  - MonteCarloPricer に Enzyme AD を統合するトレイトを定義
  - price_with_greeks() 内部での Enzyme 呼び出しロジック
  - Delta, Gamma, Vega, Theta, Rho の計算サポート
  - GreeksResult<f64> での結果返却
  - _Requirements: 4.1, 4.2, 4.5_

- [x] 5.2 並列シミュレーションでの adjoint 集約
  - スレッドローカルな AdjointAccumulator 実装
  - rayon を使用した並列パス生成と adjoint 集約
  - reduce 操作によるスレッドセーフな勾配合計
  - _Requirements: 4.4_

- [x] 5.3 Path-dependent オプションとチェックポイント連携
  - Asian, Barrier オプション計算での AD 適用
  - チェックポインティングとの組み合わせによるメモリ効率化
  - 長いシミュレーションパスでの AD 安定性確保
  - _Requirements: 4.3_

- [x] 5.4 Enzyme 無効時のフォールバック実装
  - enzyme-ad feature 無効時の有限差分による Greeks 計算
  - GreeksMode enum による計算モード分岐
  - bump-and-revalue 既存実装との統合
  - _Requirements: 4.6_

---

- [x] 6. 検証テストとベンチマーク

- [x] 6.1 Enzyme vs num-dual 比較検証テスト
  - 同一入力での Enzyme AD と num-dual の勾配比較
  - 相対誤差 1e-6 以内の確認
  - 複数のオプションタイプでの検証
  - _Requirements: 5.1, 5.2_

- [x] 6.2 有限差分法との比較検証テスト
  - Enzyme AD と有限差分の結果比較
  - bump 幅による精度変化の検証
  - 閾値超過時の警告出力
  - _Requirements: 5.3, 5.5_

- [x] 6.3 Black-Scholes 解析解との比較テスト
  - Enzyme で計算した Greeks と解析解の比較
  - Delta, Gamma, Vega, Theta, Rho の精度検証
  - ATM, ITM, OTM での検証カバレッジ
  - _Requirements: 5.4_

- [ ] 6.4 (P) criterion ベンチマークの追加
  - Enzyme vs 有限差分の速度比較ベンチマーク
  - 10,000 パス MC シミュレーションの性能測定
  - 5 倍高速化目標の確認
  - _Requirements: 7.1, 7.2, 7.4_
  - _Note: Requires enzyme-ad feature enabled with LLVM 18_

- [ ] 6.5 (P) 並列スケーリング検証
  - 1, 2, 4, 8 コアでの Greeks 計算性能測定
  - 線形スケーリングの確認
  - C++ Enzyme 実装との性能比較
  - _Requirements: 7.3, 7.5_
  - _Note: Requires enzyme-ad feature enabled with LLVM 18_

---

- [x] 7. CI/CD 統合

- [x] 7.1 (P) Enzyme 専用 CI ジョブの追加
  - nightly + Enzyme ビルドを別ジョブとして設定
  - stable クレートビルドへの影響分離
  - Enzyme テスト結果の専用レポート出力
  - _Requirements: 8.1, 8.2, 8.4_
  - _Note: Already implemented in .github/workflows/ci.yml (nightly-kernel job)_

- [x] 7.2 (P) Docker 環境での Enzyme 再現性確保
  - Dockerfile.nightly に Enzyme 環境を構築
  - LLVM 18 + Enzyme プラグインの再現可能なセットアップ
  - Enzyme 環境利用不可時の graceful degradation
  - _Requirements: 8.3, 8.5_
  - _Note: Already implemented in docker/Dockerfile.nightly_

---

## Requirements Coverage Matrix

| Requirement | Tasks |
|-------------|-------|
| 1.1, 1.2, 1.3 | 1.1 |
| 1.4 | 1.2 |
| 2.1, 2.2 | 2.1 |
| 2.3, 2.4, 2.5 | 2.2 |
| 3.1, 3.2, 3.3 | 3.1 |
| 3.4, 3.5 | 3.2 |
| 4.2 (Gamma) | 3.3 |
| 4.1, 4.2, 4.5 | 5.1 |
| 4.3 | 5.3 |
| 4.4 | 5.2 |
| 4.6 | 5.4 |
| 5.1, 5.2 | 6.1 |
| 5.3, 5.5 | 6.2 |
| 5.4 | 6.3 |
| 6.1, 6.2, 6.3 | 4.1 |
| 6.4, 6.5 | 4.2 |
| 7.1, 7.2, 7.4 | 6.4 |
| 7.3, 7.5 | 6.5 |
| 8.1, 8.2, 8.4 | 7.1 |
| 8.3, 8.5 | 7.2 |
