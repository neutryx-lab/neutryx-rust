# Research & Design Decisions: demo-webapp-pricer

---
**Purpose**: Capture discovery findings, architectural investigations, and rationale that inform the technical design.

---

## Summary
- **Feature**: `demo-webapp-pricer`
- **Discovery Scope**: Extension（既存Demo WebAppへの機能追加）
- **Key Findings**:
  - `pricer_pricing::generic_pricer` モジュールは88テスト通過、APIは安定しており直接利用可能
  - `SimpleLeg`/`SimpleCashflow` 型は `l1l2-integration` feature無しで利用可能
  - 既存ハンドラパターン（Axum + State + Json）の踏襲でシームレスな統合が可能

## Research Log

### GenericPricer API分析
- **Context**: プライシングエンドポイントからGenericPricerを呼び出す方法の調査
- **Sources Consulted**:
  - `crates/pricer_pricing/src/generic_pricer/mod.rs`
  - `crates/pricer_pricing/src/generic_pricer/pricer.rs`
- **Findings**:
  - `GenericPricer::new(model_config, pricer_config)` でインスタンス生成
  - `get_pv_simple(legs, valuation_date, reporting_currency)` が主要API
  - `SimpleLeg` は `currency`, `direction`, `cashflows` を持つ
  - `SimpleCashflow` は `payment_date`, `amount` を持つ
  - `Date` は `from_days(days)` で生成可能（内部的に days since epoch）
  - `Direction` は `Payer` / `Receiver` enum
  - `DefaultCurrency` は `USD`, `EUR`, `JPY`, `GBP`, `CHF` をサポート
- **Implications**:
  - Trade展開結果から `SimpleLeg`/`SimpleCashflow` への変換が必要
  - FXレートはプレースホルダー実装（CHFは未対応でエラー）

### Greeks計算API分析
- **Context**: BumpAndRevalueCalculatorの使用方法調査
- **Sources Consulted**:
  - `crates/pricer_pricing/src/generic_pricer/greeks_calculator.rs`
- **Findings**:
  - `BumpSizes` でバンプ幅設定（rate_bump_bp, fx_bump_pct, vol_bump_pct）
  - `BumpAndRevalueCalculator::new(pricer, bump_sizes)` でインスタンス生成
  - `calculate_delta`, `calculate_gamma`, `calculate_theta`, `calculate_vega`, `calculate_fx_delta` 関数あり
  - `TradeGreeks` 構造体でGreeks結果を保持
- **Implications**:
  - 各Greeks関数を個別に呼び出す必要あり
  - または `TradeGreeks` を使用してまとめて計算

### 既存ハンドラパターン分析
- **Context**: Demo WebApp既存のAPIハンドラパターン調査
- **Sources Consulted**:
  - `demo/gui/src/web/handlers.rs`
  - `demo/gui/src/web/curve_builder_handlers.rs`
  - `demo/gui/src/web/trade_handlers.rs`
- **Findings**:
  - `State<Arc<AppState>>` でアプリケーション状態共有
  - `Json<T>` でリクエスト/レスポンス処理
  - `validate_*_request` 関数でバリデーション
  - `*ErrorResponse` 型でエラーレスポンス
  - `pricer_types.rs` に型定義集約（57k tokens、巨大）
  - `*_handlers.rs` にハンドラ分離
- **Implications**:
  - ハンドラは新規ファイル `generic_pricer_handlers.rs` 推奨
  - 型定義は `pricer_types.rs` に追加（一貫性維持）

### フロントエンドパターン分析
- **Context**: UI実装パターン調査
- **Sources Consulted**:
  - `demo/gui/static/index.html`
  - `demo/gui/static/app.js`
  - `demo/gui/static/js/curve-builder.js`
- **Findings**:
  - `data-view` 属性でビュー切り替え
  - `navigateTo()` 関数でナビゲーション
  - 2パネルレイアウト（入力 + 結果）
  - `apiClient` で非同期API呼び出し
  - `showToast()` でユーザー通知
  - glassmorphism デザイン（`.glass` クラス）
- **Implications**:
  - 既存パターンに従いPricerモジュールを `pricer.js` として追加
  - `index.html` に `#pricer-view` セクション追加

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A. 既存拡張 | handlers.rs, pricer_types.rs に追加 | インポート変更なし | ファイル肥大化 | 非推奨 |
| B. 新規作成 | 全て新規ファイル | 独立性高 | 一部コード重複 | 検討 |
| **C. ハイブリッド** | ハンドラは新規、型は既存拡張 | バランス良 | - | **採用** |

## Design Decisions

### Decision: ハンドラファイル配置
- **Context**: APIハンドラの配置場所決定
- **Alternatives Considered**:
  1. `handlers.rs` に追加 — 既存の巨大ファイルにさらに追加
  2. `generic_pricer_handlers.rs` 新規作成 — 独立したファイル
- **Selected Approach**: `generic_pricer_handlers.rs` 新規作成
- **Rationale**: handlers.rs は既に69k tokensで非常に大きい。単一責任原則に従い独立したファイルで保守性向上
- **Trade-offs**: ファイル数増加 vs 保守性向上
- **Follow-up**: mod.rs へのモジュール登録必要

### Decision: API パス設計
- **Context**: Pricer関連エンドポイントのパス設計
- **Alternatives Considered**:
  1. `/api/price` 拡張 — 既存エンドポイント拡張
  2. `/api/pricer/*` 新規 — 専用ネームスペース
- **Selected Approach**: `/api/pricer/*` 新規
- **Rationale**: 既存 `/api/price` は別の実装。generic_pricer統合用に明確に分離
- **Trade-offs**: エンドポイント数増加 vs 責務明確化
- **Follow-up**: mod.rs の build_router() に pricer_routes 追加

### Decision: Trade型変換
- **Context**: TradeExpandResponse → SimpleLeg/SimpleCashflow 変換
- **Alternatives Considered**:
  1. フロントエンドで変換 — JS側で型変換
  2. バックエンドで変換 — Rust側で型変換
- **Selected Approach**: バックエンドで変換
- **Rationale**: Rust型安全性を活用、フロントエンドの複雑化を回避
- **Trade-offs**: バックエンド複雑化 vs フロントエンド簡素化
- **Follow-up**: 変換関数を `generic_pricer_handlers.rs` に実装

### Decision: フロントエンドモジュール配置
- **Context**: Pricer UIモジュールの配置場所
- **Alternatives Considered**:
  1. `app.js` に統合 — 既存の巨大ファイルに追加
  2. `pricer.js` 新規作成 — 独立モジュール
- **Selected Approach**: `pricer.js` 新規作成
- **Rationale**: app.js は15000+行で巨大。モジュール分離で保守性向上
- **Trade-offs**: ファイル数増加 vs 保守性向上
- **Follow-up**: index.html で pricer.js を読み込み

## Risks & Mitigations
- **型変換の複雑性** — 明確な変換関数を設計し、ユニットテストで検証
- **FXレートプレースホルダー制約** — 対応通貨（USD, EUR, JPY, GBP）を明示、CHFはエラー表示
- **UI複雑性** — 既存Curve Builderパターンを参照し、一貫したUX提供

## References
- [generic_pricer/mod.rs](crates/pricer_pricing/src/generic_pricer/mod.rs) — GenericPricer API exports
- [pricer_types.rs](demo/gui/src/web/pricer_types.rs) — 既存プライサー型定義
- [curve_builder_handlers.rs](demo/gui/src/web/curve_builder_handlers.rs) — ハンドラパターン参照
- [gap-analysis.md](.kiro/specs/demo-webapp-pricer/gap-analysis.md) — ギャップ分析結果
