# Gap Analysis: volcube-calibration-ui

## 1. 現状調査

### 1.1 再利用可能な既存資産

#### バックエンド（VolCubeエンジン - Swaption用）
| コンポーネント | ファイル | 状態 | 備考 |
|--------------|---------|------|------|
| `VolCubeBuilder<T>` | `pricer_models/src/market/volcube/builder.rs` | ✅ 完全実装 | Fluent API、キャッシュ統合 |
| `VolCube<T>` | `volcube/cube.rs` | ✅ 完全実装 | SABR補間、3D vol取得 |
| `VolatilityCube` trait | `volcube/cube.rs` | ✅ 完全実装 | `volatility()`, `probability_density()`, `cumulative_probability()` |
| `VolCubeConfig` | `volcube/config.rs` | ✅ 完全実装 | 補間/外挿/Strike軸/最適化設定 |
| `VolInstrument<T>` | `volcube/types.rs` | ✅ 完全実装 | expiry, tenor, strike, implied_vol, forward, weight |
| `SabrParams<T>` | `volcube/types.rs` | ✅ 完全実装 | alpha, beta, rho, nu |
| `BreedenLitzenberger` | `volcube/breeden_litzenberger.rs` | ✅ 完全実装 | PDF/CDF計算（VolCube用） |
| `VolCubeCache<T>` | `volcube/cache.rs` | ✅ 完全実装 | LRUキャッシュ |
| `SabrCalibrator`, `SviCalibrator` | `volcube/calibrator.rs` | ✅ 完全実装 | キャリブレーション |
| `VolCubeGraphData` | `volcube/graph.rs` | ✅ 完全実装 | AD用グラフ抽出 |

#### バックエンド（FX VolSurfaceエンジン）
| コンポーネント | ファイル | 状態 | 備考 |
|--------------|---------|------|------|
| `FxVolatilitySurface<T>` | `surfaces/fx.rs` | ✅ 完全実装 | Delta × Expiry グリッド |
| `FxDeltaPoint` enum | `surfaces/fx.rs` | ✅ 完全実装 | 10D Put〜10D Call |
| `volatility_by_delta()` | `surfaces/fx.rs` | ✅ 完全実装 | Bilinear補間 |
| `atm_volatility()` | `surfaces/fx.rs` | ✅ 完全実装 | ATM vol取得 |
| `risk_reversal_25d()` | `surfaces/fx.rs` | ✅ 完全実装 | 25D RR計算 |
| `butterfly_25d()` | `surfaces/fx.rs` | ✅ 完全実装 | 25D BF計算 |
| `VolSurfaceEnum::FxSurface` | `surfaces/vol_surface_enum.rs` | ✅ 完全実装 | Dynamic dispatch |
| FX用確率密度計算 | - | ❌ **未実装** | BreedenLitzenbergerのFxVolSurface対応が必要 |
| Delta-Strike変換 | - | ❌ **未実装** | Garman-Kohlhagen公式による変換が必要 |

#### Demo GUI インフラ
| コンポーネント | ファイル | 状態 | 備考 |
|--------------|---------|------|------|
| `AppState` | `demo/gui/src/web/mod.rs` | ✅ 完全実装 | 共有状態、キャッシュ管理 |
| `ApiError`, `ApiResult` | `web/error.rs` | ✅ 完全実装 | エラーハンドリングパターン |
| `CurveDataLoader` | `web/curve_builder_handlers.rs` | ✅ 完全実装 | JSONファイル読み込みパターン |
| Router構成 | `web/mod.rs` | ✅ 完全実装 | `/api/curves/*` パターン |
| Chart.js統合 | `demo/gui/static/index.html` | ✅ 完全実装 | 2D/3Dチャート |

#### 既存データファイル
| パス | 内容 | 状態 |
|------|------|------|
| `demo/data/input/curves/` | USD-SOFR, EUR-ESTR, JPY-TONA | ✅ 存在 |
| `demo/data/input/volcube/` | VolCube用インストゥルメント | ❌ 未作成 |

### 1.2 コード規約・パターン

- **ハンドラー命名**: `get_*`, `post_*` 形式
- **型定義**: 別ファイル（`*_types.rs`）に分離
- **データローダー**: `*DataLoader` 構造体パターン
- **ルーティング**: `/api/{domain}/{resource}` 形式
- **エラー処理**: `ApiError::not_found()`, `ApiError::validation()` パターン

---

## 2. 要件実現可能性分析

### 要件 → 既存資産マッピング

| 要件 | 既存資産 | ギャップ | 複雑度 |
|------|---------|---------|--------|
| **Req 1: ボラティリティデータ管理** | `VolInstrument<T>`, `FxVolatilitySurface<T>`, `CurveDataLoader` | JSONファイル、ローダー、API型定義 | M |
| **Req 2: 依存カーブ統合** | `curve_cache`, `/api/curves/*` | UIセクション、カーブ選択API | S |
| **Req 3: キャリブレーション設定** | `VolCubeConfig` (全設定項目完備) | API型変換のみ | S |
| **Req 4: パラメータ表示** | `SabrParams<T>`, `CalibrationDiagnostics` | API型変換、UIテーブル | S |
| **Req 5: スマイル可視化** | `VolatilityCube::volatility()`, `FxVolatilitySurface::volatility_by_delta()` | スマイルデータAPI、Chart.js統合 | M |
| **Req 6: 確率密度可視化** | `BreedenLitzenberger` (VolCube用) | 密度データAPI、統計計算 | M |
| **Req 7: 3Dサーフェス** | `VolCube` グリッドデータ | 3Dデータ整形、Three.js/Plotly | L |
| **Req 8: バックエンドAPI (VolCube)** | Router構成、`AppState` | 7エンドポイント実装 | M |
| **Req 9: サンプルデータ** | データフォーマットパターン確立済み | JSONファイル作成（FX追加） | S |
| **Req 10: FX VolSurface専用機能** | `FxVolatilitySurface<T>`, `risk_reversal_25d()`, `butterfly_25d()` | FX確率密度、Delta-Strike変換 | M |
| **Req 11: FX API** | Router構成 | 8エンドポイント実装 | M |

### 技術的ギャップ詳細

#### Missing: API層
```
demo/gui/src/web/
├── volcube_types.rs      ← 新規作成 (Swaption VolCube用)
├── volcube_handlers.rs   ← 新規作成 (Swaption VolCube用)
├── fxvol_types.rs        ← 新規作成 (FX VolSurface用)
├── fxvol_handlers.rs     ← 新規作成 (FX VolSurface用)
└── mod.rs               ← ルート追加
```

#### Missing: バックエンド拡張
```
crates/pricer_models/src/market/surfaces/
├── fx.rs                 ← 拡張: probability_density(), delta_to_strike()
└── fx_density.rs         ← 新規作成: FX用Breeden-Litzenberger実装
```

#### Missing: データファイル
```
demo/data/input/volsurface/
├── usd-sofr-swaption.json   ← 新規作成
├── eur-estr-swaption.json   ← 新規作成
├── eurusd-fx.json           ← 新規作成 (ATM, RR, BF形式)
├── usdjpy-fx.json           ← 新規作成
├── spx-equity-options.json  ← 新規作成
└── README.md                ← 新規作成
```

#### Missing: フロントエンド
```
demo/gui/static/
├── js/volcube-builder.js    ← 新規作成 (Swaption用)
├── js/fxvol-builder.js      ← 新規作成 (FX用)
└── index.html              ← Model Calib セクション更新
```

#### Missing: FX確率密度計算

現在の`BreedenLitzenberger`は`VolatilityCube<T>`トレイトに依存しているため、`FxVolatilitySurface`には直接適用できない。

**対応オプション**:
1. `FxVolatilitySurface`に`probability_density()`メソッドを追加
2. `BreedenLitzenberger`を`VolatilitySurface<T>`トレイトでも動作するように汎用化
3. FX専用の密度計算モジュール`fx_density.rs`を作成

**推奨**: オプション3（FX特有のGarman-Kohlhagen前提での実装）

#### Missing: Delta-Strike変換

FX市場ではDelta表記が標準だが、確率密度計算にはAbsolute Strikeが必要。

**必要な実装**:
- Garman-Kohlhagen公式によるDelta → Strike変換
- Premium-adjusted Delta vs Spot Deltaの選択
- Forward Delta vs Spot Deltaの選択

### 制約事項

1. **ジェネリック型**: `VolCube<T>` はジェネリックだが、APIでは `f64` に固定
2. **非同期**: `VolCubeBuilder::build()` は同期、長時間キャリブレーションはブロッキング
3. **3D描画**: Chart.jsは2Dのみ、3DにはPlotly.jsまたはThree.js追加が必要

---

## 3. 実装アプローチ選択肢

### Option A: 既存パターン完全踏襲（推奨）

**概要**: Curve Builderの実装パターンを完全に踏襲し、VolCube用に適用

**変更ファイル**:
- 新規: `volcube_types.rs`, `volcube_handlers.rs`
- 新規: `demo/data/input/volcube/*.json`
- 新規: `demo/gui/static/js/volcube-builder.js`
- 更新: `demo/gui/src/web/mod.rs` (ルート追加)
- 更新: `demo/gui/static/index.html` (Model Calib セクション)

**トレードオフ**:
- ✅ 既存パターンとの一貫性
- ✅ curve_builder_handlers.rsをテンプレートとして使用可能
- ✅ 学習コスト最小
- ❌ 3D描画ライブラリ選定が必要

### Option B: 統合キャリブレーションモジュール

**概要**: Curve BuilderとVolCube Builderを統合した汎用キャリブレーションモジュール

**変更ファイル**:
- 新規: `calibration_types.rs`, `calibration_handlers.rs`
- リファクタ: `curve_builder_handlers.rs` → `calibration_handlers.rs` に統合

**トレードオフ**:
- ✅ コード重複削減
- ✅ 統一API設計
- ❌ 既存Curve Builder APIの破壊的変更リスク
- ❌ 実装期間延長

### Option C: ハイブリッドアプローチ

**概要**: 共通ユーティリティを抽出しつつ、独立ハンドラーを維持

**変更ファイル**:
- 新規: `calibration_utils.rs` (共通ローダー、バリデーション)
- 新規: `volcube_types.rs`, `volcube_handlers.rs`
- 更新: `curve_builder_handlers.rs` (共通部分を抽出)

**トレードオフ**:
- ✅ 段階的リファクタリング可能
- ✅ 既存API維持
- ❌ 共通化の判断に時間

---

## 4. 工数・リスク評価

### 工数見積り

| フェーズ | 範囲 | 工数 |
|---------|------|------|
| API型定義 (`volcube_types.rs`) | 10+ struct/enum | S |
| APIハンドラー (`volcube_handlers.rs`) | 7 エンドポイント | M |
| データローダー | `VolCubeDataLoader` | S |
| サンプルデータ | 3 JSON + README | S |
| JavaScript (`volcube-builder.js`) | ~500 LOC | M |
| HTML更新 | Model Calib セクション | S |
| 3D描画統合 | Plotly.js/Three.js | M-L |
| テスト | Unit + Integration | M |

**総合工数**: **M～L (1～2週間)**

### リスク評価

| リスク | 影響度 | 緩和策 |
|--------|-------|--------|
| 3Dライブラリ選定 | Medium | Plotly.js推奨（Chart.jsと類似API） |
| キャリブレーション性能 | Medium | 既存LRUキャッシュ活用 |
| ジェネリック型変換 | Low | f64固定でシンプル化 |
| 大規模データ | Low | ページネーション、遅延ロード |

---

## 5. 設計フェーズへの推奨事項

### 優先アプローチ

**Option A（既存パターン完全踏襲）** を推奨

**理由**:
1. `curve_builder_handlers.rs` が参照実装として機能
2. 既存インフラ（AppState、エラーハンドリング、ルーティング）をそのまま活用
3. 実装リスク最小

### 設計フェーズで検討すべき項目

1. **3D描画ライブラリ選定**: Plotly.js vs Three.js
   - Plotly.js: 宣言的API、金融向けサンプル豊富
   - Three.js: 高度なカスタマイズ、学習コスト高

2. **非同期キャリブレーション**: 長時間実行時のUX
   - 同期: シンプル、短時間データ向け
   - 非同期（JobManager活用）: 大規模データ向け

3. **Strike軸変換**: UI上での表示軸切り替え
   - サーバーサイド変換 vs クライアントサイド変換

### Research Needed

1. Plotly.js 3D surface plotのパフォーマンス（~1000点グリッド）
2. SABR calibration のベンチマーク（典型的データサイズ）
3. VolCube JSONスキーマのベストプラクティス

---

## 6. 結論

### 実現可能性: **高**

既存のVolCubeバックエンドが完全に実装されており、Demo GUIのパターンも確立されているため、主にAPI層とフロントエンド統合の実装が中心となる。

### 次のステップ

1. `/kiro:spec-design volcube-calibration-ui` で技術設計を作成
2. 3D描画ライブラリを選定
3. サンプルデータのJSONスキーマを確定
