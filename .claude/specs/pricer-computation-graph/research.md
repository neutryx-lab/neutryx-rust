# Research & Design Decisions

## Summary
- **Feature**: `pricer-computation-graph`
- **Discovery Scope**: Complex Integration
- **Key Findings**:
  - 既存の`pricer_pricing::graph`モジュールにGraphNode/GraphEdge/ComputationGraph型が存在し、拡張可能
  - `num_traits::Float`がpricer_core全体で40箇所以上使用されており、TracedFloatはこのトレイトを実装する必要がある
  - WebAppの「Instrument Graph」は5ファイル（handlers.rs, mod.rs, index.html, app.js, style.css）で参照されている
  - `#[track_caller]`はstable Rustで利用可能、proc macroの実装には`syn`/`quote`クレートが必要

## Research Log

### num_traits::Float実装の複雑さ
- **Context**: TracedFloatが既存のジェネリック関数（`T: Float`）で動作するために必要
- **Sources Consulted**: num_traits 0.2 ドキュメント、Rust標準ライブラリドキュメント
- **Findings**:
  - `num_traits::Float`トレイトは約75メソッドを要求（`is_nan()`, `abs()`, `exp()`, `ln()`, `sqrt()`, `sin()`, `cos()`等）
  - 既存の`num_dual::Dual64`は`Float`を実装していない（コードベース内の注記で確認）
  - 多くのメソッドはデフォルト実装が提供されるが、基本演算（`add`, `mul`, `div`, `sqrt`, `exp`, `ln`）は明示的実装が必要
- **Implications**: TracedFloatの実装は中程度の工数が必要。ただし、各演算は単純なラッパー実装で済む

### `#[track_caller]`の仕様確認
- **Context**: ソースコード位置を自動取得するために使用
- **Sources Consulted**: Rust Reference、RFC 2091、stable Rust 1.46+ドキュメント
- **Findings**:
  - `#[track_caller]`は関数に付与すると、その関数内で`std::panic::Location::caller()`が呼び出し元の位置を返す
  - トレイト実装のメソッドにも使用可能
  - `'static`ライフタイムの`&Location`を返すため、所有権の問題なし
  - インライン化との相互作用に注意が必要（`#[inline]`と併用すると位置情報が呼び出し元まで伝播する可能性）
- **Implications**: TracedFloatのAdd/Mul/Div等の演算子実装に`#[track_caller]`を付与することで、演算が発生した正確なソースコード位置を自動取得可能

### proc macroによる`#[traced_scope]`実装
- **Context**: 関数境界で自動的にスコープを生成するマクロ
- **Sources Consulted**: proc_macro、syn、quote クレートドキュメント、tracingクレートの`#[instrument]`実装
- **Findings**:
  - 属性マクロは関数本体を受け取り、変換されたトークンストリームを返す
  - `syn`で関数シグネチャをパース、`quote`で新しいコードを生成
  - feature flag無効時にno-opにするには、条件付きコンパイルまたは空のマクロ展開を使用
  - `tracing::instrument`は参考実装として有用（関数名の自動取得パターン）
- **Implications**: 新しいproc-macroクレート（`neutryx_macros`または`pricer_macros`）の追加が必要

### 既存graph型の拡張
- **Context**: ComputationGraphをTracedFloat出力と互換にする
- **Sources Consulted**: `crates/pricer_pricing/src/graph/types.rs`
- **Findings**:
  - `GraphNode`は既に`node_type`, `label`, `value`, `trade_ids`を持つ
  - `NodeType`は`Input`, `Add`, `Mul`, `Exp`, `Log`, `Sqrt`, `Div`, `Output`, `Custom(u8)`をサポート
  - `GraphMetadata`に`generated_at`タイムスタンプあり
  - 拡張が必要な項目:
    - `GraphNode`に`source_location`フィールド追加
    - `GraphNode`に`scope_id`フィールド追加
    - 新しい`Scope`型の追加
    - `ComputationGraph`に`scopes`配列の追加
- **Implications**: 後方互換性を維持しつつ拡張可能。既存のPortfolio Graph機能に影響なし

### WebApp「Instrument Graph」の改名
- **Context**: UIとAPIの改名
- **Sources Consulted**: `demo/gui/static/`, `demo/gui/src/web/`
- **Findings**:
  - 5ファイルで「Instrument Graph」参照:
    - `handlers.rs`: API実装
    - `mod.rs`: ルーティング
    - `index.html`: ナビゲーションメニュー
    - `app.js`: クライアントサイドロジック
    - `style.css`: スタイリング
  - 既存エンドポイント: `/api/graph`
  - 新エンドポイント: `/api/pricer/graph`
- **Implications**: フロントエンド・バックエンド両方の変更が必要だが、単純な文字列置換で対応可能

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| TracedFloat (Wrapper Type) | f64をラップし、演算時にグラフを構築 | 既存コード変更なし、`T: Float`互換 | 実行時オーバーヘッド（トレース時のみ） | **選択** |
| Expression Template | 遅延評価で式木を構築 | 最適化可能 | 大規模な設計変更が必要 | 却下 |
| LLVM IR解析 | Enzyme出力をパース | 四則演算レベルの詳細 | 不安定、メンテナンス困難 | 却下 |

## Design Decisions

### Decision: TracedFloat配置場所
- **Context**: TracedFloat型をどのクレートに配置するか
- **Alternatives Considered**:
  1. `pricer_core::types::traced` — L1で定義、全Pricerレイヤーから利用可能
  2. `pricer_pricing::traced` — L3で定義、Enzymeと同じクレート
- **Selected Approach**: `pricer_core::types::traced`
- **Rationale**:
  - TracedFloatは純粋な数値型でありEnzymeに依存しない
  - L1に配置することで、L2/L3/L4全てから利用可能
  - A-I-P-S依存ルールに適合
- **Trade-offs**: pricer_coreのfeatureフラグが増える
- **Follow-up**: feature flagは`execution-trace`として追加

### Decision: スコープ管理方式
- **Context**: 計算グラフの階層化をどのように実現するか
- **Alternatives Considered**:
  1. 手動API（`enter_scope`/`exit_scope`） — 明示的だが侵襲的
  2. `#[traced_scope]`属性マクロ — 関数境界で自動生成
- **Selected Approach**: `#[traced_scope]`属性マクロ
- **Rationale**:
  - 既存コードへの変更が最小限
  - 関数名が自動的にスコープ名になる
  - feature flag無効時はno-op
- **Trade-offs**: proc-macroクレートの追加が必要
- **Follow-up**: `#[traced_scope(name = "...")]`でカスタム名指定も可能にする

### Decision: グラフデータのスレッドセーフ性
- **Context**: TracedFloatの内部状態をどのように管理するか
- **Alternatives Considered**:
  1. `Rc<RefCell<ExecutionTrace>>` — シングルスレッド、低オーバーヘッド
  2. `Arc<Mutex<ExecutionTrace>>` — マルチスレッド対応
  3. Thread-local storage — スレッドごとに独立
- **Selected Approach**: `Rc<RefCell<ExecutionTrace>>` をデフォルト、オプションでArc版も提供
- **Rationale**:
  - 価格計算は通常シングルスレッドで実行される
  - Portfolio並列計算時は各スレッドが独立したTracedFloatを使用
  - 低オーバーヘッドを優先
- **Trade-offs**: マルチスレッドで共有する場合は明示的にArc版を使用する必要あり
- **Follow-up**: `TracedFloat`（Rc版）と`TracedFloatSync`（Arc版）の2種類を提供

## Risks & Mitigations
- **Risk 1**: `num_traits::Float`の全メソッド実装に漏れがある → 単体テストで全メソッドをカバー
- **Risk 2**: proc macroの複雑さ → `tracing::instrument`を参考に段階的実装
- **Risk 3**: 大規模計算でのメモリ使用量 → グラフサイズ上限とサンプリングモードの提供
- **Risk 4**: WebApp改名による既存リンク破損 → リダイレクトを一時的に設定

## References
- [num_traits::Float](https://docs.rs/num-traits/0.2/num_traits/float/trait.Float.html) — トレイト定義
- [#[track_caller] RFC 2091](https://rust-lang.github.io/rfcs/2091-inline-semantic.html) — ソース位置追跡
- [syn crate](https://docs.rs/syn/latest/syn/) — proc macro パーシング
- [quote crate](https://docs.rs/quote/latest/quote/) — proc macro コード生成
- [tracing::instrument](https://docs.rs/tracing/latest/tracing/attr.instrument.html) — 参考実装
