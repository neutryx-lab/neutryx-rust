# Requirements Document

## Project Description (Input)
FrictionalBankWebAppのPricerに、マーケットの金利スワップ価格からBootstrapを通して得られた金利スワップの時価とそれをBump法とAAD法でリスク計算をする機能(経過時間比較付き)を実装。

## Introduction

本仕様は、FrictionalBank WebAppのPricerタブに新機能「IRS Bootstrap & Risk」を追加するための要件を定義する。本機能は、マーケットで観測される金利スワップ（IRS）のPar Rateからイールドカーブをブートストラップ構築し、任意のIRSの時価評価（PV）を行い、さらにBump-and-Revalue法とAAD法の両方でリスク計算（カーブ感応度）を実行して経過時間を比較表示するものである。

既存の`frictionalbank-webapp-pricer`がBlack-Scholes的なシンプルなプライシングを提供するのに対し、本機能は実務的なイールドカーブ構築とAADの性能優位性を体験できるインタラクティブなデモを提供する。

## Requirements

### Requirement 1: マーケットデータ入力インターフェース

**Objective:** As a ユーザー, I want マーケットのIRS Par Rateを入力したい, so that 実際の市場データからイールドカーブを構築できる

#### Acceptance Criteria

1.1 The WebApp Pricer shall IRS Par Rate入力用のフォームを提供する（テナー: 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y）

1.2 When ユーザーがPar Rateを入力した場合, the WebApp Pricer shall 入力値を数値としてバリデーションする

1.3 If 入力値が不正（負の値、非数値）な場合, then the WebApp Pricer shall エラーメッセージを該当フィールドに表示する

1.4 The WebApp Pricer shall デフォルトのサンプルPar Rate（現実的な市場データ）をプリセットとして提供する

1.5 When プリセット選択ボタンがクリックされた場合, the WebApp Pricer shall サンプルPar Rateでフォームを自動入力する

1.6 The WebApp Pricer shall Par Rateを小数形式（例: 0.025 = 2.5%）およびパーセント形式（例: 2.5）の両方で入力可能とする

### Requirement 2: イールドカーブ・ブートストラップ

**Objective:** As a クオンツユーザー, I want 入力されたPar Rateからイールドカーブを構築したい, so that IRSの正確な時価評価が可能になる

#### Acceptance Criteria

2.1 When 「Bootstrap」ボタンがクリックされた場合, the Bootstrap API shall 入力されたPar Rateからディスカウント・ファクターを逐次計算する

2.2 The Bootstrap API shall 線形補間（ログDF補間）を使用してカーブポイント間を補間する

2.3 When ブートストラップが完了した場合, the Bootstrap API shall 構築されたカーブ情報（テナー、DF、ゼロレート）をレスポンスに含める

2.4 The Bootstrap API shall ブートストラップの処理時間をミリ秒単位でレスポンスに含める

2.5 If ブートストラップが収束しない場合, then the Bootstrap API shall エラーレスポンス（422）と収束失敗のテナーを返却する

2.6 The WebApp Pricer shall 構築されたカーブをグラフまたはテーブル形式で可視化する

### Requirement 3: IRS時価評価（プライシング）

**Objective:** As a トレーダー, I want 任意のIRSの時価を計算したい, so that ブートストラップされたカーブを使用した評価結果を確認できる

#### Acceptance Criteria

3.1 The WebApp Pricer shall IRSパラメータ入力フォームを提供する（想定元本、固定レート、期間、支払頻度）

3.2 When 「Price IRS」ボタンがクリックされた場合, the Pricing API shall ブートストラップされたカーブを使用してIRSのNPVを計算する

3.3 The Pricing API shall 固定レグPV、変動レグPVを個別に計算しレスポンスに含める

3.4 When プライシングが完了した場合, the Pricing API shall 計算時間をマイクロ秒単位でレスポンスに含める

3.5 The WebApp Pricer shall NPV結果を3桁区切りフォーマットで表示する

3.6 If ブートストラップが未実行の場合, then the WebApp Pricer shall プライシングボタンを無効化しメッセージを表示する

### Requirement 4: Bump法によるリスク計算

**Objective:** As a リスク管理者, I want Bump-and-Revalue法でカーブ感応度を計算したい, so that 従来手法の計算時間を把握できる

#### Acceptance Criteria

4.1 When 「Calculate Risk (Bump)」ボタンがクリックされた場合, the Risk API shall 各テナーのPar Rateを1bpずつバンプしてDeltaを計算する

4.2 The Risk API shall 全テナー（9ポイント）のDeltaを順次計算し、個別の計算時間を記録する

4.3 The Risk API shall 総計算時間（全テナーDelta）をレスポンスに含める

4.4 The WebApp Pricer shall 各テナーのDelta値を表形式で表示する

4.5 The WebApp Pricer shall DV01（1bp変動に対するPV変化の合計）を計算して表示する

4.6 When Bump計算が完了した場合, the WebApp Pricer shall 各テナーの計算時間をマイクロ秒単位で表示する

### Requirement 5: AAD法によるリスク計算

**Objective:** As a クオンツ, I want AAD法でカーブ感応度を計算したい, so that AADの計算効率を体験できる

#### Acceptance Criteria

5.1 When 「Calculate Risk (AAD)」ボタンがクリックされた場合, the Risk API shall AAD（随伴微分）を使用して全テナーのDeltaを一括計算する

5.2 The Risk API shall 単一の逆伝播で全テナーのDeltaを取得し、計算時間を記録する

5.3 The Risk API shall AADモードの総計算時間をレスポンスに含める

5.4 The WebApp Pricer shall AAD計算結果とBump計算結果を並列表示する

5.5 The Risk API shall AAD計算とBump計算の結果差分（相対誤差）を計算してレスポンスに含める

5.6 If AADとBumpの差分が許容誤差（1e-6）を超える場合, then the WebApp Pricer shall 警告メッセージを表示する

### Requirement 6: 経過時間比較表示

**Objective:** As a エンドユーザー, I want BumpとAADの計算時間を比較したい, so that AADの性能優位性を定量的に理解できる

#### Acceptance Criteria

6.1 The WebApp Pricer shall Bump法とAAD法の総計算時間を並列表示するパネルを提供する

6.2 The WebApp Pricer shall 速度比（Speedup Ratio = Bump時間 / AAD時間）を計算して表示する

6.3 When 両方のリスク計算が完了した場合, the WebApp Pricer shall 計算時間の比較バーチャートを表示する

6.4 The WebApp Pricer shall 速度比に応じた色分け表示を行う（AADが10倍以上速い場合は緑、5倍以上は黄、それ以下は赤）

6.5 The WebApp Pricer shall 統計情報（平均、最小、最大時間）を表示するオプションを提供する

6.6 When ユーザーが「Run Benchmark」ボタンをクリックした場合, the WebApp Pricer shall 同じ計算を複数回（10回）実行して統計を取得する

### Requirement 7: 結果表示とUI/UX

**Objective:** As a エンドユーザー, I want 計算結果を分かりやすく表示してほしい, so that 結果を直感的に理解できる

#### Acceptance Criteria

7.1 The WebApp Pricer shall 「IRS Bootstrap & Risk」専用のサブビューを既存Pricerタブ内に追加する

7.2 The WebApp Pricer shall ワークフローを3ステップで表示する（Step 1: Market Data入力、Step 2: Bootstrap、Step 3: Pricing & Risk）

7.3 The WebApp Pricer shall 各ステップの完了状態をインジケーターで表示する

7.4 When 計算中の場合, the WebApp Pricer shall ローディングスピナーを表示する

7.5 The WebApp Pricer shall モバイル・タブレット向けのレスポンシブレイアウトを提供する

7.6 The WebApp Pricer shall ライトテーマとダークテーマの両方に対応するスタイリングを適用する

### Requirement 8: WebSocket通知

**Objective:** As a システム, I want 計算完了をリアルタイムに通知したい, so that 他のクライアントとの連携が可能になる

#### Acceptance Criteria

8.1 When ブートストラップが完了した場合, the WebSocket Handler shall `bootstrap_complete`イベントをブロードキャストする

8.2 When リスク計算が完了した場合, the WebSocket Handler shall `risk_complete`イベント（メソッド種別含む）をブロードキャストする

8.3 The WebSocket Handler shall 計算結果サマリー（NPV、速度比）を通知データに含める

8.4 When エラーが発生した場合, the WebSocket Handler shall `calculation_error`イベントをブロードキャストする

### Requirement 9: バックエンドAPI設計

**Objective:** As a 開発者, I want RESTful APIを通じて機能にアクセスしたい, so that フロントエンドとバックエンドを疎結合にできる

#### Acceptance Criteria

9.1 The Backend shall `POST /api/bootstrap`エンドポイントを提供する（Par Rateリストを受け取りカーブを構築）

9.2 The Backend shall `POST /api/price-irs`エンドポイントを提供する（IRSパラメータとカーブIDを受け取りNPVを計算）

9.3 The Backend shall `POST /api/risk/bump`エンドポイントを提供する（Bump法でリスク計算）

9.4 The Backend shall `POST /api/risk/aad`エンドポイントを提供する（AAD法でリスク計算）

9.5 The Backend shall `POST /api/risk/compare`エンドポイントを提供する（両手法を実行して比較結果を返却）

9.6 The Backend shall すべてのレスポンスにcamelCase JSONシリアライゼーションを適用する

9.7 If バリデーションエラーが発生した場合, then the Backend shall HTTP 400と詳細なエラーメッセージを返却する

9.8 If 計算エラーが発生した場合, then the Backend shall HTTP 422と計算失敗の理由を返却する
