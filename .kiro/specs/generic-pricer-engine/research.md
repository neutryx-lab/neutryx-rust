# Research & Design Decisions: generic-pricer-engine

## Summary

- **Feature**: `generic-pricer-engine`
- **Discovery Scope**: Complex New Feature
- **Key Findings**:
  - 既存の`PricingContext`パターン（3-stage rocket）を拡張して汎用プライサーを構築可能
  - `Trade → Leg → Cashflow`階層が既に完全実装済み、`PricingResult`はこの構造をミラーリングすべき
  - `MarketProvider`のArc-cachedパターンがバッチプライシングの基盤として利用可能

---

## Research Log

### Topic 1: 既存の3-Stage Rocketパターン分析
- **Findings**:
  - Stage 1: 定義（L2 pricer_models）- ModelEnum, InstrumentEnum
  - Stage 2: リンキング（PricingContext）- Arc参照をバインド
  - Stage 3: 実行（pure kernel）- HashMap lookupなしの純粋計算
  - 現行`PricingContext`は`discount_curve`と`adjustment_vol`のみ保持

- **Context**: Generic Pricer Engineが既存のアーキテクチャパターンとどう統合すべきかを調査
- **Sources Consulted**:
  - [context.rs](crates/pricer_pricing/src/context.rs) - 現行の`PricingContext`実装
  - [steering/structure.md](.kiro/steering/structure.md) - A-I-P-Sアーキテクチャ
- **Findings**:
  - Stage 1: 定義（L2 pricer_models）- ModelEnum, InstrumentEnum
  - Stage 2: リンキング（PricingContext）- Arc参照をバインド
  - Stage 3: 実行（pure kernel）- HashMap lookupなしの純粋計算
  - 現行`PricingContext`は`discount_curve`と`adjustment_vol`のみ保持
- **Implications**:
  - 新しい`GenericPricerContext`は`PricingContext`を拡張し、`ModelConfig`、`PricerConfig`を追加
  - 3-stage rocketパターンを維持しつつ、より多くのマーケットデータをサポート

### Topic 2: Trade/Leg/Cashflow階層構造
- **Findings**:
  - `Trade` → `Vec<Leg>` → `Vec<Cashflow>` の階層
  - 各`Leg`は`Direction`（Payer/Receiver）、`LegType`、`Currency`を保持
  - 各`Cashflow`は`payment_date`、`notional`、`payoff`、`currency`を保持
  - `Direction::sign()`でPV符号を決定（Payer: -1.0, Receiver: +1.0）

- **Context**: `PricingResult`の階層設計のために既存のトレード構造を分析
- **Sources Consulted**:
  - [trade.rs](crates/infra_domain/src/trade/trade.rs) - Trade構造
  - [leg.rs](crates/infra_domain/src/trade/leg.rs) - Leg構造（Direction, LegType, Currency）
  - [cashflow.rs](crates/infra_domain/src/trade/cashflow.rs) - Cashflow構造
- **Findings**:
  - `Trade` → `Vec<Leg>` → `Vec<Cashflow>` の階層
  - 各`Leg`は`Direction`（Payer/Receiver）、`LegType`、`Currency`を保持
  - 各`Cashflow`は`payment_date`、`notional`、`payoff`、`currency`を保持
  - `Direction::sign()`でPV符号を決定（Payer: -1.0, Receiver: +1.0）
- **Implications**:
  - `PricingResult`は同じ階層構造（`TradePricingResult` → `LegPricingResult` → `CashflowPricingResult`）を持つべき
  - 各レベルで通貨情報を保持し、集計時に換算可能に

### Topic 3: マーケットデータ統合パターン
- **Findings**:
  - `MarketProvider`は`RwLock<HashMap<Currency, Arc<CurveEnum<f64>>>>`でカーブをキャッシュ
  - `CurveEnum`は`Flat`, `Interpolated`, `Credit`をサポート
  - `VolSurfaceEnum`は`Flat`, `Interpolated`, `FxSurface`をサポート
  - FxRateは現在未実装 → `MarketDataError::FxRateNotFound`の追加が必要

- **Context**: MarketProviderの既存パターンとGeneric Pricer Engineの統合方法
- **Sources Consulted**:
  - [provider.rs](crates/pricer_models/src/market/provider.rs) - MarketProvider実装
  - [curves/curve_enum.rs](crates/pricer_models/src/market/curves/curve_enum.rs) - CurveEnum
  - [surfaces/vol_surface_enum.rs](crates/pricer_models/src/market/surfaces/vol_surface_enum.rs) - VolSurfaceEnum
- **Findings**:
  - `MarketProvider`は`RwLock<HashMap<Currency, Arc<CurveEnum<f64>>>>`でカーブをキャッシュ
  - `CurveEnum`は`Flat`, `Interpolated`, `Credit`をサポート
  - `VolSurfaceEnum`は`Flat`, `Interpolated`, `FxSurface`をサポート
  - FxRateは現在未実装 → `MarketDataError::FxRateNotFound`の追加が必要
- **Implications**:
  - `GenericPricerContext`は`MarketProvider`への参照を保持
  - 為替レートは`MarketProvider`に追加するか、別の`FxRateProvider`として分離

### Topic 4: 日付・時間処理
- **Findings**:
  - `DayCounter`列挙型でISDA標準のday count conventionを完全サポート
  - `DayCounter::year_fraction(start, end)`で年分数を計算
  - `infra_domain::Date`は`chrono::NaiveDate`のラッパー
  - Calendarは休日判定、営業日調整（Following, ModifiedFollowing等）をサポート

- **Context**: 日付計算とカレンダー処理の既存実装
- **Sources Consulted**:
  - [day_counters.rs](crates/infra_domain/src/time/day_counters.rs) - DayCounter実装
  - [calendars.rs](crates/infra_domain/src/time/calendars.rs) - Calendar実装
- **Findings**:
  - `DayCounter`列挙型でISDA標準のday count conventionを完全サポート
  - `DayCounter::year_fraction(start, end)`で年分数を計算
  - `infra_domain::Date`は`chrono::NaiveDate`のラッパー
  - Calendarは休日判定、営業日調整（Following, ModifiedFollowing等）をサポート
- **Implications**:
  - Generic Pricer Engineは既存の`infra_domain::time`をそのまま利用可能
  - time_to_maturity計算ヘルパーの追加が有用

### Topic 5: Enzyme AD互換性
- **Findings**:
  - 静的ディスパッチ（enum）が必須、`Box<dyn Trait>`は非推奨
  - `GreeksMode`で`BumpRevalue`、`NumDual`、`EnzymeAAD`を選択可能
  - `GreeksConfig`はBuilderパターンで構築
  - スムース近似関数（`smooth_max`, `smooth_indicator`）を使用

- **Context**: Generic Pricer EngineがEnzyme ADと互換性を持つための設計制約
- **Sources Consulted**:
  - [steering/tech.md](.kiro/steering/tech.md) - Enzyme要件
  - [greeks/config.rs](crates/pricer_pricing/src/greeks/config.rs) - GreeksConfig
- **Findings**:
  - 静的ディスパッチ（enum）が必須、`Box<dyn Trait>`は非推奨
  - `GreeksMode`で`BumpRevalue`、`NumDual`、`EnzymeAAD`を選択可能
  - `GreeksConfig`はBuilderパターンで構築
  - スムース近似関数（`smooth_max`, `smooth_indicator`）を使用
- **Implications**:
  - `PricingResult<T: Float>`はジェネリックで、AD互換
  - 全てのenum matchingは静的ディスパッチを維持
  - `ModelConfig`、`PricerConfig`も同様にBuilderパターンで構築

---

## Architecture Pattern Evaluation

| Option | [Rejected] | - | - |
|--------|-------------|-----------|---------------------|-------|
| A: PricingContext拡張 | 既存の`PricingContext`を直接拡張 | 一貫性、既存パターン活用 | 既存コードへの影響、互換性 | 推奨しない |
| B: GenericPricerEngine新規モジュール | `pricer_pricing/src/generic_pricer/`に独立モジュール作成 | 影響最小、独立開発可能 | 将来の統合作業 | **採用** |
| C: Traitベース抽象化 | `Priceable<T>`トレイトをTrade/InstrumentEnumに実装 | 拡張性、型安全 | Enzyme互換性への影響 | 補助的に使用 |

**選択: Option B + C の組み合わせ**

理由:
1. 新規モジュールとして`generic_pricer/`を作成し、既存コードへの影響を最小化
2. `Priceable<T>`トレイトを定義し、将来の拡張性を確保（ただしenum静的ディスパッチを基本とする）
3. 3-stage rocketパターンを踏襲し、`GenericPricerContext`を導入

---

## Design Decisions

### Decision 1: PricingResult階層構造

- **Context**: 要件6で「任意の粒度（Cashflow、Leg、Trade、Path）でPV内訳にアクセス」が要求
- **Alternatives Considered**:
  1. フラット構造（単一のPV値のみ）— シンプルだが内訳取得不可
  2. 階層構造（Trade → Leg → Cashflow）— 完全な内訳取得可能
  3. 遅延評価（必要時に計算）— メモリ効率良いが複雑
- **Selected Approach**: 階層構造 + パス分布オプション
  ```rust
  pub struct PricingResult<T: Float> {
      pub total_pv: T,
      pub legs: Vec<LegPricingResult<T>>,
      pub currency_breakdown: CurrencyBreakdown<T>,
      pub path_distribution: Option<PathDistribution<T>>, // MC計算時のみ
  }
  ```
- **Rationale**: Trade/Leg/Cashflow構造と1:1対応し、直感的なAPI設計が可能
- **Trade-offs**: メモリ使用量増加（全階層を保持）vs API使いやすさ
- **Follow-up**: バッチプライシング時のメモリ最適化を検討

### Decision 2: ModelConfigとPricerConfigの分離

- **Context**: 要件3と要件4で設定項目が異なる責務を持つ
- **Alternatives Considered**:
  1. 単一の`Config`構造体 — シンプルだが責務混在
  2. 分離した`ModelConfig`と`PricerConfig` — 責務明確
  3. Builder経由の統合 — 柔軟だが複雑
- **Selected Approach**: 分離した構造体、それぞれBuilderパターン
  ```rust
  // ModelConfig: シミュレーション設定
  pub struct ModelConfig {
      pub model: Option<StochasticModelEnum<f64>>,
      pub num_paths: usize,
      pub num_steps: usize,
      pub seed: Option<u64>,
  }

  // PricerConfig: Greeks・出力設定
  pub struct PricerConfig {
      pub greeks_config: GreeksConfig,
      pub default_currency: Currency,
      pub use_thread_local_buffers: bool,
  }
  ```
- **Rationale**: 責務分離によりAPIが明確、テストしやすい
- **Trade-offs**: 設定オブジェクトが2つになり、初期化が少し複雑
- **Follow-up**: デフォルト値を適切に設定し、簡単なユースケースではBuilderなしでも使用可能に

### Decision 3: 為替レート取得の統合

- **Context**: 要件6で通貨換算が必要、現在MarketProviderにFxRateがない
- **Alternatives Considered**:
  1. `MarketProvider`にFxRate機能を追加 — 統合的だが既存変更
  2. 別の`FxRateProvider`を作成 — 影響小だが重複可能性
  3. `GenericPricerContext`に直接FxRate機能を追加 — 独立だが責務拡大
- **Selected Approach**: `MarketProvider`に`FxRate`キャッシュを追加
  ```rust
  // MarketProviderへの追加
  fx_cache: RwLock<HashMap<CurrencyPair, Arc<FxRate<f64>>>>,

  pub fn get_fx_rate(&self, base: Currency, quote: Currency) -> Result<FxRate<f64>, MarketDataError>
  ```
- **Rationale**: マーケットデータ統合の一貫性を維持
- **Trade-offs**: `MarketProvider`の責務拡大
- **Follow-up**: `pricer_core::types::CurrencyPair`（現在`FxRate`にリネーム済み）を活用

---

## Risks & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Enzyme AD互換性 | High | Medium | 静的ディスパッチ維持、enum使用、feature-gatedテスト |
| パフォーマンス劣化 | High | Low | Arc-cached設計継承、ベンチマーク追加、プロファイリング |
| 既存APIとの非互換 | Medium | Low | 新規モジュールとして分離、段階的統合 |
| メモリ使用量増加 | Medium | Medium | 階層構造のオプション化、ストリーミング計算の検討 |

---

## References

- [3-Stage Rocket Pattern](crates/pricer_pricing/src/context.rs) — 既存のPricingContext実装
- [Trade Structure](crates/infra_domain/src/trade/trade.rs) — Trade/Leg/Cashflow階層
- [GreeksConfig](crates/pricer_pricing/src/greeks/config.rs) — 既存のGreeks設定パターン
- [MarketProvider](crates/pricer_models/src/market/provider.rs) — マーケットデータキャッシュパターン
- [DayCounter](crates/infra_domain/src/time/day_counters.rs) — 日付計算実装
