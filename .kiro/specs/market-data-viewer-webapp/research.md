# Gap Analysis Report: market-data-viewer-webapp

## Executive Summary

本分析は、Frictional Bank Web App 向けマーケットデータ閲覧機能の実装ギャップを調査した結果をまとめる。既存の `infra_domain::market` モジュールと `demo/gui` Web インフラを活用することで、大部分の要件を効率的に実装可能である。

### 主要な発見

| カテゴリ | 既存充足度 | ギャップの深刻度 |
|---------|-----------|----------------|
| マーケットレートデータモデル | 高 (90%) | 低 |
| Instrument マッピング | 高 (85%) | 低 |
| Convention 定義 | 高 (95%) | 低 |
| Web API インフラ | 中 (70%) | 中 |
| フロントエンド UI | 低 (30%) | 中 |
| サンプルデータセット | 低 (20%) | 中 |

---

## 1. 既存コンポーネント分析

### 1.1 マーケットレートインフラ (`infra_domain::market`)

**ファイル**: `crates/infra_domain/src/market/`

#### 利用可能なコンポーネント

| コンポーネント | ファイル | 機能 | 要件との対応 |
|--------------|---------|------|-------------|
| `MarketRate` | `rate.rs` | レート値、タイムスタンプ、ソース情報保持 | Req 1: AC 4 |
| `RateId` | `rate_id.rs` | Currency + Tenor + RateType 複合キー | Req 1: AC 1-3 |
| `RateType` | `rate_type.rs` | Deposit, Swap, Ois, FxSpot 等 9種類 | Req 1: AC 3 |
| `QuoteType` | `quote_type.rs` | Bid, Ask, Mid, Last | Req 1: AC 4 |
| `DataSource` | `data_source.rs` | Bloomberg, Reuters, Internal 等 | Req 1: AC 4 |
| `MarketRateSet` | `rate_set.rs` | レートコレクション、O(1)検索、フィルタリング | Req 4: AC 3 |
| `StandardInstrumentMapper` | `mapper.rs` | Rate → Instrument 変換 | Req 2: AC 1-3 |
| `RateIndex` | `rate_index.rs` | SOFR, EURIBOR, TONAR 等インデックス定義 | Req 1: AC 2 |

#### コード例: 既存の MarketRateSet 機能

```rust
// rate_set.rs:389-403 - 通貨フィルタリング機能
pub fn filter_by_currency(&self, currency: Currency) -> MarketRateSet

// rate_set.rs:275-279 - レートタイプフィルタリング機能
pub fn rates_by_type(&self, rate_type: RateType) -> impl Iterator<Item = &MarketRate>

// rate_set.rs:315-330 - Stale レート検出機能
pub fn stale_rates(&self, threshold: Duration) -> Vec<RateId>

// rate_set.rs:561-601 - Instrument 変換機能
pub fn to_instruments<M: InstrumentMapper>(&self, mapper: &M, valuation_date: Date)
    -> (Vec<Instrument>, Vec<(RateId, MarketRateError)>)
```

**評価**: 基盤は十分に整備されている。`serde` feature が有効な場合、JSON シリアライズもサポート済み。

### 1.2 Convention インフラ (`infra_domain::trade::convention`)

**ファイル**: `crates/infra_domain/src/trade/convention/`

#### 利用可能なコンポーネント

| コンポーネント | ファイル | プリセット | 要件との対応 |
|--------------|---------|-----------|-------------|
| `SwapConvention` | `swap.rs` | `usd_sofr()`, `eur_euribor_6m()`, `jpy_tonar()`, `gbp_sonia()` | Req 3: AC 3 |
| `FxConvention` | `fx.rs` | `usd_jpy()`, `eur_usd()`, `gbp_usd()` | Req 3: AC 3 |
| `ConventionSet` | `convention_set.rs` | `usd_standard()`, `eur_standard()`, `jpy_standard()` | Req 3: AC 1-2 |

#### Convention 詳細フィールド

```rust
// swap.rs - SwapConvention fields
pub struct SwapConvention {
    pub fixed_day_count: DayCounter,
    pub float_day_count: DayCounter,
    pub fixed_frequency: Frequency,
    pub float_frequency: Frequency,
    pub calendar: CalendarId,
    pub business_day_convention: BusinessDayConvention,
    pub spot_lag: u32,
    pub float_index: RateIndex,
}

// fx.rs - FxConvention fields
pub struct FxConvention {
    pub spot_days: u32,
    pub calendar: CalendarId,
    pub business_day_convention: BusinessDayConvention,
}
```

**評価**: Convention プリセットが整備されており、要件 3 の大部分をカバー。

### 1.3 Web App インフラ (`demo/gui`)

**ファイル**: `demo/gui/src/web/`

#### 既存パターン

| パターン | 実装例 | 活用方法 |
|---------|-------|---------|
| REST エンドポイント | `/api/portfolio`, `/api/risk` | `/api/market-data/*` ルート追加 |
| JSON レスポンス | `handlers.rs` | `MarketRate` シリアライズ |
| AppState 共有 | `mod.rs:212-230` | `MarketRateSet` キャッシュ追加 |
| 静的ファイル配信 | `ServeDir` | `index.html` ナビゲーション追加 |

#### 既存のハンドラーパターン (`handlers.rs`)

```rust
// 既存パターン: portfolio endpoint
pub async fn get_portfolio(State(state): State<Arc<AppState>>) -> Json<PortfolioResponse>

// 既存パターン: pricing endpoint
pub async fn price_instrument(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PriceRequest>,
) -> Result<Json<PriceResponse>, StatusCode>
```

**評価**: Axum ベースの REST API パターンが確立されており、新規エンドポイント追加は容易。

### 1.4 フロントエンド (`demo/gui/static`)

#### 既存ライブラリ

| ライブラリ | 用途 | 要件での活用 |
|-----------|-----|-------------|
| Chart.js | グラフ描画 | レート推移表示（オプション） |
| D3.js (複数モジュール) | データ可視化 | テーブルソート、フィルタリング |
| XLSX | Excel エクスポート | Req 7: CSV/Excel エクスポート |
| jsPDF | PDF 生成 | 将来拡張用 |

#### 既存 UI パターン (`index.html`, `app.js`)

- Glass morphism デザイン
- モーダルダイアログ
- Toast 通知
- レスポンシブレイアウト
- Font Awesome アイコン

**評価**: リッチな UI コンポーネントが整備されているが、マーケットデータ専用画面は新規作成が必要。

---

## 2. ギャップ分析

### 2.1 データモデルギャップ

| 要件 | ギャップ | 対応策 | 優先度 |
|-----|---------|-------|-------|
| Req 1: AC 2 - デフォルトレートセット | サンプルデータなし | `sample_rate_set()` 関数作成 | 高 |
| Req 2: AC 2 - 詳細表示用 DTO | Web 用 DTO なし | `MarketRateDetailResponse` 型追加 | 中 |
| Req 3: AC 4 - ユーザーフレンドリー表示 | enum → 文字列変換なし | `Display` trait 実装 or DTO | 中 |

#### 必要な新規型

```rust
// demo/gui/src/web/market_types.rs (新規)

#[derive(Serialize)]
pub struct MarketRateResponse {
    pub id: String,           // "USD-3M-Deposit"
    pub currency: String,
    pub tenor: String,        // "3M"
    pub rate_type: String,    // "Deposit"
    pub value: f64,
    pub quote_type: String,
    pub timestamp: i64,
    pub source: String,
    pub is_stale: bool,
}

#[derive(Serialize)]
pub struct MarketRateDetailResponse {
    pub rate: MarketRateResponse,
    pub instrument: Option<InstrumentResponse>,
    pub convention: Option<ConventionResponse>,
}

#[derive(Serialize)]
pub struct ConventionResponse {
    pub convention_type: String,
    pub fields: Vec<ConventionField>,
}

#[derive(Serialize)]
pub struct ConventionField {
    pub label: String,
    pub value: String,
}
```

### 2.2 API エンドポイントギャップ

| エンドポイント | 既存 | ギャップ |
|--------------|-----|---------|
| `GET /api/market-data/rates` | なし | 新規実装 |
| `GET /api/market-data/rates/{id}` | なし | 新規実装 |
| `GET /api/market-data/conventions` | なし | 新規実装 |
| `GET /api/market-data/conventions/{id}` | なし | 新規実装 |

#### 実装アプローチ

```rust
// demo/gui/src/web/market_handlers.rs (新規)

pub async fn get_market_rates(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MarketRateQuery>,
) -> Json<Vec<MarketRateResponse>>

pub async fn get_market_rate_detail(
    State(state): State<Arc<AppState>>,
    Path(rate_id): Path<String>,
) -> Result<Json<MarketRateDetailResponse>, StatusCode>

pub async fn get_conventions(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<ConventionResponse>>
```

### 2.3 フロントエンドギャップ

| コンポーネント | 既存 | ギャップ |
|--------------|-----|---------|
| Market Data ページ | なし | 新規 HTML セクション |
| ナビゲーションリンク | なし | サイドバー/タブ追加 |
| レートテーブル | なし | D3/vanilla JS 実装 |
| 詳細パネル | なし | モーダル or サイドパネル |
| フィルターコントロール | なし | 通貨/タイプセレクタ |
| エクスポートボタン | PDF 系のみ | CSV/JSON 追加 |

#### UI 実装オプション

**Option A: 単一ページ内セクション追加**
- `index.html` に新規セクション追加
- 既存の glass-card パターン流用
- 実装コスト: 低

**Option B: 専用ページ作成**
- `market-data.html` 新規作成
- より柔軟なレイアウト
- 実装コスト: 中

**推奨**: Option A（既存パターンとの一貫性）

### 2.4 サンプルデータセットギャップ

現在、デモ用のマーケットレートデータセットが存在しない。

#### 必要なサンプルデータ

| 通貨 | インデックス | テナー | レートタイプ |
|-----|------------|-------|------------|
| USD | SOFR | ON, 1W, 1M, 3M, 6M, 1Y | Ois |
| USD | - | 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y | Swap |
| EUR | EURIBOR 3M/6M | 1M, 3M, 6M, 1Y | Deposit |
| EUR | - | 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y | Swap |
| JPY | TONAR | ON, 1W, 1M, 3M, 6M, 1Y | Ois |
| JPY | - | 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y | Swap |
| - | - | USDJPY, EURUSD | FxSpot |

**実装**: `demo/gui/src/web/sample_data.rs` にハードコードまたは JSON ファイルから読み込み

---

## 3. 統合ポイント

### 3.1 AppState 拡張

```rust
// demo/gui/src/web/mod.rs - AppState に追加
pub struct AppState {
    // 既存フィールド...

    /// Market rate cache (新規)
    pub market_rates: RwLock<MarketRateSet>,
    /// Conventions cache (新規)
    pub conventions: RwLock<HashMap<String, ConventionSet>>,
}
```

### 3.2 ルーター拡張

```rust
// demo/gui/src/web/mod.rs - build_router に追加
let api_routes = Router::new()
    // 既存ルート...

    // Market Data API (新規)
    .route("/market-data/rates", get(market_handlers::get_market_rates))
    .route("/market-data/rates/:id", get(market_handlers::get_market_rate_detail))
    .route("/market-data/conventions", get(market_handlers::get_conventions))
    .route("/market-data/conventions/:id", get(market_handlers::get_convention_detail));
```

### 3.3 依存関係

```
demo/gui
├── infra_domain (既存依存)
│   ├── market (MarketRate, MarketRateSet, RateIndex)
│   ├── trade::convention (SwapConvention, FxConvention, ConventionSet)
│   └── time (Date, Tenor)
└── serde (JSON シリアライズ)
```

---

## 4. 実装アプローチ評価

### 4.1 アプローチ A: 最小実装

**スコープ**:
- REST API エンドポイント 4 個
- ハードコードサンプルデータ
- 既存 index.html にテーブル追加
- 基本フィルタリング（通貨）

**利点**:
- 実装コスト最小
- 既存パターンとの一貫性

**欠点**:
- 拡張性に限界
- UI カスタマイズ制限

### 4.2 アプローチ B: フル実装

**スコープ**:
- REST API エンドポイント 6 個
- JSON 設定ファイルからのデータ読み込み
- 専用 Market Data セクション
- 高度なフィルタリング・ソート
- リフレッシュ・エクスポート機能

**利点**:
- 要件完全カバー
- 将来拡張容易

**欠点**:
- 実装コスト高
- テスト工数増

### 4.3 推奨: ハイブリッドアプローチ

**フェーズ 1** (MVP):
- API: `/api/market-data/rates`, `/api/market-data/rates/{id}`
- サンプルデータ: ハードコード
- UI: 基本テーブル + 通貨フィルタ

**フェーズ 2** (拡張):
- API: `/api/market-data/conventions`
- UI: 詳細パネル + Convention 表示
- エクスポート: CSV/JSON

---

## 5. 技術的考慮事項

### 5.1 パフォーマンス

| 項目 | 要件 | 対応策 |
|-----|-----|-------|
| 初期読み込み | < 2秒 | サンプルデータをメモリキャッシュ |
| テーブル描画 | 500 行 | 仮想スクロールまたはページング |
| API レスポンス | < 500ms | HashMap ベースの O(1) 検索 |

### 5.2 シリアライゼーション

`infra_domain` の `serde` feature を有効にする必要あり:

```toml
# demo/gui/Cargo.toml
[dependencies]
infra_domain = { path = "../../crates/infra_domain", features = ["serde"] }
```

### 5.3 エラーハンドリング

既存の `StatusCode` ベースのエラーレスポンスパターンを踏襲:

```rust
pub async fn get_market_rate_detail(...) -> Result<Json<...>, StatusCode> {
    // 404 for not found
    // 500 for internal errors
}
```

---

## 6. 未解決事項

| 項目 | 質問 | 影響 |
|-----|-----|-----|
| データソース | デモ用サンプルデータの具体的な値は？ | サンプルデータ実装 |
| 更新頻度 | 手動リフレッシュのみ or 自動更新も必要？ | WebSocket 連携検討 |
| FX データ | FxSpot/FxForward の Instrument マッピングは必要？ | `StandardInstrumentMapper` 拡張 |
| ナビゲーション | 既存サイドバーに追加 or タブ切り替え？ | UI 設計 |

---

## 7. 次のステップ

1. ~~**デザインフェーズ**: 詳細な API 仕様とコンポーネント設計~~ ✅ 完了
2. **サンプルデータ定義**: 具体的なレート値の決定
3. **UI モックアップ**: テーブルレイアウトとフィルターデザイン
4. **実装タスク分割**: TDD ベースのタスクリスト作成

---

## 8. Design Decisions

本セクションはデザインフェーズで決定された事項を記録する。

### 8.1 Architecture Decision Records

| 決定 ID | 決定内容 | 根拠 | 日付 |
|--------|---------|-----|------|
| ADR-001 | 既存 index.html にセクション追加（Option A） | 既存 UI パターンとの一貫性、実装コスト最小化 | 2026-01-22 |
| ADR-002 | ハードコードサンプルデータ採用 | MVP フェーズでの迅速な実装、将来的に JSON 設定ファイル化可能 | 2026-01-22 |
| ADR-003 | RwLock<Option<MarketRateSet>> による遅延初期化 | 初期起動時間短縮、オンデマンドデータ生成 | 2026-01-22 |
| ADR-004 | camelCase JSON シリアライズ | 既存 pricer_types.rs パターン踏襲、JavaScript 互換性 | 2026-01-22 |

### 8.2 Resolved Questions

| 項目 | 決定 | 影響 |
|-----|-----|-----|
| UI 配置 | 既存 index.html 内セクション | フロントエンド実装簡素化 |
| データソース | ハードコード（sample_data.rs） | 将来的に JSON 設定ファイル化を検討 |
| 更新頻度 | 手動リフレッシュのみ（MVP） | WebSocket は Phase 2 以降で検討 |
| ナビゲーション | 既存タブ切り替えパターン | 新規タブ「Market Data」追加 |

### 8.3 Sample Rate Values

デザインフェーズで確定したサンプルレート値：

| Currency | Index | Tenor | Rate Type | Value |
|----------|-------|-------|-----------|-------|
| USD | SOFR | ON | Ois | 4.33% |
| USD | SOFR | 1W | Ois | 4.34% |
| USD | SOFR | 1M | Ois | 4.40% |
| USD | SOFR | 3M | Ois | 4.55% |
| USD | SOFR | 6M | Ois | 4.65% |
| USD | SOFR | 1Y | Ois | 4.75% |
| USD | - | 2Y | Swap | 4.25% |
| USD | - | 5Y | Swap | 4.10% |
| USD | - | 10Y | Swap | 4.05% |
| USD | - | 30Y | Swap | 4.15% |
| EUR | EURIBOR | 3M | Deposit | 3.65% |
| EUR | EURIBOR | 6M | Deposit | 3.75% |
| EUR | - | 2Y | Swap | 3.00% |
| EUR | - | 10Y | Swap | 2.85% |
| JPY | TONAR | ON | Ois | 0.00% |
| JPY | TONAR | 6M | Ois | 0.15% |
| JPY | - | 10Y | Swap | 1.05% |
| - | - | USDJPY | FxSpot | 150.25 |
| - | - | EURUSD | FxSpot | 1.0850 |

---

## References

- [infra_domain/market/rate_set.rs](../../crates/infra_domain/src/market/rate_set.rs) - MarketRateSet 実装
- [infra_domain/market/mapper.rs](../../crates/infra_domain/src/market/mapper.rs) - StandardInstrumentMapper 実装
- [infra_domain/trade/convention/](../../crates/infra_domain/src/trade/convention/) - Convention 定義
- [demo/gui/src/web/mod.rs](../../demo/gui/src/web/mod.rs) - Web App インフラ
- [demo/gui/static/index.html](../../demo/gui/static/index.html) - フロントエンドテンプレート
