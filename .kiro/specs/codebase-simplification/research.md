# Research & Design Decisions

## Summary
- **Feature**: `codebase-simplification`
- **Discovery Scope**: Extension (既存システムの改善)
- **Key Findings**:
  - 44個のエラー型が存在し、統合の余地が大きい
  - `#[allow(dead_code)]` が11箇所、`pub(crate)` が5箇所のみ
  - 依存関係の重複（rand, getrandom の複数バージョン）

## Research Log

### エラー型の分布調査
- **Context**: 要件6「エラー処理の統一」を設計するための現状把握
- **Sources Consulted**: `crates/*/src/**/*.rs` の `pub enum.*Error` パターン
- **Findings**:
  - 44個の公開エラー型が13クレートに分散
  - `pricer_core`: 7型 (PricingError, DateError, CurrencyError, InterpolationError, SolverError, CalibrationErrorKind, DistributionError)
  - `pricer_models`: 6型 (HestonError, SABRError, CorrelationError, AnalyticalError, CalibrationError, MarketDataError, BootstrapError)
  - `pricer_pricing`: 9型 (MonteCarloConfigError, CheckpointError, FallbackError, GraphError, GreeksConfigError, IrsGreeksError, BenchmarkError, XvaDemoError)
  - `pricer_risk`: 6型 (XvaError, ParallelGreeksError, PortfolioError, CurveShiftError, GreeksByFactorError, BucketDv01Error)
  - `infra_domain`: 4型 (MasterDataError, DateError, CurrencyError, TimeError, CounterPartyError, TradeError)
  - Adapter/Service 層: 6型
- **Implications**:
  - 共通パターン（InvalidInput, NotConverged, NumericalError）の抽出が可能
  - DateError, CurrencyError が pricer_core と infra_domain で重複
  - From 変換チェーンの簡略化が必要

### 未使用コード調査
- **Context**: 要件4「未使用コードの除去」
- **Sources Consulted**: `#[allow(dead_code)]` パターン検索
- **Findings**:
  - 11箇所の dead_code アノテーション
  - 主な場所: pricer_pricing (rng, integration_tests), pricer_core (traits), pricer_models (calibration), service_gateway (config, rest)
  - deprecated 属性は0件（既にクリーンアップ済み）
- **Implications**: 各アノテーションの必要性を個別評価、不要なものは削除

### API 可視性調査
- **Context**: 要件2「API 表面の最小化」
- **Sources Consulted**: `pub(crate)` パターン検索
- **Findings**:
  - `pub(crate)` は5箇所のみ（pricer_risk/demo, service_gateway/graph_handlers, pricer_pricing/irs_greeks, mc/pricer）
  - 多くの内部型が `pub` として公開されている可能性
  - 7クレートに prelude モジュールが存在
- **Implications**: 内部実装詳細の可視性を `pub(crate)` または `pub(super)` に変更

### モジュールサイズ調査
- **Context**: 要件3「モジュール構造の合理化」
- **Sources Consulted**: ファイル行数分析
- **Findings**:
  - 50行未満のファイル: 約30件
  - 最小ファイル: build.rs (7行), config.rs (8行), mod.rs (9-38行)
  - error.rs ファイルが多数存在（各27-38行）
- **Implications**: 小さなエラーファイルの統合、モジュール構造の見直し

### 依存関係重複調査
- **Context**: 要件4「未使用の依存関係」、要件7「Feature フラグの整理」
- **Sources Consulted**: `cargo tree --duplicates`
- **Findings**:
  - getrandom: v0.2.17 と v0.3.4 が共存
  - rand_core: v0.6.4 と v0.9.5 が共存
  - rand: v0.8.5 と v0.9.2 が共存
  - hashbrown: v0.14.5 が重複
- **Implications**: proptest が rand v0.9 系を要求、本番コードは rand v0.8 系 → 統一は困難だが監視は必要

### Feature フラグ調査
- **Context**: 要件7「Feature フラグの整理」
- **Sources Consulted**: 各 Cargo.toml の `[features]` セクション
- **Findings**:
  - 13クレートに feature 定義あり
  - 6クレートの default が空配列 `[]`
  - pricer_core: `["num-dual-mode", "serde", "equity", "parallel"]`
  - pricer_models: `["equity", "serde"]`
  - 資産クラス: equity, rates, credit, fx, commodity, exotic
  - AD モード: num-dual-mode, enzyme-mode
- **Implications**: 未使用 feature の特定、ドキュメント化の強化

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 段階的リファクタリング | 小さな変更を段階的に適用 | 低リスク、継続的検証 | 時間がかかる | 推奨アプローチ |
| 一括リファクタリング | 大規模な変更を一度に適用 | 一貫性が高い | 高リスク、テスト困難 | 非推奨 |
| ハイブリッド | 関連変更をグループ化して適用 | バランスが取れる | 依存関係の管理が複雑 | A-I-P-S 境界で分割 |

## Design Decisions

### Decision: 段階的リファクタリングアプローチ

- **Context**: 機能・性能を維持しながら簡略化する必要がある
- **Alternatives Considered**:
  1. 一括リファクタリング — 全変更を一度に適用
  2. 段階的リファクタリング — 小さな変更を順次適用
  3. ハイブリッド — A-I-P-S 境界でグループ化
- **Selected Approach**: 段階的リファクタリング（A-I-P-S 境界でグループ化）
- **Rationale**:
  - 各段階でテストを実行し、リグレッションを早期発見
  - A-I-P-S 依存関係ルールを維持しやすい
  - ロールバックが容易
- **Trade-offs**:
  - 実装期間が長くなる可能性
  - 中間状態での一時的な複雑性
- **Follow-up**: 各フェーズ完了後にベンチマーク実行

### Decision: エラー型の層別統合

- **Context**: 44個のエラー型を整理する必要がある
- **Alternatives Considered**:
  1. 全クレート共通のエラー型 — 一つの Error enum
  2. 層別エラー型 — Pricer 層、Infra 層、Service 層で分離
  3. 現状維持 — 各モジュールで個別定義
- **Selected Approach**: 層別エラー型（A-I-P-S 境界で分離）
- **Rationale**:
  - A-I-P-S 依存関係ルールとの整合性
  - 各層での独立したエラー処理が可能
  - From 変換の簡略化
- **Trade-offs**:
  - 完全な統一ではない
  - 層間のエラー変換は依然必要
- **Follow-up**: DateError, CurrencyError の重複解消を優先

### Decision: pub(crate) 適用基準

- **Context**: API 表面を最小化する必要がある
- **Alternatives Considered**:
  1. 保守的 — 明らかに内部のもののみ
  2. 積極的 — prelude 以外の全てを pub(crate)
  3. バランス型 — 利用パターンに基づく判断
- **Selected Approach**: バランス型（利用パターン分析に基づく）
- **Rationale**:
  - 外部クレートからの実際の利用を分析
  - 破壊的変更を最小化
  - 段階的な可視性縮小が可能
- **Trade-offs**:
  - 分析に時間を要する
  - 一部のユースケースを見落とす可能性
- **Follow-up**: `cargo doc --document-private-items` で内部 API を確認

## Risks & Mitigations

- **性能劣化リスク** — 各フェーズ後にベンチマーク実行、5% 以上の劣化で原因調査
- **破壊的変更リスク** — semver を遵守、deprecated アノテーションで移行期間を設ける
- **テスト不足リスク** — 変更前後でテストカバレッジを維持、新規共通コードにはテスト必須
- **依存関係破壊リスク** — `cargo tree` で定期的に検証、CI でビルド確認
- **A-I-P-S 違反リスク** — 各変更で依存関係ルールを確認、CI で検出

## References

- [Rust API Guidelines - C-REEXPORT](https://rust-lang.github.io/api-guidelines/documentation.html#c-reexport) — 再エクスポートに関するガイドライン
- [thiserror documentation](https://docs.rs/thiserror/latest/thiserror/) — エラー型設計の参考
- [Rust Edition Guide - Visibility](https://doc.rust-lang.org/edition-guide/rust-2018/module-system/path-clarity.html) — 可視性修飾子
