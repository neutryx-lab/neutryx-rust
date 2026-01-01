# Requirements Document

## Introduction

本仕様は、Neutryx XVAプライシングライブラリのパフォーマンス改善を定義する。Monte Carlo シミュレーション、Enzyme自動微分、メモリ効率、並列処理の各領域において、定量的な改善目標と測定可能な受け入れ基準を設定する。

## Requirements

### Requirement 1: Monte Carlo シミュレーション高速化

**Objective:** As a クオンツ開発者, I want Monte Carloシミュレーションの実行時間を短縮したい, so that 大規模ポートフォリオの価格計算をリアルタイムに近い速度で実行できる

#### 1.1 Acceptance Criteria

1. When 100万パスのMonte Carloシミュレーションを実行する場合, the pricer_kernel shall 現行実装比で2倍以上の速度で完了する
2. While バッチシミュレーションを実行中, the pricer_kernel shall メモリアロケーションをゼロに維持する
3. When パス生成を行う場合, the RNG shall Ziggurat法による正規乱数生成を使用し、Box-Muller法比で30%以上高速化する
4. The pricer_kernel shall SIMD命令（AVX2/AVX-512）を活用したベクトル化パス生成をサポートする
5. When ワークスペースバッファを再利用する場合, the PathWorkspace shall 初期化コストを最小化する

### Requirement 2: Enzyme自動微分最適化

**Objective:** As a リスク管理者, I want Greeks計算を高速化したい, so that リアルタイムリスクモニタリングが可能になる

#### 2.1 Acceptance Criteria

1. When Enzyme ADモードでGreeksを計算する場合, the pricer_kernel shall 有限差分法比で5倍以上高速に動作する
2. While forward-mode ADを使用中, the Enzyme bindings shall 中間変数のメモリ使用量を最小化する
3. When 複数のGreeks（Delta, Gamma, Vega, Theta）を同時計算する場合, the pricer_kernel shall バッチ処理により個別計算比で50%以上の効率化を達成する
4. The pricer_kernel shall `#[autodiff]`マクロを使用した宣言的AD定義をサポートする
5. If Enzyme計算でエラーが発生した場合, the pricer_kernel shall num-dualへの自動フォールバックを提供する

### Requirement 3: メモリ効率改善

**Objective:** As a システム管理者, I want メモリ使用量を削減したい, so that 限られたリソースで大規模計算を実行できる

#### 3.1 Acceptance Criteria

1. When パス依存オプションを評価する場合, the PathObserver shall ストリーミング統計を使用し、フルパス保存比で90%以上のメモリ削減を達成する
2. The pricer_kernel shall Structure of Arrays (SoA)レイアウトを使用し、キャッシュ効率を最適化する
3. When 大規模ポートフォリオ（1万取引以上）を処理する場合, the pricer_xva shall ピークメモリ使用量を取引数に対して線形以下に抑える
4. While チェックポイント機能を使用中, the checkpoint module shall 勾配計算に必要な最小限の状態のみを保存する
5. When バッファプールを使用する場合, the pricer_kernel shall スレッドローカルバッファによりアロケーション競合を回避する

### Requirement 4: 並列処理最適化

**Objective:** As a クオンツ開発者, I want マルチコアCPUを最大限活用したい, so that ハードウェアリソースに応じたスケーラビリティを実現できる

#### 4.1 Acceptance Criteria

1. When Rayonによる並列処理を使用する場合, the pricer_xva shall コア数に対して80%以上の並列効率を達成する
2. The pricer_xva shall ワークスチーリングによる動的負荷分散を実装する
3. When 異なるカウンターパーティのXVA計算を行う場合, the XvaCalculator shall 独立した計算を並列実行する
4. While 並列シミュレーションを実行中, the pricer_kernel shall スレッドセーフなRNG状態管理を保証する
5. When スレッド数を動的に変更する場合, the parallel module shall オーバーヘッドなくスケールする

### Requirement 5: コンパイラ最適化活用

**Objective:** As a ビルドエンジニア, I want コンパイル時最適化を最大化したい, so that 追加のコード変更なしにパフォーマンス向上を得られる

#### 5.1 Acceptance Criteria

1. The Cargo.toml shall リリースビルドでLTO（Link-Time Optimization）を有効化する
2. The Cargo.toml shall `codegen-units = 1`を設定し、最大限のインライン化を実現する
3. When ターゲットCPU固有の最適化を使用する場合, the build system shall `target-cpu=native`フラグをサポートする
4. The pricer_kernel shall 静的ディスパッチ（enum）を使用し、動的ディスパッチのオーバーヘッドを排除する
5. When プロファイルガイド最適化（PGO）を使用する場合, the build system shall ベンチマークデータに基づく最適化をサポートする

### Requirement 6: ベンチマーク・計測インフラ

**Objective:** As a 開発者, I want パフォーマンス計測を自動化したい, so that リグレッションを早期に検出できる

#### 6.1 Acceptance Criteria

1. The benchmark suite shall Criterionを使用した統計的に有意なパフォーマンス測定を提供する
2. When CIパイプラインを実行する場合, the CI shall 主要ベンチマークのパフォーマンスリグレッションを検出する
3. The benchmark suite shall Monte Carlo、Greeks計算、ポートフォリオ処理の各領域をカバーする
4. When ベンチマーク結果を記録する場合, the CI shall 履歴データとの比較グラフを生成する
5. If パフォーマンスが閾値を下回った場合, the CI shall ビルドを失敗させアラートを発行する
