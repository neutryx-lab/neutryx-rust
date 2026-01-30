# Research & Design Decisions

---
**Purpose**: service-gateway-services 機能のディスカバリー結果と設計判断を記録
---

## Summary
- **Feature**: `service-gateway-services`
- **Discovery Scope**: Extension（既存システムへの機能追加）
- **Key Findings**:
  - 既存 CurveService/PricingService パターンが確立済み、新サービスは同一パターンを踏襲可能
  - pricer_risk/pricer_models/pricer_pricing の API は十分に安定・文書化済み
  - Feature flags による選択的コンパイルは Cargo.toml の features セクションで実現可能

## Research Log

### 既存サービスパターン分析
- **Context**: 新サービス設計の参照パターンを特定
- **Sources Consulted**: `services/curve_service.rs`, `services/pricing_service.rs`, `rest/handlers/`, `rest/dto/`
- **Findings**:
  - Service は `pub struct XxxService;` で定義、全メソッドは静的（`&self` なし）
  - シグネチャ: `pub fn operation(request: &Request, state: &Arc<AppState>) -> Result<Response, ServerError>`
  - AppState 不要の場合は state 引数を省略可能（PricingService 参照）
  - Handler は thin wrapper として `State(state)` と `Json(request)` を受け取り Service に委譲
- **Implications**: 4つの新サービスすべてでこのパターンを踏襲

### pricer_risk API 調査
- **Context**: RiskService/PortfolioService が依存する API を確認
- **Sources Consulted**: `pricer_risk/src/lib.rs`, `engine/engine.rs`, `portfolio/mod.rs`, `scenarios/engine.rs`
- **Findings**:
  - `RiskEngine` - Facade パターン、`compute_greeks()`, `run_all_scenarios()` 提供
  - `Portfolio`, `PortfolioBuilder` - Builder パターンでポートフォリオ構築
  - `ScenarioEngine<T>` - シナリオ実行、`ScenarioPnL<T>` を返却
  - `GreeksConfig`, `GreeksMode` - Greeks 計算設定
  - エラー型: `RiskError`, `PortfolioError`, `GreeksError`
- **Implications**: 既存エラー型を `ServerError` に変換する `From` 実装が必要

### pricer_models Stochastic API 調査
- **Context**: ModelService が依存する確率モデル API を確認
- **Sources Consulted**: `pricer_models/src/stochastic/mod.rs`, `model_enum.rs`
- **Findings**:
  - `StochasticModelEnum<T>` - 静的ディスパッチ enum（GBM, Heston, HullWhite, CIR）
  - `ModelParams<T>` - パラメータ enum
  - Feature-gated: `equity` (GBM, Heston), `rates` (HullWhite, CIR), `exotic` (Correlated)
  - エラー型: `ModelError`, `HestonError`
- **Implications**: ModelService は pricer_models の feature に依存、キャッシュは `f64` 固定で実装

### キャッシュ実装パターン
- **Context**: 新キャッシュ（Portfolio, Model, VolSurface）の設計参考
- **Sources Consulted**: `state/cache.rs`
- **Findings**:
  - `LruCache` + `parking_lot::RwLock` の組み合わせ
  - UUID を key として使用
  - Entry 構造体でメタデータを保持（例: `CurveEntry`, `FxVolEntry`）
  - デフォルトキャパシティはコンストラクタで設定
- **Implications**: 同一パターンで `PortfolioCache`, `ModelCache`, `VolSurfaceCache` を実装

### Feature Flags 設計
- **Context**: 選択的コンパイルの実現方法
- **Sources Consulted**: `Cargo.toml` (workspace, service_gateway)
- **Findings**:
  - 既存 features: `rest`, `grpc`
  - pricer_models は `equity`, `rates`, `exotic`, `serde`, `global-bootstrap` を提供
  - pricer_risk は `enzyme-ad` を提供（nightly 専用）
- **Implications**: 新 features: `risk`, `models`, `volatility` を追加、pricer_* features との依存関係を設定

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Layered (採用) | Handler → Service → Pricer crate | 既存パターンと一致、テスト容易 | N/A | steering の A-I-P-S に準拠 |
| Direct API | Handler から pricer_* を直接呼び出し | コード削減 | ビジネスロジック分散、テスト困難 | 不採用 |

## Design Decisions

### Decision: Service 構造体パターン
- **Context**: 新サービスの構造をどう設計するか
- **Alternatives Considered**:
  1. インスタンスメソッド（`&self` 使用）
  2. 静的メソッド（関連関数のみ）
- **Selected Approach**: 静的メソッド
- **Rationale**: 既存 CurveService/PricingService と一致、状態は AppState で管理
- **Trade-offs**: サービスインスタンスの設定変更が困難（設計上問題なし）

### Decision: Feature Flag 粒度
- **Context**: どの粒度で feature を分けるか
- **Alternatives Considered**:
  1. 個別サービスごと（`risk-service`, `portfolio-service`, etc.）
  2. ドメインごと（`risk`, `models`, `volatility`）
- **Selected Approach**: ドメインごと
- **Rationale**: 依存関係が自然（RiskService と PortfolioService は pricer_risk に依存）
- **Trade-offs**: より細かい制御は不可（ユースケース上問題なし）

### Decision: エラー型構造
- **Context**: ドメインエラーをどう構造化するか
- **Alternatives Considered**:
  1. 単一ファイル維持（error.rs を拡張）
  2. モジュール分割（error/ ディレクトリ）
- **Selected Approach**: 単一ファイル維持
- **Rationale**: 現時点でエラー variant 数は管理可能、過度な分割は避ける
- **Trade-offs**: 将来的に肥大化した場合は分割を検討

## Risks & Mitigations
- **Risk**: pricer_risk の enzyme-ad feature は nightly 専用 → `#[cfg(feature = "enzyme-ad")]` で条件付きコンパイル
- **Risk**: StochasticModelEnum<T> のジェネリックとキャッシュの互換性 → `f64` 固定でキャッシュ、型パラメータ不要
- **Risk**: Feature 依存の複雑化 → Cargo.toml で明示的な依存関係を記述

## References
- [Axum Router Documentation](https://docs.rs/axum/latest/axum/routing/struct.Router.html)
- [parking_lot RwLock](https://docs.rs/parking_lot/latest/parking_lot/type.RwLock.html)
- [lru crate](https://docs.rs/lru/latest/lru/)
