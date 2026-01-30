# Requirements Document

## Project Description (Input)
service_gateway services層の充実化 - pricer_risk/pricer_pricing/pricer_models の機能をREST APIとして公開。RiskService（Greeks計算、シナリオ分析）、PortfolioService（Portfolio CRUD）、ModelService（確率モデル設定）、VolatilityService（Vol surface操作）を追加。メンテナンス性を重視し、feature flags、エラードメイン分離、一貫したパターンを採用。

## Introduction

本仕様は service_gateway crate の services 層を拡充し、pricer_* crates の豊富な機能を REST API として公開することを目的とする。現状の CurveService/PricingService に加え、RiskService、PortfolioService、ModelService、VolatilityService を追加する。

設計原則として以下を重視する：
- **一貫性**: 全サービスで同一パターン（Handler → Service → Pricer crate）
- **拡張性**: Feature flags による選択的コンパイル
- **保守性**: ドメインごとのエラー型分離、単体テスト容易性
- **A-I-P-S準拠**: Service層からPricer層への正しい依存方向

## Requirements

### Requirement 1: RiskService - Greeks計算機能

**Objective:** As a API利用者, I want REST API経由でGreeks（Delta, Gamma, Vega, Theta, Rho）を計算したい, so that フロントエンドやバッチ処理から一貫したリスク指標を取得できる

#### Acceptance Criteria
1. When Greeks計算リクエストを受信した場合, the RiskService shall pricer_risk::RiskEngine を使用してGreeksを計算し結果を返却する
2. When compute_greeks が GreeksMode::BumpAndRevalue を指定された場合, the RiskService shall bump-and-revalue方式でGreeksを計算する
3. Where enzyme-ad feature が有効な場合, the RiskService shall GreeksMode::EnzymeAAD による高速計算をサポートする
4. If Greeks計算中にエラーが発生した場合, the RiskService shall RiskError を ServerError::Risk に変換して返却する
5. The RiskService shall 計算時間（calculation_time_ms）をレスポンスに含める

### Requirement 2: RiskService - シナリオ分析機能

**Objective:** As a リスクマネージャー, I want シナリオ分析（ストレステスト）をAPI経由で実行したい, so that 市場変動に対するポートフォリオ感応度を評価できる

#### Acceptance Criteria
1. When シナリオ分析リクエストを受信した場合, the RiskService shall pricer_risk::scenarios::ScenarioEngine を使用してシナリオP&Lを計算する
2. When PresetScenario（定義済みシナリオ）が指定された場合, the RiskService shall プリセット定義に基づくシフトを適用する
3. When BumpScenario（カスタムシナリオ）が指定された場合, the RiskService shall ユーザー定義のRiskFactorShiftを適用する
4. The RiskService shall 各シナリオの結果（base_value, scenario_value, pnl）を配列形式で返却する
5. If シナリオ定義が不正な場合, the RiskService shall ServerError::InvalidRequest を返却する

### Requirement 3: PortfolioService - Portfolio CRUD機能

**Objective:** As a トレーダー, I want ポートフォリオの作成・取得・更新・削除をAPI経由で行いたい, so that 取引ポジションを管理できる

#### Acceptance Criteria
1. When POST /api/v1/portfolios リクエストを受信した場合, the PortfolioService shall 新規ポートフォリオを作成しIDを返却する
2. When GET /api/v1/portfolios/{id} リクエストを受信した場合, the PortfolioService shall キャッシュからポートフォリオを取得して返却する
3. When PUT /api/v1/portfolios/{id}/trades リクエストを受信した場合, the PortfolioService shall 指定ポートフォリオにトレードを追加する
4. When DELETE /api/v1/portfolios/{id} リクエストを受信した場合, the PortfolioService shall 指定ポートフォリオをキャッシュから削除する
5. If 指定されたportfolio_idが存在しない場合, the PortfolioService shall ServerError::NotFound を返却する
6. The PortfolioService shall ポートフォリオを AppState 内の LRUキャッシュに保持する

### Requirement 4: PortfolioService - Portfolio集計機能

**Objective:** As a リスクマネージャー, I want ポートフォリオ全体の価値やGreeksを一括計算したい, so that ポジション全体のリスクを把握できる

#### Acceptance Criteria
1. When ポートフォリオ価格計算リクエストを受信した場合, the PortfolioService shall 全トレードの現在価値を合計して返却する
2. When ポートフォリオGreeks計算リクエストを受信した場合, the PortfolioService shall pricer_risk::RiskEngine::compute_portfolio_greeks を使用して集約Greeksを計算する
3. When Counterparty別集計が要求された場合, the PortfolioService shall NettingSetごとにエクスポージャーを集約する
4. The PortfolioService shall 成功/失敗トレード数を結果に含める
5. If 一部トレードの計算に失敗した場合, the PortfolioService shall 失敗トレードの詳細をerror配列に含めて返却する

### Requirement 5: ModelService - 確率モデル設定機能

**Objective:** As a クオンツ, I want 確率モデル（GBM, Heston, Hull-White等）をAPI経由で設定・照会したい, so that 価格計算に使用するモデルを柔軟に選択できる

#### Acceptance Criteria
1. When モデル設定リクエストを受信した場合, the ModelService shall pricer_models::stochastic のモデルインスタンスを生成しキャッシュに保存する
2. The ModelService shall GBM, Heston, HullWhite, CIR, SABR モデルをサポートする
3. When モデルパラメータが不正な場合, the ModelService shall StochasticModelError を ServerError::InvalidRequest に変換して返却する
4. When GET /api/v1/models/{id} リクエストを受信した場合, the ModelService shall モデル設定の詳細を返却する
5. The ModelService shall モデルパラメータのバリデーション結果をレスポンスに含める

### Requirement 6: ModelService - モデルベース価格計算機能

**Objective:** As a クオンツ, I want 設定済みモデルを使用してオプション価格を計算したい, so that 異なるモデル間で結果を比較できる

#### Acceptance Criteria
1. When モデルベース価格計算リクエストを受信した場合, the ModelService shall 指定モデルIDのモデルを使用して価格を計算する
2. When Monte Carlo pricing が指定された場合, the ModelService shall pricer_pricing::mc::MonteCarloPricer を使用する
3. When Tree pricing が指定された場合, the ModelService shall pricer_pricing::tree::TreeMethod を使用する
4. The ModelService shall 計算手法（analytical, monte_carlo, tree）をレスポンスに含める
5. If 指定されたmodel_idが存在しない場合, the ModelService shall ServerError::NotFound を返却する

### Requirement 7: VolatilityService - Vol Surface操作機能

**Objective:** As a クオンツ, I want ボラティリティサーフェスの構築・照会をAPI経由で行いたい, so that オプション価格計算に必要なボラティリティデータを管理できる

#### Acceptance Criteria
1. When FX Vol Surface構築リクエストを受信した場合, the VolatilityService shall pricer_models::builder::vol::FxVolBuilder を使用してサーフェスを構築する
2. When Vol Cube構築リクエストを受信した場合, the VolatilityService shall pricer_models::builder::vol::VolCubeBuilder を使用してキューブを構築する
3. When implied volatility照会リクエストを受信した場合, the VolatilityService shall 指定expiry/strikeの補間ボラティリティを返却する
4. The VolatilityService shall SABR calibration結果（alpha, beta, rho, nu）をレスポンスに含める
5. If キャリブレーションが収束しなかった場合, the VolatilityService shall ServerError::Calibration を返却し残差を含める

### Requirement 8: Feature Flags による選択的コンパイル

**Objective:** As a ライブラリ利用者, I want 必要な機能のみを選択的にコンパイルしたい, so that バイナリサイズとビルド時間を最適化できる

#### Acceptance Criteria
1. The service_gateway shall default feature として rest を有効にする
2. Where risk feature が有効な場合, the service_gateway shall RiskService と PortfolioService をコンパイルに含める
3. Where models feature が有効な場合, the service_gateway shall ModelService をコンパイルに含める
4. Where volatility feature が有効な場合, the service_gateway shall VolatilityService をコンパイルに含める
5. The service_gateway shall feature無効時に対応するハンドラーを登録しない

### Requirement 9: Error Domain分離

**Objective:** As a 開発者, I want ドメインごとに分離されたエラー型を使用したい, so that エラー原因の特定とデバッグが容易になる

#### Acceptance Criteria
1. The service_gateway shall RiskError, PortfolioError, ModelError, VolatilityError の4つのドメインエラー型を定義する
2. The service_gateway shall 各ドメインエラーに #[from] 変換を実装し、pricer_* cratesのエラーから自動変換する
3. When ドメインエラーが発生した場合, the service_gateway shall ServerError に変換してHTTPレスポンスを生成する
4. The ServerError shall 各ドメインに対応するvariant（Risk, Portfolio, Model, Volatility）を持つ
5. The service_gateway shall エラーレスポンスにerror_codeとdetailsを含める

### Requirement 10: 一貫したサービスパターン

**Objective:** As a 開発者, I want 全サービスで一貫したコード構造を使用したい, so that 新規サービス追加とコードレビューが効率的になる

#### Acceptance Criteria
1. The service_gateway shall 各サービスに対応する rest/dto/{domain}.rs を配置する
2. The service_gateway shall 各サービスに対応する rest/handlers/{domain}.rs を配置する
3. The service_gateway shall Handlerはビジネスロジックを含まず、Serviceメソッドへの委譲のみを行う
4. The service_gateway shall 各Serviceは `fn operation(request: &Request, state: &Arc<AppState>) -> Result<Response, Error>` のシグネチャに従う
5. The service_gateway shall 各Serviceに #[cfg(test)] mod tests を含める

### Requirement 11: AppState拡張

**Objective:** As a 開発者, I want 新サービス用のキャッシュをAppStateに追加したい, so that 計算結果とモデル設定を効率的に再利用できる

#### Acceptance Criteria
1. The AppState shall PortfolioCache（ポートフォリオキャッシュ）を保持する
2. The AppState shall ModelCache（確率モデルキャッシュ）を保持する
3. The AppState shall VolSurfaceCache（ボラティリティサーフェスキャッシュ）を保持する
4. The AppState shall キャッシュサイズをコンストラクタで設定可能にする
5. The AppState shall parking_lot::RwLock による並行アクセスをサポートする

### Requirement 12: API バージョニング

**Objective:** As a API利用者, I want APIバージョンを明示的に指定したい, so that 将来の破壊的変更から保護される

#### Acceptance Criteria
1. The service_gateway shall 全エンドポイントを /api/v1/ プレフィックス配下に配置する
2. The service_gateway shall レスポンスヘッダーに X-API-Version を含める
3. The service_gateway shall OpenAPI仕様でAPIバージョンを記載する
4. If 未サポートのAPIバージョンが指定された場合, the service_gateway shall 400 Bad Request を返却する

### Requirement 13: Demo GUI 統合

**Objective:** As a 開発者, I want service_gateway が demo_gui のフロントエンドと API を統合して提供したい, so that 単一サーバーで完結したデモ環境を構築できる

#### Acceptance Criteria
1. The service_gateway shall /api/curves/* エンドポイント群（indices, instruments, build）を提供する
2. The service_gateway shall /api/volcube/* エンドポイント群（indices, models, instruments, calibrate）を提供する
3. The service_gateway shall /api/fxvol/* エンドポイント群（pairs, quotes）を提供する
4. The service_gateway shall /api/market/* エンドポイント群（rates/refresh, export）を提供する
5. The service_gateway shall demo/gui/static/ ディレクトリから静的ファイルを配信する
6. Where demo feature が有効な場合, the service_gateway shall DemoService をコンパイルに含める

### Requirement 14: 静的ファイル配信

**Objective:** As a フロントエンド開発者, I want service_gateway から直接 React アプリを配信したい, so that 開発・デモ時に追加のサーバー設定が不要になる

#### Acceptance Criteria
1. The service_gateway shall / ルートで index.html を配信する
2. The service_gateway shall /static/* 配下で CSS/JS/アセットを配信する
3. The service_gateway shall SPA フォールバック（存在しないパスは index.html へリダイレクト）を実装する
4. If 静的ファイルが見つからない場合, the service_gateway shall 404 Not Found を返却する
5. The service_gateway shall 適切な Content-Type ヘッダーを設定する

### Requirement 15: Demo GUI 依存制約

**Objective:** As a アーキテクト, I want demo_gui が service_gateway のみを参照する, so that A-I-P-S アーキテクチャを維持し依存方向を一貫させる

#### Acceptance Criteria
1. The demo_gui shall Cargo.toml で service_gateway のみを依存に持つ（pricer_*/infra_*/adapter_* を直接参照しない）
2. The service_gateway shall demo_gui が必要とする全ての機能をファサードとして公開する
3. The demo_gui shall pricer_* crates の型を直接使用しない（service_gateway の DTO を使用する）
4. The service_gateway shall demo feature 配下で demo_gui 用 API を完結させる
5. If demo_gui が新機能を必要とする場合, the service_gateway shall 対応する Service/Handler を追加する

## Non-Functional Requirements

### NFR-1: パフォーマンス
- 単一トレード価格計算: < 10ms（analytical）、< 100ms（MC 10,000 paths）
- ポートフォリオ計算（100トレード）: < 1秒
- Greeks計算（bump-and-revalue）: < 500ms

### NFR-2: 並行性
- 全Serviceはスレッドセーフであること
- AppState内キャッシュはparking_lot::RwLockで保護
- Handlerは async/await パターンに従う

### NFR-3: テストカバレッジ
- 各Serviceのunit testカバレッジ: > 80%
- integration tests: 主要エンドポイントの正常系・異常系

## Glossary

| 用語 | 説明 |
|------|------|
| Greeks | オプション価格の感応度指標（Delta, Gamma, Vega, Theta, Rho） |
| シナリオ分析 | リスクファクターを変動させた場合のP&L計算 |
| Vol Surface | 満期・ストライクに対するインプライドボラティリティの2次元補間面 |
| Vol Cube | 満期・テナー・ストライクに対する3次元ボラティリティ構造 |
| SABR | Stochastic Alpha Beta Rho、ボラティリティスマイルモデル |
| Netting Set | 相殺可能なトレードのグループ |
