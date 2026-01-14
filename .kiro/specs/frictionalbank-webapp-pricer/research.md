# ギャップ分析レポート: frictionalbank-webapp-pricer

## 1. 現状調査

### 1.1 既存資産のスキャン

#### ディレクトリレイアウト

```
demo/
├── frictional_bank/          # メインのTUIデモ
│   ├── src/
│   │   ├── workflow/         # EOD, Intraday, Stress Test, IRS AAD
│   │   └── lib.rs
│   └── tests/
└── gui/
    ├── src/
    │   ├── web/              # Webサーバー (axum)
    │   │   ├── handlers.rs   # REST API ハンドラー
    │   │   ├── websocket.rs  # WebSocketハンドラー
    │   │   └── mod.rs        # ルーター設定
    │   ├── visualisation.rs  # ベンチマーク可視化
    │   └── screens.rs        # TUI画面
    └── static/               # フロントエンドアセット
        ├── index.html        # ダッシュボードUI
        ├── style.css         # スタイルシート
        ├── app.js            # JavaScriptアプリケーション
        └── vendor/           # Chart.js, D3.js, etc.
```

#### 再利用可能なコンポーネント

| コンポーネント | ファイル | 再利用性 |
|--------------|---------|---------|
| WebSocket基盤 | `demo/gui/src/web/websocket.rs` | ✅ 高 - 既存のリアルタイム更新機構を活用可能 |
| REST API基盤 | `demo/gui/src/web/handlers.rs` | ✅ 高 - 既存のAPIパターンを拡張 |
| GraphCache | `demo/gui/src/web/handlers.rs` | ✅ 中 - キャッシュパターンを参考 |
| AppState | `demo/gui/src/web/mod.rs` | ✅ 高 - 状態管理パターンを拡張 |
| RealTimeUpdate | `demo/gui/src/web/websocket.rs` | ✅ 高 - 新しいメッセージタイプを追加可能 |
| BenchmarkVisualiser | `demo/gui/src/visualisation.rs` | ⚠️ 中 - Chart.js連携パターンを参考 |

#### Pricer層統合ポイント

| モジュール | パス | 用途 |
|-----------|------|------|
| PricingContext | `crates/pricer_pricing/src/context.rs` | 価格計算の3段階ロケットパターン |
| InstrumentEnum | `crates/pricer_models/src/demo.rs` | デモ用の商品定義（VanillaSwap, CmsSwap） |
| ModelEnum | `crates/pricer_models/src/demo.rs` | デモ用のモデル定義（BlackScholes, HullWhite） |
| GreeksResult | `crates/pricer_pricing/src/greeks/result.rs` | Greeks計算結果 |
| GreeksConfig | `crates/pricer_pricing/src/greeks/config.rs` | Greeks計算設定 |

### 1.2 アーキテクチャパターン

#### 既存のコンベンション

- **REST API**: `/api/` プレフィックス、JSON レスポンス
- **WebSocket**: `/api/ws` エンドポイント、`update_type` フィールドでメッセージ分類
- **状態管理**: `Arc<AppState>` でスレッドセーフな共有状態
- **キャッシュ**: `RwLock<GraphCache>` パターン（5秒TTL）
- **エラー処理**: `(StatusCode, Json<ErrorResponse>)` タプル
- **シリアライゼーション**: `serde` + `serde_json` (camelCase for JS interop)

#### フロントエンドパターン

- **ライブラリ**: Chart.js, D3.js (グラフ/チャート)
- **スタイル**: CSS変数によるテーマ、Glass morphism UI
- **アクセシビリティ**: ARIA属性、スキップリンク、コントラストモード

### 1.3 統合サーフェス

| 領域 | 既存実装 | プライサー機能への適用 |
|------|---------|---------------------|
| データモデル | `TradeData`, `PortfolioResponse` | 新規 `PricingRequest`, `PricingResponse` 追加 |
| API クライアント | `demo/gui/src/api_client.rs` | 価格計算エンドポイント追加 |
| 認証 | なし（デモ用） | 不要 |
| フロントエンド | `index.html`, `app.js` | 新規プライサーセクション追加 |

---

## 2. 要件実現可能性分析

### 2.1 技術的要件マッピング

| 要件ID | 要件 | 技術的ニーズ | 既存資産 | ギャップ |
|--------|------|-------------|---------|---------|
| Req 1 | 商品選択UI | HTML form, 動的フォーム生成 | index.html | **New**: プライサーセクションHTML |
| Req 2 | パラメータ入力 | Form validation, 商品別フィールド | style.css | **New**: 商品別フォームコンポーネント |
| Req 3 | リアルタイム計算 | WebSocket, 非同期計算 | websocket.rs | **Extend**: `pricing` メッセージタイプ |
| Req 4 | Greeks表示 | GreeksResult, Chart.js | visualisation.rs | **New**: Greeks表示コンポーネント |
| Req 5 | バックエンド統合 | PricingContext, InstrumentEnum | pricer_pricing | **Extend**: 新規ハンドラー |
| Req 6 | 市場データ | CurveEnum, VolSurfaceEnum | pricer_models/demo.rs | **Extend**: デモデータ提供 |
| Req 7 | 計算履歴 | セッション状態, LocalStorage | AppState | **New**: 履歴管理 |
| Req 8 | レスポンシブUI | CSS Grid/Flexbox | style.css | **Extend**: メディアクエリ追加 |

### 2.2 ギャップ識別

#### Missing Capabilities

1. **プライサー専用REST API** (`/api/price`)
   - リクエスト: 商品タイプ、パラメータ
   - レスポンス: PV, Greeks, エラー情報

2. **商品構築ロジック**
   - フロントエンドJSONからRust Instrument構築
   - バリデーション（必須パラメータ、範囲チェック）

3. **Greeks計算統合**
   - pricer_pricing::greeks との連携
   - 結果のシリアライゼーション

4. **フロントエンドプライサーUI**
   - 商品選択ドロップダウン
   - 動的パラメータフォーム
   - 結果表示パネル

5. **計算履歴管理**
   - セッション内状態管理
   - フロントエンドLocalStorage連携

#### Research Needed

1. **Enzyme AD統合**: Greeks計算でのEnzyme活用可否（現状: num-dual fallback可）
2. **商品カバレッジ**: pricer_models内の実装済み商品の確認
3. **パフォーマンス**: WebSocket経由のリアルタイム計算レイテンシ

#### Constraints

1. **デモ用簡略化**: 実運用向けの完全なInstrument定義ではなく、`demo.rs`の簡略版を使用
2. **市場データ**: ハードコードされたデモデータを使用（外部データソース連携なし）
3. **認証/認可**: なし（デモ環境）

### 2.3 複雑性シグナル

| 領域 | 複雑性 | 理由 |
|------|--------|------|
| バックエンドAPI | 低〜中 | 既存パターンの拡張、新規ロジックは商品構築のみ |
| フロントエンドUI | 中 | 動的フォーム生成、商品別バリデーション |
| Pricer統合 | 低 | context.rsの既存パターンを直接活用 |
| Greeks計算 | 低〜中 | greeksモジュール活用、シリアライゼーション追加 |
| 履歴管理 | 低 | シンプルなセッション状態管理 |

---

## 3. 実装アプローチオプション

### Option A: 既存コンポーネント拡張

**適用シナリオ**: 既存のhandlers.rs, websocket.rs, index.htmlを直接拡張

#### 変更対象ファイル

| ファイル | 変更内容 | 影響度 |
|---------|---------|--------|
| `handlers.rs` | `price_instrument` ハンドラー追加 | 中 |
| `websocket.rs` | `pricing_update` メッセージ追加 | 低 |
| `mod.rs` | `/api/price` ルート追加 | 低 |
| `index.html` | プライサーセクション追加 | 高 |
| `app.js` | プライサーロジック追加 | 高 |
| `style.css` | プライサースタイル追加 | 中 |

#### トレードオフ

- ✅ 最小限の新規ファイル
- ✅ 既存インフラストラクチャの活用
- ❌ index.html/app.jsの肥大化リスク
- ❌ 既存機能との結合度増加

### Option B: 新規モジュール作成

**適用シナリオ**: プライサー機能を独立したモジュールとして分離

#### 新規作成ファイル

```
demo/gui/
├── src/
│   ├── web/
│   │   ├── pricer.rs         # NEW: プライサーAPIハンドラー
│   │   └── pricer_types.rs   # NEW: リクエスト/レスポンス型
│   └── pricer_service.rs     # NEW: 価格計算サービス層
└── static/
    ├── pricer.html           # NEW: プライサーページ
    ├── pricer.js             # NEW: プライサーJS
    └── pricer.css            # NEW: プライサースタイル
```

#### トレードオフ

- ✅ 関心の明確な分離
- ✅ テスト容易性
- ✅ 既存機能への影響最小
- ❌ ファイル数増加
- ❌ ナビゲーション/ルーティング追加必要

### Option C: ハイブリッドアプローチ（推奨）

**適用シナリオ**: バックエンドは既存拡張、フロントエンドは既存ページ内にセクション追加

#### 戦略

1. **バックエンド**: `handlers.rs`に新規ハンドラー追加（Option A）
2. **型定義**: 新規 `pricer_types.rs` ファイル（Option B）
3. **フロントエンド**: `index.html`内に新規タブ/セクション追加（Option A）
4. **JS**: `app.js`内にプライサー関数を追加（モジュール化）

#### 変更対象

| ファイル | 変更 | 理由 |
|---------|------|------|
| `handlers.rs` | 拡張 | 既存パターン維持 |
| `pricer_types.rs` | 新規 | 型定義の分離 |
| `mod.rs` | 拡張 | ルート追加のみ |
| `index.html` | 拡張 | 新規タブ追加 |
| `app.js` | 拡張 | プライサー関数追加 |
| `style.css` | 拡張 | プライサースタイル追加 |

#### トレードオフ

- ✅ バランスの取れたアプローチ
- ✅ 既存パターンとの整合性
- ✅ 型定義の明確な分離
- ⚠️ 中程度の計画複雑性

---

## 4. 実装複雑性とリスク

### 工数見積もり: **M (3-7日)**

| フェーズ | 工数 | 根拠 |
|---------|------|------|
| バックエンドAPI | S (1-2日) | 既存パターンの拡張 |
| フロントエンドUI | M (2-3日) | 動的フォーム、結果表示 |
| Pricer統合 | S (1日) | context.rs活用 |
| テスト | S (1日) | 単体テスト、統合テスト |

### リスク評価: **Low-Medium**

| リスク | レベル | 緩和策 |
|--------|--------|--------|
| Pricer統合の複雑性 | 低 | 既存demo.rsパターン活用 |
| フロントエンド肥大化 | 中 | タブ/セクション分離 |
| Greeks計算パフォーマンス | 低 | キャッシュ、非同期処理 |
| 商品カバレッジ不足 | 低 | デモ用簡略版で開始 |

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option C（ハイブリッド）** を推奨

### 主要決定事項

1. **商品タイプ**: 初期実装はEquity Vanilla Option、FX Option、IRS（デモ版）に限定
2. **Greeks計算**: num-dual fallback モード使用（Enzyme不要）
3. **フロントエンド**: 既存ダッシュボードに「Pricer」タブとして追加

### 設計フェーズでの調査項目

1. **商品パラメータ仕様**: 各商品タイプの必須/オプションパラメータ詳細
2. **Greeks表示形式**: テーブル vs カード vs チャート
3. **エラーハンドリング**: バリデーションエラーのUI表示方法

---

## 6. 要件-資産マップサマリー

| 要件 | 状態 | 対応方針 |
|------|------|---------|
| Req 1: 商品選択UI | Missing | 新規HTMLセクション |
| Req 2: パラメータ入力 | Missing | 動的フォーム実装 |
| Req 3: リアルタイム計算 | Extend | WebSocket活用 |
| Req 4: Greeks表示 | Missing | 新規UIコンポーネント |
| Req 5: バックエンド統合 | Extend | handlers.rs拡張 |
| Req 6: 市場データ | Extend | demo.rsデータ活用 |
| Req 7: 計算履歴 | Missing | セッション状態管理 |
| Req 8: レスポンシブUI | Extend | CSS拡張 |
