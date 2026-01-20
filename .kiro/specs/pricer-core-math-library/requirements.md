# Requirements Document

## Introduction

本仕様は、`pricer_core`クレートの`math`モジュールを拡充し、金融デリバティブ価格計算に必要な包括的な数学ライブラリを実装することを目的とする。既存の`smoothing`、`interpolators`、`solvers`、`numeric`モジュールに加え、統計分布、数値積分、有限差分、最適化、乱数生成、線形代数、フィッティング、メッシュ生成の各機能を追加する。

すべての実装は`T: num_traits::Float`でジェネリックとし、Enzyme自動微分との互換性を確保する。

## Requirements

### Requirement 1: 確率分布モジュール（Distribution）

**Objective:** As a クオンツ開発者, I want 標準正規分布、二変量正規分布、非心カイ二乗分布、ガウシアンコピュラの確率分布関数を使用したい, so that デリバティブ価格計算やモンテカルロシミュレーションで正確な確率計算ができる。

#### Acceptance Criteria

1. When 標準正規分布の累積分布関数（CDF）が呼び出された場合, the math module shall Abramowitz-Stegun近似またはHart近似を用いて相対誤差1e-15以内の値を返す。
2. When 標準正規分布の確率密度関数（PDF）が呼び出された場合, the math module shall 解析的な公式に基づいた値を返す。
3. When 標準正規分布の逆累積分布関数（inverse CDF / quantile）が呼び出された場合, the math module shall Moro近似またはAcklam近似を用いて相対誤差1e-9以内の値を返す。
4. When 二変量正規分布の累積分布関数が呼び出された場合, the math module shall Drezner-Wesolowsky近似または数値積分を用いて相対誤差1e-10以内の値を返す。
5. When 非心カイ二乗分布の累積分布関数が呼び出された場合, the math module shall CIRモデルのゼロクーポン債価格計算に必要な精度（相対誤差1e-8以内）を達成する。
6. When ガウシアンコピュラの結合確率が計算された場合, the math module shall 相関行列と周辺分布から正しい結合確率を返す。
7. The math module shall すべての分布関数を`T: Float`でジェネリックに実装し、f64およびDual数に対応する。

### Requirement 2: 数値積分モジュール（Integrator）

**Objective:** As a クオンツ開発者, I want 高精度な数値積分手法を使用したい, so that オプション価格の数値積分やヘストンモデルの特性関数積分を正確に計算できる。

#### Acceptance Criteria

1. When 1次元積分が要求された場合, the math module shall Gauss-Legendre求積法を提供し、指定された次数（7点、15点、21点）で積分を計算する。
2. When 高精度1次元積分が要求された場合, the math module shall Gauss-Kronrod求積法（G7-K15、G10-K21）を提供し、誤差推定付きで適応的積分を実行する。
3. When 2次元積分が要求された場合, the math module shall 二重積分を実行する機能を提供する。
4. When 常微分方程式の数値解が要求された場合, the math module shall Runge-Kutta法（RK4、RK45）を提供し、時間発展問題を解く。
5. If 積分範囲が無限区間の場合, then the math module shall 変数変換（tanh-sinh変換等）を適用して収束する数値積分を実行する。
6. The math module shall 積分関数を`Fn(T) -> T`クロージャで受け取り、ジェネリックな積分を可能にする。

### Requirement 3: 有限差分モジュール（Calculus / FiniteDifference）

**Objective:** As a クオンツ開発者, I want 有限差分法による数値微分を使用したい, so that 自動微分が使用できない状況でもGreeks計算ができる。

#### Acceptance Criteria

1. When 前方差分が要求された場合, the math module shall `(f(x+h) - f(x)) / h`を計算する。
2. When 後方差分が要求された場合, the math module shall `(f(x) - f(x-h)) / h`を計算する。
3. When 中心差分が要求された場合, the math module shall `(f(x+h) - f(x-h)) / (2h)`を計算し、O(h²)の精度を達成する。
4. When 2階導関数が要求された場合, the math module shall `(f(x+h) - 2f(x) + f(x-h)) / h²`を計算する。
5. When 偏微分が要求された場合, the math module shall 多変数関数に対して指定された変数について有限差分を計算する。
6. The math module shall bump幅の自動選択機能を提供し、数値安定性を確保する。

### Requirement 4: 最適化モジュール拡張（Optimiser）

**Objective:** As a クオンツ開発者, I want 追加の最適化アルゴリズムを使用したい, so that モデルキャリブレーションや曲線フィッティングで最適なパラメータを求められる。

#### Acceptance Criteria

1. When 制約なし最適化が要求された場合, the math module shall L-BFGS法を提供し、大規模最適化問題を効率的に解く。
2. When 導関数なしの最適化が要求された場合, the math module shall Nelder-Mead法（Amoeba）を提供する。
3. When 直線探索が要求された場合, the math module shall Backtracking、Bracketing、More-Thuente、Nocedal-Wright法を提供する。
4. The math module shall 最適化の収束判定条件（勾配ノルム、関数値変化、最大反復回数）を設定可能にする。
5. While 最適化が実行中の場合, the math module shall 各反復のログ情報（関数値、勾配ノルム）をコールバックで提供可能にする。

### Requirement 5: 1次元補間モジュール拡張（Interpolator 1D）

**Objective:** As a クオンツ開発者, I want 追加の1次元補間手法を使用したい, so that イールドカーブやボラティリティスマイルを柔軟に補間できる。

#### Acceptance Criteria

1. When フラット補間が要求された場合, the math module shall 区分定数補間（左側値または右側値）を提供する。
2. When 対数線形補間が要求された場合, the math module shall y値の対数空間で線形補間を行い、ディスカウントファクターの補間に適した結果を返す。
3. When Hermiteスプライン補間が要求された場合, the math module shall 指定された導関数値を用いたC¹連続な補間を提供する。
4. When Kahale補間が要求された場合, the math module shall アービトラージフリーのボラティリティ補間を提供する。
5. When SVI（Stochastic Volatility Inspired）スライス補間が要求された場合, the math module shall SVIパラメータによるボラティリティスマイルの補間を提供する。
6. The math module shall 二分探索および線形探索によるグリッド点検索を提供する。
7. If 補間点が定義域外の場合, then the math module shall 外挿モード（フラット、線形、エラー）を選択可能にする。

### Requirement 6: 2次元・3次元補間モジュール（Interpolator 2D/3D）

**Objective:** As a クオンツ開発者, I want 多次元補間を使用したい, so that ボラティリティサーフェスやボラティリティキューブを補間できる。

#### Acceptance Criteria

1. When 2次元補間が要求された場合, the math module shall バイリニア補間を提供する。
2. When 2次元逆距離加重補間が要求された場合, the math module shall IDW（Inverse Distance Weighting）法を提供する。
3. When ボラティリティサーフェスが構築された場合, the math module shall ストライクと満期の2次元グリッドに対する補間を提供する。
4. When 3次元補間が要求された場合, the math module shall トリリニア補間を提供する。
5. When レイヤード3次元補間が要求された場合, the math module shall 2次元補間を積み重ねた階層的補間を提供する。
6. The math module shall 補間サーフェスの微分（dV/dK、dV/dT）を数値微分または解析微分で計算可能にする。

### Requirement 7: 金融関数モジュール（FinancialFunctions）

**Objective:** As a クオンツ開発者, I want 金融計算に特化した関数を使用したい, so that オプション価格計算やボラティリティ計算を効率的に行える。

#### Acceptance Criteria

1. When Black-Scholesオプション価格が要求された場合, the math module shall コール/プット価格およびGreeks（デルタ、ガンマ、ベガ、シータ、ロー）を計算する。
2. When Bachelierモデル価格が要求された場合, the math module shall 正規モデルによるオプション価格を計算する。
3. When SABRボラティリティが要求された場合, the math module shall Haganの近似公式によるインプライドボラティリティを計算する。
4. When Normal SABRボラティリティ（Antonov近似）が要求された場合, the math module shall 正規SABRモデルのボラティリティを計算する。
5. When SVIパラメータからボラティリティが要求された場合, the math module shall SVI公式によるインプライドボラティリティを計算する。
6. When アービトラージフリー検証が要求された場合, the math module shall ボラティリティサーフェスのバタフライ条件およびカレンダースプレッド条件を検証する。

### Requirement 8: フィッティングモジュール（Fitting）

**Objective:** As a クオンツ開発者, I want 曲線フィッティング手法を使用したい, so that 市場データからモデルパラメータを推定できる。

#### Acceptance Criteria

1. When 線形最小二乗フィットが要求された場合, the math module shall 正規方程式またはQR分解による解を提供する。
2. When ガウシアンフィットが要求された場合, the math module shall 正規分布へのフィッティングを提供する。
3. The math module shall フィッティング結果に決定係数（R²）および残差を含める。

### Requirement 9: 線形代数モジュール（LinearAlgebra）

**Objective:** As a クオンツ開発者, I want 基本的な線形代数演算を使用したい, so that 行列計算やコレスキー分解が必要な計算を実行できる。

#### Acceptance Criteria

1. When 行列演算が要求された場合, the math module shall 行列の加算、減算、乗算、転置を提供する。
2. When コレスキー分解が要求された場合, the math module shall 正定値対称行列のLL^T分解を提供する。
3. When LU分解が要求された場合, the math module shall 連立一次方程式の解法を提供する。
4. When 行列式が要求された場合, the math module shall 正方行列の行列式を計算する。
5. The math module shall 行列データ構造（Matrix、SquareMatrix）を`T: Float`でジェネリックに提供する。
6. If 行列演算でエラーが発生した場合, then the math module shall 次元不整合や特異行列のエラーを適切に報告する。

### Requirement 10: 乱数生成モジュール（RandomNumberGenerator）

**Objective:** As a クオンツ開発者, I want 高品質な乱数生成器を使用したい, so that モンテカルロシミュレーションで再現可能な乱数列を生成できる。

#### Acceptance Criteria

1. When Mersenne Twister乱数生成器が要求された場合, the math module shall MT19937アルゴリズムによる擬似乱数を生成する。
2. The math module shall シード値による乱数列の再現性を保証する。
3. When 一様乱数が要求された場合, the math module shall [0, 1)区間の一様分布乱数を生成する。
4. When 正規乱数が要求された場合, the math module shall Box-Muller法またはZiggurat法により標準正規分布乱数を生成する。

### Requirement 11: ルートファインダー拡張（Solver）

**Objective:** As a クオンツ開発者, I want 追加のルートファインディングアルゴリズムを使用したい, so that インプライドボラティリティ計算やキャリブレーションで確実に解を求められる。

#### Acceptance Criteria

1. When 二分法が要求された場合, the math module shall 収束保証のある二分法ソルバーを提供する。
2. When Backtracking Newton法が要求された場合, the math module shall 直線探索付きのNewton法を提供し、収束性を改善する。
3. When 汎用ソルバーが要求された場合, the math module shall Brent法、Newton法、二分法を自動選択するソルバーを提供する。
4. The math module shall ソルバーの収束状態（成功、最大反復到達、発散）を結果に含める。

### Requirement 12: メッシュ生成モジュール（Mesh）

**Objective:** As a クオンツ開発者, I want 計算グリッドを生成したい, so that 有限差分法やモンテカルロシミュレーションで適切な離散化を行える。

#### Acceptance Criteria

1. When 1次元メッシュが要求された場合, the math module shall 等間隔および対数間隔のグリッドを生成する。
2. When 2次元メッシュが要求された場合, the math module shall テンソル積グリッドを生成する。
3. The math module shall グリッドの細分化（refinement）機能を提供する。

### Requirement 13: ユーティリティ関数（UtilityFunctions）

**Objective:** As a クオンツ開発者, I want 汎用的な数学ユーティリティを使用したい, so that 繰り返し使用される計算を効率化できる。

#### Acceptance Criteria

1. The math module shall 符号関数（sign）、クランプ関数（clamp）、線形補間（lerp）を提供する。
2. The math module shall 階乗、組み合わせ、二項係数を計算する関数を提供する。
3. The math module shall 対数ガンマ関数（log_gamma）およびベータ関数を提供する。
4. The math module shall すべてのユーティリティ関数を`T: Float`でジェネリックに実装する。

### Requirement 14: 非機能要件（パフォーマンスとテスト）

**Objective:** As a システム管理者, I want 数学ライブラリが高性能かつ信頼性が高いことを保証したい, so that 本番環境で安定した計算ができる。

#### Acceptance Criteria

1. The math module shall すべての公開関数に対してユニットテストを提供する。
2. The math module shall 数値精度に関するプロパティベーステストを提供する。
3. The math module shall 各モジュールにドキュメンテーションコメントを含める。
4. While 数値計算が実行中の場合, the math module shall オーバーフロー、アンダーフロー、NaNを適切に処理する。
5. The math module shall Clippy pedanticリントに準拠する。
6. The math module shall 既存の`pricer_core`のコード品質基準（British English、ドキュメント、エラーハンドリング）に準拠する。