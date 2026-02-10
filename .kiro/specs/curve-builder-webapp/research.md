# Research & Design Decisions: curve-builder-webapp

## Summary

- **Feature**: `curve-builder-webapp`
- **Discovery Scope**: Extension
- **Key Findings**:
  1. `pricer_models::market` モジュールに完全なカーブ構築インフラが存在（SequentialBootstrapper, YieldCurve trait）
  2. 既存WebApp APIパターン（axum Router, camelCase JSON）を踏襲可能
  3. IRS機能は現在Curve Build画面に統合されており、分離削除が必要

## Research Log

### YieldCurve Trait インターフェース

**Findings**: `discount_factor(t)`, `zero_rate(t)`, `forward_rate(t)` 全て実装済み、`pillars()`, `pillar_values()` 取得可能

**Implications**: バックエンドでのParameterカーブ計算は追加実装不要、APIレスポンス型の設計のみ必要

### BootstrapInstrument 構造

**Findings**: `BootstrapInstrument<T>` enum (Ois, Irs, Fra, Future variants)、各variantに `maturity`, `rate`, `payment_frequency` フィールド、`Frequency` enum (Daily, Monthly, Quarterly, SemiAnnual, Annual)

**Implications**: JSONファイル形式はこの構造に対応する必要あり、シリアライズ用の型を別途定義

### 既存API Request/Response パターン

**Findings**: Request型 `#[serde(rename_all = "camelCase")]`, Response型 `#[serde(rename_all = "camelCase")]`, エラー型 `(StatusCode, Json<ErrorResponse>)`

**Implications**: 新型は既存パターンに完全準拠

### InterpolationMethod 選択肢

**Findings**: CurveBootstrapper (Linear, LogLinear, CubicSpline)、pricer_core (LinearInterpolator, LogLinearInterpolator, CubicSplineInterpolator, MonotonicInterpolator, HermiteInterpolator)、現在UIはLinear/LogLinearのみ公開

**Implications**: CubicSpline, Monotonicを追加、Hermiteは上級者向けオプションとして検討

### 既存WebApp構造

**Findings**: View ID `#irs-bootstrap-view` → `#curve-builder-view` にリネーム推奨、IRS関連セクション (`#irs-params-section`, `#pricing-result-card`, `#risk-result-card`)、Bootstrap処理 `handleBootstrap()` 関数、Chart.js `#curve-chart` canvas要素

**Implications**: IRS関連HTMLセクションの削除、Parameter表示タブの追加

## Architecture Pattern Evaluation

| Option | Strengths | Risks / Limitations |
|--------|-----------|---------------------|
| Option A: 既存拡張 | 開発速度、既存テスト活用 | コード複雑化 |
| Option B: 新規モジュール | 責務分離、影響範囲限定 | コード重複リスク |
| **Option C: ハイブリッド** | バランス、段階的実装 | 計画複雑性 |

**Selected**: Option C - ハイブリッド

## Design Decisions

### Decision: API構造

**Selected Approach**: Option 2 - 新規エンドポイント追加（並行運用）

**Rationale**: 後方互換性維持、既存クライアントへの影響なし

**Alternatives**: 既存エンドポイントを拡張 - 破壊的変更

### Decision: データファイル形式

**Selected Approach**: Option 2 - Index別ファイル (`{index}.json`)

**Rationale**: 管理容易性、個別更新可能、ファイルサイズ制御

**Alternatives**: 単一ファイルに全Index格納 - 管理困難

### Decision: Builderプリセット保存

**Selected Approach**: Option 1 - LocalStorage（クライアント側）

**Rationale**: シンプル、サーバー負荷なし、Demo用途に適合

**Alternatives**: サーバーAPI（永続化） - 複雑、Cookie - 容量制限

### Decision: Parameter表示モード

**Selected Approach**: Option 1 - 単一APIで全Parameter返却

**Rationale**: APIコール削減、クライアント側で切替可能

**Alternatives**: Parameter種別毎に個別API - APIコール増加

## Risks & Mitigations

- **IRS削除による他機能への影響** — Trade Pricing画面への移行パス確認、リダイレクト実装
- **Forward Rate計算精度** — 既存YieldCurve trait実装を使用、新規実装なし
- **フロントエンド複雑化** — 既存app.jsパターン踏襲、モジュール分割
- **APIエンドポイント増加** — OpenAPI仕様でドキュメント化、一貫した命名規則

## References

- [YieldCurve Trait](crates/pricer_models/src/market/curves/traits.rs) — カーブインターフェース定義
- [BootstrapInstrument](crates/pricer_models/src/market/calibration/bootstrapping/instrument.rs) — Instrument構造
- [pricer_types.rs](demo/gui/src/web/pricer_types.rs) — 既存API型パターン
- [A-I-P-S Architecture](/.kiro/steering/tech.md) — アーキテクチャ制約
