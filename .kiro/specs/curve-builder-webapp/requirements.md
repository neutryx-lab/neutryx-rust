# Requirements Document

## Introduction

本ドキュメントは、Demo WebAppのCurve Build画面を精緻化するための要件を定義する。主な目的は、各Index毎のInstrument入力データを管理し、任意のBuilderモデルを用いたカーブ構築機能を提供すること、および構築されたカーブをParameterカーブとして可視化することである。金利スワップの評価機能はこの画面から削除する。

## Project Description (Input)
Demo内のWebAppのCurve Build画面を精緻化したい。まず、各Index毎にCurveBuildに必要なInstrumentListをInputDataとしてdemo\data\inputに用意する。そのレートを入力可能として、任意のBuilderモデルを用いてカーブを構築する。構築されたカーブは任意のParmeterカーブとして確認可能。金利スワップの評価機能はこの画面からは削除。

---

## Requirements

### Requirement 1: Index別Instrument入力データ管理

**Objective:** As a 定量アナリスト, I want 各Index（SOFR、ESTR、TONA等）毎にカーブ構築に必要なInstrumentリストを入力データとして管理したい, so that カーブ構築に必要な市場データを体系的に準備・維持できる

#### Acceptance Criteria

1. The Curve Builder WebApp shall `demo/data/input/curves/` ディレクトリにIndex別のInstrumentリストJSONファイルを格納する構造を提供する
2. The Curve Builder WebApp shall 各Indexに対してDeposit、OIS、Swap等の複数のInstrumentタイプをサポートする
3. When Index設定ファイルが読み込まれたとき, the Curve Builder WebApp shall Tenor、Rate Value、Index名、Instrumentタイプを含む完全なInstrument定義を取得する
4. The Curve Builder WebApp shall USD-SOFR、EUR-ESTR、JPY-TONAの3通貨をデフォルトでサポートする
5. If Instrumentファイルが存在しないか不正な形式の場合, the Curve Builder WebApp shall 適切なエラーメッセージを表示し、デフォルトのInstrumentリストにフォールバックする

---

### Requirement 2: レート入力インターフェース

**Objective:** As a トレーダー/クォント, I want WebApp上でInstrumentレートを直接編集・入力したい, so that 市場データの更新やWhat-if分析をリアルタイムで行える

#### Acceptance Criteria

1. When ユーザーがCurve Build画面にアクセスしたとき, the Curve Builder WebApp shall 選択されたIndexの全Instrumentレートを編集可能なテーブル形式で表示する
2. The Curve Builder WebApp shall 各レート入力フィールドに対して数値バリデーション（範囲チェック、小数点精度）を実行する
3. When ユーザーがレート値を変更したとき, the Curve Builder WebApp shall 変更されたセルを視覚的にハイライトし、変更前の値を保持する
4. The Curve Builder WebApp shall 全レートをJSON形式でエクスポートする機能を提供する
5. When ユーザーがJSONファイルをインポートしたとき, the Curve Builder WebApp shall ファイル内のレート値で入力フィールドを更新する
6. The Curve Builder WebApp shall レート変更をリセットしてファイルから読み込んだ元の値に戻す機能を提供する

---

### Requirement 3: カーブBuilderモデル選択

**Objective:** As a 定量アナリスト, I want カーブ構築に使用するBuilderモデル（補間手法、ブートストラップ設定）を選択したい, so that 異なるモデル仮定の影響を比較・分析できる

#### Acceptance Criteria

1. The Curve Builder WebApp shall Linear、LogLinear、CubicSpline、Monotonic補間手法を選択肢として提供する
2. The Curve Builder WebApp shall ブートストラップ手法（Sequential、Global）を選択可能にする
3. When ユーザーがBuilderモデルを選択したとき, the Curve Builder WebApp shall 選択されたモデルの設定パラメータ（許容誤差、最大反復回数等）を表示する
4. The Curve Builder WebApp shall 頻繁に使用するBuilder設定をプリセットとして保存・読み込みする機能を提供する
5. If 選択されたBuilderモデルが入力Instrumentと互換性がない場合, the Curve Builder WebApp shall 警告メッセージを表示し、互換性のある代替案を提案する

---

### Requirement 4: カーブ構築実行

**Objective:** As a ユーザー, I want 入力レートと選択したBuilderモデルを使用してカーブを構築したい, so that 市場整合的なイールドカーブを生成できる

#### Acceptance Criteria

1. When ユーザーが「Build Curve」ボタンをクリックしたとき, the Curve Builder WebApp shall 入力レートと選択されたBuilderモデルを使用してカーブ構築を実行する
2. While カーブ構築が進行中のとき, the Curve Builder WebApp shall プログレスインジケータと現在のステップ情報を表示する
3. When カーブ構築が完了したとき, the Curve Builder WebApp shall 構築結果（成功/失敗、処理時間、使用Instrument数）をサマリとして表示する
4. If カーブ構築が失敗した場合, the Curve Builder WebApp shall 失敗原因（収束エラー、不正レート等）を詳細なエラーメッセージとして表示する
5. The Curve Builder WebApp shall 直近の構築結果をキャッシュし、設定変更時に再構築を促すUIを提供する

---

### Requirement 5: Parameterカーブ表示

**Objective:** As a 定量アナリスト, I want 構築されたカーブを任意のParameterカーブ（Discount Factor、Zero Rate、Forward Rate）として表示したい, so that カーブの特性を多角的に分析できる

#### Acceptance Criteria

1. When カーブ構築が成功したとき, the Curve Builder WebApp shall Discount Factor、Zero Rate、Forward Rateの表示モード切替タブを提供する
2. The Curve Builder WebApp shall 選択されたParameterタイプに応じたカーブをチャート形式で表示する
3. The Curve Builder WebApp shall カーブデータをテーブル形式（Tenor、Value列）でも表示する
4. When ユーザーがチャート上の特定ポイントにホバーしたとき, the Curve Builder WebApp shall そのポイントのTenorと値をツールチップで表示する
5. The Curve Builder WebApp shall 表示するTenor範囲（開始日、終了日、グリッド間隔）をカスタマイズする機能を提供する
6. The Curve Builder WebApp shall カーブデータをCSVまたはJSON形式でエクスポートする機能を提供する

---

### Requirement 6: IRS評価機能の削除

**Objective:** As a プロダクトオーナー, I want Curve Build画面からIRS評価機能を削除したい, so that 画面の目的をカーブ構築に特化させ、UIをシンプルに保てる

#### Acceptance Criteria

1. The Curve Builder WebApp shall Curve Build画面からIRS Pricing関連のUI要素（入力フォーム、計算ボタン、結果表示）を削除する
2. The Curve Builder WebApp shall IRS Pricing APIエンドポイントへの呼び出しをCurve Build画面から除去する
3. When ユーザーが旧IRS Pricing機能へのブックマークやリンクにアクセスした場合, the Curve Builder WebApp shall Curve Build画面にリダイレクトし、機能移動の通知を表示する
4. The Curve Builder WebApp shall IRS評価機能は別画面（Trade Pricing画面等）で引き続き利用可能であることをドキュメント化する

---

### Requirement 7: API設計

**Objective:** As a 開発者, I want Curve Builder機能に対応するREST APIエンドポイントを実装したい, so that フロントエンドとバックエンドの疎結合を維持できる

#### Acceptance Criteria

1. The Curve Builder WebApp shall `GET /api/curves/instruments/{index}` エンドポイントでIndex別Instrumentリストを取得可能にする
2. The Curve Builder WebApp shall `POST /api/curves/build` エンドポイントでカーブ構築リクエストを受け付ける
3. The Curve Builder WebApp shall `GET /api/curves/{curveId}/parameters` エンドポイントで構築済みカーブのParameterデータを取得可能にする
4. The Curve Builder WebApp shall `GET /api/curves/builders` エンドポイントで利用可能なBuilderモデル一覧を取得可能にする
5. When APIリクエストが不正な場合, the Curve Builder WebApp shall RFC 7807準拠のProblem Details形式でエラーレスポンスを返す
6. The Curve Builder WebApp shall 全APIエンドポイントをOpenAPI仕様でドキュメント化する

---

## Non-Functional Requirements

### Performance
- カーブ構築処理は10秒以内に完了すること（標準的なInstrument数：20〜30）
- Parameterカーブ表示のレンダリングは500ms以内に完了すること

### Usability
- レート入力時のバリデーションエラーは即座にインラインで表示すること
- Builder設定変更時は、再構築が必要である旨を明確に通知すること

### Compatibility
- 既存の `webapp_market_data.json` フォーマットとの後方互換性を維持すること
- 既存のMarket Data Viewer画面との統合を考慮すること
