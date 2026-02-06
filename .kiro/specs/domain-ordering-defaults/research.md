# Research & Design Decisions

## Summary
- **Feature**: `domain-ordering-defaults`
- **Discovery Scope**: Simple Addition
- **Key Findings**:
  - `Frequency` enum のみ並び順変更が必要（Annual→Daily を Daily→Annual に）
  - `RateType`, `StochasticModelEnum`, `CurveName` は既に正しい業務順序
  - `BootstrapInterpolation` は現状で業界標準順序（LogLinear がデフォルト）
  - serde は既に name-based serialization を使用

## Research Log

### Frequency Enum 並び順調査
- **Context**: 要件1で高頻度→低頻度の順序を要求
- **Sources Consulted**: `crates/infra_domain/src/time/frequency.rs`
- **Findings**:
  - 現在: `Annual → SemiAnnual → Quarterly → Monthly → Weekly → Daily`（低頻度→高頻度）
  - 要求: `Daily → Weekly → Monthly → Quarterly → SemiAnnual → Annual`（高頻度→低頻度）
  - `Ord` trait 派生時に variant 宣言順が使用される
  - `pricer_models` にも重複定義あり（`bootstrapping/instrument.rs`）→ 統合または整合性確認が必要
- **Implications**: variant 順序変更のみ、ロジック変更不要

### RateType Enum 並び順調査
- **Context**: 要件2でアセットクラス別グループ化を要求
- **Sources Consulted**: `crates/infra_domain/src/market/rate_type.rs`
- **Findings**:
  - 現在: `Deposit → Fra → Futures → Swap → Ois → BasisSwap → FxSpot → FxForward → Vol`
  - 既にアセットクラス別にグループ化されている（金利商品 → FX → ボラティリティ）
- **Implications**: 変更不要、ドキュメント追加のみ

### StochasticModelEnum 並び順調査
- **Context**: 要件3でモデル複雑度順を要求
- **Sources Consulted**: `crates/pricer_models/src/models/model_enum.rs`
- **Findings**:
  - 現在: `GBM → Heston → SABR → HullWhite(feature) → CIR(feature)`
  - 既に複雑度順（GBM=基本 → Heston/SABR=中級 → HW/CIR=金利専門）
- **Implications**: 変更不要、ドキュメント追加のみ

### BootstrapInterpolation 並び順調査
- **Context**: 要件4で業界使用頻度順を要求
- **Sources Consulted**: `crates/pricer_models/src/market/calibration/bootstrapping/config.rs`
- **Findings**:
  - 現在: `LogLinear → LinearZeroRate → CubicSpline → MonotonicCubic → FlatForward`
  - `LogLinear` がデフォルト（業界標準）
  - `FlatForward` は `LogLinear` の次に一般的だが、現在は最後
- **Implications**: `FlatForward` を2番目に移動推奨

### CurveName 並び順調査
- **Context**: 要件5で論理的グループ化を要求
- **Sources Consulted**: `crates/pricer_models/src/market/curves/curve_enum.rs`
- **Findings**:
  - 現在: `Ois → Sofr → Tonar → Euribor → Forward → Discount → Custom`
  - オーバーナイトレート → インターバンク → 機能別 → カスタム
  - 要件通りの順序
- **Implications**: 変更不要、ドキュメント追加のみ

### 既存正順序 Enum 確認
- **Context**: 要件6で既存の正しい順序維持を確認
- **Findings**:
  - `Tenor`: `Overnight → OneWeek → ... → ThirtyYears`（期間順）✓
  - `AssetClass`: `Rates → Fx → Equity → Credit → Commodity`（銀行組織順）✓
  - `QuoteType`: 要確認
  - `DayCounter`: ファミリー別グループ化済み ✓
  - `BusinessDayConvention`: 論理順 ✓

### Serde Serialization 調査
- **Context**: 要件7で name-based serialization 確認
- **Findings**:
  - `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]` 使用
  - デフォルトで variant 名による文字列シリアライゼーション
  - ordinal 位置ベースではない
- **Implications**: 並び順変更はシリアライゼーション互換性に影響なし

## Design Decisions

### Decision: Frequency Enum 並び順変更
- **Context**: 高頻度→低頻度の業務自然順序への変更
- **Alternatives Considered**:
  1. variant 順序変更のみ
  2. `ordering_key()` メソッド追加による明示的順序付け
- **Selected Approach**: variant 順序変更のみ
- **Rationale**: Rust enum の `Ord` 派生は宣言順を使用。追加メソッド不要。
- **Trade-offs**: シンプルだが、将来の variant 追加時に順序維持が必要
- **Follow-up**: ドキュメントで新 variant 追加位置をガイド

### Decision: pricer_models::Frequency 重複解消
- **Context**: `infra_domain` と `pricer_models` に別々の `Frequency` 定義
- **Alternatives Considered**:
  1. `pricer_models` の定義を削除し `infra_domain` を re-export
  2. 両方を維持し順序を同期
- **Selected Approach**: オプション2（両方維持、順序同期）
- **Rationale**: `pricer_models::Frequency` は bootstrapping 専用で `Weekly` が欠落。用途が異なる。
- **Trade-offs**: 重複維持だが、異なるユースケースに対応

### Decision: BootstrapInterpolation FlatForward 位置
- **Context**: FlatForward は業界で2番目に一般的
- **Alternatives Considered**:
  1. FlatForward を2番目に移動
  2. 現状維持
- **Selected Approach**: FlatForward を2番目に移動
- **Rationale**: 業界使用頻度を反映し、ユーザー期待に沿う
- **Trade-offs**: 既存の `match` パターンへの影響なし（名前ベース）

## Risks & Mitigations
- **Risk 1**: Ord 派生に依存するコードの動作変更 → 影響範囲調査、テスト追加
- **Risk 2**: pricer_models Frequency 同期漏れ → 両ファイルを同時修正
- **Risk 3**: 将来の variant 追加時の順序崩れ → ドキュメントでガイド明記

## References
- [Rust Ord derive](https://doc.rust-lang.org/std/cmp/trait.Ord.html) - enum variant 順序は宣言順
- [Serde enum serialization](https://serde.rs/enum-representations.html) - デフォルトは externally tagged（名前ベース）
