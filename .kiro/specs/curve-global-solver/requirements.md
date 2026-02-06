# Requirements Document

## Introduction

カーブ構築を行列計算（連立方程式）として解くグローバルソルバーの実装要件。逐次的なブートストラップではなく、全期間同時推定（Global Solver）により、すべての観測商品がすべてのカーブパラメータに依存する構造をJacobianとして明示的に扱う。陰関数定理（Implicit Function Theorem）を用いたAADの高速化を実現する。

## Requirements

### Requirement 1: 多次元Newton-Raphsonソルバー

**Objective:** As a クオンツ開発者, I want 金融ロジックに依存しない汎用的な多次元Newton-Raphson法ソルバーを利用する, so that 任意の連立非線形方程式を行列計算として解くことができる.

#### Acceptance Criteria

1. The Global Solver shall 連立方程式 F(x) = 0 を初期値 x₀ から反復的に解き、収束解 x* を返す
2. The Global Solver shall 収束時点でのJacobian行列 J(x*) の逆行列（またはLU分解）を結果に含める
3. When ノルム ||F(x)|| が許容誤差以下になった場合, the Global Solver shall 収束成功として結果を返す
4. If 最大反復回数に達しても収束しない場合, the Global Solver shall `CalibrationError::ConvergenceFailure` を返す
5. The Global Solver shall ndarray ベースの行列演算を使用し、BLAS Level 3 演算に最適化される
6. The Global Solver shall `SystemOfEquations` トレイトを通じて任意の連立方程式システムを受け入れる

### Requirement 2: カーブキャリブレーション問題定義

**Objective:** As a クオンツ開発者, I want カーブ構築問題を残差関数 F(x) - m = 0 として定義する, so that グローバルソルバーでカーブパラメータを同時推定できる.

#### Acceptance Criteria

1. The Calibration Problem shall パラメータ x をカーブのピラー値（Zero Rate または log Discount Factor）として定義する
2. The Calibration Problem shall ターゲット m を市場レート（Swap Rate, Futures Price, Deposit Rate等）として受け入れる
3. The Calibration Problem shall 関数 F(x) を「現在のパラメータ x からカーブを構築し、各商品の理論価格を計算する」ものとして実装する
4. The Calibration Problem shall `SystemOfEquations` トレイトを実装し、`evaluate()` と `jacobian()` メソッドを提供する
5. When キャリブレーション商品リストが空の場合, the Calibration Problem shall `CalibrationError::NoInstruments` を返す
6. The Calibration Problem shall 商品数とパラメータ数が一致することを検証する

### Requirement 3: Jacobian行列の構築

**Objective:** As a クオンツ開発者, I want Jacobian行列を効率的に構築する, so that ソルバーの収束性とAAD統合が実現できる.

#### Acceptance Criteria

1. The Jacobian Builder shall 各商品の各ピラーに対する感応度 ∂F_i/∂x_j を計算する
2. The Jacobian Builder shall 解析的微分（行列積 A·diag(-D)·W）または数値微分を選択可能とする
3. Where Enzyme AD が有効な場合, the Jacobian Builder shall AADによる自動微分でJacobianを構築する
4. The Jacobian Builder shall 補間行列 W を用いてピラーサイズに縮約されたJacobianを構築する
5. The Jacobian Builder shall キャッシュフロー行列 A をマーケットデータ不変の定数行列としてキャッシュする

### Requirement 4: AAD統合（陰関数定理）

**Objective:** As a リスク担当者, I want 収束点におけるJacobian逆行列を通じて市場データへの感応度を計算する, so that ソルバー反復を微分せずに効率的なリスク計算ができる.

#### Acceptance Criteria

1. The AAD Integration shall 陰関数定理 ∂x*/∂m = J⁻¹ を用いて市場パラメータへの感応度を計算する
2. The AAD Integration shall ソルバー結果に含まれる jacobian_inv を再利用し、反復計算のトレースを回避する
3. The AAD Integration shall カーブ感応度（DV01、Key Rate Duration）を行列演算として計算する
4. When Enzyme AD が有効な場合, the AAD Integration shall カスタム微分ルール（Shadow Function）を定義する

### Requirement 5: OIS/SOFR商品のテレスコープ法

**Objective:** As a クオンツ開発者, I want OIS/SOFR商品でテレスコープ法を使用する, so that 日次の巨大行列を回避し、効率的なカーブ構築ができる.

#### Acceptance Criteria

1. The Telescoping Evaluator shall OIS変動脚を DF(t_start)/DF(t_end) - 1 として計算する
2. The Telescoping Evaluator shall 日次のオーバーナイトレートループを使用せず、始点・終点のみに依存する
3. The Telescoping Evaluator shall Payment Delay を考慮し、支払日の Discount Factor で割引する
4. The Telescoping Evaluator shall 商品あたり2〜3個の非ゼロ要素を持つスパースなJacobian行を生成する
5. While Single Curve Framework の場合, the Telescoping Evaluator shall 完全なテレスコープ簡約化を適用する

### Requirement 6: Deposit/Futures商品サポート

**Objective:** As a クオンツ開発者, I want Deposit と Futures を効率的にカーブ構築に使用する, so that 短期カーブの構築が可能になる.

#### Acceptance Criteria

1. The Deposit Evaluator shall 1 = (1 + r·δ)·DF(t_mat) の関係からインプライドレートを計算する
2. The Deposit Evaluator shall 満期日の Discount Factor のみに依存し、Jacobian行に1要素を持つ
3. The Futures/FRA Evaluator shall Forward Rate = (DF(t_start)/DF(t_end) - 1)/δ を計算する
4. The Futures/FRA Evaluator shall 開始日と終了日の2点に依存し、Jacobian行に2要素を持つ
5. Where Convexity Adjustment が必要な場合, the Futures Evaluator shall 調整項を加算する

### Requirement 7: スワップ商品サポート

**Objective:** As a クオンツ開発者, I want 金利スワップをグローバルソルバーに統合する, so that 長期カーブの構築が可能になる.

#### Acceptance Criteria

1. The Swap Evaluator shall 固定脚 PV と変動脚 PV の差をゼロにするパーレートを計算する
2. The Swap Evaluator shall 複数の支払日に対応するキャッシュフロー行列を構築する
3. The Swap Evaluator shall 各キャッシュフロー日付を GlobalTimeGrid 上のインデックスにマッピングする
4. When OIS スワップの場合, the Swap Evaluator shall テレスコープ法を適用する

### Requirement 8: 時間グリッドと行列構築

**Objective:** As a クオンツ開発者, I want 全商品のユニークな日付リストから行列を構築する, so that 効率的な行列演算が可能になる.

#### Acceptance Criteria

1. The Time Grid Builder shall 全キャリブレーション商品からキャッシュフロー日付を収集する
2. The Time Grid Builder shall 日付をソートし、重複を除去した GlobalTimeGrid を生成する
3. The Matrix Builder shall 商品ごとに GlobalTimeGrid 上のインデックスを特定し、キャッシュフロー行列 A を構築する
4. The Matrix Builder shall 補間行列 W をピラー数 × 日付数のサイズで構築する
5. The Matrix Builder shall A と W を定数行列としてキャッシュし、反復ごとの再計算を回避する

### Requirement 9: エラーハンドリング

**Objective:** As a システム運用者, I want 明確なエラーメッセージと診断情報を得る, so that キャリブレーション失敗時に原因を特定できる.

#### Acceptance Criteria

1. If Jacobian行列が特異（Singular）の場合, the Global Solver shall `CalibrationError::SingularJacobian` を返す
2. If 初期値が適切でなく発散する場合, the Global Solver shall `CalibrationError::Divergence` を返す
3. The Global Solver shall 各反復での残差ノルムをログ出力する（デバッグモード時）
4. If 商品の理論価格計算に失敗した場合, the Calibration Problem shall 該当商品を特定するエラーを返す
5. The Global Solver shall 収束に要した反復回数と最終残差を結果に含める

### Requirement 10: 設定とカスタマイズ

**Objective:** As a クオンツ開発者, I want ソルバーのパラメータを柔軟に設定する, so that 異なるカーブ構築シナリオに対応できる.

#### Acceptance Criteria

1. The Solver Config shall 許容誤差（tolerance）をデフォルト 1e-10 で設定可能とする
2. The Solver Config shall 最大反復回数（max_iterations）をデフォルト 100 で設定可能とする
3. The Solver Config shall Jacobian計算方法（Analytical, FiniteDifference, AAD）を選択可能とする
4. The Solver Config shall 線形代数バックエンド（nalgebra, ndarray-linalg）を選択可能とする
5. The Solver Config shall Builder パターンで構成され、メソッドチェーンで設定できる
