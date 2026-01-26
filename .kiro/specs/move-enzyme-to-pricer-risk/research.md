# Research & Design Decisions: move-enzyme-to-pricer-risk

## Summary

- **Feature**: `move-enzyme-to-pricer-risk`
- **Discovery Scope**: Extension（既存システムの構造変更）
- **Key Findings**:
  1. enzyme内部ファイルは`crate::mc`、`crate::checkpoint`へ依存しており、移動後は`pricer_pricing::*`への参照に更新が必要
  2. pricer_riskはすでにpricer_pricingに依存しているため、移動後もMonteCarloPricer等へのアクセスは可能
  3. verify_enzyme.rsはpricer_pricingに残存できず、pricer_riskへ移動が必要（L3→L4依存禁止）

## Research Log

### enzyme内部の`crate::`依存関係

- **Context**: 移動後にenzymeモジュールがコンパイルできるか調査
- **Sources Consulted**: `crates/pricer_pricing/src/enzyme/*.rs`のgrep結果
- **Findings**:
  - `greeks.rs:30`: `use crate::mc::{GbmParams, MonteCarloPricer, PayoffParams, PricingResult}`
  - `verification.rs:34`: `use crate::mc::{GbmParams, MonteCarloConfig, MonteCarloPricer, PayoffParams}`
  - `checkpoint_ad.rs:40`: `use crate::checkpoint::{CheckpointManager, CheckpointStrategy}`
  - `greeks.rs:701` (tests): `use crate::mc::MonteCarloConfig`
- **Implications**: 移動後、これらは`pricer_pricing::mc::`および`pricer_pricing::checkpoint::`に変更必要

### verify_enzyme.rsの移動要件

- **Context**: pricer_pricing内のenzymeテストファイルの扱い
- **Sources Consulted**: `crates/pricer_pricing/src/verify_enzyme.rs`
- **Findings**:
  - `enzyme::gradient`を使用（移動後はpricer_risk::enzymeへ）
  - `verify::{square, square_gradient}`を使用（pricer_pricingに残る）
  - `path_dependent::PathPayoffType`を使用（pricer_pricingに残る）
- **Implications**:
  - pricer_pricingはpricer_riskに依存できない（L3→L4禁止）
  - verify_enzyme.rsはpricer_riskの統合テストに移動必須
  - または削除してpricer_riskで新規テスト作成

### Nightly Rust伝播の影響

- **Context**: pricer_riskがnightly必須になる影響範囲
- **Sources Consulted**: steering/structure.md、steering/tech.md
- **Findings**:
  - 現状: pricer_pricing (L3)のみnightly、pricer_risk (L4)はstable
  - 移動後: pricer_riskが`#![feature(autodiff)]`を必要とする
  - enzyme-ad featureはオプショナルのため、feature無効時はstable維持可能
- **Implications**:
  - `enzyme-ad` feature有効時のみnightly必要という設計を維持
  - CI/CDでstable/nightlyビルドの分岐が必要

### Dockerビルドへの影響

- **Context**: Dockerfile.nightlyの影響範囲
- **Sources Consulted**: docker/Dockerfile.nightly
- **Findings**:
  - 現在はpricer_pricingのみnightlyでビルド
  - 移動後はpricer_riskもnightlyビルド対象に
- **Implications**: Dockerfile.nightlyの修正は不要（workspaceビルドのため）

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: 完全移動 | enzyme全体をpricer_riskに移動 | 要件準拠、リスク計算とAADが同一クレート | crate::依存の全更新必要、Nightly伝播 | 推奨 |
| B: 部分移動 | AADコアのみ移動、MC連携は残留 | MC依存問題回避 | enzymeが2クレート分散、保守困難 | 非推奨 |
| C: 独立クレート | pricer_enzymeを新設 | 依存関係クリーン | 要件逸脱、クレート追加コスト | 非推奨 |

## Design Decisions

### Decision: `完全移動（Option A）採用`

- **Context**: enzymeモジュールの配置先決定
- **Alternatives Considered**:
  1. Option A — 全11ファイルをpricer_riskに移動
  2. Option B — コア機能のみ移動、MC連携は残留
  3. Option C — 独立クレートpricer_enzymeを新設
- **Selected Approach**: Option A（完全移動）
- **Rationale**:
  - 要件の意図（AADはリスク計算のため）に最も合致
  - pricer_riskはすでにpricer_pricingに依存しているため、MonteCarloPricerへのアクセスは可能
  - `crate::`参照を`pricer_pricing::`に変更するだけで実現可能
- **Trade-offs**:
  - (+) アーキテクチャ上の整合性向上
  - (+) リスク計算関連が同一クレートに集約
  - (-) pricer_riskのnightly必須化（enzyme-ad feature有効時）
  - (-) docstring/ドキュメントの大量更新
- **Follow-up**: verify_enzyme.rsの移動先決定

### Decision: `verify_enzyme.rsをpricer_risk/tests/に移動`

- **Context**: enzymeテストファイルの扱い
- **Alternatives Considered**:
  1. pricer_risk/tests/verify_enzyme.rsに移動
  2. pricer_risk/src/enzyme/tests.rsとして統合
  3. 削除して新規テスト作成
- **Selected Approach**: pricer_risk/tests/verify_enzyme.rsに移動
- **Rationale**:
  - 既存テストを維持しつつ統合テストとして再配置
  - pricer_pricingの`verify`および`path_dependent`への参照を維持
- **Trade-offs**:
  - (+) 既存テスト資産を保持
  - (-) インポートパスの大幅変更が必要
- **Follow-up**: path_dependent依存部分のテスト分離を検討

### Decision: `enzyme-ad featureによるNightly分離維持`

- **Context**: pricer_riskのstable/nightly両対応
- **Alternatives Considered**:
  1. pricer_risk全体をnightly必須化
  2. enzyme-ad featureでnightlyを分離
- **Selected Approach**: enzyme-ad featureでnightly分離
- **Rationale**:
  - 既存のpricer_pricingでの設計パターンを踏襲
  - enzyme-ad無効時はstable Rustでビルド可能
  - `#![cfg_attr(feature = "enzyme-ad", feature(autodiff))]`パターンを適用
- **Trade-offs**:
  - (+) stableビルドユーザーへの影響最小化
  - (+) CIでstable/nightlyビルドを分離可能
  - (-) feature条件分岐が増加
- **Follow-up**: CI/CD設定の更新

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| greeks.rsの`MonteCarloPricer`依存でコンパイルエラー | `pricer_pricing::mc::`への参照変更で解決可能（pricer_riskはpricer_pricingに依存） |
| docstring内のパス参照が大量更新 | sed/ripgrepによる一括置換で対応 |
| verify_enzyme.rsの移動で依存関係複雑化 | 統合テストとして分離、必要最小限のimportに限定 |
| Nightly伝播によるCI影響 | enzyme-ad featureによる分離でstableビルド維持 |
| 他クレートからのpricer_pricing::enzyme参照 | 外部参照は確認されず（docstringのみ）、影響なし |

## References

- [Rust nightly features](https://doc.rust-lang.org/unstable-book/) — `#![feature(autodiff)]`の使用方法
- [Enzyme LLVM AD](https://enzyme.mit.edu/) — Enzyme公式ドキュメント
- steering/structure.md — A-I-P-Sアーキテクチャ定義
- steering/tech.md — 技術スタック定義
