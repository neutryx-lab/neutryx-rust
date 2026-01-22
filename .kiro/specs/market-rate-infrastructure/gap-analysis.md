# Gap Analysis: market-rate-infrastructure

## 1. Current State Investigation

### 1.1 既存のドメイン関連アセット

#### infra_master::market（対象配置場所）

| ファイル | 内容 | 再利用可能性 |
|----------|------|--------------|
| `currency.rs` | `Currency` enum（ISO 4217） | ✅ そのまま使用 |
| `rate_index.rs` | `RateIndex` enum（SOFR, EURIBOR等） | ✅ そのまま使用 |
| `mod.rs` | モジュール定義 | 🔄 拡張必要 |

#### infra_master::trade（関連アセット）

| ファイル | 内容 | 関係性 |
|----------|------|--------|
| `instrument.rs` | `Instrument` enum（Deposit, FRA, Futures, ParSwap, OIS, BasisSwap, CrossCurrencySwap） | 🔗 マッピング先 |

#### infra_master::time（関連アセット）

| ファイル | 内容 | 関係性 |
|----------|------|--------|
| `period.rs` | `Tenor` enum（ON, 1W, 1M, 3M, 1Y, 10Y等） | 🔗 RateId 構成要素 |

#### adapter_feeds（上流アダプター）

| ファイル | 内容 | 関係性 |
|----------|------|--------|
| `quote.rs` | `MarketQuote` struct, `QuoteType` enum | ⚠️ 重複懸念（後述） |

#### pricer_models::market（下流コンシューマー）

| ファイル | 内容 | 関係性 |
|----------|------|--------|
| `provider.rs` | `MarketProvider`（Arc キャッシュ） | 🔗 統合候補 |
| `error.rs` | `MarketDataError` enum | ⚠️ 重複懸念（後述） |
| `calibration/bootstrapping/` | カーブ構築ロジック | 🔗 Instrument 消費者 |

### 1.2 抽出されたコンベンション

| カテゴリ | パターン |
|----------|----------|
| エラー処理 | `thiserror` による `#[derive(Error)]`、構造化バリアント |
| serde 対応 | `#[cfg_attr(feature = "serde", derive(...))]` による条件付きシリアライゼーション |
| ビルダーパターン | `with_*()` メソッドチェーン（`MarketQuote` 参照） |
| 列挙型設計 | `#[non_exhaustive]` 属性、`code()` / `name()` メソッド |
| テスト配置 | 同一ファイル内 `#[cfg(test)] mod tests` |

### 1.3 アーキテクチャ制約

```
A-I-P-S 依存ルール:
┌─────────────────────────────────────────────────────────────┐
│ adapter_feeds (A)                                           │
│   ↓ depends on                                              │
│ infra_master::market (I) ← 本仕様の対象                     │
│   ↓ depends on                                              │
│ pricer_models::market (P) ← Instrument を消費               │
└─────────────────────────────────────────────────────────────┘

❌ infra_master は pricer_* に依存してはならない
❌ infra_master は adapter_* に依存してはならない
```

---

## 2. Requirements Feasibility Analysis

### 2.1 技術的ニーズマッピング

| 要件 | 必要なデータモデル | 必要なAPI | 既存アセット | ギャップ |
|------|-------------------|-----------|-------------|----------|
| Req 1: マーケットレート型 | `MarketRate`, `RateType`, `QuoteType` | バリデーション | `adapter_feeds::QuoteType` 部分一致 | **Missing**: `MarketRate`, `RateType` |
| Req 2: レート識別子 | `RateId`, `TickerMapping` | ルックアップ | なし | **Missing**: 全て新規 |
| Req 3: レートセット管理 | `MarketRateSet` | CRUD, イテレータ | なし | **Missing**: 全て新規 |
| Req 4: Instrument マッピング | `InstrumentMapper` trait | 変換 | `infra_master::trade::Instrument` | **Missing**: マッパー |
| Req 5: バリデーション | `MarketDataError`, `RateValidator` | 検証 | `pricer_models::MarketDataError` 類似 | **Missing**: infra_master 用エラー |
| Req 6: データソース抽象化 | `DataSource`, `SourcePriority` | マージ | なし | **Missing**: 全て新規 |
| Req 7: Pricer 受け渡し | `to_instruments()`, フィルタ | 変換 | なし | **Missing**: 全て新規 |

### 2.2 重要なギャップと懸念事項

#### ⚠️ Gap 1: `QuoteType` の重複

**現状**: `adapter_feeds::QuoteType` が既に存在（Bid, Ask, Last, Mid）

**懸念**:
- 要件では `infra_master::market` に `QuoteType` を定義する想定
- 同一概念の型が複数箇所に存在すると混乱の原因

**選択肢**:
- **A**: `adapter_feeds::QuoteType` を `infra_master` に移動
- **B**: `infra_master` で新規定義し、`adapter_feeds` から参照
- **C**: `adapter_feeds` の型をそのまま使用（要件修正）

#### ⚠️ Gap 2: `MarketDataError` の重複

**現状**: `pricer_models::market::MarketDataError` が既に存在

**懸念**:
- 要件では `infra_master::market::MarketDataError` を定義する想定
- 異なるレイヤーに同名エラー型が存在すると変換が複雑化

**選択肢**:
- **A**: `infra_master` 用に別名（`RateValidationError` 等）を使用
- **B**: `pricer_models` のエラーを拡張（依存方向違反の可能性）
- **C**: 要件のエラー名を変更（`MarketRateError` 等）

#### ⚠️ Gap 3: `MarketQuote` vs `MarketRate` の関係

**現状**: `adapter_feeds::MarketQuote` が株式/FX 向けの汎用 quote 構造

**懸念**:
- `MarketRate` は金利商品特化の構造（RateType, Tenor 含む）
- 両者の責務境界が不明確

**選択肢**:
- **A**: `MarketRate` は金利特化、`MarketQuote` は株式/FX 特化として共存
- **B**: `MarketRate` を `MarketQuote` の特殊化として設計（継承的関係）
- **C**: 統一 `MarketData` 型を設計（大規模リファクタリング）

### 2.3 複雑性シグナル

| カテゴリ | 評価 |
|----------|------|
| CRUD 操作 | 単純（HashMap ベース） |
| アルゴリズム | 単純（mid 計算、バリデーション） |
| ワークフロー | 中程度（マッピング変換） |
| 外部統合 | なし（純粋な型定義） |

---

## 3. Implementation Approach Options

### Option A: 最小拡張アプローチ

**戦略**: 既存の `adapter_feeds` 型を活用し、`infra_master` は最小限の追加

**変更内容**:
- `adapter_feeds::QuoteType` をそのまま使用
- `infra_master::market/` に新規ファイル追加:
  - `rate.rs` - `MarketRate`, `RateType`, `RateId`
  - `rate_set.rs` - `MarketRateSet`
  - `mapper.rs` - `InstrumentMapper`
  - `validation.rs` - `RateValidator`, `RateValidationError`
  - `source.rs` - `DataSource`, `SourcePriority`

**Trade-offs**:
- ✅ 既存コードへの影響最小
- ✅ 依存関係がシンプル
- ❌ `adapter_feeds` への暗黙的依存が残る
- ❌ 型の一貫性が低下

### Option B: 完全独立アプローチ

**戦略**: `infra_master::market` を完全に自己完結させる

**変更内容**:
- `infra_master::market/` に全型を新規定義:
  - `quote_type.rs` - `QuoteType`（新規定義）
  - `rate.rs` - `MarketRate`, `RateType`, `RateId`
  - `rate_set.rs` - `MarketRateSet`
  - `ticker.rs` - `TickerMapping`
  - `mapper.rs` - `InstrumentMapper`, `StandardInstrumentMapper`
  - `validation.rs` - `RateValidator`, `StandardRateValidator`
  - `error.rs` - `MarketRateError`（名前変更で重複回避）
  - `source.rs` - `DataSource`, `SourcePriority`
- `adapter_feeds` を後でリファクタリング（`infra_master` から import）

**Trade-offs**:
- ✅ A-I-P-S 依存ルールに完全準拠
- ✅ 型の一貫性が高い
- ✅ 将来の拡張が容易
- ❌ 初期コード量が増加
- ❌ `adapter_feeds` のリファクタリングが必要

### Option C: ハイブリッドアプローチ（推奨）

**戦略**: 共通型を `infra_master` に移動し、既存コードは互換性を維持

**Phase 1（本仕様）**:
- `infra_master::market/` に新規型を追加
- `QuoteType` は `infra_master` で定義
- `adapter_feeds` は `infra_master::market::QuoteType` を re-export

**Phase 2（将来リファクタリング）**:
- `adapter_feeds::MarketQuote` を `infra_master` 型を使用するよう更新
- `pricer_models::MarketDataError` との統合検討

**変更内容**:
```
infra_master/src/market/
├── mod.rs           # 既存（拡張）
├── currency.rs      # 既存
├── rate_index.rs    # 既存
├── quote_type.rs    # 新規: QuoteType enum
├── rate_type.rs     # 新規: RateType enum
├── rate_id.rs       # 新規: RateId struct
├── rate.rs          # 新規: MarketRate struct
├── rate_set.rs      # 新規: MarketRateSet struct
├── ticker.rs        # 新規: TickerMapping struct
├── mapper.rs        # 新規: InstrumentMapper trait + StandardInstrumentMapper
├── validation.rs    # 新規: RateValidator trait + StandardRateValidator
├── error.rs         # 新規: MarketRateError enum
└── source.rs        # 新規: DataSource, SourcePriority
```

**Trade-offs**:
- ✅ 段階的な移行が可能
- ✅ 既存コードへの破壊的変更を最小化
- ✅ 長期的な一貫性を確保
- ❌ Phase 2 の作業が残る
- ❌ 一時的な重複が発生

---

## 4. Implementation Complexity & Risk

### Effort 評価: **M（3-7日）**

**理由**:
- 新規型定義が中心（10+ 新規ファイル）
- 既存パターン（thiserror, serde feature gate）に従う
- 複雑なアルゴリズムなし
- 外部統合なし

### Risk 評価: **Medium**

**リスク要因**:
- 型の重複問題（QuoteType, MarketDataError）の解決方針決定が必要
- `adapter_feeds` との整合性確保
- 将来の `pricer_models::MarketProvider` 統合の設計考慮

**軽減策**:
- Option C（ハイブリッド）採用で段階的移行
- `adapter_feeds` リファクタリングは別仕様として切り出し

---

## 5. Recommendations for Design Phase

### 推奨アプローチ

**Option C（ハイブリッドアプローチ）** を推奨

### 設計フェーズで決定すべき事項

1. **QuoteType の配置決定**: `infra_master` で定義し、`adapter_feeds` から re-export
2. **エラー型の命名**: `MarketRateError`（`pricer_models::MarketDataError` との衝突回避）
3. **RateId の構造決計**: `(Currency, Tenor, RateType)` のタプル構造 vs 構造体
4. **TickerMapping の実装**: 静的マッピング（compile-time）vs 動的マッピング（runtime config）

### Research Items

1. **[Research Needed]** Reuters RIC / Bloomberg ticker の標準フォーマット調査
2. **[Research Needed]** 金利商品のレート範囲の業界標準（バリデーション閾値）
3. **[Low Priority]** `pricer_models::MarketProvider` との将来統合パス

---

## Appendix: Requirement-to-Asset Map

| 要件ID | 要件名 | 既存アセット | ギャップ |
|--------|--------|-------------|----------|
| 1.1 | MarketRate struct | - | Missing |
| 1.2 | バリデーション | - | Missing |
| 1.3 | RateType enum | - | Missing |
| 1.4 | QuoteType enum | `adapter_feeds::QuoteType` | Constraint（移動推奨） |
| 1.5 | serde 対応 | パターン確立済み | - |
| 2.1 | RateId type | - | Missing |
| 2.2 | TickerMapping | - | Missing |
| 2.3 | ルックアップ API | - | Missing |
| 2.4 | 標準マッピング | - | Missing |
| 3.1 | MarketRateSet | - | Missing |
| 3.2 | bid/ask/mid 個別保持 | - | Missing |
| 3.3 | get_rate() | - | Missing |
| 3.4 | get_mid_rate() | - | Missing |
| 3.5 | RateType イテレータ | - | Missing |
| 3.6 | stale_rates() | - | Missing |
| 4.1 | InstrumentMapper trait | - | Missing |
| 4.2 | StandardInstrumentMapper | - | Missing |
| 4.3-4.6 | マッピング実装 | `Instrument` enum 存在 | Missing（マッパーのみ） |
| 4.7 | MappingError | - | Missing |
| 5.1 | MarketDataError | `pricer_models::MarketDataError` | Constraint（別名推奨） |
| 5.2-5.3 | バリデーションロジック | - | Missing |
| 5.4 | RateValidator trait | - | Missing |
| 5.5 | StandardRateValidator | - | Missing |
| 6.1 | DataSource enum | - | Missing |
| 6.2 | SourcePriority | - | Missing |
| 6.3-6.4 | マージロジック | - | Missing |
| 7.1 | Clone, Debug | パターン確立済み | - |
| 7.2 | to_instruments() | - | Missing |
| 7.3 | filter_by_currency() | - | Missing |
| 7.4 | as_of() | - | Missing |
| 7.5 | JSON シリアライゼーション | パターン確立済み | - |
