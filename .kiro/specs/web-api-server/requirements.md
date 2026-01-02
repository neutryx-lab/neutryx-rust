# Requirements Document

## Project Description (Input)
REST APIサーバー実装（pricer_server）- Axumを使用したプライシング計算のWeb API化。Layer 5として新規クレートを追加し、既存のL1-L4機能をHTTPエンドポイントとして公開。エンドポイント: オプション価格計算、ギリシャ計算、ポートフォリオXVA計算。JSON入出力、エラーハンドリング、OpenAPI仕様を含む。stable Rustで動作。

## 導入

本仕様書は、Neutryx XVA Pricing Libraryの既存機能をHTTP APIとして公開するためのWebサーバー実装（pricer_server）の要件を定義します。pricer_serverは、Layer 5として新規クレート（`pricer_server`）を追加し、Axumフレームワークを使用してRESTful APIを提供します。stable Rustで動作し、既存のL1-L4機能（pricer_core、pricer_models、pricer_kernel、pricer_xva）をHTTPエンドポイントとして公開します。

主要なエンドポイントは以下の通りです:
- オプション価格計算（バニラ、パス依存型）
- ギリシャ指標計算（デルタ、ガンマ、ベガ等）
- ポートフォリオレベルのXVA計算（CVA、DVA、FVA）

全てのエンドポイントはJSON形式の入出力をサポートし、適切なエラーハンドリングとOpenAPI仕様を提供します。

## Requirements

### Requirement 1: Axum Webサーバーの基本機能
**Objective:** プライシングライブラリの開発者として、Axumベースの安定したHTTPサーバーを構築し、既存のL1-L4機能をREST APIとして公開したい。その理由は、クライアントアプリケーションが計算機能にHTTP経由でアクセスできるようにするためである。

#### Acceptance Criteria
1. The pricer_server shall use stable Rust (Edition 2021) for all implementations
2. The pricer_server shall use Axum framework for HTTP routing and request handling
3. When the server starts, the pricer_server shall bind to a configurable host and port (default: 0.0.0.0:8080)
4. When the server receives an HTTP request, the pricer_server shall route the request to the appropriate handler based on path and method
5. The pricer_server shall support graceful shutdown on receiving SIGTERM or SIGINT signals
6. The pricer_server shall log startup, shutdown, and request information using structured logging (tracing crate)
7. The pricer_server shall respond within 30 seconds for all pricing calculation endpoints under normal load

### Requirement 2: JSON入出力とシリアライゼーション
**Objective:** APIクライアントの開発者として、標準的なJSON形式でリクエストとレスポンスをやり取りしたい。その理由は、広範なプログラミング言語やツールとの互換性を確保するためである。

#### Acceptance Criteria
1. When a client sends a request, the pricer_server shall accept JSON request bodies with Content-Type: application/json
2. When the pricer_server processes a request, the pricer_server shall serialize responses to JSON format using serde_json
3. The pricer_server shall use serde derive macros for all API request and response types
4. When serializing market data, the pricer_server shall support ISO 8601 format for dates and times
5. When serializing currency values, the pricer_server shall use ISO 4217 currency codes (leveraging pricer_core::types::currency)
6. If a client sends malformed JSON, then the pricer_server shall return HTTP 400 Bad Request with a descriptive error message
7. The pricer_server shall use camelCase naming convention for JSON field names (via serde rename)

### Requirement 3: オプション価格計算エンドポイント
**Objective:** 定量アナリストとして、HTTPリクエストを介してオプション価格を計算したい。その理由は、外部システムから既存のpricer_models機能にアクセスするためである。

#### Acceptance Criteria
1. When a client sends POST /api/v1/price/vanilla, the pricer_server shall calculate vanilla option prices using pricer_models analytical pricing
2. When a client sends POST /api/v1/price/asian, the pricer_server shall calculate Asian option prices using pricer_kernel Monte Carlo or analytical methods
3. When a client sends POST /api/v1/price/barrier, the pricer_server shall calculate barrier option prices using pricer_kernel Monte Carlo or analytical methods
4. When a client sends POST /api/v1/price/lookback, the pricer_server shall calculate lookback option prices using pricer_kernel Monte Carlo methods
5. When receiving pricing requests, the pricer_server shall accept parameters: option_type, spot, strike, maturity, risk_free_rate, volatility, and optional smoothing_epsilon
6. When a pricing calculation succeeds, the pricer_server shall return HTTP 200 OK with JSON body containing: price, calculation_method, and timestamp
7. If pricing calculation fails, then the pricer_server shall return HTTP 500 Internal Server Error with error details
8. When using Monte Carlo methods, the pricer_server shall accept optional simulation_params (num_paths, num_steps, seed) with sensible defaults

### Requirement 4: ギリシャ指標計算エンドポイント
**Objective:** リスク管理者として、オプションのギリシャ指標（デルタ、ガンマ、ベガ等）をHTTP経由で計算したい。その理由は、ポートフォリオのリスクエクスポージャーを評価するためである。

#### Acceptance Criteria
1. When a client sends POST /api/v1/greeks, the pricer_server shall calculate option Greeks using pricer_kernel AD capabilities
2. When receiving Greeks requests, the pricer_server shall accept the same option parameters as pricing endpoints plus greeks_config (which Greeks to compute)
3. When a Greeks calculation succeeds, the pricer_server shall return HTTP 200 OK with JSON body containing: delta, gamma, vega, theta, rho (as requested)
4. When using Enzyme-based Greeks, the pricer_server shall use bump-and-revalue methodology from pricer_kernel::greeks
5. If Greeks calculation fails due to numerical instability, then the pricer_server shall return HTTP 422 Unprocessable Entity with error details
6. The pricer_server shall compute Greeks for vanilla, Asian, barrier, and lookback options
7. When computing multiple Greeks, the pricer_server shall reuse simulation paths to minimize computation time

### Requirement 5: ポートフォリオXVA計算エンドポイント
**Objective:** 信用リスクアナリストとして、ポートフォリオレベルのXVA（CVA、DVA、FVA）をHTTP経由で計算したい。その理由は、取引相手信用リスクの評価を行うためである。

#### Acceptance Criteria
1. When a client sends POST /api/v1/xva/portfolio, the pricer_server shall calculate portfolio XVA metrics using pricer_xva functionality
2. When receiving XVA requests, the pricer_server shall accept portfolio definition (trades, netting sets, counterparty data) as JSON
3. When an XVA calculation succeeds, the pricer_server shall return HTTP 200 OK with JSON body containing: CVA, DVA, FVA, and exposure metrics (EE, EPE, PFE)
4. When processing large portfolios, the pricer_server shall leverage pricer_xva parallel computation (Rayon) for performance
5. If portfolio validation fails, then the pricer_server shall return HTTP 400 Bad Request with validation error details
6. When calculating exposure metrics, the pricer_server shall accept time_grid (evaluation dates) and simulation parameters
7. The pricer_server shall support counterparty-level XVA calculations via POST /api/v1/xva/counterparty endpoint

### Requirement 6: エラーハンドリングとバリデーション
**Objective:** API利用者として、明確で実行可能なエラーメッセージを受け取りたい。その理由は、問題の診断と修正を迅速に行うためである。

#### Acceptance Criteria
1. When request validation fails, the pricer_server shall return HTTP 400 Bad Request with structured JSON error response
2. When internal pricing errors occur, the pricer_server shall return HTTP 500 Internal Server Error with error type and message
3. When a requested resource is not found, the pricer_server shall return HTTP 404 Not Found
4. When an unsupported HTTP method is used, the pricer_server shall return HTTP 405 Method Not Allowed
5. If numerical parameters are out of valid range, then the pricer_server shall return HTTP 422 Unprocessable Entity with parameter constraints
6. The pricer_server shall validate all numeric inputs (spot > 0, strike > 0, maturity > 0, volatility > 0, etc.) before processing
7. The pricer_server shall log all errors with structured context (request_id, endpoint, input_params) for debugging
8. When rate limiting is exceeded, the pricer_server shall return HTTP 429 Too Many Requests

### Requirement 7: OpenAPI仕様とドキュメンテーション
**Objective:** API統合を行う開発者として、OpenAPI仕様に基づいた自動生成されたドキュメントを参照したい。その理由は、APIの構造と使用方法を理解し、クライアントコードを生成するためである。

#### Acceptance Criteria
1. The pricer_server shall generate OpenAPI 3.0 specification using utoipa crate
2. When a client sends GET /api/v1/openapi.json, the pricer_server shall return the complete OpenAPI specification in JSON format
3. When a client sends GET /docs, the pricer_server shall serve Swagger UI or RapiDoc for interactive API documentation
4. The pricer_server shall annotate all API endpoints with OpenAPI metadata (summary, description, request/response schemas)
5. The pricer_server shall document all error responses with example JSON payloads
6. The pricer_server shall include example request/response pairs for each endpoint in the OpenAPI spec
7. The pricer_server shall version the API (v1) and include version information in OpenAPI specification

### Requirement 8: ヘルスチェックとモニタリング
**Objective:** インフラストラクチャ運用者として、サーバーの健全性とパフォーマンスを監視したい。その理由は、ロードバランサー統合とサービス可用性の確保を行うためである。

#### Acceptance Criteria
1. When a client sends GET /health, the pricer_server shall return HTTP 200 OK with status: "healthy" if all dependencies are operational
2. When a client sends GET /ready, the pricer_server shall return HTTP 200 OK only after server initialization is complete
3. When a client sends GET /metrics, the pricer_server shall return Prometheus-compatible metrics (request count, latency, error rate)
4. The pricer_server shall track and expose metrics for each endpoint (pricing, greeks, xva) separately
5. The pricer_server shall expose server uptime and version information via GET /health endpoint
6. If critical dependencies are unavailable, then the pricer_server shall return HTTP 503 Service Unavailable from /health endpoint
7. The pricer_server shall use tracing middleware to measure request duration and generate access logs

### Requirement 9: セキュリティとレート制限
**Objective:** セキュリティ担当者として、APIへの不正アクセスや過度な使用を防ぎたい。その理由は、システムの安定性とデータ保護を確保するためである。

#### Acceptance Criteria
1. The pricer_server shall support optional API key authentication via Authorization header
2. Where API key authentication is enabled, the pricer_server shall validate keys before processing requests
3. If an invalid or missing API key is provided, then the pricer_server shall return HTTP 401 Unauthorized
4. The pricer_server shall implement rate limiting using tower middleware (configurable requests per minute)
5. When rate limits are exceeded, the pricer_server shall return HTTP 429 Too Many Requests with Retry-After header
6. The pricer_server shall support CORS configuration for cross-origin requests
7. The pricer_server shall set appropriate security headers (X-Content-Type-Options, X-Frame-Options, etc.)
8. The pricer_server shall not expose internal error details (stack traces, file paths) in production mode

### Requirement 10: 設定管理とデプロイメント
**Objective:** DevOpsエンジニアとして、環境固有の設定を外部化し、12-factor app原則に従ったデプロイを行いたい。その理由は、本番環境での運用性とスケーラビリティを確保するためである。

#### Acceptance Criteria
1. The pricer_server shall load configuration from environment variables (SERVER_HOST, SERVER_PORT, LOG_LEVEL, etc.)
2. Where configuration file is provided, the pricer_server shall support TOML or YAML configuration file format
3. When a configuration value is not provided, the pricer_server shall use sensible defaults (host: 0.0.0.0, port: 8080, log_level: info)
4. The pricer_server shall support environment-specific configuration (development, staging, production)
5. The pricer_server shall provide a Docker image build target using stable Rust (excluding pricer_kernel Enzyme features)
6. When deployed in container, the pricer_server shall respect container shutdown signals for graceful termination
7. The pricer_server shall expose configuration validation errors at startup with clear messages
8. The pricer_server shall support horizontal scaling (stateless design, no in-memory session storage)

### Requirement 11: レイヤーアーキテクチャ統合
**Objective:** プロジェクトアーキテクトとして、Layer 5（pricer_server）が既存の4層アーキテクチャに適切に統合され、依存関係の原則を維持したい。その理由は、コードベースの保守性と安定性を確保するためである。

#### Acceptance Criteria
1. The pricer_server shall be implemented as a new workspace member crate at crates/pricer_server/
2. The pricer_server shall depend on pricer_core (L1), pricer_models (L2), and pricer_xva (L4)
3. Where Monte Carlo pricing is required, the pricer_server shall optionally depend on pricer_kernel (L3) via feature flag
4. The pricer_server shall use stable Rust toolchain (not requiring nightly) for default builds
5. When Enzyme-based Greeks are requested, the pricer_server shall gate that functionality behind enzyme-mode feature flag
6. The pricer_server shall follow the project's import conventions (absolute imports for cross-crate, relative for same-crate)
7. The pricer_server shall not introduce new dependencies on nightly-only features outside of optional L3 integration
8. The pricer_server shall adhere to project naming conventions (snake_case modules, PascalCase types)

### Requirement 12: テストとCI/CD統合
**Objective:** 品質保証担当者として、APIエンドポイントの正確性と回帰防止のための自動テストを実行したい。その理由は、継続的インテグレーションプロセスに統合し、信頼性を確保するためである。

#### Acceptance Criteria
1. The pricer_server shall include integration tests for all API endpoints using reqwest or hyper client
2. When running tests, the pricer_server shall use in-memory test server instances (no external dependencies)
3. The pricer_server shall include unit tests for request/response serialization and validation logic
4. The pricer_server shall verify OpenAPI specification generation in automated tests
5. The pricer_server shall include property-based tests for input validation using proptest
6. When CI pipeline runs, the pricer_server shall be tested independently in .github/workflows/ci.yml
7. The pricer_server shall maintain test coverage above 80% for handler functions
8. The pricer_server shall include smoke tests verifying server startup and health endpoint responses
