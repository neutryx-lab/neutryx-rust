# ギャップ分析レポート

## 分析サマリ

**Feature**: `advanced-sensitivity-webapp`
**分析日**: 2026-01-15

### 主要な発見事項

- **既存基盤**: Greeks 計算基盤（`GreeksResult<T>`）、IRS Greeks ワークフロー、Bump vs AAD 比較機能は実装済み
- **WebApp 基盤**: REST API（`risk_bump`, `risk_aad`, `risk_compare`）、WebSocket、D3.js 計算グラフは実装済み
- **主要ギャップ**: リスクファクター毎の Greeks 集計、バケット DV01、可視化エンドポイント（ヒートマップ、時系列）、Prometheus メトリクス
- **リスク**: 中程度 - 既存パターンの拡張が主であり、新規アーキテクチャ変更は最小限

---

## 1. 現状調査

### 1.1 既存コンポーネント

#### Pricer Layer（pricer_pricing）

| モジュール | 状態 | 要件カバレッジ |
|------------|------|----------------|
| `greeks/` | Complete | Req 1: `GreeksResult<T>` に Delta, Gamma, Vega, Theta, Rho, Vanna, Volga 定義済み |
| `irs_greeks/` | Complete | Req 5: IRS Greeks ワークフロー、AAD vs Bump 比較、レイジー評価実装済み |
| `graph/` | Complete | Req 6: D3.js 互換計算グラフ |
| `pool/` | Complete | Req 4: `ThreadLocalWorkspacePool` 実装済み |

#### Risk Layer（pricer_risk）

| モジュール | 状態 | 要件カバレッジ |
|------------|------|----------------|
| `scenarios/` | Complete | Req 7: `PresetScenario`, `RiskFactorShift`, `ScenarioEngine` |
| `scenarios/aggregator.rs` | Complete | Req 1: `GreeksAggregator` でポートフォリオ集計 |

#### Demo Layer（demo/gui）

| モジュール | 状態 | 要件カバレッジ |
|------------|------|----------------|
| `web/handlers.rs` | Partial | Req 5, 8: `risk_bump`, `risk_aad`, `risk_compare`, `get_graph`, `get_speed_comparison` |
| `web/websocket.rs` | Complete | Req 6: リアルタイム更新、グラフ購読、ベンチマーク配信 |

### 1.2 アーキテクチャパターン

既存パターン:
- 3-Stage Rocket: Definition (L2) → Linking (PricingContext) → Execution
- Feature Flag: `l1l2-integration`, `enzyme-ad`, `serde`
- Static Dispatch: Enum-based (Enzyme 最適化)
- REST: Axum handlers with JSON responses
- WebSocket: tokio-tungstenite + broadcast channel

### 1.3 統合ポイント

| 統合ポイント | 現状 | 拡張方針 |
|--------------|------|----------|
| `GreeksResult<T>` | 単一計算結果 | リスクファクター ID マッピング追加 |
| `handlers.rs` | 基本エンドポイント | 一次/二次 Greeks、バケット DV01 エンドポイント追加 |
| `websocket.rs` | グラフ更新、ベンチマーク | Greeks 更新イベント追加 |
| `PresetScenario` | バックエンドのみ | Web UI 連携 |

---

## 2. 要件実現可能性分析

### Requirement 1: リスクファクター毎の Greeks 計算

| 技術要件 | 現状 | ギャップ |
|----------|------|----------|
| 一次 Greeks (Delta, Vega, Rho, Theta) | `GreeksResult<T>` に定義済み | リスクファクター ID なし |
| 二次 Greeks (Gamma, Vanna, Volga) | `GreeksResult<T>` に定義済み | リスクファクター ID なし |
| リスクファクター識別子 | 未実装 | Missing: `RiskFactorId` 型が必要 |
| AAD vs Bump 精度比較 | `risk_compare` で実装済み | 拡張のみ |

**ギャップタグ**: Missing

**実装アプローチ**:
- `GreeksResultByFactor<T>` 新規構造体: `HashMap<RiskFactorId, GreeksResult<T>>`
- `RiskFactorId` enum: `UnderlyingId(String)`, `CurveId(String)`, `VolSurfaceId(String)`

### Requirement 2: バケット感応度と Key Rate Duration

**ギャップタグ**: Missing

**実装アプローチ**:
- `BucketDv01Result` 新規構造体: テナー → DV01 マッピング
- `KeyRateDurationCalculator`: 既存 `IrsGreeksCalculator` 拡張

### Requirement 4: パフォーマンス最適化

| 技術要件 | 現状 | ギャップ |
|----------|------|----------|
| Rayon 並列処理 | `parallel/` モジュール実装済み | 統合検証のみ |
| AAD 5倍速度目標 | `BenchmarkRunner` 存在 | Research Needed: 実測検証 |
| ThreadLocalWorkspacePool | 実装済み | なし |
| チェックポイント機構 | `checkpoint/` 実装済み | 閾値トリガー未実装 |

**ギャップタグ**: Research Needed, Constraint

### Requirement 5: IRS Greeks ワークフロー統合

**ギャップタグ**: Missing (差分表示)

### Requirement 6: Greeks 可視化機能

| 技術要件 | 現状 | ギャップ |
|----------|------|----------|
| ヒートマップ（テナー × ストライク） | 未実装 | Missing: エンドポイント + D3.js |
| 時系列チャート | 未実装 | Missing: エンドポイント + D3.js |
| `/api/greeks/heatmap` | 未実装 | Missing |
| `/api/greeks/timeseries` | 未実装 | Missing |

**ギャップタグ**: Missing

### Requirement 7: シナリオ分析 UI

**ギャップタグ**: Missing (UI)

### Requirement 8: API エンドポイント拡充

**ギャップタグ**: Missing

### Requirement 9: メトリクスと監視

**ギャップタグ**: Missing

---

## 3. 実装アプローチ選択肢

### Option A: 既存コンポーネント拡張

**対象要件**: Req 1, 4, 5, 9

**トレードオフ**:
- 既存パターン活用、開発速度向上
- テストカバレッジ継承
- Rejected: `handlers.rs` の肥大化リスク（現在 3500+ LOC）

### Option B: 新規コンポーネント作成

**対象要件**: Req 2, 6, 7, 8

**トレードオフ**:
- 責務分離、テスト容易性向上
- Rejected: 新規ファイル増加、統合テスト複雑化

### Option C: ハイブリッドアプローチ（推奨）

**戦略**:
1. **フェーズ 1**: 既存拡張（Req 1, 4, 5, 9）- 即時価値提供
2. **フェーズ 2**: 新規作成（Req 2, 6, 7, 8）- アーキテクチャ整備

**選択**: Option C（ハイブリッドアプローチ）

---

## 4. 工数・リスク評価

| 要件 | 工数 | リスク | 根拠 |
|------|------|--------|------|
| Req 1: リスクファクター毎 Greeks | M (3-5日) | Low | 既存 `GreeksResult` 拡張 |
| Req 2: バケット DV01/KRD | M (3-5日) | Medium | 新規計算ロジック |
| Req 4: パフォーマンス最適化 | S (1-3日) | Low | 既存基盤活用 |
| Req 5: IRS Greeks 統合 | S (1-3日) | Low | 既存 `risk_compare` 拡張 |
| Req 6: Greeks 可視化 | L (5-7日) | Medium | 新規 D3.js 実装 |
| Req 7: シナリオ UI | L (5-7日) | Medium | 新規フロントエンド |
| Req 8: API エンドポイント | M (3-5日) | Low | 既存パターン |
| Req 9: メトリクス/監視 | S (1-3日) | Low | Prometheus crate 利用 |

**総工数**: L-XL (2-3週間)
**総合リスク**: Medium

---

## 5. 設計フェーズへの推奨事項

### 優先実装順序

1. **Req 1, 5** (High Priority): リスクファクター毎 Greeks + IRS 統合
2. **Req 8** (High Priority): API エンドポイント標準化
3. **Req 2, 4** (Medium Priority): バケット DV01 + パフォーマンス
4. **Req 6, 7** (Medium Priority): 可視化 + シナリオ UI
5. **Req 9** (Low Priority): メトリクス/監視

### Research Needed 項目

- AAD 5倍速度目標: 実環境での Enzyme vs Bump ベンチマーク
- OpenAPI 生成: `utoipa` または `aide` crate 評価
- Prometheus 統合: `prometheus` crate vs `metrics-exporter-prometheus`

### 制約事項

- `enzyme-ad` feature: pricer_pricing でのみ有効、Web は fallback
- `handlers.rs` サイズ: 分割検討必須（現在 3500+ LOC）
- LLVM 18 依存: CI/CD での Enzyme ベンチマーク制限
