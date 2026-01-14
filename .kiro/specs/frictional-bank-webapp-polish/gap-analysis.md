# Gap Analysis: frictional-bank-webapp-polish

## Executive Summary

既存のFrictional Bank Web Appは高度に実装されており、多くの要件項目が部分的に実装済み。主なギャップは以下の3カテゴリに分類される：

1. **コード品質改善** (Req 1, 8): デバッグログと構造化ログ - 新規ユーティリティ作成
2. **機能完成** (Req 2, 4, 9): プレースホルダの実装完成 - 既存コード拡張
3. **品質向上** (Req 3, 5, 6, 7, 10, 11, 12, 13): 既存機能の強化 - 増分改善

**推奨アプローチ**: ハイブリッド（Option C） - 新規ユーティリティ作成＋既存コード拡張
**総合工数見積**: M〜L (1-2週間)
**リスク**: Low〜Medium

---

## Requirement-to-Asset Map

### Requirement 1: デバッグログの本番対応

| Item | Status | Current Asset | Gap |
|------|--------|---------------|-----|
| Debug mode toggle | Missing | なし | 環境変数制御機構の新規作成 |
| Console log suppression | Missing | 14+箇所の直接console.log | Loggerラッパーへの置換 |
| Debug mode indicator | Missing | なし | 初期化時の1回ログ出力 |

**Existing Code References**:
- `demo/gui/static/app.js:7` - `console.log('[DEBUG] app.js loading...')`
- `demo/gui/static/app.js:811-830` - fetchPortfolio debug logs
- `demo/gui/static/app.js:4762-4810` - init() debug logs

**Gap Type**: Missing - 新規ユーティリティ必要

---

### Requirement 2: Zoom機能の実装完成

| Item | Status | Current Asset | Gap |
|------|--------|---------------|-----|
| Graph Viewer zoom | ✅ Implemented | `setupZoomBehavior()`, `resetGraphZoom()`, `zoomToFit()` | なし |
| Exposure Chart zoom | Placeholder | `initZoomControls()` lines 3026-3028 | console.logのみ、実装なし |
| Zoom indicator | Partial | Graph has scale extent | Chart用インジケーター不足 |

**Existing Code References**:
- `demo/gui/static/app.js:5247-5260` - D3 zoom behavior (Graph - 完全実装)
- `demo/gui/static/app.js:3026-3028` - Chart zoom (プレースホルダ)

**Gap Type**: Partial - Exposure Chartの実装のみ必要

---

### Requirement 3: エラーハンドリング改善

| Item | Status | Current Asset | Gap |
|------|--------|---------------|-----|
| Toast notification | ✅ Implemented | `showToast()` function | 汎用的なメッセージ |
| Network error message | Missing | "Failed to fetch..." | 日本語/詳細メッセージ |
| Retry button | Missing | なし | Toast内ボタン追加 |
| Exponential backoff | Missing | なし | 自動リトライ機構 |

**Existing Code References**:
- `demo/gui/static/app.js:834` - `showToast('Failed to fetch portfolio', 'error')`
- `demo/gui/static/app.js:901` - `showToast('Failed to fetch risk metrics', 'error')`

**Gap Type**: Enhancement - 既存機能の拡張

---

### Requirement 4: WebSocketの安定性強化

| Item | Status | Current Asset | Gap |
|------|--------|---------------|-----|
| Reconnect logic | Partial | 固定3秒リトライ (`setTimeout(..., 3000)`) | Exponential backoff不足 |
| Heartbeat ping | Missing | なし | ping/pong機構の追加 |
| Connection indicator | Partial | `#connection-status` 要素あり | オフライン表示の改善 |
| Reconnect limit | Missing | 無制限リトライ | 最大試行回数制限 |

**Existing Code References**:
- `demo/gui/static/app.js:2312-2346` - `connectWebSocket()` function
- `demo/gui/static/app.js:2329` - `setTimeout(connectWebSocket, 3000)`

**Gap Type**: Enhancement - 既存機能の大幅改善

---

### Requirement 5: ローディング状態の明確化

| Item | Status | Current Asset | Gap |
|------|--------|---------------|-----|
| Skeleton CSS | ✅ Implemented | `.skeleton`, `.skeleton-text`, `.skeleton-card` classes | なし |
| Loading overlay | ✅ Implemented | `#loading-overlay` element | なし |
| Panel skeletons | Missing | スタイルのみ、DOMなし | HTML追加・JS制御 |
| Timeout message | Missing | なし | 3秒タイムアウト表示 |

**Existing Code References**:
- `demo/gui/static/style.css:6512-6556` - Skeleton styles (完全実装)
- `demo/gui/static/index.html:2028-2029` - Loading overlay

**Gap Type**: Enhancement - 既存スタイルの適用拡大

---

### Requirement 6: レスポンシブデザイン改善

| Item | Status | Current Asset | Gap |
|------|--------|---------------|-----|
| Sidebar collapse | ✅ Implemented | Media queries at 768px | 検証必要 |
| Card stacking | ✅ Implemented | Grid responsive styles | 検証必要 |
| Touch targets | Unknown | 未検証 | 44x44px確認必要 |
| Table scrolling | Partial | 一部実装 | sticky column追加 |

**Existing Code References**:
- `demo/gui/static/style.css:2418-2696` - Media queries
- `demo/gui/static/style.css:7549` - 768px breakpoint

**Gap Type**: Verification + Minor Enhancement

---

### Requirement 7: キーボードナビゲーション強化

| Item | Status | Current Asset | Gap |
|------|--------|---------------|-----|
| Tab navigation | ✅ Implemented | 標準ブラウザ動作 | 順序検証 |
| Focus trap | ✅ Implemented | `trapFocus()` function | なし |
| Focus indicator | Partial | 一部要素のみ | 全要素への適用 |
| Escape close | ✅ Implemented | Modal handlers | なし |
| Table Enter | Missing | なし | テーブル行Enter処理 |

**Existing Code References**:
- `demo/gui/static/app.js:216-243` - `trapFocus()` function
- `demo/gui/static/index.html:29` - `tabindex="-1"` attributes

**Gap Type**: Enhancement - 部分的追加

---

### Requirement 8: コンソールログの構造化

| Item | Status | Current Asset | Gap |
|------|--------|---------------|-----|
| Logger utility | Missing | なし | 新規Loggerクラス作成 |
| Log levels | Missing | なし | DEBUG/INFO/WARN/ERROR |
| Timestamp | Missing | なし | ログにタイムスタンプ追加 |
| Component name | Partial | `[DEBUG]` prefix | `[WebSocket]` 等追加 |
| Log level filter | Missing | なし | FB_LOG_LEVEL環境変数 |

**Gap Type**: Missing - 新規ユーティリティ作成

---

### Requirement 9: パフォーマンス監視

| Item | Status | Current Asset | Gap |
|------|--------|---------------|-----|
| API timing | Missing | なし | fetch wrapper with timing |
| WS latency | Missing | なし | メッセージタイムスタンプ比較 |
| Slow API warning | Missing | なし | 1秒超過警告 |
| /api/metrics endpoint | Missing | なし | Rust handler追加 |
| Connection tracking | Missing | なし | AppState拡張 |

**Existing Code References**:
- `demo/gui/src/web/mod.rs:34-41` - AppState struct (拡張ポイント)
- `demo/gui/src/web/handlers.rs` - 既存endpoint handlers

**Gap Type**: Missing - 新規機能追加（Rust + JS）

---

### Requirement 10: CSP最適化

| Item | Status | Current Asset | Gap |
|------|--------|---------------|-----|
| CSP header | ✅ Implemented | `build_csp_header()` in mod.rs | なし |
| unsafe-inline (style) | In Use | style-src 'unsafe-inline' | 要検討（inline style必要性） |
| Vendor only | ✅ Implemented | `/vendor/` directory | なし |
| CSP violation logging | Partial | `CSP_DEBUG` listener | WARN level統合 |
| Nonces | Missing | なし | inline script用nonce |

**Existing Code References**:
- `demo/gui/src/web/mod.rs:145-161` - CSP header builder
- `demo/gui/static/app.js:9-26` - CSP violation listener

**Gap Type**: Enhancement - 既存設定の厳格化

---

### Requirement 11: テストカバレッジ強化

| Item | Status | Current Asset | Gap |
|------|--------|---------------|-----|
| Endpoint unit tests | ✅ Implemented | 23 tests in handlers.rs | カバレッジ確認必要 |
| WebSocket tests | ✅ Implemented | 33 tests in websocket.rs | カバレッジ確認必要 |
| Integration tests | Partial | 一部あり | API-WS連携テスト追加 |
| Error path tests | Partial | 一部あり | 網羅性確認 |
| 80% coverage | Unknown | 未計測 | tarpaulin実行必要 |

**Existing Code References**:
- `demo/gui/src/web/handlers.rs` - 23 test functions
- `demo/gui/src/web/websocket.rs` - 33 test functions
- `demo/gui/src/web/mod.rs` - 10 test functions

**Gap Type**: Verification + Enhancement

---

### Requirement 12: アクセシビリティ

| Item | Status | Current Asset | Gap |
|------|--------|---------------|-----|
| ARIA labels | ✅ Implemented | 多数のaria-*属性 | 網羅性確認 |
| Live regions | ✅ Implemented | `aria-live="polite"` on toast | なし |
| Color contrast | Unknown | 未検証 | 4.5:1比率確認 |
| Chart alt text | Missing | なし | canvas代替テキスト |
| Reduce motion | ✅ Implemented | `prefers-reduced-motion` CSS | なし |
| Table headers | Partial | 一部あり | scope属性確認 |

**Existing Code References**:
- `demo/gui/static/index.html:26` - `role="alert" aria-live="polite"`
- `demo/gui/static/style.css:97` - `@media (prefers-reduced-motion: reduce)`

**Gap Type**: Verification + Minor Enhancement

---

### Requirement 13: テーマ動作安定化

| Item | Status | Current Asset | Gap |
|------|--------|---------------|-----|
| localStorage persist | ✅ Implemented | `localStorage.setItem('theme', ...)` | なし |
| Theme on load | ✅ Implemented | `loadTheme()` function | なし |
| Auto mode | ✅ Implemented | `data-mode="auto"` button | 動作検証 |
| System preference | ✅ Implemented | `prefers-color-scheme` listener | 即時反映確認 |
| No reload | ✅ Implemented | CSS class toggle | なし |
| Chart theming | Unknown | 未検証 | Chart.js色設定確認 |

**Existing Code References**:
- `demo/gui/static/app.js:766-771` - Theme persistence
- `demo/gui/static/app.js:3660-3703` - Theme management
- `demo/gui/static/style.css:56-95` - Light/OLED theme CSS

**Gap Type**: Verification - ほぼ完成、検証のみ

---

## Implementation Approach Options

### Option A: Extend Existing Components

**対象要件**: Req 2, 3, 4, 5, 6, 7, 10, 12, 13

- `app.js`内の既存関数を拡張
- `showToast()`にリトライボタン追加
- `connectWebSocket()`にbackoff追加
- スケルトンDOMの追加

**Trade-offs**:
- ✅ ファイル数増加なし
- ✅ 既存パターン活用
- ❌ app.jsの肥大化（既に350KB超）
- ❌ 関心の分離が困難

### Option B: Create New Components

**対象要件**: Req 1, 8, 9

- `logger.js` - 構造化ログユーティリティ
- `performance.js` - パフォーマンス監視
- `demo/gui/src/web/metrics.rs` - メトリクスAPI

**Trade-offs**:
- ✅ 明確な責務分離
- ✅ 独立テスト可能
- ❌ ファイル数増加
- ❌ 既存コードへの統合コスト

### Option C: Hybrid Approach (推奨)

**Phase 1 - 新規ユーティリティ作成**:
- `logger.js` 作成（Req 1, 8統合）
- `demo/gui/src/web/metrics.rs` 作成（Req 9）

**Phase 2 - 既存コード拡張**:
- Zoom機能完成（Req 2）
- WebSocket強化（Req 4）
- エラーハンドリング改善（Req 3）

**Phase 3 - 品質検証・改善**:
- アクセシビリティ検証（Req 12）
- テストカバレッジ確認（Req 11）
- テーマ動作検証（Req 13）

**Trade-offs**:
- ✅ 段階的実装でリスク軽減
- ✅ 新規と拡張のバランス
- ❌ フェーズ間の整合性管理必要

---

## Effort & Risk Assessment

| Requirement | Effort | Risk | Justification |
|-------------|--------|------|---------------|
| Req 1: Debug logging | S (1-2日) | Low | 単純なラッパー作成 |
| Req 2: Zoom completion | S (1-2日) | Low | プレースホルダ埋め |
| Req 3: Error handling | M (3-4日) | Low | UI拡張 |
| Req 4: WebSocket stability | M (3-4日) | Medium | 状態管理複雑 |
| Req 5: Loading state | S (1-2日) | Low | CSS活用 |
| Req 6: Responsive | S (1日) | Low | 検証・微調整 |
| Req 7: Keyboard nav | S (1-2日) | Low | 部分追加 |
| Req 8: Structured logging | S (1-2日) | Low | Loggerクラス |
| Req 9: Performance | M (3-4日) | Medium | Rust+JS両方 |
| Req 10: CSP | S (1日) | Medium | 既存動作への影響 |
| Req 11: Test coverage | M (2-3日) | Low | 追加テスト |
| Req 12: Accessibility | S (1-2日) | Low | 検証・修正 |
| Req 13: Theme | S (0.5日) | Low | 検証のみ |

**Total Effort**: M-L (1-2週間)
**Overall Risk**: Low-Medium

---

## Research Items for Design Phase

1. **Chart.js Zoom Plugin**: Exposure chartのズーム実装にChart.js zoom pluginが必要か、D3.jsに統一するか
2. **CSP Nonces**: Axumでのnonce生成・テンプレート注入方法
3. **Coverage Tool**: `cargo-tarpaulin`でのdemo_gui coverage計測方法
4. **Performance API**: ブラウザPerformance APIとの統合方法

---

## Recommendations for Design Phase

1. **優先実装順序**: Req 1→8→4→3→9→2→5→7→10→11→12→6→13
   - ログ基盤を先に整備し、以降の開発・デバッグを効率化

2. **新規ファイル**:
   - `demo/gui/static/logger.js` - フロントエンドロガー
   - `demo/gui/src/web/metrics.rs` - メトリクスAPI

3. **テスト戦略**:
   - 各要件に対応するテストを先に書く（TDDアプローチ）
   - Coverage計測を早期に実施

4. **破壊的変更の回避**:
   - 既存showToast APIを維持しつつ拡張
   - WebSocket接続ロジックは段階的移行
