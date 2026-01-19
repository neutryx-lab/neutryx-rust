# Requirements Document

## Introduction

本ドキュメントは、Neutryx derivatives pricing libraryのコードベース全体に対するクリーンアップと最適化の要件を定義する。対象は不要コメントの削除、コード構造の明快化、関数間構造の最適化、重複コードの排除、およびコードミニマリズムの追求である。A-I-P-S（Adapter→Infra→Pricer→Service）アーキテクチャに準拠しつつ、最小限のコード行数・関数数・ファイル数で本番品質を維持することを目的とする。

## Project Description (Input)

全体的に、不要なコメントを徹底的に削除、コード構造、関数間構造の明快化最適化。重複などの不要なコードの削除

## Requirements

### Requirement 1: 不要コメントの削除

**Objective:** As a 開発者, I want コードベース全体から不要なコメントを削除したい, so that コードの可読性と保守性が向上し、実装の意図が明確になる

#### Acceptance Criteria

1. The Codebase shall コードの動作を単に説明するだけの冗長なコメント（例：`// increment i`、`// return value`）を含まない
2. The Codebase shall TODOコメント、FIXMEコメント、一時的なデバッグコメントを含まない（完了済みまたは不要なものに限る）
3. The Codebase shall コメントアウトされたコード（dead code）を含まない
4. The Codebase shall 「Why」（数学的導出、安全性根拠、ハードウェア制約）を説明する有意義なコメントのみを保持する
5. When コメントが数学的公式や参照論文を引用する場合, the Codebase shall 該当する論文名と方程式番号をdocstringに含める

### Requirement 2: コード構造の明快化

**Objective:** As a 開発者, I want 各モジュールとファイルの構造を明快にしたい, so that コードの理解と保守が容易になる

#### Acceptance Criteria

1. The Codebase shall 各モジュールが単一責任原則に従い、明確な目的を持つ
2. The Codebase shall A-I-P-S依存ルール（S→P/I/A、P→P/Iのみ、I→Iのみ、A→I/Pのみ）に準拠する
3. The Codebase shall 各ファイルが適切な長さ（目安：500行以下）を維持し、過度に長いファイルは論理的に分割される
4. When ファイルが複数の独立した機能を含む場合, the Codebase shall それぞれを適切なサブモジュールに分離する
5. The Codebase shall `mod.rs`ファイルがpub use re-exportと簡潔なモジュール宣言のみを含み、実装コードを含まない

### Requirement 3: 関数間構造の最適化

**Objective:** As a 開発者, I want 関数の責務分離と呼び出し構造を最適化したい, so that コードの再利用性とテスト容易性が向上する

#### Acceptance Criteria

1. The Codebase shall 各関数が単一の明確な責務を持ち、30行以下を目安とする（複雑なアルゴリズムを除く）
2. The Codebase shall 過度に深いネスト（4段階以上）を持つ関数を含まない
3. The Codebase shall early returnパターンを適用し、ガード節でエラーケースを先に処理する
4. When 関数が5つ以上のパラメータを持つ場合, the Codebase shall Builder patternまたは構造体パラメータを使用する
5. The Codebase shall 類似の処理を行う関数群に対して共通のtraitまたはヘルパー関数を定義する
6. The Codebase shall private関数を適切にスコープし、不必要にpubとしない

### Requirement 4: 重複コードの排除

**Objective:** As a 開発者, I want 重複コードを特定し排除したい, so that バグ修正や機能追加が一箇所で完結し、保守コストが削減される

#### Acceptance Criteria

1. The Codebase shall 同一または類似のロジックが3箇所以上で繰り返されていない
2. When 複数の箇所で同一パターンのコードが存在する場合, the Codebase shall 共通関数、マクロ、またはtraitとして抽出する
3. The Codebase shall コピー＆ペーストされたテストコードを持たず、テストヘルパーやfixtureを使用する
4. The Codebase shall 重複した型定義やconstant定義を持たず、適切なモジュールで一元管理する
5. Where 複数のクレートで同一のユーティリティ関数が必要な場合, the Codebase shall pricer_coreまたは適切な共通クレートで定義する

### Requirement 5: 品質基準の維持

**Objective:** As a 開発者, I want クリーンアップ後もコードの品質基準を維持したい, so that 既存の機能が損なわれず、本番環境での信頼性が保証される

#### Acceptance Criteria

1. The Codebase shall すべての既存テストがクリーンアップ後も通過する
2. The Codebase shall `cargo fmt --all -- --check`によるフォーマットチェックに合格する
3. The Codebase shall `cargo clippy --all-targets -- -D warnings`による静的解析に合格する
4. The Codebase shall British English表記規則（optimiser、serialisation、visualisation、modellingなど）に準拠する
5. When 関数やモジュールを削除または移動する場合, the Codebase shall 関連するテストも適切に更新または移動する
6. The Codebase shall `unwrap()`、`expect()`、`panic!()`をライブラリコードで使用しない（エラーはResultで伝播）

### Requirement 6: コードミニマリズムの追求

**Objective:** As a 開発者, I want コードベース全体を最小限に保ちたい, so that 認知負荷が軽減され、保守性と理解容易性が最大化される

#### Acceptance Criteria

1. The Codebase shall 同一機能を実現するために最小限のコード行数を維持する
2. The Codebase shall 不要な抽象化やラッパー関数を含まず、直接的な実装を優先する
3. The Codebase shall 使用されていない関数、型、トレイト、モジュールを含まない（dead code排除）
4. When 新規コードを追加する場合, the Codebase shall 既存コードの再利用を最大化し、重複実装を避ける
5. The Codebase shall ファイル数を最小限に保ち、論理的に関連するコードは同一ファイルに配置する（ただしRequirement 2.3の500行目安と両立させる）
6. The Codebase shall 過度に細分化された小さな関数（1-2行の単純なラッパー）を含まず、インライン化を優先する
7. The Codebase shall 将来の仮想的な要件のためのコード（YAGNI違反）を含まない
