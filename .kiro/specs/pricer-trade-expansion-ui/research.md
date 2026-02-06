# Research & Design Decisions: pricer-trade-expansion-ui

---
**Purpose**: Pricer 画面の Instrument 選択拡張と CF 展開 Trade 生成・表示機能に関する Discovery 調査結果。
---

## Summary

- **Feature**: `pricer-trade-expansion-ui`
- **Discovery Scope**: Extension（既存システムの拡張）
- **Key Findings**:
  - infra_domain の serde feature は `optional = true` で定義済み、demo/gui の依存追加で有効化可能
  - Tenor → Date リスト生成は `Tenor::add_to_date()` と `Frequency::months_per_period()` を組み合わせて実装可能
  - TradeBuilder/LegBuilder は完成済みで、直接使用可能

## Research Log

### infra_domain serde 対応状況

- **Context**: Trade 構造体を JSON シリアライズするために serde 対応が必要
- **Sources Consulted**: `crates/infra_domain/Cargo.toml`, `crates/infra_domain/src/trade/*.rs`
- **Findings**:
  - infra_domain は `serde = ["dep:serde"]` feature を持つ
  - すべての主要型は `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]` で定義
  - Trade, Leg, Cashflow, Payoff, Direction, LegType など全型が serde 対応
  - Currency, Date, DayCounter, Frequency, Tenor も serde 対応済み
- **Implications**: demo/gui の Cargo.toml で `infra_domain = { path = "...", features = ["serde"] }` を追加するだけで利用可能

### スケジュール生成ロジック

- **Context**: LegBuilder は `Vec<Date>` を入力として期待するため、Tenor + Frequency から Date リストを生成する必要がある
- **Sources Consulted**: `crates/infra_domain/src/time/period.rs`, `crates/infra_domain/src/time/frequency.rs`
- **Findings**:
  - `Tenor::add_to_date(date, EndOfMonthRule)` で満期日を計算可能
  - `Frequency::months_per_period()` で支払い間隔を取得可能
  - `Date + Period::months(n)` で日付加算可能
  - `AccrualPeriod` 構造体は参考になるが、スケジュール生成自体は未実装
- **Implications**: trade_handlers.rs にスケジュール生成ユーティリティ関数を新規実装する必要あり

### 既存 API パターン

- **Context**: 新規エンドポイントは既存パターンに従う必要がある
- **Sources Consulted**: `demo/gui/src/web/handlers.rs`, `demo/gui/src/web/pricer_types.rs`, `demo/gui/src/web/mod.rs`
- **Findings**:
  - パターン: `async fn handler(State(state): State<Arc<AppState>>, Json(request): Json<T>) -> impl IntoResponse`
  - serde: `#[serde(rename_all = "camelCase")]` で JavaScript との互換性
  - エラー: `(StatusCode, Json<ErrorResponse>)` 形式
  - ルーティング: `demo/gui/src/web/mod.rs` で `.route("/endpoint", post/get(handler))` 追加
- **Implications**: 既存パターンに完全準拠した設計が可能

### demo/gui 依存関係

- **Context**: demo/gui から infra_domain を使用するための依存設定
- **Sources Consulted**: `demo/gui/Cargo.toml`
- **Findings**:
  - 現在 `pricer_core`, `pricer_pricing` に依存
  - `infra_domain` への直接依存はなし
  - A-I-P-S ルール: S レイヤーは I レイヤーに依存可能
- **Implications**: `infra_domain = { path = "../../crates/infra_domain", features = ["serde"] }` を追加

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: 既存ファイル拡張 | pricer_types.rs, handlers.rs に追加 | ファイル数最小化 | 肥大化リスク | 既存 pricer との混在 |
| B: 新規ファイル作成 | trade_handlers.rs, trade_types.rs 新規 | 責務分離明確 | ファイル増加 | Trade 展開専用 |
| C: ハイブリッド | バックエンド新規、フロントエンド既存拡張 | バランス良好 | 設計複雑化 | **推奨** |

**選択**: Option C（ハイブリッドアプローチ）
- バックエンドは Trade 展開とプライシングで責務が異なるため新規ファイル
- フロントエンドは既存 UI 構造との統合が必要なため既存拡張

## Design Decisions

### Decision: スケジュール生成方式

- **Context**: Tenor + Frequency から支払いスケジュール（`Vec<Date>`）を生成する必要
- **Alternatives Considered**:
  1. pricer_models::schedules::ScheduleBuilder 活用 — 存在するが limited scope
  2. trade_handlers.rs にユーティリティ関数を新規実装 — シンプルで目的特化
- **Selected Approach**: Option 2（新規ユーティリティ関数）
- **Rationale**:
  - pricer_models には bootstrapping 用のスケジュール生成のみ
  - Trade 展開用途にはシンプルな関数で十分
  - infra_domain の Tenor/Frequency/Date を直接使用
- **Trade-offs**:
  - ✅ 依存関係最小化
  - ✅ 目的特化でシンプル
  - ❌ 重複コードの可能性（軽微）
- **Follow-up**: 将来的に共通化が必要な場合は infra_domain に移動を検討

### Decision: Cashflow 表示方式

- **Context**: 30 年スワップでは 100+ の Cashflow が発生、UI パフォーマンスが課題
- **Alternatives Considered**:
  1. ページネーション — 1 ページ 20 件程度、ナビゲーション付き
  2. 仮想スクロール — 可視領域のみレンダリング
- **Selected Approach**: Option 1（ページネーション）
- **Rationale**:
  - 実装容易性が高い（JavaScript で十分実装可能）
  - ユーザーは通常全 Cashflow を一度に見る必要がない
  - 既存 UI パターンとの一貫性
- **Trade-offs**:
  - ✅ 実装シンプル
  - ✅ 既存パターン踏襲
  - ❌ 大量データ時のナビゲーション手間
- **Follow-up**: パフォーマンス問題発生時に仮想スクロールへ移行

### Decision: Instrument メタデータ API 設計

- **Context**: UI で動的フォーム生成のため、Instrument パラメータスキーマが必要
- **Alternatives Considered**:
  1. バックエンドでメタデータ生成 — API 経由で取得
  2. フロントエンドでハードコード — JavaScript に定義
- **Selected Approach**: Option 1（バックエンドでメタデータ生成）
- **Rationale**:
  - Single Source of Truth
  - 型定義と同期維持が容易
  - 将来の拡張（バリデーションルール等）に対応可能
- **Trade-offs**:
  - ✅ 型安全性
  - ✅ 一貫性維持
  - ❌ API 呼び出しが必要（起動時 1 回）

## Risks & Mitigations

- **Risk 1**: スケジュール生成ロジックのバグ（月末処理等） — 単体テストで EndOfMonthRule パターンを網羅
- **Risk 2**: infra_domain 型の serde 非互換 — 実装初期に全型のシリアライズテスト実施
- **Risk 3**: 大量 Cashflow 表示時のブラウザパフォーマンス — ページネーション採用 + 必要に応じて遅延ロード

## References

- [Axum Official Documentation](https://docs.rs/axum/) — REST API 実装パターン
- [serde.rs Guide](https://serde.rs/) — JSON シリアライズ設計
- infra_domain 内部ドキュメント — Trade/Leg/Cashflow アーキテクチャ
