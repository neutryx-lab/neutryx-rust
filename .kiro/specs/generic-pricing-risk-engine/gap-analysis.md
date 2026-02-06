# Gap Analysis Report: generic-pricing-risk-engine

## 1. 現状調査

### 1.1 既存アセットマッピング

#### 汎用プライシングエンジン（部分的に実装済み）

| コンポーネント | パス | 状態 | 備考 |
|-------------|------|------|------|
| `GenericPricer` | `pricer_pricing::generic_pricer::pricer` | ✅ 実装済み | 単一取引pricing、standalone/integrated両モード |
| `BatchPricer` | `pricer_pricing::generic_pricer::batch` | ✅ 実装済み | Rayon並列処理、`BatchStats`統計情報 |
| `ModelConfig` | `pricer_pricing::generic_pricer::config` | ✅ 実装済み | MC paths/steps/seed設定、Builder pattern |
| `PricerConfig` | `pricer_pricing::generic_pricer::config` | ✅ 実装済み | Greeks設定、デフォルト通貨 |
| `PricingResult` | `pricer_pricing::generic_pricer::result` | ✅ 実装済み | Leg/Cashflowレベル分解 |

#### Greeks計算基盤（要移行）

| コンポーネント | 現在パス | 状態 | 備考 |
|-------------|---------|------|------|
| `GreeksConfig` | `pricer_pricing::greeks::config` | ✅ 実装済み | bump size設定、Builder pattern |
| `GreeksMode` | `pricer_pricing::greeks::config` | ✅ 実装済み | BumpRevalue/NumDual/EnzymeAAD |
| `GreeksResult<T>` | `pricer_pricing::greeks::result` | ✅ 実装済み | Generic AD対応 |
| `IrsGreeksCalculator` | `pricer_pricing::irs_greeks` | ✅ 実装済み | AAD/Bump両対応、Lazy評価 |
| `BenchmarkRunner` | `pricer_pricing::irs_greeks::benchmark` | ✅ 実装済み | 性能比較ハーネス |

#### シナリオ・リスク基盤

| コンポーネント | パス | 状態 | 備考 |
|-------------|------|------|------|
| `ScenarioEngine` | `pricer_risk::scenarios::engine` | ✅ 実装済み | シナリオ実行 |
| `GreeksAggregator` | `pricer_risk::scenarios::aggregator` | ✅ 実装済み | ポートフォリオ集約 |
| `RiskFactorShift` | `pricer_risk::scenarios::shifts` | ✅ 実装済み | リスクファクターシフト |
| `CurveShifter` | `pricer_risk::scenarios::curve_shifts` | ✅ 実装済み | カーブシフト操作 |

#### 設定・データローダー

| コンポーネント | パス | 状態 | 備考 |
|-------------|------|------|------|
| `Settings` | `infra_config::settings` | ✅ 実装済み | TOML/YAML/Env統合 |
| `EngineConfig` | `infra_config::settings` | ✅ 実装済み | thread_pool, memory_limit, mc_paths |
| `CsvLoader` | `adapter_loader::csv_loader` | ✅ 実装済み | CSV読み込み |
| `CsaTerms` | `adapter_loader::csa` (re-export) | ✅ 実装済み | infra_domainから再エクスポート |

#### Demo Web API

| コンポーネント | パス | 状態 | 備考 |
|-------------|------|------|------|
| `generic_pricer_handlers.rs` | `demo/gui/src/web/` | ✅ 実装済み | REST API パターン |
| `pricer_types.rs` | `demo/gui/src/web/` | ✅ 実装済み | Request/Response型 |
| `scenario_handlers.rs` | `demo/gui/src/web/` | ✅ 実装済み | シナリオAPI |

### 1.2 既存パターン・規約

```
パターン: Builder Pattern
使用箇所: ModelConfig, PricerConfig, GreeksConfig
例: ModelConfig::builder().num_paths(50_000).build()?

パターン: Dual-mode support (standalone/integrated)
使用箇所: GenericPricer
例: #[cfg(feature = "l1l2-integration")] で条件コンパイル

パターン: Web Handler分離
使用箇所: demo/gui/web/
例: *_handlers.rs + *_types.rs 構造
```

### 1.3 統合サーフェス

- **MarketProvider**: `pricer_models::market::MarketProvider` (Arc共有)
- **Trade型**: `infra_domain::trade::Trade` (CF-expanded形式)
- **Curve/Vol型**: `CurveEnum`, `VolSurfaceEnum` (static dispatch)
- **CSA型**: `infra_domain::counterparty::CsaTerms`

---

## 2. 要件実現可能性分析

### 要件-アセットマッピング

| 要件 | 技術ニーズ | 既存アセット | ギャップ |
|------|-----------|-------------|---------|
| Req 1: 設定ファイル構造 | TOML/JSON parser, validation | `infra_config::Settings` | **新規**: `PricingConfig`, `RiskConfig` スキーマ |
| Req 2: データローダー | JSON loader, glob support | `CsvLoader` | **新規**: `JsonLoader`, Trade/MarketData/CSAローダー |
| Req 3: 単一取引pricing | Pricer, MarketProvider | `GenericPricer`, `MarketProvider` | **拡張**: 設定駆動型インターフェース |
| Req 4: ポートフォリオpricing | Batch pricing, parallel | `BatchPricer`, Rayon | **拡張**: 集約機能（通貨/netting set/book） |
| Req 5: リスク計算エンジン | AAD/Bump選択, Greeks | `GreeksMode`, `GreeksConfig` | **新規**: `RiskEngine` facade in `pricer_risk` |
| Req 6: モジュール移行 | Crate restructuring | `greeks/`, `irs_greeks/` | **移行**: L3→L4への移動 |
| Req 7: リスク設定柔軟性 | Per-factor bump, scenarios | `RiskFactorShift`, `CurveShifter` | **拡張**: 設定ファイル統合 |
| Req 8: エラーハンドリング | Structured errors | `PricingError`, `GreeksError` | **拡張**: 診断データ追加 |
| Req 9: Service統合 | Async interface, serde | Demo handlers | **新規**: `pricer_risk` async wrappers |

### ギャップ詳細

#### Missing - 新規実装が必要

1. **計算設定スキーマ** (`PricingConfig`, `RiskConfig`)
   - 現状: `ModelConfig`/`PricerConfig`はコード内設定のみ
   - 必要: TOML/JSONファイルからの読み込み、パス指定、バリデーション

2. **JSONデータローダー**
   - 現状: CSVローダーのみ
   - 必要: Trade JSON、Market Data JSON、CSA JSONローダー

3. **統合RiskEngine**
   - 現状: Greeks計算がL3(`pricer_pricing`)に分散
   - 必要: L4(`pricer_risk`)に統合されたfacade

#### Constraint - アーキテクチャ制約

1. **依存関係ルール**: `pricer_risk`(L4)は`pricer_pricing`(L3)に依存可能だが、逆は不可
2. **Feature flag**: `l1l2-integration`がないと`irs_greeks`が使用不可
3. **Enzyme依存**: AADモードは`enzyme-ad` featureとnightly Rustが必要

#### Unknown - 設計フェーズで調査が必要

1. **移行戦略**: `greeks/`モジュール移行時の既存コードへの影響範囲
2. **Enzyme統合**: L4でのEnzyme呼び出しパターン（L3への委譲 vs 直接呼び出し）

---

## 3. 実装アプローチオプション

### Option A: 既存コンポーネント拡張

**対象**: Req 1, 3, 4, 7, 8

**アプローチ**:
- `infra_config::Settings`に`PricingConfig`, `RiskConfig`セクションを追加
- `GenericPricer`に設定ファイル読み込みコンストラクタを追加
- `BatchPricer`に集約機能を追加

**変更ファイル**:
- `crates/infra_config/src/settings.rs` - 設定スキーマ追加
- `crates/pricer_pricing/src/generic_pricer/pricer.rs` - ファイル読み込み
- `crates/pricer_pricing/src/generic_pricer/batch.rs` - 集約機能

**Trade-offs**:
- ✅ 既存パターン活用、学習コスト低
- ✅ 既存テスト資産の再利用
- ❌ `generic_pricer`モジュールの肥大化リスク
- ❌ L3/L4境界が曖昧になる可能性

### Option B: 新規コンポーネント作成

**対象**: Req 2, 5, 6, 9

**アプローチ**:
- `adapter_loader`に`json/`サブモジュールを新規作成
- `pricer_risk`に`engine/`モジュールを新規作成（`RiskEngine` facade）
- `greeks/`, `irs_greeks/`をL3からL4へ移動

**新規ファイル**:
```
crates/adapter_loader/src/json/
├── mod.rs
├── trade_loader.rs
├── market_loader.rs
└── csa_loader.rs

crates/pricer_risk/src/engine/
├── mod.rs
├── risk_engine.rs
├── config.rs
└── result.rs

crates/pricer_risk/src/greeks/     # L3から移行
└── (greeks/* files)

crates/pricer_risk/src/irs_greeks/ # L3から移行
└── (irs_greeks/* files)
```

**Trade-offs**:
- ✅ 責務の明確な分離（A-I-P-Sアーキテクチャ遵守）
- ✅ L4がリスク計算の中心となる
- ❌ モジュール移行は破壊的変更
- ❌ 既存の`pricer_pricing`ユーザーへの影響

### Option C: ハイブリッドアプローチ（推奨）

**フェーズ1**: 設定・ローダー拡張（Option Aベース）
- `infra_config`に`PricingConfig`, `RiskConfig`追加
- `adapter_loader`にJSONローダー追加
- 既存`GenericPricer`/`BatchPricer`の設定駆動化

**フェーズ2**: モジュール移行（Option Bベース）
- `greeks/`を`pricer_risk`にコピー
- `pricer_pricing::greeks`に`deprecated`属性付き再エクスポート
- 下流コード移行期間（1リリースサイクル）

**フェーズ3**: RiskEngine統合
- `pricer_risk::engine::RiskEngine` facade作成
- 既存`scenarios/`モジュールとの統合
- Service層async wrapper追加

**Trade-offs**:
- ✅ 段階的移行でリスク軽減
- ✅ 後方互換性維持期間あり
- ✅ 各フェーズで検証可能
- ❌ 実装期間が長くなる
- ❌ 一時的な重複コード

---

## 4. 実装複雑度とリスク

### 工数見積

| 要件 | 工数 | 根拠 |
|------|------|------|
| Req 1: 設定ファイル | S | 既存Settings拡張、serdeパターン確立済み |
| Req 2: データローダー | M | 新規モジュール、JSONスキーマ設計必要 |
| Req 3: 単一取引pricing | S | 既存GenericPricer拡張のみ |
| Req 4: ポートフォリオpricing | M | 集約ロジック追加、テスト拡充必要 |
| Req 5: RiskEngine | M | 新規facade、既存コンポーネント統合 |
| Req 6: モジュール移行 | L | 破壊的変更、依存関係更新、テスト移行 |
| Req 7: リスク設定柔軟性 | S | 既存CurveShifter/RiskFactorShift活用 |
| Req 8: エラーハンドリング | S | 既存パターン拡張 |
| Req 9: Service統合 | M | async wrapper、既存handlerパターン活用 |

**総合工数**: **L (1-2週間)**

### リスク評価

| リスク | 確率 | 影響 | 緩和策 |
|--------|------|------|--------|
| モジュール移行による破壊的変更 | 高 | 中 | deprecation期間、段階的移行 |
| Enzyme依存によるビルド複雑化 | 中 | 中 | feature flagで分離済み |
| Service層無効化状態での統合テスト困難 | 中 | 低 | demo/guiハンドラーでテスト |
| 設定スキーマの後方互換性 | 低 | 低 | バージョニング、デフォルト値 |

**総合リスク**: **Medium**

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option C (ハイブリッド)** を採用し、以下の順序で実装:

1. **Phase 1**: 設定基盤（Req 1, 7）
2. **Phase 2**: データローダー（Req 2）
3. **Phase 3**: Pricer拡張（Req 3, 4, 8）
4. **Phase 4**: モジュール移行（Req 6）
5. **Phase 5**: RiskEngine統合（Req 5, 9）

### 設計フェーズでの調査項目

1. **移行戦略詳細**: `greeks/`移行時のAPI互換レイヤー設計
2. **Enzyme呼び出しパターン**: L4からL3へのEnzyme委譲 vs 直接呼び出し
3. **JSONスキーマ**: Trade/MarketData/CSAのJSONスキーマ定義
4. **Async boundary**: sync pricer kernelとasync service layerの境界設計

### 既存仕様との整合性

- `generic-pricer-engine` spec (2026-01-23完了): 本スペックの基盤
- `curve-bootstrap-engine` spec: MarketProvider/CurveSet統合
- `rate-index-pricing-integration` spec: RateIndex pricing統合

---

_Generated: 2026-01-25_
_Analysis based on codebase state as of commit 251fba6_
