# Requirements Document

## Introduction

本ドキュメントは、モンテカルロ・シミュレーションのメモリレイアウト最適化に関する要件を定義する。`pricer_pricing/src/mc/paths.rs` および `workspace.rs` において、現行の「パスごと（Simulation First）」レイアウトを「タイムステップごと（Time Step First）」レイアウトへ移行し、さらにストリーミング型処理を導入することで、キャッシュ効率とメモリ使用量を大幅に改善する。

## Project Description (Input)

モンテカルロ・シミュレーションのメモリレイアウトの非効率性の解消
pricer_pricing/src/mc/paths.rs および workspace.rs の設計において、パス生成と評価のメモリ効率に改善の余地があります。

無駄の所在: 現在の構造が「パスごと（Simulation First）」にデータを保持している場合、CPUキャッシュミスが頻発します。また、全タイムステップのパスをメモリ上に保持する設計は、大規模なシミュレーション（数百万パス）においてメモリ帯域を圧迫します。

改善案:

キャッシュ局所性の向上: データレイアウトを「タイムステップごと（Time Step First）」に変更し、[Step][Path] の順でアクセスするようにします。これにより、ある時点の全パス計算においてベクトル化（AVX-512等）が効きやすくなります。

ストリーミング処理: 全パスをメモリに保持せず、必要なタイムステップのデータのみを生成・消費・破棄する「ストリーミング型エンジン」へ移行することで、メモリ使用量を劇的に削減（O(Steps * Paths) → O(Paths)）し、より大規模な計算を可能にします。

## Requirements

### Requirement 1: Time Step First メモリレイアウト

**Objective:** As a クオンツ開発者, I want パスデータが `[Step][Path]` の順序でメモリ上に配置される, so that 同一タイムステップの全パスに対するアクセスがキャッシュ効率的になり、ベクトル化が容易になる。

#### Acceptance Criteria

1. The MC Engine shall パスデータを `[num_steps][num_paths]` の2次元配列として内部的に保持する。
2. When パス生成が実行される, the MC Engine shall タイムステップ `t` の全パス値を連続したメモリ領域に配置する。
3. The MC Engine shall 単一タイムステップの全パスアクセスにおいてキャッシュラインの再利用率を最大化する。
4. When 既存の `PathWorkspace` が使用される, the MC Engine shall 後方互換性のためのアダプターインターフェースを提供する。

### Requirement 2: ストリーミング型パス処理

**Objective:** As a リスク計算担当者, I want 全タイムステップのパスを同時にメモリ上に保持せずに計算を実行できる, so that 数百万パス規模のシミュレーションでもメモリ使用量を抑制できる。

#### Acceptance Criteria

1. The MC Engine shall ストリーミングモードにおいてメモリ使用量を `O(num_paths)` に抑制する（従来の `O(num_steps * num_paths)` から削減）。
2. When ストリーミングモードが有効な場合, the MC Engine shall 各タイムステップのパス値を生成・消費・破棄のサイクルで処理する。
3. The MC Engine shall ストリーミング処理と従来の一括処理を設定により切り替え可能とする。
4. While ストリーミング処理中, the MC Engine shall 現在のタイムステップと直前のタイムステップのパス値のみをメモリ上に保持する。

### Requirement 3: ベクトル化対応

**Objective:** As a パフォーマンスエンジニア, I want 同一タイムステップの全パス計算がSIMD命令で効率的に処理される, so that AVX-512等のベクトル演算による高速化が実現できる。

#### Acceptance Criteria

1. The MC Engine shall タイムステップごとのパス値配列を64バイト境界にアラインメントする。
2. When パス更新が実行される, the MC Engine shall 連続メモリアクセスパターンを維持し、SIMD最適化を阻害しない。
3. The MC Engine shall `T: Float` ジェネリクスを維持しつつ、`f64` 配列に対するベクトル化を可能とする。
4. The MC Engine shall パス更新ループにおいてデータ依存による分岐を排除する。

### Requirement 4: PathObserver との統合

**Objective:** As a エキゾチック商品開発者, I want ストリーミング型エンジンがパス依存型ペイオフ（Asian, Barrier, Lookback）と統合される, so that 既存のPathObserverパターンを活用しつつメモリ効率を改善できる。

#### Acceptance Criteria

1. When ストリーミングモードでパス依存型ペイオフを評価する場合, the MC Engine shall `PathObserver` トレイトを通じてストリーミング統計を累積する。
2. The MC Engine shall Asian オプションの算術平均計算をストリーミング処理と互換させる。
3. The MC Engine shall Barrier オプションのバリア監視をストリーミング処理と互換させる。
4. The MC Engine shall Lookback オプションの最小/最大値追跡をストリーミング処理と互換させる。

### Requirement 5: 設定とAPI

**Objective:** As a ライブラリ利用者, I want メモリレイアウトと処理モードを設定により選択できる, so that ユースケースに応じて最適な設定を選択できる。

#### Acceptance Criteria

1. The MC Engine shall `PathLayoutConfig` 構造体を提供し、レイアウトモード（`TimeStepFirst`, `PathFirst`）を設定可能とする。
2. The MC Engine shall `StreamingConfig` 構造体を提供し、ストリーミングモードの有効/無効を設定可能とする。
3. The MC Engine shall ビルダーパターンによる設定APIを提供する。
4. If 無効な設定の組み合わせが指定された場合, the MC Engine shall コンパイル時または実行時に明確なエラーを報告する。

### Requirement 6: 性能要件

**Objective:** As a プロダクション運用者, I want メモリレイアウト最適化により測定可能な性能改善が得られる, so that 大規模シミュレーションの実行可能性と効率が向上する。

#### Acceptance Criteria

1. The MC Engine shall 100万パス・100ステップのシミュレーションにおいて、ストリーミングモードでピークメモリ使用量を従来比90%以上削減する。
2. The MC Engine shall Time Step First レイアウトにおいて、同一タイムステップの全パスアクセス時のキャッシュミス率を従来比50%以上削減する。
3. The MC Engine shall ストリーミングモードにおいて、従来モードと比較してスループットの低下を10%以内に抑制する。
4. The MC Engine shall ベンチマークテスト（Criterion）により性能改善を検証可能とする。

### Requirement 7: 後方互換性

**Objective:** As a 既存コードのメンテナー, I want 既存のMCプライシングコードが変更なしで動作し続ける, so that 移行コストを最小化できる。

#### Acceptance Criteria

1. The MC Engine shall 既存の `MonteCarloPricer` APIを変更せずに維持する。
2. The MC Engine shall デフォルト設定で従来と同等の動作を保証する。
3. When 既存のテストスイートが実行された場合, the MC Engine shall 全テストをパスする。
4. The MC Engine shall 新規APIを追加拡張として導入し、既存APIを非推奨としない。
