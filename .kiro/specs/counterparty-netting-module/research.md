# Research & Design Decisions

## Summary
- **Feature**: `counterparty-netting-module`
- **Discovery Scope**: Extension（既存`counterparty.rs`をモジュール構造に拡張）
- **Key Findings**:
  - `pricer_risk`に既存のID型・クレジット型が存在（型重複）
  - `infra_master`の`time/`、`trade/`、`convention/`モジュールパターンが確立済み
  - A-I-P-S依存規則に従い、`infra_master`は計算ロジックを持たない

## Research Log

### 既存実装の調査

- **Context**: 要件定義に先立ち、既存のCounterParty関連実装を調査
- **Sources Consulted**:
  - `crates/infra_master/src/counterparty.rs`
  - `crates/pricer_risk/src/portfolio/counterparty.rs`
  - `crates/pricer_risk/src/portfolio/ids.rs`
  - `crates/pricer_risk/src/portfolio/netting_set.rs`
- **Findings**:
  - `infra_master::CsaTerms`: threshold, mta, independent_amount, mpor_days, margin_currency
  - `infra_master::NettingSetConfig`: id, counterparty_id, csa_terms
  - `pricer_risk::CreditRating`: AAA～D（10段階、+/-なし）
  - `pricer_risk::CreditParams`: hazard_rate, lgd, rating, survival_prob(), default_prob()
  - `pricer_risk::Counterparty`: id, name, credit_params
  - `pricer_risk`のID型: `CounterpartyId`, `NettingSetId`, `TradeId`（新型パターン）
- **Implications**:
  - `infra_master`に移行する型は`pricer_risk`の機能を包含しつつ拡張
  - 後方互換性のため、`pricer_risk`は将来的に`infra_master`から再エクスポート

### モジュール構造パターンの調査

- **Context**: `infra_master`内の既存モジュール構造を参考にする
- **Sources Consulted**:
  - `crates/infra_master/src/time/mod.rs`
  - `crates/infra_master/src/trade/mod.rs`
  - `crates/infra_master/src/convention/mod.rs`
- **Findings**:
  - 共通パターン: `mod.rs`でサブモジュールを宣言、`pub use`で再エクスポート
  - エラー型: 各モジュールに専用`error.rs`
  - 型定義: 論理的なグループごとにファイル分割
  - Prelude: `mod.rs`内で`pub mod prelude`を定義
- **Implications**: 同一パターンに従い、`counterparty/`モジュールを構成

### CreditRating詳細設計

- **Context**: 要件3で+/-ノッチ付き20段階格付けが要求されている
- **Sources Consulted**:
  - S&P格付け体系（業界標準）
  - `pricer_risk::CreditRating`（現行10段階）
- **Findings**:
  - S&P基準: AAA, AA+, AA, AA-, A+, A, A-, BBB+, BBB, BBB-, BB+, BB, BB-, B+, B, B-, CCC, CC, C, D
  - Investment Grade境界: BBB-以上
  - `indicative_hazard_rate()`は各格付けに対応するデフォルト確率の参考値を返す
- **Implications**:
  - 20段階enumを新規定義（`pricer_risk`の10段階からの拡張）
  - `is_investment_grade()`メソッドはBBB-を含む（BBB-以上がtrue）

### XVA計算との統合

- **Context**: `pricer_risk`のXVA計算が`CreditParams`を使用
- **Sources Consulted**:
  - `crates/pricer_risk/src/xva/cva.rs`
  - `crates/pricer_risk/src/xva/params.rs`
  - `crates/pricer_risk/src/exposure/mod.rs`
- **Findings**:
  - `compute_cva()`: EE, time_grid, credit_paramsを受け取る
  - `CreditParams`: `survival_prob(t)`, `default_prob(t)`, `marginal_default_prob(t1, t2)`
  - `OwnCreditParams`: DVA計算用（hazard_rate, lgd）
  - Exposure計算: `ExposureProfile`でEE/EPE/PFE/EEPEを計算
- **Implications**:
  - `infra_master::CreditParams`は同等のメソッドを提供
  - `ExposureConfig`は設定のみ（計算ロジックは`pricer_risk`に残る）

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: 既存拡張 | `counterparty.rs`→`counterparty/`に展開 | 後方互換、既存パターン準拠 | 型重複は解決しない | 最小影響 |
| B: 統一移行 | `infra_master`構築＋`pricer_risk`即時移行 | Single Source of Truth | APIブレーク、大規模変更 | リスク大 |
| C: ハイブリッド | Phase 1: `infra_master`構築、Phase 2: 将来移行 | 段階的移行、最小リスク | 一時的型重複 | **推奨** |

## Design Decisions

### Decision: ID型の内部表現

- **Context**: `CounterPartyId`等の新型パターンでの内部表現を決定
- **Alternatives Considered**:
  1. `String` — シンプル、既存パターンと同一
  2. `Arc<str>` — Clone時のコピー削減
  3. `SmolStr` — 小文字列最適化
- **Selected Approach**: `String`
- **Rationale**:
  - `pricer_risk`の既存ID型が`String`を使用
  - 将来的な移行時の互換性確保
  - シンプルさ優先（最適化は必要時に行う）
- **Trade-offs**: メモリ効率より互換性・シンプルさを優先
- **Follow-up**: パフォーマンス要件が明確になった時点で再検討

### Decision: CreditRating +/-ノッチの実装

- **Context**: 20段階格付けの表現方法
- **Alternatives Considered**:
  1. フラットenum — 20バリアント（AAA, AAPlus, AA, AAMinus, ...）
  2. 構造体 — `Rating { base: BaseRating, modifier: Modifier }`
  3. フラットenum + Rustネーミング — `AaPlus`, `AaMinus`
- **Selected Approach**: フラットenum（Rustネーミング）
- **Rationale**:
  - シンプルな実装
  - `Ord`トレイトで自然な順序付け
  - serdeでは文字列表現（"AA+", "AA-"）に変換
- **Trade-offs**: バリアント数が多いが、型安全性と網羅性チェックが得られる
- **Follow-up**: serde属性で`AA+`→`AaPlus`のマッピングを実装

### Decision: 通貨別閾値の表現

- **Context**: CSA条件の通貨別threshold/MTA設定
- **Alternatives Considered**:
  1. `HashMap<Currency, f64>` — 柔軟だがボイラープレート
  2. `Vec<(Currency, f64)>` — 順序保持、イテレーション向き
  3. 専用構造体 — `CurrencyThresholds { entries: Vec<CurrencyThreshold> }`
- **Selected Approach**: `HashMap<Currency, f64>`
- **Rationale**:
  - 通貨でのルックアップが主なユースケース
  - `infra_master::Currency`がHashを実装済み
  - シンプルな実装
- **Trade-offs**: serde時の順序不定だが、設定データでは問題なし
- **Follow-up**: なし

### Decision: pricer_riskとの型統合戦略

- **Context**: `pricer_risk`に既存のID型・クレジット型との関係
- **Alternatives Considered**:
  1. 即時移行 — `pricer_risk`を`infra_master`依存に変更
  2. 共存 — 両方で独立した型を維持
  3. 段階的移行 — Phase 1で`infra_master`構築、Phase 2で移行
- **Selected Approach**: 段階的移行（Option C）
- **Rationale**:
  - A-I-P-S依存規則: `P`(ricer)は`I`(nfra)に依存可能
  - 後方互換性を維持しつつ、将来の統合を可能に
  - Phase 1の影響範囲を最小化
- **Trade-offs**: 一時的な型重複（許容可能）
- **Follow-up**: Phase 2で`pricer_risk`のID型を`infra_master`からの再エクスポートに変更

## Risks & Mitigations

- **型重複による混乱** — 明確なドキュメントで`infra_master`を正式な定義元と明示
- **後方互換性の破壊** — クレートルートからの再エクスポートで既存コードを維持
- **pricer_risk統合の遅延** — Phase 2として明示的にロードマップに記載

## References

- [ISO 17442 LEI Standard](https://www.iso.org/standard/75998.html) — Legal Entity Identifier仕様
- [S&P Rating Scale](https://www.standardandpoors.com) — 格付け体系
- [ISDA SIMM](https://www.isda.org/simm/) — Standard Initial Margin Model
- [UMR Regulations](https://www.bis.org/bcbs/publ/d317.htm) — Uncleared Margin Rules
