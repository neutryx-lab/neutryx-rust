# Research & Design Decisions: market-rate-infrastructure

---
**Purpose**: 設計判断の根拠とディスカバリ結果を記録する。
**Discovery Scope**: Extension（既存 infra_master::market への機能追加）

---

## Summary

- **Feature**: `market-rate-infrastructure`
- **Discovery Scope**: Extension（既存モジュールへの新規型追加）
- **Key Findings**:
  1. `adapter_feeds::QuoteType` との重複を解決するため、`infra_master` で `QuoteType` を新規定義し re-export 方式を採用
  2. `pricer_models::MarketDataError` との名前衝突を回避するため、`MarketRateError` として命名
  3. 既存の `Instrument` enum（7 バリアント）への直接マッピングが可能

## Research Log

### QuoteType の配置場所

- **Context**: 要件 1.4 で `QuoteType` を `infra_master` に定義する必要があるが、`adapter_feeds` に既存の同名型が存在
- **Sources Consulted**:
  - `adapter_feeds/src/quote.rs` - 既存 `QuoteType` 定義
  - `.kiro/steering/structure.md` - A-I-P-S 依存ルール
- **Findings**:
  - 既存 `QuoteType` は `Bid`, `Ask`, `Last`, `Mid` の 4 バリアント
  - A-I-P-S ルールでは `adapter_feeds` は `infra_master` に依存可能
- **Implications**: `infra_master::market::QuoteType` を正規定義とし、`adapter_feeds` は将来 re-export に移行

### エラー型の命名

- **Context**: 要件 5.1 で `MarketDataError` を定義する必要があるが、`pricer_models::market::MarketDataError` が既存
- **Sources Consulted**:
  - `pricer_models/src/market/error.rs` - 既存 `MarketDataError`
  - `.kiro/steering/error-handling.md` - エラー型設計パターン
- **Findings**:
  - 既存型は曲線・サーフェス操作向け（InvalidMaturity, InvalidStrike, OutOfBounds 等）
  - 本仕様のエラーはレート入力バリデーション向け（InvalidRate, StaleData, MissingRate）
- **Implications**: `MarketRateError` として別名定義、必要に応じて `From` トレイトで変換

### RateId の構造設計

- **Context**: 要件 2.1 でレートを一意に識別する `RateId` が必要
- **Sources Consulted**:
  - `infra_master::market::Currency` - 通貨型
  - `infra_master::time::Tenor` - テナー型
  - `infra_master::market::RateIndex` - 既存レートインデックス
- **Findings**:
  - `(Currency, Tenor, RateType)` のタプルで論理的に一意性を担保可能
  - `RateIndex`（SOFR, EURIBOR 等）を追加すると更に精緻な識別が可能
- **Implications**: 構造体 `RateId { currency, tenor, rate_type, rate_index: Option<RateIndex> }` を採用

### TickerMapping の実装方式

- **Context**: 要件 2.2 で外部ティッカーと `RateId` のマッピングが必要
- **Sources Consulted**: 業界標準（Reuters RIC, Bloomberg ticker フォーマット）
- **Findings**:
  - Reuters RIC: `USD3MFSR=` (SOFR 3M), `EURIBOR3MD=` (EURIBOR 3M)
  - Bloomberg: `SOFR Index`, `EUR003M Index`
  - フォーマットは標準化されておらず、ランタイム設定が現実的
- **Implications**: `HashMap<String, RateId>` ベースの動的マッピング + 主要通貨のデフォルト定義

### 金利レートのバリデーション閾値

- **Context**: 要件 5.3 で「suspiciously large」レートを検出する必要
- **Sources Consulted**: 金融業界慣行
- **Findings**:
  - 通常の金利: -5% ～ +50% の範囲（ハイパーインフレ対応）
  - FX スポット: 通貨ペアにより異なる（USDJPY: 50-200, EURUSD: 0.5-2.0）
  - ボラティリティ: 0% ～ 500%
- **Implications**: `RateType` ごとに異なる閾値を `StandardRateValidator` で定義

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Module Extension | 既存 `infra_master::market` にファイル追加 | 既存パターン遵守、依存関係シンプル | ファイル数増加 | **採用** |
| New Submodule | `infra_master::market::rates` サブモジュール作成 | 責務分離 | 既存 API との整合性 | 見送り |
| Separate Crate | 新規 `infra_rates` クレート | 完全分離 | オーバーエンジニアリング | 見送り |

## Design Decisions

### Decision: QuoteType を infra_master で新規定義

- **Context**: `adapter_feeds` に既存 `QuoteType` があるが、A-I-P-S 依存ルールでは Infra が正規の定義場所
- **Alternatives Considered**:
  1. `adapter_feeds::QuoteType` をそのまま使用 — 依存方向違反
  2. 両方に定義 — 重複、混乱の原因
- **Selected Approach**: `infra_master::market::QuoteType` を新規定義、将来 `adapter_feeds` を更新
- **Rationale**: A-I-P-S ルール遵守、長期的な型の一貫性確保
- **Trade-offs**: 短期的に 2 つの `QuoteType` が存在、Phase 2 で統合
- **Follow-up**: `adapter_feeds` リファクタリングを別仕様として計画

### Decision: MarketRateError として命名

- **Context**: `pricer_models::MarketDataError` との名前衝突回避
- **Alternatives Considered**:
  1. 同名で異なるモジュールに定義 — インポート時混乱
  2. `RateValidationError` — 狭義すぎる
- **Selected Approach**: `MarketRateError` として命名、レート入力に特化した意味を持たせる
- **Rationale**: 明確な責務分離、自明な命名
- **Trade-offs**: 若干の命名冗長性
- **Follow-up**: なし

### Decision: RateId を構造体として設計

- **Context**: タプル型 vs 構造体の選択
- **Alternatives Considered**:
  1. `type RateId = (Currency, Tenor, RateType)` — フィールドアクセスが不明瞭
  2. 構造体 — 明示的なフィールド名
- **Selected Approach**: 構造体 `RateId { currency, tenor, rate_type, rate_index }`
- **Rationale**: 可読性、将来の拡張性（`rate_index` 追加済み）
- **Trade-offs**: ボイラープレート増加（Rust では許容範囲）
- **Follow-up**: なし

### Decision: HashMap ベースの MarketRateSet

- **Context**: O(1) ルックアップ要件（NFR 1.1）
- **Alternatives Considered**:
  1. `Vec` + 線形探索 — O(n)、要件不適合
  2. `BTreeMap` — O(log n)、範囲検索に有利だが要件なし
- **Selected Approach**: `HashMap<(RateId, QuoteType), MarketRate>` 複合キー
- **Rationale**: O(1) ルックアップ、bid/ask/mid 個別保持（要件 3.2）
- **Trade-offs**: メモリ使用量増加（許容範囲）
- **Follow-up**: なし

## Risks & Mitigations

- **Risk 1**: `adapter_feeds` との一時的な型重複 — Phase 2 でリファクタリング計画
- **Risk 2**: TickerMapping のメンテナンスコスト — デフォルトマッピングを提供し、カスタマイズは設定ファイルで対応
- **Risk 3**: バリデーション閾値の妥当性 — 業界標準を参照、ユーザーカスタマイズ可能な `RateValidator` trait 提供

## References

- [infra_master::trade::Instrument](crates/infra_master/src/trade/instrument.rs) - マッピング先の型
- [infra_master::market::RateIndex](crates/infra_master/src/market/rate_index.rs) - 既存レートインデックス
- [adapter_feeds::QuoteType](crates/adapter_feeds/src/quote.rs) - 既存 QuoteType
- [pricer_models::MarketDataError](crates/pricer_models/src/market/error.rs) - 既存エラー型
- [A-I-P-S 依存ルール](../../steering/structure.md) - アーキテクチャ制約
