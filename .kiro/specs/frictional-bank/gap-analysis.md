# FrictionalBank ギャップ分析

## エグゼクティブサマリー

| 項目 | 状況 |
|------|------|
| 既存実装率 | **40%** (demo/inputs, demo/outputs実装済み) |
| 新規作成必要 | frictional_bank, gui, data, notebooks |
| 主要ギャップ | オーケストレーター、TUI、サンプルデータ |
| 推奨アプローチ | **ハイブリッド** (既存活用 + 新規作成) |
| 工数見積 | **L** (1-2週間) |
| リスクレベル | **中** |

---

## 1. 現状調査

### 1.1 既存コンポーネント分析

#### ✅ demo/inputs (実装済み)

| モジュール | ファイル | 実装状況 |
|-----------|---------|---------|
| market_data_provider | `mod.rs`, `bloomberg_sim.rs`, `reuters_sim.rs`, `synthetic.rs` | ✅ 完全 |
| trade_source | `mod.rs`, `fpml_generator.rs`, `front_office.rs` | ✅ 完全 |
| file_source | `mod.rs`, `csv_generator.rs` | ✅ 基本実装 (Parquetなし) |

**主要インターフェース:**
```rust
// market_data_provider/mod.rs:18-28
#[async_trait::async_trait]
pub trait MarketDataProvider: Send + Sync {
    async fn start(&self) -> Receiver<MarketQuote>;
    async fn stop(&self);
    async fn snapshot(&self) -> Vec<MarketQuote>;
}
```

**adapter_feedsとの連携:**
- `MarketQuote`型を使用 (`adapter_feeds::MarketQuote`)
- async_channel経由のストリーム配信

#### ✅ demo/outputs (実装済み)

| モジュール | ファイル | 実装状況 |
|-----------|---------|---------|
| regulatory | `mod.rs`, `regulator_api.rs`, `audit_store.rs` | ✅ 完全 |
| settlement | `mod.rs`, `swift_receiver.rs`, `netting_engine.rs` | ✅ 完全 |
| risk_dashboard | `mod.rs`, `websocket_sink.rs`, `metrics_store.rs` | ✅ 完全 |
| report_sink | `mod.rs`, `file_writer.rs`, `email_sender.rs` | ✅ 完全 |

**主要インターフェース:**
```rust
// report_sink/mod.rs:15-18
pub trait ReportSink: Send + Sync {
    fn send(&self, report: &Report) -> Result<(), String>;
}
```

#### ❌ demo/frictional_bank (未実装)

- ディレクトリ存在せず
- オーケストレーターロジック未実装
- 設定管理未実装

#### ❌ demo/gui (未実装)

- ディレクトリ存在せず
- TUI/Web両方未実装

#### ❌ demo/data (未実装)

- ディレクトリ存在せず
- サンプルデータなし

#### ❌ demo/notebooks (未実装)

- ディレクトリ存在せず
- Jupyterノートブックなし

### 1.2 サービス層統合ポイント

#### service_gateway (部分実装)

```rust
// service_gateway/src/rest/handlers.rs
// TODOコメントあり - pricer層との実際の統合未完了
pub async fn price_instrument(...) -> Result<Json<PriceResponse>, ServerError> {
    // TODO: Use pricer_pricing for actual pricing
    // For now, return a placeholder
}
```

**既存エンドポイント:**
- `GET /health` - ヘルスチェック
- `POST /price` - プライシング (プレースホルダー)
- `POST /portfolio` - ポートフォリオプライシング
- `POST /calibrate` - キャリブレーション (プレースホルダー)
- `POST /exposure` - エクスポージャー (プレースホルダー)

#### service_cli (部分実装)

```rust
// service_cli/src/commands/demo.rs
// 3-stage rocketデモ実装済み
pub fn run() -> Result<()> {
    let market = MarketProvider::new();
    let trades = vec![...];
    let results = run_portfolio_pricing(&trades, &market);
}
```

**既存コマンド:** `calibrate`, `check`, `price`, `report`, `demo`

#### service_python (基本実装)

**既存バインディング:**
- `PyVanillaOption` - バニラオプション
- `PyForward` - フォワード
- `PyHullWhite` - Hull-Whiteモデル
- `price_black_scholes()` - Black-Scholesプライシング
- `price_garman_kohlhagen()` - FXオプションプライシング

### 1.3 Pricer層統合ポイント

#### pricer_risk::demo (実装済み)

```rust
// pricer_risk/src/demo.rs
pub struct DemoTrade { id, ccy, model, instrument }
pub fn run_portfolio_pricing(trades: &[DemoTrade], market: &MarketProvider) -> Vec<PricingResultDemo>
```

**特徴:**
- Pull-then-Pushパターン実装
- Rayonによる並列処理
- MarketProvider経由のArcキャッシュ

#### pricer_risk::scenarios (実装済み)

| モジュール | 機能 |
|-----------|------|
| engine.rs | ScenarioEngine |
| shifts.rs | RiskFactorShift |
| presets.rs | PresetScenario |
| aggregator.rs | GreeksAggregator |

---

## 2. 要件実現可能性分析

### 2.1 要件-資産マッピング

| 要件 | 既存資産 | ギャップ |
|------|---------|---------|
| **R1: サンプルデータ** | なし | ❌ Missing - 全データ作成必要 |
| **R2: 仮想入力システム** | demo/inputs (90%) | ⚠️ Partial - Parquet未実装 |
| **R3: 仮想出力システム** | demo/outputs (100%) | ✅ Complete |
| **R4: オーケストレーター** | service_cli demo参考 | ❌ Missing - 新規作成必要 |
| **R5: ターミナルUI** | なし | ❌ Missing - 新規作成必要 |
| **R6: Jupyter連携** | service_python基本 | ⚠️ Partial - ノートブック未作成 |
| **R7: A-I-P-Sフロー** | 全層実装済み | ⚠️ Constraint - 統合テスト必要 |
| **R8: Webダッシュボード** | service_gateway REST | ⚠️ Partial - WebSocket未実装 |
| **R9: 非機能要件** | logging基盤あり | ⚠️ Partial - 設定オーバーライド未検証 |

### 2.2 技術的制約

1. **Cargo.toml未登録**: demo/inputs, demo/outputsはworkspace.membersに未登録
2. **pricer_pricing除外**: service_gatewayはpricer_pricingに依存していない（Enzyme不要）
3. **非同期統合**: demo/inputsはasync_trait使用、service_gatewayはtokio使用

### 2.3 複雑性シグナル

| 要件領域 | 複雑性 | 理由 |
|---------|--------|------|
| サンプルデータ | 低 | 静的ファイル作成 |
| オーケストレーター | 中-高 | 全層統合、ワークフロー制御 |
| TUI | 中 | ratatui学習曲線、状態管理 |
| Jupyter | 低 | 既存バインディング活用 |
| WebSocket | 中 | リアルタイム双方向通信 |

---

## 3. 実装アプローチオプション

### Option A: 既存拡張アプローチ

**概要:** service_cli::demoを拡張してオーケストレーター機能を追加

**変更ファイル:**
- `crates/service_cli/src/commands/demo.rs` - EOD/intraday/stress追加
- `crates/service_cli/src/commands/` - 新コマンド追加
- `crates/service_gateway/src/rest/handlers.rs` - 実統合

**トレードオフ:**
- ✅ 新規ファイル最小
- ✅ 既存パターン活用
- ❌ service_cliの責務肥大化
- ❌ demo固有ロジックがservice層に混入

### Option B: 新規作成アプローチ

**概要:** demo/frictional_bank, demo/guiを完全新規作成

**新規ファイル:**
```text
demo/
├── frictional_bank/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── orchestrator/
│       │   ├── mod.rs
│       │   ├── eod_batch.rs
│       │   ├── intraday.rs
│       │   └── stress_test.rs
│       └── config/
│           └── mod.rs
├── gui/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── tui/
│           ├── mod.rs
│           ├── app.rs
│           ├── dashboard.rs
│           └── ...
├── data/
│   ├── input/
│   ├── config/
│   └── output/
└── notebooks/
    └── *.ipynb
```

**トレードオフ:**
- ✅ 関心の明確な分離
- ✅ demo固有コードの隔離
- ✅ 独立したテスト可能
- ❌ ファイル数増加
- ❌ インターフェース設計必要

### Option C: ハイブリッドアプローチ（推奨）

**概要:** 既存demo/inputs, demo/outputsを活用しつつ、frictional_bank, gui, dataを新規作成

**戦略:**
1. **Phase 1**: Cargo.tomlにdemoクレート登録、data/作成
2. **Phase 2**: frictional_bank作成（既存demo/inputs, outputs使用）
3. **Phase 3**: gui/tui作成（service_gateway REST呼び出し）
4. **Phase 4**: notebooks作成（service_python活用）
5. **Phase 5**: service_gatewayの実統合（オプション）

**変更/新規ファイル:**
- `Cargo.toml` - workspace.members追加
- `demo/frictional_bank/` - 新規作成
- `demo/gui/` - 新規作成
- `demo/data/` - 新規作成
- `demo/notebooks/` - 新規作成

**トレードオフ:**
- ✅ 既存実装（40%）を最大活用
- ✅ 段階的実装可能
- ✅ A-I-P-Sアーキテクチャ準拠を維持
- ⚠️ 統合テストの複雑性

---

## 4. 工数・リスク評価

### 4.1 工数見積

| コンポーネント | 工数 | 詳細 |
|--------------|------|------|
| Cargo.toml設定 | S (1日) | workspace.members追加 |
| demo/data | S (1-2日) | サンプルデータファイル作成 |
| demo/frictional_bank | M (3-5日) | オーケストレーター実装 |
| demo/gui/tui | M (3-5日) | ratatui TUI実装 |
| demo/notebooks | S (2-3日) | Jupyterノートブック作成 |
| 統合テスト | S (1-2日) | E2Eシナリオテスト |
| **合計** | **L (1-2週間)** | - |

### 4.2 リスク評価

| リスク | レベル | 緩和策 |
|--------|--------|--------|
| ratatui学習曲線 | 中 | 公式examplesを参考に |
| 全層統合の複雑性 | 中 | 段階的実装、単体テスト先行 |
| service_gateway実統合 | 低 | pricer_pricingなしで実装可能 |
| Enzyme依存 | 低 | frictional_bankはpricer_pricing除外可 |

### 4.3 研究必要項目

| 項目 | 理由 | 優先度 |
|------|------|--------|
| ratatui state管理 | TUIのリアクティブ更新パターン | 高 |
| tokio/async_trait統合 | demo/inputsとservice_gateway間 | 中 |
| WebSocket双方向通信 | risk_dashboardリアルタイム更新 | 中（Phase 7向け） |

---

## 5. 推奨事項

### 5.1 推奨アプローチ

**Option C: ハイブリッドアプローチ**を推奨

**理由:**
1. 既存demo/inputs, demo/outputsの40%実装を活用
2. FRICTIONAL_BANK_SPEC.mdの7フェーズ構成と整合
3. A-I-P-Sアーキテクチャ準拠を維持しつつ、demo層の独立性確保

### 5.2 設計フェーズへの引継ぎ項目

1. **Cargo.toml workspace構成**: demo/*, frictional_bankの依存関係設計
2. **オーケストレーターインターフェース**: EOD/intraday/stressの共通trait設計
3. **TUI状態管理**: ratatuiのApp state設計
4. **サンプルデータフォーマット**: CSV/JSON/XMLスキーマ定義

### 5.3 優先実装順序

1. ✅ demo/inputs, demo/outputs（既存活用）
2. 🔨 demo/data（サンプルデータ）
3. 🔨 demo/frictional_bank（オーケストレーター）
4. 🔨 demo/gui/tui（ターミナルUI）
5. 🔨 demo/notebooks（Jupyter）
6. ⏳ demo/gui/web（オプション）

---

_作成日: 2026-01-10_
_分析者: Claude_
