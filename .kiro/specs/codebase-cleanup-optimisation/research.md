# Research & Design Decisions: codebase-cleanup-optimisation

---
**Purpose**: コードベースクリーンアップ・最適化の設計判断を支える調査結果と根拠を記録する。
**Usage**: 設計フェーズでの意思決定の根拠、詳細調査結果、リスク評価を文書化。
---

## Summary

- **Feature**: `codebase-cleanup-optimisation`
- **Discovery Scope**: Extension（既存コードベースの改善）
- **Key Findings**:
  - コードベース規模: 264ファイル、107,607行（crates/配下）
  - 不要コメント: TODO/FIXME 16件、コメントアウトコード 6件（対応容易）
  - 大規模ファイル: 500行超が30ファイル（sabr.rs: 2,919行が最大）
  - unwrap/expect: ライブラリコードに約260箇所（テスト除く）
  - dead_code許容: 7ファイルで15箇所の`#[allow(dead_code)]`使用

## Research Log

### コメント分類と削除基準

- **Context**: 要件1「不要コメント削除」の実装方針決定
- **Sources Consulted**: ai_rules.md、既存コードベース分析
- **Findings**:
  - TODO/FIXME: 16件（service_cli、service_gateway、adapter_fpml、pricer_pricing）
  - コメントアウトコード: 6件（verify_enzyme.rs、pricer_checkpoint.rs）
  - 数学的導出コメント（`// smooth_max(a, b, ε) = ...`等）: 保持すべき「Why」コメント
  - テスト内の期待値説明コメント（`// f(x) = x^2, f'(x) = 2x`等）: 保持すべき
- **Implications**: 削除対象は明確に限定可能、自動化より手動レビューが安全

### ファイル分割戦略

- **Context**: 要件2.3「500行以下」と要件6.5「最小ファイル数」のバランス
- **Sources Consulted**: gap-analysis.md、structure.md
- **Findings**:
  - 500行超ファイル30件のうち、分割候補は10件程度
  - sabr.rs/heston.rs: パラメータ定義、公式実装、MC統合、テストが混在
  - ベンチマーク/デモファイル: 分割不要（単一目的）
  - smoothing.rs: 関連関数群として分割不要
- **Implications**: 分割は構造的意味がある場合のみ実施、最小ファイル数原則を優先

### unwrap/expect排除戦略

- **Context**: 要件5.6「unwrap/expect/panicをライブラリコードで使用しない」
- **Sources Consulted**: Rustエラーハンドリングベストプラクティス、既存コード分析
- **Findings**:
  - 総検出数: 760箇所
  - テスト/ベンチマーク: ~500箇所（許容、変更不要）
  - ライブラリコード: ~260箇所（要対応）
    - pricer_core: ~150箇所（基盤、最優先）
    - pricer_models: ~80箇所
    - pricer_optimiser: ~30箇所
  - 主要パターン: `Option::unwrap()`→`Option::ok_or()`、`Result::expect()`→`?`演算子
- **Implications**: クレート単位で段階的に対応、依存の少ないクレートから開始

### dead code検出と排除

- **Context**: 要件6.3「未使用の関数、型、トレイト、モジュールを含まない」
- **Sources Consulted**: `cargo clippy --all-targets`、`#[allow(dead_code)]`検索
- **Findings**:
  - `#[allow(dead_code)]`使用: 7ファイル、15箇所
  - 主な箇所: service_gateway、service_cli、pricer_pricing/rng
  - 多くは開発中の機能またはAPIの将来拡張用
- **Implications**: `#[allow(dead_code)]`を削除し、本当に不要なコードは削除

### ミニマリズム原則の適用

- **Context**: 要件6「コードミニマリズムの追求」の具体化
- **Sources Consulted**: YAGNI原則、既存コード分析
- **Findings**:
  - 1-2行ラッパー関数: 検出・インライン化候補を特定する必要あり
  - 過度な抽象化: Builderパターンは適切に使用されている
  - 未使用pub関数: `cargo clippy`で検出可能
- **Implications**: clippy警告を活用し、`--warn=unused`オプションで検出

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 段階的リファクタリング | フェーズ分割で漸進的に改善 | リスク最小化、各フェーズでCI検証 | 完了まで時間を要する | 推奨アプローチ |
| 包括的リファクタリング | 全要件を一括実装 | 一貫した変更 | 大規模PR、問題切り分け困難 | 非推奨 |
| 選択的対応 | 高優先度項目のみ | 最小工数 | 一部要件未達成 | 部分的に採用可 |

## Design Decisions

### Decision: フェーズ分割戦略

- **Context**: 6要件、29受入基準を安全に実装する方法
- **Alternatives Considered**:
  1. 一括実装 — 全変更を1PRで
  2. 要件単位 — 各要件を独立PRで
  3. リスクベース — 低リスクから高リスクへ段階的に
- **Selected Approach**: リスクベースの段階的実装
  - Phase 1: 不要コメント削除（低リスク）
  - Phase 2: dead code排除（低リスク）
  - Phase 3: unwrap/expect排除（中リスク）
  - Phase 4: ファイル構造最適化（高リスク）
- **Rationale**: 各フェーズでテスト検証可能、問題発生時の切り分けが容易
- **Trade-offs**: 完了まで時間を要するが、品質リスクを最小化
- **Follow-up**: 各フェーズ完了時にcargo test/clippy/fmt通過を必須とする

### Decision: ファイル分割基準

- **Context**: 要件2.3と要件6.5のバランス（500行制限 vs 最小ファイル数）
- **Alternatives Considered**:
  1. 厳密500行制限 — 全ファイルを500行以下に
  2. 論理的分割のみ — 構造的意味がある場合のみ分割
  3. ハイブリッド — 1000行超のみ分割検討
- **Selected Approach**: 論理的分割のみ（Option 2）
  - 分割条件: 独立した責務が明確に存在する場合
  - 非分割: ベンチマーク、デモ、関連関数群
- **Rationale**: ミニマリズム原則（要件6）を優先、不要なファイル分割を避ける
- **Trade-offs**: 一部ファイルが500行を超えるが、論理的一貫性を維持
- **Follow-up**: sabr.rs/heston.rsは構造的分割の価値を個別評価

### Decision: unwrap排除の優先順位

- **Context**: 260箇所のunwrap/expectを効率的に排除
- **Alternatives Considered**:
  1. 全クレート同時 — 一括変更
  2. 依存順 — 依存の少ないクレートから
  3. 影響度順 — 呼び出し頻度の高いものから
- **Selected Approach**: 依存順（Option 2）
  - 順序: pricer_core → pricer_models → pricer_optimiser
  - テスト/ベンチマークは変更対象外
- **Rationale**: 基盤クレートの安定性を先に確保、波及効果を制御
- **Trade-offs**: 依存クレートの変更が必要になる場合あり
- **Follow-up**: 各クレートで適切なエラー型を定義

## Risks & Mitigations

- **ファイル分割によるコンパイルエラー** — 段階的分割、各段階でCI実行
- **unwrap排除による型変更波及** — クレート単位で実施、依存先から順に
- **テスト破損** — 各変更後にテスト実行、変更前後でテストカバレッジ維持
- **過度な最適化による可読性低下** — コードレビューで可読性を確認

## References

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) — エラーハンドリング、ドキュメント規約
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/) — dead code検出、unwrap警告
- `.kiro/steering/ai_rules.md` — British English、コメント規約
- `.kiro/steering/structure.md` — A-I-P-S依存ルール、命名規約
