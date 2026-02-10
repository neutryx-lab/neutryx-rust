# Gap Analysis: portfolio-graph-optimisation

## 1. Current State Investigation

### 1.1 既存のGraph関連アセット

#### pricer_pricing::graph モジュール（完全実装済み）
| ファイル | 内容 |
|----------|------|
| types.rs | `GraphNode`, `GraphEdge`, `ComputationGraph`, `GraphMetadata`, `GraphNodeUpdate`, `NodeType`, `NodeGroup` |
| extractor.rs | `GraphExtractable` trait, `SimpleGraphExtractor`, `GraphBuilder` |
| error.rs | `GraphError` (TradeNotFound, ExtractionFailed, Timeout) |

**主要な特徴:**
- D3.js互換JSON出力（`edges` → `links`, `node_type` → `type`）
- 単一トレード or 全トレード統合でのグラフ抽出
- 500msタイムアウト保護
- `GraphBuilder`による事前割り当てメモリ管理
- BFSによるパス検索、トポロジカルソートによるクリティカルパス計算

#### pricer_risk::portfolio モジュール（完全実装済み）
| ファイル | 内容 |
|----------|------|
| mod.rs | `Portfolio` struct, O(1)ルックアップ, Rayon並列イテレーション |
| builder.rs | `PortfolioBuilder` (バリデーション付きビルダーパターン) |
| trade.rs | `Trade`, `TradeBuilder`, `TradeId` |
| counterparty.rs | `Counterparty`, `CreditParams` |
| netting_set.rs | `NettingSet`, `CollateralAgreement` |

**主要な特徴:**
- Trade/Counterparty/NettingSetの3層構造
- HashMap<TradeId, Trade>による高速ルックアップ
- `trades_par_iter()`によるRayon並列処理
- `trades_in_netting_set()`, `trades_for_counterparty()`などのクエリ機能

#### pricer_risk::demo モジュール
- `DemoTrade`: 簡易トレード構造（id, ccy, model, instrument）
- `run_portfolio_pricing()`: Pull-then-Push並列プライシング
- `new_vanilla_swap()`, `new_cms_swap()`: サンプルトレード生成ファクトリ

#### Web Dashboard（demo/gui/src/web/）
| ファイル | 内容 |
|----------|------|
| mod.rs | `AppState`（graph_cache, graph_subscriptions含む）, Router構築 |
| handlers.rs | `get_graph()` handler, `GraphCache`, `sample_trades()` |
| websocket.rs | `broadcast_graph_update()`, `RealTimeUpdate::graph_update()` |

**既存APIエンドポイント:**
- `GET /api/graph` - 単一トレードまたは全トレードのグラフ取得
- `GET /api/portfolio` - ポートフォリオデータ取得（12件のサンプルトレード）
- WebSocket: `graph_update` メッセージタイプ

**ギャップ**:
- Missing: "Instrument Graph" → "Pricer Graph"改名が必要
- Missing: スコープ表示機能なし
- Missing: 詳細度切り替えなし
- Missing: ソース位置表示なし

### 1.2 コーディング規約・パターン

| 項目 | パターン |
|------|----------|
| 命名 | British English (optimiser, visualisation) |
| エラー処理 | `Result<T, Error>` + `thiserror` |
| シリアライゼーション | `serde` with feature gate |
| 並列処理 | `rayon::par_iter()` |
| キャッシュ | `RwLock<T>` + TTL (5秒) |
| API設計 | `/api/v1/...` RESTful |
| 型安全 | Newtype (TradeId, NettingSetId等) |

---

## 2. Requirements Feasibility Analysis

### 2.1 要件ごとの技術的ニーズと既存資産

| 要件 | 必要技術 | 既存資産 | Gap |
|------|----------|----------|-----|
| **Req 1: Portfolio Graph** | グラフ統合、ノード重複排除 | `SimpleGraphExtractor`, `GraphBuilder` | Missing: Portfolio単位の抽出、共有ノード検出、`trade_ids: Vec<String>` フィールド |
| **Req 2: Sample Portfolio** | 多様なInstrument生成 | `DemoTrade`, `sample_trades()` | Missing: VanillaOption/IRS/FxOption混合生成、共有マーケットデータの意図的配置 |
| **Req 3: Subgraph抽出** | トレード選択フィルタリング | `extract_graph(trade_id)` | Missing: 複数trade_id指定、共有ノード保持ロジック |
| **Req 4: Web API** | REST endpoint | `/api/graph` | Extend: `/api/v1/portfolio/graph`, `trade_ids` クエリパラメータ |
| **Req 5: Trade一覧** | Portfolio metadata API | `get_portfolio()`, `sample_trades()` | Missing: `/api/v1/portfolio/trades`, Instrument種別統計 |
| **Req 6: Performance** | 最適化、LOD | `GraphBuilder` pre-alloc | Missing: LODモード、20%ノード削減検証、diffベース更新 |

### 2.2 識別されたギャップ

#### Missing（新規実装必要）

1. **PortfolioGraphExtractor** - Portfolioを入力として統合グラフを生成する新クラス
2. **GraphNode拡張** - `trade_ids: Vec<String>` フィールド追加（複数トレード所属追跡）
3. **PortfolioGraphMetadata** - トレード数、共有ノード数、最適化率を含む拡張メタデータ
4. **SamplePortfolioBuilder** - 複数アセットクラスのサンプルPortfolio生成
5. **extract_subgraph()** - 複数trade_id指定でのサブグラフ抽出
6. **LODモード** - 10,000ノード超での簡略化グラフ生成

#### Extend（既存拡張）

1. **SimpleGraphExtractor** - Portfolio対応 or 新クラスにラップ
2. **Web API routes** - `/api/v1/portfolio/graph`, `/api/v1/portfolio/trades`
3. **AppState** - PortfolioGraphCache追加
4. **WebSocket** - `select_trades` イベント対応

---

## 3. Implementation Approach Options

### Option A: Extend Existing SimpleGraphExtractor

**適用**: 既存の`SimpleGraphExtractor`をPortfolio対応に拡張

**変更対象ファイル**:
- `pricer_pricing/src/graph/types.rs` - GraphNode拡張
- `pricer_pricing/src/graph/extractor.rs` - Portfolio対応メソッド追加
- `demo/gui/src/web/handlers.rs` - API拡張

**トレードオフ**:
- Pros: 既存コードの再利用最大化、学習コスト低い
- Cons: `SimpleGraphExtractor`が肥大化するリスク、単一責任原則に違反する可能性

### Option B: Create New PortfolioGraphExtractor

**適用**: Portfolio専用の新クラスを作成、既存Extractorをコンポジション

**新規ファイル**:
- `pricer_pricing/src/graph/portfolio_extractor.rs`
- `pricer_pricing/src/graph/portfolio_types.rs`
- `pricer_risk/src/portfolio/sample_builder.rs`
- `demo/gui/src/web/portfolio_handlers.rs`

**トレードオフ**:
- Pros: 責務の明確な分離、既存機能への影響最小化、テスト容易性向上
- Cons: 新規ファイル数増加、既存Extractorとの重複コード可能性

### Option C: Hybrid Approach（推奨）

**組み合わせ戦略**:
1. **新規作成**: TracedFloat (`pricer_core`)、neutryx_macros（proc-macro）
2. **拡張**: GraphNode/ComputationGraph (`pricer_pricing`)、WebApp UI

**段階的実装**:
1. Phase 1: GraphNode拡張（`trade_ids`オプショナルフィールド）、SimpleGraphExtractorにPortfolio対応メソッド追加、Web API拡張
2. Phase 2: PortfolioGraphExtractor分離（Phase 1の学習反映）、LODモード実装、パフォーマンス最適化

**トレードオフ**:
- Pros: 段階的リスク軽減、早期デモ可能、リファクタリング機会確保
- Cons: 2フェーズの計画管理必要

---

## 4. Implementation Complexity & Risk

### Effort Estimate: **M (3–7 days)**

**根拠:**
- 既存のgraph/portfolioモジュールが成熟済み
- 新規ロジックはノード統合/重複排除アルゴリズムが中心
- Web API追加は既存パターンに従う

### Risk Assessment: **Medium**

**主なリスク:**
| リスク | 影響 | 緩和策 |
|--------|------|--------|
| ノード統合アルゴリズムの複雑性 | パフォーマンス劣化 | ハッシュベース O(n) 設計 |
| GraphNode拡張の後方互換性 | 既存API破損 | `trade_ids` をOption化 |
| LODモードの設計不明確さ | 実装遅延 | Phase 2に分離、先にMVP |

---

## 5. Recommendations for Design Phase

### 推奨アプローチ: **Option C (Hybrid)**

### Key Decisions（設計フェーズで決定）

1. **GraphNode.trade_ids**: `Option<Vec<String>>` vs `Vec<String>` (default empty)
2. **ノード同一性判定**: ラベル一致 vs ハッシュ(label+type) vs 明示的キー
3. **LODトリガー閾値**: 10,000ノード固定 vs 設定可能
4. **サンプルPortfolioサイズ**: デフォルト10 vs 50 vs 100トレード

---

_Generated: 2026-01-19_
