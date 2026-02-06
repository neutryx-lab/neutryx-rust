# Requirements Document

## Introduction

本仕様は、金利ボラティリティキューブ（IR Vol Cube）のカリブレーションエンジンを実装する。USD/EUR/JPY向けEuropean SwaptionおよびCapFloor商品の定義から、SABRモデル等を用いたVolCubeのカリブレーション、LazyValuation・AAD対応、WebApp統合までを包括する。既存のCurveBuilder実装をリファレンスとし、商品リストとVolCube設定を入力として、カリブレーション済み補間器を出力する統合的なフレームワークを構築する。

## Requirements

### Requirement 1: IR商品定義（Swaption・CapFloor）

**Objective:** As a クオンツ開発者, I want USD/EUR/JPYのEuropean SwaptionおよびCapFloorを正確にモデル化するinstrument定義, so that VolCubeカリブレーションの入力として使用できる。

#### Acceptance Criteria

1. The `infra_domain::trade::instrument_def::rates` shall EuropeanSwaptionをexpiry、tenor、strike、payer/receiver、exercise_styleフィールドで定義する。
2. The `infra_domain::trade::instrument_def::rates` shall CapFloorをcap_strike、floor_strike、underlying_tenor、payment_frequencyフィールドで定義する。
3. The `infra_domain::trade::convention::swaption` shall USD（SOFR）、EUR（ESTR）、JPY（TONA）のswaption conventionを定義する。
4. The `infra_domain::trade::convention::capfloor` shall USD/EUR/JPYのcap/floor conventionを定義する。
5. When SwaptionまたはCapFloorが構築される, the instrument shall underlying swap/cap scheduleを自動生成する。
6. If 無効なstrike/expiry/tenor組み合わせが指定された, then the system shall `InstrumentError`を返す。
7. The instrument definitions shall `serde`によるJSON/TOML serialisation/deserialisationをサポートする。

### Requirement 2: VolCubeデータ構造

**Objective:** As a マーケットデータエンジニア, I want expiry×tenor×strikeの3次元ボラティリティデータを格納する構造, so that 効率的な補間とカリブレーションが可能になる。

#### Acceptance Criteria

1. The `pricer_models::market::volcube::VolCube` shall expiry軸、tenor軸、strike軸の3次元グリッドを保持する。
2. The VolCube shall 各グリッド点に対してmarket quote（bid/ask/mid）を格納できる。
3. The VolCube shall 任意の(expiry, tenor, strike)座標でのボラティリティ取得をサポートする。
4. While VolCubeがカリブレーション前の状態, the system shall raw market quotesのみを返す。
5. While VolCubeがカリブレーション済み, the system shall 補間されたボラティリティを返す。
6. The VolCube shall Currency（USD/EUR/JPY）およびunderlying index（SOFR/ESTR/TONA）をメタデータとして保持する。

### Requirement 3: VolCube補間器フレームワーク

**Objective:** As a クオンツ開発者, I want SABRモデルを含む補間器をプラガブルに構成できるフレームワーク, so that 異なる補間手法を柔軟に適用できる。

#### Acceptance Criteria

1. The `VolCubeInterpolator` trait shall `interpolate(expiry, tenor, strike) -> Result<f64, InterpolationError>`メソッドを定義する。
2. The system shall `SabrInterpolator`を`VolCubeInterpolator`トレイトを実装する形で提供する。
3. The system shall `FlatInterpolator`、`LinearInterpolator`を基本補間器として提供する。
4. The `VolCubeConfig` shall どのパラメータ（α、β、ρ、ν）をどの軸で補間するか設定可能とする。
5. When `VolCubeBuilder`にinterpolator設定が渡される, the system shall 対応する補間器を構築する。
6. The interpolator framework shall static dispatch（enum-based）で実装しEnzyme互換性を維持する。

### Requirement 4: SABRモデルカリブレーション

**Objective:** As a クオンツ開発者, I want 各(expiry, tenor)スライスに対してSABRパラメータをカリブレーションする機能, so that market smileを正確に再現できる。

#### Acceptance Criteria

1. The `SabrCalibrator` shall market quotes（ATM vol + smile quotes）からSABRパラメータ（α、β、ρ、ν）をカリブレーションする。
2. The calibrator shall β固定モード（β=0, 0.5, 1.0等）をサポートする。
3. The calibrator shall Levenberg-Marquardt最適化を使用する（`pricer_core::math::optimisers`）。
4. When カリブレーションが収束しない, the system shall `CalibrationError`を詳細な診断情報と共に返す。
5. The calibrator shall 各パラメータの境界制約（ρ ∈ [-1,1]、ν > 0等）を適用する。
6. If calibrated smileがarbitrage-freeでない, then the system shall 警告を発行する（Breeden-Litzenberger検証）。
7. The calibration results shall 残差、反復回数、最終パラメータ値を含む診断データを提供する。

### Requirement 5: カリブレーションエンジン（CurveBuilder参照）

**Objective:** As a クオンツ開発者, I want CurveBuilderと同様のパターンでVolCubeをカリブレーションするエンジン, so that 統一的なカリブレーションワークフローを実現できる。

#### Acceptance Criteria

1. The `VolCubeCalibrationEngine` shall instrument listとVolCube設定を入力として受け取る。
2. When エンジンが`calibrate()`を呼び出される, the system shall 全(expiry, tenor)スライスに対してカリブレーションを実行する。
3. The engine shall YieldCurveへの依存を持ち、forward rate計算に使用する。
4. The engine shall カリブレーション順序（expiry-first vs tenor-first）を設定可能とする。
5. While カリブレーション実行中, the engine shall 進捗状況を報告する（callback/channel経由）。
6. The engine shall カリブレーション結果として`CalibratedVolCube`を返す。
7. The `CalibratedVolCube` shall `VolatilitySurface` traitを実装し、pricing contextで使用可能とする。
8. The `VolCubeConfig` shall discount_curve_nameおよびprojection_curve_nameをCurveName型で指定する。
9. The engine shall CurveSetから指定されたCurveを解決し、forward swap rate計算に使用する。
10. When USD通貨が指定された, the engine shall SOFR curveをデフォルトとして使用する（EUR=ESTR、JPY=TONA）。

### Requirement 6: LazyValuation・キャッシュ最適化（統合依存グラフ）

**Objective:** As a パフォーマンスエンジニア, I want 遅延評価とキャッシュにより繰り返し計算を最小化する仕組み, so that 大量のpricing/risk計算が高速に実行できる。

#### Acceptance Criteria

1. The VolCube shall lazy initialization patternを採用し、必要時のみカリブレーションを実行する。
2. The system shall カリブレーション結果をexpiry-tenorスライス単位でキャッシュする。
3. When 同一(expiry, tenor, strike)への複数回アクセスがある, the system shall キャッシュから値を返す。
4. The cache shall thread-safe（`DashMap`または`RwLock<HashMap>`）で実装する。
5. When 入力market quotesが更新される, the system shall 影響を受けるキャッシュエントリを無効化する。
6. The system shall キャッシュヒット率、メモリ使用量のメトリクスを提供する。
7. The lazy evaluation framework shall `pricer_pricing::irs_greeks::lazy_evaluator`と同様のパターンに従う。
8. The lazy evaluation shall CurveBuilder calibrationまで遡って依存解決する。
9. The `MarketProvider` shall Curve→VolCubeの依存グラフを保持し、トポロジカル順でカリブレーションする。
10. When VolCubeが要求された, the system shall 依存するCurveが未カリブレーションなら自動的にカリブレーションを実行する。
11. When AADモードで実行される, the computation graph shall CurveQuote→CalibratedCurve→VolCube→Priceの完全なパスを保持する。
12. The `CalibrationGraph` shall ノード（Curve/VolCube）とエッジ（依存関係）を明示的に管理する。

### Requirement 7: AAD（Adjoint Algorithmic Differentiation）統合

**Objective:** As a リスクエンジニア, I want パラメータだけでなくInstrumentまでの計算グラフを持つAAD対応, so that 全market inputに対する感応度を効率的に計算できる。

#### Acceptance Criteria

1. The VolCube calibration shall `pricer_core::types::dual::DualNumber`互換のジェネリック型`T: Float`で実装する。
2. When Enzyme AADモードで実行される, the system shall カリブレーションパラメータへの感応度を計算する。
3. The computation graph shall market quote入力からcalibrated vol出力までの完全な依存関係を保持する。
4. The system shall `GraphExtractable` traitを実装し、D3.js互換のDAGをエクスポートする。
5. When AADが有効, the system shall forward/adjoint modeの両方をサポートする。
6. The AAD framework shall bump-and-revalueとのクロス検証機能を提供する。
7. If 微分が不連続点を通過する, then the system shall smooth approximationを適用する。
8. The AAD graph shall CurveQuote→CurveCalibration→ForwardRate→VolCubeCalibration→SwaptionPriceの完全なパスをサポートする。
9. When ∂SwaptionPrice/∂CurveQuoteが要求された, the system shall Curve経由の間接的感応度を計算する。
10. The system shall Vega（∂Price/∂VolQuote）とCurve Sensitivity（∂Price/∂CurveQuote）の両方を単一のAADパスで計算する。

### Requirement 8: デモWebApp統合

**Objective:** As a デモユーザー, I want WebAppからVolCubeカリブレーションを実行し結果を可視化する機能, so that カリブレーション品質を視覚的に確認できる。

#### Acceptance Criteria

1. The `demo/gui/src/web/volcube_handlers.rs` shall `/api/volcube/calibrate` POSTエンドポイントを提供する。
2. The WebApp shall USD/EUR/JPYのcurrency選択UIを提供する。
3. The WebApp shall expiry×tenor×strike smileを3Dサーフェスとして可視化する。
4. When カリブレーションが完了した, the WebApp shall SABRパラメータグリッドを表示する。
5. The WebApp shall market quote vs fitted vol の比較チャートを表示する。
6. The WebApp shall Breeden-Litzenberger密度関数を可視化する。
7. If カリブレーションエラーが発生した, then the WebApp shall エラー詳細と診断情報を表示する。
8. The WebApp shall 既存の`curve-builder-webapp`のUIパターンに従う。

### Requirement 9: 既存実装の活用と不要コードの削除

**Objective:** As a コードベース管理者, I want 既存実装を最大限活用しつつ不要になったコードを削除する, so that コードベースの整合性と保守性を維持できる。

#### Acceptance Criteria

1. The implementation shall 既存の`pricer_models::market::volcube`モジュールを拡張する。
2. The implementation shall 既存の`SabrCalibrator`（`pricer_models::market::calibration::sabr`）を再利用する。
3. The implementation shall `VolCubeBuilder`パターンを既存設計から継承する。
4. When 新実装により不要になったコードがある, the system shall 該当コードを完全に削除する。
5. The implementation shall deprecated APIに対して`#[deprecated]`属性を付与しない（即座に削除する）。
6. When 削除対象コードが特定された, the system shall 影響範囲分析を実施し依存コードを更新する。
7. The final implementation shall 未使用のimport、dead codeを含まない。

### Requirement 10: 入力データローダー

**Objective:** As a マーケットデータエンジニア, I want JSON/CSVファイルからVolCube入力データをロードする機能, so that 実際のmarket dataでカリブレーションを実行できる。

#### Acceptance Criteria

1. The `adapter_loader` shall swaption vol quote JSON/CSVファイルをパースする。
2. The loader shall capfloor vol quote JSON/CSVファイルをパースする。
3. The data format shall expiry、tenor、strike、vol（bid/ask/mid）のカラムを含む。
4. When 入力ファイルが見つからない, the system shall `LoaderError`を返す。
5. If データフォーマットが無効, then the system shall 行番号付きのパースエラーを返す。
6. The loader shall `demo/data/input/volsurface/`ディレクトリのファイル規約に従う。
7. The loaded data shall `VolCubeBuilder`に直接渡せる型に変換される。
