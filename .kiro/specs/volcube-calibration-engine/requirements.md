# Requirements Document

## Introduction

本仕様は、VolatilityCube（3次元ボラティリティ構造）のカリブレーションエンジンを定義する。CurveBuilderパターンに倣い、Instrumentリストとカリブレーション設定を入力として、任意のパラメータに基づくVolCubeを構築・キャッシュ管理する。VolCubeは`get_vol`による任意パラメータでのボラティリティ取得、確率密度関数の計算、計算グラフへの接続をサポートし、前提条件が変わらない限り再カリブレーションを回避する。

## Project Description (Input)

VolatilityCubeカリブレーションの精緻な実装。CurveBuilderのように、Instrumentのリストとカリブレーション設定(Interporatorなど)を元に、任意のParameterをベースとするVolcubeをカリブレーションする。VolCubeはGetVOlなどで任意の設定のVolを返すのが主だが、任意のtで確率密度関数を返したりもする。もちろん計算グラフはもとのInstrumentと接続しているが、前提条件が変わらない場合再カリブレーションしない。

## Requirements

### Requirement 1: VolCubeBuilder コア構築

**Objective:** As a クオンツ開発者, I want Instrumentリストとカリブレーション設定からVolCubeを構築したい, so that 市場データから一貫した3次元ボラティリティ構造を生成できる

#### Acceptance Criteria
1. When Instrumentリストとカリブレーション設定を渡した場合, the VolCubeBuilder shall 設定に基づいてカリブレーションを実行しVolCubeインスタンスを返す
2. When カリブレーション設定にInterpolatorタイプを指定した場合, the VolCubeBuilder shall 指定されたInterpolator（Linear, CubicSpline, SABR, SVI等）を使用してVolCubeを構築する
3. The VolCubeBuilder shall Builder patternを採用し、fluent APIによる設定を提供する
4. If Instrumentリストが空の場合, then the VolCubeBuilder shall `CalibrationError::InsufficientData`を返す
5. The VolCubeBuilder shall 複数の次元軸（Expiry, Tenor, Strike/Moneyness）を持つVolCubeを構築できる

### Requirement 2: VolCube インターフェース

**Objective:** As a プライシングエンジン, I want 任意のパラメータでボラティリティを取得したい, so that 任意の商品に対して適切なボラティリティを使用できる

#### Acceptance Criteria
1. When `get_vol(expiry, tenor, strike)`を呼び出した場合, the VolCube shall 補間されたボラティリティ値を返す
2. When クエリパラメータがグリッド外の場合, the VolCube shall 設定に応じてExtrapolationまたはエラーを返す
3. The VolCube shall AD互換のジェネリック型`T: Float`をサポートする
4. While VolCubeが有効な状態の場合, the VolCube shall スレッドセーフな読み取りアクセスを提供する（`Send + Sync`）
5. The VolCube shall 有効なドメイン範囲（expiry_domain, tenor_domain, strike_domain）を返すメソッドを提供する

### Requirement 3: 確率密度関数

**Objective:** As a リスクアナリスト, I want 任意の時点での確率密度関数を取得したい, so that リスクニュートラル密度分析を実行できる

#### Acceptance Criteria
1. When `probability_density(expiry, strike)`を呼び出した場合, the VolCube shall Breeden-Litzenberger公式に基づく確率密度を返す
2. When `cumulative_probability(expiry, strike)`を呼び出した場合, the VolCube shall 累積確率分布を返す
3. The VolCube shall 密度関数の数値安定性のためにsmooth approximationを使用する
4. If expiryが有効範囲外の場合, then the VolCube shall `MarketDataError::OutOfBounds`を返す

### Requirement 4: 計算グラフ接続

**Objective:** As a システム, I want VolCubeがソースInstrumentと計算グラフで接続されていることを保証したい, so that 依存関係の追跡とAADによる感度計算が可能になる

#### Acceptance Criteria
1. The VolCube shall ソースInstrumentへの参照（または識別子）を保持する
2. When VolCubeをPricingContextに登録した場合, the PricingContext shall VolCubeとソースInstrument間の依存関係をDAGに記録する
3. The VolCube shall `GraphExtractable`トレイトを実装し、D3.js互換のグラフ出力をサポートする
4. While AADモードが有効な場合, the VolCube shall ソースInstrumentへの感度（Vega, Volga, Vanna）を計算可能にする

### Requirement 5: キャッシュと再カリブレーション回避

**Objective:** As a パフォーマンス最適化, I want 前提条件が変わらない場合に再カリブレーションを回避したい, so that 計算コストを削減できる

#### Acceptance Criteria
1. The VolCubeBuilder shall 入力Instrumentリストと設定のハッシュに基づくキャッシュキーを生成する
2. When 同一のキャッシュキーでVolCubeを要求した場合, the VolCubeBuilder shall キャッシュされたVolCubeを返す（再カリブレーションなし）
3. When ソースInstrumentの市場データが更新された場合, the VolCubeBuilder shall キャッシュを無効化し再カリブレーションを実行する
4. The VolCubeBuilder shall LRUキャッシュ戦略を採用し、メモリ使用量を制限する
5. While キャッシュが有効な場合, the VolCubeBuilder shall キャッシュヒット率とメモリ使用量をメトリクスとして公開する

### Requirement 6: カリブレーション設定

**Objective:** As a クオンツ開発者, I want 詳細なカリブレーション設定を指定したい, so that 様々なモデルと補間方法に対応できる

#### Acceptance Criteria
1. The CalibrationConfig shall Interpolation方式（Linear, CubicSpline, SABR, SVI, FlatVol）を指定できる
2. The CalibrationConfig shall Extrapolation方式（Flat, Linear, Error）を指定できる
3. The CalibrationConfig shall Strike軸の表現方式（Absolute, Moneyness, LogMoneyness, Delta）を指定できる
4. The CalibrationConfig shall 最適化アルゴリズム（LevenbergMarquardt, NelderMead）を指定できる
5. The CalibrationConfig shall Builderパターンで設定を構築し、`Default`トレイトを実装する

### Requirement 7: エラーハンドリング

**Objective:** As a システム, I want 包括的なエラー情報を取得したい, so that 問題の診断と解決が容易になる

#### Acceptance Criteria
1. If カリブレーションが収束しない場合, then the VolCubeBuilder shall `CalibrationError::NotConverged`と反復回数、残差を返す
2. If Instrumentデータに不整合がある場合, then the VolCubeBuilder shall `CalibrationError::InvalidInput`と詳細を返す
3. If アービトラージ条件違反を検出した場合, then the VolCube shall `CalibrationError::ArbitrageFreeViolation`を返す
4. The CalibrationError shall `thiserror`を使用した構造化エラーを提供する
5. When エラーが発生した場合, the VolCubeBuilder shall カリブレーション診断情報（residuals, iterations, parameter_values）を含める

### Requirement 8: A-I-P-S アーキテクチャ準拠

**Objective:** As a アーキテクト, I want VolCubeエンジンがA-I-P-Sアーキテクチャに準拠することを保証したい, so that コードベースの一貫性が維持される

#### Acceptance Criteria
1. The VolCubeBuilder shall `pricer_models::market::calibration`モジュールに配置する
2. The VolCube shall `pricer_models::market::surfaces`モジュールに配置する
3. The VolCube shall 既存の`VolatilitySurface<T>`トレイトを拡張または実装する
4. The VolCubeBuilder shall `pricer_core`の数学ユーティリティ（interpolators, solvers, optimisers）を使用する
5. The VolCubeBuilder shall `infra_master`の市場データ型（Currency, RateIndex）を参照する

### Requirement 9: テストと検証

**Objective:** As a 品質保証, I want 包括的なテストカバレッジを確保したい, so that 実装の正確性が保証される

#### Acceptance Criteria
1. The VolCube shall 単体テストでArbitrage-free条件を検証する（Butterfly spread, Calendar spread）
2. The VolCube shall プロパティベーステスト（proptest）で数学的不変条件を検証する
3. The VolCubeBuilder shall 既知のSABR/SVIパラメータに対するキャリブレーション精度テストを含む
4. The VolCube shall num-dual検証モードでAAD正確性を検証する
5. The VolCubeBuilder shall ベンチマーク（criterion）でカリブレーション性能を計測する

### Requirement 10: 拡張性

**Objective:** As a 将来の開発者, I want 新しいモデルや補間方法を追加しやすくしたい, so that システムの拡張性が確保される

#### Acceptance Criteria
1. The CalibrationConfig shall enum-based static dispatchでモデル選択をサポートする
2. The InterpolationMethod shall 新しい補間方法を追加可能なenum構造を持つ
3. The VolCube shall feature flagで追加モデル（LocalVol, StochasticLocalVol）を有効化できる
4. The VolCubeBuilder shall トレイトベースの抽象化で新しいカリブレータを受け入れる
