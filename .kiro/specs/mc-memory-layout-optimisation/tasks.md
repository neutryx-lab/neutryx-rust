# Implementation Plan: MC Memory Layout Optimisation

## Task Summary

| Category | Count |
|----------|-------|
| Major Tasks | 7 |
| Sub-tasks | 21 |
| Requirements Covered | 7 (1-7) |

---

## Tasks

### Phase 1: 基盤コンポーネント

- [x] 1. Config 構造体の実装
- [x] 1.1 (P) レイアウトモード設定の実装
  - PathLayout enum（PathFirst / TimeStepFirst）を定義
  - PathLayoutConfig 構造体を作成し、レイアウトとアラインメント（デフォルト64バイト）を設定可能にする
  - Default トレイトを実装し、PathFirst をデフォルトとする
  - Clone, Copy, Debug, PartialEq, Eq を derive
  - _Requirements: 5.1_

- [x] 1.2 (P) ストリーミング設定の実装
  - StreamingConfig 構造体を作成し、有効/無効と buffer_steps を設定可能にする
  - Default トレイトを実装し、enabled = false, buffer_steps = 2 をデフォルトとする
  - buffer_steps >= 2 のバリデーションを追加
  - _Requirements: 5.2, 5.4_

- [x] 1.3 設定エラー型の定義
  - LayoutConfigError を thiserror で定義
  - StreamingRequiresTimeStepFirst, InvalidAlignment, InvalidBufferSteps バリアントを追加
  - 既存の MonteCarloConfigError と統合
  - _Requirements: 5.4_

---

- [x] 2. アラインドメモリバッファの実装
- [x] 2.1 AlignedPathBuffer 構造体の実装
  - Feature flag `simd-aligned` で aligned-vec 依存を制御
  - simd-aligned 有効時は AVec<T, ConstAlign<64>> を内部使用
  - simd-aligned 無効時は通常の Vec<T> にフォールバック
  - new(), with_alignment(), as_slice(), as_mut_slice(), alignment() メソッドを提供
  - アラインメントが2のべき乗かを検証
  - _Requirements: 3.1, 3.2_

- [x] 2.2 アラインメント検証テストの作成
  - simd-aligned 有効時に64バイト境界アラインメントを検証
  - フォールバック時の動作確認
  - 大容量バッファ（100万要素）での確保テスト
  - _Requirements: 3.1, 3.3_

---

### Phase 2: Workspace 抽象化

- [x] 3. PathWorkspaceTrait と WorkspaceEnum の実装
- [x] 3.1 PathWorkspaceTrait トレイトの定義
  - num_paths(), num_steps(), layout() メソッドを定義
  - get_path_value(), set_path_value() で個別アクセスを提供
  - get_step_slice(), get_step_slice_mut() でステップ単位スライスアクセスを提供（TimeStepFirst 専用、PathFirst は None）
  - clear() でバッファ再利用を可能にする
  - Send + Sync 境界を設定
  - _Requirements: 1.4, 7.4_

- [x] 3.2 既存 PathWorkspace への PathWorkspaceTrait 実装
  - 既存の PathWorkspace に PathWorkspaceTrait を実装
  - get_step_slice() は None を返す（PathFirst は非対応）
  - layout() は PathLayout::PathFirst を返す
  - 既存テストが引き続きパスすることを確認
  - _Requirements: 7.1, 7.2, 7.3_

- [x] 3.3 WorkspaceEnum による静的ディスパッチの実装
  - WorkspaceEnum<T> を定義し、PathFirst(PathWorkspace) と TimeStepFirst(TimeStepFirstWorkspace<T>) バリアントを持つ
  - PathWorkspaceTrait を WorkspaceEnum に実装（match で委譲）
  - MonteCarloPricer が WorkspaceEnum を直接保持するよう設計
  - dyn Trait のオーバーヘッドを回避し、インライン化を促進
  - _Requirements: 6.3, 7.4_

---

- [x] 4. TimeStepFirstWorkspace の実装
- [x] 4.1 TimeStepFirstWorkspace 構造体の実装
  - [step][path] メモリレイアウト（step_idx * num_paths + path_idx）を実装
  - AlignedPathBuffer を内部バッファとして使用
  - buffer（パス値）、randoms（乱数）、payoffs（ペイオフ）の3バッファを保持
  - new(), with_alignment() コンストラクタを提供
  - _Requirements: 1.1, 1.2_

- [x] 4.2 TimeStepFirstWorkspace のトレイト実装
  - PathWorkspaceTrait を実装
  - get_aligned_step_slice(), get_aligned_step_slice_mut() でアラインド保証付きスライスを返す
  - layout() は PathLayout::TimeStepFirst を返す
  - _Requirements: 1.3, 1.4_

- [x] 4.3 インデックス計算とスライスアクセスのテスト
  - [step][path] レイアウトのインデックス計算が正確であることを検証
  - get_aligned_step_slice() が連続メモリを返すことを確認
  - 境界チェックのテスト（範囲外アクセスでパニック）
  - _Requirements: 1.1, 1.2, 1.3_

---

### Phase 3: パス生成の拡張

- [x] 5. PathGenerator の実装
- [x] 5.1 ジェネリックパス生成関数の実装
  - generate_gbm_paths_generic<W: PathWorkspaceTrait> を実装
  - TimeStepFirst モードでは get_step_slice_mut() を使用してステップ単位で処理
  - PathFirst モードでは従来通り個別アクセスにフォールバック
  - 同一シードで同一パスを生成することを保証
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [x] 5.2 後方互換エイリアスの維持
  - 既存の generate_gbm_paths() シグネチャを維持
  - 内部で generate_gbm_paths_generic() に委譲
  - 既存の呼び出しコードが変更なしで動作することを確認
  - _Requirements: 7.1, 7.2, 7.4_

- [x] 5.3 パス生成の数値一致検証テスト
  - PathFirst と TimeStepFirst で同一シード時に同一パスを生成することを検証
  - European option の価格が両レイアウトで一致することを確認
  - 統計的検定（t検定）でパス分布の同等性を検証
  - _Requirements: 7.2, 7.3_

---

### Phase 4: ストリーミングエンジン

- [ ] 6. StreamingEngine の実装
- [ ] 6.1 StreamingEngine 構造体の実装
  - ダブルバッファ（current, previous）を AlignedPathBuffer で保持
  - RNG、設定、現在ステップインデックスを管理
  - new() で初期化、memory_usage() でメモリ使用量を返す
  - _Requirements: 2.1, 2.4_

- [ ] 6.2 StreamingObserver トレイトの定義
  - observe_step(step_idx, values) でステップごとの観測を受け取る
  - finalize() で累積統計を返す
  - reset() でオブザーバー状態をリセット
  - Send + Sync 境界を設定
  - _Requirements: 4.1_

- [ ] 6.3 ストリーミング処理ループの実装
  - run() メソッドで全ステップをループ処理
  - 各ステップで generate → observe → swap_buffers のサイクルを実行
  - ダブルバッファのスワップはポインタ交換のみ（O(1)）
  - _Requirements: 2.2, 2.3_

- [ ] 6.4 既存 PathObserver のストリーミング対応
  - PathObserver に StreamingObserver トレイトを実装
  - observe_step() 内で各パスの observe() を呼び出し
  - Asian（算術平均）、Barrier（バリア監視）、Lookback（最小/最大追跡）との互換性を確保
  - _Requirements: 4.1, 4.2, 4.3, 4.4_

- [ ] 6.5 ストリーミング処理のテスト
  - メモリ使用量が O(num_paths) であることを検証（1M paths × 100 steps）
  - ストリーミングと一括処理で Asian option 価格が一致することを確認
  - ダブルバッファスワップが正しく動作することを検証
  - _Requirements: 2.1, 2.2, 2.3, 2.4_

---

### Phase 5: 統合とAPI

- [ ] 7. MonteCarloPricer の拡張と性能検証
- [ ] 7.1 MonteCarloConfig ビルダーの拡張
  - .layout(PathLayoutConfig) メソッドを追加
  - .streaming(StreamingConfig) メソッドを追加
  - 無効な組み合わせ（PathFirst + Streaming）でエラーを返す
  - _Requirements: 5.3, 5.4_

- [ ] 7.2 MonteCarloPricer のレイアウト対応
  - WorkspaceEnum を内部保持するよう変更
  - 設定に応じて PathFirst または TimeStepFirst を選択
  - price_streaming() メソッドを追加（StreamingEngine 使用）
  - 既存の price_european() 等は変更なしで動作
  - _Requirements: 7.1, 7.2, 7.4_

- [ ] 7.3 性能ベンチマークの作成
  - Criterion で bench_timestep_first_vs_path_first を作成
  - bench_streaming_memory でメモリ使用量を測定
  - bench_aligned_vs_unaligned でアラインメント効果を測定
  - bench_static_vs_dyn_dispatch でディスパッチオーバーヘッドを比較
  - Rayon 並列実行時のスケーラビリティを検証
  - _Requirements: 6.1, 6.2, 6.3, 6.4_

- [ ] 7.4 後方互換性の最終検証
  - 既存の全テストスイートが変更なしでパスすることを確認
  - デフォルト設定（PathFirst, Streaming無効）で従来動作を保証
  - 新規 API が既存 API を破壊しないことを確認
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1 | 4.1, 4.3, 5.1 |
| 1.2 | 4.1, 4.3, 5.1 |
| 1.3 | 4.2, 4.3, 5.1 |
| 1.4 | 3.1, 4.2, 5.1 |
| 2.1 | 6.1, 6.5 |
| 2.2 | 6.3, 6.5 |
| 2.3 | 6.3, 6.5 |
| 2.4 | 6.1, 6.5 |
| 3.1 | 2.1, 2.2 |
| 3.2 | 2.1 |
| 3.3 | 2.2 |
| 3.4 | — (implicit in 5.1 loop design) |
| 4.1 | 6.2, 6.4 |
| 4.2 | 6.4 |
| 4.3 | 6.4 |
| 4.4 | 6.4 |
| 5.1 | 1.1 |
| 5.2 | 1.2 |
| 5.3 | 7.1 |
| 5.4 | 1.2, 1.3, 7.1 |
| 6.1 | 7.3 |
| 6.2 | 7.3 |
| 6.3 | 3.3, 7.3 |
| 6.4 | 7.3 |
| 7.1 | 3.2, 5.2, 7.2, 7.4 |
| 7.2 | 3.2, 5.3, 7.2, 7.4 |
| 7.3 | 3.2, 5.3, 7.4 |
| 7.4 | 3.1, 3.3, 5.2, 7.2, 7.4 |
