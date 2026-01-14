# Research & Design Decisions

## Summary

- **Feature**: `frictional-bank-webapp-polish`
- **Discovery Scope**: Extension（既存システムの改善）
- **Key Findings**:
  - 既存コードベースは高度に実装済み（skeleton CSS、trapFocus、CSP header等）
  - 主要ギャップは新規ユーティリティ作成（Logger）と既存機能拡張
  - Hybrid Approach（新規＋拡張）が最適

---

## Research Log

### デバッグログ・構造化ログ基盤（Req 1, 8）

- **Context**: `app.js`に14箇所以上の直接`console.log`呼び出しが存在
- **Sources Consulted**:
  - 既存コード: `app.js:7`（DEBUG prefix）、`app.js:9-32`（CSP_DEBUG実装パターン）
  - 環境変数パターン: `FB_CORS_ORIGINS`、`FB_CSP`（mod.rs:117, 153）
- **Findings**:
  - 環境変数ベースの設定パターンは`FB_*`プレフィックスで統一済み
  - CSP_DEBUGはURLパラメータとlocalStorageの両方をサポート
  - コンポーネント名prefix（`[CSP]`）は既に部分的に使用
- **Implications**:
  - `FB_DEBUG_MODE`、`FB_LOG_LEVEL`環境変数を追加
  - Loggerクラスは既存パターンに従いモジュールとして実装
  - HTMLテンプレート経由で環境変数を注入（Rustサーバー側）

### Zoom機能実装（Req 2）

- **Context**: Graph ViewerはD3.js zoom完全実装、Exposure Chartはプレースホルダのみ
- **Sources Consulted**:
  - `app.js:5247-5260` - D3 zoom behavior（scaleExtent、transform）
  - `app.js:3018-3028` - initZoomControls（console.logのみ）
  - Chart.js公式ドキュメント
- **Findings**:
  - Graph ViewerはD3.js `d3.zoom()`で実装済み（scaleExtent [0.1, 4]）
  - Exposure ChartはChart.jsベース、zoom pluginが必要
  - chartjs-plugin-zoomは既存vendorディレクトリになし
- **Implications**:
  - Chart.js zoom pluginをvendor/に追加、またはChart.js標準スケール操作で代替
  - 50%-200%範囲制限はChart.js limits optionで実現可能
  - Graph Viewer zoom（D3）とは別実装が必要

### WebSocket安定性（Req 4）

- **Context**: 現行は固定3秒リトライのみ
- **Sources Consulted**:
  - `app.js:2312-2346` - connectWebSocket関数
  - `websocket.rs:79-81` - ping/pong handler（サーバー側）
- **Findings**:
  - クライアント側: `setTimeout(connectWebSocket, 3000)`（固定遅延）
  - サーバー側: ping/pongメッセージ対応済み
  - 接続状態表示: `#connection-status`要素あり
- **Implications**:
  - Exponential backoff実装（1s→2s→4s→8s→max 30s）
  - クライアント側heartbeat（30秒間隔）追加
  - 再接続試行回数制限（max 10回）
  - 状態管理用state拡張（retryCount、lastPingTime等）

### エラーハンドリング（Req 3）

- **Context**: showToast関数は汎用的、リトライ機能なし
- **Sources Consulted**:
  - `app.js:3402-3460` - showToast関数（新旧フォーマット対応）
  - `app.js:182-195` - fetchJson関数
- **Findings**:
  - showToastはtype/title/message/durationをサポート
  - エラーメッセージは英語（"Failed to fetch..."）
  - リトライボタン、exponential backoff未実装
- **Implications**:
  - showToast拡張（action button callback対応）
  - fetchJson wrapper with retry logic
  - 日本語エラーメッセージマッピング

### スケルトンローディング（Req 5）

- **Context**: CSSは完全実装済み、DOM/JS制御なし
- **Sources Consulted**:
  - `style.css:6512-6556` - skeleton、skeleton-text、skeleton-card classes
  - `index.html:2028-2029` - loading-overlay要素
- **Findings**:
  - スケルトンアニメーション（shimmer効果）CSS完備
  - `.skeleton-text.short/medium/long`バリエーションあり
  - パネル別スケルトンDOMは未作成
- **Implications**:
  - 各パネル用スケルトンDOM追加（Portfolio、Risk、Exposure）
  - 状態管理でloading状態切り替え
  - 3秒タイムアウト時メッセージ表示ロジック

### キーボードナビゲーション（Req 7）

- **Context**: trapFocus、openDialog/closeDialog実装済み
- **Sources Consulted**:
  - `app.js:216-261` - trapFocus、openDialog、closeDialog関数
  - `app.js:263-282` - applyIconButtonLabels関数
- **Findings**:
  - Focus trap完全実装（Tab/Shift+Tab循環）
  - Dialog開閉時のフォーカス復帰対応済み
  - getFocusableElements関数でフォーカス可能要素取得
- **Implications**:
  - テーブル行Enterキー処理追加
  - 全フォーカス可能要素への可視インジケーター確保
  - Tab順序の論理的整合性検証

### パフォーマンス監視（Req 9）

- **Context**: 新規API endpoint必要
- **Sources Consulted**:
  - `mod.rs:163-198` - build_router関数（既存routes）
  - `handlers.rs` - 既存handler実装パターン
  - `AppState`構造体（mod.rs:34-41）
- **Findings**:
  - AppStateはbroadcast channel、graph_cache、subscriptionsを保持
  - RwLockパターンで状態管理
  - JSON応答パターン確立済み
- **Implications**:
  - AppStateにmetricsフィールド追加（AtomicU64等）
  - `/api/metrics` endpoint追加
  - フロントエンドfetch wrapper内でタイミング計測

### CSP最適化（Req 10）

- **Context**: CSPヘッダーは設定済み、'unsafe-inline'使用中
- **Sources Consulted**:
  - `mod.rs:145-161` - build_csp_header関数
  - `app.js:9-32` - CSP violation listener
- **Findings**:
  - DEFAULT_CSPで`style-src 'unsafe-inline'`使用
  - FB_CSP環境変数でオーバーライド可能
  - CSP violation listenerはCSP_DEBUG時のみ有効
- **Implications**:
  - inline styleの外部CSS化検討
  - nonce生成とテンプレート注入（HTML動的生成）
  - report-uri directive追加

### テストカバレッジ（Req 11）

- **Context**: 既存テスト多数、カバレッジ未計測
- **Sources Consulted**:
  - `handlers.rs` - 23 test functions
  - `websocket.rs` - 33 test functions
  - `mod.rs` - 10 test functions
- **Findings**:
  - 単体テスト充実（66テスト合計）
  - tokio::test使用の非同期テストパターン確立
  - integration test構造未確認
- **Implications**:
  - cargo-tarpaulinでカバレッジ計測
  - API-WebSocket連携テスト追加
  - エラーパステスト強化

---

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Option A: Extend Existing | app.js内既存関数拡張のみ | ファイル数維持、パターン統一 | app.js肥大化（既に350KB+） | 単純な修正に適切 |
| Option B: New Components | logger.js、performance.js等分離 | 責務分離、独立テスト | 統合コスト、ファイル増加 | 新規機能に適切 |
| **Option C: Hybrid** | 新規ユーティリティ＋既存拡張 | バランス、段階的実装 | フェーズ間整合性管理 | **採用** |

---

## Design Decisions

### Decision: Logger Utility分離

- **Context**: Req 1（デバッグログ）とReq 8（構造化ログ）を統合実装
- **Alternatives Considered**:
  1. app.js内inline実装 — 単純だが肥大化促進
  2. 別ファイル`logger.js` — 明確な責務分離
- **Selected Approach**: `logger.js`を新規作成、ES Module形式
- **Rationale**:
  - 他コンポーネントから独立してテスト可能
  - 将来のバンドラー導入時に移行容易
- **Trade-offs**:
  - 追加HTTPリクエスト（ただしキャッシュ可能）
  - グローバルスコープ汚染回避のためwindow.FB_Loggerに公開
- **Follow-up**: app.js内console.log置換時の網羅性確認

### Decision: Chart.js Zoom実装方式

- **Context**: Exposure ChartのZoom機能実装（Req 2）
- **Alternatives Considered**:
  1. chartjs-plugin-zoom導入 — 機能豊富だが依存追加
  2. Chart.js標準scale操作 — 軽量だが機能限定
  3. D3.js統一 — 一貫性あるがChart.js置換コスト大
- **Selected Approach**: Chart.js標準scale操作 + 手動transform
- **Rationale**:
  - 追加ライブラリ不要
  - 50%-200%範囲制限は手動制御で十分
  - vendorディレクトリ肥大化回避
- **Trade-offs**:
  - ホイールズームは手動実装必要
  - plugin利用時より実装コスト増
- **Follow-up**: UX検証後にplugin導入検討

### Decision: WebSocket再接続戦略

- **Context**: 安定性強化（Req 4）
- **Alternatives Considered**:
  1. 固定間隔リトライ維持 — 単純だがサーバー負荷懸念
  2. Exponential backoff — 業界標準、負荷分散
  3. 外部ライブラリ（reconnecting-websocket等） — 機能豊富だが依存追加
- **Selected Approach**: 自前exponential backoff実装
- **Rationale**:
  - 既存コードとの統合容易
  - 追加依存なし
  - カスタマイズ柔軟
- **Trade-offs**:
  - 実装コスト
  - テストカバレッジ要確保
- **Follow-up**: 本番環境での挙動監視

### Decision: Metrics API実装

- **Context**: パフォーマンス監視（Req 9）
- **Alternatives Considered**:
  1. Prometheus形式 — 標準的だが過剰
  2. JSON API — シンプル、既存パターン統一
  3. OpenTelemetry — 将来性あるが導入コスト大
- **Selected Approach**: JSON API（`/api/metrics`）
- **Rationale**:
  - 既存handlers.rsパターンに統一
  - フロントエンド直接利用可能
  - 軽量実装
- **Trade-offs**:
  - Prometheus等との互換性なし
  - 大規模監視システム統合時に追加対応必要
- **Follow-up**: 必要に応じてPrometheus exporter追加

---

## Risks & Mitigations

- **app.js肥大化** — logger.js分離、将来的なモジュール分割検討
- **CSP厳格化による既存機能破壊** — 段階的適用、violation監視強化
- **WebSocket再接続ループ** — 最大試行回数制限（10回）、手動介入UI
- **テストカバレッジ80%未達** — 早期計測、優先度付きテスト追加

---

## References

- [Chart.js Scale Configuration](https://www.chartjs.org/docs/latest/axes/) — zoom実装参考
- [MDN: Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP) — CSP nonce実装
- [WebSocket Reconnection Best Practices](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket) — exponential backoff
- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) — Rustカバレッジ計測
- Axum Documentation — handler/state patterns
