# Gap Analysis: pricer-trade-expansion-ui

## 概要

本ドキュメントは、Frictional Bank Web App の Pricer 画面拡張（Instrument 選択・CF 展開 Trade 生成・表示）に関する実装ギャップ分析を行う。

---

## 1. 現状調査

### 1.1 関連アセットのスキャン

#### バックエンド（Rust）

| モジュール | 場所 | 状態 | 関連性 |
|-----------|------|------|--------|
| **TradeBuilder/LegBuilder** | `crates/infra_master/src/trade/builder.rs` | ✅ 完成 | CF 展開の核心 |
| **Instrument enum** | `crates/infra_master/src/trade/instrument.rs` | ✅ 完成 | Deposit, FRA, Futures, ParSwap, OIS, BasisSwap, CrossCurrencySwap |
| **Cashflow/Leg/Trade** | `crates/infra_master/src/trade/` | ✅ 完成 | Trade 構造体 |
| **instrument_def** | `crates/infra_master/src/trade/instrument_def/` | ✅ 完成 | Rates, FX, Equity, Credit, Commodity の詳細定義 |
| **Web handlers** | `demo/gui/src/web/handlers.rs` | 🔶 拡張必要 | 既存 price/bootstrap エンドポイント |
| **pricer_types** | `demo/gui/src/web/pricer_types.rs` | 🔶 拡張必要 | 現在は EquityVanillaOption, FxOption, IRS のみ |

#### フロントエンド（HTML/JS）

| ファイル | 状態 | 関連性 |
|---------|------|--------|
| `demo/gui/static/index.html` | 🔶 拡張必要 | Pricer 画面 HTML |
| `demo/gui/static/app.js` | 🔶 拡張必要 | `handlePricerCalculate()`, `buildPricerRequest()` |
| `demo/gui/static/style.css` | 🔶 拡張必要 | Pricer フォーム・結果表示スタイル |

### 1.2 既存パターンの抽出

#### API パターン

```rust
// 既存パターン: POST /api/price
pub async fn price_instrument(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PricingRequest>,
) -> impl IntoResponse { ... }
```

- JSON シリアライズ: `serde` with `#[serde(rename_all = "camelCase")]`
- レスポンス形式: `Json<T>` または `(StatusCode, Json<ErrorResponse>)`
- 状態管理: `Arc<AppState>` でキャッシュ管理

#### UI パターン

```javascript
// 既存パターン: 動的フォーム切り替え
function handleInstrumentTypeChange(type) {
    document.getElementById('equity-option-form').style.display = type === 'equity_vanilla_option' ? 'block' : 'none';
    // ...
}
```

- フォーム切り替え: DOM display プロパティ
- 結果表示: `displayPricerResults()` 関数
- 履歴管理: `addToHistory()` 関数

### 1.3 統合サーフェス

| 統合ポイント | 現状 | 必要な変更 |
|-------------|------|-----------|
| **REST API ルーティング** | `demo/gui/src/web/mod.rs` | 新規エンドポイント追加 |
| **型定義** | `pricer_types.rs` | 新規 Instrument 型追加 |
| **infra_master 依存** | 現在なし | 依存追加必要 |

---

## 2. 要件実現可能性分析

### 2.1 技術的要件

| 要件 | 必要なコンポーネント | 現状 |
|------|-------------------|------|
| Instrument セレクタ拡張 | UI フォーム | 3 種類のみ → 15+ 種類へ |
| 動的フォーム生成 | JS ロジック | 手動切り替え → 動的生成推奨 |
| CF 展開バックエンド | TradeBuilder | ✅ 既存 |
| Trade/Cashflow 表示 | UI コンポーネント | ❌ 新規必要 |
| REST API | Axum handlers | 🔶 新規エンドポイント追加 |
| Instrument 一覧 API | メタデータ生成 | ❌ 新規必要 |

### 2.2 ギャップ識別

#### Missing（欠落）

1. **Trade 展開 API エンドポイント** (`POST /api/trade/expand`)
   - infra_master::trade への依存追加
   - TradeBuilder/LegBuilder のラッパー
   - JSON シリアライズ可能なレスポンス型

2. **Instrument メタデータ API** (`GET /api/instruments`)
   - 全 Instrument タイプのカタログ
   - パラメータスキーマ定義

3. **UI: Trade/Cashflow 表示コンポーネント**
   - Leg カード表示
   - Cashflow テーブル（ソート/ページネーション）

4. **UI: 拡張 Instrument フォーム**
   - Rates 系: Deposit, FRA, Futures, ParSwap, OIS, BasisSwap
   - FX 系: FxForward, CrossCurrencySwap
   - 既存: EquityVanillaOption, FxOption, IRS（改修不要）

#### Unknown（要調査）

1. **スケジュール生成ロジック**
   - LegBuilder は `Vec<Date>` を入力として期待
   - Tenor からの Date リスト生成ロジックが必要
   - **Research Needed**: `pricer_models::schedules` モジュールの活用可能性

2. **infra_master 型の serde 対応**
   - `feature = "serde"` が必要
   - **Research Needed**: 現在の Cargo.toml 設定確認

3. **パフォーマンス要件**
   - 多数の Cashflow 表示時の UI パフォーマンス
   - **Research Needed**: 仮想スクロール vs ページネーション

#### Constraint（制約）

1. **A-I-P-S 依存ルール**
   - Demo (S) は Infra (I) に依存可能 ✅
   - 既存パターンに従う

2. **serde feature flag**
   - infra_master の型は `#[cfg_attr(feature = "serde", derive(...))]` で定義
   - demo/gui から使用時に feature 有効化が必要

---

## 3. 実装アプローチオプション

### Option A: 既存コンポーネントの拡張

**適用場面**: 既存パターンへの自然な追加

| 拡張対象 | 変更内容 |
|---------|---------|
| `pricer_types.rs` | InstrumentType enum 拡張、新規パラメータ型追加 |
| `handlers.rs` | `expand_trade()`, `list_instruments()` ハンドラ追加 |
| `index.html` | 新規フォームセクション追加 |
| `app.js` | `handleTradeExpand()`, `displayTradeResult()` 追加 |

**トレードオフ**:
- ✅ ファイル数最小化、既存パターン踏襲
- ✅ 既存のスタイル・レイアウトとの一貫性
- ❌ `pricer_types.rs` が肥大化（現在 ~600 行 → 推定 1000+ 行）
- ❌ `app.js` の複雑化

### Option B: 新規コンポーネント作成

**適用場面**: 明確な責務分離が必要な場合

| 新規ファイル | 責務 |
|-------------|------|
| `demo/gui/src/web/trade_handlers.rs` | Trade 展開専用ハンドラ |
| `demo/gui/src/web/trade_types.rs` | Trade 展開 API 型定義 |
| `demo/gui/static/trade_expansion.js` | Trade 展開 UI ロジック |

**トレードオフ**:
- ✅ 単一責任原則の維持
- ✅ テスト容易性向上
- ✅ 既存コードへの影響最小化
- ❌ ファイル数増加
- ❌ インターフェース設計必要

### Option C: ハイブリッドアプローチ（推奨）

**戦略**:
- **バックエンド**: 新規ファイル作成（trade_handlers.rs, trade_types.rs）
- **フロントエンド**: 既存ファイル拡張（index.html, app.js）

**根拠**:
- バックエンドは責務が明確に分離（Trade 展開 vs プライシング）
- フロントエンドは既存 UI 構造との統合が必要

**フェーズ分け**:
1. **Phase 1**: API 実装（trade_handlers.rs, trade_types.rs）
2. **Phase 2**: UI 実装（index.html, app.js 拡張）
3. **Phase 3**: 統合テスト・最適化

---

## 4. 実装複雑度と リスク

### Effort 評価: **M** (3-7 days)

**根拠**:
- 既存パターンあり（Axum handlers, JS フォーム）
- 核心ロジック（TradeBuilder）は完成済み
- 新規 UI コンポーネント（Cashflow テーブル）は中程度の複雑度

### Risk 評価: **Medium**

**リスク要因**:
1. **スケジュール生成ロジック**: 要調査
2. **serde 対応**: feature flag 確認必要
3. **UI パフォーマンス**: 多数 Cashflow 表示時

**リスク軽減策**:
- Design フェーズでスケジュール生成アプローチを確定
- infra_master の serde feature を早期確認
- Cashflow テーブルはページネーション採用（仮想スクロールより実装容易）

---

## 5. 要件-アセットマップ

| 要件 | 既存アセット | ギャップ |
|------|-------------|---------|
| R1: Instrument セレクタ拡張 | `index.html` dropdown | Missing: 追加タイプ |
| R2: Instrument 別入力フォーム | `app.js` form logic | Missing: 新規フォーム定義 |
| R3: Trade 展開機能 | `TradeBuilder`, `LegBuilder` | Missing: API エンドポイント |
| R4: Trade/Cashflow 表示 | なし | Missing: UI コンポーネント |
| R5: REST API | `handlers.rs` | Missing: 新規ハンドラ |
| R6: Instrument 一覧 API | なし | Missing: メタデータ生成 |

---

## 6. Design フェーズへの推奨事項

### 優先決定事項

1. **スケジュール生成アプローチ**
   - Option A: `pricer_models::schedules::ScheduleBuilder` 活用
   - Option B: infra_master に簡易スケジュール生成追加
   - **推奨**: Option A（既存コード再利用）

2. **Cashflow 表示方式**
   - Option A: ページネーション（シンプル）
   - Option B: 仮想スクロール（パフォーマンス優先）
   - **推奨**: Option A（実装容易性優先）

3. **API 構造**
   - `/api/trade/expand` - Trade 展開
   - `/api/instruments` - Instrument 一覧
   - **推奨**: RESTful 設計維持

### 調査継続項目

1. infra_master の `serde` feature 有効化状況確認
2. `pricer_models::schedules` の Tenor → Date リスト変換機能確認
3. 既存 DayCountConvention, Frequency 型の JSON シリアライズ対応確認
