# Gap Analysis: rate-index-pricing-integration

## 1. 現状調査

### 1.1 既存コンポーネント

| コンポーネント | ファイル | 現状 |
|---------------|---------|------|
| **RateIndex** | `infra_domain/src/market/rate_index.rs` | ✅ 基本実装あり（SOFR, TONAR, EURIBOR3M/6M, SONIA, SARON）。currency(), tenor(), day_counter(), name(), code() メソッド提供 |
| **IndexType** | `infra_domain/src/trade/index.rs` | ✅ Rate, SwapRate, Fx, Equity, Inflation, Commodity バリアント |
| **IndexObservation** | `infra_domain/src/trade/index.rs` | ⚠️ 基本構造のみ（observation_lag, fixing_source）。compounding_method, reset_frequency 欠落 |
| **Payoff** | `infra_domain/src/trade/payoff.rs` | ✅ Fixed, Linear, VanillaOption, Digital。required_index() メソッドあり |
| **CurveName** | `pricer_models/src/market/curves/curve_enum.rs` | ✅ Ois, Sofr, Tonar, Euribor, Forward, Discount, Custom |
| **CurveSet** | `pricer_models/src/market/curves/curve_set.rs` | ⚠️ CurveName でのみ検索可能。RateIndex → CurveName マッピングなし |
| **GenericPricer** | `pricer_pricing/src/generic_pricer/pricer.rs` | ❌ cf.payoff を完全に無視。cf_amount = year_fraction × notional のみ |
| **Demo DTO** | `demo/gui/src/web/trade_types.rs` | ❌ rate_index フィールドなし（SwapParams, LegDto, CashflowDto） |

### 1.2 コード上の具体的ギャップ

#### GenericPricer::price_leg (pricer.rs:178)
```rust
// 現状: Payoff を無視
let cf_amount = cf.year_fraction * self.get_notional_for_cashflow(cf, leg);

// 必要: Payoff に基づく計算
match &cf.payoff {
    Payoff::Fixed { rate } => notional * rate * year_fraction,
    Payoff::Linear { index, spread, multiplier } => {
        let fwd = curve_set.forward_rate_for_index(index, start, end)?;
        notional * (fwd + spread) * multiplier * year_fraction
    }
    // ...
}
```

#### CurveSet (curve_set.rs)
```rust
// 現状: CurveName でのみ検索
pub fn get(&self, name: &CurveName) -> Option<&CurveEnum<T>>

// 必要: RateIndex での検索
pub fn get_curve_for_index(&self, index: RateIndex) -> Result<&CurveEnum<T>, MarketDataError>
```

#### RateIndex (rate_index.rs)
```rust
// 現状: 基本メタデータのみ
pub const fn currency(&self) -> Currency
pub const fn tenor(&self) -> Tenor
pub const fn day_counter(&self) -> DayCounter

// 必要: フィクシングメタデータ
pub fn fixing_calendar(&self) -> CalendarId
pub fn publication_lag(&self) -> i32
pub fn fixing_offset(&self) -> i32
pub fn compounding_method(&self) -> CompoundingMethod
```

## 2. 要件実現可能性分析

### 要件 → 既存アセットマッピング

| 要件 | 既存アセット | ギャップ |
|------|-------------|---------|
| Req 1: RateIndex メタデータ | RateIndex enum | **Missing**: fixing_calendar, publication_lag, fixing_offset, compounding_method |
| Req 2: IndexObservation 強化 | IndexObservation struct | **Missing**: reset_frequency, compounding_method, lookback_period, lockout_period |
| Req 3: カーブマッピング | CurveSet, CurveName | **Missing**: IndexCurveMapper trait, get_curve_for_index() |
| Req 4: フォワードレート計算 | YieldCurve::forward_rate() | **Missing**: インデックス固有の day_counter 適用、OIS 複利 |
| Req 5: Payoff 評価 | Payoff enum, GenericPricer | **Missing**: 全 Payoff バリアントの評価ロジック |
| Req 6: OIS コンパウンディング | DailyAccrual struct | **Missing**: 複利計算ロジック |
| Req 7: Cap/Floor 評価 | Payoff::VanillaOption | **Missing**: Black/Bachelier 評価、Vol 取得 |
| Req 8: 入力 DTO | SwapParams, RatesParams | **Missing**: rate_index フィールド |
| Req 9: 出力 DTO | LegDto, CashflowDto | **Missing**: rate_index フィールド |
| Req 10: 後方互換性 | 既存テスト | **Constraint**: API シグネチャ維持必須 |

### 複雑性シグナル

- **アルゴリズムロジック**: OIS 複利計算、Black/Bachelier オプション評価
- **データモデル変更**: RateIndex, IndexObservation への新規フィールド追加
- **統合ポイント**: infra_domain → pricer_models → pricer_pricing → demo/gui の4層

## 3. 実装アプローチオプション

### Option A: 既存コンポーネント拡張

**拡張対象ファイル:**
- `infra_domain/src/market/rate_index.rs` - IndexMetadata 追加
- `infra_domain/src/trade/index.rs` - IndexObservation 強化
- `pricer_models/src/market/curves/curve_set.rs` - get_curve_for_index() 追加
- `pricer_pricing/src/generic_pricer/pricer.rs` - price_leg() 内で Payoff 評価
- `demo/gui/src/web/trade_types.rs` - rate_index フィールド追加

**トレードオフ:**
- ✅ 最小限の新規ファイル、既存パターン活用
- ✅ 後方互換性維持が容易
- ❌ GenericPricer が肥大化するリスク
- ❌ 単一責任原則の侵害可能性

### Option B: 新規コンポーネント作成

**新規作成:**
- `pricer_models/src/market/index_curve_mapper.rs` - RateIndex → CurveName マッピング
- `pricer_pricing/src/generic_pricer/payoff_evaluator.rs` - Payoff 評価ロジック分離
- `pricer_pricing/src/generic_pricer/ois_calculator.rs` - OIS 複利計算

**トレードオフ:**
- ✅ クリーンな責任分離
- ✅ テスト容易性向上
- ❌ ファイル数増加
- ❌ インターフェース設計が必要

### Option C: ハイブリッドアプローチ（推奨）

**Phase 1: 既存拡張**
- RateIndex, IndexObservation へのメタデータ追加
- CurveSet への get_curve_for_index() 追加
- Demo DTO への rate_index フィールド追加

**Phase 2: 新規コンポーネント**
- PayoffEvaluator トレイト + 実装（pricer_pricing 内）
- IndexCurveMapper（pricer_models 内）

**Phase 3: GenericPricer 統合**
- price_leg() で PayoffEvaluator を使用
- OIS 複利計算の組み込み

**トレードオフ:**
- ✅ 段階的実装でリスク分散
- ✅ 既存機能への影響を最小化
- ✅ 各フェーズで検証可能
- ❌ 計画の複雑性増加

## 4. 実装複雑性 & リスク

### 工数見積もり: **L (1-2週間)**

**根拠:**
- 4層（infra_domain → pricer_models → pricer_pricing → demo）にまたがる変更
- OIS 複利計算、Cap/Floor 評価の数学的ロジック
- AD（自動微分）互換性の維持
- 広範なテストカバレッジ

### リスク: **Medium**

**リスク要因:**
- Float トレイト境界での Dual64 互換性（AD）
- 既存テストの回帰リスク
- GenericPricer の l1l2-integration feature flag 対応

**軽減策:**
- 既存 API シグネチャを維持（新規メソッド追加のみ）
- 各フェーズでユニットテスト追加
- feature flag による段階的有効化

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ: **Option C（ハイブリッド）**

### 設計時に調査が必要な項目

1. **Research Needed**: CalendarId の定義場所と形式（infra_domain 内の既存カレンダー実装確認）
2. **Research Needed**: Black/Bachelier 評価の既存実装確認（pricer_core に存在するか）
3. **Research Needed**: MarketProvider の Vol サーフェス取得 API の有無

### 重要な設計決定

1. **IndexMetadata の構造**: enum に直接メソッド追加 vs 別構造体
2. **PayoffEvaluator のトレイト設計**: ジェネリック T: Float での AD 互換性
3. **CurveSet の RateIndex マッピング**: HashMap 追加 vs メソッド内マッチ
