# Requirements Document

## Project Description (Input)
Frictional Bank Web Appを徹底的に改良。バグなどを徹底的に解消。

## Introduction

本仕様は、Frictional Bank Web Dashboardの品質向上を目的とする。既存実装のコード分析に基づき、デバッグログの本番対応、未実装機能の完成、エラーハンドリング強化、パフォーマンス最適化、アクセシビリティ改善を行う。

### 対象範囲

- `demo/gui/static/` - フロントエンドHTML/CSS/JavaScript
- `demo/gui/src/web/` - Axum WebサーバーとREST API

### 分析結果サマリー

現行コードの分析により以下の改善点を特定：
- 本番環境に不適切なデバッグログ（15箇所以上の`console.log`）
- 未実装のZoom機能（プレースホルダのみ）
- エラー時のユーザーフィードバック不足
- 大規模な単一ファイル構成（app.js: 350KB超）

---

## Requirements

### Requirement 1: デバッグログの本番対応

**Objective:** As a 運用担当者, I want 本番環境でデバッグログが出力されない, so that コンソールがクリーンに保たれセキュリティリスクが軽減される

#### Acceptance Criteria
1. The Web Dashboard shall provide a debug mode flag that can be toggled via environment variable (`FB_DEBUG_MODE`)
2. When `FB_DEBUG_MODE` is not set or is `false`, the Web Dashboard shall suppress all `[DEBUG]` prefixed console outputs
3. When `FB_DEBUG_MODE` is `true`, the Web Dashboard shall output debug logs to the console
4. The Web Dashboard shall not expose sensitive data (API responses, internal state) in production console logs
5. When the application initializes, the Web Dashboard shall log the current debug mode setting once

---

### Requirement 2: Zoom機能の実装完成

**Objective:** As a トレーダー, I want グラフのズーム操作が正しく動作する, so that 詳細なデータポイントを確認できる

#### Acceptance Criteria
1. When the user clicks the zoom-in button, the Graph Viewer shall increase the viewport scale by 20%
2. When the user clicks the zoom-out button, the Graph Viewer shall decrease the viewport scale by 20%
3. When the user clicks the reset button, the Graph Viewer shall restore the viewport to 100% scale and center position
4. While zooming, the Graph Viewer shall maintain the center point of the current view
5. The Graph Viewer shall limit zoom range between 50% and 200%
6. When zoom level changes, the Graph Viewer shall update the zoom indicator display

---

### Requirement 3: エラーハンドリング改善

**Objective:** As a エンドユーザー, I want API通信エラー時に分かりやすいフィードバックを受け取る, so that 問題の原因と対処法が理解できる

#### Acceptance Criteria
1. If an API request fails with a network error, the Web Dashboard shall display a toast notification with the message "ネットワークエラー: サーバーに接続できません"
2. If an API request returns HTTP 5xx status, the Web Dashboard shall display a toast notification with the message "サーバーエラー: しばらく経ってから再試行してください"
3. If an API request returns HTTP 4xx status, the Web Dashboard shall display a context-appropriate error message
4. When an API error occurs, the Web Dashboard shall provide a retry button in the notification
5. While retrying, the Web Dashboard shall display a loading indicator
6. The Web Dashboard shall implement exponential backoff for automatic retries (max 3 attempts)
7. If WebSocket connection is lost, the Web Dashboard shall display a reconnection indicator and attempt to reconnect automatically

---

### Requirement 4: WebSocketの安定性強化

**Objective:** As a システム管理者, I want WebSocket接続が安定して維持される, so that リアルタイム更新が途切れない

#### Acceptance Criteria
1. When the WebSocket connection is lost, the WebSocket Client shall attempt to reconnect with exponential backoff (1s, 2s, 4s, 8s, max 30s)
2. The WebSocket Client shall send heartbeat ping messages every 30 seconds to keep the connection alive
3. If no response is received within 10 seconds of a ping, the WebSocket Client shall consider the connection dead and initiate reconnection
4. When reconnection succeeds, the Web Dashboard shall display a brief "接続が復旧しました" notification
5. While disconnected, the Web Dashboard shall display a visible "オフライン" indicator in the header
6. The WebSocket Client shall limit reconnection attempts to 10 before requiring manual intervention

---

### Requirement 5: ローディング状態の明確化

**Objective:** As a ユーザー, I want データ読み込み中の状態が明確に表示される, so that システムが応答していることを確認できる

#### Acceptance Criteria
1. While fetching portfolio data, the Portfolio Panel shall display a skeleton loader in place of the table
2. While fetching risk metrics, the Risk Metrics Panel shall display a skeleton loader for each metric card
3. While fetching exposure data, the Exposure Chart shall display a skeleton loader
4. The Web Dashboard shall display a progress indicator when multiple API calls are in progress simultaneously
5. If data loading takes more than 3 seconds, the Web Dashboard shall display "読み込みに時間がかかっています..." message
6. When data loading completes, the Web Dashboard shall smoothly transition from skeleton to actual content

---

### Requirement 6: レスポンシブデザイン改善

**Objective:** As a モバイルユーザー, I want タブレットやスマートフォンでも操作しやすい, so that 外出先でもダッシュボードを確認できる

#### Acceptance Criteria
1. When viewport width is less than 768px, the Web Dashboard shall collapse the sidebar to an icon-only mode
2. When viewport width is less than 768px, the Web Dashboard shall stack dashboard cards vertically
3. The Web Dashboard shall ensure all interactive elements have a minimum touch target size of 44x44 pixels on mobile
4. When viewport width is less than 480px, the Web Dashboard shall hide non-essential charts and show summary metrics only
5. The Portfolio Table shall support horizontal scrolling on narrow screens while keeping trade ID column fixed

---

### Requirement 7: キーボードナビゲーション強化

**Objective:** As a キーボードユーザー, I want キーボードだけで全機能にアクセスできる, so that マウスなしで効率的に操作できる

#### Acceptance Criteria
1. When the user presses Tab, the Web Dashboard shall move focus to the next interactive element in logical order
2. When the user presses Escape, the Web Dashboard shall close any open modal or panel
3. When focus is on a table row and user presses Enter, the Web Dashboard shall open the trade detail view
4. The Web Dashboard shall display a visible focus indicator on all focusable elements
5. When a modal opens, the Web Dashboard shall trap focus within the modal until it is closed
6. When a modal closes, the Web Dashboard shall return focus to the element that triggered it

---

### Requirement 8: コンソールログの構造化

**Objective:** As a 開発者, I want ログが構造化されている, so that デバッグ時に問題を迅速に特定できる

#### Acceptance Criteria
1. The Web Dashboard shall use a centralized logging utility instead of direct `console.log` calls
2. The Logging Utility shall support log levels: DEBUG, INFO, WARN, ERROR
3. The Logging Utility shall include timestamp in all log messages
4. The Logging Utility shall include component name (e.g., [WebSocket], [API], [UI]) in all log messages
5. When `FB_LOG_LEVEL` environment variable is set, the Logging Utility shall filter logs below the specified level

---

### Requirement 9: パフォーマンス監視

**Objective:** As a DevOps担当者, I want アプリケーションのパフォーマンスメトリクスを収集できる, so that ボトルネックを特定できる

#### Acceptance Criteria
1. The Web Dashboard shall measure and log API response times for all REST endpoints
2. The Web Dashboard shall measure and log WebSocket message latency
3. When API response time exceeds 1 second, the Performance Monitor shall log a warning
4. The Web Dashboard shall expose a `/api/metrics` endpoint that returns current performance statistics in JSON format
5. The Performance Monitor shall track the number of active WebSocket connections

---

### Requirement 10: CSP（Content Security Policy）の最適化

**Objective:** As a セキュリティ担当者, I want CSPが厳格に設定されている, so that XSS攻撃のリスクが最小化される

#### Acceptance Criteria
1. The Web Dashboard shall not use `'unsafe-inline'` for script-src (remove inline scripts)
2. The Web Dashboard shall load all third-party libraries from local vendor directory only
3. The Web Dashboard shall report CSP violations to a configurable endpoint via `report-uri` directive
4. When a CSP violation occurs, the Web Dashboard shall log the violation details with WARN level
5. The Web Dashboard shall use nonces for any necessary inline scripts

---

### Requirement 11: テストカバレッジ強化

**Objective:** As a 品質保証担当者, I want フロントエンドのテストカバレッジが向上している, so that リグレッションを早期に検出できる

#### Acceptance Criteria
1. The Web API shall have unit tests for all REST endpoint handlers
2. The WebSocket module shall have unit tests for message parsing and broadcast functions
3. When running `cargo test`, the test suite shall achieve at least 80% code coverage for `demo/gui/src/web/`
4. The test suite shall include integration tests for API-WebSocket interaction scenarios
5. The test suite shall include tests for error handling paths (network failures, invalid responses)

---

### Requirement 12: アクセシビリティ（WCAG 2.1 AA準拠）

**Objective:** As a 視覚障害を持つユーザー, I want スクリーンリーダーでダッシュボードを操作できる, so that 必要な情報にアクセスできる

#### Acceptance Criteria
1. The Web Dashboard shall provide ARIA labels for all interactive elements
2. The Web Dashboard shall announce dynamic content updates using ARIA live regions
3. The Web Dashboard shall maintain a color contrast ratio of at least 4.5:1 for all text
4. When displaying charts, the Web Dashboard shall provide alternative text descriptions of the data
5. The Web Dashboard shall support system-level "Reduce Motion" preference
6. The Portfolio Table shall use proper `<th>` headers with `scope` attributes for data cells

---

### Requirement 13: ダークモード/ライトモードの動作安定化

**Objective:** As a ユーザー, I want テーマ切り替えが正しく動作する, so that 好みの表示モードで作業できる

#### Acceptance Criteria
1. When the user selects a theme mode, the Web Dashboard shall persist the selection to localStorage
2. When the page loads, the Web Dashboard shall apply the previously selected theme mode
3. When the user selects "Auto" mode, the Web Dashboard shall follow the system preference
4. When the system preference changes while "Auto" mode is active, the Web Dashboard shall update the theme immediately
5. When theme changes, the Web Dashboard shall apply the change without page reload
6. The Web Dashboard shall ensure all UI components (including charts) respect the current theme
