# Technical Design Document

## Overview

**Purpose**: 本機能は、Neutryx デリバティブ価格計算ライブラリにおける Sensitivity（Greeks）計算の精度向上と WebApp（FrictionalBank）の機能拡張を提供する。

**Users**: クオンツアナリスト、リスクマネージャー、トレーダー、DevOps エンジニアが、リスクファクター毎の Greeks 計算、バケット DV01、インタラクティブなシナリオ分析、Bump vs AAD 比較可視化を利用する。

**Impact**: 既存の `GreeksResult<T>` および `IrsGreeksCalculator` を拡張し、WebApp に新規可視化エンドポイントと監視機能を追加する。

### Goals

- リスクファクター毎の一次・二次 Greeks 計算と集計
- バケット DV01 および Key Rate Duration の計算
- Bump vs AAD 比較結果のインタラクティブ可視化
- Greeks ヒートマップ・時系列チャートの提供
- シナリオ分析 UI とプリセットシナリオ連携
- Prometheus 形式メトリクスによる運用監視

---

## Architecture

### Existing Architecture Analysis

本プロジェクトは A-I-P-S アーキテクチャに従い、Pricer Layer（L1-L4）と Demo Layer（FrictionalBank WebApp）で構成される。

**既存パターン**:
- 3-Stage Rocket: Definition (L2) → Linking (PricingContext) → Execution
- Static Dispatch: enum-based（Enzyme 最適化）
- Feature Flags: `enzyme-ad`, `l1l2-integration`, `serde`

**維持すべき統合ポイント**:
- `GreeksResult<T>` の AD 互換性（ジェネリック `T: Float`）
- `IrsGreeksCalculator` のテナー毎計算パターン
- `handlers.rs` の Axum ハンドラパターン
- WebSocket の broadcast channel パターン

### Architecture Pattern & Boundary Map

**Architecture Integration**:
- **選択パターン**: ハイブリッドアプローチ（既存拡張 + 段階的分割）
- **ドメイン境界**: Pricer（計算）→ Risk（集計）→ WebApp（API/UI）
- **維持するパターン**: Static dispatch、3-Stage Rocket、Feature flags
- **新規コンポーネント根拠**: `RiskFactorId` は Greeks 集計の識別に必須、`BucketDv01` は金利リスク詳細化に必須
- **Steering 準拠**: A-I-P-S 依存ルール維持、Pricer は Service に依存しない

---

## Requirements Traceability

| Requirement | Summary | Components | Interfaces | Flows |
|-------------|---------|------------|------------|-------|
| 1.1, 1.2 | リスクファクター毎 Greeks | `RiskFactorId`, `GreeksResultByFactor` | `GreeksCalculator` trait | Greeks 計算フロー |
| 1.3 | リスクファクター集計 | `GreeksAggregator` | `aggregate_by_factor()` | - |
| 1.4, 1.5 | AAD vs Bump 比較 | `IrsGreeksCalculator` | `compare()` | 比較フロー |
| 2.1, 2.2 | バケット DV01/KRD | `BucketDv01Calculator` | `calculate_bucket_dv01()` | - |
| 2.3, 2.4 | カーブシフト | `CurveShifter` | `apply_parallel_shift()` | - |
| 3.1-3.5 | パフォーマンス最適化 | `ThreadLocalWorkspacePool` | 既存 | - |
| 4.1-4.6 | IRS Greeks 統合 | `GreeksHandlers` | `/api/greeks/compare` | 比較フロー |
| 5.1-5.5 | Greeks 可視化 | `HeatmapHandler`, `TimeseriesHandler` | `/api/greeks/heatmap` | WebSocket |
| 6.1-6.5 | シナリオ UI | `ScenarioHandlers` | `/api/scenarios/*` | シナリオフロー |
| 7.1-7.6 | API エンドポイント | 各 Handler | `/api/v1/*` | - |
| 8.1-8.4 | メトリクス/監視 | `MetricsHandler` | `/metrics` | - |

---

## Components and Interfaces

| Component | Domain/Layer | Intent | Req Coverage | Key Dependencies | Contracts |
|-----------|--------------|--------|--------------|------------------|-----------|
| `RiskFactorId` | Pricer/greeks | リスクファクター識別 | 1.3 | - | State |
| `GreeksResultByFactor<T>` | Pricer/greeks | ファクター毎 Greeks 集計 | 1.1, 1.2, 1.3 | `RiskFactorId`, `GreeksResult` (P0) | State |
| `BucketDv01Calculator` | Pricer/irs_greeks | バケット DV01/KRD 計算 | 2.1, 2.2, 2.3, 2.4 | `IrsGreeksCalculator` (P0) | Service |
| `GreeksHandlers` | WebApp/handlers | Greeks API エンドポイント | 4, 5, 7 | `IrsGreeksCalculator` (P0), `GreeksResultByFactor` (P0) | API |
| `ScenarioHandlers` | WebApp/handlers | シナリオ分析 API | 6 | `ScenarioEngine` (P0), `PresetScenario` (P1) | API |
| `JobManager` | WebApp/jobs | 非同期ジョブ管理 | 7.6 | tokio (P0) | Service |
| `MetricsHandler` | WebApp/metrics | Prometheus メトリクス出力 | 8.2 | `metrics` crate (P0) | API |

### Pricer Layer

#### RiskFactorId

**Intent**: リスクファクターを一意に識別する enum

**Responsibilities**: 原資産、カーブ、ボラティリティサーフェスの識別、`Display`, `Hash`, `Eq` trait 実装、Serde 互換（JSON シリアライズ）

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum RiskFactorId {
    Underlying(String),
    Curve(String),
    VolSurface(String),
}
```

#### GreeksResultByFactor

**Intent**: リスクファクター毎の Greeks 計算結果を保持

**Responsibilities**: `HashMap<RiskFactorId, GreeksResult<T>>` をラップ、総合 Greeks（全ファクター合計）の計算メソッド提供、AD 互換性維持（ジェネリック `T: Float`）

```rust
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct GreeksResultByFactor<T: Float> {
    pub by_factor: HashMap<RiskFactorId, GreeksResult<T>>,
    pub mode: GreeksMode,
    pub computation_time_ns: u64,
}

impl<T: Float> GreeksResultByFactor<T> {
    pub fn total(&self) -> GreeksResult<T>;
    pub fn get(&self, factor: &RiskFactorId) -> Option<&GreeksResult<T>>;
    pub fn len(&self) -> usize;
}
```

#### BucketDv01Calculator

**Intent**: テナーポイント毎のバケット DV01 および Key Rate Duration を計算

**Responsibilities**: 標準テナーポイント（1M, 3M, 6M, 1Y, 2Y, 5Y, 10Y, 20Y, 30Y）に対する感応度計算、バケット合計と総 DV01 の整合性検証、パラレルシフト、バタフライシフトのサポート

```rust
pub struct BucketDv01Calculator {
    config: BucketDv01Config,
}

impl BucketDv01Calculator {
    pub fn calculate_bucket_dv01<T: Float>(
        &self,
        swap: &InterestRateSwap<T>,
        curves: &CurveSet<T>,
        valuation_date: Date,
    ) -> Result<BucketDv01Result<T>, IrsGreeksError>;

    pub fn calculate_krd<T: Float>(
        &self,
        swap: &InterestRateSwap<T>,
        curves: &CurveSet<T>,
        valuation_date: Date,
        key_rates: &[Tenor],
    ) -> Result<KeyRateDurationResult<T>, IrsGreeksError>;
}
```

### WebApp Layer

#### GreeksHandlers

**Intent**: Greeks 計算 API エンドポイントを提供

**API Endpoints**:
- POST `/api/v1/greeks/first-order` → `GreeksResultByFactor`
- POST `/api/v1/greeks/second-order` → `GreeksResultByFactor`
- POST `/api/v1/greeks/bucket-dv01` → `BucketDv01Result`
- POST `/api/greeks/compare` → `CompareResponse`
- GET `/api/greeks/heatmap` → `HeatmapData`
- GET `/api/greeks/timeseries` → `TimeseriesData`

#### ScenarioHandlers

**API Endpoints**:
- GET `/api/scenarios/presets` → `Vec<PresetScenario>`
- POST `/api/scenarios/run` → `ScenarioResponse` or `JobId`
- GET `/api/v1/jobs/{id}` → `JobStatus`

#### JobManager

**Intent**: 非同期ジョブの管理と進捗追跡

```rust
#[derive(Debug, Clone, Serialize)]
pub enum JobStatus {
    Pending,
    Running { progress_percent: u8 },
    Completed { result: serde_json::Value },
    Failed { error: String },
}

pub struct JobManager {
    jobs: Arc<RwLock<HashMap<Uuid, JobEntry>>>,
}

impl JobManager {
    pub fn create_job(&self) -> Uuid;
    pub fn update_progress(&self, job_id: Uuid, percent: u8);
    pub fn complete_job(&self, job_id: Uuid, result: serde_json::Value);
    pub fn get_status(&self, job_id: Uuid) -> Option<JobStatus>;
}
```

#### MetricsHandler

**Intent**: Prometheus 形式メトリクスを出力

**Responsibilities**: Counter: API リクエスト数、エラー数、Histogram: レスポンスタイム、Gauge: アクティブ接続数、メモリ使用量

---

## Data Models

### API Request/Response Schemas

```rust
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct GreeksRequest {
    pub swap: IrsDefinition,
    pub valuation_date: String, // ISO 8601
    pub mode: Option<GreeksMode>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct CompareResponse {
    pub bump_result: GreeksResultByFactor<f64>,
    pub aad_result: GreeksResultByFactor<f64>,
    pub diff: GreeksDiff,
    pub timing: TimingComparison,
}
```

---

## Error Handling

**User Errors (4xx)**: 不正なリクエストパラメータ → 詳細なバリデーションエラー
**System Errors (5xx)**: 計算エラー、内部障害 → 構造化エラーレスポンス
**Business Logic Errors (422)**: NaN/Inf 発生、整合性検証失敗 → 専用エラーコード

```rust
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}
```

**Monitoring**: エラー発生時は `tracing::error!` でログ出力、Prometheus カウンター `api_errors_total{code="..."}` でエラー集計

---

## Testing Strategy

### Unit Tests
- `RiskFactorId` の `Display`, `Hash`, `Eq` 実装
- `GreeksResultByFactor::total()` の集計正確性
- `BucketDv01Calculator` のバケット合計整合性
- `JobManager` の状態遷移

### Integration Tests
- `/api/greeks/compare` エンドポイントの Bump vs AAD 一致検証
- `/api/greeks/heatmap` のレスポンスフォーマット検証
- WebSocket 進捗通知の配信検証
- `/metrics` エンドポイントの Prometheus 形式検証

### Performance Tests
- 1000 トレードポートフォリオの Greeks 計算時間
- AAD vs Bump の速度比（5倍目標）
- 並列計算時のメモリ使用量
