# Requirements Document

## Project Description (Input)
Demo内のWebAppのModel Calib画面を精緻化したい。まず、各Index毎にVolCUbeBuildに必要なInstrumentListをInputDataとしてdemo\data\inputに用意する。そのレートを入力可能として、任意のBuilderモデルを用いてカーブを構築する（Cube構築に必要なカーブ構築もCurveBuilder画面でやっているように行う）。構築されたカーブは任意のParmeterのセットとして確認可能だし、任意の軸の断面でスマイルカーブとしても、確率密度函数としても確認可能。

## Introduction

本仕様は、Demo WebAppのModel Calibration画面を拡張し、VolCube構築のための完全なワークフローを実装する。既存のCurve Builder機能と同様のUIパターンを踏襲しつつ、ボラティリティキューブ特有の3次元構造（Expiry × Tenor × Strike）を可視化・操作可能にする。

**対象ユーザー**: クオンツアナリスト、トレーダー、リスクマネージャー

**主要機能**:
- Index毎のVolCube用インストゥルメントデータ管理
- 複数のキャリブレーションモデル（SABR、SVI、Local Vol）のサポート
- 構築済みキューブのパラメータ表示・スマイル曲線・確率密度関数の可視化

## Requirements

### Requirement 1: VolCube用インストゥルメントデータ管理

**Objective:** As a クオンツアナリスト, I want 各Index毎にVolCube構築用のインストゥルメントリストを入力データとして管理する機能, so that キャリブレーションに必要な市場データを体系的に準備できる。

#### Acceptance Criteria
1. The Model Calib UI shall provide Index選択ドロップダウン（USD-SOFR-Swaption、EUR-ESTR-Swaption、JPY-TONA-Swaption、Equity-Options、FX-Options）。
2. When Indexが選択される, the Model Calib UI shall `demo/data/input/volcube/` から対応するJSONファイルを読み込み、インストゥルメントテーブルを表示する。
3. The input data files shall follow a standardised JSON format containing: `index`, `reference_date`, `instruments[]` with each instrument having `expiry`, `tenor`, `strike`, `implied_vol`, `forward`, `weight` fields。
4. When インストゥルメントデータファイルが存在しない, the Model Calib UI shall デフォルトテンプレートを生成してユーザーに通知する。
5. The Model Calib UI shall インストゥルメントテーブルの各セル（implied_vol、forward、weight）を直接編集可能にする。
6. When インストゥルメントデータが編集される, the Model Calib UI shall 変更フラグを表示し、保存・リセット機能を提供する。
7. The Model Calib UI shall インストゥルメントデータのCSV/JSONエクスポート・インポート機能を提供する。

### Requirement 2: 依存カーブ構築統合

**Objective:** As a クオンツアナリスト, I want VolCube構築に必要なイールドカーブを同一画面から構築・選択できる機能, so that カーブとボラティリティの一貫した市場データセットを使用できる。

#### Acceptance Criteria
1. The Model Calib UI shall 「依存カーブ」セクションを提供し、Forward計算に使用するイールドカーブを選択可能にする。
2. When 依存カーブが未構築の場合, the Model Calib UI shall Curve Builder画面へのリンクまたはインラインカーブ構築オプションを提供する。
3. The Model Calib UI shall 構築済みカーブのリスト（curveId、構築日時、インストゥルメント数）を表示する。
4. When カーブが選択される, the Model Calib UI shall そのカーブのディスカウントファクターをVolCubeのForward計算に使用する。
5. If 選択されたカーブの参照日がVolCubeデータと異なる, the Model Calib UI shall 警告メッセージを表示する。

### Requirement 3: VolCubeキャリブレーション設定

**Objective:** As a クオンツアナリスト, I want 複数のキャリブレーションモデルと詳細設定を選択できる機能, so that 資産クラスや市場状況に応じた最適なモデルを使用できる。

#### Acceptance Criteria
1. The Model Calib UI shall キャリブレーションモデル選択ドロップダウン（SABR、SVI、Local Volatility）を提供する。
2. When SABRモデルが選択される, the Model Calib UI shall SABR固有パラメータ設定（Beta固定値またはキャリブレーション、Shift値）を表示する。
3. The Model Calib UI shall 共通の高度な設定（補間方法、外挿方法、Strike軸タイプ、最適化手法、許容誤差、最大反復回数）を提供する。
4. The Model Calib UI shall 設定プリセットの保存・読み込み・削除機能を提供する。
5. When 「Calibrate」ボタンがクリックされる, the Model Calib UI shall `/api/volcube/calibrate` エンドポイントにリクエストを送信し、処理中インジケータを表示する。
6. If キャリブレーションがエラーを返す, the Model Calib UI shall 診断情報（収束状況、問題のあるインストゥルメント）を表示する。

### Requirement 4: キャリブレーション結果パラメータ表示

**Objective:** As a クオンツアナリスト, I want キャリブレーション結果をパラメータセットとして確認できる機能, so that モデルの適合度と各パラメータ値を評価できる。

#### Acceptance Criteria
1. When キャリブレーションが成功する, the Model Calib UI shall 結果パネルを表示し、モデルパラメータをテーブル形式で表示する。
2. The Model Calib UI shall SABRパラメータ（Alpha、Beta、Rho、Nu）を各(Expiry, Tenor)グリッドポイントで表示する。
3. The Model Calib UI shall 適合度メトリクス（RMSE、最大誤差、R²、反復回数、処理時間）を表示する。
4. The Model Calib UI shall 各インストゥルメントの市場vol vs モデルvolの比較テーブルを表示する。
5. The Model Calib UI shall パラメータテーブルのCSV/JSONエクスポート機能を提供する。
6. While パラメータが表示されている, the Model Calib UI shall Expiry/Tenorのフィルタリング・ソート機能を提供する。

### Requirement 5: スマイルカーブ可視化

**Objective:** As a トレーダー, I want 任意のExpiry/Tenor断面でスマイルカーブを表示できる機能, so that Strike方向のボラティリティ構造を視覚的に分析できる。

#### Acceptance Criteria
1. The Model Calib UI shall Expiry選択スライダーとTenor選択スライダーを提供する。
2. When Expiry/Tenorが選択される, the Model Calib UI shall 該当断面のスマイルカーブ（Strike vs Implied Vol）をChart.jsでプロットする。
3. The Model Calib UI shall 市場観測点（マーカー）とモデル曲線（線）を同一チャートに重ねて表示する。
4. The Model Calib UI shall 複数のExpiry/Tenor断面を同時に比較表示するオプションを提供する。
5. When Strike軸タイプが変更される, the Model Calib UI shall チャートを対応する軸（Absolute、Moneyness、Log-Moneyness、Delta）で再描画する。
6. The Model Calib UI shall チャートのPNG/SVGエクスポート機能を提供する。

### Requirement 6: 確率密度関数可視化

**Objective:** As a リスクマネージャー, I want 任意のExpiry/Tenor断面でリスクニュートラル確率密度関数を表示できる機能, so that 市場が織り込むリターン分布を分析できる。

#### Acceptance Criteria
1. The Model Calib UI shall 「Density」タブまたはトグルを提供し、確率密度表示モードに切り替え可能にする。
2. When 確率密度モードが有効, the Model Calib UI shall Breeden-Litzenberger法で計算された確率密度関数（Strike/Price vs Density）を表示する。
3. The Model Calib UI shall 確率密度の統計情報（期待値、分散、歪度、尖度）を計算・表示する。
4. The Model Calib UI shall 累積分布関数（CDF）表示オプションを提供する。
5. When 複数断面が選択される, the Model Calib UI shall 確率密度関数を重ねて比較表示する。
6. If 確率密度計算で数値的問題が発生, the Model Calib UI shall 警告とともに計算可能な範囲のみを表示する。

### Requirement 7: 3Dボラティリティサーフェス可視化

**Objective:** As a クオンツアナリスト, I want VolCube全体を3Dサーフェスとして可視化できる機能, so that ボラティリティ構造の全体像を把握できる。

#### Acceptance Criteria
1. The Model Calib UI shall 3Dボラティリティサーフェスプロット領域を提供する。
2. When キャリブレーションが完了する, the Model Calib UI shall Expiry × Strike × Implied Volの3Dサーフェスを描画する（特定Tenorを固定）。
3. The Model Calib UI shall Tenor選択により異なる3Dサーフェスを切り替え表示する機能を提供する。
4. The Model Calib UI shall 3Dサーフェスのマウスドラッグ回転・ズーム操作を可能にする。
5. The Model Calib UI shall カラーマップ選択（Viridis、Plasma、Inferno、等高線表示）を提供する。
6. The Model Calib UI shall 市場観測点を3Dサーフェス上にマーカーとして表示する。

### Requirement 8: バックエンドAPI実装

**Objective:** As a 開発者, I want VolCubeキャリブレーション用のREST APIエンドポイントを実装する, so that フロントエンドがバックエンドのVolCubeエンジンと連携できる。

#### Acceptance Criteria
1. The demo-web server shall `/api/volcube/indices` GETエンドポイントを提供し、利用可能なIndex一覧を返す。
2. The demo-web server shall `/api/volcube/instruments/{index}` GETエンドポイントを提供し、指定Indexのインストゥルメントデータを返す。
3. The demo-web server shall `/api/volcube/instruments/{index}` PUTエンドポイントを提供し、インストゥルメントデータを更新・保存する。
4. The demo-web server shall `/api/volcube/calibrate` POSTエンドポイントを提供し、キャリブレーションを実行して結果を返す。
5. The demo-web server shall `/api/volcube/smile` GETエンドポイントを提供し、指定断面のスマイルデータを返す。
6. The demo-web server shall `/api/volcube/density` GETエンドポイントを提供し、確率密度データを返す。
7. The demo-web server shall `/api/volcube/surface` GETエンドポイントを提供し、3Dサーフェスデータを返す。
8. If APIリクエストがエラーを含む, the demo-web server shall 構造化されたエラーレスポンス（error_code、message、details）を返す。

### Requirement 9: サンプルデータ準備

**Objective:** As a 開発者, I want 各資産クラス・Index用のサンプルインストゥルメントデータを準備する, so that ユーザーがすぐにキャリブレーション機能を試用できる。

#### Acceptance Criteria
1. The project shall `demo/data/input/volcube/` ディレクトリを作成する。
2. The project shall USD-SOFR-Swaption用サンプルデータファイル（複数Expiry × Tenor × Strikeのグリッド）を提供する。
3. The project shall EUR-ESTR-Swaption用サンプルデータファイルを提供する。
4. The project shall Equity-Options（例：SPX）用サンプルデータファイルを提供する。
5. The project shall 各サンプルデータファイルにrealistic な市場レートと適切なweightを含める。
6. The project shall サンプルデータのフォーマット仕様をREADME.mdで文書化する。
