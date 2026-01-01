# Requirements Document

## Introduction

本仕様書は、Neutryx XVA価格ライブラリのPhase 4として、メモリ効率的な自動微分（AD）のためのチェックポイント機構およびパス依存オプションの実装要件を定義する。

Phase 3で完成したモンテカルロカーネルとEnzyme AD基盤の上に構築し、長いシミュレーションパスでのメモリ使用量を最適化しながら、アジアンオプション、バリアオプション、ルックバックオプションなどのパス依存型デリバティブをサポートする。

## Requirements

### Requirement 1: チェックポイント基盤

**Objective:** As a 計算エンジン開発者, I want チェックポイント機構によりAD計算時のメモリ使用量を制御, so that 長いモンテカルロパスでもメモリ枯渇なしに勾配計算が可能になる

#### Acceptance Criteria 1

1. The CheckpointManager shall 設定可能な間隔でシミュレーション状態を保存する
2. When チェックポイント間の再計算が必要な場合, the CheckpointManager shall 保存されたチェックポイントから順方向シミュレーションを再実行する
3. The CheckpointManager shall メモリ使用量とチェックポイント間隔のトレードオフを設定可能にする
4. While ADモードが有効な場合, the MC Kernel shall チェックポイントを活用して逆方向パスのメモリ使用量を削減する
5. The CheckpointStrategy shall 均等間隔、対数間隔、適応的間隔の戦略を提供する

### Requirement 2: パス依存オプション基盤

**Objective:** As a クオンツ開発者, I want パス依存オプションの価格計算とギリシャ計算, so that アジアン、バリア、ルックバックオプションの評価が可能になる

#### Acceptance Criteria 2

1. The PathDependentPayoff trait shall パス全体の観測値に基づくペイオフ計算を定義する
2. When シミュレーションパスが生成された場合, the PathObserver shall 必要な経路情報（最大値、最小値、平均値）を蓄積する
3. The pricer_kernel shall AsianOption（算術平均、幾何平均）のペイオフ計算を提供する
4. The pricer_kernel shall BarrierOption（ノックイン、ノックアウト、アップ、ダウン）のペイオフ計算を提供する
5. The pricer_kernel shall LookbackOption（固定ストライク、フローティングストライク）のペイオフ計算を提供する
6. The PathDependentPayoff shall Enzyme ADと互換性のある滑らかな近似関数を使用する

### Requirement 3: メモリ管理とワークスペース拡張

**Objective:** As a システム開発者, I want パス依存オプション用のメモリ効率的なワークスペース管理, so that 大規模ポートフォリオでもメモリ使用量が予測可能になる

#### Acceptance Criteria 3

1. The PathWorkspace shall パス依存計算に必要な追加バッファを事前確保する
2. The PathWorkspace shall チェックポイント保存用のメモリ領域を管理する
3. When ワークスペースが再利用される場合, the PathWorkspace shall 前回のシミュレーションデータを安全にクリアする
4. The MemoryBudget shall 利用可能なメモリに基づいてチェックポイント間隔を自動調整する
5. While 複数のパスが並列処理される場合, the PathWorkspace shall スレッドローカルなバッファを提供する

### Requirement 4: L1/L2クレート統合

**Objective:** As a アーキテクト, I want L3（pricer_kernel）とL1/L2（pricer_core, pricer_models）の統合, so that 4層アーキテクチャの完全な依存関係フローが実現する

#### Acceptance Criteria 4

1. The pricer_kernel shall pricer_coreのFloat trait、smoothing関数、market_data構造を利用する
2. The pricer_kernel shall pricer_modelsのStochasticModel trait、Instrument enum、analytical moduleを利用する
3. When Instrument enumが価格計算に渡された場合, the MC Kernel shall 適切なペイオフ関数にディスパッチする
4. The pricer_kernel shall pricer_coreのYieldCurve traitを割引計算に使用する
5. If pricer_*依存関係でコンパイルエラーが発生した場合, the build system shall 明確なエラーメッセージを表示する

### Requirement 5: 検証とテスト

**Objective:** As a 品質保証担当者, I want チェックポイント機構とパス依存オプションの正確性検証, so that 本番環境での信頼性が担保される

#### Acceptance Criteria 5

1. The pricer_kernel shall チェックポイント有無で同一の勾配結果を保証するテストを含む
2. The pricer_kernel shall アジアンオプションの解析解との比較テストを含む
3. The pricer_kernel shall バリアオプションの解析解（Black-Scholes barrier formula）との比較テストを含む
4. When Enzyme ADモードが有効な場合, the verification tests shall num-dualモードとの結果比較を実行する
5. The benchmark suite shall チェックポイント間隔によるメモリ/計算時間トレードオフを測定する

### Requirement 6: パフォーマンス要件

**Objective:** As a パフォーマンスエンジニア, I want チェックポイント機構のオーバーヘッドを最小化, so that 実用的な計算時間内でAD計算が完了する

#### Acceptance Criteria 6

1. The CheckpointManager shall チェックポイント保存時のメモリコピーをO(state_size)に制限する
2. While チェックポイントなしのAD計算と比較した場合, the checkpointed AD shall 計算時間オーバーヘッドを2倍以内に抑える
3. The CheckpointStrategy shall ベンチマークデータに基づいて最適なチェックポイント間隔を推奨する
4. The pricer_kernel shall LTO（Link-Time Optimization）とcodegen-units=1の最適化設定で動作する
5. Where Rayon並列処理が有効な場合, the PathWorkspace shall スレッド間でのメモリ競合を回避する
