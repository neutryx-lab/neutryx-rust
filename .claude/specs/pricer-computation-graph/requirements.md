# Requirements Document

## Project Description (Input)
PricerにPVを計算させたときの、末端のMarketRateに至る計算グラフを自動取得し表示する。DemoのWebAppのInstrument GraphをPricer Graphと改名し、その計算グラフをすべて表示する(四則演算レベルまでのブレイクダウンができるとなお良い)。気をつけたいのは、なるべく今の構造を変えないまま取得する仕組みを整えること(Enzymeしかできないのであれば、Enzymeモードのみに対応でも良い)、WebAppはただ受け渡されたグラフを表示するだけ、Builderの繰り返し計算などは折り畳んで特殊な線で繋げると良い

## Introduction

本仕様は、Pricerが現在値（PV）を計算する際の計算グラフを取得・可視化する機能を定義する。

**アプローチ: Source-Mapped TracedFloat**

TracedFloat型を用いた実行グラフ一本化アプローチを採用する。Enzyme Activity Analysisによる静的グラフ案は以下の理由で却下：
- LLVM IRレベルの情報はドメイン概念（MarketRate等）へのマッピングが困難
- コンパイラのデバッグ出力形式はAPI契約ではなく、バージョン間で破損リスクが高い
- モノモーフィゼーション後の構造は元コードと大きく乖離

代わりに、TracedFloatに以下を組み込む：
1. **`#[track_caller]`によるソースコード位置の自動マッピング** - ノードをクリックでソースコードにジャンプ可能
2. **`#[traced_scope]`属性マクロによる自動スコープ構造化** - 関数境界で自動的にスコープを生成し、既存コードへの変更なしで階層化
3. **詳細度切り替え** - 四則演算レベル / スコープレベルの表示切り替え

既存の価格計算関数は`T: Float`ジェネリクスを使用しているため、TracedFloat型を導入することで**既存コードを変更せずに**実行グラフを取得できる。

## Requirements

### Requirement 1: TracedFloat型による計算グラフの取得

**Objective:** As a クオンツ開発者, I want 計算グラフを自動取得したい, so that 価格計算のデバッグと検証が容易になる

#### Acceptance Criteria

1. The Pricer shall `num_traits::Float`を実装するTracedFloat型を提供する
2. The Pricer shall TracedFloatの全演算（Add, Sub, Mul, Div, Sqrt, Exp, Ln等）で計算グラフノードを自動生成する
3. The Pricer shall 各ノードに演算タイプ、入力ノードID、計算結果の値を記録する
4. When 既存の`T: Float`ジェネリック関数にTracedFloatを渡したとき, the Pricer shall 関数内の全演算を自動的にトレースする
5. The Pricer shall TracedFloatのトレース結果をComputationGraph形式でエクスポートする機能を提供する
6. The Pricer shall 入力値（MarketRate等）にラベルを付与し、グラフの末端ノードとして識別可能とする

### Requirement 2: ソースコード位置の自動マッピング

**Objective:** As a 開発者, I want グラフノードからソースコードの該当行にジャンプしたい, so that デバッグ効率が向上する

#### Acceptance Criteria

1. The Pricer shall TracedFloatの全演算メソッドに`#[track_caller]`属性を付与する
2. The Pricer shall 各ノードに`std::panic::Location`（ファイル名、行番号、列番号）を記録する
3. The Pricer shall ソースコード位置情報をグラフノードのメタデータとしてエクスポートする
4. When ノードがAPIレスポンスに含まれるとき, the Pricer shall `source_location`フィールド（file, line, column）を含める
5. The WebApp shall ノードクリック時にソースコード位置情報を表示する（IDEジャンプ連携は将来拡張）

### Requirement 3: `#[traced_scope]`属性マクロによる自動スコープ構造化

**Objective:** As a リスク管理者, I want 計算グラフを論理的な単位で階層化したい, so that 複雑な計算でもグラフが見やすくなる

#### Acceptance Criteria

1. The Pricer shall `#[traced_scope]`属性マクロを提供し、関数境界で自動的にスコープを生成する
2. When `#[traced_scope]`が付与された関数が呼び出されたとき, the Pricer shall 関数名をスコープ名として自動的にスコープを開始・終了する
3. The Pricer shall スコープ内で生成されたノードを自動的にそのスコープに紐付ける
4. The Pricer shall ネストしたスコープをサポートする（関数呼び出し階層に対応した親子関係の記録）
5. The Pricer shall `#[traced_scope]`マクロにカスタム名を指定可能とする：`#[traced_scope(name = "Payoff Calculation")]`
6. When グラフがエクスポートされたとき, the Pricer shall 各ノードに所属スコープIDを含める
7. The Pricer shall スコープ情報を使用して、スコープ単位での折り畳み表示を可能とする
8. The Pricer shall `#[traced_scope]`がfeature flag無効時にno-opとなるよう実装する（ゼロオーバーヘッド）

### Requirement 4: 詳細度の切り替え

**Objective:** As a ユーザー, I want グラフの詳細度を切り替えたい, so that 必要な粒度で計算フローを分析できる

#### Acceptance Criteria

1. The Pricer shall グラフの詳細度レベルを提供する：Operation（四則演算レベル）、Scope（スコープレベル）
2. When Scopeレベルが選択されたとき, the Pricer shall スコープを1ノードとして集約し、スコープ間のエッジのみを表示する
3. When Operationレベルが選択されたとき, the Pricer shall 全演算ノードを表示する
4. The Pricer shall 詳細度の切り替えはクライアント側で行えるよう、全情報をAPIレスポンスに含める
5. The WebApp shall 詳細度切り替えトグル（Operation / Scope）を提供する

### Requirement 5: WebApp UI改名と表示機能

**Objective:** As a エンドユーザー, I want Pricer Graphページで計算グラフを閲覧したい, so that 価格計算の透明性を確保できる

#### Acceptance Criteria

1. The WebApp shall 「Instrument Graph」ページを「Pricer Graph」に改名する
2. The WebApp shall グラフをD3.js DAGとして描画する
3. When ユーザーがノードをクリックしたとき, the WebApp shall ノードの詳細情報を表示する：
   - 演算タイプ（Add, Mul, Sqrt等）
   - 入力値と出力値
   - ソースコード位置（ファイル名:行番号）
   - 所属スコープ
4. The WebApp shall スコープを視覚的に区別可能な形式（破線枠、背景色等）で表示する
5. When ユーザーがスコープをダブルクリックしたとき, the WebApp shall スコープの展開・折り畳みを切り替える
6. The WebApp shall グラフのズーム、パン、ノード検索機能を提供する
7. The WebApp shall 詳細度切り替えトグル（Operation / Scope）を提供する

### Requirement 6: API エンドポイント

**Objective:** As a システム統合者, I want REST APIで計算グラフを取得したい, so that 外部システムと連携できる

#### Acceptance Criteria

1. The WebApp shall `/api/pricer/graph`エンドポイントで計算グラフを返却する
2. When リクエストに計算パラメータが含まれるとき, the WebApp shall 指定パラメータで計算を実行しグラフを生成する
3. The WebApp shall グラフデータをD3.js互換のJSONフォーマットで返却する：
   - nodes配列: id, operation, value, source_location, scope_id
   - edges配列: source, target
   - scopes配列: id, name, parent_id
4. If 計算エラーが発生した場合, the WebApp shall エラー詳細を含むレスポンスを返却する

### Requirement 7: 既存構造の維持とTracedFloat統合

**Objective:** As a メンテナ, I want 既存のコード構造を最小限の変更で維持したい, so that 保守性とリグレッションリスクを抑える

#### Acceptance Criteria

1. The Pricer shall TracedFloat型を`pricer_core::types::traced`モジュールに配置する
2. The Pricer shall 既存の価格計算関数（`T: Float`ジェネリック）を一切変更せずにTracedFloatを利用可能とする
3. The Pricer shall TracedFloatをfeature flag（`execution-trace`）でゲート可能とする
4. The Pricer shall feature flag無効時はTracedFloat関連コードをコンパイルから除外する
5. The Pricer shall 既存の`pricer_pricing::graph`モジュール（GraphNode、GraphEdge、ComputationGraph）を拡張して再利用する
6. The Pricer shall TracedFloatの使用が通常のf64計算のパフォーマンスに影響を与えない（型が異なるため干渉しない）
