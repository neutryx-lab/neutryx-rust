# Requirements Document

## Introduction

本仕様は `pricer_pricing` クレート（L3層）のアーキテクチャ再設計を定義する。中心に Pricer を配置し、`pricer_models`（L2層）の機能を活用して統一された `PricingResult` を生成する構造を確立する。設定ファイルを基に、Discount（解析的）、Monte Carlo、Tree の各プライシング手法が選択・実行される明確な階層構造を実現する。

**設計原則**:
- **Pricer中心アーキテクチャ**: Pricer が全てのプライシング手法を統括
- **設定駆動型**: 設定ファイルによるプライシング手法の選択
- **L2層との明確な連携**: `pricer_models` のモデル・マーケットデータを活用
- **A-I-P-S依存規則の遵守**: Pricer層はService/Adapter層に依存しない

## Requirements

### Requirement 1: Pricerコア構造

**Objective:** As a クオンツ開発者, I want 統一されたPricerインターフェースを通じて全てのプライシング手法を呼び出せる, so that プライシングロジックの一貫性と拡張性を確保できる

#### Acceptance Criteria
1. The Pricer shall プライシング手法（Discount/MC/Tree）を抽象化したトレイトを提供する
2. When プライシングリクエストを受信した場合, the Pricer shall 設定に基づいて適切なプライシング手法を選択する
3. The Pricer shall 全てのプライシング手法で共通の `PricingResult<T>` 型を返却する
4. While プライシング実行中, the Pricer shall プログレス情報をオプションで提供可能とする
5. If プライシング手法が未サポートの場合, the Pricer shall 明確なエラー型 `PricingError::UnsupportedMethod` を返却する

### Requirement 2: 設定駆動型アーキテクチャ

**Objective:** As a クオンツ開発者, I want 設定ファイルからプライシングパラメータを読み込み手法を選択できる, so that コード変更なしにプライシング設定を調整できる

#### Acceptance Criteria
1. The PricingConfig shall プライシング手法（Discount/MC/Tree）の選択を設定可能とする
2. The PricingConfig shall 手法固有のパラメータ（MC: パス数、Tree: ステップ数）を設定可能とする
3. When 設定が不完全または不正な場合, the PricingConfig shall `ConfigError` を返却する
4. The PricingConfig shall デフォルト値を提供し、最小限の設定で動作可能とする
5. Where Enzymeが有効な場合, the PricingConfig shall AAD（Adjoint AD）オプションを設定可能とする

### Requirement 3: Discountプライシング手法

**Objective:** As a クオンツ開発者, I want 解析的プライシング（Discount手法）を実行できる, so that 高速で正確なプライシングを実現できる

#### Acceptance Criteria
1. The DiscountPricer shall `pricer_models::market::curves` のYieldCurveを用いて現在価値を計算する
2. When キャッシュフローを評価する場合, the DiscountPricer shall Day Count Conventionを正しく適用する
3. The DiscountPricer shall `pricer_models::analytical` のBlack-Scholes、Garman-Kohlhagen解を呼び出し可能とする
4. If カーブデータが不足している場合, the DiscountPricer shall `PricingError::MissingMarketData` を返却する
5. The DiscountPricer shall 解析的Greeks（Delta、Gamma、Vega、Theta）を計算可能とする

### Requirement 4: Monte Carloプライシング手法

**Objective:** As a クオンツ開発者, I want Monte Carloシミュレーションによるプライシングを実行できる, so that パス依存型商品や複雑なペイオフを評価できる

#### Acceptance Criteria
1. The MonteCarloMethod shall `pricer_models::models` のStochasticModelを用いてパスを生成する
2. The MonteCarloMethod shall 設定されたパス数・タイムステップでシミュレーションを実行する
3. When パス依存型オプション（Asian/Barrier/Lookback）を評価する場合, the MonteCarloMethod shall `path_dependent` モジュールのPayoffトレイトを使用する
4. The MonteCarloMethod shall `rng` モジュールのPRNG/QMCシーケンスを使用可能とする
5. While メモリ制約がある場合, the MonteCarloMethod shall `checkpoint` モジュールを用いてメモリ使用量を制御する
6. Where Enzymeが有効な場合, the MonteCarloMethod shall `enzyme` モジュールによる自動微分でGreeksを計算する

### Requirement 5: Treeプライシング手法

**Objective:** As a クオンツ開発者, I want Treeベースのプライシングを実行できる, so that アメリカン・オプションや早期行使機能を持つ商品を評価できる

#### Acceptance Criteria
1. The TreeMethod shall Binomial Tree、Trinomial Treeの実装を提供する
2. When アメリカン・オプションを評価する場合, the TreeMethod shall 各ノードで早期行使判定を実行する
3. The TreeMethod shall ツリーのステップ数を設定可能とする
4. If 収束しない場合, the TreeMethod shall `PricingError::ConvergenceFailed` を返却する
5. The TreeMethod shall Treeベースの解析的Greeks（Delta、Gamma）を計算可能とする

### Requirement 6: PricingResult統一構造

**Objective:** As a クオンツ開発者, I want 全てのプライシング手法で統一された結果構造を受け取れる, so that 手法に依存しない後続処理を実装できる

#### Acceptance Criteria
1. The PricingResult shall 価格（PV）、使用した手法、計算時間を含む
2. The PricingResult shall オプションでGreeks（Delta、Gamma、Vega、Theta、Rho）を含む
3. The PricingResult shall ADジェネリクス `PricingResult<T: Float>` をサポートする
4. Where Monte Carlo手法の場合, the PricingResult shall 標準誤差（Standard Error）を含む
5. The PricingResult shall 計算に使用したマーケットデータのスナップショット参照を保持可能とする

### Requirement 7: pricer_modelsとの統合

**Objective:** As a クオンツ開発者, I want pricer_models（L2層）の機能をシームレスに利用できる, so that モデルとマーケットデータの再実装を避けられる

#### Acceptance Criteria
1. The Pricer shall `pricer_models::market::provider::MarketProvider` を通じてマーケットデータを取得する
2. The Pricer shall `pricer_models::models::StochasticModelEnum` をMonte Carloで使用可能とする
3. When カーブが必要な場合, the Pricer shall `pricer_models::market::curves::CurveEnum` を使用する
4. When ボラティリティサーフェスが必要な場合, the Pricer shall `pricer_models::market::surfaces::VolSurfaceEnum` を使用する
5. The Pricer shall `infra_domain::trade::PricingInstrument` を入力として受け付ける

### Requirement 8: 商品定義とPricer連携

**Objective:** As a クオンツ開発者, I want infra_domainの商品定義をPricerで直接利用できる, so that 商品定義とプライシングの一貫性を確保できる

#### Acceptance Criteria
1. The Pricer shall `infra_domain::trade::pricing_instrument::PricingInstrument` 型を受け付ける
2. When VanillaOptionを受信した場合, the Pricer shall 設定に基づいてDiscount/MC/Tree手法を選択する
3. When Forwardを受信した場合, the Pricer shall Discount手法を優先的に使用する
4. The Pricer shall `infra_domain::instrument_def` の標準商品定義との互換性を持つ
5. If 未対応の商品タイプの場合, the Pricer shall `PricingError::UnsupportedInstrument` を返却する

### Requirement 9: エラーハンドリング

**Objective:** As a クオンツ開発者, I want 明確で構造化されたエラーを受け取れる, so that 問題の診断と対処が容易になる

#### Acceptance Criteria
1. The PricingError shall `thiserror` を使用して構造化エラーを定義する
2. The PricingError shall 以下のバリアントを含む: `UnsupportedMethod`, `UnsupportedInstrument`, `MissingMarketData`, `ConfigError`, `ConvergenceFailed`, `NumericalInstability`
3. When エラーが発生した場合, the Pricer shall コンテキスト情報（商品ID、手法名）を含める
4. The PricingError shall `pricer_core::types::error` のエラー型と互換性を持つ
5. If Monte Carlo収束エラーの場合, the PricingError shall 到達した収束レベルと必要な精度を含める

### Requirement 10: モジュール構造

**Objective:** As a クオンツ開発者, I want 明確なモジュール構造を持つ, so that コードの理解と保守が容易になる

#### Acceptance Criteria
1. The pricer_pricing shall 以下のモジュール構造を持つ: `pricer/`, `methods/`, `config/`, `result/`
2. The `pricer/` shall 中央のPricer抽象と実装を含む
3. The `methods/` shall `discount/`, `mc/`, `tree/` サブモジュールを含む
4. The `config/` shall PricingConfig、MethodConfig、各手法固有の設定を含む
5. The `result/` shall PricingResult、Greeks、メタデータ型を含む
6. The pricer_pricing shall 既存の `enzyme/`, `rng/`, `checkpoint/`, `path_dependent/` モジュールを保持する
