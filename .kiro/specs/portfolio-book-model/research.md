# Research & Design Decisions

## Summary
- **Feature**: `portfolio-book-model`
- **Discovery Scope**: Complex Integration
- **Key Findings**:
  - 既存の`infra_master`は型安全なID定義マクロ、Builderパターン、thiserrorエラー型を一貫して使用
  - `pricer_risk::portfolio`はHashMapベースのO(1)ルックアップとRayon並列処理を採用
  - CounterpartyPortfolio階層（CP → ISDA → CSA → Trade）は参照実装に基づく新規設計が必要

## Research Log

### 既存IDパターンの調査
- **Context**: BookId, PortfolioIdの定義方法を既存パターンに合わせる必要性
- **Sources Consulted**: `crates/infra_master/src/ids.rs`, `crates/infra_master/src/counterparty/ids.rs`
- **Findings**:
  - `define_id!`マクロで一貫した型安全ID生成
  - 標準derive: `Clone, Debug, Default, PartialEq, Eq, Hash`
  - serde: `#[cfg_attr(feature = "serde", derive(...))]`, `#[serde(transparent)]`
  - バリデーション付きID（例: `LegalEntityId`）は`new()` + `new_unchecked()`パターン
- **Implications**: 新規ID型（IsdaAgreementId, VmCsaIdなど）も同一パターンで定義

### Builderパターン調査
- **Context**: Book, Portfolio, IsdaMasterAgreement等の構築方法
- **Sources Consulted**: `NettingSetBuilder`, `CounterPartyBuilder`, `CsaTermsBuilder`
- **Findings**:
  - 必須フィールドはコンストラクタ引数、オプションはbuilderメソッド
  - `build()`は`Result<T, Error>`を返却（バリデーション付き）
  - Method chaining: `fn xxx(mut self, ...) -> Self`
- **Implications**: 全エンティティでBuilderパターン採用、複雑なバリデーションはbuild()で実行

### エラーハンドリング調査
- **Context**: BookError, PortfolioError, NettingError等の設計
- **Sources Consulted**: `CounterPartyError`, `PortfolioError`
- **Findings**:
  - thiserror crateで`#[derive(Debug, Error, Clone, PartialEq)]`
  - `#[error("...")]`でDisplay実装自動生成
  - `From`トレイトでエラー型変換（親エラー型への伝播）
- **Implications**: ドメイン別エラー型定義、統合エラー型への`From`実装

### Portfolio構造調査
- **Context**: HashMap vs Vec、関係管理方法
- **Sources Consulted**: `pricer_risk::portfolio::Portfolio`, `PortfolioBuilder`
- **Findings**:
  - `HashMap<Id, Entity>`でO(1)ルックアップ
  - 関係はID参照（所有権ではなく参照）
  - Builderでバリデーション：重複ID、参照整合性チェック
  - Rayon `par_iter()`で並列処理サポート
- **Implications**: CounterpartyPortfolioも同様のHashMap構造 + 参照ベース関係

### CSA/Margin条件調査
- **Context**: 非対称条件（Counterparty vs Own）のモデル化
- **Sources Consulted**: `CsaTerms`, `MarginTerms`, C++参照実装
- **Findings**:
  - 現行実装は対称条件のみ（単一threshold, mta）
  - C++参照は非対称: `thresholdCtpy_`, `thresholdUser_`, `haircutCtpy_`, `haircutUser_`
  - 動的IA: `ia + k * max/min(PV, 0)`
- **Implications**: VmCsa構造体で非対称フィールド追加、`AsymmetricTerms`サブ構造体検討

### 事前計算Exposure調査
- **Context**: インクリメンタル計算のためのExposure Path構造
- **Sources Consulted**: C++参照実装`otherExposurePath_`
- **Findings**:
  - `BTreeMap<Date, Vec<f64>>`でpath単位のexposure格納
  - Currency情報必須（通貨変換のため）
  - CSA/ISDA/NoDoc各レベルで事前計算サポート
- **Implications**: `PreCalculatedExposurePath`構造体、通貨バリデーション必須

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Flat HashMap | 全エンティティをHashMapで管理 | シンプル、O(1)ルックアップ | 階層関係の表現が難しい | 現行pricer_riskパターン |
| Nested Ownership | 親が子を所有（Vec/HashMap） | 階層が明確 | 所有権の複雑さ、更新困難 | C++参照実装パターン |
| **Reference Graph** | ID参照で関係表現、フラット格納 | 柔軟性、更新容易 | 参照整合性バリデーション必要 | **採用**: 既存パターンと整合 |

## Design Decisions

### Decision: `CounterpartyPortfolio`階層の表現方法
- **Context**: CounterpartyPortfolio → ISDA → CSA → Trade の階層をRustで表現
- **Alternatives Considered**:
  1. Nested Ownership（C++参照パターン）: `Vec<IsdaMasterAgreement>`を直接所有
  2. Reference Graph: ID参照で関係表現、各エンティティはフラットに格納
- **Selected Approach**: Reference Graph + 階層ナビゲーションメソッド
- **Rationale**:
  - 既存`pricer_risk::portfolio`パターンとの一貫性
  - エンティティ更新時の所有権問題回避
  - 並列処理との親和性（HashMap → par_iter）
- **Trade-offs**:
  - バリデーションロジックが複雑になる
  - 関係ナビゲーションに追加メソッド必要
- **Follow-up**: Builder内での参照整合性バリデーション実装

### Decision: 非対称CSA条件のモデル化
- **Context**: Counterparty側とOwn側で異なるthreshold/MTA/IA/haircut
- **Alternatives Considered**:
  1. フラットフィールド: `threshold_counterparty`, `threshold_own`を直接
  2. サブ構造体: `AsymmetricTerms { counterparty: f64, own: f64 }`
- **Selected Approach**: フラットフィールド
- **Rationale**:
  - C++参照実装との1:1対応
  - シリアライゼーションがシンプル
  - 計算ロジックでの直接アクセス
- **Trade-offs**: フィールド数増加（8フィールド追加）

### Decision: IsdaMasterAgreement位置
- **Context**: ISDAをcounterparty/配下に配置 vs 独立モジュール
- **Alternatives Considered**:
  1. counterparty/isda.rs
  2. agreement/isda.rs（新規モジュール）
- **Selected Approach**: `counterparty/isda.rs`
- **Rationale**:
  - ISDAはCounterpartyとの関係が密接
  - 既存counterpartyモジュールの拡張として自然
  - netting_set, csaと同レベルの構造
- **Trade-offs**: counterpartyモジュールの肥大化リスク

### Decision: Book → NettingSet関係の方向
- **Context**: BookとNettingSetの関係設計
- **Alternatives Considered**:
  1. Book所有: `Book { netting_set_ids: Vec<NettingSetId> }`
  2. NettingSet参照: `NettingSet { book_id: BookId }`
  3. 双方向: 両方に参照
- **Selected Approach**: NettingSetからBookへの参照（`book_id`フィールド追加）
- **Rationale**:
  - 既存NettingSetへの拡張で実現可能
  - TradeがBookを参照する構造との整合性
  - クエリ: 「このBookのNettingSet一覧」はfilterで実現
- **Trade-offs**: Book→NettingSetクエリがO(n)

### Decision: XVA/Exposure計算構造の配置
- **Context**: XvaScope, ExposureConfig等をinfra_masterに配置するか
- **Alternatives Considered**:
  1. infra_master: 設定/定義のみ
  2. pricer_risk: 計算ロジックと共に
- **Selected Approach**: infra_master（設定構造体のみ）
- **Rationale**:
  - A-I-P-S階層分離原則
  - infra_masterは静的定義、pricer_riskは計算
  - 設定と計算の分離
- **Trade-offs**: 設定とロジックが分離するため、設定変更時の影響追跡が必要

## Risks & Mitigations

- **参照整合性の複雑さ**: 多数のID参照関係により、不整合発生リスク
  - Mitigation: Builder内での厳格なバリデーション、`ValidationResult::Multiple`で複数エラー収集

- **既存コード破壊**: `TradeMetadata.book`の`Option<BookId>`→`BookId`変更
  - Mitigation: 既存コード更新タスクを明示的に定義、段階的移行不要（要件確認済み）

- **パフォーマンス**: 深い階層ナビゲーションのコスト
  - Mitigation: キャッシュ機構、必要に応じてフラット化したビュー提供

- **モジュール肥大化**: counterpartyモジュールの複雑化
  - Mitigation: サブモジュール分割（isda.rs, vm_csa.rsなど）

## References

- [thiserror crate](https://docs.rs/thiserror/) — 構造化エラー型のderive macro
- [Rayon並列処理](https://docs.rs/rayon/) — Portfolio並列処理パターン
- [ISDA Master Agreement](https://www.isda.org/) — ISDAマスター契約の業界標準
- C++参照実装（プロジェクト内）— CounterpartyPortfolio階層構造の参照
