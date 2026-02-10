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
- `serde`, `serde_json` - 利用可能（ワークスペース依存）
- `strum` - 未使用（他のクレートでは広く使用中: 116ファイルでマッチ）

### 1.2 既存アセット（Service Layer）

#### service_gateway ルーター構造
現状: `/api/config` エンドポイントは存在しない

### 1.3 既存アセット（Demo Layer）

#### demo/data/config/ JSONファイル
| ファイル | サイズ | 内容 |
|---------|-------|------|
| `enums.json` | 21行 | 9種類のEnum値配列 |
| `gui_defaults.json` | 81行 | pricing, monte_carlo, bump_sizes, pricer, curve, expansion |
| `rate_index_mapping.json` | 10行 | USD→SOFR, EUR→EURIBOR3M, GBP→SONIA, JPY→TONAR, CHF→SARON |

---

## 2. Requirements Feasibility Analysis

### 2.1 技術的ニーズ

| 要件 | 必要な技術 | 既存で利用可能 |
|------|-----------|---------------|
| Req1: Enumエクスポート | `EnumValues` trait, 全バリアント列挙 | Missing `strum`未導入 |
| Req2: デフォルトエクスポート | `Default` trait → JSON | Complete `serde_json` |
| Req3: 通貨マッピング | HashMap, デフォルト値 | Complete 既存パターンあり |
| Req4: AppConfig構造体 | 統合構造体, `Serialize` | Complete `serde` |
| Req5: /api/config | Axum handler | Complete 既存パターンあり |
| Req6: 非推奨化 | JSONコメント追加 | Complete 手動対応 |

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

---

## 3. Implementation Approach Options

### Option A: 手動Enum列挙
**Trade-offs**: Complete 外部依存なし、Complete 完全な制御（出力形式のカスタマイズ可能）、Missing 手動メンテナンス（新Enumバリアント追加時に更新必要）、Missing コード重複（各Enumで同様の実装）

### Option B: strumマクロ活用
**Trade-offs**: Complete 自動列挙（新バリアント追加時に自動対応）、Complete プロジェクト内で既に広く使用（116ファイル）、Complete タイプセーフ、Missing 外部依存追加、Missing serde rename_all との整合性確保が必要（`strum(serialize_all = "snake_case")`）

### Option C: ハイブリッドアプローチ（推奨）
- Phase 1: 手動実装で最小限の機能提供
- Phase 2: strumへの移行（オプション）

**Trade-offs**: Complete 迅速な初期実装、Complete 外部依存なしで開始可能、Complete 後から改善可能（strumへの移行がオプション）、Missing 初期は手動メンテナンス必要

---

## 4. Effort & Risk Assessment

**Effort**: S（1-3日）
- 既存パターンが明確（serde, Axum handler）
- 新規ファイル数: 2-3
- 変更ファイル数: 4-5
- 外部依存変更: なし（Option C Phase 1）

**Risk**: Low
- 既存機能への影響なし（新規エンドポイント追加のみ）
- 既存テスト影響なし
- A-I-P-S依存ルール違反リスクなし
- フロントエンドとの互換性: `AppConfig` 型が既に定義済み

---

## 5. Recommendations for Design Phase

**推奨アプローチ**: Option C（ハイブリッド）のPhase 1

### Key Decisions for Design
1. **ファイル構成**: `app_config.rs` を新規作成するか、`lib.rs` に直接追加するか
2. **Enum列挙方法**: 初期は手動配列、後にstrumマクロへ移行可能な設計
3. **デフォルト値の階層**: `gui_defaults.json` の構造をそのまま踏襲するか、Rust構造体に合わせるか
4. **rate_index_mapping**: infra_config内蔵 vs 設定ファイルからのロード

---

## Summary

| 項目 | 評価 |
|------|------|
| **Effort** | S（1-3日） |
| **Risk** | Low |
| **推奨アプローチ** | Option C Phase 1（手動実装、strumへの移行オプション） |
| **主要ギャップ** | `EnumValues` trait, `AppConfig` 構造体, `/api/config` handler |
| **制約** | A-I-P-S依存ルール、serde rename_all との整合性 |
