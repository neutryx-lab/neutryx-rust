# Implementation Plan

## Task 1: Logger基盤の構築

- [x] 1.1 (P) ConfigManagerの実装
  - 環境変数（FB_DEBUG_MODE、FB_LOG_LEVEL）を読み取る設定管理機能を実装する
  - HTMLテンプレート経由で注入された設定値を解析する
  - デフォルト値（debugMode: false, logLevel: 'INFO'）を提供する
  - window.__FB_CONFIG__としてグローバルアクセス可能にする
  - _Requirements: 1.1, 8.5_

- [x] 1.2 Logger Utilityの実装
  - DEBUG/INFO/WARN/ERRORの4つのログレベルをサポートする構造化ログ機能を実装する
  - すべてのログメッセージにISO形式タイムスタンプを自動付与する
  - コンポーネント名（[WebSocket]、[API]、[UI]等）をprefix付与する
  - 設定されたログレベル以下のログを抑制するフィルタリング機能を実装する
  - window.FB_Loggerとしてグローバルアクセス可能にする
  - Task 1.1のConfigManagerに依存
  - _Requirements: 8.1, 8.2, 8.3, 8.4_

- [x] 1.3 デバッグモード制御の実装
  - FB_DEBUG_MODE環境変数によるデバッグログ出力制御を実装する
  - 本番環境（FB_DEBUG_MODE=false）ではDEBUGレベルログを完全抑制する
  - アプリケーション初期化時に現在のデバッグモード設定を1回だけログ出力する
  - API応答や内部状態などの機密データがDEBUGモード以外で出力されないことを保証する
  - _Requirements: 1.2, 1.3, 1.4, 1.5_

- [x] 1.4 既存console.logの置換
  - app.js内の14箇所以上の直接console.log呼び出しをLogger APIに置換する
  - [DEBUG]プレフィックスのログはLogger.debug()に変換する
  - エラーログはLogger.error()、警告はLogger.warn()に適切に分類する
  - CSP violation listenerのログをLogger.warn()に統合する
  - _Requirements: 8.1, 1.2_

## Task 2: WebSocket安定性強化

- [x] 2.1 Exponential Backoff再接続の実装
  - WebSocket切断時にExponential backoff再接続を実装する（1s→2s→4s→8s→16s→30s max）
  - 再接続試行回数を追跡し、最大10回に制限する
  - 10回超過時は手動介入を要求するUI状態に遷移する
  - 再接続成功時にretryCountとretryDelayをリセットする
  - _Requirements: 4.1, 4.6_

- [x] 2.2 Heartbeat機能の実装
  - 30秒間隔でWebSocketにpingメッセージを送信するheartbeat機能を実装する
  - ping送信後10秒以内にpong応答がない場合、接続断と判定する
  - heartbeatタイマーを接続確立時に開始し、切断時にクリアする
  - _Requirements: 4.2, 4.3_

- [x] 2.3 接続状態UIの強化
  - 切断中は「オフライン」インジケーターをヘッダーに可視表示する
  - 再接続成功時は「接続が復旧しました」Toastを短時間表示する
  - 再接続試行中は「再接続中...」状態を表示する
  - _Requirements: 4.4, 4.5_

## Task 3: エラーハンドリング強化

- [x] 3.1 Toast通知の拡張
  - 既存showToast関数にactionパラメータ（リトライボタン）を追加する
  - リトライボタンクリック時のコールバック実行機能を実装する
  - ローディングインジケーター表示機能を追加する
  - ToastInstanceにdismiss()とsetLoading()メソッドを実装する
  - _Requirements: 3.4, 3.5_

- [x] 3.2 エラーメッセージの日本語化
  - ネットワークエラー時は「ネットワークエラー: サーバーに接続できません」を表示する
  - HTTP 5xxエラー時は「サーバーエラー: しばらく経ってから再試行してください」を表示する
  - HTTP 4xxエラー時はコンテキスト適切なエラーメッセージを表示する
  - エラーコードと日本語メッセージのマッピングテーブルを実装する
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 3.3 FetchWrapper with Retry実装
  - Exponential backoff自動リトライ機能を持つfetchWithRetry関数を実装する
  - 最大3回のリトライ、デフォルト1秒の初期遅延を設定する
  - リトライ中はToastにローディング状態を表示する
  - リトライ試行ごとにonRetryコールバックを呼び出す
  - Task 3.1のToast拡張に依存
  - _Requirements: 3.6, 3.5_

- [x] 3.4 WebSocket切断時の再接続インジケーター
  - WebSocket接続断時に自動再接続インジケーターを表示する
  - 再接続試行回数と次回試行までの秒数を表示する
  - Task 2の再接続ロジックと連携する
  - _Requirements: 3.7_

## Task 4: Exposure Chartズーム機能

- [x] 4.1 (P) ズーム制御機能の実装
  - ズームイン時にビューポートスケールを20%増加する機能を実装する
  - ズームアウト時にビューポートスケールを20%減少する機能を実装する
  - リセット時に100%スケールと中央位置に戻す機能を実装する
  - Chart.jsのスケール操作でズーム制御を実現する
  - _Requirements: 2.1, 2.2, 2.3_

- [x] 4.2 (P) ズーム範囲制限とインジケーター
  - ズーム範囲を50%〜200%に制限する境界チェックを実装する
  - ズーム操作時に現在のビュー中心点を維持する
  - ズームレベル変更時にインジケーター表示を更新する
  - 既存initZoomControls関数のプレースホルダを実際の実装に置換する
  - _Requirements: 2.4, 2.5, 2.6_

## Task 5: ローディング状態の明確化

- [x] 5.1 (P) スケルトンDOMの追加
  - Portfolio Panel用のスケルトンローダーDOMを追加する
  - Risk Metrics Panel用の各メトリックカードスケルトンDOMを追加する
  - Exposure Chart用のスケルトンDOMを追加する
  - 既存のskeleton CSSクラスを適用する
  - _Requirements: 5.1, 5.2, 5.3_

- [x] 5.2 ローディング状態管理の実装
  - state.loadingオブジェクトでパネル別ローディング状態を管理する
  - 各fetchXxx関数の前後でローディング状態を更新する
  - ローディング状態変更時にスケルトン/コンテンツの表示切り替えを行う
  - スケルトンからコンテンツへのスムーズなトランジションを実装する
  - _Requirements: 5.6_

- [x] 5.3 タイムアウトメッセージの実装
  - データ読み込みが3秒を超過した場合に「読み込みに時間がかかっています...」メッセージを表示する
  - 複数API同時呼び出し時のプログレスインジケーターを表示する
  - タイムアウト状態の解除（データ到着時）を適切に処理する
  - _Requirements: 5.4, 5.5_

## Task 6: パフォーマンス監視

- [x] 6.1 PerformanceMetrics状態の実装（Backend）
  - AppStateにPerformanceMetrics構造体を追加する
  - API応答時間の累積記録機能を実装する（portfolio、exposure、risk、graph）
  - WebSocket接続数追跡機能を実装する（AtomicU32）
  - 直近1000件に配列サイズを制限するメモリ管理を実装する
  - _Requirements: 9.5_

- [x] 6.2 API応答時間計測の実装
  - 各REST endpointハンドラーで応答時間を計測する
  - 計測結果をPerformanceMetricsに記録する
  - 1秒超過時にWARNレベルログを出力する
  - Task 6.1のPerformanceMetrics状態に依存
  - _Requirements: 9.1, 9.3_

- [x] 6.3 /api/metrics endpointの実装
  - GET /api/metricsエンドポイントを実装する
  - API応答時間統計（平均ms）をJSON形式で返却する
  - WebSocket接続数とメッセージレイテンシを返却する
  - サーバー起動時間（uptime_seconds）を返却する
  - _Requirements: 9.4_

- [x] 6.4 WebSocketメッセージレイテンシ計測
  - WebSocketメッセージのレイテンシを計測する
  - ping/pong往復時間を記録する
  - メトリクスAPIにレイテンシ統計を含める
  - _Requirements: 9.2_

## Task 7: レスポンシブデザイン検証・改善

- [x] 7.1 (P) タッチターゲットサイズの検証と修正
  - すべてのインタラクティブ要素が44x44ピクセル以上のタッチターゲットサイズを持つことを検証する
  - 不足している要素のpadding/sizeを調整する
  - モバイルビューポート（768px未満）でのボタンサイズを確認する
  - _Requirements: 6.3_

- [x] 7.2 (P) サイドバー折りたたみの検証
  - 768px未満のビューポートでサイドバーがアイコンのみモードに折りたたまれることを検証する
  - 折りたたみ/展開のトランジションを確認する
  - 必要に応じてメディアクエリを調整する
  - _Requirements: 6.1_

- [x] 7.3 (P) カードスタッキングの検証
  - 768px未満のビューポートでダッシュボードカードが垂直にスタックされることを検証する
  - グリッドレイアウトのレスポンシブ動作を確認する
  - _Requirements: 6.2_

- [x] 7.4 ポートフォリオテーブルの横スクロール実装
  - 狭い画面でPortfolio Tableが水平スクロール可能であることを確認する
  - Trade ID列を固定（sticky）にしてスクロール時も表示を維持する
  - _Requirements: 6.5_

- [x] 7.5 (P) モバイル向けサマリー表示
  - 480px未満のビューポートで非必須チャートを非表示にする
  - サマリーメトリクスのみを表示するモバイルビューを実装する
  - _Requirements: 6.4_

## Task 8: キーボードナビゲーション強化

- [x] 8.1 (P) フォーカスインジケーターの強化
  - すべてのフォーカス可能要素に可視的なフォーカスインジケーターを確保する
  - フォーカスリングのスタイルを統一する（outline/box-shadow）
  - 既存のtabindex属性を検証し、論理的な順序を確保する
  - _Requirements: 7.4, 7.1_

- [x] 8.2 テーブル行Enterキー処理の実装
  - Portfolio Tableの行にフォーカスがある状態でEnterキーを押すと取引詳細ビューを開く
  - 行へのtabindex/aria属性を追加してキーボードアクセス可能にする
  - _Requirements: 7.3_

- [x] 8.3 (P) モーダルフォーカストラップの検証
  - モーダルオープン時にフォーカスがモーダル内に閉じ込められることを検証する
  - モーダルクローズ時にトリガー要素にフォーカスが戻ることを検証する
  - Escapeキーでモーダルがクローズすることを検証する
  - 既存trapFocus実装の動作確認
  - _Requirements: 7.5, 7.6, 7.2_

## Task 9: アクセシビリティ（WCAG 2.1 AA準拠）

- [x] 9.1 (P) ARIAラベルの網羅性確認と追加
  - すべてのインタラクティブ要素にARIAラベルが付与されていることを検証する
  - 不足している要素にaria-label/aria-labelledbyを追加する
  - アイコンボタンに適切なラベルを付与する
  - _Requirements: 12.1_

- [x] 9.2 (P) ARIAライブリージョンの実装
  - 動的コンテンツ更新にaria-live="polite"を適用する
  - Toast通知、接続状態表示にライブリージョンを設定する
  - リスクメトリクス更新時のスクリーンリーダー通知を実装する
  - _Requirements: 12.2_

- [x] 9.3 (P) カラーコントラストの検証と修正
  - すべてのテキストが4.5:1以上のコントラスト比を持つことを検証する
  - 不足している要素の色を調整する
  - ダーク/ライトモード両方でコントラストを確認する
  - _Requirements: 12.3_

- [x] 9.4 (P) チャート代替テキストの実装
  - Exposure Chartにデータの代替テキスト説明を提供する
  - aria-describedbyでチャート説明を関連付ける
  - スクリーンリーダー向けのサマリーテキストを生成する
  - _Requirements: 12.4_

- [x] 9.5 (P) テーブルヘッダーアクセシビリティ
  - Portfolio Tableの列ヘッダーに`<th scope="col">`を適用する
  - 行ヘッダーがある場合は`<th scope="row">`を適用する
  - テーブルにcaption/aria-labelを追加する
  - _Requirements: 12.6_

- [x] 9.6 (P) Reduce Motion対応の検証
  - システムレベルの「Reduce Motion」設定が尊重されることを検証する
  - prefers-reduced-motion CSSが適切に機能することを確認する
  - _Requirements: 12.5_

## Task 10: テーマ動作安定化

- [x] 10.1 (P) テーマ永続化の検証
  - ユーザーが選択したテーマモードがlocalStorageに正しく保存されることを検証する
  - ページ読み込み時に以前選択したテーマが適用されることを検証する
  - _Requirements: 13.1, 13.2_

- [x] 10.2 (P) Autoモードとシステム設定連携の検証
  - 「Auto」モード選択時にシステムのカラースキーム設定に従うことを検証する
  - Autoモード中にシステム設定が変更された場合、テーマが即時更新されることを検証する
  - _Requirements: 13.3, 13.4_

- [x] 10.3 (P) テーマ切り替えの動作検証
  - テーマ変更がページリロードなしで適用されることを検証する
  - チャートを含むすべてのUIコンポーネントがテーマを尊重することを検証する
  - Chart.jsの色設定がテーマに連動することを確認する
  - _Requirements: 13.5, 13.6_

## Task 11: CSP最適化

- [x] 11.1 (P) インラインスクリプトの検証
  - 現在のインラインスクリプト使用状況を監査する
  - script-src 'self'ポリシーが維持されていることを確認する
  - 新規追加するlogger.jsがvendorディレクトリから正しく読み込まれることを確認する
  - _Requirements: 10.1, 10.2_

- [x] 11.2 (P) CSP違反ログのLogger統合
  - CSP違反listenerのログ出力をLogger.warn()に統合する
  - 違反詳細（blockedURI、violatedDirective等）を構造化ログとして出力する
  - _Requirements: 10.4_

- [x] 11.3 report-uri設定の検討
  - CSP違反レポートを受信する設定可能なエンドポイントを検討する
  - FB_CSP_REPORT_URI環境変数でreport-uriを設定可能にする
  - 実装範囲を決定し、必要であれば将来タスクとして記録する
  - _Requirements: 10.3_

- [x] 11.4* nonce導入の検討
  - 必要なインラインスクリプトのnonce対応を検討する
  - サーバー側でのnonce生成とHTMLテンプレート注入方式を設計する
  - 実装複雑度を評価し、必要であれば将来タスクとして記録する
  - _Requirements: 10.5_

## Task 12: テストカバレッジ強化

- [x] 12.1 Backend単体テストの追加
  - get_metrics handlerの単体テストを追加する
  - PerformanceMetrics記録/取得のテストを追加する
  - WebSocket接続追跡のテストを追加する
  - _Requirements: 11.1, 11.2_

- [x] 12.2 エラーパステストの追加
  - ネットワーク障害シナリオのテストを追加する
  - 不正リクエストに対するレスポンステストを追加する
  - WebSocket切断/再接続シナリオのテストを追加する
  - _Requirements: 11.5_

- [x] 12.3 API-WebSocket連携テストの追加
  - /api/metricsがWebSocket接続数を正しく反映するテストを追加する
  - 複数同時接続時のメトリクス整合性テストを追加する
  - _Requirements: 11.4_

- [x] 12.4 カバレッジ計測と確認
  - cargo-tarpaulinを使用してdemo/gui/src/web/のカバレッジを計測する
  - 80%以上のカバレッジを達成していることを確認する
  - カバレッジ不足箇所を特定し、追加テストを作成する
  - _Requirements: 11.3_

## Task 13: 統合とHTML/Config更新

- [x] 13.1 index.html更新
  - logger.jsのscript読み込みをapp.jsより前に追加する
  - window.__FB_CONFIG__の設定注入スクリプトを追加する
  - 各パネルのスケルトンDOMを追加する
  - _Requirements: 1.1, 5.1, 5.2, 5.3_

- [x] 13.2 Backend Config注入の実装
  - index.html提供時にFB_DEBUG_MODE、FB_LOG_LEVEL環境変数を読み取る
  - HTMLテンプレートにwindow.__FB_CONFIG__を動的注入する
  - _Requirements: 1.1, 8.5_

- [x] 13.3 全機能の統合テスト
  - Logger、WebSocket、Toast、Fetch、Zoomの各機能が連携動作することを確認する
  - デバッグモードON/OFFでのログ出力を検証する
  - エラー発生時のリトライ→Toast→ログの一連のフローを検証する
  - _Requirements: 1.1, 1.2, 3.1, 4.1, 8.1_

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1 | 1.1, 1.2, 1.3, 1.4, 13.1, 13.2, 13.3 |
| 2 | 4.1, 4.2 |
| 3 | 3.1, 3.2, 3.3, 3.4, 13.3 |
| 4 | 2.1, 2.2, 2.3, 3.4, 13.3 |
| 5 | 5.1, 5.2, 5.3, 13.1 |
| 6 | 7.1, 7.2, 7.3, 7.4, 7.5 |
| 7 | 8.1, 8.2, 8.3 |
| 8 | 1.1, 1.2, 1.4, 13.2, 13.3 |
| 9 | 3.3, 6.1, 6.2, 6.3, 6.4 |
| 10 | 11.1, 11.2, 11.3, 11.4 |
| 11 | 12.1, 12.2, 12.3, 12.4 |
| 12 | 9.1, 9.2, 9.3, 9.4, 9.5, 9.6 |
| 13 | 10.1, 10.2, 10.3 |
