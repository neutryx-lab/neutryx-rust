# Gap Analysis: curve-bootstrap-engine

## 1. 分析サマリー

### スコープ
本仕様は、Index単位のカーブ定義、Bootstrap エンジン、汎用カーブインターフェース、AD対応計算グラフ、結果キャッシュを含む包括的なカーブ構築システムを要求している。

### 既存コードベースの成熟度
**High（70-80%）** - `pricer_models/src/market/calibration/bootstrapping/`モジュールに強力な基盤が存在する。

### 主要なギャップ
1. **Index-Curve Definition**: Index→Instrument集合のマッピングが未実装
2. **infra_master統合**: `BootstrapInstrument`が`infra_master::trade`と非統合
3. **結果キャッシュ**: LRUベースの構築済みカーブキャッシュが未実装
4. **設定シリアライゼーション**: serde対応が部分的

### 推奨アプローチ
**ハイブリッド（拡張 + 新規）** - 既存Bootstrapモジュールを活用しつつ、新規のIndex-Curve定義層とキャッシュ層を追加する。

---

## 2. 要件別ギャップ詳細

### Requirement 1: Index-Curve Definition（Index別カーブ定義）

| 項目 | 状態 |
|------|------|
| ギャップレベル | **HIGH** - 新規実装が必要 |

**既存コード:**
- `infra_master::trade::index.rs` に `IndexType`, `RateIndex` が定義済み
- `RateIndex`: SOFR, Euribor3M, Euribor6M, Tonar, Sonia, Tibor3M, Tibor6M, Estr

**ギャップ:**
- Index → 必要Instrument集合のマッピングが存在しない
- テナーポイント定義（1M, 3M, 6M, 1Y, 2Y, ..., 50Y）の仕組みがない
- コンベンション参照（DayCount, BDC, PaymentFrequency）との紐付けがない

**統合ポイント:**
- `SwapConvention::usd_sofr()`, `SwapConvention::eur_euribor_6m()` 等の既存コンベンション
- `infra_master::trade::convention` モジュール

**設計考慮事項:**
```rust
// 新規作成が必要
pub struct CurveDefinition {
    pub index: RateIndex,
    pub instruments: Vec<InstrumentSpec>,
    pub convention_ref: SwapConvention,
}

pub struct InstrumentSpec {
    pub instrument_type: CurveInstrumentType, // OIS, IRS, FRA, Future
    pub tenor: Tenor,
}
```

---

### Requirement 2: Curve Parameter Configuration（カーブパラメータ設定）

| 項目 | 状態 |
|------|------|
| ギャップレベル | **MEDIUM** - 拡張が必要 |

**既存コード（[config.rs:9-54](crates/pricer_models/src/market/calibration/bootstrapping/config.rs#L9-L54)）:**
```rust
pub enum BootstrapInterpolation {
    LogLinear,      // 存在
    LinearZeroRate, // 存在
    CubicSpline,    // 存在
    MonotonicCubic, // 存在
    FlatForward,    // 存在
}
```

**ギャップ:**
- **パラメータ表現種別**（`LogDiscountFactor`, `ZeroRate`, `InstantaneousForward`）が未定義
- 内部的にはLogDFを使用しているが、設定から選択する仕組みがない
- 外挿設定の詳細オプション（外挿方法の選択）が限定的

**統合ポイント:**
- `GenericBootstrapConfig<T>` に新フィールド追加
- `BootstrappedCurve<T>` の内部表現を抽象化

---

### Requirement 3: Instrument-to-Cashflow Integration（Instrument-キャッシュフロー統合）

| 項目 | 状態 |
|------|------|
| ギャップレベル | **HIGH** - ブリッジ層の新規実装が必要 |

**既存コード（分離状態）:**

**pricer_models側（[instrument.rs](crates/pricer_models/src/market/calibration/bootstrapping/instrument.rs)）:**
```rust
pub enum BootstrapInstrument<T: Float> {
    Ois { maturity: T, rate: T, payment_frequency: u32 },
    Irs { maturity: T, rate: T, fixed_frequency: u32, float_frequency: u32 },
    Fra { start: T, end: T, rate: T },
    Future { maturity: T, price: T, convexity_adjustment: T },
}
```

**infra_master側（[swap.rs](crates/infra_master/src/trade/convention/swap.rs)）:**
```rust
pub struct SwapConvention {
    pub fixed_leg: SwapLegConvention,
    pub float_leg: SwapLegConvention,
    pub float_index: RateIndex,
    pub spot_lag: u32,
}
```

**ギャップ:**
- `BootstrapInstrument`は`infra_master`のコンベンションを使用していない
- キャッシュフロー展開（`Cashflow`型）との連携がない
- Futures のConvexity調整がハードコード

**必要な作業:**
- `infra_master::trade::instrument_def` → `BootstrapInstrument` 変換器
- `SwapConvention`からキャッシュフロースケジュール生成
- `InstrumentExpander`との統合

---

### Requirement 4: Bootstrap Engine（ブートストラップエンジン）

| 項目 | 状態 |
|------|------|
| ギャップレベル | **LOW** - 既存実装で十分 |

**既存コード（[engine.rs](crates/pricer_models/src/market/calibration/bootstrapping/engine.rs)）:**
- `SequentialBootstrapper<T>` - Newton-Raphson + Brent fallback
- `bootstrap()` メソッド - 満期順ソート、逐次求解
- `BootstrapResult<T>` - 結果構造体

**完全に実装済み:**
- ✅ Newton-Raphson法による求解
- ✅ Brent法へのフォールバック
- ✅ 収束許容誤差・最大反復回数の設定
- ✅ 残差・収束ステータスの返却

**軽微な改善点:**
- 残差ベクトルの直接アクセス（現在は`BootstrapResult`に含まれる）

---

### Requirement 5: Generic Curve Interface（汎用カーブインターフェース）

| 項目 | 状態 |
|------|------|
| ギャップレベル | **LOW** - 軽微な拡張のみ |

**既存コード（[traits.rs:44-65](crates/pricer_models/src/market/curves/traits.rs#L44-L65)）:**
```rust
pub trait YieldCurve<T: Float> {
    fn discount_factor(&self, t: T) -> Result<T, MarketDataError>;
    fn zero_rate(&self, t: T) -> Result<T, MarketDataError> { ... } // デフォルト実装あり
    fn forward_rate(&self, t1: T, t2: T) -> Result<T, MarketDataError> { ... }
}
```

**ギャップ:**
- `instantaneous_forward(t)` メソッドが未定義
- pillar点アクセサがトレイトレベルで未定義（`BootstrappedCurve`には実装済み）

**必要な作業:**
- `YieldCurve`トレイトに`instantaneous_forward()`追加
- `pillar_count()`, `pillars()` メソッドをトレイトに昇格（オプション）

---

### Requirement 6: Computation Graph for AD（自動微分用計算グラフ）

| 項目 | 状態 |
|------|------|
| ギャップレベル | **MEDIUM** - 検証・拡張が必要 |

**既存コード（[sensitivity.rs](crates/pricer_models/src/market/calibration/bootstrapping/sensitivity.rs)）:**
- `SensitivityBootstrapper` - Implicit Function Theoremによる感度計算
- `BootstrapResultWithSensitivities` - Jacobian行列保持
- `verify_sensitivities()` - bump-and-revalueとの検証

**現在の制約:**
- `num-dual-mode` featureでのみAAD計算有効
- `f64`専用の`SensitivityBootstrapper`（ジェネリック非対応）

**ギャップ:**
- `pricer_core::types::Dual`との互換性検証が必要
- ジェネリックな`SensitivityBootstrapper<T>`の実装検討
- 計算グラフの明示的な保持（現在はJacobianのみ）

**設計選択肢:**
1. 現在のImplicit Function Theorem方式を維持（推奨）
2. Forward-mode ADでの直接テープ記録

---

### Requirement 7: Curve Caching（カーブキャッシュ）

| 項目 | 状態 |
|------|------|
| ギャップレベル | **HIGH** - 新規実装が必要 |

**既存コード（[cache.rs](crates/pricer_models/src/market/calibration/bootstrapping/cache.rs)）:**
```rust
// 内部メモリ最適化用（結果キャッシュではない）
pub struct CurveCache<T: Float> { ... }  // pillar/DF一時保存
pub struct BufferPool<T> { ... }         // バッファ再利用
pub struct BootstrapCache<T: Float> { ... } // bootstrap中の中間結果
```

**ギャップ:**
- **結果キャッシュが存在しない**: 同一条件でのカーブ再計算を省略する機構がない
- `BufferPool`は`RefCell`使用でスレッドセーフではない
- LRUエビクション機構がない
- キャッシュキー（Index + rates + config hash）の設計がない

**必要な新規コンポーネント:**
```rust
pub struct CurveResultCache<T: Float> {
    cache: Arc<RwLock<LruCache<CurveKey, BootstrappedCurve<T>>>>,
    max_entries: usize,
    stats: CacheStats,
}

#[derive(Hash, Eq, PartialEq)]
pub struct CurveKey {
    index: RateIndex,
    rates_hash: u64,
    config_hash: u64,
}
```

---

### Requirement 8: Multi-Curve Support（マルチカーブ対応）

| 項目 | 状態 |
|------|------|
| ギャップレベル | **LOW** - 既存実装で十分 |

**既存コード（[multi_curve.rs](crates/pricer_models/src/market/calibration/bootstrapping/multi_curve.rs)）:**
- ✅ `MultiCurveBuilder<T>` - OIS Discount + Tenor Curve構築
- ✅ `CurveSet<T>` - 複数カーブの集合管理
- ✅ `Tenor` enum - ON, 1M, 3M, 6M, 12M
- ✅ `build_parallel()` - Rayon並列構築

**軽微なギャップ:**
- 依存関係の自動解決（現在は手動で順序指定）
- 循環依存検出の明示的エラー

---

### Requirement 9: Error Handling（エラーハンドリング）

| 項目 | 状態 |
|------|------|
| ギャップレベル | **NONE** - 完全実装済み |

**既存コード（[error.rs](crates/pricer_models/src/market/calibration/bootstrapping/error.rs)）:**
```rust
#[derive(Error, Debug, Clone, PartialEq)]
pub enum BootstrapError {
    ConvergenceFailure { maturity, residual, iterations },
    DuplicateMaturity { maturity },
    InsufficientData { required, provided },
    NegativeRate { maturity, rate },
    ArbitrageDetected { maturity },
    Solver(#[from] SolverError),
    MarketData(#[from] MarketDataError),
    InvalidInput(String),
    InvalidMaturity { maturity, max_maturity },
}
```

**完全に要件を満たす:**
- ✅ `thiserror`による構造化エラー
- ✅ 詳細な診断情報（maturity, residual, iterations等）
- ✅ `From`トレイトによるエラー変換

---

### Requirement 10: Configuration Serialization（設定のシリアライゼーション）

| 項目 | 状態 |
|------|------|
| ギャップレベル | **MEDIUM** - serde対応の追加が必要 |

**既存コード:**
- `infra_master`の多くの型に`#[cfg_attr(feature = "serde", derive(...))]`あり
- `GenericBootstrapConfig`にはserde deriveなし
- `BootstrapInterpolation`にもserde deriveなし

**必要な作業:**
- `GenericBootstrapConfig<T>` に `Serialize/Deserialize` 追加
- `BootstrapInterpolation` に `Serialize/Deserialize` 追加
- 新規 `CurveDefinition` に serde 対応

---

## 3. 実装アプローチ選択肢

### Option A: 既存モジュール拡張（推奨）

**概要:** `pricer_models/src/market/calibration/bootstrapping/` を拡張

**メリット:**
- 既存のテスト・ドキュメントを活用
- A-I-P-S依存ルールに準拠（Pクレート内完結）
- 既存ユーザーへの影響最小

**デメリット:**
- `infra_master`との統合にAdapterパターンが必要
- クレート境界を跨ぐ設計が複雑化

**新規ファイル:**
```
crates/pricer_models/src/market/calibration/bootstrapping/
├── definition.rs    # CurveDefinition, InstrumentSpec
├── result_cache.rs  # CurveResultCache (LRU)
└── adapter.rs       # infra_master → BootstrapInstrument 変換
```

### Option B: 新規サブモジュール作成

**概要:** `pricer_models/src/market/curve_builder/` として新規作成

**メリット:**
- クリーンな設計、既存コードへの影響なし
- 新しいAPIを自由に設計可能

**デメリット:**
- 既存`bootstrapping`モジュールとの重複
- テスト・ドキュメントの再作成が必要

### Option C: Adapterクレート新設

**概要:** `adapter_curve` クレートを新設し、`infra_master` ↔ `pricer_models` の橋渡し

**メリット:**
- A-I-P-S依存ルールに完全準拠
- 明確な責務分離

**デメリット:**
- クレート数増加による複雑化
- ビルド時間への影響

---

## 4. 技術的リスクと調査項目

### Risk 1: Dual型互換性
**リスク:** 現在の`SensitivityBootstrapper`が`f64`専用
**調査:** `BootstrappedCurve<Dual>`のインスタンス化テスト
**軽減策:** ジェネリック化またはtrait abstractionの導入

### Risk 2: キャッシュのメモリ使用量
**リスク:** 大量のカーブをキャッシュするとメモリ圧迫
**調査:** カーブ1本あたりのメモリフットプリント測定
**軽減策:** LRUサイズ制限、weak reference活用

### Risk 3: スレッドセーフキャッシュのパフォーマンス
**リスク:** `RwLock`のcontentionによる性能劣化
**調査:** 並列ベンチマーク実施
**軽減策:** `dashmap`等のlock-free構造検討

---

## 5. 推奨実装順序

1. **Phase 1: 基盤拡張**
   - `CurveDefinition`, `InstrumentSpec` 型定義
   - `GenericBootstrapConfig` への parameter representation 追加
   - serde対応追加

2. **Phase 2: 統合層**
   - `infra_master` → `BootstrapInstrument` アダプター
   - `SwapConvention`からのキャッシュフロー展開

3. **Phase 3: キャッシュ実装**
   - `CurveResultCache` (LRU, thread-safe)
   - キャッシュキー設計（Index + rates hash + config hash）

4. **Phase 4: インターフェース拡張**
   - `YieldCurve`トレイトに`instantaneous_forward()`追加
   - Multi-curve依存関係自動解決

5. **Phase 5: 検証・最適化**
   - `Dual`型互換性テスト
   - パフォーマンスベンチマーク
   - ドキュメント整備

---

## 6. 参照ファイル一覧

| ファイル | 関連要件 | 状態 |
|----------|----------|------|
| [bootstrapping/mod.rs](crates/pricer_models/src/market/calibration/bootstrapping/mod.rs) | 全般 | 既存 |
| [bootstrapping/config.rs](crates/pricer_models/src/market/calibration/bootstrapping/config.rs) | Req 2, 10 | 拡張必要 |
| [bootstrapping/instrument.rs](crates/pricer_models/src/market/calibration/bootstrapping/instrument.rs) | Req 3 | 統合必要 |
| [bootstrapping/engine.rs](crates/pricer_models/src/market/calibration/bootstrapping/engine.rs) | Req 4 | 完了 |
| [bootstrapping/curve.rs](crates/pricer_models/src/market/calibration/bootstrapping/curve.rs) | Req 5 | 軽微拡張 |
| [bootstrapping/multi_curve.rs](crates/pricer_models/src/market/calibration/bootstrapping/multi_curve.rs) | Req 8 | 完了 |
| [bootstrapping/sensitivity.rs](crates/pricer_models/src/market/calibration/bootstrapping/sensitivity.rs) | Req 6 | 検証必要 |
| [bootstrapping/cache.rs](crates/pricer_models/src/market/calibration/bootstrapping/cache.rs) | Req 7 | 新規必要 |
| [bootstrapping/error.rs](crates/pricer_models/src/market/calibration/bootstrapping/error.rs) | Req 9 | 完了 |
| [market/curves/traits.rs](crates/pricer_models/src/market/curves/traits.rs) | Req 5 | 軽微拡張 |
| [infra_master/trade/index.rs](crates/infra_master/src/trade/index.rs) | Req 1 | 参照 |
| [infra_master/trade/convention/swap.rs](crates/infra_master/src/trade/convention/swap.rs) | Req 1, 3 | 参照 |
| [infra_master/trade/cashflow.rs](crates/infra_master/src/trade/cashflow.rs) | Req 3 | 参照 |
