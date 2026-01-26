# Implementation Plan

## Phase 1: VolCube基盤拡張

- [x] 1. VolQuote構造とマーケットデータ表現
- [x] 1.1 (P) ボラティリティクォートのデータ構造を実装する
  - マーケットクォート（bid/ask/mid）を保持する構造を定義
  - strike表現（絶対値、ATM相対、moneyness、log-moneyness）をenum化
  - クォートタイプ（normal、lognormal、shifted-lognormal）を定義
  - 通貨、underlying index、基準日をメタデータとして保持
  - serde serializationを実装
  - _Requirements: 2.2, 2.6_

- [x] 1.2 (P) クォートセットをVolCubeBuilderに渡せる形式に変換する
  - 複数クォートを集約するコレクション構造を実装
  - expiry/tenor/strikeでクォートをグループ化するヘルパーメソッドを追加
  - 既存VolCubeBuilderとの互換性を確保
  - _Requirements: 2.1, 2.3_

- [x] 2. VolCubeConfig拡張とCurve依存設定
- [x] 2.1 discount curveとprojection curveの参照設定を追加する
  - CurveName型でdiscount_curveとprojection_curveを設定可能にする
  - カリブレーション順序（expiry-first / tenor-first）を選択可能にする
  - _Requirements: 5.4, 5.8_

- [x] 2.2 通貨別デフォルトcurve設定を実装する
  - USD→SOFR、EUR→ESTR、JPY→TONAのデフォルトマッピングを定義
  - `default_for_currency()`ファクトリメソッドを追加
  - _Requirements: 5.10_

- [x] 3. 補間器フレームワーク拡張
- [x] 3.1 (P) Flat補間器を実装する
  - 最近傍グリッド点の値を返すシンプルな補間を実装
  - VolCubeInterpolator traitを実装
  - enum-based static dispatchに組み込む
  - _Requirements: 3.3, 3.6_

- [x] 3.2 (P) Linear補間器を実装する
  - 各軸方向の線形補間を実装
  - 3次元補間のためのtrilinear interpolationを実装
  - VolCubeInterpolator traitを実装
  - _Requirements: 3.3, 3.6_

- [x] 3.3 SABRパラメータ軸別補間設定を追加する
  - α、β、ρ、νの各パラメータをどの軸で補間するか設定可能にする
  - VolCubeConfigに補間設定フィールドを追加
  - _Requirements: 3.4, 3.5_

- [x] 4. SABRカリブレーション機能拡張
- [x] 4.1 既存SabrCalibratorのβ固定モードを確認・拡張する
  - β=0、0.5、1.0等の固定値設定をサポート
  - 既存実装の再利用を優先し、不足機能のみ追加
  - _Requirements: 4.2, 9.2_

- [x] 4.2 カリブレーション診断データを拡張する
  - 各スライスの残差、反復回数、最終パラメータ値を構造化
  - 収束状態（成功/失敗/警告）を明示
  - パラメータ境界違反の詳細情報を含める
  - _Requirements: 4.4, 4.5, 4.7_

- [x] 4.3 Breeden-Litzenbergerによるarbitrage-free検証を統合する
  - カリブレーション後にsmileのarbitrage条件を検証
  - 違反時は警告を発行（エラーではない）
  - 検証結果を診断データに含める
  - _Requirements: 4.6_

- [x] 5. IR商品定義の拡張
- [x] 5.1 (P) underlying schedule自動生成機能を追加する
  - Swaptionからunderlying swapのscheduleを自動構築
  - CapFloorからunderlying capのscheduleを自動構築
  - 無効なstrike/expiry/tenor組み合わせでInstrumentErrorを返す
  - _Requirements: 1.5, 1.6_

- [x] 5.2 (P) EUR ESTR swaption conventionを追加する
  - 既存EUR EURIBOR conventionを参考にESTR版を定義
  - payment frequency、day count、settlement conventionを設定
  - _Requirements: 1.3_

## Phase 2: 統合依存グラフとカリブレーションエンジン

- [x] 6. VolCubeカリブレーションエンジン
- [x] 6.1 エンジンの基本構造を実装する
  - instrument listとVolCube設定を入力として受け取る
  - カリブレーション結果として診断データ付きCalibratedVolCubeを返す
  - 既存SabrCalibratorをper-sliceで呼び出すループを実装
  - _Requirements: 5.1, 5.2, 5.6, 9.1, 9.3_

- [x] 6.2 Curve依存解決とforward rate計算を実装する
  - CurveSetから指定されたdiscount/projection curveを取得
  - 各(expiry, tenor)でforward swap rateを計算
  - Curve未発見時のエラーハンドリング
  - _Requirements: 5.3, 5.9_

- [x] 6.3 進捗報告機能を実装する
  - callback/channel経由でカリブレーション進捗を通知
  - 現在のスライス、総スライス数、反復回数、残差を報告
  - _Requirements: 5.5_

- [x] 6.4 VolatilitySurface traitを実装する
  - CalibratedVolCubeがpricing contextで使用可能になるようtraitを実装
  - volatility(expiry, tenor, strike)メソッドを公開
  - _Requirements: 5.7_

- [x] 7. CalibrationGraph（Curve→VolCube依存管理）
- [x] 7.1 依存グラフのノードとエッジ管理を実装する
  - CurveノードとVolCubeノードを定義
  - child→parent依存関係とparent→children逆依存を管理
  - カリブレーション状態（Pending/Computing/Calibrated/Stale）を追跡
  - _Requirements: 6.12_

- [x] 7.2 トポロジカルソートによるカリブレーション順序決定を実装する
  - Kahn's algorithmでカリブレーション順序を計算
  - 循環依存を検出してエラーを返す
  - 依存するCurveを先にカリブレーションする順序を保証
  - _Requirements: 6.9_

- [x] 7.3 依存Curveの自動カリブレーションを実装する
  - VolCube要求時に依存Curveが未カリブレーションなら自動実行
  - MarketProviderと連携して遅延カリブレーション
  - _Requirements: 6.8, 6.10_

- [x] 8. VolLazyEvaluator（遅延評価とキャッシュ）
- [x] 8.1 スライス単位キャッシュを実装する
  - expiry-tenorスライス単位でカリブレーション結果をキャッシュ
  - RwLock<HashMap>によるthread-safe実装
  - 同一座標への複数回アクセスでキャッシュから返す
  - _Requirements: 6.2, 6.3, 6.4_

- [x] 8.2 lazy initialization patternを実装する
  - 必要時のみカリブレーションを実行
  - double-check lockingで並行アクセスを最適化
  - IrsLazyEvaluatorパターンを踏襲
  - _Requirements: 6.1, 6.7_

- [x] 8.3 Quote更新時のキャッシュ無効化を実装する
  - 入力market quotes更新時に影響範囲を特定
  - QuoteUpdateListener traitでカスケード無効化
  - Stale状態の管理と再カリブレーショントリガー
  - _Requirements: 6.5_

- [x] 8.4 キャッシュメトリクスを実装する
  - ヒット率、ミス率、無効化回数、カリブレーション回数を追跡
  - メモリ使用量の概算を提供
  - _Requirements: 6.6_

- [x] 9. MarketProvider拡張
- [x] 9.1 VolCubeキャッシュをMarketProviderに統合する
  - 既存のCurveキャッシュと同様のパターンでVolCubeを管理
  - get_volcube(currency, index, config)メソッドを追加
  - Arc-wrapped lazy evaluationを適用
  - _Requirements: 6.9_

## Phase 3: AAD（自動微分）統合

- [ ] 10. AAD基盤とGraphExtractable実装
- [x] 10.1 VolCubeにGraphExtractable traitを実装する
  - カリブレーショングラフ（VolQuotes→SABRParams→InterpolatedVol）を抽出
  - D3.js互換のDAG形式でエクスポート可能にする
  - _Requirements: 7.3, 7.4_

- [x] 10.2 T: Float ジェネリクスでAAD互換性を確保する
  - 全数値計算をDualNumber互換の型で実装
  - Enzyme AADモードでの実行を可能にする
  - _Requirements: 7.1_

- [ ] 11. Vega計算（∂Price/∂VolQuote）
- [x] 11.1 adjoint modeによるVega計算を実装する
  - VolQuote変動に対するprice感応度を計算
  - 各(expiry, tenor, strike)点でのVega gridを出力
  - _Requirements: 7.2, 7.5_

- [x] 11.2 forward modeによるVega計算を実装する
  - 特定VolQuoteへの感応度を効率的に計算
  - adjoint modeとの結果一致を検証
  - _Requirements: 7.5_

- [x] 12. Curve経由の間接感応度
- [x] 12.1 CurveQuote→Price完全パスのAADグラフを構築する
  - CurveQuote→CurveCalibration→ForwardRate→VolCubeCalibration→Priceのパスを確立
  - CalibrationGraphと計算グラフを連携
  - _Requirements: 7.8, 6.11_

- [x] 12.2 ∂SwaptionPrice/∂CurveQuoteの間接感応度を計算する
  - Curve経由のVolCubeへの影響を追跡
  - Vegaと同時に計算可能なAADパスを構築
  - _Requirements: 7.9, 7.10_

- [x] 13. AAD検証とsmooth approximation
- [x] 13.1 bump-and-revalueとのクロス検証を実装する
  - 数値微分との比較検証機能を追加
  - 許容誤差範囲内の一致を確認
  - _Requirements: 7.6_

- [x] 13.2 不連続点でのsmooth approximationを適用する
  - 微分が不連続点を通過する場合の平滑化を実装
  - 既存pricer_core::math::smoothingを活用
  - _Requirements: 7.7_

## Phase 4: WebApp統合とデータローダー

- [x] 14. VolSurfaceLoader
- [x] 14.1 (P) CSV形式のswaption/capfloor vol quoteローダーを実装する
  - expiry、tenor、strike、vol（bid/ask/mid）カラムをパース
  - 行番号付きパースエラーを返す
  - demo/data/input/volsurface/ディレクトリ規約に従う
  - _Requirements: 10.1, 10.2, 10.3, 10.5, 10.6_

- [x] 14.2 (P) JSON形式のvol quoteローダーを実装する
  - swaption vol quote JSONをパース
  - capfloor vol quote JSONをパース
  - ファイル未発見時のLoaderErrorハンドリング
  - _Requirements: 10.1, 10.2, 10.4_

- [x] 14.3 VolQuoteSet変換を実装する
  - ロード済みデータをVolCubeBuilderに渡せる型に変換
  - strike type、quote typeの自動判別
  - _Requirements: 10.7_

- [x] 15. WebApp APIエンドポイント
- [x] 15.1 /api/volcube/calibrate POSTエンドポイントを実装する
  - 通貨とunderlying indexを受け取りカリブレーションを実行
  - SABRパラメータグリッドと診断データを返す
  - エラー発生時は詳細情報を含むエラーレスポンスを返す
  - _Requirements: 8.1, 8.4, 8.7_

- [x] 15.2 3Dサーフェス可視化用データを提供する
  - expiry×tenor×strike smileのグリッドデータを返す
  - market quote vs fitted volの比較データを提供
  - plotly.js互換の形式でデータを構造化
  - _Requirements: 8.3, 8.5_

- [x] 15.3 Breeden-Litzenberger密度関数エンドポイントを追加する
  - 指定expiry/tenorでの確率密度を計算
  - 可視化用のデータポイントを返す
  - _Requirements: 8.6_

- [x] 16. WebApp UI
- [x] 16.1 通貨選択UIを実装する
  - USD/EUR/JPYの選択コンポーネントを追加
  - curve-builder-webappのUIパターンに従う
  - _Requirements: 8.2, 8.8_

- [x] 16.2 3Dサーフェス可視化を実装する
  - plotly.jsで3Dボラティリティサーフェスを描画
  - expiry/tenor/strikeの各軸でインタラクティブに操作可能
  - _Requirements: 8.3_

- [x] 16.3 SABRパラメータグリッドと比較チャートを実装する
  - カリブレーション済みSABRパラメータをテーブル表示
  - market vs fitted volの2D比較チャートを描画
  - _Requirements: 8.4, 8.5_

## Phase 5: 統合とコード整理

- [x] 17. エンドツーエンド統合テスト
- [x] 17.1 Curve→VolCubeカリブレーションフローをテストする
  - 入力データからカリブレーション完了までのフローを検証
  - 依存Curveの自動カリブレーションを確認
  - _Requirements: 5.9, 6.8, 6.10_

- [x] 17.2 Quote更新→キャッシュ無効化をテストする
  - market quote更新時のカスケード無効化を検証
  - 再カリブレーションの正常動作を確認
  - _Requirements: 6.5_

- [x] 17.3 AAD Vega計算の精度をテストする
  - bump-and-revalueとの一致を検証
  - 許容誤差範囲内であることを確認
  - _Requirements: 7.6_

- [x] 18. コード整理と不要コード削除
- [x] 18.1 新実装により不要になったコードを特定・削除する
  - 影響範囲分析を実施
  - 依存コードを更新
  - deprecated APIは即座に削除（#[deprecated]属性不使用）
  - _Requirements: 9.4, 9.5, 9.6_

- [x] 18.2 未使用import・dead codeを除去する
  - cargo clippyで警告を確認
  - 全ての未使用コードを削除
  - _Requirements: 9.7_

---

## 要件カバレッジマトリクス

| 要件 | タスク |
|------|--------|
| 1 | 5.1, 5.2 |
| 2 | 1.1, 1.2 |
| 3 | 3.1, 3.2, 3.3 |
| 4 | 4.1, 4.2, 4.3 |
| 5 | 2.1, 2.2, 6.1, 6.2, 6.3, 6.4 |
| 6 | 7.1, 7.2, 7.3, 8.1, 8.2, 8.3, 8.4, 9.1, 12.1 |
| 7 | 10.1, 10.2, 11.1, 11.2, 12.1, 12.2, 13.1, 13.2 |
| 8 | 15.1, 15.2, 15.3, 16.1, 16.2, 16.3 |
| 9 | 4.1, 6.1, 18.1, 18.2 |
| 10 | 14.1, 14.2, 14.3 |

---

_Generated: 2026-01-25_
