# Research & Design Decisions: Production Hardening Phase 6

---
**Purpose**: Phase 6のDiscoveryフェーズで得られた調査結果とアーキテクチャ決定の根拠を記録。

---

## Summary

- **Feature**: `production-hardening-phase6`
- **Discovery Scope**: Extension（既存インフラへの機能追加）
- **Key Findings**:
  - Criterion v0.7.0が現行標準、`black_box`とharness無効化が必須
  - cargo-tarpaulinはLinux限定、grcovはクロスプラットフォーム対応
  - crates.io Trusted Publishing（OIDC）が2025年に導入、セキュアなCI/CD公開

---

## Research Log

### Criterionベンチマークのベストプラクティス

- **Context**: Requirement 3（ベンチマーク）の技術選定
- **Sources Consulted**:
  - [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/getting_started.html)
  - [GitHub: criterion.rs](https://github.com/bheisler/criterion.rs)
  - [crates.io: criterion](https://crates.io/crates/criterion)
- **Findings**:
  - 最新バージョン: v0.7.0（132.5M+ downloads）
  - 開発は新しいCriterion-rs organizationに移行
  - `black_box`で最適化防止が必須
  - `harness = false`設定でCriterion独自のハーネスを使用
  - HTML/JSON/XMLレポート生成対応
  - MSRV: Rust 1.80+
- **Implications**:
  - 各クレートのCargo.tomlに`[[bench]]`セクション追加
  - gnuplotまたはplottersでHTMLレポート生成
  - CIでベンチマーク結果をアーティファクトとして保存可能

### コードカバレッジツール比較

- **Context**: Requirement 6（品質ゲート）のカバレッジツール選定
- **Sources Consulted**:
  - [GitHub: tarpaulin](https://github.com/xd009642/tarpaulin)
  - [How to do code coverage in Rust](https://blog.rng0.io/how-to-do-code-coverage-in-rust/)
  - [Coverage - Rust Project Primer](https://rustprojectprimer.com/measure/coverage.html)
- **Findings**:

| Feature | cargo-tarpaulin | grcov |
|---------|-----------------|-------|
| Platform | Linux x86_64（LLVM engine でMac/Windows対応可） | クロスプラットフォーム |
| Ease of use | シンプル（1コマンド） | 設定が必要（RUSTFLAGS等） |
| Best for | ローカル開発、シンプルなプロジェクト | CI/CD、複雑なマルチスイート |
| Accuracy | 信頼性あり（軽微な不正確さあり） | LLVMベースで高精度 |
| Aggregation | 基本的 | 複数テスト結果の統合に優れる |

- **Implications**:
  - CIではgrcov（クロスプラットフォーム、LLVM精度）を推奨
  - ローカル開発者向けにtarpaulinもドキュメント化
  - Codecov/Coverallsへのレポートアップロード対応

### GitHub Actions リリースワークフロー

- **Context**: Requirement 7（リリース自動化）のCI/CD設計
- **Sources Consulted**:
  - [crates.io development update 2025](https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07/)
  - [GitHub: katyo/publish-crates](https://github.com/katyo/publish-crates)
  - [semantic-release-cargo](https://crates.io/crates/semantic-release-cargo)
- **Findings**:
  - **Trusted Publishing (2025新機能)**: RFC #3691に基づくOIDC認証
    - APIトークン不要、GitHub Secretsの管理負担軽減
    - `rust-lang/crates-io-auth-action@v1`使用
    - `permissions: id-token: write`が必要
  - **katyo/publish-crates**: ワークスペース対応、依存順序で自動公開
  - **cargo-release**: ドライラン機能、バージョンバンプ自動化
- **Implications**:
  - 新しいTrusted Publishingを採用（セキュリティ向上）
  - ワークスペースの依存順序に従った公開（pricer_core → pricer_models → pricer_kernel → pricer_xva）

### CHANGELOG自動生成

- **Context**: Requirement 7（リリース自動化）のCHANGELOG生成
- **Sources Consulted**:
  - [git-cliff official](https://git-cliff.org/)
  - [GitHub: git-cliff](https://github.com/orhun/git-cliff)
- **Findings**:
  - 最新バージョン: v2.11.0
  - Conventional Commits仕様に対応
  - Jinja2/Django風テンプレートエンジン
  - Rust/Python/Node.jsから利用可能
  - パス単位のinclude/exclude対応
- **Implications**:
  - `cliff.toml`設定ファイルでフォーマットカスタマイズ
  - リリースワークフローでCHANGELOG自動更新

### 現行ドキュメントカバレッジ状況

- **Context**: Requirement 1（APIドキュメント）の現状把握
- **Sources Consulted**: コードベース分析（Grep）
- **Findings**:
  - 全4クレートに`#![warn(missing_docs)]`設定済
  - `#![deny(missing_docs)]`への変更が必要
  - クレートレベルdocコメントは完備
  - 関数レベルのExamplesは不均一
- **Implications**:
  - warn → deny への変更でCI品質ゲート強化
  - パブリック関数への`# Examples`セクション追加が必要

---

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Single ci.yml | 全ジョブを1ファイルに統合 | 一元管理、依存関係が明確 | ファイル肥大化（220行→400行以上） | 既存パターン |
| Modular workflows | 目的別にファイル分離（ci/release/docs） | 責務分離、個別実行可能 | ファイル数増加、重複コード | 大規模プロジェクト向け |
| **Hybrid** | ci.yml + release.yml の2ファイル構成 | バランス、リリース分離 | 軽度の複雑化 | **採用** |

---

## Design Decisions

### Decision: CI/CDワークフロー構成

- **Context**: 既存ci.ymlにカバレッジ・ベンチマーク・リリースを追加する方法
- **Alternatives Considered**:
  1. 全てをci.ymlに追加 — シンプルだが肥大化
  2. 4ファイル分離 — 過度な分離
  3. ci.yml + release.yml — バランス型
- **Selected Approach**: Hybrid（ci.yml拡張 + release.yml新規作成）
- **Rationale**: リリースは異なるトリガー（タグ）で動作し、頻度も低いため分離が適切
- **Trade-offs**: CI/CDファイル数が2つになるが、責務が明確

### Decision: カバレッジツール選定

- **Context**: Linux CI + クロスプラットフォーム対応の両立
- **Alternatives Considered**:
  1. cargo-tarpaulin — シンプルだがLinux限定
  2. grcov — 設定複雑だがクロスプラットフォーム
  3. llvm-cov — 公式だが設定が煩雑
- **Selected Approach**: cargo-tarpaulin（主）+ grcovドキュメント化（将来対応）
- **Rationale**: 現行CIはLinuxベース、シンプルさ優先
- **Trade-offs**: Windows/macOSカバレッジは将来対応

### Decision: crates.io公開方式

- **Context**: セキュアかつ自動化された公開フロー
- **Alternatives Considered**:
  1. 従来のAPIトークン方式 — Secretsの管理負担
  2. Trusted Publishing（OIDC） — 2025年新機能、トークン不要
- **Selected Approach**: Trusted Publishing（将来対応）、現行はセマンティックバージョニング準備のみ
- **Rationale**: 新機能だが安全、将来のcrates.io公開に備える
- **Follow-up**: crates.ioでのTrusted Publishing設定が必要

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Enzymeの不安定なCI | フォールバック機構（既に実装済）、条件付きジョブ |
| ベンチマーク結果のばらつき | 複数回実行、統計的分析（Criterion標準機能） |
| カバレッジツールの互換性 | tarpaulin→grcovへの移行パスを文書化 |
| クロスプラットフォームビルドの失敗 | matrix戦略でOS別にフォールバック |

---

## References

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/getting_started.html) — ベンチマーク設定ガイド
- [GitHub: tarpaulin](https://github.com/xd009642/tarpaulin) — カバレッジツール公式
- [crates.io Trusted Publishing](https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07/) — OIDC認証方式
- [git-cliff](https://git-cliff.org/) — CHANGELOG生成ツール
- [GitHub: katyo/publish-crates](https://github.com/katyo/publish-crates) — ワークスペース公開Action
