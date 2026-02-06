# Gap Analysis: Pricer Computation Graph

## 1. Current State Investigation

### 1.1 既存の関連アセット

#### 計算グラフ型 (`pricer_pricing::graph`)

**場所**: `crates/pricer_pricing/src/graph/`

**既存の型**:
- `GraphNode` - ノード型（id, node_type, label, value, is_sensitivity_target, group, trade_ids）
- `GraphEdge` - エッジ型（source, target, weight）
- `ComputationGraph` - グラフ全体（nodes, edges, metadata）
- `GraphMetadata` - メタデータ（trade_id, node_count, edge_count, depth, generated_at）
- `NodeType` - 演算タイプ enum（Input, Add, Mul, Exp, Log, Sqrt, Div, Output, Custom）
- `NodeGroup` - 視覚グループ enum（Input, Intermediate, Output, Sensitivity）
- `PortfolioComputationGraph` - ポートフォリオ用拡張グラフ

**D3.js互換**:
- `edges` → `links` としてシリアライズ（D3.js互換）
- `node_type` → `type` としてシリアライズ
- すべてのenum variantは小文字でシリアライズ

**ギャップ**:
- ❌ `source_location` フィールドなし（要件2.4）
- ❌ `scope_id` フィールドなし（要件3.6）
- ❌ `Scope` 構造体なし（要件3）
- ❌ `Sub`, `Sin`, `Cos`, `Ln` などの演算タイプが不足

#### pricer_core types モジュール

**場所**: `crates/pricer_core/src/types/`

**既存のモジュール**:
- `dual.rs` - num-dual統合（feature: `num-dual-mode`）
- `time.rs` - 時間ユーティリティ
- `currency_pair.rs` - FxRate型
- `error.rs` - エラー型

**Feature Flags**:
- `num-dual-mode` (default) - num-dual統合
- `enzyme-mode` - Enzyme AD用

**ギャップ**:
- ❌ `traced` モジュールなし（要件7.1）
- ❌ `execution-trace` feature flagなし（要件7.3）

#### T: Float ジェネリクス使用状況

**影響範囲**: 62ファイルが `T: Float` パターンを使用

**主要ファイル**:
- `pricer_models/src/analytical/` - Black-Scholes, Garman-Kohlhagen等
- `pricer_models/src/instruments/vanilla.rs` - VanillaOption
- `pricer_models/src/market/` - カーブ、サーフェス、キャリブレーション
- `pricer_models/src/models/` - Heston, SABR, Hull-White, GBM等

**評価**: TracedFloatが`num_traits::Float`を実装すれば、既存の62ファイルは変更不要で動作する。

#### WebApp Instrument Graph

**場所**: `demo/gui/`

**影響ファイル** (5ファイル):
- `static/index.html` - UI要素（10箇所参照）
- `static/app.js` - JavaScript（15箇所参照）
- `static/style.css` - スタイル（2箇所参照）
- `src/web/handlers.rs` - APIハンドラ（3箇所参照）
- `src/web/mod.rs` - ルーティング（2箇所参照）

**既存エンドポイント**: `/api/instrument-graph`

**ギャップ**:
- ❌ 改名が必要: "Instrument Graph" → "Pricer Graph"
- ❌ スコープ表示機能なし
- ❌ 詳細度切り替えなし
- ❌ ソース位置表示なし

### 1.2 既存パターンと規約

#### アーキテクチャパターン

- **A-I-P-S単方向フロー**: Adapter → Infra → Pricer → Service
- **Pricerレイヤー階層**: L1 (pricer_core) → L2 (pricer_models) → L3 (pricer_pricing) → L4 (pricer_risk)
- **静的ディスパッチ**: Enzyme最適化のためenumベース（`Box<dyn Trait>`不使用）

#### Feature Flagパターン

```toml
# 既存パターン（pricer_core）
[features]
default = ["num-dual-mode", "serde", "equity", "parallel"]
num-dual-mode = ["dep:num-dual"]
enzyme-mode = []
```

#### テスト配置

- 単体テストは実装ファイル内に`#[cfg(test)]`で配置
- 統合テストは`tests/`ディレクトリ

### 1.3 統合サーフェス

#### 依存関係

- `pricer_core` → `infra_domain` (型定義のみ)
- `pricer_pricing` → `pricer_core`, `pricer_models`
- `demo/gui` → `pricer_pricing`, `pricer_models`, `pricer_core`

#### proc-macroクレート

**発見**: `neutryx_macros`クレートは存在しない（新規作成必要）

---

## 2. 要件実現可能性分析

### 要件-アセットマップ

| 要件 | 技術的ニーズ | 既存アセット | ギャップ |
|------|-------------|-------------|---------|
| 1.1-1.6 | TracedFloat型、Float trait実装 | `num_traits::Float` (外部) | **Missing**: TracedFloat型 |
| 2.1-2.5 | `#[track_caller]`、SourceLocation | Rust標準ライブラリ | **Missing**: SourceLocation構造体、ノード拡張 |
| 3.1-3.8 | proc-macro、スコープ管理 | なし | **Missing**: neutryx_macrosクレート、Scope型 |
| 4.1-4.5 | DetailLevel、スコープ集約 | ComputationGraph | **Missing**: DetailLevel enum、集約ロジック |
| 5.1-5.7 | UI改修、D3.js | Instrument Graph UI | **Constraint**: 改名と機能拡張必要 |
| 6.1-6.4 | REST API | `/api/instrument-graph` | **Constraint**: 新エンドポイント必要 |
| 7.1-7.6 | Feature flag、既存構造維持 | Feature flagパターン | **Missing**: `execution-trace` feature |

### 複雑性シグナル

| 領域 | 複雑性 | 理由 |
|------|--------|------|
| TracedFloat Float実装 | **高** | 約75メソッドの実装が必要 |
| proc-macro | **中** | syn/quote使用、標準的なパターン |
| グラフ型拡張 | **低** | 既存型へのフィールド追加 |
| WebApp UI | **低** | 既存パターンの改修 |

---

## 3. 実装アプローチ選択肢

### Option A: 既存コンポーネント拡張

**適用**: 計算グラフ型、WebApp UI

**対象ファイル**:
- `pricer_pricing/src/graph/types.rs` - GraphNode拡張
- `demo/gui/` - UI改修

**互換性評価**:
- ✅ GraphNodeへのフィールド追加は後方互換（`#[serde(skip_serializing_if)]`使用）
- ✅ WebApp UIは段階的改修可能

**トレードオフ**:
- ✅ 最小限の新規ファイル
- ✅ 既存パターン活用
- ❌ GraphNodeが肥大化するリスク

### Option B: 新規コンポーネント作成

**適用**: TracedFloat型、proc-macroクレート

**新規作成**:
- `pricer_core/src/types/traced.rs` - TracedFloat、ExecutionTrace
- `crates/neutryx_macros/` - proc-macroクレート

**理由**:
- TracedFloatは独自の責務を持つ新しい型
- proc-macroは独立クレートが必要（Rust制約）

**トレードオフ**:
- ✅ 明確な責務分離
- ✅ テスト容易性
- ❌ 新規クレート（neutryx_macros）の作成が必要

### Option C: ハイブリッドアプローチ（推奨）

**組み合わせ戦略**:
1. **新規作成**: TracedFloat (`pricer_core`)、neutryx_macros（proc-macro）
2. **拡張**: GraphNode/ComputationGraph (`pricer_pricing`)、WebApp UI

**段階的実装**:
1. Phase 1: TracedFloat基本実装（Float trait）
2. Phase 2: ExecutionTrace、SourceLocation
3. Phase 3: proc-macro（`#[traced_scope]`）
4. Phase 4: グラフ型拡張、GraphExporter
5. Phase 5: REST API、WebApp UI

**リスク軽減**:
- Feature flag (`execution-trace`) で段階的有効化
- 既存コードへの影響なし
- ロールバック容易

---

## 4. 実装複雑性とリスク

### 工数見積もり

| コンポーネント | 工数 | 理由 |
|---------------|------|------|
| TracedFloat基本型 | **M** (3-7日) | Float trait約75メソッド実装 |
| ExecutionTrace | **S** (1-3日) | ノード/エッジ管理、スコープスタック |
| neutryx_macros | **M** (3-7日) | proc-macro、syn/quote使用 |
| GraphExporter | **S** (1-3日) | 既存型への変換 |
| REST API | **S** (1-3日) | 既存パターンの適用 |
| WebApp UI | **M** (3-7日) | 改名、新機能追加 |
| **合計** | **L** (1-2週間) | 中規模機能追加 |

### リスク評価

| リスク | レベル | 軽減策 |
|--------|--------|--------|
| Float trait実装の複雑性 | **中** | 段階的実装、num-dual参照 |
| proc-macro互換性 | **低** | 標準的なsyn/quoteパターン |
| 既存コードへの影響 | **低** | Feature flag分離 |
| パフォーマンスオーバーヘッド | **低** | トレース時のみ影響 |

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option C: ハイブリッドアプローチ**を推奨

- 新規: TracedFloat、ExecutionTrace、neutryx_macros
- 拡張: GraphNode（source_location, scope_id追加）、WebApp UI

### キーディシジョン

1. **TracedFloat配置**: `pricer_core::types::traced`（L1レイヤー）
2. **スコープ管理**: スレッドローカル変数（`TRACE_CONTEXT`）
3. **proc-macro**: 独立クレート `neutryx_macros`
4. **Feature flag**: `execution-trace`（デフォルト無効）

### リサーチ項目

1. ~~num_traits::Float実装の全メソッドリスト~~ → 約75メソッド（確認済み）
2. ~~`#[track_caller]`の伝播挙動~~ → Rust 1.46+で安定
3. proc-macroでのfeature flag連携パターン → 設計フェーズで詳細化

---

## サマリー

- **スコープ**: TracedFloat型による計算グラフ自動取得、WebApp UI改修
- **課題**: Float trait約75メソッド実装、proc-macroクレート新規作成
- **推奨**: ハイブリッドアプローチ（新規+拡張）、feature flag分離
- **工数**: L (1-2週間)
- **リスク**: 中（Float実装の複雑性）

設計フェーズでは、TracedFloatの詳細設計とproc-macroの実装パターンを重点的に検討することを推奨。
