# Requirements Document

## Project Description (Input)
ドメインモデルの分離と「コンパイル」プロセスの導入
infra_master における市場データの定義（レシピ）と、pricer_models における計算用オブジェクト（実行体）の境界が、柔軟性に対してやや冗長、または計算効率を損なう構造になっています。

課題：MarketInstrument と CalibrationInstrument の重複 infra_master::market::MarketInstrument は、MarketConvention と Rate を結合してキャッシュフローを展開する能力を持ちますが、これをそのまま CalibrationProblem のループ内で評価するのは非効率です。

洗練化案：Immutable Cashflow Set への事前コンパイル CurveDefinition から CalibrationProblem を生成する際に、全ての MarketInstrument を静的な Trade オブジェクト（キャッシュフロー集合）へ「コンパイル」し、イテレーション中には Curve からの Discount Factor (DF) 取得とベクトル積のみが発生するように最適化すべきです。これにより、イテレーションごとのカレンダー演算やコンベンション参照を排除できます。

## Introduction

本仕様は、`infra_master::market::MarketInstrument` から `pricer_models` のキャリブレーションループで使用する静的な Trade オブジェクト（Immutable Cashflow Set）への「コンパイル」プロセスを導入するものである。

現在の課題として、`MarketInstrument` は `MarketConvention` と `Rate` を結合してキャッシュフローを展開する能力を持つが、これを `CalibrationProblem` のイテレーションループ内で直接評価すると、毎回カレンダー演算やコンベンション参照が発生し、計算効率を損なう。

本仕様では、`CurveDefinition` から `CalibrationProblem` を生成する段階で、全ての `MarketInstrument` を静的な Trade オブジェクトへ事前コンパイルし、イテレーション中には Curve からの Discount Factor (DF) 取得とベクトル積のみが発生するように最適化する。

## Requirements

### Requirement 1: Instrument Compiler Infrastructure

**Objective:** As a 量的開発者, I want MarketInstrument を静的なキャッシュフロー集合にコンパイルする仕組み, so that キャリブレーションループ内での冗長な計算を排除できる。

#### Acceptance Criteria
1. When `InstrumentCompiler::compile()` が呼び出された場合, the Compiler shall `MarketInstrument` を `CompiledInstrument` に変換し、全てのキャッシュフロー日付、年率係数、想定元本を事前計算する
2. The CompiledInstrument shall 以下の事前計算済みフィールドを保持する: cashflow_dates (Vec<Date>), year_fractions (Vec<f64>), notionals (Vec<f64>), discount_factor_indices (Vec<usize>)
3. When コンパイル時に無効なコンベンションが検出された場合, the Compiler shall `CompileError::InvalidConvention` を返却する
4. The Compiler shall Deposit, Swap, OIS, FRA, Futures の各商品タイプをサポートする
5. When `MarketConvention` が XCcyBasis, FxForward, FxSwap の場合, the Compiler shall `CompileError::UnsupportedInstrument` を返却する

### Requirement 2: CalibrationProblem Pre-compilation Integration

**Objective:** As a 量的開発者, I want CalibrationProblem の構築時に全商品を事前コンパイルする, so that イテレーション中のオーバーヘッドを最小化できる。

#### Acceptance Criteria
1. When `CalibrationProblem::from_curve_definition()` が呼び出された場合, the Builder shall 全ての `MarketInstrument` を `CompiledInstrument` にコンパイルする
2. The CalibrationProblem shall コンパイル済み商品の参照を保持し、イテレーション中に再コンパイルしない
3. When コンパイルが完了した場合, the System shall 商品数、総キャッシュフロー数、コンパイル時間をログ出力する
4. If コンパイルエラーが発生した場合, then the Builder shall エラーを伝播し、部分的にコンパイルされた状態を残さない
5. The CompiledInstrument shall `Clone` と `Debug` トレイトを実装する

### Requirement 3: Efficient Pricing Error Computation

**Objective:** As a キャリブレーションエンジン, I want コンパイル済み商品から効率的に pricing error を計算する, so that Newton 法のイテレーション速度を向上できる。

#### Acceptance Criteria
1. When `CompiledInstrument::pricing_error()` が呼び出された場合, the System shall カレンダー演算やコンベンション参照なしに、DF 取得とベクトル積のみで計算する
2. The pricing_error 計算 shall O(n) の時間計算量を維持する（n = キャッシュフロー数）
3. When 全てのキャッシュフローが処理された場合, the System shall theoretical_rate と market_rate の差を返却する
4. The CompiledInstrument shall `CalibrationInstrument<T>` トレイトを実装する
5. While キャリブレーションイテレーション中, the System shall メモリアロケーションを発生させない

### Requirement 4: Interpolation Matrix Pre-computation

**Objective:** As a 量的開発者, I want 補間行列を事前計算する, so that DF 取得のオーバーヘッドを削減できる。

#### Acceptance Criteria
1. When `InterpolationMatrix::new()` が呼び出された場合, the System shall 全キャッシュフロー日付からピラー日付への補間係数を事前計算する
2. The InterpolationMatrix shall CSR (Compressed Sparse Row) 形式で格納し、メモリ効率を確保する
3. When `InterpolationMatrix::apply()` が呼び出された場合, the System shall ピラー DF ベクトルから全キャッシュフロー日付の DF をベクトル積で計算する
4. The apply 操作 shall SIMD 最適化が可能な連続メモリレイアウトを使用する
5. When log-linear 補間が指定された場合, the System shall log(DF) 空間で補間係数を計算する

### Requirement 5: Domain Separation Enforcement

**Objective:** As a アーキテクト, I want infra_master と pricer_models の責務分離を明確化する, so that 依存関係ルールを維持できる。

#### Acceptance Criteria
1. The `InstrumentCompiler` shall `pricer_models::builder` モジュールに配置される
2. The Compiler shall `infra_master::market::MarketInstrument` を入力として受け取り、`pricer_models` 固有の型を出力する
3. The CompiledInstrument shall `infra_master` の型に依存しない（Date を除く）
4. When コンパイル後, the System shall 元の MarketInstrument への参照を保持しない
5. The A-I-P-S 依存関係ルール shall 維持される（Pricer は Adapter に依存しない）

### Requirement 6: Backward Compatibility

**Objective:** As a 既存ユーザー, I want 既存の API との後方互換性を維持する, so that 移行コストを最小化できる。

#### Acceptance Criteria
1. The `CalibrationProblem::new()` shall 既存のシグネチャを維持し、内部でコンパイルを実行する
2. The `CalibrationInstrument<T>` トレイト shall 変更なく維持される
3. When 既存の `MarketInstrument<T>` (pricer_models) が使用された場合, the System shall 既存の動作を維持する
4. The `CalibrationProblem::from_curve_definition()` shall 新規 API として追加される
5. If 既存のテストが存在する場合, the System shall 全てのテストが引き続きパスする

### Requirement 7: Performance Verification

**Objective:** As a 量的開発者, I want コンパイルによるパフォーマンス向上を検証する, so that 最適化の効果を測定できる。

#### Acceptance Criteria
1. The System shall コンパイル前後の pricing_error 計算時間を比較するベンチマークを提供する
2. When 10 商品のカーブキャリブレーションを実行した場合, the System shall イテレーションあたり 30% 以上の速度向上を達成する
3. The Benchmark shall `criterion` クレートを使用して再現可能な結果を生成する
4. When コンパイル時間が測定された場合, the System shall コンパイルコストがキャリブレーション全体の 5% 未満であることを確認する
5. The System shall メモリ使用量の増加が 20% 未満であることを確認する

### Requirement 8: Error Handling and Validation

**Objective:** As a 開発者, I want コンパイル時に包括的なエラー検証を行う, so that 実行時エラーを早期に検出できる。

#### Acceptance Criteria
1. When 満期日が評価日より前の場合, the Compiler shall `CompileError::InvalidMaturity` を返却する
2. When キャッシュフロー日付が負の年率係数を持つ場合, the Compiler shall `CompileError::InvalidYearFraction` を返却する
3. When コンベンションと商品タイプが不整合の場合, the Compiler shall `CompileError::ConventionMismatch` を返却する
4. The CompileError shall `thiserror` を使用して構造化エラーを提供する
5. When エラーが発生した場合, the System shall 問題のある商品のインデックスとレートID を含める

