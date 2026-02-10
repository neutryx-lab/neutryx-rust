# 研究・設計決定ログ

## サマリ

- **Feature**: `advanced-sensitivity-webapp`
- **Discovery Scope**: Extension（既存システム拡張）
- **主要な発見事項**:
  - `GreeksResult<T>` 基盤は完成済み、リスクファクター ID マッピングのみ追加が必要
  - WebApp は `risk_compare` エンドポイントで Bump vs AAD 比較を既に実装済み
  - 可視化（ヒートマップ、時系列）とメトリクス（Prometheus）が主要な新規開発項目

---

## 研究ログ

### 既存 Greeks 実装の分析

- **コンテキスト**: Req 1 のリスクファクター毎 Greeks 計算の実現可能性調査
- **参照ソース**: `pricer_pricing/src/greeks/`, `pricer_pricing/src/irs_greeks/`
- **発見事項**:
  - `GreeksResult<T>` は Delta, Gamma, Vega, Theta, Rho, Vanna, Volga を保持
  - AD 互換性のためジェネリック `T: Float` を使用
  - `IrsGreeksCalculator` はテナー毎の Delta 計算をサポート
  - リスクファクター識別子は未実装
- **影響**: `RiskFactorId` 型と `GreeksResultByFactor<T>` の新規追加が必要

### WebApp 既存エンドポイント分析

- **コンテキスト**: Req 4, 7 の WebApp 統合・API 拡充の基盤調査
- **参照ソース**: `demo/gui/src/web/handlers.rs`, `demo/gui/src/web/websocket.rs`
- **発見事項**:
  - `/api/risk/bump`, `/api/risk/aad`, `/api/risk/compare` が実装済み
  - `/api/graph` で D3.js 互換計算グラフを返却
  - `/api/speed-comparison` でパフォーマンス比較チャートデータを提供
  - WebSocket は graph 更新、benchmark 更新をサポート
- **影響**: 既存パターンを踏襲しつつ新規エンドポイント追加

### シナリオ分析基盤

- **コンテキスト**: Req 6 のシナリオ分析 UI 実現可能性調査
- **参照ソース**: `pricer_risk/src/scenarios/`
- **発見事項**:
  - `PresetScenario` enum: `RateShock`, `VolSpike`, `FxCrisis` 等
  - `RiskFactorShift`: 個別リスクファクターのシフト定義
  - `ScenarioEngine`: シナリオ適用と PnL 計算
  - `GreeksAggregator`: ポートフォリオレベル集計
- **影響**: バックエンドは完成、Web UI 連携のみ必要

### OpenAPI ドキュメント生成

- **コンテキスト**: Req 7.5 の OpenAPI 3.0 ドキュメント生成方式調査
- **発見事項**:
  - `utoipa`: Axum 統合、derive マクロで自動生成、Swagger UI 同梱
  - Rejected: `aide` (やや学習コスト高)、`paperclip` (OpenAPI 2.0 中心)
- **影響**: `utoipa` + `utoipa-swagger-ui` を採用（Axum 親和性、成熟度）

### Prometheus メトリクス統合

- **コンテキスト**: Req 8.2 の Prometheus メトリクス出力方式調査
- **発見事項**:
  - `metrics` + `metrics-exporter-prometheus`: facade パターン、軽量
  - Rejected: `prometheus` (公式だが重い)
- **影響**: `metrics` + `metrics-exporter-prometheus` を採用（既存構造との統合容易性）

---

## 設計決定

### 決定 1: リスクファクター識別子の設計

**選択したアプローチ**: `enum RiskFactorId` with variants
```rust
pub enum RiskFactorId {
    Underlying(String),
    Curve(String),
    VolSurface(String),
}
```
- **理由**: 型安全性と Enzyme 互換性（static dispatch）のバランス
- **トレードオフ**: 新規リスクファクタータイプ追加時は enum 変更が必要
- **選択**: `enum RiskFactorId`, Rejected: `String` 識別子、`trait RiskFactor`

### 決定 2: バケット DV01 計算方式

**選択したアプローチ**: 既存 `IrsGreeksCalculator` に `calculate_bucket_dv01` メソッド追加
- **理由**: テナー毎 Delta 計算は既に `IrsGreeksCalculator` で実装済み、最小の変更で実現可能
- **トレードオフ**: `IrsGreeksCalculator` の責務がやや拡大
- **選択**: 既存拡張, Rejected: 新規 `BucketDv01Calculator`, `GreeksAggregator` 拡張

### 決定 3: 可視化エンドポイント設計

**選択したアプローチ**: 個別 REST エンドポイント
- `/api/greeks/heatmap`
- `/api/greeks/timeseries`
- **理由**: 既存 REST パターンとの一貫性、CDN キャッシュ可能
- **トレードオフ**: エンドポイント数増加
- **選択**: 個別エンドポイント, Rejected: 単一エンドポイント + クエリパラメータ、GraphQL

### 決定 4: 非同期ジョブ API 設計

**選択したアプローチ**: ポーリング型ジョブ API
- POST → `{ job_id: "..." }` 即座返却
- GET `/api/v1/jobs/{id}` → 進捗/結果
- **理由**: REST クライアント互換性、シンプルな実装
- **トレードオフ**: ポーリングによるレイテンシ
- **選択**: ポーリング型, Rejected: 同期レスポンス + タイムアウト、WebSocket 進捗のみ

---

## リスクと緩和策

| リスク | 緩和策 |
|--------|--------|
| `handlers.rs` の更なる肥大化 | 新規エンドポイントは `handlers/greeks.rs` に分離開始 |
| Enzyme 環境依存 | `enzyme-ad` feature flag で fallback モード維持 |
| パフォーマンス目標（5倍速度）未達 | ベンチマーク駆動開発、早期計測 |
| フロントエンド工数増大 | D3.js 既存パターン活用、プリセット UI 優先 |

---

## 参照

- [utoipa - OpenAPI documentation for Rust](https://github.com/juhaku/utoipa)
- [metrics-rs](https://github.com/metrics-rs/metrics)
- [Enzyme AD](https://enzyme.mit.edu/)
