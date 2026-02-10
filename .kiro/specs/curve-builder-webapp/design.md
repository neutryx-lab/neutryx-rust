# Design Document: curve-builder-webapp

## Overview

**Purpose**: Demo WebAppのCurve Build画面を精緻化し、Index別Instrument管理、Builderモデル選択、Parameterカーブ表示機能を提供する。

**Users**: 定量アナリスト、トレーダーがカーブ構築・分析ワークフローで使用。

**Impact**: 既存の`#irs-bootstrap-view`からIRS評価機能を分離・削除し、カーブ構築に特化した画面に再構成。

### Goals

- Index別Instrumentリストのファイルベース管理と動的読み込み
- 4種類の補間手法（Linear, LogLinear, CubicSpline, Monotonic）選択
- Discount Factor, Zero Rate, Forward Rateの3モード表示切替
- IRS評価機能のCurve Build画面からの完全削除

## Architecture

### Existing Architecture Analysis

**現行パターン**: A-I-P-S単方向データフロー、`pricer_models::market::calibration::bootstrapping` にカーブ構築ロジック集約、Demo層は `demo/gui/src/web/` でaxum Routerベースの REST API提供

**維持する制約**: Pricer層はService/Adapter層に依存しない、Demo層はPricer層の計算ロジックを呼び出すのみ、静的ディスパッチ（enum）によるEnzyme最適化

### Architecture Pattern & Boundary Map

Frontend (Curve Builder UI + Chart.js + Data Table) → Backend (axum Router + curve_handlers.rs) → PricerModels (SequentialBootstrapper + YieldCurve + Interpolators) → DataFiles (curves/*.json)

**Architecture Integration**: ハイブリッド拡張（新規curve_handlers.rs + 既存handlers.rs維持）、CurveBuilder APIは`/api/curves/*`に分離、既存axum Router + camelCase JSON + Arc<AppState>パターン踏襲

## System Flows

### カーブ構築フロー

User selects Index (USD-SOFR) → UI requests GET /instruments/{index} → API returns InstrumentList → User edits rates → User selects Builder settings → User clicks Build Curve → UI posts to /build → API calls SequentialBootstrapper → API stores in CurveCache → API returns BuildResponse (curveId, pillars, DFs) → UI renders curve chart

User changes Parameter display (ZeroRate) → UI requests GET /{curveId}/parameters?type=zero_rate → API retrieves from Cache → API computes parameters → UI updates chart

## Requirements Traceability

| Requirement | Summary | Components |
|-------------|---------|------------|
| 1.1-1.5 | Index別Instrument管理 | CurveDataLoader, InstrumentListResponse |
| 2.1-2.6 | レート入力UI | RateInputTable (UI), RateImportExport |
| 3.1-3.5 | Builderモデル選択 | BuilderConfigPanel (UI), BuilderListResponse |
| 4.1-4.5 | カーブ構築実行 | CurveBuildHandler, CurveBuildRequest/Response |
| 5.1-5.6 | Parameterカーブ表示 | ParameterDisplayPanel (UI), ParameterResponse |
| 6.1-6.4 | IRS機能削除 | (削除対象) |
| 7.1-7.6 | API設計 | curve_handlers.rs |

## Components and Interfaces

### Summary

| Component | Domain/Layer | Intent | Req Coverage | Key Dependencies |
|-----------|--------------|--------|--------------|------------------|
| CurveDataLoader | Backend/Data | Index別Instrumentファイル読み込み | 1.1-1.5 | std::fs (P0) |
| CurveBuildHandler | Backend/API | カーブ構築API | 4.1-4.5, 7.2 | SequentialBootstrapper (P0) |
| CurveParameterHandler | Backend/API | Parameter取得API | 5.1-5.6, 7.3 | YieldCurve (P0), CurveCache (P1) |
| BuilderListHandler | Backend/API | Builder一覧取得 | 3.1-3.5, 7.4 | - |
| InstrumentListHandler | Backend/API | Instrument一覧取得 | 1.1-1.5, 7.1 | CurveDataLoader (P0) |
| RateInputTable | Frontend/UI | レート編集テーブル | 2.1-2.6 | - |
| ParameterDisplayPanel | Frontend/UI | Parameter表示切替 | 5.1-5.6 | Chart.js (P0) |
| BuilderConfigPanel | Frontend/UI | Builder設定UI | 3.1-3.5 | LocalStorage (P1) |

### Backend / API Layer

#### CurveDataLoader

**Intent**: `demo/data/input/curves/`からIndex別Instrumentリストを読み込む

**Requirements**: 1.1, 1.2, 1.3, 1.4, 1.5

**Responsibilities**: ファイルシステムからJSONファイルを読み込み、InstrumentListに変換。ファイル不在時はデフォルトInstrumentリストにフォールバック。

```rust
pub struct CurveDataLoader {
    base_path: PathBuf,
}

impl CurveDataLoader {
    pub fn load_instruments(&self, index: &str) -> Result<InstrumentList, CurveDataError>;
    pub fn available_indices(&self) -> Vec<String>;
}
```

#### CurveBuildHandler

**Intent**: POSTリクエストを受けてカーブ構築を実行

**Requirements**: 4.1, 4.2, 4.3, 4.4, 4.5, 7.2

**Request**:
```rust
pub struct CurveBuildRequest {
    pub index: String,
    pub instruments: Vec<InstrumentInput>,
    pub interpolation: InterpolationMethod,
    pub bootstrap_method: BootstrapMethod,
    pub tolerance: f64,
    pub max_iterations: usize,
}
```

**Response**:
```rust
pub struct CurveBuildResponse {
    pub curve_id: String,
    pub status: BuildStatus,
    pub pillars: Vec<f64>,
    pub discount_factors: Vec<f64>,
    pub zero_rates: Vec<f64>,
    pub processing_time_ms: f64,
    pub instrument_count: usize,
}
```

#### CurveParameterHandler

**Intent**: 構築済みカーブのParameterデータを取得

**Requirements**: 5.1, 5.2, 5.3, 5.5, 5.6, 7.3

**API**: `GET /api/curves/{curveId}/parameters?type={discount_factor|zero_rate|forward_rate}&start_year={n}&end_year={m}&grid_interval={i}`

**Response**:
```rust
pub struct ParameterResponse {
    pub curve_id: String,
    pub parameter_type: ParameterType,
    pub data: Vec<ParameterPoint>,
}
```

### Frontend / UI Layer

#### RateInputTable

**Intent**: Instrumentレートの編集可能テーブル

**State**:
```typescript
interface RateInputState {
  index: string;
  instruments: InstrumentRow[];
  originalValues: Map<string, number>;
  hasChanges: boolean;
}
```

**Implementation**: Index選択時にAPIから初期データロード、数値入力時に範囲チェック（-10% ～ +50%）、変更されたセルを視覚的にハイライト

#### ParameterDisplayPanel

**Intent**: DF/ZeroRate/ForwardRateのタブ切替表示

**State**:
```typescript
interface ParameterDisplayState {
  curveId: string | null;
  activeTab: 'discount_factor' | 'zero_rate' | 'forward_rate';
  chartData: ParameterPoint[];
  tableData: ParameterPoint[];
  tenorRange: { start: number; end: number; interval: number };
}
```

#### BuilderConfigPanel

**Intent**: 補間手法・ブートストラップ設定のUI

**State**:
```typescript
interface BuilderConfigState {
  interpolation: InterpolationMethod;
  bootstrapMethod: BootstrapMethod;
  tolerance: number;
  maxIterations: number;
  presets: BuilderPreset[];
}
```

**Implementation**: プリセットはLocalStorageに保存、tolerance範囲 1e-12 ～ 1e-6、maxIterations範囲 10 ～ 1000

## Data Models

**Aggregates**: `BuiltCurve` (curveId, pillars, discountFactors, metadata), `InstrumentList` (Index別Instrumentコレクション)

**Value Objects**: `InterpolationMethod`, `BootstrapMethod`, `ParameterType`

**Index別Instrumentファイル構造** (`demo/data/input/curves/{index}.json`):
```json
{
  "index": "usd-sofr",
  "currency": "USD",
  "reference_date": "2026-01-23",
  "instruments": [
    {"type": "deposit", "tenor": "1M", "tenor_years": 0.0833, "rate": 0.0525, "frequency": "annual"}
  ]
}
```

## Error Handling

**User Errors (4xx)**: 400 Bad Request (不正なリクエスト形式、バリデーションエラー), 404 Not Found (存在しないcurveId、未サポートIndex)

**System Errors (5xx)**: 500 Internal Server Error (カーブ構築失敗、ファイル読み込みエラー)

**Business Logic Errors (422)**: 収束エラー (ブートストラップが収束しない), 不正レート (負のレート、異常値検出)

**Error Response Format** (RFC 7807準拠):
```rust
pub struct ProblemDetails {
    pub r#type: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
    pub instance: Option<String>,
}
```

## Testing Strategy

**Unit Tests**: CurveDataLoader::load_instruments (ファイル読み込み、フォールバック), CurveBuildRequest バリデーション, InterpolationMethod シリアライズ/デシリアライズ

**Integration Tests**: POST /api/curves/build (正常系、エラー系), GET /api/curves/{curveId}/parameters (Parameter取得), Instrumentファイル読み込み → カーブ構築 → Parameter取得のE2Eフロー

**E2E/UI Tests**: Index選択 → レート編集 → Build Curve → チャート表示, Parameter表示モード切替, JSON インポート/エクスポート
