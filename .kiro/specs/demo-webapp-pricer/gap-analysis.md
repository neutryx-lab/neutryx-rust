# Gap Analysis: demo-webapp-pricer

## 概要

Demo WebAppのAnalysisセクション内にPricer検証機能を追加するための実装ギャップを分析する。`pricer_pricing::generic_pricer`モジュール（88テスト通過済み）をWebApp UIから呼び出し、プライシング結果とGreeks計算結果を表示・検証する機能を実装する。

---

## 1. 現状調査

### 1.1 既存アセット

#### バックエンド（Rust）

| ファイル | 説明 | 再利用可能性 |
|---------|------|-------------|
| `demo/gui/src/web/mod.rs` | メインルーター、AppState | 高（ルート追加のみ） |
| `demo/gui/src/web/handlers.rs` | 既存APIハンドラ（2800+行） | 高（パターン参照） |
| `demo/gui/src/web/pricer_types.rs` | プライサー型定義（57k tokens） | 中（拡張必要） |
| `demo/gui/src/web/trade_types.rs` | Trade展開型定義 | 高（統合可能） |
| `demo/gui/src/web/trade_handlers.rs` | Trade展開API | 高（統合可能） |
| `pricer_pricing::generic_pricer` | GenericPricer実装（88テスト） | 高（直接利用） |

#### フロントエンド（HTML/JS/CSS）

| ファイル | 説明 | 再利用可能性 |
|---------|------|-------------|
| `demo/gui/static/index.html` | メインHTML（2000+行） | 高（セクション追加） |
| `demo/gui/static/app.js` | メインJS（15000+行） | 高（モジュール追加） |
| `demo/gui/static/style.css` | グローバルCSS（12000+行） | 高（既存クラス再利用） |
| `demo/gui/static/js/curve-builder.js` | Curve Builderモジュール | 高（パターン参照） |

### 1.2 抽出されたコンベンション

**バックエンドパターン**:
- Axum + State パターン（`State<Arc<AppState>>`）
- JSON リクエスト/レスポンス（`Json<T>`）
- 型定義は `*_types.rs` に分離
- ハンドラは `*_handlers.rs` に分離
- バリデーション関数（`validate_*_request`）
- エラーレスポンス型（`*ErrorResponse`）

**フロントエンドパターン**:
- 2パネルレイアウト（入力 + 結果）
- glassmorphism デザイン（`.glass` クラス）
- `data-view` 属性によるビュー切り替え
- `navigateTo()` 関数によるナビゲーション
- `apiClient` による非同期API呼び出し
- `showToast()` によるユーザー通知

### 1.3 統合サーフェス

**既存APIエンドポイント**:
- `/api/trade/expand` - Trade展開（既存）
- `/api/instruments` - 商品タイプ一覧（既存）
- `/api/greeks/*` - Greeks計算（既存、別実装）
- `/api/curves/*` - Curve Builder（既存）

**依存関係**:
```
demo_gui (Cargo.toml)
├── pricer_pricing (generic_pricer, etc.)
├── pricer_models (market, curves)
├── pricer_core (math, traits)
└── infra_master (trade, time, market)
```

---

## 2. 要件実現可能性分析

### 2.1 技術ニーズマッピング

| 要件 | 技術ニーズ | 既存資産 | ギャップ |
|-----|----------|---------|---------|
| R1: UIナビゲーション | HTML/JS追加 | index.html, app.js | **新規セクション追加** |
| R2: Trade選択 | APIエンドポイント | trade_handlers.rs | 軽微な拡張 |
| R3: CF展開・編集 | フロントエンドUI | trade_types.rs | **編集UI新規** |
| R4: マーケットデータ | カーブ読込 | curve_builder_handlers | 既存流用可 |
| R5: モデル設定 | ModelConfig型 | pricer_types.rs | **型追加** |
| R6: プライシング実行 | GenericPricer統合 | pricer_pricing crate | **新規ハンドラ** |
| R7: PricingResult表示 | 階層表示UI | なし | **新規UI** |
| R8: Greeks計算 | BumpAndRevalue統合 | generic_pricer | **新規ハンドラ** |
| R9: 結果比較 | フロントエンド状態 | なし | **新規機能** |
| R10: APIエンドポイント | Rustハンドラ | handlers.rs | **新規追加** |

### 2.2 ギャップ識別

**Missing（実装必要）**:
- `POST /api/pricer/price` エンドポイント
- `POST /api/pricer/greeks` エンドポイント
- Pricer UI セクション（`#pricer-view`）
- PricingResult 階層表示コンポーネント
- 結果履歴・比較機能

**Unknown（要調査）**:
- `generic_pricer` の `SimpleLeg`/`SimpleCashflow` と `trade_types` の変換ロジック
- マーケットデータ（ディスカウントカーブ）の取得パス

**Constraint（制約）**:
- `pricer_pricing` は `l1l2-integration` feature なしでビルド
- FXレートはプレースホルダー実装（CHFは未対応）

### 2.3 複雑性シグナル

| カテゴリ | 複雑度 |
|---------|--------|
| 基本CRUD操作 | 低 |
| アルゴリズムロジック | 低（GenericPricer既存） |
| ワークフロー | 中（入力→展開→プライシング→表示） |
| 外部統合 | 低（内部クレートのみ） |

---

## 3. 実装アプローチオプション

### Option A: 既存コンポーネント拡張

**対象ファイル**:
- `pricer_types.rs` に `GenericPricerRequest`/`Response` 追加
- `handlers.rs` に `price_generic`/`greeks_generic` ハンドラ追加
- `index.html` に `#pricer-view` セクション追加
- `app.js` に Pricer モジュール追加

**互換性評価**:
- 既存 `pricer_types.rs` は57k tokens（大きい）→ 追加可能だが分離検討
- 既存 `handlers.rs` は69k tokens（非常に大きい）→ 新規ファイル推奨

**トレードオフ**:
- ✅ 既存パターン再利用で学習コスト低
- ✅ インポート構造変更なし
- ❌ 既存ファイルのさらなる肥大化
- ❌ コードレビュー・保守の複雑化

### Option B: 新規コンポーネント作成

**新規ファイル**:
- `demo/gui/src/web/generic_pricer_types.rs` - リクエスト/レスポンス型
- `demo/gui/src/web/generic_pricer_handlers.rs` - APIハンドラ
- `demo/gui/static/js/pricer.js` - Pricerモジュール

**統合ポイント**:
- `mod.rs`: `pub mod generic_pricer_types; pub mod generic_pricer_handlers;`
- `build_router()`: `/api/pricer/*` ルート追加
- `index.html`: `#pricer-view` セクション追加
- `app.js`: `pricer.js` インポート

**トレードオフ**:
- ✅ 既存ファイル変更最小化
- ✅ テスト・保守の独立性
- ✅ 単一責任原則の遵守
- ❌ ファイル数増加
- ❌ 一部コード重複の可能性

### Option C: ハイブリッドアプローチ（推奨）

**組み合わせ戦略**:

1. **新規作成**:
   - `generic_pricer_handlers.rs` - 新規ハンドラ
   - `demo/gui/static/js/pricer.js` - フロントエンドモジュール

2. **既存拡張**:
   - `pricer_types.rs` - GenericPricer関連型追加（型定義の一元管理）
   - `index.html` - `#pricer-view` セクション追加
   - `app.js` - Pricerモジュール統合

**フェーズ実装**:
1. Phase 1: バックエンドAPI（ハンドラ + 型）
2. Phase 2: フロントエンドUI（HTML + JS）
3. Phase 3: 結果比較機能

**トレードオフ**:
- ✅ バランスの取れた設計
- ✅ 既存パターンとの整合性維持
- ✅ 段階的な実装・検証可能
- ❌ 計画の複雑さ増加

---

## 4. 工数・リスク評価

### 工数見積もり

| 作業項目 | 工数 | 理由 |
|---------|------|------|
| バックエンドAPI | **M** (3-5日) | 既存パターン踏襲、GenericPricer統合 |
| フロントエンドUI | **M** (3-5日) | 2パネルレイアウト、階層表示 |
| 統合・テスト | **S** (1-2日) | E2Eテスト、エラーハンドリング |
| **合計** | **M-L** (7-12日) | |

### リスク評価

| リスク項目 | レベル | 緩和策 |
|-----------|--------|--------|
| GenericPricer統合 | **低** | 88テスト通過済み、API安定 |
| 型変換ロジック | **中** | SimpleLeg↔TradeExpandResponseの変換設計 |
| UI複雑性 | **中** | 既存Curve Builderパターン参照 |
| パフォーマンス | **低** | バッチプライシングは今回スコープ外 |

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ: Option C（ハイブリッド）

**理由**:
1. 既存の大規模ファイル（handlers.rs, pricer_types.rs）への追加は保守性を損なう
2. 新規ハンドラファイルで独立性確保
3. 型定義は既存 `pricer_types.rs` に集約して一貫性維持

### 調査事項（設計フェーズで解決）

1. **Trade型変換**: `TradeExpandResponse` → `Vec<SimpleLeg>` の変換ロジック設計
2. **マーケットデータ**: ディスカウントカーブの取得方法（Curve Builderのキャッシュ活用？）
3. **FXレート**: プレースホルダー実装の拡張（demo/data/input から読込）

### キー決定事項

| 決定項目 | オプション | 推奨 |
|---------|----------|------|
| ハンドラ配置 | 既存/新規ファイル | **新規** `generic_pricer_handlers.rs` |
| 型定義配置 | 既存/新規ファイル | **既存** `pricer_types.rs` 拡張 |
| フロントエンド | app.js統合/別ファイル | **別ファイル** `pricer.js` |
| API パス | `/api/price`拡張/新規 | **新規** `/api/pricer/*` |

---

## 6. 要件-資産マトリクス

| 要件ID | 既存資産 | ギャップ状態 |
|--------|---------|-------------|
| R1 | index.html, app.js | 新規セクション追加 |
| R2 | trade_handlers.rs, trade_types.rs | 軽微拡張 |
| R3 | - | **Missing**: 編集UI |
| R4 | curve_builder_handlers.rs | 既存流用 |
| R5 | pricer_types.rs | 型追加 |
| R6 | pricer_pricing::generic_pricer | **Missing**: ハンドラ |
| R7 | - | **Missing**: 階層表示UI |
| R8 | generic_pricer::greeks_calculator | **Missing**: ハンドラ |
| R9 | - | **Missing**: 比較機能 |
| R10 | handlers.rs (パターン) | **Missing**: エンドポイント |
