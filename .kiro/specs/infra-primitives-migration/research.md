# Research & Design Decisions

## Summary
- **Feature**: `infra-primitives-migration`
- **Discovery Scope**: Complex Integration（既存システムの大規模リファクタリング）
- **Key Findings**:
  - `DayCountConvention`が`pricer_core`と`infra_domain`の2箇所に重複定義（統合必須）
  - `pricer_core`の基本型は成熟しており、そのままinfra_domainへ移動可能
  - Direction型が4箇所に散在（`Direction`, `SwapDirection`, `FxForwardDirection`, `CdsDirection`）

## Research Log

### 現在のDayCountConvention重複問題

- **Context**: A-I-P-Sルール準拠のため、基本型の移動先を決定
- **Sources Consulted**:
  - `crates/pricer_core/src/types/time.rs`
  - `crates/infra_domain/src/day_count.rs`
- **Findings**:
  - **pricer_core版**（3 variants）: `ActualActual365`, `ActualActual360`, `Thirty360`
    - `year_fraction(NaiveDate, NaiveDate) -> f64`メソッド
    - `year_fraction_dates(Date, Date) -> f64`メソッド（負値対応）
    - `FromStr`, `Display`, `serde`実装完備
    - 包括的なテスト（property-based含む）
  - **infra_domain版**（7 variants）: `Actual360`, `Actual365Fixed`, `Actual36525`, `ActualActualIsda`, `Thirty360Bond`, `Thirty360European`, `ThirtyE360Isda`
    - `year_fraction(NaiveDate, NaiveDate) -> f64`のみ
    - `Default`実装あり（`Actual365Fixed`）
    - テストは基本的なもののみ
- **Implications**:
  - infra_domain版のvariant網羅性が高い（ISDA規格準拠）
  - pricer_core版の実装品質が高い（エラーハンドリング、serde、テスト）
  - **統合戦略**: infra_domain版のvariantsにpricer_core版の実装品質を適用

### Currency型の現状分析

- **Context**: `Currency`をinfra_domainへ移動する際の互換性確認
- **Sources Consulted**: `crates/pricer_core/src/types/currency.rs`
- **Findings**:
  - 5 variants: `USD`, `EUR`, `GBP`, `JPY`, `CHF`
  - `code() -> &'static str`、`decimal_places() -> u8`メソッド
  - `FromStr`（case-insensitive）、`Display`、`Hash`、`Copy`、`Clone`実装
  - `serde`フィーチャーフラグ対応
  - `#[non_exhaustive]`で将来の拡張に対応
- **Implications**: そのまま移動可能。追加通貨（AUD, CAD, SGD等）の検討余地あり

### Date型の現状分析

- **Context**: `Date`をinfra_domainへ移動する際の設計確認
- **Sources Consulted**: `crates/pricer_core/src/types/time.rs`
- **Findings**:
  - `chrono::NaiveDate`のnewtypeラッパー
  - `from_ymd(i32, u32, u32) -> Result<Date, DateError>`
  - `today() -> Date`（ローカル時刻）
  - `parse(&str) -> Result<Date, DateError>`（ISO 8601）
  - `Sub<Date> -> i64`（日数差）、`Add<i64> -> Date`（日付加算）
  - `serde(transparent)`でISO 8601形式シリアライズ
- **Implications**: そのまま移動可能。`into_inner() -> NaiveDate`で下位互換性維持

### Direction型の散在問題

- **Context**: 取引方向を表す型の統合検討
- **Sources Consulted**:
  - `crates/pricer_models/src/instruments/forward.rs` - `Direction`
  - `crates/pricer_models/src/instruments/rates/swap.rs` - `SwapDirection`
  - `crates/pricer_models/src/instruments/fx/forward.rs` - `FxForwardDirection`
  - `crates/pricer_models/src/instruments/credit/cds.rs` - `CdsDirection`
- **Findings**:
  - `Direction`: `Long`, `Short`（汎用）
  - `SwapDirection`: `PayFixed`, `ReceiveFixed`（金利スワップ専用）
  - `FxForwardDirection`: `Buy`, `Sell`（FX専用）
  - `CdsDirection`: `BuyProtection`, `SellProtection`（CDS専用）
  - 各型に`sign<T: Float>() -> T`等のメソッドあり
- **Implications**:
  - 汎用`Direction`（Long/Short）はinfra_domainへ移動可能
  - 専用型（SwapDirection等）は商品固有のため移動対象外
  - `From`トレイトで相互変換を提供

### RateIndex型の分析

- **Context**: ベンチマーク金利指標のマスターデータ化
- **Sources Consulted**: `crates/pricer_models/src/instruments/rates/swap.rs`
- **Findings**:
  - 6 variants: `Sofr`, `Tonar`, `Euribor3M`, `Euribor6M`, `Sonia`, `Saron`
  - 各指標に通貨・テナー・DCC情報が暗黙的に関連
  - 明示的なメタデータメソッドは未実装
- **Implications**:
  - infra_domainへ移動し、`currency()`, `tenor()`, `day_count_convention()`メソッド追加
  - `Tenor`型の新規追加が必要（RateIndexの依存先）

### Frequency型の分析

- **Context**: 支払頻度のマスターデータ化
- **Sources Consulted**: `crates/pricer_models/src/schedules/frequency.rs`
- **Findings**:
  - 6 variants: `Annual`, `SemiAnnual`, `Quarterly`, `Monthly`, `Weekly`, `Daily`
  - `months_per_period() -> u32`、`periods_per_year() -> u32`実装済み
- **Implications**: そのまま移動可能

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **Direct Move + Re-export** | 型をinfra_domainへ移動し、pricer_coreから再エクスポート | シンプル、後方互換性維持 | 依存関係の循環リスク | **採用**: pricer_core→infra_domainの依存追加で解決 |
| Trait Abstraction | 共通トレイトをinfra_domainに定義し、具体型は各クレートに残す | 疎結合 | 複雑、オーバーエンジニアリング | 不採用 |
| New Foundation Crate | `infra_foundation`等の新クレートを作成 | 明確な分離 | クレート数増加、メンテナンス負荷 | 不採用 |

## Design Decisions

### Decision: DayCountConvention統合戦略

- **Context**: 2箇所に異なるvariantsで重複定義されている
- **Alternatives Considered**:
  1. pricer_core版をそのまま移動（3 variants維持）
  2. infra_domain版を拡張（7 variants維持）
  3. 両方を統合した新定義を作成
- **Selected Approach**: Option 3 - 統合版を作成
- **Rationale**:
  - ISDA規格への準拠（7 variants）
  - pricer_core版の実装品質（serde, FromStr, テスト）
  - 両方の良い点を組み合わせ
- **Trade-offs**:
  - 既存コードの変更が必要（variant名の変更: `ActualActual365` → `Actual365Fixed`）
  - マイグレーション期間中の一時的な複雑さ
- **Follow-up**:
  - **旧variant名のエイリアスは追加しない**（常に正しい方向に修正）
  - 既存コードは新しいvariant名に直接更新する

### Decision: 再エクスポート戦略

- **Context**: 既存コードの後方互換性維持
- **Alternatives Considered**:
  1. 即座にすべてのインポートを変更
  2. deprecation警告付き再エクスポート
  3. 移行期間なしの破壊的変更
- **Selected Approach**: Option 2 - deprecation警告付き再エクスポート
- **Rationale**:
  - 段階的移行が可能
  - コンパイル時警告で開発者に通知
  - 将来のバージョンで完全削除可能
- **Trade-offs**:
  - 一時的なコード重複
  - 2つの有効なインポートパスが存在
- **Follow-up**:
  - CHANGELOG.mdに移行ガイド記載
  - 次のメジャーバージョンで旧パス削除

### Decision: Tenor型の設計

- **Context**: 金融期間表現の型安全な表現
- **Alternatives Considered**:
  1. Enum（固定variants: ON, 1W, 1M, 3M, 6M, 1Y, ...）
  2. Struct with numeric + unit（`Tenor { count: u32, unit: TenorUnit }`）
  3. Hybrid（一般的なtenorをenum、カスタムをstruct）
- **Selected Approach**: Option 1 - Enum
- **Rationale**:
  - 金融市場で使用されるtenorは限定的
  - 静的ディスパッチでEnzyme互換性維持
  - 型安全性が高い
- **Trade-offs**:
  - 非標準tenor（例: 18M）は表現不可
  - 将来の拡張で新variant追加が必要
- **Follow-up**:
  - `#[non_exhaustive]`で将来の拡張に対応
  - 必要に応じてカスタムtenor構造体を別途追加検討

### Decision: Tenor::add_to_date の月末処理ルール

- **Context**: 金融市場では月末日の加算処理に複数のルールが存在
- **Alternatives Considered**:
  1. 固定ルール（常に月末調整）
  2. `EndOfMonthRule` enumを引数として受け取る
  3. `BusinessDayConvention`を引数として受け取る
- **Selected Approach**: Option 2 - `EndOfMonthRule` enumを引数
- **Rationale**:
  - 月末処理と営業日調整は異なる概念（月末処理は日数計算、営業日調整はカレンダー参照）
  - 明示的なルール指定で計算の意図が明確
  - デフォルト値（`Adjust`）で一般的なケースを簡潔に記述可能
- **Trade-offs**:
  - API呼び出しが若干冗長になる
  - ルールの選択責任が呼び出し側に移る
- **Follow-up**:
  - `EndOfMonthRule::Adjust`をデフォルトとして`#[default]`を設定
  - 単体テストで各ルールの境界ケース（閏年2/29、月末31日等）を網羅

### Decision: TradeDirection の sign() メソッド配置

- **Context**: infra_domainの依存関係を軽量に保ちたい
- **Alternatives Considered**:
  1. infra_domainに`num_traits`依存を追加して`sign<T: Float>()`を実装
  2. infra_domainには基本enumのみ、`sign()`はpricer_modelsで拡張トレイトとして提供
  3. `sign()`を`f64`固定で返す（ジェネリック不要）
- **Selected Approach**: Option 2 - 拡張トレイトをpricer_modelsで提供
- **Rationale**:
  - infra_domainの依存関係を最小限に維持（`num_traits`不要）
  - 計算ロジックはPricerレイヤーの責務
  - Adapterは方向enumのみ必要、計算は不要
- **Trade-offs**:
  - `sign()`を使う場合は`TradeDirectionExt`トレイトのインポートが必要
  - 2段階のインポートが必要になる場合がある
- **Follow-up**:
  - pricer_modelsのpreludeに拡張トレイトを含める
  - ドキュメントに使用例を記載

## Risks & Mitigations

- **Risk 1**: 大規模な依存関係変更によるビルド破壊
  - **Mitigation**: Phase分割実行、各Phaseでのビルド検証
- **Risk 2**: 既存テストの大量失敗
  - **Mitigation**: 再エクスポートによる後方互換性維持
- **Risk 3**: A-I-P-Sルール違反の新たな発生
  - **Mitigation**: CI checkによる依存関係検証追加
- **Risk 4**: パフォーマンス回帰
  - **Mitigation**: 型移動は構造変更なし、ゼロコスト抽象

## References

- [Rust API Guidelines - Deprecation](https://rust-lang.github.io/api-guidelines/necessities.html#c-deprecated)
- [ISDA Day Count Conventions](https://www.isda.org/a/pIJEE/The-Actual-Actual-Day-Count-Fraction-1999.pdf)
- [ISO 4217 Currency Codes](https://www.iso.org/iso-4217-currency-codes.html)
