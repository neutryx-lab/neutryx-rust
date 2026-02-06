# Requirements Document

## Introduction
本ドキュメントは、Curve Bootstrap Engineの要件を定義する。本機能は、Index毎にカーブ構築に必要なInstrument集合と構築方法を設定駆動で定義し、Bootstrapを用いてParameterCurveを生成するエンジンを提供する。既存の`pricer_models/src/market/calibration/bootstrapping/`モジュールを拡張し、`infra_domain/src/trade/`の商品定義・キャッシュフロー展開機能と統合することで、汎用的なgetDF/getFwd機能と計算グラフ（Enzyme AD対応）を備えたカーブを構築する。また、同一条件での再構築時にはキャッシュを活用して計算を省略する。

## Requirements

### Requirement 1: Index-Curve Definition（Index別カーブ定義）
**Objective:** As a クオンツ開発者, I want Index毎にカーブ構築に必要なInstrument集合と構築パラメータを宣言的に定義したい, so that カーブ構築ロジックを設定ファイルで管理し、コード変更なしに新しいIndex対応を追加できる

#### Acceptance Criteria
1. The CurveDefinition shall Indexキー（例: `USD-SOFR`, `EUR-ESTR`）とInstrumentセット仕様の対応付けを保持する
2. When CurveDefinitionがロードされたとき, the CurveEngine shall 指定されたIndexに対応するInstrument種別（OIS, IRS, FRA, Futures）のリストを取得できる
3. The CurveDefinition shall 各Instrumentの満期テナーポイント（例: 1M, 3M, 6M, 1Y, 2Y, ..., 50Y）を定義できる
4. The CurveDefinition shall Instrument毎のコンベンション参照（DayCount, BusinessDayConvention, PaymentFrequency）を含む
5. If CurveDefinitionに未知のIndexが指定されたとき, then the CurveEngine shall `UnknownIndex`エラーを返す
6. The CurveDefinition shall `infra_domain::trade::IndexType`と互換性のある型でIndexを識別する

### Requirement 2: Curve Parameter Configuration（カーブパラメータ設定）
**Objective:** As a クオンツ開発者, I want カーブのパラメータ表現（LogDF, ZeroRate, InstantaneousForward等）と補間器を設定から指定したい, so that 異なるカーブ構築戦略を柔軟に切り替えられる

#### Acceptance Criteria
1. The CurveConfig shall パラメータ表現種別を以下から選択可能とする: `LogDiscountFactor`, `ZeroRate`, `InstantaneousForward`
2. The CurveConfig shall 補間器種別を以下から選択可能とする: `LogLinear`, `LinearZeroRate`, `CubicSpline`, `MonotonicCubic`, `FlatForward`
3. When パラメータ表現が`LogDiscountFactor`のとき, the BootstrappedCurve shall 内部的にlog(DF)を格納し補間する
4. When パラメータ表現が`ZeroRate`のとき, the BootstrappedCurve shall 連続複利ゼロレートを格納し補間する
5. The CurveConfig shall 外挿設定（許可/禁止、外挿方法）を指定できる
6. The CurveConfig shall 負金利許可フラグを指定できる
7. If 設定された補間器とパラメータ表現の組み合わせが非対応のとき, then the CurveEngine shall `InvalidConfiguration`エラーを返す

### Requirement 3: Instrument-to-Cashflow Integration（Instrument-キャッシュフロー統合）
**Objective:** As a クオンツ開発者, I want Bootstrap用Instrumentを`infra_domain::trade`の商品定義とキャッシュフロー展開機能を使って構築したい, so that 商品定義の一貫性を保ちながらカーブ構築に利用できる

#### Acceptance Criteria
1. The CurveEngine shall `infra_domain::trade::instrument_def`のRates商品定義（IRS, OIS, FRA等）からBootstrapInstrumentを生成できる
2. When OIS定義から生成するとき, the CurveEngine shall `infra_domain::trade::convention::swap`のOISコンベンションを適用する
3. When IRS定義から生成するとき, the CurveEngine shall Fixed LegとFloat Legのキャッシュフロースケジュールを`infra_domain::trade::cashflow`を用いて展開する
4. The CurveEngine shall FRA定義から期間（start, end）とレートを抽出しBootstrapInstrumentを生成する
5. The CurveEngine shall Futures定義から価格と満期を抽出し、Convexity調整を適用してBootstrapInstrumentを生成する
6. If Instrumentの定義が不完全なとき, then the CurveEngine shall `IncompleteInstrumentDefinition`エラーを返す

### Requirement 4: Bootstrap Engine（ブートストラップエンジン）
**Objective:** As a クオンツ開発者, I want 設定に基づいてInstrumentレートからParameterCurveを逐次的にBootstrapしたい, so that 市場レートと整合するカーブを構築できる

#### Acceptance Criteria
1. The BootstrapEngine shall Instrumentを満期順にソートし、短期から長期へ逐次的にカーブポイントを求解する
2. When 各Instrumentを処理するとき, the BootstrapEngine shall Newton-Raphson法で残差関数をゼロにするDFを求める
3. If Newton-Raphson法が収束しないとき, then the BootstrapEngine shall Brent法にフォールバックする
4. The BootstrapEngine shall 構築したPartial Curveを用いて後続Instrumentのキャッシュフロー現在価値を計算する
5. The BootstrapEngine shall 収束許容誤差（tolerance）と最大反復回数（max_iterations）を設定から読み取る
6. When 全Instrumentの処理が完了したとき, the BootstrapEngine shall 指定されたパラメータ表現と補間器でParameterCurveを構築する
7. The BootstrapEngine shall 構築結果として残差ベクトルと収束ステータスを返す

### Requirement 5: Generic Curve Interface（汎用カーブインターフェース）
**Objective:** As a デリバティブプライサー開発者, I want 構築されたカーブから任意の時点のDF、ゼロレート、フォワードレートを取得したい, so that 商品価格計算に利用できる

#### Acceptance Criteria
1. The ParameterCurve shall `discount_factor(t: T) -> Result<T, CurveError>` メソッドを提供する
2. The ParameterCurve shall `zero_rate(t: T) -> Result<T, CurveError>` メソッドを提供する
3. The ParameterCurve shall `forward_rate(t1: T, t2: T) -> Result<T, CurveError>` メソッドを提供する
4. The ParameterCurve shall `instantaneous_forward(t: T) -> Result<T, CurveError>` メソッドを提供する
5. When 時間tがカーブ範囲外かつ外挿禁止のとき, the ParameterCurve shall `OutOfBounds`エラーを返す
6. The ParameterCurve shall pillar点の一覧と対応するパラメータ値を取得するメソッドを提供する
7. The ParameterCurve shall `num_traits::Float`をバウンドとするジェネリック型`T`をサポートする（f64, Dual等）

### Requirement 6: Computation Graph for AD（自動微分用計算グラフ）
**Objective:** As a リスク計算開発者, I want カーブがInstrumentレートへの計算グラフを保持し、Enzyme ADで感度計算に対応したい, so that Greeks計算を効率的に実行できる

#### Acceptance Criteria
1. The ParameterCurve shall 入力レートからカーブパラメータへの微分可能な計算グラフを保持する
2. When `num-dual-mode` featureが有効なとき, the BootstrapEngine shall Implicit Function Theoremを用いてAD tapeにsolver反復を記録しない
3. The CurveEngine shall 各pillarのDF/パラメータに対する入力レート感度（Jacobian）を計算できる
4. When 感度計算が要求されたとき, the CurveEngine shall `BootstrapResultWithSensitivities`を返す
5. The ParameterCurve shall `pricer_core::types::Dual`型でインスタンス化可能である
6. If AD計算中に非微分可能な操作が発生したとき, then the CurveEngine shall 適切な警告またはフォールバックを提供する

### Requirement 7: Curve Caching（カーブキャッシュ）
**Objective:** As a パフォーマンス最適化担当者, I want 同一Index・同一レート・同一設定でのカーブ再構築時に計算を省略したい, so that バッチ処理やリアルタイム計算の効率を向上できる

#### Acceptance Criteria
1. The CurveCache shall Index、入力レート、設定のハッシュをキーとしてカーブをキャッシュする
2. When キャッシュにヒットしたとき, the CurveEngine shall 再計算せずにキャッシュされたカーブを返す
3. When キャッシュミスのとき, the CurveEngine shall カーブを構築しキャッシュに格納する
4. The CurveCache shall キャッシュサイズ上限（エントリ数またはメモリ）を設定可能とする
5. The CurveCache shall LRU（Least Recently Used）方式でキャッシュエビクションを行う
6. The CurveCache shall キャッシュを明示的にクリアするメソッドを提供する
7. While マルチスレッド環境で動作中, the CurveCache shall スレッドセーフなアクセスを保証する
8. The CurveCache shall キャッシュ統計（ヒット率、エントリ数）を取得するメソッドを提供する

### Requirement 8: Multi-Curve Support（マルチカーブ対応）
**Objective:** As a クオンツ開発者, I want Discountカーブと複数のProjectionカーブを同時に構築したい, so that OIS Discounting + Tenor Curveの現代的なカーブ構築に対応できる

#### Acceptance Criteria
1. The MultiCurveBuilder shall Discountカーブ（OIS-based）とProjectionカーブ（IBOR-based）を同時に構築できる
2. When Projectionカーブを構築するとき, the MultiCurveBuilder shall 別途構築されたDiscountカーブを参照してキャッシュフロー現在価値を計算する
3. The MultiCurveBuilder shall CurveSet（複数カーブの集合）を返す
4. The CurveSet shall Index名でカーブを検索できる
5. When 依存カーブが未構築のとき, the MultiCurveBuilder shall 依存関係に従って構築順序を自動決定する
6. If 循環依存が検出されたとき, then the MultiCurveBuilder shall `CircularDependency`エラーを返す

### Requirement 9: Error Handling（エラーハンドリング）
**Objective:** As a 開発者, I want カーブ構築の各段階で発生しうるエラーを明確に識別したい, so that デバッグと運用監視が容易になる

#### Acceptance Criteria
1. The CurveEngine shall 以下のエラー種別を区別する: `ConfigurationError`, `InstrumentError`, `BootstrapError`, `InterpolationError`, `CacheError`
2. When Bootstrap収束に失敗したとき, the BootstrapError shall 失敗したInstrumentのインデックス、満期、最終残差を含む
3. When 設定バリデーションに失敗したとき, the ConfigurationError shall 無効なフィールド名と理由を含む
4. The CurveEngine shall `thiserror`クレートを用いてエラー型を定義する
5. The CurveEngine shall エラーから`Result`型を返し、パニックを回避する
6. If 複数のエラーが発生したとき, then the CurveEngine shall 最初のエラーを返し、警告としてログに記録する

### Requirement 10: Configuration Serialization（設定のシリアライゼーション）
**Objective:** As a 運用担当者, I want カーブ定義と設定をJSON/YAML形式で外部ファイルから読み込みたい, so that 設定変更をデプロイなしに反映できる

#### Acceptance Criteria
1. The CurveDefinition shall `serde`による`Serialize`/`Deserialize`をサポートする
2. The CurveConfig shall `serde`による`Serialize`/`Deserialize`をサポートする
3. When `serde` featureが有効なとき, the CurveEngine shall JSON形式の設定ファイルを読み込める
4. The 設定ファイル shall スキーマバリデーションを通過した場合のみロードされる
5. When 設定ファイルのスキーマが不正なとき, the CurveEngine shall `ConfigurationParseError`を返す
6. The CurveEngine shall 設定のデフォルト値を提供し、省略されたフィールドに適用する

