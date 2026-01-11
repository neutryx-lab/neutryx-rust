# Research & Design Decisions

## Summary
- **Feature**: `frictional-bank`
- **Discovery Scope**: Complex Integration (既存demo/inputs, outputs活用 + 新規frictional_bank, gui, data作成)
- **Key Findings**:
  - demo/inputs, demo/outputsは実装済みだがworkspace.membersに未登録
  - ratatuiはイミディエイトモードレンダリング、状態管理は開発者責任
  - pricer_risk::scenariosのScenarioEngineをストレステストに活用可能

## Research Log

### ratatui TUI フレームワーク
- **Context**: 要件5（ターミナルUI）実現のためのフレームワーク選定
- **Sources Consulted**:
  - [ratatui公式サイト](https://ratatui.rs/)
  - [ratatui GitHub](https://github.com/ratatui/ratatui)
  - [tui-realm](https://github.com/veeso/tui-realm) - React/Elm風状態管理
- **Findings**:
  - ratatuiはtui-rsのコミュニティフォーク、アクティブに開発中
  - イミディエイトモードレンダリング: 毎フレーム全ウィジェット再描画
  - 入力処理はバックエンド（crossterm）に委任
  - 60+ FPS維持可能、30-40%低メモリ使用
  - cargo-generateでテンプレートプロジェクト作成可能
- **Implications**:
  - App構造体で状態管理、draw()で毎フレームレンダリング
  - crossterm経由のイベントループ実装必要
  - tui-realm採用でElm風Message/Eventパターン可能

### 既存demo/inputs, demo/outputs統合
- **Context**: 既存実装の活用方法
- **Sources Consulted**: gap-analysis.md、Cargo.toml分析
- **Findings**:
  - demo_inputs: async_channel + tokio + MarketDataProvider trait
  - demo_outputs: axum + ReportSink trait
  - workspace.membersに未登録（独立したCargo.toml存在）
  - async_trait使用のためtokioランタイム必須
- **Implications**:
  - frictional_bankはtokioランタイムで構築
  - demo_inputs/outputsのpreludeモジュール活用
  - workspace.members追加で統合ビルド可能に

### pricer_risk::scenarios統合
- **Context**: 要件4.3（ストレステスト）実現
- **Sources Consulted**: pricer_risk/src/scenarios/engine.rs
- **Findings**:
  - ScenarioEngine<T>: シナリオ登録・実行・結果収集
  - ScenarioPnL<T>: ベース値/ストレス値/P&L計算
  - PresetScenario: 定義済みシナリオ（金利ショック等）
  - RiskFactorShift: リスクファクターシフト定義
- **Implications**:
  - オーケストレーターからScenarioEngine直接呼び出し可能
  - PresetScenarioをデモ設定から選択可能に
  - 並列実行対応（Rayon）

### service_gateway REST API
- **Context**: TUI/WebからのデータアクセスAPI
- **Sources Consulted**: service_gateway/src/rest/handlers.rs
- **Findings**:
  - `/health`, `/price`, `/portfolio`, `/calibrate`, `/exposure`エンドポイント存在
  - TODOコメントあり - pricer層との実統合未完了
  - PriceRequest/PriceResponse型定義済み
- **Implications**:
  - TUIはREST API経由でデータ取得
  - プレースホルダー実装でもデモ動作可能
  - 将来的にpricer_models/pricing統合で実際の計算可能

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Event-driven Orchestrator | tokio channels経由でコンポーネント間通信 | 非同期処理、疎結合 | 複雑性増加 | demo/inputsのasync_channel活用 |
| Layered Orchestrator | A-I-P-S順序でシーケンシャル実行 | シンプル、デバッグ容易 | 並列化困難 | EODバッチ向け |
| Elm-Architecture TUI | Message/Update/Viewパターン | 状態管理明確 | 学習曲線 | tui-realm採用可能 |
| Simple App State TUI | App構造体 + draw関数 | シンプル、ratatui標準 | 大規模化で複雑 | 推奨 |

## Design Decisions

### Decision: ハイブリッドオーケストレーター
- **Context**: EODバッチ（シーケンシャル）とイントラデイ（イベント駆動）の両対応
- **Alternatives Considered**:
  1. 全イベント駆動 — 複雑性高い
  2. 全シーケンシャル — リアルタイム対応不可
- **Selected Approach**: DemoWorkflow traitで抽象化、EodBatch/Intraday/StressTest実装
- **Rationale**: デモ目的に適切な複雑性、既存pricer_risk::demo活用
- **Trade-offs**: trait設計のオーバーヘッド vs 拡張性
- **Follow-up**: 各ワークフローの進捗報告callback設計

### Decision: App構造体ベースTUI
- **Context**: ratatuiの状態管理アプローチ選択
- **Alternatives Considered**:
  1. tui-realm (Elm風) — 機能豊富だが依存追加
  2. App構造体 (標準) — シンプル、公式例に準拠
- **Selected Approach**: App構造体 + enum-based画面切替
- **Rationale**: デモ目的に十分、追加依存最小化
- **Trade-offs**: 大規模化時のリファクタリング vs 初期シンプルさ
- **Follow-up**: 画面遷移ロジックのテスト可能性確保

### Decision: サンプルデータはCSV/JSON形式
- **Context**: demo/dataのファイル形式選択
- **Alternatives Considered**:
  1. CSV + JSON — 人間可読、編集容易
  2. Parquet — 高効率だが編集困難
  3. XML (FpML) — 標準準拠だが冗長
- **Selected Approach**: CSV（取引/マスタ）+ JSON（設定）+ XML（FpMLサンプル）
- **Rationale**: デモ目的に人間可読性優先
- **Trade-offs**: パフォーマンス vs 可読性
- **Follow-up**: adapter_loader CSV parserとの互換性確認

## Risks & Mitigations
- **ratatui学習曲線** — 公式examples参照、シンプルなApp構造体採用で緩和
- **全層統合の複雑性** — 段階的実装（data→orchestrator→TUI→notebooks）
- **workspace.members未登録** — Phase 1で追加、依存関係検証
- **pricer_pricing Enzyme依存** — frictional_bankはpricer_pricing除外で実装

## References
- [ratatui](https://ratatui.rs/) — TUIフレームワーク公式
- [tui-realm](https://github.com/veeso/tui-realm) — Elm風状態管理（参考）
- [crossterm](https://docs.rs/crossterm/) — ターミナルバックエンド
- [async-channel](https://docs.rs/async-channel/) — 非同期チャネル
