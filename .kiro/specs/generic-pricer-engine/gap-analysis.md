# Gap Analysis: generic-pricer-engine

## 1. Analysis Summary

本分析では、Generic Pricer Engine要件を既存のNeutryxコードベースと照合し、実装ギャップと推奨アプローチを特定した。

### 主要な発見
- **既存の基盤が充実**: `MarketProvider`、`StochasticModelEnum`、`CurveEnum`、`VolSurfaceEnum`等の核心コンポーネントが実装済み
- **Trade/Leg/Cashflow構造**: `infra_master::trade`に完全なCF-expanded形式が存在
- **Greeks計算基盤**: `GreeksConfig`、`GreeksMode`（BumpRevalue、NumDual、EnzymeAAD）が整備済み
- **エラー型**: `PricingError`、`MarketDataError`が存在、一部拡張が必要
- **主要ギャップ**: 統合プライサーAPI、通貨換算、バッチプライシング機能

---

## 2. Requirement-Level Gap Analysis

### Requirement 1: コアプライシングインターフェース

| 受入基準 | 既存コンポーネント | ギャップ | 作業量 |
|---------|------------------|---------|--------|
| get_pv(valuation_date) | なし | **新規作成必要** | Medium |
| get_pv_with_currency(date, ccy) | なし | **新規作成必要** | Medium |
| get_greeks(date, config) | `GreeksConfig` あり | 統合API必要 | Small |
| PricingResult<T> 型 | `GreeksResult<T>` 参考 | **新規作成必要** | Small |
| PricingError::MissingMarketData | `MarketDataError::MissingData` 存在 | 変換追加のみ | Trivial |

**推奨アプローチ**:
```rust
// crates/pricer_pricing/src/generic_pricer/mod.rs
pub trait GenericPricer<T: Float> {
    fn get_pv(&self, valuation_date: NaiveDate) -> Result<PricingResult<T>, PricingError>;
    fn get_pv_with_currency(&self, date: NaiveDate, target: Currency) -> Result<PricingResult<T>, PricingError>;
    fn get_greeks(&self, date: NaiveDate, config: &GreeksConfig) -> Result<GreeksResult<T>, GreeksError>;
}
```

### Requirement 2: マーケットデータ統合

| 受入基準 | 既存コンポーネント | ギャップ | 作業量 |
|---------|------------------|---------|--------|
| MarketProvider受け取り | `pricer_models::market::MarketProvider` | **そのまま利用可能** | None |
| CurveEnumサポート | `CurveEnum<T>` (Flat, Interpolated) | **存在** | None |
| VolSurfaceEnumサポート | `VolSurfaceEnum<T>` (Flat, Interpolated, FxSurface) | **存在** | None |
| CurveSetから解決 | 部分実装 | CurveSet拡張必要 | Small |
| CurveNotFound/SurfaceNotFound | `MarketDataError::CurveNotFound` 存在 | SurfaceNotFound追加 | Trivial |

**既存パターン** ([provider.rs:55-60](crates/pricer_models/src/market/provider.rs#L55-L60)):
```rust
pub struct MarketProvider {
    curve_cache: RwLock<HashMap<Currency, Arc<CurveEnum<f64>>>>,
    vol_cache: RwLock<HashMap<Currency, Arc<VolSurfaceEnum<f64>>>>,
}
```

### Requirement 3: モデル構成

| 受入基準 | 既存コンポーネント | ギャップ | 作業量 |
|---------|------------------|---------|--------|
| StochasticModelEnumサポート | `StochasticModelEnum<T>` (GBM, Heston, SABR, HullWhite, CIR) | **完全実装済み** | None |
| デフォルトモデル選択 | なし | **新規作成必要** | Small |
| ModelConfig構造体 | `ModelParams<T>` 存在 | 名称変更検討 | Trivial |
| キャリブレーション連携 | `pricer_models::market::calibration` | 連携インターフェース必要 | Small |
| InvalidModelParameter | `CalibrationError::InvalidParameter` 存在 | ConfigError追加検討 | Small |

**既存パターン** ([model_enum.rs:227-251](crates/pricer_models/src/models/model_enum.rs#L227-L251)):
```rust
pub enum StochasticModelEnum<T: Float> {
    GBM(GBMModel<T>),
    Heston(HestonModel<T>),
    SABR(SABRModel<T>),
    #[cfg(feature = "rates")]
    HullWhite(HullWhiteModel<T>),
    #[cfg(feature = "rates")]
    CIR(CIRModel<T>),
}
```

### Requirement 4: プライサー設定

| 受入基準 | 既存コンポーネント | ギャップ | 作業量 |
|---------|------------------|---------|--------|
| PricerConfig構造体 | `GreeksConfig` 部分 | **拡張必要** | Medium |
| AADモード設定 | `GreeksMode::EnzymeAAD` 存在 | 統合のみ | Small |
| BumpAndRevalueモード | `GreeksMode::BumpRevalue` 存在 | 統合のみ | Small |
| Builderパターン | `GreeksConfigBuilder` 参考 | **新規作成必要** | Small |
| スレッドローカルバッファ | なし | **新規作成必要** | Medium |

**新規作成推奨**:
```rust
pub struct PricerConfig {
    pub num_paths: usize,
    pub num_steps: usize,
    pub seed: Option<u64>,
    pub greeks_config: GreeksConfig,
    pub default_currency: Currency,
    pub use_thread_local_buffers: bool,
}
```

### Requirement 5: 商品インターフェース

| 受入基準 | 既存コンポーネント | ギャップ | 作業量 |
|---------|------------------|---------|--------|
| Trade入力 | `infra_master::trade::Trade` | **完全実装済み** | None |
| InstrumentEnum入力 | `pricer_models::instruments` 参照のみ | **InstrumentEnum未実装** | Medium |
| Leg/Cashflowパース | `Trade::all_cashflows()` 存在 | プライシングロジック必要 | Medium |
| 静的ディスパッチ | enum使用パターン確立済み | 適用のみ | Small |
| UnsupportedInstrument | `PricingError::UnsupportedInstrument` 存在 | **利用可能** | None |

**既存構造** ([trade.rs:133-147](crates/infra_master/src/trade/trade.rs#L133-L147)):
```rust
pub struct Trade {
    pub id: TradeId,
    legs: Vec<Leg>,
    pub trade_type: TradeType,
    pub metadata: TradeMetadata,
}
```

### Requirement 6: 通貨・為替処理

| 受入基準 | 既存コンポーネント | ギャップ | 作業量 |
|---------|------------------|---------|--------|
| Currency列挙型 | `infra_master::market::Currency` | **完全実装済み** | None |
| 為替レート取得 | なし | **FxRateProvider新規作成必要** | Medium |
| 通貨別PV内訳 | なし | **CurrencyBreakdown新規作成必要** | Small |
| FxRateNotFound | `MarketDataError` 拡張必要 | **追加必要** | Trivial |
| デフォルト通貨設定 | なし | PricerConfig追加 | Trivial |

**拡張推奨**:
```rust
// MarketDataError への追加
FxRateNotFound { base: Currency, quote: Currency },

// 新規構造体
pub struct CurrencyBreakdown<T: Float> {
    pub pv_by_currency: HashMap<Currency, T>,
    pub total_pv: T,
    pub reporting_currency: Currency,
}
```

### Requirement 7: 日付・時間処理

| 受入基準 | 既存コンポーネント | ギャップ | 作業量 |
|---------|------------------|---------|--------|
| Calendar | `infra_master::time` 参照必要 | 確認必要 | 調査 |
| DayCountConvention | `infra_master::time` 参照必要 | 確認必要 | 調査 |
| 営業日調整 | 確認必要 | 確認必要 | 調査 |
| カーブテナー補間 | `InterpolatedCurve` 存在 | 日付→Year Fractionヘルパー必要 | Small |
| chrono::NaiveDate | `infra_master::Date` 使用中 | **互換性確認必要** | 調査 |

### Requirement 8: バッチプライシング

| 受入基準 | 既存コンポーネント | ギャップ | 作業量 |
|---------|------------------|---------|--------|
| price_batch() | なし | **新規作成必要** | Medium |
| rayon並列処理 | Cargo.tomlで使用可能 | 統合必要 | Small |
| Arc-cachedマーケット | `MarketProvider` Arc使用 | **そのまま利用可能** | None |
| BatchPricingResult | なし | **新規作成必要** | Small |
| 部分エラー継続 | なし | ロジック実装必要 | Small |

---

## 3. Implementation Approach Options

### Option A: PricingContext拡張 (推奨)

既存の`PricingContext`（[context.rs](crates/pricer_pricing/src/context.rs)）を拡張し、3-stage rocketパターンを維持。

**メリット**:
- 既存パターンとの整合性
- Enzyme最適化との親和性
- コードベース全体の一貫性

**デメリット**:
- 既存コードへの影響
- 段階的移行が必要

### Option B: GenericPricerEngine新規作成

`pricer_pricing/src/generic_pricer/`に独立モジュールとして実装。

**メリット**:
- 既存コードへの影響最小
- 独立したテスト・開発が可能
- 将来のリファクタリング容易

**デメリット**:
- 重複コードの可能性
- 統合作業が後で必要

### Option C: Traitベース抽象化

`Priceable<T>`トレイトを定義し、Trade/InstrumentEnumに実装。

**メリット**:
- 拡張性が高い
- Rustの型システムを活用

**デメリット**:
- Enzyme LLVM互換性への影響
- 静的ディスパッチパターンとの整合性

---

## 4. Risk Assessment

| リスク | 影響度 | 発生確率 | 軽減策 |
|-------|-------|---------|--------|
| Enzyme AD互換性 | 高 | 中 | 静的ディスパッチ維持、enum使用 |
| 既存テスト破損 | 中 | 低 | 新規モジュールでの開発 |
| パフォーマンス劣化 | 高 | 低 | Arc-cached設計継承、プロファイリング |
| API互換性 | 中 | 中 | 段階的公開、feature-gated |

---

## 5. Research Needs

以下の項目は設計フェーズで追加調査が必要:

1. **infra_master::time モジュール**: Calendar、DayCountConventionの現状確認
2. **InstrumentEnum設計**: 既存の商品定義との統合方針
3. **為替レート取得**: MarketProviderへのFxRate追加 vs 別Provider
4. **スレッドローカルバッファ**: rayon並列化との整合性

---

## 6. Recommended Implementation Order

1. **Phase 1 (Core)**: PricerConfig + PricingResult構造体
2. **Phase 2 (Single Pricing)**: GenericPricer trait + Trade実装
3. **Phase 3 (Currency)**: CurrencyBreakdown + FxRate統合
4. **Phase 4 (Batch)**: BatchPricingResult + rayon並列化
5. **Phase 5 (Integration)**: InstrumentEnum + Greeks統合

---

## 7. Conclusion

Generic Pricer Engineの実装は、既存の充実したマーケットデータ・モデル基盤を活用することで効率的に進められる。主要なギャップは統合API層とバッチ処理機能であり、**Option B（新規モジュール）**を推奨する。これにより既存コードへの影響を最小化しながら、段階的に機能を追加できる。
