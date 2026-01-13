# Research & Design Decisions

---
**目的**: 発見事項、アーキテクチャ調査、技術設計に影響する根拠を記録する。

**用途**:
- 発見フェーズでの調査活動と結果を記録する
- `design.md` には詳細すぎる設計決定のトレードオフを文書化する
- 将来の監査や再利用のための参照と証拠を提供する
---

## Summary

- **機能**: `computation-graph-visualisation`
- **発見スコープ**: 拡張機能（既存 FrictionalBank Web ダッシュボードへの統合）
- **主要な発見事項**:
  - D3.js SVG レンダリングは 10,000 ノードで限界に達する（Canvas/WebGL への切り替えが必要）
  - 既存の WebSocket インフラ（`broadcast::channel`）はグラフ更新メッセージに対応可能
  - A-I-P-S アーキテクチャに従い、グラフ抽出は Pricer レイヤー、API は Service レイヤーに配置

## Research Log

### D3.js 大規模グラフレンダリングのパフォーマンス

- **コンテキスト**: 要件 7.2 で 10,000 ノードのグラフを 60fps でレンダリングする必要がある
- **参照した資料**:
  - [d3-force | D3 by Observable](https://d3js.org/d3-force)
  - [Creating a large-scale interactive force-directed graph](https://dianaow.com/posts/pixijs-d3-graph)
  - [3d-force-graph GitHub](https://github.com/vasturiano/3d-force-graph)
- **発見事項**:
  - SVG の実用上限は約 10,000 要素、それ以上はアニメーションがカクつく
  - d3-force アルゴリズム自体は O(n log n) で高速だが、描画がボトルネック
  - PIXI.js + D3 の組み合わせで WebGL ハードウェアアクセラレーションを活用可能
  - Canvas レンダリングは SVG より大幅に高速
  - Web Worker でレイアウト計算をオフロードする手法も有効
- **アーキテクチャへの影響**:
  - 500 ノード以上では Canvas/WebGL レンダリングに切り替え（LOD 機能）
  - 初期実装は D3.js + SVG、パフォーマンス要件達成のため Canvas レイヤーをオプション追加

### 既存 FrictionalBank Web ダッシュボード分析

- **コンテキスト**: 既存アーキテクチャとの統合ポイントを特定
- **参照した資料**:
  - `demo/gui/src/web/mod.rs` - axum ルーター構成
  - `demo/gui/src/web/handlers.rs` - REST API ハンドラパターン
  - `demo/gui/src/web/websocket.rs` - WebSocket ブロードキャスト実装
  - `demo/gui/static/app.js` - フロントエンド状態管理
- **発見事項**:
  - REST API パターン: `State<Arc<AppState>>` + `Json<Response>`
  - WebSocket: `broadcast::channel(100)` でクライアントへブロードキャスト
  - `RealTimeUpdate` 構造体で `update_type`, `timestamp`, `data` フィールドを定義
  - ナビゲーション: `navigateTo(viewName)` パターンで画面切り替え
  - Bento Grid レイアウトとライト/ダークテーマ対応済み
- **アーキテクチャへの影響**:
  - 既存の `RealTimeUpdate` パターンに従い `graph_update` メッセージタイプを追加
  - `GET /api/graph` を既存の API ルートに追加
  - `state.graphs = {}` で状態管理を拡張

### Serde による計算グラフ JSON シリアライゼーション

- **コンテキスト**: Rust データ構造から JSON API レスポンスへの変換
- **参照した資料**:
  - [serde-rs/json GitHub](https://github.com/serde-rs/json)
  - [Serde data model](https://serde.rs/data-model.html)
- **発見事項**:
  - serde_json は 500-1000 MB/秒のデシリアライズ、600-900 MB/秒のシリアライズ性能
  - `#[derive(Serialize)]` で自動実装、カスタムシリアライズも可能
  - グラフ構造は `nodes: Vec<Node>`, `edges: Vec<Edge>` の形式が一般的
- **アーキテクチャへの影響**:
  - `ComputationGraph`, `GraphNode`, `GraphEdge` に `#[derive(Serialize)]` を付与
  - 10,000 ノードのグラフでも JSON シリアライズは数ミリ秒以内

## Architecture Pattern Evaluation

| オプション | 説明 | 強み | リスク/制限 | 備考 |
|----------|------|------|------------|------|
| Pricer 内蔵グラフ抽出 | pricer_pricing にグラフ抽出モジュールを追加 | A-I-P-S に準拠、既存の pricing context と統合可能 | pricer_pricing の依存関係増加 | **採用** |
| Service レイヤーでグラフ構築 | demo/gui でプライシング結果からグラフ再構築 | Pricer レイヤーの変更不要 | 二重実装、不整合リスク | 却下 |
| 別 crate (graph_extractor) | 独立した crate でグラフ抽出を実装 | 疎結合、再利用可能 | 新規 crate 追加のオーバーヘッド | 将来検討 |

## Design Decisions

### Decision: グラフデータ構造の設計

- **コンテキスト**: 計算グラフをフロントエンドで可視化するためのデータ構造が必要
- **検討した代替案**:
  1. petgraph 互換形式 — 豊富なグラフアルゴリズム利用可能だが複雑
  2. D3.js ネイティブ形式 — `{ nodes: [...], links: [...] }` でフロントエンド親和性高い
  3. GraphQL 形式 — 柔軟だがオーバーヘッド大
- **採用アプローチ**: D3.js ネイティブ形式
- **根拠**: フロントエンドでの追加変換が不要、既存の D3.js コードとの互換性
- **トレードオフ**: Rust 側で D3.js 用にマッピングが必要だが、シンプルなフィールドマッピングのみ
- **フォローアップ**: 実装時にメモリ使用量を計測

### Decision: 大規模グラフのレンダリング戦略

- **コンテキスト**: 10,000 ノードで 60fps レンダリングが必要（要件 7.2）
- **検討した代替案**:
  1. D3.js + SVG のみ — 実装が簡単だが 10,000 ノードで限界
  2. D3.js + Canvas — 高速だが実装コスト中
  3. PIXI.js + D3.js — 最高速だが複雑
  4. WebGL (Three.js) — 3D 可能だが学習曲線急
- **採用アプローチ**: 階層的アプローチ（SVG → Canvas フォールバック）
  - 500 ノード以下: D3.js + SVG（インタラクティブ性重視）
  - 500-10,000 ノード: D3.js + Canvas レンダリング
  - 10,000 ノード超: LOD（Level of Detail）で階層的折りたたみ
- **根拠**: 段階的実装が可能、既存 D3.js 知識を活用
- **トレードオフ**: 2 つのレンダラーのメンテナンスが必要
- **フォローアップ**: Phase 1 は SVG のみ実装、Phase 2 で Canvas 追加

### Decision: WebSocket メッセージ設計

- **コンテキスト**: マーケットデータ更新時にグラフ差分をリアルタイム配信
- **検討した代替案**:
  1. 全グラフ再送信 — 実装が簡単だが帯域幅過大
  2. 差分のみ送信 — 効率的だが複雑
  3. イベントソーシング — 最高の柔軟性だがオーバーエンジニアリング
- **採用アプローチ**: 差分送信（更新ノードのみ）
  ```json
  {
    "type": "graph_update",
    "trade_id": "T001",
    "updated_nodes": [{"id": "N1", "value": 1.234}],
    "timestamp": 1704000000000
  }
  ```
- **根拠**: 帯域幅効率と実装複雑性のバランス
- **トレードオフ**: クライアント側で差分マージのロジックが必要
- **フォローアップ**: 大規模更新時のバッチ化を検討

## Risks & Mitigations

- **リスク 1**: 10,000 ノードでのパフォーマンス未達
  - **緩和策**: Canvas レンダリングと LOD を Phase 2 で実装、Web Worker でレイアウト計算をオフロード

- **リスク 2**: Enzyme AD グラフの抽出が既存プライシングに影響
  - **緩和策**: グラフ抽出をオプション化（feature flag）、パフォーマンス影響を 5% 以内に制限

- **リスク 3**: WebSocket メッセージのサイズが大きくなりすぎる
  - **緩和策**: ページネーション対応、圧縮（gzip）オプション追加

- **リスク 4**: フロントエンドの状態管理が複雑化
  - **緩和策**: 専用の `GraphManager` クラスでカプセル化、既存パターンに従う

## References

- [d3-force | D3 by Observable](https://d3js.org/d3-force) — D3.js 力学シミュレーション公式ドキュメント
- [Creating a large-scale interactive force-directed graph](https://dianaow.com/posts/pixijs-d3-graph) — PIXI.js + D3.js 統合パターン
- [3d-force-graph GitHub](https://github.com/vasturiano/3d-force-graph) — WebGL ベースの 3D グラフライブラリ
- [serde-rs/json GitHub](https://github.com/serde-rs/json) — Rust JSON シリアライゼーション
- [Serde data model](https://serde.rs/data-model.html) — Serde データモデル解説
