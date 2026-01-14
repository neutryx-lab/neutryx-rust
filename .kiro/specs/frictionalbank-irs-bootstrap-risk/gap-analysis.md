# ギャップ分析: IRS Bootstrap & Risk 機能

## 概要

本ドキュメントは、FrictionalBank WebAppの「IRS Bootstrap & Risk」機能の要件と既存コードベースのギャップを分析し、設計フェーズへの推奨事項を提供する。

---

## 1. 現状調査結果

### 1.1 既存アセット一覧

#### WebApp フロントエンド (`demo/gui/static/`)

| ファイル | 説明 | 再利用可能性 |
|---------|------|-------------|
| `index.html` | メインHTML（Pricerビュー含む、2037-2247行） | ✅ 拡張可能 |
| `app.js` | JavaScriptアプリケーションロジック | ✅ 拡張可能 |
| `style.css` | スタイルシート（.pricer-* クラス含む） | ✅ 拡張可能 |
| `vendor/chart.umd.min.js` | Chart.jsライブラリ | ✅ 利用可能 |

#### WebApp バックエンド (`demo/gui/src/web/`)

| ファイル | 説明 | 再利用可能性 |
|---------|------|-------------|
| `handlers.rs` | HTTP APIハンドラ（`POST /api/price`等） | ✅ 拡張可能 |
| `pricer_types.rs` | API型定義（`IrsParams`, `PricingResponse`等） | ✅ 拡張可能 |
| `websocket.rs` | WebSocketハンドラ（broadcast機構） | ✅ 拡張可能 |
| `mod.rs` | Axumルーター構築（`build_router`） | ✅ 拡張可能 |

#### Bootstrap モジュール (`crates/pricer_optimiser/src/bootstrapping/`)

| モジュール | 説明 | 要件カバレッジ |
|-----------|------|---------------|
| `engine.rs` | SequentialBootstrapper | 要件2.1 ✅ |
| `curve.rs` | BootstrappedCurve | 要件2.3 ✅ |
| `instrument.rs` | BootstrapInstrument (OIS, IRS, FRA) | 要件2.1 ✅ |
| `sensitivity.rs` | SensitivityBootstrapper (AAD対応) | 要件5.1, 5.2 ✅ |
| `config.rs` | GenericBootstrapConfig | 要件2.2 ✅ |
| `error.rs` | BootstrapError | 要件2.5 ✅ |

#### IRS Greeks モジュール (`crates/pricer_pricing/src/irs_greeks/`)

| モジュール | 説明 | 要件カバレッジ |
|-----------|------|---------------|
| `calculator.rs` | IrsGreeksCalculator | 要件3.2, 3.3 ✅ |
| `benchmark.rs` | BenchmarkRunner, TimingStats | 要件4, 5, 6 ✅ |
| `result.rs` | IrsDeltaResult, DeltaBenchmarkResult | 要件4.2, 5.3 ✅ |
| `config.rs` | IrsGreeksConfig | ✅ |

#### 既存TUI/ワークフロー

| ファイル | 説明 |
|---------|------|
| `demo/gui/src/screens/irs_aad.rs` | TUI画面（IRS AADデモ） |
| `demo/frictional_bank/src/workflow/irs_aad.rs` | IrsAadWorkflow |

### 1.2 既存パターン・規約

#### API設計パターン
- RESTful JSON API（camelCase シリアライズ）
- エンドポイント命名: `/api/{resource}`
- エラーレスポンス: `PricingErrorResponse` 型
- WebSocket ブロードキャスト: `state.tx.send()`

#### フロントエンドパターン
- ビュー切り替え: `data-view` 属性
- フォームバリデーション: クライアントサイド + サーバーサイド
- 結果表示: `.pricer-results-panel` 構造
- Chart.js統合: 既存インフラ

---

## 2. 要件実現可能性分析

### 2.1 要件-アセット マッピング

| 要件 | 必要な技術要素 | 既存アセット | ステータス |
|------|---------------|-------------|-----------|
| **1.1-1.6** Par Rate入力UI | HTML form, JS validation | 部分的（基本IRSフォームあり） | **Gap** |
| **2.1-2.6** Bootstrap | SequentialBootstrapper | ✅ 完全 | **Available** |
| **3.1-3.6** IRS時価評価 | IrsGreeksCalculator | ✅ 完全 | **Available** |
| **4.1-4.6** Bump法リスク | BenchmarkRunner | ✅ 完全 | **Available** |
| **5.1-5.6** AAD法リスク | BenchmarkRunner, enzyme-ad | ✅ 完全 | **Available** |
| **6.1-6.6** 経過時間比較 | TimingStats, Chart.js | ✅ インフラあり | **Partial** |
| **7.1-7.6** UI/UX | HTML/CSS/JS | 既存パターンあり | **Gap** |
| **8.1-8.4** WebSocket通知 | websocket.rs | ✅ インフラあり | **Partial** |
| **9.1-9.8** バックエンドAPI | handlers.rs, axum | ✅ パターンあり | **Gap** |

### 2.2 ギャップ詳細

#### フロントエンド ギャップ

| ギャップ | 説明 | 複雑度 |
|---------|------|--------|
| **Par Rate入力フォーム** | 9テナーポイント（1Y-30Y）の入力UI | M |
| **IRS Bootstrap サブビュー** | Pricerタブ内の新規サブビュー | M |
| **3ステップワークフローUI** | Step 1: Data, Step 2: Bootstrap, Step 3: Risk | M |
| **カーブ可視化** | Bootstrap結果のテーブル/グラフ表示 | S |
| **Delta テーブル** | Bump/AAD 結果の並列表示 | S |
| **速度比較バーチャート** | Chart.js（既存インフラ活用） | S |
| **ベンチマーク実行UI** | 複数回実行ボタン、統計表示 | S |

#### バックエンド ギャップ

| ギャップ | 説明 | 複雑度 |
|---------|------|--------|
| **`POST /api/bootstrap`** | Par Rateからカーブ構築 | M |
| **`POST /api/price-irs`** | カーブを使用したIRS時価評価 | S |
| **`POST /api/risk/bump`** | Bump法リスク計算 | M |
| **`POST /api/risk/aad`** | AAD法リスク計算 | M |
| **`POST /api/risk/compare`** | 両手法の比較実行 | M |
| **新規API型定義** | Request/Response 構造体 | S |
| **WebSocketイベント追加** | bootstrap_complete, risk_complete | S |

### 2.3 制約事項

1. **Enzyme feature flag**: `pricer_pricing` の AAD機能は `enzyme-ad` feature が必要
2. **l1l2-integration feature**: フル統合には `l1l2-integration` feature が必要
3. **既存IRSフォーム**: 現在のフォームは簡易版（Notional, Fixed Rate, Tenor のみ）

---

## 3. 実装アプローチ オプション

### Option A: 既存コンポーネント拡張

**概要**: 既存の Pricer ビューと handlers.rs を拡張

**対象ファイル**:
- `demo/gui/static/index.html`: Pricer セクション拡張
- `demo/gui/static/app.js`: IRS Bootstrap ロジック追加
- `demo/gui/src/web/handlers.rs`: 新規 API ハンドラ追加
- `demo/gui/src/web/pricer_types.rs`: 新規型定義追加
- `demo/gui/src/web/mod.rs`: ルート追加

**トレードオフ**:
- ✅ 既存パターンに準拠、最小限の新規ファイル
- ✅ 既存のテスト・CI 基盤を活用
- ❌ index.html が大きくなる（現在 2275行）
- ❌ app.js が更に肥大化（現在 396KB）

### Option B: 新規モジュール作成

**概要**: IRS Bootstrap & Risk 用の専用モジュールを作成

**新規ファイル**:
- `demo/gui/static/irs-bootstrap.html`: 専用HTML
- `demo/gui/static/irs-bootstrap.js`: 専用JavaScript
- `demo/gui/src/web/irs_bootstrap_handlers.rs`: 専用ハンドラ
- `demo/gui/src/web/irs_bootstrap_types.rs`: 専用型定義

**トレードオフ**:
- ✅ 関心の分離が明確
- ✅ 単独でのテスト・開発が容易
- ❌ 新規ファイル数が多い
- ❌ 共通コンポーネント（WebSocket等）の重複可能性

### Option C: ハイブリッドアプローチ（推奨）

**概要**: バックエンドは既存構造を拡張、フロントエンドは適度に分離

**変更ファイル**:
- `demo/gui/static/index.html`: IRS Bootstrap サブビュー追加（新セクション）
- `demo/gui/static/style.css`: IRS Bootstrap 用スタイル追加
- `demo/gui/static/app.js`: IRS Bootstrap モジュール追加（IIFE パターン）
- `demo/gui/src/web/handlers.rs`: 新規 API ハンドラ追加
- `demo/gui/src/web/pricer_types.rs`: 新規型定義追加
- `demo/gui/src/web/mod.rs`: ルート追加
- `demo/gui/src/web/websocket.rs`: 新規イベント追加

**トレードオフ**:
- ✅ バランスの取れたアプローチ
- ✅ 既存パターンを活用しつつ、過度な肥大化を防止
- ✅ フロントエンドコードはIIFEで名前空間分離
- ❌ 中程度の変更箇所数

---

## 4. 工数・リスク評価

### 4.1 工数見積もり

| 領域 | 工数 | 根拠 |
|------|------|------|
| フロントエンド | **M** | 新規UIコンポーネント（フォーム、テーブル、チャート）、既存パターン活用 |
| バックエンド API | **M** | 5つの新規エンドポイント、既存型パターン活用 |
| 統合・テスト | **S** | 既存Bootstrap/IrsGreeksモジュールの統合 |
| **合計** | **M** | 3-7日相当 |

### 4.2 リスク評価

| リスク | レベル | 緩和策 |
|--------|--------|--------|
| Bootstrap モジュールとの統合 | **低** | 既に pricer_optimiser に完全実装済み |
| AAD機能の統合 | **中** | enzyme-ad feature フラグによる条件分岐が必要 |
| パフォーマンス | **低** | 既存ベンチマーク実装あり |
| UI複雑度 | **中** | 3ステップワークフローの設計が必要 |

---

## 5. 設計フェーズへの推奨事項

### 5.1 推奨アプローチ

**Option C（ハイブリッドアプローチ）** を推奨

理由:
1. 既存パターン（`handlers.rs`, `pricer_types.rs`）を最大限活用
2. フロントエンドは名前空間を分離しつつ、既存のビュー構造に統合
3. Bootstrap/IrsGreeks モジュールとの統合が自然

### 5.2 設計フェーズで調査が必要な項目

| 項目 | 調査内容 |
|------|----------|
| **l1l2-integration feature** | WebApp での feature flag の有効化方法 |
| **enzyme-ad fallback** | AAD不使用時のグレースフルフォールバック |
| **カーブ状態管理** | Bootstrap結果のセッション/リクエスト間共有方法 |
| **エラーハンドリング** | Bootstrap収束失敗時のUI表示 |

### 5.3 主要な設計決定事項

1. **API設計**: RESTful vs カーブIDベースのステートフルAPI
2. **カーブ永続化**: インメモリ vs セッションストレージ
3. **WebSocket統合**: 計算進捗のリアルタイム通知範囲
4. **Feature flag戦略**: enzyme-ad 無効時の動作

---

## 6. まとめ

### ギャップサマリー

- **フロントエンド**: 新規サブビュー、Par Rate入力フォーム、結果表示UI
- **バックエンド**: 5つの新規APIエンドポイント、型定義、WebSocketイベント
- **統合**: 既存の `pricer_optimiser::bootstrapping` と `pricer_pricing::irs_greeks` の活用

### 主要な強み

- Bootstrap、IRS Greeks、ベンチマーク機能は **既に完全実装済み**
- WebApp基盤（Axum, WebSocket, Chart.js）は **成熟**
- 既存パターンに従うことで **迅速な実装が可能**

### 次のステップ

1. `/kiro:spec-design frictionalbank-irs-bootstrap-risk` を実行して技術設計書を作成
2. 設計フェーズで上記の調査項目を解決
3. API設計とUI設計の詳細を決定

---

_作成日: 2026-01-14_
_言語: 日本語 (spec.json.language: ja)_
