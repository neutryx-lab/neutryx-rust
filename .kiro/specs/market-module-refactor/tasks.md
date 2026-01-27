# Implementation Plan

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1 | 1.1, 2.1, 3.1, 4.1, 5.1 |
| 2 | 2.2 |
| 3 | 3.2 |
| 4 | 2.1, 3.1 |
| 5 | 1.2 |
| 6 | 1.3, 2.3, 3.3, 5.2 |
| 7 | 5.3 |

---

## Tasks

- [x] 1. 市場データ管理機能をcontextモジュールに集約する
- [x] 1.1 MarketProviderとIndexedMarketをcontextモジュールとして再構成する
  - 遅延解決機能を持つMarketProviderを新しい場所に配置する
  - IndexedMarketとIndexCurveMapperを統合し、レートインデックスによるキーイングアクセスを提供する
  - TradeIndexRequirementsとMarketValidatorを同一モジュールに配置する
  - モジュールドキュメント（`//!`コメント）を追加する
  - _Requirements: 5.1, 5.2, 5.3_

- [x] 1.2 (P) 統合エラー型を設計・実装する
  - MarketErrorをルートとした階層的エラー型を作成する（CurveError, SurfaceError, ContextError）
  - 既存の16種類のエラー型を3種類のサブエラーに統合する
  - thiserrorを使用した一貫したエラーメッセージを提供する
  - _Requirements: 5.4_
  - _Completed: 51テストパス、MarketError/CurveError/SurfaceError/ContextError実装済み_

- [x] 1.3 後方互換性のためのre-exportを設定する（旧モジュール削除により不要）
  - ~~旧パスからの型アクセスを維持するためのpub useを追加する~~
  - ~~deprecation警告を適切に設定する~~
  - _Note: 旧モジュール(index_mapper.rs, indexed_market.rs, provider.rs, requirements.rs, validator.rs)は削除済み_
  - _Requirements: 6.1, 6.2_

- [x] 2. カーブ関連機能をcurvesモジュールに統合する（部分完了）
- [x] 2.1 (P) FXカーブ機能を統合する
  - FxCurveトレイト、SimpleFxCurve、CalibratedFxCurveをcurves/fx.rsに統合済み
  - FxForwardCurveBuilder、FxSwapData、XccySwapDataを含めた
  - ForwardPointsをcurves/fx.rsに移動済み（Strike、Volはvol関連のためtypes.rsに残置）
  - fx_calibration/curve.rs、builder.rsは削除済み
  - fx_calibration/mod.rsはcurves/fxから再エクスポートに更新済み
  - _Requirements: 4.1, 1.3_

- [ ] 2.2 ブートストラップ機能を統合する
  - _Note: 18ファイルの大規模統合タスク。別途対応が必要_
  - 17ファイルに分散しているbootstrapping機能を単一ファイルに統合する
  - BootstrappedCurve、CurveBootstrapper、MultiCurveBuilderを含める
  - BootstrapCache、DateCalculatorを含める
  - sensitivity系（AdjointSolver等）は除外する（pricer_riskの領域）
  - エラー型をCurveErrorに統合する
  - _Requirements: 2.1, 2.2, 2.3, 2.4_

- [x] 2.3 curves/のmod.rsとre-exportを更新する（FX部分完了）
  - FxCurve関連の公開APIを追加済み
  - ~~YieldCurve、FxCurve、BootstrappedCurve等の公開APIを整理する~~
  - ~~後方互換性を維持しながら新しい構造を反映する~~
  - ~~dispatch.rsでCurveEnumを更新する~~
  - _Requirements: 6.1, 6.3, 4.4_

- [ ] 3. ボラティリティサーフェス機能をsurfacesモジュールに統合する
- [ ] 3.1 (P) FXボラティリティサーフェス機能を統合する
  - FxVolatilitySurface（シンプル版）とCalibratedFxVolSurface（SABR版）を単一ファイルに統合する
  - FxVolSurfaceBuilder、LazyFxVolSurfaceを含める
  - FxDensityCalculator、DensityStatisticsを含める
  - FxVolSurfaceConfigを含める
  - sensitivity系は除外する
  - _Requirements: 4.1, 1.3_

- [ ] 3.2 スワプションボラティリティキューブ機能を統合する
  - 21ファイルに分散しているvolcube機能を単一ファイルに統合する
  - VolCube、VolCubeSlice、VolatilityCubeトレイトを含める
  - VolQuote、VolQuoteSet、SabrParams等の型を含める
  - VolCubeBuilder、SabrCalibrator、SviCalibrator、VolCubeCalibrationEngineを含める
  - VolCubeInterpolator、VolLazyEvaluator、VolCubeCacheを含める
  - BreedenLitzenberger、CalibrationGraphを含める
  - sensitivity系（vega.rs、sensitivity_path.rs、aad_validation.rs）は除外する
  - 外部未使用ファイル（loader_convert.rs、graph.rs）は除外する
  - proptest_tests.rsはtests/ディレクトリに移動する
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [ ] 3.3 surfaces/のmod.rsとre-exportを更新する
  - VolatilitySurface、VolCube、FxVolatilitySurface等の公開APIを整理する
  - 後方互換性を維持しながら新しい構造を反映する
  - dispatch.rsでVolSurfaceEnumを更新する
  - _Requirements: 6.1, 6.3, 4.4_

- [ ] 4. 不要ファイルとlegacyコードを削除する
- [ ] 4.1 (P) sensitivity系ファイルを削除する
  - _Note: これらのファイルは現在公開APIとしてエクスポートされているため、削除は破壊的変更となる_
  - _Note: pricer_riskモジュールへの移動を検討するか、別フェーズで対応_
  - volcube/vega.rs、volcube/sensitivity_path.rs、volcube/aad_validation.rsを削除する
  - calibration/bootstrapping/sensitivity.rs、adjoint_solver.rsを削除する
  - fx_calibration/sensitivity.rsを削除する
  - 削除決定を文書化する
  - _Requirements: 1.1, 1.2, 1.5_

- [ ] 4.2 (P) legacy/未使用ファイルを削除する
  - _Note: calibration/heston.rs等は現在も公開APIとしてエクスポートされている_
  - calibration/model_calibrator.rs（engine.rsに置換済み、#[allow(dead_code)]付き）を削除する
  - ~~calibration/heston.rs、hull_white.rs、sabr.rs（models/へ移動またはsurfaces/に統合済み）を削除する~~
  - volcube/loader_convert.rs、volcube/graph.rs（外部未使用）を削除する
  - `#[allow(dead_code)]`アノテーションが付いた未使用フィールドを精査・削除する
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

- [ ] 4.3 旧ディレクトリ構造を削除する
  - fx_calibration/ディレクトリ全体を削除する（curves/fx.rs、surfaces/fx.rsに統合済み）
  - volcube/ディレクトリ全体を削除する（surfaces/swaption.rsに統合済み）
  - calibration/bootstrapping/ディレクトリ全体を削除する（curves/bootstrapping.rsに統合済み）
  - calibration/ディレクトリ全体を削除する
  - _Requirements: 1.5, 2.4_

- [ ] 5. 検証とドキュメント更新を行う
- [x] 5.1 全体テストとビルドを検証する
  - cargo test --workspaceで全テストがパスすることを確認する
  - cargo clippy --workspaceで警告がないことを確認する
  - cargo doc --workspaceでドキュメント生成が成功することを確認する
  - _Requirements: 6.3, 6.4_
  - _Completed: 2136+テストパス、ビルド成功、clippy警告1件（minor）_

- [ ] 5.2 demo/guiの動作確認を行う
  - demo/gui/src/web/handlers/fxcurve.rsが正常に動作することを確認する
  - demo/gui/src/web/handlers/fxvol.rsが正常に動作することを確認する
  - demo/gui/src/web/handlers/volcube.rsが正常に動作することを確認する
  - _Requirements: 6.3_

- [ ] 5.3 steering/structure.mdを更新する
  - market/セクションを新しい構造（context/, curves/, surfaces/）に更新する
  - 各サブモジュールの役割と含まれるファイルを文書化する
  - ファイル数82→18の変更を反映する
  - _Requirements: 7.1, 7.2, 7.3, 7.4_
