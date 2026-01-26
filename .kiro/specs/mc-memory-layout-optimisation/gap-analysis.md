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
├── workspace_checkpoint.rs → CheckpointWorkspace
└── payoff.rs          → compute_payoffs

crates/pricer_pricing/src/path_dependent/
├── observer.rs        → PathObserver (ストリーミング統計 - 既存)
├── asian.rs           → AsianPayoff
├── barrier.rs         → BarrierPayoff
└── lookback.rs        → LookbackPayoff
```

### 1.2 現行メモリレイアウト

**PathWorkspace** ([workspace.rs:44-59](crates/pricer_pricing/src/mc/workspace.rs#L44-L59))

```rust
pub struct PathWorkspace {
    randoms: Vec<f64>,    // n_paths × n_steps
    paths: Vec<f64>,      // n_paths × (n_steps + 1)
    payoffs: Vec<f64>,    // n_paths
    capacity_paths: usize,
    capacity_steps: usize,
    size_paths: usize,
    size_steps: usize,
}
```

**現行インデックス計算** ([workspace.rs:287-289](crates/pricer_pricing/src/mc/workspace.rs#L287-L289)):
```rust
// Path First レイアウト: paths[path][step]
fn path_index(&self, path_idx: usize, step_idx: usize) -> usize {
    path_idx * (self.size_steps + 1) + step_idx
}
```

**パス生成ループ** ([paths.rs:157-171](crates/pricer_pricing/src/mc/paths.rs#L157-L171)):
```rust
// 外側: パス、内側: ステップ
for path_idx in 0..n_paths {
    for step in 0..n_steps {
        // パス方向にストライドアクセス
        paths[path_offset + step + 1] = paths[path_offset + step] * increment.exp();
    }
}
```

### 1.3 PathObserver（既存ストリーミングパターン）

**PathObserver** ([observer.rs:56-69](crates/pricer_pricing/src/path_dependent/observer.rs#L56-L69)):
```rust
pub struct PathObserver<T: Float> {
    running_sum: T,           // Σ S_i
    running_product_log: T,   // Σ ln(S_i)
    running_max: T,
    running_min: T,
    count: usize,
    terminal: T,
}
```

これは要件2のストリーミング処理の基盤となる既存パターン。

### 1.4 規約とパターン

| 項目 | 現状 |
|------|------|
| 命名規則 | British English (optimiser, serialisation) |
| ジェネリクス | `T: Float` for AD compatibility |
| バッファ管理 | RAII, capacity/size分離 |
| テスト配置 | 同ファイル内 `#[cfg(test)]` |
| ビルダーパターン | MonteCarloConfig で使用済み |

---

## 2. 要件実現可能性分析

### 要件-アセットマップ

| 要件 | 既存アセット | ギャップ | 複雑度 |
|------|-------------|---------|--------|
| **Req 1**: Time Step First | PathWorkspace | **Missing** - 新レイアウト | 中 |
| **Req 2**: Streaming | PathObserver | **Partial** - エンジン未実装 | 高 |
| **Req 3**: Vectorisation | なし | **Missing** - アラインメント | 中 |
| **Req 4**: PathObserver統合 | PathObserver, price_path_dependent | **Partial** - 統合拡張 | 低 |
| **Req 5**: Config API | MonteCarloConfig | **Missing** - 新Config | 低 |
| **Req 6**: Performance | Criterion benchmarks | **Constraint** - 測定基盤必要 | 中 |
| **Req 7**: 後方互換性 | 既存API | **Constraint** - 維持必須 | 低 |

### 技術的ギャップ詳細

#### Gap 1: Time Step First レイアウト

**現状**: `paths[path][step]` - 同一パスの連続ステップが連続メモリ
**必要**: `paths[step][path]` - 同一ステップの全パスが連続メモリ

```rust
// 現行 (Path First)
index = path_idx * (n_steps + 1) + step_idx

// 必要 (Time Step First)
index = step_idx * n_paths + path_idx
```

#### Gap 2: ストリーミングエンジン

**現状**: 全パス・全ステップをメモリ保持
**必要**: 2タイムステップ分のバッファのみ保持（current, previous）

```rust
// 概念設計
struct StreamingPathBuffer {
    current_step: Vec<f64>,   // n_paths
    previous_step: Vec<f64>,  // n_paths
}
```

#### Gap 3: SIMD アラインメント

**現状**: Vec<f64> は通常8バイトアラインメント
**必要**: 64バイトアラインメント（AVX-512対応）

**Research Needed**: `aligned_vec` クレートまたは手動アラインメント実装

#### Gap 4: Config拡張

**現状**: MonteCarloConfig (n_paths, n_steps, ad_mode, seed)
**必要**: PathLayoutConfig, StreamingConfig の追加

---

## 3. 実装アプローチ選択肢

### Option A: PathWorkspace 拡張

**概要**: 既存のPathWorkspaceにレイアウトモードを追加

```rust
pub enum PathLayout {
    PathFirst,      // 既存: [path][step]
    TimeStepFirst,  // 新規: [step][path]
}

pub struct PathWorkspace {
    // ... existing fields ...
    layout: PathLayout,
}
```

**Trade-offs**:
- ✅ 最小限の新規ファイル
- ✅ 既存インフラ活用
- ❌ PathWorkspace の複雑化
- ❌ レイアウト切り替えのランタイムオーバーヘッド

### Option B: 新規コンポーネント作成

**概要**: TimeStepFirstWorkspace と StreamingEngine を新規作成

```rust
// 新規: crates/pricer_pricing/src/mc/workspace_timestep.rs
pub struct TimeStepFirstWorkspace { ... }

// 新規: crates/pricer_pricing/src/mc/streaming_engine.rs
pub struct StreamingEngine { ... }
```

**Trade-offs**:
- ✅ 明確な責任分離
- ✅ テスト容易性
- ✅ 既存コードへの影響最小
- ❌ 新規ファイル増加
- ❌ インターフェース設計必要

### Option C: ハイブリッドアプローチ（推奨）

**概要**: 設定は既存Config拡張、実装は新規コンポーネント

```rust
// Phase 1: Config拡張 (config.rs)
pub struct PathLayoutConfig {
    pub layout: PathLayout,
    pub alignment: usize,  // 64 for AVX-512
}

pub struct StreamingConfig {
    pub enabled: bool,
    pub buffer_steps: usize,  // 通常2
}

// Phase 2: 新規Workspace (workspace_timestep.rs)
pub struct TimeStepFirstWorkspace { ... }

// Phase 3: StreamingEngine (streaming_engine.rs)
pub struct StreamingEngine<W: PathWorkspaceTrait> { ... }
```

**Trade-offs**:
- ✅ 段階的実装可能
- ✅ 既存APIと新APIの共存
- ✅ Feature flag による制御
- ❌ 計画の複雑性

---

## 4. 工数・リスク評価

### 工数見積

| フェーズ | 内容 | 工数 |
|---------|------|------|
| Phase 1 | Config拡張 + PathLayout enum | **S** (1-2日) |
| Phase 2 | TimeStepFirstWorkspace | **M** (3-5日) |
| Phase 3 | StreamingEngine | **L** (5-7日) |
| Phase 4 | ベンチマーク・検証 | **M** (3-5日) |
| **合計** | | **L** (2週間) |

### リスク評価

| リスク | レベル | 軽減策 |
|--------|--------|--------|
| SIMD アラインメント実装 | **中** | aligned_vec クレート調査 |
| Enzyme AD 互換性 | **中** | 既存パターン踏襲 |
| PathObserver統合 | **低** | 既存パターン活用 |
| 後方互換性維持 | **低** | デフォルト値で既存動作保証 |
| 性能目標未達 | **中** | 早期ベンチマーク実施 |

**総合リスク**: **Medium**
- 既知技術の組み合わせ
- 既存パターン（PathObserver）の拡張
- 性能目標は具体的で測定可能

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option C（ハイブリッド）** を推奨

1. 設定は既存MonteCarloConfig のビルダー拡張
2. TimeStepFirstWorkspace を新規作成（PathWorkspace と並行）
3. StreamingEngine を新規トレイトベースで設計
4. Feature flag `streaming-mc` で段階的有効化

### 設計フェーズで要調査事項

1. **SIMD アラインメント**: `aligned_vec`, `std::alloc::Layout` 調査
2. **ベンチマーク基盤**: Criterion + perf/cachegrind 統合
3. **Enzyme互換性**: TimeStepFirst レイアウトでのAD動作確認
4. **StreamingEngine API**: イテレータ vs コールバック方式

### 成功指標（測定可能）

| 指標 | 現状 | 目標 | 測定方法 |
|------|------|------|----------|
| ピークメモリ (1M paths × 100 steps) | ~800MB | <80MB | `/proc/self/status` |
| キャッシュミス率 | TBD | 50%削減 | `perf stat` |
| スループット低下 | - | <10% | Criterion |

---

## 6. 結論

本機能は既存のPathObserverパターンを基盤として、Time Step First レイアウトとストリーミング処理を追加する中規模の改善である。

- **技術的実現可能性**: 高（既存パターン拡張）
- **工数**: L（2週間）
- **リスク**: Medium（性能目標達成が主要リスク）
- **推奨アプローチ**: Option C（ハイブリッド）

設計フェーズでは、SIMD アラインメントとStreamingEngine APIの詳細設計に注力すべき。
