# Requirements Document

## Introduction

本ドキュメントは、Rustによる定量金融ライブラリ（Neutryx）における確率的ボラティリティモデル（Heston、SABR）の実装およびEnzyme自動微分（AD）統合の完成に関する要件を定義する。

本機能は以下を実現する:

1. **StochasticModelトレイト抽象化**: 確率過程モデルの統一インターフェース
2. **Hestonモデル**: 平均回帰分散過程とQE離散化スキームによる実装
3. **SABRモデル**: Haganのインプライドボラティリティ公式による実装
4. **EnzymeContext**: AD（自動微分）モード管理の完成
5. **Tangent/Adjointパス生成**: Greeks計算のための感度伝播
6. **MonteCarloPricerとの統合**: 新モデルのMCエンジンへの組み込み

全ての実装は、Enzyme LLVM互換性のためにsmooth approximations（smooth_max、smooth_indicator）を使用し、num-dualによるデュアルモード検証のためにジェネリックFloat型をサポートする必要がある。

## Requirements

### Requirement 1: StochasticModelトレイト抽象化

**Objective:** As a ライブラリ開発者, I want 確率過程モデルの統一トレイトインターフェース, so that 異なるモデル（GBM、Heston、SABR）を一貫した方法で利用でき、新しいモデルの追加が容易になる。

#### Acceptance Criteria 1

1. The pricer_models crate shall StochasticModelトレイトを提供し、パス生成に必要なメソッドを定義する
2. The StochasticModel trait shall ジェネリック型パラメータ`T: Float`を使用し、f64およびDualNumber型の両方をサポートする
3. The StochasticModel trait shall `evolve_step`メソッドを定義し、1タイムステップのシミュレーションを実行する
4. The StochasticModel trait shall `initial_state`メソッドを定義し、モデルの初期状態を返す
5. When StochasticModelを実装する際, the implementing type shall Differentiableマーカートレイトも実装し、AD互換性を保証する
6. The StochasticModel trait shall 静的ディスパッチ（enum-based）のみをサポートし、`Box<dyn Trait>`は使用禁止とする

### Requirement 2: Hestonモデル実装

**Objective:** As a クオンツ開発者, I want Hestonモデルによるパス生成機能, so that 確率的ボラティリティを考慮したエキゾチックオプションの価格計算ができる。

#### Acceptance Criteria 2

1. The HestonModel shall 以下のパラメータを持つ: スポット価格(S0)、初期分散(v0)、長期分散(theta)、平均回帰速度(kappa)、ボラティリティのボラティリティ(xi)、相関(rho)、リスクフリーレート(r)
2. The HestonModel shall Quadratic Exponential (QE) 離散化スキームを実装し、分散が非負を維持することを保証する
3. When 分散が負になるリスクがある場合, the HestonModel shall smooth_max関数を使用して滑らかに非負制約を適用する
4. The HestonModel shall 相関ブラウン運動を生成し、資産価格と分散の相関構造を正確に再現する
5. If Fellerの条件（2*kappa*theta > xi^2）が満たされない場合, the HestonModel shall 警告をログに出力し、分散フロアを適用する
6. The HestonModel shall ジェネリックFloat型で実装し、num-dualとの互換性を維持する
7. The HestonModel shall StochasticModelトレイトを実装する

### Requirement 3: SABRモデル実装

**Objective:** As a クオンツ開発者, I want SABRモデルによるインプライドボラティリティ計算機能, so that スマイルダイナミクスを考慮したオプション価格計算ができる。

#### Acceptance Criteria 3

1. The SABRModel shall 以下のパラメータを持つ: フォワード価格(F)、初期ボラティリティ(alpha)、ボラティリティのボラティリティ(nu)、相関(rho)、ベータ(beta)
2. The SABRModel shall Haganのインプライドボラティリティ公式を実装し、任意のストライクに対するインプライドボラティリティを計算する
3. When ストライクがフォワード価格に近い（ATM）場合, the SABRModel shall ATM近傍の展開公式を使用して数値安定性を確保する
4. The SABRModel shall ジェネリックFloat型で実装し、num-dualとの互換性を維持する
5. If beta = 0の場合, the SABRModel shall Normal SABRモードで動作する
6. If beta = 1の場合, the SABRModel shall Lognormal SABRモードで動作する
7. The SABRModel shall smooth_abs関数を使用してlog(F/K)の計算を滑らかに処理し、AD互換性を維持する

### Requirement 4: EnzymeContext実装

**Objective:** As a ライブラリ開発者, I want EnzymeContextによるADモード管理機能, so that Forward modeとReverse modeの切り替えを一元管理でき、将来のEnzyme完全統合に備えられる。

#### Acceptance Criteria 4

1. The EnzymeContext shall ADモードを表すenum（Forward、Reverse、Inactive）を定義する
2. The EnzymeContext shall 現在のADモードを保持し、モード切り替えAPIを提供する
3. When Forward modeが選択された場合, the EnzymeContext shall tangent（接線）値の伝播を有効化する
4. When Reverse modeが選択された場合, the EnzymeContext shall adjoint（随伴）値の逆伝播を有効化する
5. The EnzymeContext shall スレッドローカルストレージを使用し、マルチスレッド環境での安全な使用を保証する
6. The EnzymeContext shall Activity annotationの設定APIを提供し、パラメータ毎のAD活性（Const、Dual、Active、Duplicated）を管理する
7. While Enzymeの完全統合が完了していない間, the EnzymeContext shall 有限差分をフォールバックとして使用する

### Requirement 5: Tangentパス生成（Forward Mode AD）

**Objective:** As a クオンツ開発者, I want Forward Mode ADによるTangentパス生成機能, so that Delta、Vega等のGreeksを効率的に計算できる。

#### Acceptance Criteria 5

1. The pricer_kernel crate shall `generate_heston_paths_tangent`関数を提供し、Hestonモデルのtangentパスを生成する
2. The tangent path generator shall 各パラメータに対するtangent seed（d_spot、d_v0、d_kappa等）を受け取る
3. When tangent seedが1.0に設定された場合, the generated tangent paths shall 対応するパラメータに対する感度を表す
4. The tangent path generator shall primalパスとtangentパスを同時に生成し、メモリ効率を最大化する
5. The tangent path generator shall 相関ブラウン運動のtangent伝播を正確に処理する
6. The tangent path generator shall ジェネリックFloat型で実装し、num-dualによる検証を可能にする

### Requirement 6: Adjointパス生成（Reverse Mode AD）

**Objective:** As a クオンツ開発者, I want Reverse Mode ADによるAdjointパス生成機能, so that 多数のパラメータに対するGreeksを効率的に一括計算できる。

#### Acceptance Criteria 6

1. The pricer_kernel crate shall `generate_heston_paths_adjoint`関数を提供し、Hestonモデルのadjointパスを生成する
2. The adjoint path generator shall 終端payoffからの感度逆伝播を実装する
3. When adjoint modeが使用された場合, the adjoint path generator shall 全パラメータに対する感度を1回のbackward passで計算する
4. The adjoint path generator shall チェックポインティングを使用してメモリ使用量を制御する
5. If メモリが不足する場合, the adjoint path generator shall 再計算戦略にフォールバックする
6. The adjoint path generator shall Enzyme `#[autodiff_reverse]`マクロへの将来的な移行を考慮した設計とする

### Requirement 7: MonteCarloPricerとの統合

**Objective:** As a ライブラリユーザー, I want MonteCarloPricerで確率的ボラティリティモデルを使用する機能, so that 統一されたAPIでGBM、Heston、SABRモデルを切り替えて価格計算ができる。

#### Acceptance Criteria 7

1. The MonteCarloPricer shall StochasticModelトレイトを使用して任意の確率過程モデルをサポートする
2. When HestonModelが指定された場合, the MonteCarloPricer shall 2次元パス（資産価格、分散）を生成する
3. The MonteCarloPricer shall モデル固有の設定（QE discretization parameters等）を受け取るAPIを提供する
4. The MonteCarloPricer shall `price_with_greeks_ad`メソッドを提供し、ADベースのGreeks計算をサポートする
5. While ADモードが有効な間, the MonteCarloPricer shall tangent/adjointパスを使用してGreeksを計算する
6. The MonteCarloPricer shall モデルに関わらず一貫したPricingResult構造体を返す

### Requirement 8: Smooth Approximation拡張

**Objective:** As a ライブラリ開発者, I want 確率的ボラティリティモデル用の追加smooth approximation関数, so that Enzyme LLVM互換性を維持しながらモデル特有の非連続性を処理できる。

#### Acceptance Criteria 8

1. The pricer_core crate shall `smooth_sqrt`関数を提供し、ゼロ近傍での平方根を滑らかに処理する
2. The pricer_core crate shall `smooth_log`関数を提供し、ゼロ近傍での対数を滑らかに処理する
3. The pricer_core crate shall `smooth_pow`関数を提供し、非整数べき乗を滑らかに処理する
4. When epsilon（平滑化パラメータ）が小さい場合, the smooth functions shall 真の関数値に収束する
5. The smooth functions shall ジェネリックFloat型で実装し、f32、f64、DualNumber型全てをサポートする
6. The smooth functions shall 数値オーバーフローを防ぐために安定化技法（log-sum-exp trick等）を使用する

### Requirement 9: デュアルモード検証

**Objective:** As a ライブラリ開発者, I want num-dualを使用したデュアルモード検証機能, so that Enzyme ADの実装が正しいことを自動テストで検証できる。

#### Acceptance Criteria 9

1. The pricer_kernel crate shall `verify`モジュールを拡張し、Heston/SABRモデルの検証テストを追加する
2. The verification tests shall num-dual DualNumber型を使用して参照勾配を計算する
3. The verification tests shall Enzyme（または有限差分フォールバック）の勾配とnum-dual勾配を比較する
4. When 勾配の相対誤差が閾値（デフォルト: 1e-4）を超えた場合, the verification test shall 失敗し、詳細なエラーメッセージを出力する
5. The verification tests shall 複数のパラメータセット（ATM、ITM、OTM等）をカバーする
6. The verification tests shall CI/CDパイプラインで自動実行される

### Requirement 10: エラーハンドリングと境界条件

**Objective:** As a ライブラリユーザー, I want 堅牢なエラーハンドリングと境界条件の処理, so that 不正な入力や数値的な問題が発生した場合に明確なエラーメッセージを受け取れる。

#### Acceptance Criteria 10

1. The HestonModel shall パラメータ検証を実装し、不正な値（負の分散、|rho| > 1等）に対してエラーを返す
2. The SABRModel shall パラメータ検証を実装し、不正な値（負のalpha、beta範囲外等）に対してエラーを返す
3. If 数値的不安定性が検出された場合, the model implementations shall NumericalInstabilityエラーを返す
4. When 数値がNaNまたはInfinityになった場合, the implementations shall 明確なエラーメッセージと共に早期リターンする
5. The error types shall `thiserror`クレートを使用して定義し、構造化されたエラー情報を提供する
6. The implementations shall 境界条件（T=0、S=K等）を適切に処理し、特異点を回避する