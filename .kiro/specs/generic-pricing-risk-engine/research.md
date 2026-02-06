# Research & Design Decisions: generic-pricing-risk-engine

## Summary
- **Feature**: `generic-pricing-risk-engine`
- **Discovery Scope**: Complex Extension（既存システム拡張 + モジュール移行）
- **Key Findings**:
  - 既存`GenericPricer`/`BatchPricer`は設定駆動化の基盤として再利用可能
  - `greeks/`/`irs_greeks/`モジュール移行は`#[deprecated]`による段階的移行が推奨
  - `RiskEngine` facadeは既存`ScenarioEngine`/`GreeksAggregator`と統合設計
  - Feature flag (`l1l2-integration`, `enzyme-ad`) による条件コンパイルパターン確立済み

## Research Log

### 既存GenericPricerアーキテクチャ調査
- **Context**: 設定駆動型プライサーの実現可能性確認
- **Sources Consulted**:
  - `crates/pricer_pricing/src/generic_pricer/` (pricer.rs, config.rs, batch.rs, error.rs)
  - `demo/gui/src/web/generic_pricer_handlers.rs`
- **Findings**:
  - `GenericPricer::new(model_config, pricer_config)` で設定投入可能
  - `BatchPricer` はRayon並列処理、`BatchStats` で統計情報提供
  - Dual-mode: `#[cfg(feature = "l1l2-integration")]` で standalone/integrated 切替
  - 既存`PricingError`は構造化済み（trade_id, description含む）
- **Implications**: 設定ファイル読み込みコンストラクタの追加で要件達成可能

### Greeks計算基盤調査
- **Context**: greeks/irs_greeksモジュール移行の影響範囲特定
- **Sources Consulted**:
  - `crates/pricer_pricing/src/greeks/` (mod.rs, config.rs, result.rs)
  - `crates/pricer_pricing/src/irs_greeks/` (mod.rs, calculator.rs, benchmark.rs)
  - `crates/pricer_pricing/src/lib.rs` (re-exports)
- **Findings**:
  - `GreeksConfig`: Builder pattern, `GreeksMode` (BumpRevalue/NumDual/EnzymeAAD)
  - `GreeksResult<T: Float>`: Generic AD対応、serde feature付き
  - `IrsGreeksCalculator`: AAD/Bump dual-mode、`IrsLazyEvaluator` キャッシュ機構
  - Re-export: `pricer_pricing::GreeksConfig`, `GreeksMode`, `GreeksResult` 等
- **Implications**: L3 re-export を `#[deprecated]` 付きで維持し、L4 への移行を段階的に実施

### pricer_riskクレート構造調査
- **Context**: RiskEngine統合先の確認
- **Sources Consulted**:
  - `crates/pricer_risk/src/lib.rs`
  - `crates/pricer_risk/src/scenarios/mod.rs`
- **Findings**:
  - 既存モジュール: portfolio/, exposure/, xva/, scenarios/, soa/, parallel/
  - `scenarios/`: ScenarioEngine, GreeksAggregator, RiskFactorShift, CurveShifter
  - 一部機能が`TODO: Re-enable when rates instruments are restored`で無効化中
  - L4はL3 (pricer_pricing) に依存可能
- **Implications**: `engine/` モジュールを新規追加し、既存scenarios/と統合

### 設定ファイルパターン調査
- **Context**: infra_config拡張方針の決定
- **Sources Consulted**:
  - `crates/infra_config/src/settings.rs`
  - `config/` ディレクトリ構造
- **Findings**:
  - `Settings` 構造体: engine, database, service, logging セクション
  - config crateによるTOML/YAML/Env統合読み込み
  - `#[serde(default)]` によるオプショナルフィールド
  - Builder pattern未使用（derive_builder不使用）
- **Implications**: `PricingConfig`, `RiskConfig` を Settings に追加、同一パターン踏襲

### データローダーパターン調査
- **Context**: JSONローダー実装方針の決定
- **Sources Consulted**:
  - `crates/adapter_loader/src/lib.rs`
  - `crates/adapter_loader/src/csv_loader.rs`
  - `crates/adapter_loader/src/csa.rs`
- **Findings**:
  - `CsvLoader` 汎用CSV読み込み
  - `CsaTerms`, `NettingSet` は `infra_domain` から再エクスポート
  - `LoaderError` 構造化エラー
- **Implications**: `json/` サブモジュール追加、同様のre-exportパターン適用

### Demo Web Handler調査
- **Context**: Service層統合パターンの確認
- **Sources Consulted**:
  - `demo/gui/src/web/generic_pricer_handlers.rs`
  - `demo/gui/src/web/scenario_handlers.rs`
- **Findings**:
  - `*_handlers.rs` + `*_types.rs` 分離パターン
  - Axum State共有 (`Arc<AppState>`)
  - sync pricer kernelのasync handler呼び出し
  - JSON serde Request/Response
- **Implications**: 同一パターンで `risk_engine_handlers.rs` 追加可能

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Facade Pattern | RiskEngine as unified facade over existing components | Clear API surface, backward compatible | Additional abstraction layer | Aligns with existing GenericPricer pattern |
| Direct Integration | Merge greeks directly into scenarios/ | Less abstraction | Breaking change, complex migration | Not recommended |
| Adapter Pattern | Wrap L3 greeks with L4 adapter | Clean separation | Performance overhead | Overkill for internal crate migration |

**Selected**: Facade Pattern - `RiskEngine` as L4 facade wrapping L3 `enzyme::gradient` and existing `scenarios/` components

## Design Decisions

### Decision: 段階的モジュール移行戦略

- **Context**: `greeks/`, `irs_greeks/` をL3からL4へ移行する際の既存コードへの影響最小化
- **Alternatives Considered**:
  1. 即時移行 + 破壊的変更通知
  2. 段階的移行 + deprecation警告
  3. 長期並存（重複コード維持）
- **Selected Approach**: 段階的移行 + deprecation警告
  - Phase 1: L4にモジュールコピー、L4から正式公開
  - Phase 2: L3に`#[deprecated]`付きre-export
  - Phase 3: 1リリース後にL3から削除
- **Rationale**: 下流コードの移行期間を確保、コンパイラ警告で移行促進
- **Trade-offs**: 一時的なコード重複、2リリースサイクル必要
- **Follow-up**: deprecation期間中の移行ガイドドキュメント作成

### Decision: 設定ファイルスキーマ設計

- **Context**: PricingConfig/RiskConfigのTOML/JSON共通スキーマ
- **Alternatives Considered**:
  1. 単一設定ファイル（pricing + risk統合）
  2. 分離設定ファイル（pricing.toml, risk.toml）
  3. ネスト構造（unified.toml内にセクション分離）
- **Selected Approach**: ネスト構造（Settings内にpricing, riskセクション）
- **Rationale**: 既存Settingsパターンとの一貫性、単一ファイル管理の利便性
- **Trade-offs**: ファイルサイズ増加、セクション間依存の可能性
- **Follow-up**: 設定バリデーション時の相互参照チェック実装

### Decision: Enzyme呼び出しパターン

- **Context**: L4 RiskEngineからのEnzyme AAD呼び出し方法
- **Alternatives Considered**:
  1. L4から直接`pricer_pricing::enzyme::gradient`呼び出し
  2. L4に委譲インターフェース、L3で実装
  3. L4にEnzyme統合層を構築
- **Selected Approach**: L4から直接L3呼び出し（依存関係ルール遵守）
- **Rationale**: L4→L3依存は許可、追加抽象化不要
- **Trade-offs**: L3内部実装への直接依存
- **Follow-up**: Feature flag (`enzyme-ad`) の伝播確認

### Decision: RiskEngine API設計

- **Context**: 単一取引/ポートフォリオ両対応のAPI設計
- **Alternatives Considered**:
  1. オーバーロード（Rust非対応）
  2. 別メソッド（compute_single, compute_portfolio）
  3. ジェネリック入力（Into<TradeOrPortfolio>）
  4. enum入力（RiskInput::Single/Portfolio）
- **Selected Approach**: 別メソッド（明示的API）
  - `RiskEngine::compute_greeks(&self, trade: &Trade, config: &RiskConfig) -> RiskResult`
  - `RiskEngine::compute_portfolio_greeks(&self, trades: &[Trade], config: &RiskConfig) -> PortfolioRiskResult`
- **Rationale**: 明示的で型安全、ドキュメント明確
- **Trade-offs**: メソッド数増加
- **Follow-up**: Builder patternでの設定注入オプション検討

### Decision: コード完全削除戦略

- **Context**: 移行完了後の不要コード削除範囲と実行タイミング
- **Alternatives Considered**:
  1. 即時削除（移行と同時）
  2. 段階的削除（deprecation期間後）
  3. 永続的並存（後方互換性維持）
- **Selected Approach**: 段階的削除（1リリースサイクル後に完全削除）
- **Deletion Scope**:
  - `pricer_pricing/src/greeks/` — 5ファイル, ~1,279 LOC
  - `pricer_pricing/src/irs_greeks/` — 9ファイル, ~7,248 LOC
  - `pricer_pricing/src/lib.rs` re-exports — 28型
  - `generic_pricer/greeks_calculator.rs` 内 `BumpSizes` — ~50 LOC
  - **合計: ~8,577 LOC 削除**
- **Rationale**:
  - 重複コード排除によるメンテナンスコスト削減
  - A-I-P-Sアーキテクチャ準拠（Greeks計算はL4の責務）
  - pricer_pricing クレートの責務明確化（AD engine + MC kernel のみ）
- **Trade-offs**: 移行期間中は一時的なコード重複
- **Follow-up**:
  - 削除前に全downstream import更新を確認
  - `cargo test --all-features` で全テスト通過を確認
  - CHANGELOGにbreaking change記載

## Risks & Mitigations

- **Risk 1: モジュール移行による破壊的変更** — deprecation期間設定、移行ガイド提供
- **Risk 2: Enzyme feature flag伝播漏れ** — Cargo.tomlでfeature propagation確認、CI追加
- **Risk 3: 設定スキーマ後方互換性** — serde default値、optional fields、バージョニング検討
- **Risk 4: ポートフォリオ並列処理のメモリ使用量** — 既存MemoryMonitor活用、バッチサイズ制御

## References

- [Rust API Guidelines - Deprecation](https://rust-lang.github.io/api-guidelines/necessities.html#c-deprecated) — deprecation属性の標準的な使用方法
- [serde - Default Values](https://serde.rs/attr-default.html) — 設定ファイルのオプショナルフィールド処理
- [Rayon - Parallel Iterators](https://docs.rs/rayon/latest/rayon/) — ポートフォリオ並列処理パターン
- 既存仕様: `generic-pricer-engine` spec (2026-01-23完了) — GenericPricer基盤設計
