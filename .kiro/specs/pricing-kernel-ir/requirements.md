# Requirements Document

## Introduction

本仕様は、Neutryxデリバティブプライシングライブラリにおける**「Pricing Kernel IR（中間表現）」**アーキテクチャの導入を定義します。

既存のオブジェクト指向的な階層構造（`Trade → Leg → Cashflow`）を、数値計算エンジンが最適化しやすい**SoA（Structure of Arrays）形式の線形配列構造**に変換するコンパイルフェーズを導入します。これにより、SIMD命令の活用、Enzyme自動微分との親和性向上、キャッシュ局所性の改善を実現し、大規模ポートフォリオ評価のスループットを飛躍的に向上させます。

**主要コンセプト**:
- **Source**: `Trade`（階層的、日付、カレンダー、文字列）
- **Compiler**: `TradeCompiler`（日付計算、休日調整、スケジュール展開）
- **IR**: `PricingKernel`（平坦化された`f64`と`usize`の配列）

## Requirements

### Requirement 1: PricingKernel IR データ構造

**Objective:** As a クオンツ開発者, I want SoA形式のPricingKernel中間表現構造体を定義できること, so that SIMD命令とEnzyme ADに最適化されたデータレイアウトでプライシングを実行できる

#### Acceptance Criteria
1. The PricingKernel shall store payment dates as a contiguous `Vec<i32>` (days from epoch) for efficient memory access.
2. The PricingKernel shall store fixing dates as a contiguous `Vec<i32>` for observation date management.
3. The PricingKernel shall store year fractions as a contiguous `Vec<f64>` pre-computed from DayCountConvention.
4. The PricingKernel shall store notionals as a contiguous `Vec<f64>` for principal amounts.
5. The PricingKernel shall store spreads as a contiguous `Vec<f64>` for fixed spread components.
6. The PricingKernel shall store currency IDs as a contiguous `Vec<u8>` for multi-currency support.
7. The PricingKernel shall store discount curve IDs as a contiguous `Vec<u8>` for discounting reference.
8. The PricingKernel shall store forward index IDs as a contiguous `Vec<u16>` for rate index reference.
9. The PricingKernel shall maintain all arrays at equal length, representing cashflow-aligned data.
10. The PricingKernel shall be `Clone`, `Debug`, and optionally `serde::Serialize`/`Deserialize`.

### Requirement 2: Trade Compiler トレイト

**Objective:** As a クオンツ開発者, I want Trade階層構造をPricingKernel IRにコンパイルするCompilerトレイトを使用できること, so that 人間可読な取引定義と計算最適化された表現を分離できる

#### Acceptance Criteria
1. The TradeCompiler trait shall define a `compile` method accepting a `&Trade` and returning `Result<PricingKernel, CompileError>`.
2. When a Trade with fixed legs is compiled, the TradeCompiler shall expand all payment schedules to individual cashflow entries.
3. When a Trade with floating legs is compiled, the TradeCompiler shall resolve rate index references to forward index IDs.
4. When a Trade is compiled, the TradeCompiler shall apply business day adjustments using the calendar from `infra_domain`.
5. When a Trade is compiled, the TradeCompiler shall pre-compute year fractions based on the specified DayCountConvention.
6. If a Trade contains an unsupported instrument type, then the TradeCompiler shall return a `CompileError::UnsupportedInstrument`.
7. If a Trade references an undefined rate index, then the TradeCompiler shall return a `CompileError::UnknownIndex`.
8. The TradeCompiler shall support compilation of multiple trades into a single batched `PricingKernel` for portfolio evaluation.

### Requirement 3: 線形商品（Linear Products）のコンパイル

**Objective:** As a クオンツ開発者, I want IRS、Bond、FRA等の線形商品をPricingKernel IRにコンパイルできること, so that 商品タイプに依存しない統一的なプライシングループで評価できる

#### Acceptance Criteria
1. When an Interest Rate Swap (IRS) is compiled, the TradeCompiler shall generate separate entries for fixed and floating legs.
2. When a Bond is compiled, the TradeCompiler shall generate entries for coupon payments and principal redemption.
3. When a Forward Rate Agreement (FRA) is compiled, the TradeCompiler shall generate a single settlement cashflow entry.
4. When a vanilla swap with amortising notional is compiled, the TradeCompiler shall generate entries with varying notional amounts per period.
5. The compiled PricingKernel for linear products shall not contain any conditional logic (if/match) in the data representation.
6. When compiling linear products, the TradeCompiler shall sort all cashflows by payment date in ascending order.

### Requirement 4: 多通貨・X-Ccy Basis 対応

**Objective:** As a クオンツ開発者, I want X-Ccy BasisスワップやFX商品をPricingKernel IRで表現できること, so that 通貨変換を分岐なしの統一ループで処理できる

#### Acceptance Criteria
1. The PricingKernel shall include an optional `fx_index_ids: Vec<u16>` field for FX rate references.
2. When compiling a single-currency trade, the TradeCompiler shall assign a dummy FX index (identity FX=1.0) for uniformity.
3. When compiling a cross-currency swap, the TradeCompiler shall assign appropriate FX index IDs to each leg's cashflows.
4. The pricing formula `PV = flow * DF * FX` shall be applicable to all cashflows without branching on currency type.
5. The PricingKernel shall support both collateral and funding currency distinctions via separate discount curve IDs.

### Requirement 5: CMS・Convexity Adjustment 対応

**Objective:** As a クオンツ開発者, I want CMS（Constant Maturity Swap）をPricingKernel IRで表現できること, so that 凸性調整を市場モデル層に委譲しエンジンコードを汚さない

#### Acceptance Criteria
1. The PricingKernel shall use `fwd_index_ids` to reference both simple forward rates and CMS rates uniformly.
2. When a CMS coupon is compiled, the TradeCompiler shall assign a CMS-specific index ID that triggers convexity adjustment in the market model.
3. The market model interface (`get_rate(index_id, date)`) shall return convexity-adjusted rates for CMS index IDs transparently.
4. The pricing engine loop shall remain unchanged regardless of whether the index is SOFR or CMS.

### Requirement 6: 経路依存型商品（Path-Dependent Products）のIR拡張

**Objective:** As a クオンツ開発者, I want バリアオプションやアジアンオプション等の経路依存型商品をイベント駆動形式のIRで表現できること, so that 複雑なエキゾチック商品もシンプルなイベントループで評価できる

#### Acceptance Criteria
1. The system shall define a `ScriptKernel` struct with observation times, operation codes, and constant operands.
2. The ScriptKernel shall support operation codes for: CalcFixed, CalcFloat, CheckBarrier, Accumulate, Pay.
3. When a barrier option is compiled, the TradeCompiler shall generate CheckBarrier operations at each observation date.
4. When an Asian option is compiled, the TradeCompiler shall generate Accumulate operations for averaging.
5. The ScriptKernel execution shall proceed as a linear sequence of operations without runtime type dispatch.
6. If an unsupported exotic payoff is encountered, then the TradeCompiler shall return `CompileError::UnsupportedPayoff`.

### Requirement 7: Callable/Bermudan商品のブロック実行モデル

**Objective:** As a クオンツ開発者, I want Callable SwapやBermudanオプションを行使日で区切られたブロック構造で表現できること, so that Forward/Backward両パスに対応したLSMC評価を実行できる

#### Acceptance Criteria
1. The system shall define a `CallableKernel` struct containing a sequence of `CallableBlock` entries.
2. Each CallableBlock shall contain: start_date, end_date, core_flows (PricingKernel), and optional exercise_opportunity.
3. When a Bermudan swaption is compiled, the TradeCompiler shall split the underlying swap into blocks at each exercise date.
4. The execution engine shall support a Forward Pass to accumulate cashflow values to each exercise point.
5. The execution engine shall support a Backward Pass for LSMC regression at exercise points.
6. While executing a Callable product, the engine shall track continuation value and exercise value at each decision point.

### Requirement 8: プライシングエンジン統合

**Objective:** As a クオンツ開発者, I want PricingKernel IRを評価するシンプルなプライシングループを使用できること, so that 条件分岐なしのベクトル化されたコードでPV計算を実行できる

#### Acceptance Criteria
1. The pricing engine shall provide a `price_kernel` function accepting `&PricingKernel` and `&MarketData` returning `f64` PV.
2. The pricing engine main loop shall iterate over array indices without matching on instrument types.
3. The pricing engine shall use the IndexedMarket pattern for efficient market data lookup by index ID.
4. The pricing engine shall be compatible with Enzyme AD for automatic differentiation of sensitivities.
5. When pricing a batched portfolio kernel, the engine shall process all trades in a single contiguous pass.
6. The pricing engine shall support SIMD-friendly memory access patterns through aligned f64 arrays.

### Requirement 9: 日付・時間分離アーキテクチャ

**Objective:** As a クオンツ開発者, I want 契約日付（Date）と計算時間（Time）が明確に分離されたIRを使用できること, so that 静的な契約情報と動的な評価基準日を独立に管理できる

#### Acceptance Criteria
1. The PricingKernel shall store absolute dates as `i32` (days from epoch) for contractual dates.
2. The PricingKernel shall store pre-computed year fractions (τ) at compile time.
3. When evaluating, the engine shall compute relative time-to-maturity (`t`) dynamically from valuation date.
4. The separation shall allow re-evaluation at different valuation dates without recompilation.
5. The date representation shall be compatible with `infra_domain::time` calendar functions.

### Requirement 10: A-I-P-Sアーキテクチャ適合

**Objective:** As a システムアーキテクト, I want PricingKernel IRがNeutryxのA-I-P-Sデータフローに適合すること, so that 既存のレイヤー分離と3-stage rocketパターンを維持できる

#### Acceptance Criteria
1. The PricingKernel struct shall be defined in `pricer_core` (L1) as a foundational data type.
2. The TradeCompiler implementation shall reside in `pricer_models` (L2) with access to instrument definitions.
3. The pricing engine integration shall be in `pricer_pricing` (L3) for Monte Carlo and analytical evaluation.
4. The portfolio-level orchestration shall be in `pricer_risk` (L4) for batched evaluation and risk scenarios.
5. The PricingKernel shall not depend on any Service (S) or Adapter (A) layer crates.
6. The TradeCompiler shall use `infra_domain` types (Trade, Calendar, DayCountConvention) as input.
7. The design shall follow the 3-stage rocket pattern: Definition (L2) → Linking (Context) → Execution (Kernel).

### Requirement 11: パフォーマンス最適化

**Objective:** As a システムアーキテクト, I want PricingKernel IRがSIMD命令とキャッシュ効率を最大化する設計であること, so that 大規模ポートフォリオ評価で桁違いのスループットを実現できる

#### Acceptance Criteria
1. The PricingKernel arrays shall be aligned to 64-byte boundaries for AVX-512 compatibility.
2. The pricing loop shall be structured to enable LLVM auto-vectorisation.
3. The memory layout shall minimise cache misses by co-locating frequently accessed data.
4. The batched portfolio kernel shall process 10,000+ trades with linear scaling.
5. The design shall eliminate pointer indirection in the hot path.
6. While processing large portfolios, the engine shall maintain >80% CPU utilisation through Rayon parallelism.

### Requirement 12: Enzyme AD 互換性

**Objective:** As a クオンツ開発者, I want PricingKernel IRがEnzyme自動微分と完全互換であること, so that 高速なAADベースのGreeks計算を実行できる

#### Acceptance Criteria
1. The PricingKernel struct shall contain only Enzyme-compatible types (primitives, arrays).
2. The pricing function shall be free of control flow that breaks Enzyme differentiation.
3. The pricing function shall use smooth approximations for any discontinuous operations.
4. The PricingKernel pricing shall support forward-mode and reverse-mode AD via Enzyme.
5. If using num-dual fallback, the pricing kernel shall produce identical results for verification.
6. The design shall minimise generic type parameters to reduce Enzyme compilation complexity.

