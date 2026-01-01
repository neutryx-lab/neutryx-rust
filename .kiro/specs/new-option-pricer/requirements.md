# Requirements Document

## Project Description (Input)

新しいオプションプライサー - 新しい金融派生商品のプライシング機能を追加

## Introduction

本仕様は、neutryx-rust XVAプライシングライブラリに新しいオプションプライシング機能を追加するための要件を定義する。既存のインフラストラクチャ（pricer_models/analytical、pricer_models/models）を活用し、未実装のBlack-Scholes解析解、インプライドボラティリティソルバー、Hestonモデル、およびバスケットオプションを実装する。

## Requirements

### Requirement 1: Black-Scholes解析プライシング

**Objective:** As a クオンツアナリスト, I want ヨーロピアンバニラオプションの解析解プライシング機能, so that モンテカルロを使わずに高速かつ正確な価格計算ができる

#### Acceptance Criteria

1. When ヨーロピアンコールオプションの価格計算が要求された場合, the Black-Scholes Pricer shall Black-Scholes公式に基づき価格を返す
2. When ヨーロピアンプットオプションの価格計算が要求された場合, the Black-Scholes Pricer shall Put-Call Parityと整合した価格を返す
3. The Black-Scholes Pricer shall スポット価格、ストライク、満期、無リスク金利、ボラティリティを入力として受け取る
4. The Black-Scholes Pricer shall Float型ジェネリック（f64およびDual64）をサポートしAD互換性を維持する
5. If 満期が0以下の場合, the Black-Scholes Pricer shall イントリンシック価値を返す
6. The Black-Scholes Pricer shall 解析的ギリシャ（Delta, Gamma, Vega, Theta, Rho）を計算する関数を提供する

### Requirement 2: インプライドボラティリティソルバー

**Objective:** As a マーケットデータエンジニア, I want 市場価格からインプライドボラティリティを逆算する機能, so that ボラティリティサーフェスのキャリブレーションができる

#### Acceptance Criteria

1. When オプション市場価格とパラメータが与えられた場合, the IV Solver shall Newton-Raphson法でインプライドボラティリティを求める
2. The IV Solver shall 収束許容誤差（デフォルト1e-8）と最大反復回数（デフォルト100）を設定可能とする
3. If Newton-Raphsonが収束しない場合, the IV Solver shall Brent法にフォールバックする
4. If 価格がアービトラージ境界外の場合, the IV Solver shall PricingError::ArbitrageBoundsViolationを返す
5. The IV Solver shall pricer_core::math::solvers の既存ソルバーインフラを活用する
6. When 複数のオプション価格が与えられた場合, the IV Solver shall 並列処理でバッチIV計算を実行する

### Requirement 3: Hestonストキャスティックボラティリティモデル

**Objective:** As a デリバティブトレーダー, I want ボラティリティスマイルを再現できるHestonモデル, so that エキゾチックオプションのより現実的な価格計算ができる

#### Acceptance Criteria

1. The Heston Model shall StochasticModelトレイトを実装し、GBMと同じインターフェースで使用可能とする
2. The Heston Model shall 5パラメータ（κ: 平均回帰速度、θ: 長期分散、σ: ボラティリティのボラティリティ、ρ: 相関、v0: 初期分散）を受け取る
3. When evolve_stepが呼ばれた場合, the Heston Model shall Milstein離散化でスポットと分散を同時に更新する
4. The Heston Model shall TwoFactorState<T>を状態型として使用する
5. While 分散が負になりそうな場合, the Heston Model shall Full Truncationスキームで非負性を保証する
6. The Heston Model shall Feller条件（2κθ > σ²）のバリデーションを提供する
7. Where Enzyme ADが有効な場合, the Heston Model shall AD互換のsmooth approximationを使用する

### Requirement 4: バスケットオプション

**Objective:** As a ポートフォリオマネージャー, I want 複数資産のバスケットオプションプライシング, so that ポートフォリオレベルのヘッジ戦略を評価できる

#### Acceptance Criteria

1. The Basket Option shall 複数資産の加重平均に対するコール/プットペイオフを計算する
2. The Basket Option shall 各資産の重み（weights）とスポット価格のベクトルを受け取る
3. When モンテカルロでプライシングする場合, the Monte Carlo Pricer shall 相関行列を使用してコリレーテッドパスを生成する
4. The Basket Option shall PathDependentPayoffトレイトを実装し、既存のMCインフラと統合する
5. If 相関行列が正定値でない場合, the Basket Option shall PricingError::InvalidCorrelationMatrixを返す
6. The Basket Option shall Cholesky分解を使用して相関付きブラウン運動を生成する
7. Where 資産数が2の場合, the Basket Option shall スプレッドオプション（asset1 - asset2）もサポートする

### Requirement 5: テストとベンチマーク

**Objective:** As a 開発者, I want 新機能の正確性とパフォーマンスを検証する手段, so that プロダクション品質のコードを維持できる

#### Acceptance Criteria

1. The Test Suite shall Black-Scholes価格を既知の解析解と照合する
2. The Test Suite shall IV Solverのラウンドトリップテスト（BS価格→IV→BS価格）を含む
3. The Test Suite shall Hestonモデルを特殊ケース（σ=0でGBMに帰着）でGBMと比較する
4. The Test Suite shall バスケットオプションを単一資産ケースでバニラオプションと比較する
5. When Enzyme ADが有効な場合, the Verification Tests shall 解析ギリシャとADギリシャを比較する
6. The Benchmark Suite shall 新機能のcriterionベンチマークを追加する
