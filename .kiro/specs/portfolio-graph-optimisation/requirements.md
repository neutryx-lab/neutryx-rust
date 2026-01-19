# Requirements Document

## Introduction

本仕様は、FrictionalBankAppに既に実装されている計算グラフ（Computational Graph）機能を、Portfolio（ポートフォリオ）レベルで最適化・拡張するための要件を定義する。現状の単一トレード単位のグラフ可視化を、複数約定を含むPortfolio全体の計算グラフとして統合・可視化し、ユーザーがPortfolio内の約定を選択してサブグラフを動的に表示できる機能を実現する。

**現状**: `pricer_pricing::graph`モジュールに`ComputationGraph`、`GraphExtractable`トレイト、`SimpleGraphExtractor`が実装済み。D3.js互換JSON形式でWeb dashboardに表示可能。単一トレード（`trade_id`指定）またはNone（全トレード結合）でのグラフ抽出に対応。

**目標**: Portfolio単位での計算グラフ最適化、サンプル約定を多数含むPortfolioの作成、約定選択によるインタラクティブなグラフ可視化を実現する。

## Requirements

### Requirement 1: Portfolioベースの計算グラフ構造

**Objective:** As a リスクアナリスト, I want Portfolio単位で計算グラフを構築・可視化できる機能, so that ポートフォリオ全体のAAD依存関係とリスク伝播を一目で理解できる

#### Acceptance Criteria
1. The PortfolioGraphExtractor shall Portfolio単位での計算グラフ抽出を提供する
2. When Portfolioに複数トレードが含まれている場合, the PortfolioGraphExtractor shall 各トレードのサブグラフを統合した単一のComputationGraphを生成する
3. The PortfolioComputationGraph shall トレード間で共有されるマーケットデータノード（YieldCurve、VolSurface等）を単一ノードとして最適化（重複排除）する
4. When PortfolioGraphを生成する場合, the システム shall 各ノードに所属トレードID（trade_ids: Vec<String>）を付与して所属関係を追跡する
5. The PortfolioGraphMetadata shall トレード数、共有ノード数、最適化率（重複排除前後のノード数比）を含む統計情報を提供する

### Requirement 2: サンプルPortfolioの提供

**Objective:** As a 開発者/デモユーザー, I want 多数のサンプル約定を含むPortfolioを利用できる機能, so that Portfolioレベルのグラフ可視化機能を即座にテスト・デモできる

#### Acceptance Criteria
1. The SamplePortfolioBuilder shall 複数アセットクラス（Equity、Rates、FX）を含むサンプルPortfolioを生成する
2. The SamplePortfolioBuilder shall 設定可能なトレード数（デフォルト: 10〜100件）でPortfolioを生成する
3. When サンプルPortfolioを生成する場合, the システム shall トレード間で共有マーケットデータ（同一通貨ペア、同一満期日等）を持つトレードを含める
4. The サンプルPortfolio shall 少なくとも3種類のInstrument（VanillaOption、IRS、FxOption）を含む
5. If サンプルPortfolio生成に失敗した場合, then the システム shall 具体的なエラー理由を含むResult::Errを返却する

### Requirement 3: 約定選択によるサブグラフ表示

**Objective:** As a トレーダー, I want Portfolio内の特定約定を選択してそのサブグラフのみを表示できる機能, so that 関心のあるトレードの計算依存関係を集中的に分析できる

#### Acceptance Criteria
1. The PortfolioGraphExtractor shall 指定されたトレードIDリストに基づくサブグラフ抽出機能（extract_subgraph）を提供する
2. When 複数トレードを選択した場合, the システム shall 選択トレード間で共有されるノードを保持しつつサブグラフを生成する
3. When サブグラフを抽出する場合, the システム shall 選択されていないトレード専用のノードを除外する
4. The サブグラフ抽出 shall 元のグラフ構造の依存関係（エッジ）を維持する
5. If 存在しないトレードIDが指定された場合, then the GraphError::TradeNotFound shall 該当トレードIDを含むエラーメッセージを返却する

### Requirement 4: Web Dashboard統合

**Objective:** As a エンドユーザー, I want Web dashboard上でPortfolioグラフを操作できるUI, so that ブラウザからインタラクティブにグラフ分析を行える

#### Acceptance Criteria
1. The REST API shall `/api/v1/portfolio/graph`エンドポイントでPortfolio計算グラフを取得可能にする
2. The REST API shall クエリパラメータ`trade_ids`（カンマ区切り）によるサブグラフ取得をサポートする
3. When グラフデータを返却する場合, the API shall D3.js互換のJSON形式（nodes、links、metadata）を維持する
4. The API レスポンス shall PortfolioGraphMetadataを含む拡張メタデータを提供する
5. If グラフ抽出が500msを超過した場合, then the API shall HTTP 504 Gateway Timeoutを返却する

### Requirement 5: トレード一覧とグラフ連携

**Objective:** As a リスクアナリスト, I want トレード一覧からグラフ表示を直接操作できる連携機能, so that トレード選択とグラフ可視化をシームレスに行える

#### Acceptance Criteria
1. The REST API shall `/api/v1/portfolio/trades`エンドポイントでPortfolio内トレード一覧を取得可能にする
2. The トレード一覧レスポンス shall 各トレードのID、Instrument種別、通貨、想定元本、満期日を含む
3. When トレード一覧を取得する場合, the システム shall トレード総数と各Instrument種別の内訳を含む統計情報を提供する
4. The WebSocket shall トレード選択イベント（select_trades）の送受信をサポートする
5. When WebSocketでトレード選択イベントを受信した場合, the システム shall 対応するサブグラフを自動更新してブロードキャストする

### Requirement 6: パフォーマンス最適化

**Objective:** As a システム管理者, I want 大規模Portfolioでも高速にグラフを生成・表示できる性能, so that 本番環境での実用性を確保できる

#### Acceptance Criteria
1. The PortfolioGraphExtractor shall 100トレードのPortfolioグラフを1秒以内に生成する
2. The システム shall ノード重複排除により20%以上のノード数削減を達成する（共有マーケットデータを持つ典型的Portfolioの場合）
3. While グラフを増分更新する場合, the システム shall 変更ノードのみを含むdiffを計算して差分更新を提供する
4. The GraphBuilder shall 事前割り当てキャパシティを設定可能にしてメモリ再割り当てを最小化する
5. If 10,000ノードを超えるグラフの場合, the システム shall Level-of-Detail（LOD）モードでの簡略化グラフオプションを提供する
