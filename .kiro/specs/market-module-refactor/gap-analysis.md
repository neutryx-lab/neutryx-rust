# Gap Analysis: market-module-refactor

## 概要

本ドキュメントは`pricer_models::market`モジュール（82ファイル、約33,000 LOC）のリファクタリングに向けたギャップ分析を提供する。

---

## 1. 現状調査

### 1.1 モジュール構造

```
market/                          (9ファイル, 5,095 LOC)
├── mod.rs                       公開API・re-export
├── error.rs                     MarketDataError, MarketBuildError
├── provider.rs                  MarketProvider（遅延解決）
├── fx_density.rs                FxDensityCalculator
├── indexed_market.rs            IndexedMarket, IndexedMarketBuilder
├── validator.rs                 MarketValidator
├── requirements.rs              TradeIndexRequirements
├── index_mapper.rs              IndexCurveMapper
│
├── calibration/                 (11ファイル, ~2,000 LOC)
│   ├── mod.rs                   公開API・re-export (99行)
│   ├── engine.rs                CalibrationEngine (主要実装)
│   ├── model_calibrator.rs      [LEGACY] GenericCalibrator (旧実装)
│   ├── error.rs                 CalibrationError
│   ├── result.rs                CalibrationResult, CalibrationDiagnostics
│   ├── targets.rs               CalibrationTarget
│   ├── heston.rs                HestonCalibrator
│   ├── hull_white.rs            HullWhiteCalibrator
│   ├── sabr.rs                  SABRCalibrator
│   ├── swaption_calibrator.rs   SwaptionCalibrator
│   │
│   └── bootstrapping/           (17ファイル, ~13,500 LOC)
│       ├── mod.rs               公開API・re-export
│       ├── engine.rs            SequentialBootstrapper
│       ├── engine_error.rs      CurveEngineError
│       ├── error.rs             BootstrapError
│       ├── curve.rs             BootstrappedCurve
│       ├── curve_builder.rs     CurveBootstrapper
│       ├── curve_config.rs      CurveConfig
│       ├── curve_engine.rs      CurveEngine
│       ├── config.rs            GenericBootstrapConfig
│       ├── definition.rs        CurveDefinition (59K - 最大)
│       ├── multi_curve.rs       MultiCurveBuilder (61K)
│       ├── adjoint_solver.rs    AdjointSolver (AAD)
│       ├── sensitivity.rs       SensitivityBootstrapper
│       ├── adapter.rs           InstrumentAdapter
│       ├── instrument.rs        BootstrapInstrument
│       ├── date_utils.rs        DateCalculator
│       ├── cache.rs             BootstrapCache
│       └── result_cache.rs      CurveResultCache
│
├── volcube/                     (21ファイル, 16,502 LOC)
│   ├── mod.rs                   公開API・re-export (115行)
│   ├── cube.rs                  VolCube, VolatilityCube
│   ├── builder.rs               VolCubeBuilder (47K)
│   ├── engine.rs                VolCubeCalibrationEngine
│   ├── calibrator.rs            SabrCalibrator, SviCalibrator
│   ├── calibration_graph.rs     CalibrationGraph (55K - 最大)
│   ├── config.rs                VolCubeConfig (44K)
│   ├── error.rs                 VolCubeError (27K)
│   ├── cache.rs                 VolCubeCache
│   ├── interpolator.rs          VolCubeInterpolator
│   ├── lazy_evaluator.rs        VolLazyEvaluator
│   ├── quote.rs                 VolQuote, VolQuoteSet (47K)
│   ├── types.rs                 InstrumentId, SabrParams
│   ├── sabr_surface.rs          SabrParameterSurface
│   ├── vega.rs                  VolCubeVegaCalculator
│   ├── sensitivity_path.rs      SensitivityPathBuilder
│   ├── breeden_litzenberger.rs  BreedenLitzenberger
│   ├── graph.rs                 VolCubeGraphData (D3.js)
│   ├── loader_convert.rs        データ変換ユーティリティ
│   ├── aad_validation.rs        AADCrossValidator
│   └── proptest_tests.rs        [TEST] プロパティテスト
│
├── curves/                      (6ファイル, 3,539 LOC)
│   ├── mod.rs
│   ├── traits.rs                YieldCurve trait
│   ├── flat.rs                  FlatCurve
│   ├── interpolated.rs          InterpolatedCurve
│   ├── credit.rs                CreditCurve, HazardRateCurve
│   ├── curve_enum.rs            CurveEnum (static dispatch)
│   └── curve_set.rs             CurveSet
│
├── surfaces/                    (6ファイル, 2,324 LOC)
│   ├── mod.rs
│   ├── traits.rs                VolatilitySurface trait
│   ├── flat.rs                  FlatVol
│   ├── interpolated.rs          InterpolatedVolSurface
│   ├── fx.rs                    FxVolatilitySurface
│   ├── vol_surface_enum.rs      VolSurfaceEnum
│   └── volcube_slice.rs         VolCubeSlice (adapter)
│
└── fx_calibration/              (10ファイル, 6,078 LOC)
    ├── mod.rs
    ├── types.rs                 Strike, Vol, ForwardPoints
    ├── config.rs                FxVolSurfaceConfig
    ├── error.rs                 FxCalibrationError
    ├── curve.rs                 FxCurve, CalibratedFxCurve
    ├── builder.rs               FxForwardCurveBuilder
    ├── surface.rs               CalibratedFxVolSurface
    ├── vol_builder.rs           FxVolSurfaceBuilder
    ├── lazy_surface.rs          LazyFxVolSurface
    ├── sensitivity.rs           VolSurfaceSensitivity
    └── fx_market_builder.rs     FxMarketBuilder
```

### 1.2 Dead Code/Legacy Code の特定

| ファイル | 行 | 種別 | 状態 | コンテキスト |
|----------|-----|------|------|-------------|
| `calibration/mod.rs` | 56-57 | モジュール | Legacy | `model_calibrator` - `engine`に置換済み |
| `calibration/engine.rs` | 294 | 実装ブロック | Legacy | 重複`GenericCalibrator`実装 |
| `calibration/model_calibrator.rs` | 235 | 実装ブロック | Legacy | 非推奨、後方互換性のため保持 |
| `indexed_market.rs` | 94 | フィールド | 未使用 | `fallback_curve_set` |
| `indexed_market.rs` | 99 | フィールド | 未使用 | `index_mapper` |
| `fx_calibration/surface.rs` | 485-486 | フィールド | 未使用 | `fx_curve` |

**評価**: Dead codeは限定的（6箇所）。`model_calibrator`モジュールは意図的に保持されているが、次期メジャーバージョンで削除可能。

### 1.3 エラー型の分布

市場モジュール内に**16種類のエラー型**が存在:

| モジュール | エラー型 | 用途 |
|-----------|---------|------|
| `market/error.rs` | `MarketDataError`, `MarketBuildError` | トップレベル |
| `calibration/error.rs` | `CalibrationError` | モデルキャリブレーション |
| `calibration/bootstrapping/error.rs` | `BootstrapError` | カーブ構築 |
| `calibration/bootstrapping/engine_error.rs` | `CurveEngineError` | エンジン固有 |
| `volcube/error.rs` | `VolCubeError` | ボラティリティキューブ |
| `volcube/engine.rs` | `ForwardRateError` | フォワードレート |
| `volcube/interpolator.rs` | `InterpolationError` | 補間 |
| `volcube/calibration_graph.rs` | `GraphError` | キャリブレーショングラフ |
| `volcube/loader_convert.rs` | `ConversionError` | データ変換 |
| `volcube/vega.rs` | `VegaError` | Vega計算 |
| `fx_calibration/error.rs` | `FxCalibrationError` | FXキャリブレーション |
| `fx_calibration/curve.rs` | `FxCurveError` | FXカーブ |
| `fx_calibration/surface.rs` | `VolSurfaceError` | FXボラティリティ |
| `fx_calibration/vol_builder.rs` | `CalibrationError` | **名前衝突** |
| `fx_calibration/fx_market_builder.rs` | `FxMarketError` | FX市場構築 |

**問題点**:
- `fx_calibration/vol_builder.rs`に`CalibrationError`が定義され、`calibration/error.rs`と名前が衝突
- エラー型が過度に細分化されている
- 一部のエラー型は統合可能

### 1.4 重複/オーバーラップ機能

| 領域 | ファイル1 | ファイル2 | 状態 |
|-----|----------|----------|------|
| キャリブレーションエンジン | `calibration/engine.rs` | `calibration/model_calibrator.rs` | 旧実装を新実装が置換（後方互換性のため両方存在） |
| FXボラティリティサーフェス | `surfaces/fx.rs` | `fx_calibration/surface.rs` | 設計上の分離（シンプル版 vs キャリブレーション版） |
| キャッシュ実装 | `bootstrapping/cache.rs` | `bootstrapping/result_cache.rs` | 異なる関心事（プロセス vs 結果） |

### 1.5 外部依存関係

`pricer_models::market`は以下から広く参照されている:
- `pricer_models::compiler` - TradeCompilerでの市場データアクセス
- `pricer_models::models` - モデルキャリブレーション
- `pricer_models::analytical` - 分析的ソリューション
- 統合テスト（`tests/volcube_integration.rs`, `tests/curve_bootstrap_integration.rs`）

---

## 2. 要件実現可能性分析

### 要件1: ファイル監査と不要ファイルの特定

| 技術的ニーズ | 現状 | ギャップ |
|-------------|------|---------|
| 未使用ファイル検出 | `#[allow(dead_code)]`で6箇所特定済み | 網羅的な参照分析が必要 |
| 重複機能の統合 | `model_calibrator.rs`が明確な統合候補 | 後方互換性維持の戦略が必要 |
| ドキュメント化 | 部分的 | 削除決定の根拠文書化が必要 |

**複雑度**: S（1-3日） - 明確な削除候補が特定済み

### 要件2: calibration/サブモジュールの整理

| 技術的ニーズ | 現状 | ギャップ |
|-------------|------|---------|
| bootstrapping整理 | 17ファイル、階層フラット | 論理グループ化の設計が必要 |
| エラー型統合 | `error.rs`と`engine_error.rs`が分離 | 統合設計が必要 |
| テストファイル配置 | テストは`#[cfg(test)]`ブロック内 | 整理不要（現状適切） |

**複雑度**: M（3-7日） - ファイル数が多く、依存関係の整理が必要

### 要件3: volcube/サブモジュールの整理

| 技術的ニーズ | 現状 | ギャップ |
|-------------|------|---------|
| サブモジュール化 | 21ファイルがフラット配置 | グループ化設計が必要 |
| テストコード配置 | `proptest_tests.rs`がモジュール内 | テスト専用サブディレクトリ検討 |
| `loader_convert.rs`配置 | volcube内 | ユーティリティとしての再配置検討 |

**複雑度**: M（3-7日） - 最大のサブモジュール、慎重な設計が必要

### 要件4: surfaces/とcurves/の構造統一

| 技術的ニーズ | 現状 | ギャップ |
|-------------|------|---------|
| 命名規則統一 | `traits.rs`, `flat.rs`は一致、`*_enum.rs`は不一致 | 軽微なリネーム |
| コード構造統一 | 類似パターン | 詳細比較が必要 |

**複雑度**: S（1-3日） - 両モジュールとも小規模（6ファイル各）

### 要件5: ルートレベルファイルの整理

| 技術的ニーズ | 現状 | ギャップ |
|-------------|------|---------|
| `fx_density.rs`配置 | market/直下 | `fx_calibration/`への移動検討 |
| `index_mapper.rs`と`indexed_market.rs` | 分離 | 関係性の明確化が必要 |

**複雑度**: S（1-3日） - ファイル数が少ない

### 要件6: 後方互換性の維持

| 技術的ニーズ | 現状 | ギャップ |
|-------------|------|---------|
| 公開API維持 | 深いre-exportパターン採用済み | 移行パス設計が必要 |
| テスト継続性 | 統合テスト存在 | 変更後の検証計画が必要 |

**複雑度**: M（3-7日） - 全変更にわたる横断的関心事

### 要件7: ドキュメント同期

| 技術的ニーズ | 現状 | ギャップ |
|-------------|------|---------|
| structure.md更新 | 現状を反映 | 変更後の更新が必要 |
| mod.rsドキュメント | 一部存在 | 全モジュールでの一貫性確保が必要 |

**複雑度**: S（1-3日） - 実装後のドキュメント作業

---

## 3. 実装アプローチオプション

### Option A: 段階的クリーンアップ（保守的）

**アプローチ**:
1. 明確なdead code（`model_calibrator.rs`）のみ削除
2. エラー型は現状維持
3. ファイル移動は最小限

**対象ファイル変更**:
- 削除: `calibration/model_calibrator.rs`
- 編集: `calibration/mod.rs`（dead_code allowの削除）
- 編集: `indexed_market.rs`（未使用フィールドの削除または活用）

**トレードオフ**:
- ✅ 最小リスク、即時実行可能
- ✅ 後方互換性維持が容易
- ❌ 構造的改善が限定的
- ❌ 82ファイル構造は維持

**工数**: S（1-3日）
**リスク**: Low

### Option B: モジュール再構成（中間）

**アプローチ**:
1. volcube/内にサブモジュール作成（calibration/, cache/, interpolation/）
2. bootstrapping/内のエラー型統合
3. curves/とsurfaces/の命名統一
4. 後方互換性re-exportの追加

**対象ファイル変更**:
- 新規: `volcube/calibration/mod.rs`, `volcube/cache/mod.rs`等
- 移動: 関連ファイルをサブモジュールへ
- 統合: `bootstrapping/error.rs`と`bootstrapping/engine_error.rs`
- リネーム: `curve_enum.rs` → `enum.rs`等

**トレードオフ**:
- ✅ 明確な構造改善
- ✅ 保守性向上
- ❌ 移行パス設計が必要
- ❌ 外部コードへの影響調査が必要

**工数**: M（3-7日）
**リスク**: Medium

### Option C: フル再設計（積極的）

**アプローチ**:
1. エラー型の統合階層設計（MarketError → サブエラー）
2. 全サブモジュールの論理的再構成
3. 内部実装の公開範囲見直し（pub(crate)活用）
4. feature flagによるレガシーAPI分離

**対象ファイル変更**:
- 大規模な構造変更
- エラー型の全面的な再設計
- 公開API範囲の変更

**トレードオフ**:
- ✅ 最も理想的な構造を実現
- ✅ 長期的な保守性が最も高い
- ❌ 高リスク、工数大
- ❌ 破壊的変更が多い
- ❌ 外部依存コードへの影響が大きい

**工数**: L（1-2週間）
**リスク**: High

---

## 4. 推奨事項

### 推奨アプローチ: Option B（モジュール再構成）

**根拠**:
1. 現状の構造的問題（フラットな21ファイルのvolcube等）に対処
2. 後方互換性を維持しながら段階的に改善可能
3. リスクと効果のバランスが最も良い

### 設計フェーズで調査すべき項目

1. **volcubeサブモジュール設計**
   - calibration関連: `calibrator.rs`, `engine.rs`, `calibration_graph.rs`
   - cache関連: `cache.rs`, `lazy_evaluator.rs`
   - interpolation関連: `interpolator.rs`, `sabr_surface.rs`
   - 最適なグループ化の決定

2. **エラー型統合戦略**
   - `BootstrapError`と`CurveEngineError`の統合可否
   - `fx_calibration`内のエラー型整理
   - 名前衝突している`CalibrationError`の解決

3. **後方互換性re-exportパターン**
   - 旧パスからの再エクスポート戦略
   - `#[deprecated]`属性の活用
   - 移行期間の設定

4. **テストコード配置**
   - `proptest_tests.rs`の最適配置
   - `aad_validation.rs`の位置づけ（テスト vs 本番コード）

---

## 5. 実装複雑度・リスクサマリー

| 要件 | 工数 | リスク | 理由 |
|-----|------|-------|------|
| R1: ファイル監査 | S | Low | 明確な削除候補あり |
| R2: calibration整理 | M | Medium | ファイル数が多い、依存関係複雑 |
| R3: volcube整理 | M | Medium | 最大サブモジュール、慎重な設計必要 |
| R4: curves/surfaces統一 | S | Low | 小規模モジュール |
| R5: ルートファイル整理 | S | Low | ファイル数少ない |
| R6: 後方互換性 | M | Medium | 横断的関心事 |
| R7: ドキュメント同期 | S | Low | 実装後の作業 |

**総合工数**: M-L（1-2週間）
**総合リスク**: Medium

---

## 次のステップ

1. 本ギャップ分析をレビュー
2. 実装アプローチ（A/B/C）を選択
3. `/kiro:spec-design market-module-refactor`で詳細設計を生成
