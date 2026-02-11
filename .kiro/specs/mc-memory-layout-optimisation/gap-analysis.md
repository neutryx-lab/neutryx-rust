# Gap Analysis: mc-memory-layout-optimisation

## 1. 現状調査

### 1.1 ディレクトリ構成とキーファイル

```text
crates/pricer_pricing/src/mc/
├── config.rs          → MonteCarloConfig, AdMode
├── workspace.rs       → PathWorkspace (現行メモリレイアウト)
├── paths.rs           → generate_gbm_paths (パス生成)
├── pricer.rs          → MonteCarloPricer (オーケストレーション)
├── thread_local.rs    → ThreadLocalWorkspacePool, ParallelWorkspaces
└── workspace_checkpoint.rs → CheckpointWorkspace
```

### 1.2 現行メモリレイアウト

**PathWorkspace**: Path First レイアウト `paths[path][step]`

**現行インデックス計算**: `path_index = path_idx * (size_steps + 1) + step_idx`

**パス生成ループ**: 外側パス、内側ステップ（パス方向にストライドアクセス）

### 1.3 PathObserver（既存ストリーミングパターン）

**PathObserver**: running_sum, running_product_log, running_max, running_min で統計累積

これは要件2のストリーミング処理の基盤となる既存パターン。

## 2. 要件実現可能性分析

| 要件 | 既存アセット | ギャップ | 複雑度 |
|------|-------------|---------|--------|
| **Req 1**: Time Step First | PathWorkspace | Missing - 新レイアウト | Medium |
| **Req 2**: Streaming | PathObserver | Partial - エンジン未実装 | High |
| **Req 3**: Vectorisation | なし | Missing - アラインメント | Medium |
| **Req 4**: PathObserver統合 | PathObserver, price_path_dependent | Partial - 統合拡張 | Low |
| **Req 5**: Config API | MonteCarloConfig | Missing - 新Config | Low |
| **Req 6**: Performance | Criterion benchmarks | Constraint - 測定基盤必要 | Medium |
| **Req 7**: 後方互換性 | 既存API | Constraint - 維持必須 | Low |

### 技術的ギャップ詳細

#### Gap 1: Time Step First レイアウト
**現状**: `paths[path][step]`
**必要**: `paths[step][path]`
```rust
// 現行: index = path_idx * (n_steps + 1) + step_idx
// 必要: index = step_idx * n_paths + path_idx
```

#### Gap 2: ストリーミングエンジン
**現状**: 全パス・全ステップをメモリ保持
**必要**: 2タイムステップ分のバッファのみ保持（current, previous）

#### Gap 3: SIMD アラインメント
**現状**: Vec<f64> は通常8バイトアラインメント
**必要**: 64バイトアラインメント（AVX-512対応）

## 3. 実装アプローチ選択肢

### Option C: ハイブリッドアプローチ（推奨）

**概要**: 設定は既存Config拡張、実装は新規コンポーネント

**新規ファイル:**
- `workspace_timestep.rs` - TimeStepFirstWorkspace
- `streaming_engine.rs` - StreamingEngine

**既存ファイル変更:**
- `config.rs` - PathLayoutConfig, StreamingConfig追加
- `mod.rs` - re-export追加

## 4. 工数・リスク評価

### 工数見積: L (2週間)
### リスク: Medium

**リスク要因:**
1. SIMD アラインメント実装
2. Enzyme AD 互換性
3. 性能目標未達

**緩和策:**
- aligned_vec クレート調査
- 既存パターン踏襲
- 早期ベンチマーク実施

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ: Option C（ハイブリッド）

設計フェーズでは、SIMD アラインメントとStreamingEngine APIの詳細設計に注力すべき。

### 成功指標（測定可能）

| 指標 | 現状 | 目標 | 測定方法 |
|------|------|------|----------|
| ピークメモリ (1M paths × 100 steps) | ~800MB | <80MB | `/proc/self/status` |
| キャッシュミス率 | TBD | 50%削減 | `perf stat` |
| スループット低下 | - | <10% | Criterion |
