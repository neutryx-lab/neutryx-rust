# Implementation Plan

## Task Format Template

> **Parallel marker**: Append ` (P)` only to tasks that can be executed in parallel.
>
> **Optional test coverage**: When a sub-task is deferrable test work tied to acceptance criteria, mark the checkbox as `- [ ]*`.

---

## Tasks

- [ ] 1. pricer_serverクレートの初期セットアップ
- [x] 1.1 (P) Cargoワークスペースにpricer_serverクレートを追加し、依存関係を設定する
  - cratesディレクトリに新規pricer_serverクレートを作成
  - Cargo.tomlでAxum、tokio、tower、serde、utoipa、tracing等の依存関係を定義
  - pricer_core、pricer_models、pricer_xvaへの依存関係を設定
  - pricer_kernelをoptional依存としてkernel-integration featureフラグで管理
  - stable Rust (Edition 2021) で動作するよう設定
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5, 11.7, 11.8_

- [x] 1.2 (P) サーバー設定管理機能を実装する
  - 環境変数からサーバー設定を読み込む機能 (PRICER_SERVER_HOST, PRICER_SERVER_PORT, PRICER_LOG_LEVEL等)
  - TOML形式の設定ファイルからの読み込みをサポート
  - CLI引数による設定上書き機能
  - 設定値のバリデーション (ポート範囲、ログレベル検証)
  - デフォルト値の提供 (host: 0.0.0.0, port: 8080, log_level: info)
  - 環境別設定 (development, staging, production) のサポート
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.7_

- [ ] 2. Axum HTTPサーバー基盤の構築
- [x] 2.1 Axumルーターの基本構造を構築する
  - Axum Routerを使用したHTTPルーティング基盤を作成
  - エンドポイントグループ別にルーターをモジュール分割 (pricing, greeks, xva, health)
  - Router::merge()で各モジュールを統合
  - サーバー設定をArc<ServerConfig>としてStateで共有
  - 設定可能なhost/portでサーバーをbind
  - _Requirements: 1.2, 1.3, 1.4_

- [ ] 2.2 Graceful shutdown機能を実装する
  - tokio::signalでSIGTERM/SIGINTシグナルを監視
  - Axum Server::with_graceful_shutdown()で進行中リクエストの完了を待機
  - 設定可能なタイムアウト (デフォルト30秒) で強制終了
  - shutdown開始/完了をログに記録
  - コンテナ環境 (Docker/K8s) での終了シグナル対応
  - _Requirements: 1.5, 10.6_

- [ ] 2.3 構造化ログ機能を実装する
  - tracing-subscriberでJSON形式の構造化ログを設定
  - tower-http TraceLayerをmiddleware stackに適用
  - リクエストごとにrequest_id (UUID v4) を生成しspanに付与
  - 環境変数 (LOG_LEVEL) によるログレベルフィルター
  - サーバー起動/シャットダウン/リクエスト情報をログ出力
  - _Requirements: 1.6, 6.7_

- [ ] 3. JSON入出力とシリアライゼーション基盤
- [ ] 3.1 (P) APIリクエスト/レスポンス型を定義する
  - 全Request/Response構造体にserde derive macrosを適用
  - camelCase命名規則をserde(rename_all)で設定
  - chrono::DateTime<Utc>でISO 8601形式の日時処理
  - pricer_core::types::CurrencyでISO 4217通貨コード対応
  - Option型を活用したオプショナルフィールドの定義
  - _Requirements: 2.2, 2.3, 2.4, 2.5_

- [ ] 3.2 JSONリクエストの受信とレスポンス送信を実装する
  - Content-Type: application/jsonの受け入れ
  - serde_jsonによるJSON形式へのシリアライゼーション
  - 不正なJSON受信時にHTTP 400 Bad Requestを返却
  - エラーレスポンスに説明的なメッセージを含める
  - _Requirements: 2.1, 2.2, 2.6, 2.7_

- [ ] 4. エラーハンドリングとバリデーション機能
- [ ] 4.1 グローバルエラーハンドリング機能を実装する
  - ApiError列挙型でHTTPステータスコードへのマッピングを定義
  - ErrorResponse構造体でJSON形式のエラーレスポンスを生成
  - pricer_* crateのエラーをApiErrorに変換
  - 本番環境で内部エラー詳細 (スタックトレース等) をマスク
  - エラー発生時にrequest_id、endpoint、入力パラメータをログに記録
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.7_

- [ ] 4.2 入力バリデーション機能を実装する
  - 数値パラメータの範囲チェック (spot > 0, strike > 0, maturity > 0, volatility > 0等)
  - バリデーションエラー時にHTTP 422 Unprocessable Entityを返却
  - エラーメッセージにパラメータ制約条件を含める
  - 数値不安定時 (Greeks計算失敗等) のHTTP 422返却
  - _Requirements: 6.5, 6.6_

- [ ] 5. オプション価格計算エンドポイントの実装
- [ ] 5.1 バニラオプション価格計算エンドポイントを実装する
  - POST /api/v1/price/vanilla エンドポイントを作成
  - VanillaPriceRequest (option_type, spot, strike, maturity, risk_free_rate, volatility, smoothing_epsilon) を受け入れ
  - pricer_models分析解を使用してバニラオプション価格を計算
  - PriceResponse (price, calculation_method, timestamp) を返却
  - 計算エラー時にHTTP 500を返却
  - 30秒以内の応答を確保
  - _Requirements: 3.1, 3.5, 3.6, 3.7, 1.7_

- [ ] 5.2 Asianオプション価格計算エンドポイントを実装する
  - POST /api/v1/price/asian エンドポイントを作成
  - AsianPriceRequest (基本パラメータ + averaging_type, simulation_params) を受け入れ
  - pricer_kernel Monte Carlo (feature有効時) または分析解で計算
  - simulation_paramsのデフォルト値 (num_paths: 10000, num_steps: 100) を提供
  - kernel-integration feature無効時はHTTP 501を返却
  - _Requirements: 3.2, 3.5, 3.6, 3.7, 3.8_

- [ ] 5.3 Barrierオプション価格計算エンドポイントを実装する
  - POST /api/v1/price/barrier エンドポイントを作成
  - BarrierPriceRequest (基本パラメータ + barrier_type, barrier, simulation_params) を受け入れ
  - 8種類のbarrier type (up_in, up_out, down_in, down_out x call/put) をサポート
  - pricer_kernel Monte Carloまたは分析解で計算
  - _Requirements: 3.3, 3.5, 3.6, 3.7, 3.8_

- [ ] 5.4 Lookbackオプション価格計算エンドポイントを実装する
  - POST /api/v1/price/lookback エンドポイントを作成
  - LookbackPriceRequest (基本パラメータ + lookback_type, simulation_params) を受け入れ
  - fixed_strike/floating_strike両方をサポート
  - pricer_kernel Monte Carloで計算
  - _Requirements: 3.4, 3.5, 3.6, 3.7, 3.8_

- [ ] 6. ギリシャ指標計算エンドポイントの実装
- [ ] 6.1 ギリシャ指標計算エンドポイントを実装する
  - POST /api/v1/greeks エンドポイントを作成
  - GreeksRequest (オプションパラメータ + greeks_config, simulation_params) を受け入れ
  - 計算するギリシャを設定で指定 (delta, gamma, vega, theta, rho)
  - pricer_kernel AD (feature有効時) またはbump-and-revalueで計算
  - GreeksResponse (delta, gamma, vega, theta, rho, calculation_method, timestamp) を返却
  - 複数ギリシャ計算時にシミュレーションパスを再利用
  - 数値不安定時にHTTP 422を返却
  - バニラ、Asian、Barrier、Lookback各オプションのギリシャ計算をサポート
  - kernel-integration feature無効時はHTTP 501を返却
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7_

- [ ] 7. ポートフォリオXVA計算エンドポイントの実装
- [ ] 7.1 ポートフォリオXVA計算エンドポイントを実装する
  - POST /api/v1/xva/portfolio エンドポイントを作成
  - PortfolioXvaRequest (counterparties, netting_sets, trades, time_grid, simulation_params) を受け入れ
  - pricer_xvaを使用してポートフォリオXVAを計算
  - PortfolioXvaResponse (CVA, DVA, FVA, exposure_metrics) を返却
  - exposure_metricsにEE、EPE、PFE、EEPE、ENEを含める
  - Rayon並列処理で大規模ポートフォリオを効率的に計算
  - ポートフォリオバリデーション失敗時にHTTP 400を返却
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

- [ ] 7.2 カウンターパーティXVA計算エンドポイントを実装する
  - POST /api/v1/xva/counterparty エンドポイントを作成
  - カウンターパーティレベルのXVA計算を実行
  - ポートフォリオXVAと同様のレスポンス形式
  - _Requirements: 5.7_

- [ ] 8. ヘルスチェックとモニタリングの実装
- [ ] 8.1 (P) ヘルスチェックエンドポイントを実装する
  - GET /health エンドポイントを作成
  - サーバー稼働状態、バージョン情報、uptimeを返却
  - 依存関係 (pricer_core, pricer_models, pricer_xva, pricer_kernel) のステータスを確認
  - 依存関係エラー時にHTTP 503を返却
  - _Requirements: 8.1, 8.5, 8.6_

- [ ] 8.2 (P) Readinessエンドポイントを実装する
  - GET /ready エンドポイントを作成
  - 初期化完了後にHTTP 200を返却
  - 起動中はHTTP 503を返却
  - _Requirements: 8.2_

- [ ] 8.3 (P) Prometheusメトリクスエンドポイントを実装する
  - GET /metrics エンドポイントを作成
  - axum-prometheusでリクエスト数、レイテンシー、エラー率を収集
  - エンドポイント別 (pricing, greeks, xva) にメトリクスを分離
  - Prometheusテキストフォーマットで出力
  - tracing middlewareでリクエスト所要時間を計測
  - _Requirements: 8.3, 8.4, 8.7_

- [ ] 9. セキュリティとレート制限の実装
- [ ] 9.1 API key認証機能を実装する
  - AuthorizationヘッダーまたはX-API-KeyヘッダーからAPI keyを抽出
  - 設定されたAPI keyリストと照合
  - 無効/欠損API key時にHTTP 401 Unauthorizedを返却
  - api_key_required設定で認証の有効/無効を切り替え
  - _Requirements: 9.1, 9.2, 9.3_

- [ ] 9.2 レート制限機能を実装する
  - tower_governorを使用してレート制限を実装
  - 設定可能なrequests per minute (デフォルト60)
  - 制限超過時にHTTP 429 Too Many Requests + Retry-Afterヘッダーを返却
  - API key別のレート制限をサポート
  - _Requirements: 9.4, 9.5, 6.8_

- [ ] 9.3 セキュリティヘッダーとCORSを実装する
  - X-Content-Type-Options: nosniff を設定
  - X-Frame-Options: DENY を設定
  - Content-Security-Policy を設定
  - 設定可能なCORSオリジンリストをサポート
  - 本番環境で内部エラー詳細を非公開化
  - _Requirements: 9.6, 9.7, 9.8_

- [ ] 10. OpenAPI仕様とドキュメンテーションの実装
- [ ] 10.1 OpenAPI仕様の自動生成を実装する
  - utoipaを使用して全Request/Response型からスキーマを生成
  - 全エンドポイントにOpenAPIアノテーション (summary, description) を追加
  - エラーレスポンスのサンプルJSONを含める
  - API version (v1) をOpenAPI仕様に含める
  - GET /api/v1/openapi.json でJSON形式のOpenAPI仕様を提供
  - _Requirements: 7.1, 7.2, 7.4, 7.5, 7.6, 7.7_

- [ ] 10.2 Swagger UIを提供する
  - utoipa-swagger-uiでSwagger UIを埋め込む
  - GET /docs でインタラクティブなAPIドキュメントを提供
  - 各エンドポイントにリクエスト/レスポンス例を含める
  - _Requirements: 7.3, 7.6_

- [ ] 11. Middlewareスタックの統合
- [ ] 11.1 Tower middlewareスタックを構成する
  - Middleware適用順序を設定: RateLimit -> Auth -> Tracing -> CORS -> Compression
  - tower-httpでCORS、tracing、gzip圧縮を適用
  - 全middlewareをAxum Routerに統合
  - middleware順序誤りによるセキュリティリスクを防止
  - _Requirements: 1.2, 1.4, 9.4, 9.6_

- [ ] 12. テストの実装
- [ ] 12.1 (P) Request/Responseシリアライゼーションのユニットテストを実装する
  - 全Request型のJSON deserialization テスト
  - 全Response型のJSON serialization テスト
  - camelCase変換、ISO 8601日時、ISO 4217通貨コードの検証
  - _Requirements: 12.3_

- [ ] 12.2 (P) バリデーションロジックのユニットテストを実装する
  - 数値パラメータ範囲チェックのテスト (spot > 0等)
  - 境界値テスト
  - proptestを使用したプロパティベーステスト
  - _Requirements: 12.3, 12.5_

- [ ] 12.3 (P) エラー型変換のユニットテストを実装する
  - pricer_core, pricer_models, pricer_xvaエラーからApiErrorへの変換テスト
  - 各HTTPステータスコードの検証
  - _Requirements: 12.3_

- [ ] 12.4 APIエンドポイントの統合テストを実装する
  - reqwest clientを使用した全エンドポイントのテスト
  - インメモリテストサーバーインスタンスを使用
  - 価格計算、Greeks、XVA各エンドポイントの正常系/異常系テスト
  - _Requirements: 12.1, 12.2_

- [ ] 12.5 認証とレート制限の統合テストを実装する
  - 有効/無効/欠損API keyのテスト
  - 連続リクエストでHTTP 429確認
  - Retry-Afterヘッダーの検証
  - _Requirements: 12.1, 12.2_

- [ ] 12.6 サーバー起動とスモークテストを実装する
  - サーバー起動確認テスト
  - /health、/ready、/metricsエンドポイントのレスポンス検証
  - graceful shutdownテスト (可能な範囲で)
  - _Requirements: 12.8_

- [ ] 12.7 OpenAPI仕様生成の検証テストを実装する
  - OpenAPI仕様が正しく生成されることを確認
  - スキーマ整合性の検証
  - _Requirements: 12.4_

- [ ]* 12.8 ハンドラー関数のテストカバレッジ検証
  - テストカバレッジ80%以上の確認
  - カバレッジレポートの生成
  - _Requirements: 12.7_

- [ ] 13. CI/CD統合
- [ ] 13.1 CI/CD設定を追加する
  - .github/workflows/ci.ymlにpricer_serverのテストジョブを追加
  - stable Rustでのビルド・テスト設定
  - kernel-integration featureを含むnightly版テスト (オプショナル)
  - _Requirements: 12.6_

- [ ] 14. Docker設定の実装
- [ ] 14.1 (P) stable版Dockerfileを作成する
  - pricer_serverのstable Rustビルド (pricer_kernel除外)
  - 最小限のランタイムイメージ (debian:bookworm-slim)
  - ポート8080のEXPOSE
  - コンテナシャットダウンシグナルへの対応
  - _Requirements: 10.5, 10.6, 10.8_

- [ ] 14.2 (P) nightly版Dockerfileを作成する (オプショナル)
  - kernel-integration feature有効化ビルド
  - LLVM 18インストール
  - Enzyme統合環境
  - _Requirements: 11.5_

---

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1, 1.2, 1.3, 1.4 | 1.1, 2.1, 11.1 |
| 1.5, 1.6, 1.7 | 2.2, 2.3, 5.1 |
| 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7 | 3.1, 3.2 |
| 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8 | 5.1, 5.2, 5.3, 5.4 |
| 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7 | 6.1 |
| 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7 | 7.1, 7.2 |
| 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8 | 4.1, 4.2, 9.2 |
| 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 7.7 | 10.1, 10.2 |
| 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7 | 8.1, 8.2, 8.3 |
| 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8 | 9.1, 9.2, 9.3 |
| 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7, 10.8 | 1.2, 2.2, 14.1 |
| 11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 11.7, 11.8 | 1.1, 14.2 |
| 12.1, 12.2, 12.3, 12.4, 12.5, 12.6, 12.7, 12.8 | 12.1, 12.2, 12.3, 12.4, 12.5, 12.6, 12.7, 12.8, 13.1 |
