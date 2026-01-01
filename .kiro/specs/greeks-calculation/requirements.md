# Requirements Document

## Introduction

本仕様は、デリバティブのリスク感応度指標（Greeks）を計算する機能を定義する。既存のMonte Carloプライシングエンジン（pricer_kernel）およびEnzyme自動微分基盤を活用し、Delta、Gamma、Vega、Theta、Rhoの計算を実現する。Bump-and-Revalue法とAAD（Adjoint Automatic Differentiation）の両方をサポートし、num-dualモードによる検証機能も提供する。

## Requirements

### Requirement 1: 一次Greeks計算（Delta, Vega, Theta, Rho）

**Objective:** As a クオンツ開発者, I want オプション価格の一次リスク感応度を計算する機能, so that ヘッジポジションの構築やリスク管理に活用できる

#### Acceptance Criteria 1

1. When ユーザーがDelta計算をリクエストした場合, the Greeks Engine shall 原資産価格に対する価格感応度を数値として返却する
2. When ユーザーがVega計算をリクエストした場合, the Greeks Engine shall インプライドボラティリティに対する価格感応度を数値として返却する
3. When ユーザーがTheta計算をリクエストした場合, the Greeks Engine shall 残存期間に対する価格感応度（時間減衰）を数値として返却する
4. When ユーザーがRho計算をリクエストした場合, the Greeks Engine shall 無リスク金利に対する価格感応度を数値として返却する
5. The Greeks Engine shall すべての一次Greeksを`T: Float`ジェネリック型で計算し、AD互換性を維持する

### Requirement 2: 二次Greeks計算（Gamma, Vanna, Volga）

**Objective:** As a リスク管理者, I want オプション価格の二次リスク感応度を計算する機能, so that 非線形リスクの把握とガンマヘッジに活用できる

#### Acceptance Criteria 2

1. When ユーザーがGamma計算をリクエストした場合, the Greeks Engine shall Deltaの原資産価格に対する感応度（二階微分）を数値として返却する
2. When ユーザーがVanna計算をリクエストした場合, the Greeks Engine shall Deltaのボラティリティに対する感応度（クロス二階微分）を数値として返却する
3. When ユーザーがVolga計算をリクエストした場合, the Greeks Engine shall Vegaのボラティリティに対する感応度（二階微分）を数値として返却する
4. If 二階微分計算において数値安定性の問題が発生した場合, the Greeks Engine shall smooth近似関数を使用して連続的な結果を保証する

### Requirement 3: Bump-and-Revalue法による計算

**Objective:** As a プライシングエンジニア, I want 有限差分法によるGreeks計算, so that AAD未対応のペイオフでも感応度を計算できる

#### Acceptance Criteria 3

1. When Bump-and-Revalueモードが選択された場合, the Greeks Engine shall 中心差分法を使用して一次Greeksを計算する
2. When 二次Greeksが要求された場合, the Greeks Engine shall 三点差分法を使用してGammaを計算する
3. The Greeks Engine shall バンプ幅をユーザー設定可能なパラメータとして提供する（デフォルト: 0.01相対シフト）
4. While Monte Carloシミュレーション実行中, the Greeks Engine shall 同一乱数シードを使用してバンプ前後の計算を実行する

### Requirement 4: Enzyme AADによる計算

**Objective:** As a パフォーマンス重視のユーザー, I want LLVM最適化されたAADによる高速Greeks計算, so that 大規模ポートフォリオのリアルタイムリスク計算に活用できる

#### Acceptance Criteria 4

1. Where `enzyme-mode`フィーチャーが有効な場合, the Greeks Engine shall Enzyme ADを使用してGreeksを計算する
2. When Enzyme AADが使用される場合, the Greeks Engine shall Bump-and-Revalueと同等の精度を維持する
3. The Greeks Engine shall `#[autodiff]`マクロを使用して微分可能な関数を定義する
4. If Enzymeコンパイルエラーが発生した場合, the Greeks Engine shall 明確なエラーメッセージとフォールバック手段を提示する

### Requirement 5: num-dualモードによる検証

**Objective:** As a 品質保証エンジニア, I want Enzyme結果の検証手段, so that AD実装の正確性を確認できる

#### Acceptance Criteria 5

1. Where `num-dual-mode`フィーチャーが有効な場合, the Greeks Engine shall num-dual二重数によるフォワードモードADでGreeksを計算する
2. The Greeks Engine shall Enzyme結果とnum-dual結果の比較検証機能を提供する
3. When 検証モードが有効な場合, the Greeks Engine shall 両手法の結果差異をレポートする
4. The Greeks Engine shall 許容誤差閾値（デフォルト: 1e-6）をユーザー設定可能とする

### Requirement 6: Greeks結果の構造体

**Objective:** As a アプリケーション開発者, I want 統一されたGreeks結果フォーマット, so that 下流システムとの統合が容易になる

#### Acceptance Criteria 6

1. The Greeks Engine shall すべてのGreeksを含む`GreeksResult<T>`構造体を提供する
2. The Greeks Engine shall 一次Greeks（delta, vega, theta, rho）をグループ化する
3. The Greeks Engine shall 二次Greeks（gamma, vanna, volga）をオプショナルフィールドとして含める
4. When 計算されていないGreeksがアクセスされた場合, the Greeks Engine shall `None`または適切なデフォルト値を返却する
5. The Greeks Engine shall Serialize/Deserialize対応（serde feature有効時）を提供する

### Requirement 7: パス依存オプションのGreeks

**Objective:** As a エキゾチックオプショントレーダー, I want Asian/Barrier/Lookbackオプションのリスク感応度計算, so that エキゾチックポジションのヘッジが可能になる

#### Acceptance Criteria 7

1. When Asian オプションのGreeksが要求された場合, the Greeks Engine shall モンテカルロパスを使用してDelta/Vegaを計算する
2. When Barrierオプションの場合, the Greeks Engine shall smooth barrier近似を使用して連続的なGreeksを保証する
3. When LookbackオプションのGreeksが要求された場合, the Greeks Engine shall ストリーミング統計（最大値/最小値）を使用した感応度を計算する
4. The Greeks Engine shall `PathDependentPayoff`トレイトを実装したすべてのペイオフに対してGreeks計算をサポートする

### Requirement 8: ポートフォリオレベルGreeks

**Objective:** As a リスクマネージャー, I want ポートフォリオ全体の集約Greeks, so that ネットポジションのリスク把握ができる

#### Acceptance Criteria 8

1. When ポートフォリオGreeksが要求された場合, the Greeks Engine shall 個別取引のGreeksを集約して返却する
2. The Greeks Engine shall Rayonによる並列計算をサポートする
3. The Greeks Engine shall ネッティングセット単位でのGreeks集約機能を提供する
4. While 大規模ポートフォリオ処理中, the Greeks Engine shall SoA（Structure of Arrays）形式でメモリ効率的に計算する
