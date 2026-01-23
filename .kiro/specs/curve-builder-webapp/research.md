# Research & Design Decisions: curve-builder-webapp

---
**Purpose**: Discovery findings and design decisions for Curve Builder WebApp refinement
**Discovery Type**: Extension (既存システムの拡張)
---

## Summary

- **Feature**: `curve-builder-webapp`
- **Discovery Scope**: Extension
- **Key Findings**:
  1. `pricer_models::market` モジュールに完全なカーブ構築インフラが存在（SequentialBootstrapper, YieldCurve trait）
  2. 既存WebApp APIパターン（axum Router, camelCase JSON）を踏襲可能
  3. IRS機能は現在Curve Build画面に統合されており、分離削除が必要

## Research Log

### YieldCurve Trait インターフェース

- **Context**: Parameter表示（DF, ZeroRate, ForwardRate）のバックエンド対応確認
- **Sources Consulted**: `pricer_models::market::curves::traits.rs`
- **Findings**:
  - `discount_factor(t)`: DF取得（全カーブで実装済み）
  - `zero_rate(t)`: ゼロレート取得（デフォルト実装あり）
  - `forward_rate(t1, t2)`: フォワードレート取得（デフォルト実装あり）
  - `pillars()`: ピラーポイント取得（Option<&[T]>）
  - `pillar_values()`: ピラー値取得（Option<&[T]>）
- **Implications**: バックエンドでのParameterカーブ計算は追加実装不要。APIレスポンス型の設計のみ必要

### BootstrapInstrument 構造

- **Context**: Index別Instrumentデータファイル形式の設計
- **Sources Consulted**: `pricer_models::market::calibration::bootstrapping::instrument.rs`
- **Findings**:
  - `BootstrapInstrument<T>` enum: Ois, Irs, Fra, Future variants
  - 各variantに `maturity`, `rate`, `payment_frequency` フィールド
  - `Frequency` enum: Daily, Monthly, Quarterly, SemiAnnual, Annual
- **Implications**: JSONファイル形式はこの構造に対応する必要あり。シリアライズ用の型を別途定義

### 既存API Request/Response パターン

- **Context**: 新APIエンドポイントの型設計
- **Sources Consulted**: `demo/gui/src/web/pricer_types.rs`
- **Findings**:
  - Request型: `#[derive(Deserialize)]` + `#[serde(rename_all = "camelCase")]`
  - Response型: `#[derive(Serialize)]` + `#[serde(rename_all = "camelCase")]`
  - エラー型: `(StatusCode, Json<ErrorResponse>)` パターン
  - 既存 `BootstrapRequest` / `BootstrapResponse` が参考になる
- **Implications**: 新型は既存パターンに完全準拠

### InterpolationMethod 選択肢

- **Context**: UI補間手法選択の選択肢確認
- **Sources Consulted**: `pricer_models::market::calibration::bootstrapping::curve_builder.rs`, `pricer_core::math::interpolators/mod.rs`
- **Findings**:
  - CurveBootstrapper: Linear, LogLinear, CubicSpline
  - pricer_core: LinearInterpolator, LogLinearInterpolator, CubicSplineInterpolator, MonotonicInterpolator, HermiteInterpolator
  - 現在UIはLinear/LogLinearのみ公開
- **Implications**: CubicSpline, Monotonicを追加。Hermiteは上級者向けオプションとして検討

### 既存WebApp構造

- **Context**: UI変更箇所の特定
- **Sources Consulted**: `demo/gui/static/index.html`, `demo/gui/static/app.js`
- **Findings**:
  - View ID: `#irs-bootstrap-view` → `#curve-builder-view` にリネーム推奨
  - IRS関連セクション: `#irs-params-section`, `#pricing-result-card`, `#risk-result-card`
  - Bootstrap処理: `handleBootstrap()` 関数
  - Chart.js: `#curve-chart` canvas要素
- **Implications**: IRS関連HTMLセクションの削除、Parameter表示タブの追加

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Option A: 既存拡張 | handlers.rs, pricer_types.rsを直接拡張 | 開発速度、既存テスト活用 | コード複雑化 | 小規模変更向け |
| Option B: 新規モジュール | curve_handlers.rs, curve_types.rs新規作成 | 責務分離、影響範囲限定 | コード重複リスク | 大規模変更向け |
| **Option C: ハイブリッド** | 新規ハンドラ + 既存型拡張 | バランス、段階的実装 | 計画複雑性 | **推奨** |

## Design Decisions

### Decision: API構造

- **Context**: 既存`/api/bootstrap`と新規エンドポイントの関係
- **Alternatives Considered**:
  1. 既存エンドポイントを拡張（破壊的変更）
  2. 新規`/api/curves/*`エンドポイント群を追加（並行運用）
- **Selected Approach**: Option 2 - 新規エンドポイント追加
- **Rationale**: 後方互換性維持、既存クライアントへの影響なし
- **Trade-offs**: エンドポイント数増加、ドキュメント更新必要
- **Follow-up**: OpenAPI仕様の更新

### Decision: データファイル形式

- **Context**: Index別Instrumentリストの格納形式
- **Alternatives Considered**:
  1. 単一ファイルに全Index格納
  2. Index毎に個別ファイル (`{index}.json`)
- **Selected Approach**: Option 2 - Index別ファイル
- **Rationale**: 管理容易性、個別更新可能、ファイルサイズ制御
- **Trade-offs**: ファイル数増加
- **Follow-up**: ファイル命名規則の統一 (`usd-sofr.json`, `eur-estr.json`, `jpy-tona.json`)

### Decision: Builderプリセット保存

- **Context**: ユーザーのBuilder設定保存方式
- **Alternatives Considered**:
  1. LocalStorage（クライアント側）
  2. サーバーAPI（永続化）
  3. Cookie
- **Selected Approach**: Option 1 - LocalStorage
- **Rationale**: シンプル、サーバー負荷なし、Demo用途に適合
- **Trade-offs**: デバイス間同期なし
- **Follow-up**: 将来的にAPI保存への拡張を検討

### Decision: Parameter表示モード

- **Context**: DF/ZeroRate/ForwardRate切替のUI実装
- **Alternatives Considered**:
  1. 単一APIで全Parameter返却
  2. Parameter種別毎に個別API
- **Selected Approach**: Option 1 - 単一APIで全Parameter返却
- **Rationale**: APIコール削減、クライアント側で切替可能
- **Trade-offs**: レスポンスサイズ増加（許容範囲）
- **Follow-up**: 大規模Tenor範囲時のページネーション検討

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| IRS削除による他機能への影響 | Trade Pricing画面への移行パス確認、リダイレクト実装 |
| Forward Rate計算精度 | 既存YieldCurve trait実装を使用、新規実装なし |
| フロントエンド複雑化 | 既存app.jsパターン踏襲、モジュール分割 |
| APIエンドポイント増加 | OpenAPI仕様でドキュメント化、一貫した命名規則 |

## References

- [YieldCurve Trait](crates/pricer_models/src/market/curves/traits.rs) — カーブインターフェース定義
- [BootstrapInstrument](crates/pricer_models/src/market/calibration/bootstrapping/instrument.rs) — Instrument構造
- [pricer_types.rs](demo/gui/src/web/pricer_types.rs) — 既存API型パターン
- [A-I-P-S Architecture](/.kiro/steering/tech.md) — アーキテクチャ制約
