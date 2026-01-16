# Requirements Document

## Introduction

本仕様は、Neutryx デリバティブ価格計算ライブラリにおける **Sensitivity（Greeks）計算の精細化と高度化**、および **FrictionalWebApp の機能拡張** を定義する。

WebApp では、Pricer 機能内に既存の IRS Greeks ワークフローを統合しつつ、より高度なリスク分析機能とインタラクティブな可視化を実装する。また、Bump-and-Revalue 法と AAD 法の計算結果・パフォーマンス比較を可視化し、両手法の精度差と速度差をユーザーが直感的に把握できるようにする。

## Requirements

### Requirement 1: リスクファクター毎の Greeks 計算

**Objective:** As a クオンツアナリスト, I want 一次 Greeks（Delta, Vega, Rho, Theta）および二次 Greeks（Gamma, Vanna, Volga）をリスクファクター毎に計算したい, so that より精密なリスクヘッジ戦略を構築できる

#### Acceptance Criteria 1

1. When ユーザーが一次 Greeks を要求したとき, the Pricing Engine shall Delta（原資産価格）、Vega（ボラティリティ）、Rho（金利）、Theta（時間）をリスクファクター毎に計算する
2. When ユーザーが二次 Greeks を要求したとき, the Pricing Engine shall Gamma（Delta の原資産感応度）、Vanna（Delta のボラティリティ感応度）、Volga（Vega のボラティリティ感応度）をリスクファクター毎に計算する
3. The Pricing Engine shall リスクファクター識別子（原資産ID、カーブID、ボラティリティサーフェスID）毎に Greeks を集計可能とする
4. While Greeks 計算モードが AAD のとき, the Pricing Engine shall Bump-and-Revalue との精度比較結果を提供可能とする
5. The Pricing Engine shall すべての Greeks 計算において `GreeksResult<T>` 構造体を使用しAD互換性を維持する
6. If Greeks 計算でNaN または無限大が発生した場合, the Pricing Engine shall 適切なエラーメッセージとともにエラー型を返却する

### Requirement 2: バケット感応度と Key Rate Duration

**Objective:** As a リスクマネージャー, I want イールドカーブの各テナーポイントに対するバケット感応度（DV01, Key Rate Duration）を計算したい, so that 金利リスクのテナー構造を詳細に把握できる

#### Acceptance Criteria 2

1. When ユーザーがバケットDV01を要求したとき, the Pricing Engine shall 各テナーポイント（1M, 3M, 6M, 1Y, 2Y, 5Y, 10Y, 20Y, 30Y）に対する感応度を計算する
2. When ユーザーが Key Rate Duration を要求したとき, the Pricing Engine shall 指定されたキーレートポイントに対する修正デュレーションを返却する
3. The Pricing Engine shall バケット感応度の合計が総 DV01 と整合することを検証可能とする
4. While カーブシフトを適用中のとき, the Pricing Engine shall パラレルシフトとバタフライシフトの両方をサポートする

### Requirement 3: Sensitivity 計算のパフォーマンス最適化

**Objective:** As a 本番システム運用者, I want 大規模ポートフォリオの Greeks 計算を高速に実行したい, so that イントラデイのリスクモニタリング要件を満たせる

#### Acceptance Criteria 3

1. When 1000件以上のトレードを含むポートフォリオの Greeks を計算するとき, the Pricing Engine shall Rayon を用いた並列処理を適用する
2. The Pricing Engine shall AAD モードにおいて Bump-and-Revalue と比較して少なくとも5倍の速度向上を達成する
3. While 並列計算実行中のとき, the Pool Module shall `ThreadLocalWorkspacePool` を用いてメモリアロケーションを最小化する
4. The Pricing Engine shall ベンチマーク結果を Criterion フォーマットで出力可能とする
5. If メモリ使用量が設定閾値を超過した場合, the Pricing Engine shall チェックポイント機構を自動的に有効化する

### Requirement 4: Pricer 機能と IRS Greeks ワークフローの統合

**Objective:** As a リスクアナリスト, I want WebApp の Pricer 機能内で既存の IRS Greeks ワークフローを利用し、Bump 法と AAD 法の比較結果を確認したい, so that 計算手法の精度差・速度差を直感的に把握できる

#### Acceptance Criteria 4

1. The Web Dashboard shall Pricer 機能内に IRS Greeks ワークフロー（`irs_greeks/`）を統合し、IRS の Greeks 計算を実行可能とする
2. When ユーザーが Greeks 計算を要求したとき, the Web Dashboard shall Bump-and-Revalue 法と AAD 法の両方で計算を実行する
3. The Web Dashboard shall Bump 法と AAD 法の計算結果（Greeks 値）の差分を並列表示する
4. The Web Dashboard shall Bump 法と AAD 法のパフォーマンス比較（計算時間、速度倍率）をチャートで可視化する
5. When ユーザーが精度比較を要求したとき, the Web Dashboard shall 両手法の相対誤差・絶対誤差を表形式で表示する
6. The Web Dashboard shall `/api/greeks/compare` エンドポイントで Bump vs AAD の比較結果を返却する

### Requirement 5: Greeks 可視化機能の強化

**Objective:** As a リスクアナリスト, I want ブラウザ上で Greeks のヒートマップや時系列チャートを対話的に確認したい, so that リスクプロファイルの変化を直感的に把握できる

#### Acceptance Criteria 5

1. When ユーザーが Greeks ヒートマップ表示を要求したとき, the Web Dashboard shall テナー × ストライクの二次元ヒートマップを D3.js で描画する
2. When ユーザーが Greeks 時系列チャートを要求したとき, the Web Dashboard shall 選択した Greeks の時間推移を折れ線グラフで表示する
3. The Web Dashboard shall `/api/greeks/heatmap` および `/api/greeks/timeseries` エンドポイントを提供する
4. While WebSocket 接続中のとき, the Web Dashboard shall リアルタイムで Greeks 値の更新を配信する
5. The Web Dashboard shall 計算グラフ（`/api/graph`）との連携により、選択した Greeks ノードをハイライト表示する

### Requirement 6: インタラクティブなシナリオ分析 UI

**Objective:** As a ストレステスト担当者, I want Web UI 上でシナリオパラメータを動的に調整し、即座に結果を確認したい, so that What-if 分析を効率的に実施できる

#### Acceptance Criteria 6

1. When ユーザーがスライダーでリスクファクターシフト量を変更したとき, the Web Dashboard shall 即座に再計算を実行し結果を更新する
2. The Web Dashboard shall プリセットシナリオ（`PresetScenario`：Rate Shock, Vol Spike, FX Crisis 等）の選択機能を提供する
3. When ユーザーがカスタムシナリオを定義したとき, the Web Dashboard shall `RiskFactorShift` の組み合わせを保存・読み込み可能とする
4. The Web Dashboard shall シナリオ間の PnL 比較をテーブルおよびチャートで表示する
5. If シナリオ計算が5秒以上かかる場合, the Web Dashboard shall プログレスインジケータを表示する

### Requirement 7: API エンドポイントの拡充

**Objective:** As a システムインテグレーター, I want REST API を通じて高度な Greeks 計算機能にアクセスしたい, so that 既存のリスク管理システムと連携できる

#### Acceptance Criteria 7

1. The Web Server shall `/api/v1/greeks/first-order` エンドポイントで一次 Greeks を返却する
2. The Web Server shall `/api/v1/greeks/second-order` エンドポイントで二次 Greeks を返却する
3. The Web Server shall `/api/v1/greeks/bucket-dv01` エンドポイントでバケット感応度を返却する
4. When API リクエストが不正なパラメータを含む場合, the Web Server shall HTTP 400 Bad Request と詳細なエラーメッセージを返却する
5. The Web Server shall OpenAPI 3.0 形式の API ドキュメントを `/api/docs` で提供する
6. While 計算処理中のとき, the Web Server shall 非同期処理 ID を返却し `/api/v1/jobs/{id}` で進捗を確認可能とする

### Requirement 8: パフォーマンスメトリクスと監視機能

**Objective:** As a DevOps エンジニア, I want 計算パフォーマンスとシステム状態をリアルタイムで監視したい, so that 問題の早期検出と対応ができる

#### Acceptance Criteria 8

1. The Web Dashboard shall 計算時間、メモリ使用量、キャッシュヒット率のメトリクスを収集・表示する
2. The Web Server shall `/metrics` エンドポイントで Prometheus 形式のメトリクスを出力する
3. When 計算エラーが発生したとき, the Web Dashboard shall エラー発生件数とエラータイプの統計を表示する
4. If API レスポンスタイムが閾値を超えた場合, the Web Server shall 警告ログを出力する
