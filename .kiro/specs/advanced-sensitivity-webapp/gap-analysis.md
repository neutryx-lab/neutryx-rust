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

| モジュール | パス | 状態 | 要件カバレッジ |
|------------|------|------|----------------|
| `greeks/` | `greeks/result.rs`, `greeks/config.rs` | ✅ 完全 | Req 1: `GreeksResult<T>` に Delta, Gamma, Vega, Theta, Rho, Vanna, Volga 定義済み |
| `irs_greeks/` | `irs_greeks/calculator.rs`, `benchmark.rs`, `lazy_evaluator.rs` | ✅ 完全 | Req 5: IRS Greeks ワークフロー、AAD vs Bump 比較、レイジー評価実装済み |
| `graph/` | `graph/types.rs`, `graph/extractor.rs` | ✅ 完全 | Req 6: D3.js 互換計算グラフ |
| `pool/` | `pool/mod.rs` | ✅ 完全 | Req 4: `ThreadLocalWorkspacePool` 実装済み |
| `checkpoint/` | `checkpoint/` | ✅ 完全 | Req 4: メモリチェックポイント機構 |

#### Risk Layer（pricer_risk）

| モジュール | パス | 状態 | 要件カバレッジ |
|------------|------|------|----------------|
| `scenarios/` | `scenarios/presets.rs`, `shifts.rs`, `engine.rs` | ✅ 完全 | Req 7: `PresetScenario`, `RiskFactorShift`, `ScenarioEngine` |
| `scenarios/aggregator.rs` | | ✅ 完全 | Req 1: `GreeksAggregator` でポートフォリオ集計 |

#### Demo Layer（demo/gui）

| モジュール | パス | 状態 | 要件カバレッジ |
|------------|------|------|----------------|
| `web/handlers.rs` | 3500+ LOC | ✅ 部分的 | Req 5, 8: `risk_bump`, `risk_aad`, `risk_compare`, `get_graph`, `get_speed_comparison` |
| `web/websocket.rs` | 1500+ LOC | ✅ 完全 | Req 6: リアルタイム更新、グラフ購読、ベンチマーク配信 |
| `visualisation.rs` | | ✅ 完全 | Req 5: AAD vs Bump 比較チャート |

### 1.2 アーキテクチャパターン

```text
既存パターン:
- 3-Stage Rocket: Definition (L2) → Linking (PricingContext) → Execution
- Feature Flag: `l1l2-integration`, `enzyme-ad`, `serde`
- Static Dispatch: Enum-based (Enzyme 最適化)
- REST: Axum handlers with JSON responses
- WebSocket: tokio-tungstenite + broadcast channel
```

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
| リスクファクター識別子 | 未実装 | **Missing**: `RiskFactorId` 型が必要 |
| AAD vs Bump 精度比較 | `risk_compare` で実装済み | 拡張のみ |

**ギャップタグ**: Missing

**実装アプローチ**:
- `GreeksResultByFactor<T>` 新規構造体: `HashMap<RiskFactorId, GreeksResult<T>>`
- `RiskFactorId` enum: `UnderlyingId(String)`, `CurveId(String)`, `VolSurfaceId(String)`

### Requirement 2: バケット感応度と Key Rate Duration

| 技術要件 | 現状 | ギャップ |
|----------|------|----------|
| テナーポイント毎 DV01 | `IrsGreeksCalculator` でテナー毎 Delta | 集計構造なし |
| Key Rate Duration | 未実装 | **Missing**: KRD 計算ロジック |
| パラレル/バタフライシフト | `PresetScenario` に存在 | 統合のみ |

**ギャップタグ**: Missing

**実装アプローチ**:
- `BucketDv01Result` 新規構造体: テナー → DV01 マッピング
- `KeyRateDurationCalculator`: 既存 `IrsGreeksCalculator` 拡張

### Requirement 4: パフォーマンス最適化

| 技術要件 | 現状 | ギャップ |
|----------|------|----------|
| Rayon 並列処理 | `parallel/` モジュール実装済み | 統合検証のみ |
| AAD 5倍速度目標 | `BenchmarkRunner` 存在 | **Research Needed**: 実測検証 |
| ThreadLocalWorkspacePool | 実装済み | なし |
| Criterion ベンチマーク | 構造あり | LLVM 18 環境依存 |
| チェックポイント機構 | `checkpoint/` 実装済み | 閾値トリガー未実装 |

**ギャップタグ**: Research Needed, Constraint

### Requirement 5: IRS Greeks ワークフロー統合

| 技術要件 | 現状 | ギャップ |
|----------|------|----------|
| IRS Greeks 統合 | `irs_greeks/` 完全実装 | Web UI 連携のみ |
| Bump vs AAD 両方実行 | `risk_compare` 実装済み | なし |
| 結果差分表示 | 部分的（DV01 比較） | **Missing**: 個別 Greeks 差分 |
| パフォーマンス可視化 | `get_speed_comparison` 実装済み | 拡張のみ |
| `/api/greeks/compare` | 類似機能あり（`/api/risk/compare`） | エンドポイント名変更 |

**ギャップタグ**: Missing (差分表示)

### Requirement 6: Greeks 可視化機能

| 技術要件 | 現状 | ギャップ |
|----------|------|----------|
| ヒートマップ（テナー × ストライク） | 未実装 | **Missing**: エンドポイント + D3.js |
| 時系列チャート | 未実装 | **Missing**: エンドポイント + D3.js |
| `/api/greeks/heatmap` | 未実装 | **Missing** |
| `/api/greeks/timeseries` | 未実装 | **Missing** |
| WebSocket Greeks 更新 | グラフ更新のみ | 拡張必要 |
| 計算グラフ連携 | `get_graph` 実装済み | ハイライト機能追加 |

**ギャップタグ**: Missing

### Requirement 7: シナリオ分析 UI

| 技術要件 | 現状 | ギャップ |
|----------|------|----------|
| スライダー UI | 未実装（バックエンドのみ） | **Missing**: フロントエンド |
| PresetScenario 選択 | `PresetScenario` 実装済み | Web UI 連携 |
| カスタムシナリオ保存 | 未実装 | **Missing**: 永続化 |
| シナリオ PnL 比較 | `ScenarioEngine` 実装済み | 可視化追加 |
| プログレスインジケータ | 未実装 | **Missing**: WebSocket 進捗 |

**ギャップタグ**: Missing (UI)

### Requirement 8: API エンドポイント拡充

| 技術要件 | 現状 | ギャップ |
|----------|------|----------|
| `/api/v1/greeks/first-order` | 類似機能あり | **Missing**: 専用エンドポイント |
| `/api/v1/greeks/second-order` | 類似機能あり | **Missing**: 専用エンドポイント |
| `/api/v1/greeks/bucket-dv01` | 未実装 | **Missing** |
| `/api/v1/greeks/cross` | 未実装（Req 3 削除予定） | N/A |
| エラーハンドリング | 実装済み | なし |
| OpenAPI ドキュメント | 未実装 | **Missing**: `/api/docs` |
| 非同期ジョブ API | 未実装 | **Missing**: `/api/v1/jobs/{id}` |

**ギャップタグ**: Missing

### Requirement 9: メトリクスと監視

| 技術要件 | 現状 | ギャップ |
|----------|------|----------|
| メトリクス収集 | `AppState.metrics` 実装済み | 拡張のみ |
| Prometheus `/metrics` | 未実装 | **Missing** |
| エラー統計 | 部分的 | 拡張必要 |
| AAD vs Bump 比較グラフ | `visualisation.rs` 実装済み | なし |
| レスポンスタイム警告 | 未実装 | **Missing**: ログ出力 |

**ギャップタグ**: Missing

---

## 3. 実装アプローチ選択肢

### Option A: 既存コンポーネント拡張

**対象要件**: Req 1, 4, 5, 9

**拡張対象**:
| ファイル | 変更内容 |
|----------|----------|
| `greeks/result.rs` | `GreeksResultByFactor<T>` 追加 |
| `handlers.rs` | 新規エンドポイント追加 |
| `websocket.rs` | Greeks 更新イベント追加 |
| `irs_greeks/calculator.rs` | バケット DV01 メソッド追加 |

**トレードオフ**:
- ✅ 既存パターン活用、開発速度向上
- ✅ テストカバレッジ継承
- ❌ `handlers.rs` の肥大化リスク（現在 3500+ LOC）
- ❌ 単一責任原則違反の可能性

### Option B: 新規コンポーネント作成

**対象要件**: Req 2, 6, 7, 8

**新規ファイル**:
| ファイル | 責務 |
|----------|------|
| `greeks/bucket_dv01.rs` | バケット DV01/KRD 計算 |
| `handlers/greeks_handlers.rs` | Greeks 専用エンドポイント |
| `handlers/scenario_handlers.rs` | シナリオ分析エンドポイント |
| `metrics/prometheus.rs` | Prometheus エクスポーター |

**トレードオフ**:
- ✅ 責務分離、テスト容易性向上
- ✅ `handlers.rs` の分割
- ❌ 新規ファイル増加
- ❌ 統合テスト複雑化

### Option C: ハイブリッドアプローチ（推奨）

**戦略**:
1. **フェーズ 1**: 既存拡張（Req 1, 4, 5, 9）- 即時価値提供
2. **フェーズ 2**: 新規作成（Req 2, 6, 7, 8）- アーキテクチャ整備

**リスク軽減**:
- `handlers.rs` を `handlers/` ディレクトリに分割（段階的）
- Feature flag で新機能を段階的有効化

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

| 項目 | 調査内容 |
|------|----------|
| AAD 5倍速度目標 | 実環境での Enzyme vs Bump ベンチマーク |
| OpenAPI 生成 | `utoipa` または `aide` crate 評価 |
| Prometheus 統合 | `prometheus` crate vs `metrics-exporter-prometheus` |

### 制約事項

| 制約 | 影響 |
|------|------|
| `enzyme-ad` feature | pricer_pricing でのみ有効、Web は fallback |
| `handlers.rs` サイズ | 分割検討必須（現在 3500+ LOC） |
| LLVM 18 依存 | CI/CD での Enzyme ベンチマーク制限 |

---

## 6. 次のステップ

要件の修正が完了した後：

```
/kiro:spec-design advanced-sensitivity-webapp -y
```

クロス Greeks（Req 3）を削除する場合は、先に `requirements.md` を更新してください。
