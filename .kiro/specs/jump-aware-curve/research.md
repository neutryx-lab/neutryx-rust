# Research & Design Decisions

## Summary
- **Feature**: `jump-aware-curve`
- **Discovery Scope**: Extension（既存システムの拡張）
- **Key Findings**:
  - `CurveDefinition` と `EventInstrument` は既に存在するが未接続
  - `YieldCurve` trait は `enum_dispatch` パターンで実装済み
  - `BootstrappedCurve::discount_factor()` は連続内挿のみ対応

## Research Log

### 既存拡張ポイント分析

- **Context**: JumpPillar を追加するための最適な統合ポイントを特定
- **Sources Consulted**:
  - [definition/curve.rs](crates/infra_master/src/market/definition/curve.rs)
  - [event_instrument.rs](crates/infra_master/src/market/event_instrument.rs)
  - [market.rs](crates/pricer_models/src/market.rs)
- **Findings**:
  - `CurveDefinition` は Builder パターン (`with_*` メソッド) を採用
  - `EventInstrument` は `expected_spread`, `confidence`, `rate_index`, `event_date` を保持
  - `BootstrappedCurve` は pillars/discount_factors ベクタで構成
  - `YieldCurve<T>` trait は `enum_dispatch` マクロで静的ディスパッチ実装済み
- **Implications**:
  - `JumpPillar` を `CurveDefinition` にオプショナルフィールドとして追加可能
  - `BootstrappedCurve` に `jumps` フィールドと `Limit` 対応メソッド追加

### YieldCurve trait 拡張パターン

- **Context**: 左極限・右極限の分離取得インターフェース設計
- **Sources Consulted**:
  - [traits.rs](crates/pricer_core/src/math/interpolators/traits.rs)
  - [market.rs:65-96](crates/pricer_models/src/market.rs#L65-L96)
- **Findings**:
  - 既存 `discount_factor(t: T)` は単一値返却
  - `Limit` enum を追加し、オーバーロードメソッドで対応可能
  - 後方互換性維持のため、新メソッド `discount_factor_with_limit` を追加
- **Implications**:
  - デフォルト実装で `Limit::Continuous` を提供し既存コードへの影響を最小化

### ブートストラップへのジャンプ統合

- **Context**: `CurveBootstrapper` がジャンプを考慮したキャリブレーション実行方法
- **Sources Consulted**:
  - [bootstrap.rs](crates/pricer_models/src/builder/curve/bootstrap.rs)
- **Findings**:
  - 逐次ブートストラップは pillar ごとに Newton-Raphson でソルブ
  - `pricing_error` 計算時にジャンプオフセットを適用する必要あり
  - `MarketInstrument::Event` variant が既に存在（基盤あり）
- **Implications**:
  - ジャンプ日を跨ぐ商品の価格計算でオフセット適用ロジック追加

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: 既存拡張 | CurveDefinition/BootstrappedCurve に直接フィールド追加 | 最小限のファイル追加、既存パターン活用 | 既存構造体の複雑化 | 後方互換性容易 |
| B: 新規構造体 | JumpAwareCurve wrapper を新規作成 | 責務分離が明確 | ファイル数増加、CurveEnum 統合要 | 既存コードへの影響最小 |
| C: ハイブリッド | 定義は分離、実装は既存拡張 | バランス良い | 計画が複雑 | **推奨** |

## Design Decisions

### Decision: JumpPillar 配置場所

- **Context**: JumpPillar 構造体をどのモジュールに配置するか
- **Alternatives Considered**:
  1. `infra_master/market/definition/curve.rs` 内に追加
  2. 新規 `infra_master/market/definition/jump_pillar.rs` として分離
- **Selected Approach**: Option 2 - 分離ファイルとして作成
- **Rationale**:
  - curve.rs のサイズ維持
  - 単一責任原則の遵守
  - `JumpPillarBuilder` 等の関連型を同梱可能
- **Trade-offs**: ファイル数 +1、ただし保守性向上
- **Follow-up**: mod.rs でエクスポート追加

### Decision: Limit enum の配置

- **Context**: 左極限・右極限指定用 enum の配置場所
- **Alternatives Considered**:
  1. `pricer_core::types` に追加
  2. `pricer_models::market::curves` 内に追加
- **Selected Approach**: Option 1 - `pricer_core::types` に追加
- **Rationale**:
  - 曲線以外の数値計算（微分、積分）でも再利用可能
  - L1 層に配置し下流で利用
- **Trade-offs**: pricer_core の scope 拡大
- **Follow-up**: なし

### Decision: YieldCurve trait 拡張方式

- **Context**: ジャンプ対応メソッドの追加方法
- **Alternatives Considered**:
  1. `discount_factor` をオーバーロード
  2. 新メソッド `discount_factor_with_limit` 追加
  3. 別 trait `JumpAwareYieldCurve` 作成
- **Selected Approach**: Option 2 - 新メソッド追加 with デフォルト実装
- **Rationale**:
  - 後方互換性完全維持
  - 既存 `YieldCurve` 実装は変更不要
- **Trade-offs**: メソッド数増加
- **Follow-up**: `CurveEnum` variant に JumpAwareCurve 追加検討

## Risks & Mitigations

- **数学的正確性**: ジャンプ適用時の discount factor 計算式の検証 → 単体テストで複数シナリオ検証
- **Enzyme AD 互換性**: ジャンプパラメータの微分可能性 → smooth_indicator でジャンプ境界を平滑化するオプション
- **パフォーマンス**: ジャンプ日検索のオーバーヘッド → BTreeMap または事前ソート済み Vec で O(log n) 検索

## References

- [enum_dispatch crate](https://docs.rs/enum_dispatch) — 静的ディスパッチパターン
- [pricer_models::market](crates/pricer_models/src/market.rs) — YieldCurve trait 定義
- [infra_master::market::definition](crates/infra_master/src/market/definition/mod.rs) — CurveDefinition モジュール
