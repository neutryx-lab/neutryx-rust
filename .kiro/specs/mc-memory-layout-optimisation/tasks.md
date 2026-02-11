# Implementation Plan: MC Memory Layout Optimisation

## Phase 1: 基盤コンポーネント

- [x] 1. Config 構造体の実装
- [x] 1.1 レイアウトモード設定の実装
- [x] 1.2 ストリーミング設定の実装
- [x] 1.3 設定エラー型の定義

- [x] 2. アラインドメモリバッファの実装
- [x] 2.1 AlignedPathBuffer 構造体の実装
- [x] 2.2 アラインメント検証テストの作成

## Phase 2: Workspace 抽象化

- [x] 3. PathWorkspaceTrait と WorkspaceEnum の実装
- [x] 3.1 PathWorkspaceTrait トレイトの定義
- [x] 3.2 既存 PathWorkspace への PathWorkspaceTrait 実装
- [x] 3.3 WorkspaceEnum による静的ディスパッチの実装

- [x] 4. TimeStepFirstWorkspace の実装
- [x] 4.1 TimeStepFirstWorkspace 構造体の実装
- [x] 4.2 TimeStepFirstWorkspace のトレイト実装
- [x] 4.3 インデックス計算とスライスアクセスのテスト

## Phase 3: パス生成の拡張

- [x] 5. PathGenerator の実装
- [x] 5.1 ジェネリックパス生成関数の実装
- [x] 5.2 後方互換エイリアスの維持
- [x] 5.3 パス生成の数値一致検証テスト

## Phase 4: ストリーミングエンジン

- [x] 6. StreamingEngine の実装
- [x] 6.1 StreamingEngine 構造体の実装
- [x] 6.2 StreamingObserver トレイトの定義
- [x] 6.3 ストリーミング処理ループの実装
- [x] 6.4 既存 PathObserver のストリーミング対応
- [x] 6.5 ストリーミング処理のテスト

## Phase 5: 統合とAPI

- [x] 7. MonteCarloPricer の拡張と性能検証
- [x] 7.1 MonteCarloConfig ビルダーの拡張
- [x] 7.2 MonteCarloPricer のレイアウト対応
- [x] 7.3 性能ベンチマークの作成
- [x] 7.4 後方互換性の最終検証
