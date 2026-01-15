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
- **参照ソース**:
  - `crates/pricer_pricing/src/greeks/result.rs`
  - `crates/pricer_pricing/src/greeks/config.rs`
  - `crates/pricer_pricing/src/irs_greeks/calculator.rs`
- **発見事項**:
  - `GreeksResult<T>` は Delta, Gamma, Vega, Theta, Rho, Vanna, Volga を保持
  - AD 互換性のためジェネリック `T: Float` を使用
  - `IrsGreeksCalculator` はテナー毎の Delta 計算をサポート
  - リスクファクター識別子は未実装
- **影響**: `RiskFactorId` 型と `GreeksResultByFactor<T>` の新規追加が必要

### WebApp 既存エンドポイント分析

- **コンテキスト**: Req 4, 7 の WebApp 統合・API 拡充の基盤調査
- **参照ソース**:
  - `demo/gui/src/web/handlers.rs` (3500+ LOC)
  - `demo/gui/src/web/websocket.rs`
- **発見事項**:
  - `/api/risk/bump`, `/api/risk/aad`, `/api/risk/compare` が実装済み
  - `/api/graph` で D3.js 互換計算グラフを返却
  - `/api/speed-comparison` でパフォーマンス比較チャートデータを提供
  - WebSocket は graph 更新、benchmark 更新をサポート
- **影響**: 既存パターンを踏襲しつつ新規エンドポイント追加

### シナリオ分析基盤

- **コンテキスト**: Req 6 のシナリオ分析 UI 実現可能性調査
- **参照ソース**:
  - `crates/pricer_risk/src/scenarios/presets.rs`
  - `crates/pricer_risk/src/scenarios/shifts.rs`
  - `crates/pricer_risk/src/scenarios/engine.rs`
- **発見事項**:
  - `PresetScenario` enum: `RateShock`, `VolSpike`, `FxCrisis` 等
  - `RiskFactorShift`: 個別リスクファクターのシフト定義
  - `ScenarioEngine`: シナリオ適用と PnL 計算
  - `GreeksAggregator`: ポートフォリオレベル集計
- **影響**: バックエンドは完成、Web UI 連携のみ必要

### OpenAPI ドキュメント生成

- **コンテキスト**: Req 7.5 の OpenAPI 3.0 ドキュメント生成方式調査
- **参照ソース**: Web 検索（Rust OpenAPI crates）
- **発見事項**:
  - `utoipa`: Axum 統合、derive マクロで自動生成、Swagger UI 同梱
  - `aide`: Axum ネイティブ、型安全、やや学習コスト高
  - `paperclip`: OpenAPI 2.0 中心、3.0 サポート限定的
- **影響**: `utoipa` + `utoipa-swagger-ui` を採用（Axum 親和性、成熟度）

### Prometheus メトリクス統合

- **コンテキスト**: Req 8.2 の Prometheus メトリクス出力方式調査
- **参照ソース**: Web 検索（Rust Prometheus crates）
- **発見事項**:
  - `prometheus`: 公式クライアント、Counter/Gauge/Histogram サポート
  - `metrics` + `metrics-exporter-prometheus`: facade パターン、軽量
  - 既存 `AppState.metrics` は独自実装
- **影響**: `metrics` + `metrics-exporter-prometheus` を採用（既存構造との統合容易性）

---

## アーキテクチャパターン評価

| オプション | 説明 | 強み | リスク/制限 | 備考 |
|------------|------|------|-------------|------|
| A: 既存拡張 | handlers.rs に新規ハンドラ追加 | 開発速度、パターン継承 | ファイル肥大化（3500+ LOC） | 短期的には有効 |
| B: ハンドラ分割 | handlers/ ディレクトリに機能別分割 | 責務分離、保守性向上 | 初期工数増、リファクタリング | 中長期的に推奨 |
| C: ハイブリッド | 既存拡張 + 段階的分割 | バランス、段階的改善 | 一時的な複雑性 | **採用** |

**選択**: Option C（ハイブリッドアプローチ）

---

## 設計決定

### 決定 1: リスクファクター識別子の設計

- **コンテキスト**: Req 1.3 のリスクファクター毎集計の実現
- **検討した代替案**:
  1. `String` 識別子 — 柔軟だが型安全性なし
  2. `enum RiskFactorId` — 型安全、パターンマッチ可能
  3. `trait RiskFactor` — 拡張性高いが複雑
- **選択したアプローチ**: `enum RiskFactorId` with variants
  ```rust
  pub enum RiskFactorId {
      Underlying(String),
      Curve(String),
      VolSurface(String),
  }
  ```
- **理由**: 型安全性と Enzyme 互換性（static dispatch）のバランス
- **トレードオフ**: 新規リスクファクタータイプ追加時は enum 変更が必要
- **フォローアップ**: `Display` trait 実装でログ出力改善

### 決定 2: バケット DV01 計算方式

- **コンテキスト**: Req 2 のテナー毎感応度計算
- **検討した代替案**:
  1. 既存 `IrsGreeksCalculator` 拡張 — 最小変更
  2. 新規 `BucketDv01Calculator` — 責務分離
  3. `GreeksAggregator` 拡張 — ポートフォリオレベル統合
- **選択したアプローチ**: 既存 `IrsGreeksCalculator` に `calculate_bucket_dv01` メソッド追加
- **理由**: テナー毎 Delta 計算は既に `IrsGreeksCalculator` で実装済み、最小の変更で実現可能
- **トレードオフ**: `IrsGreeksCalculator` の責務がやや拡大
- **フォローアップ**: Key Rate Duration は別メソッドとして分離

### 決定 3: 可視化エンドポイント設計

- **コンテキスト**: Req 5 のヒートマップ・時系列チャート API
- **検討した代替案**:
  1. 単一エンドポイント + クエリパラメータ — シンプル
  2. 個別エンドポイント — RESTful、キャッシュ可能
  3. GraphQL — 柔軟だが導入コスト高
- **選択したアプローチ**: 個別 REST エンドポイント
  - `/api/greeks/heatmap`
  - `/api/greeks/timeseries`
- **理由**: 既存 REST パターンとの一貫性、CDN キャッシュ可能
- **トレードオフ**: エンドポイント数増加
- **フォローアップ**: レスポンスフォーマットは D3.js 互換 JSON

### 決定 4: 非同期ジョブ API 設計

- **コンテキスト**: Req 7.6 の長時間計算対応
- **検討した代替案**:
  1. 同期レスポンス + タイムアウト — シンプルだが UX 劣化
  2. WebSocket 進捗 — リアルタイムだが複雑
  3. ポーリング型ジョブ API — REST 互換、実装容易
- **選択したアプローチ**: ポーリング型ジョブ API
  - POST → `{ job_id: "..." }` 即座返却
  - GET `/api/v1/jobs/{id}` → 進捗/結果
- **理由**: REST クライアント互換性、シンプルな実装
- **トレードオフ**: ポーリングによるレイテンシ
- **フォローアップ**: WebSocket 通知との併用を検討

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

- [utoipa - OpenAPI documentation for Rust](https://github.com/juhaku/utoipa) — Axum 統合、Swagger UI
- [metrics-rs](https://github.com/metrics-rs/metrics) — Prometheus エクスポーター
- [Enzyme AD](https://enzyme.mit.edu/) — LLVM AD プラグイン
- プロジェクト内部:
  - `crates/pricer_pricing/src/greeks/` — Greeks 基盤
  - `crates/pricer_pricing/src/irs_greeks/` — IRS Greeks ワークフロー
  - `demo/gui/src/web/` — WebApp ハンドラ
