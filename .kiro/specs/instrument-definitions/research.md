# Research & Design Decisions

## Summary
- **Feature**: `instrument-definitions`
- **Discovery Scope**: Extension (pricer_models に instruments モジュールを実装)
- **Key Findings**:
  - pricer_core の smoothing 関数 (smooth_max, smooth_indicator) を活用してAD互換ペイオフを実現
  - enum dispatch アーキテクチャで Box<dyn Trait> を回避、Enzyme 最適化を確保
  - 既存の Float トレイト制約パターンを踏襲し、f64/Dual64 両対応

## Research Log

### Smoothing Infrastructure Analysis
- **Context**: Payoff 関数の微分可能性確保のため既存インフラを調査
- **Sources Consulted**: `pricer_core::math::smoothing` モジュール
- **Findings**:
  - `smooth_max(a, b, epsilon)`: LogSumExp による max 近似
  - `smooth_min(a, b, epsilon)`: smooth_max の双対
  - `smooth_indicator(x, epsilon)`: Sigmoid による Heaviside 近似
  - `smooth_abs(x, epsilon)`: Softplus による絶対値近似
  - epsilon パラメータで滑らかさを制御可能
- **Implications**: Call/Put ペイオフは smooth_max を使用、Digital は smooth_indicator を使用

### Existing Pattern Analysis
- **Context**: pricer_models の既存パターンを確認
- **Sources Consulted**: `pricer_models/src/lib.rs`, `pricer_core` 構造
- **Findings**:
  - 全ジェネリック型は `T: Float` 制約を使用
  - thiserror によるエラー型定義パターン
  - Clone, Debug derive が標準
  - Copy は小型構造体のみ
- **Implications**: 同一パターンを instruments モジュールに適用

### Enum Dispatch vs Trait Object
- **Context**: Enzyme AD 互換性のためのディスパッチ方式選定
- **Sources Consulted**: steering/tech.md, Enzyme ドキュメント
- **Findings**:
  - Box<dyn Trait> は動的ディスパッチで Enzyme 最適化を阻害
  - enum dispatch は静的ディスパッチでコンパイル時に解決
  - match 式によるパターンマッチングは Enzyme フレンドリー
- **Implications**: Instrument enum を使用、各バリアントで具体型を保持

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Enum Dispatch | 各商品タイプを enum variant で表現 | 静的ディスパッチ、Enzyme 最適化、型安全 | 新商品追加時に enum 修正必要 | 選択：Enzyme 互換性優先 |
| Trait Object | Box<dyn Instrument> による多態 | 拡張性高い、新型追加容易 | 動的ディスパッチ、Enzyme 非互換 | 却下 |
| Sealed Trait | trait + enum 組み合わせ | 型安全 + 拡張性 | 複雑度増加 | 過剰設計 |

## Design Decisions

### Decision: Enum-based Instrument Architecture
- **Context**: Enzyme AD 互換性を維持しつつ複数商品タイプを統一的に扱う
- **Alternatives Considered**:
  1. Trait Object (Box<dyn Instrument>) — 動的ディスパッチ
  2. Enum Dispatch — 静的ディスパッチ
  3. Generics with trait bounds — 型パラメータ爆発
- **Selected Approach**: Enum Dispatch
- **Rationale**:
  - steering/tech.md で明示的に推奨されている
  - Enzyme はコンパイル時に具体型が確定している方が効率的
  - match 式による分岐は微分可能
- **Trade-offs**:
  - (+) 高パフォーマンス、Enzyme 互換、型安全
  - (-) 新商品追加時に enum 修正が必要
- **Follow-up**: 将来的に商品数が増加した場合、マクロによる自動生成を検討

### Decision: Configurable Smoothing Epsilon
- **Context**: ペイオフの滑らかさとプライシング精度のトレードオフ
- **Alternatives Considered**:
  1. グローバル定数 — 単純だが柔軟性なし
  2. 商品ごとに設定 — 柔軟、わずかにメモリ増
  3. 関数呼び出し時に渡す — 毎回指定が煩雑
- **Selected Approach**: 商品ごとに設定
- **Rationale**:
  - デジタルオプションとバニラでは最適な epsilon が異なる
  - 構築時に一度設定すれば以降は自動適用
- **Trade-offs**:
  - (+) 商品特性に応じた最適化可能
  - (-) 構造体サイズがわずかに増加
- **Follow-up**: デフォルト値 (1e-6) を提供、ビルダーパターンで上書き可能に

### Decision: PayoffType as Separate Enum
- **Context**: Call/Put/Digital を表現する方法
- **Alternatives Considered**:
  1. VanillaOption 内にフラグとして埋め込み
  2. 独立した PayoffType enum
  3. 各ペイオフを別構造体として定義
- **Selected Approach**: 独立した PayoffType enum
- **Rationale**:
  - ペイオフロジックの再利用性が高い
  - 将来的に Straddle, Strangle 等の組み合わせに対応可能
  - テスト容易性が向上
- **Trade-offs**:
  - (+) モジュラー設計、テスト容易
  - (-) 若干の間接参照オーバーヘッド（無視可能）
- **Follow-up**: evaluate メソッドで epsilon を受け取り smooth_max/smooth_indicator を使用

## Risks & Mitigations
- **Risk 1**: 新商品タイプ追加時の enum 修正
  - **Mitigation**: マクロによる自動生成、または将来的に sealed trait パターン検討
- **Risk 2**: epsilon 値の不適切な設定による数値不安定
  - **Mitigation**: デフォルト値提供、バリデーション（epsilon > 0）、ドキュメント
- **Risk 3**: AD テープ一貫性の破綻
  - **Mitigation**: 分岐操作を smooth 関数で置き換え、プロパティテストで検証

## References
- [pricer_core::math::smoothing](crates/pricer_core/src/math/smoothing.rs) — Smooth 関数実装
- [steering/tech.md](.kiro/steering/tech.md) — Static dispatch via enum 推奨
- [steering/structure.md](.kiro/steering/structure.md) — L2 pricer_models 構造
