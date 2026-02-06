# Gap Analysis: config-consolidation

## 1. Current State Investigation

### 1.1 既存アセット（Infra Layer）

#### infra_config クレート構造
```
crates/infra_config/src/
├── lib.rs           # モジュール定義、prelude
├── error.rs         # ConfigError
├── settings.rs      # Settings, EngineConfig, DatabaseConfig, ServiceConfig, LoggingConfig
├── pricing_config.rs # PricingConfig, PricingMethod, TreeType, MonteCarloParams, TreeParams
└── risk_config.rs   # RiskConfig, GreeksMethod, GreekType, BumpSizes, SecondOrderMode, ShiftType
```

#### 既存Enum定義と属性
| Enum | ファイル | serde設定 | Default |
|------|---------|----------|---------|
| `PricingMethod` | pricing_config.rs:15 | `#[serde(rename_all = "snake_case")]` | `Analytical` |
| `TreeType` | pricing_config.rs:28 | `#[serde(rename_all = "snake_case")]` | `Binomial` |
| `GreeksMethod` | risk_config.rs:12 | `#[serde(rename_all = "snake_case")]` | `Bump` |
| `GreekType` | risk_config.rs:24 | `#[serde(rename_all = "snake_case")]` | なし |
| `SecondOrderMode` | risk_config.rs:136 | `#[serde(rename_all = "snake_case")]` | `Parallel` |
| `ShiftType` | risk_config.rs:170 | `#[serde(rename_all = "snake_case")]` | `Absolute` |

#### 既存Default実装
| 構造体 | 実装場所 | 主要デフォルト値 |
|--------|---------|-----------------|
| `MonteCarloParams` | pricing_config.rs:48 | `num_paths: 10_000`, `num_steps: 252` |
| `TreeParams` | pricing_config.rs:68 | `num_steps: 100`, `tree_type: Binomial` |
| `PricingConfig` | pricing_config.rs:126 | `pricing_method: Analytical`, `parallel_enabled: true` |
| `BumpSizes` | risk_config.rs:98 | `rate: 0.0001`, `vol: 0.01`, `spot: 0.01` |
| `RiskConfig` | risk_config.rs:219 | `greeks_method: Bump`, `second_order_mode: Parallel` |

#### 依存関係（Cargo.toml）
- `serde`, `serde_json` - **利用可能**（ワークスペース依存）
- `strum` - **未使用**（他のクレートでは広く使用中: 116ファイルでマッチ）

### 1.2 既存アセット（Service Layer）

#### service_gateway ルーター構造
```rust
// crates/service_gateway/src/rest/mod.rs
Router::new()
    .route("/health", get(handlers::health))
    .nest("/api", api_routes(state))  // /api/price, /api/curves/*
    .nest("/api/portfolio", portfolio_routes(graph_state))
    .merge(ws_routes(ws_state))
```

**現状**: `/api/config` エンドポイントは**存在しない**

### 1.3 既存アセット（Demo Layer）

#### demo/data/config/ JSONファイル
| ファイル | サイズ | 内容 |
|---------|-------|------|
| `enums.json` | 21行 | 9種類のEnum値配列 |
| `gui_defaults.json` | 81行 | pricing, monte_carlo, bump_sizes, pricer, curve, expansion |
| `rate_index_mapping.json` | 10行 | USD→SOFR, EUR→EURIBOR3M, GBP→SONIA, JPY→TONAR, CHF→SARON |

#### demo/gui フロントエンド
```typescript
// demo/gui/static/src/types/api.ts:18
export interface AppConfig {
  enums: Record<string, EnumValue[]>;
  defaults: Record<string, unknown>;
  rateIndexByCurrency: Record<Currency, string>;
}

// demo/gui/static/src/services/api.ts:58
export async function fetchConfig(): Promise<AppConfig> {
  return fetchJson<AppConfig>(`${API_BASE}/config`);
}
```

**現状**: `fetchConfig()` は `/api/config` を呼び出すが、エンドポイントが存在しないためエラーになる

---

## 2. Requirements Feasibility Analysis

### 2.1 技術的ニーズ

| 要件 | 必要な技術 | 既存で利用可能 |
|------|-----------|---------------|
| Req1: Enumエクスポート | `EnumValues` trait, 全バリアント列挙 | ❌ `strum`未導入 |
| Req2: デフォルトエクスポート | `Default` trait → JSON | ✅ `serde_json` |
| Req3: 通貨マッピング | HashMap, デフォルト値 | ✅ 既存パターンあり |
| Req4: AppConfig構造体 | 統合構造体, `Serialize` | ✅ `serde` |
| Req5: /api/config | Axum handler | ✅ 既存パターンあり |
| Req6: 非推奨化 | JSONコメント追加 | ✅ 手動対応 |

### 2.2 ギャップと制約

| カテゴリ | 項目 | 状態 |
|---------|------|------|
| **Missing** | `EnumValues` trait | 新規実装必要 |
| **Missing** | `EnumRegistry::to_json()` | 新規実装必要 |
| **Missing** | `DefaultsRegistry::to_json()` | 新規実装必要 |
| **Missing** | `CurrencyRateIndexMap` | 新規実装必要 |
| **Missing** | `AppConfig` 構造体 | 新規実装必要 |
| **Missing** | `/api/config` handler | 新規実装必要 |
| **Constraint** | A-I-P-S依存ルール | `infra_config` → `service_gateway` 方向のみ |
| **Constraint** | serde rename_all | 既存Enumは `snake_case` 出力 |

### 2.3 複雑性シグナル

- **単純なCRUD**: ❌
- **アルゴリズムロジック**: 低（Enum列挙、JSON変換）
- **ワークフロー**: 低
- **外部統合**: 低（フロントエンドのみ）

---

## 3. Implementation Approach Options

### Option A: 手動Enum列挙

**概要**: 各Enumに対して手動で `EnumValues` を実装

**変更ファイル**:
- `crates/infra_config/src/lib.rs` - `AppConfig`, `EnumRegistry`, `DefaultsRegistry` 追加
- `crates/infra_config/src/pricing_config.rs` - `EnumValues` 実装追加
- `crates/infra_config/src/risk_config.rs` - `EnumValues` 実装追加
- `crates/service_gateway/src/rest/handlers/mod.rs` - config handler追加
- `crates/service_gateway/src/rest/mod.rs` - ルート追加

**Trade-offs**:
- ✅ 外部依存なし
- ✅ 完全な制御（出力形式のカスタマイズ可能）
- ❌ 手動メンテナンス（新Enumバリアント追加時に更新必要）
- ❌ コード重複（各Enumで同様の実装）

### Option B: strumマクロ活用

**概要**: `strum` クレートの `EnumIter` + `IntoStaticStr` を使用して自動列挙

**変更ファイル**:
- `crates/infra_config/Cargo.toml` - `strum`, `strum_macros` 依存追加
- `crates/infra_config/src/pricing_config.rs` - derive属性追加
- `crates/infra_config/src/risk_config.rs` - derive属性追加
- `crates/infra_config/src/lib.rs` - `AppConfig` 実装
- `crates/service_gateway/src/rest/handlers/mod.rs` - config handler追加
- `crates/service_gateway/src/rest/mod.rs` - ルート追加

**Trade-offs**:
- ✅ 自動列挙（新バリアント追加時に自動対応）
- ✅ プロジェクト内で既に広く使用（116ファイル）
- ✅ タイプセーフ
- ❌ 外部依存追加
- ❌ serde rename_all との整合性確保が必要（`strum(serialize_all = "snake_case")`）

### Option C: ハイブリッドアプローチ（推奨）

**概要**:
- Phase 1: 手動実装で最小限の機能提供
- Phase 2: strumへの移行（オプション）

**変更ファイル（Phase 1）**:
- `crates/infra_config/src/app_config.rs` - 新規ファイル（`AppConfig`, registries）
- `crates/infra_config/src/lib.rs` - モジュール追加
- `crates/service_gateway/src/rest/handlers/config.rs` - 新規ファイル
- `crates/service_gateway/src/rest/handlers/mod.rs` - handler追加
- `crates/service_gateway/src/rest/mod.rs` - ルート追加

**Trade-offs**:
- ✅ 迅速な初期実装
- ✅ 外部依存なしで開始可能
- ✅ 後から改善可能（strumへの移行がオプション）
- ❌ 初期は手動メンテナンス必要

---

## 4. Effort & Risk Assessment

### Effort: **S（1-3日）**
- 既存パターンが明確（serde, Axum handler）
- 新規ファイル数: 2-3
- 変更ファイル数: 4-5
- 外部依存変更: なし（Option C Phase 1）

### Risk: **Low**
- 既存機能への影響なし（新規エンドポイント追加のみ）
- 既存テスト影響なし
- A-I-P-S依存ルール違反リスクなし
- フロントエンドとの互換性: `AppConfig` 型が既に定義済み

---

## 5. Recommendations for Design Phase

### 推奨アプローチ
**Option C（ハイブリッド）のPhase 1**を推奨

### Key Decisions for Design
1. **ファイル構成**: `app_config.rs` を新規作成するか、`lib.rs` に直接追加するか
2. **Enum列挙方法**: 初期は手動配列、後にstrumマクロへ移行可能な設計
3. **デフォルト値の階層**: `gui_defaults.json` の構造をそのまま踏襲するか、Rust構造体に合わせるか
4. **rate_index_mapping**: infra_config内蔵 vs 設定ファイルからのロード

### Research Items（設計フェーズで詳細化）
- なし（既存パターンで十分対応可能）

### フロントエンド互換性確認
```typescript
// 期待される AppConfig 形式
{
  enums: {
    pricing_method: ["analytical", "monte_carlo", "tree"],
    greeks_method: ["aad", "bump"],
    // ...
  },
  defaults: {
    pricing: { curve_rate: 0.05, volatility: 0.20 },
    monte_carlo: { num_paths: 10000, num_steps: 252 },
    // ...
  },
  rateIndexByCurrency: {
    USD: "SOFR",
    EUR: "ESTR",
    // ...
  }
}
```

---

## Summary

| 項目 | 評価 |
|------|------|
| **Effort** | S（1-3日） |
| **Risk** | Low |
| **推奨アプローチ** | Option C Phase 1（手動実装、strumへの移行オプション） |
| **主要ギャップ** | `EnumValues` trait, `AppConfig` 構造体, `/api/config` handler |
| **制約** | A-I-P-S依存ルール、serde rename_all との整合性 |
