# Gap Analysis: curve-builder-webapp

## 概要

本ドキュメントは、Curve Build画面精緻化の要件と既存コードベース間のギャップを分析し、実装アプローチの選択肢を提示する。

---

## 1. 現状調査

### 1.1 既存コンポーネント

#### バックエンド（Rust）

| コンポーネント | 場所 | 状態 | 説明 |
|---------------|------|------|------|
| SequentialBootstrapper | `pricer_models::market::calibration::bootstrapping::engine` | ✅ 完成 | 逐次ブートストラップエンジン |
| CurveBootstrapper | `pricer_models::market::calibration::bootstrapping::curve_builder` | ✅ 完成 | カーブ構築コア |
| InterpolationMethod | `pricer_models::market::calibration::bootstrapping::curve_builder` | ✅ 完成 | Linear, LogLinear, CubicSpline |
| BootstrapInstrument | `pricer_models::market::calibration::bootstrapping::instrument` | ✅ 完成 | Deposit, OIS, IRS等 |
| GenericBootstrapConfig | `pricer_models::market::calibration::bootstrapping::config` | ✅ 完成 | 設定パラメータ |
| Interpolators | `pricer_core::math::interpolators` | ✅ 完成 | Linear, LogLinear, CubicSpline, Monotonic, Hermite |
| YieldCurve trait | `pricer_models::market::curves::traits` | ✅ 完成 | discount_factor, zero_rate, forward_rate |

#### WebApp API（demo/gui/src/web）

| エンドポイント | ハンドラ | 状態 | 説明 |
|---------------|---------|------|------|
| `POST /api/bootstrap` | handlers::bootstrap_curve | ✅ 存在 | カーブ構築（par_rates入力） |
| `POST /api/price-irs` | handlers::price_irs | ✅ 存在 | IRS評価（削除対象） |
| `POST /api/risk/bump` | handlers::risk_bump | ✅ 存在 | Bump Delta計算 |
| `POST /api/risk/aad` | handlers::risk_aad | ✅ 存在 | AAD Delta計算 |
| `GET /api/market/rates` | market_handlers::get_market_rates | ✅ 存在 | レート一覧取得 |

#### フロントエンド（demo/gui/static）

| ファイル | 状態 | 説明 |
|---------|------|------|
| index.html `#irs-bootstrap-view` | ✅ 存在 | 現在のCurve Build画面（IRS統合済み） |
| app.js `handleBootstrap()` | ✅ 存在 | Bootstrap処理ロジック |
| style.css `.irs-bootstrap-*` | ✅ 存在 | 画面スタイル |

#### データファイル

| パス | 状態 | 説明 |
|------|------|------|
| `demo/data/input/market_data/webapp_market_data.json` | ✅ 存在 | USD/EUR/JPY市場データ |
| `demo/data/input/curves/` | ❌ 未作成 | Index別Instrumentリスト用ディレクトリ |

### 1.2 既存パターン・規約

- **API設計**: RESTful、camelCase JSON、axum Router
- **状態管理**: `Arc<AppState>` によるサーバー状態共有
- **エラー形式**: `(StatusCode, Json<ErrorResponse>)` パターン
- **WebSocket**: リアルタイム更新通知（bootstrap_complete等）
- **フロントエンド**: Vanilla JS、Chart.js、Font Awesome

---

## 2. 要件とのギャップ分析

### Requirement 1: Index別Instrument入力データ管理

| 技術要素 | 現状 | ギャップ | 難易度 |
|---------|------|---------|--------|
| ディレクトリ構造 | `demo/data/input/market_data/`のみ | `curves/`ディレクトリ未作成 | 低 |
| Index別ファイル形式 | 通貨別JSONあり | Index単位への再構成必要 | 中 |
| ファイル読み込みAPI | 静的ファイル配信のみ | 専用エンドポイント必要 | 中 |
| デフォルトサポート | USD/EUR/JPYあり | SOFR/ESTR/TONA形式へ変換必要 | 低 |

**ギャップタイプ**: Missing（新規作成）

### Requirement 2: レート入力インターフェース

| 技術要素 | 現状 | ギャップ | 難易度 |
|---------|------|---------|--------|
| 編集可能テーブル | Par Rate入力フォームあり | Index切替・動的行追加未対応 | 中 |
| バリデーション | 基本的な数値チェック | 範囲チェック・精度制御強化必要 | 低 |
| 変更ハイライト | なし | 新規実装必要 | 低 |
| JSON エクスポート | Market Dataにあり | Curve Builder用に移植必要 | 低 |
| JSON インポート | なし | 新規実装必要 | 中 |
| リセット機能 | Presetボタンあり | ファイル値へのリセット拡張必要 | 低 |

**ギャップタイプ**: Extend + Missing

### Requirement 3: カーブBuilderモデル選択

| 技術要素 | 現状 | ギャップ | 難易度 |
|---------|------|---------|--------|
| 補間手法選択 | Linear/LogLinearのみUI | CubicSpline/Monotonic追加必要 | 低 |
| ブートストラップ手法 | Sequential固定 | Global選択UI追加必要 | 中 |
| 設定パラメータ表示 | なし | tolerance/max_iterations表示必要 | 低 |
| プリセット保存 | なし | LocalStorage or API必要 | 中 |
| 互換性チェック | なし | バックエンド検証ロジック必要 | 中 |

**ギャップタイプ**: Extend + Missing

### Requirement 4: カーブ構築実行

| 技術要素 | 現状 | ギャップ | 難易度 |
|---------|------|---------|--------|
| Build Curveボタン | ✅ 存在 | 変更不要 | - |
| プログレス表示 | 基本的な状態表示あり | ステップ情報拡張必要 | 低 |
| 結果サマリ | 処理時間・Pillar数表示 | 成功/失敗ステータス強化 | 低 |
| エラー詳細 | 基本エラー表示 | 収束エラー詳細追加必要 | 低 |
| キャッシュ・再構築通知 | CachedCurve実装あり | UI通知追加必要 | 低 |

**ギャップタイプ**: Extend

### Requirement 5: Parameterカーブ表示

| 技術要素 | 現状 | ギャップ | 難易度 |
|---------|------|---------|--------|
| 表示モード切替 | チャート1種のみ | DF/ZeroRate/ForwardRateタブ必要 | 中 |
| チャート表示 | Chart.js実装あり | データセット切替ロジック必要 | 低 |
| テーブル表示 | なし | 新規実装必要 | 中 |
| ツールチップ | 基本あり | カスタマイズ強化必要 | 低 |
| Tenor範囲設定 | なし | 新規実装必要 | 中 |
| エクスポート | なし | CSV/JSON出力必要 | 低 |

**ギャップタイプ**: Extend + Missing

### Requirement 6: IRS評価機能の削除

| 技術要素 | 現状 | ギャップ | 難易度 |
|---------|------|---------|--------|
| UI要素削除 | IRS Parameters統合済み | HTMLセクション削除 | 低 |
| API呼び出し除去 | price-irs呼び出しあり | JS呼び出し削除 | 低 |
| リダイレクト処理 | なし | 旧URL対応必要 | 低 |
| ドキュメント更新 | なし | 移動先記載必要 | 低 |

**ギャップタイプ**: Remove（単純削除）

### Requirement 7: API設計

| 技術要素 | 現状 | ギャップ | 難易度 |
|---------|------|---------|--------|
| GET /api/curves/instruments/{index} | なし | 新規実装必要 | 中 |
| POST /api/curves/build | /api/bootstrap存在 | 名前変更 or ラップ | 低 |
| GET /api/curves/{id}/parameters | なし | 新規実装必要 | 中 |
| GET /api/curves/builders | なし | 新規実装必要 | 低 |
| RFC 7807エラー | 独自形式 | Problem Details対応必要 | 中 |
| OpenAPI仕様 | utoipa基盤あり | 新エンドポイント追記必要 | 低 |

**ギャップタイプ**: Missing + Extend

---

## 3. 実装アプローチ選択肢

### Option A: 既存コンポーネント拡張

**適用条件**: 既存のirs-bootstrap-viewをベースに改修

**拡張対象**:
- `handlers.rs`: bootstrap_curveを拡張、新エンドポイント追加
- `pricer_types.rs`: BootstrapRequest拡張、新型追加
- `index.html#irs-bootstrap-view`: IRS部分削除、Parameter表示追加
- `app.js`: handleBootstrap拡張

**トレードオフ**:
- ✅ 既存実装の再利用、開発速度
- ✅ 既存テスト・パターン活用
- ❌ 既存コードの複雑化リスク
- ❌ IRS削除による影響範囲調査必要

### Option B: 新規コンポーネント作成

**適用条件**: Curve Builder専用の独立モジュール作成

**新規作成対象**:
- `curve_handlers.rs`: 専用APIハンドラ
- `curve_types.rs`: 専用型定義
- `index.html#curve-builder-view`: 新規ビュー
- `js/curve-builder.js`: 専用JSモジュール

**トレードオフ**:
- ✅ 責務の明確な分離
- ✅ 既存機能への影響なし
- ❌ コード重複の可能性
- ❌ 開発コスト増加

### Option C: ハイブリッドアプローチ（推奨）

**戦略**:
1. **Phase 1**: 既存irs-bootstrap-viewからIRS機能を分離・削除
2. **Phase 2**: 新規curve_handlers.rsで新APIエンドポイント実装
3. **Phase 3**: フロントエンドを段階的に拡張

**具体的な分担**:
- **拡張**: `handlers.rs`（bootstrap_curve維持）、`pricer_types.rs`（型追加）
- **新規**: `curve_handlers.rs`（instrument/builder API）、`demo/data/input/curves/`
- **削除**: IRS関連UI・API呼び出し

**トレードオフ**:
- ✅ バランスの取れたアプローチ
- ✅ 段階的実装による低リスク
- ✅ 既存機能と新機能の明確な境界
- ❌ 計画の複雑性

---

## 4. 工数・リスク評価

### 工数見積

| 要件 | 工数 | 根拠 |
|------|------|------|
| Req 1: 入力データ管理 | S (1-3日) | ファイル構造作成、単純なAPIエンドポイント |
| Req 2: レート入力UI | M (3-7日) | 既存UIの拡張、バリデーション強化 |
| Req 3: Builderモデル選択 | M (3-7日) | UI追加、バックエンド設定連携 |
| Req 4: カーブ構築実行 | S (1-3日) | 既存機能の軽微な拡張 |
| Req 5: Parameterカーブ表示 | M (3-7日) | 新規タブUI、チャート切替、テーブル実装 |
| Req 6: IRS機能削除 | S (1-3日) | 単純な削除作業 |
| Req 7: API設計 | M (3-7日) | 新エンドポイント実装、OpenAPI更新 |

**総工数**: L (1-2週間)

### リスク評価

| リスク | レベル | 緩和策 |
|--------|--------|--------|
| IRS削除による他画面への影響 | 中 | 影響範囲の事前調査、Trade Pricing画面への移行確認 |
| フロントエンド複雑化 | 低 | モジュール分割、既存パターン踏襲 |
| バックエンド互換性 | 低 | 既存API維持、新APIは別パスで追加 |
| パフォーマンス（大量Instrument） | 低 | 既存ブートストラップエンジンは最適化済み |

**総合リスク**: Medium

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ
**Option C: ハイブリッドアプローチ**を推奨

### 主要な設計決定事項

1. **API構造**: 既存`/api/bootstrap`を維持しつつ、`/api/curves/*`を新設
2. **データファイル形式**: `demo/data/input/curves/{index}.json`形式を採用
3. **UI構造**: `#irs-bootstrap-view`をリネーム（`#curve-builder-view`）し、IRS部分を削除

### 調査継続項目

| 項目 | 理由 | 優先度 |
|------|------|--------|
| Trade Pricing画面へのIRS移行パス | IRS機能の代替先確認 | 高 |
| LocalStorage vs API保存 | Builderプリセット保存方式 | 中 |
| Forward Rate計算精度 | 補間手法による影響確認 | 中 |

---

## 6. 要件-資産マッピング

| 要件 | 既存資産 | ギャップタグ |
|------|---------|-------------|
| Req 1 | webapp_market_data.json | Missing（curves/ディレクトリ） |
| Req 2 | par-rate-form | Extend + Missing |
| Req 3 | interpolation-method select | Extend + Missing |
| Req 4 | bootstrap_curve handler | Extend |
| Req 5 | curve-chart canvas | Extend + Missing |
| Req 6 | irs-params-section | Remove |
| Req 7 | handlers.rs, mod.rs router | Extend + Missing |
