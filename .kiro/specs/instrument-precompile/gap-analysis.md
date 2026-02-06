# Gap Analysis: instrument-precompile

## 概要

本分析は、`instrument-precompile` 仕様の要件と既存コードベースのギャップを調査し、実装アプローチを評価する。

---

## 1. 現状調査

### 1.1 関連モジュールとファイル構造

| モジュール | ファイル | 目的 |
|-----------|----------|------|
| `infra_domain::market` | `market_instrument.rs` | MarketInstrument (CF-expandable) - `to_trade()` でキャッシュフロー展開 |
| `pricer_models::market::curves` | `market.rs` | `MarketInstrument<T>` enum (Ois, Irs, Fra, Future, Event) - キャリブレーション用軽量型 |
| `pricer_models::builder` | `instrument.rs` | `CalibrationInstrument<T>` トレイト |
| `pricer_models::builder` | `problem.rs` | `CalibrationProblem<T, I>` - Newton 法ソルバー |
| `pricer_models::builder` | `matrix.rs` | `CalibrationMatrix<T>`, `InterpolationMatrix<T>` |
| `pricer_models::builder::construction` | `engine.rs` | `CurveConstructionEngine` - CurveDefinition からカーブ構築 |
| `pricer_models::compiler` | `mod.rs` | `TradeCompiler` トレイト - Trade → PricingKernel IR |
| `pricer_core::kernel` | `pricing_kernel.rs` | `PricingKernel` - SoA IR (64-byte aligned) |
| `infra_domain::market::definition` | `curve.rs` | `CurveDefinition` - カーブ構築レシピ |

### 1.2 既存パターンの分析

#### 現在のデータフロー

```text
CurveDefinition → CurveConstructionEngine → CurveBootstrapper
                                                    ↓
                                          CalibrationProblem<T, MarketInstrument<T>>
                                                    ↓
                                          [イテレーション毎に theoretical_rate() を計算]
                                                    ↓
                                          BootstrappedCurve<T>
```

#### 2つの MarketInstrument 型

1. **`infra_domain::market::MarketInstrument`**
   - `MarketConvention` + `Rate` を結合
   - `to_trade()` でキャッシュフロー (Leg, Cashflow) に展開
   - **イテレーション毎にカレンダー演算が発生**

2. **`pricer_models::market::curves::MarketInstrument<T>`**
   - 軽量 enum (Ois, Irs, Fra, Future, Event)
   - `CalibrationInstrument<T>` を実装
   - `theoretical_rate()` でスケジュール計算を繰り返す

#### 既存の TradeCompiler パターン

```rust
pub trait TradeCompiler<T> {
    fn compile(&self, trade: &T) -> Result<PricingKernel, CompileError>;
    fn compile_batch<'a, I>(&self, trades: I) -> Result<PricingKernel, CompileError>;
}
```

- `LinearProductsCompiler`: Trade → PricingKernel (SoA, SIMD-friendly)
- `IndexMapper`: RateIndex/Currency → numeric ID

#### 既存の InterpolationMatrix

```rust
pub struct InterpolationMatrix<T: Float + RealField + Copy> {
    matrix: DMatrix<T>,  // M grid points × P pillars
    num_points: usize,
    num_pillars: usize,
}

impl InterpolationMatrix {
    fn interpolate(&self, pillar_values: &[T]) -> Vec<T>;
    fn interpolate_df(&self, log_df_pillars: &[T]) -> Vec<T>;
}
```

### 1.3 統合ポイント

| ポイント | 説明 |
|---------|------|
| `CurveConstructionEngine::build()` | CurveDefinition からカーブ構築のエントリポイント |
| `CalibrationProblem::new()` | 現在の CalibrationInstrument 受け取り |
| `CalibrationProblem::compute_residuals()` | 価格誤差計算 - **最適化対象** |
| `CalibrationProblem::compute_jacobian_finite_diff()` | ヤコビアン計算 - 繰り返し評価 |

---

## 2. 要件実現可能性分析

### 2.1 要件-資産マッピング

| 要件 | 既存資産 | ギャップ | 状態 |
|------|---------|---------|------|
| Req 1: Instrument Compiler | `TradeCompiler` パターン存在 | キャリブレーション専用コンパイラなし | Missing |
| Req 2: CalibrationProblem統合 | `CalibrationProblem` 存在 | `from_curve_definition()` API なし | Missing |
| Req 3: Pricing Error効率化 | `CalibrationInstrument::pricing_error()` | ベクトル積計算への変換なし | Missing |
| Req 4: Interpolation Matrix | `InterpolationMatrix` 存在 | CSR形式ではない (Dense DMatrix) | Partial |
| Req 5: ドメイン分離 | A-I-P-S アーキテクチャ | 明確に分離済み | ✅ Exists |
| Req 6: 後方互換性 | 既存API安定 | 新API追加のみで対応可能 | ✅ Exists |
| Req 7: パフォーマンス検証 | `criterion` ベンチマーク基盤 | キャリブレーション用ベンチマークなし | Missing |
| Req 8: エラーハンドリング | `thiserror` 使用 | `CompileError` 既存、拡張必要 | Partial |

### 2.2 技術的課題

#### 課題 1: 2つの MarketInstrument 型の橋渡し

- `infra_domain::market::MarketInstrument` (CF-expandable) と
- `pricer_models::market::curves::MarketInstrument<T>` (calibration enum)
- **解決策**: 新しい `CompiledInstrument` 型を導入し、両者からの変換を提供

#### 課題 2: 汎用性 vs 効率性のトレードオフ

- 現在の `CalibrationInstrument<T>` トレイトは汎用的
- コンパイル済み商品は特化型になる
- **解決策**: `CompiledInstrument` も `CalibrationInstrument<T>` を実装

#### 課題 3: InterpolationMatrix のメモリ効率

- 現在は Dense DMatrix (nalgebra)
- CSR形式への変換は追加作業が必要
- **Research Needed**: sprs クレートの評価

### 2.3 複雑性シグナル

| カテゴリ | 評価 |
|---------|------|
| アルゴリズム複雑性 | **Medium** - 補間行列の事前計算 |
| 統合複雑性 | **Low** - 既存パターンに従う |
| 外部依存 | **Low** - 新規依存なし |
| アーキテクチャ影響 | **Low** - 新規型追加のみ |

---

## 3. 実装アプローチオプション

### Option A: 既存コンポーネント拡張

**説明**: `pricer_models::market::curves::MarketInstrument<T>` を拡張し、事前計算フィールドを追加

**変更ファイル**:
- `pricer_models/src/market.rs` - `MarketInstrument<T>` にフィールド追加
- `pricer_models/src/builder/instrument.rs` - `CalibrationInstrument` 実装更新

**トレードオフ**:
- ✅ 最小限のファイル変更
- ✅ 既存テストの再利用
- ❌ 既存の `MarketInstrument<T>` enum が肥大化
- ❌ 後方互換性リスク (フィールド追加)

**推奨度**: ⭐⭐ (非推奨 - 責務が混在)

### Option B: 新規 CompiledInstrument 型

**説明**: `pricer_models::builder::compiled.rs` に新しい `CompiledInstrument<T>` 型を作成

**新規ファイル**:
- `pricer_models/src/builder/compiled.rs` - `CompiledInstrument<T>`
- `pricer_models/src/builder/compiler.rs` - `InstrumentCompiler`

**変更ファイル**:
- `pricer_models/src/builder/mod.rs` - モジュール追加
- `pricer_models/src/builder/problem.rs` - `from_curve_definition()` 追加
- `pricer_models/src/builder/construction/engine.rs` - コンパイル統合

**トレードオフ**:
- ✅ 明確な責務分離
- ✅ 既存コードへの影響最小
- ✅ テスト容易性
- ❌ 新規ファイル追加

**推奨度**: ⭐⭐⭐⭐⭐ (推奨)

### Option C: ハイブリッドアプローチ

**説明**: Phase 1 で軽量コンパイル、Phase 2 で完全最適化

**Phase 1**:
- `CompiledInstrument` 基本実装 (日付、年率係数の事前計算)
- `CalibrationProblem::from_compiled()` API

**Phase 2**:
- CSR 形式 `InterpolationMatrix`
- SIMD 最適化ベクトル積

**トレードオフ**:
- ✅ 段階的な価値提供
- ✅ リスク分散
- ❌ 複数フェーズの管理

**推奨度**: ⭐⭐⭐⭐ (複雑な場合に推奨)

---

## 4. 実装複雑性とリスク

### 工数見積もり

| オプション | 工数 | 根拠 |
|-----------|------|------|
| Option A | **S** (1-3日) | 既存コード変更のみ |
| Option B | **M** (3-7日) | 新規型 + 統合テスト |
| Option C | **M-L** (5-10日) | 2フェーズ + 最適化 |

### リスク評価

| リスク要因 | レベル | 軽減策 |
|-----------|-------|--------|
| 後方互換性 | **Low** | 新規API追加のみ |
| パフォーマンス目標 | **Medium** | 早期ベンチマーク |
| 型変換オーバーヘッド | **Low** | コンパイル時のみ |
| 既存テスト破損 | **Low** | 既存API維持 |

**総合リスク**: **Low-Medium**

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option B: 新規 CompiledInstrument 型** を推奨

### 主要設計決定

1. **CompiledInstrument<T> の構造**: SoA vs AoS レイアウト
2. **CalibrationProblem への統合方式**: ジェネリクス vs 専用メソッド
3. **InterpolationMatrix の最適化**: Dense vs CSR

### Research Items (設計フェーズで調査)

| 項目 | 説明 |
|------|------|
| CSR 行列ライブラリ | `sprs` クレートの nalgebra 互換性評価 |
| SIMD ベクトル積 | portable_simd vs platform-specific intrinsics |
| メモリアライメント | AlignedBuffer 再利用 vs 新規実装 |

### 次のステップ

1. `/kiro:spec-design instrument-precompile` を実行して設計フェーズへ進む
2. 設計フェーズで上記 Research Items を調査
3. CompiledInstrument の詳細 API を定義

---

## 付録: 既存コード参照

### CalibrationInstrument トレイト

```rust
pub trait CalibrationInstrument<T: Float>: Clone {
    fn market_rate(&self) -> T;
    fn theoretical_rate<C: YieldCurve<T>>(&self, curve: &C) -> Result<T, MarketDataError>;
    fn maturity(&self) -> T;
    fn pricing_error<C: YieldCurve<T>>(&self, curve: &C) -> Result<T, MarketDataError>;
    fn instrument_type(&self) -> &'static str;
}
```

### PricingKernel (参考: 類似の SoA 設計)

```rust
pub struct PricingKernel {
    pub payment_dates: AlignedBuffer<i32>,
    pub year_fractions: AlignedBuffer<f64>,
    pub notionals: AlignedBuffer<f64>,
    pub spreads: AlignedBuffer<f64>,
    pub gearings: AlignedBuffer<f64>,
    // ...
}
```

---

_生成日: 2026-02-06_
_ドキュメントバージョン: 1.0_
