# Gap Analysis: ir-vol-cube-calibration

## 概要

本分析では、IR Vol Cubeカリブレーションエンジンの要件と既存コードベースのギャップを調査し、実装戦略の選択肢を提示する。CurveBuilderパターンを参照し、Curve→VolCubeの統合依存グラフを持つカリブレーションフレームワークを構築する。

---

## 1. 現状調査

### 1.1 関連モジュール構造

```text
pricer_models/src/market/
├── volcube/
│   ├── mod.rs              → VolCubeモジュール公開、re-exports
│   ├── cube.rs             → VolCube<T> 3D構造（Expiry×Tenor×Strike）
│   ├── calibrator.rs       → VolCubeCalibrator trait, SabrCalibrator, SviCalibrator
│   ├── config.rs           → VolCubeConfig（interpolation, extrapolation, optimizer）
│   ├── builder.rs          → VolCubeBuilder<T>（存在想定）
│   └── breeden_litzenberger.rs → Breeden-Litzenberger密度抽出
├── calibration/
│   ├── sabr.rs             → SABRCalibrator（Hagan formula）
│   └── engine.rs           → CalibrationEngine（Levenberg-Marquardt）
├── provider.rs             → MarketProvider（Arc-cached lazy evaluation）
└── curves/                 → YieldCurve trait, implementations

infra_domain/src/trade/
├── instrument_def/
│   └── rates.rs            → Swaption, CapFloor定義（既存）
└── convention/
    ├── swaption.rs         → SwaptionConvention（USD SOFR, EUR ESTR, JPY TONAR）
    └── capfloor.rs         → CapFloorConvention（USD SOFR, EUR EURIBOR）

pricer_pricing/src/
├── irs_greeks/
│   └── lazy_evaluator.rs   → IrsLazyEvaluator<T>（依存グラフ、キャッシュ、AAD tape）
└── graph/
    ├── extractor.rs        → GraphExtractable trait
    └── types.rs            → ComputationGraph, GraphNode, GraphEdge

adapter_loader/src/
└── lib.rs                  → データローダーインフラ（CSV/JSON/Parquet）

demo/gui/src/web/
└── handlers.rs             → 既存WebAppハンドラー
```

### 1.2 再利用可能コンポーネント

| コンポーネント | 場所 | 再利用可能度 |
|---------------|------|-------------|
| `Swaption` struct | infra_domain/trade/instrument_def/rates.rs | ✅ 直接利用 |
| `CapFloor` struct | infra_domain/trade/instrument_def/rates.rs | ✅ 直接利用 |
| `SwaptionConvention` | infra_domain/trade/convention/swaption.rs | ✅ 直接利用（SOFR/ESTR/TONAR） |
| `CapFloorConvention` | infra_domain/trade/convention/capfloor.rs | ✅ 拡張可能（JPY追加） |
| `VolCube<T>` | pricer_models/market/volcube/cube.rs | ✅ 拡張可能 |
| `SabrCalibrator` | pricer_models/market/volcube/calibrator.rs | ✅ 直接利用 |
| `VolCubeConfig` | pricer_models/market/volcube/config.rs | ✅ 拡張可能 |
| `BreedenLitzenberger` | pricer_models/market/volcube | ✅ 直接利用 |
| `MarketProvider` | pricer_models/market/provider.rs | ✅ パターン参考 |
| `IrsLazyEvaluator<T>` | pricer_pricing/irs_greeks/lazy_evaluator.rs | ✅ パターン参考 |
| `CalibrationEngine` | pricer_models/market/calibration/engine.rs | ✅ 直接利用 |
| `GraphExtractable` trait | pricer_pricing/graph | ✅ 実装可能 |

### 1.3 アーキテクチャパターン

- **AD互換ジェネリクス**: `T: Float` で全数値型をパラメータ化（Enzyme互換）
- **Static Dispatch**: enumによるtrait object回避（LLVM最適化）
- **Builder Pattern**: fluent APIで設定を構築
- **Arc-based Lazy Cache**: `parking_lot::RwLock` + double-check locking
- **DependencyGraph**: `IrsLazyEvaluator`のtenor→swap依存グラフパターン
- **thiserror**: 構造化エラーハンドリング
- **CurveName型**: String wrapper for curve identification

---

## 2. 要件と既存資産のマッピング

### Requirement 1: IR商品定義（Swaption・CapFloor）

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 1.1 EuropeanSwaption定義 | `Swaption` in rates.rs | ✅ 既存（expiry, tenor, strike, payer/receiver） |
| 1.2 CapFloor定義 | `CapFloor` in rates.rs | ✅ 既存（cap_strike, floor_strike, underlying_tenor） |
| 1.3 USD SOFR convention | `SwaptionConvention::usd_sofr()` | ✅ 既存 |
| 1.4 EUR ESTR convention | `SwaptionConvention::eur_euribor()` | ⚠️ EURIBOR存在、ESTR追加必要 |
| 1.5 JPY TONA convention | `SwaptionConvention::jpy_tonar()` | ✅ 既存（TONAR=TONA） |
| 1.6 underlying schedule自動生成 | なし | ❌ **新規実装** |
| 1.7 serde serialisation | `#[derive(Serialize, Deserialize)]` | ✅ 既存 |

**ギャップ**: underlying swap/cap scheduleの自動生成メソッド追加が必要

### Requirement 2: VolCubeデータ構造

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 2.1 3次元グリッド（expiry×tenor×strike） | `VolCube<T>` | ✅ 既存 |
| 2.2 market quote（bid/ask/mid）格納 | なし | ⚠️ **拡張必要** |
| 2.3 任意座標でのvol取得 | `VolCube::get_vol()` | ✅ 既存（拡張可能） |
| 2.4 カリブレーション前/後状態 | なし | ❌ **新規設計** |
| 2.5 Currency/index metadata | `Currency` enum | ⚠️ index追加必要 |

**ギャップ**: bid/ask/mid quote格納、カリブレーション状態管理の追加

### Requirement 3: VolCube補間器フレームワーク

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 3.1 `VolCubeInterpolator` trait | `VolCubeCalibrator` trait | ✅ 類似構造存在 |
| 3.2 `SabrInterpolator` | `SabrCalibrator` | ✅ 既存 |
| 3.3 `FlatInterpolator`, `LinearInterpolator` | なし | ❌ **新規実装** |
| 3.4 パラメータ軸別補間設定 | `VolCubeConfig` | ✅ 拡張可能 |
| 3.5 `VolCubeBuilder` interpolator設定 | `VolCubeBuilder` | ✅ 拡張可能 |
| 3.6 static dispatch（enum-based） | `SviCalibrator`, `SabrCalibrator` | ✅ パターン存在 |

**ギャップ**: Flat/Linear補間器の追加、enum-based static dispatchの整理

### Requirement 4: SABRモデルカリブレーション

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 4.1 SABR calibration | `SabrCalibrator` | ✅ 既存 |
| 4.2 β固定モード | `SabrCalibrator::with_fixed_beta()` | ✅ 確認必要 |
| 4.3 Levenberg-Marquardt | `CalibrationEngine` | ✅ 既存 |
| 4.4 `CalibrationError`詳細 | `CalibrationError` | ✅ 既存 |
| 4.5 パラメータ境界制約 | `SabrCalibrator` | ✅ 確認必要 |
| 4.6 Breeden-Litzenberger検証 | `BreedenLitzenberger` | ✅ 既存 |
| 4.7 診断データ（残差、反復回数） | `CalibrationResult` | ⚠️ 拡張必要 |

**ギャップ**: 診断データの詳細化

### Requirement 5: カリブレーションエンジン（CurveBuilder参照）

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 5.1 instrument list + VolCube設定入力 | `VolCubeBuilder` | ✅ パターン存在 |
| 5.2 全スライスカリブレーション | `SabrCalibrator` per-slice | ✅ ループ実装必要 |
| 5.3 YieldCurve依存 | なし | ❌ **新規設計** |
| 5.4 カリブレーション順序設定 | なし | ❌ **新規設計** |
| 5.5 進捗報告（callback/channel） | なし | ❌ **新規設計** |
| 5.6 `CalibratedVolCube`返却 | `VolCube<T>` | ✅ 拡張可能 |
| 5.7 `VolatilitySurface` trait実装 | なし | ❌ **新規実装** |
| 5.8 discount/projection curve指定 | なし | ❌ **新規設計** |
| 5.9 CurveSet解決 | `MarketProvider` | ✅ パターン参考 |
| 5.10 通貨デフォルトcurve | なし | ❌ **新規設計** |

**ギャップ**: Curve依存解決、進捗報告、VolatilitySurface trait実装

### Requirement 6: LazyValuation・キャッシュ最適化（統合依存グラフ）

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 6.1 lazy initialization | `MarketProvider` | ✅ パターン参考 |
| 6.2 expiry-tenorスライス単位キャッシュ | `IrsLazyEvaluator` cache | ✅ パターン参考 |
| 6.3 同一座標キャッシュ返却 | `CachedResult<T>` | ✅ パターン参考 |
| 6.4 thread-safe cache | `DashMap` / `RwLock<HashMap>` | ✅ 利用可能 |
| 6.5 quote更新時無効化 | `DependencyGraph` | ⚠️ 拡張必要 |
| 6.6 メトリクス提供 | `CacheStats` | ✅ パターン参考 |
| 6.7 `lazy_evaluator`パターン踏襲 | `IrsLazyEvaluator` | ✅ パターン参考 |
| 6.8 CurveBuilder依存解決 | なし | ❌ **新規設計** |
| 6.9 `CalibrationGraph`管理 | なし | ❌ **新規設計** |
| 6.10 依存Curve自動カリブレーション | なし | ❌ **新規設計** |
| 6.11 AADモードでの完全パス保持 | `AadTapeCache` | ✅ パターン参考 |
| 6.12 ノード・エッジ明示管理 | `DependencyGraph` | ✅ 拡張可能 |

**ギャップ**: CalibrationGraph（Curve→VolCube依存）の新規設計が最重要

### Requirement 7: AAD（Adjoint Algorithmic Differentiation）統合

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 7.1 `DualNumber`互換`T: Float` | 全既存コンポーネント | ✅ 一貫 |
| 7.2 Enzyme AADモード | pricer_pricing enzyme | ✅ 統合可能 |
| 7.3 完全依存関係グラフ | `ComputationGraph` | ✅ 拡張可能 |
| 7.4 `GraphExtractable`実装 | trait存在 | ✅ 実装可能 |
| 7.5 forward/adjoint mode | Enzyme | ✅ 統合可能 |
| 7.6 bump-and-revalueクロス検証 | なし | ❌ **新規実装** |
| 7.7 smooth approximation | `pricer_core::math::smoothing` | ✅ 利用可能 |
| 7.8 CurveQuote→Price完全パス | なし | ❌ **新規設計** |
| 7.9 間接的Curve感応度 | なし | ❌ **新規設計** |
| 7.10 Vega + Curve Sensitivity同時計算 | なし | ❌ **新規設計** |

**ギャップ**: Curve経由の間接的感応度計算パスの設計

### Requirement 8: デモWebApp統合

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 8.1 `/api/volcube/calibrate` endpoint | なし | ❌ **新規実装** |
| 8.2 通貨選択UI | curve-builder-webapp UI | ✅ パターン参考 |
| 8.3 3Dサーフェス可視化 | なし | ❌ **新規実装** |
| 8.4 SABRパラメータグリッド表示 | なし | ❌ **新規実装** |
| 8.5 market vs fitted比較チャート | なし | ❌ **新規実装** |
| 8.6 Breeden-Litzenberger密度可視化 | なし | ❌ **新規実装** |
| 8.7 エラー詳細表示 | existing error handling | ✅ 拡張可能 |
| 8.8 curve-builder UIパターン踏襲 | demo/gui handlers | ✅ パターン参考 |

**ギャップ**: WebApp関連は全面新規実装（フロントエンド含む）

### Requirement 9: 既存実装の活用と不要コードの削除

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 9.1 `volcube`モジュール拡張 | `pricer_models::market::volcube` | ✅ 拡張可能 |
| 9.2 `SabrCalibrator`再利用 | `pricer_models::market::calibration::sabr` | ✅ 直接利用 |
| 9.3 `VolCubeBuilder`パターン継承 | 既存設計 | ✅ 拡張可能 |
| 9.4 不要コード削除 | - | ⚠️ 影響分析必要 |
| 9.5 deprecated API削除 | - | ⚠️ 影響分析必要 |
| 9.6 影響範囲分析 | - | ⚠️ 設計時実施 |
| 9.7 dead code排除 | - | ⚠️ 設計時実施 |

**ギャップ**: 統合後の不要コード特定と削除

### Requirement 10: 入力データローダー

| 受入条件 | 既存資産 | ギャップ |
|---------|---------|---------|
| 10.1 swaption vol JSON/CSV | `adapter_loader` | ✅ 拡張可能 |
| 10.2 capfloor vol JSON/CSV | `adapter_loader` | ✅ 拡張可能 |
| 10.3 expiry/tenor/strike/vol format | なし | ❌ **新規設計** |
| 10.4 `LoaderError` | 既存エラー型 | ✅ 利用可能 |
| 10.5 行番号付きパースエラー | なし | ❌ **新規実装** |
| 10.6 `demo/data/input/volsurface/`規約 | なし | ❌ **新規設計** |
| 10.7 `VolCubeBuilder`互換型変換 | なし | ❌ **新規実装** |

**ギャップ**: vol surface専用ローダーの新規実装

---

## 3. 実装アプローチ選択肢

### Option A: 既存volcubeモジュール拡張のみ

**戦略**: `pricer_models::market::volcube`を中心に拡張、Curve依存は外部注入

**対象ファイル**:
- `volcube/cube.rs`: quote格納、状態管理追加
- `volcube/config.rs`: curve reference追加
- `volcube/calibrator.rs`: Curve依存を引数で受け取る

**利点**:
- ✅ 最小限のファイル追加
- ✅ 既存volcubeとの互換性維持
- ✅ 学習コスト低

**欠点**:
- ❌ CalibrationGraphが分散
- ❌ Curve→VolCubeの依存管理が不明確
- ❌ LazyValuation統合が困難

### Option B: 完全新規CalibrationGraphシステム

**戦略**: `pricer_pricing`に統合`CalibrationGraph`を新設、Curve/VolCubeを統一管理

**新規ファイル**:
```text
pricer_pricing/src/calibration_graph/
├── mod.rs              → モジュール公開
├── graph.rs            → CalibrationGraph（DAG管理）
├── node.rs             → CalibrationNode（Curve/VolCube/etc）
├── resolver.rs         → 依存解決、トポロジカルソート
├── lazy_cache.rs       → 統合LazyCache
└── aad_tape.rs         → AADテープ管理
```

**利点**:
- ✅ Curve→VolCubeの統合管理
- ✅ AAD完全パスの実現
- ✅ 将来の拡張性（他のカリブレーション対象）

**欠点**:
- ❌ 大規模リファクタリング必要
- ❌ 既存CurveBuilderとの統合複雑
- ❌ 実装工数大

### Option C: ハイブリッドアプローチ（推奨）

**戦略**:
1. 既存`volcube`モジュールを拡張（Requirement 1-4, 9）
2. `MarketProvider`を拡張してCurve→VolCube依存を管理（Requirement 5-7）
3. WebApp/Loaderは独立実装（Requirement 8, 10）

**構成**:

```text
Phase 1: volcube拡張
├── pricer_models/src/market/volcube/
│   ├── cube.rs         → bid/ask/mid quote、状態管理追加
│   ├── engine.rs       → VolCubeCalibrationEngine（新規）
│   └── config.rs       → CurveName参照追加

Phase 2: 統合依存グラフ
├── pricer_models/src/market/
│   └── provider.rs     → MarketProvider拡張（Curve→VolCube依存）
├── pricer_pricing/src/
│   └── vol_lazy_evaluator.rs → VolCube用LazyEvaluator（新規）

Phase 3: AAD統合
├── pricer_pricing/src/graph/
│   └── extractor.rs    → VolCube GraphExtractable実装
├── pricer_pricing/src/
│   └── aad_integration.rs → 統合AADパス（新規）

Phase 4: WebApp・Loader
├── demo/gui/src/web/
│   └── volcube_handlers.rs → WebApp handlers（新規）
├── adapter_loader/src/
│   └── volsurface.rs   → vol surface loader（新規）
```

**利点**:
- ✅ 既存コード最大限再利用
- ✅ 段階的実装でリスク分散
- ✅ `MarketProvider`パターンの一貫性維持
- ✅ AADパスの段階的構築

**欠点**:
- ❌ `MarketProvider`の責任増大
- ❌ 複数フェーズの調整必要

---

## 4. 複雑性とリスク評価

### 工数見積もり

| タスク | 見積もり | 根拠 |
|--------|---------|------|
| Requirement 1-4: volcube拡張 | **M** (3-7日) | 既存コード活用、quote/状態管理追加 |
| Requirement 5: CalibrationEngine | **M** (3-7日) | CurveBuilderパターン参照、Curve依存解決 |
| Requirement 6: LazyValuation統合 | **L** (1-2週) | CalibrationGraph設計、依存解決実装 |
| Requirement 7: AAD統合 | **M** (3-7日) | 既存パターン適用、テスト重視 |
| Requirement 8: WebApp | **M** (3-7日) | 3D可視化、複数チャート |
| Requirement 9: コード整理 | **S** (1-3日) | 影響分析後の削除作業 |
| Requirement 10: Loader | **S** (1-3日) | adapter_loaderパターン踏襲 |

**総合見積もり**: **XL** (2週間以上)

### リスク評価

| リスク | レベル | 緩和策 |
|--------|--------|--------|
| CalibrationGraph設計 | High | IrsLazyEvaluator詳細分析、段階的実装 |
| Curve→VolCube依存解決 | High | MarketProviderの詳細設計、テスト駆動 |
| AAD完全パス | Medium | 既存Enzymeパターン活用、段階的統合 |
| 3D可視化 | Medium | 既存curve-builder UIパターン参照 |
| 不要コード特定 | Low | 段階的削除、影響分析ツール活用 |

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option C (ハイブリッド)** を推奨。理由:
1. 既存`volcube`モジュールの大部分が再利用可能
2. `MarketProvider`パターンの拡張でCurve依存を自然に統合
3. 段階的実装（Phase 1-4）でリスク分散
4. `IrsLazyEvaluator`パターンを`VolLazyEvaluator`として適用可能

### 設計フェーズで検討すべき項目

1. **CalibrationGraph設計**:
   - Curve/VolCubeノードの抽象化レベル
   - トポロジカルソートの実装
   - 循環依存の検出・防止

2. **Curve依存解決**:
   - `CurveName` → `CurveSet` → 具体Curveの解決パス
   - 通貨別デフォルトCurve設定
   - 欠損Curveのエラーハンドリング

3. **LazyValuation境界**:
   - どこまでlazyに評価するか（Curve? VolCube? Price?）
   - キャッシュ無効化のスコープ
   - メモリ使用量の制限

4. **AADパス設計**:
   - forward vs adjoint modeの選択基準
   - テープサイズの最適化
   - bump-and-revalue検証の自動化

5. **WebApp UX**:
   - 3Dサーフェス表示ライブラリ選定（plotly.js推奨）
   - リアルタイム更新 vs オンデマンド更新
   - エラー状態の視覚的フィードバック

### Research Needed

- **CurveBuilderとの統合パターン**: 既存`BootstrappedCurveBuilder`の詳細分析
- **MarketProvider拡張可能性**: `get_volcube(currency)`メソッド追加の影響
- **Enzyme AAD制約**: VolCubeカリブレーションへのEnzyme適用の技術的制約
- **3D補間戦略**: SABR per-slice vs 完全3D補間のトレードオフ

---

## 6. 主要ギャップサマリー

### Critical Gaps（設計必須）

| ギャップ | 要件 | 優先度 |
|---------|------|--------|
| CalibrationGraph設計 | Req 5, 6 | **Critical** |
| Curve→VolCube依存解決 | Req 5.8-5.10, 6.8-6.10 | **Critical** |
| AAD完全パス | Req 7.8-7.10 | **High** |

### Medium Gaps（実装必要）

| ギャップ | 要件 | 優先度 |
|---------|------|--------|
| underlying schedule生成 | Req 1.6 | Medium |
| bid/ask/mid quote格納 | Req 2.2 | Medium |
| Flat/Linear補間器 | Req 3.3 | Medium |
| 進捗報告callback | Req 5.5 | Medium |
| WebApp全体 | Req 8 | Medium |
| vol surface loader | Req 10 | Medium |

### Low Gaps（既存拡張）

| ギャップ | 要件 | 優先度 |
|---------|------|--------|
| EUR ESTR convention | Req 1.4 | Low |
| 診断データ詳細化 | Req 4.7 | Low |
| 不要コード削除 | Req 9 | Low |

---

_Generated: 2026-01-25_
_Document patterns and gaps, not exhaustive file listings_
