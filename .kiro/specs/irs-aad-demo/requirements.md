# Requirements Document

## Project Description (Input)
シンプルな金利スワップで、AADの効果(LazyValuation、リスク計算、XVA)として高速性がきちんと分かるような実装を作成したい。更には、FrictionalBankの一機能としてそれを体感できるようにしたい。

## Introduction

本仕様は、金利スワップ(IRS)を題材にAAD(Adjoint Algorithmic Differentiation)の高速性を実証するデモ機能の要件を定義する。Bump-and-Revalue方式との比較を通じて、LazyValuation、リスク計算(Greeks)、XVA計算におけるAADの優位性を数値的に示し、FrictionalBankデモシステムの一機能として体験可能にすることを目標とする。

## Requirements

### Requirement 1: IRS価格評価エンジン
**Objective:** クオンツアナリストとして、バニラIRSの現在価値(PV)と感応度を計算し、AADの性能優位性を検証したい

#### Acceptance Criteria
1. When ユーザーがIRSパラメータ(想定元本、固定レート、期間、通貨)を指定した場合, the IRS Pricer shall 対応するInterestRateSwapオブジェクトを生成する
2. When IRSオブジェクトとイールドカーブが提供された場合, the IRS Pricer shall 固定レグと変動レグの現在価値を計算する
3. When 価格評価が要求された場合, the IRS Pricer shall スワップの正味現在価値(NPV)を返却する
4. The IRS Pricer shall pricer_modelsのInterestRateSwap構造体およびScheduleBuilderと互換性を持つ
5. If 無効なパラメータ(負の想定元本、不正な日付範囲など)が指定された場合, the IRS Pricer shall 適切なエラーメッセージを返却する

### Requirement 2: AAD Greeks計算
**Objective:** リスク管理者として、IRSのGreeks(Delta、Gamma、DV01)をAADで高速に計算し、従来手法との性能差を把握したい

#### Acceptance Criteria
1. When AADモードが選択された場合, the Greeks Calculator shall EnzymeベースのADを使用してDelta(イールドカーブ感応度)を計算する
2. When Bump-and-Revalueモードが選択された場合, the Greeks Calculator shall 有限差分法でDeltaを計算する
3. When 両モードで計算が完了した場合, the Greeks Calculator shall 計算結果の差分が許容誤差(1e-6相対誤差)以内であることを検証する
4. The Greeks Calculator shall DV01(1bp金利変動に対するPV変化)を出力する
5. While カーブの複数のテナーポイントに対してDeltaを計算する場合, the Greeks Calculator shall AADで全テナーのDeltaを単一の逆伝播で計算する
6. The Greeks Calculator shall 計算時間をナノ秒精度で計測し、両モードの実行時間を記録する

### Requirement 3: LazyValuation機能
**Objective:** トレーダーとして、市場データ更新時に必要最小限の再計算のみを実行し、リアルタイム性能を確保したい

#### Acceptance Criteria
1. When 市場データが更新された場合, the Lazy Evaluator shall 変更されたカーブポイントに依存する計算のみを再実行する
2. When 同一の市場データで複数回価格評価が要求された場合, the Lazy Evaluator shall キャッシュされた結果を返却する
3. The Lazy Evaluator shall 依存関係グラフを構築し、変更伝播の最小化を実現する
4. When キャッシュが無効化された場合, the Lazy Evaluator shall 次回評価時に自動的に再計算を実行する
5. The Lazy Evaluator shall AAD計算においてテープの再利用を可能にする

### Requirement 4: XVA計算デモ
**Objective:** XVAアナリストとして、CVA/DVA計算においてAADによるエクスポージャー感応度の高速計算を体験したい

#### Acceptance Criteria
1. When ポートフォリオ(単一または複数のIRS)が提供された場合, the XVA Demo shall Expected Exposure(EE)プロファイルを計算する
2. When カウンターパーティの信用パラメータが指定された場合, the XVA Demo shall CVA(Credit Valuation Adjustment)を計算する
3. When 自社信用パラメータが指定された場合, the XVA Demo shall DVA(Debit Valuation Adjustment)を計算する
4. The XVA Demo shall AADを使用してXVA値のカーブ感応度を計算する
5. When Bump-and-Revalueモードが選択された場合, the XVA Demo shall 従来手法でのXVA感応度計算との性能比較を表示する
6. The XVA Demo shall pricer_riskのXvaCalculatorおよびExposureCalculatorと統合する

### Requirement 5: 性能ベンチマーク機能
**Objective:** 開発者として、AAD vs Bump-and-Revalueの性能差を定量的に比較し、AADの優位性を数値で示したい

#### Acceptance Criteria
1. The Benchmark Module shall 以下の計測項目を提供する: PV計算時間、単一Delta計算時間、全テナーDelta計算時間、XVA感応度計算時間
2. When ベンチマークが実行された場合, the Benchmark Module shall 各計測を複数回(デフォルト100回)実行し、統計値(平均、標準偏差、最小、最大)を算出する
3. The Benchmark Module shall AADモードとBump-and-Revalueモードの速度比(speedup ratio)を計算する
4. When テナーポイント数が増加した場合, the Benchmark Module shall スケーラビリティ(計算時間 vs テナー数)のグラフデータを出力する
5. The Benchmark Module shall 結果をJSON形式およびMarkdown形式で出力する

### Requirement 6: FrictionalBank統合
**Objective:** エンドユーザーとして、FrictionalBankデモシステム内でIRS AADデモを対話的に体験したい

#### Acceptance Criteria
1. When FrictionalBank TUIでIRS AAD Demoモードが選択された場合, the FrictionalBank Integration shall デモワークフローを開始する
2. The FrictionalBank Integration shall 以下の対話的機能を提供する: IRSパラメータ入力、計算モード選択(AAD/Bump-and-Revalue)、結果表示
3. When 計算が完了した場合, the FrictionalBank Integration shall PV、Greeks、計算時間を整形して表示する
4. Where Webダッシュボードが有効な場合, the FrictionalBank Integration shall リアルタイムでベンチマーク結果をWebSocket経由で配信する
5. The FrictionalBank Integration shall demo/frictional_bank/のworkflowパターンに従う
6. If ユーザーがパラメータを変更した場合, the FrictionalBank Integration shall 即座に再計算を実行し結果を更新する

### Requirement 7: 教育的可視化
**Objective:** 学習者として、AADの動作原理と性能優位性を視覚的に理解したい

#### Acceptance Criteria
1. The Visualisation Module shall AADとBump-and-Revalueの計算フローの概念図を表示する
2. When ベンチマークが完了した場合, the Visualisation Module shall 速度比較のバーチャートを表示する
3. The Visualisation Module shall テナー数増加に伴う計算時間の比較グラフを表示する
4. Where TUIモードの場合, the Visualisation Module shall ratatuiのチャートウィジェットを使用する
5. Where Webモードの場合, the Visualisation Module shall Chart.js互換のJSONデータを出力する
6. The Visualisation Module shall 計算結果の数値精度検証結果(AAD vs Bump-and-Revalue差分)を表示する